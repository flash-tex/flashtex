//! Mutation fuzzer for the compiler on stable Rust.
//!
//! ```text
//! cargo run --release --example fuzz_compile -- [--cases 50000] [--seed 0xF1A5] [--jobs 8]
//!     [--timeout-ms 5000] [--out DIR]
//! cargo run --release --example fuzz_compile -- --replay CASE [--seed S] [--out DIR]
//! cargo run --release --example fuzz_compile -- --check FILE.tex
//! cargo run --release --example fuzz_compile -- --minimise FILE.tex
//! cargo run --release --example fuzz_compile -- --diagnostics FILE.tex [--limit N] [--digest]
//! cargo run --release --example fuzz_compile -- --digest
//! ```
//!
//! Panics are caught per case; hangs (per-case watchdog) and stack overflows
//! are isolated in worker processes. A finding's input is written to
//! `DIR/case-N.tex` (and `case-N.sub.tex` for a second project document).
//! See `tests/fuzz_support/mod.rs`.

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
    base.join("fuzz-findings")
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
fn check_in_child(
    exe: &Path,
    text: &str,
    sub: Option<&str>,
    scratch: &Path,
    timeout_ms: u64,
) -> String {
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
        ])
        .output()
        .expect("spawn check");
    let stdout = String::from_utf8_lossy(&out.stdout);
    if let Some(line) = stdout.lines().find(|l| l.starts_with("signature: ")) {
        return line["signature: ".len()..].to_string();
    }
    if String::from_utf8_lossy(&out.stderr).contains("stack overflow") {
        "crash stack overflow".into()
    } else {
        format!("crash {}", out.status)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rng_seed = parse_seed(&args);
    let timeout = Duration::from_millis(arg(&args, "--timeout-ms").unwrap_or(5000));
    let out_dir: PathBuf = arg::<String>(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(default_out);

    if let Some(file) = arg::<String>(&args, "--diagnostics") {
        // Compile in-process (no isolation) and print the diagnostics.
        let case = load_case_file(Path::new(&file));
        let documents: Vec<flashtex_compiler::parser::SourceDocument<'_>> = case
            .documents
            .iter()
            .map(|(path, text)| flashtex_compiler::parser::SourceDocument { path, text })
            .collect();
        let started = std::time::Instant::now();
        let out = flashtex_compiler::incremental::compile_full_project(
            &documents,
            ENTRY,
            flashtex_compiler::layout::LayoutConstraints::default(),
        );
        println!(
            "{} ms, {} pages, {} diagnostics",
            started.elapsed().as_millis(),
            out.pages.len(),
            out.diagnostics.len()
        );
        if args.iter().any(|a| a == "--digest") {
            // `--diagnostics FILE --digest`: hashes of the pages and
            // diagnostics, to diff two builds on one input.
            use std::hash::{Hash, Hasher};
            let digest = |value: String| {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                value.hash(&mut hasher);
                hasher.finish()
            };
            println!(
                "pages {:016x} diagnostics {:016x}",
                digest(format!("{:?}", out.pages)),
                digest(format!("{:?}", out.diagnostics))
            );
        }
        for d in out
            .diagnostics
            .iter()
            .take(arg(&args, "--limit").unwrap_or(12))
        {
            println!("  {}", d.message);
        }
        // The most frequent messages, so a flood shows its source.
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for d in &out.diagnostics {
            *counts.entry(d.message.as_str()).or_default() += 1;
        }
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        println!("most frequent:");
        for (message, count) in counts.iter().take(8) {
            let short: String = message.chars().take(100).collect();
            println!("  {count:>8}x {short}");
        }
        return;
    }

    if let Some(file) = arg::<String>(&args, "--check") {
        install_quiet_panic_hook();
        let outcome = run_case(load_case_file(Path::new(&file)), timeout);
        println!("signature: {}", outcome.signature());
        if let Outcome::Panic(loc, msg) = &outcome {
            println!("location: {loc}\nmessage: {msg}");
        }
        std::process::exit(if outcome == Outcome::Hang { 3 } else { 0 });
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
        println!(
            "{tries} probes; {} -> {} bytes: {}",
            case.main().len(),
            min.len(),
            dest.display()
        );
        if min.len() < 400 {
            println!("{min:?}");
        }
        return;
    }

    let seeds = load_seeds(&repo_root());
    if args.iter().any(|a| a == "--digest") {
        // Compile every seed unmutated and print a digest of its pages and
        // diagnostics, to diff two builds on normal documents.
        use std::hash::{Hash, Hasher};
        let digest = |value: String| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        };
        for seed in &seeds {
            let documents = [flashtex_compiler::parser::SourceDocument {
                path: ENTRY,
                text: &seed.text,
            }];
            let out = flashtex_compiler::incremental::compile_full_project(
                &documents,
                ENTRY,
                flashtex_compiler::layout::LayoutConstraints::default(),
            );
            println!(
                "{}\t{} pages {:016x}\t{} diagnostics {:016x}",
                seed.name,
                out.pages.len(),
                digest(format!("{:?}", out.pages)),
                out.diagnostics.len(),
                digest(format!("{:?}", out.diagnostics))
            );
        }
        return;
    }
    assert!(
        !seeds.is_empty(),
        "no .tex seeds found under {}",
        repo_root().display()
    );

    if let Some(index) = arg::<u64>(&args, "--replay") {
        let case = generate(&seeds, rng_seed, index);
        std::fs::create_dir_all(&out_dir).unwrap();
        let path = out_dir.join(format!("case-{index}.tex"));
        std::fs::write(&path, case.main()).unwrap();
        if let Some((_, sub)) = case.documents.get(1) {
            std::fs::write(path.with_extension("sub.tex"), sub).unwrap();
        }
        println!(
            "case {index}: seed {} mutations {:?} -> {}",
            case.seed_name,
            case.mutations,
            path.display()
        );
        return;
    }

    if let Some(start) = args.iter().position(|a| a == "--worker") {
        let begin: u64 = args[start + 1].parse().unwrap();
        let end: u64 = args[start + 2].parse().unwrap();
        worker(&seeds, rng_seed, begin, end, timeout);
        return;
    }

    let config = Config {
        rng_seed,
        cases: arg(&args, "--cases").unwrap_or(50_000),
        jobs: arg(&args, "--jobs").unwrap_or(8),
        timeout,
        out_dir,
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
    let command = |start: u64, end: u64| {
        let mut c = Command::new(&exe);
        c.args([
            "--worker",
            &start.to_string(),
            &end.to_string(),
            "--seed",
            &seed,
            "--timeout-ms",
            &ms,
        ]);
        c
    };
    let report = supervise(&seeds, &config, &command);
    print_report(&report);
}
