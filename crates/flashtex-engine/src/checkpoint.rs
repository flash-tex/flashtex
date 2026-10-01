//! Checkpoints of the whole engine (DESIGN.md §5.1, §5.2).
//!
//! A checkpoint is the word space (`arena.rs`: every array global, plus the
//! scalar globals spilled into its first region) and an [`ExtRecord`] of the
//! state outside it: where every file global's stream stands, pdfTeX's
//! C-part state, the captured terminal, and the external-effect log.
//!
//! * [`Globals::checkpoint`] seals the word space's undo log and records the
//!   host state. It is valid wherever the engine is between commands:
//!   before a run, after one, and at `big_switch`, where
//!   `changes/checkpoint.ch` calls [`Globals::flashtex_checkpoint_hook`].
//! * [`Globals::restore`] puts a retained checkpoint back and keeps the run it
//!   leaves as a detached branch: its later checkpoints, a redo log of the
//!   word space and the tails of its output files.
//!   [`Globals::restore_discard`] is the plain restart that drops them.
//! * [`Globals::redo_to`] is the convergence jump (§5.3): when the live state
//!   equals the old run's at a checkpoint of the branch, jump to the old
//!   run's latest state and keep its checkpoints.
//! * [`Globals::resume_to_end`] continues a restored engine from
//!   `big_switch` to the end of the job.
//!
//! What is *not* restored, because it lives outside the engine and is not
//! the engine's state: files closed between two checkpoints keep what was
//! last written to them (a re-run rewrites them), and kpathsea's caches.

use crate::arena::{Branch, CheckpointId, Fill, Spill};
use crate::generated::globals::SCALAR_BYTES;
use crate::generated::Globals;
use crate::pdftex::CState;
use crate::system::{self, AlphaFile, ByteFile, FileSnap, FileVisit, Stream, WordFile};
use std::panic::AssertUnwindSafe;

/// The engine's state outside the word space, at a checkpoint.
#[derive(Clone)]
pub struct ExtRecord {
    /// Every file global, in `visit_files` order.
    pub files: Vec<FileSnap>,
    pub cstate: CState,
    pub terminal_len: usize,
    pub effects_len: usize,
    pub tex_input_type: bool,
    /// How much of the run's read journal (`system::ReadLog`: files,
    /// lookups, outputs opened) precedes this checkpoint.
    pub reads: (usize, usize, usize),
    /// `pdftex::last_byte_reads()` at this checkpoint.
    pub last_byte_reads: u64,
    /// `pdftex::matrix_uses()` at this checkpoint.
    pub matrix_uses: u64,
    /// How many first reads of control sequences the read-set holds
    /// (`crate::readset`, DESIGN.md §5.5).
    pub rs: usize,
}

crate::codec_struct!(ExtRecord {
    files,
    cstate,
    terminal_len,
    effects_len,
    tex_input_type,
    reads,
    last_byte_reads,
    matrix_uses,
    rs
});

/// Why a checkpoint was taken at `big_switch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    /// After `\document` expanded: S₀ (§5.1).
    BeginDocument,
    /// After a `\shipout` (§5.2).
    Shipout,
    /// After ~20 ms of engine time without one (§5.2: heavy pages).
    Timed,
    /// After `build_page` moved contributions to the current page, between
    /// shipouts (§5.2's checkpoints between pages, at §5.7's segment
    /// boundaries), at least `Layer::segment_s` after the last checkpoint.
    Segment,
    /// Inside `\document`, just after the `.aux` file was opened for
    /// reading (LaTeX's `\IfFileExists` test in `\@input`), before any of
    /// it was read: where a run restarts when only the `.aux` changed (the
    /// previous run rewrote it), instead of from the format.
    Aux,
    /// The first `big_switch` after that `.aux` file was closed: the `.aux`
    /// read is over (L5, `crate::readset`).
    AuxDone,
}

/// What an [`Observer`] asks of the run after a checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Continue,
    /// Stop the run here: `resume_to_end`/`run_to_end` return
    /// [`STOPPED`], with the engine at this checkpoint, between two
    /// commands, from where `resume_to_end` continues it.
    Stop,
}

/// Told about every checkpoint the hook takes after S₀ (L2-L4:
/// `crate::incr`).
pub trait Observer {
    fn on_checkpoint(&mut self, g: &mut Globals, id: CheckpointId, why: Point) -> Action;
    /// The observer as `Any`, to take it back after a run.
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any>;
}

/// The exit status of a run an [`Observer`] stopped.
pub const STOPPED: ExitStatus = -2;

/// The unwinding payload of a stopped run.
struct StopRun;

/// Values of `ckpt_request` (changes/checkpoint.ch).
const REQ_LOOKUP: i32 = 1;
const REQ_BEGIN_DOCUMENT: i32 = 2;
const REQ_SHIPOUT: i32 = 3;
const REQ_NOTE_SHIPOUT: i32 = 4;
const REQ_TIMED: i32 = 5;
const REQ_AUX: i32 = 6;
const REQ_SEGMENT: i32 = 7;
const REQ_AUX_DONE: i32 = 8;

/// `hash_base` (tex.web §222): `active_base + 256 + 256 + 1`, the same in
/// every configuration.
const HASH_BASE: i32 = 514;

struct Tail {
    path: String,
    base: u64,
    bytes: TailBytes,
    open_at_target: bool,
}

/// The old run's bytes of an output file from `base` on: read at the
/// restore, or -- where the file system can clone a file in O(1) (APFS's
/// `clonefile`) -- a clone of the whole file, read only if `redo_to` needs
/// it. A restore then costs the same whatever the size of the PDF.
enum TailBytes {
    /// The bytes, and the file they came from (whose spare buffer they
    /// return to when dropped).
    Read(Vec<u8>, String),
    Clone(String),
}

impl TailBytes {
    fn take(path: &str, from: u64) -> Result<TailBytes, String> {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Not in the document's directory: a new file there would change
        // its listing, which the lookups' key watches (`host::Key::dirs`).
        let dst = std::env::temp_dir()
            .join(format!("flashtex-tail-{}-{n}", std::process::id()))
            .to_string_lossy()
            .into_owned();
        if system::clone_file(path, &dst) {
            return Ok(TailBytes::Clone(dst));
        }
        let buf = SPARE_TAILS.with(|s| s.borrow_mut().remove(path).unwrap_or_default());
        Ok(TailBytes::Read(
            read_tail_into(path, from, buf)?,
            path.to_string(),
        ))
    }

