//! A document and its applied-edit IDs occupy one atomic, fsynced JSON record.
//! Returning a receipt means both are durable. On any persistence uncertainty,
//! this handle is poisoned: reopen the store and inspect recovery before retry.
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Opens `path` (a directory) so `sync_all` can fsync it after a rename.
///
/// Plain `File::open` cannot open a directory on Windows at all (it fails
/// with `ERROR_ACCESS_DENIED`, unlike POSIX's `open(2)`); it needs
/// `FILE_FLAG_BACKUP_SEMANTICS`. A read-only handle isn't enough either:
/// `sync_all`'s `FlushFileBuffers` itself requires write access on the
/// handle, or it fails with the same `ERROR_ACCESS_DENIED` (measured). See
/// `crates/project-files/src/sys.rs`'s `open_dir_std`/`open_at` for the same
/// two fixes applied there.
#[cfg(windows)]
fn open_dir_for_sync(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(not(windows))]
fn open_dir_for_sync(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

/// Atomically replaces `destination` with `temporary`, via `std::fs::rename`
/// rather than `NamedTempFile::persist`.
///
/// The two are not equivalent on Windows, and the difference is a correctness
/// bug rather than a style preference. `tempfile`'s `persist` calls
/// `MoveFileExW(MOVEFILE_REPLACE_EXISTING)`, which replaces the destination
/// through the ordinary delete path: it needs `DELETE` access on the existing
/// file, so it fails with `ERROR_ACCESS_DENIED` whenever *any* other handle to
/// the destination is open. `std::fs::rename` instead asks for
/// `FileRenameInfoEx` with `FILE_RENAME_FLAG_POSIX_SEMANTICS` (Windows 10 1607+,
/// with a `MoveFileExW` fallback), which unlinks the old file from the directory
/// and lets existing readers keep reading their now-nameless handle — the same
/// observable behaviour POSIX `rename(2)` has always had, and the behaviour the
/// rest of this module's durability reasoning assumes.
///
/// Measured on Windows 11, 300 replacements against one thread doing a bare
/// `std::fs::read` of the destination in a loop: `persist` failed 231 times with
/// `ERROR_ACCESS_DENIED`, `std::fs::rename` failed 0 times. That reader is not a
/// contrived case — the editor UI, a backup agent or a virus scanner reading
/// `document.json` was enough to make a durable save report `storage_error` and
/// leave the document at its previous revision, even though nothing was wrong
/// with the data or the disk.
///
/// `keep` disarms the temporary's delete-on-drop; on a rename failure the now
/// unowned file is removed explicitly so a failed save leaves no debris. The
/// file handle is dropped before the rename purely for tidiness: POSIX-semantics
/// rename does not require it (measured — the same 300/300 succeeded with the
/// handle still open).
fn replace_with_temporary(temporary: tempfile::NamedTempFile, destination: &Path) -> Result<()> {
    let (file, path) = temporary.keep().map_err(|e| Error::from(e.error))?;
    drop(file);
    match fs::rename(&path, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&path);
            Err(Error::from(error))
        }
    }
}

pub mod checkpoint;
pub mod history;
pub mod recovery;
pub mod retention;
pub mod service;

