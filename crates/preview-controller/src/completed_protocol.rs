//! Bounded submission-token bookkeeping for the negotiated helper adapter.
//! Tokens are recorded by the serialized request owner, never at completion time.
use std::collections::BTreeMap;

pub const CAPABILITY: &str = "completed-snapshots-v1";

#[derive(Default)]
pub struct SubmissionBindings {
    enabled: bool,
    epoch: u64,
    tokens: BTreeMap<u64, String>,
}
impl SubmissionBindings {
    /// Validate before any operation can mutate durable source.
    pub fn validate_token(token: &str) -> Result<(), &'static str> {
        if token.is_empty() || token.len() > 128 {
            return Err("source_binding_token must contain 1..128 bytes");
        }
        Ok(())
    }
    pub fn configure(&mut self, enabled: bool) -> Result<u64, &'static str> {
        self.tokens.clear();
        self.enabled = false;
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or("negotiation epoch exhausted")?;
        self.enabled = enabled;
        Ok(self.epoch)
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Call after successful admission, before polling completions. A generation's
    /// first token is immutable; duplicate bookkeeping cannot replace its origin.
    pub fn record(&mut self, generation: u64, token: &str) -> Result<(), &'static str> {
        Self::validate_token(token)?;
        if !self.enabled {
            return Err("completed snapshots not negotiated");
        }
        if self.tokens.contains_key(&generation) {
            return Err("submission already bound");
        }
        if self.tokens.len() == 64 {
            self.tokens.pop_first();
        }
        self.tokens.insert(generation, token.to_owned());
        Ok(())
    }
    /// Policy epoch must have been captured with the completion, not stamped later.
    pub fn take(&mut self, epoch: u64, generation: u64) -> Option<String> {
        if !self.enabled || epoch != self.epoch {
            return None;
        }
        self.tokens.remove(&generation)
    }
    pub fn retire_through(&mut self, generation: u64) {
        self.tokens.retain(|revision, _| *revision > generation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_token_is_opaque_immutable_and_consumed_once() {
        let mut bindings = SubmissionBindings::default();
        assert!(bindings.record(1, "token").is_err());
        let epoch = bindings.configure(true).unwrap();
        let token = "editor:α\nopaque";
        bindings.record(1, token).unwrap();
        assert!(bindings.record(1, "newer-editor").is_err());
        assert_eq!(bindings.take(epoch, 1).as_deref(), Some(token));
        assert_eq!(bindings.take(epoch, 1), None);
    }
    #[test]
    fn restart_and_negotiation_reject_old_callbacks_without_consuming_new_origin() {
        let mut bindings = SubmissionBindings::default();
        let old_epoch = bindings.configure(true).unwrap();
        bindings.record(1, "old").unwrap();
        bindings.configure(false).unwrap();
        assert_eq!(bindings.take(old_epoch, 1), None);
        let new_epoch = bindings.configure(true).unwrap();
        bindings.record(1, "new").unwrap();
        assert_eq!(bindings.take(old_epoch, 1), None);
        assert_eq!(bindings.take(new_epoch, 1).as_deref(), Some("new"));
    }
    #[test]
    fn eviction_and_current_floor_bound_optional_metadata() {
        let mut bindings = SubmissionBindings::default();
        let epoch = bindings.configure(true).unwrap();
        for generation in 1..=80 {
            bindings.record(generation, "x").unwrap();
            assert!(bindings.tokens.len() <= 64);
        }
        assert_eq!(bindings.take(epoch, 16), None);
        assert_eq!(bindings.take(epoch, 17).as_deref(), Some("x"));
        bindings.retire_through(79);
        assert_eq!(bindings.tokens.len(), 1);
        assert_eq!(bindings.take(epoch, 80).as_deref(), Some("x"));
        assert!(SubmissionBindings::validate_token("").is_err());
        assert!(SubmissionBindings::validate_token(&"α".repeat(65)).is_err());
        assert!(SubmissionBindings::validate_token(&"α".repeat(64)).is_ok());
    }
}
