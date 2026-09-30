//! L2-L4: checkpoints at every page, restart and converge, viewport first
//! (DESIGN.md §5.2-§5.4).
//!
//! A [`Session`] keeps one engine per document, like `crate::host`'s L1
//! session, and goes further:
//!
//! * **L2.** Every run takes a checkpoint after each `\shipout` and after
//!   `timed_s` of engine time without one (heavy pages), besides S₀. Each
//!   checkpoint's host record says how much of every input file the run had
//!   consumed (whole lines) and how much of the run's read journal preceded
//!   it. The undo logs are kept within a memory budget: dense near the last
//!   edit, thinner with distance (`enforce_budget`).
//! * **L3.** A compile finds the files that changed since the last run and,
//!   for the user's files, where: the common prefix and suffix of the old and
//!   new contents. It restores the newest checkpoint whose consumed input is
//!   entirely before every change (the checkpoints are ordered by
//!   consumption, so a binary search finds it), keeping the old run's future
//!   as a branch, and runs on. After each page it tests convergence with the
//!   old run's checkpoint after the same page: the same input position (the
//!   edited file's offsets moved by the edit's length difference, and the
//!   edit consumed by both), no changed file read by the old run afterwards,
//!   the same C-part state and the same word space (`Arena::diff_branch`
//!   with the position-only words of `positions_only` allowed to differ).
//!   On a match it jumps to the old run's state (`redo_to_remapped`), keeps
//!   the old run's later pages and checkpoints, and re-runs only the end of
//!   the job from the old run's last checkpoint (`\end{document}` always
//!   re-runs: it reads the `.aux` the pages wrote).
//! * **L4.** `compile(Some(page))` stops when that page has been shipped;
//!   [`Session::finish`] continues the run (to convergence or the end).
//!   Pages after the stop are the old run's, marked stale until then.
//!
//! A page's *frame* is what its interval wrote to the PDF file: its content
//! stream and objects, uncompressed in preview mode ([`Options::preview`]).
//! The soundness test (`flashtex-host soundness`) compares every frame, the
//! log and the terminal of an incremental compile with a run from scratch.

use crate::arena::CheckpointId;
use crate::checkpoint::{Action, ExtRecord, Observer, Point, STOPPED};
use crate::generated::Globals;
use crate::host::{self, Key};
use crate::persist::hash128;
use crate::system::{self, FileRead, ReadLog, RunOptions, StatSig, Stream};
use std::collections::HashMap;
use std::time::Instant;

/// One shipped page.
#[derive(Clone, Debug)]
pub struct Page {
    /// The checkpoint taken after it, while retained.
    pub ckpt: Option<CheckpointId>,
    /// The PDF bytes its interval wrote: hash and length.
    pub frame: [u64; 2],
    pub frame_len: u64,
}

/// One changed input file of the user's: `old = A X B`, `new = A Y B`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    pub path: String,
    /// `|A|`
    pub prefix: u64,
    /// `|X|` and `|Y|`
    pub old_mid: u64,
    pub new_mid: u64,
}

impl Edit {
    pub fn delta(&self) -> i64 {
        self.new_mid as i64 - self.old_mid as i64
    }
    /// The end of the changed bytes in the old file.
    pub fn old_end(&self) -> u64 {
        self.prefix + self.old_mid
    }
    /// An old offset at or after the change, in the new file.
    pub fn remap(&self, off: u64) -> u64 {
        if off >= self.old_end() {
            (off as i64 + self.delta()) as u64
        } else {
            off
        }
    }
}

/// The length of the common prefix of `a` and `b`: blocks compared as
/// slices (memcmp) first, then the bytes of the block that differs. A
/// byte-at-a-time loop took 1.5-2 ms of every keystroke's compile on a
/// 1,000-page source (2.5-4 MB).
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

/// The length of the common suffix of `a` and `b`, at most `max`.
fn common_suffix(a: &[u8], b: &[u8], max: usize) -> usize {
    const BLOCK: usize = 256;
    let n = a.len().min(b.len()).min(max);
    let (la, lb) = (a.len(), b.len());
    let mut s = 0;
    while s + BLOCK <= n && a[la - s - BLOCK..la - s] == b[lb - s - BLOCK..lb - s] {
        s += BLOCK;
    }
    s + a[..la - s]
        .iter()
        .rev()
        .zip(b[..lb - s].iter().rev())
        .take(n - s)
        .take_while(|(x, y)| x == y)
        .count()
}

/// The edit that turns `old` into `new` (common prefix and suffix).
pub fn diff_edit(path: &str, old: &[u8], new: &[u8]) -> Edit {
    let p = common_prefix(old, new);
    let max_s = old.len().min(new.len()) - p;
    let s = common_suffix(old, new, max_s);
    Edit {
        path: path.to_string(),
        prefix: p as u64,
        old_mid: (old.len() - p - s) as u64,
        new_mid: (new.len() - p - s) as u64,
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    /// Preview mode: PDF streams are stored, not compressed (the export
    /// is a separate full run; the engine state and the P-T1 log are those
    /// of a normal run but for the PDF's byte count).
    pub preview: bool,
    /// Bytes the undo logs may hold (DESIGN.md §5.2: 1 GB by default).
    pub budget: usize,
    /// A checkpoint after this much engine time without one (0: never).
    pub timed_s: f64,
    /// Checkpoints between shipouts, after `build_page` (`Point::Segment`),
    /// at least this far apart in engine time; `None`: none.
    pub segment_s: Option<f64>,
    /// Test convergence after each page of an incremental run.
    pub converge: bool,
    /// Print what differs at each convergence test to stderr.
    pub debug: bool,
    /// Compare structures where the runs allocated differently.
    pub relabel: bool,
    /// Anchor incremental runs at the `.aux` point (`Point::Aux`) rather
    /// than at S₀, so that a changed `.aux` restarts there instead of from
    /// the format (FLASHTEX_NO_AUX_POINT turns it off).
    pub aux_point: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            preview: true,
            budget: 1 << 30,
            timed_s: 0.020,
            segment_s: match std::env::var("FLASHTEX_SEGMENT_S") {
                Ok(v) if v == "off" => None,
                Ok(v) => v.parse().ok(),
                Err(_) => Some(DEFAULT_SEGMENT_S),
            },
            converge: true,
            debug: std::env::var_os("FLASHTEX_INCR_DEBUG").is_some(),
            relabel: std::env::var_os("FLASHTEX_NO_RELABEL").is_none(),
            aux_point: std::env::var_os("FLASHTEX_NO_AUX_POINT").is_none(),
        }
    }
}

/// What one compile (or its continuation) did.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// `cold`, `incremental`, `unchanged` or `continued`.
    pub mode: String,
    /// Why a compile ran in full.
    pub cold_reason: Option<String>,
    pub status: i32,
    /// Stopped at the requested page; `finish` continues it.
    pub paused: bool,
    /// Pages shipped before the restart point.
    pub restart_pages: usize,
    /// The restart point is not a page's checkpoint (a timed one), and how
    /// many bytes before the (first) edit its consumed input ends.
    pub restart_mid_page: bool,
    pub restart_gap: u64,
    /// Where the checkpoint after the restart point reads the edited file,
    /// in bytes from the (first) edit: past it (positive), which is why the
    /// restart point is the newest one before the edit. `None`: no later
    /// checkpoint reads the file.
    pub restart_next_gap: Option<i64>,
    /// The page after which the run converged with the old one.
    pub converged_at: Option<usize>,
    /// Pages this compile typeset.
    pub rerun_pages: usize,
    pub pages: usize,
    pub find_s: f64,
    /// Of `find_s`: S₀'s key check, and finding what changed.
    pub key_s: f64,
    pub changes_s: f64,
    pub restore_s: f64,
    /// From the start of the compile to the requested page (or the end).
    pub page_s: f64,
    pub total_s: f64,
    /// Convergence tests made and their total time.
    pub tests: usize,
    pub test_s: f64,
    /// Bytes the undo logs hold after the compile.
    pub log_bytes: usize,
    pub checkpoints: usize,
    /// What differed at the convergence tests (debug).
    pub diffs: Vec<String>,
    /// Each page this compile shipped, with the wall and thread CPU time
    /// since its start (the first pass only: later passes are the
    /// background's, DESIGN.md §5.5).
    pub page_times: Vec<(usize, f64, f64)>,
    /// The edited page: the first page the first pass shipped whose frame
    /// differs from the previous run's, with the wall and thread CPU time
    /// from the start of the compile to its shipout (DESIGN.md §1.2).
    pub edited: Option<(usize, f64, f64)>,
    /// Passes run (DESIGN.md §5.5: a run that changed a file it read, the
    /// `.aux`, runs again, up to five times), how each ran, what each took,
    /// and whether the passes stopped on a repeated state.
    pub passes: usize,
    pub pass_modes: Vec<String>,
    pub pass_s: Vec<f64>,
    pub oscillation: bool,
    /// What the checkpoints taken during this compile cost
    /// (`checkpoint::Stats`, as JSON).
    pub ck_stats: String,
    /// L5 (DESIGN.md §5.5), for each pass whose `.aux` changed: how many
    /// entries changed, where the first was read, the restart page -- or
    /// why the pass re-read the `.aux` from the `.aux` point instead.
    pub l5: Vec<String>,
    /// First reads the read-set holds after the compile.
    pub rs_events: usize,
    /// The run was stopped at a checkpoint because newer work arrived
    /// (`Session::set_preempt`): it is paused, `finish` would continue it,
    /// and the next `compile` keeps its checkpoints (`settle_paused`).
    pub preempted: bool,
}

impl Report {
    pub fn json(&self) -> String {
        format!(
            "{{\"mode\":\"{}\",\"cold_reason\":{},\"status\":{},\"paused\":{},\"restart_pages\":{},\"restart_mid_page\":{},\"restart_gap\":{},\"converged_at\":{},\"rerun_pages\":{},\"pages\":{},\"find_s\":{:.6},\"key_s\":{:.6},\"changes_s\":{:.6},\"restore_s\":{:.6},\"page_s\":{:.6},\"total_s\":{:.6},\"tests\":{},\"test_s\":{:.6},\"log_bytes\":{},\"checkpoints\":{},\"diffs\":{:?},\"page_times\":[{}],\"edited\":{},\"passes\":{},\"pass_modes\":{:?},\"pass_s\":[{}],\"oscillation\":{},\"ck_stats\":{},\"l5\":{:?},\"rs_events\":{},\"preempted\":{}}}",
            self.mode,
            self.cold_reason
                .as_ref()
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|| "null".into()),
            self.status,
            self.paused,
            self.restart_pages,
            self.restart_mid_page,
            self.restart_gap,
            self.converged_at
                .map(|p| p.to_string())
                .unwrap_or_else(|| "null".into()),
            self.rerun_pages,
            self.pages,
            self.find_s,
            self.key_s,
            self.changes_s,
            self.restore_s,
            self.page_s,
            self.total_s,
            self.tests,
            self.test_s,
            self.log_bytes,
            self.checkpoints,
            self.diffs,
            self.page_times
                .iter()
                .map(|(p, t, c)| format!("[{p},{t:.6},{c:.6}]"))
                .collect::<Vec<_>>()
                .join(","),
            self.edited
                .map(|(p, t, c)| format!("[{p},{t:.6},{c:.6}]"))
                .unwrap_or_else(|| "null".into()),
            self.passes,
            self.pass_modes,
            self.pass_s
                .iter()
                .map(|t| format!("{t:.6}"))
                .collect::<Vec<_>>()
                .join(","),
            self.oscillation,
            if self.ck_stats.is_empty() {
                "null"
            } else {
                &self.ck_stats
            },
            self.l5,
            self.rs_events,
            self.preempted,
        )
    }
}

/// What the observer of a run knows and collects.
struct Obs {
    /// Files either run opened for output (the old run's journal, and the
    /// live one), without a leading `./`.
    old_outputs: Vec<String>,
    /// How many files the old run had read at its last checkpoint: its
    /// reads after that are `\end{document}`'s, which always re-runs.
    old_reads_end: usize,
    t0: Instant,
    /// Pages shipped before the run's start.
    base: usize,
    new_pages: Vec<Page>,
    /// The PDF's path and length at the previous page (for frames).
    pdf: Option<(String, u64)>,
    /// The old run's pages from `base` on (convergence candidates).
    old_pages: Vec<Page>,
    edits: Vec<Edit>,
    /// Every file that changed (convergence: the old run must not read one
    /// after the convergence point).
    changed: Vec<String>,
    /// The old run's journal (files) and where each old checkpoint was in
    /// it.
    old_journal_files: Vec<String>,
    /// The same, with the files the old run closed again too (the barrier
    /// test (b') needs every later read).
    old_journal_all: Vec<String>,
    converge: bool,
    stop_at: Option<usize>,
    converged: Option<(usize, CheckpointId)>,
    /// Time of the stop page (or of the last page).
    page_s: f64,
    /// Checkpoints this run took, with the pages shipped by then.
    taken: Vec<(CheckpointId, usize)>,
    tests: usize,
    test_s: f64,
    debug: bool,
    diffs: Vec<String>,
    /// The `obj_offset` words of the objects the new run wrote, at the
    /// convergence point (byte offset in the space, word).
    positions: Vec<(usize, u64)>,
    /// The PDF's length at the restart point.
    pdf_len_r: u64,
    /// Compare structures when nodes were allocated elsewhere
    /// (`crate::iso`), and what that cost.
    relabel: bool,
    iso_s: f64,
    iso_nodes: usize,
    /// Walk each checkpoint's state alone and report cells no root reaches
    /// (`FLASHTEX_CHECKMEM`, a test of `crate::iso`'s root set).
    checkmem: bool,
    page_times: Vec<(usize, f64, f64)>,
    /// Thread CPU time at the start of the compile.
    cpu0: f64,
    /// Convergence tests missed, and the next page to test.
    fails: usize,
    next_test: usize,
    /// A test was skipped at a page shipped before the edited page.
    skipped_unchanged: bool,
    /// The old run's `last_byte_reads` at its end.
    old_last_byte_reads_end: Option<u64>,
    /// The old run's `matrix_uses` at its end.
    old_matrix_uses_end: Option<u64>,
    /// How many external effects (`\write18`, `\pdfelapsedtime`, ...) the
    /// old run had made at its end: one after a checkpoint is a barrier
    /// there (DESIGN.md §5.3).
    old_effects_end: usize,
    /// Retention during the run (`thin`): the budget, the cursor, the
    /// checkpoints never to drop, and the page and page-count maps of the
    /// checkpoints before the run.
    budget: usize,
    cursor: usize,
    s0: Option<CheckpointId>,
    keep_r: Option<CheckpointId>,
    known_pages: HashMap<CheckpointId, usize>,
    known_ck: HashMap<CheckpointId, usize>,
    /// The previous run's page frames, and the first page of this run
    /// whose frame differs (`Report::edited`).
    old_frames: Vec<[u64; 2]>,
    edited: Option<(usize, f64, f64)>,
    /// Checkpoints with L5 patches (`Session::defpatch`).
    patched: std::collections::HashSet<CheckpointId>,
    /// Preemption (`Session::set_preempt`): asked at each page and segment
    /// checkpoint of a run that may stop there, with the pass and the
    /// pages this run shipped; whether it stopped the run.
    preempt: Option<Preempt>,
    pass: usize,
    preempted: bool,
    /// The convergence test in progress may stop for newer work.
    interruptible: bool,
    /// At the convergence point: the characters the new run had shipped
    /// that the old run had not (`same_words`).
    char_or: Vec<(usize, u64)>,
}

/// Whether newer work waits: (pass, pages the run shipped) -> stop now.
pub type Preempt = std::rc::Rc<dyn Fn(usize, usize) -> bool>;

/// A convergence test stopped by newer work (`Obs::test`).
const PREEMPTED: &str = "preempted during the test";

impl Obs {
    /// Retention in the middle of a run (a long run would otherwise hold
    /// every page's log until it ends).
    fn thin(&mut self, g: &mut Globals) {
        let mut pages = self.known_pages.clone();
        let mut ck = self.known_ck.clone();
        if self.s0.is_none() {
            let l = g.layer();
            self.s0 = l.aux_point.or(l.s0);
        }
        for (i, p) in self.new_pages.iter().enumerate() {
            if let Some(c) = p.ckpt {
                pages.insert(c, self.base + i + 1);
            }
        }
        for &(c, n) in &self.taken {
            ck.insert(c, n);
        }
        thin(
            g,
            self.budget,
            self.cursor,
            self.s0,
            self.keep_r,
            &pages,
            &ck,
        );
        let ids: std::collections::HashSet<CheckpointId> = g.checkpoints().into_iter().collect();
        for p in self.new_pages.iter_mut() {
            if p.ckpt.is_some_and(|c| !ids.contains(&c)) {
                p.ckpt = None;
            }
        }
        self.taken.retain(|(c, _)| ids.contains(c));
        self.known_pages.retain(|c, _| ids.contains(c));
        self.known_ck.retain(|c, _| ids.contains(c));
    }

