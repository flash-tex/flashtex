//! `\addvspace` and `\addpenalty` follow latex.ltx's `\lastskip` rules.
//!
//! Every expected value was measured with pdflatex (TeX Live 2026, article,
//! `\parindent0pt`): the baseline gap between the two lines around the
//! vertical material, minus the 12pt `\baselineskip`.

use flashtex_compiler::parser::{parse, Block};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\parindent0pt\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// The blocks between the first paragraph and the last one, as a compact
/// trace: `v<pt>` for vertical space, `p<value>` for a penalty.
fn vertical_trace(body: &str) -> Vec<String> {
    let parsed = parse(&doc(body));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let first = parsed
        .blocks
        .iter()
        .position(|b| matches!(b, Block::Paragraph(_)))
        .expect("a first paragraph");
    let last = parsed
        .blocks
        .iter()
        .rposition(|b| matches!(b, Block::Paragraph(_)))
        .expect("a last paragraph");
    parsed.blocks[first + 1..last]
        .iter()
        .map(|b| match b {
            Block::VSpace { pt, .. } => format!("v{pt}"),
            Block::Penalty { value, .. } => format!("p{value}"),
            other => panic!("unexpected block {other:?}"),
        })
        .collect()
}

fn total_space(body: &str) -> f64 {
    vertical_trace(body)
        .iter()
        .filter_map(|s| s.strip_prefix('v').map(|v| v.parse::<f64>().unwrap()))
        .sum()
}

#[test]
fn addvspace_after_vspace_or_bigskip_adds_in_full() {
    // `\vspace`/`\bigskip` end with `\vskip\z@skip`: `\lastskip` is zero.
    assert_eq!(total_space("A\\par\n\\vspace{20pt}\\addvspace{10pt}\nB"), 30.0);
    assert_eq!(total_space("A\\par\n\\bigskip\\addvspace{5pt}\nB"), 17.0);
    assert_eq!(total_space("A\\par\n\\addvspace{9pt}\\vspace{3pt}\\addvspace{2pt}\nB"), 14.0);
}

#[test]
fn consecutive_addvspace_keeps_the_larger() {
    assert_eq!(total_space("A\\par\n\\addvspace{10pt}\\addvspace{20pt}\nB"), 20.0);
    assert_eq!(total_space("A\\par\n\\addvspace{20pt}\\addvspace{10pt}\nB"), 20.0);
    assert_eq!(vertical_trace("A\\par\n\\addvspace{20pt}\\addvspace{10pt}\nB"), ["v20"]);
}

#[test]
fn addvspace_after_a_raw_vskip_takes_the_maximum() {
    assert_eq!(total_space("A\\par\n\\vskip 8pt\\relax\\addvspace{10pt}\nB"), 10.0);
    // `\vskip 0pt` leaves `\lastskip` zero again, so the 4pt is added.
    assert_eq!(total_space("A\\par\n\\vskip 5pt\\vskip 0pt\\relax\\addvspace{4pt}\nB"), 9.0);
}

#[test]
fn negative_addvspace_reduces_pending_space() {
    assert_eq!(total_space("A\\par\n\\addvspace{10pt}\\addvspace{-4pt}\nB"), 6.0);
}

#[test]
fn addvspace_in_a_paragraph_ends_it_first() {
    // latex.ltx 2026: `\ifhmode \ifinner \@LRmoderr \else \par \fi \fi`.
    let parsed = parse(&doc("Ii Jj\\addvspace{7pt}Kk"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(total_space("Ii Jj\\addvspace{7pt}Kk"), 7.0);
}

#[test]
fn addvspace_reads_registers_and_glue() {
    // The argument is evaluated by the engine: registers, factors, rubber.
    assert_eq!(total_space("A\\par\n\\addvspace{0.5\\baselineskip}\nB"), 6.0);
    let parsed = parse(&doc("A\\par\n\\addvspace{1em plus 1pt}\nB"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let glue = parsed.blocks.iter().find_map(|b| match b {
        Block::VSpace { pt, stretch_pt, .. } => Some((*pt, *stretch_pt)),
        _ => None,
    });
    // cmr10's quad is 10.00002pt, as in TeX.
    let (pt, stretch) = glue.expect("one VSpace");
    assert!((pt - 10.00002).abs() < 1e-4 && stretch == 1.0, "{pt} plus {stretch}");
}

#[test]
fn addpenalty_goes_before_pending_glue() {
    // `\addvspace{6pt}\addpenalty{-300}\addvspace{4pt}`: 6pt in pdflatex,
    // with the penalty ahead of the glue.
    assert_eq!(
        vertical_trace("A\\par\n\\addvspace{6pt}\\addpenalty{-300}\\addvspace{4pt}\nB"),
        ["p-300", "v6"]
    );
    // With no pending glue it is a plain penalty.
    assert_eq!(vertical_trace("A\\par\n\\addpenalty{-300}\\addvspace{4pt}\nB"), ["p-300", "v4"]);
}
