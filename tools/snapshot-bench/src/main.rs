//! `snapshot-bench` — choose the FlashTeX engine-v2 checkpoint mechanism (DESIGN §5.2)
//! by measurement, on this host, against this workload model.
//!
//! Phases:
//!
//! * `snapshot`  — snapshot and restore cost per mechanism per state size.
//! * `fault`     — the kernel copy fault after `mach_vm_remap(copy=TRUE)`, priced.
//! * `hotloop`   — per-page cost with checkpointing, against `plain` (no snapshot).
//! * `dirty`     — 16 KB chunks dirtied per page, and memory per checkpoint.
//! * `retention` — 1,000 checkpoints under log-spaced retention, memory measured.
//!
//! Every phase prints a human table and, with `--out FILE`, appends one JSON object per
//! measurement to FILE.

mod alloc_count;
mod backend;
mod chunk_cow;
mod harness;
// Mechanism (a) needs Mach VM calls, so it exists only on macOS; everywhere else the
// stub keeps the harness compiling and every kernel phase reports "unsupported".
#[cfg(has_kernel_cow)]
mod kernel_cow;
#[cfg(not(has_kernel_cow))]
#[path = "kernel_stub.rs"]
mod kernel_cow;
mod layout;
#[cfg(has_kernel_cow)]
mod mach;
mod stats;
mod undo_cow;
mod workload;

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::Instant;

use backend::{Backend, FullCopy, Plain, Snapshot};
use chunk_cow::{ArcCow, BitmapCow};
use harness::{time_page, BODY_PAGE_NS, ENGINE_ROUNDS, PLOT_PAGE_NS};
use kernel_cow::{KernelCow, Region};
use layout::{standard_layouts, Layout, CHUNK_BYTES, CHUNK_WORDS};
use stats::{fmt_bytes, json_str, log_spaced_retention, Samples};
use undo_cow::UndoCow;
use workload::{DirtySet, Generator, Locality, Op, PageShape};

/// Serialises tests that read or disturb the task's memory ledger.
///
/// `phys_footprint`, `internal` and the host's free-page count are process-wide, and the
/// test harness runs tests on parallel threads. The chain tests hold hundreds of MiB of
/// state copies; run beside the ledger tests they once moved `phys_footprint` by 490 MiB
/// inside a measurement window where the kernel mechanism itself moves it by 0 (verified
/// 3/3 in isolation). Every test that measures the ledger, and every test that allocates
/// on that scale, takes this lock.
#[cfg(test)]
pub(crate) fn ledger_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[global_allocator]
static ALLOC: alloc_count::Counting = alloc_count::Counting;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out_path: Option<String> = None;
    let mut phases: Vec<String> = Vec::new();
    let mut reps = 20usize;
    let mut pages = 24usize;
    let mut retention_pages = 1000usize;
    let mut only_layout: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out_path = Some(args[i].clone());
            }
            "--reps" => {
                i += 1;
                reps = args[i].parse().expect("--reps N");
            }
            "--pages" => {
                i += 1;
                pages = args[i].parse().expect("--pages N");
            }
            "--retention-pages" => {
                i += 1;
                retention_pages = args[i].parse().expect("--retention-pages N");
            }
            "--layout" => {
                i += 1;
                only_layout = Some(args[i].clone());
            }
            "help" | "--help" | "-h" => {
                eprintln!(
                    "usage: snapshot-bench [PHASE...] [--out FILE] [--reps N] [--pages N]\n\
                     \x20            [--retention-pages N] [--layout NAME]\n\
                     phases: snapshot fault dirty table barrier hotloop chain retention all\n\
                     layouts: mem768k mem5M total64MB total200MB"
                );
                return;
            }
            p => phases.push(p.to_string()),
        }
        i += 1;
    }
    if phases.is_empty() {
        phases.push("all".to_string());
    }
    let all = phases.iter().any(|p| p == "all");

    let mut sink = Sink::new(out_path);

    let layouts: Vec<Layout> = standard_layouts()
        .into_iter()
        .filter(|l| only_layout.as_deref().is_none_or(|n| n == l.name))
        .collect();

    println!("# snapshot-bench — FlashTeX engine-v2 DESIGN §5.2");
    println!(
        "host page size {} B; software chunk {} B ({} words); reps {}; pages {}; \
         1-minute load average at start {:.2}",
        page_size(),
        CHUNK_BYTES,
        CHUNK_WORDS,
        reps,
        pages,
        load1()
    );
    for l in &layouts {
        println!("  {}", l.describe());
    }
    println!();

    if all || phases.iter().any(|p| p == "snapshot") {
        phase_snapshot(&layouts, reps, &mut sink);
    }
    if all || phases.iter().any(|p| p == "fault") {
        phase_fault(&layouts, reps, &mut sink);
    }
    if all || phases.iter().any(|p| p == "dirty") {
        phase_dirty(&layouts, &mut sink);
    }
    if all || phases.iter().any(|p| p == "barrier") {
        phase_barrier(&layouts, pages, &mut sink);
    }
    if all || phases.iter().any(|p| p == "table") {
        phase_table(&layouts, reps, &mut sink);
    }
    if all || phases.iter().any(|p| p == "hotloop") {
        phase_hotloop(&layouts, pages, &mut sink);
    }
    if all || phases.iter().any(|p| p == "chain") {
        phase_chain(&layouts, reps, retention_pages, &mut sink);
    }
    if all || phases.iter().any(|p| p == "retention") {
        phase_retention(&layouts, retention_pages, &mut sink);
    }

    sink.finish();
}

fn page_size() -> usize {
    // SAFETY: getpagesize takes no arguments and cannot fail.
    unsafe { libc_getpagesize() as usize }
}
extern "C" {
    #[link_name = "getpagesize"]
    fn libc_getpagesize() -> i32;
    fn getloadavg(loadavg: *mut f64, nelem: i32) -> i32;
}

/// One-minute load average. Recorded with the timings because this host is shared with
/// other build lanes and a phase measured under load is not comparable with a quiet one.
fn load1() -> f64 {
    let mut la = [0.0f64; 3];
    // SAFETY: la has room for 3 doubles and we ask for exactly 1.
    let got = unsafe { getloadavg(la.as_mut_ptr(), 1) };
    if got >= 1 {
        la[0]
    } else {
        f64::NAN
    }
}

// --------------------------------------------------------------------------------
// JSON sink
// --------------------------------------------------------------------------------

struct Sink {
    file: Option<std::fs::File>,
}

impl Sink {
    fn new(path: Option<String>) -> Sink {
        let file = path.map(|p| {
            if let Some(dir) = std::path::Path::new(&p).parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            std::fs::File::create(&p).unwrap_or_else(|e| panic!("cannot create {p}: {e}"))
        });
        Sink { file }
    }
    fn row(&mut self, phase: &str, fields: &[(&str, Field)]) {
        if let Some(f) = self.file.as_mut() {
            let mut s = String::from("{");
            let _ = write!(s, "\"phase\":{}", json_str(phase));
            for (k, v) in fields {
                let _ = write!(s, ",{}:{}", json_str(k), v.render());
            }
            s.push('}');
            let _ = writeln!(f, "{s}");
        }
    }
    fn finish(&mut self) {
        if let Some(f) = self.file.as_mut() {
            let _ = f.flush();
        }
    }
}

enum Field {
    S(String),
    N(f64),
    I(i64),
}
impl Field {
    fn render(&self) -> String {
        match self {
            Field::S(s) => json_str(s),
            Field::N(v) => {
                if v.is_finite() {
                    format!("{v:.6}")
                } else {
                    "null".into()
                }
            }
            Field::I(v) => format!("{v}"),
        }
    }
}
fn s(v: &str) -> Field {
    Field::S(v.to_string())
}
fn n(v: f64) -> Field {
    Field::N(v)
}
fn ii(v: usize) -> Field {
    Field::I(v as i64)
}

// --------------------------------------------------------------------------------
// Shared setup: bring a backend up to a realistic mid-document state.
// --------------------------------------------------------------------------------

fn seed_of(layout: &Layout, loc: &Locality, shape: &PageShape) -> u64 {
    // Deterministic per configuration, so every backend replays an identical stream.
    let mut h = 0x243F_6A88_85A3_08D3u64;
    for b in layout.name.bytes().chain(loc.name.bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
    }
    h ^ ((shape.touch_pm as u64) << 32)
}

/// Run `n` pages against a backend without timing, to leave it in a state whose pages
/// are resident and whose chunks are private — the state an engine is actually in when
/// it takes its next checkpoint.
fn warm<B: Backend>(b: &mut B, gen: &mut Generator, buf: &mut Vec<Op>, n: usize) -> u64 {
    let mut acc = 1;
    for _ in 0..n {
        gen.page(buf);
        acc = harness::run_page::<B, 0>(b, buf, acc);
    }
    std::hint::black_box(acc)
}

// --------------------------------------------------------------------------------
// Phase: snapshot and restore cost
// --------------------------------------------------------------------------------

