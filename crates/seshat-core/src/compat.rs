//! Regla de compatibilidad entre la app Kindle instalada y los archivers de una publicación.
//!
//! El nombre de cada ejecutable declara la versión máxima de Kindle que admite:
//! - `MSIXKFXArchiver_x64_1_<N>.exe` admite Kindle `1.1.<M>` con `M <= N`.
//! - `MSIXKFXArchiver1_<N>_final.exe` admite Kindle `1.0.<M>` con `M <= N`.
//!
//! Lo que no se puede interpretar con certeza se trata como desconocido, y eso bloquea.

use crate::KindleVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiverSupport {
    pub exe_name: String,
    pub minor: u32,
    pub max_build: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compat {
    Compatible { exe_name: String },
    TooNew { max_supported: String },
    Unknown,
}

pub fn parse_archiver(exe_name: &str) -> Option<ArchiverSupport> {
    let stem = exe_name.strip_suffix(".exe")?;
    let (minor, digits) = if let Some(rest) = stem.strip_prefix("MSIXKFXArchiver_x64_1_") {
        (1, rest)
    } else {
        (0, stem.strip_prefix("MSIXKFXArchiver1_")?.strip_suffix("_final")?)
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(ArchiverSupport { exe_name: exe_name.to_string(), minor, max_build: digits.parse().ok()? })
}

pub fn check(installed: &KindleVersion, exe_names: &[String]) -> Compat {
    if installed.major != 1 {
        return Compat::Unknown;
    }
    let same_minor: Vec<ArchiverSupport> = exe_names
        .iter()
        .filter_map(|n| parse_archiver(n))
        .filter(|a| a.minor == installed.minor)
        .collect();
    let Some(newest) = same_minor.iter().map(|a| a.max_build).max() else {
        return Compat::Unknown;
    };
    match same_minor.into_iter().filter(|a| a.max_build >= installed.build).min_by_key(|a| a.max_build) {
        Some(a) => Compat::Compatible { exe_name: a.exe_name },
        None => Compat::TooNew { max_supported: format!("1.{}.{}", installed.minor, newest) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        [
            "KFXArchiver291.exe",
            "MSIXKFXArchiver1_23620_final.exe",
            "MSIXKFXArchiver_x64_1_5634.exe",
            "compiled_agent.js",
        ]
        .map(String::from)
        .to_vec()
    }

    fn v(s: &str) -> KindleVersion {
        s.parse().unwrap()
    }

    #[test]
    fn exact_match() {
        assert!(matches!(check(&v("1.1.5634.0"), &names()),
            Compat::Compatible { exe_name } if exe_name == "MSIXKFXArchiver_x64_1_5634.exe"));
    }

    #[test]
    fn older_x64_is_compatible() {
        assert!(matches!(check(&v("1.1.4132.0"), &names()), Compat::Compatible { .. }));
    }

    #[test]
    fn picks_the_closest_archiver() {
        let mut n = names();
        n.push("MSIXKFXArchiver_x64_1_4132.exe".into());
        assert!(matches!(check(&v("1.1.4000.0"), &n),
            Compat::Compatible { exe_name } if exe_name == "MSIXKFXArchiver_x64_1_4132.exe"));
    }

    #[test]
    fn legacy_x86() {
        assert!(matches!(check(&v("1.0.22920.0"), &names()),
            Compat::Compatible { exe_name } if exe_name == "MSIXKFXArchiver1_23620_final.exe"));
    }

    #[test]
    fn newer_kindle_blocks() {
        assert!(matches!(check(&v("1.1.6000.0"), &names()),
            Compat::TooNew { max_supported } if max_supported == "1.1.5634"));
    }

    #[test]
    fn unparseable_names_are_unknown() {
        assert!(matches!(check(&v("1.1.5634.0"), &["NewTool.exe".to_string()]), Compat::Unknown));
    }

    #[test]
    fn other_major_is_unknown() {
        assert!(matches!(check(&v("2.0.1.0"), &names()), Compat::Unknown));
    }

    #[test]
    fn no_archiver_for_this_minor_is_unknown() {
        assert!(matches!(check(&v("1.2.10.0"), &names()), Compat::Unknown));
    }
}
