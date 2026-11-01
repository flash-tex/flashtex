//! Kernel feasibility probe: expand the real LaTeX kernel `latex.ltx`
//! (read in place, never vendored) from an INITEX-like state and report
//! how far the engine gets and what is missing.
//!
//! `cargo run --release --example kernel_probe -- [--primitives <file>] [--first N] [<path/to/latex.ltx>]`
//!
//! Without a path, `kpsewhich latex.ltx` locates it. `--primitives` is a
//! list of TeX-engine primitive names (one per line, e.g. dumped from
//! LuaTeX's `tex.primitives()`), used to tell unmodelled primitives apart
//! from macros that were simply never defined.
//!
//! Signals of failure, since this crate passes unknown control sequences
//! through instead of erroring:
//! - an unmodelled primitive reaching the output (`\immediate`, `\font`, ...);
//! - an undefined non-primitive control sequence reaching the output;
//! - character tokens reaching the output (INITEX loading `latex.ltx` emits
//!   essentially no text, so these are fallout of a misparse, e.g. the
//!   `=200` after an unmodelled `\tolerance`);
//! - error diagnostics.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::unnecessary_sort_by, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]
use std::collections::{BTreeMap, HashSet};
use std::time::Instant;

use flashtex_tex_expansion::{Engine, Limits, Severity, TokenKind};

