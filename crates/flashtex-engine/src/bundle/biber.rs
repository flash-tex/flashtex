//! biber with no TeX Live, fetched on demand (owner direction, #1782).
//!
//! biblatex's default backend is biber, a Perl program TeX Live ships packed
//! into one executable per platform. FlashTeX does not port it: the hosts
//! run TeX Live's own biber (`host::external`), and with no TeX Live this
//! module supplies one. `bundle-publish.yml` mirrors TeX Live's biber for
//! each platform, unmodified (the executable, gzipped), with its source and
//! licence (Artistic 2.0), as release assets; `flashtex-bundle.lock` pins
//! each one (`biber_url_<arch>`, `biber_sha256_<arch>`, `biber_size_<arch>`,
//! `<arch>` TeX Live's name for the platform with `_` for `-`, as lock keys
//! are words: `biber_url_universal_darwin`). Nothing is fetched before the
//! user agrees: the app passes `FLASHTEX_BIBER_ALLOW_FETCH=<sha256>@<url>`
//! (as `FLASHTEX_BUNDLE_ALLOW_FETCH` for the bundle). The first time a
//! document needs biber (the run wrote a `.bcf`), [`ensure`] fetches the
//! asset, checks its SHA-256 against the pin, and keeps the executable in
//! the bundle cache under that digest; the host then runs it as it runs
//! TeX Live's biber: a child process, external tools on, a trusted project.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The consent variable: `<sha256>@<url>` of the asset the user agreed to.
pub const ALLOW_ENV: &str = "FLASHTEX_BIBER_ALLOW_FETCH";

/// What the app asks before the download (the host's skip reason).
pub const CONSENT_TEXT: &str = "FlashTeX will download biber (~70 MB) to build this bibliography";

/// TeX Live's name for this build's platform (`bin/<arch>`).
pub fn platform() -> Option<&'static str> {
    if cfg!(target_os = "macos") {
        Some("universal-darwin")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-linux")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("aarch64-linux")
    } else if cfg!(windows) {
        Some("windows")
    } else {
        None
    }
}

/// One pinned biber asset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BiberSpec {
    pub url: String,
    /// SHA-256 of the asset (the gzipped executable), lowercase hex.
    pub sha256: String,
    pub size: u64,
}

/// `<arch>`'s biber in a lock's text (`key = "value"` lines, `#` comments);
/// none when the lock pins none.
pub fn spec_from_lock(text: &str, arch: &str) -> Option<BiberSpec> {
    let mut url = None;
    let mut sha = None;
    let mut size = None;
    let key_arch = arch.replace('-', "_");
    for l in text.lines() {
        let l = l.trim();
        let Some((k, v)) = l.split_once('=') else {
            continue;
        };
        let v = v.trim();
        let Some(v) = v
            .strip_prefix('"')
            .and_then(|v| v.split_once('"'))
            .map(|x| x.0)
        else {
            continue;
        };
        match k
            .trim()
            .strip_suffix(key_arch.as_str())
            .and_then(|k| k.strip_suffix('_'))
        {
            Some("biber_url") => url = Some(v.to_string()),
            Some("biber_sha256") => sha = Some(v.to_ascii_lowercase()),
            Some("biber_size") => size = v.parse().ok(),
            _ => {}
        }
    }
    let sha = sha.filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))?;
    Some(BiberSpec {
        url: url?,
        sha256: sha,
        size: size?,
    })
}

/// The pinned biber of this platform: `FLASHTEX_BIBER_URL`,
/// `FLASHTEX_BIBER_SHA256` and `FLASHTEX_BIBER_SIZE` (a developer's), else
/// the first lock of [`super::lock_candidates`] that pins one.
pub fn configured() -> Option<BiberSpec> {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    if let (Some(url), Some(sha256), Some(size)) = (
        env("FLASHTEX_BIBER_URL"),
        env("FLASHTEX_BIBER_SHA256"),
        env("FLASHTEX_BIBER_SIZE").and_then(|s| s.parse().ok()),
    ) {
        return Some(BiberSpec {
            url,
            sha256: sha256.to_ascii_lowercase(),
            size,
        });
    }
    let arch = platform()?;
    super::lock_candidates()
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .find_map(|t| spec_from_lock(&t, arch))
}

/// Where the executable of `spec` is kept: `<bundle cache>/biber/<sha256>/`.
pub fn cached_path(cache: &Path, spec: &BiberSpec) -> PathBuf {
    let exe = if cfg!(windows) { "biber.exe" } else { "biber" };
    cache.join("biber").join(&spec.sha256).join(exe)
}

