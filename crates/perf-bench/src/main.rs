//! FT-070 engine performance harness.
//!
//! Measures the product render path — the same `protocol::handle_line` the
//! `flashtex-render` worker serves — over the committed real-world corpus and
//! four generated documents, and reports cold timings with a per-phase split,
//! warm single-keystroke timings, PDF export, peak RSS and an output digest
//! per case. `--baseline ... --check` turns the same run into a CI gate.
//!
//! Three rules the harness enforces on itself, because breaking any of them
//! produces numbers that look fine and mean nothing:
//!
//! 1. **No measurement without fonts.** Latin Modern and the pinned 12 pt TFM
//!    set must load, and no render may raise a font diagnostic. A substituted
//!    metric is not a slower run of the same document, it is a different
//!    document. Any violation aborts.
//! 2. **Cold means cold.** Every repetition runs in a child process of its
//!    own, so the expander's thread-local cache, the font set's memoised
//!    faces and the allocator all start empty, and peak RSS belongs to one
//!    document.
//! 3. **Medians, with the spread shown.** Nothing is reported from a single
//!    sample, and the gate will not call anything a regression unless it is
//!    larger than that metric's own measured spread.
//!
//! See README.md for the invocations and for what the numbers do and do not
//! cover.

mod corpus;
mod fontgate;
mod measure;
mod warmprof;
mod report;
mod stats;
mod sys;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::process::Command;

use flashtex_compiler::json::{self, Value};

use corpus::{Case, Group};
use fontgate::FontConfig;
use measure::{Caps, RunConfig, Sample};
use report::{CaseReport, Report};

const CHILD_PREFIX: &str = "FTPB1 ";
/// A case that cannot be measured validly — almost always a missing face or
/// metric. Reported on its own channel so one unmeasurable fixture does not
/// cost the other thirteen their numbers, while still never producing a
/// timing for it.
const CHILD_SKIP: &str = "FTPBSKIP ";

struct Args {
    repo: PathBuf,
    fonts_flag: Option<String>,
    tfm_flag: Option<String>,
    runs: usize,
    warmup: usize,
    steps: Option<usize>,
    only: Vec<String>,
    caps: Vec<Caps>,
    scenarios: Vec<String>,
    export: bool,
    export_max_pages: usize,
    json_out: Option<String>,
    table: bool,
    baseline: Option<String>,
    check: bool,
    require_same_host: bool,
    tolerance: f64,
    pin: Option<String>,
    in_process: bool,
    dump_corpus: Option<String>,
    list: bool,
    print: Option<String>,
    flamegraph: Option<String>,
    flamegraph_case: String,
    child: Option<String>,
    warm_profile: Option<String>,
    warm_profile_scenario: String,
}

fn default_repo() -> PathBuf {
    // crates/perf-bench -> crates -> repo root.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap_or_else(|_| PathBuf::from("."))
}

