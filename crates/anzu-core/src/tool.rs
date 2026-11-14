//! Descarga, verificación y caché de la herramienta de desbloqueo.
//!
//! Nada se extrae ni se ejecuta si la huella SHA-256 del zip no coincide con la que
//! publica GitHub para esa descarga.

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::releases::{self, Release};
use crate::Error;

const MAX_ZIP_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXE_BYTES: u64 = 32 * 1024 * 1024;
const OK_MARKER: &str = ".ok";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub tag: String,
    pub dir: PathBuf,
    pub exe_names: Vec<String>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

pub fn verify_sha256(bytes: &[u8], expected_hex: &str) -> bool {
    sha256_hex(bytes).eq_ignore_ascii_case(expected_hex.trim())
}

fn valid_tag(tag: &str) -> bool {
    !tag.is_empty()
        && !tag.starts_with('.')
        && tag.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// Un ejecutable en la raíz del zip: sin separadores de ruta ni unidad.
fn root_exe_name(entry: &str) -> Option<&str> {
    let plain = !entry.chars().any(|c| std::path::is_separator(c) || c == '/' || c == ':');
    let is_exe = entry.len() > 4 && entry[entry.len() - 4..].eq_ignore_ascii_case(".exe");
    (plain && is_exe && !entry.starts_with('.')).then_some(entry)
}

fn list_exes(dir: &Path) -> Result<Vec<String>, Error> {
    let mut names: Vec<String> = std::fs::read_dir(dir)?
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| root_exe_name(n).is_some())
        .collect();
    names.sort();
    Ok(names)
}

/// Verifica el zip y extrae sus ejecutables de la raíz a `cache/<tag>/`.
pub fn install_from_zip(zip_bytes: &[u8], expected_sha256: &str, tag: &str, cache: &Path) -> Result<Tool, Error> {
    if !valid_tag(tag) {
        return Err(Error::Other(format!("etiqueta de publicación no válida: {tag:?}")));
    }
    if !verify_sha256(zip_bytes, expected_sha256) {
        return Err(Error::ChecksumMismatch);
    }
    let dir = cache.join(tag);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    std::fs::create_dir_all(&dir)?;

    let bad_zip = |e: zip::result::ZipError| Error::Other(format!("zip no válido: {e}"));
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(bad_zip)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(bad_zip)?;
        if entry.is_dir() {
            continue;
        }
        let Ok(raw_name) = entry.name().map(|n| n.into_owned()) else { continue };
        let Some(name) = root_exe_name(&raw_name).map(str::to_string) else { continue };
        let mut data = Vec::new();
        (&mut entry).take(MAX_EXE_BYTES + 1).read_to_end(&mut data)?;
        if data.len() as u64 > MAX_EXE_BYTES {
            return Err(Error::Other(format!("{name} es demasiado grande")));
        }
        std::fs::write(dir.join(&name), data)?;
    }
    let exe_names = list_exes(&dir)?;
    // El manifiesto se escribe al final: huella del zip y huella de cada ejecutable.
    let mut manifest = format!("{}\n", expected_sha256.trim().to_ascii_lowercase());
    for name in &exe_names {
        manifest.push_str(&format!("{}  {name}\n", sha256_hex(&std::fs::read(dir.join(name))?)));
    }
    std::fs::write(dir.join(OK_MARKER), manifest)?;
    Ok(Tool { tag: tag.to_string(), dir, exe_names })
}

/// Los ejecutables de una caché que sigue siendo la que se verificó: su manifiesto es de
/// esta misma descarga y cada ejecutable conserva su huella. `None` si algo no cuadra.
fn trusted_cache(dir: &Path, expected_sha256: &str) -> Option<Vec<String>> {
    let manifest = std::fs::read_to_string(dir.join(OK_MARKER)).ok()?;
    let mut lines = manifest.lines();
    if !lines.next()?.trim().eq_ignore_ascii_case(expected_sha256.trim()) {
        return None;
    }
    let mut names = Vec::new();
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let (digest, name) = line.split_once("  ")?;
        let name = root_exe_name(name)?;
        if !verify_sha256(&std::fs::read(dir.join(name)).ok()?, digest) {
            return None;
        }
        names.push(name.to_string());
    }
    // Un manifiesto sin ejecutables es de una versión anterior, o no sirve: se reinstala.
    if names.is_empty() {
        return None;
    }
    Some(names)
}

/// Devuelve la herramienta de esa publicación, descargándola si no está en caché.
pub fn ensure_tool(release: &Release, cache: &Path) -> Result<Tool, Error> {
    let asset = release.tools_zip().ok_or(Error::NoRelease)?;
    let expected = asset.sha256.as_deref().ok_or(Error::NoDigest)?;
    if !valid_tag(&release.tag) {
        return Err(Error::Other(format!("etiqueta de publicación no válida: {:?}", release.tag)));
    }
    let dir = cache.join(&release.tag);
    if let Some(exe_names) = trusted_cache(&dir, expected) {
        return Ok(Tool { tag: release.tag.clone(), exe_names, dir });
    }
    let bytes = download(&asset.url)?;
    install_from_zip(&bytes, expected, &release.tag, cache)
}

