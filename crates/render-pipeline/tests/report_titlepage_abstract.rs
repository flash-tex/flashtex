//! The `titlepage` `abstract`: a page of its own, `\vfil`-centred, with a
//! page break on each side.
//!
//! It is `report`'s and `book`'s default and `article`'s `titlepage`
//! option, and it is a *different environment* from the one-column
//! `abstract` of `tests/abstract_env.rs`, not a variant of it (report.cls
//! 441-450 against 452-462):
//!
//! ```tex
//! \newenvironment{abstract}{%
//!     \titlepage
//!     \null\vfil
//!     \@beginparpenalty\@lowpenalty
//!     \begin{center}%
//!       \bfseries \abstractname
//!       \@endparpenalty\@M
//!     \end{center}}%
//!    {\par\vfil\null\endtitlepage}
//! ```
//!
//! No `\small`, no `\vspace{-.5em}` under the head, no `quotation` — the
//! head is `\normalsize\bfseries` centred at the full measure and the body
//! is ordinary full-measure paragraphs, the first unindented (`\@doendpe`
//! after `\end{center}`). The `titlepage` environment around it
//! (report.cls 498-512) is `\newpage \thispagestyle{empty}
//! \setcounter{page}\@ne` on the way in and `\newpage` on the way out, so
//! the abstract is alone on its page and what follows `\end{abstract}`
//! starts the next one. Half of the error this file pins was that missing
//! page break, not the distance.
//!
//! Every number below is a glyph origin of the reference PDF that the
//! quoted probe produces, read with PyMuPDF: pdfTeX
//! 3.141592653-2.6-1.40.29, TeX Live 2026, `report.cls` v1.4n, letterpaper
//! at the class's own default geometry. pdflatex is an oracle only and
//! never runs in the product path.
//!
//! Before this was set, the head was not set at all and the body landed
//! where the compiler's plain text put it -- 134.765 / 140.742 / 137.753 bp
//! against pdflatex's 335.340 / 341.849 / 342.812, so **-200.575 /
//! -201.107 / -205.059 bp** at 10/11/12 pt -- on a single page with the
//! following material under it instead of two pages.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// A glyph run's first origin, with the page it fell on.
#[derive(Debug, Clone)]
struct Word {
    page: usize,
    text: String,
    x: f64,
    baseline: f64,
    size: f64,
}

struct Laid {
    pages: usize,
    /// `(code, message)` of every v1 diagnostic.
    diagnostics: Vec<(String, String)>,
    words: Vec<Word>,
}

fn layout(text: &str) -> Laid {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    let mut words = Vec::new();
    for (page, p) in r.v2.pages.iter().enumerate() {
        for it in p.resident_items() {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                words.push(Word {
                    page,
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    size: run.font_size.to_bp(),
                });
            }
        }
    }
    Laid {
        pages: r.v2.pages.len(),
        diagnostics: v1.diagnostics.iter().map(|d| (d.code.clone(), d.message.clone())).collect(),
        words,
    }
}

/// Whether the compiler's "environment 'abstract' is not implemented"
/// limitation survived — i.e. the pipeline did not set the environment.
fn abstract_unimplemented(l: &Laid) -> bool {
    l.diagnostics.iter().any(|(_, m)| m.contains("'abstract' is not implemented"))
}

fn word<'a>(l: &'a Laid, text: &str) -> &'a Word {
    l.words
        .iter()
        .find(|w| w.text == text)
        .unwrap_or_else(|| panic!("no word {text:?} in {:?}", l.words.iter().map(|w| &w.text).collect::<Vec<_>>()))
}

/// The probe. One `\begin{abstract}` whose body needs a second line, and a
/// paragraph after it, so both the environment's own two baselines and the
/// page boundary below it are observable.
const PROBE: &str = "\\documentclass[OPTS]{CLS}\n\\begin{document}\n\\begin{abstract}\n\
Body of the abstract, long enough to need a second line of its own when it is set at the measure of the environment in this class.\n\
\\end{abstract}\n\nAFTERWARDS the following material.\n\\end{document}";

fn probe(class: &str, options: &str) -> Laid {
    layout(&PROBE.replace("OPTS", options).replace("CLS", class))
}

/// The glyph gate of this series.
const GATE_BP: f64 = 0.5;

