//! Error-path and edge-case coverage for public `edit-ledger` APIs.
//! Complements the `#[cfg(test)]` suites in `src/` using only the public API.
use flashtex_edit_ledger::{
    checkpoint::{
        archive::{RotationPolicy, MAX_ARCHIVE_BYTES},
        Checkpoint, ExportAuthorization, ImportAction, ImportAuthorization, ImportPlan,
        MAX_CHECKPOINT_BYTES,
    },
    history::{GroupedEdit, HistoryMove, SourceEdit},
    recovery::{BridgeObservation, RecoveryImport},
    retention::RetentionPolicy,
    service::{BackgroundService, ServiceOptions, ServiceReply, MAX_FRAME_BYTES},
    AppliedReceipt, Document, Error, PreparedEdit, Store, MAX_EDIT_IDS,
};
use serde_json::json;

/// Machine-readable error code; `.err()` avoids the `Debug` bounds that
/// `unwrap_err` would require of success types such as `BackgroundService`.
fn code<T>(result: std::result::Result<T, Error>) -> String {
    result.err().unwrap().code
}

fn doc(text: &str) -> Document {
    Document::new("demo".into(), "main.tex".into(), 1, text.into()).unwrap()
}

fn open_init(text: &str) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    store.initialize(doc(text)).unwrap();
    (dir, store)
}

fn edit(document: &Document) -> PreparedEdit {
    PreparedEdit {
        capture_id: "capture-1".into(),
        edit_id: "edit-1".into(),
        project_id: document.project_id.clone(),
        path: document.path.clone(),
        expected_revision: document.revision,
        start_byte: 1,
        end_byte: 7,
        removed_text: "é😀".into(),
        replacement: "$x$".into(),
        document_before_sha256: document.source_sha256.clone(),
        wrap: None,
    }
}

fn token(store: &Store) -> String {
    store.export_recovery().unwrap().snapshot_token
}

fn retention(store: &Store, through: u64, keep: usize) -> RetentionPolicy {
    RetentionPolicy {
        snapshot_token: token(store),
        acknowledge_permanent_id_retention: true,
        acknowledged_through_revision: through,
        keep_latest_confirmed: keep,
    }
}

fn rec(store: &Store, observations: Vec<BridgeObservation>) -> RecoveryImport {
    RecoveryImport {
        snapshot_token: token(store),
        observations,
    }
}

fn export(store: &mut Store) -> Checkpoint {
    store
        .export_checkpoint(ExportAuthorization {
            acknowledge_private_source_export: true,
        })
        .unwrap()
}

fn auth(plan: &ImportPlan) -> ImportAuthorization {
    ImportAuthorization {
        approve_plan_id: plan.plan_id.clone(),
        allow_initialize_empty: true,
        allow_same_source_metadata: true,
    }
}

#[test]
fn document_new_rejects_bad_identifiers_and_paths() {
    let mut bad_ids: Vec<String> = ["", "has space", "slash/id", "dot.id"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    bad_ids.push("x".repeat(129));
    for bad in &bad_ids {
        let err = code(Document::new(bad.clone(), "main.tex".into(), 1, "t".into()));
        assert_eq!(err, "invalid_id", "{bad:?}");
    }
    for bad in [
        "", "/abs", "../up", "a//b", "a/./b", "a\\b", "a:b", "a\0b", ".", "a/..",
    ] {
        let err = code(Document::new("demo".into(), bad.into(), 1, "t".into()));
        assert_eq!(err, "invalid_path", "{bad:?}");
    }
    Document::new("x".repeat(128), "a/b.tex".into(), 1, "t".into()).unwrap();
}

#[test]
fn initialize_is_idempotent_only_for_identical_snapshot() {
    let (_dir, mut store) = open_init("aé😀z");
    store.initialize(doc("aé😀z")).unwrap();
}

#[test]
fn uninitialized_store_reports_document_missing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let d = doc("aé😀z");
    assert_eq!(code(store.apply(edit(&d))), "document_missing");
    assert_eq!(
        code(store.replace_document(1, "x", "y".into())),
        "document_missing"
    );
    let receipt = AppliedReceipt {
        capture_id: "c".into(),
        edit_id: "e".into(),
        new_revision: 2,
    };
    assert_eq!(code(store.confirm(&receipt)), "document_missing");
    assert_eq!(code(store.export_recovery()), "document_missing");
    assert_eq!(code(store.history_status()), "document_missing");
    let undo = HistoryMove {
        command_id: "u".into(),
        expected_revision: 1,
        expected_sha256: "x".into(),
    };
    assert_eq!(code(store.undo(undo)), "document_missing");
    // Reads that tolerate absence stay usable before initialization.
    assert!(store.document().unwrap().is_none());
    assert!(store.recovery().unwrap().is_empty());
}

