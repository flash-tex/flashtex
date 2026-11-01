//! FS-WATCH (lane T7-LAST12; DESIGN.md §5.1, §1.2): the signatures the
//! host's per-compile checks take of the files a run read, TeX Live's and
//! the document's own, vouched for by the kernel's file events instead of a
//! `stat` each.
//!
//! At every keystroke the host checks that nothing S₀ and the later
//! checkpoints read has changed: S₀'s key (`host::Key::check_files`), the
//! journal after it (`incr::Session::changes`), the lookups' directories and
//! the resolver's on-disk directories (`refresh_disk_dirs`). A beamer
//! preamble reads 189 files of TeX Live's tree, and with the directories and
//! the lookups' answers that is about 600 `stat` calls a keystroke (PC
//! strace, beamer-30). A book's own files are as many again: on *Infinite
//! Descent* ×2 the key and the journal list 262 of the document's files and
//! directories, read relative to it (`./book/...`), and the lookups' proofs
//! ask whether 885 of them are readable (an `open` each): about 60 M of the
//! 100 M instructions before the restore (lane FIND-PHASE).
//!
//! **What is vouched for.** Inside a check ([`check`], which first drains
//! the kernel's queue of events, [`sync`]), [`sig`] gives a path's
//! signature as a `stat` at an earlier check gave it, when no event has
//! arrived since that can change it (on the path as written, and as its
//! canonical path: the directories a symbolic link on the way points into):
//! * any event on the object itself, and on the directory holding it (an
//!   entry renamed over it);
//! * past a missing component, any event on the deepest existing directory
//!   (the component's creation) and on the one holding it;
//! * on every directory further up, an event about the directory itself
//!   only: renamed, moved, removed, revoked, its attributes changed
//!   (`Dep::whole` false). Its other entries being made and removed -- a
//!   home directory or `/tmp` above the document, where other programs
//!   write all the time -- cannot change what the path names: only the one
//!   entry on the way can, and that entry is a directory watched itself,
//!   whose rename or removal is an event on it (`NOTE_RENAME`,
//!   `NOTE_DELETE`; `IN_MOVE_SELF`, `IN_DELETE_SELF`; a directory can be
//!   renamed over only an empty one, which then has no entry on the way);
//! * except where the entry on the way is a symbolic link: replacing a link
//!   raises an event only on the directory holding it, which then counts
//!   every event.
//!
//! A relative path is taken in the working directory of the check (the
//! hosts change it only between compiles, `os::cwd_scope`). Anything else
//! is a `stat`:
//! * a path never seen (the watches are made *before* the `stat` that is
//!   kept, so a change between them shows as an event: the startup case);
//! * the queue overflowed or could not be read (every path is then
//!   unvouched until its next `stat`);
//! * a watch could not be made (descriptor or watch limits), the file is on
//!   a volume that is not local (NFS, SMB, FUSE, overlayfs: changes made
//!   elsewhere raise no event), or the path has a `.` or `..` component
//!   inside it, or is under the output directory ([`exclude`]), which the
//!   run writes at every compile;
//! * the kept signature is racy (`StatSig::racy`): it equals nothing, and
//!   the caller compares content; a later `stat` may give one that is not
//!   (taken under the watches already made while they all hold: the file
//!   the user types in is racy at every keystroke).
//!
//! Outside a check [`sig`] is `StatSig::of`: the run's own recording (a
//! lookup's directories as it makes them) is never served from here, so a
//! file the run itself makes meanwhile is seen as it is.
//!
//! **Why it is as good as the `stat`.** A change completed before a check
//! begins has queued its event (Linux's `inotify` and the BSD `kqueue` post
//! them in the system call that makes the change), so the check's drain
//! sees it and the path is `stat`ed again; a change after the drain is a
//! change after the check, which the `stat`s can miss as well. Not seen:
//! a write through a shared `mmap`, which raises no event (TeX Live's
//! installers and editors do not write so).
//!
//! **Platforms.** Linux: `inotify`, one watch per directory and per file
//! (a write through another hard link reaches the file's own watch); a
//! directory's events about its entries' data are ignored, each file
//! looked at having its own watch; one with a name is about its entries,
//! one without (`IN_ATTRIB`, `IN_DELETE_SELF`, `IN_MOVE_SELF`) about the
//! directory itself. macOS: `kqueue` `EVFILT_VNODE` on a descriptor opened
//! `O_EVTONLY` per directory and file (a directory's `NOTE_WRITE` and
//! `NOTE_LINK` are changes of its entries; `NOTE_RENAME`, `NOTE_DELETE`,
//! `NOTE_REVOKE` and `NOTE_ATTRIB` its own; as measured on APFS: an entry
//! renamed over a file posts `NOTE_DELETE` on the file and `NOTE_WRITE` on
//! its directory, a directory renamed `NOTE_RENAME` on it, a symbolic link
//! replaced `NOTE_WRITE` on its directory only). Elsewhere: off (every call a
//! `stat`). The host turns it on ([`enable`]); `FLASHTEX_FSWATCH=off`
//! keeps it off.

use crate::statsig::StatSig;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// The most watches made; past it a path is `stat`ed every time.
const MAX_WATCHES: usize = 8192;