    /// Whether the old run, from its checkpoint `o` to its end, never read
    /// `pdf_last_byte` (utils.c's `pdf_newline`, used only when an included
    /// PDF is written; `crate::pdftex::last_byte_reads`). Then the value
    /// there is dead for the old run's future: every read it made came
    /// after a flush set it anew, and the jump keeps that future.
    fn old_last_byte_reads_after(&self, o: &ExtRecord) -> bool {
        self.old_last_byte_reads_end == Some(o.last_byte_reads)
    }

    fn pages_so_far(&self) -> usize {
        self.base + self.new_pages.len()
    }

    /// The PDF bytes written since the previous page.
    fn frame(&mut self, rec: &ExtRecord) -> ([u64; 2], u64) {
        let now = rec.files.iter().find_map(|f| match &f.stream {
            Stream::Out { path, len } if path.ends_with(".pdf") => Some((path.clone(), *len)),
            _ => None,
        });
        let Some((path, len)) = now else {
            return ([0, 0], 0);
        };
        let from = match &self.pdf {
            Some((p, l)) if *p == path && *l <= len => *l,
            _ => 0,
        };
        let bytes = read_range(&path, from, len).unwrap_or_default();
        self.pdf = Some((path, len));
        (hash128(&bytes), bytes.len() as u64)
    }

    /// DESIGN.md §5.3's convergence test of the live state (just
    /// checkpointed as `new`) against the old run's checkpoint `old`.
    fn converged(&mut self, g: &mut Globals, new: &ExtRecord, old: CheckpointId) -> bool {
        let t = Instant::now();
        self.tests += 1;
        let r = self.test(g, new, old);
        self.test_s += t.elapsed().as_secs_f64();
        match r {
            Ok(()) => true,
            Err(why) if why == PREEMPTED => {
                self.preempted = true;
                false
            }
            Err(why) => {
                if self.debug {
                    eprintln!("[incr] page {}: {why}", self.pages_so_far());
                }
                if self.diffs.len() < 8 {
                    self.diffs
                        .push(format!("page {}: {why}", self.pages_so_far()));
                }
                false
            }
        }
    }

    fn test(&mut self, g: &mut Globals, new: &ExtRecord, old: CheckpointId) -> Result<(), String> {
        let _m = crate::memstat::scope(crate::memstat::tag::TEST);
        // L5: a checkpoint that holds the meanings an earlier `.aux` gave
        // (its later restores patch them) is not the old run's state as the
        // old run's later pages saw it
        if self.patched.contains(&old) {
            return Err("the old checkpoint holds meanings a later .aux changed".into());
        }
        let o = g
            .pending_record(old)
            .ok_or("the old checkpoint has no record")?;
        // (a) the same input position
        if o.files.len() != new.files.len() {
            return Err("file globals differ".into());
        }
        for (a, b) in o.files.iter().zip(&new.files) {
            if (a.buf, &a.line, a.pos, a.have_line, a.at_eof, a.err)
                != (b.buf, &b.line, b.pos, b.have_line, b.at_eof, b.err)
            {
                return Err("an input file's lookahead differs".into());
            }
            match (&a.stream, &b.stream) {
                (Stream::In { path, offset: x }, Stream::In { path: p, offset: y }) => {
                    if path != p {
                        return Err(format!("reading {p}, the old run {path}"));
                    }
                    match self.edits.iter().find(|e| e.path == *path) {
                        Some(e) => {
                            if *x < e.old_end() || e.remap(*x) != *y {
                                return Err(format!(
                                    "{path}: at {y}, the old run at {x} (edit at {}..{}, {:+})",
                                    e.prefix,
                                    e.old_end(),
                                    e.delta()
                                ));
                            }
                        }
                        None => {
                            if x != y {
                                return Err(format!("{path}: at {y}, the old run at {x}"));
                            }
                        }
                    }
                }
                (Stream::Out { path, .. }, Stream::Out { path: p, .. }) => {
                    if path != p {
                        return Err(format!("writing {p}, the old run {path}"));
                    }
                }
                (x, y) => {
                    if x != y {
                        return Err("a file global's stream differs".into());
                    }
                }
            }
        }
        if o.effects_len != new.effects_len || o.tex_input_type != new.tex_input_type {
            return Err("an external effect since the restart".into());
        }
        // DESIGN.md §5.3's barriers: the old run's pages from here on made
        // an external effect (`\write18`) or read the clock
        // (`\pdfelapsedtime`); keeping them would not re-do it.
        if o.effects_len < self.old_effects_end {
            return Err("the old run reads a barrier (an external effect) later".into());
        }
        // (b) nothing the old run reads from here on has changed
        let from = o.reads.0.min(self.old_journal_files.len());
        if let Some(p) = self.old_journal_files[from..]
            .iter()
            .find(|p| self.changed.contains(p))
        {
            return Err(format!("the old run reads the changed {p} later"));
        }
        // (b') ... nor a file one of the runs writes (DESIGN.md §5.3's
        // barrier "`\read` of a file written in this run"): what the new run
        // wrote there since the restart may differ from what the old run
        // read back.
        // Every later read counts here, of a file closed again as well: a
        // file both runs write and read back (beamer's `.vrb`, written and
        // `\input` for each fragile frame) was blanked in
        // `old_journal_files` once closed, and a convergence before such a
        // read kept old pages typeset from what the old run wrote there
        // (soundness sweep A on the NixOS PC, beamer-fragile: a frame
        // with another frame's title, 2026-09-30).
        let end = self.old_reads_end.min(self.old_journal_all.len());
        if from < end {
            let norm = |p: &str| p.strip_prefix("./").unwrap_or(p).to_string();
            let live = system::outputs_since(0);
            let written = |p: &str| {
                let p = norm(p);
                self.old_outputs.contains(&p) || live.iter().any(|o| norm(o) == p)
            };
            if let Some(p) = self.old_journal_all[from..end]
                .iter()
                .find(|p| !p.is_empty() && written(p))
            {
                return Err(format!("the old run reads {p} later, which the runs write"));
            }
        }
        // the C parts
        if !new.cstate.same_as(&o.cstate) {
            return Err("pdfTeX's C-part state differs".into());
        }
        // the word space
        let last_byte_dead = self.old_last_byte_reads_after(&o);
        // The old run from here on never had a `\pdfsetmatrix` in effect:
        // then nothing it runs reads the dimensions `\pdfdest` leaves unset
        // (`crate::iso`), and neither does the new run, which runs the same
        // until the first such read.
        let dest_dims_dead = self.old_matrix_uses_end == Some(o.matrix_uses);
        // Newer work (`Session::set_preempt`) stops the comparison, which
        // may walk millions of words: the run then stops at this checkpoint
        // as if the work had come just before the test (`PREEMPTED`).
        let stop: Box<dyn FnMut() -> bool> = match (&self.preempt, self.interruptible) {
            (Some(p), true) => {
                let (p, pass, pages) = (p.clone(), self.pass, self.new_pages.len());
                Box::new(move || p(pass, pages))
            }
            _ => Box::new(|| false),
        };
        let t = Instant::now();
        let mut char_or = vec![];
        let r = same_words(
            g,
            old,
            last_byte_dead,
            dest_dims_dead,
            self.relabel,
            self.debug,
            stop,
            &mut char_or,
        );
        if r.is_ok() {
            self.char_or = char_or;
        }
        self.iso_s += t.elapsed().as_secs_f64();
        r.map(|nodes| {
            self.iso_nodes = nodes;
        })
    }
}

/// DESIGN.md §5.3's comparison of the live word space with the old run's at
/// checkpoint `old` of the pending branch: equal but for dead words, free
/// cells and PDF file positions, and -- with `relabel`, where the runs
/// allocated nodes elsewhere -- structurally equal (`crate::iso`). `Ok`:
/// the nodes the structural comparison walked (0 if it was not needed).
#[allow(clippy::too_many_arguments)]
fn same_words(
    g: &mut Globals,
    old: CheckpointId,
    last_byte_dead: bool,
    dest_dims_dead: bool,
    relabel: bool,
    debug: bool,
    mut stop: Box<dyn FnMut() -> bool + '_>,
    char_or: &mut Vec<(usize, u64)>,
) -> Result<usize, String> {
    let t = Instant::now();
    let Some(d) = g.diff_pending_until(old, &mut *stop)? else {
        return Err(PREEMPTED.into());
    };
    if debug {
        eprintln!(
            "[incr] diff: {} of {} chunks differ, {:.2} ms",
            d.differing.len(),
            d.compared,
            t.elapsed().as_secs_f64() * 1e3
        );
    }
    if d.differing.is_empty() {
        return Ok(0);
    }
    if stop() {
        return Err(PREEMPTED.into());
    }
    let words = crate::statediff::words(g, &d);
    let layout = crate::statediff::scalar_layout(g);
    let g: &Globals = g;
    // `pdf_char_used` (the characters each font has shipped, a set that
    // only grows) is read only at the end of the run, to subset the fonts
    // (pdftex.web's "Output fonts definition", the font writers). When the
    // new run's set holds the old run's, the right sets after the
    // convergence jump are the old run's plus the new run's extra
    // characters: `char_or` collects them (word, bits), and the jump adds
    // them to the old run's checkpoints from the convergence point on and
    // to the live state (`Arena::or_from`), so that every later restore
    // and test sees the sets as they are for the document now. A set that
    // lost a character (the only use of a glyph deleted) does not converge.
    char_or.clear();
    let words: Vec<_> = words
        .into_iter()
        .filter(|w| {
            if w.region == "pdf_char_used" && w.new & w.old == w.old {
                char_or.push((w.off, w.new & !w.old));
                false
            } else {
                true
            }
        })
        .collect();
    let (_pos, left): (Vec<_>, Vec<_>) = words
        .into_iter()
        .filter(|w| !dead_word(g, w))
        .filter(|w| !(last_byte_dead && w.scalar == Some("pdf_last_byte")))
        .partition(|w| position_only(g, w));
    let (left, (free_o, free_n)) = drop_free_mem(g, &d, &layout, left);
    if !left.is_empty() && relabel {
        // Nodes allocated in other places: compare the structures.
        if let Some(w) = left.iter().find(|w| w.region != "mem" && !iso_covers(g, w)) {
            return Err(format!(
                "{} differs outside what the structural comparison reads: {w}",
                left.len()
            ));
        }
        let bad_mem: Vec<usize> = left
            .iter()
            .filter(|w| w.region == "mem")
            .map(|w| w.index)
            .collect();
        if stop() {
            return Err(PREEMPTED.into());
        }
        let t = Instant::now();
        let r = crate::iso::Iso::check(
            g,
            &d,
            &layout,
            free_o.as_deref(),
            free_n.as_deref(),
            &bad_mem,
            g.hyph_list.len(),
            dest_dims_dead,
            &mut *stop,
        );
        if debug {
            eprintln!(
                "[incr] iso: {} words to explain, {:.2} ms",
                bad_mem.len(),
                t.elapsed().as_secs_f64() * 1e3
            );
        }
        return r.map_err(|e| {
            if e == crate::iso::STOPPED {
                PREEMPTED.to_string()
            } else {
                format!("structures differ: {e}")
            }
        });
    }
    if left.is_empty() {
        return Ok(0);
    }
    let mut s = format!(
        "{} words differ in {} of {} chunks compared: {}",
        left.len(),
        d.differing.len(),
        d.compared,
        crate::statediff::summary(&left)
    );
    if debug {
        for w in left.iter().take(12) {
            s.push_str(&format!("\n    {w}"));
        }
    }
    Err(s)
}

/// Arrays whose elements from a pointer on are dead between two commands,
/// with that pointer (tex.web: the unused parts of stacks and buffers,
/// which are always written before they are read again). The pointer
/// itself is compared like any scalar.
fn live_len(g: &Globals, region: &str) -> Option<usize> {
    let n = match region {
        // §38: the string pool up to `pool_ptr`
        "str_pool" => g.pool_ptr,
        // §31, §331: `first` is the first unused place of `buffer`
        "buffer" => g.first,
        // §38, §43: `str_start` up to the current string's start
        // (`make_string` sets the next entry before anything reads it)
        "str_start" => g.str_ptr + 1,
        // §268, §300, §213: the save, input and nest stacks up to their
        // pointers (the current input and list are the scalars `cur_input`
        // and `cur_list`)
        "save_stack" => g.save_ptr,
        "input_stack" => g.input_ptr,
        "nest" => g.nest_ptr,
        "param_stack" => g.param_ptr,
        // §388: the parameters of the macro being called, only during
        // `macro_call`; §316: `trick_buf`, only while an error context is
        // shown
        "pstack" | "trick_buf" => 0,
        // `line_break`'s arrays (§823, §833, §869), set in each call
        // before they are read (§827, §834, §837, §846, §855, §864)
        "active_width" | "cur_active_width" | "background" | "break_width" | "minimal_demerits"
        | "best_place" | "best_pl_line" | "disc_width" => 0,
        // §892, §897, §923, §934, §962: the word being hyphenated (and its
        // letters before lowercasing), filled by each `hyphenate`,
        // `\hyphenation` or `\patterns` before it reads them
        "hc" | "hu" => 0,
        // pdftex.web: the PDF output buffer up to `pdf_ptr` (in object
        // stream mode it is saved in `pdf_op_ptr`), the object stream
        // buffer likewise
        "pdf_op_buf" => {
            if g.pdf_os_mode {
                g.pdf_op_ptr
            } else {
                g.pdf_ptr
            }
        }
        "pdf_os_buf" => {
            if g.pdf_os_mode {
                g.pdf_ptr
            } else {
                g.pdf_os_ptr
            }
        }
        _ => return None,
    };
    Some(n.max(0) as usize)
}

/// Whether a differing word lies wholly in the dead part of its array
/// (`live_len`), or differs only there: its elements below the live
/// length are equal.
fn dead_word(g: &Globals, w: &crate::statediff::WordDiff) -> bool {
    // L5's marks of the control sequences read so far: bookkeeping of the
    // read-set, rebuilt from it at a convergence (`readset::rebuild_seen`)
    if w.region == "rs_seen" {
        return true;
    }
    // The display list's side table (changes/displaylist.ch): the source
    // position of each node, which nothing TeX computes reads (DESIGN.md
    // §6.1). Where the runs differ the jump keeps the old run's positions of
    // the nodes it wrote later, as the old run's own pages have them; the
    // test left it out before it moved into the word space, too.
    if w.region == "dl_side" {
        return true;
    }
    // The intrinsics' recording scratch (`crate::intrinsics`: `intr_state`
    // elements 2..=23, `S_REC_BASE` .. `S_REC_SCANNER`): the start of every
    // recording sets them all before anything reads them, and they are read
    // only while a recording is in progress -- none is when `S_REC_SLOT`
    // (element 1, compared like the rest) is 0. The recording's `tail`
    // stays behind in them and differs between runs that allocated
    // differently.
    // `intr_pre[p]`: what `eqtb[p]` held before the recording in progress
    // wrote it, read only for an entry that recording marked written (its
    // serial in `intr_seen`): dead while none is in progress.
    if w.region == "intr_pre" && g.intr_state[crate::intrinsics::REC_SLOT] == 0 {
        return true;
    }
    if w.region == "intr_state" && g.intr_state[crate::intrinsics::REC_SLOT] == 0 {
        let (r, rel) = g.arena.region_at(w.off);
        let elem = r.elem.max(1);
        let (lo, hi) = crate::intrinsics::REC_SCRATCH;
        if rel / elem >= lo && (rel + 7) / elem <= hi {
            return true;
        }
    }
    match w.scalar {
        // pdftex.web: the length of the last stream, set by every
        // `pdf_end_stream` (or `write_zip`) before its one read there
        Some("pdf_stream_length") => return true,
        // The stacks' and the buffer's high-water marks (tex.web §31,
        // §216, §271, §321, §390, and \csname's §374): read only by the
        // statistics at the end of the log (§1334, "stack positions"),
        // which DESIGN.md §1.1 counts as accounting, not typesetting. No
        // overflow test depends on them: each compares the new pointer
        // itself with the size. An edit that lengthens the longest line
        // (a one-line paragraph) changes `max_buf_stack`, and the test
        // failed on it at every page to the end of the document.
        Some(
            "max_buf_stack" | "max_in_stack" | "max_nest_stack" | "max_param_stack"
            | "max_save_stack",
        ) => return true,
        // §970-§977, §1010: `vert_break` always sets it (its loop takes
        // at least the list's end as a champion) and the two callers read
        // it right after the call, in the same command (`\vsplit` and an
        // insertion split).
        Some("best_height_plus_depth") => return true,
        // pdftex.web §693: outside text mode (`pdf_doing_text` false, which
        // is compared), `pdf_begin_string` calls `pdf_begin_text` before it
        // reads any of these, and `pdf_begin_text` sets them all (the first
        // three through `pdf_set_origin`); `adv_char_width` and
        // `pdf_set_font` run only after it. `pdf_delta_h` is not among them
        // (`pdf_begin_string` reads it first), nor `pdf_doing_string` (a
        // direct-mode literal reads it outside text mode).
        Some(
            "pdf_h" | "pdf_v" | "pdf_tj_start_h" | "pdf_f" | "pdf_last_f" | "pdf_last_fs"
            | "pdf_cur_Tm_a",
        ) => return !g.pdf_doing_text,
        Some(_) => return false,
        None => {}
    }
    let Some(live) = live_len(g, w.region) else {
        return false;
    };
    let (r, rel) = g.arena.region_at(w.off);
    let elem = r.elem.max(1);
    let first = rel / elem;
    if first >= live {
        return true;
    }
    // Elements of 1, 2 or 4 bytes share the word: compare the live ones.
    if elem >= 8 {
        return false;
    }
    let per = 8 / elem;
    let mask_bits = (live - first).min(per) * elem * 8;
    let mask = if mask_bits >= 64 {
        u64::MAX
    } else {
        (1u64 << mask_bits) - 1
    };
    (w.old ^ w.new) & mask == 0
}

