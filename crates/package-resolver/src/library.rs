//! Local libraries: `[packages] path = { mylib = "../mylib" }` names a
//! directory whose own `flashtex.toml` says `[library] name = "mylib"`.
//! Its `.sty`/`.cls`/`.def`/`.clo` files (directly in it, no recursion)
//! resolve before the cache and before any network.
//!
//! The path is classified with the manifest crate's lexical rules — the
//! same ones `[project] texinputs` obeys: relative to the manifest's
//! directory, no absolute path, no `~`, no backslash/colon/control
//! character; `..` may climb out of the project because the manifest is the
//! user's explicit configuration. A directory reached *through a symlink*
//! in those segments, a symlinked file, or anything that is not a regular
//! file is refused with a diagnostic, never read — `crates/project-files`
//! opens `texinputs` directories with the same refusals through its rooted
//! handle; this crate cannot link that crate (the CLI links a vendored
//! copy), so the check is a canonical-path comparison instead of
//! `O_NOFOLLOW`. Git URLs are out of scope: a library is a directory.

use std::fs;
use std::path::{Path, PathBuf};

use flashtex_project_manifest::{classify_texinput, Manifest, TexInputLocation, FILE_NAME};

use crate::{cache::content_version, display, ResolvedFile};

/// The document-kind files a library contributes (never `.cfg`: that is a
/// fetched package's business, and never `.tex`/`.bib`).
pub const LIBRARY_EXTENSIONS: &[&str] = &["sty", "cls", "def", "clo"];

/// One loaded library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Library {
    /// The key in `[packages] path`, which the library's own manifest must
    /// repeat as `[library] name`.
    pub name: String,
    /// The directory, as the manifest's directory joined with the entry.
    pub dir: PathBuf,
    /// Its package files, sorted by name.
    pub files: Vec<ResolvedFile>,
}

impl Library {
    /// Whether `\usepackage{name}` / `\documentclass{name}` finds a file here.
    pub fn provides(&self, name: &str) -> bool {
        self.files.iter().any(|f| f.name.rsplit_once('.').is_some_and(|(stem, ext)| stem == name && (ext == "sty" || ext == "cls")))
    }

    /// A version for the manifest's `pin` table and the sidebar: the
    /// content hash, since a directory states none.
    pub fn version(&self) -> String {
        content_version(&self.files.iter().map(|f| (f.name.clone(), f.text.as_bytes().to_vec())).collect::<Vec<_>>())
    }
}

/// Loads every `[packages] path` entry of `manifest` against
/// `manifest_dir`, in key order. A library that cannot be used is a
/// diagnostic (`packages.path.<key>: …`) and is skipped; the rest load.
pub fn load_all(manifest: &Manifest, manifest_dir: &Path) -> (Vec<Library>, Vec<String>) {
    let mut libraries = Vec::new();
    let mut diagnostics = Vec::new();
    for (key, raw) in &manifest.packages.path {
        match load(key, raw, manifest_dir) {
            Ok(lib) => libraries.push(lib),
            Err(why) => diagnostics.push(format!("packages.path.{key} = {raw:?}: {why}")),
        }
    }
    (libraries, diagnostics)
}

/// Loads the library `key` names at `raw` (relative to `manifest_dir`).
pub fn load(key: &str, raw: &str, manifest_dir: &Path) -> Result<Library, String> {
    if !crate::is_valid_name(key) {
        return Err(format!("{key:?} is not a package name"));
    }
    let dir = match classify_texinput(raw, manifest_dir) {
        TexInputLocation::Inside(rel) => manifest_dir.join(rel),
        TexInputLocation::Outside(abs) => abs,
        TexInputLocation::Invalid(why) => return Err(why),
    };
    // No symlink in the segments the user wrote: the lexical join and the
    // filesystem's own resolution must agree once the manifest directory
    // itself is canonical (it may legitimately sit under a symlinked
    // parent such as /tmp on macOS).
    let canonical_manifest = fs::canonicalize(manifest_dir).map_err(|e| format!("cannot resolve {}: {e}", display(manifest_dir)))?;
    let canonical_dir = fs::canonicalize(&dir).map_err(|e| format!("cannot open {}: {e}", display(&dir)))?;
    let expected = rejoin(&canonical_manifest, &dir, manifest_dir);
    if expected.as_deref() != Some(canonical_dir.as_path()) {
        return Err(format!("{} is reached through a symbolic link; refused", display(&dir)));
    }
    if !canonical_dir.is_dir() {
        return Err(format!("{} is not a directory", display(&dir)));
    }
    let manifest_path = dir.join(FILE_NAME);
    let loaded = Manifest::load(&manifest_path).map_err(|e| e.to_string())?;
    let Some(found) = loaded.found else {
        return Err(format!("{} has no {FILE_NAME}; a library declares `[library] name = {key:?}`", display(&dir)));
    };
    let Some(library) = loaded.manifest.library else {
        return Err(format!("{} has no [library] section; a library declares `[library] name = {key:?}`", display(&found)));
    };
    if library.name != key {
        return Err(format!("{} says [library] name = {:?}, but this manifest calls it {key:?}", display(&found), library.name));
    }
    let mut files = Vec::new();
    let mut names: Vec<(String, PathBuf)> = fs::read_dir(&dir)
        .map_err(|e| format!("cannot list {}: {e}", display(&dir)))?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok().map(|n| (n, e.path())))
        .filter(|(n, _)| n.rsplit_once('.').is_some_and(|(stem, ext)| !stem.is_empty() && LIBRARY_EXTENSIONS.contains(&ext)))
        .collect();
    names.sort();
    for (name, path) in names {
        let meta = fs::symlink_metadata(&path).map_err(|e| format!("cannot read {}: {e}", display(&path)))?;
        if meta.file_type().is_symlink() {
            return Err(format!("{} is a symbolic link; refused", display(&path)));
        }
        if !meta.is_file() {
            continue;
        }
        let bytes = fs::read(&path).map_err(|e| format!("cannot read {}: {e}", display(&path)))?;
        let text = String::from_utf8(bytes).map_err(|_| format!("{} is not UTF-8", display(&path)))?;
        files.push(ResolvedFile { name, path, text });
    }
    Ok(Library { name: key.into(), dir, files })
}