thread_local! {
    /// Inside [`check`] on this thread.
    static IN_CHECK: Cell<bool> = const { Cell::new(false) };
    /// The working directory of the check, in which a relative path is
    /// watched (`os::cwd`, taken at the check's first relative path; `None`
    /// until then, `Some(None)`: not known).
    static CWD: std::cell::RefCell<Option<Option<String>>> = const { std::cell::RefCell::new(None) };
    /// The check's own answers for paths no watch vouches for (the output
    /// directory's files, racy signatures): one `stat` each per check, the
    /// check taken as of one moment (a change between two `stat`s of one
    /// check is a change after it, as for one `stat`). Emptied by every
    /// [`check`].
    #[allow(clippy::type_complexity)]
    static MEMO: std::cell::RefCell<HashMap<String, (Option<Option<StatSig>>, [Option<bool>; 2])>> =
        std::cell::RefCell::new(HashMap::new());
}

static W: Mutex<Option<Watch>> = Mutex::new(None);

/// Counts, for the tests and the latency accounting: signatures served,
/// `stat`s made inside checks, drains that found events, overflows.
static COUNTS: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];

/// `FLASHTEX_FSWATCH_DEBUG`: each `stat` a check makes, and why.
fn debug() -> bool {
    static D: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *D.get_or_init(|| std::env::var_os("FLASHTEX_FSWATCH_DEBUG").is_some())
}

fn count(i: usize) {
    COUNTS[i].fetch_add(1, Ordering::Relaxed);
}

pub fn counts() -> [u64; 4] {
    std::array::from_fn(|i| COUNTS[i].load(Ordering::Relaxed))
}

/// Turn the watcher on (the hosts that serve edits). Idempotent; a failure
/// to make the kernel's queue leaves it off.
pub fn enable() {
    if std::env::var("FLASHTEX_FSWATCH").is_ok_and(|v| v == "off") {
        return;
    }
    let mut g = W.lock().unwrap();
    if g.is_none() {
        *g = Watch::new();
    }
}

pub fn enabled() -> bool {
    W.lock().unwrap().is_some()
}

/// Paths under `prefix` (the output directory) are never watched:
/// `stat`ed at every check.
pub fn exclude(prefix: &str) {
    if let Some(w) = W.lock().unwrap().as_mut() {
        let p = prefix.trim_end_matches('/').to_string();
        if !p.is_empty() && !w.excluded.contains(&p) {
            w.entries.retain(|k, _| !under(k, &p));
            w.excluded.push(p);
        }
    }
}

/// Drain the kernel's queue of events now.
pub fn sync() {
    crate::os::io_time(crate::os::io::DRAIN, || {
        if let Some(w) = W.lock().unwrap().as_mut() {
            w.drain();
        }
    })
}

/// A check: [`sync`] first, then [`sig`] and [`readable`] serve vouched
/// answers on this thread until the guard is dropped.
pub struct Check {
    was: bool,
    cwd: Option<Option<String>>,
}

#[track_caller]
pub fn check() -> Check {
    if debug() {
        eprintln!("[fswatch] check at {}", std::panic::Location::caller());
    }
    sync();
    MEMO.with(|m| m.borrow_mut().clear());
    let cwd = CWD.with(|c| c.replace(None));
    let was = IN_CHECK.with(|c| c.replace(true));
    Check { was, cwd }
}

impl Drop for Check {
    fn drop(&mut self) {
        IN_CHECK.with(|c| c.set(self.was));
        CWD.with(|c| *c.borrow_mut() = self.cwd.take());
    }
}

/// `path` as the watches know it: absolute, a relative path in the check's
/// working directory (leading `./` dropped). `None`: not watched (a `.` or
/// `..` component inside it, or no working directory known).
fn watched_as(path: &str) -> Option<std::borrow::Cow<'_, str>> {
    use std::borrow::Cow;
    let p = if path.starts_with('/') {
        Cow::Borrowed(path)
    } else {
        let mut rest = path;
        while let Some(r) = rest.strip_prefix("./") {
            rest = r.trim_start_matches('/');
        }
        let rest = rest.trim_end_matches('/');
        let p = CWD.with(|c| {
            let mut c = c.borrow_mut();
            let cwd = c.get_or_insert_with(|| {
                crate::os::cwd()
                    .ok()
                    .and_then(|c| c.to_str().map(str::to_string))
                    .filter(|c| names_here(c))
            });
            let cwd = cwd.as_deref()?.trim_end_matches('/');
            Some(match rest {
                "" | "." if cwd.is_empty() => "/".to_string(),
                "" | "." => cwd.to_string(),
                r => format!("{cwd}/{r}"),
            })
        })?;
        Cow::Owned(p)
    };
    if p.contains("/./") || p.contains("/../") || p.ends_with("/.") || p.ends_with("/..") {
        return None;
    }
    Some(p)
}

