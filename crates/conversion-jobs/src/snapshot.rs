//! Atomic metadata checkpoints with explicit, conservative restart authorization.
use crate::*;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

/// Opens `path` (a directory) so `sync_all` can fsync it after a rename.
///
/// Plain `File::open` cannot open a directory on Windows at all (it fails
/// with `ERROR_ACCESS_DENIED`, unlike POSIX's `open(2)`); it needs
/// `FILE_FLAG_BACKUP_SEMANTICS`. A read-only handle isn't enough either:
/// `sync_all`'s `FlushFileBuffers` itself requires write access on the
/// handle, or it fails with the same `ERROR_ACCESS_DENIED` (measured). See
/// `crates/project-files/src/sys.rs`'s `open_dir_std`/`open_at` for the same
/// two fixes applied there. Local to this file rather than shared with
/// `bridge_adapter::open_dir_for_sync` because that module is gated behind
/// the `bridge-integration` feature and this one is not.
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
/// rather than `NamedTempFile::persist` — not equivalent on Windows, where
/// `persist`'s `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` needs `DELETE` access
/// on the existing file and fails if any other handle to it is open, unlike
/// `fs::rename`'s POSIX-semantics replace. See `crates/edit-ledger/src/lib.rs`'s
/// `replace_with_temporary` for the measured failure-rate comparison.
fn replace_with_temporary(
    temporary: tempfile::NamedTempFile,
    destination: &Path,
) -> std::io::Result<()> {
    let (file, path) = temporary.keep().map_err(|e| e.error)?;
    drop(file);
    match fs::rename(&path, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&path);
            Err(error)
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryReason {
    QueuedAwaitingAuthorization,
    ProviderMayHaveCompleted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeAuthorization {
    QueuedOnly,
    ReconciledPossibleProviderCompletion,
}
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub snapshot_bytes: usize,
    pub result_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            snapshot_bytes: 1024 * 1024,
            result_bytes: 64 * 1024,
        }
    }
}
#[derive(Debug)]
pub enum SnapshotError {
    Io(String),
    Codec(String),
    Invalid(String),
    Scheduler(Error),
}
impl From<std::io::Error> for SnapshotError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u8,
    jobs: Vec<SavedJob>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedJob {
    id: String,
    expected: ContextFingerprint,
    current: ContextFingerprint,
    state: SavedState,
}
#[derive(Serialize, Deserialize)]
enum SavedState {
    Recovery(RecoveryReason),
    Completed(Vec<u8>),
    Failed(Failure),
    Cancelled,
}
impl<T: Send + Sync + 'static> Scheduler<T> {
    /// Caller owns this private path and serializes checkpoint writers. Callbacks
    /// execute outside the scheduler mutex. The codec must impose its own allocation
    /// limits; encoded output is additionally bounded before atomic persistence.
    pub fn save_snapshot(
        &self,
        path: impl AsRef<Path>,
        limits: Limits,
        encode: impl Fn(&T) -> Result<Vec<u8>, String>,
    ) -> Result<(), SnapshotError> {
        check_limits(limits)?;
        let jobs = {
            let data = self.shared.data.lock().unwrap();
            data.jobs
                .iter()
                .map(|(id, j)| {
                    (
                        id.clone(),
                        j.expected.clone(),
                        j.current.clone(),
                        j.state.clone(),
                        j.executing,
                    )
                })
                .collect::<Vec<_>>()
        };
        let mut saved = Vec::new();
        for (id, expected, current, state, executing) in jobs {
            let state = if executing {
                SavedState::Recovery(RecoveryReason::ProviderMayHaveCompleted)
            } else {
                match state {
                    State::Queued => {
                        SavedState::Recovery(RecoveryReason::QueuedAwaitingAuthorization)
                    }
                    State::Running => {
                        SavedState::Recovery(RecoveryReason::ProviderMayHaveCompleted)
                    }
                    State::RecoveryRequired(reason) => SavedState::Recovery(reason),
                    State::Completed(value) => {
                        let bytes = encode(&value).map_err(SnapshotError::Codec)?;
                        if bytes.len() > limits.result_bytes {
                            return Err(invalid("Result exceeds codec byte budget"));
                        }
                        SavedState::Completed(bytes)
                    }
                    State::Failed(failure)
                        if matches!(
                            failure.kind,
                            FailureKind::AmbiguousProvider | FailureKind::Panicked
                        ) =>
                    {
                        SavedState::Recovery(RecoveryReason::ProviderMayHaveCompleted)
                    }
                    State::Failed(failure) => SavedState::Failed(failure),
                    State::Cancelled => SavedState::Cancelled,
                }
            };
            saved.push(SavedJob {
                id,
                expected,
                current,
                state,
            });
        }
        let bytes = serde_json::to_vec(&Snapshot {
            version: 1,
            jobs: saved,
        })
        .map_err(|e| invalid(&e.to_string()))?;
        if bytes.len() > limits.snapshot_bytes {
            return Err(invalid("Snapshot exceeds byte budget"));
        }
        let path = path.as_ref();
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        replace_with_temporary(temporary, path).map_err(SnapshotError::from)?;
        open_dir_for_sync(parent)?.sync_all()?;
        Ok(())
    }
    pub fn restore_snapshot(
        path: impl AsRef<Path>,
        workers: usize,
        max_queued: usize,
        max_retained: usize,
        limits: Limits,
        decode: impl Fn(&[u8]) -> Result<T, String>,
    ) -> Result<Self, SnapshotError> {
        check_limits(limits)?;
        let mut bytes = Vec::new();
        File::open(path)?
            .take(limits.snapshot_bytes as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > limits.snapshot_bytes {
            return Err(invalid("Snapshot exceeds byte budget"));
        }
        let saved: Snapshot =
            serde_json::from_slice(&bytes).map_err(|e| invalid(&e.to_string()))?;
        if saved.version != 1 || saved.jobs.len() > max_retained {
            return Err(invalid("Unsupported version or too many job records"));
        }
        let mut jobs = BTreeMap::new();
        for j in saved.jobs {
            if j.id.is_empty()
                || j.id.len() > 128
                || !j
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                || jobs.contains_key(&j.id)
            {
                return Err(invalid("Invalid or duplicate identity"));
            }
            validate_fingerprint(&j.expected).map_err(SnapshotError::Scheduler)?;
            validate_fingerprint(&j.current).map_err(SnapshotError::Scheduler)?;
            let state = match j.state {
                SavedState::Recovery(reason) => State::RecoveryRequired(reason),
                SavedState::Completed(bytes) => {
                    if bytes.len() > limits.result_bytes {
                        return Err(invalid("Result exceeds codec byte budget"));
                    }
                    if j.expected != j.current {
                        State::Failed(stale())
                    } else {
                        State::Completed(Arc::new(decode(&bytes).map_err(SnapshotError::Codec)?))
                    }
                }
                SavedState::Failed(failure) => {
                    if matches!(
                        failure.kind,
                        FailureKind::AmbiguousProvider | FailureKind::Panicked
                    ) {
                        State::RecoveryRequired(RecoveryReason::ProviderMayHaveCompleted)
                    } else {
                        State::Failed(failure)
                    }
                }
                SavedState::Cancelled => State::Cancelled,
            };
            jobs.insert(
                j.id,
                Job {
                    expected: j.expected,
                    current: j.current,
                    state,
                    token: CancellationToken(Arc::new(AtomicBool::new(false)), None),
                    task: None,
                    executing: false,
                },
            );
        }
        let scheduler =
            Self::new(workers, max_queued, max_retained).map_err(SnapshotError::Scheduler)?;
        scheduler.shared.data.lock().unwrap().jobs = jobs;
        Ok(scheduler)
    }
    /// Caller must first verify the previous process/worker is terminal. A snapshot
    /// is not a distributed lease; restoration while that worker runs duplicates work.
    pub fn authorize_resume(
        &self,
        id: &str,
        authorization: ResumeAuthorization,
        context: ContextFingerprint,
        convert: impl FnOnce(CancellationToken) -> Result<T, Failure> + Send + 'static,
    ) -> Result<(), Error> {
        validate_fingerprint(&context)?;
        let mut data = self.shared.data.lock().unwrap();
        if data.stopped {
            return Err(Error::Stopped);
        }
        if data.queue.len() >= self.max_queued {
            return Err(Error::QueueFull);
        }
        let job = data.jobs.get_mut(id).ok_or(Error::Missing)?;
        match &job.state {
            State::RecoveryRequired(RecoveryReason::QueuedAwaitingAuthorization) => (),
            State::RecoveryRequired(RecoveryReason::ProviderMayHaveCompleted)
                if authorization == ResumeAuthorization::ReconciledPossibleProviderCompletion => {}
            _ => return Err(Error::RecoveryAuthorizationRequired),
        }
        job.expected = context.clone();
        job.current = context;
        job.state = State::Queued;
        job.token = CancellationToken(
            Arc::new(AtomicBool::new(false)),
            Some(events::Reporter {
                id: id.into(),
                hub: self.shared.events.clone(),
            }),
        );
        job.task = Some(Box::new(convert));
        self.shared.events.emit(id, events::EventKind::Queued);
        data.queue.push_back(id.into());
        self.shared.wake.notify_one();
        Ok(())
    }
}
fn invalid(message: &str) -> SnapshotError {
    SnapshotError::Invalid(message.into())
}
fn check_limits(limits: Limits) -> Result<(), SnapshotError> {
    if limits.snapshot_bytes == 0
        || limits.snapshot_bytes > 64 * 1024 * 1024
        || limits.result_bytes > limits.snapshot_bytes
    {
        Err(invalid("Invalid byte budgets"))
    } else {
        Ok(())
    }
}
