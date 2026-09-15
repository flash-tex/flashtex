//! Normalized project graph: root directory, entry file, discovered
//! references, resolved files, cycle/missing/escape diagnostics, and the
//! runtime-v1 `documents` export.
//!
//! Resolution follows TeX's working-directory rule: every reference is
//! resolved against the project root, not the referencing file's directory
//! (`\input{chapters/a}` inside `chapters/main.tex` still means
//! `<root>/chapters/a.tex`). Files are visited depth-first in reference
//! order, so `files()` and `documents()` are deterministic for a given tree.

use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::json::Json;
use crate::path::{PathError, ProjectPath};
use crate::save::{DEFAULT_READ_LIMIT, ProjectRoot, Refused, SaveError};
use crate::scan::{ByteSpan, Reference, ReferenceKind, scan_references};
use crate::sha256::{Digest, sha256};

/// Maximum nesting of `\input`/`\include` before discovery stops descending.
pub const MAX_DEPTH: usize = 64;

/// Extensions tried, in order, for `\includegraphics{name}` without extension.
pub const GRAPHIC_EXTENSIONS: &[&str] = &["pdf", "png", "jpg", "jpeg", "eps", "svg"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FileKind {
    /// LaTeX source; scanned for references and exported as a document.
    Tex,
    /// BibTeX database; loaded as text, not scanned.
    Bibliography,
    /// Graphic asset; hashed but not loaded as text.
    Graphic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSource {
    /// Bytes read from `<root>/<path>`.
    Disk,
    /// Text supplied by the caller (unsaved editor buffer); disk was not read.
    Overlay,
}

/// One file in the graph.
#[derive(Debug, Clone)]
pub struct ProjectFile {
    pub path: ProjectPath,
    pub kind: FileKind,
    pub source: FileSource,
    /// UTF-8 text for `Tex` and `Bibliography`; `None` for graphics or when
    /// the bytes were not valid UTF-8 (a diagnostic is emitted).
    pub text: Option<String>,
    /// SHA-256 of the exact bytes (overlay text or disk contents).
    pub sha256: Digest,
    pub bytes: u64,
    /// References found in this file (`Tex` only).
    pub references: Vec<Reference>,
}

/// A resolved reference from one file to another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub from: ProjectPath,
    pub to: ProjectPath,
    pub reference: Reference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    /// No candidate for the referenced name exists.
    MissingFile {
        target: String,
        tried: Vec<ProjectPath>,
    },
    /// The referenced name is not a valid project-relative path.
    InvalidPath { target: String, error: PathError },
    /// The referenced name is, or lies under, a symbolic link (refused
    /// wherever the link points: project files are read without following
    /// symlinks), or a walked directory no longer leads back to the root.
    EscapesRootViaSymlink { target: ProjectPath },
    /// The reference closes a cycle; `chain` runs from the first repeated
    /// file to the referencing file, and the target is `chain[0]`.
    Cycle { chain: Vec<ProjectPath> },
    /// The argument contains `\` or `#` and cannot be resolved without expansion.
    UnresolvableReference { target: String },
    /// The file exists but is not valid UTF-8.
    InvalidUtf8 { path: ProjectPath },
    /// The file exists but could not be read.
    ReadError { path: ProjectPath, message: String },
    /// Nesting exceeded [`MAX_DEPTH`]; the target was not descended into.
    DepthExceeded { target: ProjectPath },
}

/// A discovery diagnostic attributed to the referencing file and span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    /// The file containing the reference (or the file itself for read errors).
    pub path: ProjectPath,
    /// Span of the whole referencing command, if any.
    pub span: Option<ByteSpan>,
    /// Span of the referenced name inside the command, if any.
    pub argument_span: Option<ByteSpan>,
    pub kind: DiagnosticKind,
}

/// Unsaved buffers that take precedence over disk contents during discovery.
#[derive(Debug, Clone, Default)]
pub struct Overlay {
    texts: BTreeMap<ProjectPath, String>,
}

