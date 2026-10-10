//! Hot spares: the next run of the engine, started before its compile
//! comes, so that a compile does not wait for the program to start, for
//! kpathsea's start-up and for `xelatex.fmt` to be undumped.
//!
//! Every run of this host is a cold run in a process of its own
//! (`compile`); until the Unicode host is resident and incremental
//! (`docs/design/xetex/INCREMENTAL.md`), what each run does before it reads
//! the document is the same at every keystroke: the process starts, kpathsea
//! reads `texmf.cnf` and the `ls-R` databases, the engine initializes its
//! tables and undumps the 27 MB format, and the output reads `pdftex.map`.
//! About half of a keystroke on a one-page article.
//!
//! **A spare** is that run started early, for the compile the connection is
//! expected to make next: started when a run starts, with the same program,
//! arguments, environment and directory, it goes as far as the engine's
//! second reading of the clock (`fix_date_and_time` just after the format
//! is loaded; the first is in `tex_body`'s initialization, before the
//! load), reads the font map (`out::fonts::Fonts::prewarm`) and, if the last
//! run looked a font up by name, the installed fonts' catalog, and waits
//! there on a pipe (`FLASHTEX_SPARE_FD`). The next run of the same command
//! takes it: the host writes the go-ahead and the spare carries on exactly
//! as a run started then would, from the same state. Up to that wait an
//! engine run reads nothing of the document but the main file's first line
//! (`%&`, texmf.cnf's `parse_first_line`, in
//! `flashtex_engine::system::configure`) and whether the main file is
//! there; the clock it read there is read again after the load, and the
//! start time (`FORCE_SOURCE_DATE`) is dropped at the go-ahead and read
//! then. So a spare stands for a run when
//!
//! * the command is the same ([`Key`]: arguments, root, format directory and
//!   the client's font formats, the only per-run environment), and
//! * what it read is unchanged: the format file has the same identity (size,
//!   modification and status-change times, inode, device), the main file is
//!   still there, and its first line is the same or neither line starts with
//!   `%&` (a line that does not start with `%&` decides nothing).
//!
//! Anything else and the spare is ended and the run started as before. The
//! fonts' catalog a spare built is kept only if no directory the catalog's
//! scan walks has changed (its modification and status-change times, inode
//! and device: what adding, removing or renaming a font changes, the rule
//! fontconfig's own cache keeps); else the run builds it again, as it would
//! have.
//!
//! A spare that has waited longer than its time to live (default 120 s,
//! `FLASHTEX_UNICODE_SPARE_TTL`) is ended too, which also bounds how stale
//! the TeX Live configuration it read can be (texmf.cnf, `ls-R`, the font
//! map: files `tlmgr`, `mktexlsr` and `updmap` change, not the document).
//! `FLASHTEX_UNICODE_SPARES=0` turns spares off, and
//! `FLASHTEX_UNICODE_SPARE_DEBUG=1` reports each spare's fate on the host's
//! standard error.
//!
//! **Memory and bounds.** A spare holds the engine's initialized tables
//! (about 200 MB resident) while it waits, so the performance mode decides
//! (protocol §6.9): none in Low Memory, where a switch to it ends the one
//! there is, and kept in Balanced and High Performance. There is at most one
//! spare a document and at most two in all (`Slots`,
//! `FLASHTEX_UNICODE_SPARE_CAP`).
//!
//! **Trust and confinement.** A spare is started by the same function as the
//! run it stands for (`compile::engine_command`), so it has the same
//! arguments (shell escape, halt on error, output directory, job name,
//! `[fonts]`), directory and environment; the [`Key`] compares all of them,
//! the host's whole environment included (read confinement and its roots,
//! Live Share's too, `openout_any`, the TeX trees), and the compile's
//! external-tools setting. Any change ends the spare.