fn download(url: &str) -> Result<Vec<u8>, Error> {
    let net = |e: ureq::Error| Error::Network(e.to_string());
    let mut resp = releases::agent().get(url).call().map_err(net)?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(Error::Network(format!("la descarga respondió {status}")));
    }
    resp.body_mut().with_config().limit(MAX_ZIP_BYTES).read_to_vec().map_err(net)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::releases::{Asset, Release};
    use std::io::Write;

    fn sample_zip() -> Vec<u8> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let opts = zip::write::SimpleFileOptions::default();
        for (name, body) in [
            ("a.exe", "MZ-a"),
            ("b.txt", "text"),
            ("sub/c.exe", "MZ-c"),
            ("..\\evil.exe", "MZ-evil"),
            ("../evil2.exe", "MZ-evil2"),
        ] {
            w.start_file(name, opts).unwrap();
            w.write_all(body.as_bytes()).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn sha(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }

    fn exes_under(dir: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "exe") {
                    found.push(p);
                }
            }
        }
        found
    }

    #[test]
    fn known_sha256() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(verify_sha256(b"abc", "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"));
        assert!(!verify_sha256(b"abd", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
    }

    #[test]
    fn installs_only_root_executables() {
        let cache = tempfile::tempdir().unwrap();
        let zip = sample_zip();
        let tool = install_from_zip(&zip, &sha(&zip), "v1", cache.path()).unwrap();
        assert_eq!(tool.tag, "v1");
        assert_eq!(tool.exe_names, vec!["a.exe".to_string()]);
        assert_eq!(tool.dir, cache.path().join("v1"));
        assert!(tool.dir.join(".ok").is_file());
        assert_eq!(std::fs::read(tool.dir.join("a.exe")).unwrap(), b"MZ-a");
        assert_eq!(exes_under(cache.path()).len(), 1);
    }

    #[test]
    fn wrong_checksum_extracts_nothing() {
        let cache = tempfile::tempdir().unwrap();
        let zip = sample_zip();
        let bad = "0".repeat(64);
        assert!(matches!(install_from_zip(&zip, &bad, "v1", cache.path()), Err(Error::ChecksumMismatch)));
        assert!(exes_under(cache.path()).is_empty());
    }

    #[test]
    fn hostile_tag_is_rejected() {
        let cache = tempfile::tempdir().unwrap();
        let zip = sample_zip();
        assert!(install_from_zip(&zip, &sha(&zip), "..\\..\\x", cache.path()).is_err());
        assert!(install_from_zip(&zip, &sha(&zip), "", cache.path()).is_err());
    }

    fn release(sha256: Option<&str>) -> Release {
        Release {
            tag: "v9".into(),
            published_at: String::new(),
            notes: String::new(),
            assets: vec![Asset {
                name: "DeDRM_tools.zip".into(),
                url: "http://127.0.0.1:9/never".into(),
                sha256: sha256.map(String::from),
            }],
        }
    }

    #[test]
    fn release_without_digest_is_refused() {
        let cache = tempfile::tempdir().unwrap();
        assert!(matches!(ensure_tool(&release(None), cache.path()), Err(Error::NoDigest)));
    }

    #[test]
    fn release_without_tools_zip_is_refused() {
        let cache = tempfile::tempdir().unwrap();
        let mut r = release(Some("00"));
        r.assets.clear();
        assert!(matches!(ensure_tool(&r, cache.path()), Err(Error::NoRelease)));
    }

    fn cached(cache: &Path) -> String {
        let zip = sample_zip();
        let digest = sha(&zip);
        install_from_zip(&zip, &digest, "v9", cache).unwrap();
        digest
    }

    #[test]
    fn cached_tool_is_reused_without_network() {
        let cache = tempfile::tempdir().unwrap();
        let digest = cached(cache.path());
        let tool = ensure_tool(&release(Some(&digest)), cache.path()).unwrap();
        assert_eq!(tool.exe_names, vec!["a.exe".to_string()]);
    }

    #[test]
    fn cache_made_for_another_digest_is_not_trusted() {
        let cache = tempfile::tempdir().unwrap();
        cached(cache.path());
        let other = "0".repeat(64);
        // La URL de prueba no responde: llegar a la red demuestra que no se usó la caché.
        assert!(matches!(ensure_tool(&release(Some(&other)), cache.path()), Err(Error::Network(_))));
    }

    #[test]
    fn tampered_cached_executable_is_not_trusted() {
        let cache = tempfile::tempdir().unwrap();
        let digest = cached(cache.path());
        std::fs::write(cache.path().join("v9").join("a.exe"), b"MZ-otro").unwrap();
        assert!(matches!(ensure_tool(&release(Some(&digest)), cache.path()), Err(Error::Network(_))));
    }

    #[test]
    fn executables_dropped_into_the_cache_are_ignored() {
        let cache = tempfile::tempdir().unwrap();
        let digest = cached(cache.path());
        std::fs::write(cache.path().join("v9").join("MSIXKFXArchiver_x64_1_9999.exe"), b"MZ").unwrap();
        let tool = ensure_tool(&release(Some(&digest)), cache.path()).unwrap();
        assert_eq!(tool.exe_names, vec!["a.exe".to_string()]);
    }

    #[test]
    fn marker_from_an_older_version_is_reinstalled() {
        let cache = tempfile::tempdir().unwrap();
        let dir = cache.path().join("v9");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("x.exe"), b"MZ").unwrap();
        // Las versiones anteriores solo guardaban la huella del zip, sin la de cada ejecutable.
        std::fs::write(dir.join(".ok"), b"00").unwrap();
        assert!(matches!(ensure_tool(&release(Some("00")), cache.path()), Err(Error::Network(_))));
    }

    #[test]
    fn marker_without_a_manifest_is_not_trusted() {
        let cache = tempfile::tempdir().unwrap();
        let dir = cache.path().join("v9");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("x.exe"), b"MZ").unwrap();
        std::fs::write(dir.join(".ok"), b"").unwrap();
        assert!(matches!(ensure_tool(&release(Some("00")), cache.path()), Err(Error::Network(_))));
    }
}
