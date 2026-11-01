//! Bounded background adapter. Admission/polling never perform filesystem or
//! pipe I/O. Native consumers must discard events from superseded session IDs.
use crate::{
    checkpoint::archive::RotationPolicy,
    checkpoint::{Checkpoint, ExportAuthorization, ImportAuthorization, ImportPlan, StoreIdentity},
    history::{GroupedEdit, HistoryMove, HistoryRetentionPolicy},
    recovery::RecoveryImport,
    retention::RetentionPolicy,
    AppliedReceipt, Document, Error, PreparedEdit, Result, Store,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_FRAME_BYTES: usize = 12 * 1024 * 1024;
#[derive(Debug, Clone)]
pub struct ServiceOptions {
    /// Includes unconsumed replies, not merely queued requests.
    pub capacity: usize,
    pub max_request_bytes: usize,
    pub max_reply_bytes: usize,
}
impl Default for ServiceOptions {
    fn default() -> Self {
        Self {
            capacity: 4,
            max_request_bytes: MAX_FRAME_BYTES,
            max_reply_bytes: 16 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ServiceReply {
    pub session_id: String,
    pub sequence: u64,
    pub id: Option<String>,
    pub document_revision: Option<u64>,
    pub document_sha256: Option<String>,
    /// True even if successful output exceeds the reply limit. Never infer
    /// rollback from an omitted oversized payload; inspect durable state.
    pub command_succeeded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}
#[derive(Deserialize)]
struct Request {
    id: String,
    #[serde(flatten)]
    operation: Operation,
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum Operation {
    Initialize {
        document: Document,
    },
    Apply {
        edit: PreparedEdit,
    },
    ReplaceDocument {
        expected_revision: u64,
        expected_sha256: String,
        text: String,
    },
    Confirm {
        receipt: AppliedReceipt,
    },
    Status,
    RecoveryExport,
    RecoveryImport {
        recovery: RecoveryImport,
    },
    Compact {
        policy: RetentionPolicy,
    },
    ApplyGroup {
        group: GroupedEdit,
    },
    Undo {
        command: HistoryMove,
    },
    Redo {
        command: HistoryMove,
    },
    RetainHistory {
        policy: HistoryRetentionPolicy,
    },
    HistoryStatus,
    CheckpointStatus,
    CheckpointRotate {
        authorization: ExportAuthorization,
        policy: RotationPolicy,
    },
    CheckpointRead {
        generation: u64,
    },
    CheckpointExport {
        authorization: ExportAuthorization,
    },
    CheckpointPlan {
        checkpoint: Box<Checkpoint>,
        expected_identity: StoreIdentity,
    },
    CheckpointImport {
        checkpoint: Box<Checkpoint>,
        plan: Box<ImportPlan>,
        authorization: ImportAuthorization,
    },
}
fn execute(store: &mut Store, operation: Operation) -> Result<Value> {
    match operation {
        Operation::Initialize { document } => {
            store.initialize(document)?;
            Ok(json!({"document": store.document()?}))
        }
        Operation::Apply { edit } => {
            let receipt = store.apply(edit)?;
            Ok(json!({"receipt": receipt, "document": store.document()?}))
        }
        Operation::ReplaceDocument {
            expected_revision,
            expected_sha256,
            text,
        } => Ok(
            json!({"document": store.replace_document(expected_revision, &expected_sha256, text)?}),
        ),
        Operation::Confirm { receipt } => {
            store.confirm(&receipt)?;
            Ok(json!({"confirmed": receipt}))
        }
        Operation::Status => {
            Ok(json!({"document": store.document()?, "pending_receipts": store.recovery()?}))
        }
        Operation::RecoveryExport => Ok(json!(store.export_recovery()?)),
        Operation::RecoveryImport { recovery } => Ok(json!(store.import_recovery(recovery)?)),
        Operation::Compact { policy } => Ok(json!(store.compact(policy)?)),
        Operation::ApplyGroup { group } => Ok(json!(store.apply_group(group)?)),
        Operation::Undo { command } => Ok(json!(store.undo(command)?)),
        Operation::Redo { command } => Ok(json!(store.redo(command)?)),
        Operation::RetainHistory { policy } => Ok(json!(store.retain_history(policy)?)),
        Operation::HistoryStatus => Ok(json!(store.history_status()?)),
        Operation::CheckpointStatus => Ok(json!(store.checkpoint_status()?)),
        Operation::CheckpointRotate {
            authorization,
            policy,
        } => Ok(json!(store.rotate_checkpoint(authorization, policy)?)),
        Operation::CheckpointRead { generation } => Ok(json!(store.read_checkpoint(generation)?)),
        Operation::CheckpointExport { authorization } => {
            Ok(json!(store.export_checkpoint(authorization)?))
        }
        Operation::CheckpointPlan {
            checkpoint,
            expected_identity,
        } => Ok(json!(
            store.plan_checkpoint_import(&checkpoint, expected_identity)?
        )),
        Operation::CheckpointImport {
            checkpoint,
            plan,
            authorization,
        } => {
            Ok(json!({"document":store.apply_checkpoint_import(&checkpoint,&plan,authorization)?}))
        }
    }
}
struct Work {
    frame: Vec<u8>,
    reply: SyncSender<ServiceReply>,
}
struct Permit(Arc<AtomicUsize>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
pub struct PendingReply {
    receiver: Receiver<ServiceReply>,
    permit: Option<Permit>,
}
impl PendingReply {
    /// Nonblocking. None means still pending; poll from a native scheduler.
    pub fn try_recv(&mut self) -> Result<Option<ServiceReply>> {
        match self.receiver.try_recv() {
            Ok(reply) => {
                self.permit.take();
                Ok(Some(reply))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.permit.take();
                Err(Error::new(
                    "service_disconnected",
                    "worker stopped or reply already consumed",
                ))
            }
        }
    }
    /// Blocking convenience for CLI/tests only, never a native UI thread.
    pub fn wait(mut self) -> Result<ServiceReply> {
        let result = self
            .receiver
            .recv()
            .map_err(|_| Error::new("service_disconnected", "worker stopped"));
        self.permit.take();
        result
    }
}
#[derive(Clone)]
pub struct BackgroundService {
    session_id: String,
    sender: SyncSender<Work>,
    in_flight: Arc<AtomicUsize>,
    options: ServiceOptions,
    stopped: Arc<AtomicBool>,
}
pub struct ShutdownWatch(Arc<AtomicBool>);
impl ShutdownWatch {
    /// Nonblocking proof that worker cleanup and store unlock finished.
    pub fn is_stopped(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
struct Completion(Arc<AtomicBool>);
impl Drop for Completion {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
impl BackgroundService {
    /// Opens the store on its worker. Startup errors become request error
    /// events. Dropping an accepted reply does not cancel its transaction.
    pub fn start(root: PathBuf, options: ServiceOptions) -> Result<Self> {
        if !(1..=16).contains(&options.capacity)
            || !(1..=MAX_FRAME_BYTES).contains(&options.max_request_bytes)
            || !(4096..=128 * 1024 * 1024).contains(&options.max_reply_bytes)
        {
            return Err(Error::new(
                "invalid_service_options",
                "capacity/frame/reply bounds exceeded",
            ));
        }
        static NEXT_SESSION: AtomicUsize = AtomicUsize::new(1);
        let session_id = format!(
            "{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT_SESSION.fetch_add(1, Ordering::Relaxed)
        );
        let (sender, receiver) = mpsc::sync_channel::<Work>(options.capacity);
        let worker_session = session_id.clone();
        let max_reply = options.max_reply_bytes;
        let stopped = Arc::new(AtomicBool::new(false));
        let finished = stopped.clone();
        std::thread::Builder::new().name("flashtex-edit-ledger".into()).spawn(move || {
            // Declared before Store, so its completion flag becomes true only
            // after Store drops and explicitly releases the writer lock.
            let _completion = Completion(finished);
            let mut store = Store::open(root);
            for (index, work) in receiver.into_iter().enumerate() {
                let mut reply = process_frame(&mut store, &worker_session, index as u64 + 1, &work.frame);
                if serde_json::to_vec(&reply).map_or(true, |bytes| bytes.len() + 1 > max_reply) {
                    reply.payload = None;
                    reply.error = Some(Error::new("reply_too_large", "result exceeds reply limit; command_succeeded and durable revision describe outcome; recover before retry"));
                }
                // One response fits each dedicated slot. Slow/dropped readers
                // cannot block processing the other admitted requests.
                let _ = work.reply.try_send(reply);
            }
        })?;
        Ok(Self {
            session_id,
            sender,
            in_flight: Arc::new(AtomicUsize::new(0)),
            options,
            stopped,
        })
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    /// Close this admission handle. All clones must close before the worker
    /// drains and stops. Poll the returned watch before reopening the same store.
    pub fn shutdown(self) -> ShutdownWatch {
        ShutdownWatch(self.stopped.clone())
    }
    /// Nonblocking admission: busy means nothing was accepted/executed.
    pub fn try_submit(&self, frame: Vec<u8>) -> Result<PendingReply> {
        if frame.is_empty()
            || frame.len() > self.options.max_request_bytes
            || frame.last() != Some(&b'\n')
            || frame[..frame.len() - 1].contains(&b'\n')
        {
            return Err(Error::new(
                "invalid_frame",
                "expected exactly one bounded newline-terminated request",
            ));
        }
        self.in_flight
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.options.capacity).then_some(n + 1)
            })
            .map_err(|_| Error::new("busy", "bounded in-flight request/reply capacity reached"))?;
        let permit = Permit(self.in_flight.clone());
        let (reply, receiver) = mpsc::sync_channel(1);
        match self.sender.try_send(Work { frame, reply }) {
            Ok(()) => Ok(PendingReply {
                receiver,
                permit: Some(permit),
            }),
            Err(TrySendError::Full(_)) => Err(Error::new(
                "busy",
                "command queue is full; nothing accepted",
            )),
            Err(TrySendError::Disconnected(_)) => {
                Err(Error::new("service_disconnected", "worker stopped"))
            }
        }
    }
}
fn process_frame(
    store: &mut Result<Store>,
    session_id: &str,
    sequence: u64,
    frame: &[u8],
) -> ServiceReply {
    let mut id = None;
    let result = match serde_json::from_slice::<Request>(frame) {
        Ok(request) if !request.id.is_empty() && request.id.len() <= 128 => {
            id = Some(request.id);
            match store {
                Ok(store) => execute(store, request.operation),
                Err(error) => Err(error.clone()),
            }
        }
        Ok(_) => Err(Error::new("invalid_id", "request ID must be 1–128 bytes")),
        Err(error) => Err(Error::new("invalid_request", error.to_string())),
    };
    let (document_revision, document_sha256) = match store
        .as_ref()
        .ok()
        .and_then(|s| s.document().ok().flatten())
    {
        Some(document) => (
            Some(document.revision),
            Some(document.source_sha256.clone()),
        ),
        None => (None, None),
    };
    let command_succeeded = result.is_ok();
    let (payload, error) = match result {
        Ok(payload) => (Some(payload), None),
        Err(error) => (None, Some(error)),
    };
    ServiceReply {
        session_id: session_id.into(),
        sequence,
        id,
        document_revision,
        document_sha256,
        command_succeeded,
        payload,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(value: Value) -> Vec<u8> {
        format!("{value}\n").into_bytes()
    }
    fn service(dir: &tempfile::TempDir, capacity: usize) -> BackgroundService {
        BackgroundService::start(
            dir.path().to_owned(),
            ServiceOptions {
                capacity,
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn serial_execution_returns_revision_tagged_events() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(&dir, 2);
        let first = service
            .try_submit(frame(
                json!({"id":"init","operation":"initialize","document":crate::tests::doc("aé😀z")}),
            ))
            .unwrap();
        let second = service
            .try_submit(frame(json!({"id":"status","operation":"status"})))
            .unwrap();
        let a = first.wait().unwrap();
        let b = second.wait().unwrap();
        assert!(a.command_succeeded && b.command_succeeded);
        assert_eq!((a.sequence, b.sequence), (1, 2));
        assert_eq!(b.document_revision, Some(1));
        assert_eq!(b.session_id, service.session_id());
        assert_eq!(b.document_sha256, Some(crate::digest("aé😀z")));
    }
    #[test]
    fn capacity_counts_unconsumed_replies_and_returns_immediate_busy() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(&dir, 1);
        let pending = service
            .try_submit(frame(json!({"id":"one","operation":"status"})))
            .unwrap();
        assert_eq!(
            service
                .try_submit(frame(json!({"id":"two","operation":"status"})))
                .err()
                .unwrap()
                .code,
            "busy"
        );
        pending.wait().unwrap();
        assert!(
            service
                .try_submit(frame(json!({"id":"three","operation":"status"})))
                .unwrap()
                .wait()
                .unwrap()
                .command_succeeded
        );
    }
    #[test]
    fn oversize_or_multiple_frames_are_never_admitted() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(&dir, 1);
        for bytes in [
            vec![b'x'; MAX_FRAME_BYTES + 1],
            b"{}\n{}\n".to_vec(),
            b"{}".to_vec(),
        ] {
            assert_eq!(
                service.try_submit(bytes).err().unwrap().code,
                "invalid_frame"
            );
        }
        assert_eq!(
            service
                .try_submit(frame(json!({"id":"ok","operation":"status"})))
                .unwrap()
                .wait()
                .unwrap()
                .sequence,
            1
        );
    }
    #[test]
    fn startup_errors_are_asynchronous_events() {
        let dir = tempfile::tempdir().unwrap();
        let _held = Store::open(dir.path()).unwrap();
        let service = service(&dir, 1);
        let reply = service
            .try_submit(frame(json!({"id":"status","operation":"status"})))
            .unwrap()
            .wait()
            .unwrap();
        assert_eq!(reply.error.unwrap().code, "store_in_use");
        assert!(!reply.command_succeeded);
    }
    #[test]
    fn reply_limit_reports_committed_state_without_claiming_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let service = BackgroundService::start(
            dir.path().into(),
            ServiceOptions {
                max_reply_bytes: 4096,
                ..Default::default()
            },
        )
        .unwrap();
        let document = crate::tests::doc(&"x".repeat(5000));
        let reply = service
            .try_submit(frame(
                json!({"id":"init","operation":"initialize","document":document}),
            ))
            .unwrap()
            .wait()
            .unwrap();
        assert!(reply.command_succeeded);
        assert_eq!(reply.document_revision, Some(1));
        assert_eq!(reply.error.as_ref().unwrap().code, "reply_too_large");
        assert!(serde_json::to_vec(&reply).unwrap().len() < 4096);
    }
    #[test]
    fn sessions_are_distinct_for_stale_callback_guard() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        assert_ne!(service(&a, 1).session_id(), service(&b, 1).session_id());
    }

    #[test]
    fn competing_edits_are_serialized_and_stale_reply_reports_current_revision() {
        let dir = tempfile::tempdir().unwrap();
        let service = service(&dir, 2);
        let document = crate::tests::doc("old");
        service
            .try_submit(frame(
                json!({"id":"init","operation":"initialize","document":document}),
            ))
            .unwrap()
            .wait()
            .unwrap();
        let one = service.try_submit(frame(json!({"id":"one","operation":"replace_document","expected_revision":1,"expected_sha256":document.source_sha256,"text":"first"}))).unwrap();
        let two = service.try_submit(frame(json!({"id":"two","operation":"replace_document","expected_revision":1,"expected_sha256":document.source_sha256,"text":"second"}))).unwrap();
        let accepted = one.wait().unwrap();
        let refused = two.wait().unwrap();
        assert!(accepted.command_succeeded);
        assert!(!refused.command_succeeded);
        assert_eq!(refused.error.unwrap().code, "document_conflict");
        assert_eq!(refused.document_revision, Some(2));
        assert_eq!(refused.document_sha256, Some(crate::digest("first")));
        assert_eq!(refused.sequence, accepted.sequence + 1);
    }
}
