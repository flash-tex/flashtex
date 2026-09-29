//! `\check@icl`/`\check@icr` in an italic (`plain`) amsthm body are
//! decided against the body's italic font.
//!
//! `plain`'s `\thm@bodyfont` is `\itshape`. The compiler kept that only in
//! the run's `italic` flag and left the NFSS state upright, so
//! `maybe_ic_upright` saw an upright font on both sides of every text font
//! command in the body. The result was a missing `\/` before `\emph{..}` and
//! `\textup{..}`, and an extra `\/` after `\textit{..}` and `\emph{..}`.
//!
//! Oracle: pdflatex (TeX Live 2026, pdfTeX 1.40.29), `\showoutput` of the
//! body below in an 11pt article with amsthm. Every kern the shipped line
//! holds around the four commands:
//!
//! ```text
//! \OT1/cmr/m/it/10.95 y
//! \kern 0.96877                 % \sw@slant before \emph{prime}
//! \glue 3.91763 ...
//! \OT1/cmr/m/n/10.95 p r i m e  % no kern after
//! ...
//! \OT1/cmr/bx/it/10.95 o ... d  % \textbf{odd}: bold italic, no kern either side
//! \OT1/cmr/m/it/10.95 o r
//! \kern 1.17863                 % \sw@slant before \textup{two}
//! \glue 3.91763 ...
//! \OT1/cmr/m/n/10.95 t w o      % no kern after
//! ...
//! \OT1/cmr/m/it/10.95 a l s o   % \textit{also}: no kern either side
//! ```
//!
//! The pipeline turns `italic_correction.before`/`.after` into those kerns
//! (`adapter::items_from_inlines_styled`). This reaches users at the next
//! `vendor/compiler` re-pin.

use flashtex_compiler::parser::{parse, Block, Inline, ItalicCorrection};

#[test]
fn text_font_commands_in_an_italic_theorem_body_take_pdflatex_corrections() {
    let source = r"\documentclass[11pt]{article}
\usepackage{amsthm}
\newtheorem{theorem}{Theorem}
\begin{document}
\begin{theorem}
Every \emph{prime} number is \textbf{odd} or \textup{two} and \textit{also} here.
\end{theorem}
\end{document}
";
    let parsed = parse(source);
    let runs: Vec<(String, ItalicCorrection, bool)> = parsed
        .blocks
        .iter()
        .flat_map(|block| match block {
            Block::Paragraph(content) => content.as_slice(),
            _ => &[],
        })
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } => Some((text.clone(), style.italic_correction, style.italic)),
            _ => None,
        })
        .collect();
    let run = |word: &str| {
        runs.iter()
            .find(|(text, ..)| text == word)
            .unwrap_or_else(|| panic!("no run {word:?} in {runs:?}"))
    };
    let ic = |before, after| ItalicCorrection { before, after };
    // `\emph` in the italic body is upright: `\check@icl` corrects the `y`
    // before it, and `\check@icr` adds nothing since the font after the
    // group is slanted.
    assert_eq!(run("prime").1, ic(true, false), "{runs:?}");
    assert!(!run("prime").2, "\\emph in an italic body is upright");
    // `\textbf` keeps the shape: bold italic, slanted, so neither check fires.
    assert_eq!(run("odd").1, ic(false, false), "{runs:?}");
    assert!(run("odd").2, "\\textbf in an italic body is bold italic");
    assert_eq!(run("two").1, ic(true, false), "{runs:?}");
    assert_eq!(run("also").1, ic(false, false), "{runs:?}");
    // The body text itself carries no correction marks.
    assert_eq!(run("number").1, ic(false, false));
}

#[test]
fn an_upright_theorem_body_keeps_its_corrections() {
    // `definition`: upright body, so `\emph{word}` is italic and `\check@icr`
    // adds `\/` after it (pdflatex: `\kern 1.1315` after the cmti `d`, then the
    // upright space, before `is`).
    let source = r"\documentclass[11pt]{article}
\usepackage{amsthm}
\theoremstyle{definition}
\newtheorem{definition}{Definition}
\begin{document}
\begin{definition}
A \emph{word} is a string.
\end{definition}
\end{document}
";
    let parsed = parse(source);
    let word = parsed
        .blocks
        .iter()
        .flat_map(|block| match block {
            Block::Paragraph(content) => content.as_slice(),
            _ => &[],
        })
        .find_map(|inline| match inline {
            Inline::Text { text, style, .. } if text == "word" => Some(style.italic_correction),
            _ => None,
        })
        .expect("a run `word`");
    assert_eq!(word, ItalicCorrection { before: false, after: true });
}