#[test]
fn confirm_rejects_unknown_and_mismatched_retained_receipts() {
    let (_dir, mut store) = open_init("aé😀z");
    let unknown = AppliedReceipt {
        capture_id: "no".into(),
        edit_id: "no".into(),
        new_revision: 2,
    };
    assert_eq!(code(store.confirm(&unknown)), "edit_missing");
    let d = store.document().unwrap().unwrap().clone();
    let receipt = store.apply(edit(&d)).unwrap();
    store.confirm(&receipt).unwrap();
    store.confirm(&receipt).unwrap();
    store.compact(retention(&store, 2, 0)).unwrap();
    let altered = AppliedReceipt {
        capture_id: "other".into(),
        ..receipt.clone()
    };
    assert_eq!(code(store.confirm(&altered)), "receipt_conflict");
    store.confirm(&receipt).unwrap();
}

#[test]
fn apply_group_rejects_stale_and_malformed_batches() {
    let (_dir, mut store) = open_init("aé😀z");
    let d = store.document().unwrap().unwrap().clone();
    let base = |id: &str| GroupedEdit {
        command_id: id.into(),
        expected_revision: d.revision,
        expected_sha256: d.source_sha256.clone(),
        label: "g".into(),
        edits: vec![SourceEdit {
            start_byte: 1,
            end_byte: 3,
            removed_text: "é".into(),
            replacement: "E".into(),
        }],
    };
    let mut stale = base("s");
    stale.expected_revision = 99;
    assert_eq!(code(store.apply_group(stale)), "document_conflict");
    let mut empty = base("e");
    empty.edits.clear();
    assert_eq!(code(store.apply_group(empty)), "invalid_group");
    let mut affinity = base("a");
    affinity.edits = vec![
        SourceEdit {
            start_byte: 1,
            end_byte: 1,
            removed_text: "".into(),
            replacement: "A".into(),
        },
        SourceEdit {
            start_byte: 1,
            end_byte: 1,
            removed_text: "".into(),
            replacement: "B".into(),
        },
    ];
    assert_eq!(code(store.apply_group(affinity)), "overlapping_edits");
    let mut big = base("b");
    big.edits[0].replacement = "x".repeat(64 * 1024 + 1);
    assert_eq!(code(store.apply_group(big)), "replacement_too_large");
    assert_eq!(code(store.apply_group(base("bad/id"))), "invalid_id");
    assert_eq!(store.document().unwrap().unwrap().revision, 1);
    assert!(store.history_status().unwrap().undo_labels.is_empty());
}