/// The files a run read and has opened for output since (the `.aux` it
/// reads at `\begin{document}` and rewrites): an unfinished run's version
/// of them is its own partial output.
fn own_outputs(j: &ReadLog) -> Vec<String> {
    let norm = |p: &str| p.strip_prefix("./").unwrap_or(p).to_string();
    let outs: Vec<String> = j.outputs.iter().map(|p| norm(p)).collect();
    let mut v: Vec<String> = j
        .files
        .iter()
        .filter(|f| outs.contains(&norm(&f.path)) && !f.written_before)
        .map(|f| f.path.clone())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// An observer that stops the run at the first checkpoint of a kind.
struct StopAt(Point);

impl Observer for StopAt {
    fn on_checkpoint(&mut self, _g: &mut Globals, _id: CheckpointId, why: Point) -> Action {
        if why == self.0 {
            Action::Stop
        } else {
            Action::Continue
        }
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

/// `empty_flag` (tex.web §124): the `link` of a free variable-size node.
const EMPTY_FLAG: u32 = 268_435_455;

/// The cells of `mem` that are free in a state (tex.web §115-§125): the
/// one-word nodes of the `avail` stack, the blocks of the variable-size
/// free ring from `rover`, and the unused middle and top of the array.
/// `word(p)` reads `mem[p]`. `None` if a list is malformed (then nothing
/// is taken to be free).
fn free_cells(
    word: &dyn Fn(usize) -> u64,
    avail: i32,
    rover: i32,
    lo_mem_max: i32,
    hi_mem_min: i32,
    mem_end: i32,
) -> Option<Vec<u64>> {
    let n = crate::generated::consts::mem_max as usize + 1;
    let mut bits = vec![0u64; n.div_ceil(64)];
    let set = |p: usize, bits: &mut Vec<u64>| bits[p >> 6] |= 1 << (p & 63);
    let rh = |w: u64| (w & 0xFFFF_FFFF) as u32 as i32;
    let lh = |w: u64| (w >> 32) as u32 as i32;
    let in_range = |p: i32| p > 0 && (p as usize) < n;
    // the avail stack (§120): one-word nodes in [hi_mem_min, mem_end]
    let mut p = avail;
    let mut steps = 0usize;
    while p != 0 {
        if !in_range(p) || p < hi_mem_min || p > mem_end || steps > n {
            return None;
        }
        set(p as usize, &mut bits);
        p = rh(word(p as usize));
        steps += 1;
    }
    // the variable-size ring (§124): link(p)=empty_flag, node_size(p)=info(p),
    // rlink(p)=link(p+1)
    if rover != 0 {
        let mut p = rover;
        let mut steps = 0usize;
        loop {
            if !in_range(p) || p >= lo_mem_max || steps > n {
                return None;
            }
            let w = word(p as usize);
            if rh(w) as u32 != EMPTY_FLAG {
                return None;
            }
            let size = lh(w);
            if size <= 0 || p + size > lo_mem_max {
                return None;
            }
            set_range(&mut bits, p as usize, (p + size) as usize);
            p = rh(word(p as usize + 1));
            steps += 1;
            if p == rover {
                break;
            }
        }
    }
    // unused: between the two areas, and above mem_end
    set_range(
        &mut bits,
        (lo_mem_max + 1).max(0) as usize,
        hi_mem_min.clamp(0, n as i32) as usize,
    );
    set_range(&mut bits, (mem_end + 1).max(0) as usize, n);
    Some(bits)
}

/// Set bits `lo..hi`.
fn set_range(bits: &mut [u64], lo: usize, hi: usize) {
    let mut i = lo;
    while i < hi {
        if i & 63 == 0 && i + 64 <= hi {
            bits[i >> 6] = u64::MAX;
            i += 64;
        } else {
            bits[i >> 6] |= 1 << (i & 63);
            i += 1;
        }
    }
}

/// Drop from `left` the differing `mem` words whose cell is free in both
/// states, and the free lists' heads `avail` and `rover`: which free cell
/// comes next only decides where later nodes go, never what they hold,
/// and the jump keeps the old run's own lists. Every other word, and so
/// every word a live structure can reach, has been compared. When a free
/// list cannot be read, nothing is dropped.
type FreeSets = (Option<Vec<u64>>, Option<Vec<u64>>);

fn drop_free_mem(
    g: &Globals,
    d: &crate::arena::ChunkDiff,
    layout: &[crate::statediff::ScalarSlot],
    left: Vec<crate::statediff::WordDiff>,
) -> (Vec<crate::statediff::WordDiff>, FreeSets) {
    if !left
        .iter()
        .any(|w| w.region == "mem" || matches!(w.scalar, Some("avail" | "rover")))
    {
        return (left, (None, None));
    }
    let mem_off = match g.arena.regions.iter().find(|r| r.name == "mem") {
        Some(r) => r.off,
        None => return (left, (None, None)),
    };
    let scalar = |name: &str| -> Option<i32> {
        let s = layout.iter().find(|s| s.name == name)?;
        let w = d.old_word(&g.arena, s.off & !7);
        Some(((w >> ((s.off & 7) * 8)) & 0xFFFF_FFFF) as u32 as i32)
    };
    let (Some(av), Some(ro), Some(lo), Some(hi), Some(me)) = (
        scalar("avail"),
        scalar("rover"),
        scalar("lo_mem_max"),
        scalar("hi_mem_min"),
        scalar("mem_end"),
    ) else {
        return (left, (None, None));
    };
    let old_word = |p: usize| d.old_word(&g.arena, mem_off + p * 8);
    let new_word = |p: usize| g.mem[p].to_bits();
    let Some(fo) = free_cells(&old_word, av, ro, lo, hi, me) else {
        return (left, (None, None));
    };
    let Some(fnw) = free_cells(
        &new_word,
        g.avail,
        g.rover,
        g.lo_mem_max,
        g.hi_mem_min,
        g.mem_end,
    ) else {
        return (left, (Some(fo), None));
    };
    let free_both = |p: usize| (fo[p >> 6] & fnw[p >> 6]) >> (p & 63) & 1 == 1;
    let left = left
        .into_iter()
        .filter(|w| {
            if matches!(w.scalar, Some("avail" | "rover")) {
                return false;
            }
            !(w.region == "mem" && free_both(w.index))
        })
        .collect();
    (left, (Some(fo), Some(fnw)))
}

/// What the structural comparison (`crate::iso`) answers for: the arrays it
/// reads whole (their live parts; the rest is dead or relaxed before), and
/// the scalars it reads, holds dead between commands, or that only say
/// where the allocator will put the next node.
fn iso_covers(g: &Globals, w: &crate::statediff::WordDiff) -> bool {
    match w.scalar {
        Some(n) => crate::iso::scalar_covered(n),
        None => {
            matches!(
                w.region,
                "save_stack"
                    | "nest"
                    | "input_stack"
                    | "param_stack"
                    | "cur_mark"
                    | "font_glue"
                    | "hyph_list"
                    | "disc_ptr"
                    | "sa_root"
                    | "if_stack"
                    | "pdf_link_stack"
            ) || (w.region == "eqtb" && (w.index as i32) + 1 < crate::iso::INT_BASE)
                || (w.region == "obj_tab" && {
                    // only the word holding obj_aux (int4) can hold a pointer;
                    // it is compared with the structures
                    let size = std::mem::size_of::<crate::generated::types::obj_entry>();
                    let at = std::mem::offset_of!(crate::generated::types::obj_entry, int4);
                    let (_, rel) = g.arena.region_at(w.off);
                    rel % size == at & !7
                })
        }
    }
}

/// `FLASHTEX_CHECKMEM`: walk the live state from the roots and print the
/// allocated cells nothing reaches (a missing root or node field in
/// `crate::iso`) to stderr.
fn check_mem(g: &mut Globals, page: usize) {
    g.spill_scalars();
    let layout = crate::statediff::scalar_layout(g);
    let g: &Globals = g;
    let w = |p: usize| g.mem[p].to_bits();
    let Some(free) = free_cells(&w, g.avail, g.rover, g.lo_mem_max, g.hi_mem_min, g.mem_end) else {
        eprintln!("[checkmem] page {page}: the free lists cannot be read");
        return;
    };
    if std::env::var_os("FLASHTEX_CHECKMEM_CLASSIFY").is_some() {
        match crate::iso::Iso::classify_unreached(g, &layout, &free, g.hyph_list.len(), 12) {
            Err(e) => eprintln!("[checkmem] page {page}: walk failed: {e}"),
            Ok((nodes, missed, n_missed, n_leaked)) => {
                if n_missed > 0 {
                    let words: Vec<String> = missed
                        .iter()
                        .map(|&p| format!("{p}:{:#018x}", g.mem[p as usize].to_bits()))
                        .collect();
                    eprintln!(
                        "[checkmem] page {page}: {n_missed} cells referenced but unreached, {n_leaked} leaked ({nodes} nodes walked): {}",
                        words.join(" ")
                    );
                } else {
                    eprintln!(
                        "[checkmem] page {page}: ok ({nodes} nodes; {n_leaked} leaked cells)"
                    );
                }
            }
        }
        return;
    }
    match crate::iso::Iso::walk_one(g, &layout, &free, g.hyph_list.len(), 12) {
        Err(e) => {
            eprintln!("[checkmem] page {page}: walk failed: {e}");
            if let Some(v) = std::env::var_os("FLASHTEX_CHECKMEM_LIST") {
                let q: usize = v.to_string_lossy().parse().unwrap_or(0);
                let ws: Vec<String> = (q..q + 6)
                    .map(|p| format!("{p}:{:#018x}", g.mem[p].to_bits()))
                    .collect();
                eprintln!("[checkmem]   at {q}: {}", ws.join(" "));
                let refs = crate::iso::who_points(g, &layout, &[q as i32], 30);
                eprintln!("[checkmem]   refs: {}", refs.join(" "));
            }
        }
        Ok((nodes, missed, count)) => {
            if count > 0 {
                let words: Vec<String> = missed
                    .iter()
                    .map(|&p| format!("{p}:{:#018x}", g.mem[p as usize].to_bits()))
                    .collect();
                eprintln!(
                    "[checkmem] page {page}: {count} allocated cells unreached ({nodes} nodes walked): {}",
                    words.join(" ")
                );
                if std::env::var_os("FLASHTEX_CHECKMEM_WHO").is_some() {
                    let cnt = |lo: i32, hi: i32| {
                        (lo..=hi)
                            .filter(|&p| free[p as usize >> 6] >> (p as usize & 63) & 1 == 1)
                            .count()
                    };
                    let miss_lo = missed.len();
                    eprintln!(
                        "[checkmem]   lo: 0..={} free {} var_used {} | hi: {}..={} free {} dyn_used {} | missed shown {}",
                        g.lo_mem_max,
                        cnt(0, g.lo_mem_max),
                        g.var_used,
                        g.hi_mem_min,
                        g.mem_end,
                        cnt(g.hi_mem_min, g.mem_end),
                        g.dyn_used,
                        miss_lo
                    );
                    // the variable-size free ring
                    let mut ring = vec![];
                    let mut q = g.rover;
                    for _ in 0..40 {
                        let w = g.mem[q as usize].to_bits();
                        ring.push(format!("{q}+{}", (w >> 32) as u32));
                        q = g.mem[q as usize + 1].to_bits() as u32 as i32;
                        if q == g.rover {
                            break;
                        }
                    }
                    eprintln!(
                        "[checkmem]   rover {} lo_mem_max {} hi_mem_min {} ring: {}",
                        g.rover,
                        g.lo_mem_max,
                        g.hi_mem_min,
                        ring.join(" ")
                    );
                    if let Some(&p0) = missed.first() {
                        let ws: Vec<String> = (p0.max(4) - 4..p0 + 16)
                            .map(|p| format!("{p}:{:#018x}", g.mem[p as usize].to_bits()))
                            .collect();
                        eprintln!("[checkmem]   around {p0}: {}", ws.join(" "));
                    }
                    {
                        let top = crate::generated::consts::mem_max as usize;
                        let heads: Vec<String> = (top - 14..=top)
                            .map(|h| format!("{h}:{:x}", g.mem[h].to_bits()))
                            .collect();
                        eprintln!("[checkmem]   static heads: {}", heads.join(" "));
                        let mut ns = vec![];
                        for k in 0..g.nest_ptr as usize {
                            let r = &g.nest[k];
                            ns.push(format!(
                                "nest{k}: mode {} head {} tail {} aux {:x}",
                                r.mode_field,
                                r.head_field,
                                r.tail_field,
                                r.aux_field.to_bits()
                            ));
                        }
                        let r = &g.cur_list;
                        ns.push(format!(
                            "cur: mode {} head {} tail {}",
                            r.mode_field, r.head_field, r.tail_field
                        ));
                        eprintln!("[checkmem]   {}", ns.join(" | "));
                        eprintln!(
                            "[checkmem]   page_tail {} page_contents {} best_page_break {} save_ptr {} input_ptr {} cond_ptr {} align_ptr {}",
                            g.page_tail, g.page_contents, g.best_page_break, g.save_ptr, g.input_ptr, g.cond_ptr, g.align_ptr
                        );
                    }
                    if let Some(v) = std::env::var_os("FLASHTEX_CHECKMEM_BACK") {
                        // follow the only reference back while there is one
                        let mut t: i32 = v.to_string_lossy().parse().unwrap_or(0);
                        let mut path = vec![];
                        for _ in 0..20000 {
                            let refs = crate::iso::who_points(g, &layout, &[t], 8);
                            let mems: Vec<&String> = refs
                                .iter()
                                .filter(|r| r.contains("@mem[") && r.ends_with("+0"))
                                .collect();
                            if mems.len() == 1 {
                                let idx: i32 = mems[0]
                                    .split("@mem[")
                                    .nth(1)
                                    .unwrap()
                                    .split(']')
                                    .next()
                                    .unwrap()
                                    .parse()
                                    .unwrap();
                                path.push(idx);
                                t = idx;
                            } else {
                                eprintln!(
                                    "[checkmem]   back chain ends at {t}: refs {}",
                                    refs.join(" ")
                                );
                                break;
                            }
                        }
                        eprintln!(
                            "[checkmem]   back path: {:?}",
                            &path[path.len().saturating_sub(12)..]
                        );
                        let ws: Vec<String> = (t - 2..t + 8)
                            .map(|p| format!("{p}:{:#018x}", g.mem[p as usize].to_bits()))
                            .collect();
                        eprintln!("[checkmem]   at {t}: {}", ws.join(" "));
                    }
                    if let Some(v) = std::env::var_os("FLASHTEX_CHECKMEM_REFS") {
                        for t in v.to_string_lossy().split(',') {
                            let t: i32 = t.parse().unwrap_or(0);
                            let refs = crate::iso::who_points(g, &layout, &[t], 30);
                            eprintln!("[checkmem]   refs to {t}: {}", refs.join(" "));
                        }
                    }
                    if let Some(v) = std::env::var_os("FLASHTEX_CHECKMEM_EQTB") {
                        let i: usize = v.to_string_lossy().parse().unwrap_or(0);
                        let w = g.eqtb[i].to_bits();
                        eprintln!(
                            "[checkmem]   eqtb[{i}] = {w:#018x} (type {} level {} equiv {})",
                            (w >> 32) & 0xFFFF,
                            w >> 48,
                            w as u32
                        );
                    }
                    // FLASHTEX_CHECKMEM_LIST=p: print the node list from p
                    if let Some(v) = std::env::var_os("FLASHTEX_CHECKMEM_LIST") {
                        let mut q: i32 = v.to_string_lossy().parse().unwrap_or(0);
                        let mut out = vec![];
                        for _ in 0..60 {
                            if q <= 0 || q as usize >= g.mem.len() {
                                break;
                            }
                            let w = g.mem[q as usize].to_bits();
                            let (t, st, l) = (((w >> 32) & 0xFFFF), (w >> 48), w as u32);
                            let w1 = g.mem[q as usize + 1].to_bits();
                            out.push(format!("{q}:t{t}s{st}[{:x}]", w1));
                            q = l as i32;
                        }
                        eprintln!("[checkmem]   list: {}", out.join(" -> "));
                    }
                    // follow references back from the first missed cell
                    if let (Some(&p0), true) = (missed.first(), page == 1) {
                        let mut cur = p0;
                        let mut chain = vec![];
                        let mut seen = std::collections::HashSet::new();
                        for _ in 0..40 {
                            // a mem cell whose link or info is `cur` (or a
                            // cell just before it: node heads)
                            let mut found = None;
                            'search: for back in 0..6 {
                                let t = cur - back;
                                if t < 0 {
                                    break;
                                }
                                let refs = crate::iso::who_points(g, &layout, &[t], 64);
                                for r in &refs {
                                    if let Some(rest) = r.split('@').nth(1) {
                                        if let Some(m) = rest.strip_prefix("mem[") {
                                            let idx: i32 = m
                                                .split(']')
                                                .next()
                                                .unwrap_or("0")
                                                .parse()
                                                .unwrap_or(0);
                                            if seen.insert(idx) {
                                                found = Some((t, idx, r.clone()));
                                                break 'search;
                                            }
                                        } else {
                                            chain.push(format!("ROOT? {r}"));
                                        }
                                    }
                                }
                            }
                            match found {
                                Some((t, idx, r)) => {
                                    chain.push(format!("{t}<-{r}"));
                                    cur = idx;
                                }
                                None => break,
                            }
                        }
                        eprintln!("[checkmem]   back from {p0}: {}", chain.join(" | "));
                    }
                    for &p in
                        missed
                            .iter()
                            .take(if std::env::var_os("FLASHTEX_CHECKMEM_ALL").is_some() {
                                3
                            } else {
                                0
                            })
                    {
                        let t: Vec<i32> = vec![p];
                        eprintln!(
                            "[checkmem]   near {p}: {}",
                            crate::iso::who_points(g, &layout, &t, 60).join(" ")
                        );
                    }
                }
            } else {
                eprintln!("[checkmem] page {page}: ok ({nodes} nodes)");
            }
        }
    }
}

/// Whether a differing word only records where bytes went in the PDF file
/// (DESIGN.md §5.3: file positions are relocatable). No typesetting reads
/// them: they are written into the file's cross-reference table at the end
/// and `redo_to` never keeps an old run's end (the end re-runs). A word
/// that is one of them is allowed to differ; `relocate_positions` sets the
/// old run's values right after the jump.
fn position_only(g: &Globals, w: &crate::statediff::WordDiff) -> bool {
    match w.scalar {
        Some("pdf_gone" | "pdf_save_offset" | "pdf_stream_length_offset") => true,
        Some(_) => false,
        None => {
            if w.region != "obj_tab" {
                return false;
            }
            // obj_offset is obj_entry's i64 `int2`
            let (_, rel) = g.arena.region_at(w.off);
            let size = std::mem::size_of::<crate::generated::types::obj_entry>();
            if rel % size != std::mem::offset_of!(crate::generated::types::obj_entry, int2) {
                return false;
            }
            // a byte offset only when the object is not in an object stream
            // (obj_os_idx = -1) in both runs
            let k = rel / size;
            let old_ptr = w.old as i64;
            let new_ptr = w.new as i64;
            let idx_new = g.obj_tab.get(k).map(|e| e.int3);
            idx_new == Some(-1) && old_ptr >= 0 && new_ptr >= 0
        }
    }
}

impl Observer for Obs {
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }

    fn on_checkpoint(&mut self, g: &mut Globals, id: CheckpointId, why: Point) -> Action {
        if self.taken.len() % 32 == 31 && g.arena.log_bytes() > self.budget {
            self.thin(g);
        }
        if why != Point::Shipout {
            self.taken.push((id, self.pages_so_far()));
            if why == Point::Segment && self.preempt_now() {
                return Action::Stop;
            }
            return Action::Continue;
        }
        let rec = match g.record_of(id) {
            Ok(r) => r,
            Err(_) => return Action::Continue,
        };
        let (frame, frame_len) = self.frame(&rec);
        self.new_pages.push(Page {
            ckpt: Some(id),
            frame,
            frame_len,
        });
        let j = self.pages_so_far();
        self.taken.push((id, j));
        if self.checkmem {
            check_mem(g, j);
        }
        self.page_s = self.t0.elapsed().as_secs_f64();
        let cpu = thread_cpu_s() - self.cpu0;
        self.page_times.push((j, self.page_s, cpu));
        let unchanged = self.old_frames.get(j - 1) == Some(&frame);
        if self.edited.is_none() && !unchanged {
            self.edited = Some((j, self.page_s, cpu));
        }
        // newer work first: not even a convergence test
        if self.stop_at != Some(j) && self.preempt_now() {
            return Action::Stop;
        }
        // The edited page first (DESIGN.md §1.2): a restart just before
        // the edited paragraph ships the page before it again when the
        // paragraph starts the next page (TeX breaks the page once it has
        // read it), unchanged. The state there holds the edited paragraph
        // and rarely equals the old run's; the test (up to ~20 ms on a
        // 1,000-page hyperref document) would delay the edited page. So
        // the first unchanged page before the edited one is not tested;
        // an edit that changes no page costs one page more.
        let skip = self.edited.is_none() && unchanged && !self.skipped_unchanged;
        if skip {
            self.skipped_unchanged = true;
        }
        if self.converge && j >= self.next_test && !skip {
            if let Some(old) = self
                .old_pages
                .get(j - self.base - 1)
                .and_then(|p| p.ckpt)
                .filter(|o| g.pending_ids().contains(o))
            {
                self.interruptible = self.stop_at != Some(j);
                if self.converged(g, &rec, old) {
                    self.converged = Some((j, old));
                    self.positions = new_positions(g, self.pdf_len_r);
                    return Action::Stop;
                }
                if self.preempted {
                    // newer work came during the test
                    return Action::Stop;
                }
                // Back off after three misses (an edit that reflows the
                // rest never converges): test at 1, 2, 4, ... pages on.
                self.fails += 1;
                self.next_test = j + if self.fails < 3 {
                    1
                } else {
                    1 << (self.fails - 2).min(6)
                };
            }
        }
        if self.stop_at == Some(j) {
            return Action::Stop;
        }
        Action::Continue
    }
}

impl Obs {
    /// Newer work waits: stop the run at this checkpoint.
    fn preempt_now(&mut self) -> bool {
        let stop = self
            .preempt
            .as_ref()
            .is_some_and(|p| p(self.pass, self.new_pages.len()));
        if stop {
            self.preempted = true;
        }
        stop
    }
}

/// CPU time of this thread, in seconds (the machine is shared: wall time
/// includes other processes' load, this does not).
pub fn thread_cpu_s() -> f64 {
    #[repr(C)]
    struct Timespec {
        sec: i64,
        nsec: i64,
    }
    extern "C" {
        fn clock_gettime(clk: i32, tp: *mut Timespec) -> i32;
    }
    #[cfg(target_os = "macos")]
    const CLOCK_THREAD_CPUTIME_ID: i32 = 16;
    #[cfg(not(target_os = "macos"))]
    const CLOCK_THREAD_CPUTIME_ID: i32 = 3;
    let mut t = Timespec { sec: 0, nsec: 0 };
    // SAFETY: an out-parameter of the right layout.
    unsafe { clock_gettime(CLOCK_THREAD_CPUTIME_ID, &mut t) };
    t.sec as f64 + t.nsec as f64 * 1e-9
}

fn read_range(path: &str, from: u64, to: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek};
    let mut f = std::fs::File::open(path).ok()?;
    f.seek(std::io::SeekFrom::Start(from)).ok()?;
    let mut b = vec![0u8; to.saturating_sub(from) as usize];
    f.read_exact(&mut b).ok()?;
    Some(b)
}

