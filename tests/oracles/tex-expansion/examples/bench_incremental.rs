//! Keystroke re-expansion latency on a ~500 KB synthetic LaTeX document.
//!
//! `cargo run --release --example bench_incremental [-- <bytes> <edits>]`
//!
//! Builds a document with a macro/counter preamble and many paragraphs,
//! expands it once, then simulates typing: single-character insertions
//! (and a few deletions) at random body positions, timing each
//! `IncrementalExpander::edit` call. Prints full-expansion time and
//! p50/p95/max per-keystroke latency.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(clippy::manual_is_multiple_of, reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored")]

use std::time::Instant;

use flashtex_tex_expansion::{expand_str, Edit, IncrementalExpander, Limits};

fn synthetic_document(target_bytes: usize) -> String {
    let mut doc = String::from(
        "\\documentclass{article}\n\\newcounter{para}\n\\newcommand{\\emphx}[1]{<#1>}\n\
         \\newcommand{\\note}[2][n]{(#1: #2)}\n\\def\\sep{ -- }\n\\begin{document}\n",
    );
    let mut i = 0usize;
    while doc.len() < target_bytes {
        if i % 40 == 0 {
            doc.push_str(&format!("\\section{{Section {i}}}\n\n"));
        }
        doc.push_str(&format!(
            "\\stepcounter{{para}}Paragraph \\arabic{{para}} talks about \\emphx{{topic {i}}}\\sep and more \
             text so that lines have a realistic length. {{\\def\\sep{{ / }}Grouped\\sep text}} \\note{{remark {i}}} \
             \\ifnum\\value{{para}}>3 many\\else few\\fi. Lorem ipsum dolor sit amet, consectetur adipiscing.\n\n"
        ));
        i += 1;
    }
    doc.push_str("\\end{document}\n");
    doc
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bytes: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(500_000);
    let edits: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300);
    let doc = synthetic_document(bytes);
    // The default (IDE) limits: a keystroke can create an infinite macro
    // loop, which then costs at most the step limit.
    let limits = Limits::default();

    let t = Instant::now();
    let full = expand_str(&doc);
    let full_ms = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let mut inc = IncrementalExpander::with_options(&doc, limits, 2048);
    let initial_ms = t.elapsed().as_secs_f64() * 1e3;

    let body_start = doc.find("\\begin{document}").unwrap() + 20;
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    let mut rand = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let mut times = Vec::with_capacity(edits);
    let mut converged = 0usize;
    for k in 0..edits {
        let src = inc.source();
        let mut pos = body_start + (rand() as usize) % (src.len() - body_start - 20);
        while !src.is_char_boundary(pos) {
            pos -= 1;
        }
        let edit = if k % 10 == 9 && pos + 1 < src.len() && src.is_char_boundary(pos + 1) {
            Edit { start: pos, end: pos + 1, replacement: String::new() }
        } else {
            Edit { start: pos, end: pos, replacement: "x".into() }
        };
        let removed = inc.source()[edit.start..edit.end].to_string();
        // Keystroke, then its undo (typing + backspace): both are timed,
        // and the document cannot drift into a permanently broken state
        // (a deleted `}` can turn `\def\sep{ / }` into an infinite loop).
        let undo = Edit { start: edit.start, end: edit.start + edit.replacement.len(), replacement: removed.clone() };
        for (label, e) in [("keystroke", &edit), ("undo", &undo)] {
            let t = Instant::now();
            let stats = inc.edit(e);
            let ms = t.elapsed().as_secs_f64() * 1e3;
            times.push(ms);
            if ms > 50.0 && std::env::var_os("BENCH_VERBOSE").is_some() {
                eprintln!("slow {label} #{k}: {ms:.1} ms, removed {removed:?}, inserted {:?}, {stats:?}", edit.replacement);
            }
            if stats.converged_at.is_some() {
                converged += 1;
            }
        }
    }
    // Verify the final state against a from-scratch expansion.
    let check = expand_str(inc.source());
    let equal = check.tokens == inc.tokens();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("document: {} bytes, {} output tokens, {} diagnostics", doc.len(), full.tokens.len(), full.diagnostics.len());
    println!("full expansion (expand_str): {full_ms:.1} ms");
    println!("initial incremental run (with {} checkpoints): {initial_ms:.1} ms", inc.checkpoint_count());
    let edits = times.len();
    println!(
        "keystroke re-expansion over {edits} edits (keystroke+undo pairs): p50 {:.3} ms, p95 {:.3} ms, max {:.3} ms; converged early {converged}/{edits}",
        percentile(&times, 0.5),
        percentile(&times, 0.95),
        times.last().unwrap()
    );
    println!("final incremental tokens == full re-expansion: {equal}");
}