#[test]
fn recovery_import_rejects_duplicate_oversized_and_regressed_observations() {
    let (_dir, mut store) = open_init("aé😀z");
    let d = store.document().unwrap().unwrap().clone();
    let edit = edit(&d);
    let receipt = store.apply(edit.clone()).unwrap();
    let plan = store.import_recovery(rec(&store, vec![])).unwrap();
    assert!(plan.actions.is_empty());
    assert_eq!(plan.recovery.snapshot_token, token(&store));
    let dup = || BridgeObservation::Unavailable {
        capture_id: "capture-1".into(),
        reason: "x".into(),
    };
    assert_eq!(
        code(store.import_recovery(rec(&store, vec![dup(), dup()]))),
        "duplicate_observation"
    );
    let bad = AppliedReceipt {
        new_revision: 99,
        ..receipt.clone()
    };
    let obs = vec![BridgeObservation::Applied { receipt: bad }];
    assert_eq!(
        code(store.import_recovery(rec(&store, obs))),
        "receipt_conflict"
    );
    let mut changed = edit.clone();
    changed.replacement = "other".into();
    let obs = vec![BridgeObservation::Prepared { edit: changed }];
    assert_eq!(
        code(store.import_recovery(rec(&store, obs))),
        "edit_id_conflict"
    );
    let long = BridgeObservation::Unavailable {
        capture_id: "capture-1".into(),
        reason: "x".repeat(4097),
    };
    assert_eq!(
        code(store.import_recovery(rec(&store, vec![long]))),
        "recovery_limit"
    );
    let one = BridgeObservation::Unavailable {
        capture_id: "c".into(),
        reason: "r".into(),
    };
    let many = vec![one; MAX_EDIT_IDS + 1];
    assert_eq!(
        code(store.import_recovery(rec(&store, many))),
        "recovery_limit"
    );
    store.confirm(&receipt).unwrap();
    let obs = vec![BridgeObservation::Prepared { edit }];
    assert_eq!(
        code(store.import_recovery(rec(&store, obs))),
        "bridge_state_regressed"
    );
    assert!(store.recovery().unwrap().is_empty());
}

#[test]
fn compact_rejects_stale_snapshot_invalid_policy_and_skips_newer() {
    let (_dir, mut store) = open_init("aé😀z");
    let d = store.document().unwrap().unwrap().clone();
    let receipt = store.apply(edit(&d)).unwrap();
    store.confirm(&receipt).unwrap();
    let stale = token(&store);
    let current = store.document().unwrap().unwrap().clone();
    store
        .replace_document(current.revision, &current.source_sha256, "new".into())
        .unwrap();
    let policy = |snapshot_token: String, through: u64, keep: usize| RetentionPolicy {
        snapshot_token,
        acknowledge_permanent_id_retention: true,
        acknowledged_through_revision: through,
        keep_latest_confirmed: keep,
    };
    assert_eq!(
        code(store.compact(policy(stale, 3, 0))),
        "stale_recovery_snapshot"
    );
    let fresh = token(&store);
    assert_eq!(
        code(store.compact(policy(fresh.clone(), 999, 0))),
        "invalid_retention_policy"
    );
    assert_eq!(
        code(store.compact(policy(fresh.clone(), 3, MAX_EDIT_IDS + 1))),
        "invalid_retention_policy"
    );
    assert_eq!(
        store
            .compact(policy(fresh.clone(), 1, 0))
            .unwrap()
            .compacted_payloads,
        0
    );
    assert_eq!(
        store
            .compact(policy(fresh, 3, 0))
            .unwrap()
            .compacted_payloads,
        1
    );
}

#[test]
fn checkpoint_import_plans_noop_and_denies_unapproved_restore() {
    let (_dir, mut store) = open_init("aé😀z");
    let d = store.document().unwrap().unwrap().clone();
    store.apply(edit(&d)).unwrap();
    let checkpoint = export(&mut store);
    let noop = store
        .plan_checkpoint_import(&checkpoint, checkpoint.identity.clone())
        .unwrap();
    assert_eq!(noop.action, ImportAction::NoOp);
    let before = token(&store);
    store
        .apply_checkpoint_import(&checkpoint, &noop, auth(&noop))
        .unwrap();
    assert_eq!(token(&store), before);
    let fresh = tempfile::tempdir().unwrap();
    let mut target = Store::open(fresh.path()).unwrap();
    let init = target
        .plan_checkpoint_import(&checkpoint, checkpoint.identity.clone())
        .unwrap();
    assert_eq!(init.action, ImportAction::InitializeEmpty);
    let mut denied = auth(&init);
    denied.allow_initialize_empty = false;
    assert_eq!(
        code(target.apply_checkpoint_import(&checkpoint, &init, denied)),
        "import_not_authorized"
    );
}