/// A journal cut back to its first `n` files, lookups and outputs.
fn truncate_journal(j: &ReadLog, n: (usize, usize, usize)) -> ReadLog {
    let mut out = ReadLog::keeping_content();
    out.files = j.files[..n.0.min(j.files.len())].to_vec();
    out.lookups = j.lookups[..n.1.min(j.lookups.len())].to_vec();
    out.outputs = j.outputs[..n.2.min(j.outputs.len())].to_vec();
    out.barriers = j.barriers.clone();
    // The directories the kept lookups depend on, as they are now (the
    // compile checked the lookups against them).
    out.dirs = j
        .dirs
        .iter()
        .map(|(d, _)| (d.clone(), StatSig::of(d).unwrap_or_default()))
        .collect();
    let paths: Vec<String> = out.files.iter().map(|f| f.path.clone()).collect();
    for p in &paths {
        out.mark_seen(p);
    }
    out
}

/// One document's resident engine with L2-L4.
pub struct Session {
    pub g: Option<Box<Globals>>,
    first_line: Vec<u8>,
    clock: (i64, i32),
    s0: Option<host::S0>,
    /// The document's pages as the last complete run left them.
    pub pages: Vec<Page>,
    /// Pages shipped before each retained checkpoint.
    ck_pages: HashMap<CheckpointId, usize>,
    /// What the last complete run read.
    journal: Option<ReadLog>,
    pub opts: Options,
    /// A run stopped at its requested page (L4), with what it was doing.
    paused: Option<Paused>,
    /// The page of the last edit (retention keeps checkpoints dense there).
    cursor: usize,
    /// PDF file position corrections of checkpoints inherited from an old
    /// run at a convergence, applied in order after a restore.
    reloc: HashMap<CheckpointId, Vec<Reloc>>,
    /// L5: new meanings of `.aux` entries to put into a checkpoint's state
    /// after restoring it, in order (the checkpoints from `Point::AuxDone`
    /// to where a pass with a changed `.aux` restarted hold the meanings
    /// the `.aux` read gave before; `crate::readset`).
    defpatch: HashMap<CheckpointId, Vec<std::sync::Arc<crate::readset::Patch>>>,
    /// Newer work is waiting: a running pass stops at its next page or
    /// segment checkpoint (`set_preempt`).
    preempt: Option<Preempt>,
    /// The last incremental pass's restart point: the next edit, typed
    /// near the last, most likely restarts there (`prepare_next`).
    last_restart: Option<CheckpointId>,
    /// The pass being run (1 for the compile's first).
    pass: usize,
    /// Files a paused run was writing when a new compile arrived, which the
    /// run read before (the `.aux`): the next pass takes them as its run
    /// read them (`settle_paused`), not as the unfinished run left them.
    fixed_inputs: Vec<String>,
    /// The session as the running pass found it (`Before`).
    before_pass: Option<Before>,
    /// An abandoned run shipped pages from this checkpoint on, which the
    /// restored run's differ from: the next pass restarts there at the
    /// latest, so that they are shipped again (a display holds them).
    reemit_from: Option<CheckpointId>,
    /// The directories the journal's lookups depend on (`Key::dirs`).
    lookup_dirs: Vec<(String, StatSig)>,
    /// What S₀'s key covers of the journal: the files read before S₀,
    /// less the input files open there.
    key_cover: (usize, Vec<String>),
    /// The job's command line (to set the process up for it again after a
    /// warm-up, `warm_up`).
    run_options: RunOptions,
}

struct Paused {
    report: Report,
    t0: Instant,
}

/// What a pass changes in the session before it completes (`changes`
/// updates the journal, an L5 restart attaches patches): put back when a
/// paused pass is abandoned (`Session::abandon_paused`).
struct Before {
    journal: Option<ReadLog>,
    defpatch: HashMap<CheckpointId, Vec<std::sync::Arc<crate::readset::Patch>>>,
    cursor: usize,
    aux_done: Option<CheckpointId>,
    aux_close_rs: Option<usize>,
}

impl Session {
    pub fn new(o: RunOptions, clock: Option<(i64, i32)>, opts: Options) -> Session {
        let clock = clock.or_else(host::pin_clock_from_env).unwrap_or_else(|| {
            let d = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (d.as_secs() as i64, d.subsec_micros() as i32)
        });
        crate::pdftex::utils::pin_clock(Some(clock));
        let first_line = host::first_line_of(&o);
        system::configure(o.clone());
        system::capture_terminal(true);
        crate::pdftex::set_preview(opts.preview);
        Session {
            run_options: o,
            g: None,
            first_line,
            clock,
            s0: None,
            pages: vec![],
            ck_pages: HashMap::new(),
            journal: None,
            opts,
            paused: None,
            cursor: 0,
            reloc: HashMap::new(),
            defpatch: HashMap::new(),
            preempt: None,
            last_restart: None,
            pass: 1,
            fixed_inputs: vec![],
            before_pass: None,
            reemit_from: None,
            lookup_dirs: vec![],
            key_cover: (0, vec![]),
        }
    }

    pub fn terminal(&self) -> Vec<u8> {
        system::terminal_bytes()
    }

    /// Persist S₀ (DESIGN.md §5.1) to `path`: (bytes, bytes on disk).
    pub fn save_s0(&mut self, path: &str) -> Result<(u64, u64), String> {
        let s0 = self.s0.as_ref().ok_or("no S0 to save")?;
        let (id, key) = (s0.id, s0.key.clone());
        let g = self.g.as_mut().ok_or("no engine")?;
        host::write_s0(g, id, &key, path)
    }

    /// Warm the process up before a document is opened (DESIGN.md §1.2's
    /// reopen target): compile a one-page document in `dir`, which starts
    /// kpathsea (texmf.cnf, the `ls-R` databases) and parses the font map
    /// once for the process (`mapfile::MapCache`), then set the process up
    /// for this session's job again. Returns the seconds it took.
    pub fn warm_up(&mut self, dir: &str) -> Result<f64, String> {
        let t = Instant::now();
        let here = std::env::current_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(dir).map_err(|e| format!("{dir}: {e}"))?;
        std::fs::write(
            format!("{dir}/flashtex-warm.tex"),
            "\\documentclass{article}\\begin{document}Warm.\\end{document}\n",
        )
        .map_err(|e| format!("{dir}: {e}"))?;
        std::env::set_current_dir(dir).map_err(|e| format!("{dir}: {e}"))?;
        let mut o = self.run_options.clone();
        o.args = vec!["flashtex-warm.tex".into()];
        let mut w = Session::new(o, Some(self.clock), self.opts.clone());
        let r = w.compile(None).map(|_| ());
        drop(w);
        std::env::set_current_dir(&here).map_err(|e| e.to_string())?;
        system::configure(self.run_options.clone());
        crate::pdftex::utils::pin_clock(Some(self.clock));
        system::truncate_terminal(0);
        self.g = None;
        self.s0 = None;
        r?;
        Ok(t.elapsed().as_secs_f64())
    }

