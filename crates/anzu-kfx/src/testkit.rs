//! Test-only helpers: a minimal writer of binary Ion and of KFX containers.

use crate::ion::Ion;

pub const BVM: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];

pub fn var_uint(mut n: u64) -> Vec<u8> {
    let mut out = vec![(n & 0x7F) as u8 | 0x80];
    n >>= 7;
    while n > 0 {
        out.insert(0, (n & 0x7F) as u8);
        n >>= 7;
    }
    out
}

fn tlv(kind: u8, body: &[u8]) -> Vec<u8> {
    let mut out = if body.len() < 14 {
        vec![(kind << 4) | body.len() as u8]
    } else {
        let mut h = vec![(kind << 4) | 14];
        h.extend(var_uint(body.len() as u64));
        h
    };
    out.extend_from_slice(body);
    out
}

fn magnitude(n: u64) -> Vec<u8> {
    let bytes = n.to_be_bytes();
    let skip = bytes.iter().take_while(|b| **b == 0).count();
    bytes[skip..].to_vec()
}

/// Encodes one value, without a version marker.
pub fn encode(value: &Ion) -> Vec<u8> {
    match value {
        Ion::Null => vec![0x0F],
        Ion::Bool(b) => vec![0x10 | u8::from(*b)],
        Ion::Int(i) if *i >= 0 => tlv(2, &magnitude(*i as u64)),
        Ion::Int(i) => tlv(3, &magnitude(i.unsigned_abs())),
        Ion::Float(f) => tlv(4, &f.to_be_bytes()),
        Ion::Symbol(s) => tlv(7, &magnitude(u64::from(*s))),
        Ion::String(s) => tlv(8, s.as_bytes()),
        Ion::Clob(b) => tlv(9, b),
        Ion::Blob(b) => tlv(10, b),
        Ion::List(items) => tlv(11, &items.iter().flat_map(encode).collect::<Vec<u8>>()),
        Ion::Sexp(items) => tlv(12, &items.iter().flat_map(encode).collect::<Vec<u8>>()),
        Ion::Struct(fields) => {
            let mut body = Vec::new();
            for (sid, v) in fields {
                body.extend(var_uint(u64::from(*sid)));
                body.extend(encode(v));
            }
            tlv(13, &body)
        }
        Ion::Annotated(annots, inner) => {
            let ids: Vec<u8> = annots.iter().flat_map(|a| var_uint(u64::from(*a))).collect();
            let mut body = var_uint(ids.len() as u64);
            body.extend(ids);
            body.extend(encode(inner));
            tlv(14, &body)
        }
        Ion::Decimal(_) | Ion::Timestamp(_) => unimplemented!("the test kit does not write this type"),
    }
}

/// A complete Ion stream holding a single value.
pub fn ion_doc(value: &Ion) -> Vec<u8> {
    [&BVM[..], &encode(value)].concat()
}

pub fn st(fields: Vec<(u32, Ion)>) -> Ion {
    Ion::Struct(fields)
}

pub fn s(text: &str) -> Ion {
    Ion::String(text.to_string())
}

pub fn sym(sid: u32) -> Ion {
    Ion::Symbol(sid)
}

pub fn int(n: i64) -> Ion {
    Ion::Int(n)
}

pub fn list(items: Vec<Ion>) -> Ion {
    Ion::List(items)
}

/// First local symbol identifier with the known `YJ_symbols` table.
pub const FIRST_LOCAL: u32 = 860;

pub struct TestEntity {
    pub id: u32,
    pub ftype: u32,
    /// For an Ion fragment, a complete stream (`ion_doc`); for a raw one, its bytes.
    pub payload: Vec<u8>,
}

fn entity_bytes(payload: &[u8]) -> Vec<u8> {
    let info = ion_doc(&st(vec![(410, int(0)), (411, int(0))]));
    let header_len = 10 + info.len() as u32;
    let mut out = b"ENTY".to_vec();
    out.extend(1u16.to_le_bytes());
    out.extend(header_len.to_le_bytes());
    out.extend(info);
    out.extend_from_slice(payload);
    out
}

/// Builds a `CONT` container with the given local symbols and entities.
/// With `locals` empty, the container declares no symbol table.
pub fn build_container(locals: &[&str], entities: &[TestEntity]) -> Vec<u8> {
    const HEADER: usize = 18;

    let doc_symbols = if locals.is_empty() {
        Vec::new()
    } else {
        let import = st(vec![(4, s("YJ_symbols")), (5, int(10)), (8, int(859))]);
        let table = st(vec![(6, list(vec![import])), (7, list(locals.iter().map(|l| s(l)).collect()))]);
        ion_doc(&Ion::Annotated(vec![3], Box::new(table)))
    };

    let mut payload_area = Vec::new();
    let mut index = Vec::new();
    for e in entities {
        let bytes = entity_bytes(&e.payload);
        index.extend(e.id.to_le_bytes());
        index.extend(e.ftype.to_le_bytes());
        index.extend((payload_area.len() as u64).to_le_bytes());
        index.extend((bytes.len() as u64).to_le_bytes());
        payload_area.extend(bytes);
    }

    let symbols_offset = HEADER;
    let index_offset = symbols_offset + doc_symbols.len();
    let info_offset = index_offset + index.len();
    let info = ion_doc(&st(vec![
        (409, s("CR!TEST")),
        (410, int(0)),
        (411, int(0)),
        (412, int(4096)),
        (413, int(index_offset as i64)),
        (414, int(index.len() as i64)),
        (415, int(symbols_offset as i64)),
        (416, int(doc_symbols.len() as i64)),
    ]));
    let header_len = info_offset + info.len();

    let mut out = b"CONT".to_vec();
    out.extend(2u16.to_le_bytes());
    out.extend((header_len as u32).to_le_bytes());
    out.extend((info_offset as u32).to_le_bytes());
    out.extend((info.len() as u32).to_le_bytes());
    assert_eq!(out.len(), HEADER);
    out.extend(doc_symbols);
    out.extend(index);
    out.extend(info);
    out.extend(payload_area);
    out
}

/// Builds the fragments of a test book in memory.
pub struct BookBuilder {
    pub fragments: crate::container::Fragments,
}

impl BookBuilder {
    pub fn new() -> Self {
        BookBuilder { fragments: crate::container::Fragments::new() }
    }

    /// Identifier of a local symbol, registering it if needed.
    pub fn local(&mut self, name: &str) -> u32 {
        if let Some(id) = self.fragments.symbols.id(name) {
            return id;
        }
        self.fragments.symbols.extend_local(vec![name.to_string()]);
        self.fragments.symbols.id(name).expect("the symbol was just registered")
    }

    /// A local symbol as an Ion value.
    pub fn lsym(&mut self, name: &str) -> Ion {
        Ion::Symbol(self.local(name))
    }

    pub fn add(&mut self, ftype: &str, fid: &str, value: Ion) {
        self.local(fid);
        self.fragments.items.push(crate::container::Fragment {
            ftype: ftype.to_string(),
            fid: fid.to_string(),
            value: crate::container::FragmentValue::Ion(value),
        });
    }

    pub fn add_raw(&mut self, ftype: &str, fid: &str, bytes: Vec<u8>) {
        self.fragments.items.push(crate::container::Fragment {
            ftype: ftype.to_string(),
            fid: fid.to_string(),
            value: crate::container::FragmentValue::Raw(bytes),
        });
    }
}
