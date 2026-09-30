//! The host's Typst `World`: one project root, files read from disk and
//! confined to it, fonts from files, no packages yet.
//!
//! * **Confinement** (DESIGN.md §15.2, "File access"): every path is
//!   canonicalised and must stay inside the canonical project root, so a
//!   symlink cannot leave it (typst#5454 is open upstream; this closes it for
//!   us, as §4.5 does for LaTeX).
//! * **Packages** (`@preview/...`) are refused with a located error: the
//!   FlashTeX package lock, offline mode and first-use consent (§15.2) come
//!   with T1, and until then the host never touches the network.
//! * **Fonts** are files (`--font-path`, and the system's when enabled);
//!   nothing is embedded (§15.2).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook, FontInfo};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::{FontPath, FontStore};

/// Where the host finds fonts.
#[derive(Clone, Debug, Default)]
pub struct FontOptions {
    /// Directories scanned recursively, in order (earlier wins a tie).
    pub paths: Vec<PathBuf>,
    /// Also scan the operating system's font directories.
    pub system: bool,
}

/// The fonts, loaded once per process and shared by every document.
pub struct Fonts {
    store: FontStore,
    /// For each font index: the file and face it came from.
    files: Vec<(PathBuf, u32)>,
}

impl Fonts {
    pub fn load(opts: &FontOptions) -> Fonts {
        let mut entries: Vec<(FontPath, FontInfo)> = Vec::new();
        for p in &opts.paths {
            entries.extend(typst_kit::fonts::scan(p));
        }
        if opts.system {
            entries.extend(typst_kit::fonts::system());
        }
        let files = entries
            .iter()
            .map(|(p, _)| (p.path.clone(), p.index))
            .collect();
        let mut store = FontStore::new();
        store.extend(entries);
        Fonts { store, files }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// A file id for a path in the project (`/`-separated, relative to the root).
pub fn project_file(path: &str) -> Result<FileId, String> {
    let vp = VirtualPath::new(path).map_err(|e| format!("{path}: {e}"))?;
    Ok(RootedPath::new(VirtualRoot::Project, vp).intern())
}

pub struct HostWorld<'f> {
    root: PathBuf,
    main: FileId,
    library: LazyHash<Library>,
    fonts: &'f Fonts,
    /// Which font file each loaded `Font` came from (for `FONT.file`).
    loaded: Mutex<HashMap<Font, usize>>,
    /// Parsed sources, kept across compiles so edits reparse incrementally.
    sources: Mutex<HashMap<FileId, Source>>,
    /// Raw files, read once per compile.
    files: Mutex<HashMap<FileId, FileResult<Bytes>>>,
}

impl<'f> HostWorld<'f> {
    /// `root` must be an existing directory; `main` a path inside it.
    pub fn new(root: &Path, main: &str, fonts: &'f Fonts) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|e| format!("root {}: {e}", root.display()))?;
        if !root.is_dir() {
            return Err(format!("root {} is not a directory", root.display()));
        }
        let main = project_file(main)?;
        Ok(HostWorld {
            root,
            main,
            library: LazyHash::new(Library::builder().build()),
            fonts,
            loaded: Mutex::new(HashMap::new()),
            sources: Mutex::new(HashMap::new()),
            files: Mutex::new(HashMap::new()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Forget the raw files read by the last compile, so the next one sees
    /// the disk as it is now. Parsed sources are kept and updated in place
    /// (`Source::replace` reparses only what changed).
    pub fn reset(&mut self) {
        self.files.get_mut().unwrap().clear();
    }

    /// The absolute path of a project file (for `SOURCES` and diagnostics).
    pub fn path_of(&self, id: FileId) -> Option<PathBuf> {
        if !matches!(id.root(), VirtualRoot::Project) {
            return None;
        }
        Some(self.root.join(id.vpath().get_without_slash()))
    }

    /// The font file (and face) a loaded font came from.
    pub fn font_file(&self, font: &Font) -> Option<(PathBuf, u32)> {
        let i = *self.loaded.lock().unwrap().get(font)?;
        self.fonts.files.get(i).cloned()
    }

    /// Resolve a project path, refusing anything outside the root (after
    /// following symlinks). For a path that does not exist yet (a buffer the
    /// editor has not saved), its parent directory must resolve inside.
    pub fn confine(&self, rel: &Path) -> Result<PathBuf, String> {
        confine(&self.root, rel)
    }

    fn read(&self, id: FileId) -> FileResult<Bytes> {
        if let VirtualRoot::Package(spec) = id.root() {
            return Err(FileError::Other(Some(
                format!(
                    "package {spec} is not available: this Typst host has no package support yet \
                     (the FlashTeX package lock, offline mode and consent come first; DESIGN.md §15.2)"
                )
                .into(),
            )));
        }
        let rel = Path::new(id.vpath().get_without_slash());
        let path = self.confine(rel).map_err(|_| FileError::AccessDenied)?;
        let data = std::fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
        Ok(Bytes::new(data))
    }
}

/// See [`HostWorld::confine`].
pub fn confine(root: &Path, rel: &Path) -> Result<PathBuf, String> {
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!(
            "{} is not a path inside the project",
            rel.display()
        ));
    }
    let full = root.join(rel);
    let resolved = match full.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            let parent = full.parent().ok_or("no parent directory")?;
            let parent = parent
                .canonicalize()
                .map_err(|e| format!("{}: {e}", parent.display()))?;
            parent.join(full.file_name().ok_or("no file name")?)
        }
    };
    if !resolved.starts_with(root) {
        return Err(format!("{} leaves the project root", rel.display()));
    }
    Ok(resolved)
}

impl World for HostWorld<'_> {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.store.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        let bytes = self.file(id)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| FileError::InvalidUtf8)?;
        // Typst reads a leading BOM as text; strip it as typst-cli does.
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut sources = self.sources.lock().unwrap();
        match sources.get_mut(&id) {
            Some(s) => {
                if s.text() != text {
                    s.replace(text);
                }
                Ok(s.clone())
            }
            None => {
                let s = Source::new(id, text.to_string());
                sources.insert(id, s.clone());
                Ok(s)
            }
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let mut files = self.files.lock().unwrap();
        files.entry(id).or_insert_with(|| self.read(id)).clone()
    }