    /// Bytes `skip..` of the tail that starts at `base`.
    fn get(&self, base: u64, skip: u64) -> Result<Vec<u8>, String> {
        match self {
            TailBytes::Read(b, _) => Ok(b.get(skip as usize..).unwrap_or(&[]).to_vec()),
            TailBytes::Clone(p) => read_tail(p, base + skip),
        }
    }
}

impl Drop for TailBytes {
    fn drop(&mut self) {
        match self {
            TailBytes::Clone(p) => {
                let _ = std::fs::remove_file(p);
            }
            // keep the buffer for the next restore's tail of the same file,
            // while the spares fit in SPARE_TAILS_MAX
            TailBytes::Read(b, path) => SPARE_TAILS.with(|s| {
                let mut s = s.borrow_mut();
                let others: usize = s
                    .iter()
                    .filter(|(k, _)| *k != path)
                    .map(|(_, v)| v.capacity())
                    .sum();
                if others + b.capacity() <= SPARE_TAILS_MAX {
                    s.insert(std::mem::take(path), std::mem::take(b));
                } else {
                    s.remove(path.as_str());
                }
            }),
        }
    }
}

thread_local! {
    /// The buffers of the last restore's output tails, by file, for the
    /// next restore's tails of the same files (`TailBytes::take` where files
    /// cannot be cloned: Linux). A 1,000-page preview PDF is 13-14 MB; a
    /// fresh buffer each keystroke had the kernel map and zero it again,
    /// once the host's heap was small enough for glibc to give the freed
    /// one back (review of #1300: 9-12 ms at p95 of a plain-1000 restore,
    /// against 2-5 ms with the old 5 GB heap).
    static SPARE_TAILS: std::cell::RefCell<std::collections::HashMap<String, Vec<u8>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// The most the spare tail buffers hold together. They are outside the undo
/// logs' budget (DESIGN.md §5.2), so they are capped instead: a PDF larger
/// than this reads into a fresh buffer each restore, as before.
const SPARE_TAILS_MAX: usize = 64 << 20;

/// Bytes the spare tail buffers hold (memory accounting).
pub fn spare_tail_bytes() -> usize {
    SPARE_TAILS.with(|s| s.borrow().values().map(|v| v.capacity()).sum())
}

/// A branch detached by `restore`, until `redo_to` or another restore.
pub struct Pending {
    branch: Branch,
    /// The old run's latest host state.
    live: ExtRecord,
    /// The old run's bytes of every file open for output at the restore
    /// target or at the old run's end: (path, length at the target -- 0
    /// if it was not open there --, the bytes from there on, open at the
    /// target).
    tails: Vec<Tail>,
    terminal_tail: (usize, Vec<u8>),
    /// Host records of the detached checkpoints.
    records: Vec<(CheckpointId, ExtRecord)>,
    /// The read-set of the old run (at its latest state).
    rs_old: crate::readset::ReadSet,
}

/// The checkpoint layer's bookkeeping, kept in `Globals::arena.extra`.
#[derive(Default)]
pub struct Layer {
    records: Vec<(CheckpointId, ExtRecord)>,
    pending: Option<Pending>,
    /// The control sequence whose expansion arms S₀ (`document`).
    arm_name: Option<Vec<u8>>,
    /// Checkpoints taken by the hook, in order, with why.
    pub taken: Vec<(CheckpointId, Point)>,
    /// S₀, once taken.
    pub s0: Option<CheckpointId>,
    /// The read-set when S₀ was taken.
    pub s0_reads: Option<system::ReadLog>,
    /// Take a checkpoint at the `.aux` point of this run (`Point::Aux`).
    pub want_aux_point: bool,
    /// The `.aux` point, once taken.
    pub aux_point: Option<CheckpointId>,
    /// Stop the run with `EngineExit(-1)` right after S₀ is taken.
    pub stop_at_s0: bool,
    /// Errors the hook met (a checkpoint it could not take).
    pub errors: Vec<String>,
    /// Hash of the whole word space at each checkpoint the hook takes, for
    /// the bit-identity tests (costly: off by default).
    pub hash_states: bool,
    pub state_hashes: Vec<(CheckpointId, [u64; 2])>,
    /// Time spent taking checkpoints, in seconds.
    pub seconds: f64,
    /// When the current run started, and how far into it S₀ was taken.
    pub run_started: Option<std::time::Instant>,
    pub s0_elapsed: f64,
    /// Seconds into the current run at which each shipout finished (the
    /// next `big_switch`), when noted.
    pub shipout_times: Vec<f64>,
    /// What checkpoints cost, and what they hold.
    pub stats: Stats,
    /// Told about every checkpoint after S₀.
    pub observer: Option<Box<dyn Observer>>,
    /// Take a checkpoint when this much engine time has passed without
    /// one (checked when a line is read: `system::input_ln`); 0 = never.
    pub timed_s: f64,
    /// When the last checkpoint was taken.
    pub last_checkpoint: Option<std::time::Instant>,
    /// Segment checkpoints (`Point::Segment`): the least engine time since
    /// the last checkpoint for one to be taken.
    pub segment_s: f64,
    /// Lines read (`input_ln`) so far, and when the last checkpoint was
    /// taken: a segment checkpoint needs a line read since the last one
    /// (two checkpoints with the same input consumed are the same restart
    /// point, and the later one is the better).
    pub lines: u64,
    pub lines_at_checkpoint: u64,
    /// The read-set of the run (`crate::readset`), and for the `.aux` it
    /// reads at the `.aux` point: its path, whether its close is still to
    /// be noted (a run from the `.aux` point), the read-set's length then,
    /// and the checkpoint after it (`Point::AuxDone`).
    pub rs: crate::readset::ReadSet,
    pub aux_path: Option<String>,
    pub aux_armed: bool,
    pub aux_close_rs: Option<usize>,
    pub aux_done: Option<CheckpointId>,
    aux_done_pending: bool,
}

/// Where the time of `checkpoint` goes, and how much of the word space each
/// interval wrote (DESIGN.md §5.2's sizing: chunks per page, by array).
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub count: u64,
    /// Flushing and recording the file streams.
    pub files_s: f64,
    /// Copying pdfTeX's C-part state.
    pub cstate_s: f64,
    /// Spilling the scalars and sealing the undo log.
    pub seal_s: f64,
    /// The same, in this thread's CPU time (the machine may be busy).
    pub seal_cpu_s: f64,
    /// Chunks written in the intervals the checkpoints closed.
    pub dirty_chunks: u64,
    /// Checkpoints taken between shipouts (`Point::Segment`), and those
    /// requested but not taken.
    pub segments: u64,
    pub segments_skipped: u64,
    /// The thread CPU time of the whole of `checkpoint` (the host state,
    /// the scalars, the seal).
    pub total_cpu_s: f64,
    /// The same, by the array each chunk starts in.
    pub dirty_by_region: std::collections::BTreeMap<&'static str, u64>,
}

