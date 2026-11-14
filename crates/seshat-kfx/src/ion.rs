//! Lector de Amazon Ion 1.0 en su codificación binaria.
//!
//! Sigue la especificación pública del formato. Cada lectura comprueba límites: un
//! archivo truncado o malformado produce un `Err`, nunca un pánico.

use crate::Error;

const BVM: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];
const MAX_DEPTH: usize = 256;
const VAR_UINT_MAX_BYTES: usize = 9;

#[derive(Debug, Clone, PartialEq)]
pub enum Ion {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    /// Coeficiente y exponente en base 10, como `"15e-1"`.
    Decimal(String),
    /// Bytes del valor en hexadecimal; la conversión no los interpreta.
    Timestamp(String),
    Symbol(u32),
    String(String),
    Clob(Vec<u8>),
    Blob(Vec<u8>),
    List(Vec<Ion>),
    Sexp(Vec<Ion>),
    Struct(Vec<(u32, Ion)>),
    Annotated(Vec<u32>, Box<Ion>),
}

impl Ion {
    /// El valor sin su envoltorio de anotaciones.
    pub fn unannotated(&self) -> &Ion {
        match self {
            Ion::Annotated(_, inner) => inner.unannotated(),
            other => other,
        }
    }

    /// El valor de un campo, si esto es un struct que lo tiene.
    pub fn field(&self, sid: u32) -> Option<&Ion> {
        match self.unannotated() {
            Ion::Struct(fields) => fields.iter().find(|(id, _)| *id == sid).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self.unannotated() {
            Ion::Int(i) => Some(*i as f64),
            Ion::Float(f) => Some(*f),
            Ion::Decimal(d) => d.parse().ok(),
            _ => None,
        }
    }
}

fn err(msg: &str) -> Error {
    Error::Ion(msg.to_string())
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    fn done(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn byte(&mut self) -> Result<u8, Error> {
        let b = *self.buf.get(self.pos).ok_or_else(|| err("datos truncados"))?;
        self.pos += 1;
        Ok(b)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(len).filter(|e| *e <= self.buf.len());
        let end = end.ok_or_else(|| err("longitud declarada mayor que los datos"))?;
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn var_uint(&mut self) -> Result<u64, Error> {
        let mut value: u64 = 0;
        for _ in 0..VAR_UINT_MAX_BYTES {
            let b = self.byte()?;
            value = (value << 7) | u64::from(b & 0x7F);
            if b & 0x80 != 0 {
                return Ok(value);
            }
        }
        Err(err("VarUInt demasiado largo"))
    }

    fn var_int(&mut self) -> Result<i64, Error> {
        let first = self.byte()?;
        let negative = first & 0x40 != 0;
        let mut value = i64::from(first & 0x3F);
        let mut last = first;
        let mut read = 1;
        while last & 0x80 == 0 {
            if read >= VAR_UINT_MAX_BYTES {
                return Err(err("VarInt demasiado largo"));
            }
            last = self.byte()?;
            value = (value << 7) | i64::from(last & 0x7F);
            read += 1;
        }
        Ok(if negative { -value } else { value })
    }

    /// Lee un valor. `None` significa relleno (NOP) que se debe ignorar.
    fn value(&mut self, depth: usize) -> Result<Option<Ion>, Error> {
        if depth > MAX_DEPTH {
            return Err(err("anidamiento excesivo"));
        }
        let descriptor = self.byte()?;
        let (kind, low) = (descriptor >> 4, descriptor & 0x0F);

        if kind == 15 {
            return Err(err("tipo reservado"));
        }
        if kind == 1 {
            return match low {
                0 => Ok(Some(Ion::Bool(false))),
                1 => Ok(Some(Ion::Bool(true))),
                15 => Ok(Some(Ion::Null)),
                _ => Err(err("booleano no válido")),
            };
        }
        if low == 15 {
            return Ok(Some(Ion::Null));
        }

        let sorted_struct = kind == 13 && low == 1;
        let len = if low == 14 || sorted_struct {
            usize::try_from(self.var_uint()?).map_err(|_| err("longitud fuera de rango"))?
        } else {
            usize::from(low)
        };
        let body = self.take(len)?;

        let value = match kind {
            0 => return Ok(None),
            2 | 3 => {
                let value = i64::try_from(magnitude(body)?).map_err(|_| err("entero fuera de rango"))?;
                Ion::Int(if kind == 3 { -value } else { value })
            }
            4 => match body.len() {
                0 => Ion::Float(0.0),
                4 => Ion::Float(f64::from(f32::from_be_bytes([body[0], body[1], body[2], body[3]]))),
                8 => {
                    let mut raw = [0u8; 8];
                    raw.copy_from_slice(body);
                    Ion::Float(f64::from_be_bytes(raw))
                }
                _ => return Err(err("tamaño de float no válido")),
            },
            5 => Ion::Decimal(decimal(body)?),
            6 => Ion::Timestamp(body.iter().map(|b| format!("{b:02x}")).collect()),
            7 => Ion::Symbol(u32::try_from(magnitude(body)?).map_err(|_| err("símbolo fuera de rango"))?),
            8 => Ion::String(std::str::from_utf8(body).map_err(|_| err("cadena con UTF-8 no válido"))?.to_string()),
            9 => Ion::Clob(body.to_vec()),
            10 => Ion::Blob(body.to_vec()),
            11 | 12 => {
                let mut inner = Reader::new(body);
                let mut items = Vec::new();
                while !inner.done() {
                    if let Some(v) = inner.value(depth + 1)? {
                        items.push(v);
                    }
                }
                if kind == 11 { Ion::List(items) } else { Ion::Sexp(items) }
            }
            13 => {
                let mut inner = Reader::new(body);
                let mut fields = Vec::new();
                while !inner.done() {
                    let sid = u32::try_from(inner.var_uint()?).map_err(|_| err("campo fuera de rango"))?;
                    if let Some(v) = inner.value(depth + 1)? {
                        fields.push((sid, v));
                    }
                }
                Ion::Struct(fields)
            }
            14 => {
                let mut inner = Reader::new(body);
                let annot_len = usize::try_from(inner.var_uint()?).map_err(|_| err("anotación fuera de rango"))?;
                let mut annots_reader = Reader::new(inner.take(annot_len)?);
                let mut annots = Vec::new();
                while !annots_reader.done() {
                    let sid = annots_reader.var_uint()?;
                    annots.push(u32::try_from(sid).map_err(|_| err("anotación fuera de rango"))?);
                }
                let wrapped = inner.value(depth + 1)?.ok_or_else(|| err("anotación sin valor"))?;
                if !inner.done() {
                    return Err(err("anotación con datos sobrantes"));
                }
                Ion::Annotated(annots, Box::new(wrapped))
            }
            _ => return Err(err("tipo desconocido")),
        };
        Ok(Some(value))
    }
}

/// Entero sin signo en big-endian de hasta 8 bytes.
fn magnitude(body: &[u8]) -> Result<u64, Error> {
    if body.len() > 8 {
        return Err(err("entero de más de 8 bytes"));
    }
    Ok(body.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b)))
}

fn decimal(body: &[u8]) -> Result<String, Error> {
    if body.is_empty() {
        return Ok("0e0".to_string());
    }
    let mut reader = Reader::new(body);
    let exponent = reader.var_int()?;
    let rest = &body[reader.pos..];
    let (negative, coefficient) = match rest.split_first() {
        None => (false, 0u64),
        Some((first, tail)) => {
            let mut bytes = vec![first & 0x7F];
            bytes.extend_from_slice(tail);
            (first & 0x80 != 0, magnitude(&bytes)?)
        }
    };
    let sign = if negative { "-" } else { "" };
    Ok(format!("{sign}{coefficient}e{exponent}"))
}

/// Lee todos los valores de nivel superior de un flujo Ion binario.
pub fn parse(bytes: &[u8]) -> Result<Vec<Ion>, Error> {
    if !bytes.starts_with(&BVM) {
        return Err(err("falta el marcador de versión de Ion"));
    }
    let mut reader = Reader::new(bytes);
    let mut values = Vec::new();
    while !reader.done() {
        if reader.buf[reader.pos..].starts_with(&BVM) {
            reader.pos += BVM.len();
            continue;
        }
        if let Some(v) = reader.value(0)? {
            values.push(v);
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BVM: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];

    fn doc(body: &[u8]) -> Vec<u8> {
        [&BVM[..], body].concat()
    }

    fn one(body: &[u8]) -> Ion {
        let mut values = parse(&doc(body)).unwrap();
        assert_eq!(values.len(), 1, "se esperaba un solo valor");
        values.remove(0)
    }

    fn var_uint(mut n: usize) -> Vec<u8> {
        let mut out = vec![(n & 0x7F) as u8 | 0x80];
        n >>= 7;
        while n > 0 {
            out.insert(0, (n & 0x7F) as u8);
            n >>= 7;
        }
        out
    }

    #[test]
    fn positive_and_negative_ints() {
        assert_eq!(one(&[0x21, 0x05]), Ion::Int(5));
        assert_eq!(one(&[0x31, 0x05]), Ion::Int(-5));
        assert_eq!(one(&[0x20]), Ion::Int(0));
        assert_eq!(one(&[0x22, 0x01, 0x00]), Ion::Int(256));
    }

    #[test]
    fn int_wider_than_i64_is_an_error() {
        let mut body = vec![0x29];
        body.extend([0xFF; 9]);
        assert!(parse(&doc(&body)).is_err());
        let mut body = vec![0x28];
        body.extend([0xFF; 8]);
        assert!(parse(&doc(&body)).is_err(), "u64::MAX no cabe en i64");
    }

    #[test]
    fn bools_and_nulls() {
        assert_eq!(one(&[0x10]), Ion::Bool(false));
        assert_eq!(one(&[0x11]), Ion::Bool(true));
        assert_eq!(one(&[0x0F]), Ion::Null);
        assert_eq!(one(&[0x2F]), Ion::Null);
        assert_eq!(one(&[0xDF]), Ion::Null);
    }

    #[test]
    fn floats() {
        assert_eq!(one(&[0x40]), Ion::Float(0.0));
        let mut body = vec![0x48];
        body.extend(1.5f64.to_be_bytes());
        assert_eq!(one(&body), Ion::Float(1.5));
        let mut body = vec![0x44];
        body.extend(2.5f32.to_be_bytes());
        assert_eq!(one(&body), Ion::Float(2.5));
    }

    #[test]
    fn decimals_keep_coefficient_and_exponent() {
        // exponente VarInt 0xC1 = -1, coeficiente 15 -> 1.5
        let v = one(&[0x52, 0xC1, 0x0F]);
        assert_eq!(v, Ion::Decimal("15e-1".into()));
        assert_eq!(v.as_f64(), Some(1.5));
        // coeficiente negativo: bit de signo en el primer byte
        assert_eq!(one(&[0x52, 0xC1, 0x8F]).as_f64(), Some(-1.5));
        // decimal vacío = 0
        assert_eq!(one(&[0x50]).as_f64(), Some(0.0));
    }

    #[test]
    fn numbers_convert_to_f64() {
        assert_eq!(Ion::Int(3).as_f64(), Some(3.0));
        assert_eq!(Ion::Float(0.5).as_f64(), Some(0.5));
        assert_eq!(Ion::String("x".into()).as_f64(), None);
    }

    #[test]
    fn symbols_strings_and_lobs() {
        assert_eq!(one(&[0x71, 0x0A]), Ion::Symbol(10));
        assert_eq!(one(&[0x72, 0x03, 0x5B]), Ion::Symbol(859));
        assert_eq!(one(&[0x83, b'a', b'b', b'c']), Ion::String("abc".into()));
        assert_eq!(one(&[0x80]), Ion::String(String::new()));
        assert_eq!(one(&[0xA2, 0x01, 0x02]), Ion::Blob(vec![1, 2]));
        assert_eq!(one(&[0x91, 0x41]), Ion::Clob(vec![0x41]));
    }

    #[test]
    fn non_ascii_strings() {
        let text = "añö—é";
        let mut body = vec![0x80 | text.len() as u8];
        body.extend(text.as_bytes());
        assert_eq!(one(&body), Ion::String(text.into()));
    }

    #[test]
    fn invalid_utf8_is_an_error() {
        assert!(parse(&doc(&[0x82, 0xFF, 0xFE])).is_err());
    }

    #[test]
    fn long_values_use_a_var_uint_length() {
        let mut body = vec![0x8E, 0x90];
        body.extend([b'x'; 16]);
        assert_eq!(one(&body), Ion::String("x".repeat(16)));
    }

    #[test]
    fn lists_and_sexps() {
        assert_eq!(one(&[0xB4, 0x21, 0x01, 0x21, 0x02]), Ion::List(vec![Ion::Int(1), Ion::Int(2)]));
        assert_eq!(one(&[0xB0]), Ion::List(vec![]));
        assert_eq!(one(&[0xC2, 0x21, 0x07]), Ion::Sexp(vec![Ion::Int(7)]));
    }

    #[test]
    fn structs() {
        assert_eq!(one(&[0xD3, 0x84, 0x21, 0x07]), Ion::Struct(vec![(4, Ion::Int(7))]));
        assert_eq!(one(&[0xD0]), Ion::Struct(vec![]));
        // L = 1: struct ordenado, con la longitud como VarUInt
        assert_eq!(one(&[0xD1, 0x83, 0x84, 0x21, 0x07]), Ion::Struct(vec![(4, Ion::Int(7))]));
    }

    #[test]
    fn struct_lookup_by_field_id() {
        let s = Ion::Struct(vec![(4, Ion::Int(7)), (5, Ion::Bool(true))]);
        assert_eq!(s.field(5), Some(&Ion::Bool(true)));
        assert_eq!(s.field(9), None);
        assert_eq!(Ion::Int(1).field(4), None);
    }

    #[test]
    fn annotations_wrap_a_value() {
        assert_eq!(
            one(&[0xE4, 0x81, 0x84, 0x21, 0x07]),
            Ion::Annotated(vec![4], Box::new(Ion::Int(7)))
        );
        let a = Ion::Annotated(vec![4], Box::new(Ion::Int(7)));
        assert_eq!(a.unannotated(), &Ion::Int(7));
        assert_eq!(Ion::Int(7).unannotated(), &Ion::Int(7));
    }

    #[test]
    fn nop_padding_is_skipped() {
        assert_eq!(parse(&doc(&[0x00, 0x21, 0x05, 0x01, 0xFF])).unwrap(), vec![Ion::Int(5)]);
        assert_eq!(
            one(&[0xD5, 0x84, 0x21, 0x07, 0x80, 0x00]),
            Ion::Struct(vec![(4, Ion::Int(7))]),
            "relleno dentro de un struct"
        );
    }

    #[test]
    fn several_top_level_values_and_repeated_markers() {
        let bytes = [&BVM[..], &[0x21, 0x01], &BVM[..], &[0x21, 0x02]].concat();
        assert_eq!(parse(&bytes).unwrap(), vec![Ion::Int(1), Ion::Int(2)]);
        assert!(parse(&BVM).unwrap().is_empty());
    }

    #[test]
    fn missing_version_marker_is_an_error() {
        assert!(parse(&[0x21, 0x05]).is_err());
        assert!(parse(&[]).is_err());
        assert!(parse(&[0xE0, 0x01]).is_err());
    }

    #[test]
    fn declared_length_beyond_the_buffer_is_an_error() {
        assert!(parse(&doc(&[0x85, b'a', b'b'])).is_err());
        assert!(parse(&doc(&[0xB4, 0x21, 0x01])).is_err());
        assert!(parse(&doc(&[0x8E])).is_err(), "VarUInt cortado");
        assert!(parse(&doc(&[0x8E, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0x7F, 0xFF])).is_err());
    }

    #[test]
    fn child_longer_than_its_container_is_an_error() {
        // la lista declara 2 bytes, pero su hijo declara 5
        assert!(parse(&doc(&[0xB2, 0x85, b'a', b'b', b'c', b'd', b'e'])).is_err());
    }

    #[test]
    fn reserved_type_is_an_error() {
        assert!(parse(&doc(&[0xF0])).is_err());
    }

    #[test]
    fn runaway_nesting_is_an_error_not_a_stack_overflow() {
        let mut body = vec![0x21, 0x01];
        for _ in 0..300 {
            let mut wrapped = if body.len() < 14 {
                vec![0xB0 | body.len() as u8]
            } else {
                let mut h = vec![0xBE];
                h.extend(var_uint(body.len()));
                h
            };
            wrapped.extend(&body);
            body = wrapped;
        }
        assert!(parse(&doc(&body)).is_err());
    }

    #[test]
    fn moderate_nesting_is_fine() {
        let mut body = vec![0x21, 0x01];
        for _ in 0..50 {
            let mut wrapped = if body.len() < 14 {
                vec![0xB0 | body.len() as u8]
            } else {
                let mut h = vec![0xBE];
                h.extend(var_uint(body.len()));
                h
            };
            wrapped.extend(&body);
            body = wrapped;
        }
        assert!(parse(&doc(&body)).is_ok());
    }
}
