//! A source `\addvspace` merges with the pipeline's own `\addvspace` glue
//! (latex.ltx `\@xaddvskip`: the larger natural skip is kept, never the sum).
//!
//! The compiler marks such glue `Block::AddVSpace`; this file builds only
//! with the `compiler-addvspace` feature, i.e. against a `vendor/compiler`
//! re-pinned past that variant (KERNEL-REGISTERS-LEFTOVER, PR #1066).
//!
//! ## Oracle
//!
//! pdfTeX 1.40 (MacTeX 2026), `article` 10pt, `\parindent0pt`, PyMuPDF span
//! origins in bp. The baselines below are pdflatex's for the two probe
//! bodies. What they pin:
//!
//! * `\end{itemize}\addvspace{20pt}` 20pt (not `\@topsepadd` 8pt + 20pt);
//!   `\addvspace{3pt}` there 8pt; the same for `enumerate`, `description`
//!   and a nested list's end.
//! * inside a list, `\end{itemize}` (level 2) `\addvspace{15pt}\item`: 15pt,
//!   plus the outer `\parsep`.
//! * `\section{S}\addvspace{20pt}` 20pt after the heading, `\addvspace{5pt}`
//!   the heading's own 2.3ex after-skip.
//! * `\addvspace{20pt}\begin{itemize}` 20pt; `Ac\par\addvspace{3pt}` before
//!   a list: `\topsep` + `\partopsep` 10pt.
//! * `\addvspace{20pt}\section`: max(20pt, the 3.5ex before-skip).
//! * `\end{center}\addvspace{20pt}` and a display's `\belowdisplayshortskip`
//!   followed by `\addvspace{20pt}`: 20pt.
//! * `\end{itemize}\vspace{5pt}\addvspace{20pt}`: 8 + 5 + 20pt (`\vspace`
//!   leaves `\lastskip` zero); `\end{itemize}\addvspace{20pt}\vspace{5pt}`:
//!   20 + 5pt.
//!
//! pdflatex is an oracle only and never runs in the product path.
#![cfg(feature = "compiler-addvspace")]

mod common;

const WORD_TOL_BP: f64 = 0.5;

fn check(body: &str, expected: &[(&str, f64)]) {
    let source = format!("\\documentclass{{article}}\n\\parindent0pt\n\\begin{{document}}\n{body}\\end{{document}}\n");
    let words = common::words_of(&common::render_one(&source));
    for (text, baseline) in expected {
        let word = words
            .iter()
            .find(|w| w.text.trim() == *text)
            .unwrap_or_else(|| panic!("no run `{text}` in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()));
        assert!(
            (word.baseline - baseline).abs() <= WORD_TOL_BP,
            "{text}: {:.3} bp, pdflatex {baseline:.3} bp ({:+.3})",
            word.baseline,
            word.baseline - baseline
        );
    }
}

#[test]
fn addvspace_after_lists_and_headings_keeps_the_larger_skip() {
    check(
        "Aa
\\begin{itemize}\\item Ib\\end{itemize}
Ac
\\begin{itemize}\\item Id\\end{itemize}
\\addvspace{20pt}
Ae
\\begin{itemize}\\item If\\end{itemize}
\\addvspace{3pt}
Ag
\\begin{enumerate}\\item Eh\\end{enumerate}
\\addvspace{20pt}
Ai
\\begin{description}\\item[Dj] x\\end{description}
\\addvspace{20pt}
Ak
\\begin{itemize}\\item Il \\begin{itemize}\\item Im\\end{itemize}\\end{itemize}
\\addvspace{20pt}
An
\\begin{itemize}\\item Io \\begin{itemize}\\item Ip\\end{itemize}
\\addvspace{15pt}
\\item Iq\\end{itemize}
Ar
\\section{Ss}
\\addvspace{20pt}
At
\\section{Su}
\\addvspace{5pt}
Av
",
        &[
            ("Aa", 134.765),
            ("Ib", 154.690),
            ("Ac", 174.615),
            ("Id", 194.540),
            ("Ae", 226.421),
            ("If", 246.346),
            ("Ag", 266.271),
            ("Eh", 286.197),
            ("Ai", 318.077),
            ("Dj", 338.002),
            ("Ak", 369.883),
            ("Il", 389.808),
            ("Im", 409.733),
            ("An", 441.614),
            ("Io", 461.539),
            ("Ip", 481.464),
            ("Iq", 512.349),
            ("Ar", 532.274),
            ("At", 597.100),
            ("Av", 651.867),
        ],
    );
}

#[test]
fn addvspace_before_lists_and_headings_and_after_other_skips() {
    check(
        "Aa
\\addvspace{20pt}
\\begin{itemize}\\item Ib\\end{itemize}
Ac\\par
\\addvspace{3pt}
\\begin{itemize}\\item Id\\end{itemize}
Ae
\\addvspace{20pt}
\\section{Sf}
Ag
\\addvspace{5pt}
\\section{Sh}
Ai
\\begin{center}Cj\\end{center}
\\addvspace{20pt}
Ak
\\begin{itemize}\\item Il\\end{itemize}
\\vspace{5pt}\\addvspace{20pt}
Am
\\begin{itemize}\\item In\\end{itemize}
\\addvspace{20pt}\\vspace{5pt}
Ao
\\[ x=1 \\]
\\addvspace{20pt}
Ap
",
        &[
            ("Aa", 134.765),
            ("Ib", 166.645),
            ("Ac", 188.563),
            ("Id", 210.481),
            ("Ae", 232.399),
            ("Ag", 292.078),
            ("Ai", 346.844),
            ("Cj", 366.770),
            ("Ak", 398.650),
            ("Il", 418.575),
            ("Am", 463.407),
            ("In", 483.332),
            ("Ao", 520.194),
            ("Ap", 564.030),
        ],
    );
}
