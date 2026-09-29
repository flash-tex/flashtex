//! hyperref bookmarks in the exported PDF (`crate::outline`), against
//! pdflatex.
//!
//! Every expectation is pdflatex's own outline (MacTeX 2026: pdfTeX
//! 1.40.29, hyperref 7.01p, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`, two
//! or three runs for the `.out` file), read back with `qpdf --json`: each
//! entry's title, depth in the tree, target page, `/XYZ` left and top and
//! `/Count` (negative: closed with that many children; positive: open with
//! that many visible descendants).

mod common;

use std::collections::BTreeMap;

use common::{lm_available, render_one};
use flashtex_pdf::reader::{Obj, PdfFile};
use flashtex_render_pipeline::pdf::write_pdf_exact;

/// `(title, depth, page, left, top, count)`.
type Row = (String, usize, usize, f64, f64, Option<i64>);

fn decode(obj: &Obj) -> String {
    let Obj::String(bytes) = obj else { panic!("expected a string, got {obj:?}") };
    match bytes.strip_prefix(&[0xFE, 0xFF]) {
        Some(utf16) => String::from_utf16(&utf16.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect::<Vec<_>>()).unwrap(),
        None => bytes.iter().map(|&b| b as char).collect(),
    }
}

fn name_tree(pdf: &PdfFile, node: &Obj, out: &mut BTreeMap<String, Obj>) {
    let node = pdf.resolve(node).as_dict().expect("name tree node");
    if let Some(names) = node.get("Names").and_then(Obj::as_array) {
        for pair in names.chunks(2) {
            out.insert(decode(&pair[0]), pair[1].clone());
        }
    }
    for kid in node.get("Kids").and_then(Obj::as_array).unwrap_or(&[]) {
        name_tree(pdf, kid, out);
    }
}

/// The outline of `bytes` in document order, and the catalog's
/// `/PageMode`.
fn outline(bytes: &[u8]) -> (Vec<Row>, Option<String>) {
    let pdf = PdfFile::parse(bytes).expect("our PDF parses");
    let catalog = pdf.catalog().unwrap().clone();
    let kids: Vec<u32> = pdf
        .get(&catalog, "Pages")
        .and_then(Obj::as_dict)
        .and_then(|p| p.get("Kids"))
        .and_then(Obj::as_array)
        .unwrap()
        .iter()
        .map(|k| match k {
            Obj::Ref(n, _) => *n,
            other => panic!("kid {other:?}"),
        })
        .collect();
    let mut dests = BTreeMap::new();
    if let Some(names) = pdf.get(&catalog, "Names").and_then(Obj::as_dict) {
        name_tree(&pdf, names.get("Dests").unwrap(), &mut dests);
    }
    let number = |o: &Obj| o.as_number().unwrap().parse::<f64>().unwrap();
    let mut rows = Vec::new();
    fn walk(pdf: &PdfFile, first: Option<&Obj>, depth: usize, visit: &mut dyn FnMut(&PdfFile, &std::collections::BTreeMap<String, Obj>, usize)) {
        let mut current = first.cloned();
        while let Some(item_ref) = current {
            let item = pdf.resolve(&item_ref).as_dict().expect("outline item").clone();
            visit(pdf, &item, depth);
            walk(pdf, item.get("First"), depth + 1, visit);
            current = item.get("Next").cloned();
        }
    }
    if let Some(root) = pdf.get(&catalog, "Outlines").and_then(Obj::as_dict) {
        let first = root.get("First").cloned();
        walk(&pdf, first.as_ref(), 0, &mut |pdf, item, depth| {
            let action = pdf.get(item, "A").and_then(Obj::as_dict).expect("outline action");
            let name = decode(pdf.get(action, "D").unwrap());
            let dest = pdf.resolve(dests.get(&name).unwrap_or_else(|| panic!("destination {name:?}"))).as_dict().unwrap();
            let d = pdf.get(dest, "D").and_then(Obj::as_array).unwrap();
            let page = match &d[0] {
                Obj::Ref(n, _) => kids.iter().position(|k| k == n).unwrap() + 1,
                other => panic!("page {other:?}"),
            };
            assert_eq!(d[1].as_name(), Some("XYZ"), "{name}");
            rows.push((
                decode(pdf.get(item, "Title").unwrap()),
                depth,
                page,
                number(&d[2]),
                number(&d[3]),
                pdf.get(item, "Count").and_then(Obj::as_number).map(|c| c.parse().unwrap()),
            ));
        });
    }
    let mode = pdf.get(&catalog, "PageMode").and_then(Obj::as_name).map(str::to_string);
    (rows, mode)
}