pub const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_REPLACEMENT_BYTES: usize = 64 * 1024;
pub const MAX_STORE_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_EDIT_IDS: usize = 4096;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Error {
    pub code: String,
    pub message: String,
}
impl Error {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new("storage_error", e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

/// Wire-compatible with transfer-v1 `PreparedEdit`; no dependency on the bridge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedEdit {
    pub capture_id: String,
    pub edit_id: String,
    pub project_id: String,
    pub path: String,
    pub expected_revision: u64,
    pub start_byte: usize,
    pub end_byte: usize,
    pub removed_text: String,
    pub replacement: String,
    pub document_before_sha256: String,
}

/// Send this payload as `capture_applied`, only after `apply` returns success.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppliedReceipt {
    pub capture_id: String,
    pub edit_id: String,
    pub new_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Document {
    pub project_id: String,
    pub path: String,
    pub revision: u64,
    pub text: String,
    pub source_sha256: String,
}
impl Document {
    pub fn new(project_id: String, path: String, revision: u64, text: String) -> Result<Self> {
        let value = Self {
            source_sha256: digest(&text),
            project_id,
            path,
            revision,
            text,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        identifier(&self.project_id)?;
        relative_path(&self.path)?;
        if self.text.len() > MAX_DOCUMENT_BYTES || self.source_sha256 != digest(&self.text) {
            return Err(Error::new("invalid_document", "document size/hash invalid"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppliedTransaction {
    pub edit: PreparedEdit,
    pub receipt: AppliedReceipt,
    /// Retained until the bridge acknowledges the matching receipt.
    pub document_before: Option<Document>,
    pub document_after_sha256: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct State {
    schema_version: u8,
    document: Document,
    transactions: BTreeMap<String, AppliedTransaction>,
    #[serde(default)]
    retained_ids: BTreeMap<String, retention::RetainedEditId>,
    #[serde(default)]
    history: history::HistoryState,
    #[serde(default)]
    store_id: String,
}

pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(Error::new(
            "invalid_id",
            "expected 1–128 ASCII letters, digits, hyphens or underscores",
        ));
    }
    Ok(())
}
fn relative_path(value: &str) -> Result<()> {
    if value.is_empty()
        || value.contains(['\\', ':', '\0'])
        || value
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(Error::new(
            "invalid_path",
            "expected normalized relative path",
        ));
    }
    Ok(())
}

fn apply_to(document: &Document, edit: &PreparedEdit) -> Result<Document> {
    identifier(&edit.capture_id)?;
    identifier(&edit.edit_id)?;
    if edit.project_id != document.project_id || edit.path != document.path {
        return Err(Error::new(
            "document_conflict",
            "edit targets another document",
        ));
    }
    if edit.expected_revision != document.revision {
        return Err(Error::new("revision_conflict", "edit revision is stale"));
    }
    if edit.document_before_sha256 != digest(&document.text) {
        return Err(Error::new(
            "source_hash_conflict",
            "source differs from prepared snapshot",
        ));
    }
    let Some(removed) = document.text.get(edit.start_byte..edit.end_byte) else {
        return Err(Error::new(
            "invalid_source_range",
            "range must be ordered UTF-8 scalar boundaries within source",
        ));
    };
    if removed != edit.removed_text {
        return Err(Error::new(
            "removed_text_conflict",
            "selected source differs from removed_text",
        ));
    }
    if edit.replacement.len() > MAX_REPLACEMENT_BYTES {
        return Err(Error::new(
            "replacement_too_large",
            "replacement exceeds 64 KiB",
        ));
    }
    let revision = document
        .revision
        .checked_add(1)
        .ok_or_else(|| Error::new("revision_overflow", "revision exhausted"))?;
    let mut text = document.text.clone();
    text.replace_range(edit.start_byte..edit.end_byte, &edit.replacement);
    Document::new(
        document.project_id.clone(),
        document.path.clone(),
        revision,
        text,
    )
}

impl State {
    fn validate(&self) -> Result<()> {
        self.document.validate()?;
        if !matches!(self.schema_version, 1..=4)
            || (self.schema_version == 1 && !self.retained_ids.is_empty())
            || self.transactions.len() + self.retained_ids.len() > MAX_EDIT_IDS
        {
            return Err(Error::new(
                "invalid_store",
                "unknown schema or too many edit IDs",
            ));
        }
        let mut capture_ids = std::collections::BTreeSet::new();
        for (id, tx) in &self.transactions {
            identifier(id)?;
            identifier(&tx.edit.capture_id)?;
            if id != &tx.edit.edit_id
                || tx.receipt.edit_id != *id
                || tx.receipt.capture_id != tx.edit.capture_id
                || tx.edit.expected_revision.checked_add(1) != Some(tx.receipt.new_revision)
                || tx.receipt.new_revision > self.document.revision
                || tx.edit.project_id != self.document.project_id
                || tx.edit.path != self.document.path
                || !capture_ids.insert(&tx.edit.capture_id)
                || tx.document_after_sha256.len() != 64
                || !tx
                    .document_after_sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
            {
                return Err(Error::new(
                    "invalid_store",
                    "transaction identity/revision mismatch",
                ));
            }
            match (&tx.document_before, tx.confirmed) {
                (Some(before), false) => {
                    before.validate()?;
                    let after = apply_to(before, &tx.edit)?;
                    if after.source_sha256 != tx.document_after_sha256 {
                        return Err(Error::new(
                            "invalid_store",
                            "transaction source hash mismatch",
                        ));
                    }
                }
                (None, true) => {}
                _ => {
                    return Err(Error::new(
                        "invalid_store",
                        "pending receipt lacks recovery snapshot",
                    ))
                }
            }
            if tx.receipt.new_revision == self.document.revision
                && tx.document_after_sha256 != self.document.source_sha256
            {
                return Err(Error::new(
                    "invalid_store",
                    "current document disagrees with latest transaction",
                ));
            }
        }
        for (id, retained) in &self.retained_ids {
            identifier(id)?;
            identifier(&retained.receipt.capture_id)?;
            if self.transactions.contains_key(id)
                || id != &retained.receipt.edit_id
                || !capture_ids.insert(&retained.receipt.capture_id)
                || retained.receipt.new_revision > self.document.revision
                || retained.prepared_sha256.len() != 64
                || !retained
                    .prepared_sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
                || retained.document_after_sha256.len() != 64
                || !retained
                    .document_after_sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
                || (retained.receipt.new_revision == self.document.revision
                    && retained.document_after_sha256 != self.document.source_sha256)
            {
                return Err(Error::new(
                    "invalid_store",
                    "retained edit-ID identity/hash mismatch",
                ));
            }
        }
        self.history.validate(&self.document)?;
        if (self.schema_version < 4 && !self.store_id.is_empty())
            || (self.schema_version == 4
                && (self.store_id.len() != 64
                    || !self.store_id.bytes().all(|b| b.is_ascii_hexdigit())))
        {
            return Err(Error::new(
                "invalid_store",
                "persistent store identity requires schema 4",
            ));
        }
        if self.schema_version < 3 && !self.history.is_empty() {
            return Err(Error::new("invalid_store", "history requires schema 3"));
        }
        Ok(())
    }
}

/// One writer per directory. The parent directory must already exist.
/// Treat `document()` as authoritative; a separate `.tex` file is an export.
pub struct Store {
    root: PathBuf,
    _lock: File,
    state: Option<State>,
    poisoned: bool,
    #[cfg(test)]
    failpoint: Option<&'static str>,
}
impl Drop for Store {
    fn drop(&mut self) {
        // flock belongs to the shared open-file description. A concurrently
        // forked child may hold that description until exec even with CLOEXEC.
        // Explicitly relinquish ownership rather than waiting for every inherited
        // descriptor to close. The child is not an authorized store writer.
        let _ = FileExt::unlock(&self._lock);
    }
}
impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&root) {
            Ok(()) => open_dir_for_sync(
                root.parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(Path::new(".")),
            )?
            .sync_all()?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && root.is_dir() => {}
            Err(e) => return Err(e.into()),
        }
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(root.join(".edit-ledger.lock"))?;
        lock.try_lock_exclusive()
            .map_err(|_| Error::new("store_in_use", "another process owns this document store"))?;
        let state = match File::open(root.join("document.json")) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(MAX_STORE_BYTES + 1).read_to_end(&mut bytes)?;
                if bytes.len() as u64 > MAX_STORE_BYTES {
                    return Err(Error::new("invalid_store", "store size limit exceeded"));
                }
                let state: State = serde_json::from_slice(&bytes)
                    .map_err(|e| Error::new("invalid_store", e.to_string()))?;
                state.validate()?;
                Some(state)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        // Reestablish durability if a prior writer died after rename but before
        // syncing the directory. No receipt can escape open() before this gate.
        if state.is_some() {
            // `sync_all`'s `FlushFileBuffers` requires write access on the
            // handle on Windows, even for a file this call never writes to
            // (measured; see `open_dir_for_sync` above for the same rule
            // applied to directory handles).
            OpenOptions::new()
                .write(true)
                .open(root.join("document.json"))?
                .sync_all()?;
        }
        open_dir_for_sync(&root)?.sync_all()?;
        Ok(Self {
            root,
            _lock: lock,
            state,
            poisoned: false,
            #[cfg(test)]
            failpoint: None,
        })
    }

    fn ready(&self) -> Result<()> {
        if self.poisoned {
            return Err(Error::new(
                "recovery_required",
                "persistence failed; drop handle and reopen before retry",
            ));
        }
        Ok(())
    }
    pub fn document(&self) -> Result<Option<&Document>> {
        self.ready()?;
        Ok(self.state.as_ref().map(|s| &s.document))
    }
    /// Idempotent only for an exactly matching existing snapshot. Never resets a ledger.
    pub fn initialize(&mut self, document: Document) -> Result<()> {
        self.ready()?;
        document.validate()?;
        if let Some(state) = &self.state {
            return if state.document == document {
                Ok(())
            } else {
                Err(Error::new(
                    "document_exists",
                    "load existing durable source instead of overwriting",
                ))
            };
        }
        self.commit(State {
            schema_version: 4,
            document,
            transactions: BTreeMap::new(),
            retained_ids: BTreeMap::new(),
            history: history::HistoryState::default(),
            store_id: checkpoint::new_store_id()?,
        })
    }
    /// Atomic source+ledger application. An identical retry returns its original
    /// receipt even if the document has since advanced; it never inserts twice.
    pub fn apply(&mut self, edit: PreparedEdit) -> Result<AppliedReceipt> {
        self.ready()?;
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| Error::new("document_missing", "initialize source first"))?;
        if let Some(retained) = state.retained_ids.get(&edit.edit_id) {
            return if retained.prepared_sha256 == retention::prepared_digest(&edit)? {
                Ok(retained.receipt.clone())
            } else {
                Err(Error::new(
                    "edit_id_conflict",
                    "retained edit ID binds different prepared fields",
                ))
            };
        }
        if let Some(tx) = state.transactions.get(&edit.edit_id) {
            return if tx.edit == edit {
                Ok(tx.receipt.clone())
            } else {
                Err(Error::new(
                    "edit_id_conflict",
                    "edit ID already binds different prepared fields",
                ))
            };
        }
        if state
            .transactions
            .values()
            .any(|t| t.edit.capture_id == edit.capture_id)
            || state
                .retained_ids
                .values()
                .any(|t| t.receipt.capture_id == edit.capture_id)
        {
            return Err(Error::new(
                "capture_id_conflict",
                "capture already applied under another edit ID",
            ));
        }
        if state.transactions.len() + state.retained_ids.len() >= MAX_EDIT_IDS {
            return Err(Error::new(
                "ledger_full",
                "edit IDs cannot be evicted; archive the document explicitly",
            ));
        }
        let after = apply_to(&state.document, &edit)?;
        let receipt = AppliedReceipt {
            capture_id: edit.capture_id.clone(),
            edit_id: edit.edit_id.clone(),
            new_revision: after.revision,
        };
        let tx = AppliedTransaction {
            edit,
            receipt: receipt.clone(),
            document_before: Some(state.document.clone()),
            document_after_sha256: after.source_sha256.clone(),
            confirmed: false,
        };
        let mut next = state.clone();
        next.document = after;
        next.transactions.insert(receipt.edit_id.clone(), tx);
        history::record(
            &mut next,
            &state.document,
            format!("Capture {}", receipt.capture_id),
        )?;
        self.commit(next)?;
        Ok(receipt)
    }
    /// Durable ordinary typing/undo; it does not remove any applied-ID tombstone.
    pub fn replace_document(
        &mut self,
        expected_revision: u64,
        expected_sha256: &str,
        text: String,
    ) -> Result<Document> {
        self.replace_document_in_place(expected_revision, expected_sha256, text)?;
        Ok(self
            .document()?
            .ok_or_else(|| Error::new("document_missing", "saved source missing"))?
            .clone())
    }
    /// Same durable edit as replace_document, without cloning a response document.
    pub fn replace_document_in_place(
        &mut self,
        expected_revision: u64,
        expected_sha256: &str,
        text: String,
    ) -> Result<()> {
        self.ready()?;
        let mut next = self
            .state
            .clone()
            .ok_or_else(|| Error::new("document_missing", "initialize source first"))?;
        let before = next.document.clone();
        if next.document.revision != expected_revision
            || next.document.source_sha256 != expected_sha256
        {
            return Err(Error::new(
                "document_conflict",
                "ordinary edit snapshot is stale",
            ));
        }
        let revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| Error::new("revision_overflow", "revision exhausted"))?;
        next.document = Document::new(
            next.document.project_id.clone(),
            next.document.path.clone(),
            revision,
            text,
        )?;
        history::record(&mut next, &before, "Source edit".into())?;
        self.commit(next)?;
        Ok(())
    }
    /// Keep a recovery snapshot until an exactly matching bridge acknowledgement.
    pub fn confirm(&mut self, receipt: &AppliedReceipt) -> Result<()> {
        self.ready()?;
        let mut next = self
            .state
            .clone()
            .ok_or_else(|| Error::new("document_missing", "initialize source first"))?;
        if let Some(retained) = next.retained_ids.get(&receipt.edit_id) {
            return if retained.receipt == *receipt {
                Ok(())
            } else {
                Err(Error::new(
                    "receipt_conflict",
                    "acknowledgement differs from retained receipt",
                ))
            };
        }
        let tx = next
            .transactions
            .get_mut(&receipt.edit_id)
            .ok_or_else(|| Error::new("edit_missing", "receipt does not name an applied edit"))?;
        if tx.receipt != *receipt {
            return Err(Error::new(
                "receipt_conflict",
                "acknowledgement differs from durable receipt",
            ));
        }
        if tx.confirmed {
            return Ok(());
        }
        tx.confirmed = true;
        tx.document_before = None;
        self.commit(next)
    }
    /// In revision order: ask bridge capture_status first; reopen each original
    /// snapshot before replaying a missing receipt, then reopen the current source.
    /// A transport failure must leave these entries untouched.
    pub fn recovery(&self) -> Result<Vec<AppliedTransaction>> {
        self.ready()?;
        let mut pending: Vec<_> = self
            .state
            .iter()
            .flat_map(|s| s.transactions.values())
            .filter(|t| !t.confirmed)
            .cloned()
            .collect();
        pending.sort_by_key(|t| t.receipt.new_revision);
        Ok(pending)
    }

