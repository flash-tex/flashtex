//! Internal bounded origin bookkeeping for opt-in historical display; no native protocol.
use flashtex_document_runtime::{CompletedSnapshot, Event};
use flashtex_project_index::VersionSnapshot;
use serde_json::Value;
use std::collections::BTreeMap;

/// A completed older source snapshot. It cannot authorize source navigation or edits.
#[derive(Debug)]
pub struct HistoricalPreview {
    request_id: String,
    compile_revision: u64,
    source_versions: VersionSnapshot,
    result: Value,
    incarnation: String,
    epoch: u64,
}
impl HistoricalPreview {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn compile_revision(&self) -> u64 {
        self.compile_revision
    }
    pub fn source_versions(&self) -> &VersionSnapshot {
        &self.source_versions
    }
    /// Move the validated result into a wire frame without cloning its display list.
    pub fn into_result(self) -> Value {
        self.result
    }
    pub fn result(&self) -> &Value {
        &self.result
    }
}
struct Binding {
    origin: String,
    request_id: String,
    versions: VersionSnapshot,
}
#[derive(Default)]
pub(crate) struct HistoricalState {
    pub enabled: bool,
    incarnation: String,
    epoch: u64,
    floor: u64,
    bindings: BTreeMap<u64, Binding>,
    retained: Option<HistoricalPreview>,
}
impl HistoricalState {
    pub fn configure(&mut self, enabled: bool) -> Result<(), String> {
        if enabled && self.incarnation.is_empty() {
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
            self.incarnation = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        }
        self.invalidate();
        if self.epoch == u64::MAX {
            return Err("historical policy epoch exhausted".into());
        }
        self.enabled = enabled;
        Ok(())
    }
    pub fn invalidate(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
        if self.epoch == u64::MAX {
            self.enabled = false;
        }
        self.bindings.clear();
        self.retained = None;
    }
    pub fn origin(&self, generation: u64) -> Option<String> {
        self.enabled
            .then(|| format!("{}:{}:{generation}", self.incarnation, self.epoch))
    }
    pub fn record(
        &mut self,
        generation: u64,
        origin: String,
        request_id: String,
        versions: VersionSnapshot,
    ) {
        if self.bindings.len() == 64 {
            self.bindings.pop_first();
        }
        self.bindings.insert(
            generation,
            Binding {
                origin,
                request_id,
                versions,
            },
        );
    }
    pub fn consume(&mut self, completed: CompletedSnapshot) {
        let Some(binding) = self.bindings.remove(&completed.revision) else {
            return;
        };
        if !self.enabled
            || completed.origin != binding.origin
            || completed.request_id != binding.request_id
            || completed.project_id != binding.versions.project_id
            || completed.revision <= self.floor
        {
            return;
        }
        self.retained = Some(HistoricalPreview {
            request_id: completed.request_id,
            compile_revision: completed.revision,
            source_versions: binding.versions,
            result: completed.result,
            incarnation: self.incarnation.clone(),
            epoch: self.epoch,
        });
    }
    pub fn retire(&mut self, event: &Event) {
        let id = match event {
            Event::Preview { id, revision, .. } => {
                self.floor = self.floor.max(*revision);
                self.retained = None;
                id
            }
            Event::Failed { .. } => {
                self.invalidate();
                return;
            }
            Event::Stale { id, .. } | Event::Cancelled { id } | Event::Superseded { id, .. } => id,
        };
        self.bindings.retain(|_, binding| binding.request_id != *id);
    }
    pub fn take(&mut self) -> Option<HistoricalPreview> {
        self.retained.take()
    }
    pub fn claim(
        &mut self,
        preview: &HistoricalPreview,
        project: &str,
        current_generation: u64,
    ) -> bool {
        if !self.enabled
            || preview.incarnation != self.incarnation
            || preview.epoch != self.epoch
            || preview.source_versions.project_id != project
            || preview.compile_revision <= self.floor
            || preview.compile_revision >= current_generation
        {
            return false;
        }
        self.floor = preview.compile_revision;
        true
    }
    pub fn binding_count(&self) -> usize {
        self.bindings.len()
    }
}
