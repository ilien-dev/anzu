//! Contenedor KFX (`CONT`) y archivos `.kfx-zip`.
//!
//! Un libro KFX es un conjunto de fragmentos, cada uno con un tipo y un identificador.
//! En disco se reparten en uno o varios contenedores:
//!
//! ```text
//! "CONT" | versión u16 | long. cabecera u32 | offset info u32 | long. info u32
//! container_info (Ion): offsets de la tabla de entidades y de los símbolos del documento
//! tabla de entidades: registros de 24 bytes (id u32, tipo u32, offset u64, longitud u64)
//! entidades: "ENTY" | versión u16 | long. cabecera u32 | info (Ion) | datos
//! ```
//!
//! Todos los enteros de la cabecera son little-endian. Referencia de formato:
//! `kfxlib/kfx_container.py` del plugin KFX Input de John Howell (GPL-3.0).

use std::io::Read;
use std::path::Path;

use crate::Error;
use crate::ion::{self, Ion};
use crate::symbols::{SymbolTable, YJ_MAX_ID};

const CONT_SIGNATURE: &[u8] = b"CONT";
const ENTY_SIGNATURE: &[u8] = b"ENTY";
const DRMION_SIGNATURE: &[u8] = b"\xeaDRMION\xee";
const ZIP_SIGNATURE: &[u8] = b"PK\x03\x04";

const CONT_MIN_HEADER: usize = 18;
const ENTY_MIN_HEADER: usize = 10;
const INDEX_RECORD: usize = 24;

const SID_IMPORTS: u32 = 6;
const SID_SYMBOLS: u32 = 7;
const SID_MAX_ID: u32 = 8;
const SYSTEM_SYMBOLS: i64 = 9;

const INFO_INDEX_OFFSET: u32 = 413;
const INFO_INDEX_LENGTH: u32 = 414;
const INFO_SYMBOLS_OFFSET: u32 = 415;
const INFO_SYMBOLS_LENGTH: u32 = 416;

/// Tipos cuyo contenido son bytes en bruto (imágenes, fuentes) y no Ion.
const RAW_TYPES: [u32; 2] = [417, 418];
/// Identificador de los fragmentos únicos de su tipo.
const SINGLETON_ID: u32 = 348;

const MAX_ZIP_ENTRY: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum FragmentValue {
    Ion(Ion),
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fragment {
    pub ftype: String,
    pub fid: String,
    pub value: FragmentValue,
}

#[derive(Debug, Clone)]
pub struct Fragments {
    pub items: Vec<Fragment>,
    pub symbols: SymbolTable,
    symbols_loaded: bool,
}

impl Default for Fragments {
    fn default() -> Self {
        Self::new()
    }
}

impl Fragments {
    pub fn new() -> Self {
        Fragments { items: Vec::new(), symbols: SymbolTable::yj(), symbols_loaded: false }
    }

    pub fn of_type<'a>(&'a self, ftype: &'a str) -> impl Iterator<Item = &'a Fragment> + 'a {
        self.items.iter().filter(move |f| f.ftype == ftype)
    }

    pub fn get(&self, ftype: &str, fid: &str) -> Option<&Fragment> {
        self.items.iter().find(|f| f.ftype == ftype && f.fid == fid)
    }
}

fn bad(msg: &str) -> Error {
    Error::Container(msg.to_string())
}

fn slice(data: &[u8], offset: usize, len: usize) -> Result<&[u8], Error> {
    offset
        .checked_add(len)
        .filter(|end| *end <= data.len())
        .map(|end| &data[offset..end])
        .ok_or_else(|| bad("un bloque apunta fuera del archivo"))
}

fn u32_at(data: &[u8], at: usize) -> Result<usize, Error> {
    let b = slice(data, at, 4)?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

fn u64_at(data: &[u8], at: usize) -> Result<usize, Error> {
    let b = slice(data, at, 8)?;
    let mut raw = [0u8; 8];
    raw.copy_from_slice(b);
    usize::try_from(u64::from_le_bytes(raw)).map_err(|_| bad("offset fuera de rango"))
}

/// Un contenedor con la cabecera ya validada.
struct Layout<'a> {
    data: &'a [u8],
    header_len: usize,
    info: Ion,
}