fn pdf_of(source: &str) -> Vec<u8> {
    let r = render_one(source);
    write_pdf_exact(&r.v2, &[], None).expect("exact PDF").bytes
}

/// Titles, depths, pages and counts exactly; `/XYZ` coordinates within
/// 0.05bp (pdfTeX writes three decimals, the pipeline's own positions
/// differ from pdfTeX's in the last one).
fn check(source: &str, expected: &[(&str, usize, usize, f64, f64, Option<i64>)]) {
    let (rows, mode) = outline(&pdf_of(source));
    assert_eq!(mode.as_deref(), Some("UseOutlines"));
    let shape: Vec<(&str, usize, usize, Option<i64>)> = rows.iter().map(|r| (r.0.as_str(), r.1, r.2, r.5)).collect();
    let want: Vec<(&str, usize, usize, Option<i64>)> = expected.iter().map(|e| (e.0, e.1, e.2, e.5)).collect();
    assert_eq!(shape, want);
    for (got, want) in rows.iter().zip(expected) {
        assert!((got.3 - want.3).abs() <= 0.05 && (got.4 - want.4).abs() <= 0.05, "{}: /XYZ {} {} against pdflatex's {} {}", want.0, got.3, got.4, want.3, want.4);
    }
}

#[test]
fn article_sections_math_and_a_starred_section_match_pdflatex() {
    if !lm_available() {
        return;
    }
    check(
        r"\documentclass{article}
\usepackage{hyperref}
\begin{document}
\section{Introduction}
Some text here.
\subsection{Background and $x^2$ math}
More text.
\subsubsection{Deep}
Deep text.
\section*{Starred}
Starred text.
\section{Second \emph{emph} \texorpdfstring{$\alpha$}{alpha}~tie}
\subsection{Sub two}
\paragraph{Para}
Text.
\newpage
\section{On page two -- dash}
Final.
\end{document}
",
        &[
            ("Introduction", 0, 1, 133.768, 667.198, Some(-1)),
            ("Background and x2 math", 1, 1, 133.768, 621.474, Some(-1)),
            ("Deep", 2, 1, 133.768, 575.196, None),
            ("Second emph alpha tie", 0, 1, 133.768, 475.072, Some(-1)),
            ("Sub two", 1, 1, 133.768, 444.483, None),
            ("On page two – dash", 0, 2, 133.768, 667.198, None),
        ],
    );
}

#[test]
fn report_parts_chapters_and_a_phantom_section_match_pdflatex() {
    if !lm_available() {
        return;
    }
    check(
        r"\documentclass{report}
\usepackage[bookmarksnumbered,bookmarksopen]{hyperref}
\begin{document}
\part{First part}
\chapter{Alpha}
Text.
\section{A one}
Text.
\chapter{Beta}
\section{B one}
\subsection{B one one}
\chapter*{Unnumbered}
\phantomsection
\addcontentsline{toc}{chapter}{Unnumbered}
Text.
\end{document}
",
        &[
            ("I First part", 0, 1, 133.768, 504.426, Some(6)),
            ("1 Alpha", 1, 2, 133.768, 667.198, Some(1)),
            ("1.1 A one", 2, 2, 133.768, 465.884, None),
            ("2 Beta", 1, 3, 133.768, 667.198, Some(2)),
            ("2.1 B one", 2, 3, 133.768, 492.852, Some(1)),
            ("2.1.1 B one one", 3, 3, 133.768, 465.053, None),
            ("Unnumbered", 1, 4, 133.768, 537.684, None),
        ],
    );
}

