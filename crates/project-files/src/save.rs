//! Rooted, symlink-refusing, lock-serialized atomic saves (issue #18).
//!
//! A [`ProjectRoot`] is an open directory handle. Every read and write walks
//! the normalized [`ProjectPath`] one component at a time with
//! `openat(O_NOFOLLOW)` from that handle, so no component — parent directory
//! or the file itself — may be a symlink, and nothing outside the selected
//! root is ever opened for writing. Each walked directory's `..` is compared
//! by device/inode with the handle it was opened from.
//!
//! Saves are serialized by an advisory `flock(LOCK_EX)` on
//! `<root>/.flashtex/project.lock`, held across hash check, temp write, fsync,
//! rename and directory fsync. See the README for the exact concurrency
//! contract: writers that do not take the lock are out of contract; their
//! interference is *detected* (pre-rename re-verification and post-rename
//! identity/hash verification) and reported as a conflict, not prevented.

use std::fmt;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::path::ProjectPath;
use crate::sha256::{Digest, hex, sha256};
use crate::sys;

/// Relative path of the per-project lock file.
pub const LOCK_FILE: &str = ".flashtex/project.lock";

/// Largest file `read` will load unless the caller passes a smaller limit.
pub const DEFAULT_READ_LIMIT: u64 = 64 * 1024 * 1024;

/// Proof of a completed save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveReceipt {
    pub path: ProjectPath,
    pub bytes: u64,
    pub sha256: Digest,
    /// Modification time reported by `fstat` on the renamed file.
    pub mtime: SystemTime,
    /// Device/inode of the file that now sits at `path`.
    pub identity: FileIdentity,
}

impl SaveReceipt {
    pub fn sha256_hex(&self) -> String {
        hex(&self.sha256)
    }
}

/// Device and inode number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub dev: u64,
    pub ino: u64,
}

/// A bounded, symlink-refusing read of one project file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootedRead {
    pub path: ProjectPath,
    pub bytes: Vec<u8>,
    pub sha256: Digest,
    pub mtime: SystemTime,
    pub identity: FileIdentity,
    /// Permission bits (`st_mode & 0o7777`).
    pub mode: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveConflictKind {
    /// The file exists with content different from `expected`.
    ModifiedExternally,
    /// `expected` was given but the file no longer exists.
    DeletedExternally,
    /// No `expected` hash (new file) but a file already exists there.
    AlreadyExists,
    /// The target changed between the hash check and the rename, or the
    /// renamed file was replaced/modified before post-rename verification:
    /// a writer that did not take the project lock interfered.
    ModifiedDuringSave,
}

/// Why a non-forced save was refused by content comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveConflict {
    pub path: ProjectPath,
    pub kind: SaveConflictKind,
    /// The hash the caller expected on disk (`None` for a new file); for
    /// `ModifiedDuringSave`, the hash of the bytes we wrote.
    pub ours: Option<Digest>,
    /// The hash actually observed (`None` if deleted).
    pub theirs: Option<Digest>,
    pub mtime: Option<SystemTime>,
    pub size: Option<u64>,
}

/// Why an operation was refused before touching content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// A parent directory or the file itself is a symbolic link.
    SymlinkComponent { component: String },
    /// A path component that must be a directory is not one.
    NotADirectory { component: String },
    /// The target exists but is not a regular file.
    NotARegularFile { component: String },
    /// A walked directory's `..` is not the directory it was opened from.
    EscapesRoot { component: String },
    /// The file is larger than the read limit.
    TooLarge { limit: u64, size: u64 },
    /// Another open handle (any process) holds the project lock.
    LockUnavailable { lock_path: PathBuf },
    /// This target has no rooted file operations; nothing was attempted.
    Unsupported,
}

#[derive(Debug)]
pub enum SaveError {
    Conflict(Box<SaveConflict>),
    Refused(Refused),
    Io(io::Error),
    /// `fsync` of the containing directory failed **after** the rename. The
    /// file may or may not be durable; re-read it before trusting it.
    DirectorySync(io::Error),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Conflict(c) => write!(f, "save of {} refused: {:?}", c.path, c.kind),
            SaveError::Refused(r) => write!(f, "refused: {r:?}"),
            SaveError::Io(e) => write!(f, "I/O error: {e}"),
            SaveError::DirectorySync(e) => {
                write!(
                    f,
                    "directory fsync failed after rename (durability unknown): {e}"
                )
            }
        }
    }
}

impl std::error::Error for SaveError {}

impl From<io::Error> for SaveError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::Unsupported {
            SaveError::Refused(Refused::Unsupported)
        } else {
            SaveError::Io(e)
        }
    }
}

/// What the caller believes is on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// The path should not exist yet.
    NewFile,
    /// The path should contain bytes hashing to this digest.
    Hash(Digest),
    /// Do not compare content (symlink refusal still applies).
    Any,
}

/// An open handle to the project root directory.
#[derive(Debug)]
pub struct ProjectRoot {
    path: PathBuf,
    dir: File,
}

/// The per-project advisory lock, held for the lifetime of the value.
#[derive(Debug)]
pub struct ProjectLock<'a> {
    root: &'a ProjectRoot,
    file: File,
}

impl Drop for ProjectLock<'_> {
    fn drop(&mut self) {
        sys::unlock(&self.file);
    }
}

fn identity(meta: &std::fs::Metadata) -> FileIdentity {
    FileIdentity {
        dev: meta.dev(),
        ino: meta.ino(),
    }
}

fn refused(r: Refused) -> SaveError {
    SaveError::Refused(r)
}

/// Maps an `openat(O_NOFOLLOW)` failure for `component` to the refusal it means.
fn classify_open(err: io::Error, component: &str) -> SaveError {
    if sys::errno_is(&err, sys::ELOOP) {
        refused(Refused::SymlinkComponent {
            component: component.to_string(),
        })
    } else if sys::errno_is(&err, sys::ENOTDIR) {
        refused(Refused::NotADirectory {
            component: component.to_string(),
        })
    } else {
        err.into()
    }
}

/// Current on-disk observation of a target, used for before/after comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observation {
    identity: FileIdentity,
    size: u64,
    mtime: SystemTime,
    sha256: Digest,
    mode: u32,
}

