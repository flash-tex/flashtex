//! One-writer, atomic capture journal. Receipts are emitted only after fsync.
use crate::{identifier, BridgeError, CaptureRecord, Result};
use fs2::FileExt;
use std::{
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
/// Not equivalent on Windows: `persist` calls `MoveFileExW
/// (MOVEFILE_REPLACE_EXISTING)`, which needs `DELETE` access on the existing
/// file and fails with `ERROR_ACCESS_DENIED` whenever any other handle to it
/// is open (a reader, e.g.). `std::fs::rename` asks for `FileRenameInfoEx`
/// with POSIX semantics instead (Windows 10 1607+, falling back to
/// `MoveFileExW`), which unlinks the old name and lets existing readers keep
/// reading their now-nameless handle — ordinary POSIX `rename(2)` behavior.
/// Measured in `crates/edit-ledger` (same bug, same fix): 300 replacements
/// against a concurrent reader failed 231/300 via `persist`, 0/300 via
/// `fs::rename`.
fn replace_with_temporary(temporary: tempfile::NamedTempFile, destination: &Path) -> Result<()> {
    let (file, path) = temporary.keep().map_err(|e| BridgeError::from(e.error))?;
    drop(file);
    match fs::rename(&path, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&path);
            Err(BridgeError::from(error))
        }
    }
}

pub struct Store {
    root: PathBuf,
    _lock: File,
}
impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_owned();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&root)?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(&root)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join(".bridge.lock"))?;
        lock.try_lock_exclusive().map_err(|_| {
            BridgeError::new(
                "store_in_use",
                "Another bridge process owns this capture journal",
            )
        })?;
        Ok(Self { root, _lock: lock })
    }
    pub fn get(&self, id: &str) -> Result<Option<CaptureRecord>> {
        identifier(id)?;
        let path = self.root.join(format!("{id}.json"));
        let file = match File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let mut bytes = Vec::new();
        file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(BridgeError::new(
                "invalid_journal",
                "Capture journal record exceeds its size limit",
            ));
        }
        let record: CaptureRecord = serde_json::from_slice(&bytes)?;
        if record.schema_version != 1
            || record.capture.capture_id != id
            || record.request_sha256 != crate::digest(&serde_json::to_vec(&record.capture)?)
        {
            return Err(BridgeError::new(
                "invalid_journal",
                "Capture journal identity or integrity check failed",
            ));
        }
        // Disk recovery must satisfy the same proposal bounds as a fresh
        // provider response before any cached conversion can bypass validation.
        if let Some(proposal) = &record.proposal {
            proposal.validate().map_err(|error| {
                BridgeError::new(
                    "invalid_journal",
                    format!("Saved proposal is invalid: {}", error.message),
                )
            })?;
        }
        validate_derived_state(&record)?;
        Ok(Some(record))
    }
    pub fn require(&self, id: &str) -> Result<CaptureRecord> {
        self.get(id)?.ok_or_else(|| {
            BridgeError::new("capture_missing", "Capture has not been durably received")
        })
    }
    pub fn save(&mut self, record: &CaptureRecord) -> Result<()> {
        identifier(&record.capture.capture_id)?;
        let mut temporary = tempfile::NamedTempFile::new_in(&self.root)?;
        serde_json::to_writer(&mut temporary, record)?;
        temporary.write_all(b"\n")?;
        temporary.as_file().sync_all()?;
        replace_with_temporary(
            temporary,
            &self.root.join(format!("{}.json", record.capture.capture_id)),
        )?;
        open_dir_for_sync(&self.root)?.sync_all()?;
        Ok(())
    }
}

// Validate durable state before any of it can authorize a replay. Missing legacy
// context/binding remains readable so the bridge can request safe reselection.
fn validate_derived_state(record: &CaptureRecord) -> Result<()> {
    let invalid = || {
        BridgeError::new(
            "invalid_journal",
            "Saved capture state has contradictory or invalid fields",
        )
    };
    let hash_valid = |hash: &str| {
        hash.len() == 64
            && hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    if record.rejected && (record.prepared.is_some() || record.applied.is_some()) {
        return Err(invalid());
    }
    if let Some(binding) = &record.destination_binding {
        if identifier(&binding.project_id).is_err()
            || crate::relative_path(&binding.path).is_err()
            || binding.start_byte > binding.end_byte
            || binding.end_byte > crate::MAX_DOCUMENT_BYTES
            || !hash_valid(&binding.source_sha256)
            || binding.revision != record.capture.base_revision
        {
            return Err(invalid());
        }
    }
    if let Some(context) = &record.context {
        if identifier(&context.project_id).is_err() || crate::relative_path(&context.path).is_err()
        {
            return Err(invalid());
        }
        if let Some(binding) = &record.destination_binding {
            if context.project_id != binding.project_id
                || context.path != binding.path
                || context.revision < binding.revision
            {
                return Err(invalid());
            }
        }
        let mut paths = std::collections::BTreeSet::new();
        for dependency in &context.dependencies {
            if crate::relative_path(&dependency.path).is_err()
                || !hash_valid(&dependency.source_sha256)
                || !paths.insert(&dependency.path)
            {
                return Err(invalid());
            }
        }
    }
    if let Some(edit) = &record.prepared {
        let Some(proposal) = &record.proposal else {
            return Err(invalid());
        };
        if edit.capture_id != record.capture.capture_id
            || edit.edit_id != format!("capture-{}", record.capture.capture_id)
            || identifier(&edit.project_id).is_err()
            || crate::relative_path(&edit.path).is_err()
            || edit.start_byte > edit.end_byte
            || edit.end_byte > crate::MAX_DOCUMENT_BYTES
            || edit.end_byte - edit.start_byte != edit.removed_text.len()
            || edit.replacement != proposal.latex
            || !hash_valid(&edit.document_before_sha256)
        {
            return Err(invalid());
        }
        if let Some(binding) = &record.destination_binding {
            if edit.project_id != binding.project_id
                || edit.path != binding.path
                || edit.expected_revision < binding.revision
            {
                return Err(invalid());
            }
        }
        if let Some(context) = &record.context {
            if edit.project_id != context.project_id
                || edit.path != context.path
                || edit.expected_revision < context.revision
            {
                return Err(invalid());
            }
        }
    }
    if let Some(receipt) = &record.applied {
        let Some(edit) = &record.prepared else {
            return Err(invalid());
        };
        if receipt.edit_id != edit.edit_id || receipt.new_revision <= edit.expected_revision {
            return Err(invalid());
        }
    }
    Ok(())
}