fn at(w: &Word, page: usize, baseline: f64, what: &str) {
    assert_eq!(w.page, page, "{what}: on page {}, pdflatex page {page}", w.page);
    assert!(
        (w.baseline - baseline).abs() < GATE_BP,
        "{what}: baseline {} bp, pdflatex {baseline} bp ({} off)",
        w.baseline,
        w.baseline - baseline
    );
}

/// `report`'s default: the head, the body's two lines and the page that
/// follows, at all three class sizes.
///
/// The head sits `\topskip` + one `\vfil`'s share + `\topsep + \partopsep`
/// + `\baselineskip` below the top of the text area, and the body's first
/// line one `\@topsepadd` (`\end{center}`) plus one `\baselineskip` below
/// the head: 22 pt at a 10 pt base, 25.6 at 11 pt, 27.5 at 12 pt. The
/// `\vfil` share is what the whole page's natural height decides, so
/// getting the head right and the body wrong (or the reverse) is not a
/// thing this can do — which is exactly why the *page* assertion below
/// carries as much of the gate as the baselines do.
#[test]
fn the_titlepage_abstract_is_a_page_of_its_own() {
    if !lm_available() {
        return;
    }
    // (options, head, body line 1, body line 2, the paragraph after it)
    for (size, head, line1, line2, after) in [
        ("10pt", 313.422, 335.340, 347.295, 134.765),
        ("11pt", 316.345, 341.849, 355.399, 140.742),
        ("12pt", 315.415, 342.812, 357.258, 137.753),
    ] {
        let l = probe("report", size);
        assert_eq!(l.pages, 2, "{size}: the abstract has a page of its own and the material after it the next one");
        at(word(&l, "Abstract"), 0, head, &format!("{size} head"));
        at(word(&l, "Body"), 0, line1, &format!("{size} body line 1"));
        at(word(&l, "set"), 0, line2, &format!("{size} body line 2"));
        at(word(&l, "AFTERWARDS"), 1, after, &format!("{size} the paragraph after \\end{{abstract}}"));
        assert!(
            !abstract_unimplemented(&l),
            "{size}: the environment is set, so its compiler limitation is superseded: {:?}",
            l.diagnostics
        );
    }
}

/// `article`'s `titlepage` option is the same environment (article.cls
/// 367-375 is report.cls 441-450 verbatim), and pdflatex sets it at the
/// same baselines: `\@listI` does not differ by class, and neither does
/// this.
#[test]
fn the_article_titlepage_option_is_the_same_environment() {
    if !lm_available() {
        return;
    }
    for (size, head, line1) in [("10pt", 313.422, 335.340), ("11pt", 316.345, 341.849), ("12pt", 315.415, 342.812)] {
        let l = probe("article", &format!("{size},titlepage"));
        assert_eq!(l.pages, 2, "{size}: article[titlepage] gets the page too");
        at(word(&l, "Abstract"), 0, head, &format!("article {size} head"));
        at(word(&l, "Body"), 0, line1, &format!("article {size} body"));
    }
}

/// The head is `\normalsize` and the body is at the full measure: this
/// branch has neither the `\small` nor the `quotation` of the one-column
/// one, and the first body paragraph is unindented (`\@doendpe`).
///
/// `report` at 10 pt: `\normalsize` is 9.963 bp (cmr10 at 10 pt), the text
/// area starts at 133.768 bp and the head is centred in it.
#[test]
fn the_head_is_normalsize_and_the_body_is_the_full_measure() {
    if !lm_available() {
        return;
    }
    let l = probe("report", "10pt");
    let head = word(&l, "Abstract");
    assert!((head.size - 9.963).abs() < 0.01, "head at \\normalsize, not \\small: {} bp", head.size);
    const TEXT_X: f64 = 133.768;
    assert!(head.x > TEXT_X + 100.0, "head centred, not at the margin: {}", head.x);
    for name in ["Body", "set"] {
        let w = word(&l, name);
        assert!(
            (w.x - TEXT_X).abs() < GATE_BP,
            "{name:?} at the full measure and unindented: {} bp, pdflatex {TEXT_X} bp",
            w.x
        );
        assert!((w.size - 9.963).abs() < 0.01, "{name:?} at \\normalsize: {} bp", w.size);
    }
}