impl Stats {
    pub fn json(&self) -> String {
        let mut regions: Vec<_> = self.dirty_by_region.iter().collect();
        regions.sort_by_key(|r| std::cmp::Reverse(*r.1));
        let top: Vec<String> = regions
            .iter()
            .take(8)
            .map(|(k, v)| format!("{k:?}:{v}"))
            .collect();
        format!(
            "{{\"checkpoints\":{},\"segments\":{},\"segments_skipped\":{},\"total_cpu_s\":{:.6},\"files_s\":{:.6},\"cstate_s\":{:.6},\"seal_s\":{:.6},\"seal_cpu_s\":{:.6},\"dirty_chunks\":{},\"dirty_by_region\":{{{}}}}}",
            self.count,
            self.segments,
            self.segments_skipped,
            self.total_cpu_s,
            self.files_s,
            self.cstate_s,
            self.seal_s,
            self.seal_cpu_s,
            self.dirty_chunks,
            top.join(",")
        )
    }
}

/// What `resume_to_end` and `run_to_end` return: the process exit status
/// pdfTeX would have ended with (-1 if the host stopped the run at S₀).
pub type ExitStatus = i32;

struct SnapFiles {
    out: Vec<FileSnap>,
    err: Option<String>,
}

impl SnapFiles {
    fn push(&mut self, r: Result<FileSnap, String>) {
        match r {
            Ok(s) => self.out.push(s),
            Err(e) => {
                self.err.get_or_insert(e);
                self.out.push(empty_snap());
            }
        }
    }
}

fn empty_snap() -> FileSnap {
    FileSnap {
        buf: 0,
        line: vec![],
        pos: 0,
        have_line: false,
        at_eof: false,
        err: 0,
        stream: Stream::None,
    }
}

impl FileVisit for SnapFiles {
    fn alpha(&mut self, f: &mut AlphaFile) {
        let r = f.snapshot();
        self.push(r)
    }
    fn byte(&mut self, f: &mut ByteFile) {
        let r = f.snapshot();
        self.push(r)
    }
    fn word(&mut self, f: &mut WordFile) {
        let r = f.snapshot();
        self.push(r)
    }
}

struct RestoreFiles<'a> {
    snaps: &'a [FileSnap],
    i: usize,
    err: Option<String>,
}

impl RestoreFiles<'_> {
    fn next(&mut self) -> &FileSnap {
        self.i += 1;
        &self.snaps[self.i - 1]
    }
    fn note(&mut self, r: Result<(), String>) {
        if let Err(e) = r {
            self.err.get_or_insert(e);
        }
    }
}

impl FileVisit for RestoreFiles<'_> {
    fn alpha(&mut self, f: &mut AlphaFile) {
        let s = self.next().clone();
        let r = f.restore(&s);
        self.note(r)
    }
    fn byte(&mut self, f: &mut ByteFile) {
        let s = self.next().clone();
        let r = f.restore(&s);
        self.note(r)
    }
    fn word(&mut self, f: &mut WordFile) {
        let s = self.next().clone();
        let r = f.restore(&s);
        self.note(r)
    }
}

/// Bytes `from..` of `path`.
/// `Err` if `rec` has a file open for output that a run opens more than
/// once (`Globals::restorable`).
fn outputs_restorable(rec: &ExtRecord) -> Result<(), String> {
    for f in &rec.files {
        if let Stream::Out { path, .. } = &f.stream {
            if system::volatile_output(path) {
                return Err(format!(
                    "{path} is open for output there, and a run rewrites it"
                ));
            }
        }
    }
    Ok(())
}

fn read_tail(path: &str, from: u64) -> Result<Vec<u8>, String> {
    read_tail_into(path, from, Vec::new())
}

/// Bytes `from..` of the file at `path`, read into `buf` (cleared; its
/// capacity is reused): only the tail is read, once.
fn read_tail_into(path: &str, from: u64, mut buf: Vec<u8>) -> Result<Vec<u8>, String> {
    use std::io::{Read, Seek, SeekFrom};
    let e = |x: std::io::Error| format!("{path}: {x}");
    let mut f = std::fs::File::open(path).map_err(e)?;
    let len = f.metadata().map_err(e)?.len();
    buf.clear();
    if from < len {
        buf.reserve((len - from) as usize);
        f.seek(SeekFrom::Start(from)).map_err(e)?;
        f.read_to_end(&mut buf).map_err(e)?;
    }
    Ok(buf)
}

impl Globals {
    fn put_layer(&mut self, l: Layer) {
        self.arena.extra = Some(Box::new(l));
    }

    /// The checkpoint layer's bookkeeping (created on first use).
    pub fn layer(&mut self) -> &mut Layer {
        if self.arena.extra.is_none() {
            self.put_layer(Layer::default());
        }
        self.arena
            .extra
            .as_mut()
            .unwrap()
            .downcast_mut::<Layer>()
            .expect("arena.extra holds the checkpoint layer")
    }

    fn layer_ref(&self) -> Option<&Layer> {
        self.arena.extra.as_ref()?.downcast_ref::<Layer>()
    }

    /// The live word space against the old run's at checkpoint `old` of the
    /// branch the last `restore` detached (the convergence test's first
    /// part, `Arena::diff_branch`). Spills the scalars first.
    pub fn diff_pending(&mut self, old: CheckpointId) -> Result<crate::arena::ChunkDiff, String> {
        self.spill_scalars();
        let l = self.layer_ref().ok_or("no checkpoint layer")?;
        let p = l.pending.as_ref().ok_or("no restore is pending")?;
        self.arena.diff_branch(&p.branch, old)
    }

    /// [`diff_pending`](Self::diff_pending), asking `stop` while it works:
    /// `Ok(None)` when it said to stop.
    pub fn diff_pending_until(
        &mut self,
        old: CheckpointId,
        stop: &mut dyn FnMut() -> bool,
    ) -> Result<Option<crate::arena::ChunkDiff>, String> {
        self.spill_scalars();
        let l = self.layer_ref().ok_or("no checkpoint layer")?;
        let p = l.pending.as_ref().ok_or("no restore is pending")?;
        self.arena.diff_branch_until(&p.branch, old, stop)
    }

