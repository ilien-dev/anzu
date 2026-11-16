//! Releases of `Satsuoni/DeDRM_tools` on GitHub.
//!
//! Every release in the repository is marked as `prerelease`, so `/releases/latest`
//! returns nothing useful: the list is requested and the first one is used.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use serde::Deserialize;

use crate::Error;

pub const RELEASES_URL: &str = "https://api.github.com/repos/Satsuoni/DeDRM_tools/releases?per_page=5";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    /// SHA-256 digest in hexadecimal, as GitHub publishes it.
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub published_at: String,
    pub notes: String,
    pub assets: Vec<Asset>,
}

impl Release {
    /// The zip with the archivers: `DeDRM_tools.zip`, or variants such as `DeDRM_toolsv2.zip`.
    pub fn tools_zip(&self) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name.starts_with("DeDRM_tools") && a.name.ends_with(".zip"))
    }
}

#[derive(Deserialize)]
struct RawAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<RawAsset>,
}

#[derive(Deserialize)]
struct ApiMessage {
    message: String,
}

pub fn parse_releases(json: &str) -> Result<Vec<Release>, Error> {
    match serde_json::from_str::<Vec<RawRelease>>(json) {
        Ok(raw) => Ok(raw
            .into_iter()
            .map(|r| Release {
                tag: r.tag_name,
                published_at: r.published_at.unwrap_or_default(),
                notes: r.body.unwrap_or_default(),
                assets: r
                    .assets
                    .into_iter()
                    .map(|a| Asset {
                        name: a.name,
                        url: a.browser_download_url,
                        sha256: a
                            .digest
                            .as_deref()
                            .and_then(|d| d.strip_prefix("sha256:"))
                            .map(|h| h.to_ascii_lowercase()),
                    })
                    .collect(),
            })
            .collect()),
        Err(e) => match serde_json::from_str::<ApiMessage>(json) {
            Ok(m) if m.message.to_ascii_lowercase().contains("rate limit") => Err(Error::RateLimited),
            Ok(m) => Err(Error::BadResponse(m.message)),
            Err(_) => Err(Error::BadResponse(e.to_string())),
        },
    }
}

/// Queries GitHub and returns the most recent release.
pub fn fetch_latest() -> Result<Release, Error> {
    let body = http_get_string(RELEASES_URL)?;
    parse_releases(&body)?.into_iter().next().ok_or(Error::NoRelease)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .user_agent("anzu")
        .build()
        .into()
}

/// The web build has no network access of its own.
#[cfg(target_arch = "wasm32")]
pub(crate) fn offline() -> Error {
    Error::Network("the web build does not go online".into())
}

#[cfg(target_arch = "wasm32")]
fn http_get_string(_url: &str) -> Result<String, Error> {
    Err(offline())
}

#[cfg(not(target_arch = "wasm32"))]
fn http_get_string(url: &str) -> Result<String, Error> {
    let mut resp = agent()
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| Error::Network(e.to_string()))?;
    let status = resp.status().as_u16();
    let body = resp.body_mut().read_to_string().map_err(|e| Error::Network(e.to_string()))?;
    match status {
        200 => Ok(body),
        403 | 429 => Err(Error::RateLimited),
        _ => Err(Error::Network(format!("GitHub responded with {status}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../tests/fixtures/releases.json");

    #[test]
    fn first_release_is_the_latest() {
        let r = parse_releases(FIXTURE).unwrap();
        assert_eq!(r[0].tag, "v10.0.35");
        assert_eq!(r[0].published_at, "2026-11-08T04:14:11Z");
        assert!(r[0].notes.contains("1.5634"));
    }

    #[test]
    fn tools_zip_carries_url_and_digest() {
        let r = parse_releases(FIXTURE).unwrap();
        let zip = r[0].tools_zip().unwrap();
        assert_eq!(zip.name, "DeDRM_tools.zip");
        assert_eq!(
            zip.url,
            "https://github.com/Satsuoni/DeDRM_tools/releases/download/v10.0.35/DeDRM_tools.zip"
        );
        assert_eq!(
            zip.sha256.as_deref(),
            Some("5c85cce954b17836c2b421d6ae8b313f0aaeb190a9a3f81881636dbc80e25bc3")
        );
    }

    #[test]
    fn tools_zip_accepts_renamed_asset() {
        let r = parse_releases(FIXTURE).unwrap();
        let v31 = r.iter().find(|r| r.tag == "v10.0.31").unwrap();
        assert_eq!(v31.tools_zip().unwrap().name, "DeDRM_toolsv2.zip");
    }

    #[test]
    fn missing_or_foreign_digest_is_none() {
        let json = r#"[{"tag_name":"v1","published_at":"x","body":null,"assets":[
            {"name":"DeDRM_tools.zip","browser_download_url":"u","digest":null},
            {"name":"DeDRM_tools2.zip","browser_download_url":"u","digest":"md5:abc"},
            {"name":"DeDRM_tools3.zip","browser_download_url":"u"}]}]"#;
        let r = parse_releases(json).unwrap();
        assert!(r[0].assets.iter().all(|a| a.sha256.is_none()));
        assert_eq!(r[0].notes, "");
    }

    #[test]
    fn empty_list_parses() {
        assert!(parse_releases("[]").unwrap().is_empty());
    }

    #[test]
    fn rate_limit_body_is_reported_as_such() {
        let body = r#"{"message":"API rate limit exceeded for 1.2.3.4.","documentation_url":"x"}"#;
        assert!(matches!(parse_releases(body), Err(Error::RateLimited)));
    }

    #[test]
    fn other_garbage_is_a_bad_response() {
        assert!(matches!(parse_releases("<html>"), Err(Error::BadResponse(_))));
        assert!(matches!(parse_releases(r#"{"message":"Not Found"}"#), Err(Error::BadResponse(_))));
    }
}