    fn commit(&mut self, next: State) -> Result<()> {
        next.validate()?;
        let bytes =
            serde_json::to_vec(&next).map_err(|e| Error::new("invalid_store", e.to_string()))?;
        if bytes.len() as u64 > MAX_STORE_BYTES {
            return Err(Error::new(
                "ledger_full",
                "durable store size limit exceeded",
            ));
        }
        if let Err(error) = self.persist(&bytes) {
            self.poisoned = true;
            return Err(error);
        }
        self.state = Some(next);
        Ok(())
    }
    fn persist(&self, bytes: &[u8]) -> Result<()> {
        let mut temporary = tempfile::NamedTempFile::new_in(&self.root)?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        #[cfg(test)]
        self.inject("before_rename")?;
        replace_with_temporary(temporary, &self.root.join("document.json"))?;
        #[cfg(test)]
        self.inject("after_rename")?;
        open_dir_for_sync(&self.root)?.sync_all()?;
        Ok(())
    }
    #[cfg(test)]
    fn inject(&self, point: &'static str) -> Result<()> {
        if self.failpoint == Some(point) {
            return Err(Error::new("injected_io_error", point));
        }
        Ok(())
    }
}

#[cfg(test)]
mod cost_benchmark;
#[cfg(test)]
mod tests;