use flashtex_display_list::json::{obj, s as js, Json};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The environment variable naming a spare's go-ahead descriptor.
pub const SPARE_ENV: &str = "FLASHTEX_SPARE_FD";
/// Set for a spare that is to build the fonts' catalog while it waits.
pub const SPARE_FONTS_ENV: &str = "FLASHTEX_SPARE_FONTS";
/// Set for every run the host starts: the run says on its standard error
/// what the next run's spare should get ready ([`FONTS_HINT`]).
pub const HINTS_ENV: &str = "FLASHTEX_HOST_HINTS";
/// A run's hint that it looked a font up by name.
pub const FONTS_HINT: &str = "FlashTeX hint: fonts by name";
/// The exit status of a spare ended without its go-ahead.
pub const STALE_EXIT: i32 = 75;

/// What a run the host started is to do as such (from its environment).
#[derive(Clone, Copy, Debug, Default)]
pub struct ChildEnv {
    /// A spare's go-ahead descriptor.
    pub go: Option<i32>,
    /// A spare builds the fonts' catalog while it waits.
    pub fonts: bool,
    /// The run gives the host its hints.
    pub hints: bool,
    /// The run looked a font up by name (`find_native_font`).
    pub named_fonts: bool,
}

/// In the engine: what the host asked of this run. The variables are
/// removed, and the descriptor closed on `exec`, so that nothing the
/// engine starts sees either.
pub fn take_env() -> ChildEnv {
    let fd = std::env::var(SPARE_ENV)
        .ok()
        .and_then(|v| v.parse::<RawFd>().ok());
    let fonts = std::env::var_os(SPARE_FONTS_ENV).is_some();
    let hints = std::env::var_os(HINTS_ENV).is_some();
    for k in [SPARE_ENV, SPARE_FONTS_ENV, HINTS_ENV] {
        std::env::remove_var(k);
    }
    if let Some(fd) = fd {
        // SAFETY: fcntl(2) on a descriptor number; failure leaves it as it is.
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    ChildEnv {
        go: fd,
        fonts: fd.is_some() && fonts,
        hints,
        named_fonts: false,
    }
}

/// In the engine, as it exits: the hints for the next run's spare.
pub fn hints(g: &crate::Globals) {
    if g.host.child_env.hints && g.host.child_env.named_fonts {
        eprintln!("{FONTS_HINT}");
    }
}

/// The directories the fonts' catalog's scan walks
/// (`flashtex_font_discovery`'s walk: links followed, hidden entries left
/// out, at most 6 levels down), each with its identity, and the directory
/// [`crate::fontmgr::catalog::os_extra_dirs`] lists.
fn font_dirs() -> Vec<(PathBuf, Option<Ident>)> {
    fn tree(dir: &Path, depth: usize, out: &mut Vec<(PathBuf, Option<Ident>)>) {
        if depth > 6 {
            return;
        }
        out.push((dir.to_path_buf(), ident(dir)));
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut subs: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                !p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            })
            .filter(|p| std::fs::metadata(p).is_ok_and(|m| m.is_dir()))
            .collect();
        subs.sort();
        for s in subs {
            tree(&s, depth + 1, out);
        }
    }
    let mut dirs = flashtex_font_discovery::scan_dirs(None);
    for d in crate::fontmgr::catalog::os_extra_dirs() {
        if !dirs.contains(&d) {
            dirs.push(d);
        }
    }
    let mut out = vec![];
    if cfg!(target_os = "macos") {
        let a = Path::new("/System/Library/AssetsV2");
        out.push((a.to_path_buf(), ident(a)));
    }
    for d in dirs {
        tree(&d, 0, &mut out);
    }
    out
}