impl<'a> Layout<'a> {
    fn parse(data: &'a [u8]) -> Result<Self, Error> {
        if data.starts_with(DRMION_SIGNATURE) {
            return Err(Error::StillEncrypted);
        }
        if !data.starts_with(CONT_SIGNATURE) {
            return Err(Error::NotKfx);
        }
        if data.len() < CONT_MIN_HEADER {
            return Err(bad("cabecera truncada"));
        }
        let header_len = u32_at(data, 6)?;
        if header_len < CONT_MIN_HEADER || header_len > data.len() {
            return Err(bad("longitud de cabecera fuera de rango"));
        }
        let info_bytes = slice(data, u32_at(data, 10)?, u32_at(data, 14)?)?;
        let info = ion::parse(info_bytes)
            .map_err(|e| bad(&format!("container_info ilegible: {e}")))?
            .into_iter()
            .next()
            .ok_or_else(|| bad("container_info vacío"))?;
        Ok(Layout { data, header_len, info })
    }

    fn info_usize(&self, sid: u32) -> Result<usize, Error> {
        match self.info.field(sid).map(Ion::unannotated) {
            None | Some(Ion::Null) => Ok(0),
            Some(Ion::Int(n)) => usize::try_from(*n).map_err(|_| bad("offset negativo en container_info")),
            Some(_) => Err(bad("container_info con un campo que no es entero")),
        }
    }

    /// Los símbolos locales del documento, si este contenedor los trae.
    fn load_symbols(&self, into: &mut Fragments) -> Result<(), Error> {
        let len = self.info_usize(INFO_SYMBOLS_LENGTH)?;
        if len == 0 || into.symbols_loaded {
            return Ok(());
        }
        let bytes = slice(self.data, self.info_usize(INFO_SYMBOLS_OFFSET)?, len)?;
        let table = ion::parse(bytes)
            .map_err(|e| bad(&format!("tabla de símbolos ilegible: {e}")))?
            .into_iter()
            .next()
            .ok_or_else(|| bad("tabla de símbolos vacía"))?;

        // En el archivo, `max_id` cuenta también los símbolos de sistema.
        let shared = match table.field(SID_IMPORTS).map(Ion::unannotated) {
            Some(Ion::List(imports)) => imports
                .iter()
                .find_map(|i| match i.field(SID_MAX_ID).map(Ion::unannotated) {
                    Some(Ion::Int(n)) => u32::try_from((*n - SYSTEM_SYMBOLS).max(0)).ok(),
                    _ => None,
                })
                .unwrap_or(YJ_MAX_ID),
            _ => YJ_MAX_ID,
        };
        let locals = match table.field(SID_SYMBOLS).map(Ion::unannotated) {
            Some(Ion::List(items)) => items
                .iter()
                .map(|i| match i.unannotated() {
                    Ion::String(s) => s.clone(),
                    _ => String::new(),
                })
                .collect(),
            _ => Vec::new(),
        };
        into.symbols = SymbolTable::with_import_max(shared);
        into.symbols.extend_local(locals);
        into.symbols_loaded = true;
        Ok(())
    }

    fn load_entities(&self, into: &mut Fragments) -> Result<(), Error> {
        let len = self.info_usize(INFO_INDEX_LENGTH)?;
        if len == 0 {
            return Ok(());
        }
        let index = slice(self.data, self.info_usize(INFO_INDEX_OFFSET)?, len)?;
        if index.len() % INDEX_RECORD != 0 {
            return Err(bad("tabla de entidades con tamaño incorrecto"));
        }
        for record in index.chunks_exact(INDEX_RECORD) {
            let id = u32_at(record, 0)? as u32;
            let ftype = u32_at(record, 4)? as u32;
            let start = self
                .header_len
                .checked_add(u64_at(record, 8)?)
                .ok_or_else(|| bad("offset de entidad fuera de rango"))?;
            let bytes = slice(self.data, start, u64_at(record, 16)?)?;
            let fragment = entity(bytes, id, ftype, &into.symbols)?;
            into.items.push(fragment);
        }
        Ok(())
    }
}