/// Whether `dir` (absolute) names the process's working directory now, in
/// which the system resolves a relative path: the watches are on the name
/// (`os::cwd` is taken once per compile, `os::cwd_scope`), so a name that
/// went elsewhere -- the directory renamed under the process -- watches
/// nothing relative. (A rename after this is an event of the directory's
/// own on its watch, and this is asked again at the next check.)
fn names_here(dir: &str) -> bool {
    dir.starts_with('/')
        && match (std::fs::metadata("."), std::fs::metadata(dir)) {
            (Ok(a), Ok(b)) => a.is_dir() && file_id(&a) == file_id(&b),
            _ => false,
        }
}

/// `path`'s signature: inside a [`check`], the kept one where the events
/// vouch for it; else a `stat` (kept, inside a check, for the next).
pub fn sig(path: &str) -> Option<StatSig> {
    if !IN_CHECK.with(|c| c.get()) {
        return StatSig::of(path);
    }
    let mut g = W.lock().unwrap();
    let Some(w) = g.as_mut() else {
        drop(g);
        return StatSig::of(path);
    };
    let key = watched_as(path);
    let kept = key.as_deref().and_then(|k| w.kept(k));
    if let Some(s) = kept.and_then(|e| e.sig) {
        count(0);
        return s;
    }
    if let Some(s) = MEMO.with(|m| m.borrow().get(path).and_then(|e| e.0)) {
        return s;
    }
    count(1);
    // (an entry whose watches all hold needs none made again: the racy
    // file the user types in, at every keystroke)
    let deps = match kept {
        Some(e) => Some(e.deps.clone()),
        None => key.as_deref().and_then(|k| w.arm(k)),
    };
    let s = StatSig::of(path);
    if debug() {
        eprintln!(
            "[fswatch] stat {path}: {}",
            if deps.is_none() {
                "not watched"
            } else if s.is_some_and(|s| s.racy) {
                "racy"
            } else {
                "kept"
            }
        );
    }
    match (deps, &key) {
        (Some(deps), Some(k)) if !s.is_some_and(|s| s.racy) => w.entry(k, deps).sig = Some(s),
        _ => MEMO.with(|m| m.borrow_mut().entry(path.to_string()).or_default().0 = Some(s)),
    }
    s
}

/// The readable test of `path` that `lookupproof::dep_sig` makes
/// (kpathsea's: not a directory, and readable, here by opening it):
/// vouched for like [`sig`].
pub fn readable(path: &str) -> bool {
    readable_as(path, Readable::Open, |p| {
        std::fs::metadata(p).is_ok_and(|m| !m.is_dir()) && std::fs::File::open(p).is_ok()
    })
}

/// Which readable test an answer is ([`readable_as`]): each is kept apart,
/// so that a test is only ever answered by its own earlier result.
#[derive(Clone, Copy)]
pub enum Readable {
    /// [`readable`]'s.
    Open = 0,
    /// kpathsea's own `kpathsea_readable_file` (the resolver's lookup memo).
    Kpathsea = 1,
}

/// `test(path)` (a readable test, `kind`), vouched for like [`sig`].
pub fn readable_as(path: &str, kind: Readable, test: impl FnOnce(&str) -> bool) -> bool {
    let k = kind as usize;
    if !IN_CHECK.with(|c| c.get()) {
        return test(path);
    }
    let mut g = W.lock().unwrap();
    let Some(w) = g.as_mut() else {
        drop(g);
        return test(path);
    };
    let key = watched_as(path);
    let kept = key.as_deref().and_then(|kk| w.kept(kk));
    if let Some(r) = kept.and_then(|e| e.readable[k]) {
        count(0);
        return r;
    }
    if let Some(r) = MEMO.with(|m| m.borrow().get(path).and_then(|e| e.1[k])) {
        return r;
    }
    count(1);
    let deps = match kept {
        Some(e) => Some(e.deps.clone()),
        None => key.as_deref().and_then(|kk| w.arm(kk)),
    };
    let r = test(path);
    if debug() {
        eprintln!(
            "[fswatch] readable{k} {path}: {}",
            if deps.is_none() {
                "not watched"
            } else {
                "kept"
            }
        );
    }
    match (deps, &key) {
        (Some(deps), Some(kk)) => w.entry(kk, deps).readable[k] = Some(r),
        _ => MEMO.with(|m| m.borrow_mut().entry(path.to_string()).or_default().1[k] = Some(r)),
    }
    r
}

#[derive(Default)]
struct Entry {
    /// The signature (`Some(None)`: no such file), if one that is not racy
    /// was taken under `deps`.
    sig: Option<Option<StatSig>>,
    /// Each readable test's answer (`Readable`), if taken under `deps`.
    readable: [Option<bool>; 2],
    /// The watches it depends on, with their generations when it was kept.
    deps: Vec<Dep>,
}

/// A watch an entry depends on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Dep {
    target: u32,
    /// Every event on it counts (`Target::gen`), not only those about the
    /// object itself (`Target::own`): see the module's documentation.
    whole: bool,
    /// That generation when the entry was kept.
    gen: u64,
}

struct Target {
    /// Every event so far.
    gen: u64,
    /// The events about the object itself (renamed, moved, removed,
    /// revoked, its attributes changed), not about a directory's entries.
    own: u64,
    /// The object's device and inode when the watch was made.
    id: (u64, u64),
    /// Dead (the kernel dropped it, or its object was moved or deleted): a
    /// later arm makes the watch again.
    dead: bool,
    #[cfg(target_os = "macos")]
    fd: i32,
}