fn parse_args() -> Args {
    let mut a = Args {
        repo: default_repo(),
        fonts_flag: None,
        tfm_flag: None,
        runs: 5,
        warmup: 1,
        steps: None,
        only: Vec::new(),
        caps: vec![Caps::V1, Caps::V2],
        scenarios: measure::SCENARIOS.iter().map(|s| s.to_string()).collect(),
        export: true,
        export_max_pages: 500,
        json_out: None,
        table: true,
        baseline: None,
        check: false,
        require_same_host: false,
        tolerance: 5.0,
        pin: None,
        in_process: false,
        dump_corpus: None,
        list: false,
        print: None,
        flamegraph: None,
        flamegraph_case: "hw1".into(),
        child: None,
        warm_profile: None,
        warm_profile_scenario: "type-paragraph".into(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut next = || it.next().unwrap_or_else(|| fail(&format!("{arg} needs a value")));
        match arg.as_str() {
            "--repo" => a.repo = PathBuf::from(next()),
            "--fonts" => a.fonts_flag = Some(next()),
            "--tfm-dirs" => a.tfm_flag = Some(next()),
            "--runs" => {
                a.runs = next().parse().unwrap_or_else(|_| fail("--runs N"));
                if a.runs == 0 {
                    fail("--runs must be at least 1; nothing is reported from zero samples");
                }
            }
            "--warmup" => a.warmup = next().parse().unwrap_or_else(|_| fail("--warmup N")),
            "--steps" => a.steps = Some(next().parse().unwrap_or_else(|_| fail("--steps N"))),
            "--only" => a.only.push(next()),
            "--caps" => {
                a.caps = match next().as_str() {
                    "v1" => vec![Caps::V1],
                    "v2" => vec![Caps::V2],
                    "both" => vec![Caps::V1, Caps::V2],
                    other => fail(&format!("--caps v1|v2|both, not {other}")),
                }
            }
            "--scenarios" => a.scenarios = next().split(',').map(str::to_string).collect(),
            "--no-export" => a.export = false,
            "--export-max-pages" => a.export_max_pages = next().parse().unwrap_or_else(|_| fail("--export-max-pages N")),
            "--quick" => {
                a.runs = 3;
                a.warmup = 0;
                a.steps = Some(6);
                a.caps = vec![Caps::V2];
            }
            "--json" | "--update-baseline" => a.json_out = Some(next()),
            "--quiet" => a.table = false,
            "--baseline" => a.baseline = Some(next()),
            "--check" => a.check = true,
            "--require-same-host" => a.require_same_host = true,
            "--tolerance" => a.tolerance = next().parse().unwrap_or_else(|_| fail("--tolerance PCT")),
            "--pin" => a.pin = Some(next()),
            "--in-process" => a.in_process = true,
            "--dump-corpus" => a.dump_corpus = Some(next()),
            "--list" => a.list = true,
            "--print" => a.print = Some(next()),
            "--flamegraph" => a.flamegraph = Some(next()),
            "--flamegraph-case" => a.flamegraph_case = next(),
            "--child" => a.child = Some(next()),
            "--warm-profile" => a.warm_profile = Some(next()),
            "--warm-profile-scenario" => a.warm_profile_scenario = next(),
            "--help" | "-h" => {
                println!("{}", usage());
                std::process::exit(0);
            }
            other => fail(&format!("unknown argument {other}")),
        }
    }
    a
}

fn usage() -> &'static str {
    "flashtex-perf-bench [options]

  --repo PATH           repository root (default: two levels above this crate)
  --fonts DIR[:DIR]     font directories, exactly (no host probing)
  --tfm-dirs DIR[:DIR]  metric directories (default: derived from <fonts>/texmf)
  --runs N              repetitions per case, each in its own process (5)
  --warmup N            repetitions run and discarded first (1)
  --steps N             keystrokes per warm scenario (auto: 20/12/8 by size)
  --only SUBSTR         measure only cases whose id contains SUBSTR (repeatable)
  --caps v1|v2|both     negotiated layout capabilities (both)
  --scenarios a,b,c     warm scenarios (all four)
  --no-export           skip the PDF export measurement
  --export-max-pages N  do not attempt an export above N pages (500)
  --quick               3 runs, no warmup, 6 steps, v2 only (smoke test)
  --json PATH           write the JSON report (also spelled --update-baseline)
  --quiet               suppress the human table
  --baseline PATH       compare against a committed baseline report
  --check               with --baseline: exit 1 on a regression or a digest change
  --require-same-host   with --check: enforce timings only when the baseline's
                        host, build profile and font configuration match; a
                        digest change still fails anywhere
  --tolerance PCT       regression threshold (5.0)
  --pin CPU             run children under `taskset -c CPU`
  --in-process          do not fork per repetition (debugging; RSS is cumulative)
  --dump-corpus DIR     write every case's documents to DIR and exit
  --list                list the corpus and exit
  --print REPORT.json   re-render a committed report as the human table and exit;
                        with --baseline, compare two recorded runs instead of
                        measuring a new one
  --flamegraph DIR      opt-in profile of one case into DIR (needs `perf`)
  --flamegraph-case ID  which case to profile (hw1)
  --warm-profile ID     per-phase split of ONE WARM KEYSTROKE for that case and
                        exit (the cold split in the table is a different shape)
  --warm-profile-scenario NAME  which warm scenario to split (type-paragraph)

Fonts resolve from --fonts/--tfm-dirs, else FLASHTEX_FONT_DIRS/FLASHTEX_TFM_DIRS,
else <repo>/apps/mac/Fonts with the metric directories derived from its texmf.
The set is built with FontSet::with_dirs, so a host TeX Live installation is
never searched and cannot silently supply substitute metrics."
}

