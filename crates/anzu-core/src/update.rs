//! New versions of Anzu itself, published as GitHub releases of this repository.
//!
//! There is no server of our own: the app asks GitHub for the latest release, which is
//! one small request. A release is an update only when it is newer than the running
//! version and carries the installer with its SHA-256 digest. The installer is
//! downloaded next to the app data and verified before anything runs it.

use std::fmt;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{Error, releases};

/// The latest published release of Anzu. Drafts and pre-releases are never returned here.
pub const LATEST_URL: &str = "https://api.github.com/repos/ilien-dev/anzu/releases/latest";
/// The asset of a release that installs it: `anzu-<version>-windows-x64-setup.exe`.
pub const SETUP_SUFFIX: &str = "-windows-x64-setup.exe";
/// The arguments that make the downloaded installer update the installation in `dir`
/// without asking anything, once the running app has closed, and open it again.
pub fn installer_args(dir: &Path) -> Vec<std::ffi::OsString> {
    vec!["--update".into(), "--dir".into(), dir.as_os_str().to_owned()]
}
/// An installer larger than this is not what we publish.
const MAX_SETUP_BYTES: u64 = 256 * 1024 * 1024;

/// A version of Anzu: `major.minor.patch`, as Cocogitto tags it (`v0.3.0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AppVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl AppVersion {
    /// The version of this build.
    pub fn current() -> AppVersion {
        env!("CARGO_PKG_VERSION").parse().expect("the package version is major.minor.patch")
    }
}

impl FromStr for AppVersion {
    type Err = Error;

    /// Takes `1.2.3` or `v1.2.3`. A pre-release (`1.2.3-rc.1`) or anything else is refused.
    fn from_str(s: &str) -> Result<Self, Error> {
        let bad = || Error::BadVersion(s.to_string());
        let text = s.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let mut parts = text.split('.');
        let mut next = || -> Result<u64, Error> {
            let part = parts.next().ok_or_else(bad)?;
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad());
            }
            part.parse().map_err(|_| bad())
        };
        let version = AppVersion { major: next()?, minor: next()?, patch: next()? };
        if parts.next().is_some() {
            return Err(bad());
        }
        Ok(version)
    }
}

impl fmt::Display for AppVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A newer release that can be installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub version: AppVersion,
    /// File name of the installer.
    pub name: String,
    pub url: String,
    /// SHA-256 of the installer, lowercase hexadecimal.
    pub sha256: String,
    pub size: u64,
    /// The release page, with its notes.
    pub page: String,
}

#[derive(Deserialize)]
struct RawAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

/// A file name that is safe to write in our folder as is.
fn plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// What the latest release offers to a build of `current`: `None` when it is not newer,
/// or brings no installer.
pub fn offer_in(json: &str, current: AppVersion) -> Result<Option<Offer>, Error> {
    let release: RawRelease = serde_json::from_str(json).map_err(|e| Error::BadResponse(e.to_string()))?;
    if release.draft || release.prerelease {
        return Ok(None);
    }
    let Ok(version) = release.tag_name.parse::<AppVersion>() else { return Ok(None) };
    if version <= current {
        return Ok(None);
    }
    let Some(asset) = release.assets.into_iter().find(|a| a.name.ends_with(SETUP_SUFFIX)) else {
        return Ok(None);
    };
    if !plain_file_name(&asset.name) {
        return Err(Error::BadResponse(format!("unexpected installer name: {:?}", asset.name)));
    }
    let sha256 = asset
        .digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .map(str::to_ascii_lowercase)
        .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or(Error::NoDigest)?;
    Ok(Some(Offer {
        version,
        name: asset.name,
        url: asset.browser_download_url,
        sha256,
        size: asset.size,
        page: release.html_url,
    }))
}

/// Asks GitHub whether there is a newer release than this build.
#[cfg(target_arch = "wasm32")]
pub fn check() -> Result<Option<Offer>, Error> {
    Err(releases::offline())
}