fn entity(bytes: &[u8], id: u32, ftype: u32, symbols: &SymbolTable) -> Result<Fragment, Error> {
    if !bytes.starts_with(ENTY_SIGNATURE) || bytes.len() < ENTY_MIN_HEADER {
        return Err(bad("entidad sin firma ENTY"));
    }
    let header_len = u32_at(bytes, 6)?;
    if header_len < ENTY_MIN_HEADER || header_len > bytes.len() {
        return Err(bad("cabecera de entidad fuera de rango"));
    }
    let payload = &bytes[header_len..];
    let name = |sid: u32| symbols.name(sid).map_or_else(|| format!("${sid}"), str::to_string);
    let type_name = name(ftype);
    let fid = if id == SINGLETON_ID { type_name.clone() } else { name(id) };

    let value = if RAW_TYPES.contains(&ftype) {
        FragmentValue::Raw(payload.to_vec())
    } else {
        let parsed = ion::parse(payload)?.into_iter().next().ok_or_else(|| bad("entidad sin contenido"))?;
        FragmentValue::Ion(match parsed {
            Ion::Annotated(annots, inner) if id == SINGLETON_ID && annots.contains(&ftype) => *inner,
            other => other,
        })
    };
    Ok(Fragment { ftype: type_name, fid, value })
}

/// Lee un contenedor `CONT` y añade sus fragmentos.
pub fn read_container(bytes: &[u8], into: &mut Fragments) -> Result<(), Error> {
    let layout = Layout::parse(bytes)?;
    layout.load_symbols(into)?;
    layout.load_entities(into)
}