fn phase_snapshot(layouts: &[Layout], reps: usize, sink: &mut Sink) {
    println!("## Phase: snapshot / restore cost (median of {reps}+)");
    println!(
        "| state | mechanism | snapshot | restore | note |\n\
         |---|---|---|---|---|"
    );

    let loc = Locality::tex_freelist();
    let shape = PageShape::touch(1);

    for l in layouts {
        let seed = seed_of(l, &loc, &shape);
        let mut buf: Vec<Op> = Vec::new();

        // --- (a) kernel: mach_vm_remap(copy=TRUE) ------------------------------
        if !kernel_cow::SUPPORTED {
            unsupported(sink, l, "kernel-remap");
            unsupported(sink, l, "kernel-vm_copy");
        } else {
            let mut b = KernelCow::new(l);
            let mut g = Generator::new(*l, loc, shape, seed);
            warm(&mut b, &mut g, &mut buf, 3);
            let mut snap_s = Samples::new();
            let mut rest_s = Samples::new();
            let mut acc = 7u64;
            for _ in 0..reps.max(20) {
                g.page(&mut buf);
                acc = harness::run_page::<KernelCow, 0>(&mut b, &buf, acc);
                let t = Instant::now();
                let snap = b.try_snapshot().expect("mach_vm_remap(copy=TRUE)");
                snap_s.push(t.elapsed());
                g.page(&mut buf);
                acc = harness::run_page::<KernelCow, 0>(&mut b, &buf, acc);
                let t = Instant::now();
                b.try_restore(&snap).expect("mach_vm_remap(OVERWRITE)");
                rest_s.push(t.elapsed());
            }
            std::hint::black_box(acc);
            let note = format!(
                "cur_prot {:#x}, max_prot {:#x}",
                b.last_cur_prot, b.last_max_prot
            );
            emit(sink, l, "kernel-remap", &snap_s, &rest_s, &note);
        }

        // --- (a') kernel: vm_copy into a pre-allocated region -------------------
        if kernel_cow::SUPPORTED {
            let mut b = KernelCow::new(l);
            let mut g = Generator::new(*l, loc, shape, seed);
            warm(&mut b, &mut g, &mut buf, 3);
            let dst = Region::allocate(l.total_bytes()).expect("dst region");
            let mut snap_s = Samples::new();
            let mut acc = 7u64;
            let mut note = String::from("dest region pre-allocated and reused");
            for _ in 0..reps.max(20) {
                g.page(&mut buf);
                acc = harness::run_page::<KernelCow, 0>(&mut b, &buf, acc);
                let t = Instant::now();
                match b.try_vm_copy_into(&dst) {
                    Ok(()) => snap_s.push(t.elapsed()),
                    Err(e) => {
                        note = format!("FAILED: {e}");
                        break;
                    }
                }
            }
            std::hint::black_box(acc);
            emit(sink, l, "kernel-vm_copy", &snap_s, &Samples::new(), &note);
        }

        // --- (b1) Arc::make_mut -------------------------------------------------
        {
            let mut b = ArcCow::new(l);
            let mut g = Generator::new(*l, loc, shape, seed);
            warm(&mut b, &mut g, &mut buf, 3);
            let (snap_s, rest_s) = measure_sw(&mut b, &mut g, &mut buf, reps);
            emit(
                sink,
                l,
                "arc-make-mut",
                &snap_s,
                &rest_s,
                "snapshot = chunk-table clone (one atomic increment per chunk)",
            );
        }

        // --- (b2) chunk table + dirty bitmap ------------------------------------
        {
            let mut b = BitmapCow::new(l);
            let mut g = Generator::new(*l, loc, shape, seed);
            warm(&mut b, &mut g, &mut buf, 3);
            let (snap_s, rest_s) = measure_sw(&mut b, &mut g, &mut buf, reps);
            emit(
                sink,
                l,
                "chunk-bitmap",
                &snap_s,
                &rest_s,
                "snapshot = chunk-table clone + bitmap clear",
            );
        }

        // --- (b3) flat state + dirty bitmap + undo log --------------------------
        {
            let mut b = UndoCow::new(l);
            let mut g = Generator::new(*l, loc, shape, seed);
            warm(&mut b, &mut g, &mut buf, 3);
            let (snap_s, rest_s) = measure_sw(&mut b, &mut g, &mut buf, reps);
            emit(
                sink,
                l,
                "flat-undo-log",
                &snap_s,
                &rest_s,
                "snapshot = drop the log + clear the bitmap; restore = replay one log",
            );
        }

        // --- (c) full memcpy ----------------------------------------------------
        {
            let mut b = FullCopy::new(l);
            let mut g = Generator::new(*l, loc, shape, seed);
            warm(&mut b, &mut g, &mut buf, 3);
            let (snap_s, rest_s) = measure_sw(&mut b, &mut g, &mut buf, reps);
            emit(
                sink,
                l,
                "memcpy",
                &snap_s,
                &rest_s,
                "Vec::clone: alloc + memcpy",
            );

            // Pooled variant: the memcpy alone, no allocation.
            let mut pool = b.buffer();
            let mut snap_s = Samples::new();
            let mut acc = 7u64;
            for _ in 0..reps.max(20) {
                g.page(&mut buf);
                acc = harness::run_page::<FullCopy, 0>(&mut b, &buf, acc);
                let t = Instant::now();
                b.snapshot_into(&mut pool);
                snap_s.push(t.elapsed());
            }
            std::hint::black_box(acc);
            emit(
                sink,
                l,
                "memcpy-pooled",
                &snap_s,
                &Samples::new(),
                "copy_from_slice into a reused buffer",
            );
        }
    }
    println!();
}

fn unsupported(sink: &mut Sink, l: &Layout, mech: &str) {
    println!(
        "| {} | `{}` | unsupported on this target | — | needs macOS |",
        l.name, mech
    );
    sink.row(
        "snapshot",
        &[
            ("layout", s(l.name)),
            ("mechanism", s(mech)),
            ("note", s("unsupported")),
        ],
    );
}

fn measure_sw<B: Backend>(
    b: &mut B,
    g: &mut Generator,
    buf: &mut Vec<Op>,
    reps: usize,
) -> (Samples, Samples) {
    let mut snap_s = Samples::new();
    let mut rest_s = Samples::new();
    let mut acc = 7u64;
    for _ in 0..reps.max(20) {
        g.page(buf);
        acc = harness::run_page::<B, 0>(b, buf, acc);
        let t = Instant::now();
        let snap = b.snapshot();
        snap_s.push(t.elapsed());
        g.page(buf);
        acc = harness::run_page::<B, 0>(b, buf, acc);
        let t = Instant::now();
        b.restore(&snap);
        rest_s.push(t.elapsed());
        drop(snap);
    }
    std::hint::black_box(acc);
    (snap_s, rest_s)
}

fn emit(sink: &mut Sink, l: &Layout, mech: &str, snap: &Samples, rest: &Samples, note: &str) {
    let sm = if snap.is_empty() {
        f64::NAN
    } else {
        snap.median()
    };
    let rm = if rest.is_empty() {
        f64::NAN
    } else {
        rest.median()
    };
    println!(
        "| {} ({:.0} MiB) | `{}` | {} | {} | {} |",
        l.name,
        l.total_bytes() as f64 / (1024.0 * 1024.0),
        mech,
        fmt_dur(sm, snap),
        fmt_dur(rm, rest),
        note
    );
    sink.row(
        "snapshot",
        &[
            ("layout", s(l.name)),
            ("state_bytes", ii(l.total_bytes())),
            ("mechanism", s(mech)),
            ("snapshot_ns_min", n(snap.quantile(0.0))),
            ("snapshot_ns_median", n(sm)),
            ("snapshot_ns_p90", n(snap.quantile(0.90))),
            (
                "snapshot_ns_max",
                n(if snap.is_empty() {
                    f64::NAN
                } else {
                    snap.max()
                }),
            ),
            ("restore_ns_min", n(rest.quantile(0.0))),
            ("restore_ns_median", n(rm)),
            ("restore_ns_p90", n(rest.quantile(0.90))),
            ("reps", ii(snap.len())),
            ("load1", n(load1())),
            ("note", s(note)),
        ],
    );
}

fn unit_ns(v: f64) -> String {
    if v < 1000.0 {
        format!("{v:.0} ns")
    } else if v < 1_000_000.0 {
        format!("{:.2} µs", v / 1000.0)
    } else {
        format!("{:.3} ms", v / 1e6)
    }
}

/// Median, with the minimum and p90 beside it.
///
/// The minimum matters on this host specifically. It is shared with other build lanes, and
/// under load the median of a sequential phase moves by about 2x while the minimum barely
/// does: the fastest repetition is the one that ran without being descheduled or having its
/// cache evicted, so it is the best available estimate of the mechanism's own cost. The
/// median and p90 are what the mechanism costs on a machine as busy as this one was.
fn fmt_dur(median_ns: f64, all: &Samples) -> String {
    if !median_ns.is_finite() {
        return "—".to_string();
    }
    format!(
        "{} (min {}, p90 {})",
        unit_ns(median_ns),
        unit_ns(all.quantile(0.0)),
        unit_ns(all.quantile(0.90))
    )
}

// --------------------------------------------------------------------------------
// Phase: the kernel copy fault
// --------------------------------------------------------------------------------

fn phase_fault(layouts: &[Layout], reps: usize, sink: &mut Sink) {
    println!("## Phase: kernel copy fault after `mach_vm_remap(copy=TRUE)`");
    if !kernel_cow::SUPPORTED {
        println!("unsupported on this target: mechanism (a) needs macOS.\n");
        return;
    }
    println!(
        "| state | pages stored to | no snapshot | snapshot outstanding | copy fault per page |\n\
         |---|---|---|---|---|"
    );
    for l in layouts {
        let pages = (l.chunks() / 4).clamp(64, 4096);
        let mut clean = Samples::new();
        let mut dirty = Samples::new();
        for r in 0..reps.max(20) {
            // Baseline: pages already private, so the stores take no fault.
            let mut b = KernelCow::new(l);
            b.fault_probe(pages, 1); // first touch: zero-fill faults
            b.fault_probe(pages, 2); // now warm
            let t = Instant::now();
            b.fault_probe(pages, 3 + r as u64);
            clean.push(t.elapsed());

            // With a copy-on-write snapshot outstanding, each store faults once.
            let snap = b.try_snapshot().expect("snapshot");
            let t = Instant::now();
            b.fault_probe(pages, 100 + r as u64);
            dirty.push(t.elapsed());
            // `snap` is released here, at the end of the iteration, before `b`.
            let _ = &snap;
        }
        let delta = (dirty.median() - clean.median()) / pages as f64;
        let delta_min = (dirty.quantile(0.0) - clean.quantile(0.0)) / pages as f64;
        println!(
            "| {} | {} | {} | {} | **{:.0} ns/page** (min-based {:.0} ns/page) |",
            l.name,
            pages,
            fmt_dur(clean.median(), &clean),
            fmt_dur(dirty.median(), &dirty),
            delta,
            delta_min
        );
        sink.row(
            "fault",
            &[
                ("layout", s(l.name)),
                ("pages", ii(pages)),
                ("clean_ns_median", n(clean.median())),
                ("cow_ns_median", n(dirty.median())),
                ("delta_ns_per_page", n(delta)),
                ("delta_ns_per_page_min", n(delta_min)),
                ("reps", ii(clean.len())),
                ("load1", n(load1())),
            ],
        );
    }
    println!();
}

