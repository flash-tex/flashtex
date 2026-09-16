//! Line breaks inside inline math against pdflatex: TeX breaks a paragraph
//! inside a text-style formula after a top-level Bin atom (`\binoppenalty`
//! 700) or Rel atom (`\relpenalty` 500), never inside `\left...\right`,
//! fractions or scripts, and the `\medmuskip`/`\thickmuskip` glue after
//! the operator vanishes at the break (and stretches with the line when
//! no break is taken).
//!
//! Fixture: `fixtures/real-world/inline-math/main.tex` (the user's Chain
//! Rule theorem/proof paragraph, which overflowed the margin when every
//! formula was one unbreakable box, plus five more paragraphs of dense
//! inline math). Expected data: `fixtures/inline-math/expected.txt`, every
//! word's origin and baseline read from the content streams of pdflatex's
//! own PDF (MacTeX 2026, `reference.pdf` next to the source) by
//! `tools/inline-math-oracle/generate.py`. No TeX runs here.
//!
//! A line is compared by its letters (the oracle writes `?` for symbol
//! glyphs, and script rows are skipped): the same letters on the same
//! lines means the same breaks. The Chain Rule paragraphs must match line for line and no
//! line may be overfull (pdflatex reports none). Lines listed in
//! `REPORTED` differ for reasons outside this lane and are printed, not
//! gated.

mod common;

use common::*;
use std::collections::BTreeMap;

/// Expected lines (by their letters) that still break differently:
/// * "Linear maps": `\ker T`/`\dim` operator names (the pipeline sets
///   them as `dimker` with no thin space) shift the paragraph's last
///   three lines and one `op` subscript row (12 letters, so not skipped);
/// * "Number theory": `\cdots` between `p_k` factors (three ordinary
///   dots, spaced differently) moves `prime` up one line;
/// * "Inequalities": the paragraph's last two lines (`and` moves up).
const REPORTED: &[&str] = &[
    "opijopjjmjij",
    "imageTRTxxRsatisfydimkerTdimTRnandTisinjectiveifandonly",
    "ifkerTifandonlyifTxcxforsomecandallxR",
    "acontradictionConsequentlyeverynfactorsasnpppwithppp",
    "primeandeuniquelyandneeenpp",
    "triangleinequalityuvuvandalsouvuvsothatxxisLipschitz",
    "andinparticularcontinuousonR",
];

fn repo_fixture() -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/real-world/inline-math");
    std::fs::read_to_string(dir.join("main.tex")).expect("fixtures/real-world/inline-math/main.tex")
}

fn expected() -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/inline-math/expected.txt")).expect("expected.txt")
}

/// The letters of a line's text: the oracle writes `?` for a glyph it
/// cannot name and joins glyphs into words by gap where our runs split at
/// every font change, so the comparison is on the concatenated ASCII
/// letters (with the blackboard-bold letters the oracle names by their
/// msbm slot mapped back).
fn letters(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\u{2115}' => Some('N'),
            '\u{2124}' => Some('Z'),
            '\u{211A}' => Some('Q'),
            '\u{211D}' => Some('R'),
            c if c.is_ascii_alphabetic() => Some(c),
            _ => None,
        })
        .collect()
}

type Lines = Vec<((u32, i64), String)>;

/// Lines as `((page, baseline in 0.1bp), letters)` from `(page, x,
/// baseline, text)` words, skipping lines of fewer than 12 letters: a
/// script row (`n`, `op`; the pipeline's subscript baselines are not the
/// oracle's, outside this lane), a page number, a paragraph's short last
/// line (whose break the line before it already fixes).
fn lines_of(words: impl Iterator<Item = (u32, f64, f64, String)>) -> Lines {
    let mut by_line: BTreeMap<(u32, i64), Vec<(i64, String)>> = BTreeMap::new();
    for (page, x, baseline, text) in words {
        by_line.entry((page, (baseline * 10.0).round() as i64)).or_default().push(((x * 1000.0).round() as i64, text));
    }
    let mut out = Vec::new();
    for (key, mut ws) in by_line {
        ws.sort();
        let sig: String = ws.iter().map(|(_, t)| letters(t)).collect();
        if sig.chars().count() >= 12 {
            out.push((key, sig));
        }
    }
    out
}

fn oracle_lines(data: &str) -> Lines {
    lines_of(data.lines().filter(|l| l.starts_with("word ")).map(|l| {
        let f: Vec<&str> = l.splitn(6, ' ').collect();
        (f[1].parse().unwrap(), f[2].parse().unwrap(), f[3].parse().unwrap(), f[5].to_string())
    }))
}

fn our_lines(r: &flashtex_render_pipeline::Rendered) -> Lines {
    lines_of(words_of(r).into_iter().map(|w| (w.page, w.x, w.baseline, w.text)))
}

/// Longest common subsequence alignment of the two line lists: the pairs
/// of equal lines, and the expected lines left unmatched.
fn align(want: &Lines, got: &Lines) -> (usize, Vec<String>) {
    let (n, m) = (want.len(), got.len());
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if want[i].1 == got[j].1 { lcs[i + 1][j + 1] + 1 } else { lcs[i + 1][j].max(lcs[i][j + 1]) };
        }
    }
    let (mut i, mut j, mut same, mut missing) = (0, 0, 0, Vec::new());
    while i < n && j < m {
        if want[i].1 == got[j].1 {
            same += 1;
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            missing.push(want[i].1.clone());
            i += 1;
        } else {
            j += 1;
        }
    }
    missing.extend(want[i..].iter().map(|l| l.1.clone()));
    (same, missing)
}

