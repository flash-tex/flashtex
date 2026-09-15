//! Chapters whose whole body is one captioned float (no prose): each
//! `\chapter` still clears to its own page, in reading order, whether the
//! chapter comes from an `\include`d file (with or without `\includeonly`)
//! or stands in the entry file. A file without a paragraph has no unit for
//! the adapter to lay its `\chapter` out before; before the fix its heading
//! and page break were lost and the floats piled onto one page.
//!
//! Expected data: MacTeX 2026 pdflatex, three runs, `report` 10pt; every
//! line of every page and its baseline read from the PDF with PyMuPDF.
//! No TeX runs here.

mod common;

use common::*;

const ONE: &str = "\\chapter{One}\n\\begin{figure}[h]\\caption{Fig one.}\\label{f1}\\end{figure}\n";
const TWO: &str = "\\chapter{Two}\n\\begin{table}[h]\\caption{Tab two.}\\label{t2}\\end{table}\n";
const THREE: &str = "\\chapter{Three}\n\\begin{figure}[h]\\caption{Fig three.}\\label{f3}\\end{figure}\n";

/// Each page's lines (words joined, left to right) with their baselines.
fn lines(r: &flashtex_render_pipeline::Rendered) -> Vec<Vec<(String, f64)>> {
    let words = words_of(r);
    r.v2
        .pages
        .iter()
        .map(|page| {
            let mut on_page: Vec<&Word> = words.iter().filter(|w| w.page == page.number).collect();
            on_page.sort_by(|a, b| a.baseline.total_cmp(&b.baseline).then(a.x.total_cmp(&b.x)));
            let mut out: Vec<(f64, Vec<&str>)> = Vec::new();
            for w in on_page {
                match out.last_mut() {
                    Some((b, line)) if (w.baseline - *b).abs() < 0.5 => line.push(&w.text),
                    _ => out.push((w.baseline, vec![&w.text])),
                }
            }
            out.into_iter().map(|(b, l)| (l.join(" "), b)).collect()
        })
        .collect()
}

/// pdflatex pages as `(line, baseline)`; the folio closes every page.
type Pages = &'static [&'static [(&'static str, f64)]];

fn check(name: &str, main: &str, expected: Pages) {
    let r = render_docs(&[("three.tex", THREE), ("two.tex", TWO), ("one.tex", ONE), ("main.tex", main)], "main.tex");
    let got = lines(&r);
    eprintln!("{name}: {got:#?}");
    assert_eq!(got.len(), expected.len(), "{name}: page count (pdflatex: {})", expected.len());
    for (p, (got, want)) in got.iter().zip(expected).enumerate() {
        let texts: Vec<&str> = got.iter().map(|(t, _)| t.as_str()).collect();
        let want_texts: Vec<&str> = want.iter().map(|(t, _)| *t).collect();
        assert_eq!(texts, want_texts, "{name}: lines of page {}", p + 1);
        for ((text, b), (_, wb)) in got.iter().zip(*want) {
            assert!((b - wb).abs() < 0.25, "{name}: page {} `{text}` baseline {b:.2}, pdflatex {wb:.2}", p + 1);
        }
    }
}

const CH1: &[(&str, f64)] = &[("Chapter 1", 209.48), ("One", 259.30), ("Figure 1.1: Fig one.", 327.87), ("1", 702.64)];
const CH2: &[(&str, f64)] = &[("Chapter 2", 209.48), ("Two", 259.30), ("Table 2.1: Tab two.", 327.99), ("2", 702.64)];
const CH3: &[(&str, f64)] = &[("Chapter 3", 209.48), ("Three", 259.30), ("Figure 3.1: Fig three.", 327.99), ("3", 702.64)];
/// `\includeonly{one,three}`: chapter Three is chapter 2.
const CH3_AS_2: &[(&str, f64)] = &[("Chapter 2", 209.48), ("Three", 259.30), ("Figure 2.1: Fig three.", 327.99), ("2", 702.64)];
const CH2_REFS: &[(&str, f64)] = &[("Chapter 2", 209.48), ("Two", 259.30), ("Table 2.1: Tab two.", 327.99), ("Refs 1.1 and 2.1.", 351.90), ("2", 702.64)];
const REFS_2_3: &[(&str, f64)] = &[("Refs 1.1 and 2.1.", 134.77), ("3", 702.64)];
const REFS_3_4: &[(&str, f64)] = &[("Refs 1.1, 2.1 and 3.1.", 134.77), ("4", 702.64)];

#[test]
fn consecutive_float_only_included_chapters_clear_their_own_pages() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    check(
        "two",
        "\\documentclass{report}\n\\begin{document}\n\\include{one}\n\\include{two}\nRefs \\ref{f1} and \\ref{t2}.\n\\end{document}\n",
        &[CH1, CH2, REFS_2_3],
    );
    check(
        "three",
        "\\documentclass{report}\n\\begin{document}\n\\include{one}\n\\include{two}\n\\include{three}\nRefs \\ref{f1}, \\ref{t2} and \\ref{f3}.\n\\end{document}\n",
        &[CH1, CH2, CH3, REFS_3_4],
    );
}

#[test]
fn float_only_included_chapters_under_includeonly() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    check(
        "includeonly",
        "\\documentclass{report}\n\\includeonly{one,three}\n\\begin{document}\n\\include{one}\n\\include{two}\n\\include{three}\nRefs \\ref{f1} and \\ref{f3}.\n\\end{document}\n",
        &[CH1, CH3_AS_2, REFS_2_3],
    );
    // Nothing after the last `\include`.
    check(
        "includeonly, no trailing text",
        "\\documentclass{report}\n\\includeonly{one,three}\n\\begin{document}\n\\include{one}\n\\include{two}\n\\include{three}\n\\end{document}\n",
        &[CH1, CH3_AS_2],
    );
}

#[test]
fn float_only_chapters_in_one_file() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    check(
        "single file",
        "\\documentclass{report}\n\\begin{document}\n\\chapter{One}\n\\begin{figure}[h]\\caption{Fig one.}\\label{f1}\\end{figure}\n\\chapter{Two}\n\\begin{table}[h]\\caption{Tab two.}\\label{t2}\\end{table}\nRefs \\ref{f1} and \\ref{t2}.\n\\end{document}\n",
        &[CH1, CH2_REFS],
    );
    // The last `\chapter` has nothing but its float after it.
    check(
        "single file, no trailing text",
        "\\documentclass{report}\n\\begin{document}\n\\chapter{One}\n\\begin{figure}[h]\\caption{Fig one.}\\label{f1}\\end{figure}\n\\chapter{Two}\n\\begin{table}[h]\\caption{Tab two.}\\label{t2}\\end{table}\n\\end{document}\n",
        &[CH1, CH2],
    );
}