struct Watch {
    q: i32,
    targets: Vec<Target>,
    live: usize,
    by_path: HashMap<String, u32>,
    #[cfg(target_os = "linux")]
    by_wd: HashMap<i32, Vec<u32>>,
    entries: HashMap<String, Entry>,
    /// Device -> whether it is a local volume.
    local: HashMap<u64, bool>,
    excluded: Vec<String>,
}

impl Watch {
    fn with_queue(q: i32) -> Watch {
        Watch {
            q,
            targets: vec![],
            live: 0,
            by_path: HashMap::new(),
            #[cfg(target_os = "linux")]
            by_wd: HashMap::new(),
            entries: HashMap::new(),
            local: HashMap::new(),
            excluded: vec![],
        }
    }

    /// The entry for `path` while every watch it depends on is unchanged.
    fn kept(&self, path: &str) -> Option<&Entry> {
        let e = self.entries.get(path)?;
        e.deps
            .iter()
            .all(|d| {
                self.targets
                    .get(d.target as usize)
                    .is_some_and(|t| !t.dead && (if d.whole { t.gen } else { t.own }) == d.gen)
            })
            .then_some(e)
    }

    /// `path`'s entry under `deps`: the one kept if it was taken under the
    /// same watches and generations, else a new one.
    fn entry(&mut self, path: &str, deps: Vec<Dep>) -> &mut Entry {
        let e = self.entries.entry(path.to_string()).or_default();
        if e.deps != deps {
            *e = Entry {
                deps,
                ..Entry::default()
            };
        }
        e
    }

    /// The watches `path` (absolute, [`watched_as`]) depends on, made
    /// where missing: every directory on its way (as written and as its
    /// canonical path) and the object itself if it exists, or, past a
    /// missing component, the deepest existing directory; each counting
    /// every event or only those about itself (`Dep::whole`, [`Watch::walk`]).
    /// `None`: the path is not watched, or a watch could not be made.
    fn arm(&mut self, path: &str) -> Option<Vec<Dep>> {
        if !path.starts_with('/') {
            return None;
        }
        if self.excluded.iter().any(|x| under(path, x)) {
            return None;
        }
        let mut deps = self.walk(path)?;
        // (a link on the way: the directories it points into, taken once
        // the path's own watches are made, so that a link changed since is
        // an event on the directory holding it)
        let canon = std::fs::canonicalize(path).ok().or_else(|| {
            let parent = std::path::Path::new(path).parent()?;
            std::fs::canonicalize(parent).ok()
        });
        if let Some(c) = canon.and_then(|c| c.to_str().map(str::to_string)) {
            if c != path {
                for d in self.walk(&c)? {
                    match deps.iter_mut().find(|x| x.target == d.target) {
                        Some(x) if x.whole || !d.whole => {}
                        Some(x) => *x = d,
                        None => deps.push(d),
                    }
                }
            }
        }
        Some(deps)
    }

    /// The watches on the way to `path` (absolute, no `.` or `..`
    /// components), top down, made where missing, as far as its components
    /// exist; each counting every event (`Dep::whole`) where it is the
    /// object or the directory holding it, or past a missing component the
    /// deepest existing directory or the one holding it, or a directory
    /// holding a symbolic link on the way; elsewhere only events about the
    /// directory itself (the module's documentation). A component is
    /// looked at (`lstat`) before its watch is made and after: one that
    /// changed meanwhile leaves the path unwatched.
    fn walk(&mut self, path: &str) -> Option<Vec<Dep>> {
        let mut v: Vec<Dep> = vec![];
        for a in ancestors(path) {
            // (a missing component: nothing past it exists, and its
            // creation is an event on the deepest existing directory)
            let Ok(before) = std::fs::symlink_metadata(&a) else {
                break;
            };
            let m = if before.file_type().is_symlink() {
                std::fs::metadata(&a).ok()?
            } else {
                before.clone()
            };
            if !self.is_local(&a, &m) {
                return None;
            }
            let i = self.target(&a, file_id(&m))?;
            let after = std::fs::symlink_metadata(&a).ok()?;
            if !same_entry(&before, &after) {
                return None;
            }
            if before.file_type().is_symlink() {
                if let Some(d) = v.last_mut() {
                    d.whole = true;
                }
            }
            v.push(Dep {
                target: i,
                whole: false,
                gen: 0,
            });
        }
        let n = v.len();
        for d in v.iter_mut().skip(n.saturating_sub(2)) {
            d.whole = true;
        }
        // (no event is counted before the next check's drain)
        for d in &mut v {
            let t = &self.targets[d.target as usize];
            d.gen = if d.whole { t.gen } else { t.own };
        }
        Some(v)
    }

    #[cfg(not(unix))]
    fn is_local(&mut self, _path: &str, _m: &std::fs::Metadata) -> bool {
        false
    }

    #[cfg(unix)]
    fn is_local(&mut self, path: &str, m: &std::fs::Metadata) -> bool {
        use std::os::unix::fs::MetadataExt;
        let dev = m.dev();
        if let Some(&l) = self.local.get(&dev) {
            return l;
        }
        let l = statfs_local(path);
        self.local.insert(dev, l);
        l
    }

