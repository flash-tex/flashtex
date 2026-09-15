//! Worker-thread editor controller. Durable source precedes disposable caches.
pub mod completed_protocol;
mod display;
mod metadata_edit;
pub use display::RawDisplayPayload;
pub use metadata_edit::{
    DocumentMetadata, MetadataEditOutcome, MetadataGroupOutcome, MetadataHistory,
};
pub mod experimental_delivery;
pub mod file_project;
mod historical;
use flashtex_document_runtime::{Document as InputDocument, Event, Limits, Request, Session};
use flashtex_edit_ledger::history::{GroupedEdit, HistoryMove, HistoryResult, HistoryStatus};
use flashtex_edit_ledger::{AppliedReceipt, AppliedTransaction, Document, PreparedEdit, Store};
use flashtex_project_index::{ProjectIndex, VersionSnapshot};
pub use historical::HistoricalPreview;
use serde_json::Value;
use std::{collections::BTreeMap, process::Command, time::Instant};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompileAdmission {
    pub request_id: String,
    pub compile_revision: u64,
}
#[derive(Debug)]
pub struct EditOutcome {
    /// This exact source is durable even when preview submission fails.
    pub document: Document,
    pub compile_admission: Option<CompileAdmission>,
    pub preview_error: Option<String>,
    /// Includes fsync/index/submission; excludes compiler completion and paint.
    pub save_and_submit_ms: f64,
}
/// An explicit application boundary: construct only from the user's approval
/// action after displaying the exact prepared edit. This type is not a verifier
/// of human intent and must never be constructed automatically by conversion.
pub struct ApprovedEdit(PreparedEdit);
impl ApprovedEdit {
    pub fn from_explicit_user_approval(edit: PreparedEdit) -> Self {
        Self(edit)
    }
}
pub enum HistoryAction {
    Group(GroupedEdit),
    Undo(HistoryMove),
    Redo(HistoryMove),
}
#[derive(Debug)]
pub struct HistoryOutcome {
    pub history: HistoryResult,
    pub source: EditOutcome,
}
#[derive(Debug)]
pub struct AppliedOutcome {
    pub receipt: AppliedReceipt,
    pub source: EditOutcome,
}
#[derive(Debug)]
pub struct Preview {
    pub missing_layout_capabilities: Vec<String>,
    pub request_id: String,
    pub compile_revision: u64,
    pub source_versions: VersionSnapshot,
    pub result: Value,
    pub runtime_total_ms: f64,
    /// Controller invocation through observed result, including successful save/index work.
    pub controller_total_ms: f64,
}
#[derive(Debug)]
pub enum Update {
    Preview(Preview),
    Discarded {
        request_id: String,
        compile_revision: u64,
    },
    Runtime(Event),
}

