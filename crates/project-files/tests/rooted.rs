//! Issue #18: rooted, symlink-refusing, lock-serialized saves.
//!
//! These run on every platform with rooted file operations. The behaviour
//! under test — a symlinked path component is refused, never followed — is
//! the same everywhere; only the call that *creates* the link differs, so it
//! lives behind `common::symlink_dir`/`symlink_file` and the test bodies are
//! shared. Windows additionally gets a junction case, which no POSIX
//! equivalent covers and which needs no special privilege to create.

mod common;

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use common::{TempDir, pp, symlink_dir, symlink_file};
use flashtex_project_files::{
    Expected, ProjectRoot, Refused, SaveConflictKind, SaveError, save_atomic, sha256,
};

fn is_refused_symlink(err: &SaveError, component: &str) -> bool {
    matches!(err, SaveError::Refused(Refused::SymlinkComponent { component: c }) if c == component)
}

/// The exact issue #18 reproduction: `linked/` is a symlink to a directory
/// outside the root and `save_atomic(root, "linked/victim.tex", ...,
/// Expected::Hash(current), false)` used to overwrite the external file.
#[test]
fn issue_18_symlinked_parent_directory_is_refused() {
    let outside = TempDir::new("i18-outside");
    let victim = outside.write("victim.tex", "external content");
    let t = TempDir::new("i18-root");
    symlink_dir(outside.root(), &t.root().join("linked"));

    let current = sha256(b"external content");
    let err = save_atomic(
        t.root(),
        &pp("linked/victim.tex"),
        "overwritten",
        Expected::Hash(current),
        false,
    )
    .unwrap_err();
    assert!(is_refused_symlink(&err, "linked"), "{err:?}");
    let err = save_atomic(
        t.root(),
        &pp("linked/victim.tex"),
        "overwritten",
        Expected::Any,
        true,
    )
    .unwrap_err();
    assert!(
        is_refused_symlink(&err, "linked"),
        "force does not follow symlinks either: {err:?}"
    );
    assert_eq!(
        fs::read_to_string(&victim).unwrap(),
        "external content",
        "outside_project_file_was_overwritten must be false"
    );

    // Reads refuse the same path, so a stale-but-honest hash cannot be obtained through it.
    let root = ProjectRoot::open(t.root()).unwrap();
    let err = root.read(&pp("linked/victim.tex"), 1 << 20).unwrap_err();
    assert!(is_refused_symlink(&err, "linked"));
    // Nested: a real directory containing a symlinked directory.
    fs::create_dir(t.root().join("real")).unwrap();
    symlink_dir(outside.root(), &t.root().join("real/link"));
    let err = root
        .save(&pp("real/link/victim.tex"), b"x", Expected::Any, true)
        .unwrap_err();
    assert!(is_refused_symlink(&err, "link"), "{err:?}");
    assert_eq!(fs::read_to_string(&victim).unwrap(), "external content");
}