fn frame(value: serde_json::Value) -> Vec<u8> {
    format!("{value}\n").into_bytes()
}

fn ask(service: &BackgroundService, value: serde_json::Value) -> ServiceReply {
    service.try_submit(frame(value)).unwrap().wait().unwrap()
}

fn export_auth() -> ExportAuthorization {
    ExportAuthorization {
        acknowledge_private_source_export: true,
    }
}

fn rotation(keep_latest: usize, max_total_bytes: u64) -> RotationPolicy {
    RotationPolicy {
        acknowledge_checkpoint_deletion: true,
        keep_latest,
        max_total_bytes,
    }
}

fn group(document: &Document) -> GroupedEdit {
    GroupedEdit {
        command_id: "group-1".into(),
        expected_revision: document.revision,
        expected_sha256: document.source_sha256.clone(),
        label: "g".into(),
        edits: vec![SourceEdit {
            start_byte: 1,
            end_byte: 7,
            removed_text: "é😀".into(),
            replacement: "$x$".into(),
        }],
    }
}

#[test]
fn service_start_rejects_out_of_bound_options() {
    let dir = tempfile::tempdir().unwrap();
    // (capacity, max_request_bytes, max_reply_bytes); defaults fill the rest.
    let full = ServiceOptions::default();
    let bad = [
        (0, full.max_request_bytes, full.max_reply_bytes),
        (17, full.max_request_bytes, full.max_reply_bytes),
        (full.capacity, 0, full.max_reply_bytes),
        (full.capacity, MAX_FRAME_BYTES + 1, full.max_reply_bytes),
        (full.capacity, full.max_request_bytes, 4095),
        (full.capacity, full.max_request_bytes, 128 * 1024 * 1024 + 1),
    ];
    for (capacity, max_request_bytes, max_reply_bytes) in bad {
        let options = ServiceOptions {
            capacity,
            max_request_bytes,
            max_reply_bytes,
        };
        assert_eq!(
            code(BackgroundService::start(dir.path().into(), options)),
            "invalid_service_options"
        );
    }
    for (capacity, max_request_bytes, max_reply_bytes) in
        [(1, 1, 4096), (16, MAX_FRAME_BYTES, 128 * 1024 * 1024)]
    {
        let options = ServiceOptions {
            capacity,
            max_request_bytes,
            max_reply_bytes,
        };
        let service = BackgroundService::start(dir.path().into(), options).unwrap();
        assert!(!service.session_id().is_empty());
    }
}

#[test]
fn service_custom_frame_limit_and_empty_frame_are_rejected_before_execution() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServiceOptions {
        capacity: 2,
        max_request_bytes: 64,
        ..ServiceOptions::default()
    };
    let service = BackgroundService::start(dir.path().into(), options).unwrap();
    assert_eq!(code(service.try_submit(Vec::new())), "invalid_frame");
    let oversize = frame(json!({"id": "x".repeat(60), "operation": "status"}));
    assert!(oversize.len() > 64);
    assert_eq!(code(service.try_submit(oversize)), "invalid_frame");
    let reply = ask(&service, json!({"id": "ok", "operation": "status"}));
    assert!(reply.command_succeeded);
    assert_eq!(reply.sequence, 1);
    assert_eq!(reply.id.as_deref(), Some("ok"));
}

