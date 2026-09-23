//! Package resolution for FlashTeX projects
//! (docs/proposals/packages-fonts-manifest.md §2.3 and §S3,
//! docs/user/project-manifest.md `[packages]`).
//!
//! Given a package name that neither the compiler models nor the project
//! (its own directory, the manifest's `texinputs`) supplies, this crate
//! answers *where its LaTeX source files are*, in this order:
//!
//! 1. a **local library** the manifest names in `[packages] path`
//!    (a directory whose own `flashtex.toml` has `[library] name = "…"`,
//!    [`library`]);
//! 2. the **per-user cache** (`<cache root>/<name>/<version>/…`,
//!    [`cache`]), which is what an offline build has;
//! 3. the **source** the manifest names in `[packages] source` — CTAN, a
//!    registry URL in the CTAN archive layout, or `"none"` ([`source`]) —
//!    which is the only step that touches the network, and only under
//!    the consent rules below.
//!
//! Consent is the caller's, never this crate's: with `fetch = "ask"`
//! [`Resolver::resolve`] returns [`Resolution::NeedsConsent`] describing
//! exactly what would be fetched from where, and the CLI or the app calls
//! [`Resolver::resolve_with_consent`] once the user said yes. `"always"`
//! fetches on the spot (the manifest is the remembered consent);
//! `"never"` and `source = "none"` stop at the cache. Nothing here is
//! executed: the files are handed to the compiler as documents at the
//! virtual project paths `packages/<name>/<file>`. A package that ships
//! `.ins`/`.dtx` sources has them fetched too and unpacked with
//! [`flashtex_docstrip`] (an interpreter of docstrip's batch language,
//! not a TeX) right after the download; the generated `.sty`/`.cls`/…
//! are cached next to the shipped ones with their provenance recorded
//! in `manifest.json`.
//!
//! The network is an injected [`Fetcher`]; unit tests use a fake one over an
//! in-memory archive and never open a socket. The `network` feature adds
//! [`http::HttpFetcher`], the one real client.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod cache;
#[cfg(feature = "network")]
pub mod http;
pub mod library;
pub mod scan;
pub mod source;

pub use cache::GeneratedFrom;
pub use flashtex_project_manifest::{FetchPolicy, PackageSource, Packages};
pub use library::Library;

/// Overrides the platform cache root (a directory path).
pub const CACHE_ENV: &str = "FLASHTEX_PACKAGE_CACHE";

/// The LaTeX source files a package contributes to the document set:
/// what `\usepackage`/`\documentclass`/`\RequirePackage`/`\LoadClass` and
/// the class kernel (`.def`, `.clo`, `.cfg`) can ask for. `.ins`/`.dtx`
/// sources are fetched only to be run through docstrip and are not
/// cached; documentation, fonts and everything else in a CTAN directory
/// is never fetched.
pub const PACKAGE_EXTENSIONS: &[&str] = &["sty", "cls", "def", "clo", "cfg"];

/// The project-relative directory resolved files are mounted at:
/// `packages/<name>/<file>`, after the entry closure and the manifest's
/// `texinputs`, so a project file of the same name still wins.
pub const VIRTUAL_PREFIX: &str = "packages";

/// `packages/<name>/<file>`, the document-set path of one resolved file.
pub fn virtual_path(package: &str, file: &str) -> String {
    format!("{VIRTUAL_PREFIX}/{package}/{file}")
}

/// Whether a file name is one a package contributes ([`PACKAGE_EXTENSIONS`]).
pub fn is_package_file(name: &str) -> bool {
    match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => PACKAGE_EXTENSIONS.contains(&ext),
        _ => false,
    }
}

/// A package name as it may appear in `\usepackage{…}` and as a cache
/// directory: letters, digits, `-`, `_`, `.`, `+`, no separators, not a
/// dot name. Anything else is refused before it reaches a URL or a path.
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name.len() <= 100
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
}

/// The cache root for this user: [`CACHE_ENV`] when set, else
/// - macOS `~/Library/Application Support/FlashTeX/packages`,
/// - Linux (and every other Unix) `${XDG_CACHE_HOME:-~/.cache}/flashtex/packages`,
/// - Windows `%LOCALAPPDATA%\FlashTeX\packages`.
///
/// `None` when no home directory is known. The directory is created on the
/// first store, never here.
pub fn default_cache_root() -> Option<PathBuf> {
    cache_root_from(&|k| std::env::var_os(k).map(PathBuf::from), std::env::consts::OS)
}

/// [`default_cache_root`] over an explicit environment and OS name, so the
/// rule is testable on any machine.
pub fn cache_root_from(var: &dyn Fn(&str) -> Option<PathBuf>, os: &str) -> Option<PathBuf> {
    if let Some(explicit) = var(CACHE_ENV).filter(|p| !p.as_os_str().is_empty()) {
        return Some(explicit);
    }
    match os {
        "macos" => Some(var("HOME")?.join("Library/Application Support/FlashTeX/packages")),
        "windows" => Some(var("LOCALAPPDATA")?.join("FlashTeX").join("packages")),
        _ => {
            let base = match var("XDG_CACHE_HOME").filter(|p| !p.as_os_str().is_empty()) {
                Some(xdg) => xdg,
                None => var("HOME")?.join(".cache"),
            };
            Some(base.join("flashtex/packages"))
        }
    }
}

