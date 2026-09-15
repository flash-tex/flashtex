pub mod navigation;
use super::native::ContextIdentity;
use super::*;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io::Read, sync::RwLock};
#[derive(Debug, Clone, Copy)]
pub struct InboxLimits {
    pub entries: usize,
    pub history: usize,
    pub bytes: usize,
}
impl Default for InboxLimits {
    fn default() -> Self {
        Self {
            entries: 64,
            history: 4096,
            bytes: 32 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewIntent {
    AcceptForPreparation,
    RejectCapture,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRequest {
    pub decision_id: String,
    pub capture_id: String,
    pub expected_context: ContextIdentity,
    pub expected_proposal_sha256: String,
    pub intent: ReviewIntent,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewIntentHandoff {
    pub decision: DecisionRequest,
    pub token_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEntry {
    pub capture_id: String,
    pub context: ContextIdentity,
    pub current_context: ContextIdentity,
    pub proposal: Proposal,
    pub proposal_sha256: String,
    pub sequence: u64,
    pub cancelled: bool,
    #[serde(default)]
    pub expires_at: Option<u64>,
    #[serde(default)]
    pub expired: bool,
    #[serde(default)]
    pub context_revoked: bool,
    pub decision_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InboxSnapshot {
    pub schema_version: u8,
    #[serde(default)]
    pub generation: u64,
    pub selected_capture: Option<String>,
    pub next_sequence: u64,
    pub entries: BTreeMap<String, ReviewEntry>,
    pub decisions: BTreeMap<String, DecisionRequest>,
    pub retired: BTreeSet<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewState {
    AwaitingSelectionOrDecision,
    AcceptedForPreparation,
    RejectedIntent,
    Cancelled,
    StaleContext,
    Expired,
}
#[derive(Debug)]
pub enum InboxError {
    Io(std::io::Error),
    Invalid,
    Busy,
    Capacity,
    Missing,
    Conflict,
    SelectionRequired,
    StaleContext,
    Cancelled,
    Retired,
    RecoveryRequired,
    Expired,
}
type InboxResult<T> = std::result::Result<T, InboxError>;
impl From<std::io::Error> for InboxError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
#[derive(Clone)]
pub struct InboxView(Arc<RwLock<Arc<InboxSnapshot>>>, Arc<AtomicBool>);
impl InboxView {
    pub fn snapshot(&self) -> InboxResult<Arc<InboxSnapshot>> {
        if self.1.load(Ordering::Acquire) {
            return Err(InboxError::RecoveryRequired);
        }
        Ok(self.0.read().unwrap().clone())
    }
}
pub struct ReviewInbox {
    root: PathBuf,
    _lock: File,
    limits: InboxLimits,
    state: InboxSnapshot,
    view: InboxView,
    events: navigation::Hub,
}
impl ReviewInbox {
    pub fn open(root: impl AsRef<Path>, limits: InboxLimits) -> InboxResult<Self> {
        if limits.entries == 0
            || limits.entries > 4096
            || limits.history == 0
            || limits.history > 65536
            || limits.bytes == 0
            || limits.bytes > 64 * 1024 * 1024
        {
            return Err(InboxError::Invalid);
        }
        let root = root.as_ref().to_owned();
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
            .open(root.join(".review.lock"))?;
        lock.try_lock_exclusive().map_err(|_| InboxError::Busy)?;
        let mut state: InboxSnapshot = match File::open(root.join("inbox.json")) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(limits.bytes as u64 + 1).read_to_end(&mut bytes)?;
                if bytes.len() > limits.bytes {
                    return Err(InboxError::Capacity);
                }
                serde_json::from_slice(&bytes).map_err(|_| InboxError::Invalid)?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => InboxSnapshot {
                schema_version: 1,
                generation: 0,
                selected_capture: None,
                next_sequence: 0,
                entries: BTreeMap::new(),
                decisions: BTreeMap::new(),
                retired: BTreeSet::new(),
            },
            Err(e) => return Err(e.into()),
        };
        for entry in state.entries.values_mut() {
            entry.context_revoked |= entry.context != entry.current_context;
        }
        validate(&state, limits)?;
        let view = InboxView(
            Arc::new(RwLock::new(Arc::new(state.clone()))),
            Arc::new(AtomicBool::new(false)),
        );
        Ok(Self {
            root,
            _lock: lock,
            limits,
            state,
            view,
            events: navigation::Hub::new(),
        })
    }
    /// Recover using the caller's persisted-clock domain and apply due expiry
    /// before making recovered selection or decisions visible to the caller.
    pub fn open_at(root: impl AsRef<Path>, limits: InboxLimits, now: u64) -> InboxResult<Self> {
        let mut inbox = Self::open(root, limits)?;
        inbox.expire_due(now)?;
        Ok(inbox)
    }
    /// Refresh against actual bridge documents. Missing context revokes the
    /// card durably instead of preserving an old accepted status.
    pub fn refresh_from_bridge(
        &mut self,
        bridge: &Bridge,
        id: &str,
        supported_features: Vec<String>,
    ) -> InboxResult<()> {
        self.ready()?;
        if !self.state.entries.contains_key(id) {
            return Err(InboxError::Missing);
        }
        match super::native::NativeService::context_identity(bridge, id, supported_features) {
            Ok(current)
                if {
                    let entry = &self.state.entries[id];
                    current.project_id == entry.context.project_id
                        && current.path == entry.context.path
                        && current.revision >= entry.current_context.revision
                } =>
            {
                self.update_context(id, current)
            }
            _ => {
                let mut next = self.state.clone();
                next.entries.get_mut(id).unwrap().context_revoked = true;
                self.commit(next)?;
                Err(InboxError::StaleContext)
            }
        }
    }
    pub fn decide_at(
        &mut self,
        request: DecisionRequest,
        now: u64,
    ) -> InboxResult<ReviewIntentHandoff> {
        self.expire_due(now)?;
        self.decide(request)
    }
    pub fn validate_handoff_at(
        &mut self,
        token: &ReviewIntentHandoff,
        current: &ContextIdentity,
        now: u64,
    ) -> InboxResult<()> {
        self.expire_due(now)?;
        self.validate_handoff(token, current)
    }
    pub fn view(&self) -> InboxView {
        self.view.clone()
    }
    /// Consume only a proposal-ready durable snapshot from the native adapter.
    /// This copies review data; it does not select or accept the card.
    pub fn admit_ready(&mut self, status: &super::native::StatusSnapshot) -> InboxResult<()> {
        if !status.capture_durably_received
            || !matches!(status.intent, super::native::IntentState::ProposalJournaled)
        {
            return Err(InboxError::Invalid);
        }
        if status.context != status.current_context {
            return Err(InboxError::StaleContext);
        }
        let super::native::ConversionState::ProposalReady(proposal) = &status.conversion else {
            return Err(InboxError::Invalid);
        };
        self.admit(&status.capture_id, status.context.clone(), proposal.clone())
    }
    pub fn admit(
        &mut self,
        capture_id: &str,
        context: ContextIdentity,
        proposal: Proposal,
    ) -> InboxResult<()> {
        self.ready()?;
        valid_id(capture_id)?;
        valid_context(&context)?;
        proposal.validate().map_err(|_| InboxError::Invalid)?;
        let hash = proposal_hash(&proposal)?;
        if self.state.retired.contains(capture_id) {
            return Err(InboxError::Retired);
        }
        if let Some(old) = self.state.entries.get(capture_id) {
            return if old.context == context && old.proposal_sha256 == hash {
                Ok(())
            } else {
                Err(InboxError::Conflict)
            };
        }
        if self.state.entries.len() >= self.limits.entries {
            return Err(InboxError::Capacity);
        }
        let mut next = self.state.clone();
        let sequence = next.next_sequence;
        next.next_sequence = sequence.checked_add(1).ok_or(InboxError::Capacity)?;
        next.entries.insert(
            capture_id.into(),
            ReviewEntry {
                capture_id: capture_id.into(),
                context: context.clone(),
                current_context: context,
                proposal,
                proposal_sha256: hash,
                sequence,
                cancelled: false,
                expires_at: None,
                expired: false,
                context_revoked: false,
                decision_id: None,
            },
        );
        self.commit(next)
    }
    pub fn select(&mut self, capture_id: Option<&str>) -> InboxResult<()> {
        self.ready()?;
        if capture_id.is_some_and(|id| !self.state.entries.contains_key(id)) {
            return Err(InboxError::Missing);
        }
        let mut next = self.state.clone();
        next.selected_capture = capture_id.map(str::to_owned);
        self.commit(next)
    }
    pub fn update_context(&mut self, id: &str, current: ContextIdentity) -> InboxResult<()> {
        self.ready()?;
        valid_context(&current)?;
        let entry = self.state.entries.get(id).ok_or(InboxError::Missing)?;
        if current.project_id != entry.context.project_id
            || current.path != entry.context.path
            || current.revision < entry.current_context.revision
        {
            return Err(InboxError::Conflict);
        }
        let mut next = self.state.clone();
        let updated = next.entries.get_mut(id).unwrap();
        updated.context_revoked |= updated.context != current;
        updated.current_context = current;
        self.commit(next)
    }
    /// Set a caller-clock deadline before deciding. Existing deadlines may only
    /// become earlier. Call expire_due with the same clock before decisions and
    /// immediately after recovery; clock rollback never revives expired cards.
    pub fn set_expiry(&mut self, id: &str, expires_at: u64) -> InboxResult<()> {
        self.ready()?;
        let entry = self.state.entries.get(id).ok_or(InboxError::Missing)?;
        if entry.expired {
            return Err(InboxError::Expired);
        }
        if entry.decision_id.is_some() || entry.expires_at.is_some_and(|old| expires_at > old) {
            return Err(InboxError::Conflict);
        }
        let mut next = self.state.clone();
        next.entries.get_mut(id).unwrap().expires_at = Some(expires_at);
        self.commit(next)
    }
    /// Atomically expire at most the configured retained-entry bound. Selection
    /// is cleared if its card expires; cards/history are never silently evicted.
    pub fn expire_due(&mut self, now: u64) -> InboxResult<usize> {
        self.ready()?;
        let mut next = self.state.clone();
        let mut count = 0;
        for entry in next.entries.values_mut() {
            if !entry.expired && entry.expires_at.is_some_and(|deadline| now >= deadline) {
                entry.expired = true;
                count += 1;
                if next.selected_capture.as_deref() == Some(&entry.capture_id) {
                    next.selected_capture = None;
                }
            }
        }
        if count > 0 {
            self.commit(next)?;
        }
        Ok(count)
    }
    pub fn cancel(&mut self, id: &str) -> InboxResult<()> {
        self.ready()?;
        if !self.state.entries.contains_key(id) {
            return Err(InboxError::Missing);
        }
        let mut next = self.state.clone();
        next.entries.get_mut(id).unwrap().cancelled = true;
        self.commit(next)
    }
    /// Input is an explicit review intention, not proof of human presence or final
    /// approval of an exact PreparedEdit. The controller must perform that later.
    pub fn decide(&mut self, request: DecisionRequest) -> InboxResult<ReviewIntentHandoff> {
        self.ready()?;
        valid_decision(&request)?;
        if let Some(old) = self.state.decisions.get(&request.decision_id) {
            if old != &request {
                return Err(InboxError::Conflict);
            }
            let token = handoff(request)?;
            self.validate_handoff(&token, &token.decision.expected_context)?;
            return Ok(token);
        }
        if self.state.decisions.len() >= self.limits.history {
            return Err(InboxError::Capacity);
        }
        let entry = self
            .state
            .entries
            .get(&request.capture_id)
            .ok_or(InboxError::Missing)?;
        if self.state.selected_capture.as_deref() != Some(request.capture_id.as_str()) {
            return Err(InboxError::SelectionRequired);
        }
        if entry.expired {
            return Err(InboxError::Expired);
        }
        if entry.cancelled {
            return Err(InboxError::Cancelled);
        }
        if entry.decision_id.is_some() {
            return Err(InboxError::Conflict);
        }
        if request.expected_proposal_sha256 != entry.proposal_sha256 {
            return Err(InboxError::Conflict);
        }
        if request.expected_context != entry.current_context {
            return Err(InboxError::StaleContext);
        }
        if request.intent == ReviewIntent::AcceptForPreparation
            && (entry.context_revoked || entry.context != entry.current_context)
        {
            return Err(InboxError::StaleContext);
        }
        let mut next = self.state.clone();
        next.entries
            .get_mut(&request.capture_id)
            .unwrap()
            .decision_id = Some(request.decision_id.clone());
        next.decisions
            .insert(request.decision_id.clone(), request.clone());
        self.commit(next)?;
        handoff(request)
    }
    pub fn validate_handoff(
        &self,
        token: &ReviewIntentHandoff,
        current: &ContextIdentity,
    ) -> InboxResult<()> {
        self.ready()?;
        if handoff(token.decision.clone())? != *token {
            return Err(InboxError::Conflict);
        }
        let saved = self
            .state
            .decisions
            .get(&token.decision.decision_id)
            .ok_or(InboxError::Missing)?;
        if saved != &token.decision {
            return Err(InboxError::Conflict);
        }
        let entry = self
            .state
            .entries
            .get(&saved.capture_id)
            .ok_or(InboxError::Retired)?;
        if entry.expired {
            return Err(InboxError::Expired);
        }
        if entry.cancelled {
            return Err(InboxError::Cancelled);
        }
        if current != &entry.current_context || current != &saved.expected_context {
            return Err(InboxError::StaleContext);
        }
        if saved.intent == ReviewIntent::AcceptForPreparation
            && (entry.context_revoked || current != &entry.context)
        {
            return Err(InboxError::StaleContext);
        }
        Ok(())
    }
    pub fn state(&self, id: &str) -> InboxResult<ReviewState> {
        self.ready()?;
        let entry = self.state.entries.get(id).ok_or(InboxError::Missing)?;
        if entry.expired {
            return Ok(ReviewState::Expired);
        }
        if entry.cancelled {
            return Ok(ReviewState::Cancelled);
        }
        if entry.context_revoked || entry.context != entry.current_context {
            return Ok(ReviewState::StaleContext);
        }
        Ok(
            match entry
                .decision_id
                .as_ref()
                .and_then(|id| self.state.decisions.get(id))
            {
                None => ReviewState::AwaitingSelectionOrDecision,
                Some(d) if d.intent == ReviewIntent::AcceptForPreparation => {
                    ReviewState::AcceptedForPreparation
                }
                Some(_) => ReviewState::RejectedIntent,
            },
        )
    }
    /// Retire terminal UI records, retaining bounded tombstones and decision history.
    pub fn retire(&mut self, id: &str) -> InboxResult<()> {
        self.ready()?;
        let entry = self.state.entries.get(id).ok_or(InboxError::Missing)?;
        if !entry.cancelled && !entry.expired && entry.decision_id.is_none() {
            return Err(InboxError::Conflict);
        }
        if self.state.retired.len() >= self.limits.history {
            return Err(InboxError::Capacity);
        }
        let mut next = self.state.clone();
        next.entries.remove(id);
        next.retired.insert(id.into());
        if next.selected_capture.as_deref() == Some(id) {
            next.selected_capture = None;
        }
        self.commit(next)
    }
    fn ready(&self) -> InboxResult<()> {
        if self.view.1.load(Ordering::Acquire) {
            Err(InboxError::RecoveryRequired)
        } else {
            Ok(())
        }
    }
    fn commit(&mut self, mut next: InboxSnapshot) -> InboxResult<()> {
        next.generation = self
            .state
            .generation
            .checked_add(1)
            .ok_or(InboxError::Capacity)?;
        validate(&next, self.limits)?;
        let event = navigation::describe(&self.state, &next);
        let mut output = Bounded {
            bytes: Vec::new(),
            limit: self.limits.bytes,
        };
        serde_json::to_writer(&mut output, &next).map_err(|_| InboxError::Capacity)?;
        let persisted = (|| -> InboxResult<()> {
            let mut file = tempfile::NamedTempFile::new_in(&self.root)?;
            file.write_all(&output.bytes)?;
            file.as_file().sync_all()?;
            replace_with_temporary(file, &self.root.join("inbox.json")).map_err(InboxError::Io)?;
            open_dir_for_sync(&self.root)?.sync_all()?;
            Ok(())
        })();
        if let Err(error) = persisted {
            self.view.1.store(true, Ordering::Release);
            self.events.emit(navigation::InboxEvent {
                generation: self.state.generation,
                capture_id: None,
                project_id: None,
                kind: navigation::InboxEventKind::PersistenceUncertain,
            });
            return Err(error);
        }
        self.state = next;
        let published = Arc::new(self.state.clone());
        *self.view.0.write().unwrap() = published;
        self.events.emit(event);
        Ok(())
    }
}
pub fn proposal_hash(proposal: &Proposal) -> InboxResult<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(proposal).map_err(|_| InboxError::Invalid)?)
    ))
}
fn handoff(decision: DecisionRequest) -> InboxResult<ReviewIntentHandoff> {
    let token_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&decision).map_err(|_| InboxError::Invalid)?)
    );
    Ok(ReviewIntentHandoff {
        decision,
        token_sha256,
    })
}
fn valid_id(id: &str) -> InboxResult<()> {
    flashtex_bridge::identifier(id).map_err(|_| InboxError::Invalid)
}
fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_context(context: &ContextIdentity) -> InboxResult<()> {
    valid_id(&context.project_id)?;
    flashtex_bridge::relative_path(&context.path).map_err(|_| InboxError::Invalid)?;
    if !valid_hash(&context.sha256) {
        return Err(InboxError::Invalid);
    }
    Ok(())
}
fn valid_decision(d: &DecisionRequest) -> InboxResult<()> {
    valid_id(&d.capture_id)?;
    valid_id(&d.decision_id)?;
    valid_context(&d.expected_context)?;
    if !valid_hash(&d.expected_proposal_sha256) {
        return Err(InboxError::Invalid);
    }
    Ok(())
}
fn validate(state: &InboxSnapshot, limits: InboxLimits) -> InboxResult<()> {
    if state.schema_version != 1
        || state.entries.len() > limits.entries
        || state.decisions.len() > limits.history
        || state.retired.len() > limits.history
        || state
            .selected_capture
            .as_ref()
            .is_some_and(|id| !state.entries.contains_key(id))
    {
        return Err(InboxError::Invalid);
    }
    let mut sequences = BTreeSet::new();
    for (id, entry) in &state.entries {
        valid_id(id)?;
        valid_context(&entry.context)?;
        valid_context(&entry.current_context)?;
        entry.proposal.validate().map_err(|_| InboxError::Invalid)?;
        if id != &entry.capture_id
            || (entry.expired && entry.expires_at.is_none())
            || state.retired.contains(id)
            || entry.context.project_id != entry.current_context.project_id
            || entry.context.path != entry.current_context.path
            || entry.current_context.revision < entry.context.revision
            || entry.sequence >= state.next_sequence
            || !sequences.insert(entry.sequence)
            || entry.proposal_sha256 != proposal_hash(&entry.proposal)?
        {
            return Err(InboxError::Invalid);
        }
        if let Some(decision) = &entry.decision_id {
            let saved = state.decisions.get(decision).ok_or(InboxError::Invalid)?;
            if saved.capture_id != *id || saved.expected_proposal_sha256 != entry.proposal_sha256 {
                return Err(InboxError::Invalid);
            }
        }
    }
    for (id, decision) in &state.decisions {
        valid_decision(decision)?;
        if id != &decision.decision_id {
            return Err(InboxError::Invalid);
        }
        if let Some(entry) = state.entries.get(&decision.capture_id) {
            if entry.decision_id.as_ref() != Some(id)
                || decision.expected_context.project_id != entry.context.project_id
                || decision.expected_context.path != entry.context.path
                || decision.expected_context.revision < entry.context.revision
                || decision.expected_context.revision > entry.current_context.revision
                || (decision.intent == ReviewIntent::AcceptForPreparation
                    && decision.expected_context != entry.context)
            {
                return Err(InboxError::Invalid);
            }
        } else if !state.retired.contains(&decision.capture_id) {
            return Err(InboxError::Invalid);
        }
    }
    for id in &state.retired {
        valid_id(id)?;
    }
    Ok(())
}
struct Bounded {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("review inbox size limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
