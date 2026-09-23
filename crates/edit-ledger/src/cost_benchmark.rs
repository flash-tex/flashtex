//! Explicit opt-in local filesystem benchmark, not a native paint benchmark.
use super::*;
use crate::{
    checkpoint::{archive::RotationPolicy, ExportAuthorization},
    history::{HistoryMove, HistoryRetentionPolicy},
};
use serde_json::json;
use std::{collections::BTreeMap, hint::black_box, time::Instant};

fn measure<T>(
    samples: &mut BTreeMap<&'static str, Vec<u128>>,
    name: &'static str,
    f: impl FnOnce() -> T,
) -> T {
    let start = Instant::now();
    let result = f();
    samples
        .entry(name)
        .or_default()
        .push(start.elapsed().as_nanos());
    result
}

#[test]
#[ignore = "explicit release-mode local filesystem benchmark"]
fn durable_cost_matrix() {
    for bytes in [5_000, 50_000, 500_000] {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        store
            .initialize(
                Document::new("cost".into(), "main.tex".into(), 1, "x".repeat(bytes)).unwrap(),
            )
            .unwrap();
        let mut samples = BTreeMap::new();
        let mut edits = Vec::new();
        for index in 0..20 {
            let doc = store.document().unwrap().unwrap().clone();
            let edit = PreparedEdit {
                capture_id: format!("capture-{index}"),
                edit_id: format!("edit-{index}"),
                project_id: doc.project_id.clone(),
                path: doc.path.clone(),
                expected_revision: doc.revision,
                start_byte: 0,
                end_byte: 1,
                removed_text: doc.text[..1].into(),
                replacement: if index % 2 == 0 { "y" } else { "x" }.into(),
                document_before_sha256: doc.source_sha256.clone(),
                wrap: None,
            };
            let receipt = measure(&mut samples, "edit", || store.apply(edit.clone()).unwrap());
            store.confirm(&receipt).unwrap();
            let current = store.document().unwrap().unwrap();
            let command = HistoryMove {
                command_id: format!("undo-{index}"),
                expected_revision: current.revision,
                expected_sha256: current.source_sha256.clone(),
            };
            measure(&mut samples, "undo", || store.undo(command).unwrap());
            let current = store.document().unwrap().unwrap();
            let command = HistoryMove {
                command_id: format!("redo-{index}"),
                expected_revision: current.revision,
                expected_sha256: current.source_sha256.clone(),
            };
            store.redo(command).unwrap();
            let token = store.export_recovery().unwrap().snapshot_token;
            store
                .retain_history(HistoryRetentionPolicy {
                    snapshot_token: token,
                    acknowledge_undo_redo_loss: true,
                    keep_latest_undo: 8,
                    keep_latest_redo: 0,
                })
                .unwrap();
            measure(&mut samples, "checkpoint_rotate", || {
                store
                    .rotate_checkpoint(
                        ExportAuthorization {
                            acknowledge_private_source_export: true,
                        },
                        RotationPolicy {
                            acknowledge_checkpoint_deletion: true,
                            keep_latest: 2,
                            max_total_bytes: 64 * 1024 * 1024,
                        },
                    )
                    .unwrap()
            });
            // Isolate actual production commit phases on the current valid state.
            let state = store.state.as_ref().unwrap();
            measure(&mut samples, "state_clone", || black_box(state.clone()));
            measure(&mut samples, "state_validate", || state.validate().unwrap());
            let encoded = measure(&mut samples, "state_serialize", || {
                serde_json::to_vec(state).unwrap()
            });
            measure(&mut samples, "atomic_write_sync", || {
                store.persist(&encoded).unwrap()
            });
            let cp = store
                .export_checkpoint(ExportAuthorization {
                    acknowledge_private_source_export: true,
                })
                .unwrap();
            measure(&mut samples, "checkpoint_encode", || {
                black_box(cp.encode().unwrap())
            });
            // Same-process comparison avoids treating scheduler/CPU changes
            // between separate runs as an optimization benefit. This is the
            // exact former encode path, including all existing validation.
            let legacy = measure(&mut samples, "checkpoint_encode_legacy", || {
                cp.validate().unwrap();
                serde_json::to_vec(&cp).unwrap()
            });
            assert_eq!(cp.encode().unwrap(), legacy);
            edits.push((edit, receipt));
        }
        let before = serde_json::to_value(store.state.as_ref().unwrap()).unwrap();
        let store_bytes = fs::metadata(dir.path().join("document.json"))
            .unwrap()
            .len();
        drop(store);
        let mut store = Store::open(dir.path()).unwrap();
        assert_eq!(
            serde_json::to_value(store.state.as_ref().unwrap()).unwrap(),
            before
        );
        for (edit, receipt) in edits {
            assert_eq!(store.apply(edit).unwrap(), receipt);
        }
        let metrics: BTreeMap<_,_>=samples.into_iter().map(|(name,mut ns)|{
            // Discard first four warmup samples; summarize 16 measured rounds.
            ns.drain(..4); ns.sort();
            (name,json!({"median_us":ns[ns.len()/2] as f64/1000.0,"max_us":ns.last().unwrap().to_owned() as f64/1000.0,"samples":ns.len()}))
        }).collect();
        println!(
            "BENCH {}",
            json!({"document_bytes":bytes,"source_sha256":store.document().unwrap().unwrap().source_sha256,"store_bytes":store_bytes,"restart_state_exact":true,"retained_history":8,"metrics":metrics})
        );
    }
}

#[test]
#[ignore = "explicit release-mode growing-history attribution"]
fn growing_source_history_cost() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let original = "x".repeat(521792);
    store
        .initialize(Document::new("cost".into(), "main.tex".into(), 1, original.clone()).unwrap())
        .unwrap();
    for index in 0..20 {
        let doc = store.document().unwrap().unwrap().clone();
        let mut source = original.clone();
        source.replace_range(0..3, &format!("{index:03}"));
        let started = Instant::now();
        store
            .replace_document(doc.revision, &doc.source_sha256, source)
            .unwrap();
        let edit_us = started.elapsed().as_micros();
        let state = store.state.as_ref().unwrap();
        let mut samples = BTreeMap::new();
        measure(&mut samples, "clone", || black_box(state.clone()));
        measure(&mut samples, "validate", || state.validate().unwrap());
        let bytes = measure(&mut samples, "serialize", || {
            serde_json::to_vec(state).unwrap()
        });
        measure(&mut samples, "persist", || store.persist(&bytes).unwrap());
        println!(
            "GROWING {}",
            json!({"edit":index+1,"edit_us":edit_us,"store_bytes":bytes.len(),"phases_ns":samples})
        );
    }
    let exact = serde_json::to_vec(store.state.as_ref().unwrap()).unwrap();
    drop(store);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(
        serde_json::to_vec(reopened.state.as_ref().unwrap()).unwrap(),
        exact
    );
    println!("GROWING_REOPEN_EXACT true");
}
