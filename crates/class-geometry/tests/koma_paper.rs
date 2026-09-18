//! KOMA-Script paper size against pdflatex, pinned (issue #842, first half).
//!
//! pdflatex is an ORACLE ONLY: it never runs here. Every expectation below
//! was read back out of pdflatex (TeX Live 2026, pdfTeX 1.40.29) on the lane
//! machine and is committed as a literal:
//!
//! * `\documentclass{scrartcl}` (likewise `scrreprt`, `scrbook`) makes a
//!   595.276 x 841.89 (A4) PDF: KOMA defaults to A4 *and* writes its paper
//!   into `\pdfpagewidth`/`\pdfpageheight` (`\the\pdfpagewidth` reads
//!   597.50793pt, exactly `\paperwidth`).
//! * `\documentclass[a4paper]{article}` stays 612 x 792 (letter): the
//!   standard classes set `\paperwidth` but never the PDF dimensions, so the
//!   media keeps the engine default. That near-miss is pinned here so nobody
//!   "fixes" it to A4 later.
//!
//! Method: `\the\paperwidth`, `\the\pdfpagewidth`, `\f@size`,
//! `\if@twoside` / `\if@titlepage` / `\if@openright` `\typeout` probes plus
//! `pdfinfo` on filler documents. The text block (`typearea`) is NOT pinned
//! here — it is still the standard-class formula and does not match
//! pdflatex; that is the second half of #842.

use flashtex_class_geometry::*;

fn setup(class: &str, options: &str) -> DocumentSetup {
    let src = format!("\\documentclass[{options}]{{{class}}}\n\\begin{{document}}\n");
    DocumentSetup::from_preamble(&src).expect("class must parse")
}

fn resolved(class: &str, options: &str) -> ResolvedDocument {
    resolve(&setup(class, options))
}

fn media(r: &ResolvedDocument) -> (Sp, Sp) {
    (r.frame.pdf_page_width, r.frame.pdf_page_height)
}

/// Exact paper sizes. pdflatex prints these as 597.50793pt x 845.04694pt
/// (A4) and 421.10083pt x 597.50793pt (A5) — 5-decimal rounding, so the test
/// compares `Sp` values, never the rounded strings.
fn a4() -> (Sp, Sp) {
    Paper::A4.size()
}

fn a5() -> (Sp, Sp) {
    Paper::A5.size()
}

fn letter() -> (Sp, Sp) {
    (
        Sp::parse("8.5in").unwrap(),
        Sp::parse("11in").unwrap(),
    )
}

#[test]
fn scrartcl_defaults_to_a4_media() {
    let r = resolved("scrartcl", "");
    assert_eq!(media(&r), a4());
    assert_eq!((r.params.paperwidth, r.params.paperheight), a4());
    // Oracle defaults: 11pt (`\f@size` 10.95), oneside, no title page.
    assert_eq!(r.options.size, BaseSize::Pt11);
    assert!(!r.options.twoside);
    assert!(!r.options.titlepage);
    assert!(r.options.unused.is_empty());
}

#[test]
fn scrbook_and_scrreprt_mirror_book_and_report_sides() {
    let book = resolved("scrbook", "");
    assert_eq!(media(&book), a4());
    assert!(book.options.twoside, "scrbook is two-sided like book");
    assert!(book.options.titlepage);
    assert!(book.options.openright, "scrbook opens right like book");
    let reprt = resolved("scrreprt", "");
    assert_eq!(media(&reprt), a4());
    assert!(!reprt.options.twoside, "scrreprt is one-sided like report");
    assert!(reprt.options.titlepage);
    assert!(!reprt.options.openright, "scrreprt opens any like report");
}

#[test]
fn koma_paper_options_select_the_media() {
    assert_eq!(media(&resolved("scrartcl", "paper=letter")), letter());
    assert_eq!(media(&resolved("scrartcl", "letterpaper")), letter());
    assert_eq!(media(&resolved("scrartcl", "paper=a5")), a5());
    let land = resolved("scrartcl", "landscape");
    assert_eq!(
        (land.frame.pdf_page_width, land.frame.pdf_page_height),
        (a4().1, a4().0),
        "landscape swaps the A4 media"
    );
}