fn location(engine: &Engine, source_id: u32, line: usize) -> String {
    if source_id == 0 {
        return format!("latex.ltx:{line}");
    }
    match engine.opened_files().iter().find(|(id, _)| *id == source_id) {
        Some((_, name)) => name.clone(),
        None => format!("source#{source_id}"),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut primitives: HashSet<String> = HashSet::new();
    let mut first_n = 40usize;
    let mut max_lines = usize::MAX;
    let mut trace = false;
    let mut path = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--primitives" => {
                primitives = std::fs::read_to_string(args.next().unwrap()).unwrap().lines().map(|l| l.trim().to_string()).collect()
            }
            "--first" => first_n = args.next().unwrap().parse().unwrap(),
            "--lines" => max_lines = args.next().unwrap().parse().unwrap(),
            "--trace" => trace = true,
            _ => path = Some(a),
        }
    }
    let path = path.unwrap_or_else(|| {
        let out = std::process::Command::new("kpsewhich").arg("latex.ltx").output().expect("kpsewhich");
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    });
    let bytes = std::fs::read(&path).expect("read latex.ltx");
    let full = String::from_utf8_lossy(&bytes).into_owned();
    // --lines N: only the first N lines (for bisecting a failure).
    let src: String = full.split_inclusive('\n').take(max_lines).collect();
    let line_starts: Vec<usize> = std::iter::once(0).chain(src.match_indices('\n').map(|(i, _)| i + 1)).collect();
    let line_of = |b: usize| line_starts.partition_point(|&s| s <= b);
    let total_lines = line_starts.len();

    let limits = Limits { max_expansion_steps: 20_000_000, max_output_tokens: 5_000_000, ..Limits::default() };
    let mut engine = Engine::new_initex(&src, limits);
    // `\input` resolves through kpathsea, like a real format build.
    engine.set_file_reader(std::rc::Rc::new(|name: &str| {
        let out = std::process::Command::new("kpsewhich").arg(name).output().ok()?;
        let path = String::from_utf8(out.stdout).ok()?.trim().to_string();
        if path.is_empty() {
            return None;
        }
        std::fs::read(&path).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
    }));
    let t = Instant::now();
    let mut prim: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // name -> (count, first line)
    let mut undef: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut events: Vec<(String, String)> = Vec::new();
    let mut chars = 0usize;
    let mut char_lines: BTreeMap<usize, usize> = BTreeMap::new();
    let mut max_pos = 0usize;
    let mut out_tokens = 0usize;
    let mut seen_diags = 0usize;
    while let Some(tok) = engine.next_content_token() {
        out_tokens += 1;
        if trace {
            let l = if tok.span.source_id == 0 { line_of(tok.span.start as usize) } else { 0 };
            println!("TOKEN line {l}: {:?}", tok.kind);
            for d in &engine.diagnostics()[seen_diags..] {
                println!("DIAG {}", d.message.replace('\n', " / "));
            }
        }
        let line = if tok.span.source_id == 0 {
            max_pos = max_pos.max(tok.span.end as usize);
            line_of(tok.span.start as usize)
        } else {
            0
        };
        match &tok.kind {
            // Executed no-ops and group boundaries are expected output.
            TokenKind::ControlSequence(n) if matches!(n.as_str(), "relax" | "par" | "begingroup" | "endgroup") => {}
            TokenKind::Char(_, flashtex_tex_expansion::CatCode::BeginGroup | flashtex_tex_expansion::CatCode::EndGroup) => {}
            TokenKind::ControlSequence(n) => {
                let map = if primitives.contains(n) { &mut prim } else { &mut undef };
                let e = map.entry(n.clone()).or_insert((0, line));
                if e.0 == 0 && events.len() < first_n {
                    let kind = if primitives.contains(n) { "unmodelled primitive" } else { "undefined control sequence" };
                    events.push((location(&engine, tok.span.source_id, line), format!("{kind} \\{n}")));
                }
                e.0 += 1;
            }
            TokenKind::Char(c, _) if c.is_whitespace() => {}
            TokenKind::Char(..) | TokenKind::ActiveChar(_) => {
                chars += 1;
                *char_lines.entry(line).or_default() += 1;
            }
            _ => {}
        }
        let diags = engine.diagnostics();
        while seen_diags < diags.len() {
            let d = &diags[seen_diags];
            if d.severity == Severity::Error && events.len() < first_n {
                let l = if d.span.source_id == 0 && !d.span.is_synthetic() { line_of(d.span.start as usize) } else { line };
                let loc = location(&engine, d.span.source_id.min(tok.span.source_id), l);
                events.push((loc, format!("error: {}", d.message.lines().last().unwrap_or_default())));
            }
            seen_diags += 1;
        }
    }
    let elapsed = t.elapsed();
    let diags = engine.take_diagnostics();
    let reached = line_of(max_pos);
    println!("latex.ltx: {path}");
    println!("{} bytes, {total_lines} lines", src.len());
    println!(
        "expansion stopped after {:.1} ms, {} engine steps, {out_tokens} output tokens; furthest source position line {reached} ({:.1}%)",
        elapsed.as_secs_f64() * 1e3,
        engine.steps(),
        100.0 * reached as f64 / total_lines as f64
    );
    let errors: Vec<_> = diags.iter().filter(|d| d.severity == Severity::Error).collect();
    println!("{} error diagnostics, {chars} stray character tokens on {} lines", errors.len(), char_lines.len());
    println!("\nFirst {first_n} failure events in source order of discovery:");
    for (l, e) in &events {
        println!("  {l:>22}: {e}");
    }
    println!("\nFiles read via \\input: {:?}", engine.opened_files().iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>());
    let print_top = |m: &BTreeMap<String, (usize, usize)>, label: &str| {
        let mut v: Vec<_> = m.iter().collect();
        v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
        println!("\n{label}: {} distinct", v.len());
        for (n, (c, l)) in v.iter().take(60) {
            println!("  \\{n:<28} x{c:<6} first line {l}");
        }
    };
    print_top(&prim, "Unmodelled primitives reaching output");
    print_top(&undef, "Undefined non-primitive control sequences reaching output");
    let mut by_msg: BTreeMap<String, usize> = BTreeMap::new();
    for d in &errors {
        *by_msg.entry(d.message.lines().last().unwrap_or_default().chars().take(80).collect()).or_default() += 1;
    }
    let mut v: Vec<_> = by_msg.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    println!("\nError diagnostics by message:");
    for (m, c) in v.iter().take(30) {
        println!("  {c:>6}  {m}");
    }
}
