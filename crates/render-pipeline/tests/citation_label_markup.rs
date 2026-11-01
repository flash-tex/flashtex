//! A natbib `\bibitem` label's markup is set in the citation, not printed
//! (#956).
//!
//! revtex/`apsrmp`-style labels read `{\citenamefont{Abeyesinghe}
//! \emph{et~al.}(2006)\citenamefont{...}}`. The compiler flattened them to
//! characters, so every citation of an "et al." paper printed `\emphet~al.`
//! (1253 times in arXiv quant-ph/0702225). pdfTeX sets "et al." in the
//! italic face with a tie, the blank before `\emph` in the upright one, an
//! accented author as the accented letter, and -- inside italic text -- the
//! `\emph` upright with `\check@icl`'s italic correction before its space.
//!
//! ## Oracle
//!
//! pdfTeX 1.40.29 (TeX Live 2026, `/Library/TeX/texbin/pdflatex`, two runs)
//! on [`PROBE`]: the origin of each word's first glyph from the PDF's content
//! stream (`tools/visual-oracle/pdftext.py`), in bp, baseline from the page
//! top. The `Größer` words start at their `G`; pdfTeX's OT1 `\"o` is an
//! accent glyph over `o`, so the accented letter itself is not compared.
//! pdflatex is an oracle only and never runs in the product path.

mod common;

const TOL: f64 = 0.5;

const PROBE: &str = r#"\documentclass{article}
\usepackage{natbib}
\pagestyle{empty}
\begin{document}
Text \citep{AbeyesingheDHW2006-fullSW} and \citep{Abe}, then \citet{Gross} and
\citet{AbeyesingheDHW2006-fullSW} with \citep[p.~5]{Abe,Gross}.
\textit{In italics \citet{AbeyesingheDHW2006-fullSW} too.}
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

/// pdfTeX's glyphs: (text, pdfTeX font, x bp, baseline y bp), the first
/// glyph of every word of the three citation lines.
const EXPECT: &[(&str, &str, f64, f64)] = &[
    ("T", "CMR10", 148.712, 134.765),  // Text
    ("(", "CMR10", 171.230, 134.765),  // (Abeyesinghe
    ("e", "CMTI10", 231.991, 134.765), // et
    ("a", "CMTI10", 242.783, 134.765), // al.,
    ("2", "CMR10", 258.835, 134.765),  // 2006)
    ("a", "CMR10", 285.225, 134.765),  // and
    ("(", "CMR10", 303.877, 134.765),  // (Abe
    ("a", "CMR10", 328.055, 134.765),  // and
    ("R", "CMR10", 346.697, 134.765),  // Rajagopal,
    ("2", "CMR10", 396.200, 134.765),  // 2001),
    ("t", "CMR10", 425.497, 134.765),  // then
    ("G", "CMR10", 447.469, 134.765),  // Größer
    ("(", "CMR10", 133.768, 146.720),  // (1999)
    ("a", "CMR10", 165.059, 146.720),  // and
    ("A", "CMR10", 184.737, 146.720),  // Abeyesinghe
    ("e", "CMTI10", 242.649, 146.720), // et
    ("a", "CMTI10", 254.387, 146.720), // al.
    ("(", "CMR10", 268.697, 146.720),  // (2006)
    ("w", "CMR10", 299.988, 146.720),  // with
    ("(", "CMR10", 322.987, 146.720),  // (Abe
    ("a", "CMR10", 348.191, 146.720),  // and
    ("R", "CMR10", 367.859, 146.720),  // Rajagopal,
    ("2", "CMR10", 418.389, 146.720),  // 2001;
    ("G", "CMR10", 444.698, 146.720),  // Größer,
    ("1", "CMR10", 133.768, 158.675),  // 1999,
    ("p", "CMR10", 159.778, 158.675),  // p.
    ("5", "CMR10", 171.409, 158.675),  // 5).
    ("I", "CMTI10", 187.455, 158.675), // In
    ("i", "CMTI10", 200.465, 158.675), // italics
    ("A", "CMTI10", 229.737, 158.675), // Abeyesinghe
    ("e", "CMR10", 286.272, 158.675),  // et
    ("a", "CMR10", 297.892, 158.675),  // al.
    ("(", "CMTI10", 311.975, 158.675), // (2006)
    ("t", "CMTI10", 344.057, 158.675), // too.
];

#[test]
fn citation_label_markup_matches_pdftex() {
    if !common::lm_available() {
        eprintln!("SKIP citation_label_markup: Latin Modern not installed");
        return;
    }
    common::assert_pdftex_glyphs(PROBE, EXPECT, TOL);
}

/// No control word reaches the page, the accent is composed, and "et al."
/// is set in the face pdfTeX uses (CMTI10 in upright text, CMR10 in italic
/// text).
#[test]
fn citation_label_markup_is_set_not_printed() {
    if !common::lm_available() {
        eprintln!("SKIP citation_label_markup: Latin Modern not installed");
        return;
    }
    let r = common::render_one(PROBE);
    let font = |id: &str| r.v2.fonts.iter().find(|f| &*f.font_id == id).map(|f| f.postscript_name.clone()).unwrap_or_default();
    let mut text = String::new();
    let mut et_faces = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        let flashtex_render_pipeline::display::Item::GlyphRun(run) = item else { continue };
        text.push_str(&run.text);
        text.push(' ');
        if run.text.starts_with("et") {
            et_faces.push(font(&run.font_id));
        }
    }
    assert!(!text.contains('\\') && !text.contains('~'), "{text}");
    assert!(text.contains("Größer"), "{text}");
    assert_eq!(et_faces.len(), 3, "{et_faces:?}");
    assert!(et_faces[0].contains("Italic") && et_faces[1].contains("Italic"), "{et_faces:?}");
    assert!(!et_faces[2].contains("Italic"), "{et_faces:?}");
}
