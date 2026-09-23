//! amsthm theorem and proof boundaries on the vertical list, against pdflatex.
//!
//! * A theorem-like environment and `proof` are a `\trivlist` whose head is
//!   an `\item`: `\@item` puts `\addpenalty\@beginparpenalty` in front of
//!   the opening `\addvspace\@topsep` (unless `\@nobreak`, right after a
//!   heading), and `\@endparenv` puts `\addpenalty\@endparpenalty` in front
//!   of the closing `\addvspace\@topsepadd` -- both -51 in article. The
//!   pipeline gave lists these penalties but not theorems, so a page that
//!   pdflatex ends between a proof and the next theorem (`\tracingpages`:
//!   `t=586.04164 ... b=56 p=-51 c=5#`) broke one theorem earlier.
//! * `\addpenalty` moves its penalty in front of a nonzero `\lastskip`
//!   (`\vskip-\lastskip\penalty#1\vskip\@tempskipb`), so the breakpoint is
//!   weighed without the skip the previous environment left behind:
//!   `\penalty -51` then `\glue 8.0 plus 2.0 minus 4.0`, never the reverse.
//! * A list that ends a theorem closes first: its `\endtrivlist` sees the
//!   last item's skip (none after a line of text) and its `\addvspace
//!   \@topsepadd` goes in before the theorem's; `\@xaddvskip` keeps the
//!   earlier one whole on a tie. The pipeline merged the theorem's skip
//!   first, then added the list's `\parsep` on top of it (+4pt at 10pt,
//!   +4.5pt at 11pt), and a list closing a proof kept the proof's
//!   `8.0 plus 7.0 minus 1.0` instead of the list's `8.0 plus 2.0 minus 4.0`,
//!   which moved the page break of `linalg-theorem-lists`.
//!
//! Oracle: MacTeX 2026 pdflatex (`SOURCE_DATE_EPOCH=0`, two runs); page
//! contents from `pdftotext`, baselines from the content stream
//! (`tools/visual-oracle/pdftext.py`). No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of, Word};

/// The runs of each page's first line, joined by blanks.
fn first_lines(words: &[Word]) -> Vec<String> {
    let pages = words.iter().map(|w| w.page).max().unwrap_or(0);
    (1..=pages)
        .map(|page| {
            let on: Vec<&Word> = words.iter().filter(|w| w.page == page).collect();
            let top = on.iter().map(|w| w.baseline).fold(f64::INFINITY, f64::min);
            let mut line: Vec<&&Word> = on.iter().filter(|w| (w.baseline - top).abs() < 0.05).collect();
            line.sort_by(|a, b| a.x.total_cmp(&b.x));
            line.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
        })
        .collect()
}

/// Distinct page-1 baselines, top to bottom (bp).
fn baselines(words: &[Word]) -> Vec<f64> {
    let mut ys: Vec<f64> = words.iter().filter(|w| w.page == 1).map(|w| w.baseline).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() < 0.05);
    ys
}