impl Overlay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: ProjectPath, text: impl Into<String>) -> &mut Self {
        self.texts.insert(path, text.into());
        self
    }

    pub fn remove(&mut self, path: &ProjectPath) -> Option<String> {
        self.texts.remove(path)
    }

    pub fn get(&self, path: &ProjectPath) -> Option<&str> {
        self.texts.get(path).map(String::as_str)
    }

    pub fn paths(&self) -> impl Iterator<Item = &ProjectPath> {
        self.texts.keys()
    }
}

/// Why discovery could not even start.
#[derive(Debug)]
pub enum DiscoverError {
    RootNotDirectory(PathBuf),
    EntryMissing(ProjectPath),
    EntryUnreadable(ProjectPath, io::Error),
    EntryNotUtf8(ProjectPath),
}

impl fmt::Display for DiscoverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscoverError::RootNotDirectory(p) => {
                write!(f, "project root {} is not a directory", p.display())
            }
            DiscoverError::EntryMissing(p) => write!(f, "entry file {p} does not exist"),
            DiscoverError::EntryUnreadable(p, e) => {
                write!(f, "entry file {p} could not be read: {e}")
            }
            DiscoverError::EntryNotUtf8(p) => write!(f, "entry file {p} is not valid UTF-8"),
        }
    }
}

impl std::error::Error for DiscoverError {}

/// A runtime-v1 document: `{path, text}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ProjectGraph {
    root: PathBuf,
    entry: ProjectPath,
    files: Vec<ProjectFile>,
    edges: Vec<Edge>,
    diagnostics: Vec<Diagnostic>,
}

impl ProjectGraph {
    /// Discovers the graph from disk only.
    pub fn discover(root: &Path, entry: &ProjectPath) -> Result<ProjectGraph, DiscoverError> {
        Self::discover_with(root, entry, &Overlay::default())
    }