/// The resolution policy for one package, read off the manifest's
/// `[packages]` table ([`Policy::for_package`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    pub source: PackageSource,
    pub fetch: FetchPolicy,
    /// `[packages] pin` for this name, when set.
    pub pin: Option<String>,
}

impl Policy {
    /// The policy `packages` (a manifest's table) gives `name`.
    pub fn for_package(packages: &Packages, name: &str) -> Policy {
        Policy { source: packages.source.clone(), fetch: packages.fetch, pin: packages.pin.get(name).cloned() }
    }

    /// Never touch the network: what a tool without a manifest uses.
    pub fn never() -> Policy {
        Policy { source: PackageSource::Ctan, fetch: FetchPolicy::Never, pin: None }
    }

    /// `self` with the fetch policy replaced (the CLI's `--fetch`).
    pub fn with_fetch(mut self, fetch: FetchPolicy) -> Policy {
        self.fetch = fetch;
        self
    }
}

/// One resolved file: its name inside the package, where it is on disk and
/// its text. The document-set path is [`virtual_path`]`(package, name)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFile {
    pub name: String,
    pub path: PathBuf,
    pub text: String,
    /// Set when docstrip generated the file from the package's sources.
    pub generated_from: Option<GeneratedFrom>,
}

/// Where a [`Resolution::Cached`] answer came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provenance {
    /// A local library named in `[packages] path`: its key and directory.
    Library { name: String, dir: PathBuf },
    /// The per-user cache.
    Cache,
}

/// What [`Resolver::resolve`] / [`Resolver::resolve_with_consent`] answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Already on this machine: a local library or the cache. No network.
    Cached { name: String, version: String, files: Vec<ResolvedFile>, from: Provenance },
    /// Not on this machine and `fetch = "ask"`: nothing was fetched. The
    /// caller asks the user and, on yes, calls
    /// [`Resolver::resolve_with_consent`]. `version` is the source's
    /// current version when the source states one (CTAN), `would_fetch`
    /// the file names, `source_url` where from.
    NeedsConsent { name: String, version: Option<String>, source_url: String, would_fetch: Vec<String> },
    /// Fetched now and stored in the cache. `notes` is what docstrip
    /// reported while unpacking `.ins`/`.dtx` sources (empty when the
    /// package shipped its files or the batch file was plain docstrip).
    Fetched { name: String, version: String, files: Vec<ResolvedFile>, source_url: String, notes: Vec<String> },
    /// Cannot be resolved, with the reason a diagnostic can carry verbatim:
    /// the policy forbids fetching, the source has no such package, it
    /// ships `.dtx` sources without a `.ins` to run them (or docstrip
    /// produced no package file from them), a pin the source cannot
    /// satisfy, or a network or disk failure.
    NotAvailable { name: String, reason: String },
}

impl Resolution {
    pub fn name(&self) -> &str {
        match self {
            Resolution::Cached { name, .. }
            | Resolution::NeedsConsent { name, .. }
            | Resolution::Fetched { name, .. }
            | Resolution::NotAvailable { name, .. } => name,
        }
    }

    /// The files a `Cached` or `Fetched` answer delivers; empty otherwise.
    pub fn files(&self) -> &[ResolvedFile] {
        match self {
            Resolution::Cached { files, .. } | Resolution::Fetched { files, .. } => files,
            _ => &[],
        }
    }

    pub fn version(&self) -> Option<&str> {
        match self {
            Resolution::Cached { version, .. } | Resolution::Fetched { version, .. } => Some(version),
            Resolution::NeedsConsent { version, .. } => version.as_deref(),
            Resolution::NotAvailable { .. } => None,
        }
    }
}

/// The one network primitive: `GET url` → the body, or why not. The
/// resolver never issues anything else (no POST, no headers of its own).
/// A directory URL (trailing `/`) is expected to answer with a listing in
/// which each file appears as an `href`.
pub trait Fetcher {
    fn get(&self, url: &str) -> Result<Vec<u8>, FetchError>;
}

/// Why a `GET` failed, in the words a diagnostic shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// The server answered with a status other than 2xx (`404` for a
    /// package CTAN does not know).
    Status(u16),
    /// No answer at all: DNS, connection, TLS, timeout, or a `file://`
    /// path that does not exist.
    Transport(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Status(404) => write!(f, "not found (HTTP 404)"),
            FetchError::Status(s) => write!(f, "HTTP {s}"),
            FetchError::Transport(m) => write!(f, "{m}"),
        }
    }
}