fn fail(msg: &str) -> ! {
    eprintln!("flashtex-perf-bench: {msg}");
    std::process::exit(2);
}

/// Bigger documents get fewer keystrokes: the point is a stable median, and
/// at 2 MB twenty steps cost minutes without narrowing the distribution.
fn steps_for(bytes: usize) -> usize {
    match bytes {
        b if b > 1_000_000 => 5,
        b if b > 300_000 => 10,
        _ => 20,
    }
}

/// ... and fewer repetitions, for the same reason. Every metric carries its
/// own `n`, so a reader always sees how many samples a number rests on.
fn runs_for(bytes: usize, runs: usize) -> usize {
    match bytes {
        b if b > 1_000_000 => runs.min(2),
        b if b > 300_000 => runs.min(4),
        _ => runs,
    }
}

/// Above 1 MB one full render costs tens of seconds, so the largest case
/// measures one capability set and two scenarios rather than eight
/// combinations. Two scenarios still answer the question the target asks —
/// whether a keystroke is anywhere near 10 ms — and the reduction is recorded
/// in `meta` and visible in each metric's `n`.
fn plan_for(bytes: usize, caps: &[Caps], scenarios: &[String]) -> (Vec<Caps>, Vec<String>) {
    if bytes <= 1_000_000 {
        return (caps.to_vec(), scenarios.to_vec());
    }
    let keep = ["type-paragraph", "type-display-eq"];
    let narrowed: Vec<String> = scenarios.iter().filter(|s| keep.contains(&s.as_str())).cloned().collect();
    (vec![Caps::V2], if narrowed.is_empty() { scenarios.to_vec() } else { narrowed })
}

