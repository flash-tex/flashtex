//! Diagnostic floods: a runaway loop reports each of its problems once, in
//! bounded time and memory, instead of once per iteration.
//!
//! Before the bound, `\def\a{\n\a}\a` reported 667k diagnostics (0.7 s,
//! 600 MB in release) and a mutated microtype fixture 3.8 million (4.5 s,
//! 4.9 GB). Each case here runs in a child process, the test binary
//! re-executed as the fuzz harness does (`tests/fuzz_support`), so a
//! regression that exhausts memory or never finishes is killed by the
//! parent's watchdog instead of taking the test runner down, and the child's
//! peak heap is measured from a clean start.
//!
//! Bounds are for an unoptimised test build on a loaded machine: generous
//! against the measured values, far below the flood's.

use std::alloc::{GlobalAlloc, Layout, System};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::SourceDocument;

/// Counts live heap bytes and their peak.
struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new = unsafe { System.realloc(ptr, layout, new_size) };
        if !new.is_null() {
            if new_size >= layout.size() {
                let live = LIVE.fetch_add(new_size - layout.size(), Ordering::Relaxed)
                    + (new_size - layout.size());
                PEAK.fetch_max(live, Ordering::Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        new
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const CASE_ENV: &str = "FLASHTEX_FLOOD_CASE";
const WATCHDOG: Duration = Duration::from_secs(120);

/// What the child process measured.
struct Flood {
    messages: Vec<String>,
    millis: u128,
    peak_bytes: usize,
}

impl Flood {
    fn count(&self, message: &str) -> usize {
        self.messages.iter().filter(|m| *m == message).count()
    }
}

/// In the parent: re-run this test binary for `test` with `CASE_ENV` set,
/// kill it after [`WATCHDOG`], and parse what it measured. In the child
/// (`CASE_ENV` names `test`): compile `source`, print the measurements, and
/// return `None` so the test body does nothing more.
fn measure(test: &str, source: &str) -> Option<Flood> {
    if std::env::var(CASE_ENV).is_ok_and(|name| name == test) {
        PEAK.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
        let before = LIVE.load(Ordering::Relaxed);
        let started = Instant::now();
        let documents = [SourceDocument {
            path: "main.tex",
            text: source,
        }];
        let out = compile_full_project(&documents, "main.tex", LayoutConstraints::default());
        let millis = started.elapsed().as_millis();
        let peak = PEAK.load(Ordering::Relaxed).saturating_sub(before);
        // The harness has already printed "test <name> ... " on this line.
        println!("\nFLOOD\t{millis}\t{peak}");
        for d in &out.diagnostics {
            let short: String = d.message.chars().take(300).collect();
            println!("MSG\t{}", short.replace('\n', " "));
        }
        return None;
    }
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CASE_ENV, test)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn the case process");
    let stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || std::io::read_to_string(stdout).unwrap_or_default());
    let deadline = Instant::now() + WATCHDOG;
    loop {
        if child.try_wait().expect("wait").is_some() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{test}: the case process ran past {WATCHDOG:?} and was killed");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = reader.join().unwrap();
    let mut flood = Flood {
        messages: Vec::new(),
        millis: 0,
        peak_bytes: 0,
    };
    let mut measured = false;
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("FLOOD\t") {
            let (millis, peak) = rest.split_once('\t').expect("FLOOD line");
            flood.millis = millis.parse().unwrap();
            flood.peak_bytes = peak.parse().unwrap();
            measured = true;
        } else if let Some(message) = line.strip_prefix("MSG\t") {
            flood.messages.push(message.to_string());
        }
    }
    assert!(measured, "{test}: the case process reported nothing:\n{output}");
    eprintln!(
        "{test}: {} diagnostics, {} ms, peak heap {} MB",
        flood.messages.len(),
        flood.millis,
        flood.peak_bytes / MB
    );
    Some(flood)
}

const MB: usize = 1024 * 1024;

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn a_runaway_loop_reports_its_unknown_command_once() {
    // Before: 667,415 diagnostics, one per `\n` emitted before the step
    // limit; 705 MB peak heap in this build. After: 2 diagnostics, 144 MB.
    let Some(flood) = measure(
        "a_runaway_loop_reports_its_unknown_command_once",
        &document("\\def\\a{\\n\\a}\\a"),
    ) else {
        return;
    };
    assert_eq!(flood.count("\\n is not supported by this compiler version"), 1, "{:?}", flood.messages);
    assert_eq!(flood.count("expansion step limit exceeded (possible infinite macro loop)"), 1);
    assert!(flood.messages.len() <= 4, "{:?}", flood.messages);
    assert!(flood.peak_bytes < 300 * MB, "peak heap {} MB", flood.peak_bytes / MB);
    assert!(flood.millis < 60_000, "took {} ms", flood.millis);
}

#[test]
fn a_runaway_loop_reports_each_engine_error_once() {
    // "Missing number" and "Missing =" once per iteration, plus the
    // unsupported command, until the step limit. Before: 572,281
    // diagnostics, 390 MB peak heap. After: 4, 112 MB.
    let Some(flood) = measure(
        "a_runaway_loop_reports_each_engine_error_once",
        &document("\\def\\a{\\ifnum\\relax<1 \\fi\\efcode\\a}\\a"),
    ) else {
        return;
    };
    assert_eq!(flood.count("Missing number, treated as zero."), 1, "{:?}", flood.messages);
    assert_eq!(flood.count("\\efcode is not supported by this compiler version"), 1);
    assert_eq!(flood.count("expansion step limit exceeded (possible infinite macro loop)"), 1);
    assert!(flood.messages.len() <= 6, "{:?}", flood.messages);
    assert!(flood.peak_bytes < 250 * MB, "peak heap {} MB", flood.peak_bytes / MB);
    assert!(flood.millis < 60_000, "took {} ms", flood.millis);
}

#[test]
fn a_preamble_loop_that_hits_the_input_stack_reports_once() {
    // Before: 19,999 "\section is not supported in the document preamble".
    let Some(flood) = measure(
        "a_preamble_loop_that_hits_the_input_stack_reports_once",
        "\\documentclass{article}\n\\def\\a{\\section{\\a}}\\a\n\\begin{document}\nx\n\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(flood.count("\\section is not supported in the document preamble"), 1, "{:?}", flood.messages);
    assert_eq!(flood.count("TeX capacity exceeded, sorry [input stack size=10000]."), 1);
    assert!(flood.messages.len() <= 4, "{:?}", flood.messages);
    assert!(flood.peak_bytes < 100 * MB, "peak heap {} MB", flood.peak_bytes / MB);
}

#[test]
fn a_runaway_loop_over_a_long_environment_name_stays_bounded() {
    // `\end` with a 100k-character name that never matches, forever: two
    // engine errors per `\end`, each holding the name (fuzz case 10082 peaked
    // at 2.9 GB of them), and the parser's own report per `\end`. Before:
    // 54 diagnostics, 74 MB peak heap. After: 4, 33 MB.
    let name = "x".repeat(100_000);
    let source = document(&format!("\\def\\a{{\\end{{{name}}}\\a}}\\a"));
    let Some(flood) = measure("a_runaway_loop_over_a_long_environment_name_stays_bounded", &source) else {
        return;
    };
    assert_eq!(flood.count("expansion step limit exceeded (possible infinite macro loop)"), 1);
    assert!(flood.messages.len() <= 6, "{:?}", flood.messages.len());
    assert!(flood.peak_bytes < 56 * MB, "peak heap {} MB", flood.peak_bytes / MB);
    assert!(flood.millis < 60_000, "took {} ms", flood.millis);
}

#[test]
fn distinct_diagnostics_past_the_per_code_budget_are_summarised() {
    // 1200 real, distinct reports of one code: the first 1000 are kept and
    // the rest are counted in one summary at the first suppressed span.
    let body: String = (0..1200).map(|i| format!("\\efcode{i} x\n")).collect();
    let Some(flood) = measure(
        "distinct_diagnostics_past_the_per_code_budget_are_summarised",
        &document(&body),
    ) else {
        return;
    };
    assert_eq!(flood.count("\\efcode is not supported by this compiler version"), 1000);
    assert_eq!(flood.count("further 200 similar diagnostics suppressed"), 1);
    assert_eq!(flood.messages.len(), 1001);
    assert_eq!(flood.messages.last().map(String::as_str), Some("further 200 similar diagnostics suppressed"));
}