/// Resolves package names against local libraries, the cache and a source
/// (see the crate documentation for the order and the consent rules).
pub struct Resolver<'a> {
    cache: cache::Store,
    libraries: Vec<Library>,
    fetcher: &'a dyn Fetcher,
}

impl<'a> Resolver<'a> {
    /// A resolver over `cache_root` (created on the first store) that
    /// fetches through `fetcher`.
    pub fn new(cache_root: impl Into<PathBuf>, fetcher: &'a dyn Fetcher) -> Resolver<'a> {
        Resolver { cache: cache::Store::new(cache_root), libraries: Vec::new(), fetcher }
    }

    /// The local libraries consulted before the cache, in manifest order
    /// ([`library::load_all`]).
    pub fn with_libraries(mut self, libraries: Vec<Library>) -> Resolver<'a> {
        self.libraries = libraries;
        self
    }

    pub fn cache(&self) -> &cache::Store {
        &self.cache
    }

    pub fn libraries(&self) -> &[Library] {
        &self.libraries
    }

    /// Resolves `name` without fetching unless the policy is `always`:
    /// library, cache, then — for `ask` — a description of what a fetch
    /// would bring ([`Resolution::NeedsConsent`]). Describing a CTAN
    /// package is one metadata request to the CTAN JSON API and one
    /// directory listing; no package file is downloaded.
    pub fn resolve(&self, name: &str, policy: &Policy) -> Resolution {
        match self.local(name, policy) {
            Local::Found(r) => return r,
            Local::Refused(r) => return r,
            Local::Missing => {}
        }
        match policy.fetch {
            FetchPolicy::Never => Resolution::NotAvailable {
                name: name.into(),
                reason: format!("{name} is not in the project, a library or the package cache, and the fetch policy is \"never\""),
            },
            FetchPolicy::Ask => match source::describe(self.fetcher, &policy.source, name) {
                Ok(listing) => {
                    if let Some(reason) = listing.pin_conflict(policy.pin.as_deref()) {
                        return Resolution::NotAvailable { name: name.into(), reason };
                    }
                    Resolution::NeedsConsent {
                        name: name.into(),
                        version: listing.version.clone(),
                        source_url: listing.base_url.clone(),
                        would_fetch: listing.files.clone(),
                    }
                }
                Err(reason) => Resolution::NotAvailable { name: name.into(), reason },
            },
            FetchPolicy::Always => self.fetch(name, policy),
        }
    }

    /// [`Resolver::resolve`] after the user consented to the fetch the
    /// `NeedsConsent` answer described: library and cache still win (a
    /// concurrent fetch may have landed), then the fetch. `fetch = "never"`
    /// and `source = "none"` are still refused: consent to a question that
    /// was never asked is not a policy change.
    pub fn resolve_with_consent(&self, name: &str, policy: &Policy) -> Resolution {
        match self.local(name, policy) {
            Local::Found(r) => return r,
            Local::Refused(r) => return r,
            Local::Missing => {}
        }
        if policy.fetch == FetchPolicy::Never {
            return Resolution::NotAvailable {
                name: name.into(),
                reason: format!("{name} is not cached and the fetch policy is \"never\""),
            };
        }
        self.fetch(name, policy)
    }

    /// Fetches `name` from the policy's source into the cache and answers
    /// `Fetched` (or `NotAvailable`), regardless of `fetch`: the explicit
    /// `flashtex packages fetch <name>` command. `source = "none"` is still
    /// refused.
    pub fn fetch(&self, name: &str, policy: &Policy) -> Resolution {
        if !is_valid_name(name) {
            return Resolution::NotAvailable { name: name.into(), reason: format!("{name:?} is not a package name") };
        }
        let listing = match source::describe(self.fetcher, &policy.source, name) {
            Ok(l) => l,
            Err(reason) => return Resolution::NotAvailable { name: name.into(), reason },
        };
        if let Some(reason) = listing.pin_conflict(policy.pin.as_deref()) {
            return Resolution::NotAvailable { name: name.into(), reason };
        }
        let mut bytes: Vec<(String, Vec<u8>)> = Vec::with_capacity(listing.files.len());
        for file in &listing.files {
            let url = format!("{}{file}", listing.base_url);
            match self.fetcher.get(&url) {
                Ok(b) => bytes.push((file.clone(), b)),
                Err(e) => {
                    return Resolution::NotAvailable { name: name.into(), reason: format!("cannot fetch {url}: {e}") };
                }
            }
        }
        // A registry URL states no version; the content (sources included)
        // is the version.
        let version = listing.version.clone().unwrap_or_else(|| cache::content_version(&bytes));
        if let Some(pin) = policy.pin.as_deref() {
            if pin != version {
                return Resolution::NotAvailable {
                    name: name.into(),
                    reason: format!("{name} is pinned to {pin} but {} has {version}; remove the pin or point `source` at an archive that has {pin}", listing.base_url),
                };
            }
        }
        let (files, generated) = match unpack(name, bytes) {
            Ok(u) => u,
            Err(reason) => return Resolution::NotAvailable { name: name.into(), reason },
        };
        match self.cache.store_with(name, &version, &listing.base_url, &files, &generated) {
            Ok(entry) => match entry.read() {
                Ok(files) => Resolution::Fetched { name: name.into(), version, files, source_url: listing.base_url, notes: generated.notes },
                Err(reason) => Resolution::NotAvailable { name: name.into(), reason },
            },
            Err(reason) => Resolution::NotAvailable { name: name.into(), reason: format!("cannot store {name} in the package cache: {reason}") },
        }
    }

