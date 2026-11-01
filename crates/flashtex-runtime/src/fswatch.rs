//! FS-WATCH (lane T7-LAST12; DESIGN.md §5.1, §1.2): the signatures the
//! host's per-compile checks take of files outside the document, vouched
//! for by the kernel's file events instead of a `stat` each.
//!
//! At every keystroke the host checks that nothing S₀ and the later
//! checkpoints read has changed: S₀'s key (`host::Key::check_files`), the
//! journal after it (`incr::Session::changes`), the lookups' directories and
//! the resolver's on-disk directories (`refresh_disk_dirs`). A beamer
//! preamble reads 189 files of TeX Live's tree, and with the directories and
//! the lookups' answers that is about 600 `stat` calls a keystroke (PC
//! strace, beamer-30).
//!
//! **What is vouched for.** Inside a check ([`check`], which first drains
//! the kernel's queue of events, [`sync`]), [`sig`] gives a path's
//! signature as a `stat` at an earlier check gave it, when no event has
//! arrived since for the path itself or for any directory on the way to it
//! (as written, and as its canonical path: the directories a symbolic link
//! on the way points into). Anything else is a `stat`:
//! * a path never seen (the watches are made *before* the `stat` that is
//!   kept, so a change between them shows as an event: the startup case);
//! * the queue overflowed or could not be read (every path is then
//!   unvouched until its next `stat`);
//! * a watch could not be made (descriptor or watch limits), the file is on
//!   a volume that is not local (NFS, SMB, FUSE, overlayfs: changes made
//!   elsewhere raise no event), or the path is relative, or under the
//!   document's output directory ([`exclude`]), which the run writes at
//!   every compile;
//! * the path is under the document's directory ([`barrier`]) and this
//!   check's barrier did not hold: before its drain the check writes a
//!   sentinel file it watches, and the drain must find that write's event
//!   (lane TYPING-50MS: the watcher proven current at each keystroke, which
//!   the document's own files, the ones a user edits, are served under);
//! * the kept signature is racy (`StatSig::racy`): it equals nothing, and
//!   the caller compares content; a later `stat` may give one that is not.
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
//! installers do not write so).
//!
//! **Platforms.** Linux: `inotify`, one watch per directory and per file
//! (a write through another hard link reaches the file's own watch); a
//! directory's events about its entries' data are ignored, each file
//! looked at having its own watch. macOS: `kqueue` `EVFILT_VNODE` on a
//! descriptor opened `O_EVTONLY` per directory and file (a directory's
//! `NOTE_WRITE` is a change of its entries). Elsewhere: off (every call a
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
    /// The check's own answers for paths no watch vouches for (the
    /// document's files): one `stat` each per check, the check taken as of
    /// one moment (a change between two `stat`s of one check is a change
    /// after it, as for one `stat`). Emptied by every [`check`].
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

/// Paths under `prefix` (the document's output directory) are never
/// watched: `stat`ed every time.
pub fn exclude(prefix: &str) {
    if let Some(w) = W.lock().unwrap().as_mut() {
        let p = prefix.trim_end_matches('/').to_string();
        if !p.is_empty() && !w.excluded.contains(&p) {
            w.entries.retain(|k, _| !under(k, &p));
            w.excluded.push(p);
        }
    }
}

/// Paths under `prefix` (the document's directory) are served only by a
/// check whose barrier held (the drain found the event of the sentinel's
/// write just before it): else `stat`ed. (FLASHTEX_FSWATCH_DOC=off keeps
/// them `stat`ed every time, as [`exclude`] does.)
///
/// The sentinel is written in a private temporary directory, so document
/// and output files cannot replace it.
pub fn barrier(prefix: &str) {
    if std::env::var("FLASHTEX_FSWATCH_DOC").is_ok_and(|v| v == "off") {
        exclude(prefix);
        return;
    }
    if let Some(w) = W.lock().unwrap().as_mut() {
        let p = prefix.trim_end_matches('/').to_string();
        if !p.is_empty() && !w.barriered.contains(&p) {
            w.barriered.push(p);
        }
    }
}