fn fixture(name: &str) -> String {
    let path = format!("{}/../../fixtures/proof-corpus/{name}/main.tex", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn page_breaks_between_a_proof_and_the_next_theorem_like_pdflatex() {
    if !lm_available() {
        return;
    }
    // pdflatex: page 2 of each opens with this theorem head.
    for (name, want) in [("paper-cref", "Corollary 3.1."), ("linalg-theorem-lists", "Remark 2.2.")] {
        let r = render_one(&fixture(name));
        let firsts = first_lines(&words_of(&r));
        assert_eq!(firsts.len(), 2, "{name}: pages {firsts:#?}");
        assert!(
            firsts[1].replace(' ', "").starts_with(&want.replace(' ', "")),
            "{name}: page 2 opens with {:?}; pdflatex opens it with {want:?}",
            firsts[1]
        );
    }
}

#[test]
fn a_list_ending_a_theorem_leaves_only_the_theorem_skip() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\\usepackage{amsmath,amssymb,amsthm}\n\\usepackage{enumitem}\n\
        \\newtheorem{theorem}{Theorem}\n\\begin{document}\n\
        \\begin{theorem}Statement:\\begin{enumerate}\\item first;\\item second.\\end{enumerate}\\end{theorem}\n\
        \\begin{proof}Next text.\\end{proof}\n\
        \\begin{theorem}Plain statement.\\end{theorem}\n\
        \\begin{proof}Next text.\\end{proof}\n\\end{document}\n";
    let ys = baselines(&words_of(&render_one(src)));
    // pdflatex baselines (bp): 134.765 154.690 174.615 194.540 214.466
    // 234.391 -- the proof head sits 12pt + \topsep (8pt) below the last
    // item, as every other gap here does; the pipeline had it 4pt lower.
    let want = [134.765, 154.690, 174.615, 194.540, 214.466, 234.391];
    assert!(ys.len() >= want.len(), "baselines {ys:?}");
    for (i, (got, want)) in ys.iter().zip(want).enumerate() {
        assert!((got - want).abs() < 0.02, "line {}: baseline {got:.3}bp, pdflatex {want:.3}bp (all: {ys:?})", i + 1);
    }
}

#[test]
fn a_claim_and_its_proof_nested_in_a_proof_keep_the_outer_proof_open() {
    if !lm_available() {
        return;
    }
    // A lemma opened inside a proof reads the proof's `\topsep6\p@\@plus6\p@`
    // (18pt below the proof's first line, not 20pt); the inner proof's end
    // is an `\@endparenv` of its own although the outer proof goes on, so
    // the text after it sits 12pt + 8pt lower, not 12pt.
    let src = "\\documentclass{article}\n\\usepackage{amsmath,amssymb,amsthm}\n\\newtheorem{theorem}{Theorem}\n\\newtheorem{lemma}[theorem]{Lemma}\n\\begin{document}\n\\begin{proof}Opening text of the proof.\n\\begin{lemma}Inner claim.\\end{lemma}\n\\begin{proof}Inner proof.\\end{proof}\nClosing text of the proof.\\end{proof}\nAfter paragraph.\n\\end{document}\n";
    let ys = baselines(&words_of(&render_one(src)));
    // pdflatex baselines (bp).
    let want = [134.765, 152.700, 172.620, 192.550, 212.470];
    assert!(ys.len() >= want.len(), "baselines {ys:?}");
    for (i, (got, want)) in ys.iter().zip(want).enumerate() {
        assert!((got - want).abs() < 0.02, "line {}: baseline {got:.3}bp, pdflatex {want:.3}bp (all: {ys:?})", i + 1);
    }
}

#[test]
fn a_proof_right_after_an_item_label_shares_the_label_line() {
    if !lm_available() {
        return;
    }
    // `\item[(a)] \begin{proof}`: `\@noparlist`, so pdflatex sets `(a)` and
    // the proof head on one line and adds no `\topsep` around the proof.
    let src = "\\documentclass{article}\n\\usepackage{amsmath,amssymb,amsthm}\n\\begin{document}\n\\begin{proof}We prove both.\n\\begin{itemize}\n  \\item[(a)] \\begin{proof}[Proof of (a)]Induct on n.\\end{proof}\n  \\item[(b)] \\begin{proof}[Proof of (b)]Induct again.\\end{proof}\n\\end{itemize}\n\\end{proof}\n\\end{document}\n";
    let ys = baselines(&words_of(&render_one(src)));
    // pdflatex baselines (bp): the proof, then one line per item.
    let want = [134.765, 154.690, 174.620];
    assert!(ys.len() >= want.len(), "baselines {ys:?}");
    for (i, (got, want)) in ys.iter().zip(want).enumerate() {
        assert!((got - want).abs() < 0.02, "line {}: baseline {got:.3}bp, pdflatex {want:.3}bp (all: {ys:?})", i + 1);
    }
}
