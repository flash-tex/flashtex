//! The project closure: the entry file and every `\input`/`\include` it
//! reaches, resolved from the project root by project-files' graph
//! discovery (rooted reads through a directory handle, `O_NOFOLLOW` at
//! every component, so no include escapes the root through `..` or a
//! symlink). The compiler resolves the same references again by
//! project-relative path when it parses the documents.
//!
//! With a `flashtex.toml` (docs/user/project-manifest.md) the closure grows
//! by the manifest's `texinputs`: every `.sty`/`.cls`/`.tex`/`.bib`/`.def`/
//! `.clo` file directly in each listed directory, appended after the entry
//! closure in manifest order (files sorted by name within a directory), so
//! the compiler's package/class resolver finds `name.sty` in the document
//! set without any file access of its own. A directory inside the root
//! keeps its real project-relative paths; one the manifest explicitly
//! places outside the root (`../shared-macros`) is mounted at the virtual
//! `texinputs/<index>/<file>` and read through its own rooted handle, so
//! the symlink rules hold there too. Without a manifest nothing here
//! changes: the closure is exactly the graph's.

use std::path::{Path, PathBuf};

use flashtex_project_files::{DiagnosticKind, ProjectGraph, ProjectPath, ProjectRoot, Severity, DEFAULT_READ_LIMIT};
use flashtex_project_manifest::{is_texinput_file, Loaded, Manifest, TexInputLocation};

/// One source document, as runtime-v1 carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub path: String,
    pub text: String,
}

/// A diagnostic from discovery (a missing or unreadable include, a path
/// escaping the root, a cycle, a manifest key this version does not
/// know), attributed like the engine's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDiagnostic {
    pub path: String,
    pub start_byte: Option<usize>,
    pub end_byte: Option<usize>,
    pub error: bool,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Project {
    /// Absolute project root (every path below is relative to it).
    pub root: PathBuf,
    /// The entry file, project-relative (`main.tex`, `paper/main.tex`).
    pub entry: String,
    /// The entry file as the caller should name it in output paths: what
    /// the user typed when they named a file (so `main.pdf` lands next to
    /// `main.tex` exactly as before), the derived absolute path otherwise.
    pub entry_file: PathBuf,
    pub documents: Vec<Document>,
    pub diagnostics: Vec<ProjectDiagnostic>,
    /// Every file in the closure (documents, bibliographies, graphics,
    /// `texinputs` files inside the root), project-relative, for `watch`.
    pub files: Vec<String>,
    /// Files outside the root that `watch` must also poll: the manifest
    /// itself and every file from an outside `texinputs` directory.
    pub outside_files: Vec<PathBuf>,
    /// The manifest that governed this load, if one was found.
    pub manifest: Option<PathBuf>,
    /// `[project] output` resolved against the manifest's directory.
    pub output_dir: Option<PathBuf>,
}

/// What the user pointed `flashtex` at: a file, a directory, or nothing
/// (the current directory).
#[derive(Debug, Clone)]
pub struct Input {
    /// The entry document.
    pub entry: PathBuf,
    /// The entry as the user typed it, when they named a file.
    pub typed: Option<PathBuf>,
    /// The manifest found for it (walking up from the entry's directory).
    pub manifest: Loaded,
    /// The directory the manifest lives in (defaults resolve against it).
    pub manifest_dir: Option<PathBuf>,
}