impl ProjectRoot {
    /// Opens `path` as the project root. The final component must be a real
    /// directory (not a symlink). Components *above* it are the caller's
    /// choice: a symlinked ancestor is followed on purpose (see
    /// [`sys::open_dir_nofollow`]), and the directory it leads to is pinned.
    /// Symlinks are refused only inside the root.
    pub fn open(path: &Path) -> Result<ProjectRoot, SaveError> {
        if !sys::SUPPORTED {
            return Err(refused(Refused::Unsupported));
        }
        let dir = sys::open_dir_nofollow(path).map_err(|e| {
            classify_open(
                e,
                &path
                    .file_name()
                    .map_or_else(|| "<root>".into(), |n| n.to_string_lossy()),
            )
        })?;
        Ok(ProjectRoot {
            path: path.to_path_buf(),
            dir,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the directory at [`Self::path`] is still the one this handle
    /// pinned (same device and inode, not a symlink). `Ok(false)` when the
    /// path is gone or now names something else: the root was renamed,
    /// removed or replaced after it was opened. Ancestors of the path are
    /// resolved normally, as in [`Self::open`].
    pub fn is_still_at_path(&self) -> io::Result<bool> {
        let pinned = identity(&self.dir.metadata()?);
        match std::fs::symlink_metadata(&self.path) {
            Ok(meta) => Ok(meta.is_dir() && identity(&meta) == pinned),
            Err(e) if e.kind() == io::ErrorKind::NotFound || sys::errno_is(&e, sys::ENOTDIR) => {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Walks to the directory containing `path`, refusing symlinks and
    /// verifying each `..` against the handle it was reached from. With
    /// `create`, missing intermediate directories are created (mode 0o755).
    fn walk(&self, path: &ProjectPath, create: bool) -> Result<File, SaveError> {
        let mut cur = self.dir.try_clone()?;
        let parent = path.parent_dir();
        if parent.is_empty() {
            return Ok(cur);
        }
        for component in parent.split('/') {
            let child = match sys::open_dir_at_nofollow(&cur, component) {
                Ok(f) => f,
                Err(e) if create && sys::errno_is(&e, sys::ENOENT) => {
                    match sys::mkdir_at(&cur, component, 0o755) {
                        Ok(()) => {}
                        Err(e) if sys::errno_is(&e, sys::EEXIST) => {}
                        Err(e) => return Err(e.into()),
                    }
                    sys::open_dir_at_nofollow(&cur, component)
                        .map_err(|e| classify_open(e, component))?
                }
                Err(e) => return Err(classify_open(e, component)),
            };
            let up = sys::open_at(&child, "..", sys::O_RDONLY | sys::O_DIRECTORY, 0)?;
            if identity(&up.metadata()?) != identity(&cur.metadata()?) {
                return Err(refused(Refused::EscapesRoot {
                    component: component.to_string(),
                }));
            }
            cur = child;
        }
        Ok(cur)
    }

    /// Classifies the entry `name` of `dir` with
    /// `fstatat(AT_SYMLINK_NOFOLLOW)`: nothing is followed or opened.
    /// `Ok(None)` when absent; a symlink is `SymlinkComponent` and any other
    /// non-regular entry (directory, FIFO, socket, device) `NotARegularFile`.
    fn stat_regular(dir: &File, name: &str) -> Result<Option<sys::EntryStat>, SaveError> {
        let st = match sys::stat_at_nofollow(dir, name.as_bytes()) {
            Ok(st) => st,
            Err(e) if sys::errno_is(&e, sys::ENOENT) => return Ok(None),
            Err(e) => return Err(classify_open(e, name)),
        };
        if st.is_symlink() {
            return Err(refused(Refused::SymlinkComponent {
                component: name.to_string(),
            }));
        }
        if !st.is_file() {
            return Err(refused(Refused::NotARegularFile {
                component: name.to_string(),
            }));
        }
        Ok(Some(st))
    }

    /// Opens the regular file `name` in `dir` for reading without following
    /// a symlink. `Ok(None)` when absent.
    ///
    /// The entry is classified with [`Self::stat_regular`] on the directory
    /// descriptor *before* it is opened, so a symlink, FIFO, socket or device
    /// that is already there is refused without ever being opened. The name
    /// can still be swapped between that `fstatat` and the `openat`, and
    /// POSIX has no way to close that window. For a swap in the window:
    ///
    /// - a symlink is refused by `O_NOFOLLOW` and never followed;
    /// - a FIFO open returns at once because of `O_NONBLOCK`, and the
    ///   post-open `fstat` refuses it;
    /// - a device node is opened (with `O_NONBLOCK`) and then refused by the
    ///   post-open `fstat`; nothing is read from it. Not hanging and not
    ///   having an open side effect are **best-effort** here: both depend on
    ///   the driver honouring `O_NONBLOCK`, which POSIX does not require.
    ///   (Getting a device node into the directory at all needs either the
    ///   privilege to create one or an existing node on the same filesystem
    ///   to rename or hard-link.)
    fn open_target(dir: &File, name: &str) -> Result<Option<(File, std::fs::Metadata)>, SaveError> {
        if Self::stat_regular(dir, name)?.is_none() {
            return Ok(None);
        }
        race_point!(BeforeOpen, name);
        let file = match sys::open_at(dir, name, sys::O_RDONLY | sys::O_NOFOLLOW, 0) {
            Ok(f) => f,
            Err(e) if sys::errno_is(&e, sys::ENOENT) => return Ok(None),
            Err(e) => return Err(classify_open(e, name)),
        };
        let meta = file.metadata()?;
        if !meta.is_file() {
            return Err(refused(Refused::NotARegularFile {
                component: name.to_string(),
            }));
        }
        Ok(Some((file, meta)))
    }

    fn observe(
        dir: &File,
        name: &str,
        limit: u64,
    ) -> Result<Option<(Observation, Vec<u8>)>, SaveError> {
        let Some((mut file, meta)) = Self::open_target(dir, name)? else {
            return Ok(None);
        };
        if meta.len() > limit {
            return Err(refused(Refused::TooLarge {
                limit,
                size: meta.len(),
            }));
        }
        let mut bytes = Vec::with_capacity(meta.len() as usize);
        (&mut file).take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(refused(Refused::TooLarge {
                limit,
                size: bytes.len() as u64,
            }));
        }
        let obs = Observation {
            identity: identity(&meta),
            size: meta.len(),
            mtime: meta.modified()?,
            sha256: sha256(&bytes),
            mode: meta.mode() & 0o7777,
        };
        Ok(Some((obs, bytes)))
    }

    /// Resolves `path`'s leaf to the name that actually backs it in `dir`,
    /// mirroring `Discovery::resolve_existing` / `resolve_via_directory_listing`
    /// in `graph.rs` for the save/conflict-detection path (issue #45 finding
    /// 3, save-path extension).
    ///
    /// The literal spelling (`path.file_name()`) is tried first via the
    /// existing fd-rooted, `O_NOFOLLOW` `open_target`. On a
    /// normalization-insensitive filesystem (APFS) that alone already finds
    /// an NFD-spelled reference to an NFC-spelled on-disk file. On a
    /// normalization-*sensitive* filesystem (ext4) the literal lookup for
    /// whichever spelling isn't on disk fails (`ENOENT`), so this lists the
    /// parent directory — a plain, non-fd-rooted read, the same trust level
    /// `Discovery::resolve_existing`'s literal `is_file()` check already has
    /// — and matches entries by [`ProjectPath`] identity (Unicode-NFC
    /// comparison), which is spelling-insensitive regardless of what the
    /// filesystem does.
    ///
    /// This never grants extra trust: the returned name is only ever used as
    /// the `name` argument to the existing fd-rooted, symlink-refusing
    /// `openat`/`renameat` calls the rest of `save_with` already makes,
    /// exactly like a literal candidate would be. A symlink at the literal
    /// spelling is still refused directly by `open_target` (it errors rather
    /// than reporting "missing"), and a genuinely absent target — no match
    /// under any spelling — falls back to the literal name unchanged, so
    /// `check_expected` still sees `None` and reports `DeletedExternally` /
    /// allows `NewFile` as before.
    fn resolve_target_name(&self, dir: &File, path: &ProjectPath) -> Result<String, SaveError> {
        let literal = path.file_name();
        if Self::open_target(dir, literal)?.is_some() {
            return Ok(literal.to_string());
        }
        Ok(self
            .resolve_leaf_via_directory_listing(path)
            .unwrap_or_else(|| literal.to_string()))
    }

    /// The directory-listing fallback itself, factored out so it can be unit
    /// tested directly (see `tests::directory_listing_fallback_resolves_nfd_candidate_to_nfc_name`
    /// below) without depending on `open_target`'s literal lookup, whose
    /// result varies by host filesystem: APFS's own lookup is already
    /// normalization-insensitive and would find the file before this ever
    /// runs, so calling only `resolve_target_name` on this (macOS) machine
    /// cannot, by itself, prove this fallback works -- exactly the same
    /// reasoning `graph.rs`'s `resolve_via_directory_listing` unit test
    /// documents for the discovery path this mirrors.
    fn resolve_leaf_via_directory_listing(&self, path: &ProjectPath) -> Option<String> {
        let parent_dir = path.parent_dir();
        let mut dir_os_path = self.path.clone();
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
            // Filtered here too, belt-and-suspenders: a symlink entry never
            // reaches the fd-rooted open/rename calls that follow because
            // they use this resolved name with `O_NOFOLLOW` exactly as they
            // would the literal spelling.
            if !file_type.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().into_string().ok() else {
                continue; // not valid UTF-8; cannot match a ProjectPath
            };
            let candidate_str = if parent_dir.is_empty() {
                name.clone()
            } else {
                format!("{parent_dir}/{name}")
            };
            let Ok(on_disk) = ProjectPath::normalize(&candidate_str) else {
                continue;
            };
            if &on_disk == path {
                return Some(name);
            }
        }
        None
    }

    /// Opens the regular file `path` through the rooted walk (no symlink
    /// component, nothing but a regular file opened); `Ok(None)` when absent.
    /// The watcher uses this to stat and hash tracked files.
    pub(crate) fn open_regular(
        &self,
        path: &ProjectPath,
    ) -> Result<Option<(File, std::fs::Metadata)>, SaveError> {
        let dir = self.walk(path, false)?;
        Self::open_target(&dir, path.file_name())
    }

    /// Reads `path` (at most `limit` bytes) without following any symlink.
    /// `Ok(None)` when the file does not exist.
    pub fn read(&self, path: &ProjectPath, limit: u64) -> Result<Option<RootedRead>, SaveError> {
        let dir = self.walk(path, false)?;
        Ok(
            Self::observe(&dir, path.file_name(), limit)?.map(|(o, bytes)| RootedRead {
                path: path.clone(),
                bytes,
                sha256: o.sha256,
                mtime: o.mtime,
                identity: o.identity,
                mode: o.mode,
            }),
        )
    }

    /// Reads `path` as UTF-8 text. `Ok(None)` when absent; a non-UTF-8 file
    /// is an `InvalidData` I/O error.
    pub fn read_text(
        &self,
        path: &ProjectPath,
        limit: u64,
    ) -> Result<Option<(String, RootedRead)>, SaveError> {
        let Some(read) = self.read(path, limit)? else {
            return Ok(None);
        };
        let text = std::str::from_utf8(&read.bytes)
            .map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{path} is not valid UTF-8"),
                )
            })?
            .to_string();
        Ok(Some((text, read)))
    }

    /// Takes the per-project lock (`<root>/.flashtex/project.lock`) without
    /// blocking. `Refused::LockUnavailable` if any other open handle holds it.
    pub fn lock(&self) -> Result<ProjectLock<'_>, SaveError> {
        let lock_path = ProjectPath::normalize(LOCK_FILE).expect("constant lock path");
        let dir = self.walk(&lock_path, true)?;
        let name = lock_path.file_name();
        // `O_CREAT | O_EXCL` never opens an existing entry, so a new lock file
        // is created as a regular file. An existing one is classified on the
        // directory descriptor before it is opened (see `open_target` for the
        // residual swap window the post-open `fstat` backstops).
        let create = sys::O_RDWR | sys::O_CREAT | sys::O_EXCL | sys::O_NOFOLLOW;
        let file = match sys::open_at(&dir, name, create, 0o644) {
            Ok(f) => f,
            Err(e) if sys::errno_is(&e, sys::EEXIST) => {
                if Self::stat_regular(&dir, name)?.is_none() {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "project lock file disappeared while it was being opened",
                    )
                    .into());
                }
                sys::open_at(&dir, name, sys::O_RDWR | sys::O_NOFOLLOW, 0)
                    .map_err(|e| classify_open(e, name))?
            }
            Err(e) => return Err(classify_open(e, name)),
        };
        if !file.metadata()?.is_file() {
            return Err(refused(Refused::NotARegularFile {
                component: name.to_string(),
            }));
        }
        if !sys::try_lock_exclusive(&file)? {
            return Err(refused(Refused::LockUnavailable {
                lock_path: self.path.join(LOCK_FILE),
            }));
        }
        Ok(ProjectLock { root: self, file })
    }

    /// Lock, save, unlock. See [`ProjectLock::save`].
    pub fn save(
        &self,
        path: &ProjectPath,
        bytes: &[u8],
        expected: Expected,
        force: bool,
    ) -> Result<SaveReceipt, SaveError> {
        self.lock()?.save(path, bytes, expected, force)
    }

    /// Lock, remove, unlock. See [`ProjectLock::remove`].
    pub fn remove(&self, path: &ProjectPath) -> Result<bool, SaveError> {
        self.lock()?.remove(path)
    }
}

