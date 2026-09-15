//! Issue #18: rooted, symlink-refusing, lock-serialized saves.
#![cfg(unix)]

mod common;

use std::fs;
use std::os::unix::fs::{FileTypeExt, MetadataExt, symlink};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use common::{TempDir, pp};
use flashtex_project_files::{
    ChangeKind, DiagnosticKind, DiscoverError, Expected, ProjectGraph, ProjectRoot, Refused,
    SaveConflictKind, SaveError, Snapshot, save_atomic, sha256,
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
    symlink(outside.root(), t.root().join("linked")).unwrap();

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
    symlink(outside.root(), t.root().join("real/link")).unwrap();
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
    symlink(&victim, t.root().join("alias.tex")).unwrap();
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
    symlink(real.root(), &link).unwrap();
    let err = ProjectRoot::open(&link).unwrap_err();
    assert!(is_refused_symlink(&err, "project"), "{err:?}");
    assert!(ProjectRoot::open(real.root()).is_ok());
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
    assert_eq!(receipt.identity.ino, meta.ino());
    assert_eq!(receipt.identity.dev, meta.dev());
    assert_eq!(receipt.mtime, meta.modified().unwrap());
    // A rooted read agrees with the receipt.
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
                    let _ = symlink(&victim, &target);
                } else {
                    let _ = fs::write(&target, "racer");
                }
                flips += 1;
            }
            flips
        })
    };
    let root = ProjectRoot::open(t.root()).unwrap();
    let mut outcomes = [0u32; 3];
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
    eprintln!("racing outcomes ok/refused/conflict = {outcomes:?}, racer flips = {flips}");
}

#[cfg(target_os = "macos")]
type ModeT = u16;
#[cfg(not(target_os = "macos"))]
type ModeT = u32;

unsafe extern "C" {
    fn mkfifo(path: *const std::os::raw::c_char, mode: ModeT) -> std::os::raw::c_int;
}

fn make_fifo(path: &std::path::Path) {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    // SAFETY: `c` is a valid NUL-terminated path that outlives the call.
    let rc = unsafe { mkfifo(c.as_ptr(), 0o644) };
    assert_eq!(
        rc,
        0,
        "mkfifo {}: {}",
        path.display(),
        std::io::Error::last_os_error()
    );
}

/// Upper bound for one call that must not block. A blocked FIFO open never
/// returns, so this only needs to exceed a saturated runner's scheduling
/// delay, not the call's normal duration.
const HANG_LIMIT: Duration = Duration::from_secs(20);

/// Runs `f` on a worker thread and fails the test if it has not returned
/// within `limit`. A hung worker is leaked; the test binary still exits.
fn within<T: Send + 'static>(
    limit: Duration,
    what: &str,
    f: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(limit).unwrap_or_else(|_| {
        panic!("{what} did not return within {limit:?} (blocked on a FIFO open?)")
    })
}

/// A FIFO where a regular file or directory is expected is refused at once;
/// neither the rooted read, the directory walk, nor discovery of a
/// FIFO entry blocks waiting for a writer.
#[test]
fn fifo_is_refused_without_blocking() {
    let t = TempDir::new("fifo");
    make_fifo(&t.root().join("pipe.tex"));
    let root_path = t.root().to_path_buf();
    let limit = HANG_LIMIT;

    let r = root_path.clone();
    let err = within(limit, "ProjectRoot::read", move || {
        ProjectRoot::open(&r)
            .unwrap()
            .read(&pp("pipe.tex"), 1024)
            .map(|_| ())
    })
    .unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::NotARegularFile { component }) if component == "pipe.tex"),
        "{err:?}"
    );

    // A FIFO as a directory component: the O_DIRECTORY open refuses it and
    // the symlink-classification probe that follows must not block either.
    let r = root_path.clone();
    let err = within(
        limit,
        "ProjectRoot::read through a FIFO component",
        move || {
            ProjectRoot::open(&r)
                .unwrap()
                .read(&pp("pipe.tex/x.tex"), 1024)
                .map(|_| ())
        },
    )
    .unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::NotADirectory { component }) if component == "pipe.tex"),
        "{err:?}"
    );

    let r = root_path.clone();
    let err = within(limit, "ProjectGraph::discover", move || {
        ProjectGraph::discover(&r, &pp("pipe.tex")).map(|_| ())
    })
    .unwrap_err();
    assert!(
        matches!(&err, DiscoverError::EntryUnreadable(p, e) if p.as_str() == "pipe.tex" && e.to_string().contains("not a regular file")),
        "{err:?}"
    );
}