    /// The host record of checkpoint `old` of the pending branch.
    pub fn pending_record(&self, old: CheckpointId) -> Option<ExtRecord> {
        let p = self.layer_ref()?.pending.as_ref()?;
        if p.branch.ids().first() == Some(&old) {
            return self
                .layer_ref()?
                .records
                .iter()
                .rev()
                .find(|(i, _)| *i == old)
                .map(|(_, r)| r.clone());
        }
        p.records
            .iter()
            .find(|(i, _)| *i == old)
            .map(|(_, r)| r.clone())
    }

    /// Set checkpoint `id`'s journal counts (a persisted S₀ opened with a
    /// journal rebuilt from its key, `incr::Session::open_s0`).
    pub fn set_record_reads(&mut self, id: CheckpointId, reads: (usize, usize, usize)) {
        for (i, r) in self.layer().records.iter_mut() {
            if *i == id {
                r.reads = reads;
            }
        }
    }

    /// The checkpoints of the pending branch (the restore target first).
    pub fn pending_ids(&self) -> Vec<CheckpointId> {
        self.layer_ref()
            .and_then(|l| l.pending.as_ref())
            .map(|p| p.branch.ids().to_vec())
            .unwrap_or_default()
    }

    /// Drop the pending branch (the old run's future): the new run will
    /// not converge.
    pub fn abandon_pending(&mut self) {
        self.drop_pending();
    }

    /// Copy the scalar globals into the word space's scalar region, through
    /// the barrier.
    pub fn spill_scalars(&mut self) {
        let mut sp = Spill {
            buf: Vec::with_capacity(SCALAR_BYTES),
        };
        self.visit_scalars(&mut sp);
        assert_eq!(sp.buf.len(), SCALAR_BYTES);
        self.arena.write_through(0, &sp.buf);
    }

    /// Read the scalar globals back from the scalar region.
    pub fn fill_scalars(&mut self) {
        let img = self.arena.read(0, SCALAR_BYTES).to_vec();
        let mut f = Fill { src: &img, pos: 0 };
        self.visit_scalars(&mut f);
    }

    /// The whole engine state as bytes (scalars spilled first): what "bit
    /// identical" compares.
    pub fn state_bytes(&mut self) -> &[u8] {
        self.spill_scalars();
        self.arena.bytes()
    }

    /// A hash of `state_bytes` (every nonzero chunk of the word space).
    pub fn state_hash(&mut self) -> [u64; 2] {
        self.spill_scalars();
        self.arena.hash_nonzero()
    }

    /// The host state now.
    pub fn capture_ext(&mut self) -> Result<ExtRecord, String> {
        let _m = crate::memstat::scope(crate::memstat::tag::RECORD);
        let t = std::time::Instant::now();
        let mut v = SnapFiles {
            out: vec![],
            err: None,
        };
        self.visit_files(&mut v);
        if let Some(e) = v.err {
            return Err(format!("cannot checkpoint: {e}"));
        }
        let t1 = std::time::Instant::now();
        let cstate = crate::pdftex::snapshot_state()?;
        let s = &mut self.layer().stats;
        s.files_s += (t1 - t).as_secs_f64();
        s.cstate_s += t1.elapsed().as_secs_f64();
        Ok(ExtRecord {
            files: v.out,
            cstate,
            terminal_len: system::terminal_len(),
            effects_len: system::external_effects_len(),
            tex_input_type: system::tex_input_type(),
            reads: system::reads_len(),
            last_byte_reads: crate::pdftex::last_byte_reads(),
            matrix_uses: crate::pdftex::matrix_uses(),
            rs: self.layer().rs.len(),
        })
    }

    /// Put the host state of `rec` back.
    pub fn restore_ext(&mut self, rec: &ExtRecord) -> Result<(), String> {
        let mut v = RestoreFiles {
            snaps: &rec.files,
            i: 0,
            err: None,
        };
        self.visit_files(&mut v);
        let err = v.err;
        crate::pdftex::restore_state(rec.cstate.clone());
        system::truncate_terminal(rec.terminal_len);
        system::truncate_external_effects(rec.effects_len);
        system::set_tex_input_type_flag(rec.tex_input_type);
        crate::pdftex::set_last_byte_reads(rec.last_byte_reads);
        crate::pdftex::set_matrix_uses(rec.matrix_uses);
        self.layer().rs.truncate(rec.rs);
        match err {
            Some(e) => Err(format!("cannot restore the files: {e}")),
            None => Ok(()),
        }
    }

    /// Whether checkpoint `id` can be restored: no file it has open for
    /// output is one a run opens more than once (`system::volatile_output`),
    /// whose bytes on disk may be another instance's by now.
    pub fn restorable(&mut self, id: CheckpointId) -> bool {
        self.record_of(id)
            .is_ok_and(|r| outputs_restorable(&r).is_ok())
    }

    /// Take a checkpoint now. The engine must be between commands (before
    /// or after a run, or inside `flashtex_checkpoint_hook`).
    pub fn checkpoint(&mut self) -> Result<CheckpointId, String> {
        let cpu0 = crate::incr::thread_cpu_s();
        let ext = self.capture_ext()?;
        let t = std::time::Instant::now();
        let cpu = crate::incr::thread_cpu_s();
        self.spill_scalars();
        let dirty = self.arena.open_log_len() as u64;
        // Measurement only (FLASHTEX_CHECKPOINT_REGIONS=1): the dirty chunks
        // by array, for sizing (docs/evidence/p4-l1-2026-09-29).
        static REGIONS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let regions =
            *REGIONS.get_or_init(|| std::env::var_os("FLASHTEX_CHECKPOINT_REGIONS").is_some());
        let by = if !regions || self.arena.checkpoint_ids().is_empty() {
            vec![]
        } else {
            self.arena.open_log_by_region()
        };
        let id = self.arena.checkpoint();
        let l = self.layer();
        l.stats.seal_s += t.elapsed().as_secs_f64();
        let cpu1 = crate::incr::thread_cpu_s();
        l.stats.seal_cpu_s += cpu1 - cpu;
        l.stats.total_cpu_s += cpu1 - cpu0;
        l.stats.count += 1;
        l.stats.dirty_chunks += dirty;
        for (k, v) in by {
            *l.stats.dirty_by_region.entry(k).or_default() += v as u64;
        }
        l.records.push((id, ext));
        Ok(id)
    }

