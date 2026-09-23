//! The sources a package may be fetched from, and what one fetch would
//! bring ([`Listing`]).
//!
//! **CTAN** (`[packages] source = "ctan"`, the default): two endpoints.
//!
//! 1. `GET https://ctan.org/json/2.0/pkg/<name>` — the CTAN JSON API 2.0
//!    record: `version.number` is the package's current version string and
//!    `ctan.path` its directory in the archive (`/macros/latex/contrib/<name>`).
//!    A 404 means CTAN has no package of that name.
//! 2. `GET https://mirrors.ctan.org<ctan.path>/` — the directory listing on
//!    the mirror `mirrors.ctan.org` redirects to (the mirrors serve the
//!    archive root directly; a `tex-archive/` prefix 404s on them). Every
//!    `href` whose name is a package file ([`crate::is_package_file`]) is
//!    fetched from `https://mirrors.ctan.org<ctan.path>/<file>` after
//!    consent. Listings are HTML whose exact shape varies by mirror; the
//!    parser reads only `href="…"` attributes and ignores anything with a
//!    path separator, a query or a fragment.
//!
//! A directory with a `.ins` batch file also has every `.ins` and `.dtx`
//! fetched: after the download the resolver runs `flashtex-docstrip` over
//! them (no TeX is executed; the interpreter copies bytes the sources
//! contain) and the generated `.sty`/`.cls`/`.def`/`.clo`/`.cfg` join the
//! shipped ones in the cache. A directory with `.dtx` sources but no
//! `.ins` (a self-installing `.dtx` or a `filecontents` package) is
//! reported as *needs docstrip* with the reason.
//!
//! **A registry URL** (`source = "https://…"`) is an archive root in the
//! same layout: the package lives at `<url>/macros/latex/contrib/<name>/`
//! and is found through that directory's listing. Such a registry states no
//! version, so the fetched content is the version
//! ([`crate::cache::content_version`]).
//!
//! CTAN keeps only the current version of a package, so a `pin` that names
//! another version cannot be satisfied from CTAN; the answer names both
//! versions rather than fetching the wrong one.

use serde_json::Value;

use flashtex_docstrip::{is_batch_file, is_source_file};

use crate::{is_package_file, FetchError, Fetcher, PackageSource};

/// The CTAN JSON API 2.0 package record endpoint.
pub const CTAN_API: &str = "https://ctan.org/json/2.0/pkg/";
/// The CTAN mirror redirector; the archive root is served directly under it.
pub const CTAN_MIRROR: &str = "https://mirrors.ctan.org";
/// Where LaTeX packages live in the archive layout.
pub const CONTRIB: &str = "/macros/latex/contrib/";

/// What a fetch of one package would bring: the version the source states
/// (CTAN) or `None` (a registry), the directory URL and the package files
/// in it, sorted by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub name: String,
    pub version: Option<String>,
    /// Ends with `/`; files are `base_url + name`.
    pub base_url: String,
    pub files: Vec<String>,
    /// A human label for the source (`CTAN`, or the registry URL).
    pub source_label: String,
}

impl Listing {
    /// Why `pin` cannot be satisfied by this listing, when it cannot: the
    /// source states a different version. A registry (no stated version)
    /// is checked after the download instead.
    pub fn pin_conflict(&self, pin: Option<&str>) -> Option<String> {
        let (pin, version) = (pin?, self.version.as_deref()?);
        if pin == version {
            return None;
        }
        Some(format!(
            "{} is pinned to {pin} but {} has {version}, and CTAN keeps only the current version; change or remove the pin",
            self.name, self.source_label
        ))
    }
}

/// Describes `name` at `source` without downloading any package file.
pub fn describe(fetcher: &dyn Fetcher, source: &PackageSource, name: &str) -> Result<Listing, String> {
    match source {
        PackageSource::None => Err(format!("{name}: `[packages] source` is \"none\"")),
        PackageSource::Ctan => describe_ctan(fetcher, name),
        PackageSource::Url(base) => describe_registry(fetcher, base, name),
    }
}

