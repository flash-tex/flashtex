//! A kernel `\cite` label is an `\hbox`, and the rest of the kernel
//! citation follows latex.ltx.
//!
//! `\@citex` sets every label as `\@cite@ofmt{\csname b@\@citeb\endcsname}`
//! with `\let\@cite@ofmt\hbox`, an undefined key as `\hbox{\reset@font
//! \bfseries ?}`, and `,\penalty\@m\ ` between labels; `\@cite` puts
//! `[#1, #2]` around them. So a line never breaks inside a label and the
//! blanks of an author-year label (`[Abe and Rajagopal(2001)]`) do not
//! stretch with the line, where they used to (up to 23.9 bp on these
//! probes' lines, and a word moved to another line). The note's `~` is a
//! tie (it broke the line at `p.~5`, while the blank after the note's
//! comma did not break), and the undefined key's `?` is bold. In
//! `thebibliography`, `\@item`'s `\everypar` puts `\penalty\z@` after the
//! label box, so an entry whose label is wider than the line starts its
//! text on the next line.
//!
//! ## Oracle
//!
//! pdfTeX 1.40.29 (TeX Live 2026, `/Library/TeX/texbin/pdflatex`, two runs)
//! on each probe: the origin of each word's first glyph from the PDF's
//! content stream (`tools/visual-oracle/pdftext.py`), in bp, baseline from
//! the page top. pdfTeX's OT1 `\"o` is an accent glyph over `o`, so the
//! word that starts there is not compared. pdflatex is an oracle only and
//! never runs in the product path. Measured with this commit: every word
//! within 0.006 bp.

mod common;

const TOL: f64 = 0.5;

/// Plain author-year labels, one and two keys, a line that has to break
/// near a label, and the bibliography with one label wider than the line.
const PLAIN: &str = r#"\documentclass{article}
\pagestyle{empty}
\begin{document}
Text text text text text text text text text \cite{Abe} and then \cite{Abe,Gross} with
lots of words to fill the line \cite{Gross} and more words to fill up the line so
it breaks near a citation label \cite{AbeyesingheDHW2006-fullSW} and here.
\begin{thebibliography}{3}
\bibitem[Abe and Rajagopal(2001)]{Abe}
Abe, 2001.
\bibitem[Abeyesinghe et~al.(2006)Abeyesinghe, Devetak, Hayden, and
  Winter]{AbeyesingheDHW2006-fullSW}
Abeyesinghe, 2006.
\bibitem[Gross(1999)]{Gross}
Gross, 1999.
\end{thebibliography}
\end{document}
"#;