    /// The watch on `p`, made if missing or if it watches another object
    /// than `p` names now (`id`: its device and inode, followed): its index.
    fn target(&mut self, p: &str, id: (u64, u64)) -> Option<u32> {
        if let Some(&i) = self.by_path.get(p) {
            let t = &self.targets[i as usize];
            if !t.dead && t.id == id {
                return Some(i);
            }
            if !t.dead {
                self.bump(i as usize, true, true);
            }
        }
        if self.live >= MAX_WATCHES {
            return None;
        }
        let i = self.targets.len() as u32;
        let mut t = self.add(p, i)?;
        t.id = id;
        self.targets.push(t);
        self.live += 1;
        self.by_path.insert(p.to_string(), i);
        Some(i)
    }

    /// An event on watch `i`: about the object itself (`own`) or about a
    /// directory's entries; the watch is gone (`gone`: an event of its own
    /// too, and it is made again by the next [`Watch::target`]).
    fn bump(&mut self, i: usize, own: bool, gone: bool) {
        let Some(t) = self.targets.get_mut(i) else {
            return;
        };
        t.gen += 1;
        if own || gone {
            t.own += 1;
        }
        if gone && !t.dead {
            t.dead = true;
            self.live -= 1;
            #[cfg(target_os = "macos")]
            // SAFETY: our descriptor (closing it removes its event filter).
            unsafe {
                libc::close(t.fd)
            };
        }
    }

    fn bump_all(&mut self) {
        for t in &mut self.targets {
            t.gen += 1;
            t.own += 1;
        }
        count(3);
    }
}

/// A file's device and inode, as `metadata` gives them (0 where unknown).
fn file_id(m: &std::fs::Metadata) -> (u64, u64) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        (m.dev(), m.ino())
    }
    #[cfg(not(unix))]
    {
        let _ = m;
        (0, 0)
    }
}

/// Two `lstat`s of one path found the same entry: the same object, of the
/// same type.
fn same_entry(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    file_id(a) == file_id(b) && a.file_type() == b.file_type()
}

/// `path` is `dir` or under it.
fn under(path: &str, dir: &str) -> bool {
    path.strip_prefix(dir)
        .is_some_and(|r| r.is_empty() || r.starts_with('/'))
}

