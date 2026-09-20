//! CJK.sty's `CJK` environment (`\usepackage{CJKutf8}`): every CJK
//! character is a 1 em box of its `C70` subfont's metrics with `\CJKglue`
//! between characters (`src/cjk.rs`, `typeset::Context::cjk_items`), so the
//! Latin words around a CJK run and the CJK runs themselves land where
//! pdflatex puts them.
//!
//! The expectations are pdflatex's word origins (TeX Live 2026,
//! `SOURCE_DATE_EPOCH=0`, `tools/visual-oracle/pdftext.py` on the PDF of the
//! probe below at each class size), in bp from the page's top-left; the
//! probe covers a Japanese line, a Chinese (`gbsn`) line, a mixed
//! Latin/CJK line, CJK inside a Latin sentence with spaces, the `。`/`、`/
//! `「」` no-break punctuation, a paragraph of CJK that wraps (line breaks
//! between characters), `\textbf` (fake bold, 1.03 em), `CJK*`, Korean
//! (`mj`) and traditional Chinese (`bsmi`). Positions only: the outlines are
//! painted from an installed font that is never the wadalab/arphic/uhc
//! design (`font_face_substituted`), and where no CJK font is installed the
//! boxes keep their place (`missing_glyph`), so this passes on any machine
//! that resolves Latin Modern.

mod common;

use common::{lm_available, render_one, words_of, Word};

const PROBE: &str = r#"\documentclass[SIZEpt]{article}
\usepackage[T1]{fontenc}
\usepackage[utf8]{inputenc}
\usepackage[margin=1in]{geometry}
\usepackage{CJKutf8}
\begin{document}
\section{Japanese}
\begin{CJK}{UTF8}{min}
東京は日本の首都です。
\end{CJK}
(``Tokyo is the capital of Japan.'')

\begin{CJK}{UTF8}{gbsn}
排版是一门艺术。
\end{CJK}
(``Typesetting is an art.'')

\section{Mixed}
The r\'esum\'e of Søren --- filed under ``K'' --- lists Zürich, Łódź and
\begin{CJK}{UTF8}{min}東京\end{CJK} as places of residence, 2019--2026.

Inside: \begin{CJK}{UTF8}{min}東京 and 大阪 are big; 「東京」、大阪。end\end{CJK} after it.

\begin{CJK}{UTF8}{min}
日本語の文章はこのように長く書かれると自動的に行を折り返します。日本語の文章はこのように長く書かれると自動的に行を折り返します。日本語の文章はこのように長く書かれると自動的に行を折り返します。「括弧」や、句読点。の前後では行が分かれません。おわり
\end{CJK}
fin

Bold \begin{CJK}{UTF8}{min}\textbf{東京}\end{CJK} x and star
\begin{CJK*}{UTF8}{min}東京 大阪 abc 東 \end{CJK*} y and Korean
\begin{CJK}{UTF8}{mj}한국어 문장\end{CJK} z and traditional
\begin{CJK}{UTF8}{bsmi}藝術\end{CJK} w.
\end{document}
"#;

/// The words of the probe in reading order, each with pdflatex's `x` (bp).
/// A one-character needle is a CJK character's box (its own glyph run);
/// repeated needles (`and`, `東`) are consumed in order.
type Expect = &'static [(&'static str, f64)];

const AT_10PT: Expect = &[
    ("東", 72.000), ("(“Tokyo", 188.234), ("Japan.”)", 296.466),
    ("排", 86.944), ("(“Typesetting", 173.290), ("art.”)", 259.436),
    ("and", 339.682), ("東", 359.046), ("as", 382.289), ("residence,", 435.245),
    ("Inside:", 86.944), ("東", 119.928), ("and", 143.171), ("大", 162.535), ("are", 185.778), ("big;", 202.414), ("「", 221.778), ("after", 320.289), ("it.", 343.832),
    ("fin", 397.448),
    ("Bold", 86.944), ("東", 110.597), ("x", 134.438), ("star", 162.387), ("東", 182.388), ("東", 240.775), ("y", 254.065), ("Korean", 282.004), ("한", 316.611), ("z", 373.069), ("traditional", 400.187), ("藝", 449.460), ("w.", 472.703),
];

const AT_11PT: Expect = &[
    ("東", 72.000), ("(“Tokyo", 199.233), ("Japan.”)", 317.126),
    ("排", 88.936), ("(“Typesetting", 183.442), ("art.”)", 277.279),
    ("and", 381.617), ("東", 404.188), ("as", 432.006), ("residence,", 494.133),
    ("Inside:", 88.936), ("東", 124.857), ("and", 150.287), ("大", 171.385), ("are", 196.814), ("big;", 214.901), ("「", 235.993), ("after", 343.762), ("it.", 369.374),
    ("fin", 515.597),
    ("Bold", 88.936), ("東", 114.703), ("x", 140.787), ("star", 171.221), ("東", 192.975), ("東", 256.811), ("y", 271.331), ("Korean", 301.765), ("한", 339.429), ("z", 401.207), ("traditional", 430.738), ("藝", 484.378), ("w.", 509.818),
];

const AT_12PT: Expect = &[
    ("東", 72.000), ("(“Tokyo", 211.314), ("Japan.”)", 338.481),
    ("排", 89.559), ("(“Typesetting", 193.007), ("art.”)", 294.240),
    ("and", 380.305), ("東", 402.543), ("as", 429.849), ("residence,", 490.518),
    ("Inside:", 89.559), ("東", 128.303), ("and", 156.111), ("大", 178.875), ("are", 206.683), ("big;", 226.185), ("「", 248.949), ("after", 366.692), ("it.", 394.322),
    ("fin", 175.448),
    ("Bold", 89.559), ("東", 117.192), ("x", 145.574), ("star", 178.090), ("東", 201.414), ("東", 270.855), ("y", 286.564), ("Korean", 319.080), ("한", 359.567), ("z", 426.827), ("traditional", 458.379), ("藝", 516.086), ("w.", 72.000),
];