/// `\input{pipe}` whose target is swapped between a regular file and a FIFO
/// while discovery runs. The property under test is only that no discovery
/// hangs, whichever interleaving occurs: a fixed number of runs, each bounded
/// by a generous per-call limit (a blocked open never returns, so the limit
/// only has to exceed a slow runner's scheduling delay). Which diagnostic a
/// run produces depends on the interleaving and is not asserted.
#[test]
fn input_swapped_to_fifo_after_probe_does_not_hang_discovery() {
    let t = TempDir::new("fifo-swap");
    t.write("main.tex", "\\input{pipe}\n");
    t.write("keep-regular", "Regular.");
    make_fifo(&t.root().join("keep-fifo"));
    fs::copy(t.root().join("keep-regular"), t.root().join("pipe.tex")).unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let racer = {
        let (stop, root) = (stop.clone(), t.root().to_path_buf());
        thread::spawn(move || {
            let mut flips = 0u64;
            while !stop.load(Ordering::Relaxed) {
                for keep in ["keep-fifo", "keep-regular"] {
                    let tmp = root.join("swap.tmp");
                    let _ = fs::remove_file(&tmp);
                    fs::hard_link(root.join(keep), &tmp).unwrap();
                    fs::rename(&tmp, root.join("pipe.tex")).unwrap();
                }
                flips += 1;
            }
            flips
        })
    };

    let (mut runs, mut refused) = (0u32, 0u32);
    while runs < 200 {
        let root = t.root().to_path_buf();
        let graph = match within(HANG_LIMIT, "ProjectGraph::discover", move || {
            ProjectGraph::discover(&root, &pp("main.tex"))
        }) {
            Ok(g) => g,
            Err(e) => {
                stop.store(true, Ordering::Relaxed);
                panic!("discover failed: {e:?}");
            }
        };
        runs += 1;
        refused += graph
            .diagnostics()
            .iter()
            .filter(|d| {
                matches!(&d.kind, DiagnosticKind::ReadError { path, message }
                    if path.as_str() == "pipe.tex" && message.contains("not a regular file"))
            })
            .count() as u32;
    }
    stop.store(true, Ordering::Relaxed);
    let flips = racer.join().unwrap();
    eprintln!("fifo swap: {runs} discoveries, {refused} refused, racer flips = {flips}");
    assert!(flips > 0);
}

/// Review finding 1: the watcher stats and hashes tracked files through the
/// rooted reader. A tracked file that is (or becomes) a symlink, or lies
/// under a symlinked directory, is reported as absent and its target's
/// contents are never hashed.
#[test]
fn watcher_never_hashes_through_a_symlinked_tracked_file() {
    let outside = TempDir::new("watch-outside");
    let victim = outside.write("victim.tex", "external secret");
    let t = TempDir::new("watch-symlink");
    t.write("a.tex", "inside");
    symlink(&victim, t.root().join("alias.tex")).unwrap();
    symlink(outside.root(), t.root().join("linked")).unwrap();
    let paths = [
        pp("a.tex"),
        pp("alias.tex"),
        pp("linked/victim.tex"),
        pp("not-yet/created.tex"),
    ];

    let snap = Snapshot::take(t.root(), &paths).unwrap();
    assert!(snap.state(&pp("not-yet/created.tex")).is_none());
    assert_eq!(snap.state(&pp("a.tex")).unwrap().sha256, sha256(b"inside"));
    assert!(snap.state(&pp("alias.tex")).is_none(), "symlinked file");
    assert!(
        snap.state(&pp("linked/victim.tex")).is_none(),
        "file under a symlinked directory"
    );

    // A tracked regular file swapped for a symlink is reported deleted.
    fs::remove_file(t.root().join("a.tex")).unwrap();
    symlink(&victim, t.root().join("a.tex")).unwrap();
    let diff = snap.diff().unwrap();
    let kinds: Vec<_> = diff
        .changes
        .iter()
        .map(|c| (c.path.as_str(), c.kind, c.after))
        .collect();
    assert_eq!(kinds, [("a.tex", ChangeKind::Deleted, None)]);

    // Changing the external file is invisible through every tracked path,
    // while a real file created under a new directory is still reported.
    fs::write(&victim, "external secret, edited").unwrap();
    assert!(diff.snapshot.diff().unwrap().is_empty());
    t.write("not-yet/created.tex", "new");
    let created: Vec<_> = diff
        .snapshot
        .diff()
        .unwrap()
        .changes
        .into_iter()
        .map(|c| (c.path.as_str().to_string(), c.kind))
        .collect();
    assert_eq!(
        created,
        [("not-yet/created.tex".to_string(), ChangeKind::Created)]
    );
    let mut tracked = diff.snapshot.clone();
    tracked.track(&pp("alias.tex")).unwrap();
    assert!(tracked.state(&pp("alias.tex")).is_none());
}