/// In the engine, at its second reading of the clock: get ready, then wait
/// for the go-ahead. Without it (the host ended the spare) the process
/// ends, having written nothing.
pub fn wait(g: &mut crate::Globals, fd: RawFd) {
    if let Some(o) = g.host.out.as_mut() {
        o.doc.fonts.prewarm();
    }
    // the fonts' catalog, as the first lookup by name would make it, and
    // the directories it was made from, read first
    let fonts = (g.host.child_env.fonts && g.host.font_mgr.is_none()).then(|| {
        let dirs = font_dirs();
        let catalog = crate::fontmgr::FontCatalog::system_cached(None);
        (dirs, crate::fontmgr::FontMgr::new(Arc::new(catalog)))
    });
    // SAFETY: the descriptor was inherited for this wait alone.
    let mut f = unsafe { File::from_raw_fd(fd) };
    let mut b = [0u8; 1];
    let go = loop {
        match f.read(&mut b) {
            Ok(1) => break b[0] == b'g',
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            _ => break false,
        }
    };
    drop(f);
    if !go {
        std::process::exit(STALE_EXIT);
    }
    // The run starts now: the start time (`FORCE_SOURCE_DATE` without
    // `SOURCE_DATE_EPOCH`) is the clock's now, not the spare's start.
    g.host.start = None;
    if let Some((dirs, mgr)) = fonts {
        if dirs.iter().all(|(d, id)| ident(d) == *id) {
            g.host.font_mgr = Some(Arc::new(mgr));
        }
    }
}

/// What a spare was started as: a run it stands for is started the same
/// way. Both are made by one function (`compile::engine_command`), so the
/// only inputs to a run's command are these: the arguments (format, shell
/// escape, halt on error, output directory, job name, the `[fonts]` first
/// line, the main file), the root (the working directory), the format
/// directory (`TEXFORMATS`), the client's font formats, and the
/// environment the host itself runs in, which every child inherits: read
/// confinement (`FLASHTEX_CONFINE_READS` and its roots, Live Share's
/// included), `openout_any`/`openin_any`, `shell_escape`, the `TEXMF*` trees.
/// The compile's external-tools setting is part of the key too, though the
/// engine run does not read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub argv: Vec<String>,
    pub root: PathBuf,
    pub format_dir: PathBuf,
    /// `FLASHTEX_DISPLAY_LIST_FONT_FORMATS`.
    pub font_formats: String,
    pub external_tools: Option<String>,
    /// The host's environment when the command was made, sorted.
    pub env: Vec<(std::ffi::OsString, std::ffi::OsString)>,
}

/// The host's environment, sorted: what a child inherits.
pub fn host_env() -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    let mut v: Vec<_> = std::env::vars_os().collect();
    v.sort();
    v
}

/// A file's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ident {
    size: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
    ino: u64,
    dev: u64,
}

fn ident(p: &Path) -> Option<Ident> {
    let m = std::fs::metadata(p).ok()?;
    Some(Ident {
        size: m.len(),
        mtime: (m.mtime(), m.mtime_nsec()),
        ctime: (m.ctime(), m.ctime_nsec()),
        ino: m.ino(),
        dev: m.dev(),
    })
}

/// The main file's first line, as `parse_first_line_of` reads it (to the
/// first newline, line ends removed); `None` when it cannot be read.
fn first_line(p: &Path) -> Option<Vec<u8>> {
    let mut l = vec![];
    BufReader::new(File::open(p).ok()?)
        .read_until(b'\n', &mut l)
        .ok()?;
    while matches!(l.last(), Some(b'\n' | b'\r')) {
        l.pop();
    }
    Some(l)
}

/// What a run reads of the outside before the spare's wait, and could see
/// changed after it.
#[derive(Clone, Debug, PartialEq)]
pub struct Inputs {
    fmt: Option<Ident>,
    /// The main file's first line when the engine resolves the main file
    /// (the first argument is a file name), else `None`.
    main: Option<Option<Vec<u8>>>,
}

impl Inputs {
    /// Read now. `None` when a spare could not stand for the run: the
    /// format file is not there, or the main file is not a `.tex` file in
    /// the root (kpathsea would look for it elsewhere).
    pub fn read(fmt: &Path, root: &Path, main: Option<&str>) -> Option<Inputs> {
        let fmt = Some(ident(fmt)?);
        let main = match main {
            None => None,
            Some(m) => {
                let p = root.join(m);
                if !m.ends_with(".tex") || !p.is_file() {
                    return None;
                }
                Some(first_line(&p))
            }
        };
        Some(Inputs { fmt, main })
    }

