//! Ledger-local recovery exchange. Imports contain bridge observations, never
//! document text. A snapshot token prevents an old response from mutating newer
//! ledger state while the Mac adapter awaits the bridge.
use crate::{AppliedReceipt, AppliedTransaction, Document, Error, PreparedEdit, Result, Store};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryExport {
    pub snapshot_token: String,
    pub current_document: Document,
    pub pending_receipts: Vec<AppliedTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryImport {
    pub snapshot_token: String,
    pub observations: Vec<BridgeObservation>,
}

/// These are local adapter fields, not additions to transfer-v1 messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
#[allow(
    clippy::large_enum_variant,
    reason = "public serde type; boxing the variant would change its API"
)]
pub enum BridgeObservation {
    Applied { receipt: AppliedReceipt },
    Prepared { edit: PreparedEdit },
    Unavailable { capture_id: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum RecoveryAction {
    Confirmed {
        receipt: AppliedReceipt,
    },
    /// Reopen this original source on the bridge, replay receipt, then restore
    /// current durable source. This never modifies source in the local store.
    ReplayReceipt {
        document_before: Document,
        receipt: AppliedReceipt,
    },
    RetryStatus {
        capture_id: String,
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPlan {
    pub actions: Vec<RecoveryAction>,
    /// Fresh token and current source after any acknowledgement persistence.
    pub recovery: RecoveryExport,
}

impl Store {
    pub fn export_recovery(&self) -> Result<RecoveryExport> {
        self.ready()?;
        let state = self
            .state
            .as_ref()
            .ok_or_else(|| Error::new("document_missing", "initialize source first"))?;
        let bytes =
            serde_json::to_vec(state).map_err(|e| Error::new("invalid_store", e.to_string()))?;
        Ok(RecoveryExport {
            snapshot_token: format!("{:x}", Sha256::digest(bytes)),
            current_document: state.document.clone(),
            pending_receipts: self.recovery()?,
        })
    }

    /// Import verified capture_status observations as one atomic acknowledgement
    /// batch. A missing/failed bridge response retains the original snapshot.
    /// Conflicts reject the entire batch without partial confirmation.
    pub fn import_recovery(&mut self, input: RecoveryImport) -> Result<RecoveryPlan> {
        let exported = self.export_recovery()?;
        if exported.snapshot_token != input.snapshot_token {
            return Err(Error::new(
                "stale_recovery_snapshot",
                "source or ledger advanced while querying bridge; export again",
            ));
        }
        if input.observations.len() > crate::MAX_EDIT_IDS {
            return Err(Error::new("recovery_limit", "too many bridge observations"));
        }
        let mut next = self
            .state
            .as_ref()
            .expect("export checked initialization")
            .clone();
        let mut seen = BTreeSet::new();
        let mut actions = Vec::new();
        let mut changed = false;
        for observation in input.observations {
            let capture_id = match &observation {
                BridgeObservation::Applied { receipt } => &receipt.capture_id,
                BridgeObservation::Prepared { edit } => &edit.capture_id,
                BridgeObservation::Unavailable { capture_id, .. } => capture_id,
            };
            if !seen.insert(capture_id.clone()) {
                return Err(Error::new(
                    "duplicate_observation",
                    "one observation per capture is required",
                ));
            }
            let tx = next
                .transactions
                .values_mut()
                .find(|t| &t.edit.capture_id == capture_id)
                .ok_or_else(|| {
                    Error::new(
                        "capture_missing",
                        "observation does not name a locally applied capture",
                    )
                })?;
            match observation {
                BridgeObservation::Applied { receipt } => {
                    if tx.receipt != receipt {
                        return Err(Error::new(
                            "receipt_conflict",
                            "bridge applied fields differ from local durable receipt",
                        ));
                    }
                    if !tx.confirmed {
                        tx.confirmed = true;
                        tx.document_before = None;
                        changed = true;
                    }
                    actions.push(RecoveryAction::Confirmed { receipt });
                }
                BridgeObservation::Prepared { edit } => {
                    if tx.edit != edit {
                        return Err(Error::new(
                            "edit_id_conflict",
                            "bridge prepared fields differ from the durable local edit",
                        ));
                    }
                    let Some(document_before) = tx.document_before.clone() else {
                        return Err(Error::new("bridge_state_regressed", "bridge lost a previously acknowledged receipt; original snapshot is no longer retained"));
                    };
                    actions.push(RecoveryAction::ReplayReceipt {
                        document_before,
                        receipt: tx.receipt.clone(),
                    });
                }
                BridgeObservation::Unavailable { capture_id, reason } => {
                    if reason.len() > 4096 {
                        return Err(Error::new(
                            "recovery_limit",
                            "failure reason exceeds 4096 bytes",
                        ));
                    }
                    actions.push(RecoveryAction::RetryStatus { capture_id, reason });
                }
            }
        }
        actions.sort_by_key(|action| match action {
            RecoveryAction::Confirmed { receipt }
            | RecoveryAction::ReplayReceipt { receipt, .. } => receipt.new_revision,
            RecoveryAction::RetryStatus { .. } => u64::MAX,
        });
        if changed {
            self.commit(next)?;
        }
        Ok(RecoveryPlan {
            actions,
            recovery: self.export_recovery()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::setup;

    #[test]
    fn replay_plan_retains_original_and_never_reseeds_current_source() {
        let (_dir, mut store, edit) = setup();
        let receipt = store.apply(edit.clone()).unwrap();
        let current = store.document().unwrap().unwrap().clone();
        store
            .replace_document(
                current.revision,
                &current.source_sha256,
                "later typing".into(),
            )
            .unwrap();
        let exported = store.export_recovery().unwrap();
        let result = store
            .import_recovery(RecoveryImport {
                snapshot_token: exported.snapshot_token,
                observations: vec![BridgeObservation::Prepared { edit }],
            })
            .unwrap();
        assert_eq!(result.recovery.current_document.text, "later typing");
        assert_eq!(result.recovery.current_document.revision, 3);
        assert_eq!(
            result.actions,
            vec![RecoveryAction::ReplayReceipt {
                document_before: crate::tests::doc("aé😀z"),
                receipt
            }]
        );
    }

    #[test]
    fn unavailable_status_keeps_recovery_data_and_token() {
        let (_dir, mut store, edit) = setup();
        store.apply(edit).unwrap();
        let exported = store.export_recovery().unwrap();
        let result = store
            .import_recovery(RecoveryImport {
                snapshot_token: exported.snapshot_token.clone(),
                observations: vec![BridgeObservation::Unavailable {
                    capture_id: "capture-1".into(),
                    reason: "bridge disconnected".into(),
                }],
            })
            .unwrap();
        assert_eq!(result.recovery.snapshot_token, exported.snapshot_token);
        assert!(result.recovery.pending_receipts[0]
            .document_before
            .is_some());
    }

    #[test]
    fn changed_source_or_ledger_invalidates_export_token() {
        let (_dir, mut store, edit) = setup();
        let receipt = store.apply(edit).unwrap();
        let exported = store.export_recovery().unwrap();
        store.confirm(&receipt).unwrap();
        assert_eq!(
            store
                .import_recovery(RecoveryImport {
                    snapshot_token: exported.snapshot_token,
                    observations: vec![]
                })
                .unwrap_err()
                .code,
            "stale_recovery_snapshot"
        );
    }

    #[test]
    fn applied_import_is_durable_and_returns_new_token() {
        let (dir, mut store, edit) = setup();
        let receipt = store.apply(edit).unwrap();
        let exported = store.export_recovery().unwrap();
        let plan = store
            .import_recovery(RecoveryImport {
                snapshot_token: exported.snapshot_token.clone(),
                observations: vec![BridgeObservation::Applied { receipt }],
            })
            .unwrap();
        assert_ne!(plan.recovery.snapshot_token, exported.snapshot_token);
        assert!(plan.recovery.pending_receipts.is_empty());
        drop(store);
        assert!(Store::open(dir.path())
            .unwrap()
            .recovery()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn conflicting_batch_does_not_partially_confirm() {
        let (_dir, mut store, edit) = setup();
        let receipt = store.apply(edit).unwrap();
        let token = store.export_recovery().unwrap().snapshot_token;
        let error = store
            .import_recovery(RecoveryImport {
                snapshot_token: token.clone(),
                observations: vec![
                    BridgeObservation::Applied { receipt },
                    BridgeObservation::Unavailable {
                        capture_id: "unknown-capture".into(),
                        reason: "missing".into(),
                    },
                ],
            })
            .unwrap_err();
        assert_eq!(error.code, "capture_missing");
        assert_eq!(store.export_recovery().unwrap().snapshot_token, token);
        assert_eq!(store.recovery().unwrap().len(), 1);
    }
}
