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

/// The edit that turns `old` into `new` (common prefix and suffix).
pub fn diff_edit(path: &str, old: &[u8], new: &[u8]) -> Edit {
    let p = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let max_s = old.len().min(new.len()) - p;
    let s = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(max_s)
        .take_while(|(a, b)| a == b)
        .count();
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
    /// Test convergence after each page of an incremental run.
    pub converge: bool,
    /// Print what differs at each convergence test to stderr.
    pub debug: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            preview: true,
            budget: 1 << 30,
            timed_s: 0.020,
            converge: true,
            debug: std::env::var_os("FLASHTEX_INCR_DEBUG").is_some(),
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
    /// The page after which the run converged with the old one.
    pub converged_at: Option<usize>,
    /// Pages this compile typeset.
    pub rerun_pages: usize,
    pub pages: usize,
    pub find_s: f64,
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
    /// Each page this compile shipped, with the time since its start.
    pub page_times: Vec<(usize, f64)>,
}

impl Report {
    pub fn json(&self) -> String {
        format!(
            "{{\"mode\":\"{}\",\"cold_reason\":{},\"status\":{},\"paused\":{},\"restart_pages\":{},\"converged_at\":{},\"rerun_pages\":{},\"pages\":{},\"find_s\":{:.6},\"restore_s\":{:.6},\"page_s\":{:.6},\"total_s\":{:.6},\"tests\":{},\"test_s\":{:.6},\"log_bytes\":{},\"checkpoints\":{},\"diffs\":{:?},\"page_times\":[{}]}}",
            self.mode,
            self.cold_reason
                .as_ref()
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|| "null".into()),
            self.status,
            self.paused,
            self.restart_pages,
            self.converged_at
                .map(|p| p.to_string())
                .unwrap_or_else(|| "null".into()),
            self.rerun_pages,
            self.pages,
            self.find_s,
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
                .map(|(p, t)| format!("[{p},{t:.6}]"))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

/// What the observer of a run knows and collects.
struct Obs {
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
    page_times: Vec<(usize, f64)>,
    /// The old run's `last_byte_reads` at its end.
    old_last_byte_reads_end: Option<u64>,
}

impl Obs {
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
        // (b) nothing the old run reads from here on has changed
        let from = o.reads.0.min(self.old_journal_files.len());
        if let Some(p) = self.old_journal_files[from..]
            .iter()
            .find(|p| self.changed.contains(p))
        {
            return Err(format!("the old run reads the changed {p} later"));
        }
        // the C parts
        if !new.cstate.same_as(&o.cstate) {
            return Err("pdfTeX's C-part state differs".into());
        }
        // the word space
        let d = g.diff_pending(old)?;
        if d.differing.is_empty() {
            return Ok(());
        }
        let words = crate::statediff::words(g, &d);
        let last_byte_dead = self.old_last_byte_reads_after(&o);
        let layout = crate::statediff::scalar_layout(g);
        let g: &Globals = g;
        let (_pos, left): (Vec<_>, Vec<_>) = words
            .into_iter()
            .filter(|w| !dead_word(g, w))
            .filter(|w| !(last_byte_dead && w.scalar == Some("pdf_last_byte")))
            .partition(|w| position_only(g, w));
        let left = drop_free_mem(g, &d, &layout, left);
        if left.is_empty() {
            return Ok(());
        }
        let mut s = format!(
            "{} words differ in {} of {} chunks compared: {}",
            left.len(),
            d.differing.len(),
            d.compared,
            crate::statediff::summary(&left)
        );
        if self.debug {
            for w in left.iter().take(12) {
                s.push_str(&format!("\n    {w}"));
            }
        }
        Err(s)
    }
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
    match w.scalar {
        // pdftex.web: the length of the last stream, set by every
        // `pdf_end_stream` (or `write_zip`) before its one read there
        Some("pdf_stream_length") => return true,
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
fn drop_free_mem(
    g: &Globals,
    d: &crate::arena::ChunkDiff,
    layout: &[crate::statediff::ScalarSlot],
    left: Vec<crate::statediff::WordDiff>,
) -> Vec<crate::statediff::WordDiff> {
    if !left
        .iter()
        .any(|w| w.region == "mem" || matches!(w.scalar, Some("avail" | "rover")))
    {
        return left;
    }
    let mem_off = match g.arena.regions.iter().find(|r| r.name == "mem") {
        Some(r) => r.off,
        None => return left,
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
        return left;
    };
    let old_word = |p: usize| d.old_word(&g.arena, mem_off + p * 8);
    let new_word = |p: usize| g.mem[p].to_bits();
    let Some(fo) = free_cells(&old_word, av, ro, lo, hi, me) else {
        return left;
    };
    let Some(fnw) = free_cells(
        &new_word,
        g.avail,
        g.rover,
        g.lo_mem_max,
        g.hi_mem_min,
        g.mem_end,
    ) else {
        return left;
    };
    let free_both = |p: usize| (fo[p >> 6] & fnw[p >> 6]) >> (p & 63) & 1 == 1;
    left.into_iter()
        .filter(|w| {
            if matches!(w.scalar, Some("avail" | "rover")) {
                return false;
            }
            !(w.region == "mem" && free_both(w.index))
        })
        .collect()
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
        if why != Point::Shipout {
            self.taken.push((id, self.pages_so_far()));
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
        self.page_s = self.t0.elapsed().as_secs_f64();
        self.page_times.push((j, self.page_s));
        if self.converge {
            if let Some(old) = self
                .old_pages
                .get(j - self.base - 1)
                .and_then(|p| p.ckpt)
                .filter(|o| g.pending_ids().contains(o))
            {
                if self.converged(g, &rec, old) {
                    self.converged = Some((j, old));
                    self.positions = new_positions(g, self.pdf_len_r);
                    return Action::Stop;
                }
            }
        }
        if self.stop_at == Some(j) {
            return Action::Stop;
        }
        Action::Continue
    }
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
    /// The directories the journal's lookups depend on (`Key::dirs`).
    lookup_dirs: Vec<(String, StatSig)>,
}

struct Paused {
    report: Report,
    t0: Instant,
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
        system::configure(o);
        system::capture_terminal(true);
        crate::pdftex::set_preview(opts.preview);
        Session {
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
            lookup_dirs: vec![],
        }
    }

    pub fn terminal(&self) -> Vec<u8> {
        system::terminal_bytes()
    }

    pub fn is_paused(&self) -> bool {
        self.paused.is_some()
    }

    /// Compile the document as it is now. With `stop_at`, stop once that
    /// page has been shipped (L4); `finish` continues.
    pub fn compile(&mut self, stop_at: Option<usize>) -> Result<Report, String> {
        let t0 = Instant::now();
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
        let s0_id = s0.id;
        let (edits, changed, bad_lookup) = match self.changes() {
            Ok(x) => x,
            Err(why) => return self.cold(t0, stop_at, Some(why)),
        };
        if changed.is_empty() && bad_lookup.is_none() {
            return Ok(Report {
                mode: "unchanged".into(),
                pages: self.pages.len(),
                total_s: t0.elapsed().as_secs_f64(),
                ..Report::default()
            });
        }
        let r = match self.restart_point(&edits, &changed, bad_lookup) {
            Some(r) => r,
            None => s0_id,
        };
        let find_s = t0.elapsed().as_secs_f64();
        self.incremental(t0, r, edits, changed, stop_at, find_s)
    }

    /// The files that changed since the last run: edits of the user's files
    /// (where their old content is known), every changed path, and the
    /// index of the first lookup that finds something else now.
    #[allow(clippy::type_complexity)]
    fn changes(&mut self) -> Result<(Vec<Edit>, Vec<String>, Option<usize>), String> {
        let j = self.journal.as_mut().ok_or("no journal")?;
        let mut edits = vec![];
        let mut changed = vec![];
        for f in j.files.iter_mut() {
            if StatSig::of(&f.path).as_ref() == Some(&f.stat) {
                continue;
            }
            let now = std::fs::read(&f.path).ok();
            let h = now.as_deref().map(hash128);
            if h == Some(f.hash) {
                if let Some(s) = StatSig::of(&f.path) {
                    f.stat = s;
                }
                continue;
            }
            if !changed.contains(&f.path) {
                changed.push(f.path.clone());
                match (&f.content, &now) {
                    (Some(old), Some(new)) => edits.push(diff_edit(&f.path, old, new)),
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
                f.hash = hash128(&n);
                f.stat = StatSig::of(&f.path).unwrap_or_default();
                if f.content.is_some() {
                    f.content = Some(std::sync::Arc::new(n));
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
                // Opened more than once before this point: unknown.
                if r.files
                    .iter()
                    .filter(|f| matches!(&f.stream, Stream::In { path, .. } if path == p))
                    .count()
                    != 1
                {
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
            t0,
            base,
            new_pages: vec![],
            pdf: None,
            old_pages: vec![],
            edits: vec![],
            changed: vec![],
            old_journal_files: vec![],
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
            page_times: vec![],
            old_last_byte_reads_end: None,
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
        self.s0 = None;
        self.g = None;
        self.pages.clear();
        self.ck_pages.clear();
        self.journal = None;
        crate::pdftex::reset_state();
        crate::pdftex::utils::arm_pinned_seed();
        system::truncate_terminal(0);
        system::truncate_external_effects(0);
        system::record_reads_into(Some(ReadLog::keeping_content()));
        system::set_command_line(vec![self.first_line.clone()]);
        let mut g = Globals::new();
        g.arm_begin_document();
        g.checkpoint_every_shipout(true);
        g.layer().timed_s = self.opts.timed_s;
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

    fn incremental(
        &mut self,
        t0: Instant,
        r: CheckpointId,
        edits: Vec<Edit>,
        changed: Vec<String>,
        stop_at: Option<usize>,
        find_s: f64,
    ) -> Result<Report, String> {
        let base = *self
            .ck_pages
            .get(&r)
            .ok_or("restart point without a page count")?;
        self.cursor = base;
        let journal = self.journal.as_ref().ok_or("no journal")?;
        let old_files: Vec<String> = journal.files.iter().map(|f| f.path.clone()).collect();
        let jr = journal.clone();
        let mut obs = self.observer(t0, base, stop_at);
        // The live state is the old run's end.
        obs.old_last_byte_reads_end = Some(crate::pdftex::last_byte_reads());
        let t1 = Instant::now();
        let g = self.g.as_mut().unwrap();
        let rec = g.record_of(r)?;
        // Output files written and closed before `r` are the new run's own
        // too: nothing to put back. Restore, keeping the old future.
        g.restore(r)?;
        if let Some(rs) = self.reloc.get(&r) {
            for x in rs {
                x.apply(g);
            }
        }
        system::record_reads_into(Some(truncate_journal(&jr, rec.reads)));
        let restore_s = t1.elapsed().as_secs_f64();
        obs.old_pages = self.pages[base..].to_vec();
        obs.edits = edits;
        obs.changed = changed;
        obs.old_journal_files = old_files;
        obs.converge = self.opts.converge;
        obs.pdf = rec.files.iter().find_map(|f| match &f.stream {
            Stream::Out { path, len } if path.ends_with(".pdf") => Some((path.clone(), *len)),
            _ => None,
        });
        obs.pdf_len_r = pdf_len(&rec);
        g.layer().observer = Some(Box::new(obs));
        g.checkpoint_every_shipout(true);
        g.layer().timed_s = self.opts.timed_s;
        // The pages before `r` stay; the rest are the old run's until
        // redone.
        let status = g.resume_to_end().inspect_err(|_| {
            system::record_reads_into(None);
        })?;
        let mut rep = Report {
            mode: "incremental".into(),
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
        let Some(p) = self.paused.take() else {
            return Err("no run is paused".into());
        };
        let g = self.g.as_mut().unwrap();
        if let Some(o) = g.layer().observer.as_mut() {
            let _ = o;
        }
        let status = g.resume_to_end().inspect_err(|_| {
            system::record_reads_into(None);
        })?;
        let mut rep = p.report;
        rep.mode = "continued".into();
        self.after_run(p.t0, status, &mut rep)?;
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
        rep.diffs.extend(obs.diffs.iter().cloned());
        rep.page_s = if rep.page_s > 0.0 {
            rep.page_s
        } else {
            obs.page_s
        };
        if status == STOPPED && obs.converged.is_none() {
            // Paused at the requested page.
            rep.status = status;
            rep.paused = true;
            rep.page_s = obs.page_s;
            rep.rerun_pages += obs.new_pages.len();
            rep.pages = obs.pages_so_far().max(self.pages.len());
            rep.total_s = t0.elapsed().as_secs_f64();
            let g = self.g.as_mut().unwrap();
            let mut obs = obs;
            obs.stop_at = None;
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
            };
            g.redo_to_remapped(old, &in_remap)?;
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
            // `\end{document}` re-runs: from the old run's last checkpoint.
            let g = self.g.as_mut().unwrap();
            let last = *g.checkpoints().last().ok_or("no checkpoint")?;
            let last_pages = *self.ck_pages.get(&last).unwrap_or(&self.pages.len());
            let rec_last = g.record_of(last)?;
            g.restore_discard(last)?;
            if let Some(rs) = self.reloc.get(&last) {
                for x in rs {
                    x.apply(g);
                }
            }
            system::record_reads_into(Some(truncate_journal(&jn, rec_last.reads)));
            self.pages.truncate(last_pages);
            let mut o2 = self.observer(t0, last_pages, None);
            o2.pdf = rec_last.files.iter().find_map(|f| match &f.stream {
                Stream::Out { path, len } if path.ends_with(".pdf") => Some((path.clone(), *len)),
                _ => None,
            });
            let g = self.g.as_mut().unwrap();
            g.layer().observer = Some(Box::new(o2));
            let st = g.resume_to_end().inspect_err(|_| {
                system::record_reads_into(None);
            })?;
            let o2: Box<Obs> = g
                .layer()
                .observer
                .take()
                .and_then(|o| o.into_any().downcast::<Obs>().ok())
                .ok_or("the run lost its observer")?;
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
        // S₀: taken by a cold run.
        let g = self.g.as_mut().unwrap();
        if self.s0.is_none() {
            let (s0_id, s0_reads) = {
                let l = g.layer();
                (l.s0, l.s0_reads.take())
            };
            let _ = s0_reads;
            if let (Some(id), Some(j)) = (s0_id, self.journal.as_ref()) {
                let rec = g.record_of(id)?;
                match host::make_key(g, &rec, j, self.clock, &self.first_line) {
                    Ok(key) => {
                        self.s0 = Some(host::S0 { id, key });
                        self.ck_pages.insert(id, 0);
                    }
                    Err(e) => eprintln!("flashtex-host: no S0: {e}"),
                }
            }
        }
        // Forget page counts of checkpoints no longer retained.
        let ids: std::collections::HashSet<CheckpointId> = g.checkpoints().into_iter().collect();
        self.ck_pages.retain(|k, _| ids.contains(k));
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

    /// Keep the undo logs within the budget (DESIGN.md §5.2): drop
    /// checkpoints far from the cursor first, log-spaced, doubling the
    /// spacing until the logs fit. S₀ and the newest are always kept.
    fn enforce_budget(&mut self) {
        let budget = self.opts.budget;
        let cursor = self.cursor;
        let Some(g) = self.g.as_mut() else { return };
        if g.arena.log_bytes() <= budget {
            return;
        }
        let s0 = self.s0.as_ref().map(|s| s.id);
        let mut factor = 1usize;
        while g.arena.log_bytes() > budget * 9 / 10 && factor < 1 << 20 {
            let ck = &self.ck_pages;
            let keep = |id: CheckpointId| -> bool {
                if Some(id) == s0 {
                    return true;
                }
                let Some(&p) = ck.get(&id) else {
                    return false;
                };
                let d = p.abs_diff(cursor);
                if d <= 16 / factor.min(16) {
                    return true;
                }
                // spacing grows with the distance's octave, times `factor`
                let octave = (usize::BITS - (d / 16 + 1).leading_zeros()) as usize;
                let spacing = (1usize << octave.min(20)) * factor;
                p % spacing == 0
            };
            g.retain_checkpoints(&keep);
            factor *= 2;
        }
        let ids: std::collections::HashSet<CheckpointId> = g.checkpoints().into_iter().collect();
        self.ck_pages.retain(|k, _| ids.contains(k));
        for p in self.pages.iter_mut() {
            if p.ckpt.is_some_and(|c| !ids.contains(&c)) {
                p.ckpt = None;
            }
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
    }
}

// Silence unused warnings for items only the host binary uses.
#[allow(dead_code)]
fn _unused(_: &FileRead, _: &Key) {}