const PLAIN_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("T", "CMR10", 148.712, 134.765), // Text
    ("t", "CMR10", 172.197, 134.765), // text
    ("t", "CMR10", 193.188, 134.765), // text
    ("t", "CMR10", 214.179, 134.765), // text
    ("t", "CMR10", 235.170, 134.765), // text
    ("t", "CMR10", 256.152, 134.765), // text
    ("t", "CMR10", 277.143, 134.765), // text
    ("t", "CMR10", 298.134, 134.765), // text
    ("t", "CMR10", 319.125, 134.765), // text
    ("[", "CMR10", 340.116, 134.765), // [Abe
    ("a", "CMR10", 363.915, 134.765), // and
    ("R", "CMR10", 383.294, 134.765), // Rajagopal(2001)]
    ("a", "CMR10", 461.428, 134.765), // and
    ("t", "CMR10", 133.768, 146.720), // then
    ("[", "CMR10", 156.547, 146.720), // [Abe
    ("a", "CMR10", 180.346, 146.720), // and
    ("R", "CMR10", 199.725, 146.720), // Rajagopal(2001),
    ("G", "CMR10", 277.720, 146.720), // Gross(1999)]
    ("w", "CMR10", 336.128, 146.720), // with
    ("l", "CMR10", 358.908, 146.720), // lots
    ("o", "CMR10", 377.878, 146.720), // of
    ("w", "CMR10", 389.311, 146.720), // words
    ("t", "CMR10", 417.992, 146.720), // to
    ("fi", "CMR10", 430.255, 146.720), // fill
    ("t", "CMR10", 444.733, 146.720), // the
    ("l", "CMR10", 461.977, 146.720), // line
    ("[", "CMR10", 133.768, 158.675), // [Gross(1999)]
    ("a", "CMR10", 194.934, 158.675), // and
    ("m", "CMR10", 214.383, 158.675), // more
    ("w", "CMR10", 239.403, 158.675), // words
    ("t", "CMR10", 268.065, 158.675), // to
    ("fi", "CMR10", 280.318, 158.675), // fill
    ("u", "CMR10", 294.786, 158.675), // up
    ("t", "CMR10", 309.263, 158.675), // the
    ("l", "CMR10", 326.497, 158.675), // line
    ("s", "CMR10", 345.393, 158.675), // so
    ("i", "CMR10", 357.700, 158.675), // it
    ("b", "CMR10", 367.740, 158.675), // breaks
    ("n", "CMR10", 399.171, 158.675), // near
    ("a", "CMR10", 421.424, 158.675), // a
    ("c", "CMR10", 429.803, 158.675), // citation
    ("l", "CMR10", 466.409, 158.675), // la-
    ("b", "CMR10", 133.768, 170.630), // bel
    ("[", "CMR10", 152.167, 170.630), // [Abeyesinghe
    ("e", "CMR10", 212.558, 170.630), // et
    ("a", "CMR10", 224.178, 170.630), // al.(2006)Abeyesinghe,
    ("D", "CMR10", 322.750, 170.630), // Devetak,
    ("H", "CMR10", 364.403, 170.630), // Hayden,
    ("a", "CMR10", 403.429, 170.630), // and
    ("W", "CMR10", 422.798, 170.630), // Winter]
    ("a", "CMR10", 461.433, 170.630), // and
    ("h", "CMR10", 133.768, 182.585), // here.
    ("R", "CMBX12", 133.768, 215.531), // References
    ("[", "CMR10", 133.768, 237.352), // [Abe
    ("a", "CMR10", 157.567, 237.352), // and
    ("R", "CMR10", 176.936, 237.352), // Rajagopal(2001)]
    ("A", "CMR10", 256.504, 237.352), // Abe,
    ("2", "CMR10", 280.303, 237.352), // 2001.
    ("[", "CMR10", 133.768, 257.277), // [Abeyesinghe
    ("e", "CMR10", 194.149, 257.277), // et
    ("a", "CMR10", 205.779, 257.277), // al.(2006)Abeyesinghe,
    ("D", "CMR10", 304.351, 257.277), // Devetak,
    ("H", "CMR10", 346.004, 257.277), // Hayden,
    ("a", "CMR10", 385.020, 257.277), // and
    ("W", "CMR10", 404.389, 257.277), // Winter]
    ("A", "CMR10", 149.266, 269.233), // Abeyesinghe,
    ("2", "CMR10", 209.647, 269.233), // 2006.
    ("[", "CMR10", 133.768, 289.158), // [Gross(1999)]
    ("G", "CMR10", 196.518, 289.158), // Gross,
    ("1", "CMR10", 227.163, 289.158), // 1999.
];

/// Labels with markup (`\emph{et~al.}`, `{\"o}{\ss}`), a note with a tie,
/// an undefined key, three labels in one citation, and a citation inside
/// `\textit`.
const MARKUP: &str = r#"\documentclass{article}
\pagestyle{empty}
\begin{document}
Kernel citations with markup in the label: see \cite{AbeyesingheDHW2006-fullSW} and
\cite{Abe,Gross} for details, and \cite[p.~5]{Gross} with a note; an undefined
\cite{nokey} key, then \cite{Abe,AbeyesingheDHW2006-fullSW,Gross} three labels in one
citation that the line has to break around somewhere near the end of the line.
\textit{In italics \cite{AbeyesingheDHW2006-fullSW} too.}
\begin{thebibliography}{3}
\expandafter\ifx\csname citenamefont\endcsname\relax
  \def\citenamefont#1{#1}\fi
