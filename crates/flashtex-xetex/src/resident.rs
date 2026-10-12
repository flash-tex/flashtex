//! The resident engine: runs of the Unicode engine in its host's own
//! process (docs/design/xetex/INCREMENTAL.md §5 steps 2 and 3; DESIGN.md
//! §5.1), one after another, instead of a process per run.
//!
//! * **The format is loaded once.** A run's work before it reads the
//!   document -- `initialize` and `init_prim` over XeTeX's Unicode-sized
//!   `eqtb`, the terminal and the first line, the undump of the format --
//!   depends only on the command line, the format file and the session's
//!   clock. The first run keeps a checkpoint of the engine where the format
//!   is loaded (`changes/resident.ch`'s `flashtex_format_ready`: the word
//!   space with the scalar globals, and the state outside it that a
//!   checkpoint saves, `state.rs`), and a later run of the same command
//!   line from the same, unchanged format file restores it and runs the
//!   rest of the main program (`run_after_format`). Anything else (another
//!   job, another format file, a changed one) is a run from the start that
//!   keeps a new point.
//! * **S₀.** A run from the start or from the format point takes S₀ at the
//!   document's start (`crate::ckpt`, `changes/checkpoint.ch`). A later run
//!   whose S₀ key still holds -- nothing the run read before S₀ changed: an
//!   edit after the preamble -- restores S₀ and runs only the document's
//!   body (`resume_main_control`); otherwise it runs from the format point
//!   and takes a new S₀.
//! * **Pages.** After S₀, a run takes a checkpoint after each shipout
//!   (`crate::ckpt::PageCk`); a compile whose S₀ holds restarts from the
//!   newest one that what changed leaves valid ([`Engine::restart_page`]),
//!   sends the pages before it again and runs on to the end.
//! * **Exits return.** `final_end` and the other exits unwind to [`Engine::run`]
//!   with [`EngineExit`] instead of ending the process (`system::exit_process`).
//! * **The terminal is captured** (no standard input: a run that asks for a
//!   line reads the end of the file, as a child whose standard input is
//!   `/dev/null` does), and so are the output's reports.
//! * **The clock is the session's** ([`Engine::clock`]): `\time`, `\day`,
//!   `\month`, `\year`, the random seed and the PDF's dates, as the pdfTeX
//!   engine's resident runs pin it (DESIGN.md §4.5), so that two runs of a
//!   session read the same clock.
//! * **A run stops when its host asks** (a newer compile, a cancel): the
//!   cancel flag is read between commands, at every macro call, at every
//!   line of input and at every page (`flashtex_poll`), and the run
//!   unwinds with [`Cancelled`].
//! * **The fonts' catalog** (the installed fonts' names, which a lookup by
//!   name reads) is kept across runs by the host ([`Fonts`]) while the font
//!   directories are unchanged.
//!
//! The engine's runtime (`flashtex_engine::system`: the run's
//! configuration, the resolver, the read journal) is still per process (the
//! journal per thread), so a process runs one resident engine at a time, on
//! one thread; R2 makes it per engine (PLAN.md §3.3).

use crate::ckpt::{PageCk, RunState, Taken, REQ_LOOKUP, REQ_PAGE, S0};
use crate::fontmgr::FontCatalog;
use crate::generated::Globals;
use crate::system::{self, AlphaFile, ByteFile, FileVisit, WordFile};
use flashtex_engine::arena::CheckpointId;
use flashtex_engine::resolver::Format;
use flashtex_engine::system::{ReadLog, StatSig};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The payload a resident engine's run unwinds with when the program ends:
/// its exit status.
pub struct EngineExit(pub i32);

/// The payload a resident engine's run unwinds with when its host asked it
/// to stop.
pub struct Cancelled;

/// Where a resident engine's runs get the installed fonts' catalog (the
/// host's, which keeps it across runs).
pub trait Fonts: Send {
    /// The catalog for a lookup by name.
    fn catalog(&mut self) -> Arc<FontCatalog>;
    /// The state of what a catalog made now is made from (the font
    /// directories): the same value, the same catalog.
    fn dirs(&mut self) -> [u64; 2];
}

/// The catalog made at every lookup, from the font directories as they are.
struct Unkept;

impl Fonts for Unkept {
    fn catalog(&mut self) -> Arc<FontCatalog> {
        Arc::new(FontCatalog::system_cached(None))
    }
    fn dirs(&mut self) -> [u64; 2] {
        // nothing watched: never the same
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        [t, !t]
    }
}