#[test]
fn service_request_id_and_operation_validation_never_echoes_unapproved_ids() {
    let dir = tempfile::tempdir().unwrap();
    let service = BackgroundService::start(dir.path().into(), ServiceOptions::default()).unwrap();
    for (request, expected) in [
        (json!({"operation": "status"}), "invalid_request"),
        (json!({"id": "", "operation": "status"}), "invalid_id"),
        (
            json!({"id": "x".repeat(129), "operation": "status"}),
            "invalid_id",
        ),
        (
            json!({"id": "nope", "operation": "explode"}),
            "invalid_request",
        ),
    ] {
        let reply = ask(&service, request);
        assert!(!reply.command_succeeded);
        assert_eq!(reply.error.as_ref().unwrap().code, expected);
        assert_eq!(reply.id, None);
    }
    let boundary = "y".repeat(128);
    let reply = ask(&service, json!({"id": boundary, "operation": "status"}));
    assert!(reply.command_succeeded);
    assert_eq!(reply.id.as_deref(), Some(boundary.as_str()));
}

#[test]
fn rotation_policy_bounds_are_rejected_before_any_archive_write() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    for policy in [
        rotation(0, MAX_ARCHIVE_BYTES),
        rotation(33, MAX_ARCHIVE_BYTES),
        rotation(1, 0),
        rotation(1, MAX_ARCHIVE_BYTES + 1),
    ] {
        assert_eq!(
            code(store.rotate_checkpoint(export_auth(), policy)),
            "invalid_rotation_policy"
        );
    }
    // Boundary policies pass validation and fail only on the missing document.
    for policy in [rotation(1, 1), rotation(32, MAX_ARCHIVE_BYTES)] {
        assert_eq!(
            code(store.rotate_checkpoint(export_auth(), policy)),
            "document_missing"
        );
    }
    assert!(!dir.path().join("checkpoints").exists());
}

#[test]
fn stray_archive_files_fail_closed_and_pending_flags_interruption() {
    let (dir, mut store) = open_init("aé😀z");
    store
        .rotate_checkpoint(export_auth(), rotation(2, MAX_ARCHIVE_BYTES))
        .unwrap();
    let root = dir.path().join("checkpoints");
    std::fs::write(root.join("stray.txt"), b"not a checkpoint").unwrap();
    assert_eq!(code(store.checkpoint_status()), "invalid_archive_file");
    assert_eq!(
        code(store.rotate_checkpoint(export_auth(), rotation(2, MAX_ARCHIVE_BYTES))),
        "invalid_archive_file"
    );
    // The committed index never scans the directory, so indexed reads survive.
    assert_eq!(store.list_checkpoints().unwrap().len(), 1);
    std::fs::remove_file(root.join("stray.txt")).unwrap();
    // A correctly named file with an undecodable body fails at decode.
    let corrupt = format!("{}-{}.json", "00000000000000000002", "0".repeat(64));
    std::fs::write(root.join(&corrupt), b"garbage").unwrap();
    assert_eq!(code(store.checkpoint_status()), "invalid_checkpoint");
    std::fs::remove_file(root.join(&corrupt)).unwrap();
    std::fs::write(root.join(".checkpoint-pending"), b"partial").unwrap();
    let status = store.checkpoint_status().unwrap();
    assert!(status.interrupted_write);
    assert!(status.unindexed_generations.is_empty());
    std::fs::remove_file(root.join(".checkpoint-pending")).unwrap();
    assert!(!store.checkpoint_status().unwrap().interrupted_write);
    {
        // Seventy valid checkpoint files plus the index exceed the 68-entry
        // scan bound even though every file decodes: rotation debris cannot
        // grow without bound after interrupted rotations.
        let (dir, mut store) = open_init("aé😀z");
        let rotated = store.rotate_checkpoint(export_auth(), rotation(32, MAX_ARCHIVE_BYTES));
        let first = rotated.unwrap();
        let bytes = store
            .read_checkpoint(first.created.generation)
            .unwrap()
            .encode()
            .unwrap();
        for generation in 2..=70u64 {
            let name = format!("{generation:020}-{}.json", first.created.checkpoint_digest);
            std::fs::write(dir.path().join("checkpoints").join(name), &bytes).unwrap();
        }
        assert_eq!(code(store.checkpoint_status()), "checkpoint_archive_limit");
    }
}

