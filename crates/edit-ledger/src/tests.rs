use super::*;

pub(crate) fn doc(text: &str) -> Document {
    Document::new("demo".into(), "main.tex".into(), 1, text.into()).unwrap()
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
pub(crate) fn setup() -> (tempfile::TempDir, Store, PreparedEdit) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let document = doc("aé😀z");
    let edit = edit(&document);
    store.initialize(document).unwrap();
    (dir, store, edit)
}

#[test]
fn application_receipt_and_source_survive_reopen() {
    let (dir, mut store, edit) = setup();
    let receipt = store.apply(edit.clone()).unwrap();
    assert_eq!(receipt.new_revision, 2);
    assert_eq!(store.document().unwrap().unwrap().text, "a$x$z");
    drop(store);
    let mut reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.document().unwrap().unwrap().text, "a$x$z");
    assert_eq!(reopened.apply(edit).unwrap(), receipt);
    assert_eq!(reopened.document().unwrap().unwrap().revision, 2);
    assert_eq!(
        reopened.recovery().unwrap()[0]
            .document_before
            .as_ref()
            .unwrap()
            .text,
        "aé😀z"
    );
}

#[test]
fn confirm_requires_exact_receipt_and_preserves_idempotence_after_undo() {
    let (dir, mut store, edit) = setup();
    let receipt = store.apply(edit.clone()).unwrap();
    for altered in [
        AppliedReceipt {
            capture_id: "wrong".into(),
            ..receipt.clone()
        },
        AppliedReceipt {
            new_revision: 3,
            ..receipt.clone()
        },
    ] {
        assert_eq!(
            store.confirm(&altered).unwrap_err().code,
            "receipt_conflict"
        );
    }
    store.confirm(&receipt).unwrap();
    assert!(store.recovery().unwrap().is_empty());
    let current = store.document().unwrap().unwrap().clone();
    store
        .replace_document(current.revision, &current.source_sha256, "aé😀z".into())
        .unwrap();
    drop(store);
    let mut reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.apply(edit).unwrap(), receipt);
    assert_eq!(reopened.document().unwrap().unwrap().text, "aé😀z");
    assert_eq!(reopened.document().unwrap().unwrap().revision, 3);
    reopened.confirm(&receipt).unwrap();
}

#[test]
fn all_guards_reject_before_mutation() {
    let (_dir, mut store, valid) = setup();
    let mut cases = Vec::new();
    let mut e = valid.clone();
    e.project_id = "elsewhere".into();
    cases.push((e, "document_conflict"));
    let mut e = valid.clone();
    e.path = "other.tex".into();
    cases.push((e, "document_conflict"));
    let mut e = valid.clone();
    e.expected_revision = 2;
    cases.push((e, "revision_conflict"));
    let mut e = valid.clone();
    e.document_before_sha256 = "0".repeat(64);
    cases.push((e, "source_hash_conflict"));
    let mut e = valid.clone();
    e.removed_text = "é".into();
    cases.push((e, "removed_text_conflict"));
    let mut e = valid.clone();
    e.edit_id = "bad/id".into();
    cases.push((e, "invalid_id"));
    let mut e = valid.clone();
    e.replacement = "x".repeat(MAX_REPLACEMENT_BYTES + 1);
    cases.push((e, "replacement_too_large"));
    for (e, expected) in cases {
        assert_eq!(store.apply(e).unwrap_err().code, expected);
        assert_eq!(store.document().unwrap().unwrap().text, "aé😀z");
        assert!(store.recovery().unwrap().is_empty());
    }
}

#[test]
fn unicode_interior_and_invalid_ranges_rejected() {
    let (_dir, mut store, valid) = setup();
    for (start, end) in [
        (2, 7),
        (4, 7),
        (5, 7),
        (6, 7),
        (1, 2),
        (1, 4),
        (1, 5),
        (1, 6),
        (7, 1),
        (1, 9),
        (usize::MAX, usize::MAX),
    ] {
        let e = PreparedEdit {
            start_byte: start,
            end_byte: end,
            ..valid.clone()
        };
        assert_eq!(store.apply(e).unwrap_err().code, "invalid_source_range");
    }
}

#[test]
fn empty_range_insertion_is_supported() {
    let (_dir, mut store, valid) = setup();
    let e = PreparedEdit {
        end_byte: 1,
        removed_text: String::new(),
        ..valid
    };
    store.apply(e).unwrap();
    assert_eq!(store.document().unwrap().unwrap().text, "a$x$é😀z");
}