/// Review findings 1 and 6: a tracked path that is a FIFO is classified
/// before it is opened, so the watcher neither blocks nor reads it.
#[test]
fn watcher_does_not_open_a_tracked_fifo() {
    let t = TempDir::new("watch-fifo");
    make_fifo(&t.root().join("pipe.tex"));
    let root = t.root().to_path_buf();
    let snap = within(HANG_LIMIT, "Snapshot::take", move || {
        Snapshot::take(&root, &[pp("pipe.tex")]).unwrap()
    });
    assert!(snap.state(&pp("pipe.tex")).is_none());
    let diff = within(HANG_LIMIT, "Snapshot::diff", move || snap.diff().unwrap());
    assert!(diff.is_empty());
}

/// Review finding 2: a project root whose final component is a symlink with
/// a non-UTF-8 name is refused like any other symlinked root. Filesystems
/// that only store UTF-8 names (APFS, HFS+) cannot hold such a name; there
/// the test has nothing to exercise and says so.
#[test]
fn non_utf8_named_root_symlink_is_refused() {
    use std::os::unix::ffi::OsStrExt;
    let real = TempDir::new("nonutf8-real");
    real.write("main.tex", "Real.");
    let holder = TempDir::new("nonutf8-holder");
    let link = holder
        .root()
        .join(std::ffi::OsStr::from_bytes(b"project-\xff"));
    if let Err(e) = symlink(real.root(), &link) {
        eprintln!("skipping: this filesystem rejects non-UTF-8 names ({e})");
        return;
    }
    let err = ProjectRoot::open(&link).unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::SymlinkComponent { .. })),
        "{err:?}"
    );
    assert!(Snapshot::take(&link, &[pp("main.tex")]).is_err());
    assert!(ProjectGraph::discover(&link, &pp("main.tex")).is_err());
}

/// Review finding 6: an existing entry is classified with `fstatat` on the
/// directory descriptor before any open. A socket cannot be opened at all
/// (`openat` fails with `ENXIO`/`EOPNOTSUPP`), so reaching the
/// not-a-regular-file refusal proves no open was attempted; the same holds
/// for a socket or FIFO where the project lock file belongs, and for a socket
/// used as a directory component.
/// Binds a unix socket and moves it to `dest`. Socket paths are limited to
/// about 100 bytes, which a `TempDir` path can exceed, so the socket is
/// bound under a short name in the same temp directory and renamed.
fn make_socket(dest: &std::path::Path) -> std::os::unix::net::UnixListener {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let short = std::env::temp_dir().join(format!(
        "ftx{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let listener = std::os::unix::net::UnixListener::bind(&short).unwrap();
    fs::rename(&short, dest).unwrap();
    listener
}

#[test]
fn special_files_are_refused_before_they_are_opened() {
    let t = TempDir::new("sock");
    let _sock = make_socket(&t.root().join("s.tex"));
    let root = ProjectRoot::open(t.root()).unwrap();
    let err = root.read(&pp("s.tex"), 1024).unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::NotARegularFile { component }) if component == "s.tex"),
        "{err:?}"
    );
    let err = root.read(&pp("s.tex/x.tex"), 1024).unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::NotADirectory { component }) if component == "s.tex"),
        "{err:?}"
    );

    fs::create_dir(t.root().join(".flashtex")).unwrap();
    let lock = t.root().join(".flashtex/project.lock");
    let listener = make_socket(&lock);
    let err = root.lock().unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::NotARegularFile { component }) if component == "project.lock"),
        "{err:?}"
    );
    drop(listener);
    fs::remove_file(&lock).unwrap();
    make_fifo(&lock);
    let r = t.root().to_path_buf();
    let err = within(HANG_LIMIT, "ProjectRoot::lock on a FIFO", move || {
        ProjectRoot::open(&r).unwrap().lock().map(|_| ())
    })
    .unwrap_err();
    assert!(
        matches!(&err, SaveError::Refused(Refused::NotARegularFile { component }) if component == "project.lock"),
        "{err:?}"
    );
    assert!(fs::symlink_metadata(&lock).unwrap().file_type().is_fifo());
}
