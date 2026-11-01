//! Durable edit acknowledgement without copying source into the response.
use crate::{Controller, HistoryAction};
use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Serialize)]
pub struct DocumentMetadata {
    pub project_id: String,
    pub path: String,
    pub revision: u64,
    pub source_sha256: String,
    pub byte_length: usize,
}
#[derive(Debug, Serialize)]
pub struct MetadataEditOutcome {
    pub document: DocumentMetadata,
    pub compile_admission: Option<crate::CompileAdmission>,
    pub preview_error: Option<String>,
    pub save_and_submit_ms: f64,
}
impl From<&flashtex_edit_ledger::Document> for DocumentMetadata {
    fn from(saved: &flashtex_edit_ledger::Document) -> Self {
        Self {
            project_id: saved.project_id.clone(),
            path: saved.path.clone(),
            revision: saved.revision,
            source_sha256: saved.source_sha256.clone(),
            byte_length: saved.text.len(),
        }
    }
}
#[derive(Debug, Serialize)]
pub struct MetadataHistory {
    pub document: DocumentMetadata,
    pub command_revision: u64,
    pub replayed_command: bool,
    pub can_undo: bool,
    pub can_redo: bool,
}
#[derive(Debug, Serialize)]
pub struct MetadataGroupOutcome {
    pub history: MetadataHistory,
    pub compile_admission: Option<crate::CompileAdmission>,
    pub preview_error: Option<String>,
    pub save_and_submit_ms: f64,
}
impl Controller {
    pub fn apply_group_metadata(
        &mut self,
        path: &str,
        group: flashtex_edit_ledger::history::GroupedEdit,
    ) -> Result<MetadataGroupOutcome, String> {
        self.apply_history_metadata(path, HistoryAction::Group(group))
    }
    pub fn apply_history_metadata(
        &mut self,
        path: &str,
        action: HistoryAction,
    ) -> Result<MetadataGroupOutcome, String> {
        if self.closed {
            return Err("project closed".into());
        }
        let started = Instant::now();
        self.submitted = None;
        let store = self.stores.get_mut(path).ok_or("unknown document")?;
        let result = match action {
            HistoryAction::Group(group) => store.apply_group_status(group),
            HistoryAction::Undo(command) => store.undo_status(command),
            HistoryAction::Redo(command) => store.redo_status(command),
        }
        .map_err(|e| e.to_string())?;
        let saved = store
            .document()
            .map_err(|e| e.to_string())?
            .ok_or("saved source missing")?;
        let history = MetadataHistory {
            document: DocumentMetadata::from(saved),
            command_revision: result.command_revision,
            replayed_command: result.replayed_command,
            can_undo: result.can_undo,
            can_redo: result.can_redo,
        };
        let indexed = Self::index_saved_document(&mut self.index, saved);
        let (preview_error, save_and_submit_ms, compile_admission) =
            self.finish_saved_index_with_admission(indexed, started);
        Ok(MetadataGroupOutcome {
            history,
            compile_admission,
            preview_error,
            save_and_submit_ms,
        })
    }
    pub fn replace_document_metadata(
        &mut self,
        path: &str,
        expected_revision: u64,
        expected_sha256: &str,
        text: String,
    ) -> Result<MetadataEditOutcome, String> {
        if self.closed {
            return Err("project closed".into());
        }
        let started = Instant::now();
        self.submitted = None;
        let store = self.stores.get_mut(path).ok_or("unknown document")?;
        store
            .replace_document_in_place(expected_revision, expected_sha256, text)
            .map_err(|e| e.to_string())?;
        let saved = store
            .document()
            .map_err(|e| e.to_string())?
            .ok_or("saved source missing")?;
        let document = DocumentMetadata::from(saved);
        let indexed = Self::index_saved_document(&mut self.index, saved);
        let (preview_error, save_and_submit_ms, compile_admission) =
            self.finish_saved_index_with_admission(indexed, started);
        Ok(MetadataEditOutcome {
            document,
            compile_admission,
            preview_error,
            save_and_submit_ms,
        })
    }
}
