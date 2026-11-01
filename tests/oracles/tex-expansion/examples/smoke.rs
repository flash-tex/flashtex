//! Real-document smoke test: expand every `.tex` file under the given
//! directories and classify each as clean or unsupported.
//!
//! `cargo run --release --example smoke -- [--primitives <file>] <dir>...`
//!
//! - **clean**: no error diagnostics and no resource-limit stop.
//! - **unsupported**: anything else; the first error is shown.
//!
//! Control sequences reaching the output are *not* errors (they are the
//! typesetter's business, see CONTRACT.md), but they are tallied: names
//! that are TeX/e-TeX/pdfTeX primitives (from `--primitives`, one name per
//! line, e.g. dumped from LuaTeX's `tex.primitives()`) are primitives this
//! crate does not model; the rest are macros no loaded code defined
//! (class/package commands).

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::unnecessary_sort_by, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use flashtex_tex_expansion::{Engine, Limits, Severity, TokenKind};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "tex") {
            out.push(p);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let mut primitives: HashSet<String> = HashSet::new();
    let mut roots = Vec::new();
    while let Some(a) = args.next() {
        if a == "--primitives" {
            let f = args.next().expect("--primitives <file>");
            primitives = std::fs::read_to_string(f).unwrap().lines().map(|l| l.trim().to_string()).collect();
        } else {
            roots.push(PathBuf::from(a));
        }
    }
    let mut files = Vec::new();
    for r in &roots {
        collect(r, &mut files);
    }
    let mut prim_freq: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // name -> (docs, occurrences)
    let mut macro_freq: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut err_freq: BTreeMap<String, usize> = BTreeMap::new();
    let (mut clean, mut unsupported) = (0, 0);
    for f in &files {
        let bytes = std::fs::read(f).unwrap();
        let src = String::from_utf8_lossy(&bytes);
        let limits = Limits { max_expansion_steps: 5_000_000, ..Limits::default() };
        let t = Instant::now();
        let mut e = Engine::with_limits(&src, limits);
        let toks = e.run();
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let diags = e.take_diagnostics();
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == Severity::Error).collect();
        let mut seen_p = HashSet::new();
        let mut seen_m = HashSet::new();
        for t in &toks {
            if let TokenKind::ControlSequence(n) = &t.kind {
                if matches!(n.as_str(), "relax" | "par" | "document" | "enddocument" | "begingroup" | "endgroup") {
                    continue;
                }
                let (map, seen) = if primitives.contains(n) { (&mut prim_freq, &mut seen_p) } else { (&mut macro_freq, &mut seen_m) };
                let entry = map.entry(n.clone()).or_default();
                entry.1 += 1;
                if seen.insert(n.clone()) {
                    entry.0 += 1;
                }
            }
        }
        for d in &errors {
            let key: String = d.message.lines().last().unwrap_or_default().chars().take(70).collect();
            *err_freq.entry(key).or_default() += 1;
        }
        let rel = f.display().to_string();
        if errors.is_empty() {
            clean += 1;
            println!("CLEAN       {rel} ({} tokens, {ms:.1} ms)", toks.len());
        } else {
            unsupported += 1;
            let first = errors[0].message.lines().last().unwrap_or_default();
            println!("UNSUPPORTED {rel} ({} errors, {ms:.1} ms): {first}", errors.len());
        }
    }
    println!("\n{} files: {clean} clean, {unsupported} unsupported", files.len());
    let top = |m: &BTreeMap<String, (usize, usize)>, label: &str| {
        let mut v: Vec<_> = m.iter().collect();
        v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(b.1 .1.cmp(&a.1 .1)));
        println!("\nTop {label} (docs, occurrences):");
        for (n, (d, o)) in v.iter().take(25) {
            println!("  \\{n:<24} {d:>3} {o:>6}");
        }
    };
    if !primitives.is_empty() {
        top(&prim_freq, "unmodelled TeX/e-TeX/pdfTeX primitives passed through");
    }
    top(&macro_freq, "undefined (class/package) control sequences passed through");
    let mut ev: Vec<_> = err_freq.into_iter().collect();
    ev.sort_by(|a, b| b.1.cmp(&a.1));
    println!("\nError messages by frequency:");
    for (m, c) in ev.iter().take(15) {
        println!("  {c:>4}  {m}");
    }
}