    /// Whether a run started now would read what `self` read, as far as
    /// what it does up to the spare's wait goes.
    fn same_as(&self, now: &Inputs) -> bool {
        let lines_agree = match (&self.main, &now.main) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                let amp = |l: &Option<Vec<u8>>| l.as_ref().is_some_and(|l| l.starts_with(b"%&"));
                a == b || (a.is_some() && b.is_some() && !amp(a) && !amp(b))
            }
            _ => false,
        };
        self.fmt.is_some() && self.fmt == now.fmt && lines_agree
    }
}

/// A started engine run: the child, the socket its display list comes
/// through, and the reader of its standard error.
pub struct Started {
    pub child: Child,
    pub listener: Option<UnixListener>,
    pub sock: PathBuf,
    pub err: JoinHandle<String>,
}

impl Started {
    /// End it: its process group killed, reaped in the background.
    pub fn end(self) {
        super::proc::kill_group(self.child.id());
        let _ = std::fs::remove_file(&self.sock);
        let Started { mut child, err, .. } = self;
        std::thread::spawn(move || {
            let _ = child.wait();
            let _ = err.join();
        });
    }
}

/// `FLASHTEX_UNICODE_SPARE_DEBUG=1`: the host says on its standard error
/// what became of each spare.
fn debug() -> bool {
    std::env::var_os("FLASHTEX_UNICODE_SPARE_DEBUG").is_some()
}

/// A run waiting for its go-ahead.
pub struct Spare {
    key: Key,
    inputs: Inputs,
    run: Started,
    go: File,
    born: Instant,
}

/// The spares of every connection of the host: at most one a document (the
/// root and main file) and at most `cap` in all
/// (`FLASHTEX_UNICODE_SPARE_CAP`, default 2; 0 turns spares off). A new
/// spare ends the document's other spare and, at the cap, the oldest one;
/// its connection finds it ended when it next looks.
pub struct Slots {
    cap: usize,
    live: std::sync::Mutex<Vec<Slot>>,
}

struct Slot {
    pid: u32,
    doc: (PathBuf, String),
}

impl Slots {
    pub fn from_env() -> Slots {
        Slots::new(
            std::env::var("FLASHTEX_UNICODE_SPARE_CAP")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
        )
    }

    pub fn new(cap: usize) -> Slots {
        Slots {
            cap,
            live: std::sync::Mutex::new(vec![]),
        }
    }

    /// Spare `pid` of `doc` is waiting: make room for it. Only a process
    /// that is still listed is killed, and a connection takes its spare off
    /// the list ([`Slots::release`]) before it reaps or starts it, so the
    /// pid killed is never another process's.
    fn admit(&self, pid: u32, doc: (PathBuf, String)) {
        let mut live = self.live.lock().unwrap_or_else(|p| p.into_inner());
        live.retain(|s| {
            let same = s.doc == doc;
            if same {
                super::proc::kill_group(s.pid);
            }
            !same
        });
        while !live.is_empty() && live.len() >= self.cap {
            let old = live.remove(0);
            super::proc::kill_group(old.pid);
        }
        live.push(Slot { pid, doc });
    }

    /// Spare `pid` is no longer waiting (taken or ended).
    fn release(&self, pid: u32) {
        let mut live = self.live.lock().unwrap_or_else(|p| p.into_inner());
        live.retain(|s| s.pid != pid);
    }

