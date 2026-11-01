//! Oracle conformance tests: compares this crate's expansion engine
//! against real TeX/e-TeX/pdfLaTeX output, captured ahead of time by
//! `tests/oracle/gen/cases.py` (which shells out to the actual `tex`,
//! `etex`, and `pdflatex` binaries -- oracle-only, per KC-101; this test
//! binary never invokes TeX itself, it only reads the committed
//! `tests/oracle/manifest.json` fixture).
//!
//! To regenerate the fixture after adding cases, run (with MacTeX/TeX Live
//! on PATH):
//!   python3 tests/oracle/gen/cases.py
//!
//! Comparison depends on the capture mode (see cases.py):
//! - `tex`/`etex`/`latex-write`: `\immediate\write` text vs. our display
//!   string (control sequences printed `\name `; executed `\relax`
//!   tokens dropped).
//! - `latex-doc`: setup in the preamble, `\begin{document}`, then a write.
//! - `latex-render`: pdftotext of typeset body text vs. our content
//!   characters (non-printing `\relax`/`\document` markers dropped,
//!   whitespace runs collapsed as pdftotext does).
//! - `*-err`: the `! ...` error lines of the TeX log vs. the last line of
//!   each of our error diagnostics (line numbers normalised).

use flashtex_tex_expansion::{expand_str, is_group_token, tokens_to_display_string, Severity, Token, TokenKind};
use serde_json::Value;
use std::fs;

fn manifest() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/oracle/manifest.json");
    let text = fs::read_to_string(path).expect("tests/oracle/manifest.json missing -- run tests/oracle/gen/cases.py");
    serde_json::from_str(&text).expect("manifest.json is not valid JSON")
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalise_err(line: &str) -> String {
    let line = line.trim_start_matches("! ").trim_end();
    match line.find("after line ") {
        Some(i) => format!("{}after line N.", &line[..i]),
        None => line.to_string(),
    }
}

/// The engine input for a case, mirroring the file cases.py hands to TeX:
/// in write modes a non-expandable token (`\immediate` there, `\relax`
/// here) separates setup from the written text, so a keyword/number scan
/// at the end of setup cannot run into `expr`; render mode has a newline
/// there; err mode has nothing.
pub fn case_source(mode: &str, setup: &str, expr: &str) -> String {
    match mode {
        "latex-doc" => format!("{setup}\\begin{{document}}\\relax {expr}"),
        "latex-render" => format!("\\begin{{document}}{setup}\n{expr}"),
        "latex-err" => format!("\\begin{{document}}{setup}{expr}"),
        "latex-write" => format!("\\begin{{document}}{setup}\\relax {expr}"),
        m if m.ends_with("-err") => format!("{setup}{expr}"),
        _ => format!("{setup}\\relax {expr}"),
    }
}

/// Returns (expected, actual) for one case in comparable form.
fn evaluate(case: &Value) -> (String, String, String) {
    let setup = case["setup"].as_str().unwrap_or_default();
    let expr = case["expr"].as_str().unwrap_or_default();
    let expected = case["expected"].as_str().unwrap_or_default();
    let mode = case["mode"].as_str().unwrap_or("tex");
    let source = case_source(mode, setup, expr);
    let result = expand_str(&source);
    // `\relax` tokens executed by main control (setup's `\setlength`,
    // `\loop`, the separator above) are never written/typeset.
    let visible: Vec<Token> = result
        .tokens
        .iter()
        .filter(|t| !matches!(&t.kind, TokenKind::ControlSequence(n) if n == "document" || n == "relax"))
        // Grouping tokens are emitted for the typesetter; TeX neither
        // writes executed braces nor typesets them.
        .filter(|t| !is_group_token(t))
        .cloned()
        .collect();
    if mode.ends_with("-err") {
        let actual: Vec<String> = result
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| normalise_err(d.message.lines().last().unwrap_or_default()))
            .collect();
        let expected: Vec<String> = expected.lines().filter(|l| !l.is_empty()).map(normalise_err).collect();
        return (source, expected.join("\n"), actual.join("\n"));
    }
    if mode == "latex-render" {
        let text: Vec<Token> =
            visible.into_iter().filter(|t| !matches!(&t.kind, TokenKind::ControlSequence(n) if n == "par")).collect();
        return (source, collapse_ws(expected), collapse_ws(&tokens_to_display_string(&text)));
    }
    (source, expected.to_string(), tokens_to_display_string(&visible))
}

#[test]
fn oracle_corpus_matches_real_tex() {
    let manifest = manifest();
    let cases = manifest.as_object().expect("manifest.json root must be an object");
    assert!(cases.len() >= 250, "expected a substantial oracle corpus, found {}", cases.len());

    let mut failures = Vec::new();
    for (id, case) in cases {
        let (source, expected, actual) = evaluate(case);
        if actual != expected {
            let diags = expand_str(&source).diagnostics;
            failures.push(format!(
                "{id} [{}]: source={source:?}\n    expected: {expected:?}\n    actual:   {actual:?}\n    diagnostics: {:?}",
                case["mode"].as_str().unwrap_or("tex"),
                diags.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
            ));
        }
    }
    if !failures.is_empty() {
        panic!("{} / {} oracle cases mismatched:\n{}", failures.len(), cases.len(), failures.join("\n"));
    }
}

/// Prints a one-line pass/fail count without panicking on individual
/// mismatches, useful while iterating on the engine (`cargo test
/// oracle_summary -- --nocapture`).
#[test]
fn oracle_summary() {
    let manifest = manifest();
    let cases = manifest.as_object().unwrap();
    let mut pass = 0;
    let mut failed = Vec::new();
    for (id, case) in cases {
        let (_, expected, actual) = evaluate(case);
        if actual == expected {
            pass += 1;
        } else {
            failed.push(id.as_str());
        }
    }
    println!("oracle: {pass} passed, {} failed, {} total; failed: {failed:?}", failed.len(), cases.len());
}