#[test]
fn symlinked_file_is_refused_for_read_save_and_remove() {
    let outside = TempDir::new("file-outside");
    let victim = outside.write("victim.tex", "external");
    let t = TempDir::new("file-root");
    symlink_file(&victim, &t.root().join("alias.tex"));
    let root = ProjectRoot::open(t.root()).unwrap();

    assert!(is_refused_symlink(
        &root.read(&pp("alias.tex"), 1 << 20).unwrap_err(),
        "alias.tex"
    ));
    let err = root
        .save(
            &pp("alias.tex"),
            b"mine",
            Expected::Hash(sha256(b"external")),
            false,
        )
        .unwrap_err();
    assert!(is_refused_symlink(&err, "alias.tex"), "{err:?}");
    let err = root
        .save(&pp("alias.tex"), b"mine", Expected::Any, true)
        .unwrap_err();
    assert!(is_refused_symlink(&err, "alias.tex"), "{err:?}");
    let err = root.remove(&pp("alias.tex")).unwrap_err();
    assert!(is_refused_symlink(&err, "alias.tex"), "{err:?}");
    assert_eq!(fs::read_to_string(&victim).unwrap(), "external");
    assert!(
        fs::symlink_metadata(t.root().join("alias.tex"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn symlinked_root_is_refused() {
    let real = TempDir::new("root-real");
    let holder = TempDir::new("root-holder");
    let link = holder.root().join("project");
    symlink_dir(real.root(), &link);
    let err = ProjectRoot::open(&link).unwrap_err();
    assert!(is_refused_symlink(&err, "project"), "{err:?}");
    assert!(ProjectRoot::open(real.root()).is_ok());
}

/// The Windows-only escape route POSIX has no analogue for: a *junction*
/// (`IO_REPARSE_TAG_MOUNT_POINT`) rather than a symlink
/// (`IO_REPARSE_TAG_SYMLINK`). Junctions redirect exactly as symlinks do but
/// need no Developer Mode or elevation to create, which makes them the
/// likelier issue-#18 vector on Windows. They must be refused on the same
/// name-surrogate grounds, both as the project root and as an interior
/// component.
#[cfg(windows)]
#[test]
fn junctions_are_refused_as_root_and_as_a_parent_component() {
    fn mklink_junction(link: &std::path::Path, target: &std::path::Path) -> bool {
        std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()
            .is_ok_and(|o| o.status.success())
    }

    let outside = TempDir::new("junction-outside");
    let victim = outside.write("victim.tex", "external content");
    let t = TempDir::new("junction-root");
    let link = t.root().join("linked");
    if !mklink_junction(&link, outside.root()) {
        panic!("`mklink /J` failed; a junction is required for this test");
    }

    let root = ProjectRoot::open(t.root()).unwrap();
    let err = root
        .save(&pp("linked/victim.tex"), b"overwritten", Expected::Any, true)
        .unwrap_err();
    assert!(is_refused_symlink(&err, "linked"), "{err:?}");
    let err = root.read(&pp("linked/victim.tex"), 1 << 20).unwrap_err();
    assert!(is_refused_symlink(&err, "linked"), "{err:?}");
    assert_eq!(fs::read_to_string(&victim).unwrap(), "external content");

    // The same refusal when the junction *is* the root handed to `open`.
    let holder = TempDir::new("junction-holder");
    let root_link = holder.root().join("project");
    assert!(mklink_junction(&root_link, outside.root()));
    let err = ProjectRoot::open(&root_link).unwrap_err();
    assert!(is_refused_symlink(&err, "project"), "{err:?}");
}

#[test]
fn not_a_directory_component_is_refused() {
    let t = TempDir::new("notdir");
    t.write("file.tex", "x");
    let root = ProjectRoot::open(t.root()).unwrap();
    let err = root
        .save(&pp("file.tex/child.tex"), b"y", Expected::Any, true)
        .unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::NotADirectory { component }) if component == "file.tex"),
        "{err:?}"
    );
}

#[test]
fn lock_is_exclusive_across_handles_and_released_on_drop() {
    let t = TempDir::new("lock");
    let a = ProjectRoot::open(t.root()).unwrap();
    let b = ProjectRoot::open(t.root()).unwrap();
    let held = a.lock().unwrap();
    assert!(t.root().join(".flashtex/project.lock").is_file());
    let err = b.lock().unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::LockUnavailable { lock_path }) if lock_path.ends_with(".flashtex/project.lock")),
        "{err:?}"
    );
    let err = b
        .save(&pp("x.tex"), b"x", Expected::NewFile, false)
        .unwrap_err();
    assert!(matches!(
        err,
        SaveError::Refused(Refused::LockUnavailable { .. })
    ));
    assert!(
        !t.root().join("x.tex").exists(),
        "nothing written without the lock"
    );
    // The convenience wrapper takes the lock too.
    assert!(matches!(
        save_atomic(t.root(), &pp("x.tex"), "x", Expected::NewFile, false),
        Err(SaveError::Refused(Refused::LockUnavailable { .. }))
    ));
    // Same handle: a second lock attempt on the same root is also refused.
    assert!(matches!(
        a.lock(),
        Err(SaveError::Refused(Refused::LockUnavailable { .. }))
    ));
    drop(held);
    b.save(&pp("x.tex"), b"x", Expected::NewFile, false)
        .unwrap();
    assert_eq!(t.read("x.tex"), "x");
}

