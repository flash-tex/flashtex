//! A bibliography read from a `.bbl` by `\bibliography` is set as the same
//! `thebibliography` written inline, under natbib too.
//!
//! Three things differed, all found on a `plainnat` document whose `.bbl`
//! BibTeX wrote:
//!
//! - natbib's author-year geometry (no label, `\leftmargin\bibhang`,
//!   `\itemindent-\leftmargin`) was chosen by reading `\usepackage{natbib}`
//!   from the *file* the list is in; a `.bbl` never loads natbib, so its
//!   entries took the class's `[n]` label width, 15.5 bp to the right. The
//!   package is now read once for the document.
//! - `\bibliographystyle{plainnat}` makes every citation `[Knuth, 1984]`
//!   (natbib's `\bibstyle@plainnat`, compiler `natbib::Options::bibstyle`).
//! - `60:\penalty0 3461--3466` (every `plainnat` article entry): the blank
//!   after `\penalty0` is the number's optional space, not glue, and the
//!   dash may end the line (TeX's empty discretionary after a hyphen char,
//!   §1039, which applies to a word with no letters too). A space was set
//!   after `60:` and the entry broke there instead.
//!
//! ## Oracle
//!
//! pdfTeX 1.40.29 (TeX Live 2026, `/Library/TeX/texbin/pdflatex` and
//! `bibtex`, pdflatex twice after bibtex) on each probe: the origin of each
//! word's first glyph from the PDF's content stream
//! (`tools/visual-oracle/pdftext.py`), in bp, baseline from the page top.
//! pdflatex and bibtex are oracles only and never run in the product path;
//! the `.bbl` is committed here as BibTeX wrote it. Measured with this
//! commit: every word within 0.005 bp.

mod common;

const TOL: f64 = 0.5;

const MAIN: &str = r#"\documentclass{article}
\usepackage{natbib}
\pagestyle{empty}
\begin{document}
Text \citep{knuth84} and \citet{lamport94}, also \citep{abe}.
\bibliographystyle{plainnat}
\bibliography{refs}
\end{document}
"#;