/// Deterministic race-test support. **Not API**, and **not compiled into
/// normal builds**: the module and every firing point exist only under
/// `cfg(test)` or the non-default `race-hook` cargo feature, which this
/// crate's own integration tests enable through a self dev-dependency. A
/// release build of the library (or of any crate depending on it without
/// that feature) contains no hook code.
///
/// Every check-then-use pair in this module that POSIX cannot make atomic
/// calls [`fire`] in the exact window between the check and the syscall it
/// guards. A test installs a callback for its own thread, swaps the entry
/// from inside the callback, and then asserts what the operation did, so
/// the security property is tested at the worst interleaving every run
/// instead of by timing. The hook is thread-local: a hook installed by one
/// test never fires in another thread's operation.
#[cfg(any(test, feature = "race-hook"))]
#[doc(hidden)]
pub mod race_hook {
    use std::cell::RefCell;
    use std::rc::Rc;

    /// The window a hook fires in.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Window {
        /// An existing entry was classified as a regular file with
        /// `fstatat(AT_SYMLINK_NOFOLLOW)`; `openat` is next.
        BeforeOpen,
        /// A save verified that the temp name still names the file it wrote;
        /// the rename that installs it is next. Fired with the target name.
        BeforeRename,
        /// A save's install syscall returned; the check that the target now
        /// names the written file is next. Fired with the target name.
        AfterInstall,
    }

    type Hook = Rc<dyn Fn(Window, &str)>;

    thread_local! {
        static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }

    /// Restores the previously installed hook (usually none) on drop.
    #[must_use = "the hook is removed when the guard is dropped"]
    pub struct Guard(Option<Hook>);

    impl Drop for Guard {
        fn drop(&mut self) {
            let previous = self.0.take();
            HOOK.with(|h| *h.borrow_mut() = previous);
        }
    }

    /// Installs `hook` for the current thread until the guard drops. It is
    /// called with the window and the entry name (the last path component,
    /// relative to its pinned directory).
    pub fn install(hook: impl Fn(Window, &str) + 'static) -> Guard {
        let previous = HOOK.with(|h| h.borrow_mut().replace(Rc::new(hook)));
        Guard(previous)
    }

    pub(crate) fn fire(window: Window, name: &str) {
        // Cloned out first, so a hook may itself call into this crate.
        let hook = HOOK.with(|h| h.borrow().clone());
        if let Some(hook) = hook {
            hook(window, name);
        }
    }
}

