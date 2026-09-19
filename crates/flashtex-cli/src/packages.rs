//! Package resolution on the command line (docs/user/project-manifest.md
//! `[packages]`, crates/package-resolver): the packages and classes the
//! documents ask for that neither the compiler models nor the document set
//! supplies are resolved from the manifest's local libraries, the per-user
//! cache and — under the fetch policy — the source, and join the document
//! set at `packages/<name>/<file>` after the `texinputs` files.
//!
//! Policy: `--fetch ask|always|never` outranks the manifest's `fetch`;
//! without a manifest and without `--fetch` nothing here runs at all (a
//! project without a manifest builds exactly as before, and a batch tool
//! must not prompt or touch the network unasked). `ask` prompts once per
//! package when stdin and stderr are terminals; otherwise it is `never`
//! with a diagnostic saying how to fetch. A fetched version is recorded in
//! the manifest's `pin` table only with `--write-pins`: a build must not
//! rewrite the user's manifest as a side effect, and a pin is a decision
//! (the project now depends on that version) the user makes explicitly.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use flashtex_package_resolver::scan::{package_references, RefKind};
use flashtex_package_resolver::{library, virtual_path, FetchPolicy, Policy, Resolution, Resolver};
use flashtex_project_manifest::Manifest;

use crate::project::{Document, Input, Project, ProjectDiagnostic};

/// The classes LaTeX itself ships (and `beamer`, which the engine models):
/// never fetched, since CTAN's contrib tree has no directory for them.
const BASE_CLASSES: &[&str] = &["article", "report", "book", "letter", "proc", "slides", "minimal", "beamer"];

/// How `build`/`check`/`watch` resolve packages.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// `--fetch`; `None` takes the manifest's policy.
    pub fetch: Option<FetchPolicy>,
    /// `--write-pins`.
    pub write_pins: bool,
}

impl Options {
    /// Whether resolution runs at all: with a manifest, or with `--fetch`.
    pub fn applies(&self, project: &Project) -> bool {
        self.fetch.is_some() || project.manifest.is_some()
    }
}

/// A question `ask` puts to the user: what would be fetched from where.
pub struct Question<'a> {
    pub name: &'a str,
    pub version: Option<&'a str>,
    pub source_url: &'a str,
    pub files: &'a [String],
}

impl Question<'_> {
    /// The prompt line (without the `[y/N]`).
    pub fn text(&self) -> String {
        let what = match self.version {
            Some(v) => format!("{} {v}", self.name),
            None => self.name.to_string(),
        };
        let source = if self.source_url.starts_with("https://mirrors.ctan.org/") { "CTAN" } else { "the registry" };
        format!("\\usepackage{{{}}} is not in this project or the cache. Fetch {what} from {source} ({})?", self.name, self.source_url)
    }
}

/// What resolution produced: documents to append, diagnostics to report,
/// and the pins a `--write-pins` records.
#[derive(Debug, Default)]
pub struct Outcome {
    pub documents: Vec<Document>,
    pub diagnostics: Vec<ProjectDiagnostic>,
    pub pins: BTreeMap<String, String>,
}

/// The package/class names the documents ask for that the document set
/// does not supply and the compiler does not model, in first-use order.
pub fn unresolved_names(project: &Project) -> Vec<String> {
    let supplied: BTreeSet<String> = project
        .documents
        .iter()
        .filter_map(|d| Path::new(&d.path).file_name().and_then(|n| n.to_str()).map(str::to_string))
        .collect();
    let modelled: BTreeSet<&str> = flashtex_compiler::supported::inventory().packages.iter().map(|p| p.name).collect();
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for d in &project.documents {
        if !d.path.ends_with(".tex") && !d.path.ends_with(".sty") && !d.path.ends_with(".cls") {
            continue;
        }
        for r in package_references(&d.text) {
            let modelled = match r.kind {
                RefKind::Package => modelled.contains(r.name.as_str()),
                RefKind::Class => BASE_CLASSES.contains(&r.name.as_str()),
            };
            if modelled || supplied.contains(&r.kind.file_name(&r.name)) || !seen.insert(r.name.clone()) {
                continue;
            }
            out.push(r.name);
        }
    }
    out
}