    fn font(&self, index: usize) -> Option<Font> {
        let f = self.fonts.store.font(index)?;
        self.loaded
            .lock()
            .unwrap()
            .entry(f.clone())
            .or_insert(index);
        Some(f)
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        // UTC plus the requested offset (typst-cli's rule), from the system
        // clock, without a date library.
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs() as i64;
        let hours = offset.map(|d| d.hours() as i64).unwrap_or(0);
        let days = (secs + hours * 3600).div_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        Datetime::from_ymd(y as i32, m as u8, d as u8)
    }
}

/// Days since 1970-01-01 to (year, month, day), proleptic Gregorian
/// (Howard Hinnant's `civil_from_days`).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_726), (2026, 9, 30));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn confinement_refuses_escapes() {
        let dir = std::env::temp_dir().join(format!("ftth-confine-{}", std::process::id()));
        let root = dir.join("proj");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(dir.join("secret.txt"), "x").unwrap();
        std::fs::write(root.join("ok.typ"), "x").unwrap();
        let root = root.canonicalize().unwrap();
        assert!(confine(&root, Path::new("ok.typ")).is_ok());
        assert!(confine(&root, Path::new("new.typ")).is_ok());
        assert!(confine(&root, Path::new("../secret.txt")).is_err());
        assert!(confine(&root, Path::new("/etc/passwd")).is_err());
        std::os::unix::fs::symlink(dir.join("secret.txt"), root.join("link.txt")).unwrap();
        assert!(confine(&root, Path::new("link.txt")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