/// A biber to run: the cached one, else fetched now if the user agreed.
/// The error is the reason to show (the app asks for consent on
/// [`CONSENT_TEXT`]).
pub fn ensure() -> Result<PathBuf, String> {
    let Some(spec) = configured() else {
        return Err(
            "biber is not installed: no TeX Live, and no biber is pinned for this platform".into(),
        );
    };
    let cache = super::default_cache_dir().ok_or("no cache directory for biber")?;
    let allow = std::env::var(ALLOW_ENV).unwrap_or_default();
    ensure_with(&spec, &cache, &allow)
}

/// [`ensure`] with its inputs given.
pub fn ensure_with(spec: &BiberSpec, cache: &Path, allow: &str) -> Result<PathBuf, String> {
    let exe = cached_path(cache, spec);
    if exe.is_file() {
        return Ok(exe);
    }
    if !super::fetch_allowed(allow, &spec.sha256, &spec.url) {
        return Err(format!("biber is not installed: {CONSENT_TEXT}"));
    }
    let data = super::fetch::open(&spec.url)?.read_range(0, spec.size)?;
    let got = super::hex(&Sha256::digest(&data));
    if got != spec.sha256 {
        return Err(format!(
            "the biber download from {} has SHA-256 {got}, not the pinned {}",
            spec.url, spec.sha256
        ));
    }
    let program = super::gz::gunzip(&data, data.len() * 3)?;
    let dir = exe.parent().ok_or("biber's cache directory")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = dir.join(format!(".biber.{}", std::process::id()));
    std::fs::write(&tmp, &program).map_err(|e| format!("{}: {e}", tmp.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    }
    std::fs::rename(&tmp, &exe).map_err(|e| format!("{}: {e}", exe.display()))?;
    Ok(exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = r#"
url = "https://example.org/b.ttb"
digest = "0000000000000000000000000000000000000000000000000000000000000000"
biber_url_universal_darwin = "https://example.org/biber-universal-darwin.gz"
biber_sha256_universal_darwin = "ABababababababababababababababababababababababababababababababab"
biber_size_universal_darwin = "123"
biber_url_x86_64_linux = "https://example.org/biber-x86_64-linux.gz" # a comment
"#;

    #[test]
    fn the_lock_pins_a_biber_per_platform() {
        let s = spec_from_lock(LOCK, "universal-darwin").unwrap();
        assert_eq!(s.url, "https://example.org/biber-universal-darwin.gz");
        assert_eq!(s.sha256, "ab".repeat(32));
        assert_eq!(s.size, 123);
        // a platform without its digest and size is not pinned
        assert_eq!(spec_from_lock(LOCK, "x86_64-linux"), None);
        assert_eq!(spec_from_lock(LOCK, "windows"), None);
    }

    /// Nothing without consent; a download is checked against its pin and
    /// kept; a wrong digest is refused and nothing is kept.
    #[test]
    fn biber_is_fetched_only_with_consent_and_checked() {
        let dir = std::env::temp_dir().join(format!("ftx-biber-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let program = b"#!/bin/sh\necho biber\n".repeat(50);
        let gz = super::super::gz::gzip(&program, 9);
        let asset = dir.join("biber.gz");
        std::fs::write(&asset, &gz).unwrap();
        let url = format!("file://{}", asset.display());
        let spec = BiberSpec {
            url: url.clone(),
            sha256: super::super::hex(&Sha256::digest(&gz)),
            size: gz.len() as u64,
        };
        let cache = dir.join("cache");
        let e = ensure_with(&spec, &cache, "").unwrap_err();
        assert!(e.contains(CONSENT_TEXT), "{e}");
        assert!(!cached_path(&cache, &spec).exists());
        let other = format!("{}@https://elsewhere.example/biber.gz", spec.sha256);
        assert!(
            ensure_with(&spec, &cache, &other).is_err(),
            "consent is for a URL"
        );
        let bad = BiberSpec {
            sha256: "0".repeat(64),
            ..spec.clone()
        };
        let e = ensure_with(&bad, &cache, &format!("{}@{url}", bad.sha256)).unwrap_err();
        assert!(e.contains("not the pinned"), "{e}");
        assert!(!cached_path(&cache, &bad).exists());
        let exe = ensure_with(&spec, &cache, &format!("{}@{url}", spec.sha256)).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), program);
        // cached: no consent needed to run it again
        assert_eq!(ensure_with(&spec, &cache, "").unwrap(), exe);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