    /// Open a persisted S₀ (`save_s0`, in this or another process) and run
    /// the body from it, stopping once page `stop_at` is shipped (L4; the
    /// rest with `finish`). `Err` if the file does not fit or its key no
    /// longer holds: the caller compiles instead.
    pub fn open_s0(&mut self, path: &str, stop_at: Option<usize>) -> Result<Report, String> {
        let _m = crate::memstat::scope(crate::memstat::tag::ENGINE);
        let t0 = Instant::now();
        let first_line = self.first_line.clone();
        let mut clock = self.clock;
        let (g, s0, orep) = host::read_s0(path, &mut |key| {
            // The session's clock is the one S₀ was taken with.
            clock = key.clock;
            crate::pdftex::utils::pin_clock(Some(key.clock));
            key.check(key.clock, &first_line)
        })?;
        self.clock = clock;
        self.paused = None;
        self.pages.clear();
        self.ck_pages.clear();
        self.reloc.clear();
        let id = s0.id;
        let mut g = g;
        let rec = g.record_of(id)?;
        // The journal from the key: what the run read before S₀, and the
        // input files open there as they are now (the key checked their
        // prefixes).
        let mut j = ReadLog::keeping_content();
        for (path, hash, stat) in &s0.key.files {
            j.files.push(FileRead {
                path: path.clone(),
                hash: *hash,
                stat: *stat,
                content: None,
                closed_at: None,
                written_before: false,
            });
            j.mark_seen(path);
        }
        let mut open = vec![];
        for f in &rec.files {
            if let Stream::In { path, .. } = &f.stream {
                let d = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
                j.files.push(FileRead {
                    path: path.clone(),
                    hash: hash128(&d),
                    stat: StatSig::of(path).unwrap_or_default(),
                    content: system::is_user_file(path).then(|| std::sync::Arc::new(d)),
                    closed_at: None,
                    written_before: false,
                });
                j.mark_seen(path);
                open.push(path.clone());
            }
        }
        for (name, fmt, must, found) in &s0.key.lookups {
            j.lookups.push(system::Lookup {
                name: name.clone(),
                format: crate::resolver::Format::all()[*fmt as usize],
                must_exist: *must,
                found: found.clone(),
            });
        }
        j.dirs = s0.key.dirs.clone();
        let counts = (j.files.len(), j.lookups.len(), j.outputs.len());
        g.set_record_reads(id, counts);
        self.key_cover = (counts.0, open);
        self.ck_pages.insert(id, 0);
        self.s0 = Some(s0);
        self.g = Some(g);
        system::record_reads_into(Some(j));
        let mut obs = self.observer(t0, 0, stop_at);
        obs.preempt = self.preempt.clone();
        let g = self.g.as_mut().unwrap();
        g.restore_discard(id)?;
        // L5: the anchor is the `.aux` point (its `.aux` open, unread): the
        // run's close of it begins the read-set
        if let Some(aux) = rec.files.iter().find_map(|f| match &f.stream {
            Stream::In { path, .. } if path.ends_with(".aux") => Some(path.clone()),
            _ => None,
        }) {
            let l = g.layer();
            l.aux_point = Some(id);
            l.aux_path = Some(aux);
            l.aux_armed = true;
        }
        g.checkpoint_every_shipout(true);
        g.layer().timed_s = self.opts.timed_s;
        g.checkpoint_segments(self.opts.segment_s);
        g.layer().observer = Some(Box::new(obs));
        let status = g.resume_to_end().inspect_err(|_| {
            system::record_reads_into(None);
        })?;
        let mut rep = Report {
            mode: "open".into(),
            restore_s: orep.total_s,
            find_s: orep.config_s,
            ..Report::default()
        };
        self.after_run(t0, status, &mut rep)?;
        Ok(rep)
    }

    /// Memory accounting (lane P4-MEMORY; DESIGN.md §5.2's budget): the
    /// process's resident bytes, the heap by tag (`crate::memstat`, with
    /// the feature `mem-stats`), the checkpoint layer's parts
    /// (`Globals::mem_stats`) and the session's own tables.
    pub fn mem_stats(&self) -> Vec<(String, i64)> {
        let mut v: Vec<(String, i64)> = vec![];
        if let Some((now, peak)) = crate::memstat::rss() {
            v.push(("rss".into(), now as i64));
            v.push(("rss_peak".into(), peak as i64));
        }
        if let Some((now, peak, by)) = crate::memstat::heap() {
            v.push(("heap".into(), now));
            v.push(("heap_peak".into(), peak));
            for (k, b) in crate::memstat::live_by_tag() {
                v.push((format!("heap_{k}"), b));
            }
            for (k, b) in by {
                v.push((format!("heap_peak_{k}"), b));
            }
        }
        if let Some(g) = &self.g {
            v.extend(g.mem_stats().into_iter().map(|(k, x)| (k.to_string(), x)));
        }
        v.push(("pages".into(), self.pages.len() as i64));
        v.push(("defpatch".into(), self.defpatch.len() as i64));
        v.push((
            "reloc".into(),
            self.reloc.values().map(|r| r.len()).sum::<usize>() as i64,
        ));
        v
    }

    /// While the engine waits for the next edit: work out the restore to
    /// the last compile's restart point now (`Arena::prepare_restore`), so
    /// that the next compile, if it restarts there, copies the state in
    /// instead of rewinding the logs from the document's end. `stop` ends
    /// it early (a new request). Nothing it does changes what the engine
    /// computes; a restore elsewhere, or after anything that changed the
    /// checkpoints, does not use it.
    pub fn prepare_next(&mut self, stop: &mut dyn FnMut() -> bool) -> bool {
        let _m = crate::memstat::scope(crate::memstat::tag::PREPARE);
        if self.paused.is_some() {
            return false;
        }
        let (Some(r), Some(g)) = (self.last_restart, self.g.as_mut()) else {
            return false;
        };
        let t = Instant::now();
        let ok = g.arena.prepare_restore(r, stop);
        if self.opts.debug {
            eprintln!(
                "[incr] prepared the restore to {r}: {ok}, {:.2} ms",
                t.elapsed().as_secs_f64() * 1e3
            );
        }
        ok
    }

    /// Stop a running pass at its next page or segment checkpoint when
    /// `p(pass, pages the run shipped)` says newer work waits (the host: a
    /// newer COMPILE is queued, or the client cancelled). The compile then
    /// returns paused with `Report::preempted`; `finish` would continue it,
    /// and the next `compile` keeps what it typeset (`settle_paused`).
    pub fn set_preempt(&mut self, p: Option<Preempt>) {
        self.preempt = p;
    }

    /// A new compile arrived while a run was paused (preempted, or stopped
    /// at a viewport page): keep what that run typeset -- its checkpoints,
    /// pages and journal become the document's -- and drop the old run's
    /// future it was converging towards. The document is then complete up
    /// to the paused point only; the next compile restarts at or before it
    /// like any other and, converging with it, runs the rest from its last
    /// page (`after_run`). Files the paused run was writing and had read
    /// (the `.aux`) are taken as it read them for the next pass
    /// (`fixed_inputs`): the unfinished run's partial output is not an
    /// input.
    fn settle_paused(&mut self) -> Result<(), String> {
        if self.paused.take().is_none() {
            return Ok(());
        }
        let g = self.g.as_mut().ok_or("no engine")?;
        let obs: Box<Obs> = g
            .layer()
            .observer
            .take()
            .and_then(|o| o.into_any().downcast::<Obs>().ok())
            .ok_or("the paused run lost its observer")?;
        g.abandon_pending();
        let mut pages: Vec<Page> = self.pages[..obs.base.min(self.pages.len())].to_vec();
        pages.extend(obs.new_pages.iter().cloned());
        self.pages = pages;
        for (id, n) in &obs.taken {
            self.ck_pages.insert(*id, *n);
        }
        self.journal = system::record_reads_into(None);
        if let Some(j) = &self.journal {
            self.lookup_dirs = j.dirs.clone();
            self.fixed_inputs = own_outputs(j);
        }
        let g = self.g.as_mut().unwrap();
        let ids: std::collections::HashSet<CheckpointId> = g.checkpoints().into_iter().collect();
        self.ck_pages.retain(|k, _| ids.contains(k));
        self.defpatch.retain(|k, _| ids.contains(k));
        for p in self.pages.iter_mut() {
            if p.ckpt.is_some_and(|c| !ids.contains(&c)) {
                p.ckpt = None;
            }
        }
        Ok(())
    }

    /// A paused run and the files as they are now: `Some(false)` if
    /// nothing changed since it read them, `Some(true)` if something did
    /// that it has already read (its newest checkpoint is not a restart
    /// point for the change), `None` otherwise.
    fn paused_vs_changes(&mut self) -> Option<bool> {
        let live = system::reads_so_far()?;
        // (what it is writing itself, the `.aux`, is not a change)
        let own = own_outputs(&live);
        let saved = self.journal.replace(live);
        let saved_fixed = std::mem::replace(&mut self.fixed_inputs, own);
        let r = self.changes();
        self.fixed_inputs = saved_fixed;
        let answer = match r {
            Ok((edits, changed, bad)) => {
                if changed.is_empty() && bad.is_none() {
                    Some(false)
                } else {
                    // the paused run's own restart point
                    let started = self.g.as_mut().and_then(|g| {
                        let o = g.layer().observer.take()?;
                        let o = o.into_any().downcast::<Obs>().ok()?;
                        let k = o.keep_r;
                        g.layer().observer = Some(o);
                        k
                    });
                    let ids = self.g.as_ref().map(|g| g.checkpoints()).unwrap_or_default();
                    let pos = |id: CheckpointId| ids.iter().position(|&i| i == id);
                    let newest = ids.last().copied();
                    match (self.restart_point(&edits, &changed, bad), newest, started) {
                        // a checkpoint the paused run took before the change
                        (Some(rp), Some(n), Some(k)) if rp != n && pos(rp) > pos(k) => Some(true),
                        _ => None,
                    }
                }
            }
            Err(_) => None,
        };
        self.journal = saved;
        answer
    }

    /// Abandon a paused run: the complete run it was replacing comes back
    /// whole (`Globals::reattach_pending`: its checkpoints, state, output
    /// files and terminal), with the session as that pass found it. Falls
    /// back to `settle_paused` where there is nothing to go back to.
    fn abandon_paused(&mut self) -> Result<(), String> {
        let g = self.g.as_mut().ok_or("no engine")?;
        if g.pending_ids().is_empty() || self.before_pass.is_none() {
            return self.settle_paused();
        }
        self.paused = None;
        let obs = g
            .layer()
            .observer
            .take()
            .and_then(|o| o.into_any().downcast::<Obs>().ok());
        self.reemit_from = obs
            .filter(|o| !o.new_pages.is_empty())
            .and_then(|o| o.keep_r);
        let g = self.g.as_mut().unwrap();
        g.reattach_pending()?;
        system::record_reads_into(None);
        let b = self.before_pass.take().unwrap();
        self.journal = b.journal;
        self.defpatch = b.defpatch;
        self.cursor = b.cursor;
        // DESIGN §5.5, the previous run's `.aux` is fixed input: the next
        // pass takes the files that run read and rewrote (the `.aux` the
        // abandoned pass was to act on) as that run read them; the pass
        // after it sees the change
        self.fixed_inputs = self.journal.as_ref().map(own_outputs).unwrap_or_default();
        let g = self.g.as_mut().unwrap();
        let l = g.layer();
        l.aux_done = b.aux_done;
        l.aux_close_rs = b.aux_close_rs;
        l.aux_armed = false;
        Ok(())
    }

    pub fn is_paused(&self) -> bool {
        self.paused.is_some()
    }

    /// Compile the document as it is now. With `stop_at`, stop once that
    /// page has been shipped (L4); `finish` continues. A run that changed a
    /// file it read (the `.aux` its `\end{document}` rewrote, the `.toc`)
    /// is followed by further passes (`more_passes`, DESIGN.md §5.5).
    pub fn compile(&mut self, stop_at: Option<usize>) -> Result<Report, String> {
        let _m = crate::memstat::scope(crate::memstat::tag::ENGINE);
        let t0 = Instant::now();
        // A run stopped for this compile (preempted, or at a viewport).
        if self.paused.is_some() {
            match self.paused_vs_changes() {
                // nothing new: it goes on
                Some(false) => return self.finish(),
                // it typeset pages before the change: they stay
                Some(true) => self.settle_paused()?,
                // it has not reached the change, or all it did is after
                // the change (typing: the same paragraph again), or cannot
                // tell: back to the complete run it was replacing, whose
                // checkpoints are as near the change and whose later pages
                // can still be converged with
                None => self.abandon_paused()?,
            }
        }
        if let Some(g) = self.g.as_mut() {
            g.layer().stats = Default::default();
        }
        self.pass = 1;
        let mut rep = self.compile_pass(t0, stop_at)?;
        rep.passes = 1;
        rep.pass_modes.push(rep.mode.clone());
        if !rep.paused {
            rep.pass_s.push(rep.total_s);
            self.more_passes(t0, &mut rep)?;
        }
        Ok(rep)
    }

    /// DESIGN.md §5.5: while the last pass changed a file it read (its
    /// `\end{document}` wrote an `.aux` other than the one it read, a
    /// `.toc` appeared), run another pass on what it wrote -- an ordinary
    /// incremental compile, which restarts before the first read of what
    /// changed -- up to `MAX_PASSES` in all, and stop early when the files
    /// the passes read repeat a state an earlier pass read (oscillation).
    /// These passes are the background's: the first pass's edited page is
    /// out before them (`Report::edited`, `page_times`).
    fn more_passes(&mut self, t0: Instant, rep: &mut Report) -> Result<(), String> {
        let mut seen: Vec<Vec<(String, [u64; 2])>> = vec![];
        while rep.passes < MAX_PASSES {
            let Some(lookup_or_key) = self.dirty() else {
                break;
            };
            if !lookup_or_key {
                if seen.is_empty() {
                    seen.push(self.read_state());
                }
                let now = self.disk_state();
                if seen.contains(&now) {
                    rep.oscillation = true;
                    break;
                }
            }
            self.pass = rep.passes + 1;
            let p = self.compile_pass(Instant::now(), None)?;
            rep.passes += 1;
            if p.paused {
                // preempted: `finish` goes on, or the next compile settles it
                rep.pass_modes.push(p.mode.clone());
                rep.paused = true;
                rep.preempted = p.preempted;
                rep.pages = p.pages;
                rep.total_s = t0.elapsed().as_secs_f64();
                if let Some(pp) = self.paused.as_mut() {
                    pp.report = rep.clone();
                }
                return Ok(());
            }
            rep.pass_modes.push(p.mode.clone());
            rep.pass_s.push(p.total_s);
            rep.status = p.status;
            rep.pages = p.pages;
            rep.log_bytes = p.log_bytes;
            rep.checkpoints = p.checkpoints;
            rep.tests += p.tests;
            rep.test_s += p.test_s;
            rep.rerun_pages += p.rerun_pages;
            rep.l5
                .extend(p.l5.iter().map(|s| format!("pass {}: {s}", rep.passes)));
            if rep.diffs.len() < 8 {
                rep.diffs
                    .extend(p.diffs.iter().take(8 - rep.diffs.len()).cloned());
            }
            seen.push(self.read_state());
        }
        rep.total_s = t0.elapsed().as_secs_f64();
        if let Some(g) = self.g.as_mut() {
            rep.ck_stats = g.layer().stats.json();
            rep.rs_events = g.layer().rs.len();
        }
        Ok(())
    }

    /// Whether the files or lookups the last run read changed since it
    /// read them (`None`: nothing did), and if so whether through its key
    /// or a lookup (a file that appeared) rather than a file's content.
    /// Changes nothing (`changes` does, for the pass that follows).
    fn dirty(&mut self) -> Option<bool> {
        if self.paused.is_some() {
            return None;
        }
        let s0 = self.s0.as_ref()?;
        if s0.key.check(self.clock, &self.first_line).is_err() {
            return Some(true);
        }
        let saved = self.journal.clone();
        let r = self.changes();
        self.journal = saved;
        // A file the run wrote before it read it (beamer's `.vrb`) holds
        // what the run itself put there: not an input a pass sees change.
        let own = |p: &str| {
            self.journal.as_ref().is_some_and(|j| {
                j.files
                    .iter()
                    .find(|f| f.path == p)
                    .is_some_and(|f| f.written_before)
            })
        };
        match r {
            Ok((_, changed, bad)) => {
                if bad.is_some() {
                    Some(true)
                } else if changed.iter().all(|p| own(p)) {
                    None
                } else {
                    Some(false)
                }
            }
            Err(_) => Some(true),
        }
    }

    /// The files the last run read, each with the hash of what it read.
    fn read_state(&self) -> Vec<(String, [u64; 2])> {
        let mut v: Vec<(String, [u64; 2])> = vec![];
        if let Some(j) = &self.journal {
            for f in &j.files {
                if f.closed_at.is_some() || f.written_before || v.iter().any(|(p, _)| *p == f.path)
                {
                    continue;
                }
                let h = match &f.content {
                    Some(c) => hash128(c),
                    None => f.hash,
                };
                v.push((f.path.clone(), h));
            }
        }
        v.sort();
        v
    }

    /// The same files as `read_state`, each with the hash of what it holds
    /// now.
    fn disk_state(&self) -> Vec<(String, [u64; 2])> {
        let mut v: Vec<(String, [u64; 2])> = self
            .read_state()
            .into_iter()
            .map(|(p, _)| {
                let h = std::fs::read(&p).map(|d| hash128(&d)).unwrap_or([0, 0]);
                (p, h)
            })
            .collect();
        v.sort();
        v
    }

