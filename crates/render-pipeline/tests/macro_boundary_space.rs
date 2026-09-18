//! GH-919: a user macro whose body is a text command (`\newcommand{\ul}[1]
//! {\uline{#1}}`) invented an interword space on each side of its expansion
//! at a punctuation boundary. `"\ul{a b}" x "\ul{c d}" y.` set `a`/`b`
//! +3.32 bp, `x` +6.64, `c`/`d` +9.96 and `y.` +13.28 right of pdflatex,
//! while the same line with `\uline` written directly was exact.
//!
//! The compiler gives every token of a replacement text the invocation's
//! `\ul` span. `adapter::token_gap` re-reads the source between spans for
//! the interword gaps, and did two things wrong with that span: it took a
//! textless replacement token (the underline box) as spaced from whatever
//! came before it, and after the box it read from the end of `\ul` — the
//! invocation's own `{a b}` — as if those bytes sat between the box and the
//! closing quote. It now reads the body up to the box (`\uline` is its first
//! byte: no blank) and resumes after the invocation's arguments.
//!
//! ## Oracle
//!
//! Every number below is a word origin (first glyph x, bp) read by
//! `tools/visual-oracle/pdftext.py` from pdfLaTeX's output for the same
//! document: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026, MacTeX). The
//! direct and macro rows are identical there. pdflatex is an oracle only,
//! never in the product path.

mod common;

use flashtex_render_pipeline::display::Item;

/// The corpus harness's exact-route tolerance for a word origin.
const TOL: f64 = 0.01;

/// `(text, x)` of every glyph run on the given baseline (bp from the page
/// top), in order. The item's bullet is skipped: it sits 2.77 bp left of
/// pdflatex's on every item that holds an underline, direct or via the
/// macro, which is a separate defect of the label box, not of this seam.
fn words_on(text: &str, baseline: f64) -> Vec<(String, f64)> {
    let r = common::render_one(text);
    assert_eq!(r.v2.pages.len(), 1);
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        let Some(g) = run.glyphs.first() else { continue };
        if (g.baseline_y.to_bp() - baseline).abs() > 0.05 || run.text.trim() == "\u{2022}" {
            continue;
        }
        out.push((run.text.trim().to_string(), g.origin_x.to_bp()));
    }
    out
}

fn doc(items: &[&str]) -> String {
    let mut s = String::from(
        "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage[normalem]{ulem}\n\
         \\newcommand{\\ul}[1]{\\uline{#1}}\n\\begin{document}\n\\begin{itemize}\n",
    );
    for item in items {
        s.push_str("\\item ");
        s.push_str(item);
        s.push('\n');
    }
    s.push_str("\\end{itemize}\n\\end{document}\n");
    s
}

/// One boundary: the macro line, the direct line, and pdflatex's origins
/// for the letters of both (the punctuation runs are checked for equality
/// between the two lines, not pinned: pdftext folds them into neighbours).
struct Boundary {
    name: &'static str,
    via_macro: &'static str,
    direct: &'static str,
    letters: &'static [(&'static str, f64)],
}

const BOUNDARIES: [Boundary; 3] = [
    Boundary {
        name: "quotes",
        via_macro: "\"\\ul{a b}\" x \"\\ul{c d}\" y.",
        direct: "\"\\uline{a b}\" x \"\\uline{c d}\" y.",
        letters: &[("a", 163.655), ("b", 171.955), ("x", 185.786), ("c", 199.346), ("d", 207.093), ("y.", 220.924)],
    },
    Boundary {
        name: "parens",
        via_macro: "(\\ul{a b}) x (\\ul{c d}) y.",
        direct: "(\\uline{a b}) x (\\uline{c d}) y.",
        letters: &[("a", 162.548), ("b", 170.849), ("x", 183.573), ("c", 196.026), ("d", 203.773), ("y.", 216.497)],
    },
    Boundary {
        name: "em dash",
        via_macro: "p---\\ul{a b}---x q---\\ul{c d}---y.",
        direct: "p---\\uline{a b}---x q---\\uline{c d}---y.",
        letters: &[("a", 174.169), ("b", 182.469), ("c", 221.756), ("d", 229.503)],
    },
];