/// An article `\part` (hyperref level 0, its number in the text), a user
/// macro, `\section*` + `\addcontentsline` (to the starred head's own
/// anchor), a `\phantomsection` inside a paragraph (raised a
/// `\baselineskip`; pdflatex's left is the anchor's own x, 266.078, where
/// the pipeline gives the column's left edge) and `\texorpdfstring`.
#[test]
fn article_part_macros_starred_and_phantom_anchors_match_pdflatex() {
    if !lm_available() {
        return;
    }
    let (rows, _) = outline(&pdf_of(
        r"\documentclass{article}
\usepackage{amssymb}
\usepackage{hyperref}
\newcommand{\proj}{FlashTeX}
\begin{document}
\part{Opening part}
\section{About \proj}
Some text in a paragraph that runs on for a while so that it fills a line or two of the page and then ends here.
\section*{Acknowledgements}
\addcontentsline{toc}{section}{Acknowledgements}
Thanks to everyone.
\subsection{Math $E=mc^2$ result}
Text with a phantom anchor.
\phantomsection
\addcontentsline{toc}{subsection}{Phantom entry}
More text after the phantom anchor.
\section{\texorpdfstring{$\mathbb{R}^n$}{Rn} spaces}
Final words.
\end{document}
",
    ));
    let expected = [
        ("I Opening part", 0, 1, 133.768, 667.198, Some(-3)),
        ("About FlashTeX", 1, 1, 133.768, 615.443, None),
        ("Acknowledgements", 1, 1, 133.768, 552.738, Some(-2)),
        ("Math E=mc2 result", 2, 1, 133.768, 497.107, None),
        ("Phantom entry", 2, 1, 133.768, 478.662, None),
        ("Rn spaces", 1, 1, 133.768, 449.757, None),
    ];
    assert_eq!(rows.len(), expected.len(), "{rows:#?}");
    for (got, want) in rows.iter().zip(&expected) {
        assert_eq!((got.0.as_str(), got.1, got.2, got.5), (want.0, want.1, want.2, want.5));
        assert!((got.3 - want.3).abs() <= 0.05 && (got.4 - want.4).abs() <= 0.05, "{}: {got:?}", want.0);
    }
}

/// `\pdfbookmark` and its relatives: levels from the last bookmark's,
/// `\belowpdfbookmark` past `bookmarksdepth` dropped, one right before
/// `\end{document}` below the last head. Titles, depths, pages and counts
/// are pdflatex's; the positions are checked only once the compiler sets
/// these commands' arguments as nothing (on this pin it still prints them,
/// which moves the heads below).
#[test]
fn pdfbookmark_levels_match_pdflatex() {
    if !lm_available() {
        return;
    }
    let (rows, mode) = outline(&pdf_of(
        r"\documentclass{article}
\usepackage{hyperref}
\begin{document}
\pdfbookmark[0]{Top level}{top}
Text.
\section{Sec}
\pdfbookmark[2]{Deeper bookmark}{deep}
\currentpdfbookmark{Current}{cur}
\subpdfbookmark{Sub bookmark}{sub}
\belowpdfbookmark{Below}{below}
\section{Another}
\subsection{X}
\belowpdfbookmark{Below X}{bx}
\end{document}
",
    ));
    assert_eq!(mode.as_deref(), Some("UseOutlines"));
    let shape: Vec<(&str, usize, usize, Option<i64>)> = rows.iter().map(|r| (r.0.as_str(), r.1, r.2, r.5)).collect();
    assert_eq!(
        shape,
        vec![
            ("Top level", 0, 1, Some(-2)),
            ("Sec", 1, 1, Some(-2)),
            ("Deeper bookmark", 2, 1, None),
            ("Current", 2, 1, Some(-1)),
            ("Sub bookmark", 3, 1, None),
            ("Another", 1, 1, Some(-1)),
            ("X", 2, 1, Some(-1)),
            ("Below X", 3, 1, None),
        ]
    );
    // pdflatex: `Top level` at the text top, 133.768 667.198.
    assert!((rows[0].3 - 133.768).abs() <= 0.05 && (rows[0].4 - 667.198).abs() <= 0.05, "{:?}", rows[0]);
    // `Below X` stands below `X`, not at the last page's top.
    assert!(rows[7].4 < rows[6].4, "{:?} {:?}", rows[6], rows[7]);
}

#[test]
fn book_front_matter_and_appendix_match_pdflatex() {
    if !lm_available() {
        return;
    }
    check(
        r"\documentclass{book}
\usepackage[bookmarksnumbered]{hyperref}
\begin{document}
\frontmatter
\chapter{Preface}
Preface text.
\tableofcontents
\mainmatter
\chapter{First}
\section{One}
Text.
\section{Two}
Text.
\appendix
\chapter{Extra}
\section{Details}
Text.
\end{document}
",
        &[
            ("Preface", 0, 1, 106.869, 668.127, None),
            ("1 First", 0, 5, 106.869, 668.127, Some(-2)),
            ("1.1 One", 1, 5, 106.869, 493.781, None),
            ("1.2 Two", 1, 5, 106.869, 439.014, None),
            ("A Extra", 0, 7, 106.869, 668.127, Some(-1)),
            ("A.1 Details", 1, 7, 106.869, 493.781, None),
        ],
    );
}