/// `bibtex main` with `plainnat.bst` (TeX Live 2026) on the `.bib` below it
/// in the probe; the file natbib-style arXiv sources ship.
const BBL: &str = r#"\begin{thebibliography}{3}
\providecommand{\natexlab}[1]{#1}
\providecommand{\url}[1]{\texttt{#1}}
\expandafter\ifx\csname urlstyle\endcsname\relax
  \providecommand{\doi}[1]{doi: #1}\else
  \providecommand{\doi}{doi: \begingroup \urlstyle{rm}\Url}\fi

\bibitem[Abe et~al.(1999)Abe, Rajagopal, Author, and Person]{abe}
Sumiyoshi Abe, A.~K. Rajagopal, Some~Other Author, and Fourth Person.
\newblock Quantum entanglement inferred by the principle of maximum nonadditive
  entropy with a rather long title that wraps.
\newblock \emph{Physical Review A}, 60:\penalty0 3461--3466, 1999.

\bibitem[Knuth(1984)]{knuth84}
Donald~E. Knuth.
\newblock \emph{The {\TeX}book}.
\newblock Addison-Wesley, 1984.

\bibitem[Lamport(1994)]{lamport94}
Leslie Lamport.
\newblock \emph{{\LaTeX}: A Document Preparation System}.
\newblock Addison-Wesley, second edition, 1994.

\end{thebibliography}
"#;

const BBL_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("T", "CMR10", 148.712, 134.765), // Text
    ("[", "CMR10", 171.958, 134.765), // [Knuth,
    ("1", "CMR10", 208.761, 134.765), // 1984]
    ("a", "CMR10", 234.781, 134.765), // and
    ("L", "CMR10", 254.150, 134.765), // Lamport
    ("[", "CMR10", 295.550, 134.765), // [1994],
    ("a", "CMR10", 327.095, 134.765), // also
    ("[", "CMR10", 347.082, 134.765), // [Abe
    ("e", "CMR10", 370.881, 134.765), // et
    ("a", "CMR10", 382.500, 134.765), // al.,
    ("1", "CMR10", 399.112, 134.765), // 1999].
    ("R", "CMBX12", 133.768, 167.711), // References
    ("S", "CMR10", 133.768, 189.531), // Sumiyoshi
    ("A", "CMR10", 183.062, 189.531), // Abe,
    ("A", "CMR10", 208.913, 189.531), // A.
    ("K", "CMR10", 224.114, 189.531), // K.
    ("R", "CMR10", 239.592, 189.531), // Rajagopal,
    ("S", "CMR10", 291.864, 189.531), // Some
    ("O", "CMR10", 320.072, 189.531), // Other
    ("A", "CMR10", 350.521, 189.531), // Author,
    ("a", "CMR10", 389.959, 189.531), // and
    ("F", "CMR10", 410.972, 189.531), // Fourth
    ("P", "CMR10", 445.429, 189.531), // Person.
    ("Q", "CMR10", 143.731, 201.487), // Quantum
    ("e", "CMR10", 189.557, 201.487), // entanglement
    ("i", "CMR10", 252.261, 201.487), // inferred
    ("b", "CMR10", 290.395, 201.487), // by
    ("t", "CMR10", 305.503, 201.487), // the
    ("p", "CMR10", 323.923, 201.487), // principle
    ("o", "CMR10", 366.181, 201.487), // of
    ("m", "CMR10", 378.800, 201.487), // maximum
    ("n", "CMR10", 426.561, 201.487), // nonadditive
    ("e", "CMR10", 143.731, 213.442), // entropy
    ("w", "CMR10", 181.230, 213.442), // with
    ("a", "CMR10", 205.145, 213.442), // a
    ("r", "CMR10", 214.660, 213.442), // rather
    ("l", "CMR10", 245.826, 213.442), // long
    ("t", "CMR10", 268.634, 213.442), // title
    ("t", "CMR10", 290.879, 213.442), // that
    ("w", "CMR10", 313.687, 213.442), // wraps.
    ("P", "CMTI10", 350.047, 213.442), // Physical
    ("R", "CMTI10", 390.260, 213.442), // Review
    ("A", "CMTI10", 425.134, 213.442), // A,
    ("6", "CMR10", 439.839, 213.442), // 60:3461--
    ("3", "CMR10", 143.731, 225.397), // 3466,
    ("1", "CMR10", 169.741, 225.397), // 1999.
    ("D", "CMR10", 133.768, 245.322), // Donald
    ("E", "CMR10", 168.497, 245.322), // E.
    ("K", "CMR10", 181.372, 245.322), // Knuth.
    ("T", "CMTI10", 216.503, 245.322), // The
    ("T", "CMTI10", 236.874, 245.322), // T
    ("E", "CMTI10", 242.302, 247.467), // E
    ("X", "CMTI10", 247.787, 245.322), // Xbook.
    ("A", "CMR10", 280.706, 245.322), // Addison-Wesley,
    ("1", "CMR10", 355.264, 245.322), // 1984.
    ("L", "CMR10", 133.768, 265.248), // Leslie
    ("L", "CMR10", 163.036, 265.248), // Lamport.
    ("L", "CMTI10", 212.493, 265.248), // L
    ("A", "CMTI7", 215.072, 263.168), // A
    ("T", "CMTI10", 219.583, 265.248), // T
    ("E", "CMTI10", 225.014, 267.392), // E
    ("X", "CMTI10", 230.499, 265.248), // X:
    ("A", "CMTI10", 248.123, 265.248), // A
    ("D", "CMTI10", 260.390, 265.248), // Document
    ("P", "CMTI10", 308.918, 265.248), // Preparation
    ("S", "CMTI10", 364.322, 265.248), // System.
    ("A", "CMR10", 406.249, 265.248), // Addison-Wesley,
    ("s", "CMR10", 143.731, 277.203), // second
    ("e", "CMR10", 175.884, 277.203), // edition,
    ("1", "CMR10", 211.868, 277.203), // 1994.
];

/// `\penalty<number>` with and without a blank before the number.
const PENALTIES: &str = r#"\documentclass{article}
\pagestyle{empty}
\begin{document}
Volume 60:\penalty0 3461--3466, 1999 and x\penalty100 y and z\penalty-50 w.

Volume 60:\penalty0 3461 and 61:\penalty 0 12 end.
\end{document}
"#;

const PENALTY_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("V", "CMR10", 148.712, 134.765), // Volume
    ("6", "CMR10", 184.688, 134.765), // 60:3461--3466,
    ("1", "CMR10", 248.335, 134.765), // 1999
    ("a", "CMR10", 271.588, 134.765), // and
    ("x", "CMR10", 290.957, 134.765), // xy
    ("a", "CMR10", 304.791, 134.765), // and
    ("z", "CMR10", 324.170, 134.765), // zw.
    ("V", "CMR10", 148.712, 146.720), // Volume
    ("6", "CMR10", 184.688, 146.720), // 60:3461
    ("a", "CMR10", 220.661, 146.720), // and
    ("6", "CMR10", 240.040, 146.720), // 61:12
    ("e", "CMR10", 266.051, 146.720), // end.
];

/// A bibliography entry and a paragraph that may break after `--`.
const DASHES: &str = r#"\documentclass{article}
\pagestyle{empty}
\begin{document}
\begin{thebibliography}{9}
\bibitem{a} Some words of a bibliography entry that run on to fill this line, with
60:\penalty0 3461--3466, 1999.
\bibitem{b} Entropy with a rather long title that wraps. \newblock \emph{Physical Review A}, 60:\penalty0 3461--3466, 1999.
\end{thebibliography}
Some words of a paragraph that run on to fill up this line up to the pages ab
3461--3466, and more.
\end{document}
"#;

const DASH_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("R", "CMBX12", 133.768, 134.765), // References
    ("[", "CMR10", 133.768, 156.586), // [1]
    ("S", "CMR10", 149.266, 156.586), // Some
    ("w", "CMR10", 176.746, 156.586), // words
    ("o", "CMR10", 206.254, 156.586), // of
    ("a", "CMR10", 218.514, 156.586), // a
    ("b", "CMR10", 227.739, 156.586), // bibliography
    ("e", "CMR10", 286.243, 156.586), // entry
    ("t", "CMR10", 313.195, 156.586), // that
    ("r", "CMR10", 335.705, 156.586), // run
    ("o", "CMR10", 354.912, 156.586), // on
    ("t", "CMR10", 369.662, 156.586), // to
    ("fi", "CMR10", 382.762, 156.586), // fill
    ("t", "CMR10", 398.067, 156.586), // this
    ("l", "CMR10", 418.407, 156.586), // line,
    ("w", "CMR10", 441.136, 156.586), // with
    ("6", "CMR10", 464.752, 156.586), // 60:
    ("3", "CMR10", 149.266, 168.541), // 3461--3466,
    ("1", "CMR10", 200.183, 168.541), // 1999.
    ("[", "CMR10", 133.768, 188.466), // [2]
    ("E", "CMR10", 149.266, 188.466), // Entropy
    ("w", "CMR10", 188.501, 188.466), // with
    ("a", "CMR10", 211.798, 188.466), // a
    ("r", "CMR10", 220.705, 188.466), // rather
    ("l", "CMR10", 251.253, 188.466), // long
    ("t", "CMR10", 273.444, 188.466), // title
    ("t", "CMR10", 295.080, 188.466), // that
    ("w", "CMR10", 317.271, 188.466), // wraps.
    ("P", "CMTI10", 351.799, 188.466), // Physical
    ("R", "CMTI10", 391.443, 188.466), // Review
    ("A", "CMTI10", 425.749, 188.466), // A,
    ("6", "CMR10", 439.847, 188.466), // 60:3461--
    ("3", "CMR10", 149.266, 200.421), // 3466,
    ("1", "CMR10", 175.276, 200.421), // 1999.
    ("S", "CMR10", 133.768, 222.339), // Some
    ("w", "CMR10", 160.740, 222.339), // words
    ("o", "CMR10", 189.740, 222.339), // of
    ("a", "CMR10", 201.492, 222.339), // a
    ("p", "CMR10", 210.199, 222.339), // paragraph
    ("t", "CMR10", 258.261, 222.339), // that
    ("r", "CMR10", 280.252, 222.339), // run
    ("o", "CMR10", 298.951, 222.339), // on
    ("t", "CMR10", 313.193, 222.339), // to
    ("fi", "CMR10", 325.785, 222.339), // fill
    ("u", "CMR10", 340.582, 222.339), // up
    ("t", "CMR10", 355.378, 222.339), // this
    ("l", "CMR10", 375.211, 222.339), // line
    ("u", "CMR10", 394.434, 222.339), // up
    ("t", "CMR10", 409.231, 222.339), // to
    ("t", "CMR10", 421.813, 222.339), // the
    ("p", "CMR10", 439.376, 222.339), // pages
    ("a", "CMR10", 466.966, 222.339), // ab
    ("3", "CMR10", 133.768, 234.294), // 3461--3466,
    ("a", "CMR10", 184.685, 234.294), // and
    ("m", "CMR10", 204.064, 234.294), // more.
];

#[test]
fn a_bbl_bibliography_matches_pdftex() {
    if !common::lm_available() {
        eprintln!("SKIP bbl_bibliography: Latin Modern not installed");
        return;
    }
    let r = common::render_docs(&[("main.tex", MAIN), ("main.bbl", BBL)], "main.tex");
    assert_eq!(r.v2.pages.len(), 1);
    let mut actual: Vec<(String, f64, f64, bool)> = common::painted_glyphs(&r).into_iter().map(|(t, x, y)| (t, x, y, false)).collect();
    let mut misses = Vec::new();
    for &(text, font, x, y) in BBL_GLYPHS {
        let hit = actual
            .iter()
            .position(|(t, ax, ay, used)| !used && t == text && (ax - x).abs() <= TOL && (ay - y).abs() <= TOL);
        match hit {
            Some(i) => actual[i].3 = true,
            None => misses.push(format!("{text:?} ({font} at {x}, {y})")),
        }
    }
    assert!(misses.is_empty(), "{} of {} pdfTeX glyphs unmatched:\n{}", misses.len(), BBL_GLYPHS.len(), misses.join("\n"));
}

#[test]
fn a_penalty_eats_the_blank_after_its_number() {
    if !common::lm_available() {
        eprintln!("SKIP bbl_bibliography: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(PENALTIES, PENALTY_GLYPHS, TOL);
}

#[test]
fn a_line_may_break_after_a_dash_in_a_number_range() {
    if !common::lm_available() {
        eprintln!("SKIP bbl_bibliography: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(DASHES, DASH_GLYPHS, TOL);
}
