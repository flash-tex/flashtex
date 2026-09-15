//! Crash recovery journal: unsaved buffers are written atomically to
//! `<root>/.flashtex/recovery/<sha256(path)>.json` with the text, the hash of
//! the on-disk base they were edited from, and a timestamp. After a crash the
//! journal can be listed, checked against the current file, restored, or
//! discarded. Restoring never silently overwrites a file that moved on from
//! the recorded base.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::json::Json;
use crate::path::ProjectPath;
use crate::save::{DEFAULT_READ_LIMIT, Expected, ProjectLock, ProjectRoot, SaveError, SaveReceipt};
use crate::sha256::{Digest, hex, parse_hex, sha256, sha256_hex};

pub const RECOVERY_DIR: &str = ".flashtex/recovery";
const SCHEMA_VERSION: u64 = 1;
/// Journal entries larger than this are treated as unreadable.
const JOURNAL_READ_LIMIT: u64 = 64 * 1024 * 1024;

/// One journaled buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEntry {
    pub path: ProjectPath,
    pub text: String,
    pub text_sha256: Digest,
    /// Hash of the on-disk content the buffer was edited from (`None` for a
    /// file that did not exist yet).
    pub base_sha256: Option<Digest>,
    pub saved_at: SystemTime,
    /// The journal file this entry was read from or written to.
    pub journal_file: PathBuf,
}

/// Relationship between the current on-disk file and a journal entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentState {
    /// The file does not exist on disk.
    Missing,
    /// Disk still equals the recorded base: restoring loses nothing.
    MatchesBase,
    /// Disk already equals the journaled text: nothing to restore.
    MatchesJournal,
    /// Disk differs from both: restoring would overwrite someone's work.
    Diverged(Digest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreCheck {
    pub current: CurrentState,
    /// True when a non-forced restore will succeed.
    pub safe: bool,
}

#[derive(Debug)]
pub enum RecoveryError {
    Io(io::Error),
    Malformed { file: PathBuf, message: String },
    Save(SaveError),
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecoveryError::Io(e) => write!(f, "recovery journal I/O error: {e}"),
            RecoveryError::Malformed { file, message } => {
                write!(f, "malformed journal file {}: {message}", file.display())
            }
            RecoveryError::Save(e) => write!(f, "restore failed: {e}"),
        }
    }
}

impl std::error::Error for RecoveryError {}

impl From<io::Error> for RecoveryError {
    fn from(e: io::Error) -> Self {
        RecoveryError::Io(e)
    }
}

impl From<SaveError> for RecoveryError {
    fn from(e: SaveError) -> Self {
        RecoveryError::Save(e)
    }
}

/// Journal listing: readable entries sorted by path, plus files that could
/// not be parsed (kept on disk for manual inspection).
#[derive(Debug, Default)]
pub struct Listing {
    pub entries: Vec<RecoveryEntry>,
    pub malformed: Vec<(PathBuf, String)>,
}
/// Journal bound to an open [`ProjectRoot`]. Reads go through the rooted,
/// symlink-refusing reader; writes and removals require the project lock.
#[derive(Debug, Clone, Copy)]
pub struct RecoveryJournal<'r> {
    root: &'r ProjectRoot,
}