/// Drain the kernel's queue of events now, behind the barrier (`Watch::
/// sync`).
pub fn sync() {
    crate::os::io_time(crate::os::io::DRAIN, || {
        if let Some(w) = W.lock().unwrap().as_mut() {
            w.sync();
        }
    })
}

#[cfg(test)]
thread_local! {
    /// (the tests: the next barrier's sentinel is not written, as when the
    /// queue dropped its event)
    static DROP_SENTINEL: Cell<bool> = const { Cell::new(false) };
}

/// A check: [`sync`] first, then [`sig`] and [`readable`] serve vouched
/// answers on this thread until the guard is dropped.
pub struct Check {
    was: bool,
}

pub fn check() -> Check {
    sync();
    MEMO.with(|m| m.borrow_mut().clear());
    let was = IN_CHECK.with(|c| c.replace(true));
    Check { was }
}

impl Drop for Check {
    fn drop(&mut self) {
        IN_CHECK.with(|c| c.set(self.was));
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
    if let Some(s) = w.served(path).and_then(|e| e.sig) {
        count(0);
        return s;
    }
    if let Some(s) = MEMO.with(|m| m.borrow().get(path).and_then(|e| e.0)) {
        return s;
    }
    count(1);
    let deps = w.arm(path);
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
    match deps {
        Some(deps) if !s.is_some_and(|s| s.racy) => w.entry(path, deps).sig = Some(s),
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
    if let Some(r) = w.served(path).and_then(|e| e.readable[k]) {
        count(0);
        return r;
    }
    if let Some(r) = MEMO.with(|m| m.borrow().get(path).and_then(|e| e.1[k])) {
        return r;
    }
    count(1);
    let deps = w.arm(path);
    let r = test(path);
    match deps {
        Some(deps) => w.entry(path, deps).readable[k] = Some(r),
        None => MEMO.with(|m| m.borrow_mut().entry(path.to_string()).or_default().1[k] = Some(r)),
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
    deps: Vec<(u32, u64)>,
}

struct Target {
    gen: u64,
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
    /// The prefixes served only behind the barrier ([`barrier`]).
    barriered: Vec<String>,
    /// The last [`Watch::sync`]'s barrier held.
    current: bool,
    /// The barrier's sentinel file, its watch and that watch's generation
    /// after the last barrier.
    sentinel: Option<(std::fs::File, u32, u64)>,
    /// Private directory where the sentinel is made.
    sentinel_dir: Option<std::path::PathBuf>,
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
            barriered: vec![],
            current: false,
            sentinel: None,
            sentinel_dir: None,
        }
    }

    /// [`kept`](Watch::kept), for a path under a [`barrier`] prefix only
    /// when the last sync's barrier held.
    fn served(&self, path: &str) -> Option<&Entry> {
        if !self.current && self.barriered.iter().any(|x| under(path, x)) {
            return None;
        }
        self.kept(path)
    }

    /// Drain the queue; with a [`barrier`] prefix, first write the
    /// sentinel, and the barrier holds if the drain found its event: every
    /// change completed before the write had queued its event before it (the
    /// kernel posts them in the system call that makes the change), so the
    /// drain saw them too. A sentinel that cannot be made or written, or
    /// whose event is missing, leaves the barrier not holding (those paths
    /// `stat`ed) until a later one holds.
    fn sync(&mut self) {
        self.current = false;
        let wrote = !self.barriered.is_empty() && self.write_sentinel();
        self.drain();
        if wrote {
            if let Some((_, i, gen)) = self.sentinel.as_mut() {
                let t = &self.targets[*i as usize];
                self.current = !t.dead && t.gen != *gen;
                *gen = t.gen;
                if t.dead {
                    self.sentinel = None;
                }
            }
        }
    }

    /// Write one byte in place into the sentinel file (made, and watched,
    /// the first time): an event on its own watch only.
    #[cfg(unix)]
    fn write_sentinel(&mut self) -> bool {
        use std::os::unix::fs::FileExt;
        if self.sentinel_dir.as_ref().is_some_and(|d| !d.is_dir()) {
            self.sentinel = None;
            self.sentinel_dir = None;
        }
        if self.sentinel.is_none() {
            let opened = (|| {
                let d = match &self.sentinel_dir {
                    Some(d) => d.clone(),
                    None => {
                        let d = private_sentinel_dir()?;
                        self.sentinel_dir = Some(d.clone());
                        d
                    }
                };
                let f = d.join(".flashtex-fswatch");
                // (made once: later, written in place)
                if !f.is_file() {
                    std::fs::write(&f, b"0").ok()?;
                }
                let p = std::fs::canonicalize(&f).ok()?.to_str()?.to_string();
                let h = std::fs::OpenOptions::new().write(true).open(&p).ok()?;
                let (i, gen) = self.target(&p)?;
                Some((h, i, gen))
            })();
            let Some(opened) = opened else {
                if let Some(d) = self.sentinel_dir.take() {
                    let _ = std::fs::remove_dir_all(d);
                }
                return false;
            };
            self.sentinel = Some(opened);
        }
        #[cfg(test)]
        if DROP_SENTINEL.with(|c| c.replace(false)) {
            return true;
        }
        let Some((h, _, _)) = self.sentinel.as_ref() else {
            return false;
        };
        h.write_at(b"1", 0).is_ok()
    }

    #[cfg(not(unix))]
    fn write_sentinel(&mut self) -> bool {
        false
    }

    /// Whether watch `i` is the sentinel's (its events are the barrier's,
    /// not a change: not counted as a drain that found events).
    #[cfg_attr(not(any(target_os = "macos", target_os = "linux")), allow(dead_code))]
    fn is_sentinel(&self, i: usize) -> bool {
        self.sentinel.as_ref().is_some_and(|s| s.1 as usize == i)
    }

    /// The entry for `path` while every watch it depends on is unchanged.
    fn kept(&self, path: &str) -> Option<&Entry> {
        let e = self.entries.get(path)?;
        e.deps
            .iter()
            .all(|&(i, g)| {
                self.targets
                    .get(i as usize)
                    .is_some_and(|t| !t.dead && t.gen == g)
            })
            .then_some(e)
    }

    /// `path`'s entry under `deps`: the one kept if it was taken under the
    /// same watches and generations, else a new one.
    fn entry(&mut self, path: &str, deps: Vec<(u32, u64)>) -> &mut Entry {
        let e = self.entries.entry(path.to_string()).or_default();
        if e.deps != deps {
            *e = Entry {
                deps,
                ..Entry::default()
            };
        }
        e
    }

    /// The watches `path` depends on, made where missing: every directory
    /// on its way (as written and as its canonical path) and the object
    /// itself if it exists, or, past a missing component, the deepest
    /// existing directory. `None`: the path is not watched, or a watch could
    /// not be made.
    fn arm(&mut self, path: &str) -> Option<Vec<(u32, u64)>> {
        if !path.starts_with('/')
            || path.contains("/./")
            || path.contains("/../")
            || path.ends_with("/.")
            || path.ends_with("/..")
        {
            return None;
        }
        if self.excluded.iter().any(|x| under(path, x)) {
            return None;
        }
        let mut chain: Vec<String> = ancestors(path);
        // (a link on the way: the directories it points into)
        let canon = std::fs::canonicalize(path).ok().or_else(|| {
            let parent = std::path::Path::new(path).parent()?;
            std::fs::canonicalize(parent).ok()
        });
        if let Some(c) = canon.and_then(|c| c.to_str().map(str::to_string)) {
            if c != path {
                for a in ancestors(&c) {
                    if !chain.contains(&a) {
                        chain.push(a);
                    }
                }
            }
        }
        let mut deps = Vec::with_capacity(chain.len());
        for t in &chain {
            // (a missing component: the deepest existing directory above
            // it is watched, and the component's creation is an event there)
            let Ok(m) = std::fs::metadata(t) else {
                continue;
            };
            if !self.is_local(t, &m) {
                return None;
            }
            deps.push(self.target(t)?);
        }
        Some(deps)
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

    /// The watch on `p`, made if missing: (index, generation).
    fn target(&mut self, p: &str) -> Option<(u32, u64)> {
        if let Some(&i) = self.by_path.get(p) {
            let t = &self.targets[i as usize];
            if !t.dead {
                return Some((i, t.gen));
            }
        }
        if self.live >= MAX_WATCHES {
            return None;
        }
        let i = self.targets.len() as u32;
        let t = self.add(p, i)?;
        self.targets.push(t);
        self.live += 1;
        self.by_path.insert(p.to_string(), i);
        Some((i, 0))
    }

    fn bump(&mut self, i: usize, gone: bool) {
        let Some(t) = self.targets.get_mut(i) else {
            return;
        };
        t.gen += 1;
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
        }
        count(3);
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        if let Some(d) = &self.sentinel_dir {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

#[cfg(unix)]
fn private_sentinel_dir() -> Option<std::path::PathBuf> {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let mut path = std::ffi::CString::new(
        std::env::temp_dir()
            .join("flashtex-fswatch-XXXXXX")
            .as_os_str()
            .as_bytes(),
    )
    .ok()?
    .into_bytes_with_nul();
    // SAFETY: mkdtemp replaces the six Xs in this writable, NUL-terminated buffer.
    if unsafe { libc::mkdtemp(path.as_mut_ptr().cast()) }.is_null() {
        return None;
    }
    path.pop(); // NUL
    Some(std::ffi::OsString::from_vec(path).into())
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
                for i in ids {
                    any |= !self.is_sentinel(i as usize);
                    self.bump(i as usize, gone);
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
            for &ev in &evs[..n as usize] {
                let gone = ev.flags & libc::EV_ERROR != 0
                    || ev.fflags & (libc::NOTE_DELETE | libc::NOTE_RENAME | libc::NOTE_REVOKE) != 0;
                any |= !self.is_sentinel(ev.udata as usize);
                self.bump(ev.udata as usize, gone);
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

    /// The document's directory (`barrier`): its files are served only by a
    /// check whose barrier held. A file written just before a check is seen
    /// by it; a check whose sentinel event is missing (as when the queue
    /// dropped it) serves nothing there, the next one does again; an
    /// overflowed or unreadable queue (`bump_all`) unvouches every path.
    #[test]
    fn document_files_are_served_behind_the_barrier() {
        let d = std::env::temp_dir().join(format!("fswatch-doc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("out")).unwrap();
        let d = std::fs::canonicalize(&d).unwrap();
        let f = d.join("chap.tex");
        std::fs::write(&f, b"one").unwrap();
        let p = f.to_str().unwrap().to_string();
        let missing = d.join("pkg.sty").to_str().unwrap().to_string();
        std::env::set_var("FLASHTEX_RACY_MS", "0");
        enable();
        exclude(d.join("out").to_str().unwrap());
        barrier(d.to_str().unwrap());
        let served = |path: &str| {
            W.lock()
                .unwrap()
                .as_ref()
                .is_some_and(|w| w.served(path).is_some_and(|e| e.sig.is_some()))
        };
        // (bounded: other tests and processes make files in the temporary
        // directory, on the way, which unvouches both for a check)
        let until_served = || {
            let start = std::time::Instant::now();
            loop {
                let c = check();
                assert_eq!(
                    sig(&p).map(|s| s.len),
                    std::fs::metadata(&p).ok().map(|m| m.len())
                );
                assert!(sig(&missing).is_none());
                if served(&p) && served(&missing) {
                    return c;
                }
                assert!(
                    start.elapsed() < std::time::Duration::from_secs(10),
                    "served behind the barrier"
                );
                drop(c);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        drop(until_served());
        let sentinel_dir = W
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .sentinel_dir
            .clone()
            .unwrap();
        assert!(sentinel_dir.join(".flashtex-fswatch").is_file());
        assert!(!sentinel_dir.starts_with(&d));
        // written immediately before the check, in place, the same length
        // and its time put back: seen
        let mtime = std::fs::metadata(&f).unwrap().modified().unwrap();
        {
            use std::io::Write;
            let mut h = std::fs::File::options().write(true).open(&f).unwrap();
            h.write_all(b"two").unwrap();
            h.set_modified(mtime).unwrap();
        }
        {
            let _c = check();
            assert!(!served(&p), "a file written before the check is not served");
            sig(&p);
        }
        std::fs::write(&missing, b"x").unwrap();
        {
            let _c = check();
            assert!(
                sig(&missing).is_some(),
                "a file made before the check is seen"
            );
        }
        std::fs::remove_file(&missing).unwrap();
        drop(until_served());
        // the barrier's event missing: nothing there is served
        DROP_SENTINEL.with(|c| c.set(true));
        {
            let _c = check();
            assert!(!served(&p) && !served(&missing), "no barrier, no serving");
            assert_eq!(sig(&p).map(|s| s.len), Some(3));
        }
        drop(until_served());
        // an overflowed or unreadable queue: nothing is served until a
        // `stat` is kept again
        W.lock().unwrap().as_mut().unwrap().bump_all();
        {
            let _c = check();
            assert!(
                !served(&p) && !served(&missing),
                "an overflow unvouches all"
            );
            assert_eq!(sig(&p).map(|s| s.len), Some(3));
        }
        drop(until_served());
        let mut g = W.lock().unwrap();
        let w = g.as_mut().unwrap();
        w.sentinel = None;
        if let Some(dir) = w.sentinel_dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn barrier_recovers_after_private_dir_is_deleted() {
        let mut w = Watch::new().unwrap();
        w.barriered
            .push(std::env::temp_dir().to_string_lossy().into_owned());
        w.sync();
        assert!(w.current);
        let old = w.sentinel_dir.clone().unwrap();
        std::fs::remove_dir_all(&old).unwrap();
        w.sync();
        assert!(w.current, "the next sync restores the barrier");
        assert_ne!(w.sentinel_dir.as_ref(), Some(&old));
        assert!(w.sentinel_dir.as_ref().unwrap().is_dir());
    }

    #[test]
    fn symlink_sentinel_does_not_write_outside_the_project() {
        use std::os::unix::fs::symlink;

        let d = std::env::temp_dir().join(format!("fswatch-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("out")).unwrap();
        let outside = d.with_extension("outside");
        std::fs::write(&outside, b"ABCDEF").unwrap();
        symlink(&outside, d.join("out/.flashtex-fswatch")).unwrap();
        enable();
        barrier(d.to_str().unwrap());
        let mut g = W.lock().unwrap();
        let w = g.as_mut().unwrap();
        w.sentinel = None;
        w.sync();
        assert_eq!(std::fs::read(&outside).unwrap(), b"ABCDEF");
        w.sentinel = None;
        if let Some(dir) = w.sentinel_dir.take() {
            let _ = std::fs::remove_dir_all(dir);
        }
        drop(g);
        let _ = std::fs::remove_dir_all(&d);
        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn fifo_sentinel_does_not_block() {
        let d = std::env::temp_dir().join(format!("fswatch-fifo-{}", std::process::id()));
        if std::env::var_os("FLASHTEX_FSWATCH_FIFO_CHILD").is_some() {
            enable();
            assert!(enabled());
            barrier(d.to_str().unwrap());
            let _c = check();
            *W.lock().unwrap() = None;
            return;
        }
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("out")).unwrap();
        use std::os::unix::ffi::OsStrExt;
        let fifo =
            std::ffi::CString::new(d.join("out/.flashtex-fswatch").as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("fswatch::tests::fifo_sentinel_does_not_block")
            .env("FLASHTEX_FSWATCH_FIFO_CHILD", d.join("out"))
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > std::time::Duration::from_secs(3) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("check blocked on a FIFO sentinel");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(status.success());
        use std::io::Read;
        let mut output = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut output)
            .unwrap();
        assert!(output.contains("1 passed"), "child ran no test: {output}");
        let _ = std::fs::remove_dir_all(&d);
    }
}