    /// One pass: from scratch, or from the newest checkpoint before what
    /// changed.
    fn compile_pass(&mut self, t0: Instant, stop_at: Option<usize>) -> Result<Report, String> {
        self.before_pass = self
            .g
            .as_mut()
            .map(|g| {
                let l = g.layer();
                (l.aux_done, l.aux_close_rs)
            })
            .map(|(aux_done, aux_close_rs)| Before {
                journal: self.journal.clone(),
                defpatch: self.defpatch.clone(),
                cursor: self.cursor,
                aux_done,
                aux_close_rs,
            });
        if self.paused.is_some() {
            // A new compile abandons the paused run: its later pages are
            // redone by this one.
            self.paused = None;
            return self.cold(
                t0,
                stop_at,
                Some("a compile arrived while one was paused".into()),
            );
        }
        let Some(s0) = &self.s0 else {
            return self.cold(t0, stop_at, None);
        };
        if let Err(why) = s0.key.check(self.clock, &self.first_line) {
            return self.cold(t0, stop_at, Some(why));
        }
        let key_s = t0.elapsed().as_secs_f64();
        let s0_id = s0.id;
        let changes = self.changes();
        // (cleared when this pass has run; a cold run below uses them)
        let fixed = self.fixed_inputs.clone();
        let (edits, changed, bad_lookup) = match changes {
            Ok(x) => x,
            Err(why) => return self.cold(t0, stop_at, Some(why)),
        };
        let changes_s = t0.elapsed().as_secs_f64() - key_s;
        // pages an abandoned run shipped are to be shipped again
        let reemit = self
            .reemit_from
            .take()
            .filter(|e| self.g.as_ref().is_some_and(|g| g.checkpoints().contains(e)));
        if changed.is_empty() && bad_lookup.is_none() && reemit.is_none() {
            return Ok(Report {
                mode: "unchanged".into(),
                pages: self.pages.len(),
                total_s: t0.elapsed().as_secs_f64(),
                ..Report::default()
            });
        }
        let mut l5 = vec![];
        let (mut r, mut patch) = match self.l5_restart(&edits, &changed, bad_lookup, &mut l5) {
            Some((r, p)) => (r, Some(p)),
            None => (
                self.restart_point(&edits, &changed, bad_lookup)
                    .unwrap_or(s0_id),
                None,
            ),
        };
        if let Some(e) = reemit {
            let g = self.g.as_mut().unwrap();
            let ids = g.checkpoints();
            let pos = |id: CheckpointId| ids.iter().position(|&i| i == id);
            if pos(e) < pos(r) {
                // (an earlier restart is always sound; one before the end of
                // the `.aux` read reads the `.aux` itself)
                let q = g.layer().aux_done.and_then(pos);
                if q.is_none_or(|q| pos(e) < Some(q)) {
                    patch = None;
                }
                r = e;
            }
        }
        // A run from before a fixed input's read reads it: as the run it
        // stands for read it (not the unfinished run's rewrite)
        if self.opts.debug {
            eprintln!("[incr] fixed inputs {fixed:?}, restart {r}");
        }
        if !fixed.is_empty() {
            if let (Some(j), Some(g)) = (self.journal.as_ref(), self.g.as_mut()) {
                let rr = g.record_of(r)?.reads.0;
                if self.opts.debug {
                    let at: Vec<(usize, &str, bool)> = j
                        .files
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| fixed.contains(&f.path))
                        .map(|(i, f)| (i, f.path.as_str(), f.content.is_some()))
                        .collect();
                    eprintln!("[incr] restart reads {rr}; fixed reads {at:?}");
                }
                let rec = g.record_of(r)?;
                for p in &fixed {
                    // the run reads it: open for input at the restart
                    // point, or opened again after it
                    let open = rec
                        .files
                        .iter()
                        .any(|f| matches!(&f.stream, Stream::In { path, .. } if path == p));
                    let later = j
                        .files
                        .iter()
                        .enumerate()
                        .any(|(i, f)| f.path == *p && f.closed_at.is_none() && i >= rr);
                    let content = j
                        .files
                        .iter()
                        .find(|f| f.path == *p)
                        .and_then(|f| f.content.clone());
                    if open || later {
                        if let Some(c) = content {
                            std::fs::write(p, c.as_slice()).map_err(|e| format!("{p}: {e}"))?;
                        }
                    }
                }
            }
        }
        let find_s = t0.elapsed().as_secs_f64();
        let rep = self.incremental(t0, r, edits, changed, stop_at, find_s, patch);
        self.fixed_inputs.clear();
        let mut rep = rep?;
        rep.key_s = key_s;
        rep.changes_s = changes_s;
        rep.l5 = l5;
        Ok(rep)
    }

    /// L5 (DESIGN.md §5.5; `crate::readset`): when files only the `.aux`
    /// read at the `.aux` point read have changed, re-run that read alone,
    /// find which entries' meanings changed, and restart at the newest
    /// checkpoint before the first read of one of them (and before every
    /// other change), with the new meanings put in. `None`: not
    /// applicable, or the change is not one `aux_delta` describes (`note`
    /// says why): the caller restarts from before the `.aux` read.
    fn l5_restart(
        &mut self,
        edits: &[Edit],
        changed: &[String],
        bad_lookup: Option<usize>,
        note: &mut Vec<String>,
    ) -> Option<(CheckpointId, std::sync::Arc<crate::readset::Patch>)> {
        if std::env::var_os("FLASHTEX_NO_L5").is_some() {
            return None;
        }
        let g = self.g.as_mut()?;
        let (p_aux, q) = {
            let l = g.layer();
            match (l.aux_point, l.aux_done) {
                (Some(a), Some(b)) => (a, b),
                (a, b) => {
                    if changed.iter().any(|p| p.ends_with(".aux")) {
                        note.push(format!(
                            "no .aux point ({a:?}) or no end of its read ({b:?})"
                        ));
                    }
                    return None;
                }
            }
        };
        let ids = g.checkpoints();
        let pos = |id: CheckpointId| ids.iter().position(|&i| i == id);
        let (Some(pp), Some(pq)) = (pos(p_aux), pos(q)) else {
            note.push("the .aux point or the end of its read is not retained".into());
            return None;
        };
        let rec_p = g.record_of(p_aux).ok()?;
        let rec_q = g.record_of(q).ok()?;
        let end = self.end_point()?;
        let g = self.g.as_mut()?;
        let old_end = g.record_of(end).ok()?.reads.0;
        let j = self.journal.as_ref()?;
        // a changed file read inside the `.aux` read and not after it (but
        // by `\end{document}`, which always re-runs); reads before the
        // `.aux` point are LaTeX's `\IfFileExists` tests, which the anchor
        // stands for (`restart_point` takes it as good)
        let aux_path = g.layer().aux_path.clone();
        let in_aux_read = |path: &str| {
            // the `.aux` the `.aux` point opened, or a file the read opened
            // (an `\include`d part's `.aux`)
            let entries: Vec<usize> = j
                .files
                .iter()
                .enumerate()
                .filter(|(i, f)| f.path == path && *i < old_end)
                .map(|(i, _)| i)
                .collect();
            let inside = |i: usize| rec_p.reads.0 <= i && i < rec_q.reads.0;
            (aux_path.as_deref() == Some(path)
                || entries
                    .iter()
                    .any(|&i| inside(i) && j.files[i].closed_at.is_none()))
                && entries.iter().all(|&i| i < rec_q.reads.0)
        };
        let (aux, others): (Vec<String>, Vec<String>) =
            changed.iter().cloned().partition(|p| in_aux_read(p));
        if aux.is_empty() {
            if changed.iter().any(|p| p.ends_with(".aux")) {
                let reads: Vec<(usize, &str, Option<u64>)> = j
                    .files
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.path.ends_with(".aux"))
                    .map(|(i, f)| (i, f.path.as_str(), f.closed_at))
                    .collect();
                note.push(format!(
                    "the changed .aux is read outside the .aux read [{}, {}) (end {old_end}): {reads:?}; changed {changed:?}",
                    rec_p.reads.0, rec_q.reads.0
                ));
            }
            return None;
        }
        let r_files = self.restart_point(edits, &others, bad_lookup)?;
        let g = self.g.as_mut()?;
        let ids = g.checkpoints();
        let pos = |id: CheckpointId| ids.iter().position(|&i| i == id);
        if pos(r_files)? < pq {
            // another change is before the `.aux` read ends: re-read it all
            note.push("another change is before the end of the .aux read".into());
            return None;
        }
        let _ = pp;
        let before = self
            .defpatch
            .get(&q)
            .map(|v| {
                v.iter()
                    .fold(crate::readset::Patch::default(), |a, p| a.then(p))
            })
            .unwrap_or_default();
        let t = Instant::now();
        let patch = match self.aux_patch(p_aux, q, &before) {
            Ok(p) => p,
            Err(e) => {
                note.push(format!("re-read from the .aux point: {e}"));
                return None;
            }
        };
        let g = self.g.as_mut()?;
        let keys = patch.keys();
        let (first, close) = {
            let l = g.layer();
            (
                l.rs.first_read(l.aux_close_rs.unwrap_or(usize::MAX), &keys),
                l.aux_close_rs,
            )
        };
        if std::env::var_os("FLASHTEX_L5_DEBUG").is_some() {
            let l = g.layer();
            for (n, _) in &patch.defs {
                let k = n.key();
                let at: Vec<usize> =
                    l.rs.events
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| e.name == k)
                        .map(|(i, _)| i)
                        .collect();
                eprintln!(
                    "[l5] {n}: events {at:?} (close {close:?}, {} events)",
                    l.rs.len()
                );
            }
        }
        close?;
        let ids = g.checkpoints();
        let r_d = match first {
            Some(i) if i < rec_q.rs => {
                note.push(format!(
                    "{} entries changed; one is read right after the .aux read: re-read from the .aux point",
                    patch.defs.len()
                ));
                return None;
            }
            Some(i) => {
                // the newest checkpoint from Q on taken before event i
                let mut best = q;
                for &id in &ids[pq..] {
                    match g.record_of(id) {
                        Ok(rec) if rec.rs <= i => best = id,
                        _ => {}
                    }
                }
                best
            }
            None => self.end_point()?,
        };
        let r = if pos(r_d)? < pos(r_files)? {
            r_d
        } else {
            r_files
        };
        note.push(format!(
            "{} entries changed ({}), first read at event {:?}; restart at page {} ({:.1} ms to find)",
            patch.defs.len(),
            patch
                .defs
                .iter()
                .take(4)
                .map(|(n, _)| n.to_string())
                .collect::<Vec<_>>()
                .join(" "),
            first,
            self.ck_pages.get(&r).copied().unwrap_or(0),
            1000.0 * t.elapsed().as_secs_f64()
        ));
        Some((r, std::sync::Arc::new(patch)))
    }

    /// Re-run the `.aux` read alone (from the `.aux` point `p_aux` to
    /// `Point::AuxDone`), compare its end with the old run's `q`
    /// (`readset::aux_delta`), and go back to the old run.
    fn aux_patch(
        &mut self,
        p_aux: CheckpointId,
        q: CheckpointId,
        before: &crate::readset::Patch,
    ) -> Result<crate::readset::Patch, String> {
        let journal = self.journal.clone().ok_or("no journal")?;
        let g = self.g.as_mut().ok_or("no engine")?;
        let rec_p = g.record_of(p_aux)?;
        let (close0, done0) = {
            let l = g.layer();
            (l.aux_close_rs, l.aux_done)
        };
        system::record_reads_into(Some(truncate_journal(&journal, rec_p.reads)));
        if let Err(e) = g.restore(p_aux) {
            system::record_reads_into(None);
            return Err(format!("cannot restore the .aux point: {e}"));
        }
        let rec_p_str = g.str_ptr;
        {
            let l = g.layer();
            l.aux_armed = true;
            l.aux_done = None;
        }
        let saved_obs = g.layer().observer.take();
        g.layer().observer = Some(Box::new(StopAt(Point::AuxDone)));
        let st = g.resume_to_end();
        g.layer().observer = saved_obs;
        let result = (|| -> Result<crate::readset::Patch, String> {
            let st = st?;
            if st != STOPPED {
                return Err("the .aux read did not end".into());
            }
            let q_new = g
                .layer()
                .aux_done
                .ok_or("no checkpoint after the .aux read")?;
            let new = g.record_of(q_new)?;
            let old = g
                .pending_record(q)
                .ok_or("no record of the old .aux read")?;
            if !new.cstate.same_as(&old.cstate)
                || new.effects_len != old.effects_len
                || new.tex_input_type != old.tex_input_type
                || new.files.len() != old.files.len()
            {
                return Err("the .aux read left other host state".into());
            }
            for (a, b) in new.files.iter().zip(&old.files) {
                match (&a.stream, &b.stream) {
                    (Stream::Out { path, len: ln }, Stream::Out { path: p2, len: lo }) => {
                        if path != p2 {
                            return Err(format!("{path} and {p2} open"));
                        }
                        let from = rec_p.files.iter().find_map(|f| match &f.stream {
                            Stream::Out { path: pp, len } if pp == path => Some(*len),
                            _ => None,
                        });
                        let Some(from) = from else {
                            return Err(format!("{path} opened by the .aux read"));
                        };
                        let nb = read_range(path, from, *ln).ok_or("cannot read the output")?;
                        let ob = g
                            .pending_old_bytes(path, from, *lo)
                            .ok_or("the old output is not kept")?;
                        if nb != ob {
                            return Err(format!("the .aux read wrote other bytes to {path}"));
                        }
                    }
                    (x, y) => {
                        if (a.buf, &a.line, a.pos, a.have_line, a.at_eof, a.err)
                            != (b.buf, &b.line, b.pos, b.have_line, b.at_eof, b.err)
                        {
                            return Err("a file's lookahead differs".into());
                        }
                        match (x, y) {
                            (
                                Stream::In { path, offset },
                                Stream::In {
                                    path: p2,
                                    offset: o2,
                                },
                            ) => {
                                if path != p2 || offset != o2 {
                                    return Err(format!(
                                        "reading {path} at {offset}, the old run {p2} at {o2}"
                                    ));
                                }
                            }
                            (x, y) if x != y => return Err("a file stream differs".into()),
                            _ => {}
                        }
                    }
                }
            }
            let term = system::terminal_bytes();
            let nt = term
                .get(rec_p.terminal_len..new.terminal_len)
                .unwrap_or(&[])
                .to_vec();
            let ot = g
                .pending_old_terminal(rec_p.terminal_len, old.terminal_len)
                .ok_or("the old terminal is not kept")?;
            if nt != ot {
                return Err("the .aux read printed something else".into());
            }
            let d = g.diff_pending(q)?;
            let (patch, back) = crate::readset::aux_delta(g, &d, before, &|g, w| dead_word(g, w))?;
            if std::env::var_os("FLASHTEX_L5_DEBUG").is_some() {
                let old = crate::readset::View::old(g, &d)?;
                let new = crate::readset::View::live(g)?;
                let (so, sn) = (old.str_ptr(), new.str_ptr());
                let from = rec_p_str;
                let list = |v: &crate::readset::View, n: i32| -> Vec<String> {
                    (from..n)
                        .map(|s| String::from_utf8_lossy(&v.string_bytes(s)).into_owned())
                        .collect()
                };
                eprintln!(
                    "[l5] strings since the .aux point: old {:?}\n[l5]   new {:?}",
                    list(&old, so),
                    list(&new, sn)
                );
                for (n, m) in &back.defs {
                    eprintln!("[l5] back {n}: {m:?}");
                    // who else holds its old list
                    if let Some(p) = old.lookup(n) {
                        let e = old.eqtb(p) as u32 as i32;
                        let others: Vec<String> = (1..crate::readset::UNDEFINED_CONTROL_SEQUENCE)
                            .filter(|&q| {
                                q != p
                                    && old.eqtb(q) as u32 as i32 == e
                                    && ((old.eqtb(q) >> 32) & 0xFFFF) >= 114
                            })
                            .map(|q| old.name(q).map_or(format!("{q}"), |x| x.to_string()))
                            .take(4)
                            .collect();
                        eprintln!("[l5]   old list {e} also held by {others:?}");
                    }
                }
            }
            let (olds, counts) = {
                let old = crate::readset::View::old(g, &d)?;
                let olds: Vec<Vec<u8>> = (rec_p_str..old.str_ptr())
                    .map(|s| old.string_bytes(s))
                    .collect();
                let c = (
                    old.scalar_i32("cs_count").unwrap_or(0),
                    old.scalar_i32("hash_used").unwrap_or(0),
                );
                (olds, c)
            };
            drop(d);
            // Verify: with the old run's meanings put back (and its order of
            // the names the read made), the state is the old run's up to
            // where things were allocated.
            crate::readset::apply_patch(g, &back)?;
            crate::readset::permute_strings(g, rec_p_str, &olds, counts)?;
            let mut char_or = vec![];
            same_words(
                g,
                q,
                false,
                false,
                true,
                false,
                Box::new(|| false),
                &mut char_or,
            )
            .and_then(|n| {
                // (no jump here to carry extra characters into the old
                // run's later states: the sets must be equal)
                if char_or.is_empty() {
                    Ok(n)
                } else {
                    Err("pdf_char_used differs".to_string())
                }
            })
            .map_err(|e| format!("besides {} changed entries: {e}", back.defs.len()))?;
            Ok(patch)
        })();
        system::record_reads_into(None);
        g.reattach_pending()?;
        let l = g.layer();
        l.aux_close_rs = close0;
        l.aux_done = done0;
        l.aux_armed = false;
        result
    }

    /// The files that changed since the last run: edits of the user's files
    /// (where their old content is known), every changed path, and the
    /// index of the first lookup that finds something else now.
    #[allow(clippy::type_complexity)]
    fn changes(&mut self) -> Result<(Vec<Edit>, Vec<String>, Option<usize>), String> {
        let (key_files, key_open) = self.key_cover.clone();
        let j = self.journal.as_mut().ok_or("no journal")?;
        let mut edits = vec![];
        let mut changed = vec![];
        // A file the journal lists more than once (read again) is read once.
        let mut now_of: HashMap<String, Option<std::sync::Arc<Vec<u8>>>> = HashMap::new();
        let fixed = self.fixed_inputs.clone();
        for (i, f) in j.files.iter_mut().enumerate() {
            // S₀'s key checked the files read before it, except those still
            // open there (only their prefix is keyed).
            if i < key_files && !key_open.contains(&f.path) {
                continue;
            }
            if fixed.contains(&f.path) {
                continue;
            }
            if StatSig::of(&f.path).as_ref() == Some(&f.stat) {
                continue;
            }
            let now = now_of
                .entry(f.path.clone())
                .or_insert_with(|| std::fs::read(&f.path).ok().map(std::sync::Arc::new))
                .clone();
            // With the old content at hand, compare bytes (a 1,000-page
            // source is 4 MB: hashing it costs 1.5 ms, comparing 0.2).
            let same = match (&f.content, &now) {
                (Some(old), Some(new)) => old.as_slice() == new.as_slice(),
                _ => now.as_deref().map(|n| hash128(n)) == Some(f.hash),
            };
            if same {
                if let Some(s) = StatSig::of(&f.path) {
                    f.stat = s;
                }
                continue;
            }
            if !changed.contains(&f.path) {
                changed.push(f.path.clone());
                match (&f.content, &now) {
                    (Some(old), Some(new)) => edits.push(diff_edit(&f.path, old, new.as_slice())),
                    _ => edits.push(Edit {
                        path: f.path.clone(),
                        prefix: 0,
                        old_mid: u64::MAX / 4,
                        new_mid: u64::MAX / 4,
                    }),
                }
            }
            // The run about to start reads the file as it is now (a restart
            // point is before the change): the journal says so.
            if let Some(n) = now {
                f.stat = StatSig::of(&f.path).unwrap_or_default();
                if f.content.is_some() {
                    f.content = Some(n);
                    // what the content hashes to, when a check needs it
                    f.hash = [0, 0];
                } else {
                    f.hash = hash128(&n);
                }
            }
        }
        // Lookups after S₀ (the key checked those before it).
        let s0_lookups = self
            .s0
            .as_ref()
            .and_then(|s| self.g.as_mut().map(|g| (s.id, g)))
            .and_then(|(id, g)| g.record_of(id).ok())
            .map_or(0, |r| r.reads.1);
        let mut bad = None;
        let dirs_same = !self.lookup_dirs.is_empty()
            && self
                .lookup_dirs
                .iter()
                .all(|(d, s)| StatSig::of(d).as_ref() == Some(s));
        for (i, l) in j
            .lookups
            .iter()
            .enumerate()
            .skip(s0_lookups)
            .filter(|_| !dirs_same)
        {
            if system::lookup_again(l) != l.found {
                bad = Some(i);
                break;
            }
        }
        Ok((edits, changed, bad))
    }

    /// The newest retained checkpoint (S₀ or later) that consumed nothing
    /// changed: every changed file it had read is one it is still reading,
    /// at an offset at or before the change, and every lookup it made still
    /// finds the same. Checkpoints are in the order the run took them, so
    /// their consumption only grows: a binary search finds the last good one.
    fn restart_point(
        &mut self,
        edits: &[Edit],
        changed: &[String],
        bad_lookup: Option<usize>,
    ) -> Option<CheckpointId> {
        let s0 = self.s0.as_ref()?.id;
        let j = self.journal.as_ref()?;
        let g = self.g.as_mut()?;
        let ids = g.checkpoints();
        let lo = ids.iter().position(|&i| i == s0)?;
        let first_read: HashMap<&str, usize> = {
            let mut m = HashMap::new();
            for (i, f) in j.files.iter().enumerate() {
                m.entry(f.path.as_str()).or_insert(i);
            }
            m
        };
        let mut good = |id: CheckpointId| -> bool {
            let Ok(r) = g.record_of(id) else {
                return false;
            };
            if bad_lookup.is_some_and(|b| r.reads.1 > b) {
                return false;
            }
            for p in changed {
                let Some(&i) = first_read.get(p.as_str()) else {
                    continue;
                };
                if i >= r.reads.0 {
                    continue; // read after this checkpoint
                }
                let e = edits.iter().find(|e| e.path == *p);
                let open_before = r.files.iter().any(|f| match &f.stream {
                    Stream::In { path, offset } => {
                        path == p && e.is_some_and(|e| *offset <= e.prefix)
                    }
                    _ => false,
                });
                // A file read before and no longer open, or open past the
                // change, was consumed.
                if !open_before {
                    return false;
                }
                // Open twice at once: unknown.
                if r.files
                    .iter()
                    .filter(|f| matches!(&f.stream, Stream::In { path, .. } if path == p))
                    .count()
                    != 1
                {
                    return false;
                }
                // An earlier read, closed by now (a file `\input` twice, a
                // `\IfFileExists` test), must have stopped before the change.
                let upto = r.reads.0.min(j.files.len());
                if j.files[..upto].iter().any(|f| {
                    f.path == *p && f.closed_at.is_some_and(|n| e.is_none_or(|e| n > e.prefix))
                }) {
                    return false;
                }
            }
            true
        };
        // ids[lo] (S₀) is good (its key was checked); find the last good.
        let (mut a, mut b) = (lo, ids.len());
        while b - a > 1 {
            let m = (a + b) / 2;
            if good(ids[m]) {
                a = m;
            } else {
                b = m;
            }
        }
        Some(ids[a])
    }

    fn observer(&self, t0: Instant, base: usize, stop_at: Option<usize>) -> Obs {
        Obs {
            old_outputs: vec![],
            old_reads_end: 0,
            t0,
            base,
            new_pages: vec![],
            pdf: None,
            old_pages: vec![],
            edits: vec![],
            changed: vec![],
            old_journal_files: vec![],
            old_journal_all: vec![],
            converge: false,
            stop_at,
            converged: None,
            page_s: 0.0,
            taken: vec![],
            tests: 0,
            test_s: 0.0,
            debug: self.opts.debug,
            diffs: vec![],
            positions: vec![],
            pdf_len_r: 0,
            relabel: self.opts.relabel,
            iso_s: 0.0,
            iso_nodes: 0,
            checkmem: std::env::var_os("FLASHTEX_CHECKMEM").is_some(),
            page_times: vec![],
            cpu0: thread_cpu_s(),
            fails: 0,
            skipped_unchanged: false,
            next_test: 0,
            old_last_byte_reads_end: None,
            old_matrix_uses_end: None,
            old_effects_end: 0,
            budget: self.opts.budget,
            cursor: self.cursor,
            s0: self.s0.as_ref().map(|s| s.id),
            keep_r: None,
            known_pages: self
                .pages
                .iter()
                .enumerate()
                .filter_map(|(i, p)| p.ckpt.map(|c| (c, i + 1)))
                .collect(),
            known_ck: self.ck_pages.clone(),
            old_frames: self.pages.iter().map(|p| p.frame).collect(),
            edited: None,
            patched: self.defpatch.keys().copied().collect(),
            preempt: None,
            pass: self.pass,
            preempted: false,
            interruptible: false,
            char_or: vec![],
        }
    }

    /// A full run from the format, taking S₀ and a checkpoint after every
    /// page.
    fn cold(
        &mut self,
        t0: Instant,
        stop_at: Option<usize>,
        reason: Option<String>,
    ) -> Result<Report, String> {
        // The files a stopped run was rewriting are read as the run this
        // pass stands for read them (`fixed_inputs`): put back for a run
        // from scratch, which reads them all
        let fixed = std::mem::take(&mut self.fixed_inputs);
        if let Some(j) = &self.journal {
            for p in &fixed {
                if let Some(c) = j
                    .files
                    .iter()
                    .find(|f| f.path == *p)
                    .and_then(|f| f.content.clone())
                {
                    std::fs::write(p, c.as_slice()).map_err(|e| format!("{p}: {e}"))?;
                }
            }
        }
        self.s0 = None;
        self.g = None;
        self.pages.clear();
        self.ck_pages.clear();
        self.journal = None;
        // Checkpoint ids start again with a new engine.
        self.reloc.clear();
        self.defpatch.clear();
        crate::pdftex::reset_state();
        crate::pdftex::utils::arm_pinned_seed();
        system::truncate_terminal(0);
        system::truncate_external_effects(0);
        system::record_reads_into(Some(ReadLog::keeping_content()));
        system::set_command_line(vec![self.first_line.clone()]);
        let mut g = Globals::new();
        g.arm_begin_document();
        g.layer().want_aux_point = self.opts.aux_point;
        g.checkpoint_every_shipout(true);
        g.layer().timed_s = self.opts.timed_s;
        g.checkpoint_segments(self.opts.segment_s);
        let obs = self.observer(t0, 0, stop_at);
        g.layer().observer = Some(Box::new(obs));
        let status = g.run_to_end();
        self.g = Some(g);
        let status = status.inspect_err(|_| {
            system::record_reads_into(None);
        })?;
        let mut rep = Report {
            mode: "cold".into(),
            cold_reason: reason,
            ..Report::default()
        };
        self.after_run(t0, status, &mut rep)?;
        Ok(rep)
    }

    #[allow(clippy::too_many_arguments)]
    fn incremental(
        &mut self,
        t0: Instant,
        r: CheckpointId,
        edits: Vec<Edit>,
        changed: Vec<String>,
        stop_at: Option<usize>,
        find_s: f64,
        patch: Option<std::sync::Arc<crate::readset::Patch>>,
    ) -> Result<Report, String> {
        let base = *self
            .ck_pages
            .get(&r)
            .ok_or("restart point without a page count")?;
        self.cursor = base;
        self.last_restart = Some(r);
        let journal = self.journal.as_ref().ok_or("no journal")?;
        // (a close reads nothing: the convergence test checks the streams
        // still open by their positions)
        let old_files: Vec<String> = journal
            .files
            .iter()
            .map(|f| match f.closed_at {
                Some(_) => String::new(),
                None => f.path.clone(),
            })
            .collect();
        let jr = journal.clone();
        let mut obs = self.observer(t0, base, stop_at);
        // The live state is the old run's end.
        obs.old_last_byte_reads_end = Some(crate::pdftex::last_byte_reads());
        obs.old_matrix_uses_end = Some(crate::pdftex::matrix_uses());
        obs.old_effects_end = system::external_effects_len();
        let t1 = Instant::now();
        let end = self.end_point();
        let g = self.g.as_mut().unwrap();
        let rec = g.record_of(r)?;
        let next_gap: Option<i64> = {
            let ids = g.checkpoints();
            ids.iter()
                .position(|&i| i == r)
                .and_then(|p| ids.get(p + 1))
                .copied()
                .and_then(|n| g.record_of(n).ok())
                .and_then(|nr| {
                    edits
                        .iter()
                        .filter_map(|e| {
                            nr.files.iter().find_map(|f| match &f.stream {
                                Stream::In { path, offset } if *path == e.path => {
                                    Some(*offset as i64 - e.prefix as i64)
                                }
                                _ => None,
                            })
                        })
                        .min()
                })
        };
        let old_reads_end = match end {
            Some(e) => g.record_of(e)?.reads.0,
            None => 0,
        };
        // Output files written and closed before `r` are the new run's own
        // too: nothing to put back. Restore, keeping the old future: the
        // restore saves the old bytes of every output file open at `r` or at
        // the old run's end, and of every file the old run opened after `r`
        // (its journal says which), so the journal must be in place.
        system::record_reads_into(Some(jr.clone()));
        if let Err(e) = g.restore(r) {
            // An output the restore needs is gone: a run that failed
            // removes its PDF (pdfTeX's "no output PDF file produced"),
            // which the checkpoints before it had open. Start again.
            return self.cold(t0, stop_at, Some(format!("cannot restore: {e}")));
        }
        if let Some(rs) = self.reloc.get(&r) {
            for x in rs {
                x.apply(g);
            }
        }
        if g.layer().aux_point == Some(r) {
            // the run reads the `.aux` again: its close begins the read-set
            g.layer().aux_armed = true;
        }
        let mut patches = self.defpatch.get(&r).cloned().unwrap_or_default();
        patches.extend(patch.iter().cloned());
        for p in &patches {
            if let Err(e) = crate::readset::apply_patch(g, p) {
                return self.cold(
                    t0,
                    stop_at,
                    Some(format!("cannot patch the .aux entries: {e}")),
                );
            }
        }
        if let Some(p) = &patch {
            // the checkpoints from the `.aux` read's end to here hold the
            // meanings it gave before
            let g = self.g.as_mut().unwrap();
            if let Some(q) = g.layer().aux_done {
                let ids = g.checkpoints();
                if let (Some(a), Some(b)) = (
                    ids.iter().position(|&i| i == q),
                    ids.iter().position(|&i| i == r),
                ) {
                    for &id in &ids[a..=b] {
                        self.defpatch.entry(id).or_default().push(p.clone());
                    }
                }
            }
        }
        let g = self.g.as_mut().unwrap();
        system::record_reads_into(Some(truncate_journal(&jr, rec.reads)));
        let restore_s = t1.elapsed().as_secs_f64();
        obs.old_pages = self.pages[base..].to_vec();
        obs.edits = edits;
        obs.changed = changed;
        obs.old_journal_files = old_files;
        obs.old_journal_all = jr.files.iter().map(|f| f.path.clone()).collect();
        obs.old_outputs = jr
            .outputs
            .iter()
            .map(|p| p.strip_prefix("./").unwrap_or(p).to_string())
            .collect();
        obs.old_reads_end = old_reads_end;
        obs.converge = self.opts.converge;
        obs.pdf = rec.files.iter().find_map(|f| match &f.stream {
            Stream::Out { path, len } if path.ends_with(".pdf") => Some((path.clone(), *len)),
            _ => None,
        });
        obs.pdf_len_r = pdf_len(&rec);
        obs.keep_r = Some(r);
        obs.preempt = self.preempt.clone();
        g.checkpoint_every_shipout(true);
        g.layer().timed_s = self.opts.timed_s;
        g.checkpoint_segments(self.opts.segment_s);
        // The pages before `r` stay; the rest are the old run's until
        // redone.
        let gap = obs
            .edits
            .iter()
            .filter_map(|e| {
                rec.files.iter().find_map(|f| match &f.stream {
                    Stream::In { path, offset } if *path == e.path => {
                        Some(e.prefix.saturating_sub(*offset))
                    }
                    _ => None,
                })
            })
            .min()
            .unwrap_or(u64::MAX);
        let mid = !self.pages.iter().any(|p| p.ckpt == Some(r));
        let g = self.g.as_mut().unwrap();
        g.layer().observer = Some(Box::new(obs));
        let status = g.resume_to_end().inspect_err(|_| {
            system::record_reads_into(None);
        })?;
        let mut rep = Report {
            mode: "incremental".into(),
            restart_mid_page: mid,
            restart_gap: gap,
            restart_next_gap: next_gap,
            restart_pages: base,
            find_s,
            restore_s,
            ..Report::default()
        };
        self.after_run(t0, status, &mut rep)?;
        Ok(rep)
    }

    /// Continue a run stopped at its requested page, to convergence or the
    /// end.
    pub fn finish(&mut self) -> Result<Report, String> {
        let _m = crate::memstat::scope(crate::memstat::tag::ENGINE);
        let Some(p) = self.paused.take() else {
            return Err("no run is paused".into());
        };
        let g = self.g.as_mut().unwrap();
        let status = g.resume_to_end().inspect_err(|_| {
            system::record_reads_into(None);
        })?;
        let mut rep = p.report;
        rep.mode = "continued".into();
        rep.paused = false;
        rep.preempted = false;
        self.after_run(p.t0, status, &mut rep)?;
        if rep.paused {
            return Ok(rep);
        }
        rep.pass_s.push(rep.total_s);
        self.more_passes(p.t0, &mut rep)?;
        Ok(rep)
    }

    /// After a run returned: pause, converge or complete.
    fn after_run(&mut self, t0: Instant, status: i32, rep: &mut Report) -> Result<(), String> {
        let g = self.g.as_mut().unwrap();
        let obs = g.layer().observer.take();
        let obs: Box<Obs> = obs
            .and_then(|o| o.into_any().downcast::<Obs>().ok())
            .ok_or("the run lost its observer")?;
        rep.tests += obs.tests;
        rep.test_s += obs.test_s;
        rep.page_times.extend(obs.page_times.iter().copied());
        if rep.edited.is_none() {
            rep.edited = obs.edited;
        }
        rep.diffs.extend(obs.diffs.iter().cloned());
        rep.page_s = if rep.page_s > 0.0 {
            rep.page_s
        } else {
            obs.page_s
        };
        if status == STOPPED && obs.converged.is_none() {
            // Paused at the requested page, or preempted.
            rep.status = status;
            rep.paused = true;
            rep.preempted = obs.preempted;
            rep.page_s = obs.page_s;
            rep.rerun_pages += obs.new_pages.len();
            rep.pages = obs.pages_so_far().max(self.pages.len());
            rep.total_s = t0.elapsed().as_secs_f64();
            let g = self.g.as_mut().unwrap();
            let mut obs = obs;
            obs.stop_at = None;
            obs.preempted = false;
            // What the report has counted already.
            obs.tests = 0;
            obs.test_s = 0.0;
            obs.diffs.clear();
            obs.page_times.clear();
            // The pages so far are part of the document already (the
            // later ones are stale until the run goes on).
            g.layer().observer = Some(obs);
            self.paused = Some(Paused {
                report: rep.clone(),
                t0,
            });
            return Ok(());
        }
        let base = obs.base;
        let mut pages: Vec<Page> = self.pages[..base.min(self.pages.len())].to_vec();
        pages.extend(obs.new_pages.iter().cloned());
        for (id, n) in &obs.taken {
            self.ck_pages.insert(*id, *n);
        }
        rep.rerun_pages += obs.new_pages.len();
        if let Some((j, old)) = obs.converged {
            rep.converged_at = Some(j);
            let edits = obs.edits.clone();
            let g = self.g.as_mut().unwrap();
            let rec_old = g
                .pending_record(old)
                .ok_or("no record at the convergence point")?;
            let in_remap = |path: &str, off: u64| -> u64 {
                match edits.iter().find(|e| e.path == path) {
                    Some(e) => e.remap(off),
                    None => off,
                }
            };
            let new_id = obs.taken.last().map(|(i, _)| *i).ok_or("no checkpoint")?;
            let reloc = Reloc {
                threshold: pdf_len(&rec_old) as i64,
                delta: pdf_len(&g.record_of(new_id)?) as i64 - pdf_len(&rec_old) as i64,
                overrides: obs.positions.clone(),
                rebuild_rs: true,
            };
            g.redo_to_remapped(old, &in_remap)?;
            // the new run's extra characters, into the old run's states
            // from the convergence point on (see `same_words`)
            for &(off, bits) in &obs.char_or {
                g.arena.or_from(old, off, bits)?;
            }
            // The old run's checkpoints from the convergence point on hold
            // its PDF file positions: correct them whenever one is restored.
            let chain = g.checkpoints();
            if let Some(at) = chain.iter().position(|&i| i == old) {
                for &id in &chain[at..] {
                    self.reloc.entry(id).or_default().push(reloc.clone());
                }
            }
            // The journal: the new run's so far, then the old run's after
            // the convergence point.
            let mut jn = system::record_reads_into(None).unwrap_or_default();
            if let Some(jo) = &self.journal {
                for f in &jo.files[rec_old.reads.0.min(jo.files.len())..] {
                    jn.files.push(f.clone());
                    jn.mark_seen(&f.path);
                }
                jn.lookups
                    .extend_from_slice(&jo.lookups[rec_old.reads.1.min(jo.lookups.len())..]);
                jn.outputs
                    .extend_from_slice(&jo.outputs[rec_old.reads.2.min(jo.outputs.len())..]);
            }
            // The old run's later pages.
            let old_base = self.pages.len().min(j);
            pages.extend(self.pages[old_base..].iter().cloned());
            self.pages = pages;
            // `\end{document}` re-runs: from the old run's last page's
            // checkpoint (later ones may be past its re-read of the .aux).
            let last = self.end_point().ok_or("no checkpoint")?;
            let g = self.g.as_mut().unwrap();
            let last_pages = *self.ck_pages.get(&last).unwrap_or(&self.pages.len());
            let rec_last = g.record_of(last)?;
            g.restore_discard(last)?;
            if let Some(rs) = self.reloc.get(&last) {
                for x in rs {
                    x.apply(g);
                }
            }
            for p in self.defpatch.get(&last).cloned().unwrap_or_default() {
                crate::readset::apply_patch(g, &p)?;
            }
            system::record_reads_into(Some(truncate_journal(&jn, rec_last.reads)));
            self.pages.truncate(last_pages);
            let mut o2 = self.observer(t0, last_pages, None);
            o2.pdf = rec_last.files.iter().find_map(|f| match &f.stream {
                Stream::Out { path, len } if path.ends_with(".pdf") => Some((path.clone(), *len)),
                _ => None,
            });
            // (after an unfinished old run -- a preempted one kept by
            // `settle_paused` -- this is the rest of the document)
            o2.preempt = self.preempt.clone();
            let g = self.g.as_mut().unwrap();
            g.layer().observer = Some(Box::new(o2));
            let st = g.resume_to_end().inspect_err(|_| {
                system::record_reads_into(None);
            })?;
            let mut o2: Box<Obs> = g
                .layer()
                .observer
                .take()
                .and_then(|o| o.into_any().downcast::<Obs>().ok())
                .ok_or("the run lost its observer")?;
            if st == STOPPED {
                // preempted: `finish` completes it (`after_run` with this
                // observer), or the next compile settles it
                rep.status = st;
                rep.paused = true;
                rep.preempted = true;
                rep.rerun_pages += o2.new_pages.len();
                rep.pages = o2.pages_so_far();
                rep.total_s = t0.elapsed().as_secs_f64();
                o2.preempted = false;
                o2.tests = 0;
                o2.test_s = 0.0;
                o2.diffs.clear();
                o2.page_times.clear();
                g.layer().observer = Some(o2);
                self.paused = Some(Paused {
                    report: rep.clone(),
                    t0,
                });
                return Ok(());
            }
            self.pages.extend(o2.new_pages.iter().cloned());
            for (id, n) in &o2.taken {
                self.ck_pages.insert(*id, *n);
            }
            rep.status = st;
        } else {
            self.pages = pages;
            rep.status = status;
            let g = self.g.as_mut().unwrap();
            g.abandon_pending();
        }
        self.journal = system::record_reads_into(None);
        if let Some(j) = &self.journal {
            self.lookup_dirs = j.dirs.clone();
        }
        // The anchor of incremental runs, taken by a cold run: the `.aux`
        // point if there was one (inside `\document`, before the `.aux` is
        // read), else S₀. A file the run read before it is covered by its
        // key; one read after it (the `.aux`) by the journal, like the
        // document's own files.
        let g = self.g.as_mut().unwrap();
        if self.s0.is_none() {
            let (s0_id, s0_reads) = {
                let l = g.layer();
                (l.aux_point.or(l.s0), l.s0_reads.take())
            };
            let _ = s0_reads;
            if let (Some(id), Some(j)) = (s0_id, self.journal.as_ref()) {
                let rec = g.record_of(id)?;
                match host::make_key(g, &rec, j, self.clock, &self.first_line) {
                    Ok(key) => {
                        self.s0 = Some(host::S0 { id, key });
                        self.ck_pages.insert(id, 0);
                        let open: Vec<String> = rec
                            .files
                            .iter()
                            .filter_map(|f| match &f.stream {
                                Stream::In { path, .. } => Some(path.clone()),
                                _ => None,
                            })
                            .collect();
                        self.key_cover = (rec.reads.0, open);
                    }
                    Err(e) => eprintln!("flashtex-host: no S0: {e}"),
                }
            }
        }
        // Forget page counts of checkpoints no longer retained.
        let ids: std::collections::HashSet<CheckpointId> = g.checkpoints().into_iter().collect();
        self.ck_pages.retain(|k, _| ids.contains(k));
        self.defpatch.retain(|k, _| ids.contains(k));
        for p in self.pages.iter_mut() {
            if p.ckpt.is_some_and(|c| !ids.contains(&c)) {
                p.ckpt = None;
            }
        }
        self.enforce_budget();
        let g = self.g.as_mut().unwrap();
        rep.pages = self.pages.len();
        rep.log_bytes = g.arena.log_bytes();
        rep.checkpoints = g.checkpoints().len();
        rep.total_s = t0.elapsed().as_secs_f64();
        if rep.page_s == 0.0 {
            rep.page_s = rep.total_s;
        }
        Ok(())
    }

    /// Where `\end{document}` re-runs from: the checkpoint of the newest
    /// page whose checkpoint is retained (checkpoints after the last page
    /// may be inside `\end{document}`, past its re-read of the `.aux`),
    /// else the anchor.
    fn end_point(&self) -> Option<CheckpointId> {
        self.pages
            .iter()
            .rev()
            .find_map(|p| p.ckpt)
            .or_else(|| self.s0.as_ref().map(|s| s.id))
    }

    /// Keep the undo logs within the budget (DESIGN.md §5.2); see [`thin`].
    fn enforce_budget(&mut self) {
        let cursor = self.cursor;
        let s0 = self.s0.as_ref().map(|s| s.id);
        let budget = self.opts.budget;
        let Some(g) = self.g.as_mut() else { return };
        let pages: HashMap<CheckpointId, usize> = self
            .pages
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.ckpt.map(|c| (c, i + 1)))
            .collect();
        thin(g, budget, cursor, s0, None, &pages, &self.ck_pages);
        let ids: std::collections::HashSet<CheckpointId> = g.checkpoints().into_iter().collect();
        self.ck_pages.retain(|k, _| ids.contains(k));
        self.defpatch.retain(|k, _| ids.contains(k));
        for p in self.pages.iter_mut() {
            if p.ckpt.is_some_and(|c| !ids.contains(&c)) {
                p.ckpt = None;
            }
        }
    }
}