/// Fires [`race_hook`] at a check-then-use window; expands to nothing unless
/// `cfg(test)` or the `race-hook` feature is on.
macro_rules! race_point {
    ($window:ident, $name:expr) => {
        #[cfg(any(test, feature = "race-hook"))]
        race_hook::fire(race_hook::Window::$window, $name);
    };
}
use race_point;

/// Injection points for tests; production uses [`Hooks::default`].
pub(crate) struct Hooks<'h> {
    /// Runs after the temp file is fsynced and before the pre-rename
    /// re-verification (the window an out-of-contract writer can hit).
    pub after_temp_write: Option<&'h dyn Fn()>,
    /// Directory fsync after rename.
    pub sync_dir: fn(&File) -> io::Result<()>,
}

impl Default for Hooks<'_> {
    fn default() -> Self {
        Hooks {
            after_temp_write: None,
            sync_dir: File::sync_all,
        }
    }
}

impl ProjectLock<'_> {
    pub fn root(&self) -> &ProjectRoot {
        self.root
    }

    /// Saves `bytes` to `<root>/<path>` atomically while holding the lock.
    ///
    /// 1. Walk to the parent directory (creating missing directories),
    ///    refusing any symlink component.
    /// 2. Observe the target with `O_NOFOLLOW` (identity, size, mtime, hash,
    ///    mode). Compare with `expected` unless `force`.
    /// 3. Write a temp file with an unpredictable name in the same directory
    ///    (`O_CREAT|O_EXCL|O_NOFOLLOW`), `fsync` it, copy the existing
    ///    permission bits. The descriptor stays open until the save ends.
    /// 4. Re-observe the target; if it changed since step 2 (unless `force`),
    ///    remove the temp file and return `ModifiedDuringSave`. A target that
    ///    became a symlink is refused regardless of `force`.
    /// 5. Check with `fstatat(AT_SYMLINK_NOFOLLOW)` that the temp name still
    ///    names the file written in step 3 (same device/inode as the open
    ///    descriptor), refusing otherwise; `renameat` temp over target; check
    ///    that the target now names that file, returning an error otherwise;
    ///    `fsync` the directory (a failure here is `DirectorySync`,
    ///    durability unknown). See `TempFile::verify` for the residual.
    /// 6. Re-open the target with `O_NOFOLLOW` and verify device/inode equal
    ///    the temp file's and the bytes hash to what was written; otherwise
    ///    `ModifiedDuringSave`.
    pub fn save(
        &self,
        path: &ProjectPath,
        bytes: &[u8],
        expected: Expected,
        force: bool,
    ) -> Result<SaveReceipt, SaveError> {
        self.save_with(path, bytes, expected, force, &Hooks::default())
    }

    pub(crate) fn save_with(
        &self,
        path: &ProjectPath,
        bytes: &[u8],
        expected: Expected,
        force: bool,
        hooks: &Hooks<'_>,
    ) -> Result<SaveReceipt, SaveError> {
        let dir = self.root.walk(path, true)?;
        // Resolve the literal spelling to whatever actually backs it on
        // disk (NFC/NFD-insensitive), so conflict detection below compares
        // against the real file even when `path`'s raw bytes are a
        // differently-normalized spelling of an existing name (ext4; see
        // `resolve_target_name`).
        let name = self.root.resolve_target_name(&dir, path)?;
        let name = name.as_str();

        // Step 2: observe and compare.
        let before = ProjectRoot::observe(&dir, name, DEFAULT_READ_LIMIT)?.map(|(o, _)| o);
        if !force {
            check_expected(path, expected, before.as_ref())?;
        }

        // Step 3: temp write.
        let temp_name = temp_name(name)?;
        let mode = before.as_ref().map_or(0o644, |o| o.mode);
        let flags = sys::O_WRONLY | sys::O_CREAT | sys::O_EXCL | sys::O_NOFOLLOW;
        let mut temp = sys::open_at(&dir, &temp_name, flags, mode)
            .map_err(|e| classify_open(e, &temp_name))?;
        // Identity of the file we created, from its descriptor: every later
        // use of the temp *name* is checked against it.
        let temp_identity = match temp.metadata() {
            Ok(meta) => identity(&meta),
            Err(e) => {
                // Without the identity the name cannot be recognised safely,
                // so it is left in place rather than risk unlinking an entry
                // someone swapped in.
                return Err(e.into());
            }
        };
        let tmp = TempFile {
            dir: &dir,
            name: &temp_name,
            identity: temp_identity,
        };
        if let Err(e) = (|| -> io::Result<()> {
            temp.write_all(bytes)?;
            temp.sync_all()?;
            if before.is_some() {
                temp.set_permissions(std::fs::Permissions::from_mode(mode))?;
            }
            Ok(())
        })() {
            tmp.discard();
            return Err(e.into());
        }

        if let Some(hook) = hooks.after_temp_write {
            hook();
        }

        // Step 4: re-verify the target right before rename.
        let recheck = match ProjectRoot::observe(&dir, name, DEFAULT_READ_LIMIT) {
            Ok(o) => o.map(|(o, _)| o),
            Err(e) => {
                tmp.discard();
                return Err(e);
            }
        };
        if !force && recheck != before {
            tmp.discard();
            return Err(SaveError::Conflict(Box::new(SaveConflict {
                path: path.clone(),
                kind: SaveConflictKind::ModifiedDuringSave,
                ours: Some(sha256(bytes)),
                theirs: recheck.as_ref().map(|o| o.sha256),
                mtime: recheck.as_ref().map(|o| o.mtime),
                size: recheck.as_ref().map(|o| o.size),
            })));
        }

        // Step 5: verified rename + directory fsync.
        if let Err(e) = tmp.verify(path) {
            tmp.discard();
            return Err(e);
        }
        race_point!(BeforeRename, name);
        if let Err(e) = sys::rename_at(&dir, &temp_name, name) {
            tmp.discard();
            return Err(e.into());
        }
        race_point!(AfterInstall, name);
        verify_installed(&dir, name, path, temp_identity, bytes)?;
        drop(temp);
        (hooks.sync_dir)(&dir).map_err(SaveError::DirectorySync)?;

        // Step 6: post-rename verification.
        let written = sha256(bytes);
        let after = ProjectRoot::observe(&dir, name, DEFAULT_READ_LIMIT)?.map(|(o, _)| o);
        match after {
            Some(o) if o.identity == temp_identity && o.sha256 == written => Ok(SaveReceipt {
                path: path.clone(),
                bytes: o.size,
                sha256: written,
                mtime: o.mtime,
                identity: o.identity,
            }),
            other => Err(SaveError::Conflict(Box::new(SaveConflict {
                path: path.clone(),
                kind: SaveConflictKind::ModifiedDuringSave,
                ours: Some(written),
                theirs: other.as_ref().map(|o| o.sha256),
                mtime: other.as_ref().map(|o| o.mtime),
                size: other.as_ref().map(|o| o.size),
            }))),
        }
    }

    /// Unlinks `path` (never following symlinks; a symlink at `path` is
    /// refused rather than removed). Returns whether a file was removed.
    pub fn remove(&self, path: &ProjectPath) -> Result<bool, SaveError> {
        let dir = self.root.walk(path, false)?;
        // Refuse to unlink a symlink or non-regular file so callers never
        // "remove" something they did not create.
        if ProjectRoot::open_target(&dir, path.file_name())?.is_none() {
            return Ok(false);
        }
        match sys::unlink_at(&dir, path.file_name()) {
            Ok(()) => Ok(true),
            Err(e) if sys::errno_is(&e, sys::ENOENT) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

fn check_expected(
    path: &ProjectPath,
    expected: Expected,
    current: Option<&Observation>,
) -> Result<(), SaveError> {
    let (kind, ours) = match (expected, current) {
        (Expected::Any, _) => return Ok(()),
        (Expected::NewFile, None) => return Ok(()),
        (Expected::NewFile, Some(_)) => (SaveConflictKind::AlreadyExists, None),
        (Expected::Hash(h), None) => (SaveConflictKind::DeletedExternally, Some(h)),
        (Expected::Hash(h), Some(o)) => {
            if o.sha256 == h {
                return Ok(());
            }
            (SaveConflictKind::ModifiedExternally, Some(h))
        }
    };
    Err(SaveError::Conflict(Box::new(SaveConflict {
        path: path.clone(),
        kind,
        ours,
        theirs: current.map(|o| o.sha256),
        mtime: current.map(|o| o.mtime),
        size: current.map(|o| o.size),
    })))
}

/// A temp file name no other process can predict: 128 bits from the OS
/// random source (see [`sys::random_bytes`]). Together with
/// `O_CREAT|O_EXCL|O_NOFOLLOW` this means the name cannot be pre-planted, and
/// a writer who wants to swap it has to discover it by listing the directory
/// first.
fn temp_name(name: &str) -> io::Result<String> {
    let mut random = [0u8; 16];
    sys::random_bytes(&mut random)?;
    let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!(".{name}.flashtex-tmp-{suffix}"))
}

/// The temp file of one save: its name in the pinned directory and the
/// identity of the file this save created there.
struct TempFile<'a> {
    dir: &'a File,
    name: &'a str,
    identity: FileIdentity,
}