#[test]
fn durability_bytes_hash_and_identity_match_after_save() {
    let t = TempDir::new("durable");
    let root = ProjectRoot::open(t.root()).unwrap();
    let payload = "durable — 😀\n".repeat(1000);
    let receipt = root
        .save(
            &pp("deep/er/file.tex"),
            payload.as_bytes(),
            Expected::NewFile,
            false,
        )
        .unwrap();
    let on_disk = fs::read(t.root().join("deep/er/file.tex")).unwrap();
    assert_eq!(on_disk, payload.as_bytes());
    assert_eq!(receipt.sha256, sha256(&on_disk));
    assert_eq!(receipt.bytes, on_disk.len() as u64);
    let meta = fs::metadata(t.root().join("deep/er/file.tex")).unwrap();
    assert_eq!(receipt.mtime, meta.modified().unwrap());
    // A freshly observed identity for the same file equals the receipt's.
    // Compared as whole `FileIdentity` values rather than through raw
    // `st_dev`/`st_ino`, so this asserts the property that matters -- "the
    // receipt names the file that is actually there" -- on every platform,
    // whatever the identity is made of underneath.
    let read = root
        .read(&pp("deep/er/file.tex"), 1 << 20)
        .unwrap()
        .unwrap();
    assert_eq!(read.sha256, receipt.sha256);
    assert_eq!(read.identity, receipt.identity);
    // Bounded read refuses oversize files.
    let err = root.read(&pp("deep/er/file.tex"), 16).unwrap_err();
    assert!(matches!(
        err,
        SaveError::Refused(Refused::TooLarge { limit: 16, .. })
    ));
    let no_temp: Vec<_> = fs::read_dir(t.root().join("deep/er"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(no_temp, ["file.tex"]);
    // A *different* file is never mistaken for it, so the identity the
    // receipt carries actually discriminates.
    let other = root
        .save(&pp("deep/er/other.tex"), b"other", Expected::NewFile, false)
        .unwrap();
    assert_ne!(other.identity, receipt.identity);
}

#[test]
fn expected_hash_conflicts_are_still_reported() {
    let t = TempDir::new("conflict");
    let root = ProjectRoot::open(t.root()).unwrap();
    let r1 = root
        .save(&pp("a.tex"), b"v1", Expected::NewFile, false)
        .unwrap();
    fs::write(t.root().join("a.tex"), "external").unwrap();
    match root.save(&pp("a.tex"), b"v2", Expected::Hash(r1.sha256), false) {
        Err(SaveError::Conflict(c)) => {
            assert_eq!(c.kind, SaveConflictKind::ModifiedExternally);
            assert_eq!(c.ours, Some(r1.sha256));
            assert_eq!(c.theirs, Some(sha256(b"external")));
            assert_eq!(c.size, Some(8));
        }
        other => panic!("{other:?}"),
    }
    assert!(
        matches!(root.save(&pp("a.tex"), b"v2", Expected::NewFile, false), Err(SaveError::Conflict(c)) if c.kind == SaveConflictKind::AlreadyExists)
    );
    fs::remove_file(t.root().join("a.tex")).unwrap();
    assert!(
        matches!(root.save(&pp("a.tex"), b"v2", Expected::Hash(r1.sha256), false), Err(SaveError::Conflict(c)) if c.kind == SaveConflictKind::DeletedExternally)
    );
}

/// A thread keeps swapping the target between a regular file and a symlink
/// to an outside file while saves run. Whatever each save returns (Ok,
/// Refused, or Conflict), the outside file must never change and the
/// project file, when it is a regular file, must hold either a value we
/// wrote or the racer's value — never a torn write.
#[test]
fn racing_symlink_swap_never_reaches_the_outside_file() {
    let outside = TempDir::new("race-outside");
    let victim = outside.write("victim.tex", "untouchable");
    let t = TempDir::new("race-root");
    let target = t.root().join("a.tex");
    fs::write(&target, "racer").unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let racer = {
        let stop = stop.clone();
        let target = target.clone();
        let victim = victim.clone();
        thread::spawn(move || {
            let mut flips = 0u32;
            while !stop.load(Ordering::Relaxed) {
                let _ = fs::remove_file(&target);
                if flips.is_multiple_of(2) {
                    common::try_symlink_file(&victim, &target);
                } else {
                    let _ = fs::write(&target, "racer");
                }
                flips += 1;
            }
            flips
        })
    };
    let root = ProjectRoot::open(t.root()).unwrap();
    let mut outcomes = [0u32; 4];
    for i in 0..400 {
        let payload = format!("mine-{i}");
        match root.save(&pp("a.tex"), payload.as_bytes(), Expected::Any, true) {
            Ok(r) => {
                assert_eq!(r.sha256, sha256(payload.as_bytes()));
                outcomes[0] += 1;
            }
            Err(SaveError::Refused(Refused::SymlinkComponent { .. })) => outcomes[1] += 1,
            Err(SaveError::Conflict(c)) => {
                assert_eq!(c.kind, SaveConflictKind::ModifiedDuringSave);
                outcomes[2] += 1;
            }
            // Windows file sharing is mandatory rather than advisory, and a
            // deleted name lingers in a "delete pending" state instead of
            // vanishing atomically. So a save racing a writer that keeps
            // recreating the target can be refused outright where POSIX
            // would simply have succeeded:
            //
            //   32 ERROR_SHARING_VIOLATION - the racer holds the name while
            //      `CreateSymbolicLinkW` or its write is in flight.
            //    5 ERROR_ACCESS_DENIED     - likewise, for a delete in flight.
            //    2 ERROR_FILE_NOT_FOUND    - the target is delete-pending, so
            //      renaming over it is refused; `sys` maps that onto `ENOENT`
            //      because the name is, to any caller, already gone.
            //
            // Each is a transient refusal to act, never a partial write. The
            // invariant this test exists for -- the outside file is never
            // reached -- is asserted on every iteration regardless.
            #[cfg(windows)]
            Err(SaveError::Io(e)) if matches!(e.raw_os_error(), Some(2 | 5 | 32)) => {
                outcomes[3] += 1
            }
            Err(other) => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            fs::read_to_string(&victim).unwrap(),
            "untouchable",
            "iteration {i}"
        );
    }
    stop.store(true, Ordering::Relaxed);
    let flips = racer.join().unwrap();
    assert!(flips > 0);
    assert_eq!(fs::read_to_string(&victim).unwrap(), "untouchable");
    // The run must have actually exercised the save path rather than bailing
    // out every time, or it would prove nothing.
    assert!(
        outcomes[0] + outcomes[1] + outcomes[2] > 0,
        "every save was a transient sharing failure: {outcomes:?}"
    );
    eprintln!(
        "racing outcomes ok/refused/conflict/sharing = {outcomes:?}, racer flips = {flips}"
    );
}