    /// Discovers the graph, preferring `overlay` buffers over disk contents.
    pub fn discover_with(
        root: &Path,
        entry: &ProjectPath,
        overlay: &Overlay,
    ) -> Result<ProjectGraph, DiscoverError> {
        if !root.is_dir() {
            return Err(DiscoverError::RootNotDirectory(root.to_path_buf()));
        }
        // Opens the root once as a directory handle; every subsequent read
        // walks from this handle with `openat(O_NOFOLLOW)` at each
        // component (see `sys.rs`/`save.rs`), so containment is enforced on
        // the exact file descriptor that is then read — never re-resolved
        // from a path string after being validated (issue #45).
        let project_root = ProjectRoot::open(root)
            .map_err(|_| DiscoverError::RootNotDirectory(root.to_path_buf()))?;
        let mut d = Discovery {
            root: root.to_path_buf(),
            project_root,
            overlay,
            graph: ProjectGraph {
                root: root.to_path_buf(),
                entry: entry.clone(),
                files: Vec::new(),
                edges: Vec::new(),
                diagnostics: Vec::new(),
            },
            index: BTreeMap::new(),
            stack: Vec::new(),
        };
        // The entry must load; anything else is a diagnostic.
        match d.load(entry, FileKind::Tex) {
            Resolution::Other(Loaded::Ok(file)) => {
                if file.text.is_none() {
                    return Err(DiscoverError::EntryNotUtf8(entry.clone()));
                }
                d.visit_loaded(file);
            }
            Resolution::Other(Loaded::Missing) => {
                return Err(DiscoverError::EntryMissing(entry.clone()));
            }
            Resolution::Other(Loaded::Error(e)) => {
                return Err(DiscoverError::EntryUnreadable(entry.clone(), e));
            }
            Resolution::Escapes(escape) => {
                return Err(DiscoverError::EntryUnreadable(
                    entry.clone(),
                    io::Error::other(escape.describe(entry)),
                ));
            }
        }
        Ok(d.graph)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entry(&self) -> &ProjectPath {
        &self.entry
    }

    /// Files in deterministic depth-first discovery order; the entry is first.
    pub fn files(&self) -> &[ProjectFile] {
        &self.files
    }

    pub fn file(&self, path: &ProjectPath) -> Option<&ProjectFile> {
        self.files.iter().find(|f| &f.path == path)
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }

    /// Paths of all text files (tex and bib) in discovery order.
    pub fn text_paths(&self) -> impl Iterator<Item = &ProjectPath> {
        self.files
            .iter()
            .filter(|f| f.kind != FileKind::Graphic)
            .map(|f| &f.path)
    }

    /// Paths of every discovered file, including graphics.
    pub fn all_paths(&self) -> impl Iterator<Item = &ProjectPath> {
        self.files.iter().map(|f| &f.path)
    }

    /// The runtime-v1 `documents` list: every `Tex` file with valid UTF-8
    /// text, entry first, then depth-first reference order.
    pub fn documents(&self) -> Vec<Document> {
        self.documents_where(|f| f.kind == FileKind::Tex)
    }

    /// Like [`documents`](Self::documents) but also carries `.bib` sources
    /// (for a bibliography-aware compiler; runtime-v1 does not forbid them).
    pub fn documents_including_bibliography(&self) -> Vec<Document> {
        self.documents_where(|f| f.kind != FileKind::Graphic)
    }

    fn documents_where(&self, keep: impl Fn(&ProjectFile) -> bool) -> Vec<Document> {
        self.files
            .iter()
            .filter(|f| keep(f))
            .filter_map(|f| {
                f.text.as_ref().map(|t| Document {
                    path: f.path.as_str().to_string(),
                    text: t.clone(),
                })
            })
            .collect()
    }

    /// The runtime-v1 `compile` payload object for `project_id` at `revision`.
    pub fn compile_payload(&self, project_id: &str, revision: u64) -> Json {
        let docs = self
            .documents()
            .into_iter()
            .map(|d| {
                let mut o = Json::object();
                o.insert("path", d.path).insert("text", d.text);
                o
            })
            .collect::<Vec<_>>();
        let mut payload = Json::object();
        payload
            .insert("project_id", project_id)
            .insert("revision", revision)
            .insert("entry_path", self.entry.as_str())
            .insert("documents", docs);
        payload
    }

    /// A complete runtime-v1 `compile` envelope line (no trailing newline).
    pub fn compile_envelope(&self, id: &str, project_id: &str, revision: u64) -> String {
        let mut env = Json::object();
        env.insert("protocol_version", 1u64)
            .insert("id", id)
            .insert("type", "compile")
            .insert("payload", self.compile_payload(project_id, revision));
        env.to_string_compact()
    }
}

enum Loaded {
    Ok(ProjectFile),
    Missing,
    Error(io::Error),
}

/// Outcome of a single fd-based, symlink-refusing disk access. `Escapes` is
/// pulled out as its own case (rather than folded into `Loaded::Error`) so
/// callers can distinguish "refused for safety" from an ordinary I/O
/// failure and raise the right diagnostic.
enum Resolution {
    Other(Loaded),
    /// The rooted walk refused a symlink component (the file itself or an
    /// ancestor directory) or detected a walked directory's `..` no longer
    /// matching the handle it was opened from.
    Escapes(Escape),
}

/// Why a rooted access was refused as an escape. Neither case follows the
/// link, so where a symlink points (inside or outside the root) is unknown.
enum Escape {
    /// `component` (the file itself or an ancestor directory) is a symlink.
    Symlink(String),
    /// Directory `component`'s `..` is not the directory it was reached from.
    LeavesRoot(String),
}

impl Escape {
    fn describe(&self, target: &ProjectPath) -> String {
        match self {
            Escape::Symlink(c) if c == target.as_str() => format!(
                "{target} is a symbolic link; project files are read without following symlinks"
            ),
            Escape::Symlink(c) => format!(
                "{target}: `{c}` is a symbolic link; project files are read without following symlinks"
            ),
            Escape::LeavesRoot(c) => format!(
                "{target}: directory `{c}` does not lead back to the project root; refusing to read through it"
            ),
        }
    }
}

/// Maps a rooted-access refusal to how discovery should treat it. Only a
/// symlink component or a `..`-identity mismatch counts as an escape
/// attempt; everything else becomes a typed I/O error (never a panic, never
/// silently ignored).
fn classify_refusal(refused: Refused) -> Resolution {
    match refused {
        Refused::SymlinkComponent { component } => Resolution::Escapes(Escape::Symlink(component)),
        Refused::EscapesRoot { component } => Resolution::Escapes(Escape::LeavesRoot(component)),
        // A directory component turned out not to be a directory: treat
        // like "the candidate doesn't actually exist", matching how a
        // plain ENOENT is handled.
        Refused::NotADirectory { .. } => Resolution::Other(Loaded::Missing),
        Refused::NotARegularFile { component } => Resolution::Other(Loaded::Error(
            io::Error::other(format!("{component} exists but is not a regular file")),
        )),
        Refused::TooLarge { limit, size } => Resolution::Other(Loaded::Error(io::Error::other(
            format!("file is {size} bytes, larger than the {limit}-byte discovery limit"),
        ))),
        Refused::LockUnavailable { .. } => Resolution::Other(Loaded::Error(io::Error::other(
            "unexpected lock contention while reading",
        ))),
        Refused::Unsupported => Resolution::Other(Loaded::Error(io::Error::other(
            "rooted file access is not supported on this platform",
        ))),
    }
}

struct Discovery<'a> {
    root: PathBuf,
    project_root: ProjectRoot,
    overlay: &'a Overlay,
    graph: ProjectGraph,
    index: BTreeMap<ProjectPath, usize>,
    stack: Vec<ProjectPath>,
}