pub struct Controller {
    historical: historical::HistoricalState,
    display_enabled: bool,
    raw_display_prototype: bool,
    project_id: String,
    entry_path: String,
    stores: BTreeMap<String, Store>,
    index: ProjectIndex,
    runtime: Option<Session>,
    generation: u64,
    layout_capabilities: Vec<String>,
    submitted: Option<(String, VersionSnapshot, Instant)>,
    closed: bool,
    /// Canonical project directory forwarded to every compiler session
    /// (`payload.project_root`); `None` for store-backed projects.
    project_root: Option<String>,
}
/// Canonicalizes a project root for forwarding to the producer: it must be an
/// absolute, existing, UTF-8 directory whose path is already canonical, so a
/// symlinked or `..`-containing spelling is refused instead of silently
/// re-pointed (the producer's rooted reads refuse symlinks the same way).
pub fn canonical_project_root(root: &std::path::Path) -> Result<String, String> {
    if !root.is_absolute() {
        return Err("project_root must be absolute".into());
    }
    let canonical = root
        .canonicalize()
        .map_err(|e| format!("project_root cannot be resolved: {e}"))?;
    if canonical != root {
        return Err("project_root must be canonical (no symlink or relative components)".into());
    }
    if !canonical.is_dir() {
        return Err("project_root is not a directory".into());
    }
    let text = canonical
        .to_str()
        .ok_or("project_root must be UTF-8")?
        .to_owned();
    flashtex_document_runtime::validate_project_root(&text)?;
    Ok(text)
}
#[cfg(test)]
mod project_root_tests {
    use super::canonical_project_root;
    #[test]
    fn only_canonical_existing_directories_are_forwarded() {
        let dir = tempfile::tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        assert_eq!(
            canonical_project_root(&canonical).unwrap(),
            canonical.to_str().unwrap()
        );
        assert!(canonical_project_root(std::path::Path::new("relative")).is_err());
        assert!(canonical_project_root(&canonical.join("missing")).is_err());
        std::fs::write(canonical.join("file.tex"), "x").unwrap();
        assert!(canonical_project_root(&canonical.join("file.tex")).is_err());
        std::fs::create_dir(canonical.join("real")).unwrap();
        // Spell the non-canonical path textually rather than through
        // `PathBuf::join`. Measured on Windows: `canonicalize` returns a
        // verbatim (`\\?\`) path, and joining `real/../real` onto a verbatim
        // base yields `…\real` — std normalizes `.`/`..` itself when pushing
        // onto one, because the OS deliberately does not normalize verbatim
        // paths and an unnormalized result would be unopenable. Going through
        // `join` would therefore hand this function an already-clean path and
        // assert nothing. Both platforms still refuse the textual spelling: on
        // POSIX `canonicalize` resolves it to `…/real`, which differs from the
        // argument; on Windows the literal `..` component does not exist under a
        // verbatim root, so `canonicalize` fails outright.
        let sep = std::path::MAIN_SEPARATOR;
        let dotdot =
            std::path::PathBuf::from(format!("{}{sep}real{sep}..{sep}real", canonical.display()));
        assert!(canonical_project_root(&dotdot).is_err());
        #[cfg(windows)]
        {
            // The spelling a Win32 client reaches for first: a plain
            // drive-letter path. It names the same directory, but
            // `canonicalize` reports the verbatim form, so it is not the
            // canonical spelling and is refused. Callers on Windows must
            // forward the `\\?\`-prefixed path.
            let drive = canonical.to_str().unwrap().strip_prefix(r"\\?\").unwrap();
            assert!(canonical_project_root(std::path::Path::new(drive)).is_err());
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(canonical.join("real"), canonical.join("link")).unwrap();
            assert!(canonical_project_root(&canonical.join("link")).is_err());
        }
    }
}
impl Controller {
    /// Forward `root` (validated by [`canonical_project_root`]) as
    /// `payload.project_root` on every later compile request, including
    /// across `restart`. `None` restores the unchanged legacy request.
    pub fn set_project_root(&mut self, root: Option<&std::path::Path>) -> Result<(), String> {
        let root = root.map(canonical_project_root).transpose()?;
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.set_project_root(root.clone())?;
        }
        self.project_root = root;
        Ok(())
    }
    pub fn project_root(&self) -> Option<&str> {
        self.project_root.as_deref()
    }
    /// All stores must already contain initialized durable documents. Ownership of
    /// their exclusive locks transfers here. No source is imported or overwritten.
    pub fn new(
        project_id: String,
        entry_path: String,
        stores: Vec<Store>,
        command: Command,
        limits: Limits,
    ) -> Result<Self, String> {
        let mut controller = Self::open_without_compiler(project_id, entry_path, stores)?;
        controller.runtime = Some(Session::spawn_command(command, limits)?);
        Ok(controller)
    }
    /// Open authoritative source and navigation even when no compiler is available.
    pub fn open_without_compiler(
        project_id: String,
        entry_path: String,
        stores: Vec<Store>,
    ) -> Result<Self, String> {
        Self::open_with_bibliography(project_id, entry_path, stores, &[])
    }
    /// Explicit source kinds at construction; declarations must be supplied again
    /// on reopen. Extensions never infer bibliography semantics.
    pub fn open_with_bibliography(
        project_id: String,
        entry_path: String,
        stores: Vec<Store>,
        bibliography_paths: &[String],
    ) -> Result<Self, String> {
        let kinds: std::collections::BTreeSet<_> = bibliography_paths.iter().collect();
        if kinds.len() != bibliography_paths.len() || kinds.contains(&entry_path) {
            return Err("entry or duplicate bibliography declaration".into());
        }
        let mut by_path = BTreeMap::new();
        let mut index = ProjectIndex::new(&project_id).map_err(|e| e.to_string())?;
        for store in stores {
            let document = store
                .document()
                .map_err(|e| e.to_string())?
                .ok_or("uninitialized document store")?;
            if document.project_id != project_id || by_path.contains_key(&document.path) {
                return Err("wrong project or duplicate document store".into());
            }
            let result = if kinds.contains(&document.path) {
                index.replace_bibliography_document(
                    &document.path,
                    document.revision,
                    &document.text,
                )
            } else {
                index.replace_document(&document.path, document.revision, &document.text)
            };
            result.map_err(|e| e.to_string())?;
            by_path.insert(document.path.clone(), store);
        }
        if kinds.iter().any(|path| !by_path.contains_key(*path)) {
            return Err("unknown bibliography source".into());
        }
        if !by_path.contains_key(&entry_path) {
            return Err("entry store missing".into());
        }
        Ok(Self {
            historical: historical::HistoricalState::default(),
            display_enabled: false,
            raw_display_prototype: false,
            project_id,
            entry_path,
            stores: by_path,
            index,
            runtime: None,
            generation: 0,
            layout_capabilities: Vec::new(),
            submitted: None,
            closed: false,
            project_root: None,
        })
    }
    pub fn index(&self) -> &ProjectIndex {
        &self.index
    }
    pub fn document(&self, path: &str) -> Result<&Document, String> {
        self.stores
            .get(path)
            .ok_or("unknown document")?
            .document()
            .map_err(|e| e.to_string())?
            .ok_or("uninitialized document".into())
    }
    /// Add an initialized durable source to this live session. Membership checks
    /// use the whole prior snapshot so stale UI requests cannot change the project.
    pub fn attach_document(
        &mut self,
        expected: &VersionSnapshot,
        store: Store,
    ) -> Result<EditOutcome, String> {
        self.attach_document_with_kind(expected, store, flashtex_project_index::DocumentKind::Latex)
    }
    /// Attach with an explicitly selected lexical kind. Surviving document kinds
    /// are preserved by the same atomic membership update.
    pub fn attach_document_with_kind(
        &mut self,
        expected: &VersionSnapshot,
        store: Store,
        kind: flashtex_project_index::DocumentKind,
    ) -> Result<EditOutcome, String> {
        if self.closed || expected != &self.index.snapshot() {
            return Err("project closed or membership snapshot is stale".into());
        }
        if self.stores.len() >= 256 {
            return Err("project exceeds 256 source stores".into());
        }
        let document = store
            .document()
            .map_err(|e| e.to_string())?
            .ok_or("uninitialized document")?
            .clone();
        if document.project_id != self.project_id || self.stores.contains_key(&document.path) {
            return Err("wrong project or document already attached".into());
        }
        let started = Instant::now();
        let mut members = self.membership_documents(None)?;
        members.push(document.clone());
        self.replace_membership(expected, &members, Some((&document.path, kind)))?;
        self.submitted = None;
        self.stores.insert(document.path.clone(), store);
        Ok(self.after_save(document, started))
    }

