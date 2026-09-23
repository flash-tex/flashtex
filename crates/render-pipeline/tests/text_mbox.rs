//! Kernel `\mbox` (and amsmath's text-mode `\text`) is one `\hbox`, in
//! running text and in math.
//!
//! Text-mode `\mbox` was rejected ("\mbox is a math command") and its
//! argument set as loose, breakable words or dropped; `\text` was spliced
//! into the paragraph. latex.ltx `\mbox{#1}` is `\leavevmode\hbox{#1}`: the
//! line never breaks inside it, its blanks keep their natural width, and a
//! blank just inside a brace is glue in the box. In math `\mbox` is set in
//! the text font at the *text* size whatever the style (`x_{\mbox{max}}`),
//! unlike amsmath's `\text`, and so is a font command (`\textrm`,
//! `\textit`) without amstext, whose `\nfss@text` is `{\mbox{#1}}` and adds
//! no italic correction.
//!
//! ## Oracle
//!
//! pdfTeX 1.40.29 (TeX Live 2026, `/Library/TeX/texbin/pdflatex`, two runs)
//! on each probe: the origin of each word's first glyph from the PDF's
//! content stream (`tools/visual-oracle/pdftext.py`), in bp, baseline from
//! the page top. pdflatex is an oracle only and never runs in the product
//! path. Measured with this commit: every word within 0.005 bp except the
//! `\textit{it}` subscript of [`EDGES`], whose baseline is 0.25 bp high
//! (the italic text face's height, not the box).

mod common;

const TOL: f64 = 0.5;

/// Boxes in running text and in math, and a box that has to move to the
/// next line whole.
const BREAKS: &str = r#"\documentclass{article}
\pagestyle{empty}
\begin{document}
Some text with an \mbox{unbreakable box} in it and \mbox{another} one here.
Then math $a\mbox{ and }b$ and $x_{\mbox{max}}$ follow here.
A long line that must break somewhere near here so we test that the box
does not \mbox{split across two lines ever} even when the line is full.
\end{document}
"#;

const BREAKS_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("S", "CMR10", 148.712, 134.765), // Some
    ("t", "CMR10", 175.315, 134.765), // text
    ("w", "CMR10", 196.097, 134.765), // with
    ("a", "CMR10", 218.827, 134.765), // an
    ("u", "CMR10", 232.701, 134.765), // unbreakable
    ("b", "CMR10", 288.078, 134.765), // box
    ("i", "CMR10", 307.200, 134.765), // in
    ("i", "CMR10", 318.860, 134.765), // it
    ("a", "CMR10", 328.860, 134.765), // and
    ("a", "CMR10", 348.259, 134.765), // another
    ("o", "CMR10", 384.854, 134.765), // one
    ("h", "CMR10", 403.155, 134.765), // here.
    ("T", "CMR10", 428.738, 134.765), // Then
    ("m", "CMR10", 454.788, 134.765), // math
    ("a", "CMMI10", 133.768, 146.720), // a
    ("a", "CMR10", 142.352, 146.720), // and
    ("b", "CMMI10", 161.731, 146.720), // b
    ("a", "CMR10", 170.201, 146.720), // and
    ("x", "CMMI10", 190.447, 146.720), // x
    ("m", "CMR10", 196.143, 148.214), // max
    ("f", "CMR10", 219.380, 146.720), // follow
    ("h", "CMR10", 249.033, 146.720), // here.
    ("A", "CMR10", 277.156, 146.720), // A
    ("l", "CMR10", 288.822, 146.720), // long
    ("l", "CMR10", 311.282, 146.720), // line
    ("t", "CMR10", 330.984, 146.720), // that
    ("m", "CMR10", 353.444, 146.720), // must
    ("b", "CMR10", 379.010, 146.720), // break
    ("s", "CMR10", 407.309, 146.720), // somewhere
    ("n", "CMR10", 458.630, 146.720), // near
    ("h", "CMR10", 133.768, 158.675), // here
    ("s", "CMR10", 155.458, 158.675), // so
    ("w", "CMR10", 167.765, 158.675), // we
    ("t", "CMR10", 182.506, 158.675), // test
    ("t", "CMR10", 202.009, 158.675), // that
    ("t", "CMR10", 223.671, 158.675), // the
    ("b", "CMR10", 240.906, 158.675), // box
    ("d", "CMR10", 260.078, 158.675), // does
    ("n", "CMR10", 282.627, 158.675), // not
    ("s", "CMR10", 300.415, 158.675), // split
    ("a", "CMR10", 322.607, 158.675), // across
    ("t", "CMR10", 352.085, 158.675), // two
    ("l", "CMR10", 370.896, 158.675), // lines
    ("e", "CMR10", 393.650, 158.675), // ever
    ("e", "CMR10", 414.784, 158.675), // even
    ("w", "CMR10", 437.551, 158.675), // when
    ("t", "CMR10", 463.641, 158.675), // the
    ("l", "CMR10", 133.768, 170.630), // line
    ("i", "CMR10", 152.583, 170.630), // is
    ("f", "CMR10", 162.608, 170.630), // full.
];

