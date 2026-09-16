//! A text font command that switches to an upright font puts the italic
//! correction of the character *before* it under the interword space
//! (latex.ltx `\DeclareTextFontCommand`: `#2\check@icl ##1\check@icr`, and
//! `\check@icl` is `\maybe@ic` -> `\sw@slant`, which `\unskip`s the space,
//! adds `\/` and puts the space back).
//!
//! amsmath's `\eqref` is `\textup{\tagform@{..}}`, so `Ref \eqref{e:c}` gets
//! the `f` correction too: display-placement fixture 17 had `(2.1).` 0.784 bp
//! left of pdflatex's.
//!
//! ## Oracle
//!
//! pdfTeX 1.40 (MacTeX 2026), `\showbox` of `\hbox{of \textbf{x}}` at 10 pt
//! (T1 Latin Modern):
//!
//! ```text
//! .\T1/lmr/m/n/10 f
//! .\kern 0.7922
//! .\glue 3.33333 plus 1.66666 minus 1.11111
//! .\T1/lmr/bx/n/10 x
//! ```
//!
//! and the same kern for `\eqref`, `\textup`, `\texttt` and `f\textbf{x}`
//! (no space: the kern follows `f` directly), none for `\textit` (the new font
//! is slanted) or `\textbf{.}` (`\nocorrlist`), and the italic `f`'s 1.72777 pt
//! for `\textit{of \emph{x}}`. The word origins below are pdftotext's for
//! [`PROBE`] (`oracle.py refs` conventions), in bp.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

const TOL: f64 = 0.5;

const PROBE: &str = r"\documentclass[10pt]{article}
\usepackage[T1]{fontenc}
\usepackage{lmodern}
\usepackage{amsmath}
\pagestyle{empty}
\begin{document}
Ref of \textbf{bold} and f\textbf{x} and of \texttt{tt}, of \textit{it} of~\textup{u} \textit{of \emph{x}} of \textbf{.}
\end{document}
";

/// pdflatex's word origins: (word, occurrence, x in bp).
const EXPECT: &[(&str, usize, f64)] = &[
    ("bold", 0, 178.9764),
    ("and", 0, 204.2545),
    ("and", 1, 236.89),
    ("tt", 0, 268.3897),
    ("it", 0, 296.289),
    ("u", 0, 318.7895),
    ("x", 1, 341.1496),
    (".", 0, 361.2382),
];

#[test]
fn an_upright_text_command_corrects_the_character_before_its_space() {
    if !common::lm_available() {
        eprintln!("SKIP text_command_left_italic_correction: Latin Modern not installed");
        return;
    }
    let words = common::words_of(&common::render_one(PROBE));
    for &(word, nth, x) in EXPECT {
        let got = words
            .iter()
            .filter(|w| w.text.trim() == word || w.text.trim().trim_end_matches(',') == word)
            .nth(nth)
            .unwrap_or_else(|| panic!("no run #{nth} `{word}` in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()));
        assert!((got.x - x).abs() <= TOL, "`{word}` #{nth}: x {:.3} bp, pdflatex {x:.3} bp ({:+.3})", got.x, got.x - x);
    }
}
