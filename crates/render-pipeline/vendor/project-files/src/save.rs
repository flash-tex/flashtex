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
// Windows port (apps/windows, uncommitted): this vendored snapshot predates the
// cross-platform rework the live `crates/project-files` already carries, so it
// would not COMPILE on a non-unix target at all. `sys.rs` here already reports
// `SUPPORTED = false` off unix, so every save path is inert on Windows; the
// cfg-gated shims below exist purely so `crates/render-pipeline` (which only
// ever uses the read side, for rooted TFM reads) builds. Unix behaviour is
// byte-identical: the `#[cfg(unix)]` arms are the original expressions.
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
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

#[cfg(unix)]
fn identity(meta: &std::fs::Metadata) -> FileIdentity {
    FileIdentity {
        dev: meta.dev(),
        ino: meta.ino(),
    }
}

/// Non-unix shim: see the cfg note at the top of this file. Never reached at
/// run time (`sys::SUPPORTED` is false, so every caller refuses first).
#[cfg(not(unix))]
fn identity(_meta: &std::fs::Metadata) -> FileIdentity {
    FileIdentity { dev: 0, ino: 0 }
}

/// The unix permission bits of `meta`; the non-unix shim reports the same
/// default this module already uses when there is no prior observation.
#[cfg(unix)]
fn permission_mode(meta: &std::fs::Metadata) -> u32 {
    meta.mode() & 0o7777
}

#[cfg(not(unix))]
fn permission_mode(_meta: &std::fs::Metadata) -> u32 {
    0o644
}

/// Restores `mode` on a freshly created temp file (unix only; a no-op elsewhere).
#[cfg(unix)]
fn restore_mode(file: &File, mode: u32) -> io::Result<()> {
    file.set_permissions(std::fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn restore_mode(_file: &File, _mode: u32) -> io::Result<()> {
    Ok(())
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
    /// directory (not a symlink); components above it are the caller's
    /// choice and are not inspected.
    pub fn open(path: &Path) -> Result<ProjectRoot, SaveError> {
        if !sys::SUPPORTED {
            return Err(refused(Refused::Unsupported));
        }
        let dir = sys::open_dir_nofollow(path).map_err(|e| {
            classify_open(
                e,
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<root>"),
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

    /// Opens the regular file `name` in `dir` for reading without following
    /// a symlink. `Ok(None)` when absent.
    fn open_target(dir: &File, name: &str) -> Result<Option<File>, SaveError> {
        match sys::open_at(dir, name, sys::O_RDONLY | sys::O_NOFOLLOW, 0) {
            Ok(f) => Ok(Some(f)),
            Err(e) if sys::errno_is(&e, sys::ENOENT) => Ok(None),
            Err(e) => Err(classify_open(e, name)),
        }
    }

    fn observe(
        dir: &File,
        name: &str,
        limit: u64,
    ) -> Result<Option<(Observation, Vec<u8>)>, SaveError> {
        let Some(mut file) = Self::open_target(dir, name)? else {
            return Ok(None);
        };
        let meta = file.metadata()?;
        if !meta.is_file() {
            return Err(refused(Refused::NotARegularFile {
                component: name.to_string(),
            }));
        }
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
            mode: permission_mode(&meta),
        };
        Ok(Some((obs, bytes)))
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
        let flags = sys::O_RDWR | sys::O_CREAT | sys::O_NOFOLLOW;
        let file = sys::open_at(&dir, lock_path.file_name(), flags, 0o644)
            .map_err(|e| classify_open(e, lock_path.file_name()))?;
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

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    /// 3. Write a temp file in the same directory (`O_CREAT|O_EXCL|O_NOFOLLOW`),
    ///    `fsync` it, copy the existing permission bits.
    /// 4. Re-observe the target; if it changed since step 2 (unless `force`),
    ///    remove the temp file and return `ModifiedDuringSave`. A target that
    ///    became a symlink is refused regardless of `force`.
    /// 5. `renameat` temp over target; `fsync` the directory (a failure here
    ///    is `DirectorySync`, durability unknown).
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
        let name = path.file_name();
        let dir = self.root.walk(path, true)?;

        // Step 2: observe and compare.
        let before = ProjectRoot::observe(&dir, name, DEFAULT_READ_LIMIT)?.map(|(o, _)| o);
        if !force {
            check_expected(path, expected, before.as_ref())?;
        }

        // Step 3: temp write.
        let temp_name = temp_name(name);
        let mode = before.as_ref().map_or(0o644, |o| o.mode);
        let flags = sys::O_WRONLY | sys::O_CREAT | sys::O_EXCL | sys::O_NOFOLLOW;
        let mut temp = sys::open_at(&dir, &temp_name, flags, mode)
            .map_err(|e| classify_open(e, &temp_name))?;
        let temp_identity = match (|| -> io::Result<FileIdentity> {
            temp.write_all(bytes)?;
            temp.sync_all()?;
            if before.is_some() {
                restore_mode(&temp, mode)?;
            }
            Ok(identity(&temp.metadata()?))
        })() {
            Ok(id) => id,
            Err(e) => {
                let _ = sys::unlink_at(&dir, &temp_name);
                return Err(e.into());
            }
        };
        drop(temp);

        if let Some(hook) = hooks.after_temp_write {
            hook();
        }

        // Step 4: re-verify the target right before rename.
        let recheck = match ProjectRoot::observe(&dir, name, DEFAULT_READ_LIMIT) {
            Ok(o) => o.map(|(o, _)| o),
            Err(e) => {
                let _ = sys::unlink_at(&dir, &temp_name);
                return Err(e);
            }
        };
        if !force && recheck != before {
            let _ = sys::unlink_at(&dir, &temp_name);
            return Err(SaveError::Conflict(Box::new(SaveConflict {
                path: path.clone(),
                kind: SaveConflictKind::ModifiedDuringSave,
                ours: Some(sha256(bytes)),
                theirs: recheck.as_ref().map(|o| o.sha256),
                mtime: recheck.as_ref().map(|o| o.mtime),
                size: recheck.as_ref().map(|o| o.size),
            })));
        }

        // Step 5: rename + directory fsync.
        if let Err(e) = sys::rename_at(&dir, &temp_name, name) {
            let _ = sys::unlink_at(&dir, &temp_name);
            return Err(e.into());
        }
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

fn temp_name(name: &str) -> String {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(".{name}.flashtex-tmp-{}-{n}", std::process::id())
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
}