/// Pages around the cursor whose checkpoints are all kept.
const DENSE: usize = 16;

/// Segment checkpoints at least this far apart by default (seconds of
/// engine time). Measured on the benchmark documents
/// (docs/evidence/p4-l5-2026-09-29): every `build_page` with a line read
/// since the last checkpoint is ~15 checkpoints a page on the "full"
/// documents and costs more than it saves; 0.5 ms keeps two to three a
/// page there (one on "plain" pages) for ~3% more instructions, and a
/// restart re-runs about one paragraph before the edit instead of the
/// page's start.
pub const DEFAULT_SEGMENT_S: f64 = 0.0005;

/// DESIGN.md §5.5: at most this many passes per compile.
pub const MAX_PASSES: usize = 5;

/// Drop checkpoints until the undo logs fit `budget` (DESIGN.md §5.2:
/// dense near the cursor, log-spaced elsewhere, the spacing driven by the
/// budget). Within `DENSE` pages of the cursor every checkpoint stays;
/// further out only page checkpoints stay, every `s * 2^k`-th page at a
/// distance in `[DENSE * 2^k, DENSE * 2^(k+1))`, with the base spacing
/// `s` = 1, 2, 3, 4, 6, 8, ... raised until the logs fit. S₀, the newest
/// checkpoint and `keep_also` are always kept. `pages` maps a page
/// checkpoint to its page, `ck_pages` any checkpoint to the pages before it.
fn thin(
    g: &mut Globals,
    budget: usize,
    cursor: usize,
    s0: Option<CheckpointId>,
    keep_also: Option<CheckpointId>,
    pages: &HashMap<CheckpointId, usize>,
    ck_pages: &HashMap<CheckpointId, usize>,
) {
    if g.arena.log_bytes() <= budget {
        return;
    }
    let aux_done = g.layer().aux_done;
    // the last page's checkpoint: where `\end{document}` re-runs from
    let last_page = pages.iter().max_by_key(|(_, &j)| j).map(|(&c, _)| c);
    for s in [1usize, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 1 << 20] {
        let keep = |id: CheckpointId| -> bool {
            if Some(id) == s0
                || Some(id) == keep_also
                || Some(id) == aux_done
                || Some(id) == last_page
            {
                return true;
            }
            match pages.get(&id) {
                Some(&j) => {
                    let d = j.abs_diff(cursor);
                    if d <= DENSE {
                        return true;
                    }
                    let k = (usize::BITS - 1 - (d / DENSE).leading_zeros()) as usize;
                    j % (s << k.min(40)).max(1) == 0
                }
                None => ck_pages
                    .get(&id)
                    .is_some_and(|&p| s == 1 && p.abs_diff(cursor) <= DENSE),
            }
        };
        g.retain_checkpoints(&keep);
        if g.arena.log_bytes() <= budget / 20 * 19 {
            break;
        }
    }
}