    /// Memory accounting (`crate::memstat`, lane P4-MEMORY): the word
    /// space and logs (`Arena::mem_stats`), the host records, the detached
    /// branch, the terminal.
    pub fn mem_stats(&self) -> Vec<(&'static str, i64)> {
        let l = match self.layer_ref() {
            Some(l) => l,
            None => return self.arena.mem_stats(None),
        };
        let mut v = self.arena.mem_stats(l.pending.as_ref().map(|p| &p.branch));
        let lines = |rs: &[(CheckpointId, ExtRecord)]| -> usize {
            rs.iter()
                .map(|(_, r)| r.files.iter().map(|f| f.line.capacity()).sum::<usize>())
                .sum()
        };
        v.push(("records", l.records.len() as i64));
        v.push(("record_lines", lines(&l.records) as i64));
        if let Some(p) = &l.pending {
            v.push(("pending_records", p.records.len() as i64));
            v.push(("pending_record_lines", lines(&p.records) as i64));
            let tails: usize = p
                .tails
                .iter()
                .map(|t| match &t.bytes {
                    TailBytes::Read(b, _) => b.capacity(),
                    TailBytes::Clone(_) => 0,
                })
                .sum();
            v.push(("pending_tails_read", tails as i64));
            v.push(("pending_terminal_tail", p.terminal_tail.1.capacity() as i64));
        }
        v.push(("spare_tails", spare_tail_bytes() as i64));
        v.push(("terminal", system::terminal_len() as i64));
        v.push(("taken", l.taken.len() as i64));
        v
    }

    /// The retained checkpoints, oldest first.
    pub fn checkpoints(&self) -> Vec<CheckpointId> {
        self.arena.checkpoint_ids().to_vec()
    }

    /// The host record of checkpoint `id`.
    pub fn record_of(&mut self, id: CheckpointId) -> Result<ExtRecord, String> {
        self.layer()
            .records
            .iter()
            .rev()
            .find(|(i, _)| *i == id)
            .map(|(_, r)| r.clone())
            .ok_or_else(|| format!("checkpoint {id} has no host record"))
    }

    fn drop_pending(&mut self) {
        if let Some(p) = self.layer().pending.take() {
            self.arena.drop_branch(p.branch);
        }
    }

    /// Restore checkpoint `id` and drop every later one: the plain restart.
    pub fn restore_discard(&mut self, id: CheckpointId) -> Result<(), String> {
        let rec = self.record_of(id)?;
        outputs_restorable(&rec)?;
        self.drop_pending();
        self.arena.restore_discard(id)?;
        self.fill_scalars();
        let keep: std::collections::HashSet<CheckpointId> =
            self.arena.checkpoint_ids().iter().copied().collect();
        self.layer().records.retain(|(i, _)| keep.contains(i));
        self.restore_ext(&rec)
    }

    /// Restore checkpoint `id`, keeping the run it leaves (its later
    /// checkpoints, the word space's redo log, its output files' tails) so
    /// that `redo_to` can jump back to it.
    pub fn restore(&mut self, id: CheckpointId) -> Result<(), String> {
        let _m = crate::memstat::scope(crate::memstat::tag::BRANCH);
        let t0 = std::time::Instant::now();
        let rec = self.record_of(id)?;
        outputs_restorable(&rec)?;
        self.drop_pending();
        let t_drop = t0.elapsed();
        let live = self.capture_ext()?;
        // The old run's output beyond what the target had written: every
        // file open for output at the target (it may have been closed
        // since) or at the old run's end.
        let mut tails: Vec<Tail> = vec![];
        for f in &rec.files {
            if let Stream::Out { path, len } = &f.stream {
                tails.push(Tail {
                    path: path.clone(),
                    base: *len,
                    bytes: TailBytes::take(path, *len)?,
                    open_at_target: true,
                });
            }
        }
        for f in &live.files {
            if let Stream::Out { path, .. } = &f.stream {
                if !tails.iter().any(|t| &t.path == path) {
                    tails.push(Tail {
                        path: path.clone(),
                        base: 0,
                        bytes: TailBytes::take(path, 0)?,
                        open_at_target: false,
                    });
                }
            }
        }
        // Files the old run opened for output after the target and has
        // closed since (the journal lists them, when one is recorded).
        for path in system::outputs_since(rec.reads.2) {
            if !tails.iter().any(|t| t.path == path) {
                if let Ok(bytes) = TailBytes::take(&path, 0) {
                    tails.push(Tail {
                        path,
                        base: 0,
                        bytes,
                        open_at_target: false,
                    });
                }
            }
        }
        let t_tails = t0.elapsed();
        let term = system::terminal_bytes();
        let terminal_tail = (
            rec.terminal_len,
            term.get(rec.terminal_len..).unwrap_or(&[]).to_vec(),
        );
        let rs_old = self.layer().rs.clone();
        self.spill_scalars();
        let t_pre = t0.elapsed();
        let branch = self.arena.restore_branch(id)?;
        self.fill_scalars();
        let t_branch = t0.elapsed();
        let detached_ids: std::collections::HashSet<CheckpointId> =
            branch.ids()[1..].iter().copied().collect();
        let layer = self.layer();
        let (records, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut layer.records)
            .into_iter()
            .partition(|(i, _)| detached_ids.contains(i));
        layer.records = kept;
        layer.pending = Some(Pending {
            branch,
            live,
            tails,
            terminal_tail,
            records,
            rs_old,
        });
        let r = self.restore_ext(&rec);
        if std::env::var_os("FLASHTEX_INCR_DEBUG").is_some() {
            let ms = |d: std::time::Duration| d.as_secs_f64() * 1e3;
            eprintln!(
                "[ckpt] restore {id}: drop pending {:.2}, output tails {:.2}, terminal and spill {:.2}, undo {:.2} ({} logs), host state {:.2} ms",
                ms(t_drop),
                ms(t_tails - t_drop),
                ms(t_pre - t_tails),
                ms(t_branch - t_pre),
                self.arena.checkpoint_ids().len(),
                ms(t0.elapsed() - t_branch)
            );
        }
        r
    }