impl<'r> RecoveryJournal<'r> {
    pub fn new(root: &'r ProjectRoot) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &'r ProjectRoot {
        self.root
    }

    pub fn dir(&self) -> PathBuf {
        self.root.path().join(RECOVERY_DIR)
    }

    /// Project-relative journal path for `path`.
    pub fn journal_path(&self, path: &ProjectPath) -> ProjectPath {
        ProjectPath::normalize(&format!(
            "{RECOVERY_DIR}/{}.json",
            sha256_hex(path.as_str().as_bytes())
        ))
        .expect("journal path is a fixed relative path")
    }

    pub fn journal_file(&self, path: &ProjectPath) -> PathBuf {
        self.journal_path(path).to_os_path(self.root.path())
    }

    /// Atomically records `text` for `path` under the project lock.
    /// Replaces any previous entry.
    pub fn record(
        &self,
        lock: &ProjectLock<'_>,
        path: &ProjectPath,
        text: &str,
        base_sha256: Option<Digest>,
    ) -> Result<RecoveryEntry, RecoveryError> {
        let now = SystemTime::now();
        let millis = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let text_sha256 = sha256(text.as_bytes());
        let mut doc = Json::object();
        doc.insert("schema_version", SCHEMA_VERSION)
            .insert("path", path.as_str())
            .insert("text", text)
            .insert("text_sha256", hex(&text_sha256))
            .insert("base_sha256", base_sha256.as_ref().map(hex))
            .insert("saved_at_unix_ms", millis);
        let journal_path = self.journal_path(path);
        lock.save(
            &journal_path,
            doc.to_string_compact().as_bytes(),
            Expected::Any,
            true,
        )?;
        Ok(RecoveryEntry {
            path: path.clone(),
            text: text.to_string(),
            text_sha256,
            base_sha256,
            saved_at: UNIX_EPOCH + Duration::from_millis(millis),
            journal_file: self.journal_file(path),
        })
    }

    fn read_entry(
        &self,
        journal_path: &ProjectPath,
    ) -> Result<Option<RecoveryEntry>, RecoveryError> {
        let file = journal_path.to_os_path(self.root.path());
        match self.root.read(journal_path, JOURNAL_READ_LIMIT)? {
            Some(read) => parse_entry(&file, &read.bytes).map(Some),
            None => Ok(None),
        }
    }

    /// Loads the entry for `path`, if any.
    pub fn load(&self, path: &ProjectPath) -> Result<Option<RecoveryEntry>, RecoveryError> {
        self.read_entry(&self.journal_path(path))
    }

    /// Lists all entries. Files that are not valid entries are reported, not
    /// deleted. The journal directory is listed from its pinned descriptor
    /// through the rooted walk (a symlinked `.flashtex` or `recovery` is
    /// refused, not enumerated); each file is then read through the rooted
    /// reader.
    pub fn list(&self) -> Result<Listing, RecoveryError> {
        let mut listing = Listing::default();
        let probe = ProjectPath::normalize(&format!("{RECOVERY_DIR}/entry.json"))
            .expect("constant recovery path");
        let Some(names) = self.root.list_parent_of(&probe)? else {
            return Ok(listing);
        };
        for raw in names {
            let Ok(name) = String::from_utf8(raw) else {
                continue;
            };
            if name.starts_with('.') || !name.ends_with(".json") {
                continue;
            }
            let Ok(journal_path) = ProjectPath::normalize(&format!("{RECOVERY_DIR}/{name}")) else {
                continue;
            };
            let file = journal_path.to_os_path(self.root.path());
            match self.read_entry(&journal_path) {
                Ok(Some(e)) => listing.entries.push(e),
                Ok(None) => {}
                Err(RecoveryError::Malformed { file, message }) => {
                    listing.malformed.push((file, message))
                }
                Err(RecoveryError::Io(e)) => listing.malformed.push((file, e.to_string())),
                Err(RecoveryError::Save(e)) => listing.malformed.push((file, e.to_string())),
            }
        }
        listing.entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(listing)
    }

    /// Compares the current on-disk file with the entry's base and text.
    pub fn check(&self, entry: &RecoveryEntry) -> Result<RestoreCheck, RecoveryError> {
        let current = match self.root.read(&entry.path, DEFAULT_READ_LIMIT)? {
            Some(read) => {
                let h = read.sha256;
                if h == entry.text_sha256 {
                    CurrentState::MatchesJournal
                } else if Some(h) == entry.base_sha256 {
                    CurrentState::MatchesBase
                } else {
                    CurrentState::Diverged(h)
                }
            }
            None => CurrentState::Missing,
        };
        let safe = match current {
            CurrentState::MatchesBase | CurrentState::MatchesJournal => true,
            CurrentState::Missing => entry.base_sha256.is_none(),
            CurrentState::Diverged(_) => false,
        };
        Ok(RestoreCheck { current, safe })
    }

    /// Writes the journaled text to the project file under the lock with the
    /// same conflict rules as [`ProjectLock::save`]: the disk must still
    /// match the recorded base (or be absent for a new file) unless `force`.
    /// On success the entry is discarded. A file that already equals the
    /// journal text is treated as restored without rewriting.
    pub fn restore_to_disk(
        &self,
        lock: &ProjectLock<'_>,
        entry: &RecoveryEntry,
        force: bool,
    ) -> Result<Option<SaveReceipt>, RecoveryError> {
        let check = self.check(entry)?;
        if check.current == CurrentState::MatchesJournal {
            self.discard(lock, &entry.path)?;
            return Ok(None);
        }
        let expected = match entry.base_sha256 {
            Some(h) => Expected::Hash(h),
            None => Expected::NewFile,
        };
        let receipt = lock.save(&entry.path, entry.text.as_bytes(), expected, force)?;
        self.discard(lock, &entry.path)?;
        Ok(Some(receipt))
    }

    /// Removes the entry for `path`. Returns whether one existed.
    pub fn discard(
        &self,
        lock: &ProjectLock<'_>,
        path: &ProjectPath,
    ) -> Result<bool, RecoveryError> {
        Ok(lock.remove(&self.journal_path(path))?)
    }
}

fn parse_entry(file: &Path, bytes: &[u8]) -> Result<RecoveryEntry, RecoveryError> {
    let malformed = |message: String| RecoveryError::Malformed {
        file: file.to_path_buf(),
        message,
    };
    let text = std::str::from_utf8(bytes).map_err(|_| malformed("not UTF-8".into()))?;
    let doc = Json::parse(text).map_err(|e| malformed(e.to_string()))?;
    let version = doc
        .get("schema_version")
        .and_then(Json::as_u64)
        .ok_or_else(|| malformed("missing schema_version".into()))?;
    if version != SCHEMA_VERSION {
        return Err(malformed(format!("unsupported schema_version {version}")));
    }
    let path_str = doc
        .get("path")
        .and_then(Json::as_str)
        .ok_or_else(|| malformed("missing path".into()))?;
    let path = ProjectPath::normalize(path_str).map_err(|e| malformed(format!("bad path: {e}")))?;
    let body = doc
        .get("text")
        .and_then(Json::as_str)
        .ok_or_else(|| malformed("missing text".into()))?;
    let recorded_hash = doc
        .get("text_sha256")
        .and_then(Json::as_str)
        .and_then(parse_hex)
        .ok_or_else(|| malformed("missing text_sha256".into()))?;
    let text_sha256 = sha256(body.as_bytes());
    if text_sha256 != recorded_hash {
        return Err(malformed(
            "text_sha256 does not match text (truncated or corrupted)".into(),
        ));
    }
    let base_sha256 = match doc.get("base_sha256") {
        None | Some(Json::Null) => None,
        Some(v) => Some(
            v.as_str()
                .and_then(parse_hex)
                .ok_or_else(|| malformed("bad base_sha256".into()))?,
        ),
    };
    let millis = doc
        .get("saved_at_unix_ms")
        .and_then(Json::as_u64)
        .ok_or_else(|| malformed("missing saved_at_unix_ms".into()))?;
    Ok(RecoveryEntry {
        path,
        text: body.to_string(),
        text_sha256,
        base_sha256,
        saved_at: UNIX_EPOCH + Duration::from_millis(millis),
        journal_file: file.to_path_buf(),
    })
}
