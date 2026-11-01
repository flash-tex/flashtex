//! letter.cls at 10pt with no `\address` and no `\signature`: the five
//! baselines of the minimal letter, against pdflatex.
//!
//! Source:
//!
//! ```tex
//! \documentclass{letter}
//! \date{January 1, 1970}
//! \begin{document}
//! \begin{letter}{Addr}
//! \opening{Dear X,}
//! Body
//! \closing{Yours}
//! \end{letter}
//! \end{document}
//! ```
//!
//! (`\date` pins the frozen-epoch `\today` the oracle row was measured with,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 pdflatex min.tex`; the string --
//! and its metrics -- are identical either way.)
//!
//! Measured with pdflatex (TeX Live 2026,
//! `pdfTeX 3.141592653-2.6-1.40.29`) via
//! `tools/visual-oracle/pdftext.py`, x and baseline in bp from the page's
//! left and top edges:
//!
//! ```text
//! 408.504 x 305.351  January 1, 1970  (the date, `{\raggedleft\@date\par}`)
//! 134.144 x 338.228  Addr             (the recipient)
//! 134.144 x 364.131  Dear X,          (the salutation)
//! 134.144 x 383.060  Body
//! 134.144 x 406.749  Yours            (the closing)
//! ```
//!
//! Two things this pins, each of which failed independently before:
//!
//! 1. **A trailing `\\` keeps its `-\parskip`.** `{\raggedright \toname
//!    \\\toaddress \par}` (letter.cls line 276) ends with `\\`, which under
//!    `\raggedright` is `\@centercr`: `\par` plus `\addvspace{-\parskip}`.
//!    Between two set lines the next paragraph's own `\parskip` cancels it,
//!    but with an empty `\toaddress` no paragraph follows, so `Addr` to
//!    `Dear X,` is 26.0pt -- `\vspace{2\parskip}` minus one `\parskip`,
//!    plus `\parskip` plus interline glue -- not 33.0pt.
//! 2. **An unsigned closing still sets its strut line.**
//!    `\parbox{\indentedwidth}{\raggedright Yours\\[6\medskipamount]
//!    \fromsig\strut}` with both empty ends in a `\strut`-only line
//!    `6\medskipamount` (41.99982pt) below `Yours` (pdflatex `\showoutput`:
//!    `\glue 41.99982` then `\hbox(8.39996+3.60004)` holding a rule). The
//!    strut line keeps the `\lineskip` before the closing box and gives the
//!    `$\vcenter$`ed box its 29.72pt depth, which the page-1 `\@texttop`
//!    shift keys off -- so the absolute height follows once the gaps do.
//!
//! pdflatex is an oracle only and never runs here or in the product.

mod common;

use common::*;
use flashtex_render_pipeline::v1::Capabilities;

/// The minimal letter. `\date` pins the frozen-epoch `\today`.
const SOURCE: &str = "\\documentclass{letter}\n\\date{January 1, 1970}\n\\begin{document}\n\
    \\begin{letter}{Addr}\n\\opening{Dear X,}\nBody\n\\closing{Yours}\n\
    \\end{letter}\n\\end{document}\n";

/// `(word, left edge, baseline)`, read off pdflatex's PDF with
/// `tools/visual-oracle/pdftext.py` (see the module note).
const REFERENCE: &[(&str, f64, f64)] = &[
    ("January", 408.504, 305.351),
    ("Addr", 134.144, 338.228),
    ("Dear", 134.144, 364.131),
    ("Body", 134.144, 383.060),
    ("Yours", 134.144, 406.749),
];

#[track_caller]
fn close(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() <= 0.1,
        "{what}: got {got:.3} bp, pdflatex {want:.3} bp ({:+.3} off, tolerance 0.1)",
        got - want
    );
}

#[test]
fn minimal_letter_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    let words = words_of(&render_one(SOURCE));
    for (text, x, y) in REFERENCE {
        let Some(w) = words.iter().find(|w| w.text == *text) else {
            panic!("{text:?} not set (words: {:?})", words.iter().map(|w| &w.text).collect::<Vec<_>>());
        };
        close(w.x, *x, &format!("left edge of {text:?}"));
        close(w.baseline, *y, &format!("baseline of {text:?}"));
    }
}

/// `\opening` and `\closing` are defined by letter.cls alone. Under plain
/// `article` pdflatex answers `! Undefined control sequence. l.3 \opening`
/// (likewise `\closing`), so the pipeline must report the command -- and
/// must NOT eat the brace group, which real LaTeX typesets after the error.
#[test]
fn letter_commands_are_rejected_under_article() {
    if !lm_available() {
        return;
    }
    let source = "\\documentclass{article}\n\\begin{document}\n\\opening{Dear reader,}\n\\closing{Yours,}\n\\end{document}\n";
    let r = render_one(source);
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    for command in ["\\opening", "\\closing"] {
        assert!(
            v1.diagnostics.iter().any(|d| d.message.contains(command)),
            "{command} drew no diagnostic: {:?}",
            v1.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
    let words = words_of(&r);
    let texts: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    for kept in ["Dear", "reader,", "Yours,"] {
        assert!(texts.contains(&kept), "the argument text was swallowed (words: {texts:?})");
    }
}