    fn membership_documents(&self, omitted: Option<&str>) -> Result<Vec<Document>, String> {
        self.stores
            .keys()
            .filter(|path| omitted != Some(path.as_str()))
            .map(|path| self.document(path).cloned())
            .collect()
    }
    fn replace_membership(
        &mut self,
        expected: &VersionSnapshot,
        documents: &[Document],
        added_kind: Option<(&str, flashtex_project_index::DocumentKind)>,
    ) -> Result<(), String> {
        let members: Vec<_> = documents
            .iter()
            .map(|doc| {
                (
                    doc.path.as_str(),
                    doc.revision,
                    doc.text.as_str(),
                    added_kind
                        .filter(|(path, _)| *path == doc.path)
                        .map(|(_, kind)| kind)
                        .unwrap_or_else(|| {
                            self.index
                                .document_kind(expected, &doc.path)
                                .unwrap_or(flashtex_project_index::DocumentKind::Latex)
                        }),
                )
            })
            .collect();
        self.index
            .replace_membership(expected, &members)
            .map_err(|e| e.to_string())?;
        self.historical.invalidate();
        Ok(())
    }
    /// Exclude a source from this session, releasing its lock but never deleting
    /// its ledger or disk file. Opening the project again restores retained sources.
    pub fn detach_document(
        &mut self,
        expected: &VersionSnapshot,
        path: &str,
    ) -> Result<Option<String>, String> {
        if self.closed || expected != &self.index.snapshot() {
            return Err("project closed or membership snapshot is stale".into());
        }
        if path == self.entry_path {
            return Err("cannot detach the entry document".into());
        }
        self.document(path)?;
        let members = self.membership_documents(Some(path))?;
        self.replace_membership(expected, &members, None)?;
        self.submitted = None;
        self.stores.remove(path);
        Ok(self.compile_current().err())
    }