#[test]
fn checkpoint_decode_enforces_size_and_syntax_limits() {
    let (_dir, mut store) = open_init("aé😀z");
    let checkpoint = export(&mut store);
    let bytes = checkpoint.encode().unwrap();
    assert_eq!(Checkpoint::decode(&bytes).unwrap().document().text, "aé😀z");
    assert_eq!(code(Checkpoint::decode(b"not json")), "invalid_checkpoint");
    assert_eq!(
        code(Checkpoint::decode(b"{\"format_version\":1")),
        "invalid_checkpoint"
    );
    assert_eq!(
        code(Checkpoint::decode(&vec![b'x'; MAX_CHECKPOINT_BYTES + 1])),
        "checkpoint_too_large"
    );
}

#[test]
fn same_source_restore_requires_its_own_authorization() {
    let (_dir, mut source) = open_init("aé😀z");
    let document = source.document().unwrap().unwrap().clone();
    let receipt = source.apply(edit(&document)).unwrap();
    let pending = export(&mut source);
    let fresh = tempfile::tempdir().unwrap();
    let mut target = Store::open(fresh.path()).unwrap();
    let init = target
        .plan_checkpoint_import(&pending, pending.identity.clone())
        .unwrap();
    assert_eq!(init.action, ImportAction::InitializeEmpty);
    target
        .apply_checkpoint_import(&pending, &init, auth(&init))
        .unwrap();
    source.confirm(&receipt).unwrap();
    let acknowledged = export(&mut source);
    let plan = target
        .plan_checkpoint_import(&acknowledged, acknowledged.identity.clone())
        .unwrap();
    assert_eq!(plan.action, ImportAction::SameSourceMetadata);
    let mut denied = auth(&plan);
    denied.allow_same_source_metadata = false;
    assert_eq!(
        code(target.apply_checkpoint_import(&acknowledged, &plan, denied)),
        "import_not_authorized"
    );
    target
        .apply_checkpoint_import(&acknowledged, &plan, auth(&plan))
        .unwrap();
    assert!(target.recovery().unwrap().is_empty());
    assert_eq!(target.apply(edit(&document)).unwrap(), receipt);
}

#[test]
fn checkpoint_losing_history_is_blocked_from_import() {
    let (_dir, mut store) = open_init("aé😀z");
    let document = store.document().unwrap().unwrap().clone();
    let grouped = store.apply_group(group(&document)).unwrap();
    let before_undo = export(&mut store);
    let at = |id: &str, document: &Document| HistoryMove {
        command_id: id.into(),
        expected_revision: document.revision,
        expected_sha256: document.source_sha256.clone(),
    };
    let undone = store.undo(at("undo-1", &grouped.document)).unwrap();
    store.redo(at("redo-1", &undone.document)).unwrap();
    // Same source text as the checkpoint, but the checkpoint predates two
    // command IDs. Every history command also bumps the revision, so the live
    // source conflict is expected alongside the history-loss conflict.
    assert_eq!(store.document().unwrap().unwrap().text, "a$x$z");
    let plan = store
        .plan_checkpoint_import(&before_undo, before_undo.identity.clone())
        .unwrap();
    assert_eq!(plan.action, ImportAction::Blocked);
    assert!(
        plan.conflicts.iter().any(|c| c.contains("lose history")),
        "{:?}",
        plan.conflicts
    );
    assert_eq!(
        code(store.apply_checkpoint_import(&before_undo, &plan, auth(&plan))),
        "restore_conflict"
    );
}