/// Resolves every unresolved name of `project` (see [`unresolved_names`]).
/// `ask` answers a [`Question`] (the terminal prompt; tests substitute).
pub fn resolve(project: &Project, input: &Input, options: &Options, ask: &mut dyn FnMut(&Question) -> bool) -> Outcome {
    let mut outcome = Outcome::default();
    let manifest_name = project.manifest.as_deref().and_then(Path::file_name).map_or_else(|| flashtex_project_manifest::FILE_NAME.to_string(), |n| n.to_string_lossy().into_owned());
    let diag = |code: &'static str, message: String| ProjectDiagnostic { path: manifest_name.clone(), start_byte: None, end_byte: None, error: false, code, message };
    let names = unresolved_names(project);
    if names.is_empty() {
        return outcome;
    }
    let Some(cache_root) = flashtex_package_resolver::default_cache_root() else {
        outcome.diagnostics.push(diag("package_cache", format!("no package cache: set {} (no home directory is known)", flashtex_package_resolver::CACHE_ENV)));
        return outcome;
    };
    let fetcher = match flashtex_package_resolver::http::HttpFetcher::new() {
        Ok(f) => f,
        Err(e) => {
            outcome.diagnostics.push(diag("package_fetch", e));
            return outcome;
        }
    };
    let manifest_dir = input.manifest_dir.clone().unwrap_or_else(|| project.root.clone());
    let (libraries, library_diagnostics) = library::load_all(&input.manifest.manifest, &manifest_dir);
    for m in library_diagnostics {
        outcome.diagnostics.push(diag("manifest_packages_path", m));
    }
    let resolver = Resolver::new(cache_root, &fetcher).with_libraries(libraries);
    let interactive = std::io::stdin().is_terminal() && std::io::stderr().is_terminal();
    let mut delivered: BTreeSet<String> = BTreeSet::new();
    for name in names {
        let mut policy = Policy::for_package(&input.manifest.manifest.packages, &name);
        if let Some(f) = options.fetch {
            policy = policy.with_fetch(f);
        }
        let mut resolution = resolver.resolve(&name, &policy);
        if let Resolution::NeedsConsent { version, source_url, would_fetch, .. } = &resolution {
            if !interactive {
                outcome.diagnostics.push(diag(
                    "package_fetch",
                    format!("{name} is not in this project or the package cache; the fetch policy is \"ask\" but this is not a terminal: run `flashtex packages fetch {name}` or build with `--fetch always`"),
                ));
                continue;
            }
            let question = Question { name: &name, version: version.as_deref(), source_url, files: would_fetch };
            if ask(&question) {
                resolution = resolver.resolve_with_consent(&name, &policy);
            } else {
                outcome.diagnostics.push(diag("package_fetch", format!("{name}: fetch declined; `[packages] fetch = \"never\"` in {manifest_name} stops the question")));
                continue;
            }
        }
        match resolution {
            Resolution::Cached { files, from, .. } => {
                let mount = match &from {
                    flashtex_package_resolver::Provenance::Library { name, .. } => name.clone(),
                    flashtex_package_resolver::Provenance::Cache => name.clone(),
                };
                deliver(&mut outcome, &mut delivered, &mount, &files);
            }
            Resolution::Fetched { version, files, source_url, .. } => {
                eprintln!("flashtex: fetched {name} {version} from {source_url}");
                outcome.pins.insert(name.clone(), version);
                deliver(&mut outcome, &mut delivered, &name, &files);
            }
            Resolution::NotAvailable { reason, .. } => outcome.diagnostics.push(diag("package_unavailable", reason)),
            Resolution::NeedsConsent { .. } => unreachable!("answered above"),
        }
    }
    outcome
}

fn deliver(outcome: &mut Outcome, delivered: &mut BTreeSet<String>, mount: &str, files: &[flashtex_package_resolver::ResolvedFile]) {
    for f in files {
        let path = virtual_path(mount, &f.name);
        if delivered.insert(path.clone()) {
            outcome.documents.push(Document { path, text: f.text.clone() });
        }
    }
}

/// The terminal prompt for one [`Question`]: `… [y/N] ` on stderr, one
/// line from stdin; anything but `y`/`yes` is no. A package declined once
/// is not asked about again in this process (`watch` rebuilds many times).
pub fn prompt(q: &Question) -> bool {
    static DECLINED: std::sync::Mutex<BTreeSet<String>> = std::sync::Mutex::new(BTreeSet::new());
    if DECLINED.lock().map(|d| d.contains(q.name)).unwrap_or(false) {
        return false;
    }
    let yes = prompt_once(q);
    if !yes {
        if let Ok(mut d) = DECLINED.lock() {
            d.insert(q.name.to_string());
        }
    }
    yes
}

fn prompt_once(q: &Question) -> bool {
    let mut err = std::io::stderr().lock();
    let _ = write!(err, "flashtex: {} [y/N] ", q.text());
    let _ = err.flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// `--write-pins`: records `pins` into the manifest's `pin` table (merged
/// over the existing pins; the rest of the file is kept byte for byte).
/// Without a manifest one is created next to the entry from the template.
pub fn write_pins(project: &Project, pins: &BTreeMap<String, String>) -> Result<Option<PathBuf>, String> {
    if pins.is_empty() {
        return Ok(None);
    }
    let (path, text) = match &project.manifest {
        Some(p) => (p.clone(), std::fs::read_to_string(p).map_err(|e| format!("cannot read {}: {e}", p.display()))?),
        None => (project.root.join(flashtex_project_manifest::FILE_NAME), Manifest::template(&project.entry)),
    };
    let mut merged = Manifest::parse(&text).map_err(|e| e.to_string())?.manifest.packages.pin;
    merged.extend(pins.iter().map(|(k, v)| (k.clone(), v.clone())));
    let rewritten = Manifest::with_packages(&text, None, Some(&merged));
    if rewritten == text {
        return Ok(None);
    }
    std::fs::write(&path, rewritten).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(Some(path))
}