/// Non-ASCII text is written UTF-16BE; accents, `\ss`, a `\texorpdfstring`
/// Greek letter and the escaped specials come out as pdflatex's.
#[test]
fn non_ascii_titles_match_pdflatex() {
    if !lm_available() {
        return;
    }
    check(
        "\\documentclass{article}
\\usepackage[utf8]{inputenc}
\\usepackage[T1]{fontenc}
\\usepackage[unicode]{hyperref}
\\begin{document}
\\section{Café naïve}
\\section{Stra\\ss e \\\"Uber}
\\section{Greek \\texorpdfstring{$\\beta$}{β}}
\\section{Quotes ``hi'' and `x'}
\\section{Tilde~a\\&b 50\\% \\#1 \\textbackslash}
\\end{document}
",
        &[
            ("Café naïve", 0, 1, 133.768, 667.198, None),
            ("Straße Über", 0, 1, 133.768, 647.372, None),
            ("Greek β", 0, 1, 133.768, 619.576, None),
            ("Quotes ``hi'' and `x'", 0, 1, 133.768, 588.99, None),
            ("Tilde a&b 50% #1 \\", 0, 1, 133.768, 561.195, None),
        ],
    );
}

#[test]
fn bookmarks_open_level_matches_pdflatex() {
    if !lm_available() {
        return;
    }
    check(
        r"\documentclass{article}
\usepackage[bookmarksopen,bookmarksopenlevel=2]{hyperref}
\begin{document}
\section{A}
\subsection{A1}
\subsubsection{A11}
\section{B}
\subsection{B1}
\end{document}
",
        &[
            ("A", 0, 1, 133.768, 667.198, Some(1)),
            ("A1", 1, 1, 133.768, 647.37, Some(-1)),
            ("A11", 2, 1, 133.768, 626.988, None),
            ("B", 0, 1, 133.768, 608.598, Some(1)),
            ("B1", 1, 1, 133.768, 580.8, None),
        ],
    );
}

/// No hyperref, or `bookmarks=false`: pdflatex writes no `/Outlines`; with
/// hyperref its `/PageMode` is still there (`UseNone` without bookmarks),
/// and a hyperref document with no bookmark at all (only starred heads)
/// still gets `UseOutlines` (fixtures/real-world/cv under pdflatex).
#[test]
fn no_outline_without_hyperref_bookmarks() {
    if !lm_available() {
        return;
    }
    for (preamble, heading, want) in [
        ("", r"\section{Intro}", None),
        (r"\usepackage[bookmarks=false]{hyperref}", r"\section{Intro}", Some("UseNone")),
        (r"\usepackage{hyperref}", r"\section*{Intro}", Some("UseOutlines")),
    ] {
        let bytes = pdf_of(&format!("\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n{heading}\nText.\n\\end{{document}}\n"));
        let (rows, mode) = outline(&bytes);
        assert!(rows.is_empty(), "{preamble}: {rows:?}");
        assert_eq!(mode.as_deref(), want, "{preamble}");
        assert!(!String::from_utf8_lossy(&bytes).contains("/Outlines"), "{preamble}");
    }
}

/// The display list carries the outline for the PDF writer only: its
/// envelope is the same with and without hyperref's bookmarks.
#[test]
fn the_outline_does_not_reach_the_display_list_envelope() {
    if !lm_available() {
        return;
    }
    let on = render_one("\\documentclass{article}\n\\usepackage[bookmarks=true,]{hyperref}\n\\begin{document}\n\\section{Intro}\nText.\n\\end{document}\n");
    let off = render_one("\\documentclass{article}\n\\usepackage[bookmarks=false]{hyperref}\n\\begin{document}\n\\section{Intro}\nText.\n\\end{document}\n");
    assert_eq!(on.v2.outline.as_ref().map(|o| o.entries.len()), Some(1));
    assert_eq!(off.v2.outline.as_ref().map(|o| o.entries.len()), Some(0));
    assert_eq!(on.v2.pages, off.v2.pages, "bookmarks move nothing on the page");
}
