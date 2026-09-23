//! Theorem styles' own skips, against pdflatex (TeX Live 2026, `article`
//! 10pt; baselines from the content stream, `tools/visual-oracle`).
//!
//! amsthm's `remark` style halves `\topsep` for both `\thm@preskip` and
//! `\thm@postskip` (`\thm@preskip\topsep \divide\thm@preskip\tw@`), so a
//! remark sits 4pt from the text around it at 10pt where a theorem sits 8pt.
//! The pipeline gave every `\newtheorem` environment the full `\topsep`,
//! 3.98bp too low before and 7.97bp too low after. Declared styles
//! (`\newtheoremstyle`, thmtools) are unit-tested in `src/thmstyles.rs`:
//! their heads need the compiler half, which reaches this crate at the next
//! `vendor/compiler` re-pin.

mod common;

use common::{lm_available, render_one, words_of};

#[test]
fn remark_style_takes_half_the_topsep_on_both_sides() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{article}\n\\usepackage{amsthm}\n\\theoremstyle{remark}\\newtheorem{remark}{Remark}\n\
        \\begin{document}\nSome text before the remark here.\n\\begin{remark}Remark body text.\\end{remark}\n\
        Text after the remark.\n\\end{document}\n";
    let words = words_of(&render_one(src));
    let mut ys: Vec<f64> = words.iter().filter(|w| w.page == 1).map(|w| w.baseline).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() < 0.05);
    // pdflatex baselines (bp): 134.765, 150.705, 166.645.
    for (i, want) in [134.765, 150.705, 166.645].into_iter().enumerate() {
        assert!((ys[i] - want).abs() < 0.03, "line {}: {:.3}bp, pdflatex {want:.3}bp ({ys:?})", i + 1, ys[i]);
    }
}
