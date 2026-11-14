use std::fmt;
use std::str::FromStr;

use crate::Error;

/// Versión de un paquete de Microsoft Store: `major.minor.build.revision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KindleVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
    pub revision: u32,
}

impl FromStr for KindleVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        let bad = || Error::BadVersion(s.to_string());
        let parts: Vec<&str> = s.trim().split('.').collect();
        if parts.len() != 4 {
            return Err(bad());
        }
        let mut n = [0u32; 4];
        for (slot, part) in n.iter_mut().zip(&parts) {
            *slot = part.parse().map_err(|_| bad())?;
        }
        Ok(KindleVersion { major: n[0], minor: n[1], build: n[2], revision: n[3] })
    }
}

impl fmt::Display for KindleVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.major, self.minor, self.build, self.revision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_store_version() {
        let v: KindleVersion = "1.1.5634.0".parse().unwrap();
        assert_eq!((v.major, v.minor, v.build, v.revision), (1, 1, 5634, 0));
        assert_eq!(v.to_string(), "1.1.5634.0");
    }

    #[test]
    fn rejects_garbage() {
        assert!("".parse::<KindleVersion>().is_err());
        assert!("1.x.3.0".parse::<KindleVersion>().is_err());
        assert!("1.1".parse::<KindleVersion>().is_err());
    }
}