fn pdf_len(r: &ExtRecord) -> u64 {
    r.files
        .iter()
        .find_map(|f| match &f.stream {
            Stream::Out { path, len } if path.ends_with(".pdf") => Some(*len),
            _ => None,
        })
        .unwrap_or(0)
}

/// The `obj_offset` words (byte offset in the space, value) of every object
/// the live run wrote at or after `from` in the PDF file: where the run
/// that converges put them (`Reloc::overrides`).
fn new_positions(g: &Globals, from: u64) -> Vec<(usize, u64)> {
    let Some(r) = g.arena.regions.iter().find(|r| r.name == "obj_tab") else {
        return vec![];
    };
    let size = std::mem::size_of::<crate::generated::types::obj_entry>();
    let at = std::mem::offset_of!(crate::generated::types::obj_entry, int2);
    let n = (g.obj_ptr.max(0) as usize).min(g.obj_tab.len().saturating_sub(1));
    (1..=n)
        .filter(|&k| g.obj_tab[k].int3 == -1 && g.obj_tab[k].int2 >= from as i64)
        .map(|k| (r.off + k * size + at, g.obj_tab[k].int2 as u64))
        .collect()
}

/// The PDF file position corrections for a checkpoint an old run took after
/// a point where a new run converged with it (DESIGN.md §5.3: file
/// positions are relocatable). In the spliced file the old run's bytes from
/// `threshold` (its length at the convergence point) on lie `delta` further;
/// the objects the new run wrote before converging lie where it wrote them
/// (`overrides`: their `obj_offset` words, from the convergence test).
#[derive(Clone, Debug)]
struct Reloc {
    threshold: i64,
    delta: i64,
    overrides: Vec<(usize, u64)>,
    /// The checkpoint's `rs_seen` is the old run's, its read-set the
    /// spliced one: rebuild the first from the second.
    rebuild_rs: bool,
}

impl Reloc {
    fn apply(&self, g: &mut Globals) {
        let (t, d) = (self.threshold, self.delta);
        let mv = |x: i64| if x >= t { x + d } else { x };
        if d != 0 {
            g.pdf_gone = mv(g.pdf_gone);
            if g.pdf_save_offset > 0 {
                g.pdf_save_offset = mv(g.pdf_save_offset);
            }
            if g.pdf_stream_length_offset > 0 {
                g.pdf_stream_length_offset = mv(g.pdf_stream_length_offset);
            }
            let n = (g.obj_ptr.max(0) as usize).min(g.obj_tab.len().saturating_sub(1));
            for k in 1..=n {
                let e = g.obj_tab[k];
                if e.int3 == -1 && e.int2 >= t {
                    g.obj_tab[k].int2 = e.int2 + d;
                }
            }
        }
        for &(off, v) in &self.overrides {
            g.arena.write_through(off, &v.to_le_bytes());
        }
        if self.rebuild_rs && g.rs_on {
            crate::readset::rebuild_seen(g);
        }
    }
}

// Silence unused warnings for items only the host binary uses.
#[allow(dead_code)]
fn _unused(_: &FileRead, _: &Key) {}

#[cfg(test)]
mod tests {
    use super::diff_edit;

    /// `diff_edit` against the byte-at-a-time definition, on edits of every
    /// kind near block boundaries and at the ends.
    #[test]
    fn diff_edit_matches_the_definition() {
        let naive = |old: &[u8], new: &[u8]| {
            let p = old.iter().zip(new).take_while(|(a, b)| a == b).count();
            let max_s = old.len().min(new.len()) - p;
            let s = old
                .iter()
                .rev()
                .zip(new.iter().rev())
                .take(max_s)
                .take_while(|(a, b)| a == b)
                .count();
            (
                p as u64,
                (old.len() - p - s) as u64,
                (new.len() - p - s) as u64,
            )
        };
        let mut seed = 12345u64;
        let mut rnd = |n: usize| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as usize % n.max(1)
        };
        for len in [0usize, 1, 5, 255, 256, 257, 511, 512, 513, 1000, 4096] {
            let base: Vec<u8> = (0..len).map(|i| b"ab\nxy"[i % 5]).collect();
            for _ in 0..200 {
                let at = rnd(len + 1);
                let del = rnd(len - at + 1).min(3);
                let ins: Vec<u8> = (0..rnd(4)).map(|_| b"abx\n"[rnd(4)]).collect();
                let mut new = base.clone();
                new.splice(at..at + del, ins.iter().copied());
                let e = diff_edit("f", &base, &new);
                assert_eq!(
                    (e.prefix, e.old_mid, e.new_mid),
                    naive(&base, &new),
                    "len {len} at {at} del {del} ins {ins:?}"
                );
            }
        }
    }
}
