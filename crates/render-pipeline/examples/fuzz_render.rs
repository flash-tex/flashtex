//! Mutation fuzzer for the render pipeline and the exact PDF route, on
//! stable Rust.
//!
//! ```text
//! cargo run --release --example fuzz_render -- [--cases 50000] [--seed 0xF1A5] [--jobs 8]
//!     [--timeout-ms 10000] [--max-rss-mb 4096] [--out DIR]
//! cargo run --release --example fuzz_render -- --replay CASE [--seed S] [--out DIR]
//! cargo run --release --example fuzz_render -- --check FILE.tex
//! cargo run --release --example fuzz_render -- --minimise FILE.tex
//! cargo run --release --example fuzz_render -- --diagnostics FILE.tex
//! ```
//!
//! Each case runs the compiler alone (`parse`), `protocol::handle_line`
//! (`render`: what `flashtex-render` and the worker serve) and
//! `pdf::write_pdf_exact` (`pdf`: what `flashtex build` writes) against a
//! project directory holding the case and hostile images. Panics are caught
//! per case; hangs (per-case watchdog), stack overflows and memory blow-ups
//! (supervisor RSS watchdog) are isolated in worker processes. A finding's
//! input is written to `DIR/case-N.tex` (and `case-N.sub.tex` for a second
//! project document). See `tests/fuzz_support/mod.rs`.

#[path = "../tests/fuzz_support/mod.rs"]
mod fuzz_support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use fuzz_support::*;

fn arg<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    let at = args.iter().position(|a| a == name)?;
    args.get(at + 1)?.parse().ok()
}

fn parse_seed(args: &[String]) -> u64 {
    let raw: String = arg(args, "--seed").unwrap_or_else(|| "0xF1A5_7E40".into());
    let raw = raw.replace('_', "");
    match raw.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).expect("--seed"),
        None => raw.parse().expect("--seed"),
    }
}

fn default_out() -> PathBuf {
    let base = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("fuzz-render-findings")
}

fn load_case_file(path: &Path) -> Case {
    let mut case = Case::from_text(std::fs::read_to_string(path).expect("read case"));
    let sub = path.with_extension("sub.tex");
    if let Ok(text) = std::fs::read_to_string(&sub) {
        case.documents.push(("sub.tex".into(), text));
    }
    case
}