/// `notitlepage` is the *other* environment and is unchanged: the `\small`
/// centred head and the `quotation` body of `tests/abstract_env.rs`, all on
/// one page. #738 measured this branch exact; this pins it so a change to
/// the `titlepage` one cannot move it.
#[test]
fn notitlepage_keeps_the_small_head_and_the_quotation() {
    if !lm_available() {
        return;
    }
    // (options, head, head size, body line 1, body x, the paragraph after)
    for (size, head, head_size, line1, body_x, after) in [
        ("10pt", 134.765, 8.966, 150.381, 172.498, 179.273),
        ("11pt", 140.742, 9.963, 158.924, 168.015, 193.395),
        ("12pt", 137.753, 10.909, 157.981, 156.483, 197.932),
    ] {
        let l = probe("report", &format!("{size},notitlepage"));
        assert_eq!(l.pages, 1, "{size},notitlepage: no page of its own");
        let h = word(&l, "Abstract");
        at(h, 0, head, &format!("{size},notitlepage head"));
        assert!((h.size - head_size).abs() < 0.01, "{size},notitlepage head at \\small: {} bp", h.size);
        let b = word(&l, "Body");
        at(b, 0, line1, &format!("{size},notitlepage body"));
        assert!(
            (b.x - body_x).abs() < GATE_BP,
            "{size},notitlepage body at the quotation measure: {} bp, pdflatex {body_x} bp",
            b.x
        );
        at(word(&l, "AFTERWARDS"), 0, after, &format!("{size},notitlepage the paragraph after"));
    }
}

/// `\maketitle` and then an `abstract` in a `report` is *two* title pages,
/// one after the other, and the body on a third: `\titlepage` is
/// `\newpage`, and a `\newpage` on a page that has nothing on it yet ships
/// nothing, so there is no blank page between them.
#[test]
fn maketitle_and_an_abstract_are_two_title_pages() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass[10pt]{report}\n\\title{A Title}\\author{An Author}\\date{March 14, 2025}\n\
\\begin{document}\n\\maketitle\n\n\\begin{abstract}\n\
Body of the abstract, long enough to need a second line of its own when it is set at the measure of the environment in this class.\n\
\\end{abstract}\n\nAFTERWARDS the following material.\n\\end{document}";
    let l = layout(src);
    assert_eq!(l.pages, 3, "title page, abstract page, body");
    at(word(&l, "March"), 0, 409.729, "the date, on the title page");
    at(word(&l, "Abstract"), 1, 313.422, "the head, on the abstract page");
    at(word(&l, "Body"), 1, 335.340, "the body, on the abstract page");
    at(word(&l, "AFTERWARDS"), 2, 134.765, "the material after it, on the third page");
}

/// `book.cls` defines no `abstract` environment at all — pdflatex stops
/// with `! LaTeX Error: Environment abstract undefined.` — so a `book`'s
/// `\begin{abstract}` keeps the compiler's own "not implemented" warning
/// and its body stays plain text. Nothing here sets a page for it.
#[test]
fn book_has_no_abstract_environment() {
    if !lm_available() {
        return;
    }
    for size in ["10pt", "11pt", "12pt"] {
        let l = probe("book", size);
        assert_eq!(l.pages, 1, "{size}: book gets no abstract page");
        assert!(
            !l.words.iter().any(|w| w.text == "Abstract"),
            "{size}: book sets no head, the environment not existing: {:?}",
            l.words.iter().map(|w| &w.text).collect::<Vec<_>>()
        );
        assert!(
            abstract_unimplemented(&l),
            "{size}: book keeps the compiler's own limitation: {:?}",
            l.diagnostics
        );
    }
}

/// A two-column `titlepage` document is *not* set: `\titlepage` opens
/// `\if@twocolumn \@restonecoltrue\onecolumn`, so pdflatex gives the
/// abstract a one-column page inside a two-column document and goes back to
/// two columns after it (head 313.428 bp, body at the full 72 bp measure,
/// the material after it on page 2). This pipeline has no mid-document
/// column switch, and setting the abstract in the column it finds would be
/// a worse answer than none — so the compiler's own warning stands, and
/// says so.
#[test]
fn a_two_column_titlepage_abstract_keeps_the_compiler_warning() {
    if !lm_available() {
        return;
    }
    let l = probe("report", "10pt,twocolumn");
    assert!(
        abstract_unimplemented(&l),
        "the two-column titlepage abstract is declared, not approximated: {:?}",
        l.diagnostics
    );
    assert!(!l.words.iter().any(|w| w.text == "Abstract"), "and no head is set");
}