/// The wrapped CJK paragraph: CJK boxes per line, pdflatex's line breaks.
const WRAP_10PT: &[usize] = &[45, 46, 32];
const WRAP_11PT: &[usize] = &[41, 42, 40];
const WRAP_12PT: &[usize] = &[37, 39, 39, 8];

fn reading_order(mut words: Vec<Word>) -> Vec<Word> {
    words.sort_by(|a, b| (a.page, (a.baseline * 100.0).round() as i64, (a.x * 1000.0).round() as i64).cmp(&(b.page, (b.baseline * 100.0).round() as i64, (b.x * 1000.0).round() as i64)));
    words
}

fn is_cjk(text: &str) -> bool {
    let mut chars = text.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if c as u32 >= 0x2e80)
}

fn check(size: u32, expect: Expect, wrap: &[usize]) {
    let source = PROBE.replace("SIZE", &size.to_string());
    let rendered = render_one(&source);
    let words = reading_order(words_of(&rendered));
    let mut at = 0usize;
    let mut failures = Vec::new();
    for (needle, x) in expect {
        let Some(i) = words[at..].iter().position(|w| w.text == *needle) else {
            failures.push(format!("{size}pt: {needle:?} not found after word {at}"));
            continue;
        };
        let w = &words[at + i];
        if (w.x - x).abs() > 0.05 {
            failures.push(format!("{size}pt: {needle:?} at x {:.3} bp, pdflatex {x:.3} (dx {:+.3})", w.x, w.x - x));
        }
        at += i + 1;
    }
    // The wrapped paragraph: the lines between `it.` and `fin`, counted by
    // baseline.
    let it = words.iter().position(|w| w.text == "it.").expect("`it.`");
    let fin = words.iter().position(|w| w.text == "fin").expect("`fin`");
    let mut lines: Vec<(i64, usize)> = Vec::new();
    for w in &words[it + 1..=fin] {
        if !is_cjk(&w.text) {
            continue;
        }
        let key = (w.baseline * 100.0).round() as i64;
        match lines.last_mut() {
            Some((k, n)) if *k == key => *n += 1,
            _ => lines.push((key, 1)),
        }
    }
    let counts: Vec<usize> = lines.iter().map(|(_, n)| *n).collect();
    if counts != wrap {
        failures.push(format!("{size}pt: wrapped paragraph has {counts:?} CJK boxes per line, pdflatex {wrap:?}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn probe_at_10pt_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    check(10, AT_10PT, WRAP_10PT);
}

#[test]
fn probe_at_11pt_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    check(11, AT_11PT, WRAP_11PT);
}

#[test]
fn probe_at_12pt_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    check(12, AT_12PT, WRAP_12PT);
}

/// The fixture line that ranked 8th in the visual oracle
/// (`docs/evidence/visual-oracle-2026-09-19T210411Z`, `Japan.”)` -39.86 bp):
/// pdflatex's origins from `fixtures/real-world/unicode-accents/reference.pdf`.
#[test]
fn unicode_accents_cjk_lines_match_the_reference() {
    if !lm_available() {
        return;
    }
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/unicode-accents/main.tex")).expect("fixture");
    let rendered = render_one(&source);
    let words = reading_order(words_of(&rendered));
    let expect: &[(&str, f64, f64)] = &[
        ("東", 72.000, 522.856),
        ("(“Tokyo", 199.233, 522.856),
        ("Japan.”)", 317.126, 522.856),
        ("排", 88.936, 536.405),
        ("(“Typesetting", 183.442, 536.405),
        ("art.”)", 277.279, 536.405),
        ("Zürich,", 314.244, 595.123),
        ("and", 381.617, 595.123),
        ("東", 404.188, 595.123),
        ("as", 432.006, 595.123),
        ("residence,", 494.133, 595.123),
    ];
    let mut at = 0usize;
    for (needle, x, y) in expect {
        let i = words[at..].iter().position(|w| w.text == *needle && (w.baseline - y).abs() < 0.5).unwrap_or_else(|| panic!("{needle:?} on baseline {y}"));
        let w = &words[at + i];
        assert!((w.x - x).abs() < 0.5, "{needle:?} at x {:.3} bp, pdflatex {x:.3}", w.x);
        at += i + 1;
    }
    // One substitution note (or one missing-font warning) per family, and
    // no per-character `missing_glyph` for the CJK text itself.
    let diags = &rendered.v2.diagnostics;
    for family in ["C70/min", "C70/gbsn"] {
        let named = diags.iter().filter(|d| d.message.starts_with(family) || d.message.contains(&format!("paints {family}"))).count();
        assert_eq!(named, 1, "one diagnostic names {family}: {:?}", diags.iter().map(|d| &d.message).collect::<Vec<_>>());
    }
    assert!(!diags.iter().any(|d| d.code == "missing_glyph" && d.message.contains("U+6771")), "東 is set, not reported missing: {:?}", diags.iter().map(|d| &d.message).collect::<Vec<_>>());
}

/// `\begin{CJK}` without the package: pdflatex's "Environment CJK
/// undefined" leaves the arguments in the text, and so does the compiler.
#[test]
fn without_the_package_the_arguments_stay_text() {
    if !lm_available() {
        return;
    }
    let rendered = render_one("\\documentclass{article}\\usepackage[utf8]{inputenc}\\begin{document}\\begin{CJK}{UTF8}{min}東\\end{CJK} x\\end{document}");
    let words = words_of(&rendered);
    assert!(words.iter().any(|w| w.text.contains("UTF8")), "{:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>());
}