    /// The number of spares waiting.
    pub fn len(&self) -> usize {
        self.live.lock().unwrap_or_else(|p| p.into_inner()).len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A pipe whose two ends are closed on `exec`.
fn pipe_cloexec() -> std::io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: `fds` has room for the two descriptors pipe(2) writes.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    for fd in fds {
        // SAFETY: a descriptor pipe(2) just returned.
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    // SAFETY: both descriptors are new and owned by nothing else.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// Make `cmd` a spare's: it inherits the read end of a new pipe, named in
/// [`SPARE_ENV`]; the write end, returned, is the go-ahead.
pub fn arm(cmd: &mut Command, fonts: bool) -> std::io::Result<(File, OwnedFd)> {
    let (read, write) = pipe_cloexec()?;
    let fd = read.as_raw_fd();
    cmd.env(SPARE_ENV, fd.to_string());
    if fonts {
        cmd.env(SPARE_FONTS_ENV, "1");
    }
    // SAFETY: fcntl(2) is async-signal-safe, and only the child's copy of
    // this descriptor changes.
    unsafe {
        cmd.pre_exec(move || {
            if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok((File::from(write), read))
}

/// A connection's spare, if any, and whether it may have one.
pub struct Spares {
    enabled: bool,
    /// The connection's performance mode allows spares (not Low Memory).
    profile_allows: bool,
    slots: Arc<Slots>,
    ttl: Duration,
    spare: Option<Spare>,
    /// The last run looked a font up by name ([`FONTS_HINT`]).
    fonts: bool,
    /// Runs started on this connection, spares included.
    runs: u64,
}

impl Spares {
    /// On unless `FLASHTEX_UNICODE_SPARES=0`, the host's cap is 0 or the
    /// mode is Low Memory; the time to live from
    /// `FLASHTEX_UNICODE_SPARE_TTL` (seconds, default 120).
    pub fn new(slots: Arc<Slots>, mode: flashtex_engine::profile::Mode) -> Spares {
        let enabled =
            std::env::var("FLASHTEX_UNICODE_SPARES").map_or(true, |v| v != "0") && slots.cap > 0;
        let ttl = std::env::var("FLASHTEX_UNICODE_SPARE_TTL")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(120);
        Spares {
            enabled,
            profile_allows: allows(mode),
            slots,
            ttl: Duration::from_secs(ttl),
            spare: None,
            fonts: false,
            runs: 0,
        }
    }

    /// The number of the next run started on this connection.
    pub fn next_run(&mut self) -> u64 {
        self.runs += 1;
        self.runs
    }

    /// A run ended, having said `stderr`: its hints.
    pub fn hints(&mut self, stderr: &str) {
        self.fonts = stderr.lines().any(|l| l == FONTS_HINT);
    }

    /// Whether the next spare is to build the fonts' catalog.
    pub fn fonts(&self) -> bool {
        self.fonts
    }

    pub fn enabled(&self) -> bool {
        self.enabled && self.profile_allows
    }

    /// The connection's performance mode changed (`PROFILE`): Low Memory
    /// keeps no spare, and ends the one there is.
    pub fn set_mode(&mut self, mode: flashtex_engine::profile::Mode) {
        self.profile_allows = allows(mode);
        if !self.profile_allows {
            self.clear();
        }
    }

    /// `HELLO.profile` and the `PROFILE` reply's knobs for `mode`
    /// (protocol §6.9; informative).
    pub fn profile_json(&self, mode: flashtex_engine::profile::Mode) -> Json {
        obj([
            ("mode", js(mode.name())),
            ("spare", Json::Bool(self.enabled && allows(mode))),
            ("spare_ttl_ms", Json::Int(self.ttl.as_millis() as i64)),
            ("spare_cap", Json::Int(self.slots.cap as i64)),
        ])
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// The spare, if it stands for a run of `key` that would read `inputs`
    /// now; any other spare is ended.
    pub fn take(&mut self, key: &Key, inputs: Option<&Inputs>) -> Option<(Started, File)> {
        let mut s = self.spare.take()?;
        // off the host's list first: from here only this connection ends it
        self.slots.release(s.run.child.id());
        let alive = matches!(s.run.child.try_wait(), Ok(None));
        let fits = alive
            && s.key == *key
            && s.born.elapsed() < self.ttl
            && inputs.is_some_and(|i| s.inputs.same_as(i));
        if debug() {
            let why = if !alive {
                "it has ended (or another spare took its place)"
            } else if s.key != *key {
                "another command"
            } else if s.born.elapsed() >= self.ttl {
                "its time to live is over"
            } else if fits {
                "taken"
            } else {
                "the format or the main file's first line changed"
            };
            eprintln!("flashtex-host-unicode: spare {}: {why}", s.run.child.id());
        }
        if fits {
            Some((s.run, s.go))
        } else {
            s.run.end();
            None
        }
    }

    /// Keep `run` as the spare for `key` of document `doc` (its root and
    /// main file), ending the one there was.
    pub fn put(
        &mut self,
        key: Key,
        inputs: Inputs,
        doc: (PathBuf, String),
        run: Started,
        go: File,
    ) {
        self.clear();
        self.slots.admit(run.child.id(), doc);
        self.spare = Some(Spare {
            key,
            inputs,
            run,
            go,
            born: Instant::now(),
        });
    }

    /// End the spare, if any.
    pub fn clear(&mut self) {
        if let Some(s) = self.spare.take() {
            self.slots.release(s.run.child.id());
            s.run.end();
        }
    }

    /// End the spare if it has outlived its time to live.
    pub fn expire(&mut self) {
        if self
            .spare
            .as_ref()
            .is_some_and(|s| s.born.elapsed() >= self.ttl)
        {
            self.clear();
        }
    }
}

impl Drop for Spares {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Whether a performance mode keeps spares: not Low Memory (a spare holds
/// about 200 MB while it waits).
fn allows(mode: flashtex_engine::profile::Mode) -> bool {
    mode != flashtex_engine::profile::Mode::LowMemory
}

/// Give a taken spare its go-ahead: the run carries on. `false` if it
/// could not be told (it has ended).
pub fn go(mut go: File) -> bool {
    go.write_all(b"g").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A process in a group of its own, standing for a spare.
    fn sleeper() -> Child {
        Command::new("sleep")
            .arg("60")
            .process_group(0)
            .spawn()
            .unwrap()
    }

    fn ended(c: &mut Child) -> bool {
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(10) {
            if matches!(c.try_wait(), Ok(Some(_))) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn one_spare_a_document_and_a_cap_in_all() {
        let slots = Slots::new(2);
        let doc = |n: &str| (PathBuf::from("/p"), n.to_string());
        let mut a = sleeper();
        let mut a2 = sleeper();
        let mut b = sleeper();
        let mut c = sleeper();
        slots.admit(a.id(), doc("a.tex"));
        slots.admit(a2.id(), doc("a.tex"));
        assert!(ended(&mut a), "a document's second spare ends its first");
        assert_eq!(slots.len(), 1);
        slots.admit(b.id(), doc("b.tex"));
        assert_eq!(slots.len(), 2);
        slots.admit(c.id(), doc("c.tex"));
        assert!(ended(&mut a2), "at the cap the oldest ends");
        assert_eq!(slots.len(), 2);
        // a released spare is never killed by the list
        slots.release(b.id());
        let mut d = sleeper();
        slots.admit(d.id(), doc("d.tex"));
        assert!(matches!(b.try_wait(), Ok(None)), "released: not the list's");
        assert!(matches!(c.try_wait(), Ok(None)), "under the cap");
        assert_eq!(slots.len(), 2);
        for p in [&mut b, &mut c, &mut d] {
            let _ = p.kill();
            let _ = p.wait();
        }
        let none = Slots::new(0);
        assert_eq!(none.cap, 0);
    }

    #[test]
    fn first_lines_decide_only_with_a_percent_ampersand() {
        let i = |l: Option<&[u8]>| Inputs {
            fmt: Some(Ident {
                size: 1,
                mtime: (1, 0),
                ctime: (1, 0),
                ino: 1,
                dev: 1,
            }),
            main: Some(l.map(<[u8]>::to_vec)),
        };
        let a = i(Some(b"\\documentclass{article}"));
        assert!(a.same_as(&i(Some(b"\\documentclass{book}"))));
        assert!(!a.same_as(&i(Some(b"%&xetex"))));
        assert!(!i(Some(b"%&xetex")).same_as(&i(Some(b"%&xelatex"))));
        assert!(i(Some(b"%&xetex")).same_as(&i(Some(b"%&xetex"))));
        assert!(!a.same_as(&i(None)), "a main file that went");
        let mut b = a.clone();
        b.fmt = Some(Ident {
            size: 2,
            ..b.fmt.unwrap()
        });
        assert!(!a.same_as(&b), "another format file");
        let no_main = Inputs {
            main: None,
            ..a.clone()
        };
        assert!(no_main.same_as(&no_main.clone()));
        assert!(!no_main.same_as(&a));
    }
}