/// Runs `--check` on `text` in a child process and returns its signature.
fn check_in_child(exe: &Path, text: &str, sub: Option<&str>, scratch: &Path, timeout_ms: u64) -> String {
    let file = scratch.join("probe.tex");
    std::fs::write(&file, text).unwrap();
    let sub_file = scratch.join("probe.sub.tex");
    match sub {
        Some(s) => std::fs::write(&sub_file, s).unwrap(),
        None => {
            let _ = std::fs::remove_file(&sub_file);
        }
    }
    let out = Command::new(exe)
        .args([
            "--check",
            file.to_str().unwrap(),
            "--timeout-ms",
            &timeout_ms.to_string(),
            "--out",
            scratch.to_str().unwrap(),
        ])
        .output()
        .expect("spawn check");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if let Some(line) = stdout.lines().find(|l| l.starts_with("signature: ")) {
        return line["signature: ".len()..].to_string();
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stage = stdout
        .lines()
        .filter_map(|l| l.strip_prefix("STAGE\t"))
        .last()
        .unwrap_or("setup");
    if stderr.contains("stack overflow") || stderr.contains("overflowed its stack") {
        format!("crash stack overflow in {stage}")
    } else if stderr.contains("memory allocation") {
        format!("crash allocation failure in {stage}")
    } else {
        format!("crash {} in {stage}", out.status)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rng_seed = parse_seed(&args);
    let timeout = Duration::from_millis(arg(&args, "--timeout-ms").unwrap_or(10_000));
    let out_dir: PathBuf = arg::<String>(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(default_out);

    if let Some(file) = arg::<String>(&args, "--diagnostics") {
        // In-process (no isolation): stage timings and the PDF outcome.
        let case = load_case_file(Path::new(&file));
        let scratch = scratch_dir(&out_dir);
        prepare_project(&scratch);
        let fonts = flashtex_render_pipeline::FontSet::with_default_dirs(&[]);
        let t = render_case(&case, &fonts, &scratch);
        println!(
            "parse {:.0} ms, render {:.0} ms ({}, {} pages, {} diagnostics), pdf {:.0} ms: {:?}",
            t.parse_ms, t.render_ms, t.status, t.pages, t.diagnostics, t.pdf_ms, t.pdf
        );
        let _ = std::fs::remove_dir_all(&scratch);
        return;
    }

    if let Some(file) = arg::<String>(&args, "--check") {
        std::env::set_var("FLASHTEX_FUZZ_WORKER_PROTOCOL", "1");
        install_quiet_panic_hook();
        std::fs::create_dir_all(&out_dir).unwrap();
        let scratch = scratch_dir(&out_dir);
        let runner = Runner::new(timeout, scratch.clone());
        let outcome = runner.run(load_case_file(Path::new(&file)));
        println!("signature: {}", outcome.signature());
        if let Outcome::Panic(loc, msg) = &outcome {
            println!("location: {loc}\nmessage: {msg}");
        }
        let _ = std::fs::remove_dir_all(&scratch);
        std::process::exit(if matches!(outcome, Outcome::Hang(_)) { 3 } else { 0 });
    }

    if let Some(file) = arg::<String>(&args, "--minimise") {
        let exe = std::env::current_exe().unwrap();
        let case = load_case_file(Path::new(&file));
        let sub = case.documents.get(1).map(|d| d.1.clone());
        let scratch = out_dir.join(format!("min-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let ms = timeout.as_millis() as u64;
        let target = check_in_child(&exe, case.main(), sub.as_deref(), &scratch, ms);
        println!("target signature: {target}");
        if target == "ok" {
            return;
        }
        let mut tries = 0;
        let min = minimise(case.main(), &mut |candidate| {
            tries += 1;
            check_in_child(&exe, candidate, sub.as_deref(), &scratch, ms) == target
        });
        let dest = Path::new(&file).with_extension("min.tex");
        std::fs::write(&dest, &min).unwrap();
        let _ = std::fs::remove_dir_all(&scratch);
        println!("{tries} probes; {} -> {} bytes: {}", case.main().len(), min.len(), dest.display());
        if min.len() < 400 {
            println!("{min:?}");
        }
        return;
    }

    let seeds = load_seeds(&repo_root());
    assert!(!seeds.is_empty(), "no .tex seeds found under {}", repo_root().display());

    if let Some(index) = arg::<u64>(&args, "--replay") {
        let case = generate(&seeds, rng_seed, index);
        std::fs::create_dir_all(&out_dir).unwrap();
        let path = out_dir.join(format!("case-{index}.tex"));
        std::fs::write(&path, case.main()).unwrap();
        if let Some((_, sub)) = case.documents.get(1) {
            std::fs::write(path.with_extension("sub.tex"), sub).unwrap();
        }
        println!("case {index}: seed {} mutations {:?} -> {}", case.seed_name, case.mutations, path.display());
        return;
    }

    if let Some(start) = args.iter().position(|a| a == "--worker") {
        let begin: u64 = args[start + 1].parse().unwrap();
        let end: u64 = args[start + 2].parse().unwrap();
        worker(&seeds, rng_seed, begin, end, timeout, &out_dir);
        return;
    }

    let config = Config {
        rng_seed,
        cases: arg(&args, "--cases").unwrap_or(50_000),
        jobs: arg(&args, "--jobs").unwrap_or(8),
        timeout,
        out_dir,
        max_rss_mb: arg(&args, "--max-rss-mb").unwrap_or(DEFAULT_MAX_RSS_MB),
    };
    println!(
        "fuzz: {} seeds, {} cases, seed {:#x}, {} jobs, findings -> {}",
        seeds.len(),
        config.cases,
        config.rng_seed,
        config.jobs,
        config.out_dir.display()
    );
    let exe = std::env::current_exe().unwrap();
    let ms = config.timeout.as_millis().to_string();
    let seed = config.rng_seed.to_string();
    let out = config.out_dir.display().to_string();
    let command = |start: u64, end: u64| {
        let mut c = Command::new(&exe);
        c.args(["--worker", &start.to_string(), &end.to_string(), "--seed", &seed, "--timeout-ms", &ms, "--out", &out]);
        c
    };
    let report = supervise(&seeds, &config, &command);
    print_report(&report);
}
