//! Offline integration example with the real bridge journal and document model.
//! The single-threaded adapter owns document transactions; no live Grok call.
use base64::{engine::general_purpose::STANDARD, Engine};
use flashtex_bridge::{
    store::Store, Bridge, CaptureImage, CaptureSubmit, Context, Document, Proposal,
};
use flashtex_conversion_jobs::{
    snapshot::Limits, CancellationToken, ContextFingerprint, Failure, Scheduler, State,
};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs::{self, File, OpenOptions},
    io::{Cursor, Write},
    path::Path,
    thread,
    time::{Duration, Instant},
};
type AdapterResult<T> = Result<T, Box<dyn Error>>;

/// Opens `path` (a directory) so `sync_all` can fsync it after a rename. See
/// `crates/project-files/src/sys.rs`'s `open_dir_std`/`open_at` — plain
/// `File::open` can't open a directory on Windows at all, and even once
/// opened (via `FILE_FLAG_BACKUP_SEMANTICS`), `sync_all` needs write access
/// on the handle too (measured).
#[cfg(windows)]
fn open_dir_for_sync(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(not(windows))]
fn open_dir_for_sync(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

/// Atomically replaces `destination` with `temporary`, via `std::fs::rename`
/// rather than `NamedTempFile::persist` — not equivalent on Windows, where
/// `persist`'s `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` needs `DELETE` access
/// on the existing file and fails if any other handle to it is open, unlike
/// `fs::rename`'s POSIX-semantics replace. See `crates/edit-ledger/src/lib.rs`'s
/// `replace_with_temporary` for the measured failure-rate comparison.
fn replace_with_temporary(
    temporary: tempfile::NamedTempFile,
    destination: &Path,
) -> std::io::Result<()> {
    let (file, path) = temporary.keep().map_err(|e| e.error)?;
    drop(file);
    match fs::rename(&path, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&path);
            Err(error)
        }
    }
}
fn fingerprint(documents: &[Document]) -> ContextFingerprint {
    ContextFingerprint {
        revision: documents[0].revision,
        sha256: format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(documents).unwrap())
        ),
    }
}
fn atomic_write(path: &Path, bytes: &[u8]) -> AdapterResult<()> {
    let parent = path.parent().unwrap();
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    replace_with_temporary(file, path)?;
    open_dir_for_sync(parent)?.sync_all()?;
    Ok(())
}
fn capture() -> AdapterResult<CaptureSubmit> {
    let mut image = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(1, 1).write_to(&mut image, image::ImageFormat::Png)?;
    Ok(CaptureSubmit {
        capture_id: "example-capture".into(),
        destination_id: "example-anchor".into(),
        base_revision: 1,
        image: CaptureImage {
            mime_type: "image/png".into(),
            data_base64: STANDARD.encode(image.into_inner()),
        },
        instructions: "Keep existing notation".into(),
    })
}
fn pipeline(
    directory: &Path,
    convert: impl FnOnce(CaptureSubmit, Context, CancellationToken) -> Result<Proposal, Failure>
        + Send
        + 'static,
) -> AdapterResult<Proposal> {
    let mut bridge = Bridge::new(Store::open(directory.join("captures"))?);
    let documents = vec![Document {
        project_id: "example".into(),
        path: "main.tex".into(),
        revision: 1,
        text: "Before after".into(),
    }];
    bridge.open_document(documents[0].clone())?;
    bridge.pin("example-anchor", "example", "main.tex", 1, 7, 7)?;
    let capture = capture()?;
    let record = bridge.receive(capture.clone())?;
    // The durable proposal, not a detached scheduler result, is authoritative.
    if let Some(proposal) = record.proposal {
        return Ok(proposal);
    }
    let intent = directory.join("conversion-intent.json");
    if intent.exists() {
        return Err("Recovery required: prior provider completion may be ambiguous; reconcile before explicit retry".into());
    }
    let context = bridge.context(&capture, vec![])?;
    let expected = fingerprint(&documents);
    // Persist the intent BEFORE submit may start the worker. This covers a crash
    // before the first scheduler checkpoint and is conservative about billing.
    atomic_write(
        &intent,
        &serde_json::to_vec(
            &serde_json::json!({"capture_id":capture.capture_id,"fingerprint":expected,"state":"provider_may_have_started"}),
        )?,
    )?;
    let scheduler = Scheduler::new(1, 2, 2).map_err(|e| format!("{e:?}"))?;
    let id = capture.capture_id.clone();
    let frozen = context.clone();
    scheduler
        .submit(&id, expected.clone(), move |token| {
            convert(capture, frozen, token)
        })
        .map_err(|e| format!("{e:?}"))?;
    let checkpoint = directory.join("jobs.json");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match scheduler.state(&id).map_err(|e| format!("{e:?}"))? {
            State::Completed(_) | State::Failed(_) | State::Cancelled => break,
            _ => {
                if Instant::now() > deadline {
                    scheduler.cancel(&id).map_err(|e| format!("{e:?}"))?;
                    return Err(
                        "Example conversion timed out; durable intent requires reconciliation"
                            .into(),
                    );
                }
                thread::yield_now();
            }
        }
    }
    scheduler
        .save_snapshot(&checkpoint, Limits::default(), |proposal| {
            serde_json::to_vec(proposal).map_err(|e| e.to_string())
        })
        .map_err(|e| format!("{e:?}"))?;
    // In a native adapter, these checks and journal promotion run in the document
    // owner's transaction. Include every relevant current document in fingerprint.
    let current = fingerprint(&documents);
    scheduler
        .update_context(&id, current.clone())
        .map_err(|e| format!("{e:?}"))?;
    if current != expected || bridge.context(&record.capture, vec![])? != context {
        return Err("Current source changed; discard result and require new review".into());
    }
    let State::Completed(proposal) = scheduler.state(&id).map_err(|e| format!("{e:?}"))? else {
        return Err("Conversion did not complete; inspect checkpoint and reconcile intent".into());
    };
    proposal.validate()?;
    let mut record = bridge.store.require(&id)?;
    if record.prepared.is_some() || record.applied.is_some() || record.rejected {
        return Err("Capture is no longer eligible for conversion promotion".into());
    }
    record.context = Some(context);
    record.proposal = Some((*proposal).clone());
    bridge.store.save(&record)?;
    fs::remove_file(intent)?;
    open_dir_for_sync(directory)?.sync_all()?;
    scheduler.forget(&id).map_err(|e| format!("{e:?}"))?;
    // No source insertion: the Mac must display proposal/diagnostics and obtain review.
    Ok((*proposal).clone())
}
fn main() -> AdapterResult<()> {
    let directory = tempfile::tempdir()?;
    let proposal = pipeline(directory.path(), |_, _, token| {
        token.progress(50, "offline fixture conversion").unwrap();
        Ok(Proposal {
            latex: "$x^2$".into(),
            ambiguities: vec![],
            required_dependencies: vec![],
        })
    })?;
    println!("Offline fixture journaled for review: {}", proposal.latex);
    Ok(())
}
#[test]
fn real_bridge_journal_prevents_repeated_conversion_after_restart() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    let directory = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let first = pipeline(directory.path(), move |_, _, _| {
        count.fetch_add(1, Ordering::SeqCst);
        Ok(Proposal {
            latex: "$x$".into(),
            ambiguities: vec![],
            required_dependencies: vec![],
        })
    })
    .unwrap();
    let repeated = pipeline(directory.path(), |_, _, _| {
        panic!("durable result must avoid another call")
    })
    .unwrap();
    assert_eq!(first, repeated);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
#[test]
fn ambiguous_persisted_intent_blocks_automatic_retry() {
    let directory = tempfile::tempdir().unwrap();
    atomic_write(&directory.path().join("conversion-intent.json"), b"pending").unwrap();
    assert!(pipeline(directory.path(), |_, _, _| panic!(
        "ambiguous provider call must not be retried"
    ))
    .unwrap_err()
    .to_string()
    .contains("Recovery required"));
}