/// What a resident engine's run is (`Host::resident`).
pub struct Resident {
    /// Set by the host: the run stops at its next poll.
    pub cancel: Arc<AtomicBool>,
    /// The format directory, searched first (a child's `TEXFORMATS=DIR:`).
    format_dir: Option<PathBuf>,
    /// Keep the point where the format is loaded.
    keep_point: bool,
    /// The point kept by this run, and the format file it loaded with its
    /// identity then.
    point: Option<(CheckpointId, RunState, Option<FormatFile>)>,
    /// Take S₀ (`crate::ckpt`).
    pub(crate) want_s0: bool,
    /// S₀ as this run took it, or why it could not.
    pub(crate) s0: Option<Result<Taken, String>>,
    /// Take page checkpoints (after S₀: this run's, or the one it started
    /// from).
    pub(crate) pages_on: bool,
    /// The page checkpoints this run took.
    pub(crate) pages: Vec<PageCk>,
    /// The output files the runs opened, in order (`system`'s
    /// `open_output_file`).
    pub(crate) opens: Vec<String>,
    /// The output files this run cut back or created, with what they held
    /// before (`None`: no file): a cancelled run puts them back, so that
    /// the next compile reads the `.aux` the last complete run wrote, as
    /// S₀'s key and the page checkpoints expect.
    pub(crate) guard: Vec<(String, Option<Vec<u8>>)>,
    /// A run from S₀ or a page: the last complete run's later state, for
    /// the convergence test (`crate::conv`).
    pub(crate) conv: Option<crate::conv::Old>,
    fonts: Box<dyn Fonts>,
}

impl Resident {
    /// Keep what output file `path` holds before this run first writes it
    /// (not the PDF, which a run writes at its end, nor the log, which no
    /// run reads).
    pub(crate) fn guard_out(&mut self, path: &str) {
        let p = Path::new(path);
        if p.extension().is_some_and(|e| e == "pdf" || e == "log")
            || self.guard.iter().any(|(q, _)| q == path)
        {
            return;
        }
        self.guard.push((path.to_string(), std::fs::read(p).ok()));
    }

    /// The fonts' catalog for a lookup by name.
    pub(crate) fn font_catalog(&mut self) -> Arc<FontCatalog> {
        self.fonts.catalog()
    }

    /// The font directories' state now (`Fonts::dirs`).
    pub(crate) fn font_dirs(&mut self) -> [u64; 2] {
        self.fonts.dirs()
    }
}

impl Globals {
    /// `changes/resident.ch`: the format is loaded and closed. A resident
    /// engine's run that is to keep the point takes a checkpoint here.
    pub fn flashtex_format_ready(&mut self) {
        if !self.host.resident.as_ref().is_some_and(|r| r.keep_point) {
            return;
        }
        let id = self.checkpoint();
        let st = RunState::of(self);
        // the format just loaded (`input_path` keeps the path it opened)
        let fmt = self
            .host
            .full_name_of_file
            .clone()
            .and_then(|p| Ident::of(&p).map(|i| (p, i)));
        if let Some(r) = self.host.resident.as_mut() {
            r.point = Some((id, st, fmt));
        }
    }

    /// `changes/resident.ch`: between two commands and at each macro call.
    #[inline]
    pub fn flashtex_poll(&mut self) {
        self.resident_poll();
    }

    /// A resident engine's run stops here (unwinds with [`Cancelled`]) when
    /// its host asked it to.
    #[inline]
    pub fn resident_poll(&mut self) {
        if let Some(r) = self.host.resident.as_ref() {
            if r.cancel.load(Ordering::Relaxed) {
                std::panic::resume_unwind(Box::new(Cancelled));
            }
        }
    }

    /// A format a resident engine's run finds in its format directory.
    pub(crate) fn resident_format(&self, format: Format, name: &str) -> Option<String> {
        if format != Format::Fmt || name.contains('/') {
            return None;
        }
        let dir = self.host.resident.as_ref()?.format_dir.as_ref()?;
        let p = dir.join(name);
        p.is_file().then(|| p.to_string_lossy().into_owned())
    }
}

/// The format file a run loaded, and its identity then.
type FormatFile = (String, Ident);

/// A file's identity: size, modification and status-change times, inode,
/// device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ident(u64, i128, i128, u64, u64);

impl Ident {
    fn of(path: &str) -> Option<Ident> {
        let m = std::fs::metadata(path).ok()?;
        let s = flashtex_engine::os::file_stat(&m);
        Some(Ident(s.size, s.mtime_ns, s.ctime_ns, s.ino, s.dev))
    }
}

/// What the state after the format load depends on: the command line as
/// configured (`flashtex_engine::system::run`, which includes the main
/// file's `%&` line), the first line, the directory and the format file.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PointKey {
    run: String,
    first_line: Option<Vec<u8>>,
    no_pdf: bool,
    root: PathBuf,
    format_dir: Option<PathBuf>,
}

struct Point {
    key: PointKey,
    id: CheckpointId,
    state: RunState,
    format: Option<FormatFile>,
}

impl Point {
    /// The format file is the one a run would load now, unchanged.
    fn format_holds(&self) -> bool {
        let Some((path, id)) = &self.format else {
            return false;
        };
        // found in the format directory then: still there; found on the
        // default path: none in the format directory now
        if let Some(dir) = &self.key.format_dir {
            let name = Path::new(path).file_name();
            let in_dir = name.map(|n| dir.join(n));
            match &in_dir {
                Some(p) if p.to_string_lossy() == path.as_str() => {}
                Some(p) if p.is_file() => return false,
                _ => {}
            }
        }
        Ident::of(path).is_some_and(|now| now == *id)
    }
}