/// Resolves `arg` — a `.tex` file, a directory, or `None` for the current
/// directory — to the entry document and its manifest. A directory (or no
/// argument) needs the manifest's `[project] entry`, or exactly one `.tex`
/// file in the directory; a file is always the entry, and the manifest
/// only contributes `texinputs` and `output`. Usage errors (exit 2).
pub fn resolve(arg: Option<&Path>) -> Result<Input, String> {
    let given = arg.map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
    let meta = std::fs::metadata(&given).map_err(|e| format!("cannot read {}: {e}", given.display()))?;
    if meta.is_file() {
        let abs = std::fs::canonicalize(&given).map_err(|e| format!("cannot read {}: {e}", given.display()))?;
        let dir = abs.parent().map(Path::to_path_buf).ok_or_else(|| format!("{} has no parent directory", given.display()))?;
        let (manifest, manifest_dir) = load_manifest(&dir)?;
        return Ok(Input { entry: abs, typed: Some(given), manifest, manifest_dir });
    }
    if !meta.is_dir() {
        return Err(format!("{} is neither a file nor a directory", given.display()));
    }
    let dir = std::fs::canonicalize(&given).map_err(|e| format!("cannot open {}: {e}", given.display()))?;
    let (manifest, manifest_dir) = load_manifest(&dir)?;
    if let (Some(name), Some(mdir)) = (manifest.manifest.project.entry.as_deref(), manifest_dir.as_deref()) {
        // `entry` is relative to the manifest's directory, not to the
        // directory the user named (which may be a subdirectory of it).
        let entry = mdir.join(name);
        if !entry.is_file() {
            return Err(format!(
                "{}: [project] entry {name:?} is not a file under {}",
                manifest.found.as_deref().map_or(String::new(), |p| p.display().to_string()),
                mdir.display()
            ));
        }
        let entry = std::fs::canonicalize(&entry).map_err(|e| format!("cannot read {}: {e}", entry.display()))?;
        return Ok(Input { entry, typed: None, manifest, manifest_dir });
    }
    let mut tex: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("cannot list {}: {e}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "tex") && p.is_file())
        .collect();
    tex.sort();
    match tex.as_slice() {
        [one] => Ok(Input { entry: one.clone(), typed: None, manifest, manifest_dir }),
        [] => Err(format!(
            "{} has no .tex file; name the entry (`flashtex build main.tex`) or add `[project] entry` to {}",
            given.display(),
            flashtex_project_manifest::FILE_NAME
        )),
        many => Err(format!(
            "{} has {} .tex files ({}); name the entry or set `[project] entry` in {}",
            given.display(),
            many.len(),
            many.iter().filter_map(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).collect::<Vec<_>>().join(", "),
            flashtex_project_manifest::FILE_NAME
        )),
    }
}

/// `Manifest::locate` from `dir` and `Manifest::load` of what it finds; a
/// syntax error in the manifest is a usage error because nothing past it
/// can be trusted.
fn load_manifest(dir: &Path) -> Result<(Loaded, Option<PathBuf>), String> {
    match Manifest::locate(dir) {
        None => Ok((Loaded::default(), None)),
        Some(path) => {
            let loaded = Manifest::load(&path).map_err(|e| e.to_string())?;
            let mdir = path.parent().map(Path::to_path_buf);
            Ok((loaded, mdir))
        }
    }
}

/// Discovers the closure of `input`. The root is `project_root` when
/// given, else the manifest's directory when there is a manifest (so an
/// entry in `paper/` still sees `styles/` next to the manifest), else the
/// entry's directory — exactly as before manifests existed. Errors are
/// usage errors (exit 2): an entry outside the root, an unreadable root.
pub fn load(input: &Input, project_root: Option<&Path>) -> Result<Project, String> {
    let main_abs = &input.entry;
    let root = match project_root {
        Some(r) => std::fs::canonicalize(r).map_err(|e| format!("cannot open project root {}: {e}", r.display()))?,
        None => match &input.manifest_dir {
            Some(d) => d.clone(),
            None => main_abs.parent().map(Path::to_path_buf).ok_or_else(|| format!("{} has no parent directory", main_abs.display()))?,
        },
    };
    let rel = main_abs
        .strip_prefix(&root)
        .map_err(|_| format!("{} is not inside the project root {}", main_abs.display(), root.display()))?;
    let rel = rel.to_str().ok_or_else(|| format!("{} is not valid UTF-8", rel.display()))?;
    let entry = ProjectPath::normalize(rel).map_err(|e| format!("{rel}: {e}"))?;
    let graph = ProjectGraph::discover(&root, &entry).map_err(|e| e.to_string())?;
    let mut documents: Vec<Document> = graph
        .documents()
        .into_iter()
        .map(|d| Document { path: d.path, text: d.text })
        .collect();
    let mut diagnostics: Vec<ProjectDiagnostic> = graph
        .diagnostics()
        .iter()
        .map(|d| ProjectDiagnostic {
            path: d.path.as_str().to_string(),
            start_byte: d.span.map(|s| s.start),
            end_byte: d.span.map(|s| s.end),
            error: d.severity == Severity::Error,
            code: match d.kind {
                DiagnosticKind::MissingFile { .. } => "missing_file",
                DiagnosticKind::InvalidPath { .. } => "invalid_path",
                DiagnosticKind::EscapesRootViaSymlink { .. } => "path_escapes_root",
                DiagnosticKind::Cycle { .. } => "include_cycle",
                DiagnosticKind::UnresolvableReference { .. } => "unresolved_reference",
                DiagnosticKind::InvalidUtf8 { .. } => "not_utf8",
                DiagnosticKind::ReadError { .. } => "read_error",
                DiagnosticKind::DepthExceeded { .. } => "include_depth",
            },
            message: d.message.clone(),
        })
        .collect();
    let mut files: Vec<String> = graph.all_paths().map(|p| p.as_str().to_string()).collect();
    let mut outside_files = Vec::new();

    let manifest_name = input.manifest.found.as_deref().and_then(Path::file_name).map_or_else(
        || flashtex_project_manifest::FILE_NAME.to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    for w in &input.manifest.warnings {
        diagnostics.push(ProjectDiagnostic {
            path: manifest_name.clone(),
            start_byte: None,
            end_byte: None,
            error: false,
            code: "manifest_unknown_key",
            message: w.to_string(),
        });
    }
    if let (Some(found), Some(mdir)) = (&input.manifest.found, &input.manifest_dir) {
        outside_files.push(found.clone());
        let mut seen: std::collections::BTreeSet<String> = documents.iter().map(|d| d.path.clone()).collect();
        for t in input.manifest.manifest.texinputs(mdir) {
            let key = format!("project.texinputs[{}]", t.index);
            let diag = |message: String| ProjectDiagnostic {
                path: manifest_name.clone(),
                start_byte: None,
                end_byte: None,
                error: false,
                code: "manifest_texinputs",
                message: format!("{key} = {:?}: {message}", t.raw),
            };
            match &t.location {
                TexInputLocation::Invalid(why) => diagnostics.push(diag(why.clone())),
                TexInputLocation::Inside(dir) => {
                    // `dir` is relative to the manifest's directory; the
                    // project root may be an explicit parent of it, so
                    // re-express it against the root before the rooted read.
                    let abs = mdir.join(dir);
                    let Ok(rel) = abs.strip_prefix(&root) else {
                        diagnostics.push(diag(format!("{} is outside the project root {}", abs.display(), root.display())));
                        continue;
                    };
                    let Some(rel) = rel.to_str().and_then(|r| ProjectPath::normalize(r).ok()) else {
                        diagnostics.push(diag("not a valid project path".into()));
                        continue;
                    };
                    let Some(root_handle) = open_root(&root, &mut diagnostics, &diag) else { continue };
                    for name in list_texinput_files(&abs, &mut diagnostics, &diag) {
                        let path = rel.with_appended_name(&name);
                        if !seen.insert(path.as_str().to_string()) {
                            continue;
                        }
                        match root_handle.read_text(&path, DEFAULT_READ_LIMIT) {
                            Ok(Some((text, _))) => {
                                files.push(path.as_str().to_string());
                                documents.push(Document { path: path.as_str().to_string(), text });
                            }
                            Ok(None) => {}
                            Err(e) => diagnostics.push(diag(format!("{path}: {e}"))),
                        }
                    }
                }
                TexInputLocation::Outside(dir) => {
                    let Some(handle) = open_root(dir, &mut diagnostics, &diag) else { continue };
                    let virtual_dir = flashtex_project_manifest::outside_virtual_dir(t.index);
                    for name in list_texinput_files(dir, &mut diagnostics, &diag) {
                        let Ok(leaf) = ProjectPath::normalize(&name) else { continue };
                        let path = format!("{virtual_dir}/{name}");
                        if !seen.insert(path.clone()) {
                            continue;
                        }
                        match handle.read_text(&leaf, DEFAULT_READ_LIMIT) {
                            Ok(Some((text, _))) => {
                                outside_files.push(dir.join(&name));
                                documents.push(Document { path, text });
                            }
                            Ok(None) => {}
                            Err(e) => diagnostics.push(diag(format!("{}: {e}", dir.join(&name).display()))),
                        }
                    }
                }
            }
        }
    }
    let output_dir = input.manifest_dir.as_deref().and_then(|d| input.manifest.manifest.output_dir(d));
    Ok(Project {
        root,
        entry: entry.as_str().to_string(),
        entry_file: input.typed.clone().unwrap_or_else(|| main_abs.clone()),
        documents,
        diagnostics,
        files,
        outside_files,
        manifest: input.manifest.found.clone(),
        output_dir,
    })
}

trait AppendName {
    fn with_appended_name(&self, name: &str) -> ProjectPath;
}

impl AppendName for ProjectPath {
    /// `styles` + `x.sty` → `styles/x.sty`; `name` is a listed directory
    /// entry, so it has no separator and normalizes as one segment.
    fn with_appended_name(&self, name: &str) -> ProjectPath {
        ProjectPath::normalize(&format!("{}/{name}", self.as_str())).unwrap_or_else(|_| self.clone())
    }
}

/// Opens `dir` as a rooted handle (refusing a symlinked or missing
/// directory) or records why not.
fn open_root(dir: &Path, diagnostics: &mut Vec<ProjectDiagnostic>, diag: &dyn Fn(String) -> ProjectDiagnostic) -> Option<ProjectRoot> {
    match ProjectRoot::open(dir) {
        Ok(h) => Some(h),
        Err(e) => {
            diagnostics.push(diag(format!("cannot open {}: {e}", dir.display())));
            None
        }
    }
}

/// The `texinputs` files directly in `dir`, by name: regular files only
/// (symlinks are skipped here and would be refused by the rooted read
/// anyway), no recursion — a `texinputs` entry names one directory, as
/// `TEXINPUTS` does without `//`.
fn list_texinput_files(dir: &Path, diagnostics: &mut Vec<ProjectDiagnostic>, diag: &dyn Fn(String) -> ProjectDiagnostic) -> Vec<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            diagnostics.push(diag(format!("cannot list {}: {e}", dir.display())));
            return Vec::new();
        }
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| is_texinput_file(n))
        .collect();
    names.sort();
    names
}