fn overfull(r: &flashtex_render_pipeline::Rendered) -> Vec<String> {
    r.v2.diagnostics.iter().filter(|d| d.code == "overfull_hbox").map(|d| d.message.clone()).collect()
}

#[test]
fn inline_math_breaks_against_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let tex = repo_fixture();
    let want = oracle_lines(&expected());
    assert!(want.len() >= 40, "expected data has {} prose lines", want.len());

    let r = render_one(&tex);
    assert_eq!(r.v2.pages.len(), 2, "pdflatex sets the fixture on two pages");
    let got = our_lines(&r);
    let (same, missing) = align(&want, &got);
    eprintln!("inline-math: {same}/{} lines break as pdflatex; overfull {:?}", want.len(), overfull(&r));
    for l in &missing {
        eprintln!("    differs: {l}");
    }
    // pdflatex reports no overfull box for the fixture; before formulas
    // could break, the Chain Rule proof overflowed by 30pt.
    assert!(overfull(&r).is_empty(), "overfull lines: {:?}", overfull(&r));
    // The Chain Rule theorem and proof (everything before the "Sequences"
    // heading) break exactly as pdflatex.
    let chain: Vec<&String> = want.iter().map(|l| &l.1).take_while(|l| !l.starts_with("Sequencesandseries")).collect();
    assert!(chain.len() >= 10, "the Chain Rule section has {} prose lines", chain.len());
    for l in &chain {
        assert!(!missing.contains(l), "Chain Rule line breaks differently from pdflatex: {l}");
    }
    let unexpected: Vec<&String> = missing.iter().filter(|l| !REPORTED.contains(&l.as_str())).collect();
    assert!(unexpected.is_empty(), "lines breaking differently from pdflatex and not in REPORTED: {unexpected:?}");
    assert!(same + REPORTED.len() >= want.len(), "{same} of {} lines match; REPORTED covers {}", want.len(), REPORTED.len());

    // With every formula one unbreakable box the proof overflows: the
    // break points are what fixes it.
    std::env::set_var("FLASHTEX_INLINE_MATH_BREAKS", "0");
    let one_box = render_one(&tex);
    std::env::remove_var("FLASHTEX_INLINE_MATH_BREAKS");
    assert!(!overfull(&one_box).is_empty(), "the fixture should overflow without breaks inside formulas");

    // Pieces of an unbroken formula sit exactly where the one box's
    // children did: on a paragraph's last line (set at natural width) the
    // display list is the same with and without the cuts.
    let doc = "\\documentclass{article}\\usepackage{amssymb}\\begin{document}Let $r\\in\\mathbb{Q}\\setminus\\{0\\}$ and $a+b=c$ with $f(x) = \\frac{a+b}{2} + \\left(x - 1\\right) \\le 3 x$ hold.\n\nThen $\\|L_f\\|_{\\mathrm{op}} |v| \\to 0$ as $h\\to 0$; also $\\lim_{h\\to 0} |E_g(h)|/|h| = 0$.\\end{document}";
    std::env::set_var("FLASHTEX_INLINE_MATH_BREAKS", "0");
    let whole = render_one(doc);
    std::env::remove_var("FLASHTEX_INLINE_MATH_BREAKS");
    let cut = render_one(doc);
    assert!(overfull(&cut).is_empty() && overfull(&whole).is_empty());
    assert_eq!(cut.v2.pages, whole.v2.pages, "an unbroken formula must render exactly as one box");
}

/// A paragraph narrower than its formula breaks after a Bin/Rel atom (the
/// operator ends the upper line), never inside `\left...\right`.
#[test]
fn breaks_after_operator_not_inside_fences() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // 2in of text: the formula alone is wider than the line.
    let doc = "\\documentclass{article}\\usepackage[textwidth=2in]{geometry}\\begin{document}\\noindent Now $f(g(a+h)) = f(b+k(h)) = f(b) + L_f(k(h)) + E_f(k(h))$ so.\n\n\\noindent Now $\\left(a+b+c+d+e+f+g+h+i+j+k+l+m+n\\right) = z$ so.\\end{document}";
    let r = render_one(doc);
    let words = words_of(&r);
    // The first paragraph's lines in order: every line before its last
    // ends after the operator the formula broke at.
    let mut by_line: BTreeMap<i64, Vec<(i64, String)>> = BTreeMap::new();
    for w in &words {
        by_line.entry((w.baseline * 10.0).round() as i64).or_default().push(((w.x * 1000.0).round() as i64, w.text.clone()));
    }
    let ends: Vec<String> = by_line
        .into_values()
        .map(|mut ws| {
            ws.sort();
            ws.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>().join(" ")
        })
        .collect();
    eprintln!("{ends:#?}");
    let last = ends.iter().position(|l| l.contains("so.")).expect("the first paragraph ends with `so.`");
    assert!(last >= 1, "the first formula must break: {ends:?}");
    for l in &ends[..last] {
        let end = l.trim_end().chars().last().unwrap();
        assert!(matches!(end, '=' | '+'), "a line inside the formula must end after the operator, got {l:?}");
    }
    // The fenced sum never breaks: its only break is after `=`, so the
    // whole `(...)` is one line's run and overflows.
    let fenced: Vec<&String> = ends.iter().filter(|l| l.contains("a + b")).collect();
    assert_eq!(fenced.len(), 1, "the \\left...\\right body must stay on one line: {ends:?}");
    assert!(fenced[0].contains("+ n"), "{:?}", fenced[0]);
    assert!(!overfull(&r).is_empty(), "the unbreakable fenced sum is wider than 2in and must be reported overfull");
}