    /// An Err means the save did not report success; on storage uncertainty reopen
    /// the authoritative ledger before retry. A compile error is an Ok outcome.
    pub fn replace_document(
        &mut self,
        path: &str,
        expected_revision: u64,
        expected_sha256: &str,
        text: String,
    ) -> Result<EditOutcome, String> {
        if self.closed {
            return Err("project closed".into());
        }
        let started = Instant::now();
        self.submitted = None;
        let document = self
            .stores
            .get_mut(path)
            .ok_or("unknown document")?
            .replace_document(expected_revision, expected_sha256, text)
            .map_err(|e| e.to_string())?;
        Ok(self.after_save(document, started))
    }
    fn after_save(&mut self, document: Document, started: Instant) -> EditOutcome {
        let indexed = Self::index_saved_document(&mut self.index, &document);
        let (preview_error, save_and_submit_ms, compile_admission) =
            self.finish_saved_index_with_admission(indexed, started);
        EditOutcome {
            document,
            compile_admission,
            preview_error,
            save_and_submit_ms,
        }
    }
    fn index_saved_document(index: &mut ProjectIndex, document: &Document) -> Result<(), String> {
        if index.snapshot().documents.get(&document.path) == Some(&document.revision) {
            Ok(())
        } else {
            let kind = index.document_kind(&index.snapshot(), &document.path);
            let result = if kind == Ok(flashtex_project_index::DocumentKind::Bibliography) {
                index.replace_bibliography_document(
                    &document.path,
                    document.revision,
                    &document.text,
                )
            } else {
                index.replace_document(&document.path, document.revision, &document.text)
            };
            result.map(|_| ()).map_err(|e| e.to_string())
        }
    }
    fn finish_saved_index_with_admission(
        &mut self,
        indexed: Result<(), String>,
        started: Instant,
    ) -> (Option<String>, f64, Option<CompileAdmission>) {
        self.submitted = None;
        let result = match indexed {
            Ok(()) => self.compile_current_with_admission(),
            Err(error) => Err(format!("source saved; index recovery required: {error}")),
        };
        let (preview_error, admission) = match result {
            Ok(admission) => {
                if let Some((_, _, submitted_at)) = self.submitted.as_mut() {
                    *submitted_at = started;
                }
                (None, Some(admission))
            }
            Err(error) => (Some(error), None),
        };
        (
            preview_error,
            started.elapsed().as_secs_f64() * 1000.0,
            admission,
        )
    }
    /// The returned receipt is durable before any compile attempt. A matching
    /// retry returns the original receipt and cannot apply the source edit twice.
    pub fn apply_reviewed(&mut self, approved: ApprovedEdit) -> Result<AppliedOutcome, String> {
        if self.closed {
            return Err("project closed".into());
        }
        let started = Instant::now();
        self.submitted = None;
        let path = approved.0.path.clone();
        let store = self.stores.get_mut(&path).ok_or("unknown document")?;
        let receipt = store.apply(approved.0).map_err(|e| e.to_string())?;
        let document = store
            .document()
            .map_err(|e| e.to_string())?
            .ok_or("uninitialized document")?
            .clone();
        Ok(AppliedOutcome {
            receipt,
            source: self.after_save(document, started),
        })
    }
    /// Pending durable receipts for bridge reconciliation, including after reopen.
    pub fn recovery(&self, path: &str) -> Result<Vec<AppliedTransaction>, String> {
        self.stores
            .get(path)
            .ok_or("unknown document")?
            .recovery()
            .map_err(|e| e.to_string())
    }
    /// Invoke only after the bridge acknowledges this exact applied receipt.
    pub fn confirm_receipt(&mut self, path: &str, receipt: &AppliedReceipt) -> Result<(), String> {
        self.stores
            .get_mut(path)
            .ok_or("unknown document")?
            .confirm(receipt)
            .map_err(|e| e.to_string())
    }
    pub fn history_status(&self, path: &str) -> Result<HistoryStatus, String> {
        self.stores
            .get(path)
            .ok_or("unknown document")?
            .history_status()
            .map_err(|e| e.to_string())
    }
    /// Ordinary explicitly requested editor history operations. Permanent command
    /// IDs and source changes are committed by the authoritative ledger together.
    pub fn apply_history(
        &mut self,
        path: &str,
        action: HistoryAction,
    ) -> Result<HistoryOutcome, String> {
        if self.closed {
            return Err("project closed".into());
        }
        let started = Instant::now();
        self.submitted = None;
        let store = self.stores.get_mut(path).ok_or("unknown document")?;
        let history = match action {
            HistoryAction::Group(group) => store.apply_group(group),
            HistoryAction::Undo(command) => store.undo(command),
            HistoryAction::Redo(command) => store.redo(command),
        }
        .map_err(|e| e.to_string())?;
        let source = self.after_save(history.document.clone(), started);
        Ok(HistoryOutcome { history, source })
    }
    /// Explicit native opt-in after implementing the requested draw capabilities.
    /// Empty restores legacy output. Every subsequent compile binds this request.
    pub fn configure_layout(&mut self, capabilities: Vec<String>) -> Result<(), String> {
        if self.closed {
            return Err("project closed".into());
        }
        flashtex_document_runtime::validate_layout_capabilities(&capabilities)?;
        if capabilities.iter().any(|cap| cap == "display-list-v2") && !self.display_enabled {
            return Err("display candidates must be enabled before requesting their layout".into());
        }
        self.historical.invalidate();
        self.layout_capabilities = capabilities;
        self.submitted = None;
        self.compile_current()
    }
    /// Latest successfully admitted compiler generation.
    pub fn compile_revision(&self) -> u64 {
        self.generation
    }
    pub fn compile_current(&mut self) -> Result<(), String> {
        self.compile_current_with_admission().map(|_| ())
    }
    fn compile_current_with_admission(&mut self) -> Result<CompileAdmission, String> {
        let started = Instant::now();
        if self.closed {
            return Err("project closed".into());
        }
        let generation = self
            .generation
            .checked_add(1)
            .filter(|g| *g < (1u64 << 53))
            .ok_or("compile revision exhausted")?;
        let mut documents = Vec::new();
        let indexed = self.index.snapshot();
        for store in self.stores.values() {
            let document = store
                .document()
                .map_err(|e| e.to_string())?
                .ok_or("uninitialized document")?;
            if indexed.documents.get(&document.path) != Some(&document.revision) {
                return Err("index differs from durable source; restart required".into());
            }
            documents.push(InputDocument {
                path: document.path.clone(),
                text: document.text.clone(),
            });
        }
        let id = format!("preview-{generation}");
        let origin = self.historical.origin(generation);
        let request = Request {
            id: id.clone(),
            project_id: self.project_id.clone(),
            revision: generation,
            entry_path: self.entry_path.clone(),
            documents,
        };
        let runtime = self
            .runtime
            .as_mut()
            .ok_or("compiler unavailable; source remains saved")?;
        if let Some(origin) = origin {
            runtime.submit_with_snapshot_origin(
                request,
                self.layout_capabilities.clone(),
                origin.clone(),
            )?;
            self.historical
                .record(generation, origin, id.clone(), indexed);
        } else {
            runtime.submit_with_capabilities(request, self.layout_capabilities.clone())?;
        }
        self.generation = generation;
        self.submitted = Some((id.clone(), self.index.snapshot(), started));
        Ok(CompileAdmission {
            request_id: id,
            compile_revision: generation,
        })
    }
    /// Internal opt-in only; this does not negotiate or activate any native helper messages.
    pub fn configure_completed_snapshots(&mut self, enabled: bool) -> Result<(), String> {
        if self.closed {
            return Err("project closed".into());
        }
        if enabled && self.display_enabled {
            return Err(
                "display candidates and historical snapshots are mutually exclusive".into(),
            );
        }
        self.historical.configure(enabled)?;
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.set_completed_snapshots_enabled(false)?;
            runtime.set_completed_snapshots_enabled(enabled)?;
        }
        Ok(())
    }
    pub fn take_completed_snapshot(&mut self) -> Option<HistoricalPreview> {
        self.historical.take()
    }
    /// Call immediately before historical display on the controller's serialized owner.
    /// A true result grants display only, never current-source actions or export authority.
    pub fn claim_historical_display(&mut self, preview: &HistoricalPreview) -> bool {
        !self.closed
            && self
                .historical
                .claim(preview, &self.project_id, self.generation)
    }
    pub fn historical_binding_count(&self) -> usize {
        self.historical.binding_count()
    }
    /// Recheck immediately before applying a retained result on the UI thread.
    /// Dispatching a preview event is not permission to paint it after a newer edit.
    pub fn is_current_preview(&self, preview: &Preview) -> bool {
        !self.closed
            && preview.compile_revision == self.generation
            && self.submitted.as_ref().is_some_and(|(id, snapshot, _)| {
                id == &preview.request_id
                    && snapshot == &preview.source_versions
                    && snapshot == &self.index.snapshot()
            })
    }
    pub fn poll(&mut self) -> Vec<Update> {
        let Some(runtime) = self.runtime.as_mut() else {
            return Vec::new();
        };
        let events = runtime.poll();
        if let Some(completed) = runtime.take_completed_snapshot() {
            self.historical.consume(completed);
        }
        for event in &events {
            self.historical.retire(event);
        }
        if events.is_empty() {
            return Vec::new();
        }
        let current = self.index.snapshot();
        events
            .into_iter()
            .map(|event| match event {
                Event::Preview {
                    id,
                    revision,
                    result,
                    total_ms,
                    ..
                } => {
                    if !self.closed
                        && revision == self.generation
                        && self
                            .submitted
                            .as_ref()
                            .is_some_and(|(expected, snapshot, _)| {
                                expected == &id && snapshot == &current
                            })
                    {
                        let accepted = result["payload"]["layout_capabilities"].as_array();
                        let missing_layout_capabilities = self
                            .layout_capabilities
                            .iter()
                            .filter(|cap| {
                                !accepted.is_some_and(|items| {
                                    items.iter().any(|item| item.as_str() == Some(cap.as_str()))
                                })
                            })
                            .cloned()
                            .collect();
                        Update::Preview(Preview {
                            missing_layout_capabilities,
                            request_id: id,
                            compile_revision: revision,
                            source_versions: current.clone(),
                            result,
                            runtime_total_ms: total_ms,
                            controller_total_ms: self
                                .submitted
                                .as_ref()
                                .unwrap()
                                .2
                                .elapsed()
                                .as_secs_f64()
                                * 1000.0,
                        })
                    } else {
                        Update::Discarded {
                            request_id: id,
                            compile_revision: revision,
                        }
                    }
                }
                other => Update::Runtime(other),
            })
            .collect()
    }
    /// Rebuild disposable index and compiler session from locked durable sources.
    pub fn restart(&mut self, command: Command, limits: Limits) -> Result<(), String> {
        if self.closed {
            return Err("project closed".into());
        }
        let expected = self.index.snapshot();
        let documents = self.membership_documents(None)?;
        let mut runtime = if self.raw_display_prototype {
            Session::spawn_command_raw_display_prototype(command, limits)?
        } else {
            Session::spawn_command(command, limits)?
        };
        self.display_enabled = false;
        self.layout_capabilities
            .retain(|cap| cap != "display-list-v2");
        runtime.set_completed_snapshots_enabled(self.historical.enabled)?;
        runtime.set_project_root(self.project_root.clone())?;
        self.replace_membership(&expected, &documents, None)?;
        self.runtime = Some(runtime);
        self.submitted = None;
        self.compile_current()
    }
    pub fn close(&mut self) -> Result<(), String> {
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.close_project(&self.project_id)?;
        }
        self.historical.invalidate();
        self.display_enabled = false;
        self.closed = true;
        self.submitted = None;
        Ok(())
    }
}
