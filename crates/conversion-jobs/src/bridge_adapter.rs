pub mod native;
pub mod review;
use crate::*;
use flashtex_bridge::{Bridge, CaptureSubmit, Context, Proposal};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
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
pub(crate) fn open_dir_for_sync(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(not(windows))]
pub(crate) fn open_dir_for_sync(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

/// Atomically replaces `destination` with `temporary`, via `std::fs::rename`
/// rather than `NamedTempFile::persist` — not equivalent on Windows, where
/// `persist`'s `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` needs `DELETE` access
/// on the existing file and fails if any other handle to it is open, unlike
/// `fs::rename`'s POSIX-semantics replace. See `crates/edit-ledger/src/lib.rs`'s
/// `replace_with_temporary` for the measured failure-rate comparison.
pub(crate) fn replace_with_temporary(
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

pub type Provider = dyn Fn(CaptureSubmit, Context, CancellationToken) -> std::result::Result<Proposal, Failure>
    + Send
    + Sync
    + 'static;
#[derive(Debug, Clone)]
pub enum AdapterState {
    Queued,
    Running,
    AwaitingJournal,
    Proposal(Arc<Proposal>),
    Failed(Failure),
    Cancelled,
    RecoveryRequired,
    StaleContext,
    Prepared,
    Applied,
    Rejected,
}
#[derive(Debug)]
pub enum AdapterError {
    Bridge(flashtex_bridge::BridgeError),
    Io(std::io::Error),
    Scheduler(Error),
    InvalidState,
    RetentionFull,
}
impl From<flashtex_bridge::BridgeError> for AdapterError {
    fn from(e: flashtex_bridge::BridgeError) -> Self {
        Self::Bridge(e)
    }
}
impl From<std::io::Error> for AdapterError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<Error> for AdapterError {
    fn from(e: Error) -> Self {
        Self::Scheduler(e)
    }
}
type Result<T> = std::result::Result<T, AdapterError>;
#[derive(Clone)]
pub struct StatusHandle {
    shared: Arc<Shared<Proposal>>,
    settled: Arc<Mutex<BTreeMap<String, AdapterState>>>,
}
impl StatusHandle {
    /// Does not wait for conversion, codecs, checkpoint writes or bridge IO.
    pub fn status(&self, id: &str) -> Result<AdapterState> {
        if let Some(state) = self.settled.lock().unwrap().get(id) {
            return Ok(state.clone());
        }
        let data = self.shared.data.lock().unwrap();
        let job = data.jobs.get(id).ok_or(Error::Missing)?;
        if data.stopped && matches!(job.state, State::Queued | State::Running) {
            return Ok(AdapterState::Cancelled);
        }
        Ok(match &job.state {
            State::Queued => AdapterState::Queued,
            State::Running => AdapterState::Running,
            State::Completed(_) => AdapterState::AwaitingJournal,
            State::Failed(f) => AdapterState::Failed(f.clone()),
            State::Cancelled => AdapterState::Cancelled,
            State::RecoveryRequired(_) => AdapterState::RecoveryRequired,
        })
    }
}
struct Input {
    capture: CaptureSubmit,
    context: Context,
}
pub struct BridgeAdapter {
    scheduler: Scheduler<Proposal>,
    status: StatusHandle,
    inputs: BTreeMap<String, Input>,
    directory: PathBuf,
    provider: Arc<Provider>,
    retained: usize,
}
impl BridgeAdapter {
    /// Create on the background document/journal actor. Provider closures must set
    /// their own network deadlines and heed cancellation; no implicit retry exists.
    pub fn new(
        directory: impl AsRef<Path>,
        workers: usize,
        max_queued: usize,
        max_retained: usize,
        provider: Arc<Provider>,
    ) -> Result<Self> {
        let directory = directory.as_ref().to_owned();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&directory)?;
        }
        #[cfg(not(unix))]
        fs::create_dir_all(&directory)?;
        let scheduler = Scheduler::new(workers, max_queued, max_retained)?;
        let status = StatusHandle {
            shared: scheduler.shared.clone(),
            settled: Arc::new(Mutex::new(BTreeMap::new())),
        };
        Ok(Self {
            scheduler,
            status,
            inputs: BTreeMap::new(),
            directory,
            provider,
            retained: max_retained,
        })
    }
    pub fn status_handle(&self) -> StatusHandle {
        self.status.clone()
    }
    /// The bridge must already have durably received this capture. Intents are
    /// exclusively created and fsynced BEFORE any provider worker can be started.
    pub fn start(
        &mut self,
        bridge: &Bridge,
        id: &str,
        supported: Vec<String>,
    ) -> Result<AdapterState> {
        if self.inputs.contains_key(id) || self.status.settled.lock().unwrap().contains_key(id) {
            return self.status.status(id);
        }
        let count = self.inputs.len()
            + self
                .status
                .settled
                .lock()
                .unwrap()
                .keys()
                .filter(|id| !self.inputs.contains_key(*id))
                .count();
        if count >= self.retained {
            return Err(AdapterError::RetentionFull);
        }
        let record = bridge.store.require(id)?;
        if record.rejected {
            return Ok(self.settle(id, AdapterState::Rejected));
        }
        if record.applied.is_some() {
            return Ok(self.settle(id, AdapterState::Applied));
        }
        if record.prepared.is_some() {
            return Ok(self.settle(id, AdapterState::Prepared));
        }
        let context = bridge.context(&record.capture, supported)?;
        if let Some(proposal) = record.proposal {
            let state = if record.context.as_ref() == Some(&context) {
                AdapterState::Proposal(Arc::new(proposal))
            } else {
                AdapterState::StaleContext
            };
            return Ok(self.settle(id, state));
        }
        let fingerprint = fingerprint(&context)?;
        let intent = self.directory.join(format!("{id}.intent.json"));
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&intent)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Ok(self.settle(id, AdapterState::RecoveryRequired))
            }
            Err(e) => return Err(e.into()),
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        let bytes=serde_json::to_vec(&serde_json::json!({"schema_version":1,"capture_id":id,"context":fingerprint,"state":"provider_may_have_started"})).map_err(|_|AdapterError::InvalidState)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        open_dir_for_sync(&self.directory)?.sync_all()?;
        let capture = record.capture.clone();
        let input_context = context.clone();
        let provider = self.provider.clone();
        match self.scheduler.submit(id, fingerprint, move |token| {
            let proposal = provider(capture, input_context, token)?;
            proposal
                .validate()
                .map_err(|e| Failure::new(FailureKind::Provider, e.message))?;
            Ok(proposal)
        }) {
            Ok(()) => {
                self.inputs.insert(
                    id.into(),
                    Input {
                        capture: record.capture,
                        context,
                    },
                );
                self.status.status(id)
            }
            Err(e) => {
                // Rejected submission proves no closure was executed. Remove this
                // newly created intent so backpressure can be retried explicitly.
                fs::remove_file(intent)?;
                open_dir_for_sync(&self.directory)?.sync_all()?;
                Err(e.into())
            }
        }
    }
    /// Invoke on the document actor after edits and before accepting a result.
    /// Pending intent remains durable on stale completion or cancellation.
    pub fn reconcile(&mut self, bridge: &mut Bridge, id: &str) -> Result<AdapterState> {
        if matches!(
            self.status.status(id)?,
            AdapterState::Cancelled | AdapterState::StaleContext
        ) {
            return self.status.status(id);
        }
        let Some(input) = self.inputs.get(id) else {
            return self.status.status(id);
        };
        let record = bridge.store.require(id)?;
        if record.capture != input.capture {
            return Err(AdapterError::InvalidState);
        }
        if record.rejected {
            self.scheduler.cancel(id)?;
            return Ok(self.settle(id, AdapterState::Rejected));
        }
        if record.applied.is_some() {
            return Ok(self.settle(id, AdapterState::Applied));
        }
        if record.prepared.is_some() {
            return Ok(self.settle(id, AdapterState::Prepared));
        }
        let current = match bridge.context(&input.capture, input.context.supported_features.clone())
        {
            Ok(context) => context,
            Err(_) => {
                self.scheduler.cancel(id)?;
                return Ok(self.settle(id, AdapterState::StaleContext));
            }
        };
        self.scheduler.update_context(id, fingerprint(&current)?)?;
        if current != input.context {
            self.scheduler.cancel(id)?;
            return Ok(self.settle(id, AdapterState::StaleContext));
        }
        if let State::Completed(proposal) = self.scheduler.state(id)? {
            let mut record = record;
            // Another authorized document transaction may already have journaled
            // this proposal; preserve it and never overwrite a conflicting result.
            if record
                .proposal
                .as_ref()
                .is_some_and(|p| p != proposal.as_ref())
            {
                return Err(AdapterError::InvalidState);
            }
            record.context = Some(current);
            record.proposal = Some((*proposal).clone());
            bridge.store.save(&record)?;
            return Ok(self.settle(id, AdapterState::Proposal(proposal)));
        }
        self.status.status(id)
    }
    pub fn cancel(&self, id: &str) -> Result<()> {
        if matches!(
            self.status.status(id)?,
            AdapterState::Queued | AdapterState::Running | AdapterState::AwaitingJournal
        ) {
            self.scheduler.cancel(id)?;
            self.settle(id, AdapterState::Cancelled);
        }
        Ok(())
    }
    /// Retire only after no converter is physically running. Durable capture and
    /// intent records remain, so restarting this identity cannot invoke it again.
    pub fn retire(&mut self, id: &str) -> Result<()> {
        if self.inputs.contains_key(id) {
            self.scheduler.forget(id)?;
            self.inputs.remove(id);
        }
        self.status.settled.lock().unwrap().remove(id);
        Ok(())
    }
    pub fn usage(&self) -> events::UsageEvidence {
        self.scheduler.usage()
    }
    fn settle(&self, id: &str, state: AdapterState) -> AdapterState {
        self.status
            .settled
            .lock()
            .unwrap()
            .insert(id.into(), state.clone());
        state
    }
}
fn fingerprint(context: &Context) -> Result<ContextFingerprint> {
    if context.dependencies.is_empty() {
        return Err(AdapterError::InvalidState);
    }
    Ok(ContextFingerprint {
        revision: context.revision,
        sha256: format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(context).map_err(|_| AdapterError::InvalidState)?)
        ),
    })
}