/// `dir` re-expressed against the canonical manifest directory: the
/// segments of `dir` beyond `manifest_dir`, applied lexically (`..` pops).
fn rejoin(canonical_manifest: &Path, dir: &Path, manifest_dir: &Path) -> Option<PathBuf> {
    let mut out = canonical_manifest.to_path_buf();
    let mut lexical = manifest_dir.to_path_buf();
    // `dir` is `manifest_dir` with some `..` pops and pushes (classify_texinput's
    // construction); replay them: pop while `dir` is not under `lexical`.
    let mut ups = 0;
    while !dir.starts_with(&lexical) {
        if !lexical.pop() {
            return None;
        }
        ups += 1;
    }
    for _ in 0..ups {
        if !out.pop() {
            return None;
        }
    }
    out.push(dir.strip_prefix(&lexical).ok()?);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::tmp;

    fn project_with_library(root: &Path, key: &str, library_name: &str) -> (PathBuf, PathBuf) {
        let project = root.join("project");
        let lib = root.join("lib");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&lib).unwrap();
        fs::write(lib.join("flashtex.toml"), format!("[library]\nname = \"{library_name}\"\n")).unwrap();
        fs::write(lib.join(format!("{key}.sty")), "\\ProvidesPackage{x}\n").unwrap();
        fs::write(lib.join("extra.cls"), "%cls\n").unwrap();
        fs::write(lib.join("README.md"), "no\n").unwrap();
        fs::write(lib.join("thing.tex"), "no\n").unwrap();
        (project, lib)
    }

    #[test]
    fn loads_a_library_and_diagnoses_the_rest() {
        let root = tmp("library");
        let (project, lib) = project_with_library(&root, "mylib", "mylib");
        let l = load("mylib", "../lib", &project).unwrap();
        assert_eq!(l.name, "mylib");
        assert_eq!(l.dir, lib);
        assert_eq!(l.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["extra.cls", "mylib.sty"]);
        assert!(l.provides("mylib") && l.provides("extra") && !l.provides("README") && !l.provides("thing"));
        assert_eq!(l.version().len(), 12);

        assert!(load("mylib", "/abs", &project).unwrap_err().contains("absolute"));
        assert!(load("mylib", "~/x", &project).unwrap_err().contains("absolute"));
        assert!(load("mylib", "a\\b", &project).unwrap_err().contains("forbidden"));
        assert!(load("mylib", "../nowhere", &project).unwrap_err().contains("cannot open"));
        assert!(load("mylib", "", &project).is_err());
        assert!(load("../x", "../lib", &project).unwrap_err().contains("not a package name"));
        fs::write(lib.join("flashtex.toml"), "[library]\nname = \"other\"\n").unwrap();
        assert!(load("mylib", "../lib", &project).unwrap_err().contains("calls it \"mylib\""));
        fs::write(lib.join("flashtex.toml"), "[project]\n").unwrap();
        assert!(load("mylib", "../lib", &project).unwrap_err().contains("no [library] section"));
        fs::remove_file(lib.join("flashtex.toml")).unwrap();
        assert!(load("mylib", "../lib", &project).unwrap_err().contains("has no flashtex.toml"));

        // Inside the project too.
        let inner = project.join("libs/inner");
        fs::create_dir_all(&inner).unwrap();
        fs::write(inner.join("flashtex.toml"), "[library]\nname = \"inner\"\n").unwrap();
        fs::write(inner.join("inner.sty"), "%\n").unwrap();
        let l = load("inner", "libs/inner", &project).unwrap();
        assert_eq!(l.dir, inner);
        assert_eq!(l.files.len(), 1);

        let text = "[packages]\npath = { inner = \"libs/inner\", broken = \"../nowhere\" }\n";
        let manifest = Manifest::parse(text).unwrap().manifest;
        let (libs, diags) = load_all(&manifest, &project);
        assert_eq!(libs.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(), ["inner"]);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].starts_with("packages.path.broken = \"../nowhere\": cannot open"), "{}", diags[0]);
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_refused_not_read() {
        let root = tmp("library-symlink");
        let (project, lib) = project_with_library(&root, "mylib", "mylib");
        // A symlinked file inside an otherwise fine library.
        std::os::unix::fs::symlink(root.join("outside.sty"), lib.join("linked.sty")).unwrap();
        fs::write(root.join("outside.sty"), "secret\n").unwrap();
        let err = load("mylib", "../lib", &project).unwrap_err();
        assert!(err.contains("symbolic link"), "{err}");
        fs::remove_file(lib.join("linked.sty")).unwrap();
        // A directory reached through a symlink in the written segments.
        std::os::unix::fs::symlink(&lib, root.join("via-link")).unwrap();
        let err = load("mylib", "../via-link", &project).unwrap_err();
        assert!(err.contains("symbolic link"), "{err}");
        // The real path still works, even when the project itself sits under a symlinked parent.
        assert!(load("mylib", "../lib", &project).is_ok());
        let _ = fs::remove_dir_all(&root);
    }
}