    /// The offline part: a library that has `<name>.sty`/`<name>.cls`, else
    /// the cache. `source = "none"` is answered here too, so no policy ever
    /// reaches the network with it.
    fn local(&self, name: &str, policy: &Policy) -> Local {
        if !is_valid_name(name) {
            return Local::Refused(Resolution::NotAvailable { name: name.into(), reason: format!("{name:?} is not a package name") });
        }
        if let Some(lib) = self.libraries.iter().find(|l| l.provides(name)) {
            return Local::Found(Resolution::Cached {
                name: name.into(),
                version: lib.version(),
                files: lib.files.clone(),
                from: Provenance::Library { name: lib.name.clone(), dir: lib.dir.clone() },
            });
        }
        match self.cache.lookup(name, policy.pin.as_deref()) {
            Ok(Some(entry)) => match entry.read() {
                Ok(files) => {
                    return Local::Found(Resolution::Cached { name: name.into(), version: entry.version.clone(), files, from: Provenance::Cache });
                }
                Err(reason) => return Local::Refused(Resolution::NotAvailable { name: name.into(), reason }),
            },
            Ok(None) => {}
            Err(reason) => return Local::Refused(Resolution::NotAvailable { name: name.into(), reason }),
        }
        if policy.source == PackageSource::None {
            return Local::Refused(Resolution::NotAvailable {
                name: name.into(),
                reason: format!("{name} is not in the project, a library or the package cache, and `[packages] source` is \"none\""),
            });
        }
        Local::Missing
    }
}

enum Local {
    Found(Resolution),
    Refused(Resolution),
    Missing,
}

/// The files to cache from a download and what docstrip contributed.
pub type Unpacked = (Vec<(String, Vec<u8>)>, cache::Generated);

/// The files to cache from a download: the shipped package files plus
/// what docstrip generates from the `.ins`/`.dtx` among them, with the
/// provenance of each generated file and docstrip's notes. Every `.ins`
/// is run in name order against the downloaded sources (and against
/// what earlier runs generated). A shipped file always wins over a
/// generated one of the same name; a generated name that is not a
/// plain package file name (a `.tex`, a `.drv`, a path) is dropped with
/// a note. An error is a package with sources from which nothing usable
/// came out.
pub fn unpack(name: &str, fetched: Vec<(String, Vec<u8>)>) -> Result<Unpacked, String> {
    let mut files: Vec<(String, Vec<u8>)> = fetched.iter().filter(|(n, _)| is_package_file(n)).cloned().collect();
    let sources: BTreeMap<String, Vec<u8>> = fetched.into_iter().filter(|(n, _)| source::is_docstrip_input(n)).collect();
    let mut batches: Vec<&String> = sources.keys().filter(|n| flashtex_docstrip::is_batch_file(n)).collect();
    batches.sort();
    let mut generated = cache::Generated::default();
    if batches.is_empty() {
        return Ok((files, generated));
    }
    let shipped: std::collections::BTreeSet<String> = files.iter().map(|(n, _)| n.clone()).collect();
    let options = flashtex_docstrip::Options { today: Some(today()) };
    let mut produced = 0usize;
    for batch in batches {
        let outcome = flashtex_docstrip::run_with(batch, &sources[batch], &sources, &options);
        for d in &outcome.diagnostics {
            generated.notes.push(d.to_string());
        }
        for f in outcome.files {
            produced += 1;
            let plain = !f.name.contains(['/', '\\']);
            if !plain || !is_package_file(&f.name) {
                generated.notes.push(format!("{batch}: {} generated but not cached (not a .sty/.cls/.def/.clo/.cfg file name)", f.name));
                continue;
            }
            if shipped.contains(&f.name) {
                let same = files.iter().any(|(n, b)| *n == f.name && *b == f.bytes);
                generated.notes.push(format!("{batch}: {} is shipped by the package; the shipped copy is kept ({})", f.name, if same { "docstrip's is identical" } else { "docstrip's differs" }));
                continue;
            }
            files.retain(|(n, _)| *n != f.name);
            files.push((f.name.clone(), f.bytes));
            generated.from.insert(f.name, GeneratedFrom { batch: batch.clone(), sources: f.sources });
        }
        if !outcome.completed {
            generated.notes.push(format!("{batch}: the run did not complete"));
        }
    }
    if files.is_empty() {
        let why = generated.notes.iter().take(3).map(String::as_str).collect::<Vec<_>>().join("; ");
        return Err(format!(
            "needs docstrip: {name} ships only sources and running its batch file generated no .sty/.cls/.def/.clo/.cfg ({produced} file(s) generated; {})",
            if why.is_empty() { "no notes".to_string() } else { why }
        ));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((files, generated))
}

/// Today as (year, month, day) in UTC, for `\AddGenerationDate` headers.
fn today() -> (i32, u32, u32) {
    let iso = iso_utc(std::time::SystemTime::now());
    let y = iso[0..4].parse().unwrap_or(0);
    let m = iso[5..7].parse().unwrap_or(0);
    let d = iso[8..10].parse().unwrap_or(0);
    (y, m, d)
}

/// The pins a set of resolutions would record: `name = version` for every
/// `Fetched` answer (the CLI's `--write-pins`, the app's "remember").
pub fn pins_of<'r>(resolutions: impl IntoIterator<Item = &'r Resolution>) -> BTreeMap<String, String> {
    resolutions
        .into_iter()
        .filter_map(|r| match r {
            Resolution::Fetched { name, version, .. } => Some((name.clone(), version.clone())),
            _ => None,
        })
        .collect()
}