/// `/a/b/c` -> `/`, `/a`, `/a/b`, `/a/b/c`.
fn ancestors(path: &str) -> Vec<String> {
    let path = path.trim_end_matches('/');
    let mut v = vec!["/".to_string()];
    let mut at = 0;
    while let Some(k) = path.get(at + 1..).and_then(|r| r.find('/')) {
        at += 1 + k;
        v.push(path[..at].to_string());
    }
    if !path.is_empty() {
        v.push(path.to_string());
    }
    v
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn statfs_local(path: &str) -> bool {
    let Ok(c) = std::ffi::CString::new(path) else {
        return false;
    };
    // SAFETY: a zeroed plain C struct.
    let mut s: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: a NUL-terminated path and a buffer of the type statfs fills.
    if unsafe { libc::statfs(c.as_ptr(), &mut s) } != 0 {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        s.f_flags & libc::MNT_LOCAL as u32 != 0
    }
    #[cfg(target_os = "linux")]
    {
        // statfs(2)'s magic numbers of the network and user-space file
        // systems (NFS, SMB, CIFS, SMB2, FUSE, AFS, Ceph, 9P, Coda, NCP) and
        // of overlayfs, whose lower layers' changes raise no event.
        const REMOTE: [i64; 11] = [
            0x6969,
            0x517B,
            0xFF53_4D42u32 as i64,
            0xFE53_4D42u32 as i64,
            0x6573_5546,
            0x5346_414F,
            0x00C3_6400,
            0x0102_1997,
            0x7375_7245,
            0x564C,
            0x794C_7630,
        ];
        !REMOTE.contains(&(s.f_type as i64))
    }
}

#[cfg(target_os = "linux")]
impl Watch {
    fn new() -> Option<Watch> {
        // SAFETY: a plain system call.
        let q = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        (q >= 0).then(|| Watch::with_queue(q))
    }

    fn add(&mut self, p: &str, i: u32) -> Option<Target> {
        let c = std::ffi::CString::new(p).ok()?;
        // Every change to the object (its data, attributes, links, its
        // removal or move) and, for a directory, to its entries.
        let mask = libc::IN_MODIFY
            | libc::IN_ATTRIB
            | libc::IN_CLOSE_WRITE
            | libc::IN_CREATE
            | libc::IN_DELETE
            | libc::IN_DELETE_SELF
            | libc::IN_MOVE_SELF
            | libc::IN_MOVED_FROM
            | libc::IN_MOVED_TO;
        // SAFETY: a NUL-terminated path.
        let wd = unsafe { libc::inotify_add_watch(self.q, c.as_ptr(), mask) };
        if wd < 0 {
            return None;
        }
        self.by_wd.entry(wd).or_default().push(i);
        Some(Target {
            gen: 0,
            own: 0,
            id: (0, 0),
            dead: false,
        })
    }

    fn drain(&mut self) {
        // (aligned for the events the kernel writes into it)
        let mut buf = [0u64; 2048];
        let mut any = false;
        loop {
            // SAFETY: a buffer of the length given.
            let n =
                unsafe { libc::read(self.q, buf.as_mut_ptr().cast(), std::mem::size_of_val(&buf)) };
            if n < 0 {
                let e = std::io::Error::last_os_error().raw_os_error();
                if e == Some(libc::EAGAIN) || e == Some(libc::EWOULDBLOCK) {
                    break;
                }
                if e == Some(libc::EINTR) {
                    continue;
                }
                // (the queue cannot be read: nothing is vouched for)
                self.bump_all();
                break;
            }
            if n == 0 {
                break;
            }
            any = true;
            let bytes = buf.as_ptr() as *const u8;
            let hdr = std::mem::size_of::<libc::inotify_event>();
            let mut at = 0usize;
            while at + hdr <= n as usize {
                // SAFETY: the kernel wrote whole events into the first `n` bytes.
                let ev: libc::inotify_event =
                    unsafe { std::ptr::read_unaligned(bytes.add(at).cast()) };
                at += hdr + ev.len as usize;
                if ev.mask & libc::IN_Q_OVERFLOW != 0 {
                    self.bump_all();
                    continue;
                }
                // An event about a directory's entry (it has a name) counts
                // when the entries change. The entry's data and attributes
                // are its own watch's (every object looked at has one), so a
                // log written in a home directory above TeX Live's tree does
                // not unvouch the whole tree.
                let entries =
                    libc::IN_CREATE | libc::IN_DELETE | libc::IN_MOVED_FROM | libc::IN_MOVED_TO;
                if ev.len > 0 && ev.mask & (entries | libc::IN_IGNORED | libc::IN_UNMOUNT) == 0 {
                    continue;
                }
                let gone = ev.mask
                    & (libc::IN_IGNORED
                        | libc::IN_DELETE_SELF
                        | libc::IN_MOVE_SELF
                        | libc::IN_UNMOUNT)
                    != 0;
                let Some(ids) = self.by_wd.get(&ev.wd).cloned() else {
                    continue;
                };
                // (one without a name is about the watched object itself)
                let own = ev.len == 0;
                for i in ids {
                    self.bump(i as usize, own, gone);
                }
                if gone {
                    self.by_wd.remove(&ev.wd);
                    if ev.mask & libc::IN_IGNORED == 0 {
                        // SAFETY: a watch of this queue.
                        unsafe { libc::inotify_rm_watch(self.q, ev.wd) };
                    }
                }
            }
        }
        if any {
            count(2);
        }
    }
}

#[cfg(target_os = "macos")]
impl Watch {
    fn new() -> Option<Watch> {
        // (a watch is a descriptor: room for them under the soft limit)
        // SAFETY: plain system calls on a local struct.
        unsafe {
            let mut r: libc::rlimit = std::mem::zeroed();
            if libc::getrlimit(libc::RLIMIT_NOFILE, &mut r) == 0 {
                let want = r.rlim_max.min(10240);
                if r.rlim_cur < want {
                    r.rlim_cur = want;
                    libc::setrlimit(libc::RLIMIT_NOFILE, &r);
                }
            }
        }
        // SAFETY: a plain system call.
        let q = unsafe { libc::kqueue() };
        (q >= 0).then(|| Watch::with_queue(q))
    }

    fn add(&mut self, p: &str, i: u32) -> Option<Target> {
        // (room left for the run's own files)
        // SAFETY: a zeroed plain C struct.
        let mut r: libc::rlimit = unsafe { std::mem::zeroed() };
        // SAFETY: a local struct.
        if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut r) } != 0
            || self.live as u64 + 512 > r.rlim_cur
        {
            return None;
        }
        let c = std::ffi::CString::new(p).ok()?;
        // SAFETY: a NUL-terminated path.
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_EVTONLY | libc::O_CLOEXEC) };
        if fd < 0 {
            return None;
        }
        let ev = libc::kevent {
            ident: fd as usize,
            filter: libc::EVFILT_VNODE,
            flags: libc::EV_ADD | libc::EV_CLEAR,
            fflags: libc::NOTE_DELETE
                | libc::NOTE_WRITE
                | libc::NOTE_EXTEND
                | libc::NOTE_ATTRIB
                | libc::NOTE_LINK
                | libc::NOTE_RENAME
                | libc::NOTE_REVOKE,
            data: 0,
            udata: i as usize as *mut libc::c_void,
        };
        // SAFETY: one change in, no events out.
        let r = unsafe { libc::kevent(self.q, &ev, 1, std::ptr::null_mut(), 0, std::ptr::null()) };
        if r < 0 {
            // SAFETY: our descriptor.
            unsafe { libc::close(fd) };
            return None;
        }
        Some(Target {
            gen: 0,
            own: 0,
            id: (0, 0),
            dead: false,
            fd,
        })
    }

    fn drain(&mut self) {
        let zero = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: zeroed plain C structs.
        let mut evs: [libc::kevent; 256] = unsafe { std::mem::zeroed() };
        let mut any = false;
        loop {
            // SAFETY: an out buffer of the length given; no changes in.
            let n = unsafe {
                libc::kevent(
                    self.q,
                    std::ptr::null(),
                    0,
                    evs.as_mut_ptr(),
                    evs.len() as i32,
                    &zero,
                )
            };
            if n < 0 {
                if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                self.bump_all();
                break;
            }
            if n == 0 {
                break;
            }
            any = true;
            for &ev in &evs[..n as usize] {
                let gone = ev.flags & libc::EV_ERROR != 0
                    || ev.fflags & (libc::NOTE_DELETE | libc::NOTE_RENAME | libc::NOTE_REVOKE) != 0;
                // (a directory's `NOTE_WRITE` and `NOTE_LINK` are about its
                // entries; on a file every event is the file's own, and a
                // file is always watched whole)
                let own = ev.fflags & libc::NOTE_ATTRIB != 0;
                self.bump(ev.udata as usize, own, gone);
            }
            if (n as usize) < evs.len() {
                break;
            }
        }
        if any {
            count(2);
        }
    }
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "linux"))))]
fn statfs_local(_path: &str) -> bool {
    false
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl Watch {
    fn new() -> Option<Watch> {
        None
    }
    fn add(&mut self, _p: &str, _i: u32) -> Option<Target> {
        None
    }
    fn drain(&mut self) {}
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::*;

    #[test]
    fn ancestors_of_a_path() {
        assert_eq!(ancestors("/a/b/c"), ["/", "/a", "/a/b", "/a/b/c"]);
        assert_eq!(ancestors("/a/"), ["/", "/a"]);
        assert_eq!(ancestors("/"), ["/"]);
    }

    /// Each change -- a watched file's data, an entry made where a lookup
    /// found none, a directory on the way replaced -- is seen at the next
    /// check; an unchanged path is served without a `stat`; outside a
    /// check every call is a `stat`.
    #[test]
    fn changes_are_seen_at_the_next_check() {
        let d = std::env::temp_dir().join(format!("fswatch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        let d = std::fs::canonicalize(&d).unwrap();
        let f = d.join("sub/f.tex");
        std::fs::write(&f, b"one").unwrap();
        let p = f.to_str().unwrap().to_string();
        let missing = d.join("sub/g.tex").to_str().unwrap().to_string();
        // (a signature taken now would be racy, and is never kept)
        std::env::set_var("FLASHTEX_RACY_MS", "0");
        enable();
        assert!(enabled());
        // (whether a check, its queue drained, serves `path` without a
        // `stat`: this thread's answer, not the process-wide counts, which
        // the crate's other tests move as well)
        let vouched = |path: &str| {
            W.lock()
                .unwrap()
                .as_ref()
                .is_some_and(|w| w.kept(path).is_some_and(|e| e.sig.is_some()))
        };
        {
            let _c = check();
            assert!(sig(&p).is_some());
            assert!(sig(&missing).is_none());
        }
        // An entry made or removed in a directory on the way -- the
        // temporary directory, where the crate's other tests and other
        // processes make files -- unvouches both, as it should; a check with
        // no event since the last serves both. (Bounded: also past the
        // racy window, should another test have fixed it at 2 s first.)
        let start = std::time::Instant::now();
        loop {
            let c = check();
            if vouched(&p) && vouched(&missing) {
                assert_eq!(sig(&p).map(|s| s.len), Some(3));
                assert!(sig(&missing).is_none());
                break;
            }
            assert_eq!(sig(&p).map(|s| s.len), Some(3));
            assert!(sig(&missing).is_none());
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "both served from the watches"
            );
            drop(c);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        std::fs::write(&f, b"three").unwrap();
        {
            let _c = check();
            assert_eq!(sig(&p).map(|s| s.len), Some(5));
        }
        std::fs::write(&missing, b"x").unwrap();
        {
            let _c = check();
            assert!(sig(&missing).is_some());
            assert!(readable(&missing));
        }
        std::fs::rename(d.join("sub"), d.join("old")).unwrap();
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(&f, b"fourth!").unwrap();
        {
            let _c = check();
            assert_eq!(sig(&p).map(|s| s.len), Some(7));
            assert!(!readable(&missing));
        }
        std::fs::write(&f, b"x").unwrap();
        assert_eq!(sig(&p).map(|s| s.len), Some(1));
        exclude(d.to_str().unwrap());
        {
            let _c = check();
            sig(&p);
        }
        {
            let _c = check();
            assert!(!vouched(&p), "an excluded path is never served");
            assert_eq!(sig(&p).map(|s| s.len), Some(1));
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Lane FIND-PHASE: a document's own files, deep under directories
    /// other programs write in. Entries
    /// made and removed further up leave a path served (no `stat`); every
    /// change that can change what it names is seen at the next check: the
    /// file rewritten in place with its time put back, another file renamed
    /// over it, a directory on the way replaced, a symbolic link on the way
    /// pointed elsewhere, a missing file made.
    #[test]
    fn the_documents_own_files_are_watched_and_their_changes_seen() {
        let d = std::env::temp_dir().join(format!("fswatch-doc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("doc/a/b")).unwrap();
        std::fs::create_dir_all(d.join("doc/real")).unwrap();
        std::fs::create_dir_all(d.join("doc/real2")).unwrap();
        let d = std::fs::canonicalize(&d).unwrap();
        let doc = d.join("doc");
        let s = |p: &std::path::Path| p.to_str().unwrap().to_string();
        let f = doc.join("a/b/f.tex");
        std::fs::write(&f, b"one").unwrap();
        std::fs::write(doc.join("real/h.tex"), b"hh").unwrap();
        std::fs::write(doc.join("real2/h.tex"), b"other").unwrap();
        std::os::unix::fs::symlink(doc.join("real"), doc.join("ln")).unwrap();
        let (pf, ph) = (s(&f), s(&doc.join("ln/h.tex")));
        let missing = s(&doc.join("a/b/new.tex"));
        std::env::set_var("FLASHTEX_RACY_MS", "0");
        enable();
        assert!(enabled());
        let vouched = |path: &str| {
            W.lock()
                .unwrap()
                .as_ref()
                .is_some_and(|w| w.kept(path).is_some_and(|e| e.sig.is_some()))
        };
        let all = || {
            (
                sig(&pf).map(|s| s.len),
                sig(&ph).map(|s| s.len),
                sig(&missing).map(|s| s.len),
            )
        };
        let start = std::time::Instant::now();
        loop {
            let _c = check();
            assert_eq!(all(), (Some(3), Some(2), None));
            if [&pf, &ph, &missing].iter().all(|p| vouched(p)) {
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "served from the watches"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // Entries made and removed above the file's directory: still served.
        for k in 0..3 {
            for dir in [&d, &doc, &doc.join("a")] {
                let junk = dir.join(format!("junk{k}"));
                std::fs::write(&junk, b"j").unwrap();
                std::fs::remove_file(&junk).unwrap();
            }
            let _c = check();
            assert!(vouched(&pf), "an entry made further up unvouched it");
            assert_eq!(all(), (Some(3), Some(2), None));
        }
        // in place, the same length, the time put back
        {
            let mtime = std::fs::metadata(&f).unwrap().modified().unwrap();
            use std::io::Write;
            let mut h = std::fs::File::options().write(true).open(&f).unwrap();
            h.write_all(b"two").unwrap();
            h.set_modified(mtime).unwrap();
        }
        let old = StatSig::of_within(&pf, 0).unwrap();
        {
            let _c = check();
            assert!(!vouched(&pf));
            assert!(sig(&pf).is_some_and(|s| s.same_fields(&old)));
        }
        // another file renamed over it
        std::fs::write(doc.join("a/b/tmp"), b"three!").unwrap();
        std::fs::rename(doc.join("a/b/tmp"), &f).unwrap();
        {
            let _c = check();
            assert_eq!(all(), (Some(6), Some(2), None));
        }
        // a directory on the way replaced
        std::fs::rename(doc.join("a"), doc.join("a-old")).unwrap();
        std::fs::create_dir_all(doc.join("a/b")).unwrap();
        std::fs::write(&f, b"four").unwrap();
        {
            let _c = check();
            assert_eq!(all(), (Some(4), Some(2), None));
        }
        // a symbolic link on the way pointed elsewhere
        std::fs::remove_file(doc.join("ln")).unwrap();
        std::os::unix::fs::symlink(doc.join("real2"), doc.join("ln")).unwrap();
        {
            let _c = check();
            assert_eq!(all(), (Some(4), Some(5), None));
        }
        // a missing file made
        std::fs::write(&missing, b"new!!!!").unwrap();
        {
            let _c = check();
            assert_eq!(all(), (Some(4), Some(5), Some(7)));
            assert!(readable(&missing));
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A relative path is watched as the working directory's entry, while
    /// the check's name for the directory is the process's (`names_here`).
    #[test]
    fn relative_paths_are_watched_in_the_working_directory() {
        let here = std::env::current_dir().unwrap();
        let here = here.to_str().unwrap().to_string();
        assert!(names_here(&here));
        assert!(!names_here(std::env::temp_dir().to_str().unwrap()));
        assert!(!names_here("relative"));
        enable();
        {
            let _c = check();
            assert_eq!(
                watched_as("./src/fswatch.rs").as_deref(),
                Some(format!("{here}/src/fswatch.rs").as_str())
            );
            assert_eq!(
                watched_as("././src").as_deref(),
                Some(format!("{here}/src").as_str())
            );
            assert_eq!(watched_as(".").as_deref(), Some(here.as_str()));
            assert_eq!(watched_as("src/../src"), None);
            assert_eq!(watched_as("/a/./b"), None);
        }
        // (the crate's own source, which nothing writes while the tests run)
        let abs = format!("{here}/src/fswatch.rs");
        let start = std::time::Instant::now();
        loop {
            let _c = check();
            let s = sig("./src/fswatch.rs");
            assert!(s.is_some());
            let served = W
                .lock()
                .unwrap()
                .as_ref()
                .is_some_and(|w| w.kept(&abs).is_some_and(|e| e.sig.is_some()));
            if served {
                assert_eq!(sig(&abs).map(|s| s.len), s.map(|s| s.len));
                break;
            }
            assert!(
                start.elapsed() < std::time::Duration::from_secs(10),
                "a relative path served from the watches"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