/// The first item's baseline in this document (pdflatex), then per item
/// `\baselineskip` + `\itemsep` + `\parsep` (12 + 4 + 4 pt = 19.925 bp).
const FIRST_BASELINE: f64 = 134.765;
const BASELINESKIP: f64 = 19.925;

#[test]
fn macro_and_direct_lines_are_identical_and_match_pdflatex() {
    if !common::lm_available() {
        return;
    }
    let items: Vec<&str> = BOUNDARIES.iter().flat_map(|b| [b.via_macro, b.direct]).collect();
    let text = doc(&items);
    for (k, b) in BOUNDARIES.iter().enumerate() {
        let expanded = words_on(&text, FIRST_BASELINE + BASELINESKIP * (2 * k) as f64);
        let plain = words_on(&text, FIRST_BASELINE + BASELINESKIP * (2 * k + 1) as f64);
        assert!(!expanded.is_empty(), "{}: no words on the macro line", b.name);
        assert_eq!(
            expanded.len(),
            plain.len(),
            "{}: the macro line has different runs\n macro: {expanded:?}\n direct: {plain:?}",
            b.name
        );
        for ((text, x), (want_text, want_x)) in expanded.iter().zip(&plain) {
            assert_eq!(text, want_text, "{}: run order differs", b.name);
            assert!((x - want_x).abs() < TOL, "{}: `{text}` at {x:.3} via the macro, {want_x:.3} directly", b.name);
        }
        for &(letter, want_x) in b.letters {
            let (_, x) = expanded
                .iter()
                .find(|(t, _)| t == letter)
                .unwrap_or_else(|| panic!("{}: no run `{letter}` on the macro line: {expanded:?}", b.name));
            assert!((x - want_x).abs() < TOL, "{}: `{letter}` at {x:.3}, pdflatex {want_x:.3}", b.name);
        }
    }
}

/// The unquoted macro line was already exact; it stays so.
#[test]
fn unquoted_macro_keeps_its_blank() {
    if !common::lm_available() {
        return;
    }
    let text = doc(&["\\ul{a b} x."]);
    let words = words_on(&text, FIRST_BASELINE);
    // pdflatex: a 158.675, b 166.975, x. 175.826.
    for (letter, want_x) in [("a", 158.675), ("b", 166.975), ("x.", 175.826)] {
        let (_, x) = words.iter().find(|(t, _)| t == letter).unwrap_or_else(|| panic!("no `{letter}`: {words:?}"));
        assert!((x - want_x).abs() < TOL, "`{letter}` at {x:.3}, pdflatex {want_x:.3}");
    }
}

// ---- GH-925: the same seams through a macro with an optional argument ----
//
// `\newcommand{\lb}[2][z]{\uline{#1#2}}` and `\newcommand{\lt}[2][z]{#1#2}`.
// An optional argument goes through `\@protected@testopt` -> `\@ifnextchar`
// -> `\futurelet`, whose lookahead dropped the invocation origin of the
// tokens it re-inserted (tex-expansion `do_futurelet`), so the compiler
// placed every replacement token at its *definition* bytes and `token_gap`
// read preamble text as the gap before `)`: `)` sat +3.32 bp right of
// pdflatex. With the origin restored, the adapter also has to count the
// `[..]` as `#1` (`macro_arg_index`) and read the default's tokens (`z`)
// where the body reaches `#1`, since they are not in the body.
//
// pdflatex (same oracle as above) sets each macro line identically to its
// direct spelling; the numbers are its word origins.

/// `(cluster text, x)` of every glyph cluster on the baseline, in order:
/// the compiler cuts a replacement text's `z` and the argument's `a` into
/// separate runs, so runs cannot be compared with the direct line, but the
/// glyphs can.
fn glyphs_on(text: &str, baseline: f64) -> Vec<(String, f64)> {
    let r = common::render_one(text);
    assert_eq!(r.v2.pages.len(), 1);
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let Item::GlyphRun(run) = item else { continue };
        let Some(g) = run.glyphs.first() else { continue };
        if (g.baseline_y.to_bp() - baseline).abs() > 0.05 || run.text.trim() == "\u{2022}" {
            continue;
        }
        for (i, c) in run.clusters.iter().enumerate() {
            let Some(g) = run.glyphs.iter().find(|g| g.cluster as usize == i) else { continue };
            out.push((run.text[c.text_start_byte..c.text_end_byte].to_string(), g.origin_x.to_bp()));
        }
    }
    out
}