/// Asks GitHub whether there is a newer release than this build.
#[cfg(not(target_arch = "wasm32"))]
pub fn check() -> Result<Option<Offer>, Error> {
    let net = |e: ureq::Error| Error::Network(e.to_string());
    let mut resp = releases::agent()
        .get(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(net)?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().with_config().limit(1024 * 1024).read_to_string().map_err(net)?;
    match status {
        200 => offer_in(&body, AppVersion::current()),
        // No release published yet.
        404 => Ok(None),
        403 | 429 => Err(Error::RateLimited),
        _ => Err(Error::Network(format!("GitHub responded with {status}"))),
    }
}

/// Where downloaded installers are kept, inside the app data folder.
pub fn updates_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("updates")
}

/// Copies `reader` to `dir/<offer.name>`, reporting the bytes written so far, and keeps
/// the file only if its SHA-256 is the published one.
// On the web build a file holds nothing to close, but the drop is what closes it on Windows.
#[cfg_attr(target_arch = "wasm32", allow(clippy::drop_non_drop))]
pub fn save_verified(
    offer: &Offer,
    reader: &mut dyn Read,
    dir: &Path,
    progress: &mut dyn FnMut(u64),
) -> Result<PathBuf, Error> {
    if !plain_file_name(&offer.name) {
        return Err(Error::BadResponse(format!("unexpected installer name: {:?}", offer.name)));
    }
    std::fs::create_dir_all(dir)?;
    let target = dir.join(&offer.name);
    let part = dir.join(format!("{}.part", offer.name));
    let result = (|| {
        let mut file = std::fs::File::create(&part)?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];
        let mut written = 0u64;
        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            written += read as u64;
            if written > MAX_SETUP_BYTES {
                return Err(Error::InstallerTooLarge);
            }
            hasher.update(&buffer[..read]);
            file.write_all(&buffer[..read])?;
            progress(written);
        }
        file.sync_all()?;
        drop(file);
        let digest: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if digest != offer.sha256 {
            return Err(Error::ChecksumMismatch);
        }
        std::fs::rename(&part, &target)?;
        Ok(target.clone())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    result
}

/// Downloads the installer of `offer` into `dir` and verifies it.
#[cfg(target_arch = "wasm32")]
pub fn download(_offer: &Offer, _dir: &Path, _progress: &mut dyn FnMut(u64)) -> Result<PathBuf, Error> {
    Err(releases::offline())
}