/// One run's request.
pub struct Spec {
    /// The command line, as xelatex would be run (`argv[0]` is `xelatex`).
    pub argv: Vec<String>,
    /// The working directory.
    pub root: PathBuf,
    /// The format directory, searched first (`None`: the default path).
    pub format_dir: Option<PathBuf>,
    /// Where the display list goes (`None`: no display list).
    pub dl: Option<Box<dyn Write + Send>>,
    /// The font formats whose programs travel in the display list (`None`:
    /// all).
    pub font_formats: Option<HashSet<String>>,
    /// The host sets it to stop the run.
    pub cancel: Arc<AtomicBool>,
    /// Keep nothing after the run (Low Memory): the engine's memory goes
    /// back, and the next run starts from the start.
    pub lean: bool,
    /// A preview: the PDF's streams are stored, not compressed.
    pub preview: bool,
}

/// Where a run started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Start {
    /// From the start: `initialize`, the format's undump.
    #[default]
    Fresh,
    /// From the state after the format load.
    Format,
    /// From S₀: the document's body only.
    S0,
    /// From the page checkpoint after this many pages.
    Page(usize),
}

impl Start {
    pub fn name(self) -> &'static str {
        match self {
            Start::Fresh => "fresh",
            Start::Format => "format",
            Start::S0 => "s0",
            Start::Page(_) => "page",
        }
    }

    /// The run continues an earlier run's document: from S₀ or a page.
    fn in_body(self) -> bool {
        matches!(self, Start::S0 | Start::Page(_))
    }
}

/// How a run ended.
#[derive(Debug, Default)]
pub struct Report {
    /// The exit status (`None`: the run failed: see `failure`).
    pub exit_code: Option<i32>,
    /// The host asked the run to stop.
    pub cancelled: bool,
    /// The run broke off (an engine panic or the host's failure to set it
    /// up): its message.
    pub failure: Option<String>,
    /// What the run wrote to its terminal.
    pub terminal: Vec<u8>,
    /// The output's reports (`FlashTeX output: ...`).
    pub reports: Vec<String>,
    /// Where the run started.
    pub start: Start,
    /// Why S₀ was not used (`start` is not S₀ though one was kept), or why
    /// the run took none.
    pub s0_note: Option<String>,
    /// How long deciding where to start took (S₀'s key check, the changed
    /// files and the restart page).
    pub check_ms: f64,
    /// A run from a page checkpoint: the pages before it, not re-typeset.
    pub restart_page: Option<usize>,
}

/// A resident engine: the engine kept between runs, the point where its
/// format is loaded, S₀, and the page checkpoints after it.
pub struct Engine {
    g: Option<Box<Globals>>,
    point: Option<Point>,
    s0: Option<S0>,
    /// The page checkpoints after S₀, in page order.
    pages: Vec<PageCk>,
    /// The last run's read journal (its prefix up to where the run
    /// started is the earlier runs'), with the user's files' contents.
    journal: Option<ReadLog>,
    /// The output files the runs opened, in order (as `journal`).
    opens: Vec<String>,
    /// The user's input files as the last run read them.
    texts: HashMap<String, Arc<Vec<u8>>>,
    /// The last run's terminal: a page checkpoint keeps its length only.
    terminal: Vec<u8>,
    /// The session's clock (DESIGN.md §4.5).
    pub clock: (i64, i32),
    fonts: Option<Box<dyn Fonts>>,
    /// Keep and use the point after the format load (`FLASHTEX_UNICODE_FORMAT_POINT=0`
    /// turns it off: every run loads the format).
    pub keep_point: bool,
    /// Take and use S₀ (`FLASHTEX_UNICODE_S0=0` turns it off).
    pub keep_s0: bool,
    /// Take and use page checkpoints (`FLASHTEX_UNICODE_PAGES=0` turns them
    /// off).
    pub keep_pages: bool,
    /// The output's font map index and font programs, kept across runs.
    font_cache: Arc<crate::out::fonts::FontCache>,
    /// The convergence measurement (`crate::conv`): the last complete
    /// run's pages, and the `eqtb` pointer entries of each checkpoint
    /// restored.
    last_pages: Vec<Arc<crate::out::doc::Built>>,
    eqtb_pointers: HashMap<CheckpointId, Arc<Vec<i32>>>,
}

/// Resets every file of the engine: closed, as at the start of a run.
struct CloseFiles;

impl FileVisit for CloseFiles {
    fn alpha(&mut self, f: &mut AlphaFile) {
        *f = AlphaFile::default();
    }
    fn byte(&mut self, f: &mut ByteFile) {
        *f = ByteFile::default();
    }
    fn word(&mut self, f: &mut WordFile) {
        *f = WordFile::default();
    }
}

fn env_on(name: &str) -> bool {
    std::env::var(name).map_or(true, |v| v != "0")
}

/// The journal `j` cut to its first `n` (files, lookups, outputs), to go on
/// recording into (the pdfTeX engine's `incr::truncate_journal`): every
/// path it keeps is marked read, so that a later read of one is listed
/// again; the lookups' directories have no signature, so the next check
/// makes them again.
fn truncate_journal(j: &ReadLog, n: (usize, usize, usize)) -> ReadLog {
    let mut out = ReadLog::keeping_content();
    out.files = j.files[..n.0.min(j.files.len())].to_vec();
    out.lookups = j.lookups[..n.1.min(j.lookups.len())].to_vec();
    out.outputs = j.outputs[..n.2.min(j.outputs.len())].to_vec();
    out.barriers = j.barriers.clone();
    out.dirs = j
        .dirs
        .iter()
        .map(|(d, _)| (d.clone(), StatSig::default()))
        .collect();
    let paths: Vec<String> = out.files.iter().map(|f| f.path.clone()).collect();
    for p in &paths {
        out.mark_seen(p);
    }
    out
}