// --------------------------------------------------------------------------------
// Phase: dirty set per page
// --------------------------------------------------------------------------------

/// Mean distinct words written and distinct 16 KB chunks dirtied per page, over 8 pages
/// once the drifting `mem` window has settled.
fn measure_dirty(l: &Layout, loc: Locality, shape: PageShape) -> (f64, f64) {
    let mut g = Generator::new(*l, loc, shape, seed_of(l, &loc, &shape));
    let mut buf = Vec::new();
    for _ in 0..3 {
        g.page(&mut buf);
    }
    let rounds = 8;
    let mut chunk_sum = 0usize;
    let mut word_sum = 0usize;
    let mut word_seen = vec![false; l.total_words];
    for _ in 0..rounds {
        g.page(&mut buf);
        let mut ds = DirtySet::new(l.chunks());
        let mut words = 0usize;
        let mut touched: Vec<usize> = Vec::new();
        for op in buf.iter().filter(|o| o.is_write()) {
            let i = op.index();
            ds.mark_chunk(i >> layout::CHUNK_SHIFT);
            if !word_seen[i] {
                word_seen[i] = true;
                touched.push(i);
                words += 1;
            }
        }
        for i in touched {
            word_seen[i] = false;
        }
        chunk_sum += ds.count;
        word_sum += words;
    }
    (
        word_sum as f64 / rounds as f64,
        chunk_sum as f64 / rounds as f64,
    )
}

#[allow(clippy::too_many_arguments)]
fn dirty_row(
    sink: &mut Sink,
    phase: &str,
    l: &Layout,
    loc: &Locality,
    label: &str,
    pct: u32,
    words: f64,
    chunks: f64,
) {
    let bytes = chunks * CHUNK_BYTES as f64;
    println!(
        "| {} | {} | {}% | {:.0} ({:.2}% of state) | {:.0} / {} | {:.1}% | {} |",
        l.name,
        loc.name,
        pct,
        words,
        100.0 * words / l.total_words as f64,
        chunks,
        l.chunks(),
        100.0 * chunks / l.chunks() as f64,
        fmt_bytes(bytes as usize)
    );
    sink.row(
        phase,
        &[
            ("layout", s(l.name)),
            ("locality", s(loc.name)),
            ("budget", s(label)),
            ("budget_pct", ii(pct as usize)),
            ("state_bytes", ii(l.total_bytes())),
            ("state_chunks", ii(l.chunks())),
            ("distinct_words_written", n(words)),
            ("dirty_word_pct", n(100.0 * words / l.total_words as f64)),
            ("dirty_chunks", n(chunks)),
            ("dirty_chunk_pct", n(100.0 * chunks / l.chunks() as f64)),
            ("dirty_chunk_bytes", n(bytes)),
        ],
    );
}