impl Discovery<'_> {
    /// Resolves `path` to the file that actually backs it, if any.
    ///
    /// A literal lookup (overlay, then the exact on-disk bytes) is tried
    /// first and returns `path` unchanged. On a normalization-insensitive
    /// filesystem (APFS) that is already enough: `\input{café}` and
    /// `\input{cafe´}` (NFD) both resolve the same physical file because the
    /// OS itself treats the two byte spellings as the same lookup. On a
    /// normalization-*sensitive* filesystem (ext4) the literal lookup for
    /// whichever spelling was not used on disk fails, so a second pass lists
    /// the containing directory and matches entries by [`ProjectPath`]
    /// identity (Unicode-NFC-normalized), which is spelling-insensitive
    /// regardless of what the filesystem does. This keeps resolution
    /// filesystem-independent: the same graph comes out on ext4 and APFS.
    ///
    /// The returned path carries the *on-disk* raw bytes (never the
    /// as-referenced spelling), so a physical file always gets exactly one
    /// graph entry no matter how many differently-normalized spellings
    /// reference it (issue #45 finding 3), and the subsequent rooted read in
    /// [`Discovery::load`] is against bytes that actually exist on disk.
    fn resolve_existing(&self, path: &ProjectPath) -> Option<ProjectPath> {
        if self.overlay.get(path).is_some() {
            return Some(path.clone());
        }
        if path.to_os_path(&self.root).is_file() {
            return Some(path.clone());
        }
        self.resolve_via_directory_listing(path)
    }

    /// Lists `path`'s parent directory (a plain, non-fd-rooted read — the
    /// same trust level `resolve_existing`'s literal `is_file()` check
    /// already has) looking for an entry whose name is the *same*
    /// [`ProjectPath`] identity as `path` (NFC-normalized comparison, so any
    /// differently-normalized spelling of the same name matches). This never
    /// grants extra trust: whatever name is found here still has to pass
    /// through the fd-rooted, symlink-refusing [`Discovery::load`] before its
    /// content is read, exactly like a literal candidate would.
    fn resolve_via_directory_listing(&self, path: &ProjectPath) -> Option<ProjectPath> {
        let parent_dir = path.parent_dir();
        let mut dir_os_path = self.root.clone();
        if !parent_dir.is_empty() {
            for seg in parent_dir.split('/') {
                dir_os_path.push(seg);
            }
        }
        let entries = std::fs::read_dir(&dir_os_path).ok()?;
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            // A symlink entry is filtered here too (its own file type, not
            // the target's), but this is belt-and-suspenders: `load` refuses
            // to follow it either way.
            if !file_type.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().into_string().ok() else {
                continue; // not valid UTF-8; cannot match a ProjectPath
            };
            let candidate_str = if parent_dir.is_empty() {
                name
            } else {
                format!("{parent_dir}/{name}")
            };
            let Ok(on_disk) = ProjectPath::normalize(&candidate_str) else {
                continue;
            };
            if &on_disk == path {
                return Some(on_disk);
            }
        }
        None
    }

    /// Loads `path` through the rooted, symlink-refusing primitive that
    /// `save.rs` already uses: containment is checked and the content is
    /// read from the exact same file descriptor in one walk, so there is no
    /// window between "is this safe" and "read it" for a symlink swap to
    /// win (issue #45 finding 1).
    fn load(&self, path: &ProjectPath, kind: FileKind) -> Resolution {
        if kind != FileKind::Graphic
            && let Some(text) = self.overlay.get(path)
        {
            return Resolution::Other(Loaded::Ok(ProjectFile {
                path: path.clone(),
                kind,
                source: FileSource::Overlay,
                text: Some(text.to_string()),
                sha256: sha256(text.as_bytes()),
                bytes: text.len() as u64,
                references: Vec::new(),
            }));
        }
        match self.project_root.read(path, DEFAULT_READ_LIMIT) {
            Ok(Some(r)) => {
                let len = r.bytes.len() as u64;
                let text = if kind == FileKind::Graphic {
                    None
                } else {
                    String::from_utf8(r.bytes).ok()
                };
                Resolution::Other(Loaded::Ok(ProjectFile {
                    path: path.clone(),
                    kind,
                    source: FileSource::Disk,
                    text,
                    sha256: r.sha256,
                    bytes: len,
                    references: Vec::new(),
                }))
            }
            Ok(None) => Resolution::Other(Loaded::Missing),
            Err(SaveError::Refused(refused)) => classify_refusal(refused),
            Err(SaveError::Io(e)) => Resolution::Other(Loaded::Error(e)),
            Err(SaveError::DirectorySync(e)) => Resolution::Other(Loaded::Error(e)),
            Err(SaveError::Conflict(c)) => Resolution::Other(Loaded::Error(io::Error::other(
                format!("unexpected conflict during read: {c:?}"),
            ))),
        }
    }

    fn add_file(&mut self, mut file: ProjectFile) -> usize {
        if file.kind == FileKind::Tex
            && let Some(text) = &file.text
        {
            file.references = scan_references(text);
        }
        let idx = self.graph.files.len();
        self.index.insert(file.path.clone(), idx);
        self.graph.files.push(file);
        idx
    }

    fn visit_loaded(&mut self, file: ProjectFile) {
        let path = file.path.clone();
        if file.kind == FileKind::Tex && file.text.is_none() {
            self.graph.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!("{path} is not valid UTF-8"),
                path: path.clone(),
                span: None,
                argument_span: None,
                kind: DiagnosticKind::InvalidUtf8 { path: path.clone() },
            });
        }
        let idx = self.add_file(file);
        self.stack.push(path.clone());
        let refs = self.graph.files[idx].references.clone();
        for r in refs {
            self.follow(&path, &r);
        }
        self.stack.pop();
    }

    fn diag(
        &mut self,
        from: &ProjectPath,
        r: &Reference,
        severity: Severity,
        message: String,
        kind: DiagnosticKind,
    ) {
        self.graph.diagnostics.push(Diagnostic {
            severity,
            message,
            path: from.clone(),
            span: Some(r.span),
            argument_span: Some(r.argument_span),
            kind,
        });
    }

    fn follow(&mut self, from: &ProjectPath, r: &Reference) {
        if !r.literal {
            self.diag(
                from,
                r,
                Severity::Warning,
                format!(
                    "\\{}{{{}}} cannot be resolved without macro expansion",
                    r.kind.command(),
                    r.argument
                ),
                DiagnosticKind::UnresolvableReference {
                    target: r.argument.clone(),
                },
            );
            return;
        }
        let base = match ProjectPath::normalize(&r.argument) {
            Ok(p) => p,
            Err(e) => {
                self.diag(
                    from,
                    r,
                    Severity::Error,
                    format!("\\{}{{{}}}: {e}", r.kind.command(), r.argument),
                    DiagnosticKind::InvalidPath {
                        target: r.argument.clone(),
                        error: e,
                    },
                );
                return;
            }
        };
        let (kind, candidates) = candidates(r.kind, &base);
        let Some(target) = candidates.iter().find_map(|c| self.resolve_existing(c)) else {
            let severity = if kind == FileKind::Graphic {
                Severity::Warning
            } else {
                Severity::Error
            };
            self.diag(
                from,
                r,
                severity,
                format!(
                    "\\{}{{{}}}: no file found (tried {})",
                    r.kind.command(),
                    r.argument,
                    candidates
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                DiagnosticKind::MissingFile {
                    target: r.argument.clone(),
                    tried: candidates,
                },
            );
            return;
        };
        // Single fd-based check-and-read: safety and content come from the
        // exact same rooted operation (see `load`), so there is no window
        // for a symlink swapped in after a separate check to be followed
        // (issue #45 finding 1). The result is reused below rather than
        // touching disk a second time.
        let loaded = match self.load(&target, kind) {
            Resolution::Escapes(escape) => {
                self.diag(
                    from,
                    r,
                    Severity::Error,
                    format!(
                        "\\{}{{{}}}: {}",
                        r.kind.command(),
                        r.argument,
                        escape.describe(&target)
                    ),
                    DiagnosticKind::EscapesRootViaSymlink { target },
                );
                return;
            }
            Resolution::Other(loaded) => loaded,
        };
        self.graph.edges.push(Edge {
            from: from.clone(),
            to: target.clone(),
            reference: r.clone(),
        });

        if let Some(pos) = self.stack.iter().position(|p| p == &target) {
            let chain = self.stack[pos..].to_vec();
            self.diag(
                from,
                r,
                Severity::Error,
                format!(
                    "\\{}{{{}}} closes an include cycle: {} -> {target}",
                    r.kind.command(),
                    r.argument,
                    chain
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" -> ")
                ),
                DiagnosticKind::Cycle { chain },
            );
            return;
        }
        if self.index.contains_key(&target) {
            return; // already discovered (diamond); the edge is recorded above
        }
        if kind == FileKind::Tex && self.stack.len() >= MAX_DEPTH {
            self.diag(
                from,
                r,
                Severity::Error,
                format!(
                    "\\{}{{{}}}: nesting deeper than {MAX_DEPTH} files",
                    r.kind.command(),
                    r.argument
                ),
                DiagnosticKind::DepthExceeded { target },
            );
            return;
        }
        match loaded {
            Loaded::Ok(file) => {
                if kind == FileKind::Tex {
                    self.visit_loaded(file);
                } else {
                    if kind == FileKind::Bibliography && file.text.is_none() {
                        self.diag(
                            from,
                            r,
                            Severity::Warning,
                            format!("{target} is not valid UTF-8"),
                            DiagnosticKind::InvalidUtf8 {
                                path: target.clone(),
                            },
                        );
                    }
                    self.add_file(file);
                }
            }
            Loaded::Missing => {
                // Raced with an external deletion between exists() and read().
                self.diag(
                    from,
                    r,
                    Severity::Error,
                    format!(
                        "\\{}{{{}}}: {target} disappeared during discovery",
                        r.kind.command(),
                        r.argument
                    ),
                    DiagnosticKind::MissingFile {
                        target: r.argument.clone(),
                        tried: vec![target],
                    },
                );
            }
            Loaded::Error(e) => {
                self.diag(
                    from,
                    r,
                    Severity::Error,
                    format!("{target} could not be read: {e}"),
                    DiagnosticKind::ReadError {
                        path: target,
                        message: e.to_string(),
                    },
                );
            }
        }
    }
}