/// The length of the common prefix of `a` and `b`.
fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    const BLOCK: usize = 256;
    let n = a.len().min(b.len());
    let mut i = 0;
    while i + BLOCK <= n && a[i..i + BLOCK] == b[i..i + BLOCK] {
        i += BLOCK;
    }
    i + a[i..n]
        .iter()
        .zip(&b[i..n])
        .take_while(|(x, y)| x == y)
        .count()
}

/// What changed since the last run, of what its document's body read: each
/// changed file with the first byte that differs (for the user's files,
/// whose contents are kept; `None`: somewhere), and the first lookup (by its
/// index in the journal) that finds something else now.
struct Changes {
    files: HashMap<String, Option<u64>>,
    lookup: Option<usize>,
}

impl Engine {
    /// A resident engine whose session clock is `clock` (`None`: now) and
    /// whose runs get the fonts' catalog from `fonts` (`None`: made at every
    /// lookup).
    pub fn new(clock: Option<(i64, i32)>, fonts: Option<Box<dyn Fonts>>) -> Engine {
        let clock = clock.unwrap_or_else(|| {
            let d = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (d.as_secs() as i64, d.subsec_micros() as i32)
        });
        Engine {
            g: None,
            point: None,
            s0: None,
            pages: vec![],
            journal: None,
            opens: vec![],
            texts: HashMap::new(),
            terminal: vec![],
            clock,
            fonts,
            keep_point: env_on("FLASHTEX_UNICODE_FORMAT_POINT"),
            keep_s0: env_on("FLASHTEX_UNICODE_S0"),
            keep_pages: env_on("FLASHTEX_UNICODE_PAGES"),
            font_cache: Arc::default(),
            last_pages: vec![],
            eqtb_pointers: HashMap::new(),
        }
    }

    /// Forget the engine, its point, S₀ and the pages (memory back; the
    /// next run starts from the start).
    pub fn clear(&mut self) {
        self.pages.clear();
        self.s0 = None;
        self.point = None;
        self.journal = None;
        self.opens.clear();
        self.g = None;
    }

    /// Where this run can start, and why not from S₀ when one is kept.
    fn start_for(
        &mut self,
        key: &PointKey,
        first_line: &[u8],
        dl_on: bool,
        now: Option<&HashMap<String, Arc<Vec<u8>>>>,
    ) -> (Start, Option<String>) {
        let point_ok = self.keep_point
            && self.g.is_some()
            && self
                .point
                .as_ref()
                .is_some_and(|p| p.key == *key && p.format_holds());
        if !point_ok {
            self.s0 = None;
            self.pages.clear();
            return (Start::Fresh, None);
        }
        let Some(s0) = self.s0.as_mut() else {
            self.pages.clear();
            return (Start::Format, None);
        };
        let why = match s0.key.check_refresh(self.clock, first_line) {
            Err(why) => Some(why),
            Ok(()) if s0.dl_on != dl_on => {
                Some("the display list's source map is made in one run, not the other".into())
            }
            Ok(()) => match s0.font_dirs {
                None => None,
                Some(d) => match self.fonts.as_mut().map(|f| f.dirs()) {
                    Some(now) if now == d => None,
                    Some(_) => Some("the font directories changed".to_string()),
                    None => Some("the font directories are not watched".to_string()),
                },
            },
        };
        if let Some(w) = why {
            self.s0 = None;
            self.pages.clear();
            return (Start::Format, Some(w));
        }
        match now.and_then(|now| self.restart_page(now)) {
            Some(i) => (Start::Page(i), None),
            None => (Start::S0, None),
        }
    }

    /// The user's files, as they are now, of those the last run read
    /// (its journal's contents) or kept (`texts`).
    fn texts_now(&self) -> HashMap<String, Arc<Vec<u8>>> {
        let mut paths: Vec<&str> = self.texts.keys().map(String::as_str).collect();
        if let Some(j) = &self.journal {
            paths.extend(
                j.files
                    .iter()
                    .filter(|f| f.content.is_some())
                    .map(|f| f.path.as_str()),
            );
        }
        paths.sort_unstable();
        paths.dedup();
        paths
            .into_iter()
            .filter_map(|p| Some((p.to_string(), Arc::new(std::fs::read(p).ok()?))))
            .collect()
    }