fn main() {
    let args = parse_args();

    if let Some(id) = &args.child {
        run_child(&args, id);
        return;
    }

    // Warm per-phase split. In-process on purpose: the point is the shape of a
    // keystroke against a warm cache, and a fresh child would have no warm cache.
    if let Some(want) = &args.warm_profile {
        let cases = corpus::all_cases(&args.repo);
        let Some(case) = cases.iter().find(|c| c.id.contains(want.as_str())) else {
            fail(&format!("no case matching '{want}'"));
        };
        let steps = args.steps.unwrap_or(12);
        let cfg = run_config(&args, case.bytes(), steps);
        if let Err(e) = warmprof::run(case, &cfg, &args.warm_profile_scenario) {
            fail(&e);
        }
        return;
    }

    // Re-render a report that was measured elsewhere or earlier. The table is
    // derived from the JSON, never stored twice, so a committed report and its
    // committed table can never drift apart.
    if let Some(p) = &args.print {
        let r = read_report(p);
        if args.table {
            print!("{}", report::human_table(&r));
        }
        // With --baseline as well, compare two recorded runs instead of
        // measuring a new one. Running the same commit twice and diffing the
        // results is how the harness's own reproducibility is shown, rather
        // than asserted.
        if let Some(b) = &args.baseline {
            let base = read_report(b);
            let outcome = report::gate(&base, &r, args.tolerance);
            print!("\n{}", report::gate_table(&outcome, args.tolerance));
            println!("{}", report::gate_summary(&outcome, args.require_same_host));
            if args.check && report::gate_failed(&outcome, args.require_same_host) {
                std::process::exit(1);
            }
        }
        return;
    }

    let mut cases = corpus::all_cases(&args.repo);
    if !args.only.is_empty() {
        cases.retain(|c| args.only.iter().any(|o| c.id.contains(o.as_str())));
    }
    if cases.is_empty() {
        fail("no cases selected");
    }

    if args.list {
        for c in &cases {
            println!("{:<20} {:<12} {:>9} bytes  {} document(s)  entry {}", c.id, c.group.as_str(), c.bytes(), c.docs.len(), c.entry);
        }
        return;
    }
    if let Some(dir) = &args.dump_corpus {
        dump_corpus(&cases, Path::new(dir));
        return;
    }
    if let Some(dir) = &args.flamegraph {
        flamegraph(&args, &cases, Path::new(dir));
        return;
    }

    // Fail before spending minutes on a run that cannot produce a valid
    // number, and with the same message the child would print.
    let font_config = fontgate::resolve(&args.repo, args.fonts_flag.as_deref(), args.tfm_flag.as_deref());
    {
        let fonts = font_config.build();
        if let Err(e) = fontgate::preflight(&fonts) {
            fail(&format!(
                "font preflight failed, refusing to measure:\n  {e}\n  configuration came from {} -> fonts {} / metrics {}",
                font_config.source,
                font_config.font_dirs_arg(),
                font_config.tfm_dirs_arg()
            ));
        }
    }

    let cal_before = sys::calibration_ns();
    let load_before = sys::load_avg();
    let started = sys::utc_now();
    let mut reports: Vec<CaseReport> = Vec::new();
    let mut unmeasured: Vec<(String, String)> = Vec::new();
    let mut load_max: f64 = load_before.map_or(0.0, |l| l.0);

    for case in &cases {
        let steps = args.steps.unwrap_or_else(|| steps_for(case.bytes()));
        let runs = runs_for(case.bytes(), args.runs);
        eprint!("{:<20} {} bytes, {} run(s) x {} step(s): ", case.id, case.bytes(), runs, steps);
        let mut samples: Vec<Sample> = Vec::new();
        let mut refused: Option<String> = None;
        for rep in 0..(runs + args.warmup) {
            let sample = if args.in_process {
                measure::run(case, &run_config(&args, case.bytes(), steps))
            } else {
                spawn_child(&args, case, steps)
            };
            match sample {
                Ok(sample) => {
                    if let Some(l) = sample.load_after {
                        load_max = load_max.max(l.0);
                    }
                    if rep >= args.warmup {
                        samples.push(sample);
                    }
                    eprint!(".");
                }
                Err(why) => {
                    refused = Some(why);
                    break;
                }
            }
        }
        if let Some(why) = refused {
            eprintln!(" NOT MEASURED");
            eprintln!("    {why}");
            unmeasured.push((case.id.clone(), why));
            continue;
        }
        let r = report::aggregate(case.group.as_str(), &samples);
        eprintln!(
            " cold {:.2} ms, warm p50 {:.3} ms, peak {} MiB",
            r.metrics.get("cold.render_ms").map_or(f64::NAN, |s| s.median),
            r.metrics.iter().filter(|(k, _)| k.starts_with("warm.")).map(|(_, s)| s.median).fold(f64::NAN, f64::max),
            (r.metrics.get("mem.peak_rss_kb").map_or(0.0, |s| s.median) / 1024.0).round()
        );
        reports.push(r);
    }

    let cal_after = sys::calibration_ns();
    let fonts = font_config.build();
    let meta = build_meta(&args, &started, cal_before, cal_after, load_before, load_max, &font_config, &fonts);
    let report = Report { meta, cases: reports, unmeasured: unmeasured.clone() };

    if args.table {
        print!("{}", report::human_table(&report));
    }
    if let Some(p) = &args.json_out {
        let text = format!("{}\n", json::write(&report.to_json()));
        std::fs::write(p, text).unwrap_or_else(|e| fail(&format!("writing {p}: {e}")));
        eprintln!("wrote {p}");
    }

    if !unmeasured.is_empty() {
        eprintln!();
        eprintln!("{} case(s) produced no timing at all, because a valid measurement was not possible:", unmeasured.len());
        for (id, why) in &unmeasured {
            eprintln!("  {id}: {why}");
        }
        eprintln!("A run on substituted faces or metrics is a different workload, not a slower one, so no number is reported for these.");
    }

    let mut failed = false;
    if let Some(p) = &args.baseline {
        let base = read_report(p);
        let outcome = report::gate(&base, &report, args.tolerance);
        print!("\n{}", report::gate_table(&outcome, args.tolerance));
        println!("{}", report::gate_summary(&outcome, args.require_same_host));
        failed = report::gate_failed(&outcome, args.require_same_host);
    }
    if args.check && failed {
        std::process::exit(1);
    }
}

fn read_report(path: &str) -> Report {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| fail(&format!("reading {path}: {e}")));
    let v = json::parse(&text).unwrap_or_else(|e| fail(&format!("parsing {path}: {e:?}")));
    Report::from_json(&v).unwrap_or_else(|| fail(&format!("{path} is not a perf-bench report")))
}