fn doc_optional(items: &[&str]) -> String {
    let mut s = String::from(
        "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage[normalem]{ulem}\n\
         \\newcommand{\\lb}[2][z]{\\uline{#1#2}}\n\\newcommand{\\lt}[2][z]{#1#2}\n\
         \\begin{document}\n\\begin{itemize}\n",
    );
    for item in items {
        s.push_str("\\item ");
        s.push_str(item);
        s.push('\n');
    }
    s.push_str("\\end{itemize}\n\\end{document}\n");
    s
}

/// `letters`: pdflatex's word origins, pinned in order on the first glyph
/// of each word (pdftext folds punctuation into its neighbours).
const OPTIONAL_BOUNDARIES: [Boundary; 4] = [
    Boundary {
        name: "parens, default then given",
        via_macro: "(\\lb{a b}) x (\\lb[y]{c d}) y.",
        direct: "(\\uline{za b}) x (\\uline{yc d}) y.",
        letters: &[("(", 158.676), ("z", 162.548), ("b", 175.275), ("x", 187.999), ("(", 196.584), ("y", 200.453), ("d", 213.456), ("y", 226.180)],
    },
    Boundary {
        name: "quotes, given then default",
        via_macro: "\"\\lb[y]{a b}\" x \"\\lb{c d}\" y.",
        direct: "\"\\uline{ya b}\" x \"\\uline{zc d}\" y.",
        letters: &[("y", 163.655), ("b", 176.935), ("x", 190.766), ("z", 204.326), ("d", 216.500), ("y", 230.331)],
    },
    Boundary {
        name: "plain-text body in parens",
        via_macro: "(\\lt{a}) x (\\lt[y]{b}) w.",
        direct: "(za) x (yb) w.",
        letters: &[("(", 158.676), ("x", 179.147), ("(", 187.722), ("w", 209.586)],
    },
    Boundary {
        name: "em dash",
        via_macro: "p---\\lb{a b}---x q---\\lb[y]{c d}---y.",
        direct: "p---\\uline{za b}---x q---\\uline{yc d}---y.",
        letters: &[("p", 158.676), ("z", 174.169), ("b", 186.896), ("q", 210.965), ("y", 226.183), ("d", 239.187)],
    },
];

#[test]
fn optional_argument_macro_lines_are_identical_and_match_pdflatex() {
    if !common::lm_available() {
        return;
    }
    let items: Vec<&str> = OPTIONAL_BOUNDARIES.iter().flat_map(|b| [b.via_macro, b.direct]).collect();
    let text = doc_optional(&items);
    for (k, b) in OPTIONAL_BOUNDARIES.iter().enumerate() {
        let expanded = glyphs_on(&text, FIRST_BASELINE + BASELINESKIP * (2 * k) as f64);
        let plain = glyphs_on(&text, FIRST_BASELINE + BASELINESKIP * (2 * k + 1) as f64);
        assert!(!expanded.is_empty(), "{}: no glyphs on the macro line", b.name);
        assert_eq!(
            expanded.len(),
            plain.len(),
            "{}: the macro line has different glyphs\n macro: {expanded:?}\n direct: {plain:?}",
            b.name
        );
        for ((text, x), (want_text, want_x)) in expanded.iter().zip(&plain) {
            assert_eq!(text, want_text, "{}: glyph order differs", b.name);
            assert!((x - want_x).abs() < TOL, "{}: `{text}` at {x:.3} via the macro, {want_x:.3} directly", b.name);
        }
        let mut from = 0;
        for &(letter, want_x) in b.letters {
            let at = expanded[from..]
                .iter()
                .position(|(t, _)| t == letter)
                .unwrap_or_else(|| panic!("{}: no glyph `{letter}` after #{from} on the macro line: {expanded:?}", b.name));
            let x = expanded[from + at].1;
            assert!((x - want_x).abs() < TOL, "{}: `{letter}` at {x:.3}, pdflatex {want_x:.3}", b.name);
            from += at + 1;
        }
    }
}