    /// What changed of what the body read in the last run (after S₀):
    /// `now` is the user's files as they are.
    fn changes(&self, now: &HashMap<String, Arc<Vec<u8>>>) -> Option<Changes> {
        let j = self.journal.as_ref()?;
        let s0 = self.s0.as_ref()?;
        let mut files: HashMap<String, Option<u64>> = HashMap::new();
        let mut seen = HashSet::new();
        // every file read since S₀, and those open at S₀ (the main file)
        let open_at_s0 = s0.rec.files.iter().filter_map(|f| match &f.stream {
            crate::system::Stream::In { path, .. } => Some(path.as_str()),
            _ => None,
        });
        let read = j.files[s0.reads.0.min(j.files.len())..]
            .iter()
            .map(|f| f.path.as_str());
        for p in open_at_s0.chain(read) {
            if !seen.insert(p) {
                continue;
            }
            let old = self.texts.get(p).cloned().or_else(|| {
                j.files
                    .iter()
                    .find(|f| f.path == p)
                    .and_then(|f| f.content.clone())
            });
            let change = match old {
                // the user's file: where it changed
                Some(old) => match now.get(p) {
                    Some(new) if new[..] == old[..] => None,
                    Some(new) => Some(Some(common_prefix(&old, new) as u64)),
                    None => Some(None),
                },
                // another file: whether it changed (its signature, else its
                // content's hash)
                None => {
                    let f = j.files.iter().find(|f| f.path == p)?;
                    let same = StatSig::of(p).is_some_and(|s| s == f.stat)
                        || flashtex_engine::hashcache::hash_file(p).ok() == Some(f.hash);
                    (!same).then_some(None)
                }
            };
            if let Some(c) = change {
                files.insert(p.to_string(), c);
            }
        }
        // the first lookup since S₀ that finds something else now (of
        // those before the newest page checkpoint: the later ones are
        // made again whatever the restart)
        let upto = self.pages.iter().map(|c| c.reads.1).max().unwrap_or(0);
        let lookup = j.lookups[s0.reads.1.min(j.lookups.len())
            ..upto
                .min(j.lookups.len())
                .max(s0.reads.1.min(j.lookups.len()))]
            .iter()
            .position(|l| flashtex_engine::system::lookup_again(l) != l.found)
            .map(|i| s0.reads.1 + i);
        Some(Changes { files, lookup })
    }