/// Blanks at the box's edges, an empty box, nested commands and math in a
/// box, kernel font commands in a subscript.
const EDGES: &str = r#"\documentclass{article}
\pagestyle{empty}
\begin{document}
Edge \mbox{ lead} and \mbox{trail } and \mbox{} empty, \textbf{bold \mbox{inner box} x}
and \mbox{\emph{emph} word} and \mbox{math $x^2+y$ inside} then more.
Subscripts $x_{\textrm{max}}$ and $y_{\textit{it}}$ and $\mbox{A}^2$ and $\hbox{B}_1$.
\mbox{Start} of a sentence. Filling words to make this paragraph long enough to
wrap across several lines so that spacing is measured, \mbox{here is a long box} end.

\mbox{Alone}
\end{document}
"#;

const EDGES_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("E", "CMR10", 148.712, 134.765), // Edge
    ("l", "CMR10", 177.570, 134.765), // lead
    ("a", "CMR10", 199.087, 134.765), // and
    ("t", "CMR10", 218.944, 134.765), // trail
    ("a", "CMR10", 244.361, 134.765), // and
    ("e", "CMR10", 268.024, 134.765), // empty,
    ("b", "CMBX10", 301.018, 134.765), // bold
    ("i", "CMBX10", 327.351, 134.765), // inner
    ("b", "CMBX10", 357.059, 134.765), // box
    ("x", "CMBX10", 379.572, 134.765), // x
    ("a", "CMR10", 389.424, 134.765), // and
    ("e", "CMTI10", 409.282, 134.765), // emph
    ("w", "CMR10", 436.280, 134.765), // word
    ("a", "CMR10", 461.431, 134.765), // and
    ("m", "CMR10", 133.768, 146.720), // math
    ("x", "CMMI10", 159.778, 146.720), // x
    ("2", "CMR7", 165.476, 143.104), // 2
    ("+", "CMR10", 172.159, 146.720), // +
    ("y", "CMMI10", 182.120, 146.720), // y
    ("i", "CMR10", 190.680, 146.720), // inside
    ("t", "CMR10", 219.060, 146.720), // then
    ("m", "CMR10", 241.839, 146.720), // more.
    ("S", "CMR10", 270.912, 146.720), // Subscripts
    ("x", "CMMI10", 319.291, 146.720), // x
    ("m", "CMR10", 324.986, 148.214), // max
    ("a", "CMR10", 347.435, 146.720), // and
    ("y", "CMMI10", 366.894, 146.720), // y
    ("i", "CMTI10", 371.780, 149.817), // it
    ("a", "CMR10", 382.053, 146.720), // and
    ("A", "CMR10", 401.512, 146.720), // A
    ("2", "CMR7", 408.985, 142.375), // 2
    ("a", "CMR10", 416.864, 146.720), // and
    ("B", "CMR10", 436.323, 146.720), // B
    ("1", "CMR7", 443.382, 148.214), // 1
    (".", "CMR10", 447.851, 146.720), // .
    ("S", "CMR10", 455.311, 146.720), // Start
    ("o", "CMR10", 133.768, 158.675), // of
    ("a", "CMR10", 144.803, 158.675), // a
    ("s", "CMR10", 152.793, 158.675), // sentence.
    ("F", "CMR10", 196.189, 158.675), // Filling
    ("w", "CMR10", 227.288, 158.675), // words
    ("t", "CMR10", 255.561, 158.675), // to
    ("m", "CMR10", 267.415, 158.675), // make
    ("t", "CMR10", 293.114, 158.675), // this
    ("p", "CMR10", 312.229, 158.675), // paragraph
    ("l", "CMR10", 359.573, 158.675), // long
    ("e", "CMR10", 380.847, 158.675), // enough
    ("t", "CMR10", 414.842, 158.675), // to
    ("w", "CMR10", 426.706, 158.675), // wrap
    ("a", "CMR10", 451.329, 158.675), // across
    ("s", "CMR10", 133.768, 170.630), // several
    ("l", "CMR10", 166.510, 170.630), // lines
    ("s", "CMR10", 189.255, 170.630), // so
    ("t", "CMR10", 201.483, 170.630), // that
    ("s", "CMR10", 223.076, 170.630), // spacing
    ("i", "CMR10", 258.551, 170.630), // is
    ("m", "CMR10", 268.565, 170.630), // measured,
    ("h", "CMR10", 315.700, 170.630), // here
    ("i", "CMR10", 337.310, 170.630), // is
    ("a", "CMR10", 347.324, 170.630), // a
    ("l", "CMR10", 355.633, 170.630), // long
    ("b", "CMR10", 377.216, 170.630), // box
    ("e", "CMR10", 396.308, 170.630), // end.
    ("A", "CMR10", 148.712, 182.585), // Alone
];

