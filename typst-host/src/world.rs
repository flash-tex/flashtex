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

/// Open a project file for writing (`buffers`, `edits`: spec §6.3) without
/// ever following a symlink. [`confine`] is right for reading but not for
/// writing: for a path that does not exist it resolves only the parent, and
/// a write would then follow a dangling final symlink out of the root.
///
/// Here the path is walked from the root with `openat`, every directory
/// opened `O_NOFOLLOW | O_DIRECTORY` and the file `O_NOFOLLOW`, so a symlink
/// anywhere under the root (the final component or a directory) is refused,
/// including one swapped in after any earlier check. As a second guard the
/// opened descriptor's own path is checked to lie under the root before the
/// caller truncates or writes anything. `create`: create the file if it is
/// missing (a buffer); otherwise it must exist (an edit). The file is opened
/// read-write and not truncated.
pub fn open_for_write(root: &Path, rel: &Path, create: bool) -> Result<std::fs::File, String> {
    use std::ffi::CString;
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;

    let names: Vec<&std::ffi::OsStr> = rel
        .components()
        .map(|c| match c {
            std::path::Component::Normal(n) => Ok(n),
            _ => Err(format!(
                "{} is not a path inside the project",
                rel.display()
            )),
        })
        .collect::<Result<_, _>>()?;
    let Some((file, dirs)) = names.split_last() else {
        return Err("empty path".into());
    };
    let refuse = |what: &str, e: std::io::Error| {
        if e.raw_os_error() == Some(libc::ELOOP) || e.raw_os_error() == Some(libc::ENOTDIR) {
            format!("{} is refused: {what} is a symlink or not a directory (the host never writes through a symlink)", rel.display())
        } else {
            format!("{}: {e}", rel.display())
        }
    };
    let cstr = |s: &std::ffi::OsStr| {
        CString::new(s.as_bytes()).map_err(|_| "a NUL byte in the path".to_string())
    };
    let open_at =
        |dir: libc::c_int, name: &CString, flags: libc::c_int| -> std::io::Result<OwnedFd> {
            // SAFETY: a valid directory descriptor (or AT_FDCWD for the absolute
            // root) and a NUL-terminated name; the result is owned exactly once.
            let fd = unsafe {
                libc::openat(
                    dir,
                    name.as_ptr(),
                    flags | libc::O_CLOEXEC,
                    0o644 as libc::c_uint,
                )
            };
            if fd < 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(unsafe { OwnedFd::from_raw_fd(fd) })
            }
        };
    use std::os::fd::AsRawFd;
    let root_c = cstr(root.as_os_str())?;
    let mut dir = open_at(libc::AT_FDCWD, &root_c, libc::O_RDONLY | libc::O_DIRECTORY)
        .map_err(|e| format!("root {}: {e}", root.display()))?;
    for d in dirs {
        let name = cstr(d)?;
        dir = open_at(
            dir.as_raw_fd(),
            &name,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
        )
        .map_err(|e| refuse(&d.to_string_lossy(), e))?;
    }
    let name = cstr(file)?;
    let mut flags = libc::O_RDWR | libc::O_NOFOLLOW;
    if create {
        flags |= libc::O_CREAT;
    }
    let fd =
        open_at(dir.as_raw_fd(), &name, flags).map_err(|e| refuse(&file.to_string_lossy(), e))?;
    let f = std::fs::File::from(fd);
    let md = f.metadata().map_err(|e| e.to_string())?;
    if !md.is_file() {
        return Err(format!("{} is not a regular file", rel.display()));
    }
    // A hard link shares its inode with a name that may lie outside the
    // root; writing it would change that file too.
    if std::os::unix::fs::MetadataExt::nlink(&md) > 1 {
        return Err(format!(
            "{} is refused: it has {} hard links (the host never writes a file linked from elsewhere)",
            rel.display(),
            std::os::unix::fs::MetadataExt::nlink(&md)
        ));
    }
    let real = fd_path(&f)?;
    if !real.starts_with(root) {
        return Err(format!(
            "{} leaves the project root ({})",
            rel.display(),
            real.display()
        ));
    }
    Ok(f)
}

/// The path the kernel has for an open descriptor.
fn fd_path(f: &std::fs::File) -> Result<PathBuf, String> {
    use std::os::fd::AsRawFd;
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let mut buf = vec![0u8; libc::PATH_MAX as usize];
        // SAFETY: F_GETPATH writes at most MAXPATHLEN (= PATH_MAX) bytes.
        if unsafe { libc::fcntl(f.as_raw_fd(), libc::F_GETPATH, buf.as_mut_ptr()) } < 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let n = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        Ok(PathBuf::from(std::ffi::OsStr::from_bytes(&buf[..n])))
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        std::fs::read_link(format!("/proc/self/fd/{}", f.as_raw_fd())).map_err(|e| e.to_string())
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        let _ = f;
        Err("no descriptor-path query on this OS".to_string())
    }
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

    /// Writes never follow a symlink: not a dangling final component (the
    /// reviewer's `notes.typ -> <outside>/escaped.txt`), not an existing one,
    /// not a symlinked directory; regular files inside the root still work.
    #[test]
    fn writes_never_follow_symlinks() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("ftth-wconfine-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("proj");
        let outside = dir.join("outside");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let root = root.canonicalize().unwrap();
        let outside = outside.canonicalize().unwrap();
        std::fs::write(outside.join("victim.txt"), "keep").unwrap();
        let link = |to: &Path, at: &str| std::os::unix::fs::symlink(to, root.join(at)).unwrap();
        link(&outside.join("escaped.txt"), "notes.typ"); // dangling
        link(&outside.join("victim.txt"), "victim.typ"); // existing target
        link(&outside, "out"); // a symlinked directory
        link(&root.join("sub"), "inner"); // a symlink even inside the root

        for (rel, create) in [
            ("notes.typ", true),
            ("victim.typ", true),
            ("victim.typ", false),
            ("out/new.typ", true),
            ("out/victim.txt", false),
            ("inner/x.typ", true),
        ] {
            let r = open_for_write(&root, Path::new(rel), create);
            assert!(r.is_err(), "{rel} (create {create}) was opened for writing");
        }
        assert!(
            !outside.join("escaped.txt").exists(),
            "the dangling link was followed"
        );
        assert!(
            !outside.join("new.typ").exists(),
            "the symlinked directory was followed"
        );
        assert_eq!(
            std::fs::read_to_string(outside.join("victim.txt")).unwrap(),
            "keep"
        );
        assert!(!root.join("sub/x.typ").exists());
        for rel in ["../x.typ", "/etc/x", "a/../../x"] {
            assert!(
                open_for_write(&root, Path::new(rel), true).is_err(),
                "{rel}"
            );
        }
        assert!(open_for_write(&root, Path::new("missing.typ"), false).is_err());

        // A hard link to a file outside the root is refused, create or not.
        std::fs::write(outside.join("hard.txt"), "keep").unwrap();
        std::fs::hard_link(outside.join("hard.txt"), root.join("hard.typ")).unwrap();
        for create in [true, false] {
            let err = open_for_write(&root, Path::new("hard.typ"), create).unwrap_err();
            assert!(err.contains("hard links"), "{err}");
        }
        assert_eq!(
            std::fs::read_to_string(outside.join("hard.txt")).unwrap(),
            "keep"
        );

        let mut f = open_for_write(&root, Path::new("sub/ok.typ"), true).unwrap();
        f.write_all(b"fine").unwrap();
        drop(f);
        assert_eq!(
            std::fs::read_to_string(root.join("sub/ok.typ")).unwrap(),
            "fine"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