/// Find the op budget at which a page's *distinct* write set is `target_pct` of the
/// state, and report what that budget actually achieves.
///
/// Bisection rather than arithmetic: the ratio of distinct words to writes falls as the
/// budget grows, because a bigger page rewrites the same window more times, and it falls
/// at a rate that depends on the locality profile. Probes use 2 pages, not 8, because
/// bisection needs the trend rather than a settled mean, and the largest budgets here are
/// millions of ops.
fn calibrate_dirty(l: &Layout, loc: Locality, target_pct: f64) -> (PageShape, f64, f64) {
    let probe = |touch_pm: u32| -> (f64, f64) {
        let shape = PageShape {
            touch_pm,
            write_pm: 1000,
        };
        let mut g = Generator::new(*l, loc, shape, seed_of(l, &loc, &shape));
        let mut buf = Vec::new();
        for _ in 0..3 {
            g.page(&mut buf);
        }
        let mut word_seen = vec![false; l.total_words];
        let mut chunk_sum = 0usize;
        let mut word_sum = 0usize;
        for _ in 0..2 {
            g.page(&mut buf);
            let mut ds = DirtySet::new(l.chunks());
            let mut touched: Vec<usize> = Vec::new();
            for op in buf.iter().filter(|o| o.is_write()) {
                let i = op.index();
                ds.mark_chunk(i >> layout::CHUNK_SHIFT);
                if !word_seen[i] {
                    word_seen[i] = true;
                    touched.push(i);
                }
            }
            word_sum += touched.len();
            for i in touched {
                word_seen[i] = false;
            }
            chunk_sum += ds.count;
        }
        (word_sum as f64 / 2.0, chunk_sum as f64 / 2.0)
    };

    // 400 per mille of a 200 MB state is 10.5M writes per page: far past any real page,
    // and enough headroom that the cap only binds where the arenas themselves cap it.
    let (mut lo, mut hi) = (1u32, 400u32);
    let target_words = target_pct / 100.0 * l.total_words as f64;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if probe(mid).0 < target_words {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let shape = PageShape {
        touch_pm: lo,
        write_pm: 1000,
    };
    let (words, chunks) = measure_dirty(l, loc, shape);
    (shape, words, chunks)
}

fn phase_dirty(layouts: &[Layout], sink: &mut Sink) {
    println!("## Phase: 16 KB chunks dirtied per page, and memory per checkpoint");
    println!(
        "Both copy-on-write mechanisms copy at 16 KB on this host, so one table covers\n\
         both: memory per checkpoint is the dirty-chunk count times 16 KB either way.\n\
         Chunks, not words, are what a checkpoint costs, so the gap between the two\n\
         columns is the whole question.\n"
    );

    println!("### Budget: % of state words *touched* (read or written) per page\n");
    println!(
        "| state | locality | touched/page | distinct words written | chunks dirty | % of state | memory per checkpoint |\n\
         |---|---|---|---|---|---|---|"
    );
    for l in layouts {
        for loc in [Locality::tex_freelist(), Locality::uniform()] {
            for pct in [1u32, 5u32] {
                let (words, chunks) = measure_dirty(l, loc, PageShape::touch(pct));
                dirty_row(sink, "dirty", l, &loc, "touched", pct, words, chunks);
            }
        }
    }
    println!();

    println!(
        "### Budget: % of state words *written* per page, calibrated\n\n\
         This is the row the §5.2 question asks for: memory per checkpoint at 1% and 5%\n\
         **dirty** per page. Every `mem` access is made a write and the op budget is then\n\
         searched until the *distinct* words written land on the target, because a page\n\
         rewrites many words more than once. The achieved fraction is shown, not the\n\
         target, and a row is marked `(capped)` where the arena cannot be dirtied that\n\
         far — `hash` and `font_info` are never written mid-page.\n"
    );
    println!(
        "| state | locality | dirty target | distinct words written | chunks dirty | % of state | memory per checkpoint |\n\
         |---|---|---|---|---|---|---|"
    );
    for l in layouts {
        for loc in [Locality::tex_freelist(), Locality::uniform()] {
            for pct in [1u32, 5u32] {
                let (shape, words, chunks) = calibrate_dirty(l, loc, pct as f64);
                let achieved = 100.0 * words / l.total_words as f64;
                let label = if achieved < pct as f64 * 0.8 {
                    "written (capped)"
                } else {
                    "written"
                };
                println!(
                    "| {} | {} | {}% ({} ops/page) | {:.0} ({:.2}% of state) | {:.0} / {} | {:.1}% | {} |",
                    l.name,
                    loc.name,
                    pct,
                    shape.ops(l),
                    words,
                    achieved,
                    chunks,
                    l.chunks(),
                    100.0 * chunks / l.chunks() as f64,
                    fmt_bytes((chunks * CHUNK_BYTES as f64) as usize)
                );
                sink.row(
                    "dirty_calibrated",
                    &[
                        ("layout", s(l.name)),
                        ("locality", s(loc.name)),
                        ("budget", s(label)),
                        ("budget_pct", ii(pct as usize)),
                        ("ops_per_page", ii(shape.ops(l))),
                        ("state_bytes", ii(l.total_bytes())),
                        ("state_chunks", ii(l.chunks())),
                        ("distinct_words_written", n(words)),
                        ("dirty_word_pct", n(achieved)),
                        ("dirty_chunks", n(chunks)),
                        ("dirty_chunk_pct", n(100.0 * chunks / l.chunks() as f64)),
                        ("dirty_chunk_bytes", n(chunks * CHUNK_BYTES as f64)),
                    ],
                );
            }
        }
    }
    println!();

    println!(
        "### Budget: % of state words *written* per page, uncalibrated\n\n\
         The same shape with the op budget set straight to the target, for reference: the\n\
         achieved figure is below the target because a\n\
         page rewrites some words more than once.\n"
    );
    println!(
        "| state | locality | dirty target | distinct words written | chunks dirty | % of state | memory per checkpoint |\n\
         |---|---|---|---|---|---|---|"
    );
    for l in layouts {
        for loc in [Locality::tex_freelist(), Locality::uniform()] {
            for pct in [1u32, 5u32] {
                let (words, chunks) = measure_dirty(l, loc, PageShape::dirty(pct));
                dirty_row(
                    sink,
                    "dirty_written",
                    l,
                    &loc,
                    "written",
                    pct,
                    words,
                    chunks,
                );
            }
        }
    }
    println!();

    // Sensitivity: how much scatter it takes before chunk copy-on-write stops paying.
    println!("### Sensitivity to `mem` write scatter (total64MB, 1% touched/page)\n");
    println!(
        "| uniform-write fraction of `mem` writes | chunks dirty | % of state | memory per checkpoint |\n\
         |---|---|---|---|"
    );
    if let Some(l) = layouts.iter().find(|l| l.name == "total64MB") {
        for pm in [0u32, 20, 50, 100, 200, 500, 1000] {
            let loc = Locality::tex_freelist().with_spread(pm);
            let (_, chunks) = measure_dirty(l, loc, PageShape::touch(1));
            println!(
                "| {:.1}% | {:.0} | {:.1}% | {} |",
                pm as f64 / 10.0,
                chunks,
                100.0 * chunks / l.chunks() as f64,
                fmt_bytes((chunks * CHUNK_BYTES as f64) as usize)
            );
            sink.row(
                "dirty_sensitivity",
                &[
                    ("layout", s(l.name)),
                    ("mem_write_spread_pm", ii(pm as usize)),
                    ("dirty_chunks", n(chunks)),
                    ("dirty_chunk_pct", n(100.0 * chunks / l.chunks() as f64)),
                    ("dirty_chunk_bytes", n(chunks * CHUNK_BYTES as f64)),
                ],
            );
        }
    }
    println!();
}

// --------------------------------------------------------------------------------
// Phase: hot loop
// --------------------------------------------------------------------------------

struct HotResult {
    per_page_ns: f64,
    ns_per_op: f64,
    ops: usize,
}

fn hot_run<B: Backend, const ROUNDS: u32>(
    l: &Layout,
    loc: Locality,
    shape: PageShape,
    pages: usize,
    checkpoint: bool,
) -> HotResult {
    let mut b = B::new(l);
    let seed = seed_of(l, &loc, &shape);
    let mut g = Generator::new(*l, loc, shape, seed);
    let mut buf = Vec::new();
    let mut acc = warm(&mut b, &mut g, &mut buf, 3);
    let mut per_page = Samples::new();
    // Keep only the newest snapshot alive: that is the state an engine is in between
    // checkpoints, and it is what makes the copy-on-write barrier actually fire.
    let mut held: Option<B::Snap> = None;
    for _ in 0..pages.max(20) {
        if checkpoint {
            held = Some(b.snapshot());
        }
        g.page(&mut buf);
        let t = time_page::<B, ROUNDS>(&mut b, &buf, acc);
        acc = t.acc;
        per_page.push_ns(t.ns);
    }
    drop(held);
    std::hint::black_box(acc);
    let ops = g.ops_per_page();
    HotResult {
        per_page_ns: per_page.median(),
        ns_per_op: per_page.median() / ops as f64,
        ops,
    }
}

/// One mechanism, measured twice: with no checkpoint outstanding (so the barrier runs but
/// never copies) and with a checkpoint at every page boundary (so it copies every dirty
/// chunk once). The difference separates the two costs the §5.2 gate conflates.
struct Pair {
    label: &'static str,
    quiet: HotResult,
    checkpointed: HotResult,
}

fn pair<B: Backend, const ROUNDS: u32>(
    label: &'static str,
    l: &Layout,
    loc: Locality,
    shape: PageShape,
    pages: usize,
) -> Pair {
    Pair {
        label,
        quiet: hot_run::<B, ROUNDS>(l, loc, shape, pages, false),
        checkpointed: hot_run::<B, ROUNDS>(l, loc, shape, pages, true),
    }
}

fn pairs_for<const ROUNDS: u32>(
    l: &Layout,
    loc: Locality,
    shape: PageShape,
    pages: usize,
) -> Vec<Pair> {
    let mut v = Vec::new();
    if kernel_cow::SUPPORTED {
        v.push(pair::<KernelCow, ROUNDS>(
            "kernel-remap",
            l,
            loc,
            shape,
            pages,
        ));
    }
    v.extend([
        pair::<ArcCow, ROUNDS>("arc-make-mut", l, loc, shape, pages),
        pair::<BitmapCow, ROUNDS>("chunk-bitmap", l, loc, shape, pages),
        pair::<UndoCow, ROUNDS>("flat-undo-log", l, loc, shape, pages),
        pair::<FullCopy, ROUNDS>("memcpy", l, loc, shape, pages),
    ]);
    v
}

fn phase_hotloop(layouts: &[Layout], pages: usize, sink: &mut Sink) {
    println!("## Phase: hot loop");
    println!(
        "`plain` is a `Vec<u64>` with no snapshot support: the denominator. Each mechanism\n\
         is then measured twice.\n\n\
         * **barrier** — no checkpoint outstanding, so the barrier runs on every write but\n\
           never copies. This is the standing tax on the hot loop, the thing DESIGN §5.2\n\
           caps at 3%.\n\
         * **+copies** — a checkpoint at every page boundary, so the barrier's slow path\n\
           copies every chunk the page dirties, once. This is per-checkpoint work, not a\n\
           per-access tax, and it belongs with the snapshot cost rather than the barrier.\n\n\
         `extra/page` is absolute, so it can be read against the DESIGN B.1 page budgets\n\
         (2.4 ms body page, 90 ms heavy pgfplots page) whatever this synthetic loop's own\n\
         speed is. `replay x` is the ratio against `plain` on the bare replay loop: the\n\
         pessimistic bound, since a real engine does far more work per access.\n\n\
         The snapshot itself is **not** in these numbers; it is in the snapshot phase.\n"
    );

    for l in layouts {
        for loc in [Locality::tex_freelist(), Locality::uniform()] {
            for pct in [1u32, 5u32] {
                let shape = PageShape::touch(pct);
                println!(
                    "### {} ({:.0} MiB), {}, {}% touched/page, {} ops/page",
                    l.name,
                    l.total_bytes() as f64 / (1024.0 * 1024.0),
                    loc.name,
                    pct,
                    shape.ops(l)
                );
                println!(
                    "| mechanism | cost | work | ns/op | ms/page | extra/page | vs 2.4 ms page | vs 90 ms page | replay x |\n\
                     |---|---|---|---|---|---|---|---|---|"
                );
                for rounds in [0u32, ENGINE_ROUNDS] {
                    let work = if rounds == 0 { "replay" } else { "engine" };
                    let (base, ps) = if rounds == 0 {
                        (
                            hot_run::<Plain, 0>(l, loc, shape, pages, false),
                            pairs_for::<0>(l, loc, shape, pages),
                        )
                    } else {
                        (
                            hot_run::<Plain, ENGINE_ROUNDS>(l, loc, shape, pages, false),
                            pairs_for::<ENGINE_ROUNDS>(l, loc, shape, pages),
                        )
                    };
                    let base_ns = base.per_page_ns;
                    hot_row(
                        sink,
                        l,
                        &loc,
                        pct,
                        work,
                        "—",
                        "plain (no snapshot)",
                        &base,
                        base_ns,
                    );
                    for p in &ps {
                        hot_row(
                            sink, l, &loc, pct, work, "barrier", p.label, &p.quiet, base_ns,
                        );
                        hot_row(
                            sink,
                            l,
                            &loc,
                            pct,
                            work,
                            "+copies",
                            p.label,
                            &p.checkpointed,
                            base_ns,
                        );
                    }
                }
                println!();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn hot_row(
    sink: &mut Sink,
    l: &Layout,
    loc: &Locality,
    pct: u32,
    work: &str,
    cost: &str,
    mech: &str,
    r: &HotResult,
    base_ns: f64,
) {
    let extra = r.per_page_ns - base_ns;
    println!(
        "| `{}` | {} | {} | {:.2} | {:.3} | {} | {} | {} | {:.3}x |",
        mech,
        cost,
        work,
        r.ns_per_op,
        r.per_page_ns / 1e6,
        if extra.abs() < 1.0 {
            "—".to_string()
        } else {
            format!("{:+.3} ms", extra / 1e6)
        },
        pct_of(extra, BODY_PAGE_NS),
        pct_of(extra, PLOT_PAGE_NS),
        r.per_page_ns / base_ns
    );
    sink.row(
        "hotloop",
        &[
            ("layout", s(l.name)),
            ("locality", s(loc.name)),
            ("touch_pct", ii(pct as usize)),
            ("work", s(work)),
            ("cost", s(cost)),
            ("mechanism", s(mech)),
            ("ops_per_page", ii(r.ops)),
            ("ns_per_op", n(r.ns_per_op)),
            ("page_ns_median", n(r.per_page_ns)),
            ("extra_ns_per_page", n(extra)),
            ("pct_of_body_page", n(100.0 * extra / BODY_PAGE_NS)),
            ("pct_of_plot_page", n(100.0 * extra / PLOT_PAGE_NS)),
            ("replay_ratio", n(r.per_page_ns / base_ns)),
        ],
    );
}

fn pct_of(extra_ns: f64, budget_ns: f64) -> String {
    if extra_ns.abs() < 1.0 {
        return "—".to_string();
    }
    let p = 100.0 * extra_ns / budget_ns;
    if p.abs() >= 3.0 {
        format!("**{p:+.2}%**")
    } else {
        format!("{p:+.2}%")
    }
}

// --------------------------------------------------------------------------------
// Phase: the write barrier, measured with the mechanisms interleaved
// --------------------------------------------------------------------------------

/// The `hotloop` phase runs each mechanism in its own pass, which leaves the comparison
/// exposed to drift between passes — allocation placement, frequency, and whatever else
/// the machine is doing. On the bare replay loop that drift is ±0.2 ns per access, the
/// same order as the barrier being measured, so a single pass per mechanism cannot
/// resolve the 3% gate.
///
/// This phase keeps **all five backends live at once** and pushes the *same* page's op
/// stream through each of them inside every round, taking the median per backend across
/// rounds. Drift then applies to every mechanism equally. The order is reversed on odd
/// rounds so no mechanism keeps a first-mover cache advantage.
///
/// `plain` and `memcpy` are both a bare `Vec<u64>` with identical access code, so the
/// difference between those two columns is this experiment's own noise floor: any barrier
/// smaller than that gap is not resolved.
fn phase_barrier(layouts: &[Layout], rounds: usize, sink: &mut Sink) {
    println!("## Phase: write barrier, mechanisms interleaved within each round");
    println!(
        "All five backends live at once; the same page's op stream replayed through each,\n\
         median over {} rounds, order reversed on odd rounds. `plain` and `memcpy` run\n\
         identical `Vec<u64>` access code, so the gap between them is the noise floor.\n\n\
         Checkpoint `none` isolates the standing barrier: it runs on every write but never\n\
         copies. `every page` is §5.2's `\\shipout` trigger, and `every 8 pages` is its other\n\
         trigger — a checkpoint per ~20 ms of engine time, which on a 2.4 ms body page is\n\
         one per eight pages — where the same copies are amortised over more work.\n\n\
         The `engine` rows add dependent integer work per access, which is what a real hot\n\
         loop has and what lets a superscalar core hide a cheap barrier.\n",
        rounds.max(20)
    );

    for l in layouts {
        for loc in [Locality::tex_freelist(), Locality::uniform()] {
            for pct in [1u32, 5u32] {
                let shape = PageShape::touch(pct);
                println!(
                    "### {} ({:.0} MiB), {}, {}% touched/page, {} ops/page",
                    l.name,
                    l.total_bytes() as f64 / (1024.0 * 1024.0),
                    loc.name,
                    pct,
                    shape.ops(l)
                );
                println!(
                    "| work | checkpoint | plain | kernel-remap | arc-make-mut | chunk-bitmap | flat-undo-log | memcpy (control) |\n\
                     |---|---|---|---|---|---|---|---|"
                );
                for cp in [0usize, 1, 8] {
                    barrier_round::<0>(l, loc, shape, rounds, cp, "replay", sink);
                    barrier_round::<ENGINE_ROUNDS>(l, loc, shape, rounds, cp, "engine", sink);
                }
                println!();
            }
        }
    }
}

/// `cp_every` is the checkpoint interval in pages: 0 never checkpoints (the standing
/// barrier alone), 1 checkpoints at every page (DESIGN §5.2's \shipout trigger), and larger
/// values model §5.2's other trigger, a checkpoint every ~20 ms of engine time, which on a
/// 2.4 ms body page is one checkpoint per eight pages.
fn barrier_round<const ROUNDS: u32>(
    l: &Layout,
    loc: Locality,
    shape: PageShape,
    rounds: usize,
    cp_every: usize,
    work: &str,
    sink: &mut Sink,
) {
    let mut plain = Plain::new(l);
    let mut kernel = kernel_cow::SUPPORTED.then(|| KernelCow::new(l));
    let mut arc = ArcCow::new(l);
    let mut bitmap = BitmapCow::new(l);
    let mut undo = UndoCow::new(l);
    let mut full = FullCopy::new(l);

    let mut g = Generator::new(*l, loc, shape, seed_of(l, &loc, &shape));
    let mut buf = Vec::new();
    let mut acc = 0x9E37_79B9u64;

    // Warm every backend on the same pages, so none of them starts cold.
    for _ in 0..3 {
        g.page(&mut buf);
        acc = harness::run_page::<Plain, 0>(&mut plain, &buf, acc);
        if let Some(k) = kernel.as_mut() {
            acc = harness::run_page::<KernelCow, 0>(k, &buf, acc);
        }
        acc = harness::run_page::<ArcCow, 0>(&mut arc, &buf, acc);
        acc = harness::run_page::<BitmapCow, 0>(&mut bitmap, &buf, acc);
        acc = harness::run_page::<UndoCow, 0>(&mut undo, &buf, acc);
        acc = harness::run_page::<FullCopy, 0>(&mut full, &buf, acc);
    }

    let mut s_plain = Samples::new();
    let mut s_kernel = Samples::new();
    let mut s_arc = Samples::new();
    let mut s_bitmap = Samples::new();
    let mut s_undo = Samples::new();
    let mut s_full = Samples::new();

    // Only the newest checkpoint of each mechanism stays alive, as an engine between
    // checkpoints would have.
    let mut h_kernel: Option<kernel_cow::RemapSnap> = None;
    let mut h_arc: Option<chunk_cow::ArcSnap> = None;
    let mut h_bitmap: Option<chunk_cow::BitmapSnap> = None;
    let mut h_undo: Option<undo_cow::UndoSnap> = None;
    let mut h_full: Option<backend::FullCopySnap> = None;

    for r in 0..rounds.max(20) {
        g.page(&mut buf);
        if cp_every > 0 && r % cp_every == 0 {
            h_kernel = kernel.as_mut().map(|k| k.snapshot());
            h_arc = Some(arc.snapshot());
            h_bitmap = Some(bitmap.snapshot());
            h_undo = Some(undo.snapshot());
            h_full = Some(full.snapshot());
        }
        // Forward on even rounds, reverse on odd: no mechanism is always first.
        if r % 2 == 0 {
            let t = time_page::<Plain, ROUNDS>(&mut plain, &buf, acc);
            acc = t.acc;
            s_plain.push_ns(t.ns);
            if let Some(k) = kernel.as_mut() {
                let t = time_page::<KernelCow, ROUNDS>(k, &buf, acc);
                acc = t.acc;
                s_kernel.push_ns(t.ns);
            }
            let t = time_page::<ArcCow, ROUNDS>(&mut arc, &buf, acc);
            acc = t.acc;
            s_arc.push_ns(t.ns);
            let t = time_page::<BitmapCow, ROUNDS>(&mut bitmap, &buf, acc);
            acc = t.acc;
            s_bitmap.push_ns(t.ns);
            let t = time_page::<UndoCow, ROUNDS>(&mut undo, &buf, acc);
            acc = t.acc;
            s_undo.push_ns(t.ns);
            let t = time_page::<FullCopy, ROUNDS>(&mut full, &buf, acc);
            acc = t.acc;
            s_full.push_ns(t.ns);
        } else {
            let t = time_page::<FullCopy, ROUNDS>(&mut full, &buf, acc);
            acc = t.acc;
            s_full.push_ns(t.ns);
            let t = time_page::<UndoCow, ROUNDS>(&mut undo, &buf, acc);
            acc = t.acc;
            s_undo.push_ns(t.ns);
            let t = time_page::<BitmapCow, ROUNDS>(&mut bitmap, &buf, acc);
            acc = t.acc;
            s_bitmap.push_ns(t.ns);
            let t = time_page::<ArcCow, ROUNDS>(&mut arc, &buf, acc);
            acc = t.acc;
            s_arc.push_ns(t.ns);
            if let Some(k) = kernel.as_mut() {
                let t = time_page::<KernelCow, ROUNDS>(k, &buf, acc);
                acc = t.acc;
                s_kernel.push_ns(t.ns);
            }
            let t = time_page::<Plain, ROUNDS>(&mut plain, &buf, acc);
            acc = t.acc;
            s_plain.push_ns(t.ns);
        }
    }
    drop((h_kernel, h_arc, h_bitmap, h_undo, h_full));
    std::hint::black_box(acc);

    let ops = shape.ops(l) as f64;
    let base = s_plain.median();
    let cell = |sm: &Samples| -> String {
        if sm.is_empty() {
            return "unsupported".to_string();
        }
        let extra = sm.median() - base;
        format!(
            "{:.2} ns/op ({:+.2}, {})",
            sm.median() / ops,
            extra / ops,
            pct_of(extra, BODY_PAGE_NS)
        )
    };
    println!(
        "| {} | {} | {:.2} ns/op | {} | {} | {} | {} | {} |",
        work,
        match cp_every {
            0 => "none".to_string(),
            1 => "every page".to_string(),
            k => format!("every {k} pages"),
        },
        base / ops,
        cell(&s_kernel),
        cell(&s_arc),
        cell(&s_bitmap),
        cell(&s_undo),
        cell(&s_full)
    );

    for (mech, sm) in [
        ("plain", &s_plain),
        ("kernel-remap", &s_kernel),
        ("arc-make-mut", &s_arc),
        ("chunk-bitmap", &s_bitmap),
        ("flat-undo-log", &s_undo),
        ("memcpy", &s_full),
    ] {
        if sm.is_empty() {
            continue;
        }
        let extra = sm.median() - base;
        sink.row(
            "barrier",
            &[
                ("layout", s(l.name)),
                ("locality", s(loc.name)),
                ("touch_pct", ii(pct_of_shape(shape))),
                ("work", s(work)),
                ("checkpoint_every_pages", ii(cp_every)),
                ("mechanism", s(mech)),
                ("ops_per_page", ii(ops as usize)),
                ("ns_per_op", n(sm.median() / ops)),
                ("extra_ns_per_op", n(extra / ops)),
                ("page_ns_median", n(sm.median())),
                ("page_ns_p90", n(sm.quantile(0.90))),
                ("extra_ns_per_page", n(extra)),
                ("pct_of_body_page", n(100.0 * extra / BODY_PAGE_NS)),
                ("rounds", ii(sm.len())),
                ("load1", n(load1())),
            ],
        );
    }
}

fn pct_of_shape(shape: PageShape) -> usize {
    (shape.touch_pm / 10) as usize
}

// --------------------------------------------------------------------------------
// Phase: the cost of the snapshot's own bookkeeping table
// --------------------------------------------------------------------------------

/// Mechanism (b)'s snapshot is a clone of the chunk table, and at 200 MB that is 12,800
/// `Arc` clones. Each one is an atomic read-modify-write on a control block that sits in
/// its own 16 KB allocation, so the pass is one cache miss per chunk — which is why the
/// snapshot cost grows to the edge of the 1 ms gate.
///
/// The alternative is to keep the reference counts out of the chunks: a chunk table of
/// 32-bit version ids plus a contiguous reference-count array. A snapshot is then a
/// `memcpy` of the id table plus a pass of increments over an array small enough to stay
/// in L2. This phase prices all three so the §5.2 condition can be stated with a number.
fn phase_table(layouts: &[Layout], reps: usize, sink: &mut Sink) {
    println!("## Phase: snapshot bookkeeping, `Arc` table vs side reference counts");
    println!(
        "| state | chunks | `Arc` table clone | id-table `memcpy` | + side refcount pass |\n\
         |---|---|---|---|---|"
    );
    for l in layouts {
        let nc = l.chunks();

        // (b2) as implemented: Vec<Arc<CellChunk>>::clone.
        let mut b = BitmapCow::new(l);
        let mut arc_s = Samples::new();
        for _ in 0..reps.max(20) {
            let t = Instant::now();
            let snap = b.snapshot();
            arc_s.push(t.elapsed());
            drop(snap);
        }

        // Side tables: `live` holds a version id per chunk, `rc` a count per version.
        let live: Vec<u32> = (0..nc as u32).collect();
        let mut rc: Vec<u32> = vec![1; nc];
        let mut memcpy_s = Samples::new();
        let mut rc_s = Samples::new();
        let mut sink_id: u64 = 0;
        for _ in 0..reps.max(20) {
            let t = Instant::now();
            let copy = live.clone();
            memcpy_s.push(t.elapsed());
            sink_id ^= copy[nc - 1] as u64;

            let t = Instant::now();
            let copy2 = live.clone();
            for &v in &copy2 {
                rc[v as usize] += 1;
            }
            rc_s.push(t.elapsed());
            for &v in &copy2 {
                rc[v as usize] -= 1;
            }
            sink_id ^= rc[0] as u64;
        }
        std::hint::black_box(sink_id);

        println!(
            "| {} | {} | {} | {} | {} |",
            l.name,
            nc,
            fmt_dur(arc_s.median(), &arc_s),
            fmt_dur(memcpy_s.median(), &memcpy_s),
            fmt_dur(rc_s.median(), &rc_s)
        );
        sink.row(
            "table",
            &[
                ("layout", s(l.name)),
                ("chunks", ii(nc)),
                ("arc_table_clone_ns", n(arc_s.median())),
                ("id_table_memcpy_ns", n(memcpy_s.median())),
                ("id_table_plus_refcount_ns", n(rc_s.median())),
                ("reps", ii(arc_s.len())),
                ("load1", n(load1())),
            ],
        );
    }
    println!();
}

// --------------------------------------------------------------------------------
// Phase: N checkpoints under log-spaced retention
// --------------------------------------------------------------------------------

const DENSE: usize = 16;
const PER_OCTAVE: usize = 4;
/// Do not actually hold more than this while measuring; report over-budget
/// configurations from the measured dirty-chunk rate instead of paging the machine.
const RETENTION_BUDGET: usize = 5 << 30; // 5 GiB

fn phase_retention(layouts: &[Layout], total_pages: usize, sink: &mut Sink) {
    let retained = log_spaced_retention(total_pages, DENSE, PER_OCTAVE);
    println!(
        "## Phase: {total_pages} checkpoints, log-spaced retention \
         (dense {DENSE}, {PER_OCTAVE} per octave)"
    );
    println!(
        "Retention keeps **{} of {total_pages}** checkpoints: {:?}... and the newest {DENSE}.\n\
         `measured` rows are real: the software mechanisms through a counting global\n\
         allocator, the kernel mechanism through the host's free-memory drop, because its
         pages are invisible to the task's own ledger. `modelled`\n\
         rows would have needed more than {} live at once, so they are the measured\n\
         dirty-chunk rate times the retained count instead.\n",
        retained.len(),
        &retained[..6.min(retained.len())],
        fmt_bytes(RETENTION_BUDGET)
    );
    println!(
        "| state | locality | touched/page | mechanism | retained | total memory | per checkpoint | how |\n\
         |---|---|---|---|---|---|---|---|"
    );

    for l in layouts {
        for loc in [Locality::tex_freelist(), Locality::uniform()] {
            for pct in [1u32, 5u32] {
                let shape = PageShape::touch(pct);
                let dirty = measure_dirty(l, loc, shape).1;
                // Upper bound on what a measured run would hold: every retained
                // checkpoint owning a full page's worth of fresh chunks.
                let projected = retained.len() * (dirty.ceil() as usize) * CHUNK_BYTES;
                let affordable = projected <= RETENTION_BUDGET;

                if affordable {
                    let (bytes, kept, unique, slow) = retention_bitmap(l, loc, shape, total_pages);
                    let how = format!(
                        "measured (counting allocator, heap peak {}); chunk-version \
                         accounting {}; {:.0} barrier slow paths/page",
                        fmt_bytes(alloc_count::peak_bytes()),
                        fmt_bytes(unique * CHUNK_BYTES),
                        slow as f64 / total_pages as f64
                    );
                    row(sink, l, &loc, pct, "chunk-bitmap", kept, bytes, &how);

                    let (bytes, kept) = retention_arc(l, loc, shape, total_pages);
                    row(
                        sink,
                        l,
                        &loc,
                        pct,
                        "arc-make-mut",
                        kept,
                        bytes,
                        "measured (counting allocator)",
                    );

                    if !kernel_cow::SUPPORTED {
                        row(
                            sink,
                            l,
                            &loc,
                            pct,
                            "kernel-remap",
                            0,
                            0,
                            "unsupported on this target",
                        );
                    } else {
                        let (bytes, kept, failed) = retention_kernel(l, loc, shape, total_pages);
                        let how = if failed == 0 {
                            "measured (host free-memory drop; see the ledger caveat)".to_string()
                        } else {
                            format!("measured (host free-memory drop); {failed} remaps failed")
                        };
                        row(sink, l, &loc, pct, "kernel-remap", kept, bytes, &how);
                    }
                } else {
                    let modelled = format!(
                        "modelled: {:.0} dirty chunks/page x {} retained",
                        dirty,
                        retained.len()
                    );
                    row(
                        sink,
                        l,
                        &loc,
                        pct,
                        "chunk-bitmap",
                        retained.len(),
                        projected,
                        &modelled,
                    );
                    row(
                        sink,
                        l,
                        &loc,
                        pct,
                        "arc-make-mut",
                        retained.len(),
                        projected,
                        &modelled,
                    );
                    row(
                        sink,
                        l,
                        &loc,
                        pct,
                        "kernel-remap",
                        retained.len(),
                        projected,
                        &modelled,
                    );
                }

                // (c) needs no measurement: a full copy per checkpoint, by definition.
                row(
                    sink,
                    l,
                    &loc,
                    pct,
                    "memcpy",
                    retained.len(),
                    retained.len() * l.total_bytes(),
                    "analytic: retained x full state",
                );
            }
        }
    }
    println!();
}

#[allow(clippy::too_many_arguments)]
fn row(
    sink: &mut Sink,
    l: &Layout,
    loc: &Locality,
    pct: u32,
    mech: &str,
    kept: usize,
    bytes: usize,
    how: &str,
) {
    println!(
        "| {} | {} | {}% | `{}` | {} | {} | {} | {} |",
        l.name,
        loc.name,
        pct,
        mech,
        kept,
        fmt_bytes(bytes),
        fmt_bytes(bytes.checked_div(kept).unwrap_or(0)),
        how
    );
    sink.row(
        "retention",
        &[
            ("layout", s(l.name)),
            ("locality", s(loc.name)),
            ("touch_pct", ii(pct as usize)),
            ("mechanism", s(mech)),
            ("retained", ii(kept)),
            ("bytes", ii(bytes)),
            (
                "bytes_per_checkpoint",
                ii(bytes.checked_div(kept).unwrap_or(0)),
            ),
            ("measurement", s(how)),
        ],
    );
}

/// Walk `total` pages, checkpointing each one and thinning by the retention policy.
///
/// Returns (allocator-measured bytes, retained snapshots, distinct chunk versions alive,
/// barrier slow-path entries).
fn retention_bitmap(
    l: &Layout,
    loc: Locality,
    shape: PageShape,
    total: usize,
) -> (usize, usize, usize, u64) {
    let mut b = BitmapCow::new(l);
    let mut g = Generator::new(*l, loc, shape, seed_of(l, &loc, &shape));
    let mut buf = Vec::new();
    let mut acc = warm(&mut b, &mut g, &mut buf, 2);
    let base = alloc_count::live_bytes();
    let slow_base = b.slow_path;
    let mut store: Vec<Option<chunk_cow::BitmapSnap>> = Vec::with_capacity(total);
    for step in 0..total {
        store.push(Some(b.snapshot()));
        g.page(&mut buf);
        acc = harness::run_page::<BitmapCow, 0>(&mut b, &buf, acc);
        thin(&mut store, step + 1);
    }
    std::hint::black_box(acc);
    let kept = store.iter().filter(|sn| sn.is_some()).count();
    let bytes = alloc_count::live_bytes().saturating_sub(base);
    // Cross-check: chunk versions no other snapshot shares, plus the live state's own.
    let unique: usize = store
        .iter()
        .flatten()
        .map(|sn| sn.unique_chunks())
        .sum::<usize>()
        + l.chunks();
    let slow = b.slow_path - slow_base;
    (bytes + b.table_bytes(), kept, unique, slow)
}

fn retention_arc(l: &Layout, loc: Locality, shape: PageShape, total: usize) -> (usize, usize) {
    let mut b = ArcCow::new(l);
    let mut g = Generator::new(*l, loc, shape, seed_of(l, &loc, &shape));
    let mut buf = Vec::new();
    let mut acc = warm(&mut b, &mut g, &mut buf, 2);
    let base = alloc_count::live_bytes();
    let mut store: Vec<Option<chunk_cow::ArcSnap>> = Vec::with_capacity(total);
    for step in 0..total {
        store.push(Some(b.snapshot()));
        g.page(&mut buf);
        acc = harness::run_page::<ArcCow, 0>(&mut b, &buf, acc);
        thin(&mut store, step + 1);
    }
    std::hint::black_box(acc);
    let kept = store.iter().filter(|sn| sn.is_some()).count();
    let bytes = alloc_count::live_bytes().saturating_sub(base);
    // The Snapshot trait's own accounting, kept in step with the allocator figure.
    let nominal: usize = store
        .iter()
        .flatten()
        .map(|sn| sn.nominal_bytes())
        .sum::<usize>();
    debug_assert!(nominal <= bytes + l.total_bytes());
    (bytes, kept)
}

fn retention_kernel(
    l: &Layout,
    loc: Locality,
    shape: PageShape,
    total: usize,
) -> (usize, usize, usize) {
    let mut b = KernelCow::new(l);
    let mut g = Generator::new(*l, loc, shape, seed_of(l, &loc, &shape));
    let mut buf = Vec::new();
    let mut acc = warm(&mut b, &mut g, &mut buf, 2);
    // Not the task ledger: `mach::tests::kernel_cow_memory_is_invisible_to_the_task_ledger`
    // shows that 512 MiB of verified private copy-on-write pages move phys_footprint,
    // internal, compressed and resident_size by exactly zero, while the host's free-page
    // count falls by 509 MiB. So the host's view is the only one that sees this mechanism's
    // memory. It also sees every other process on the machine, which is why the figure is
    // reported with the caveat and cross-checked against the dirty-chunk accounting.
    let base = kernel_cow::host_free_bytes();
    let mut failed = 0usize;
    let mut store: Vec<Option<kernel_cow::RemapSnap>> = Vec::with_capacity(total);
    for step in 0..total {
        match b.try_snapshot() {
            Ok(sn) => store.push(Some(sn)),
            Err(e) => {
                if failed == 0 {
                    eprintln!("  kernel snapshot {step} failed: {e}");
                }
                failed += 1;
                store.push(None);
            }
        }
        g.page(&mut buf);
        acc = harness::run_page::<KernelCow, 0>(&mut b, &buf, acc);
        thin(&mut store, step + 1);
    }
    std::hint::black_box(acc);
    let kept = store.iter().filter(|sn| sn.is_some()).count();
    let bytes = base.saturating_sub(kernel_cow::host_free_bytes()) as usize;
    (bytes, kept, failed)
}

/// Drop every snapshot the policy no longer retains, given `n` checkpoints so far.
fn thin<T>(store: &mut [Option<T>], n: usize) {
    let keep = log_spaced_retention(n, DENSE, PER_OCTAVE);
    let mut it = keep.iter().peekable();
    for (idx, slot) in store.iter_mut().enumerate() {
        while it.peek().is_some_and(|&&k| k < idx) {
            it.next();
        }
        if !it.peek().is_some_and(|&&k| k == idx) {
            *slot = None;
        }
    }
}

// --------------------------------------------------------------------------------
// Phase: chained undo logs — deep restore, redo jump, compaction (DESIGN §5.3)
// --------------------------------------------------------------------------------

/// Skip the uncompacted chain where it would hold more than this.
const CHAIN_BUDGET: usize = 10 << 30;
/// Workers for the parallel restore column. The copies are bandwidth-bound, so more than
/// this buys little on an M5 Pro and competes with the engine thread.
const PAR: usize = 8;

/// The retained checkpoint whose distance from the newest is the smallest one at least
/// `want`, or the oldest retained if none is that far back.
fn at_distance(
    ids: &[undo_cow::CheckpointId],
    newest: u64,
    want: u64,
) -> (undo_cow::CheckpointId, u64) {
    let mut best: Option<(undo_cow::CheckpointId, u64)> = None;
    for &id in ids.iter().rev() {
        let d = newest - id;
        if d >= want {
            best = Some((id, d));
            break;
        }
    }
    best.unwrap_or_else(|| (ids[0], newest - ids[0]))
}

fn phase_chain(layouts: &[Layout], reps: usize, total: usize, sink: &mut Sink) {
    use undo_cow::{UndoChain, Walk};
    println!("## Phase: chained undo logs — deep restore, redo jump, compaction");
    println!(
        "{total} checkpoints, one page of `tex-freelist` 1%-touched writes after each. \
         `restore` = `restore_branch` to the checkpoint that far back, including the redo \
         capture §5.3 needs; `jump` = `converge` straight back to the old end, the worst case \
         (a real convergence at a later checkpoint j re-applies only chunks the old run wrote \
         after j). Every repetition returns to the end state, and the state is checked bit \
         for bit against a copy on the first and last repetition. Median of {} (p90).\n",
        reps.max(20)
    );
    println!(
        "| state | chain | distance | logs walked | entries walked | chunks restored | restore, dedup | restore, dedup x{PAR} | restore, naive walk | redo jump | redo jump x{PAR} |\n\
         |---|---|---|---|---|---|---|---|---|---|---|"
    );
    let loc = Locality::tex_freelist();
    let shape = PageShape::touch(1);
    for l in layouts
        .iter()
        .filter(|l| l.name == "total64MB" || l.name == "total200MB" || layouts.len() == 1)
    {
        let per_page = measure_dirty(l, loc, shape).1;
        let projected = (per_page * total as f64 * CHUNK_BYTES as f64 * 1.3) as usize;
        if projected > CHAIN_BUDGET {
            println!(
                "| {} | skipped: {} projected, over the {} budget | | | | | | | | | |",
                l.name,
                fmt_bytes(projected),
                fmt_bytes(CHAIN_BUDGET)
            );
            continue;
        }

        let mut c = UndoChain::new(l);
        let mut g = Generator::new(*l, loc, shape, seed_of(l, &loc, &shape));
        let mut buf = Vec::new();
        let mut acc = warm(&mut c, &mut g, &mut buf, 2);
        let mut seal = Samples::new();
        for _ in 0..total {
            let t = Instant::now();
            c.checkpoint();
            seal.push(t.elapsed());
            g.page(&mut buf);
            acc = harness::run_page::<UndoChain, 0>(&mut c, &buf, acc);
        }
        std::hint::black_box(acc);
        let end: Vec<u64> = c.state().to_vec();
        let newest = *c.checkpoint_ids().last().unwrap();
        let full_bytes = c.chain_bytes();
        let full_entries = c.log_entries();

        let retained: std::collections::HashSet<u64> =
            log_spaced_retention(total, DENSE, PER_OCTAVE)
                .into_iter()
                .map(|i| i as u64)
                .collect();

        let mut compact_ns = f64::NAN;
        for compacted in [false, true] {
            if compacted {
                let t = Instant::now();
                c.retain(|id| retained.contains(&id));
                compact_ns = t.elapsed().as_nanos() as f64;
            }
            let label = if compacted {
                format!("log-spaced, {} kept", c.checkpoint_ids().len())
            } else {
                format!("all {} kept", c.checkpoint_ids().len())
            };
            let ids = c.checkpoint_ids().to_vec();
            for want in [0u64, 10, 100, total as u64 - 1] {
                let (target, dist) = at_distance(&ids, newest, want);
                let mut results: Vec<(Samples, Samples, usize, usize, usize)> = Vec::new();
                for (walk, threads) in [(Walk::Dedup, 1), (Walk::Dedup, PAR), (Walk::Naive, 1)] {
                    c.threads = threads;
                    let mut rs = Samples::new();
                    let mut js = Samples::new();
                    let (mut logs_walked, mut entries, mut chunks) = (0, 0, 0);
                    let n = reps.max(20);
                    for rep in 0..n {
                        let t = Instant::now();
                        let br = c.restore_branch(target, walk);
                        rs.push(t.elapsed());
                        logs_walked = br.logs_walked();
                        entries = br.entries_walked;
                        chunks = br.redo_chunks();
                        let t = Instant::now();
                        c.converge(br, target);
                        js.push(t.elapsed());
                        if rep == 0 || rep + 1 == n {
                            assert!(
                                c.state() == &end[..],
                                "{}: jump from {target} ({walk:?}) did not restore the end state",
                                l.name
                            );
                        }
                    }
                    results.push((rs, js, logs_walked, entries, chunks));
                }
                c.threads = 1;
                let (d_rs, d_js, logs_walked, entries, chunks) = &results[0];
                let (p_rs, p_js, _, _, _) = &results[1];
                let (n_rs, _, _, _, _) = &results[2];
                let flag = |v: f64| if v > 1e6 { " **> 1 ms**" } else { "" };
                println!(
                    "| {} | {} | {} | {} | {} | {} | {}{} | {}{} | {}{} | {}{} | {}{} |",
                    l.name,
                    label,
                    dist,
                    logs_walked,
                    entries,
                    chunks,
                    fmt_dur(d_rs.median(), d_rs),
                    flag(d_rs.median()),
                    fmt_dur(p_rs.median(), p_rs),
                    flag(p_rs.median()),
                    fmt_dur(n_rs.median(), n_rs),
                    flag(n_rs.median()),
                    fmt_dur(d_js.median(), d_js),
                    flag(d_js.median()),
                    fmt_dur(p_js.median(), p_js),
                    flag(p_js.median()),
                );
                sink.row(
                    "chain",
                    &[
                        ("layout", s(l.name)),
                        (
                            "compacted",
                            s(if compacted { "log-spaced" } else { "none" }),
                        ),
                        ("checkpoints_in_chain", ii(ids.len())),
                        ("distance", ii(dist as usize)),
                        ("logs_walked", ii(*logs_walked)),
                        ("entries_walked", ii(*entries)),
                        ("chunks_restored", ii(*chunks)),
                        ("restore_dedup_ns_median", n(d_rs.median())),
                        ("restore_dedup_ns_p90", n(d_rs.quantile(0.9))),
                        ("restore_parallel_ns_median", n(p_rs.median())),
                        ("restore_parallel_ns_p90", n(p_rs.quantile(0.9))),
                        ("redo_jump_parallel_ns_median", n(p_js.median())),
                        ("parallel_threads", ii(PAR)),
                        ("restore_naive_ns_median", n(n_rs.median())),
                        ("restore_naive_ns_p90", n(n_rs.quantile(0.9))),
                        ("redo_jump_ns_median", n(d_js.median())),
                        ("redo_jump_ns_p90", n(d_js.quantile(0.9))),
                        ("chain_bytes", ii(c.chain_bytes())),
                        ("load1", n(load1())),
                    ],
                );
            }
        }
        println!(
            "| {} | seal (checkpoint) {} ; chain {} in {} entries uncompacted, {} after \
             compaction ({} ms to compact) | | | | | | | | | |",
            l.name,
            fmt_dur(seal.median(), &seal),
            fmt_bytes(full_bytes),
            full_entries,
            fmt_bytes(c.chain_bytes()),
            format_args!("{:.1}", compact_ns / 1e6),
        );
        sink.row(
            "chain_summary",
            &[
                ("layout", s(l.name)),
                ("checkpoints", ii(total)),
                ("seal_ns_median", n(seal.median())),
                ("seal_ns_p90", n(seal.quantile(0.9))),
                ("chain_bytes_uncompacted", ii(full_bytes)),
                ("entries_uncompacted", ii(full_entries)),
                ("chain_bytes_compacted", ii(c.chain_bytes())),
                ("compaction_ns", n(compact_ns)),
                ("load1", n(load1())),
            ],
        );
    }
    println!();
}

// --------------------------------------------------------------------------------
// Correctness self-tests: a mechanism that does not actually snapshot is worthless.
// --------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> Layout {
        Layout::with_mem_words("test", 200_000)
    }

    /// Snapshot, mutate, restore: the state must come back exactly, and the snapshot
    /// must survive being restored from more than once (L3 restarts repeatedly).
    fn roundtrip<B: Backend>(mut b: B) {
        let l = small();
        let loc = Locality::tex_freelist();
        let shape = PageShape::touch(2);
        let mut g = Generator::new(l, loc, shape, 12345);
        let mut buf = Vec::new();

        let mut acc = warm(&mut b, &mut g, &mut buf, 2);
        let at_snapshot = b.checksum();
        let snap = b.snapshot();
        assert_eq!(
            b.checksum(),
            at_snapshot,
            "{}: snapshot mutated the state",
            b.name()
        );

        for round in 0..3 {
            g.page(&mut buf);
            acc = harness::run_page::<B, 0>(&mut b, &buf, acc);
            assert_ne!(
                b.checksum(),
                at_snapshot,
                "{}: a page of writes changed nothing (round {round})",
                b.name()
            );
            b.restore(&snap);
            assert_eq!(
                b.checksum(),
                at_snapshot,
                "{}: restore did not reproduce the snapshot (round {round})",
                b.name()
            );
        }
        std::hint::black_box(acc);
    }

    #[cfg(has_kernel_cow)]
    #[test]
    fn kernel_remap_roundtrip() {
        let l = small();
        roundtrip(KernelCow::new(&l));
    }

    #[test]
    fn arc_make_mut_roundtrip() {
        let l = small();
        roundtrip(ArcCow::new(&l));
    }

    #[test]
    fn chunk_bitmap_roundtrip() {
        let l = small();
        roundtrip(BitmapCow::new(&l));
    }

    #[test]
    fn flat_undo_chain_roundtrip() {
        let l = small();
        roundtrip(undo_cow::UndoChain::new(&l));
    }

    #[test]
    fn flat_undo_log_roundtrip() {
        let l = small();
        roundtrip(UndoCow::new(&l));
    }

    /// The undo log must hold each dirtied chunk exactly once per interval, however many
    /// times the page writes it: that is what makes its memory equal to shadow paging's.
    #[test]
    fn undo_log_holds_each_dirty_chunk_once() {
        let l = small();
        let mut b = UndoCow::new(&l);
        let _ = b.snapshot();
        for _ in 0..50 {
            b.set(0, 1);
            b.set(CHUNK_WORDS - 1, 2);
            b.set(CHUNK_WORDS, 3);
        }
        assert_eq!(b.slow_path, 2, "two chunks touched, so two saves");
        assert_eq!(b.log_bytes() >= 2 * CHUNK_WORDS * 8, true);
    }

    #[test]
    fn memcpy_roundtrip() {
        let l = small();
        roundtrip(FullCopy::new(&l));
    }

    /// Two snapshots taken at different times must stay independent.
    #[test]
    fn two_snapshots_are_independent() {
        let l = small();
        let loc = Locality::tex_freelist();
        let shape = PageShape::touch(2);
        let mut b = BitmapCow::new(&l);
        let mut g = Generator::new(l, loc, shape, 999);
        let mut buf = Vec::new();
        let mut acc = warm(&mut b, &mut g, &mut buf, 1);

        let c1 = b.checksum();
        let s1 = b.snapshot();
        g.page(&mut buf);
        acc = harness::run_page::<BitmapCow, 0>(&mut b, &buf, acc);
        let c2 = b.checksum();
        let s2 = b.snapshot();
        g.page(&mut buf);
        acc = harness::run_page::<BitmapCow, 0>(&mut b, &buf, acc);
        std::hint::black_box(acc);

        assert_ne!(c1, c2);
        b.restore(&s2);
        assert_eq!(b.checksum(), c2);
        b.restore(&s1);
        assert_eq!(b.checksum(), c1);
        b.restore(&s2);
        assert_eq!(b.checksum(), c2);
    }

    /// Kernel restore must leave the live range writable. If `mach_vm_remap` dropped
    /// `VM_PROT_WRITE` the next store would fault forever, so this is the test that
    /// catches the protection-argument pitfall.
    #[cfg(has_kernel_cow)]
    #[test]
    fn kernel_state_is_writable_after_restore() {
        let l = small();
        let mut b = KernelCow::new(&l);
        b.set(0, 0xAAAA);
        let snap = b.try_snapshot().expect("snapshot");
        b.set(0, 0xBBBB);
        b.try_restore(&snap).expect("restore");
        assert_eq!(b.get(0), 0xAAAA);
        b.set(0, 0xCCCC);
        assert_eq!(b.get(0), 0xCCCC);
        assert!(
            b.last_cur_prot & mach::VM_PROT_WRITE != 0,
            "restore reported cur_prot {:#x}, without VM_PROT_WRITE",
            b.last_cur_prot
        );
    }

    /// `vm_copy` into a pre-allocated region must reproduce the state.
    #[cfg(has_kernel_cow)]
    #[test]
    fn vm_copy_reproduces_the_state() {
        let l = small();
        let mut b = KernelCow::new(&l);
        for i in 0..1000 {
            b.set(i * 977 % b.words(), 0xDEAD_0000 + i as u64);
        }
        let dst = Region::allocate(l.total_bytes()).expect("dst");
        b.try_vm_copy_into(&dst).expect("vm_copy");
        // SAFETY: dst is a mapped region of exactly l.total_bytes().
        let copied = unsafe { std::slice::from_raw_parts(dst.as_ptr(), dst.words()) };
        for i in 0..1000 {
            let k = i * 977 % b.words();
            assert_eq!(copied[k], b.get(k), "word {k} differs after vm_copy");
        }
    }

    /// The workload must actually respect its touch budget and its write share.
    #[test]
    fn workload_shape_is_as_specified() {
        let l = Layout::with_total_bytes("t", 64 * 1024 * 1024);
        let shape = PageShape::touch(1);
        let mut g = Generator::new(l, Locality::tex_freelist(), shape, 7);
        let mut buf = Vec::new();
        g.page(&mut buf);
        assert_eq!(buf.len(), shape.ops(&l));
        let writes = buf.iter().filter(|o| o.is_write()).count();
        let frac = writes as f64 / buf.len() as f64;
        assert!(
            (0.15..0.45).contains(&frac),
            "write share {frac:.3} is outside the modelled range"
        );
        assert!(buf.iter().all(|o| o.index() < l.total_words));
    }

    /// Locality must be the thing that changes the dirty-chunk count, and the pessimal
    /// profile must dirty far more than the realistic one.
    #[test]
    fn locality_drives_the_dirty_chunk_count() {
        let l = Layout::with_total_bytes("t", 64 * 1024 * 1024);
        let shape = PageShape::touch(1);
        let count = |loc: Locality| {
            let mut g = Generator::new(l, loc, shape, 3);
            let mut buf = Vec::new();
            for _ in 0..3 {
                g.page(&mut buf);
            }
            let mut ds = DirtySet::new(l.chunks());
            for op in buf.iter().filter(|o| o.is_write()) {
                ds.mark_chunk(op.index() >> layout::CHUNK_SHIFT);
            }
            ds.count
        };
        let tex = count(Locality::tex_freelist());
        let uni = count(Locality::uniform());
        assert!(
            uni > tex * 4,
            "uniform dirtied {uni} chunks, tex-freelist {tex}: the model is not \
             distinguishing locality"
        );
        assert!(uni <= l.chunks());
    }
}