/// Abre un `.kfx-zip` (todas sus entradas `CONT`) o un contenedor KFX suelto.
pub fn open(path: &Path) -> Result<Fragments, Error> {
    let data = std::fs::read(path)?;
    let mut fragments = Fragments::new();
    if !data.starts_with(ZIP_SIGNATURE) {
        read_container(&data, &mut fragments)?;
        return Ok(fragments);
    }

    let bad_zip = |e: zip::result::ZipError| bad(&format!("zip no válido: {e}"));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&data)).map_err(bad_zip)?;
    let mut containers: Vec<Vec<u8>> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(bad_zip)?;
        if entry.is_dir() {
            continue;
        }
        let mut bytes = Vec::new();
        (&mut entry).take(MAX_ZIP_ENTRY).read_to_end(&mut bytes)?;
        if bytes.starts_with(DRMION_SIGNATURE) {
            return Err(Error::StillEncrypted);
        }
        if bytes.starts_with(CONT_SIGNATURE) {
            containers.push(bytes);
        }
    }
    if containers.is_empty() {
        return Err(Error::NotKfx);
    }

    // Los símbolos del documento pueden venir en cualquier contenedor: primero se cargan
    // todos, y solo después se resuelven los nombres de las entidades.
    let layouts = containers.iter().map(|c| Layout::parse(c)).collect::<Result<Vec<_>, _>>()?;
    for layout in &layouts {
        layout.load_symbols(&mut fragments)?;
    }
    for layout in &layouts {
        layout.load_entities(&mut fragments)?;
    }
    Ok(fragments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::*;
    use std::io::Write;

    fn sample_entities() -> Vec<TestEntity> {
        vec![
            TestEntity { id: FIRST_LOCAL, ftype: 260, payload: ion_doc(&st(vec![(174, sym(FIRST_LOCAL))])) },
            TestEntity { id: FIRST_LOCAL + 1, ftype: 417, payload: vec![0xFF, 0xD8, 0xFF, 0x00, 0x01] },
        ]
    }

    fn sample() -> Vec<u8> {
        build_container(&["sec1", "img1"], &sample_entities())
    }

    fn read(bytes: &[u8]) -> Result<Fragments, Error> {
        let mut f = Fragments::new();
        read_container(bytes, &mut f)?;
        Ok(f)
    }

    #[test]
    fn encrypted_book_is_reported_as_such() {
        let mut bytes = b"\xeaDRMION\xee".to_vec();
        bytes.extend([0u8; 64]);
        assert!(matches!(read(&bytes), Err(Error::StillEncrypted)));
    }

    #[test]
    fn unknown_signature_is_not_kfx() {
        assert!(matches!(read(b"PK\x03\x04 whatever, long enough to have a header"), Err(Error::NotKfx)));
        assert!(matches!(read(b""), Err(Error::NotKfx)));
        assert!(matches!(read(b"CON"), Err(Error::NotKfx)));
    }

    #[test]
    fn reads_ion_and_raw_entities_with_resolved_names() {
        let f = read(&sample()).unwrap();
        assert_eq!(f.items.len(), 2);

        let section = f.get("$260", "sec1").expect("sección");
        assert_eq!(section.value, FragmentValue::Ion(st(vec![(174, sym(FIRST_LOCAL))])));

        let image = f.get("$417", "img1").expect("recurso crudo");
        assert_eq!(image.value, FragmentValue::Raw(vec![0xFF, 0xD8, 0xFF, 0x00, 0x01]));

        assert_eq!(f.of_type("$260").count(), 1);
        assert_eq!(f.of_type("$999").count(), 0);
        assert!(f.get("$260", "nope").is_none());
        assert_eq!(f.symbols.name(FIRST_LOCAL + 1), Some("img1"));
    }

    #[test]
    fn singleton_fragments_take_their_type_as_id() {
        // id $348 y valor anotado con su propio tipo: el fragmento se identifica por el tipo.
        let value = Ion::Annotated(vec![538], Box::new(st(vec![(169, list(vec![]))])));
        let bytes = build_container(&[], &[TestEntity { id: 348, ftype: 538, payload: ion_doc(&value) }]);
        let f = read(&bytes).unwrap();
        let frag = f.get("$538", "$538").expect("fragmento único");
        assert_eq!(frag.value, FragmentValue::Ion(st(vec![(169, list(vec![]))])));
    }

    #[test]
    fn ids_outside_the_symbol_table_still_get_a_name() {
        let bytes = build_container(&[], &[TestEntity { id: 5000, ftype: 260, payload: ion_doc(&int(1)) }]);
        let f = read(&bytes).unwrap();
        assert_eq!(f.items[0].fid, "$5000");
    }

    #[test]
    fn header_length_out_of_range_is_an_error() {
        let mut bytes = sample();
        bytes[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(read(&bytes), Err(Error::Container(_))));
    }

    #[test]
    fn container_info_out_of_range_is_an_error() {
        let mut bytes = sample();
        bytes[10..14].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(read(&bytes), Err(Error::Container(_))));
        let mut bytes = sample();
        bytes[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(read(&bytes), Err(Error::Container(_))));
    }

    #[test]
    fn entity_beyond_the_file_is_an_error() {
        let bytes = sample();
        let cut = &bytes[..bytes.len() - 3];
        assert!(matches!(read(cut), Err(Error::Container(_))));
    }

    #[test]
    fn entity_with_a_bad_header_is_an_error() {
        let mut bytes = sample();
        let at = bytes.windows(4).position(|w| w == b"ENTY").unwrap();
        bytes[at + 6..at + 10].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(read(&bytes), Err(Error::Container(_))));

        let mut bytes = sample();
        let at = bytes.windows(4).position(|w| w == b"ENTY").unwrap();
        bytes[at..at + 4].copy_from_slice(b"XXXX");
        assert!(matches!(read(&bytes), Err(Error::Container(_))));
    }

    #[test]
    fn every_truncation_fails_cleanly() {
        let bytes = sample();
        for len in 0..bytes.len() {
            assert!(read(&bytes[..len]).is_err(), "truncado a {len} bytes debería fallar");
        }
    }

    fn write_zip(path: &Path, entries: &[(&str, Vec<u8>)]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, data) in entries {
            w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(data).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn opens_a_kfx_zip_whose_symbols_come_in_a_later_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("book.kfx-zip");
        let main = build_container(&[], &sample_entities()[..1]);
        let meta = build_container(&["sec1", "img1"], &sample_entities()[1..]);
        write_zip(&path, &[("book.azw", main), ("notes.txt", b"hola".to_vec()), ("book.azw.md", meta)]);

        let f = open(&path).unwrap();
        assert_eq!(f.items.len(), 2);
        assert!(f.get("$260", "sec1").is_some(), "los símbolos del segundo contenedor deben valer para el primero");
        assert!(f.get("$417", "img1").is_some());
    }

    #[test]
    fn opens_a_single_container_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("book.kfx");
        std::fs::write(&path, sample()).unwrap();
        assert_eq!(open(&path).unwrap().items.len(), 2);
    }

    #[test]
    fn zip_with_an_encrypted_entry_is_still_encrypted() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("book.kfx-zip");
        write_zip(&path, &[("book.azw", b"\xeaDRMION\xee and more bytes".to_vec())]);
        assert!(matches!(open(&path), Err(Error::StillEncrypted)));
    }

    #[test]
    fn files_that_are_not_books_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let text = tmp.path().join("a.txt");
        std::fs::write(&text, b"hola, esto no es un libro").unwrap();
        assert!(matches!(open(&text), Err(Error::NotKfx)));

        let empty_zip = tmp.path().join("b.zip");
        write_zip(&empty_zip, &[("readme.txt", b"nada".to_vec())]);
        assert!(matches!(open(&empty_zip), Err(Error::NotKfx)));

        assert!(matches!(open(&tmp.path().join("missing.kfx")), Err(Error::Io(_))));
    }
}
