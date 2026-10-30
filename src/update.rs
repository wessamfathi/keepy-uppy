//! Manual update check against GitHub Releases. No auto-install, no background
//! checks: the network is only touched when the user clicks the menu item.

use std::process::Command;

const RELEASES: &str = "https://github.com/wessamfathi/keepy-uppy/releases";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    /// Tag of the newer release, e.g. `v0.2.0`.
    Available(String),
    Failed,
}

/// Returns the newer release's tag, or `None` when up to date.
/// Blocking; run it off the main thread.
pub fn check() -> Result<Option<String>, String> {
    // `/releases/latest` redirects to `/releases/tag/<tag>`, which avoids the
    // JSON API and its rate limit.
    let out = Command::new("/usr/bin/curl")
        .args(["-sS", "--max-time", "15", "-o", "/dev/null"])
        .args(["-w", "%{http_code} %{redirect_url}"])
        .arg(format!("{RELEASES}/latest"))
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (code, url) = text.split_once(' ').unwrap_or((&text, ""));
    if code != "302" {
        return Err(format!("update check: HTTP {code}"));
    }
    let tag = latest_tag(url).ok_or("update check: no release tag")?;
    Ok(is_newer(tag, env!("CARGO_PKG_VERSION")).then(|| tag.to_string()))
}

pub fn release_page(tag: &str) -> String {
    format!("{RELEASES}/tag/{tag}")
}

fn latest_tag(redirect: &str) -> Option<&str> {
    redirect
        .split_once("/releases/tag/")
        .map(|(_, tag)| tag)
        .filter(|t| !t.is_empty())
}

fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.strip_prefix('v').unwrap_or(version).split('.');
    let mut next = || parts.next()?.parse().ok();
    let v = (next()?, next()?, next()?);
    parts.next().is_none().then_some(v)
}

fn is_newer(tag: &str, current: &str) -> bool {
    matches!((parse(tag), parse(current)), (Some(t), Some(c)) if t > c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_from_redirect() {
        let url = "https://github.com/wessamfathi/keepy-uppy/releases/tag/v1.2.3";
        assert_eq!(latest_tag(url), Some("v1.2.3"));
        assert_eq!(latest_tag("https://github.com/x/y/releases"), None);
        assert_eq!(latest_tag(""), None);
    }

    #[test]
    fn version_compare() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0.0", "0.99.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.0.9", "0.1.0"));
    }

    #[test]
    fn malformed_tags_are_not_newer() {
        assert!(!is_newer("v1.0", "0.1.0"));
        assert!(!is_newer("v1.0.0-beta", "0.1.0"));
        assert!(!is_newer("v1.0.0.1", "0.1.0"));
        assert!(!is_newer("nightly", "0.1.0"));
    }
}