/// amsmath: `\text` scales in a subscript where `\mbox` does not, and in
/// text mode it is `\mbox`.
const AMS: &str = r#"\documentclass{article}
\usepackage{amsmath}
\pagestyle{empty}
\begin{document}
Subscripts $x_{\textrm{max}}$ and $y_{\text{it}}$ and $x_{\mbox{max}}$ and
$\mbox{A}^{\mbox{b}}$. Text mode \text{some text} and \text{ lead} and \text{trail }x.
A long line of words to wrap that ends with a text-mode box across the break
point \text{here is a long text box} end of it all.
\end{document}
"#;

const AMS_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("S", "CMR10", 148.712, 136.065), // Subscripts
    ("x", "CMMI10", 196.921, 136.065), // x
    ("m", "CMR7", 202.619, 137.560), // max
    ("a", "CMR10", 221.061, 136.065), // and
    ("y", "CMMI10", 240.351, 136.065), // y
    ("i", "CMR7", 245.238, 137.560), // it
    ("a", "CMR10", 254.347, 136.065), // and
    ("x", "CMMI10", 273.637, 136.065), // x
    ("m", "CMR10", 279.334, 137.560), // max
    ("a", "CMR10", 301.616, 136.065), // and
    ("A", "CMR10", 320.906, 136.065), // A
    ("b", "CMR10", 328.381, 131.720), // b
    (".", "CMR10", 334.414, 136.065), // .
    ("T", "CMR10", 341.585, 136.065), // Text
    ("m", "CMR10", 364.751, 136.065), // mode
    ("s", "CMR10", 391.514, 136.065), // some
    ("t", "CMR10", 416.481, 136.065), // text
    ("a", "CMR10", 437.153, 136.065), // and
    ("l", "CMR10", 459.770, 136.065), // lead
    ("a", "CMR10", 133.768, 148.020), // and
    ("t", "CMR10", 152.709, 148.020), // trail
    ("x", "CMR10", 174.330, 148.020), // x.
    ("A", "CMR10", 186.640, 148.020), // A
    ("l", "CMR10", 197.001, 148.020), // long
    ("l", "CMR10", 218.155, 148.020), // line
    ("o", "CMR10", 236.542, 148.020), // of
    ("w", "CMR10", 247.467, 148.020), // words
    ("t", "CMR10", 275.620, 148.020), // to
    ("w", "CMR10", 287.375, 148.020), // wrap
    ("t", "CMR10", 311.878, 148.020), // that
    ("e", "CMR10", 333.033, 148.020), // ends
    ("w", "CMR10", 355.349, 148.020), // with
    ("a", "CMR10", 377.621, 148.020), // a
    ("t", "CMR10", 385.491, 148.020), // text-mode
    ("b", "CMR10", 432.660, 148.020), // box
    ("a", "CMR10", 451.324, 148.020), // across
    ("t", "CMR10", 133.768, 159.976), // the
    ("b", "CMR10", 150.923, 159.976), // break
    ("p", "CMR10", 178.355, 159.976), // point
    ("h", "CMR10", 204.366, 159.976), // here
    ("i", "CMR10", 225.976, 159.976), // is
    ("a", "CMR10", 236.000, 159.976), // a
    ("l", "CMR10", 244.299, 159.976), // long
    ("t", "CMR10", 265.882, 159.976), // text
    ("b", "CMR10", 286.644, 159.976), // box
    ("e", "CMR10", 305.736, 159.976), // end
    ("o", "CMR10", 324.552, 159.976), // of
    ("i", "CMR10", 335.895, 159.976), // it
    ("a", "CMR10", 345.865, 159.976), // all.
];

#[test]
fn mbox_breaks_and_math_match_pdftex() {
    if !common::lm_available() {
        eprintln!("SKIP text_mbox: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(BREAKS, BREAKS_GLYPHS, TOL);
}

#[test]
fn mbox_edges_and_font_commands_match_pdftex() {
    if !common::lm_available() {
        eprintln!("SKIP text_mbox: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(EDGES, EDGES_GLYPHS, TOL);
}

#[test]
fn amsmath_text_and_mbox_match_pdftex() {
    if !common::lm_available() {
        eprintln!("SKIP text_mbox: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(AMS, AMS_GLYPHS, TOL);
}