/// A nested `\noindent`/`\begin{itemize}` inside a `titlepage` abstract
/// keeps its own indentation and item label: the fix only normalises the
/// paragraph right after `\begin{abstract}` (`\@doendpe`'s unindent and the
/// head's own `env_open`), not every paragraph of the body. Before the fix,
/// the loop forced `indent = k != 0` and cleared `list`/`env_open`/
/// `env_close` on *every* body paragraph, so a `\noindent` second paragraph
/// was re-indented and an itemize's bullet vanished.
#[test]
fn nested_noindent_and_list_survive_the_titlepage_wrapper() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass[10pt]{report}\n\\begin{document}\n\\begin{abstract}\n\
First paragraph of the abstract.\n\n\
\\noindent Second paragraph, explicitly not indented.\n\n\
\\begin{itemize}\n\\item Bulleted point.\n\\end{itemize}\n\
\\end{abstract}\n\\end{document}";
    let l = layout(src);
    assert!(
        !abstract_unimplemented(&l),
        "the titlepage abstract is set: {:?}",
        l.diagnostics
    );
    const TEXT_X: f64 = 133.768;
    let first = word(&l, "First");
    assert!((first.x - TEXT_X).abs() < GATE_BP, "first body paragraph unindented by \\@doendpe: {} bp", first.x);
    let second = word(&l, "Second");
    assert!(
        (second.x - TEXT_X).abs() < GATE_BP,
        "\\noindent must survive the wrapper, not be re-indented to \\listparindent: {} bp, expected {TEXT_X} bp",
        second.x
    );
    assert!(
        l.words.iter().any(|w| w.text == "•"),
        "the nested itemize's own list metadata (and its bullet) must survive the wrapper: {:?}",
        l.words.iter().map(|w| &w.text).collect::<Vec<_>>()
    );
}

/// `\vspace` written *inside* `\begin{abstract}...\end{abstract}` (after the
/// command, before the body's first character) is glue between the head and
/// the body, not glue that stood before `\begin{abstract}`: it must neither
/// move above the head nor be dropped by the page break's own lead-skip
/// reset. Before the fix, `drop_lead` zeroed `vspace_before` unconditionally,
/// discarding the 20pt outright.
#[test]
fn vspace_inside_the_titlepage_abstract_stays_between_head_and_body() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass[10pt]{report}\n\\begin{document}\n\\begin{abstract}\n\\vspace{20pt}\n\
Body text after the vspace.\n\\end{abstract}\n\\end{document}";
    let with_vspace = layout(src);
    let without = "\\documentclass[10pt]{report}\n\\begin{document}\n\\begin{abstract}\n\
Body text after the vspace.\n\\end{abstract}\n\\end{document}";
    let baseline = layout(without);
    // The three `\vfil`s (see the module doc comment) redistribute when a
    // \vspace grows the page's natural content height, so the head itself
    // shifts a little too; what isolates the inserted glue from that shared
    // redistribution is the *gap* between the head and the body, which only
    // the environment's own skips and this \vspace can widen.
    let head_y = word(&with_vspace, "Abstract").baseline;
    let base_head_y = word(&baseline, "Abstract").baseline;
    let body_y = word(&with_vspace, "Body").baseline;
    let base_body_y = word(&baseline, "Body").baseline;
    let gap = body_y - head_y;
    let base_gap = base_body_y - base_head_y;
    assert!(
        (gap - base_gap - 20.0).abs() < GATE_BP,
        "the 20pt must land between the head and the body, not be dropped or moved above the head: \
         gap {gap} bp vs {base_gap} bp without the \\vspace (delta {}, expected +20)",
        gap - base_gap
    );
}

/// `twoside` is `\flushbottom`, so the three `\vfil`s share the page's
/// slack without `\raggedbottom`'s `.0001fil` fourth claimant, and the
/// abstract lands 0.006 bp lower than in the one-sided document.
/// `\endtitlepage` resets `\c@page` only when one-sided, so the page after
/// a two-sided abstract is numbered 2, not 1.
#[test]
fn a_two_sided_titlepage_abstract_keeps_its_page_number() {
    if !lm_available() {
        return;
    }
    let l = probe("report", "10pt,twoside");
    assert_eq!(l.pages, 2);
    at(word(&l, "Abstract"), 0, 313.428, "twoside head");
    at(word(&l, "Body"), 0, 335.346, "twoside body");
    at(word(&l, "AFTERWARDS"), 1, 134.765, "twoside, the material after it");
    let folio = l.words.iter().find(|w| w.page == 1 && w.baseline > 690.0).expect("a page number on page 2");
    assert_eq!(folio.text, "2", "\\endtitlepage resets \\c@page only when one-sided");
}