    /// The newest page checkpoint the edit leaves valid: everything the run
    /// had read by then is as it is now (a file still open there, read up
    /// to before its first changed byte), no lookup it made finds another
    /// file, and its output files are as the runs left them and not opened
    /// again later.
    fn restart_page(&self, now: &HashMap<String, Arc<Vec<u8>>>) -> Option<usize> {
        if self.pages.is_empty() {
            return None;
        }
        let ch = self.changes(now)?;
        let j = self.journal.as_ref()?;
        'pages: for (i, ck) in self.pages.iter().enumerate().rev() {
            if ch.lookup.is_some_and(|l| l < ck.reads.1) {
                continue;
            }
            for (p, first) in &ch.files {
                let reads: Vec<&flashtex_engine::system::FileRead> = j.files
                    [..ck.reads.0.min(j.files.len())]
                    .iter()
                    .filter(|f| f.path == *p)
                    .collect();
                if reads.is_empty() {
                    continue;
                }
                let open = ck.rec.files.iter().find_map(|f| match &f.stream {
                    crate::system::Stream::In { path, offset } if path == p => Some(*offset),
                    _ => None,
                });
                let ok = match (open, first) {
                    (Some(off), Some(d)) => {
                        off <= *d && reads.len() == 1 && reads[0].closed_at.is_none()
                    }
                    _ => false,
                };
                if !ok {
                    continue 'pages;
                }
            }
            // no external command between S₀ and it (`\write18` in the body:
            // a run from scratch runs it again, and what it does is no
            // input the journal knows)
            if self
                .s0
                .as_ref()
                .is_some_and(|s0| ck.rec.effects > s0.rec.effects)
            {
                continue;
            }
            // the output files: open there, as the runs left them; none
            // opened before it opened again after it
            let before = &self.opens[..ck.opens.min(self.opens.len())];
            let after = &self.opens[ck.opens.min(self.opens.len())..];
            if after.iter().any(|p| before.contains(p)) {
                continue;
            }
            for f in &ck.rec.files {
                if let crate::system::Stream::Out { path, .. } = &f.stream {
                    if flashtex_engine::system::outside_change(path).is_some() {
                        continue 'pages;
                    }
                }
            }
            return Some(i);
        }
        None
    }

    /// Run the engine once, as `spec` asks.
    pub fn run(&mut self, spec: Spec) -> Report {
        if let Err(e) = std::env::set_current_dir(&spec.root) {
            return Report {
                failure: Some(format!("{}: {e}", spec.root.display())),
                ..Report::default()
            };
        }
        let c = crate::driver::command(&spec.argv);
        flashtex_engine::system::configure(c.options);
        // a directory made since the last run is searched (#1493)
        flashtex_engine::system::refresh_disk_dirs();
        let first_line = (!c.args.is_empty()).then(|| system::command_line(&c.args));
        let line = first_line.clone().unwrap_or_default();
        let key = PointKey {
            run: format!("{:?}", flashtex_engine::system::run()),
            first_line: first_line.clone(),
            no_pdf: c.no_pdf,
            root: spec.root.clone(),
            format_dir: spec.format_dir.clone(),
        };
        let t_check = std::time::Instant::now();
        let dl_on = spec.dl.is_some() && env_on("FLASHTEX_SOURCE_MAP");
        // The user's files as they are before the run reads any: where the
        // edit is looked for now, and the next run's (read once, before
        // the restore reopens a file: an edit saved while this run reads
        // is a change from them the next run sees).
        let texts_then = self.keep_pages.then(|| self.texts_now());
        let (mut start, mut s0_note) = self.start_for(&key, &line, dl_on, texts_then.as_ref());
        // (tests: a pause between where the edit was looked for and the
        // restore, for an edit saved in between)
        if let Some(ms) = std::env::var("FLASHTEX_UNICODE_TEST_START_PAUSE_MS")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
        let check_ms = t_check.elapsed().as_secs_f64() * 1e3;
        let mut rewrite_failed = None;
        if start.in_body() {
            // the files the run wrote and closed before S₀, as it left them
            if let Err(e) = self.s0.as_ref().expect("checked").key.rewrite_outputs() {
                rewrite_failed = Some(e);
            }
        }
        if let Some(e) = rewrite_failed {
            self.s0 = None;
            self.pages.clear();
            start = Start::Format;
            s0_note = Some(e);
        }
        // The output files a restore cuts back, as they are (a cancelled
        // run puts them back: `Resident::guard`), and the bookkeeping of
        // the last complete run, which a cancelled one leaves as it was.
        let mut guard = Resident {
            cancel: spec.cancel.clone(),
            format_dir: None,
            keep_point: false,
            point: None,
            want_s0: false,
            s0: None,
            pages_on: false,
            pages: vec![],
            opens: vec![],
            guard: vec![],
            conv: None,
            fonts: Box::new(Unkept),
        };
        if start.in_body() {
            let recs = self
                .s0
                .iter()
                .map(|s| &s.rec)
                .chain(self.pages.iter().map(|c| &c.rec));
            for rec in recs {
                for f in &rec.files {
                    if let crate::system::Stream::Out { path, .. } = &f.stream {
                        guard.guard_out(path);
                    }
                }
            }
        }
        let before = (self.journal.clone(), self.opens.clone());
        let mut conv: Option<crate::conv::Old> = None;
        // the convergence measurement: the edits and what changed
        let measure = crate::conv::on() && start.in_body() && !c.no_pdf;
        let mut conv_changes = None;
        if measure {
            if let Some(now) = texts_then.as_ref() {
                let ch = self.changes(now);
                let edits: Vec<crate::conv::Edit> = now
                    .iter()
                    .filter_map(|(p, new)| {
                        let old = self.texts.get(p)?;
                        (old[..] != new[..]).then(|| crate::conv::Edit::between(p, old, new))
                    })
                    .collect();
                if let Some(ch) = ch {
                    conv_changes = Some((edits, ch.files.into_keys().collect(), ch.lookup));
                }
            }
        }
        // Put the kept state back: from a page, else S₀, else the format
        // point, else the start (a restore that fails -- a file it reopens
        // gone -- falls back to the next).
        loop {
            if start == Start::Fresh {
                // the old engine's memory goes before the new one's comes
                self.clear();
                self.g = Some(Globals::new());
                break;
            }
            let g = self.g.as_mut().expect("kept with its point");
            let restored = match start {
                Start::Format => {
                    let p = self.point.as_ref().expect("checked");
                    g.restore(p.id).map(|()| {
                        g.visit_files(&mut CloseFiles);
                        p.state.put(g);
                    })
                }
                Start::Page(_) | Start::S0 | Start::Fresh => {
                    // (the old run's later state kept as a branch: the
                    // convergence test, `crate::conv`)
                    let (id, rec, base, later) = match start {
                        Start::Page(i) => (
                            self.pages[i].id,
                            &self.pages[i].rec,
                            Some(&self.terminal[..]),
                            &self.pages[i + 1..],
                        ),
                        _ => {
                            let s0 = self.s0.as_ref().expect("checked");
                            (s0.id, &s0.rec, None, &self.pages[..])
                        }
                    };
                    match conv_changes.take() {
                        Some((edits, changed, lookup)) => {
                            let old: Vec<crate::conv::OldPage> = later
                                .iter()
                                .map(|c| crate::conv::OldPage {
                                    pages: c.pages,
                                    id: c.id,
                                    rec: c.rec.clone(),
                                    reads: c.reads,
                                })
                                .collect();
                            let tables = g.host.dl.tables();
                            match g.restore_branch(id) {
                                Err(e) => Err(e),
                                Ok(branch) => {
                                    let ptrs = self
                                        .eqtb_pointers
                                        .entry(id)
                                        .or_insert_with(|| Arc::new(crate::iso::eqtb_pointers(g)))
                                        .clone();
                                    conv = Some(crate::conv::Old {
                                        branch,
                                        pages: old,
                                        edits,
                                        changed,
                                        lookup,
                                        journal: self.journal.clone(),
                                        tables,
                                        eqtb_pointers: ptrs,
                                        claim: None,
                                        tests: vec![],
                                    });
                                    g.term_out = AlphaFile::captured_terminal();
                                    g.restore_record(rec, base)
                                }
                            }
                        }
                        None => g.restore(id).and_then(|()| {
                            g.term_out = AlphaFile::captured_terminal();
                            g.restore_record(rec, base)
                        }),
                    }
                }
            };
            match restored {
                Ok(()) => {
                    // the checkpoints after the one restored are gone
                    match start {
                        Start::Page(i) => self.pages.truncate(i + 1),
                        _ => self.pages.clear(),
                    }
                    break;
                }
                Err(e) => {
                    if let Some(c) = conv.take() {
                        g.arena.drop_branch(c.branch);
                    }
                    s0_note = Some(format!("the kept state ({}): {e}", start.name()));
                    start = match start {
                        Start::Page(_) => Start::S0,
                        Start::S0 => {
                            self.s0 = None;
                            Start::Format
                        }
                        _ => Start::Fresh,
                    };
                    self.pages.clear();
                }
            }
        }
        // the user's files as this run reads them (where the next run's
        // edit is looked for)
        // (a cancelled run puts back the last complete run's: its page
        // checkpoints stand, and an edit saved since is a change to them)
        let texts_before = texts_then.map(|t| std::mem::replace(&mut self.texts, t));
        let want_s0 = self.keep_s0 && self.keep_point && !spec.lean && !start.in_body();
        let want_pages = self.keep_pages && self.keep_s0 && self.keep_point && !spec.lean;
        // the journal and the output opens up to where the run starts
        let (journal, opens) = match start {
            Start::Page(i) => (
                self.journal
                    .as_ref()
                    .map(|j| truncate_journal(j, self.pages[i].reads)),
                self.opens[..self.pages[i].opens.min(self.opens.len())].to_vec(),
            ),
            Start::S0 => {
                let s0 = self.s0.as_ref().expect("checked");
                (
                    self.journal.as_ref().map(|j| truncate_journal(j, s0.reads)),
                    self.opens[..s0.opens.min(self.opens.len())].to_vec(),
                )
            }
            _ => (None, vec![]),
        };
        let record = want_s0 || want_pages;
        let resident = Box::new(Resident {
            cancel: spec.cancel.clone(),
            format_dir: spec.format_dir.clone(),
            keep_point: self.keep_point && start == Start::Fresh && !spec.lean,
            point: None,
            want_s0,
            s0: None,
            pages_on: want_pages && start.in_body(),
            pages: vec![],
            opens,
            guard: guard.guard,
            conv,
            fonts: self.fonts.take().unwrap_or_else(|| Box::new(Unkept)),
        });
        let mut out = (!c.no_pdf).then(|| {
            let mut o = crate::out::Output::to_writer(
                spec.dl,
                spec.font_formats,
                Some(self.font_cache.clone()),
            );
            o.compress = !spec.preview;
            Box::new(o)
        });
        let mut resumed = Ok(());
        if let (Start::Page(i), Some(o)) = (start, out.as_mut()) {
            // the pages before the restart point, sent again from the
            // checkpoint's document
            if let Some(snap) = &self.pages[i].out {
                resumed = o.resume(snap, self.g.as_ref().expect("restored"));
            }
        }
        let g = self.g.as_mut().expect("made above");
        if !start.in_body() {
            g.term_out = AlphaFile::captured_terminal();
        }
        g.host.clock = Some(self.clock);
        // the point and S₀ are after the run's first reading of the clock
        g.host.seed_pending = start == Start::Fresh;
        g.host.resident = Some(resident);
        g.host.out = out;
        // the source map, when the display list is made (a restored state
        // made it alike: `start_for`)
        let mem_len = g.mem.len();
        g.host.dl.set_on(dl_on, mem_len);
        system::set_no_pdf(g, c.no_pdf);
        // S₀ is armed by the run's first `big_switch` (`crate::ckpt`); a run
        // from S₀ or a page asks for no S₀
        g.ckpt_request = if want_s0 { REQ_LOOKUP } else { 0 };
        g.ckpt_on_shipout = if want_pages { REQ_PAGE } else { 0 };
        if !start.in_body() {
            g.ckpt_arm_cs = 0;
            g.ckpt_arm_level = 0;
        }
        if start == Start::Fresh {
            if let Some(l) = first_line {
                system::set_first_line(g, l);
            }
        }
        if record {
            flashtex_engine::system::record_reads_into(Some(
                journal.unwrap_or_else(ReadLog::keeping_content),
            ));
        }
        // the external effects (`\write18`) are the run's own: from the
        // start or the format point, none yet (a run from S₀ or a page has
        // its point's, `restore_record`); else one in a body would count against every
        // later S₀ of the session
        if !start.in_body() {
            flashtex_engine::system::truncate_external_effects(0);
        }
        let ran = match resumed {
            Ok(()) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                match start {
                    Start::Fresh => g.tex_body(),
                    Start::Format => g.run_after_format(),
                    Start::S0 | Start::Page(_) => g.resume_main_control(),
                }
                system::final_end(g)
            })),
            Err(e) => {
                Err(Box::new(format!("resuming the output: {e}")) as Box<dyn std::any::Any + Send>)
            }
        };
        let journal = if record {
            flashtex_engine::system::record_reads(false)
        } else {
            None
        };
        let mut rep = Report {
            start,
            s0_note,
            check_ms,
            restart_page: match start {
                Start::Page(i) => self.pages.get(i).map(|c| c.pages),
                _ => None,
            },
            ..Report::default()
        };
        match ran {
            Ok(never) => match never {},
            Err(payload) => {
                if let Some(e) = payload.downcast_ref::<EngineExit>() {
                    rep.exit_code = Some(e.0);
                } else if payload.is::<Cancelled>() {
                    rep.cancelled = true;
                } else {
                    let msg = payload
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "the engine panicked".into());
                    rep.failure = Some(msg);
                }
            }
        }
        rep.terminal = g.term_out.take_captured();
        let mut final_pages = None;
        if let Some(o) = g.host.out.take() {
            final_pages = Some(o.doc.pages.clone());
            // the font files the next run is likely to read again
            self.font_cache.retain(&o.doc.fonts.programs_read());
            rep.reports = o.reports.unwrap_or_default();
            // the display list's writer goes with it: its reader sees the end
        }
        // every file closed (written out), as a process's exit leaves them
        g.visit_files(&mut CloseFiles);
        let mut kept = None;
        let mut taken = None;
        let mut new_pages = vec![];
        let mut opens = vec![];
        let mut restored_files = vec![];
        if let Some(r) = g.host.resident.take() {
            self.fonts = Some(r.fonts);
            kept = r.point;
            taken = r.s0;
            new_pages = r.pages;
            opens = r.opens;
            if let Some(c) = r.conv {
                if !rep.cancelled && rep.failure.is_none() {
                    let t = crate::conv::truth(
                        &c,
                        final_pages.as_deref().unwrap_or(&[]),
                        &self.last_pages,
                    );
                    eprintln!("[conv] run: {}; {}", c.summary(), t);
                }
                g.arena.drop_branch(c.branch);
            }
            if rep.cancelled {
                // the output files as the last complete run left them
                for (p, d) in r.guard {
                    let _ = match d {
                        Some(d) => std::fs::write(&p, d),
                        None => std::fs::remove_file(&p),
                    };
                    restored_files.push(p);
                }
            }
        }
        if rep.cancelled && start.in_body() {
            // the checkpoints of the last complete run up to the restart
            // point stand (the files are its own again); this run's later
            // ones and its journal do not
            new_pages.clear();
            (self.journal, opens) = before;
            if let Some(t) = texts_before {
                self.texts = t;
            }
        }
        if crate::conv::on() && !rep.cancelled && rep.failure.is_none() {
            self.last_pages = final_pages.unwrap_or_default();
        }
        // (the pointer entries of checkpoints no longer kept go)
        if !self.eqtb_pointers.is_empty() {
            let ids: HashSet<CheckpointId> = self
                .s0
                .iter()
                .map(|s| s.id)
                .chain(self.pages.iter().map(|c| c.id))
                .collect();
            self.eqtb_pointers.retain(|k, _| ids.contains(k));
        }
        if rep.failure.is_some() || spec.lean {
            // a broken run's engine is not used again; Low Memory keeps none
            // (nor the fonts' files)
            self.clear();
            if spec.lean {
                self.font_cache = Arc::default();
            }
            return rep;
        }
        // what the run wrote is the engine's (a restore cuts it back only
        // while no other program has changed it)
        let mut stamped = HashSet::new();
        for p in restored_files.iter().chain(&opens) {
            if stamped.insert(p.as_str()) {
                flashtex_engine::system::stamp_output(p);
            }
        }
        self.opens = opens;
        // (the page checkpoints' records are cut from the last complete
        // run's terminal)
        if !(rep.cancelled && start.in_body()) {
            self.terminal = rep.terminal.clone();
        }
        if let Some((id, state, format)) = kept {
            self.point = Some(Point {
                key,
                id,
                state,
                format,
            });
        }
        // S₀, from a run that went on to its end (a cancelled run's journal
        // is whole up to S₀, which is all the key reads)
        match taken {
            Some(Ok(t)) => match crate::ckpt::make_key(&t, self.clock, &line) {
                Ok(k) => {
                    let reads = (
                        t.reads.files.len(),
                        t.reads.lookups.len(),
                        t.reads.outputs.len(),
                    );
                    self.s0 = Some(S0 {
                        id: t.id,
                        rec: t.rec,
                        key: k,
                        font_dirs: t.font_dirs,
                        reads,
                        opens: t.opens,
                        dl_on,
                    })
                }
                Err(e) => rep.s0_note = Some(format!("no S0: {e}")),
            },
            Some(Err(e)) => rep.s0_note = Some(format!("no S0: {e}")),
            None if want_s0 => {
                rep.s0_note = Some("no S0: the document's start was not reached".into())
            }
            None => {}
        }
        if self.s0.is_some() {
            self.pages.extend(new_pages);
            if !(rep.cancelled && start.in_body()) {
                self.journal = journal;
            }
            // the user's files this run read for the first time
            if let Some(j) = &self.journal {
                for f in &j.files {
                    if let Some(c) = &f.content {
                        self.texts
                            .entry(f.path.clone())
                            .or_insert_with(|| c.clone());
                    }
                }
            }
        } else {
            self.pages.clear();
            self.journal = None;
        }
        rep
    }
}
