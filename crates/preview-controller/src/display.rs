//! Current-source correlation only; forwarded display data still needs renderer validation.
use crate::Controller;
use serde::Serialize;
use serde_json::value::RawValue;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct RawDisplayPayload {
    kind: &'static str,
    untrusted: bool,
    source_actions_enabled: bool,
    request_id: String,
    project_id: String,
    compile_revision: u64,
    source_versions: BTreeMap<String, u64>,
    membership_generation: u64,
    display_list: Box<RawValue>,
}

impl Controller {
    /// Fixed startup strategy, before any compiler request or session exists.
    pub fn select_raw_display_prototype(&mut self) -> Result<(), String> {
        if self.closed || self.runtime.is_some() || self.generation != 0 {
            return Err("display decoder strategy is fixed before compiler startup".into());
        }
        self.raw_display_prototype = true;
        Ok(())
    }
    pub fn display_candidate_capability(&self) -> &'static str {
        if self.raw_display_prototype {
            "display-candidates-raw-v1"
        } else {
            "display-candidates-v1"
        }
    }

    pub fn take_current_raw_display_payload(&mut self) -> Option<RawDisplayPayload> {
        if self.closed || !self.display_enabled || !self.raw_display_prototype {
            return None;
        }
        let candidate = self
            .runtime
            .as_mut()?
            .take_current_raw_display_candidate()?;
        let (id, submitted, _) = self.submitted.as_ref()?;
        let current = self.index.snapshot();
        if candidate.request_id() != id
            || candidate.project_id() != self.project_id
            || candidate.revision() != self.generation
            || submitted != &current
            || candidate.sources().len() != current.documents.len()
        {
            return None;
        }
        for source in candidate.sources() {
            let document = self.document(&source.path).ok()?;
            if current.documents.get(&source.path) != Some(&document.revision)
                || source.sha256 != document.source_sha256
                || source.byte_length != document.text.len()
            {
                return None;
            }
        }
        Some(RawDisplayPayload {
            kind: "display_candidate",
            untrusted: true,
            source_actions_enabled: false,
            request_id: candidate.request_id().into(),
            project_id: candidate.project_id().into(),
            compile_revision: candidate.revision(),
            source_versions: current.documents,
            membership_generation: current.generation,
            display_list: candidate.into_raw(),
        })
    }
    /// Transport timing only; source text and renderer validation are excluded.
    pub fn last_display_profile(
        &self,
    ) -> Option<&flashtex_document_runtime::DisplayResponseProfile> {
        if self.closed || !self.display_enabled {
            return None;
        }
        self.runtime.as_ref()?.last_display_profile()
    }
    /// Returns a separate compile error after successful policy negotiation.
    /// No already-submitted work is retroactively enrolled.
    pub fn configure_display_candidates(
        &mut self,
        enabled: bool,
    ) -> Result<Option<String>, String> {
        if self.closed {
            return Err("project closed".into());
        }
        if enabled && self.historical.enabled {
            return Err(
                "display candidates and historical snapshots are mutually exclusive".into(),
            );
        }
        if enabled
            && !self
                .runtime
                .as_ref()
                .is_some_and(|runtime| runtime.is_alive())
        {
            return Err("compiler unavailable".into());
        }
        let mut capabilities = self.layout_capabilities.clone();
        capabilities.retain(|cap| cap != "display-list-v2");
        if enabled {
            capabilities.push("display-list-v2".into());
        }
        flashtex_document_runtime::validate_layout_capabilities(&capabilities)?;
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.set_display_candidates_enabled(enabled)?;
        }
        self.display_enabled = enabled;
        self.layout_capabilities = capabilities;
        self.submitted = None;
        Ok(self.compile_current().err())
    }

    /// Call only when optional output can be admitted. Moves one candidate, then
    /// checks controller identity on the serialized owner before constructing output.
    pub fn take_current_display_payload(&mut self) -> Option<Value> {
        if self.closed || !self.display_enabled {
            return None;
        }
        let candidate = self.runtime.as_mut()?.take_current_display_candidate()?;
        let (id, submitted, _) = self.submitted.as_ref()?;
        let current = self.index.snapshot();
        if candidate.request_id() != id
            || candidate.project_id() != self.project_id
            || candidate.revision() != self.generation
            || submitted != &current
            || candidate.sources().len() != current.documents.len()
        {
            return None;
        }
        for source in candidate.sources() {
            let document = self.document(&source.path).ok()?;
            if current.documents.get(&source.path) != Some(&document.revision)
                || source.sha256 != document.source_sha256
                || source.byte_length != document.text.len()
            {
                return None;
            }
        }
        let mut payload = json!({"kind":"display_candidate","untrusted":true,
            "source_actions_enabled":false,"request_id":candidate.request_id(),
            "project_id":candidate.project_id(),"compile_revision":candidate.revision(),
            "source_versions":current.documents,"membership_generation":current.generation});
        payload["display_list"] = candidate.into_envelope();
        Some(payload)
    }
}