/// Downloads the installer of `offer` into `dir` and verifies it.
#[cfg(not(target_arch = "wasm32"))]
pub fn download(offer: &Offer, dir: &Path, progress: &mut dyn FnMut(u64)) -> Result<PathBuf, Error> {
    let net = |e: ureq::Error| Error::Network(e.to_string());
    let mut resp = releases::agent().get(&offer.url).call().map_err(net)?;
    let status = resp.status().as_u16();
    if status != 200 {
        return Err(Error::Network(format!("the download responded with {status}")));
    }
    let mut reader = resp.body_mut().with_config().limit(MAX_SETUP_BYTES).reader();
    save_verified(offer, &mut reader, dir, progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> AppVersion {
        text.parse().unwrap()
    }

    const DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn release(tag: &str, assets: &str) -> String {
        format!(
            r#"{{"tag_name":"{tag}","html_url":"https://github.com/ilien-dev/anzu/releases/tag/{tag}","draft":false,"prerelease":false,"assets":[{assets}]}}"#
        )
    }

    fn setup_asset(version: &str, digest: &str) -> String {
        format!(
            r#"{{"name":"anzu-{version}-windows-x64-setup.exe","browser_download_url":"https://example.invalid/anzu-{version}-windows-x64-setup.exe","size":3,"digest":"{digest}"}}"#
        )
    }

    #[test]
    fn versions_parse_and_order_numerically() {
        assert_eq!(v("v0.10.2"), AppVersion { major: 0, minor: 10, patch: 2 });
        assert!(v("0.10.0") > v("0.9.9"));
        assert!(v("1.0.0") > v("0.99.99"));
        assert_eq!(v(" 1.2.3 ").to_string(), "1.2.3");
        for bad in ["", "1.2", "1.2.3.4", "1.2.3-rc.1", "v", "1..3", "a.b.c", "+1.2.3"] {
            assert!(bad.parse::<AppVersion>().is_err(), "{bad}");
        }
        AppVersion::current();
    }

    #[test]
    fn a_newer_release_with_an_installer_is_offered() {
        let json = release("v0.4.0", &setup_asset("0.4.0", &format!("sha256:{}", DIGEST.to_uppercase())));
        let offer = offer_in(&json, v("0.3.0")).unwrap().unwrap();
        assert_eq!(offer.version, v("0.4.0"));
        assert_eq!(offer.name, "anzu-0.4.0-windows-x64-setup.exe");
        assert_eq!(offer.sha256, DIGEST);
        assert!(offer.page.ends_with("/v0.4.0"));
    }

    #[test]
    fn the_same_or_an_older_release_is_not_an_update() {
        let json = release("v0.3.0", &setup_asset("0.3.0", &format!("sha256:{DIGEST}")));
        assert_eq!(offer_in(&json, v("0.3.0")).unwrap(), None);
        assert_eq!(offer_in(&json, v("0.4.1")).unwrap(), None);
    }

    #[test]
    fn a_release_without_an_installer_is_not_an_update() {
        let zip = r#"{"name":"anzu-0.4.0-windows-x64.zip","browser_download_url":"https://example.invalid/a.zip","size":3}"#;
        assert_eq!(offer_in(&release("v0.4.0", zip), v("0.3.0")).unwrap(), None);
    }

    #[test]
    fn drafts_pre_releases_and_odd_tags_are_ignored() {
        let asset = setup_asset("0.4.0", &format!("sha256:{DIGEST}"));
        let pre = release("v0.4.0", &asset).replace(r#""prerelease":false"#, r#""prerelease":true"#);
        assert_eq!(offer_in(&pre, v("0.3.0")).unwrap(), None);
        let draft = release("v0.4.0", &asset).replace(r#""draft":false"#, r#""draft":true"#);
        assert_eq!(offer_in(&draft, v("0.3.0")).unwrap(), None);
        assert_eq!(offer_in(&release("nightly", &asset), v("0.3.0")).unwrap(), None);
    }

    #[test]
    fn an_installer_without_a_digest_is_refused() {
        for digest in ["", "md5:abc", "sha256:xyz"] {
            let json = release("v0.4.0", &setup_asset("0.4.0", digest));
            assert!(matches!(offer_in(&json, v("0.3.0")), Err(Error::NoDigest)), "{digest}");
        }
    }

    #[test]
    fn an_installer_name_with_a_path_is_refused() {
        let asset = r#"{"name":"..\\evil-windows-x64-setup.exe","browser_download_url":"https://example.invalid/x","size":3,"digest":"sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}"#;
        assert!(matches!(offer_in(&release("v0.4.0", asset), v("0.3.0")), Err(Error::BadResponse(_))));
    }

    #[test]
    fn garbage_is_a_bad_response() {
        assert!(matches!(offer_in("{not json", v("0.3.0")), Err(Error::BadResponse(_))));
    }

    fn offer(sha256: &str) -> Offer {
        Offer {
            version: v("0.4.0"),
            name: "anzu-0.4.0-windows-x64-setup.exe".into(),
            url: String::new(),
            sha256: sha256.into(),
            size: 3,
            page: String::new(),
        }
    }

    #[test]
    fn a_verified_download_is_kept_and_reports_progress() {
        let dir = tempfile::tempdir().unwrap();
        let mut seen = Vec::new();
        let path = save_verified(&offer(DIGEST), &mut &b"abc"[..], dir.path(), &mut |n| seen.push(n)).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"abc");
        assert_eq!(seen.last(), Some(&3));
        assert!(!dir.path().join("anzu-0.4.0-windows-x64-setup.exe.part").exists());
    }

    #[test]
    fn a_download_that_does_not_match_is_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let result = save_verified(&offer(DIGEST), &mut &b"abd"[..], dir.path(), &mut |_| {});
        assert!(matches!(result, Err(Error::ChecksumMismatch)));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