\bibitem[{\citenamefont{Abe and Rajagopal}(2001)}]{Abe}
Abe, 2001.
\bibitem[{\citenamefont{Abeyesinghe}
  \emph{et~al.}(2006)\citenamefont{Abeyesinghe, Devetak, Hayden, and
  Winter}}]{AbeyesingheDHW2006-fullSW}
Abeyesinghe, 2006.
\bibitem[{\citenamefont{Gr{\"o}{\ss}er}(1999)}]{Gross}
Gross, 1999.
\end{thebibliography}
\end{document}
"#;

const MARKUP_GLYPHS: &[(&str, &str, f64, f64)] = &[
    ("K", "CMR10", 148.712, 134.765), // Kernel
    ("c", "CMR10", 179.733, 134.765), // citations
    ("w", "CMR10", 219.083, 134.765), // with
    ("m", "CMR10", 240.677, 134.765), // markup
    ("i", "CMR10", 276.403, 134.765), // in
    ("t", "CMR10", 286.917, 134.765), // the
    ("l", "CMR10", 302.966, 134.765), // label:
    ("s", "CMR10", 330.367, 134.765), // see
    ("[", "CMR10", 345.363, 134.765), // [Abeyesinghe
    ("e", "CMTI10", 405.744, 134.765), // et
    ("a", "CMTI10", 417.203, 134.765), // al.(2006)Abeyesinghe,
    ("D", "CMR10", 515.962, 134.765), // Devetak,
    ("H", "CMR10", 557.606, 134.765), // Hayden,
    ("a", "CMR10", 596.631, 134.765), // and
    ("W", "CMR10", 616.001, 134.765), // Winter]
    ("a", "CMR10", 133.768, 146.720), // and
    ("[", "CMR10", 154.303, 146.720), // [Abe
    ("a", "CMR10", 178.102, 146.720), // and
    ("R", "CMR10", 197.481, 146.720), // Rajagopal(2001),
    ("G", "CMR10", 276.541, 146.720), // Gr?o?er(1999)]
    ("f", "CMR10", 341.478, 146.720), // for
    ("d", "CMR10", 357.900, 146.720), // details,
    ("a", "CMR10", 393.722, 146.720), // and
    ("[", "CMR10", 414.257, 146.720), // [Gr?o?er(1999),
    ("p", "CMR10", 133.768, 158.675), // p.
    ("5", "CMR10", 144.283, 158.675), // 5]
    ("w", "CMR10", 154.243, 158.675), // with
    ("a", "CMR10", 175.837, 158.675), // a
    ("n", "CMR10", 183.030, 158.675), // note;
    ("a", "CMR10", 207.196, 158.675), // an
    ("u", "CMR10", 219.925, 158.675), // undefined
    ("[", "CMR10", 264.202, 158.675), // [?]
    ("k", "CMR10", 277.360, 158.675), // key,
    ("t", "CMR10", 296.406, 158.675), // then
    ("[", "CMR10", 317.990, 158.675), // [Abe
    ("a", "CMR10", 341.789, 158.675), // and
    ("R", "CMR10", 361.158, 158.675), // Rajagopal(2001),
    ("A", "CMR10", 437.957, 158.675), // Abeyesinghe
    ("e", "CMTI10", 495.571, 158.675), // et
    ("a", "CMTI10", 507.030, 158.675), // al.(2006)Abeyesinghe,
    ("D", "CMR10", 605.779, 158.675), // Devetak,
    ("H", "CMR10", 647.433, 158.675), // Hayden,
    ("a", "CMR10", 686.448, 158.675), // and
    ("W", "CMR10", 705.827, 158.675), // Winter,
    ("G", "CMR10", 133.768, 170.630), // Gr?
    ("t", "CMR10", 196.494, 170.630), // three
    ("l", "CMR10", 220.932, 170.630), // labels
    ("i", "CMR10", 247.881, 170.630), // in
    ("o", "CMR10", 258.455, 170.630), // one
    ("c", "CMR10", 275.670, 170.630), // citation
    ("t", "CMR10", 311.151, 170.630), // that
    ("t", "CMR10", 331.688, 170.630), // the
    ("l", "CMR10", 347.797, 170.630), // line
    ("h", "CMR10", 365.566, 170.630), // has
    ("t", "CMR10", 382.273, 170.630), // to
    ("b", "CMR10", 393.400, 170.630), // break
    ("a", "CMR10", 419.776, 170.630), // around
    ("s", "CMR10", 452.518, 170.630), // some-
    ("w", "CMR10", 133.768, 182.585), // where
    ("n", "CMR10", 161.467, 182.585), // near
    ("t", "CMR10", 182.525, 182.585), // the
    ("e", "CMR10", 198.584, 182.585), // end
    ("o", "CMR10", 216.293, 182.585), // of
    ("t", "CMR10", 226.531, 182.585), // the
    ("l", "CMR10", 242.580, 182.585), // line.
    ("I", "CMTI10", 264.910, 182.585), // In
    ("i", "CMTI10", 276.893, 182.585), // italics
    ("[", "CMTI10", 305.159, 182.585), // [Abeyesinghe
    ("e", "CMR10", 364.740, 182.585), // et
    ("a", "CMR10", 376.359, 182.585), // al.(2006)Abeyesinghe,
    ("D", "CMTI10", 474.225, 182.585), // Devetak,
    ("H", "CMTI10", 515.101, 182.585), // Hayden,
    ("a", "CMTI10", 554.334, 182.585), // and
    ("W", "CMTI10", 573.686, 182.585), // Winter]
    ("t", "CMTI10", 133.768, 194.541), // too.
    ("R", "CMBX12", 133.768, 227.486), // References
    ("[", "CMR10", 133.768, 249.307), // [Abe
    ("a", "CMR10", 157.567, 249.307), // and
    ("R", "CMR10", 176.936, 249.307), // Rajagopal(2001)]
    ("A", "CMR10", 256.504, 249.307), // Abe,
    ("2", "CMR10", 280.303, 249.307), // 2001.
    ("[", "CMR10", 133.768, 269.233), // [Abeyesinghe
    ("e", "CMTI10", 194.149, 269.233), // et
    ("a", "CMTI10", 205.608, 269.233), // al.(2006)Abeyesinghe,
    ("D", "CMR10", 304.358, 269.233), // Devetak,
    ("H", "CMR10", 346.011, 269.233), // Hayden,
    ("a", "CMR10", 385.037, 269.233), // and
    ("W", "CMR10", 404.406, 269.233), // Winter]
    ("A", "CMR10", 149.266, 281.188), // Abeyesinghe,
    ("2", "CMR10", 209.647, 281.188), // 2006.
    ("[", "CMR10", 133.768, 301.113), // [Gr?
    ("G", "CMR10", 201.971, 301.113), // Gross,
    ("1", "CMR10", 232.616, 301.113), // 1999.
];

#[test]
fn kernel_cite_labels_are_boxes() {
    if !common::lm_available() {
        eprintln!("SKIP kernel_cite_hbox: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(PLAIN, PLAIN_GLYPHS, TOL);
}

#[test]
fn kernel_cite_markup_notes_and_undefined_keys() {
    if !common::lm_available() {
        eprintln!("SKIP kernel_cite_hbox: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(MARKUP, MARKUP_GLYPHS, TOL);
}

/// The undefined key's `?` is `\reset@font\bfseries`: bold and upright.
#[test]
fn an_undefined_key_is_a_bold_question_mark() {
    if !common::lm_available() {
        eprintln!("SKIP kernel_cite_hbox: Latin Modern not installed");
        return;
    }
    let r = common::render_one(MARKUP);
    let font = |id: &str| r.v2.fonts.iter().find(|f| &*f.font_id == id).map(|f| f.postscript_name.clone()).unwrap_or_default();
    let faces: Vec<String> = r.v2.pages[0]
        .resident_items()
        .iter()
        .filter_map(|item| match item {
            flashtex_render_pipeline::display::Item::GlyphRun(run) if &*run.text == "?" => Some(font(&run.font_id)),
            _ => None,
        })
        .collect();
    assert_eq!(faces.len(), 1, "{faces:?}");
    assert!(faces[0].contains("Bold") && !faces[0].contains("Italic"), "{faces:?}");
}