fn run_config(args: &Args, bytes: usize, steps: usize) -> RunConfig {
    let (caps, scenarios) = plan_for(bytes, &args.caps, &args.scenarios);
    RunConfig {
        fonts: fontgate::resolve(&args.repo, args.fonts_flag.as_deref(), args.tfm_flag.as_deref()),
        steps,
        export_max_pages: args.export_max_pages,
        caps,
        scenarios,
        export: args.export,
    }
}

fn run_child(args: &Args, id: &str) {
    let cases = corpus::all_cases(&args.repo);
    let Some(case) = cases.iter().find(|c| c.id == id) else {
        fail(&format!("child: no case {id}"));
    };
    let steps = args.steps.unwrap_or_else(|| steps_for(case.bytes()));
    match measure::run(case, &run_config(args, case.bytes(), steps)) {
        Ok(sample) => println!("{CHILD_PREFIX}{}", json::write(&sample.to_json())),
        Err(e) => {
            println!("{CHILD_SKIP}{}", e.replace('\n', " | "));
            std::process::exit(3);
        }
    }
}

fn spawn_child(args: &Args, case: &Case, steps: usize) -> Result<Sample, String> {
    let exe = std::env::current_exe().unwrap_or_else(|e| fail(&format!("current_exe: {e}")));
    let mut cmd = match &args.pin {
        Some(cpu) => {
            let mut c = Command::new("taskset");
            c.arg("-c").arg(cpu).arg(&exe);
            c
        }
        None => Command::new(&exe),
    };
    let fonts = fontgate::resolve(&args.repo, args.fonts_flag.as_deref(), args.tfm_flag.as_deref());
    cmd.arg("--child").arg(&case.id);
    cmd.arg("--repo").arg(&args.repo);
    cmd.arg("--fonts").arg(fonts.font_dirs_arg());
    cmd.arg("--tfm-dirs").arg(fonts.tfm_dirs_arg());
    cmd.arg("--steps").arg(steps.to_string());
    let (caps, scenarios) = plan_for(case.bytes(), &args.caps, &args.scenarios);
    cmd.arg("--caps").arg(match caps[..] {
        [Caps::V1] => "v1",
        [Caps::V2] => "v2",
        _ => "both",
    });
    cmd.arg("--scenarios").arg(scenarios.join(","));
    if !args.export {
        cmd.arg("--no-export");
    }
    cmd.arg("--export-max-pages").arg(args.export_max_pages.to_string());
    let out = cmd.output().unwrap_or_else(|e| fail(&format!("spawning the measurement child: {e}")));
    let stdout = String::from_utf8_lossy(&out.stdout);
    if let Some(why) = stdout.lines().find_map(|l| l.strip_prefix(CHILD_SKIP)) {
        return Err(why.to_string());
    }
    let Some(line) = stdout.lines().find_map(|l| l.strip_prefix(CHILD_PREFIX)) else {
        // The child died without reporting — a panic, a signal, an OOM kill.
        // That is worth knowing about and worth a non-zero exit at the end,
        // but it must not throw away the results of every other case: a
        // thirty-minute suite that loses everything to one crashing fixture
        // is a suite nobody will run.
        let stderr = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = stderr.trim().lines().rev().take(4).collect();
        return Err(format!(
            "the measurement child exited {:?} without a result: {}",
            out.status.code(),
            tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
        ));
    };
    let v = json::parse(line).unwrap_or_else(|e| fail(&format!("{}: unreadable child result: {e:?}", case.id)));
    Ok(Sample::from_json(&v).unwrap_or_else(|| fail(&format!("{}: incomplete child result", case.id))))
}