fn describe_ctan(fetcher: &dyn Fetcher, name: &str) -> Result<Listing, String> {
    let api = format!("{CTAN_API}{name}");
    let record = match fetcher.get(&api) {
        Ok(b) => b,
        Err(FetchError::Status(404)) => return Err(format!("CTAN has no package named {name} ({api}: not found (HTTP 404))")),
        Err(e) => return Err(format!("cannot reach CTAN ({api}): {e}")),
    };
    let v: Value = serde_json::from_slice(&record).map_err(|e| format!("{api} did not answer with JSON: {e}"))?;
    let version = v
        .get("version")
        .and_then(|v| v.get("number"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{api} states no version for {name}"))?
        .to_string();
    let path = v.get("ctan").and_then(|c| c.get("path")).and_then(Value::as_str).ok_or_else(|| format!("{api} states no archive path for {name} (a package without files on CTAN)"))?;
    if !path.starts_with('/') || path.contains("..") || path.contains("://") {
        return Err(format!("{api} states an archive path FlashTeX refuses: {path:?}"));
    }
    let base_url = format!("{CTAN_MIRROR}{}/", path.trim_end_matches('/'));
    let files = list_directory(fetcher, &base_url, name)?;
    Ok(Listing { name: name.into(), version: Some(version), base_url, files, source_label: "CTAN".into() })
}

fn describe_registry(fetcher: &dyn Fetcher, base: &str, name: &str) -> Result<Listing, String> {
    let base_url = format!("{}{CONTRIB}{name}/", base.trim_end_matches('/'));
    let files = list_directory(fetcher, &base_url, name)?;
    Ok(Listing { name: name.into(), version: None, base_url, files, source_label: base.to_string() })
}

/// The files a directory listing offers: the package files, plus every
/// `.ins` and `.dtx` when there is a batch file to run them with. A
/// directory with sources but no batch file, or with nothing usable, is
/// an error with the reason.
fn list_directory(fetcher: &dyn Fetcher, base_url: &str, name: &str) -> Result<Vec<String>, String> {
    let html = match fetcher.get(base_url) {
        Ok(b) => b,
        Err(FetchError::Status(404)) => return Err(format!("{base_url}: not found (HTTP 404); the archive has no directory for {name}")),
        Err(e) => return Err(format!("cannot list {base_url}: {e}")),
    };
    let names = hrefs(&String::from_utf8_lossy(&html));
    let has_batch = names.iter().any(|n| is_batch_file(n));
    let mut files: Vec<String> = names.iter().filter(|n| is_package_file(n) || (has_batch && is_docstrip_input(n))).cloned().collect();
    files.sort();
    files.dedup();
    if files.is_empty() {
        let sources: Vec<&String> = names.iter().filter(|n| is_source_file(n)).collect();
        if !sources.is_empty() {
            return Err(format!(
                "needs docstrip but ships no .ins batch file: {name} has only {} at {base_url} (a `.dtx` that installs itself is TeX to execute, which FlashTeX does not do; a `.sty` produced by running it can be placed in the project or a texinputs directory)",
                sources.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }
        return Err(format!("{base_url} lists no .sty/.cls/.def/.clo/.cfg file for {name}"));
    }
    Ok(files)
}

/// A file docstrip reads: a `.ins` batch file or a `.dtx` source.
pub fn is_docstrip_input(name: &str) -> bool {
    is_batch_file(name) || is_source_file(name)
}

/// The `href="…"` values of a listing that name a file: the last path
/// segment of the link (mirrors write `cancel.sty`, `./cancel.sty` or the
/// absolute `/pub/CTAN/…/cancel.sty`), skipping directory links (trailing
/// `/`), sort links (`?…`), anchors (`#…`), other hosts (`:`), `.`/`..`,
/// percent-decoded once.
pub fn hrefs(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut at = 0;
    while let Some(i) = lower[at..].find("href=") {
        at += i + 5;
        let quote = match html[at..].chars().next() {
            Some(q @ ('"' | '\'')) => q,
            _ => continue,
        };
        at += 1;
        let Some(len) = html[at..].find(quote) else { break };
        let raw = &html[at..at + len];
        at += len + 1;
        let value = percent_decode(raw);
        if value.is_empty() || value.ends_with('/') || value.contains(['\\', '?', '#', ':']) || value.chars().any(char::is_control) {
            continue;
        }
        let name = value.rsplit('/').next().unwrap_or("").to_string();
        if name.is_empty() || name == "." || name == ".." {
            continue;
        }
        out.push(name);
    }
    out
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeFetcher;

    #[test]
    fn listing_parser_keeps_only_plain_file_names() {
        let html = r##"<a href="?C=N;O=D">Name</a> <a href="../">Parent</a> <a href="/pub/CTAN/x/">up</a>
            <a href='cancel.sty'>cancel.sty</a> <a href="cancel.pdf">doc</a> <a href="my%20file.sty">x</a> <a href="#top">t</a>"##;
        assert_eq!(hrefs(html), ["cancel.sty", "cancel.pdf", "my file.sty"]);
        assert_eq!(hrefs(r#"<A HREF="./a.sty">a</A> <a href="/pub/CTAN/macros/x/b.cls">b</a> <a href="/pub/CTAN/macros/x/">dir</a> <a href="https://other/c.sty">c</a>"#), ["a.sty", "b.cls"]);
        assert_eq!(percent_decode("a%2Fb"), "a/b");
        assert_eq!(percent_decode("100%"), "100%");
    }

    #[test]
    fn ctan_record_and_listing_become_a_listing() {
        let f = FakeFetcher::new().with_ctan_package("cancel", "2.2", &[("cancel.sty", "s"), ("cancel.tex", "t"), ("cancel.pdf", "p")]);
        let l = describe(&f, &PackageSource::Ctan, "cancel").unwrap();
        assert_eq!(l.version.as_deref(), Some("2.2"));
        assert_eq!(l.base_url, "https://mirrors.ctan.org/macros/latex/contrib/cancel/");
        assert_eq!(l.files, ["cancel.sty"]);
        assert_eq!(l.pin_conflict(None), None);
        assert_eq!(l.pin_conflict(Some("2.2")), None);
        assert!(l.pin_conflict(Some("2.0")).unwrap().contains("CTAN has 2.2"));
        assert!(describe(&f, &PackageSource::None, "cancel").unwrap_err().contains("\"none\""));
        assert!(describe(&f, &PackageSource::Ctan, "zzz").unwrap_err().contains("no package named zzz"));
    }

    #[test]
    fn a_record_without_files_or_with_a_bad_path_is_refused() {
        let f = FakeFetcher::new()
            .with("https://ctan.org/json/2.0/pkg/meta", r#"{"version":{"number":"1"}}"#)
            .with("https://ctan.org/json/2.0/pkg/evil", r#"{"version":{"number":"1"},"ctan":{"path":"/../etc"}}"#)
            .with("https://ctan.org/json/2.0/pkg/broken", "not json")
            .with("https://ctan.org/json/2.0/pkg/empty", r#"{"version":{"number":"1"},"ctan":{"path":"/macros/latex/contrib/empty"}}"#)
            .with("https://mirrors.ctan.org/macros/latex/contrib/empty/", "<a href=\"README\">r</a>");
        assert!(describe(&f, &PackageSource::Ctan, "meta").unwrap_err().contains("no archive path"));
        assert!(describe(&f, &PackageSource::Ctan, "evil").unwrap_err().contains("refuses"));
        assert!(describe(&f, &PackageSource::Ctan, "broken").unwrap_err().contains("JSON"));
        assert!(describe(&f, &PackageSource::Ctan, "empty").unwrap_err().contains("lists no .sty"));
        let f = FakeFetcher::new()
            .with_ctan_package("lipsum", "2.7", &[("lipsum.dtx", "%"), ("lipsum.ins", "%"), ("lipsum.pdf", "%"), ("README.md", "#")])
            .with_ctan_package("selfins", "1", &[("selfins.dtx", "%"), ("selfins.pdf", "%")])
            .with_ctan_package("both", "1", &[("both.sty", "%"), ("both.dtx", "%"), ("both.ins", "%"), ("extra.cfg", "%")]);
        assert_eq!(describe(&f, &PackageSource::Ctan, "lipsum").unwrap().files, ["lipsum.dtx", "lipsum.ins"], "a batch file brings every source with it");
        assert!(describe(&f, &PackageSource::Ctan, "selfins").unwrap_err().starts_with("needs docstrip but ships no .ins"));
        assert_eq!(describe(&f, &PackageSource::Ctan, "both").unwrap().files, ["both.dtx", "both.ins", "both.sty", "extra.cfg"]);
        let transport = FakeFetcher::new();
        struct Down;
        impl Fetcher for Down {
            fn get(&self, _: &str) -> Result<Vec<u8>, FetchError> {
                Err(FetchError::Transport("dns: no such host".into()))
            }
        }
        let _ = transport;
        assert!(describe(&Down, &PackageSource::Ctan, "cancel").unwrap_err().contains("cannot reach CTAN"));
    }

    #[test]
    fn a_registry_lists_the_contrib_directory_and_states_no_version() {
        let f = FakeFetcher::new().with("https://r.example/archive/macros/latex/contrib/p/", "<a href=\"p.sty\">p</a><a href=\"p.dtx\">d</a>");
        let l = describe(&f, &PackageSource::Url("https://r.example/archive/".into()), "p").unwrap();
        assert_eq!(l.version, None);
        assert_eq!(l.base_url, "https://r.example/archive/macros/latex/contrib/p/");
        assert_eq!(l.files, ["p.sty"], "a .dtx without a .ins is documentation source only");
        assert_eq!(l.pin_conflict(Some("anything")), None, "checked after the download instead");
        assert!(describe(&f, &PackageSource::Url("https://r.example/archive".into()), "q").unwrap_err().contains("404"));
    }
}
