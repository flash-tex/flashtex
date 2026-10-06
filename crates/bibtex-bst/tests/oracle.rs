//! Compares the engine with committed TeX Live `bibtex` output
//! (tests/data/expected, produced by tests/oracle/generate.py).
//! This test never runs `bibtex`.
//!
//! * `.bbl`: byte-identical.
//! * `.blg`: byte-identical is reported; the pass criterion is equality after
//!   removing the banner line, the `Capacity:` line and the usage-statistics
//!   block (`You've used ...` through the 37 call counts), which describe the
//!   C implementation rather than the bibliography.
//!
//! `ORACLE_CASE=<substring>` filters cases, `ORACLE_VERBOSE=1` prints diffs.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::unnecessary_map_or, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

use std::path::{Path, PathBuf};

use flashtex_bibtex_bst::{run, DirSource, Options};

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

/// Drops the banner, `Capacity:` and statistics lines.
fn normalize_blg(blg: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut stats = 0usize;
    for (i, line) in blg.split(|&c| c == b'\n').enumerate() {
        if i == 0 && line.starts_with(b"This is BibTeX") {
            continue;
        }
        if line.starts_with(b"Capacity: ") {
            continue;
        }
        if line.starts_with(b"You've used ") {
            stats = 41;
        }
        if stats > 0 {
            stats -= 1;
            continue;
        }
        out.push(line.to_vec());
    }
    out
}

fn first_diff(a: &[u8], b: &[u8]) -> String {
    let al: Vec<&[u8]> = a.split(|&c| c == b'\n').collect();
    let bl: Vec<&[u8]> = b.split(|&c| c == b'\n').collect();
    for i in 0..al.len().max(bl.len()) {
        let x = al.get(i).copied().unwrap_or(b"<EOF>");
        let y = bl.get(i).copied().unwrap_or(b"<EOF>");
        if x != y {
            return format!(
                "line {}:\n    engine: {}\n    bibtex: {}",
                i + 1,
                String::from_utf8_lossy(x),
                String::from_utf8_lossy(y)
            );
        }
    }
    "identical".into()
}

#[test]
fn oracle_cases() {
    let data = data();
    let filter = std::env::var("ORACLE_CASE").ok();
    let verbose = std::env::var("ORACLE_VERBOSE").is_ok();
    let mut names: Vec<String> = std::fs::read_dir(data.join("cases"))
        .expect("tests/data/cases missing: run tests/oracle/generate.py")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| filter.as_ref().map_or(true, |f| n.contains(f.as_str())))
        .collect();
    names.sort();
    let (mut bbl_ok, mut blg_norm_ok, mut blg_exact_ok) = (0, 0, 0);
    let mut failures = Vec::new();
    for name in &names {
        let dir = data.join("cases").join(name);
        let src = DirSource {
            aux_dirs: vec![dir.clone()],
            bst_dirs: vec![dir.clone(), data.join("bst")],
            bib_dirs: vec![dir.clone(), data.join("bib")],
        };
        let out = run("job", &src, &Options::default()).expect("job.aux");
        let exp_bbl = std::fs::read(data.join("expected").join(format!("{name}.bbl"))).unwrap();
        let exp_blg = std::fs::read(data.join("expected").join(format!("{name}.blg"))).unwrap();
        let b = out.bbl == exp_bbl;
        let n = normalize_blg(&out.blg) == normalize_blg(&exp_blg);
        let x = out.blg == exp_blg;
        bbl_ok += b as usize;
        blg_norm_ok += n as usize;
        blg_exact_ok += x as usize;
        if !b || !n {
            failures.push(name.clone());
        }
        if verbose && (!b || !n || !x) {
            println!("== {name}: bbl {b}, blg(normalized) {n}, blg(exact) {x}");
            if !b {
                println!("  bbl {}", first_diff(&out.bbl, &exp_bbl));
            }
            if !x {
                println!("  blg {}", first_diff(&out.blg, &exp_blg));
            }
        }
    }
    println!(
        "oracle: {} cases; bbl byte-identical {}/{}; blg identical after normalization {}/{}; blg byte-identical {}/{}",
        names.len(),
        bbl_ok,
        names.len(),
        blg_norm_ok,
        names.len(),
        blg_exact_ok,
        names.len()
    );
    assert!(failures.is_empty(), "oracle mismatches: {failures:?}");
}