fn build_meta(
    args: &Args,
    started: &str,
    cal_before: u64,
    cal_after: u64,
    load_before: Option<(f64, f64, f64)>,
    load_max: f64,
    font_config: &FontConfig,
    fonts: &flashtex_render_pipeline::FontSet,
) -> Vec<(String, Value)> {
    let host = sys::Host::detect();
    let (commit, dirty) = sys::git_state(&args.repo);
    let s = |x: &str| json::str_(x);
    let font_desc = fontgate::describe(font_config, fonts);
    let font_summary = font_desc.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(" ");
    let mut meta = vec![
        ("harness".to_string(), s("flashtex-perf-bench")),
        ("run_utc".to_string(), s(started)),
        ("finished_utc".to_string(), s(&sys::utc_now())),
        ("repo_commit".to_string(), s(&commit)),
        ("repo_dirty".to_string(), Value::Bool(dirty)),
        ("host".to_string(), s(&host.hostname)),
        ("host_fingerprint".to_string(), s(&host.fingerprint())),
        ("cpu".to_string(), s(&host.cpu_model)),
        ("logical_cpus".to_string(), json::num(host.logical_cpus as f64)),
        ("cpu_governor".to_string(), host.cpu_governor.as_deref().map_or(Value::Null, s)),
        ("cpu_boost".to_string(), host.boost.as_deref().map_or(Value::Null, s)),
        ("mem_total_kb".to_string(), host.mem_total_kb.map_or(Value::Null, |x| json::num(x as f64))),
        ("os".to_string(), s(&format!("{} {}", host.os, host.arch))),
        ("build_profile".to_string(), s(env!("FT_PROFILE"))),
        ("build_opt_level".to_string(), s(env!("FT_OPT_LEVEL"))),
        ("build_debug".to_string(), s(env!("FT_DEBUG"))),
        ("build_target".to_string(), s(env!("FT_TARGET"))),
        ("build_rustflags".to_string(), s(env!("FT_RUSTFLAGS"))),
        ("build_lto".to_string(), s("false (cargo stock release; codegen-units=16)")),
        ("runs".to_string(), json::num(args.runs as f64)),
        ("warmup_runs".to_string(), json::num(args.warmup as f64)),
        ("steps".to_string(), args.steps.map_or(s("auto: 20 / 10 at >300 KB / 5 at >1 MB"), |n| json::num(n as f64))),
        ("runs_scaling".to_string(), s("requested runs, capped at 4 above 300 KB and 2 above 1 MB; above 1 MB also v2 only and two scenarios. Each metric carries its own n")),
        ("export_max_pages".to_string(), json::num(args.export_max_pages as f64)),
        ("isolation".to_string(), s(if args.in_process { "in-process (RSS is cumulative, not per case)" } else { "one child process per repetition" })),
        ("pinned_cpu".to_string(), args.pin.as_deref().map_or(Value::Null, s)),
        ("caps".to_string(), s(&args.caps.iter().map(|c| c.as_str()).collect::<Vec<_>>().join("+"))),
        ("calibration_ns".to_string(), json::num(((cal_before + cal_after) / 2) as f64)),
        ("calibration_drift_pct".to_string(), json::num(stats::round6((cal_after as f64 - cal_before as f64) / cal_before as f64 * 100.0))),
        ("load1_start".to_string(), load_before.map_or(Value::Null, |l| json::num(l.0))),
        ("load1_max".to_string(), json::num(stats::round6(load_max))),
        ("font_config".to_string(), s(&font_summary)),
    ];
    for (k, v) in font_desc {
        meta.push((format!("fonts_{k}"), json::str_(v)));
    }
    meta
}

fn dump_corpus(cases: &[Case], dir: &Path) {
    for c in cases {
        if c.group != Group::Synthetic {
            continue;
        }
        let out = dir.join(&c.id);
        std::fs::create_dir_all(&out).unwrap_or_else(|e| fail(&format!("{}: {e}", out.display())));
        for d in &c.docs {
            let p = out.join(&d.path);
            std::fs::write(&p, &d.text).unwrap_or_else(|e| fail(&format!("{}: {e}", p.display())));
            println!("{} ({} bytes)", p.display(), d.text.len());
        }
    }
    println!("real-world cases are read from fixtures/real-world and are not re-written");
}