impl TempFile<'_> {
    /// Whether the temp name still names the file this save wrote (regular
    /// file, same device and inode), classified with
    /// `fstatat(AT_SYMLINK_NOFOLLOW)` so nothing is followed or opened.
    fn still_ours(&self) -> io::Result<bool> {
        match sys::stat_at_nofollow(self.dir, self.name.as_bytes()) {
            Ok(st) => Ok(st.is_file()
                && FileIdentity {
                    dev: st.dev,
                    ino: st.ino,
                } == self.identity),
            Err(e) if sys::errno_is(&e, sys::ENOENT) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// The check around a save's install, and its residual.
    ///
    /// Immediately before the temp file is renamed (or linked) into place, the
    /// temp name is classified with `fstatat(AT_SYMLINK_NOFOLLOW)` and must name
    /// the regular file this save created and fsynced (same device and inode as
    /// the still-open descriptor); otherwise the save is refused and nothing is
    /// installed. Immediately after, the target name must name that same file;
    /// otherwise the save is `ModifiedDuringSave` and never reports success.
    ///
    /// **Residual (not closable with POSIX `renameat`):** the rename binds a
    /// *name*. A process that can write the project directory, has discovered
    /// the random temp name, and swaps it in the window between the check and
    /// the rename gets its entry moved to the target name. That entry is never
    /// followed or opened by the rename (a symlink is moved as a link), nothing
    /// outside the pinned directory is touched, the post-install check turns the
    /// save into a conflict, and that process could have created or replaced the
    /// target entry directly anyway.
    fn verify(&self, path: &ProjectPath) -> Result<(), SaveError> {
        if self.still_ours()? {
            return Ok(());
        }
        Err(SaveError::Io(io::Error::other(format!(
            "save of {path} refused: its temp file {} was replaced or removed by \
             another process before it was installed; nothing was installed",
            self.name
        ))))
    }

    /// Best-effort cleanup on a failed save: unlinks the temp name only if it
    /// still names our file, so an entry someone else put there is left
    /// alone. (The same check-then-unlink residual as `remove` applies.)
    fn discard(&self) {
        if let Ok(true) = self.still_ours() {
            let _ = sys::unlink_at(self.dir, self.name);
        }
    }
}

/// After the install syscall: the target must now name the file this save
/// wrote. Checked with `fstatat(AT_SYMLINK_NOFOLLOW)` first; on a mismatch
/// the save is `ModifiedDuringSave` (never success), describing whatever is
/// there now through the no-follow, regular-files-only observer.
fn verify_installed(
    dir: &File,
    name: &str,
    path: &ProjectPath,
    written: FileIdentity,
    bytes: &[u8],
) -> Result<(), SaveError> {
    let installed = match sys::stat_at_nofollow(dir, name.as_bytes()) {
        Ok(st) => {
            st.is_file()
                && FileIdentity {
                    dev: st.dev,
                    ino: st.ino,
                } == written
        }
        Err(e) if sys::errno_is(&e, sys::ENOENT) => false,
        Err(e) => return Err(e.into()),
    };
    if installed {
        Ok(())
    } else {
        Err(modified_during_save(dir, name, path, bytes))
    }
}

/// The `ModifiedDuringSave` conflict for whatever is at `name` now.
fn modified_during_save(dir: &File, name: &str, path: &ProjectPath, bytes: &[u8]) -> SaveError {
    let theirs = match ProjectRoot::observe(dir, name, DEFAULT_READ_LIMIT) {
        Ok(o) => o.map(|(o, _)| o),
        Err(e) => return e,
    };
    SaveError::Conflict(Box::new(SaveConflict {
        path: path.clone(),
        kind: SaveConflictKind::ModifiedDuringSave,
        ours: Some(sha256(bytes)),
        theirs: theirs.as_ref().map(|o| o.sha256),
        mtime: theirs.as_ref().map(|o| o.mtime),
        size: theirs.as_ref().map(|o| o.size),
    }))
}

/// Convenience: open `root`, lock, save, unlock.
pub fn save_atomic(
    root: &Path,
    path: &ProjectPath,
    text: &str,
    expected: Expected,
    force: bool,
) -> Result<SaveReceipt, SaveError> {
    ProjectRoot::open(root)?.save(path, text.as_bytes(), expected, force)
}

/// Convenience for raw bytes; see [`save_atomic`].
pub fn save_atomic_bytes(
    root: &Path,
    path: &ProjectPath,
    bytes: &[u8],
    expected: Expected,
    force: bool,
) -> Result<SaveReceipt, SaveError> {
    ProjectRoot::open(root)?.save(path, bytes, expected, force)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct Temp(PathBuf);
    impl Temp {
        fn new(tag: &str) -> Self {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!(
                "flashtex-save-unit-{tag}-{}-{n}",
                std::process::id()
            ));
            fs::create_dir_all(&p).unwrap();
            Temp(p)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn pp(s: &str) -> ProjectPath {
        ProjectPath::normalize(s).unwrap()
    }

    #[test]
    fn directory_fsync_failure_is_a_hard_error_after_rename() {
        let t = Temp::new("dirsync");
        let root = ProjectRoot::open(&t.0).unwrap();
        let lock = root.lock().unwrap();
        fn failing(_: &File) -> io::Result<()> {
            Err(io::Error::other("simulated fsync failure"))
        }
        let hooks = Hooks {
            after_temp_write: None,
            sync_dir: failing,
        };
        let err = lock
            .save_with(&pp("a.tex"), b"payload", Expected::NewFile, false, &hooks)
            .unwrap_err();
        assert!(matches!(err, SaveError::DirectorySync(_)), "{err}");
        // The rename already happened; the caller must re-read to learn the truth.
        assert_eq!(fs::read(t.0.join("a.tex")).unwrap(), b"payload");
    }

    #[test]
    fn out_of_contract_writer_between_check_and_rename_is_reported() {
        let t = Temp::new("race-write");
        fs::write(t.0.join("a.tex"), "base").unwrap();
        let root = ProjectRoot::open(&t.0).unwrap();
        let lock = root.lock().unwrap();
        let target = t.0.join("a.tex");
        let interfere = move || fs::write(&target, "sneaky").unwrap();
        let hooks = Hooks {
            after_temp_write: Some(&interfere),
            sync_dir: File::sync_all,
        };
        let err = lock
            .save_with(
                &pp("a.tex"),
                b"mine",
                Expected::Hash(sha256(b"base")),
                false,
                &hooks,
            )
            .unwrap_err();
        match err {
            SaveError::Conflict(c) => {
                assert_eq!(c.kind, SaveConflictKind::ModifiedDuringSave);
                assert_eq!(c.ours, Some(sha256(b"mine")));
                assert_eq!(c.theirs, Some(sha256(b"sneaky")));
            }
            other => panic!("expected conflict, got {other:?}"),
        }
        assert_eq!(
            fs::read_to_string(t.0.join("a.tex")).unwrap(),
            "sneaky",
            "their write is left alone"
        );
        let leftovers: Vec<_> = fs::read_dir(&t.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.contains("flashtex-tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp file cleaned up: {leftovers:?}");
    }

    #[test]
    fn new_file_does_not_clobber_a_concurrently_created_target() {
        let t = Temp::new("race-newfile");
        let root = ProjectRoot::open(&t.0).unwrap();
        let lock = root.lock().unwrap();
        let target = t.0.join("new.tex");
        let create = move || fs::write(&target, "created meanwhile").unwrap();
        let hooks = Hooks {
            after_temp_write: Some(&create),
            sync_dir: File::sync_all,
        };
        let err = lock
            .save_with(&pp("new.tex"), b"mine", Expected::NewFile, false, &hooks)
            .unwrap_err();
        match err {
            SaveError::Conflict(c) => {
                assert_eq!(c.kind, SaveConflictKind::ModifiedDuringSave);
                assert_eq!(c.theirs, Some(sha256(b"created meanwhile")));
            }
            other => panic!("expected conflict, got {other:?}"),
        }
        assert_eq!(
            fs::read_to_string(t.0.join("new.tex")).unwrap(),
            "created meanwhile"
        );
    }

    #[cfg(unix)]
    #[test]
    fn target_swapped_for_symlink_between_check_and_rename_is_refused() {
        let outside = Temp::new("race-outside");
        let victim = outside.0.join("victim.tex");
        fs::write(&victim, "untouchable").unwrap();
        let t = Temp::new("race-symlink");
        fs::write(t.0.join("a.tex"), "base").unwrap();
        let root = ProjectRoot::open(&t.0).unwrap();
        let lock = root.lock().unwrap();
        let target = t.0.join("a.tex");
        let victim2 = victim.clone();
        let swap = move || {
            fs::remove_file(&target).unwrap();
            std::os::unix::fs::symlink(&victim2, &target).unwrap();
        };
        let hooks = Hooks {
            after_temp_write: Some(&swap),
            sync_dir: File::sync_all,
        };
        let err = lock
            .save_with(
                &pp("a.tex"),
                b"mine",
                Expected::Hash(sha256(b"base")),
                true,
                &hooks,
            )
            .unwrap_err();
        assert!(
            matches!(err, SaveError::Refused(Refused::SymlinkComponent { .. })),
            "{err:?}"
        );
        assert_eq!(fs::read_to_string(&victim).unwrap(), "untouchable");
        assert!(
            fs::symlink_metadata(t.0.join("a.tex"))
                .unwrap()
                .file_type()
                .is_symlink(),
            "symlink left in place"
        );
    }

    /// CI failure at `tests/recovery.rs:287`
    /// (`nfc_nfd_spelling_does_not_bypass_save_conflict_detection`): on ext4
    /// an NFD-spelled path is a *different* file from the NFC one, so a
    /// literal `openat` lookup for the NFD candidate against an NFC on-disk
    /// file fails (`ENOENT`) and used to make save-conflict detection
    /// observe "target missing" (`DeletedExternally`) instead of "modified
    /// by A" (`ModifiedExternally`).
    ///
    /// This deterministically simulates that ext4 behavior by calling the
    /// directory-listing fallback (`resolve_leaf_via_directory_listing`)
    /// directly, bypassing the literal `open_target` lookup that precedes it
    /// in `resolve_target_name`. That bypass is necessary for the test to be
    /// meaningful on any host: APFS's own `openat` is normalization
    /// *insensitive*, so it already finds an NFC file for an NFD-spelled
    /// lookup before the fallback would ever run -- calling only the
    /// composed `resolve_target_name` on this (macOS) machine would pass
    /// whether or not this fallback existed, exactly as `graph.rs`'s
    /// `resolve_via_directory_listing` unit test documents for the
    /// discovery path this mirrors.
    #[test]
    fn directory_listing_fallback_resolves_nfd_candidate_to_nfc_name_simulating_ext4() {
        let t = Temp::new("save-nfc-nfd-fallback");
        let nfc_stem = "caf\u{e9}"; // "café", 'é' precomposed (NFC) -- the only file written to disk
        let nfd_stem = "cafe\u{301}"; // "café", 'e' + combining acute (NFD) -- how it's looked up
        fs::write(t.0.join(format!("{nfc_stem}.tex")), "original").unwrap();

        let root = ProjectRoot::open(&t.0).unwrap();
        let nfd_candidate = pp(&format!("{nfd_stem}.tex"));
        let resolved = root
            .resolve_leaf_via_directory_listing(&nfd_candidate)
            .expect(
                "directory listing must find the on-disk NFC file for an NFD-spelled candidate",
            );
        assert_eq!(
            resolved,
            format!("{nfc_stem}.tex"),
            "must resolve to the on-disk (NFC) raw bytes, not the NFD spelling it was looked up with"
        );

        // Exactly one physical file backs this -- the fallback must never
        // itself create or duplicate anything, only report a name.
        assert_eq!(fs::read_dir(&t.0).unwrap().count(), 1);
    }

    /// Full pipeline, through the public `save` API: an NFD-spelled save
    /// against a since-modified NFC file must be refused as
    /// `ModifiedExternally` (never bypassed into `DeletedExternally` or a
    /// silent overwrite), and the refusal must leave exactly the original
    /// NFC file on disk -- no NFD-spelled duplicate or leftover temp file
    /// from the attempted write. `directory_listing_fallback_resolves_nfd_candidate_to_nfc_name_simulating_ext4`
    /// above proves the resolution primitive itself is correct on any
    /// filesystem; this proves `save_with` actually wires it in.
    #[test]
    fn save_through_nfd_spelling_conflicts_against_modified_nfc_file_without_duplicating() {
        let t = Temp::new("save-nfc-nfd-conflict");
        let nfc_stem = "caf\u{e9}";
        let nfd_stem = "cafe\u{301}";
        let path_nfc = pp(&format!("{nfc_stem}.tex"));
        let path_nfd = pp(&format!("{nfd_stem}.tex"));
        assert_eq!(path_nfc, path_nfd, "sanity: same ProjectPath identity");

        let root = ProjectRoot::open(&t.0).unwrap();
        let lock = root.lock().unwrap();
        let base = lock
            .save(&path_nfc, b"original", Expected::NewFile, false)
            .unwrap();

        // "Editor A" saves under the NFC spelling.
        lock.save(&path_nfc, b"edit-A", Expected::Hash(base.sha256), false)
            .unwrap();

        // "Editor B" saves under the NFD spelling with the stale base hash.
        let err = lock
            .save(&path_nfd, b"edit-B", Expected::Hash(base.sha256), false)
            .unwrap_err();
        match err {
            SaveError::Conflict(c) => {
                assert_eq!(c.kind, SaveConflictKind::ModifiedExternally);
                assert_eq!(c.theirs, Some(sha256(b"edit-A")));
            }
            other => panic!("expected ModifiedExternally, got {other:?}"),
        }

        // Exactly one `.tex` file on disk, still holding A's content, spelled
        // NFC (`.flashtex/` -- the project lock directory `root.lock()`
        // created -- is unrelated to this check and excluded).
        let entries: Vec<_> = fs::read_dir(&t.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n != ".flashtex")
            .collect();
        assert_eq!(
            entries,
            vec![format!("{nfc_stem}.tex")],
            "no second (NFD-spelled or temp) file may exist after the refused save"
        );
        assert_eq!(
            fs::read_to_string(t.0.join(format!("{nfc_stem}.tex"))).unwrap(),
            "edit-A"
        );
    }

    /// A target that was never created under any spelling -- not a
    /// normalization twin of an existing file -- must still resolve to the
    /// literal (missing) name and report `DeletedExternally`, exactly as
    /// before this change: the directory-listing fallback must not invent a
    /// match, and a genuinely absent file is not confused with a
    /// differently-spelled reference to an existing one.
    #[test]
    fn truly_missing_target_still_reports_deleted_externally() {
        let t = Temp::new("save-missing-target");
        let root = ProjectRoot::open(&t.0).unwrap();
        let lock = root.lock().unwrap();

        let missing = pp("never-existed.tex");
        let err = lock
            .save(
                &missing,
                b"new content",
                Expected::Hash(sha256(b"whatever")),
                false,
            )
            .unwrap_err();
        match err {
            SaveError::Conflict(c) => {
                assert_eq!(c.kind, SaveConflictKind::DeletedExternally);
                assert_eq!(c.theirs, None);
            }
            other => panic!("expected DeletedExternally, got {other:?}"),
        }
        assert!(!t.0.join("never-existed.tex").exists());

        // Also true for an NFD-spelled path with no NFC twin on disk.
        let missing_nfd = pp("cafe\u{301}-missing.tex");
        assert!(
            root.resolve_leaf_via_directory_listing(&missing_nfd)
                .is_none()
        );
        let err2 = lock
            .save(
                &missing_nfd,
                b"new content",
                Expected::Hash(sha256(b"whatever")),
                false,
            )
            .unwrap_err();
        assert!(matches!(
            err2,
            SaveError::Conflict(c) if c.kind == SaveConflictKind::DeletedExternally
        ));
    }

    fn temp_entries(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.contains("flashtex-tmp"))
            .collect()
    }

    /// Temp names carry 128 random bits, not a pid and a counter.
    #[test]
    fn temp_names_are_unpredictable() {
        let (a, b) = (temp_name("a.tex").unwrap(), temp_name("a.tex").unwrap());
        assert_ne!(a, b);
        for n in [&a, &b] {
            let suffix = n.strip_prefix(".a.tex.flashtex-tmp-").unwrap();
            assert_eq!(suffix.len(), 32, "{n}");
            assert!(suffix.bytes().all(|c| c.is_ascii_hexdigit()), "{n}");
        }
    }

    /// Third review, finding 1: a temp name swapped for a symlink (or any
    /// other entry) after the write is refused before the rename. Nothing is
    /// installed, the target keeps its content, and the swapped-in entry is
    /// not unlinked by the cleanup.
    #[cfg(unix)]
    #[test]
    fn temp_name_swapped_before_install_is_refused() {
        for force in [false, true] {
            let outside = Temp::new("temp-swap-outside");
            let victim = outside.0.join("victim.tex");
            fs::write(&victim, "untouchable").unwrap();
            let t = Temp::new("temp-swap");
            fs::write(t.0.join("a.tex"), "base").unwrap();
            let root = ProjectRoot::open(&t.0).unwrap();
            let lock = root.lock().unwrap();
            let (dir, victim2) = (t.0.clone(), victim.clone());
            let swap = move || {
                let [name] = temp_entries(&dir).try_into().unwrap();
                fs::remove_file(dir.join(&name)).unwrap();
                std::os::unix::fs::symlink(&victim2, dir.join(&name)).unwrap();
            };
            let hooks = Hooks {
                after_temp_write: Some(&swap),
                ..Hooks::default()
            };
            let err = lock
                .save_with(
                    &pp("a.tex"),
                    b"mine",
                    Expected::Hash(sha256(b"base")),
                    force,
                    &hooks,
                )
                .unwrap_err();
            assert!(
                matches!(&err, SaveError::Io(e) if e.to_string().contains("temp file")),
                "force={force}: {err:?}"
            );
            assert_eq!(fs::read_to_string(t.0.join("a.tex")).unwrap(), "base");
            assert_eq!(fs::read_to_string(&victim).unwrap(), "untouchable");
            let [left] = temp_entries(&t.0).try_into().unwrap();
            assert!(
                fs::symlink_metadata(t.0.join(left))
                    .unwrap()
                    .file_type()
                    .is_symlink(),
                "the swapped-in entry is not ours to unlink"
            );
        }
    }

    /// Third review, finding 1, residual: a temp name swapped in the window
    /// between the temp check and the rename is moved to the target as an
    /// entry (never followed), and the save is a conflict, never success.
    #[cfg(unix)]
    #[test]
    fn temp_name_swapped_in_the_rename_window_is_never_reported_as_saved() {
        let outside = Temp::new("rename-window-outside");
        let victim = outside.0.join("victim.tex");
        fs::write(&victim, "untouchable").unwrap();
        let t = Temp::new("rename-window-temp");
        fs::write(t.0.join("a.tex"), "base").unwrap();
        let root = ProjectRoot::open(&t.0).unwrap();
        let lock = root.lock().unwrap();
        let (dir, victim2) = (t.0.clone(), victim.clone());
        let _guard = race_hook::install(move |w, name| {
            if w == race_hook::Window::BeforeRename && name == "a.tex" {
                let [temp] = temp_entries(&dir).try_into().unwrap();
                fs::remove_file(dir.join(&temp)).unwrap();
                std::os::unix::fs::symlink(&victim2, dir.join(&temp)).unwrap();
            }
        });
        let result = lock.save(
            &pp("a.tex"),
            b"mine",
            Expected::Hash(sha256(b"base")),
            false,
        );
        assert!(
            matches!(
                &result,
                Err(SaveError::Refused(Refused::SymlinkComponent { .. }))
            ),
            "{result:?}"
        );
        assert_eq!(fs::read_to_string(&victim).unwrap(), "untouchable");
        assert!(
            fs::symlink_metadata(t.0.join("a.tex"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    /// Third review, finding 1: if the target stops naming the written file
    /// right after the install, the save is `ModifiedDuringSave`.
    #[test]
    fn target_replaced_right_after_install_is_a_conflict() {
        let t = Temp::new("after-install");
        fs::write(t.0.join("a.tex"), "base").unwrap();
        let root = ProjectRoot::open(&t.0).unwrap();
        let lock = root.lock().unwrap();
        let dir = t.0.clone();
        let _guard = race_hook::install(move |w, name| {
            if w == race_hook::Window::AfterInstall && name == "a.tex" {
                fs::write(dir.join("theirs.tmp"), "theirs").unwrap();
                fs::rename(dir.join("theirs.tmp"), dir.join("a.tex")).unwrap();
            }
        });
        let err = lock
            .save(
                &pp("a.tex"),
                b"mine",
                Expected::Hash(sha256(b"base")),
                false,
            )
            .unwrap_err();
        assert!(
            matches!(&err, SaveError::Conflict(c) if c.kind == SaveConflictKind::ModifiedDuringSave
                && c.theirs == Some(sha256(b"theirs"))),
            "{err:?}"
        );
    }
}