    /// Abandon the run since the last `restore`: back to the old run's
    /// latest state, with its checkpoints, host records, output files,
    /// terminal and read-set, as if the restore had not happened (L5: a
    /// re-read of the `.aux` alone, `crate::incr`).
    pub fn reattach_pending(&mut self) -> Result<(), String> {
        let Some(p) = self.layer().pending.take() else {
            return Err("reattach: no restore is pending".into());
        };
        let Pending {
            branch,
            live,
            tails,
            terminal_tail,
            records,
            rs_old,
        } = p;
        self.spill_scalars();
        self.arena.reattach(branch)?;
        self.fill_scalars();
        let keep: std::collections::HashSet<CheckpointId> =
            self.arena.checkpoint_ids().iter().copied().collect();
        {
            let layer = self.layer();
            layer.records.retain(|(i, _)| keep.contains(i));
            for (i, r) in records {
                if keep.contains(&i) {
                    layer.records.push((i, r));
                }
            }
            layer.rs = rs_old;
        }
        for t in &tails {
            use std::io::{Seek, Write};
            let mut h = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&t.path)
                .map_err(|e| format!("{}: {e}", t.path))?;
            h.set_len(t.base).map_err(|e| format!("{}: {e}", t.path))?;
            h.seek(std::io::SeekFrom::Start(t.base))
                .map_err(|e| format!("{}: {e}", t.path))?;
            h.write_all(&t.bytes.get(t.base, 0)?)
                .map_err(|e| format!("{}: {e}", t.path))?;
        }
        system::truncate_terminal(terminal_tail.0);
        system::append_terminal(&terminal_tail.1);
        self.restore_ext(&live)
    }

    /// The old run's bytes `from..to` of output file `path`, from the
    /// branch the last `restore` detached (`None`: not kept).
    pub fn pending_old_bytes(&self, path: &str, from: u64, to: u64) -> Option<Vec<u8>> {
        let p = self.layer_ref()?.pending.as_ref()?;
        let t = p.tails.iter().find(|t| t.path == path)?;
        let skip = from.checked_sub(t.base)?;
        let b = t.bytes.get(t.base, skip).ok()?;
        b.get(..to.checked_sub(from)? as usize).map(|s| s.to_vec())
    }

    /// The old run's terminal output `from..to` (in its terminal's bytes),
    /// from the branch the last `restore` detached.
    pub fn pending_old_terminal(&self, from: usize, to: usize) -> Option<Vec<u8>> {
        let p = self.layer_ref()?.pending.as_ref()?;
        let (base, b) = &p.terminal_tail;
        b.get(from.checked_sub(*base)?..to.checked_sub(*base)?)
            .map(|s| s.to_vec())
    }

    /// The convergence jump (DESIGN.md §5.3). After `restore(k)` and a
    /// re-run that reached a state equal to the old run's at checkpoint
    /// `id` (the caller has checked; with no re-run at all, `id` is `k`),
    /// jump to the old run's latest state and keep its checkpoints from
    /// `id` on. Open output streams get the old run's bytes from their
    /// length at `id`.
    pub fn redo_to(&mut self, id: CheckpointId) -> Result<(), String> {
        self.redo_to_remapped(id, &|_, off| off)
    }

    /// `redo_to` after a re-run that read a changed input and wrote output
    /// of other lengths than the old run up to `id` (L3). Every output file
    /// becomes the new run's bytes up to its length now, then the old run's
    /// bytes from its length at `id`; the terminal likewise. The old run's
    /// host records from `id` on are shifted to match: output lengths and
    /// the terminal by the difference at `id`, the read journal's counts
    /// likewise, and input offsets by `in_remap(path, old offset)` (the
    /// caller's edit: bytes after an edit moved by its length difference).
    pub fn redo_to_remapped(
        &mut self,
        id: CheckpointId,
        in_remap: &dyn Fn(&str, u64) -> u64,
    ) -> Result<(), String> {
        let Some(p) = self.layer().pending.take() else {
            return Err("redo_to: no restore to jump back from".into());
        };
        let Pending {
            branch,
            live,
            tails,
            terminal_tail,
            records,
            rs_old,
        } = p;
        let at_id: ExtRecord = if branch.ids().first() == Some(&id) {
            self.record_of(id)?
        } else {
            match records.iter().find(|(i, _)| *i == id) {
                Some((_, r)) => r.clone(),
                None => {
                    self.arena.drop_branch(branch);
                    return Err(format!("checkpoint {id} is not in the detached run"));
                }
            }
        };
        // The convergence point: the live state, sealed with nothing written
        // after it. Its output streams must be the ones the old run had open.
        let now = self.capture_ext()?;
        let mut out_delta: Vec<(String, i64)> = vec![];
        for (k, f) in at_id.files.iter().enumerate() {
            match (&f.stream, &now.files[k].stream) {
                (Stream::Out { path, len }, Stream::Out { path: p, len: l }) => {
                    if path != p {
                        self.arena.drop_branch(branch);
                        return Err(format!(
                            "redo_to: {p} is open where the old run had {path} at checkpoint {id}"
                        ));
                    }
                    out_delta.push((path.clone(), *l as i64 - *len as i64));
                }
                (Stream::Out { path, .. }, _) | (_, Stream::Out { path, .. }) => {
                    self.arena.drop_branch(branch);
                    return Err(format!(
                        "redo_to: {path} is open for output in one run only at checkpoint {id}"
                    ));
                }
                _ => {}
            }
        }
        let term_delta = now.terminal_len as i64 - at_id.terminal_len as i64;
        let reads_delta = (
            now.reads.0 as i64 - at_id.reads.0 as i64,
            now.reads.1 as i64 - at_id.reads.1 as i64,
            now.reads.2 as i64 - at_id.reads.2 as i64,
        );
        let rs_delta = now.rs as i64 - at_id.rs as i64;
        let remap = |r: &ExtRecord| -> ExtRecord {
            let mut r = r.clone();
            for f in r.files.iter_mut() {
                match &mut f.stream {
                    Stream::Out { path, len } => {
                        if let Some((_, d)) = out_delta.iter().find(|(p, _)| p == path) {
                            *len = (*len as i64 + d) as u64;
                        }
                    }
                    Stream::In { path, offset } => *offset = in_remap(path, *offset),
                    _ => {}
                }
            }
            r.terminal_len = (r.terminal_len as i64 + term_delta) as usize;
            r.reads = (
                (r.reads.0 as i64 + reads_delta.0) as usize,
                (r.reads.1 as i64 + reads_delta.1) as usize,
                (r.reads.2 as i64 + reads_delta.2) as usize,
            );
            r.rs = (r.rs as i64 + rs_delta) as usize;
            r
        };
        // The read-set: the new run's up to here, then the old run's after
        // `id` (its first reads there; a name either part read twice is
        // only conservative).
        {
            let layer = self.layer();
            let mut ev = layer.rs.events[..now.rs.min(layer.rs.len())].to_vec();
            ev.extend_from_slice(rs_old.events.get(at_id.rs..).unwrap_or(&[]));
            layer.rs = crate::readset::ReadSet::from_events(ev);
        }
        self.spill_scalars();
        // The convergence test lets the live state differ from the old
        // run's at `id` where that cannot change what the old run did next
        // (dead words, free cells, node addresses: `incr`). But the old
        // run's later checkpoints hold only the chunks it wrote after `id`;
        // every other chunk must be its own state at `id`. Take that state
        // over whole, so that the jump and every later restore of an old
        // checkpoint give exactly the old run's states.
        let adopt: Vec<(usize, Vec<u8>)> = match self.arena.diff_branch_all(&branch, id) {
            Ok(d) => d
                .differing
                .iter()
                .map(|&(c, old, _)| {
                    // SAFETY: `old` points at a whole chunk owned by the
                    // arena or the branch, unchanged until `d` is dropped.
                    let b = unsafe {
                        std::slice::from_raw_parts(old as *const u8, crate::arena::CHUNK_BYTES)
                    };
                    ((c as usize) << crate::arena::CHUNK_SHIFT, b.to_vec())
                })
                .collect(),
            Err(e) => {
                self.arena.drop_branch(branch);
                return Err(e);
            }
        };
        for (off, b) in &adopt {
            self.arena.write_through(*off, b);
        }
        self.arena.checkpoint();
        self.arena.converge(branch, id)?;
        self.fill_scalars();
        // Host records: the new run's up to `id` (whose record now is the
        // live one at the convergence point), then the old run's from `id`
        // on, shifted.
        let keep: std::collections::HashSet<CheckpointId> =
            self.arena.checkpoint_ids().iter().copied().collect();
        {
            let layer = self.layer();
            layer.records.retain(|(i, _)| keep.contains(i) && *i != id);
            layer.records.push((id, now.clone()));
            for (i, r) in records {
                if keep.contains(&i) && i != id {
                    layer.records.push((i, remap(&r)));
                }
            }
        }
        // Output files: the new run's bytes up to their length now, then the
        // old run's from their length at `id`. A file open at `id` is
        // spliced there; one the old run opened after `id` is the old run's
        // alone; one open at the restore target but closed by `id` is what
        // the new run left.
        for t in &tails {
            let at = at_id.files.iter().find_map(|f| match &f.stream {
                Stream::Out { path, len } if *path == t.path => Some(*len),
                _ => None,
            });
            let (from, skip) = match at {
                Some(len) => {
                    let d = out_delta
                        .iter()
                        .find(|(p, _)| *p == t.path)
                        .map_or(0, |x| x.1);
                    (
                        (len as i64 + d) as u64,
                        len.checked_sub(t.base).ok_or_else(|| {
                            format!("redo_to: {} is shorter at {id} than at the restore", t.path)
                        })?,
                    )
                }
                None if !t.open_at_target => (0, 0),
                None => continue,
            };
            use std::io::{Seek, Write};
            let mut h = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&t.path)
                .map_err(|e| format!("{}: {e}", t.path))?;
            h.set_len(from).map_err(|e| format!("{}: {e}", t.path))?;
            h.seek(std::io::SeekFrom::Start(from))
                .map_err(|e| format!("{}: {e}", t.path))?;
            h.write_all(&t.bytes.get(t.base, skip)?)
                .map_err(|e| format!("{}: {e}", t.path))?;
        }
        system::truncate_terminal(now.terminal_len);
        let skip = at_id.terminal_len.saturating_sub(terminal_tail.0);
        system::append_terminal(terminal_tail.1.get(skip..).unwrap_or(&[]));
        let mut live = remap(&live);
        live.terminal_len = system::terminal_len();
        let r = self.restore_ext(&live);
        // `rs_seen` is the old run's; the read-set is the spliced one
        crate::readset::rebuild_seen(self);
        r
    }

    /// Drop every checkpoint `keep` rejects, except the newest (their undo
    /// logs merge into their predecessors').
    pub fn retain_checkpoints(&mut self, keep: &dyn Fn(CheckpointId) -> bool) {
        self.arena.retain(keep);
        let ids: std::collections::HashSet<CheckpointId> =
            self.arena.checkpoint_ids().iter().copied().collect();
        self.layer().records.retain(|(i, _)| ids.contains(i));
    }

    /// Drop every checkpoint.
    pub fn forget_checkpoints(&mut self) {
        self.drop_pending();
        self.arena.forget_checkpoints();
        let l = self.layer();
        l.records.clear();
        l.taken.clear();
        l.s0 = None;
    }

    // ---- the hook and its requests -------------------------------------

    /// Take S₀ at the begin-document point of this run (§5.1): the first
    /// `big_switch` after the expansion of `\document` has been consumed.
    pub fn arm_begin_document(&mut self) {
        self.layer().arm_name = Some(b"document".to_vec());
        self.ckpt_request = REQ_LOOKUP;
    }

    /// Take a checkpoint at the first `big_switch` after every `\shipout`
    /// (§5.2).
    pub fn checkpoint_every_shipout(&mut self, on: bool) {
        self.ckpt_on_shipout = if on { REQ_SHIPOUT } else { 0 };
    }

    /// Take a checkpoint after `build_page` whenever `min_s` of engine
    /// time has passed since the last one (`Point::Segment`); `None`: never.
    pub fn checkpoint_segments(&mut self, min_s: Option<f64>) {
        match min_s {
            Some(s) => {
                self.ckpt_on_segment = REQ_SEGMENT;
                self.layer().segment_s = s;
            }
            None => self.ckpt_on_segment = 0,
        }
    }

    /// Only note when each shipout finished (`Layer::shipout_times`), for
    /// "time to the first page". Ignored while checkpoints are requested.
    pub fn note_shipouts(&mut self, on: bool) {
        if self.ckpt_on_shipout != REQ_SHIPOUT {
            self.ckpt_on_shipout = if on { REQ_NOTE_SHIPOUT } else { 0 };
        }
    }

    /// The control sequence named `name`, without entering it (tex.web
    /// §259's lookup with `no_new_control_sequence`, as a read-only scan).
    pub fn find_cs(&self, name: &[u8]) -> Option<i32> {
        for (i, h) in self.hash.iter().enumerate() {
            let t = h.rh();
            if t <= 0 || t >= self.str_ptr {
                continue;
            }
            let (a, b) = (
                self.str_start[t as usize] as usize,
                self.str_start[t as usize + 1] as usize,
            );
            if b - a == name.len()
                && self.str_pool[a..b]
                    .iter()
                    .zip(name)
                    .all(|(&c, &n)| c == n as i32)
            {
                return Some(i as i32 + HASH_BASE);
            }
        }
        None
    }

    /// Called at `big_switch` whenever `ckpt_request` is nonzero
    /// (changes/checkpoint.ch).
    pub fn flashtex_checkpoint_hook(&mut self) {
        let req = std::mem::replace(&mut self.ckpt_request, 0);
        match req {
            REQ_LOOKUP => {
                let name = self.layer().arm_name.clone();
                if let Some(cs) = name.and_then(|n| self.find_cs(&n)) {
                    self.ckpt_arm_cs = cs;
                }
            }
            REQ_BEGIN_DOCUMENT => self.hook_checkpoint(Point::BeginDocument),
            REQ_SHIPOUT => self.hook_checkpoint(Point::Shipout),
            REQ_TIMED => self.hook_checkpoint(Point::Timed),
            REQ_AUX => {
                // L5: the read-set begins when this `.aux` has been read
                // (`note_aux_close`)
                self.layer().aux_armed = true;
                self.hook_checkpoint(Point::Aux)
            }
            REQ_AUX_DONE => self.hook_checkpoint(Point::AuxDone),
            REQ_SEGMENT => {
                let l = self.layer();
                let due = l.lines > l.lines_at_checkpoint
                    && l.last_checkpoint
                        .is_none_or(|t| t.elapsed().as_secs_f64() >= l.segment_s);
                if due {
                    l.stats.segments += 1;
                    self.hook_checkpoint(Point::Segment);
                } else {
                    l.stats.segments_skipped += 1;
                }
            }
            REQ_NOTE_SHIPOUT => {
                let l = self.layer();
                let t = l.run_started.map_or(0.0, |s| s.elapsed().as_secs_f64());
                l.shipout_times.push(t);
            }
            _ => {}
        }
        if self.ckpt_request == 0 && self.layer().aux_done_pending {
            self.layer().aux_done_pending = false;
            self.ckpt_request = REQ_AUX_DONE;
        }
    }

    /// The `.aux` read at the `.aux` point was closed (`system`'s
    /// `a_close`): the read-set begins (the `.aux` read's own reads of the
    /// names it defines are not a page's), and `Point::AuxDone` is taken at
    /// the next `big_switch`.
    pub fn note_aux_close(&mut self, path: &str) {
        let l = self.layer();
        if !l.aux_armed || l.aux_path.as_deref() != Some(path) {
            return;
        }
        l.aux_armed = false;
        l.aux_close_rs = Some(l.rs.len());
        self.rs_on = true;
        if self.ckpt_request == 0 {
            self.ckpt_request = REQ_AUX_DONE;
        } else {
            self.layer().aux_done_pending = true;
        }
    }

    fn hook_checkpoint(&mut self, why: Point) {
        let t = std::time::Instant::now();
        match self.checkpoint() {
            Ok(id) => {
                let hash = if self.layer().hash_states {
                    Some(self.state_hash())
                } else {
                    None
                };
                let reads = if why == Point::BeginDocument {
                    system::reads_so_far()
                } else {
                    None
                };
                let l = self.layer();
                l.taken.push((id, why));
                if let Some(h) = hash {
                    l.state_hashes.push((id, h));
                }
                if why == Point::Aux {
                    l.aux_point = Some(id);
                }
                if why == Point::AuxDone {
                    l.aux_done = Some(id);
                }
                if why == Point::BeginDocument {
                    l.s0 = Some(id);
                    l.s0_reads = reads;
                    l.s0_elapsed = l.run_started.map_or(0.0, |s| s.elapsed().as_secs_f64());
                }
                l.seconds += t.elapsed().as_secs_f64();
                l.last_checkpoint = Some(std::time::Instant::now());
                l.lines_at_checkpoint = l.lines;
                if why == Point::BeginDocument && l.stop_at_s0 {
                    std::panic::resume_unwind(Box::new(system::EngineExit(-1)));
                }
                if let Some(mut obs) = self.layer().observer.take() {
                    let a = obs.on_checkpoint(self, id, why);
                    self.layer().observer = Some(obs);
                    if a == Action::Stop {
                        std::panic::resume_unwind(Box::new(StopRun));
                    }
                }
            }
            Err(e) => self.layer().errors.push(e),
        }
    }

    /// Called by `system::input_ln` for every line read: request a
    /// checkpoint when `timed_s` of engine time has passed since the last.
    /// An input file named `*.aux` was opened: inside `\document` (armed
    /// for S₀), and when asked for, request the `.aux` point.
    pub fn note_aux_open(&mut self, path: &str) {
        if self.ckpt_arm_level <= 0 || self.ckpt_request != 0 {
            return;
        }
        let l = self.layer();
        if l.want_aux_point && l.aux_point.is_none() && l.s0.is_none() {
            l.aux_path = Some(path.to_string());
            self.ckpt_request = REQ_AUX;
        }
    }

    pub fn maybe_request_timed_checkpoint(&mut self) {
        if self.ckpt_request != 0 {
            self.layer().lines += 1;
            return;
        }
        let l = self.layer();
        l.lines += 1;
        if l.timed_s <= 0.0 {
            return;
        }
        if l.last_checkpoint
            .is_some_and(|t| t.elapsed().as_secs_f64() >= l.timed_s)
        {
            self.ckpt_request = REQ_TIMED;
        }
    }

    // ---- running -------------------------------------------------------

    fn exit_status(&self) -> ExitStatus {
        if self.history > 1 {
            1
        } else {
            0
        }
    }

    fn catch_exit(&mut self, f: impl FnOnce(&mut Globals)) -> Result<ExitStatus, String> {
        system::set_resident(true);
        system::reset_run_flags();
        let l = self.layer();
        l.run_started = Some(std::time::Instant::now());
        l.last_checkpoint = l.run_started;
        l.shipout_times.clear();
        let r = std::panic::catch_unwind(AssertUnwindSafe(|| f(self)));
        match r {
            Ok(()) => Ok(self.exit_status()),
            Err(p) if p.is::<StopRun>() => Ok(STOPPED),
            Err(p) => match p.downcast::<system::EngineExit>() {
                Ok(e) => Ok(e.0),
                Err(p) => {
                    let msg = p
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "engine panicked".into());
                    Err(msg)
                }
            },
        }
    }

    /// Run the job from the start (the program's main body) inside the
    /// host: the end of the job returns here instead of ending the process.
    pub fn run_to_end(&mut self) -> Result<ExitStatus, String> {
        self.catch_exit(|g| {
            g.tex_body();
            g.flush_outputs();
        })
    }

    /// Continue a restored engine from `big_switch` to the end of the job
    /// (tex.web §1337's tail: `main_control`, `final_cleanup`,
    /// `close_files_and_terminate`).
    pub fn resume_to_end(&mut self) -> Result<ExitStatus, String> {
        self.catch_exit(|g| {
            g.ckpt_resuming = true;
            g.main_control();
            g.final_cleanup();
            g.close_files_and_terminate();
            g.ready_already = 0;
            g.flush_outputs();
        })
    }

    fn flush_outputs(&mut self) {
        use crate::system::PasFile;
        self.log_file.flush();
        for f in self.write_file.iter_mut() {
            f.flush();
        }
        self.dvi_file.flush();
        self.pdf_file.flush();
    }
}
