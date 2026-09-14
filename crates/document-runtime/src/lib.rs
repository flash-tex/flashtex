//! Persistent original-compiler transport. Poll from an application worker, not
//! the UI thread. Results are never substituted across revisions.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, SyncSender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

mod decode_lane;
mod display_candidate;
mod raw_display;
use decode_lane::{Decoder, Input, RawInput};
pub use display_candidate::{SourceBinding, UntrustedDisplayCandidate};
pub use raw_display::UntrustedRawDisplayCandidate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub path: String,
    pub text: String,
}
#[derive(Clone, Debug)]
pub struct Request {
    pub id: String,
    pub project_id: String,
    pub revision: u64,
    pub entry_path: String,
    pub documents: Vec<Document>,
}
#[derive(Clone, Debug)]
pub struct Limits {
    pub max_frame: usize,
    pub max_projects: usize,
    pub max_pending_events: usize,
    pub timeout: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_frame: 8 * 1024 * 1024,
            max_projects: 32,
            max_pending_events: 256,
            timeout: Duration::from_secs(5),
        }
    }
}
#[derive(Debug)]
pub enum Event {
    Cancelled {
        id: String,
    },
    Superseded {
        id: String,
        by_id: String,
    },
    Preview {
        id: String,
        project_id: String,
        revision: u64,
        result: Value,
        queue_ms: f64,
        compiler_ms: f64,
        total_ms: f64,
    },
    Stale {
        id: String,
        revision: u64,
    },
    Failed {
        id: String,
        reason: String,
    },
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct ResponseProfile {
    pub request_id: String,
    pub response_bytes: usize,
    pub encode_ms: f64,
    /// Waiting after decode completion for the serialized owner to poll.
    pub reader_delivery_wait_ms: f64,
    /// Raw-frame wait for the single decoding/validation permit.
    pub decode_queue_wait_ms: f64,
    pub dispatch_to_first_byte_ms: f64,
    pub frame_read_ms: f64,
    pub parse_ms: f64,
    pub validation_ms: f64,
}
/// Scalar transport timings for the last current source-bound candidate only.
/// No native decoding, font validation, rendering, paint or source text is included.
#[derive(Clone, Debug, serde::Serialize)]
pub struct DisplayResponseProfile {
    pub request_id: String,
    pub project_id: String,
    pub revision: u64,
    pub display_epoch: u64,
    pub response_bytes: usize,
    pub parse_ms: f64,
    pub decode_queue_wait_ms: f64,
    pub reader_delivery_wait_ms: f64,
    pub source_binding_ms: f64,
}
/// Optional historical display data. This is never a current-preview/source-action grant.
/// `origin` is opaque caller metadata captured with the original submitted request.
#[derive(Debug)]
pub struct CompletedSnapshot {
    pub request_id: String,
    pub project_id: String,
    pub revision: u64,
    pub origin: String,
    pub result: Value,
}
struct Pending {
    awaiting_display: bool,
    display_epoch: u64,
    snapshot_origin: Option<(u64, String)>,
    encode_ms: f64,
    capabilities: Vec<String>,
    cancelled: bool,
    request: Request,
    bytes: Vec<u8>,
    queued: Instant,
    sent: Option<Instant>,
}
struct Process {
    #[cfg(all(test, target_os = "linux"))]
    writer_finished: Option<mpsc::Receiver<()>>,
    #[cfg(all(test, target_os = "linux"))]
    io_finished: Option<[mpsc::Receiver<()>; 2]>,
    child: Child,
    writer: SyncSender<Vec<u8>>,
    reader: Decoder,
}
impl Process {
    fn spawn(mut command: Command, limit: usize, raw_display: bool) -> Result<Self, String> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("compiler launch: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("compiler stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("compiler stdout unavailable")?;
        let mut stderr = child.stderr.take().ok_or("compiler stderr unavailable")?;
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(1);
        let (out_tx, out_rx) = mpsc::sync_channel(4);
        let failures = out_tx.clone();
        #[cfg(all(test, target_os = "linux"))]
        let (writer_done, writer_finished) = mpsc::channel();
        #[cfg(all(test, target_os = "linux"))]
        let (stdout_done, stdout_finished) = mpsc::channel();
        #[cfg(all(test, target_os = "linux"))]
        let (stderr_done, stderr_finished) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(bytes) = rx.recv() {
                if stdin.write_all(&bytes).and_then(|_| stdin.flush()).is_err() {
                    let _ = failures.send(RawInput::Failure("compiler input closed".into()));
                    break;
                }
            }
            #[cfg(all(test, target_os = "linux"))]
            let _ = writer_done.send(());
        });
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut frame = Vec::new();
                if reader.fill_buf().is_err() {
                    let _ = out_tx.send(RawInput::Failure("compiler output read failed".into()));
                    break;
                }
                let first_byte = Instant::now();
                let result = reader
                    .by_ref()
                    .take(limit as u64 + 1)
                    .read_until(b'\n', &mut frame);
                match result {
                    Ok(0) => {
                        let _ = out_tx.send(RawInput::Failure("compiler output closed".into()));
                        break;
                    }
                    Ok(_) if frame.len() <= limit && frame.last() == Some(&b'\n') => {
                        if out_tx
                            .send(RawInput::Frame(frame, first_byte, Instant::now()))
                            .is_err()
                        {
                            break;
                        }
                    }
                    _ => {
                        let _ = out_tx.send(RawInput::Failure(
                            "compiler output malformed, truncated or oversized".into(),
                        ));
                        break;
                    }
                }
            }
            #[cfg(all(test, target_os = "linux"))]
            let _ = stdout_done.send(());
        });
        // Drain continuously in a fixed buffer. Logs are not retained or exposed to UI.
        thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut buffer) {
                if n == 0 {
                    break;
                }
            }
            #[cfg(all(test, target_os = "linux"))]
            let _ = stderr_done.send(());
        });
        Ok(Self {
            #[cfg(all(test, target_os = "linux"))]
            io_finished: Some([stdout_finished, stderr_finished]),
            #[cfg(all(test, target_os = "linux"))]
            writer_finished: Some(writer_finished),
            child,
            writer: tx,
            reader: Decoder::spawn_mode(out_rx, raw_display),
        })
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct Session {
    raw_candidate: Option<UntrustedRawDisplayCandidate>,
    display_enabled: bool,
    display_epoch: u64,
    display_candidate: Option<UntrustedDisplayCandidate>,
    last_display_profile: Option<DisplayResponseProfile>,
    process: Option<Process>,
    limits: Limits,
    active: Option<Pending>,
    queue: VecDeque<Pending>,
    latest: BTreeMap<String, (u64, String)>,
    events: VecDeque<Event>,
    last_profile: Option<ResponseProfile>,
    snapshot_epoch: u64,
    completed_snapshots_enabled: bool,
    completed_snapshot: Option<CompletedSnapshot>,
    /// PROPOSAL (display-list-v2-images §2): absolute directory sent as
    /// `payload.project_root` on every compile request. `None` keeps the
    /// frozen runtime-v1 request bytes unchanged.
    project_root: Option<String>,
}
impl Session {
    pub fn spawn(executable: impl AsRef<Path>, limits: Limits) -> Result<Self, String> {
        Self::spawn_command(Command::new(executable.as_ref()), limits)
    }
    /// Allows explicit original compiler flags/environment without shell parsing.
    pub fn spawn_command(command: Command, limits: Limits) -> Result<Self, String> {
        Self::spawn_mode(command, limits, false)
    }
    /// Experimental fixed-session raw decoding strategy. Candidate delivery stays disabled
    /// until explicitly enabled; downstream rendering validation remains mandatory.
    pub fn spawn_command_raw_display_prototype(
        command: Command,
        limits: Limits,
    ) -> Result<Self, String> {
        Self::spawn_mode(command, limits, true)
    }
    fn spawn_mode(command: Command, limits: Limits, raw_display: bool) -> Result<Self, String> {
        if limits.max_frame < 128
            || limits.max_frame > 64 * 1024 * 1024
            || limits.max_projects == 0
            || limits.max_pending_events == 0
            || limits.timeout.is_zero()
        {
            return Err("invalid runtime limits".into());
        }
        let process = Process::spawn(command, limits.max_frame, raw_display)?;
        Ok(Self {
            raw_candidate: None,
            display_enabled: false,
            display_epoch: 0,
            display_candidate: None,
            last_display_profile: None,
            process: Some(process),
            limits,
            active: None,
            queue: VecDeque::new(),
            latest: BTreeMap::new(),
            events: VecDeque::new(),
            last_profile: None,
            snapshot_epoch: 0,
            completed_snapshots_enabled: false,
            completed_snapshot: None,
            project_root: None,
        })
    }
    /// Directory the producer reads `\includegraphics` files from, forwarded
    /// per request as `payload.project_root` (display-list-v2-images §2).
    /// Only the textual shape is checked here; callers must pass a canonical
    /// existing directory. Applies to requests submitted after this call.
    pub fn set_project_root(&mut self, root: Option<String>) -> Result<(), String> {
        if let Some(root) = root.as_deref() {
            validate_project_root(root)?;
        }
        self.project_root = root;
        Ok(())
    }
    pub fn project_root(&self) -> Option<&str> {
        self.project_root.as_deref()
    }
    pub fn submit(&mut self, request: Request) -> Result<(), String> {
        self.submit_with_capabilities(request, Vec::new())
    }
    pub fn submit_with_capabilities(
        &mut self,
        request: Request,
        capabilities: Vec<String>,
    ) -> Result<(), String> {
        self.submit_internal(request, capabilities, None)
    }
    /// Enable historical completion retention explicitly. Toggling invalidates old origins.
    /// Disabled by default; at most one completed result is retained across all projects.
    pub fn set_completed_snapshots_enabled(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && self.display_enabled {
            return Err(
                "display candidates and historical snapshots are mutually exclusive".into(),
            );
        }
        if self.completed_snapshots_enabled != enabled {
            self.completed_snapshot = None;
            self.completed_snapshots_enabled = false;
            self.snapshot_epoch = self
                .snapshot_epoch
                .checked_add(1)
                .ok_or("snapshot epoch exhausted")?;
            self.completed_snapshots_enabled = enabled;
        }
        Ok(())
    }
    /// Caller must bind this token to immutable source versions and its session incarnation.
    /// Existing submit APIs never opt a request into historical completion retention.
    pub fn submit_with_snapshot_origin(
        &mut self,
        request: Request,
        capabilities: Vec<String>,
        origin: String,
    ) -> Result<(), String> {
        if !self.completed_snapshots_enabled
            || origin.is_empty()
            || origin.len() > 1024
            || origin.chars().any(char::is_control)
        {
            return Err("historical snapshots disabled or invalid origin token".into());
        }
        self.submit_internal(request, capabilities, Some((self.snapshot_epoch, origin)))
    }
    /// Move the last validated historical completion out. Poll first; no cloning or recompile.
    /// Consumers still must reject obsolete session/project/display generations.
    pub fn take_completed_snapshot(&mut self) -> Option<CompletedSnapshot> {
        self.completed_snapshot.take()
    }
    /// Opt-in source-bound transport candidates, never rendering validation.
    /// Mutually exclusive with the historical optional-output slot.
    pub fn set_display_candidates_enabled(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && self.completed_snapshots_enabled {
            return Err(
                "display candidates and historical snapshots are mutually exclusive".into(),
            );
        }
        self.display_candidate = None;
        self.raw_candidate = None;
        self.last_display_profile = None;
        self.display_epoch = self
            .display_epoch
            .checked_add(1)
            .ok_or("display epoch exhausted")?;
        self.display_enabled = enabled;
        Ok(())
    }
    /// Moves untrusted data; downstream must validate rendering and live source/session epochs.
    pub fn take_current_raw_display_candidate(&mut self) -> Option<UntrustedRawDisplayCandidate> {
        self.raw_candidate.take()
    }
    pub fn take_current_display_candidate(&mut self) -> Option<UntrustedDisplayCandidate> {
        self.display_candidate.take()
    }
    fn submit_internal(
        &mut self,
        request: Request,
        capabilities: Vec<String>,
        snapshot_origin: Option<(u64, String)>,
    ) -> Result<(), String> {
        validate_layout_capabilities(&capabilities)?;
        if capabilities.iter().any(|c| c == "display-list-v2") && !self.display_enabled {
            return Err("display candidates disabled".into());
        }
        if self.events.len() >= self.limits.max_pending_events {
            return Err("poll pending events before submitting more edits".into());
        }
        if self.process.is_none() {
            return Err(
                "compiler session failed; create a new session with complete snapshots".into(),
            );
        }
        let encode_start = Instant::now();
        let bytes = encode_rooted(
            &request,
            self.limits.max_frame,
            &capabilities,
            self.project_root.as_deref(),
        )?;
        let encode_ms = encode_start.elapsed().as_secs_f64() * 1000.0;
        if self.latest.values().any(|(_, id)| id == &request.id)
            || self
                .active
                .as_ref()
                .is_some_and(|p| p.request.id == request.id)
            || self.queue.iter().any(|p| p.request.id == request.id)
        {
            return Err("duplicate live request ID".into());
        }
        if let Some((revision, _)) = self.latest.get(&request.project_id) {
            if request.revision <= *revision {
                return Err("revision must advance monotonically".into());
            }
        } else if self.latest.len() >= self.limits.max_projects {
            return Err("project capacity reached".into());
        }
        // Admission is complete. Do not retain caller reserve capacity or serializer
        // growth slack alongside immutable snapshots for the lifetime of the queue.
        let request = compact_request(request);
        let bytes = bytes.into_boxed_slice().into_vec();
        let capabilities = capabilities
            .into_iter()
            .map(compact_string)
            .collect::<Vec<_>>()
            .into_boxed_slice()
            .into_vec();
        let snapshot_origin =
            snapshot_origin.map(|(epoch, origin)| (epoch, compact_string(origin)));
        if let Some(index) = self
            .queue
            .iter()
            .position(|p| p.request.project_id == request.project_id)
        {
            let old = self.queue.remove(index).unwrap();
            self.events.push_back(Event::Superseded {
                id: old.request.id,
                by_id: request.id.clone(),
            });
        }
        self.latest.insert(
            request.project_id.clone(),
            (request.revision, request.id.clone()),
        );
        self.display_candidate = None;
        self.raw_candidate = None;
        self.last_display_profile = None;
        self.queue.push_back(Pending {
            awaiting_display: false,
            display_epoch: self.display_epoch,
            snapshot_origin,
            encode_ms,
            capabilities,
            cancelled: false,
            request,
            bytes,
            queued: Instant::now(),
            sent: None,
        });
        self.dispatch();
        Ok(())
    }
    /// Release a closed project's slot and explicitly cancel accepted work.
    /// An in-flight wire request is still drained before another is dispatched.
    pub fn close_project(&mut self, project_id: &str) -> Result<(), String> {
        if self.events.len() >= self.limits.max_pending_events {
            return Err("poll pending events before closing projects".into());
        }
        if self
            .completed_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.project_id == project_id)
        {
            self.completed_snapshot = None;
        }
        self.display_candidate = None;
        self.raw_candidate = None;
        self.last_display_profile = None;
        self.latest.remove(project_id);
        if let Some(active) = self.active.as_mut() {
            if active.request.project_id == project_id && !active.cancelled {
                active.cancelled = true;
                self.events.push_back(Event::Cancelled {
                    id: active.request.id.clone(),
                });
            }
        }
        let mut retained = VecDeque::new();
        for pending in self.queue.drain(..) {
            if pending.request.project_id == project_id {
                self.events.push_back(Event::Cancelled {
                    id: pending.request.id,
                });
            } else {
                retained.push_back(pending);
            }
        }
        self.queue = retained;
        Ok(())
    }
    fn dispatch(&mut self) {
        if self.active.is_some() || self.process.is_none() {
            return;
        }
        self.process
            .as_ref()
            .unwrap()
            .reader
            .set_budget(raw_display::MetadataBudget::default());
        if let Some(mut pending) = self.queue.pop_front() {
            self.process
                .as_ref()
                .unwrap()
                .reader
                .set_budget(raw_display::MetadataBudget::from_request(&pending.request));
            pending.sent = Some(Instant::now());
            let bytes = std::mem::take(&mut pending.bytes);
            self.active = Some(pending);
            if self
                .process
                .as_ref()
                .unwrap()
                .writer
                .try_send(bytes)
                .is_err()
            {
                self.fail("compiler writer unavailable");
            }
        }
    }
    fn fail(&mut self, reason: &str) {
        self.display_candidate = None;
        self.raw_candidate = None;
        self.last_display_profile = None;
        self.completed_snapshot = None;
        self.process.take();
        if let Some(p) = self.active.take().filter(|p| !p.cancelled) {
            self.events.push_back(Event::Failed {
                id: p.request.id,
                reason: reason.into(),
            });
        }
        for p in self.queue.drain(..) {
            self.events.push_back(Event::Failed {
                id: p.request.id,
                reason: reason.into(),
            });
        }
    }
    pub fn poll(&mut self) -> Vec<Event> {
        while let Some(process) = self.process.as_ref() {
            let message = process.reader.try_recv();
            match message {
                Ok(Input::Failure(reason)) => {
                    self.fail(&reason);
                    break;
                }
                Ok(Input::Frame(mut frame)) => {
                    let reader_delivery_wait_ms = frame.decoded_at.elapsed().as_secs_f64() * 1000.0;
                    let Some(pending) = self.active.as_ref() else {
                        self.fail("unsolicited compiler reply");
                        break;
                    };
                    let parsed = frame.value.take();
                    if pending.awaiting_display {
                        let binding_start = Instant::now();
                        let candidate = match frame
                            .raw
                            .take()
                            .map(|raw| raw.validate(&pending.request).map(|c| (None, Some(c))))
                            .unwrap_or_else(|| {
                                display_candidate::validate(parsed.unwrap(), &pending.request)
                                    .map(|c| (Some(c), None))
                            }) {
                            Ok(candidate) => candidate,
                            Err(reason) => {
                                self.fail(&reason);
                                break;
                            }
                        };
                        if self.display_enabled
                            && pending.display_epoch == self.display_epoch
                            && !pending.cancelled
                            && self.latest.get(&pending.request.project_id).is_some_and(
                                |(rev, id)| {
                                    *rev == pending.request.revision && id == &pending.request.id
                                },
                            )
                        {
                            self.last_display_profile = Some(DisplayResponseProfile {
                                request_id: pending.request.id.clone(),
                                project_id: pending.request.project_id.clone(),
                                revision: pending.request.revision,
                                display_epoch: pending.display_epoch,
                                response_bytes: frame.response_bytes,
                                parse_ms: frame.parse_ms,
                                decode_queue_wait_ms: frame.decode_queue_wait_ms,
                                reader_delivery_wait_ms,
                                source_binding_ms: binding_start.elapsed().as_secs_f64() * 1000.0,
                            });
                            self.display_candidate = candidate.0;
                            self.raw_candidate = candidate.1;
                        }
                        self.active.take();
                        self.dispatch();
                        continue;
                    }
                    let validate_at = Instant::now();
                    let result = match parsed
                        .ok_or_else(|| "unexpected raw sibling".to_string())
                        .and_then(|parsed| {
                            validate_reply_value(parsed, &pending.request, &pending.capabilities)
                        }) {
                        Ok(value) => value,
                        Err(reason) => {
                            self.fail(&reason);
                            break;
                        }
                    };
                    self.last_profile = Some(ResponseProfile {
                        request_id: pending.request.id.clone(),
                        response_bytes: frame.response_bytes,
                        encode_ms: pending.encode_ms,
                        reader_delivery_wait_ms,
                        decode_queue_wait_ms: frame.decode_queue_wait_ms,
                        dispatch_to_first_byte_ms: frame
                            .first_byte
                            .saturating_duration_since(pending.sent.unwrap())
                            .as_secs_f64()
                            * 1000.0,
                        frame_read_ms: frame
                            .reader_done
                            .saturating_duration_since(frame.first_byte)
                            .as_secs_f64()
                            * 1000.0,
                        parse_ms: frame.parse_ms,
                        validation_ms: validate_at.elapsed().as_secs_f64() * 1000.0,
                    });
                    let mut pending = self.active.take().unwrap();
                    pending.awaiting_display = result["payload"]["status"] != "failed"
                        && result["payload"]["layout_capabilities"]
                            .as_array()
                            .is_some_and(|caps| caps.iter().any(|c| c == "display-list-v2"));
                    if pending.cancelled {
                        if pending.awaiting_display {
                            self.active = Some(pending);
                        }
                        self.dispatch();
                        continue;
                    }
                    let sent = pending.sent.unwrap();
                    let now = Instant::now();
                    if self
                        .latest
                        .get(&pending.request.project_id)
                        .is_some_and(|(rev, id)| {
                            *rev == pending.request.revision && id == &pending.request.id
                        })
                    {
                        if self.completed_snapshot.as_ref().is_some_and(|snapshot| {
                            snapshot.project_id == pending.request.project_id
                        }) {
                            self.completed_snapshot = None;
                        }
                        self.events.push_back(Event::Preview {
                            id: pending.request.id.clone(),
                            project_id: pending.request.project_id.clone(),
                            revision: pending.request.revision,
                            result,
                            queue_ms: sent.duration_since(pending.queued).as_secs_f64() * 1000.0,
                            compiler_ms: now.duration_since(sent).as_secs_f64() * 1000.0,
                            total_ms: now.duration_since(pending.queued).as_secs_f64() * 1000.0,
                        });
                    } else {
                        if let Some((epoch, origin)) = pending.snapshot_origin.clone() {
                            if self.completed_snapshots_enabled && epoch == self.snapshot_epoch {
                                self.completed_snapshot = Some(CompletedSnapshot {
                                    request_id: pending.request.id.clone(),
                                    project_id: pending.request.project_id.clone(),
                                    revision: pending.request.revision,
                                    origin,
                                    result,
                                });
                            }
                        }
                        self.events.push_back(Event::Stale {
                            id: pending.request.id.clone(),
                            revision: pending.request.revision,
                        });
                    }
                    if pending.awaiting_display {
                        self.active = Some(pending);
                    }
                    self.dispatch();
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.fail("compiler readers disconnected");
                    break;
                }
            }
        }
        if self
            .active
            .as_ref()
            .and_then(|p| p.sent)
            .is_some_and(|t| t.elapsed() >= self.limits.timeout)
        {
            self.fail("compiler response timeout");
        }
        self.events.drain(..).collect()
    }
    /// Last current display transport timing; survives candidate take, but clears
    /// on submit, close, policy reset or failure. Epoch is local to this Session.
    pub fn last_display_profile(&self) -> Option<&DisplayResponseProfile> {
        self.last_display_profile.as_ref()
    }
    /// Last fully validated response, including stale/cancelled work. Match its
    /// request ID; these phases do not measure native paint or compiler CPU alone.
    pub fn last_profile(&self) -> Option<&ResponseProfile> {
        self.last_profile.as_ref()
    }
    pub fn is_alive(&self) -> bool {
        self.process.is_some()
    }
}
fn compact_string(value: String) -> String {
    value.into_boxed_str().into_string()
}
fn compact_request(mut request: Request) -> Request {
    request.id = compact_string(request.id);
    request.project_id = compact_string(request.project_id);
    request.entry_path = compact_string(request.entry_path);
    for document in &mut request.documents {
        document.path = compact_string(std::mem::take(&mut document.path));
        document.text = compact_string(std::mem::take(&mut document.text));
    }
    request.documents = request.documents.into_boxed_slice().into_vec();
    request
}
fn safe_path(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains(['\\', ':', '\0'])
        && !p.split('/').any(|s| s.is_empty() || s == "." || s == "..")
}
/// Textual checks for a forwarded project root: absolute, UTF-8 (by type),
/// bounded, no NUL/control characters and no empty, `.` or `..` components.
/// Existence, directory-ness and symlink canonicalization are the caller's
/// filesystem checks (preview-controller canonicalizes before forwarding).
pub fn validate_project_root(root: &str) -> Result<(), String> {
    let components = root.strip_prefix('/').map(|rest| rest.split('/'));
    let ok = root.len() <= 4096
        && !root.chars().any(char::is_control)
        && components.is_some_and(|mut parts| {
            root == "/" || parts.all(|s| !s.is_empty() && s != "." && s != "..")
        });
    if ok {
        Ok(())
    } else {
        Err("project_root must be a normalized absolute directory path".into())
    }
}
#[cfg(test)]
fn encode(r: &Request, limit: usize, capabilities: &[String]) -> Result<Vec<u8>, String> {
    encode_rooted(r, limit, capabilities, None)
}
fn encode_rooted(
    r: &Request,
    limit: usize,
    capabilities: &[String],
    project_root: Option<&str>,
) -> Result<Vec<u8>, String> {
    if r.id.is_empty()
        || r.id.len() > 128
        || r.project_id.is_empty()
        || r.project_id.len() > 128
        || r.revision > ((1u64 << 53) - 1)
        || !safe_path(&r.entry_path)
    {
        return Err("invalid request identity, revision or entry".into());
    }
    let mut paths = BTreeMap::new();
    let mut size = 0usize;
    for d in &r.documents {
        size = size.saturating_add(d.text.len());
        if size > limit || !safe_path(&d.path) || paths.insert(&d.path, ()).is_some() {
            return Err("invalid/oversized document snapshots".into());
        }
    }
    if !paths.contains_key(&r.entry_path) {
        return Err("entry snapshot missing".into());
    }
    #[derive(Serialize)]
    struct Payload<'a> {
        project_id: &'a str,
        revision: u64,
        entry_path: &'a str,
        documents: &'a [Document],
        #[serde(skip_serializing_if = "<[String]>::is_empty")]
        layout_capabilities: &'a [String],
        #[serde(skip_serializing_if = "Option::is_none")]
        project_root: Option<&'a str>,
    }
    #[derive(Serialize)]
    struct Envelope<'a> {
        protocol_version: u8,
        id: &'a str,
        #[serde(rename = "type")]
        kind: &'static str,
        payload: Payload<'a>,
    }
    let envelope = Envelope {
        protocol_version: 1,
        id: &r.id,
        kind: "compile",
        payload: Payload {
            project_id: &r.project_id,
            revision: r.revision,
            entry_path: &r.entry_path,
            documents: &r.documents,
            layout_capabilities: capabilities,
            project_root,
        },
    };
    let mut bytes = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    if bytes.len() > limit {
        return Err("request frame too large".into());
    }
    Ok(bytes)
}
fn validate_reply(bytes: &[u8], r: &Request, requested: &[String]) -> Result<Value, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|_| "compiler returned malformed JSON")?;
    validate_reply_value(v, r, requested)
}
// Gather common text fields in one traversal; preserve absence separately for source/font.
struct DisplayFields<'a> {
    kind: &'a Value,
    text: &'a Value,
    font_size: &'a Value,
    x: &'a Value,
    baseline: &'a Value,
    source: Option<&'a Value>,
    font: Option<&'a Value>,
}
impl<'a> DisplayFields<'a> {
    fn read(item: &'a Value) -> Self {
        let mut fields = Self {
            kind: &Value::Null,
            text: &Value::Null,
            font_size: &Value::Null,
            x: &Value::Null,
            baseline: &Value::Null,
            source: None,
            font: None,
        };
        if let Some(object) = item.as_object() {
            for (key, value) in object {
                match key.as_str() {
                    "kind" => fields.kind = value,
                    "text" => fields.text = value,
                    "font_size_pt" => fields.font_size = value,
                    "x_pt" => fields.x = value,
                    "baseline_y_pt" => fields.baseline = value,
                    "source" => fields.source = Some(value),
                    "font" => fields.font = Some(value),
                    _ => (),
                }
            }
        }
        fields
    }
}
fn validate_reply_value(v: Value, r: &Request, requested: &[String]) -> Result<Value, String> {
    if v["protocol_version"] != 1
        || v["id"] != r.id
        || v["type"] != "compile_result"
        || v["payload"]["project_id"] != r.project_id
        || v["payload"]["revision"].as_u64() != Some(r.revision)
    {
        return Err("compiler reply correlation mismatch".into());
    }
    let p = &v["payload"];
    let accepted: Vec<String> = match p.get("layout_capabilities") {
        None => Vec::new(),
        Some(value) => {
            serde_json::from_value(value.clone()).map_err(|_| "invalid accepted capabilities")?
        }
    };
    validate_layout_capabilities(&accepted)?;
    if accepted.iter().any(|cap| {
        !requested.contains(cap)
            || !matches!(
                cap.as_str(),
                "rules-v1" | "font-hints-v1" | "display-list-v2" | "display-list-v2-images"
                    | "display-list-v2-diagnostics"
            )
    }) {
        return Err("compiler accepted unknown or unrequested capability".into());
    }
    // PROPOSAL display-list-v2-images §1: honoured only together with
    // `display-list-v2`; the display sibling stays opaque to this runtime.
    if accepted.iter().any(|cap| cap == "display-list-v2-images")
        && !accepted.iter().any(|cap| cap == "display-list-v2")
    {
        return Err("compiler accepted display-list-v2-images without display-list-v2".into());
    }
    // PROPOSAL display-list-v2-diagnostics: same pairing rule as images.
    if accepted.iter().any(|cap| cap == "display-list-v2-diagnostics")
        && !accepted.iter().any(|cap| cap == "display-list-v2")
    {
        return Err("compiler accepted display-list-v2-diagnostics without display-list-v2".into());
    }

    if !matches!(p["status"].as_str(), Some("ok" | "recovered" | "failed"))
        || !p["pages"].is_array()
        || !p["diagnostics"].is_array()
    {
        return Err("invalid compiler result shape".into());
    }
    let documents: BTreeMap<&str, &str> = r
        .documents
        .iter()
        .map(|document| (document.path.as_str(), document.text.as_str()))
        .collect();
    let span = |source: &Value| -> Result<(), String> {
        if source.is_null() {
            return Ok(());
        }
        let path = source["path"].as_str().ok_or("missing source path")?;
        let text = documents.get(path).ok_or("unknown source snapshot")?;
        let start = source["start_byte"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or("invalid source start")?;
        let end = source["end_byte"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or("invalid source end")?;
        if start > end
            || end > text.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
        {
            return Err("invalid UTF-8 source range".into());
        }
        Ok(())
    };
    for diagnostic in p["diagnostics"].as_array().unwrap() {
        if !matches!(diagnostic["severity"].as_str(), Some("error" | "warning"))
            || !diagnostic["message"].is_string()
            || diagnostic.get("source").is_none()
        {
            return Err("invalid diagnostic".into());
        }
        span(&diagnostic["source"])?;
    }
    for (index, page) in p["pages"].as_array().unwrap().iter().enumerate() {
        if page["number"].as_u64() != Some(index as u64 + 1)
            || !["width_pt", "height_pt"].iter().all(|key| {
                page[*key]
                    .as_f64()
                    .is_some_and(|n| n.is_finite() && n > 0.0)
            })
        {
            return Err("invalid page geometry".into());
        }
        for item in page["items"].as_array().ok_or("missing page items")? {
            let fields = DisplayFields::read(item);
            if fields.kind == "rule" {
                if !accepted.iter().any(|cap| cap == "rules-v1")
                    || !["x_pt", "y_pt"].iter().all(|key| {
                        item[*key]
                            .as_f64()
                            .is_some_and(|n| n.is_finite() && n.abs() <= 1_000_000.0)
                    })
                    || !["width_pt", "height_pt"].iter().all(|key| {
                        item[*key]
                            .as_f64()
                            .is_some_and(|n| n.is_finite() && n > 0.0 && n <= 1_000_000.0)
                    })
                    || item["source"].is_null()
                {
                    return Err("unrequested or malformed rule".into());
                }
                span(&item["source"])?;
                continue;
            }
            if let Some(font) = fields.font {
                if !accepted.iter().any(|cap| cap == "font-hints-v1")
                    || !font["family"].as_str().is_some_and(|name| {
                        !name.is_empty() && name.len() <= 128 && !name.chars().any(char::is_control)
                    })
                    || !matches!(font["weight"].as_str(), Some("normal" | "bold"))
                    || !matches!(font["style"].as_str(), Some("normal" | "italic"))
                {
                    return Err("unrequested or malformed font hint".into());
                }
            }
            if fields.kind != "text"
                || !fields.text.is_string()
                || !fields
                    .font_size
                    .as_f64()
                    .is_some_and(|n| n.is_finite() && n > 0.0)
                || ![fields.x, fields.baseline]
                    .iter()
                    .all(|value| value.as_f64().is_some_and(f64::is_finite))
                || fields.source.is_none()
            {
                return Err("unsupported or malformed display item".into());
            }
            span(fields.source.unwrap())?;
        }
    }
    Ok(v)
}

pub fn validate_layout_capabilities(capabilities: &[String]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    if capabilities.len() > 16
        || capabilities
            .iter()
            .any(|cap| cap.is_empty() || cap.len() > 64 || !seen.insert(cap))
    {
        return Err("invalid or duplicate layout capabilities".into());
    }
    Ok(())
}

pub mod experimental_chunks;

#[cfg(test)]
mod project_root_tests {
    use super::*;
    fn request() -> Request {
        Request {
            id: "r1".into(),
            project_id: "p".into(),
            revision: 1,
            entry_path: "main.tex".into(),
            documents: vec![Document {
                path: "main.tex".into(),
                text: "x".into(),
            }],
        }
    }
    #[test]
    fn project_root_textual_validation() {
        for ok in ["/", "/Users/me/paper", "/private/var/folders/a b/π"] {
            assert!(validate_project_root(ok).is_ok(), "{ok}");
        }
        let long = format!("/{}", "a".repeat(4096));
        for bad in [
            "",
            "relative/dir",
            "./x",
            "/a//b",
            "/a/",
            "/a/./b",
            "/a/../b",
            "/a\0b",
            "/a\nb",
            long.as_str(),
        ] {
            assert!(validate_project_root(bad).is_err(), "{bad:?}");
        }
    }
    #[test]
    fn absent_root_keeps_legacy_request_bytes_and_present_root_is_forwarded() {
        let r = request();
        let legacy = encode(&r, 1 << 20, &[]).unwrap();
        assert_eq!(encode_rooted(&r, 1 << 20, &[], None).unwrap(), legacy);
        assert!(!String::from_utf8_lossy(&legacy).contains("project_root"));
        let caps = vec!["display-list-v2-images".to_string()];
        let rooted = encode_rooted(&r, 1 << 20, &caps, Some("/tmp/proj")).unwrap();
        let v: Value = serde_json::from_slice(&rooted).unwrap();
        assert_eq!(v["payload"]["project_root"], "/tmp/proj");
        assert_eq!(
            v["payload"]["layout_capabilities"][0],
            "display-list-v2-images"
        );
        assert_eq!(v["payload"]["entry_path"], "main.tex");
    }
    #[test]
    fn image_capability_is_accepted_only_when_requested_with_display_list() {
        let r = request();
        let reply = |caps: Value| {
            serde_json::json!({"protocol_version":1,"id":"r1","type":"compile_result",
                "payload":{"project_id":"p","revision":1,"status":"ok","pages":[],
                "diagnostics":[],"layout_capabilities":caps}})
        };
        let both: Vec<String> = vec!["display-list-v2".into(), "display-list-v2-images".into()];
        assert!(validate_reply_value(reply(serde_json::json!(both)), &r, &both).is_ok());
        // Requested but only display-list-v2 honoured (old producer): fine.
        assert!(
            validate_reply_value(reply(serde_json::json!(["display-list-v2"])), &r, &both).is_ok()
        );
        // Not requested: refused.
        let plain: Vec<String> = vec!["display-list-v2".into()];
        assert!(validate_reply_value(reply(serde_json::json!(both)), &r, &plain).is_err());
        // Images echoed without display-list-v2: refused.
        assert!(validate_reply_value(
            reply(serde_json::json!(["display-list-v2-images"])),
            &r,
            &both
        )
        .is_err());
    }
    #[test]
    fn diagnostics_capability_is_accepted_only_when_requested_with_display_list() {
        let r = request();
        let reply = |caps: Value| {
            serde_json::json!({"protocol_version":1,"id":"r1","type":"compile_result",
                "payload":{"project_id":"p","revision":1,"status":"ok","pages":[],
                "diagnostics":[],"layout_capabilities":caps}})
        };
        let both: Vec<String> = vec![
            "display-list-v2".into(),
            "display-list-v2-diagnostics".into(),
        ];
        assert!(validate_reply_value(reply(serde_json::json!(both)), &r, &both).is_ok());
        assert!(
            validate_reply_value(reply(serde_json::json!(["display-list-v2"])), &r, &both).is_ok()
        );
        let plain: Vec<String> = vec!["display-list-v2".into()];
        assert!(validate_reply_value(reply(serde_json::json!(both)), &r, &plain).is_err());
        // Alone: pairing error, not the unknown-cap error.
        assert_eq!(
            validate_reply_value(
                reply(serde_json::json!(["display-list-v2-diagnostics"])),
                &r,
                &both
            )
            .unwrap_err(),
            "compiler accepted display-list-v2-diagnostics without display-list-v2"
        );
        // Unknown: still refused even when requested.
        let unknown: Vec<String> = vec!["display-list-v2".into(), "not-a-capability".into()];
        assert_eq!(
            validate_reply_value(
                reply(serde_json::json!(unknown)),
                &r,
                &unknown
            )
            .unwrap_err(),
            "compiler accepted unknown or unrequested capability"
        );
    }
}

#[cfg(all(test, unix))]
mod decode_cancellation_probe {
    use super::*;
    #[test]
    #[ignore = "near-limit cancellation probe; run separately in a quiet window"]
    fn cancellation_during_entered_serde_invalidates_source_ownership() {
        let mut command = std::process::Command::new("/usr/bin/python3");
        command.arg("-c").arg(r#"import sys,json
r=json.loads(sys.stdin.readline());p=r['payload']
v={'opaque':[''],'protocol_version':1,'type':'compile_result','id':r['id'],'payload':{'project_id':p['project_id'],'revision':p['revision'],'status':'ok','pages':[],'diagnostics':[]}}
s=json.dumps(v,separators=(',',':'));v['opaque'][0]='x'*(8388608-1-len(s));print(json.dumps(v,separators=(',',':')),flush=True);sys.stdin.read()
"#);
        let mut s =
            Session::spawn_command_raw_display_prototype(command, Limits::default()).unwrap();
        let (entered_tx, entered) = std::sync::mpsc::sync_channel(1);
        let (resume, resume_rx) = std::sync::mpsc::sync_channel(1);
        s.process
            .as_ref()
            .unwrap()
            .reader
            .install_gate(crate::raw_display::DecodeGate {
                entered: entered_tx,
                resume: resume_rx,
            });
        s.submit(Request {
            id: "r".into(),
            project_id: "p".into(),
            revision: 1,
            entry_path: "main.tex".into(),
            documents: vec![Document {
                path: "main.tex".into(),
                text: "x".into(),
            }],
        })
        .unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        let start = Instant::now();
        s.close_project("p").unwrap();
        let cancel_ms = start.elapsed().as_secs_f64() * 1000.0;
        assert!(s.active.as_ref().unwrap().cancelled);
        assert!(s.last_display_profile().is_none());
        resume.send(()).unwrap();
        let started = Instant::now();
        let mut events = vec![];
        while s.active.is_some() {
            events.extend(s.poll());
            assert!(
                s.is_alive() && started.elapsed() < Duration::from_secs(10),
                "{events:?}"
            );
            std::thread::yield_now();
        }
        assert!(events.iter().any(|e| matches!(e, Event::Cancelled { .. })));
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::Preview { .. } | Event::Failed { .. })));
        assert!(s.take_current_raw_display_candidate().is_none());
        let drained_ms = started.elapsed().as_secs_f64() * 1000.0;
        let start = Instant::now();
        drop(s);
        let drop_ms = start.elapsed().as_secs_f64() * 1000.0;
        let peak = std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VmHWM:"))
                    .map(str::to_owned)
            });
        println!(
            "{}",
            serde_json::json!({"framed_bytes":8388608,"entered_serde_before_cancel":true,"cancel_ms":cancel_ms,"drain_after_release_ms":drained_ms,"drop_after_drain_ms":drop_ms,"cancelled_preview_suppressed":true,"process_peak":peak,"scope":"one test process; owner cancellation does not interrupt serde; no native responsiveness guarantee"})
        );
    }
}

#[cfg(test)]
mod queue_accounting;

#[cfg(all(test, target_os = "linux"))]
mod blocked_writer;