/// Opt-in profiling. The basic harness never needs a profiler; this exists so
/// a regression the phase split points at can be taken down to a symbol.
fn flamegraph(args: &Args, cases: &[Case], dir: &Path) {
    let Some(case) = cases.iter().find(|c| c.id == args.flamegraph_case) else {
        fail(&format!("--flamegraph-case {}: no such case", args.flamegraph_case));
    };
    let fonts = fontgate::resolve(&args.repo, args.fonts_flag.as_deref(), args.tfm_flag.as_deref());
    if Command::new("perf").arg("--version").output().is_err() {
        fail("`perf` is not on PATH. The basic harness does not need it; install linux-perf (or use samply) for --flamegraph.");
    }
    std::fs::create_dir_all(dir).unwrap_or_else(|e| fail(&format!("{}: {e}", dir.display())));
    let exe = std::env::current_exe().unwrap_or_else(|e| fail(&format!("current_exe: {e}")));
    let data = dir.join(format!("{}.perf.data", case.id));
    let steps = args.steps.unwrap_or_else(|| steps_for(case.bytes()));
    eprintln!("perf record -> {}", data.display());
    let status = Command::new("perf")
        .args(["record", "-F", "997", "-g", "--call-graph", "dwarf,16384", "-o"])
        .arg(&data)
        .arg("--")
        .arg(&exe)
        .args(["--child", &case.id, "--repo"])
        .arg(&args.repo)
        .args(["--fonts", &fonts.font_dirs_arg(), "--tfm-dirs", &fonts.tfm_dirs_arg()])
        .args(["--steps", &steps.to_string(), "--caps", "v2", "--no-export"])
        .status()
        .unwrap_or_else(|e| fail(&format!("perf record: {e}")));
    if !status.success() {
        fail("perf record failed. On a locked-down kernel: sysctl kernel.perf_event_paranoid=1");
    }
    let folded = dir.join(format!("{}.folded", case.id));
    let script = Command::new("perf").arg("script").arg("-i").arg(&data).output().unwrap_or_else(|e| fail(&format!("perf script: {e}")));
    std::fs::write(&folded, &script.stdout).unwrap_or_else(|e| fail(&format!("{}: {e}", folded.display())));
    eprintln!("wrote {} ({} bytes of raw `perf script` output)", folded.display(), script.stdout.len());
    for tool in ["inferno-flamegraph", "flamegraph.pl"] {
        if Command::new(tool).arg("--help").output().is_ok() {
            eprintln!("  collapse and render with:  perf script -i {} | inferno-collapse-perf | {tool} > {}/{}.svg", data.display(), dir.display(), case.id);
            break;
        }
    }
    let attribution = Command::new("perf")
        .args(["report", "--stdio", "--no-children", "-g", "none", "--percent-limit", "0.5", "-i"])
        .arg(&data)
        .output()
        .unwrap_or_else(|e| fail(&format!("perf report: {e}")));
    let out = dir.join(format!("{}.symbols.txt", case.id));
    std::fs::write(&out, &attribution.stdout).unwrap_or_else(|e| fail(&format!("{}: {e}", out.display())));
    eprintln!("wrote {}", out.display());
    eprintln!("Self time by crate (the split `typeset` cannot give from outside the pipeline):");
    print!("{}", crate_attribution(&String::from_utf8_lossy(&attribution.stdout)));
}

/// Folds `perf report --stdio` self-time percentages into the engine crates,
/// which is the finer phase split (shaping vs line breaking vs math vs page
/// building) that the public seams cannot provide.
fn crate_attribution(report: &str) -> String {
    let buckets: &[(&str, &str)] = &[
        ("flashtex_paragraph_layout", "paragraph/line breaking"),
        ("flashtex_math_layout", "math layout"),
        ("flashtex_font_engine", "font engine / shaping"),
        ("flashtex_render_pipeline::pagebuild", "page building"),
        ("flashtex_render_pipeline::typeset", "typesetting"),
        ("flashtex_render_pipeline::display", "display-list emission"),
        ("flashtex_render_pipeline::adapter", "adapt"),
        ("flashtex_pdf", "pdf export"),
        ("flashtex_compiler::expansion", "expansion"),
        ("flashtex_compiler::parser", "parse"),
        ("flashtex_compiler::json", "json"),
        ("flashtex_vector_graphics", "tikz"),
    ];
    let mut totals: Vec<(f64, &str)> = buckets.iter().map(|(_, label)| (0.0, *label)).collect();
    let mut other = 0.0;
    for line in report.lines() {
        let line = line.trim();
        let Some(pct) = line.split('%').next().and_then(|p| p.trim().parse::<f64>().ok()) else { continue };
        match buckets.iter().position(|(pat, _)| line.contains(pat)) {
            Some(i) => totals[i].0 += pct,
            None => other += pct,
        }
    }
    totals.sort_by(|a, b| b.0.partial_cmp(&a.0).expect("finite"));
    let mut out = String::new();
    for (pct, label) in totals.iter().filter(|(p, _)| *p > 0.0) {
        out.push_str(&format!("  {pct:>6.1}%  {label}\n"));
    }
    out.push_str(&format!("  {other:>6.1}%  everything else (std, allocator, unresolved)\n"));
    out
}