#[test]
fn conflicting_edit_and_capture_retries_refused() {
    let (_dir, mut store, valid) = setup();
    store.apply(valid.clone()).unwrap();
    let changed = PreparedEdit {
        replacement: "other".into(),
        ..valid.clone()
    };
    assert_eq!(store.apply(changed).unwrap_err().code, "edit_id_conflict");
    let reused_capture = PreparedEdit {
        edit_id: "edit-2".into(),
        ..valid
    };
    assert_eq!(
        store.apply(reused_capture).unwrap_err().code,
        "capture_id_conflict"
    );
}

#[test]
fn io_failure_before_rename_keeps_old_source_without_receipt() {
    let (dir, mut store, edit) = setup();
    store.failpoint = Some("before_rename");
    assert_eq!(
        store.apply(edit.clone()).unwrap_err().code,
        "injected_io_error"
    );
    assert_eq!(
        store.apply(edit.clone()).unwrap_err().code,
        "recovery_required"
    );
    assert_eq!(store.document().unwrap_err().code, "recovery_required");
    drop(store);
    let mut reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.document().unwrap().unwrap().text, "aé😀z");
    assert!(reopened.recovery().unwrap().is_empty());
    reopened.apply(edit).unwrap();
}

#[test]
fn io_failure_after_rename_recovers_both_source_and_ledger() {
    let (dir, mut store, edit) = setup();
    store.failpoint = Some("after_rename");
    assert_eq!(
        store.apply(edit.clone()).unwrap_err().code,
        "injected_io_error"
    );
    drop(store);
    let mut reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.document().unwrap().unwrap().text, "a$x$z");
    let pending = reopened.recovery().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(reopened.apply(edit).unwrap(), pending[0].receipt);
    assert_eq!(reopened.document().unwrap().unwrap().revision, 2);
}

#[test]
fn failed_confirm_retains_recovery_snapshot() {
    let (dir, mut store, edit) = setup();
    let receipt = store.apply(edit).unwrap();
    store.failpoint = Some("before_rename");
    assert!(store.confirm(&receipt).is_err());
    drop(store);
    let reopened = Store::open(dir.path()).unwrap();
    assert!(reopened.recovery().unwrap()[0].document_before.is_some());
}

#[test]
fn missing_store_initializes_but_corrupt_existing_store_never_resets() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("document.json"), b"{partial").unwrap();
    assert_eq!(Store::open(dir.path()).err().unwrap().code, "invalid_store");
    assert_eq!(
        fs::read(dir.path().join("document.json")).unwrap(),
        b"{partial"
    );
}

#[test]
fn read_errors_fail_closed_instead_of_becoming_empty() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("document.json")).unwrap();
    assert_eq!(Store::open(dir.path()).err().unwrap().code, "storage_error");
}

#[test]
fn second_writer_is_excluded_until_first_drops() {
    let (dir, store, _) = setup();
    assert_eq!(Store::open(dir.path()).err().unwrap().code, "store_in_use");
    drop(store);
    assert!(Store::open(dir.path()).is_ok());
}

#[test]
fn startup_rejects_tampered_document_and_receipt() {
    for field in ["source", "receipt"] {
        let (dir, mut store, edit) = setup();
        store.apply(edit).unwrap();
        drop(store);
        let path = dir.path().join("document.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if field == "source" {
            value["document"]["text"] = "changed".into();
        } else {
            value["transactions"]["edit-1"]["receipt"]["new_revision"] = 99.into();
        }
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(Store::open(dir.path()).is_err());
    }
}

#[test]
fn abandoned_temporary_file_does_not_replace_committed_snapshot() {
    let (dir, store, _) = setup();
    fs::write(dir.path().join(".tmp-crashed-writer"), b"{incomplete").unwrap();
    drop(store);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(reopened.document().unwrap().unwrap().text, "aé😀z");
}

#[test]
fn stale_ordinary_edit_and_initialization_cannot_discard_history() {
    let (_dir, mut store, edit) = setup();
    store.apply(edit).unwrap();
    assert_eq!(
        store.initialize(doc("aé😀z")).unwrap_err().code,
        "document_exists"
    );
    assert_eq!(
        store
            .replace_document(1, &digest("aé😀z"), "lost".into())
            .unwrap_err()
            .code,
        "document_conflict"
    );
    assert_eq!(store.recovery().unwrap().len(), 1);
}

#[test]
fn revision_overflow_and_document_size_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let mut document = doc("aé😀z");
    document.revision = u64::MAX;
    let edit = edit(&document);
    store.initialize(document).unwrap();
    assert_eq!(store.apply(edit).unwrap_err().code, "revision_overflow");
    assert!(Document::new(
        "demo".into(),
        "main.tex".into(),
        1,
        "x".repeat(MAX_DOCUMENT_BYTES + 1)
    )
    .is_err());
}
