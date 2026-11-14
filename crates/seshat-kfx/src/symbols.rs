//! Tabla de símbolos de un documento KFX.
//!
//! Los identificadores 1..=9 son los símbolos de sistema de Ion. A continuación va la
//! tabla compartida `YJ_symbols`, cuyos símbolos se nombran por su propio identificador
//! (`$10`, `$11`, ...), y después los símbolos locales que declara cada libro.
//! Referencia de formato: `kfxlib/yj_symbol_catalog.py` del plugin KFX Input de
//! John Howell (GPL-3.0).

const SYSTEM: [&str; 9] = [
    "$ion",
    "$ion_1_0",
    "$ion_symbol_table",
    "name",
    "version",
    "imports",
    "symbols",
    "max_id",
    "$ion_shared_symbol_table",
];

/// `max_id` de la versión de `YJ_symbols` conocida: símbolos `$10` a `$859`.
pub const YJ_MAX_ID: u32 = 850;
/// Límite defensivo para un `max_id` leído de un archivo.
const MAX_IMPORT: u32 = 100_000;

#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// `names[sid - 1]`.
    names: Vec<String>,
    shared_len: usize,
}

impl SymbolTable {
    pub fn yj() -> Self {
        Self::with_import_max(YJ_MAX_ID)
    }

    /// Tabla cuya parte compartida ocupa `max_id` símbolos tras los de sistema.
    pub fn with_import_max(max_id: u32) -> Self {
        let max_id = max_id.min(MAX_IMPORT);
        let first = SYSTEM.len() as u32 + 1;
        let mut names: Vec<String> = SYSTEM.iter().map(|s| s.to_string()).collect();
        names.extend((first..first + max_id).map(|sid| format!("${sid}")));
        let shared_len = names.len();
        SymbolTable { names, shared_len }
    }

    pub fn extend_local(&mut self, names: Vec<String>) {
        self.names.extend(names);
    }

    pub fn name(&self, sid: u32) -> Option<&str> {
        let index = (sid as usize).checked_sub(1)?;
        self.names.get(index).map(String::as_str)
    }

    pub fn id(&self, name: &str) -> Option<u32> {
        if let Some(n) = name.strip_prefix('$').and_then(|d| d.parse::<u32>().ok())
            && n as usize > SYSTEM.len()
            && n as usize <= self.shared_len
        {
            return Some(n);
        }
        self.names.iter().position(|n| n == name).map(|i| i as u32 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_symbols_come_first() {
        let t = SymbolTable::yj();
        assert_eq!(t.name(1), Some("$ion"));
        assert_eq!(t.name(3), Some("$ion_symbol_table"));
        assert_eq!(t.name(4), Some("name"));
        assert_eq!(t.name(9), Some("$ion_shared_symbol_table"));
    }

    #[test]
    fn yj_symbols_are_named_after_their_id() {
        let t = SymbolTable::yj();
        assert_eq!(t.name(10), Some("$10"));
        assert_eq!(t.name(259), Some("$259"));
        assert_eq!(t.name(859), Some("$859"));
    }

    #[test]
    fn unknown_ids_have_no_name() {
        let t = SymbolTable::yj();
        assert_eq!(t.name(0), None);
        assert_eq!(t.name(860), None);
        assert_eq!(t.name(u32::MAX), None);
    }

    #[test]
    fn local_symbols_follow_the_shared_table() {
        let mut t = SymbolTable::yj();
        t.extend_local(vec!["cover".into(), "ch1".into()]);
        assert_eq!(t.name(860), Some("cover"));
        assert_eq!(t.name(861), Some("ch1"));
        assert_eq!(t.name(862), None);
        t.extend_local(vec!["ch2".into()]);
        assert_eq!(t.name(862), Some("ch2"));
    }

    #[test]
    fn import_max_id_decides_where_locals_start() {
        let mut t = SymbolTable::with_import_max(842);
        assert_eq!(t.name(851), Some("$851"));
        assert_eq!(t.name(852), None);
        t.extend_local(vec!["a".into()]);
        assert_eq!(t.name(852), Some("a"));
    }

    #[test]
    fn name_lookup_resolves_back_to_an_id() {
        let mut t = SymbolTable::yj();
        t.extend_local(vec!["cover".into()]);
        assert_eq!(t.id("$259"), Some(259));
        assert_eq!(t.id("cover"), Some(860));
        assert_eq!(t.id("nope"), None);
    }
}