/// `SystemTime` as `YYYY-MM-DDTHH:MM:SSZ` (what `manifest.json` records).
pub(crate) fn iso_utc(t: std::time::SystemTime) -> String {
    let secs = t.duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, rem % 3600 / 60, rem % 60);
    // Howard Hinnant's civil-from-days.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// The directory `path` names, for messages.
pub(crate) fn display(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
pub(crate) mod testing {
    //! A fake archive: URL → bytes, with an optional `file://` directory
    //! the way the CLI and helper tests use one.

    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::{FetchError, Fetcher};

    #[derive(Default)]
    pub struct FakeFetcher {
        pub responses: BTreeMap<String, Vec<u8>>,
        pub requests: RefCell<Vec<String>>,
    }

    impl FakeFetcher {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with(mut self, url: &str, body: impl AsRef<[u8]>) -> Self {
            self.responses.insert(url.to_string(), body.as_ref().to_vec());
            self
        }

        /// A CTAN package: the JSON API record, the directory listing and
        /// the files, all under the real endpoints.
        pub fn with_ctan_package(self, name: &str, version: &str, files: &[(&str, &str)]) -> Self {
            let path = format!("/macros/latex/contrib/{name}");
            let json = format!(
                r#"{{"id":"{name}","name":"{name}","version":{{"number":"{version}","date":""}},"ctan":{{"path":"{path}","file":true}},"texlive":"{name}"}}"#
            );
            let listing = files
                .iter()
                .map(|(f, _)| format!("<a href=\"{f}\">{f}</a>"))
                .chain(std::iter::once("<a href=\"../\">Parent</a><a href=\"?C=N;O=D\">Name</a>".to_string()))
                .collect::<Vec<_>>()
                .join("\n");
            let base = format!("https://mirrors.ctan.org{path}/");
            let mut me = self
                .with(&format!("https://ctan.org/json/2.0/pkg/{name}"), json)
                .with(&base, format!("<html><body>{listing}</body></html>"));
            for (f, text) in files {
                me = me.with(&format!("{base}{f}"), text);
            }
            me
        }

        pub fn request_count(&self) -> usize {
            self.requests.borrow().len()
        }
    }

    impl Fetcher for FakeFetcher {
        fn get(&self, url: &str) -> Result<Vec<u8>, FetchError> {
            self.requests.borrow_mut().push(url.to_string());
            match self.responses.get(url) {
                Some(b) => Ok(b.clone()),
                None => Err(FetchError::Status(404)),
            }
        }
    }

    pub fn tmp(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("flashtex-package-resolver-{tag}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{tmp, FakeFetcher};
    use super::*;

    fn ask() -> Policy {
        Policy { source: PackageSource::Ctan, fetch: FetchPolicy::Ask, pin: None }
    }

    #[test]
    fn cache_root_follows_the_platform_and_the_override() {
        let env = |vars: &[(&str, &str)]| {
            let map: BTreeMap<String, String> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
            move |k: &str| map.get(k).map(PathBuf::from)
        };
        assert_eq!(cache_root_from(&env(&[("HOME", "/Users/x")]), "macos"), Some(PathBuf::from("/Users/x/Library/Application Support/FlashTeX/packages")));
        assert_eq!(cache_root_from(&env(&[("HOME", "/home/x")]), "linux"), Some(PathBuf::from("/home/x/.cache/flashtex/packages")));
        assert_eq!(cache_root_from(&env(&[("HOME", "/home/x"), ("XDG_CACHE_HOME", "/c")]), "linux"), Some(PathBuf::from("/c/flashtex/packages")));
        assert_eq!(cache_root_from(&env(&[("LOCALAPPDATA", "C:\\Users\\x\\AppData\\Local")]), "windows"), Some(PathBuf::from("C:\\Users\\x\\AppData\\Local").join("FlashTeX").join("packages")));
        assert_eq!(cache_root_from(&env(&[("HOME", "/h"), (CACHE_ENV, "/override")]), "macos"), Some(PathBuf::from("/override")));
        assert_eq!(cache_root_from(&env(&[]), "linux"), None);
    }

    #[test]
    fn names_and_virtual_paths() {
        assert!(is_valid_name("siunitx"));
        assert!(is_valid_name("l3kernel-2e"));
        assert!(!is_valid_name("../x"));
        assert!(!is_valid_name(".hidden"));
        assert!(!is_valid_name("a b"));
        assert!(!is_valid_name(""));
        assert_eq!(virtual_path("siunitx", "siunitx.sty"), "packages/siunitx/siunitx.sty");
        assert!(is_package_file("a.sty") && is_package_file("b.cls") && is_package_file("c.def") && is_package_file("d.clo") && is_package_file("e.cfg"));
        assert!(!is_package_file("a.dtx") && !is_package_file("a.pdf") && !is_package_file(".sty"));
        assert_eq!(iso_utc(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000)), "2023-11-14T22:13:20Z");
        assert_eq!(iso_utc(std::time::UNIX_EPOCH), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn ask_describes_without_fetching_and_consent_fetches_into_the_cache() {
        let root = tmp("ask");
        let fetcher = FakeFetcher::new().with_ctan_package("cancel", "2.2", &[("cancel.sty", "\\ProvidesPackage{cancel}\n"), ("cancel.pdf", "%PDF"), ("README", "hi")]);
        let resolver = Resolver::new(&root, &fetcher);
        let r = resolver.resolve("cancel", &ask());
        assert_eq!(
            r,
            Resolution::NeedsConsent {
                name: "cancel".into(),
                version: Some("2.2".into()),
                source_url: "https://mirrors.ctan.org/macros/latex/contrib/cancel/".into(),
                would_fetch: vec!["cancel.sty".into()],
            }
        );
        assert_eq!(fetcher.request_count(), 2, "the JSON record and the listing; no file");
        assert!(!root.join("cancel").exists(), "ask stores nothing");

        let r = resolver.resolve_with_consent("cancel", &ask());
        match &r {
            Resolution::Fetched { version, files, source_url, .. } => {
                assert_eq!(version, "2.2");
                assert_eq!(source_url, "https://mirrors.ctan.org/macros/latex/contrib/cancel/");
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].name, "cancel.sty");
                assert_eq!(files[0].text, "\\ProvidesPackage{cancel}\n");
                assert_eq!(files[0].path, root.join("cancel/2.2/cancel.sty"));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(fetcher.request_count(), 5, "record + listing again, then one file");
        let manifest = std::fs::read_to_string(root.join("cancel/2.2/manifest.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        assert_eq!(v["name"], "cancel");
        assert_eq!(v["version"], "2.2");
        assert_eq!(v["source_url"], "https://mirrors.ctan.org/macros/latex/contrib/cancel/");
        assert_eq!(v["files"][0]["name"], "cancel.sty");
        assert_eq!(v["files"][0]["sha256"].as_str().unwrap().len(), 64);
        assert!(v["fetched_utc"].as_str().unwrap().ends_with('Z'));

        // Now cached: no network at all, whatever the policy.
        let before = fetcher.request_count();
        let r = resolver.resolve("cancel", &Policy::never());
        assert!(matches!(&r, Resolution::Cached { from: Provenance::Cache, version, .. } if version == "2.2"), "{r:?}");
        assert_eq!(fetcher.request_count(), before);
        assert_eq!(pins_of([&r]).len(), 0, "cached answers pin nothing");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn never_and_none_stop_at_the_cache_and_always_fetches() {
        let root = tmp("never");
        let fetcher = FakeFetcher::new().with_ctan_package("cancel", "2.2", &[("cancel.sty", "x")]);
        let resolver = Resolver::new(&root, &fetcher);
        let r = resolver.resolve("cancel", &Policy::never());
        assert!(matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("\"never\"")), "{r:?}");
        let r = resolver.resolve("cancel", &Policy { source: PackageSource::None, fetch: FetchPolicy::Always, pin: None });
        assert!(matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("\"none\"")), "{r:?}");
        let r = resolver.resolve_with_consent("cancel", &Policy::never());
        assert!(matches!(r, Resolution::NotAvailable { .. }));
        assert_eq!(fetcher.request_count(), 0);
        let r = resolver.resolve("cancel", &ask().with_fetch(FetchPolicy::Always));
        assert!(matches!(r, Resolution::Fetched { .. }), "{r:?}");
        assert_eq!(pins_of([&r]).get("cancel").map(String::as_str), Some("2.2"));
        let _ = std::fs::remove_dir_all(&root);
    }

    const INS: &str = "\\input docstrip.tex\n\\keepsilent\n\\preamble\nHello.\n\\endpreamble\n\\generate{\\file{lipsum.sty}{\\from{lipsum.dtx}{package}}\\file{lipsum.tex}{\\from{lipsum.dtx}{driver}}}\n\\newread\\x\n\\endbatchfile\n";
    const DTX: &str = "% \\iffalse\n%<*driver>\n\\documentclass{ltxdoc}\n%</driver>\n% \\fi\n%<*package>\n\\ProvidesPackage{lipsum}\n%</package>\n";

    #[test]
    fn a_dtx_ins_package_is_unpacked_with_docstrip() {
        let root = tmp("docstrip");
        let fetcher = FakeFetcher::new()
            .with_ctan_package("lipsum", "2.7", &[("lipsum.dtx", DTX), ("lipsum.ins", INS), ("lipsum.pdf", "%"), ("README", "r")])
            .with_ctan_package("selfins", "1", &[("selfins.dtx", DTX), ("selfins.pdf", "%")])
            .with_ctan_package("nothing", "1", &[("nothing.dtx", DTX), ("nothing.ins", "\\input docstrip\n\\generate{\\file{nothing.tex}{\\from{nothing.dtx}{driver}}}\n")])
            .with_ctan_package("both", "1", &[("both.sty", "shipped\n"), ("both.dtx", DTX.replace("lipsum", "both").as_str()), ("both.ins", INS.replace("lipsum", "both").as_str())]);
        let resolver = Resolver::new(&root, &fetcher);
        // Asking names the sources that would be fetched; nothing runs yet.
        let r = resolver.resolve("lipsum", &ask());
        assert!(matches!(&r, Resolution::NeedsConsent { would_fetch, .. } if would_fetch == &["lipsum.dtx".to_string(), "lipsum.ins".to_string()]), "{r:?}");
        assert!(!root.join("lipsum").exists());
        let r = resolver.resolve_with_consent("lipsum", &ask());
        match &r {
            Resolution::Fetched { files, notes, version, .. } => {
                assert_eq!(version, "2.7");
                assert_eq!(files.len(), 1, "{files:?}");
                assert_eq!(files[0].name, "lipsum.sty");
                assert!(files[0].text.starts_with("%%\n%% This is file `lipsum.sty',\n%% generated with the docstrip utility.\n"), "{}", files[0].text);
                assert!(files[0].text.ends_with("%% Hello.\n\\ProvidesPackage{lipsum}\n\\endinput\n%%\n%% End of file `lipsum.sty'.\n"), "{}", files[0].text);
                assert_eq!(files[0].generated_from, Some(GeneratedFrom { batch: "lipsum.ins".into(), sources: vec!["lipsum.dtx".into()] }));
                assert!(notes.iter().any(|n| n.starts_with("lipsum.ins:7: \\newread is not a docstrip command")), "{notes:?}");
                assert!(notes.iter().any(|n| n.contains("lipsum.tex generated but not cached")), "{notes:?}");
            }
            other => panic!("{other:?}"),
        }
        let dir = root.join("lipsum/2.7");
        assert!(dir.join("lipsum.sty").is_file());
        assert!(!dir.join("lipsum.dtx").exists() && !dir.join("lipsum.ins").exists(), "sources are not cached");
        let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["files"][0]["generated_from"]["sources"][0], "lipsum.dtx");
        assert!(manifest["docstrip"]["notes"].as_array().unwrap().len() >= 2);
        // Cached: the provenance survives.
        let r = resolver.resolve("lipsum", &Policy::never());
        assert!(matches!(&r, Resolution::Cached { files, .. } if files[0].generated_from.is_some()), "{r:?}");
        // A .dtx without a .ins, and a .ins that generates no package file.
        let r = resolver.resolve_with_consent("selfins", &ask());
        assert!(matches!(&r, Resolution::NotAvailable { reason, .. } if reason.starts_with("needs docstrip but ships no .ins")), "{r:?}");
        let r = resolver.resolve_with_consent("nothing", &ask());
        assert!(matches!(&r, Resolution::NotAvailable { reason, .. } if reason.starts_with("needs docstrip") && reason.contains("generated no .sty")), "{r:?}");
        assert!(!root.join("nothing").exists());
        // A shipped file wins over docstrip's copy of the same name.
        let r = resolver.resolve_with_consent("both", &ask());
        match &r {
            Resolution::Fetched { files, notes, .. } => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].text, "shipped\n");
                assert_eq!(files[0].generated_from, None);
                assert!(notes.iter().any(|n| n.contains("both.sty is shipped by the package; the shipped copy is kept (docstrip's differs)")), "{notes:?}");
            }
            other => panic!("{other:?}"),
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_package_and_pins() {
        let root = tmp("pins");
        let fetcher = FakeFetcher::new().with_ctan_package("cancel", "2.2", &[("cancel.sty", "x")]);
        let resolver = Resolver::new(&root, &fetcher);
        let r = resolver.resolve("nosuch", &ask());
        assert!(matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("CTAN has no package") && reason.contains("404")), "{r:?}");
        // A pin the archive cannot satisfy names both versions, before any file moves.
        let pinned = Policy { pin: Some("2.1".into()), ..ask() };
        let r = resolver.resolve("cancel", &pinned);
        assert!(matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("2.1") && reason.contains("2.2")), "{r:?}");
        let r = resolver.resolve_with_consent("cancel", &pinned);
        assert!(matches!(r, Resolution::NotAvailable { .. }));
        assert!(!root.join("cancel").exists());
        // A satisfiable pin fetches; a later pin change to a cached version is served from the cache.
        let r = resolver.resolve_with_consent("cancel", &Policy { pin: Some("2.2".into()), ..ask() });
        assert!(matches!(r, Resolution::Fetched { .. }), "{r:?}");
        let r = resolver.resolve("cancel", &Policy { pin: Some("2.2".into()), ..Policy::never() });
        assert!(matches!(r, Resolution::Cached { .. }));
        let r = resolver.resolve("cancel", &Policy { pin: Some("9.9".into()), ..Policy::never() });
        assert!(matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains("\"never\"")), "the cached 2.2 does not satisfy 9.9: {r:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_registry_url_is_content_versioned() {
        let root = tmp("url");
        let base = "https://registry.example/tex";
        let fetcher = FakeFetcher::new()
            .with(&format!("{base}/macros/latex/contrib/mypkg/"), "<a href=\"mypkg.sty\">mypkg.sty</a><a href=\"mypkg.cfg\">c</a>")
            .with(&format!("{base}/macros/latex/contrib/mypkg/mypkg.sty"), "\\def\\x{1}")
            .with(&format!("{base}/macros/latex/contrib/mypkg/mypkg.cfg"), "cfg");
        let resolver = Resolver::new(&root, &fetcher);
        let policy = Policy { source: PackageSource::Url(base.into()), fetch: FetchPolicy::Ask, pin: None };
        let r = resolver.resolve("mypkg", &policy);
        assert_eq!(
            r,
            Resolution::NeedsConsent { name: "mypkg".into(), version: None, source_url: format!("{base}/macros/latex/contrib/mypkg/"), would_fetch: vec!["mypkg.cfg".into(), "mypkg.sty".into()] }
        );
        let r = resolver.resolve_with_consent("mypkg", &policy);
        let version = match &r {
            Resolution::Fetched { version, files, .. } => {
                assert_eq!(files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["mypkg.cfg", "mypkg.sty"]);
                version.clone()
            }
            other => panic!("{other:?}"),
        };
        assert_eq!(version.len(), 12);
        assert!(root.join("mypkg").join(&version).join("mypkg.sty").is_file());
        // The pin is the content hash; a different pin is a conflict after the download, never a store.
        let r = resolver.resolve("mypkg", &Policy { pin: Some(version.clone()), ..policy.clone() });
        assert!(matches!(r, Resolution::Cached { .. }));
        let _ = std::fs::remove_dir_all(root.join("mypkg"));
        let r = resolver.resolve_with_consent("mypkg", &Policy { pin: Some("000000000000".into()), ..policy });
        assert!(matches!(&r, Resolution::NotAvailable { reason, .. } if reason.contains(&version) && reason.contains("000000000000")), "{r:?}");
        assert!(!root.join("mypkg").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_local_library_wins_over_the_cache_and_the_network() {
        let root = tmp("lib");
        let project = root.join("project");
        let lib = root.join("mylib");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(lib.join("flashtex.toml"), "[library]\nname = \"mylib\"\n").unwrap();
        std::fs::write(lib.join("mylib.sty"), "\\ProvidesPackage{mylib}\n").unwrap();
        std::fs::write(lib.join("helper.def"), "%def\n").unwrap();
        std::fs::write(lib.join("notes.txt"), "not a document\n").unwrap();
        let text = "[packages]\npath = { mylib = \"../mylib\" }\n";
        let manifest = flashtex_project_manifest::Manifest::parse(text).unwrap().manifest;
        let (libraries, diagnostics) = library::load_all(&manifest, &project);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(libraries.len(), 1);
        let fetcher = FakeFetcher::new().with_ctan_package("mylib", "1.0", &[("mylib.sty", "from ctan")]);
        let cache = root.join("cache");
        let resolver = Resolver::new(&cache, &fetcher).with_libraries(libraries);
        let r = resolver.resolve("mylib", &Policy::for_package(&manifest.packages, "mylib"));
        match &r {
            Resolution::Cached { from: Provenance::Library { name, dir }, files, .. } => {
                assert_eq!(name, "mylib");
                assert_eq!(dir, &lib);
                assert_eq!(files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["helper.def", "mylib.sty"]);
                assert_eq!(files[1].text, "\\ProvidesPackage{mylib}\n");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(fetcher.request_count(), 0);
        // A package the library does not provide still goes to the source.
        assert!(matches!(resolver.resolve("other", &ask()), Resolution::NotAvailable { .. }));
        let _ = std::fs::remove_dir_all(&root);
    }
}