/// Candidate paths for a reference, in the order TeX-like tools try them.
pub fn candidates(kind: ReferenceKind, base: &ProjectPath) -> (FileKind, Vec<ProjectPath>) {
    match kind {
        ReferenceKind::Input | ReferenceKind::Include => {
            if base.extension() == Some("tex") {
                (FileKind::Tex, vec![base.clone()])
            } else {
                (
                    FileKind::Tex,
                    vec![base.with_appended_extension("tex"), base.clone()],
                )
            }
        }
        ReferenceKind::Bibliography => {
            if base.extension() == Some("bib") {
                (FileKind::Bibliography, vec![base.clone()])
            } else {
                (
                    FileKind::Bibliography,
                    vec![base.with_appended_extension("bib")],
                )
            }
        }
        ReferenceKind::AddBibResource => {
            if base.extension().is_some() {
                (FileKind::Bibliography, vec![base.clone()])
            } else {
                (
                    FileKind::Bibliography,
                    vec![base.with_appended_extension("bib")],
                )
            }
        }
        ReferenceKind::IncludeGraphics => {
            let has_known = base
                .extension()
                .is_some_and(|e| GRAPHIC_EXTENSIONS.iter().any(|g| g.eq_ignore_ascii_case(e)));
            if has_known {
                (FileKind::Graphic, vec![base.clone()])
            } else {
                (
                    FileKind::Graphic,
                    GRAPHIC_EXTENSIONS
                        .iter()
                        .map(|e| base.with_appended_extension(e))
                        .collect(),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Issue #45 finding 3, unit-tested directly against
    /// `resolve_via_directory_listing` rather than through the full
    /// `ProjectGraph::discover` pipeline. `Discovery::resolve_existing` tries
    /// a literal lookup first and only falls back to a directory listing
    /// when that literal lookup fails -- which it always does on ext4 for a
    /// spelling that doesn't match what's on disk, but *not* on APFS, whose
    /// own normalization-insensitive lookup already succeeds for either
    /// spelling. That asymmetry is exactly how this regressed unnoticed on
    /// macOS, so an end-to-end test run on this (macOS) machine cannot, by
    /// itself, prove the fallback path works -- it would pass whether or not
    /// this function existed at all. Calling `resolve_via_directory_listing`
    /// directly sidesteps that: it deterministically exercises the exact
    /// fallback code that ext4 depends on, regardless of what the host
    /// filesystem's own lookup semantics happen to be for this pair of
    /// spellings.
    #[test]
    fn directory_listing_fallback_finds_nfd_reference_against_nfc_file() {
        let dir = std::env::temp_dir().join(format!(
            "flashtex-graph-nfc-nfd-unit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let nfc_stem = "caf\u{e9}"; // "café", 'é' precomposed (NFC) -- the only file written to disk
        let nfd_stem = "cafe\u{301}"; // "café", 'e' + combining acute (NFD) -- how it's looked up
        fs::write(dir.join(format!("{nfc_stem}.tex")), "Cafe content.").unwrap();

        let overlay = Overlay::default();
        let project_root = ProjectRoot::open(&dir).unwrap();
        let discovery = Discovery {
            root: dir.clone(),
            project_root,
            overlay: &overlay,
            graph: ProjectGraph {
                root: dir.clone(),
                entry: ProjectPath::normalize("main.tex").unwrap(),
                files: Vec::new(),
                edges: Vec::new(),
                diagnostics: Vec::new(),
            },
            index: BTreeMap::new(),
            stack: Vec::new(),
        };

        let nfd_candidate = ProjectPath::normalize(&format!("{nfd_stem}.tex")).unwrap();
        let resolved = discovery
            .resolve_via_directory_listing(&nfd_candidate)
            .expect(
                "directory listing must find the on-disk NFC file for an NFD-spelled candidate",
            );
        assert_eq!(
            resolved.as_str(),
            format!("{nfc_stem}.tex"),
            "the resolved path must carry the on-disk (NFC) raw bytes, not the NFD spelling it was looked up with"
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