#[test]
fn koma_conflicts_resolve_in_source_order_with_legacy_paper_winning() {
    // All four probed against pdflatex; article keeps declaration order for
    // the same pairs (twoside and 12pt win regardless of position).
    let o = |opts: &str| ClassOptions::parse(ClassKind::ScrArticle, opts);
    assert!(!o("twoside,oneside").twoside);
    assert!(o("oneside,twoside").twoside);
    assert_eq!(o("12pt,10pt").size, BaseSize::Pt10);
    assert_eq!(o("10pt,fontsize=12pt").size, BaseSize::Pt12);
    assert_eq!(o("paper=letter,a4paper").paper, Paper::A4);
    assert_eq!(o("a4paper,paper=letter").paper, Paper::A4);
    assert_eq!(o("paper=letter").paper, Paper::Letter);
}

#[test]
fn koma_typearea_keys_warn_instead_of_silently_doing_nothing() {
    // `DIV=`/`BCOR=` are real KOMA options (no pdflatex warning), but this
    // crate does not apply them yet; they must surface as unused rather
    // than vanish. `paper=`/`fontsize=` values this slice honours are used.
    let o = ClassOptions::parse(ClassKind::ScrArticle, "DIV=12,BCOR=5mm,paper=letter");
    assert_eq!(o.unused, vec!["DIV=12".to_string(), "BCOR=5mm".to_string()]);
    assert_eq!(o.paper, Paper::Letter);
    // pdflatex also warns for `[openright]{scrartcl}`; mirror that exactly.
    let open = ClassOptions::parse(ClassKind::ScrArticle, "openright");
    assert_eq!(open.unused, vec!["openright".to_string()]);
    assert!(!open.openright, "an undeclared option takes no effect");
}

#[test]
fn koma_classes_parse_from_the_preamble() {
    // Before this slice `from_preamble` returned `None` for every KOMA
    // class, and the pipeline silently fell back to article geometry with
    // an empty page style (which is why scrartcl's text band did not even
    // equal article's). Unknown classes still fall back.
    for (src, want) in [
        ("scrartcl", ClassKind::ScrArticle),
        ("scrreprt", ClassKind::ScrReport),
        ("scrbook", ClassKind::ScrBook),
    ] {
        let s = setup(src, "");
        assert_eq!(s.class, want);
    }
    assert!(DocumentSetup::from_preamble("\\documentclass{memoir}\n\\begin{document}\n").is_none());
}

#[test]
fn koma_default_pagestyle_is_plain() {
    // Page 2 of a section-bearing document shows a bare page-number footer
    // and no running head under all three classes (pdflatex probe).
    for cls in ["scrartcl", "scrreprt", "scrbook"] {
        let r = resolved(cls, "");
        assert_eq!(r.pagestyle, PageStyle::Plain, "{cls}");
    }
}

#[test]
fn a4paper_article_keeps_letter_media() {
    // THE near-miss (issue #842): article's `a4paper` sets `\paperwidth`
    // but not the PDF dimensions, so pdflatex emits a Letter PDF whose text
    // block is A4-derived. Both halves must hold.
    let r = resolved("article", "a4paper");
    assert_eq!(media(&r), letter(), "media stays the engine default");
    assert_eq!(
        (r.params.paperwidth, r.params.paperheight),
        a4(),
        "but the paper (and hence the text block) is A4"
    );
    let plain = resolved("article", "");
    assert_eq!(media(&plain), letter());
}

#[test]
fn depths_mirror_the_standard_classes() {
    assert_eq!(resolve(&setup("scrartcl", "")).secnumdepth, 3);
    assert_eq!(resolve(&setup("scrreprt", "")).secnumdepth, 2);
    assert_eq!(resolve(&setup("scrbook", "")).secnumdepth, 2);
}
