//! The mutation fuzzer as an ignored test (debug build, so overflow checks
//! and debug assertions are on and stack frames are at their largest).
//!
//! ```text
//! FLASHTEX_FUZZ_CASES=2000 cargo test --test fuzz_compile -- --ignored --nocapture
//! ```
//!
//! The test re-executes its own binary as the worker process (see
//! `fuzz_support`). For long runs use the example:
//! `cargo run --release --example fuzz_compile`.

mod fuzz_support;

use std::process::Command;
use std::time::Duration;

use fuzz_support::*;

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[test]
#[ignore = "fuzzing run; minutes long"]
fn fuzz_compile_mutations() {
    if let Ok(range) = std::env::var("FLASHTEX_FUZZ_WORKER") {
        // Worker process: run a range and report on stdout.
        let (start, end) = range.split_once("..").expect("start..end");
        let seeds = load_seeds(&repo_root());
        worker(
            &seeds,
            env_u64("FLASHTEX_FUZZ_SEED", 0xF1A5_7E40),
            start.parse().unwrap(),
            end.parse().unwrap(),
            Duration::from_millis(env_u64("FLASHTEX_FUZZ_TIMEOUT_MS", 5000)),
        );
        return;
    }
    let seeds = load_seeds(&repo_root());
    assert!(!seeds.is_empty());
    let config = Config {
        rng_seed: env_u64("FLASHTEX_FUZZ_SEED", 0xF1A5_7E40),
        cases: env_u64("FLASHTEX_FUZZ_CASES", 2000),
        jobs: env_u64("FLASHTEX_FUZZ_JOBS", 4) as usize,
        timeout: Duration::from_millis(env_u64("FLASHTEX_FUZZ_TIMEOUT_MS", 20_000)),
        out_dir: std::env::temp_dir().join("flashtex-fuzz-findings"),
    };
    let exe = std::env::current_exe().unwrap();
    let command = |start: u64, end: u64| {
        let mut c = Command::new(&exe);
        c.args(["fuzz_compile_mutations", "--exact", "--ignored", "--nocapture", "--test-threads=1"])
            .env("FLASHTEX_FUZZ_WORKER", format!("{start}..{end}"))
            .env("FLASHTEX_FUZZ_SEED", config.rng_seed.to_string())
            .env("FLASHTEX_FUZZ_TIMEOUT_MS", config.timeout.as_millis().to_string());
        c
    };
    let report = supervise(&seeds, &config, &command);
    print_report(&report);
    assert!(report.findings.is_empty(), "the fuzzer found {} unique failures", report.findings.len());
}
