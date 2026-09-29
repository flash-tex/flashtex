//! Rule 15e delimiters of a generalized fraction below text style
//! (`\binom`, `\choose`, `\atopwithdelims()` in a subscript).
//!
//! tex.web §707 (`var_delimiter`) looks for the *small* variant in the
//! current size's font and then in every larger size's font, down to the
//! text size, before it turns to the large variant. In script style the
//! fraction wants `\delim2` of `cmsy7` (1.157144 × 7pt = 8.1pt); `cmr7`'s
//! `(` is 7pt tall, so pdfTeX takes `cmr10`'s 10pt `(`, not the next `cmex`
//! glyph and not the 7pt one. math-layout used to stop at the current size
//! and set the 7pt parenthesis: the box 3pt short and 1.53pt narrow.
//!
//! The numbers are pdfTeX 3.141592653 (TeX Live 2026) `\showbox` of
//! `\hbox{$\scriptstyle{a\atopwithdelims() b}$}` and friends in a 10pt
//! `article` (kernel fonts: cmex10 `sfixed`).

use flashtex_math_layout::{Atom, CmMathMetrics, MathList, Style, layout};

const TOLERANCE_PT: f64 = 1e-4;

fn genfrac(left: char, right: char) -> Atom {
    Atom::genfrac(MathList::new(vec![Atom::ord('a')]), MathList::new(vec![Atom::ord('b')]), Some(0.0), Some(left), Some(right))
}

fn check(what: &str, atom: Atom, style: Style, want: (f64, f64, f64)) {
    let m = CmMathMetrics::latex_10pt();
    let b = layout(&MathList::new(vec![atom]), style, &m);
    let got = (b.height, b.depth, b.width);
    assert!(
        (got.0 - want.0).abs() < TOLERANCE_PT && (got.1 - want.1).abs() < TOLERANCE_PT && (got.2 - want.2).abs() < TOLERANCE_PT,
        "{what}: got {got:?}, pdfTeX {want:?}"
    );
}

#[test]
fn script_style_parentheses_come_from_the_text_size_font() {
    // \hbox(6.75+3.25)x11.64995: cmr10 `(`/`)` (3.8889pt wide each).
    check("\\scriptstyle{a\\atopwithdelims() b}", genfrac('(', ')'), Style::SCRIPT, (6.75, 3.25, 11.64995));
    // \hbox(6.75+3.25)x9.42772: cmr10 `[`/`]`.
    check("\\scriptstyle{a\\atopwithdelims[] b}", genfrac('[', ']'), Style::SCRIPT, (6.75, 3.25, 9.42772));
}

#[test]
fn scriptscript_style_walks_up_to_the_text_size_font() {
    // \hbox(6.25+3.75)x11.64995: `\delim2` of cmsy5 is past cmr5's and
    // cmr7's `(`, so cmr10's, centred on cmsy5's axis.
    check("\\scriptscriptstyle{a\\atopwithdelims() b}", genfrac('(', ')'), Style::SCRIPT_SCRIPT, (6.25, 3.75, 11.64995));
}

#[test]
fn a_small_enough_script_delimiter_stays_at_script_size() {
    // `\scriptstyle\left(x\right)`: \hbox(5.25+1.75)x10.78476, each
    // parenthesis cmr7's (3.12502pt): the walk stops at the first fit.
    check(
        "\\scriptstyle\\left(x\\right)",
        Atom::left_right(Some('('), Some(')'), MathList::new(vec![Atom::ord('x')])),
        Style::SCRIPT,
        (5.25, 1.75, 10.78476),
    );
}
