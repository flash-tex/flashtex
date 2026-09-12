//! Exact-number tests against values printed by BasicTeX pdflatex
//! (TeX Live 2026, pdfTeX 1.40.29) for `\documentclass[<size>,<paper>]{article}`.
//! The probe document and its output are reproduced in the crate README.

use flashtex_document_style::*;

fn sheet(size: BaseSize, paper: Paper) -> Stylesheet {
    Stylesheet::article(ClassOptions { paper, size })
}

fn close(a: f64, b: f64, tol: f64, what: &str) {
    assert!(
        (a - b).abs() <= tol,
        "{what}: got {a}, expected {b} (tol {tol})"
    );
}

/// (size, paper, textwidth, textheight, oddsidemargin, evensidemargin,
/// topmargin, marginparwidth) as printed by `\the` in pdflatex.
type ProbeRow = (BaseSize, Paper, f64, f64, f64, f64, f64, f64);
const PROBE: &[ProbeRow] = &[
    (
        BaseSize::Pt10,
        Paper::Letter,
        345.0,
        550.0,
        62.0,
        62.0,
        16.0,
        65.0,
    ),
    (
        BaseSize::Pt10,
        Paper::A4,
        345.0,
        598.0,
        53.0,
        54.0,
        17.0,
        57.0,
    ),
    (BaseSize::Pt10, Paper::A5, 276.0, 346.0, 0.0, 0.0, 19.0, 3.0),
    (
        BaseSize::Pt10,
        Paper::Legal,
        345.0,
        766.0,
        62.0,
        62.0,
        17.0,
        65.0,
    ),
    (
        BaseSize::Pt11,
        Paper::Letter,
        360.0,
        541.40024,
        54.0,
        55.0,
        21.0,
        59.0,
    ),
    (
        BaseSize::Pt11,
        Paper::A4,
        360.0,
        595.80026,
        46.0,
        46.0,
        18.0,
        50.0,
    ),
    (
        BaseSize::Pt11,
        Paper::A5,
        276.0,
        351.00015,
        0.0,
        0.0,
        17.0,
        4.0,
    ),
    (
        BaseSize::Pt11,
        Paper::Legal,
        360.0,
        759.00034,
        54.0,
        55.0,
        20.0,
        59.0,
    ),
    (
        BaseSize::Pt12,
        Paper::Letter,
        390.0,
        548.5,
        39.0,
        40.0,
        17.0,
        44.0,
    ),
    (
        BaseSize::Pt12,
        Paper::A4,
        390.0,
        592.0,
        31.0,
        31.0,
        20.0,
        35.0,
    ),
    (BaseSize::Pt12, Paper::A5, 276.0, 345.5, 0.0, 0.0, 20.0, 4.0),
    (
        BaseSize::Pt12,
        Paper::Legal,
        390.0,
        766.0,
        39.0,
        40.0,
        17.0,
        44.0,
    ),
];

#[test]
fn article_page_params_match_pdflatex_for_every_size_and_paper() {
    for &(size, paper, tw, th, odd, even, top, mpw) in PROBE {
        let l = sheet(size, paper).page_layout().latex;
        let tag = format!("{} {}", size.name(), paper.name());
        close(l.textwidth.0, tw, 1e-9, &format!("{tag} textwidth"));
        // \textheight = n\baselineskip + \topskip; 13.6pt is not exact in sp.
        close(l.textheight.0, th, 1e-3, &format!("{tag} textheight"));
        close(
            l.oddsidemargin.0,
            odd,
            1e-9,
            &format!("{tag} oddsidemargin"),
        );
        close(
            l.evensidemargin.0,
            even,
            1e-9,
            &format!("{tag} evensidemargin"),
        );
        close(l.topmargin.0, top, 1e-9, &format!("{tag} topmargin"));
        close(
            l.marginparwidth.0,
            mpw,
            1e-9,
            &format!("{tag} marginparwidth"),
        );
        assert_eq!(l.headheight.0, 12.0);
        assert_eq!(l.headsep.0, 25.0);
        assert_eq!(l.footskip.0, 30.0);
    }
}

#[test]
fn paper_sizes_in_tex_points_and_big_points() {
    let (w, h) = Paper::Letter.size();
    close(w.0, 614.295, 1e-9, "letter width pt");
    close(h.0, 794.97, 1e-9, "letter height pt");
    close(w.to_bp(), 612.0, 1e-9, "letter width bp");
    close(h.to_bp(), 792.0, 1e-9, "letter height bp");
    let (w, h) = Paper::A4.size();
    close(w.0, 597.50787, 1e-4, "a4 width pt");
    close(h.0, 845.04684, 1e-4, "a4 height pt");
    let (w, h) = Paper::A5.size();
    close(w.0, 421.10078, 1e-4, "a5 width pt");
    close(h.0, 597.50787, 1e-4, "a5 height pt");
    let (w, h) = Paper::Legal.size();
    close(w.0, 614.295, 1e-9, "legal width pt");
    close(h.0, 1011.78, 1e-9, "legal height pt");
}

#[test]
fn article_12pt_letter_text_area() {
    let page = sheet(BaseSize::Pt12, Paper::Letter).page_layout();
    assert_eq!(page.columns, 1);
    // x = 1in + \oddsidemargin; y = 1in + \topmargin + \headheight + \headsep.
    close(page.text_area.x.0, 72.27 + 39.0, 1e-9, "x");
    close(page.text_area.y.0, 72.27 + 17.0 + 12.0 + 25.0, 1e-9, "y");
    assert_eq!(page.text_area.width.0, 390.0);
    assert_eq!(page.text_area.height.0, 548.5);
    assert_eq!(page.top_skip.0, 12.0);
    close(
        page.first_baseline_y().0,
        126.27 + 12.0,
        1e-9,
        "first baseline",
    );
    let (x, y, w, h) = page.text_area_bp();
    close(x, 111.27 / 1.00375, 1e-9, "x bp");
    close(y, 126.27 / 1.00375, 1e-9, "y bp");
    close(w, 390.0 / 1.00375, 1e-9, "w bp");
    close(h, 548.5 / 1.00375, 1e-9, "h bp");
}

#[test]
fn geometry_margin_one_inch_on_letter() {
    let page = sheet(BaseSize::Pt12, Paper::Letter)
        .with_geometry(Geometry::margin(Pt::inches(1.0)))
        .page_layout();
    // pdflatex: \textwidth=469.75502pt \textheight=650.43001pt
    // \oddsidemargin=0.0pt \topmargin=-37.0pt.
    close(page.latex.textwidth.0, 469.75502, 1e-4, "textwidth");
    close(page.latex.textwidth.to_bp(), 468.0, 1e-9, "textwidth bp");
    close(page.latex.textheight.0, 650.43001, 1e-4, "textheight");
    close(page.latex.oddsidemargin.0, 0.0, 1e-9, "oddsidemargin");
    close(page.latex.topmargin.0, -37.0, 1e-9, "topmargin");
    close(page.text_area.x.0, 72.27, 1e-9, "x");
    close(page.text_area.y.0, 72.27, 1e-9, "y");
    // Same at 10pt: geometry does not depend on the size option.
    let page = sheet(BaseSize::Pt10, Paper::Letter)
        .with_geometry(Geometry::margin(Pt::inches(1.0)))
        .page_layout();
    close(page.latex.textwidth.0, 469.75502, 1e-4, "10pt textwidth");
    close(page.latex.textheight.0, 650.43001, 1e-4, "10pt textheight");
}

#[test]
fn geometry_four_sides_and_a4_cm() {
    let g = Geometry {
        top: Some(Pt::inches(1.0)),
        bottom: Some(Pt::inches(1.0)),
        left: Some(Pt::inches(1.25)),
        right: Some(Pt::inches(1.0)),
        ..Geometry::default()
    };
    let l = sheet(BaseSize::Pt12, Paper::Letter)
        .with_geometry(g)
        .page_layout()
        .latex;
    close(l.textwidth.0, 451.68752, 1e-4, "textwidth");
    close(l.textheight.0, 650.43001, 1e-4, "textheight");
    close(l.oddsidemargin.0, 18.0675, 1e-4, "oddsidemargin");
    close(l.topmargin.0, -37.0, 1e-9, "topmargin");

    let l = sheet(BaseSize::Pt12, Paper::A4)
        .with_geometry(Geometry::margin(Pt::cm(2.0)))
        .page_layout()
        .latex;
    close(l.textwidth.0, 483.69687, 1e-4, "a4 textwidth");
    close(l.textheight.0, 731.23584, 1e-4, "a4 textheight");
    close(l.oddsidemargin.0, -15.36449, 1e-4, "a4 oddsidemargin");
    close(l.topmargin.0, -52.36449, 1e-4, "a4 topmargin");
}

#[test]
fn geometry_partial_specifications_follow_gm_detall() {
    let base = || sheet(BaseSize::Pt12, Paper::Letter);
    // textwidth only: centred 1:1; height defaults to 0.7 paper, 2:3.
    let l = base()
        .with_geometry(Geometry {
            textwidth: Some(Pt(400.0)),
            ..Geometry::default()
        })
        .page_layout()
        .latex;
    close(l.textwidth.0, 400.0, 1e-9, "tw");
    close(l.oddsidemargin.0, 34.8775, 1e-4, "odd");
    close(l.textheight.0, 556.47656, 1e-2, "th");
    close(l.topmargin.0, -13.87262, 1e-2, "top");
    // textwidth + left.
    let l = base()
        .with_geometry(Geometry {
            textwidth: Some(Pt(400.0)),
            left: Some(Pt::inches(1.0)),
            ..Geometry::default()
        })
        .page_layout()
        .latex;
    close(l.oddsidemargin.0, 0.0, 1e-9, "odd");
    // textheight only: top gets 2/5 of the remainder.
    let l = base()
        .with_geometry(Geometry {
            textheight: Some(Pt(600.0)),
            ..Geometry::default()
        })
        .page_layout()
        .latex;
    close(l.textheight.0, 600.0, 1e-9, "th");
    close(l.topmargin.0, -31.28201, 1e-4, "top");
    close(l.textwidth.0, 430.00462, 1e-2, "tw");
    // left only: the right margin keeps its default-body share.
    let l = base()
        .with_geometry(Geometry {
            left: Some(Pt::inches(1.0)),
            ..Geometry::default()
        })
        .page_layout()
        .latex;
    close(l.textwidth.0, 449.87982, 1e-2, "tw");
    close(l.oddsidemargin.0, 0.0, 1e-9, "odd");
    // top only: the bottom margin takes the 2-share (names are swapped in geometry).
    let l = base()
        .with_geometry(Geometry {
            top: Some(Pt::inches(1.0)),
            ..Geometry::default()
        })
        .page_layout()
        .latex;
    close(l.textheight.0, 627.30263, 1e-2, "th");
    close(l.topmargin.0, -37.0, 1e-9, "top");
    // Empty geometry: 0.7 scale both ways.
    let l = base()
        .with_geometry(Geometry::default())
        .page_layout()
        .latex;
    close(l.textwidth.0, 430.00462, 1e-2, "tw");
    close(l.textheight.0, 556.47656, 1e-2, "th");
    close(l.oddsidemargin.0, 19.8752, 1e-2, "odd");
    close(l.topmargin.0, -13.87262, 1e-2, "top");
    // includehead moves the header inside the body.
    let l = base()
        .with_geometry(Geometry {
            includehead: true,
            ..Geometry::margin(Pt::inches(1.0))
        })
        .page_layout()
        .latex;
    close(l.textheight.0, 613.43001, 1e-4, "th");
    close(l.topmargin.0, 0.0, 1e-9, "top");
}

#[test]
fn font_size_table_matches_size_clo_files() {
    let t = |b, n| {
        let f = font_size(b, n);
        (f.size.0, f.baselineskip.0)
    };
    assert_eq!(t(BaseSize::Pt10, SizeName::NormalSize), (10.0, 12.0));
    assert_eq!(t(BaseSize::Pt11, SizeName::NormalSize), (10.95, 13.6));
    assert_eq!(t(BaseSize::Pt12, SizeName::NormalSize), (12.0, 14.5));
    assert_eq!(t(BaseSize::Pt12, SizeName::LARGE2), (17.28, 22.0));
    assert_eq!(t(BaseSize::Pt12, SizeName::Large), (14.4, 18.0));
    assert_eq!(t(BaseSize::Pt12, SizeName::Small), (10.95, 13.6));
    assert_eq!(t(BaseSize::Pt12, SizeName::FootnoteSize), (10.0, 12.0));
    assert_eq!(t(BaseSize::Pt12, SizeName::HUGE2), (24.88, 30.0));
    assert_eq!(t(BaseSize::Pt10, SizeName::LARGE2), (14.4, 18.0));
    assert_eq!(t(BaseSize::Pt10, SizeName::Large), (12.0, 14.0));
    assert_eq!(t(BaseSize::Pt10, SizeName::Tiny), (5.0, 6.0));
    assert_eq!(t(BaseSize::Pt11, SizeName::Tiny), (6.0, 7.0));
    assert_eq!(t(BaseSize::Pt11, SizeName::Huge), (20.74, 25.0));
}

#[test]
fn parindent_parskip_and_font_units() {
    close(
        size_params(BaseSize::Pt10).parindent.0,
        15.0,
        1e-9,
        "10pt parindent",
    );
    close(
        size_params(BaseSize::Pt11).parindent.0,
        17.0,
        1e-9,
        "11pt parindent",
    );
    close(
        size_params(BaseSize::Pt12).parindent.0,
        17.62482,
        1e-4,
        "12pt parindent",
    );
    assert_eq!(PARSKIP, Skip::new(0.0, 1.0, 0.0));
    close(
        size_params(BaseSize::Pt12).normal.x_height.0,
        5.16667,
        1e-9,
        "12pt ex",
    );
    close(
        size_params(BaseSize::Pt12).normal.quad.0,
        11.74988,
        1e-9,
        "12pt em",
    );
    close(
        size_params(BaseSize::Pt10).normal.x_height.0,
        4.30554,
        1e-9,
        "10pt ex",
    );
    close(
        size_params(BaseSize::Pt11).normal.x_height.0,
        4.71457,
        1e-9,
        "11pt ex",
    );
}

#[test]
fn headings_at_12pt() {
    let s = sheet(BaseSize::Pt12, Paper::Letter);
    let h1 = s.resolve(&[Block::Document, Block::Heading(1)]).unwrap();
    assert_eq!(h1.font_size.0, 17.28);
    assert_eq!(h1.baselineskip.0, 22.0);
    assert!(h1.bold && !h1.italic);
    assert!(!h1.first_line_indent);
    // 3.5ex plus 1ex minus .2ex of the body font (ex = 5.16667pt).
    close(h1.space_before.pt, 3.5 * 5.16667, 1e-9, "h1 before");
    close(h1.space_before.plus, 5.16667, 1e-9, "h1 before plus");
    close(
        h1.space_before.minus,
        0.2 * 5.16667,
        1e-9,
        "h1 before minus",
    );
    close(h1.space_after.pt, 2.3 * 5.16667, 1e-9, "h1 after");
    close(h1.space_after.plus, 0.2 * 5.16667, 1e-9, "h1 after plus");
    assert_eq!(h1.run_in_after, None);

    let h2 = s
        .resolve(&[Block::Document, Block::Section(1), Block::Heading(2)])
        .unwrap();
    assert_eq!(h2.font_size.0, 14.4);
    assert_eq!(h2.baselineskip.0, 18.0);
    close(h2.space_before.pt, 3.25 * 5.16667, 1e-9, "h2 before");
    close(h2.space_after.pt, 1.5 * 5.16667, 1e-9, "h2 after");

    let h3 = s.resolve(&[Block::Document, Block::Heading(3)]).unwrap();
    assert_eq!(h3.font_size.0, 12.0);
    assert_eq!(h3.baselineskip.0, 14.5);
    assert!(h3.bold);

    // \paragraph is a run-in heading: 1em after, no vertical after-skip.
    let h4 = s.resolve(&[Block::Document, Block::Heading(4)]).unwrap();
    assert_eq!(h4.space_after, Skip::ZERO);
    close(h4.run_in_after.unwrap().0, 11.74988, 1e-9, "h4 run-in");
    close(h4.space_before.pt, 3.25 * 5.16667, 1e-9, "h4 before");
    assert!(indent_after_heading(4) && !indent_after_heading(1));

    // \subparagraph is additionally indented by \parindent.
    let h5 = s.resolve(&[Block::Document, Block::Heading(5)]).unwrap();
    close(h5.left_margin.0, 17.62482, 1e-4, "h5 indent");

    // Baseline gaps: heading leading + before; body leading + after.
    close(
        s.heading_gap_before(1).pt,
        22.0 + 18.083345,
        1e-6,
        "gap before",
    );
    close(
        s.heading_gap_after(1).pt,
        14.5 + 11.883341,
        1e-6,
        "gap after",
    );
}

#[test]
fn headings_at_10pt_and_11pt_use_smaller_table() {
    let h1 = sheet(BaseSize::Pt10, Paper::Letter)
        .resolve(&[Block::Document, Block::Heading(1)])
        .unwrap();
    assert_eq!((h1.font_size.0, h1.baselineskip.0), (14.4, 18.0));
    close(h1.space_before.pt, 3.5 * 4.30554, 1e-9, "10pt h1 before");
    let h2 = sheet(BaseSize::Pt11, Paper::Letter)
        .resolve(&[Block::Document, Block::Heading(2)])
        .unwrap();
    assert_eq!((h2.font_size.0, h2.baselineskip.0), (12.0, 14.0));
    close(h2.space_before.pt, 3.25 * 4.71457, 1e-9, "11pt h2 before");
}

#[test]
fn paragraph_inherits_size_but_not_heading_spacing() {
    let s = sheet(BaseSize::Pt12, Paper::Letter);
    let p = s
        .resolve(&[Block::Document, Block::Section(1), Block::Paragraph])
        .unwrap();
    assert_eq!(p.font_size.0, 12.0);
    assert_eq!(p.baselineskip.0, 14.5);
    assert!(!p.bold);
    assert_eq!(p.alignment, Alignment::Justified);
    close(p.parindent.0, 17.62482, 1e-4, "parindent");
    assert!(p.first_line_indent);
    // \parskip, not the section's 3.5ex.
    assert_eq!(p.space_before, Skip::new(0.0, 1.0, 0.0));
    assert_eq!(p.space_after, Skip::ZERO);
    assert_eq!(p.left_margin, Pt::ZERO);

    let after = s
        .resolve(&[
            Block::Document,
            Block::Section(1),
            Block::ParagraphAfterHeading,
        ])
        .unwrap();
    assert!(!after.first_line_indent);
    assert_eq!(after.font_size.0, 12.0);

    // Inline styles inherit and toggle.
    let e = s
        .resolve(&[
            Block::Document,
            Block::Paragraph,
            Block::Inline(InlineStyle::Emph),
        ])
        .unwrap();
    assert!(e.italic && !e.bold);
    let ee = s
        .resolve(&[
            Block::Document,
            Block::Paragraph,
            Block::Inline(InlineStyle::Emph),
            Block::Inline(InlineStyle::Emph),
        ])
        .unwrap();
    assert!(!ee.italic);
    let sm = s
        .resolve(&[
            Block::Document,
            Block::Paragraph,
            Block::Inline(InlineStyle::Size(SizeName::Small)),
        ])
        .unwrap();
    assert_eq!((sm.font_size.0, sm.baselineskip.0), (10.95, 13.6));
    assert_eq!(sm.space_before, Skip::ZERO);
}

#[test]
fn list_indents_and_spacing_per_level_at_12pt() {
    let s = sheet(BaseSize::Pt12, Paper::Letter);
    let mut path = vec![Block::Document];
    // pdflatex: \leftmargini=29.3747pt ii=25.84969pt iii=21.97221pt iv=19.97475pt
    let expected = [29.3747, 25.84969, 21.97221, 19.97475];
    let mut cumulative = 0.0;
    for (i, lm) in expected.iter().enumerate() {
        path.push(Block::List(ListKind::Itemize));
        let l = s.resolve(&path).unwrap();
        let list = l.list.expect("list style");
        assert_eq!(list.depth as usize, i + 1);
        close(
            list.leftmargin.0,
            *lm,
            1e-3,
            &format!("leftmargin {}", i + 1),
        );
        cumulative += lm;
        close(
            l.left_margin.0,
            cumulative,
            1e-3,
            &format!("cumulative {}", i + 1),
        );
        close(list.labelsep.0, 5.87494, 1e-4, "labelsep");
        close(
            list.labelwidth.0,
            lm - 5.87494,
            1e-3,
            &format!("labelwidth {}", i + 1),
        );
        assert_eq!(l.parindent, Pt::ZERO);
        assert_eq!(l.font_size.0, 12.0);
        path.push(Block::Item);
    }
    let l1 = list_level(BaseSize::Pt12, 1);
    assert_eq!(l1.topsep, Skip::new(10.0, 4.0, 6.0));
    assert_eq!(l1.parsep, Skip::new(5.0, 2.5, 1.0));
    assert_eq!(l1.itemsep, Skip::new(5.0, 2.5, 1.0));
    assert_eq!(l1.partopsep, Skip::new(3.0, 2.0, 2.0));
    let l2 = list_level(BaseSize::Pt12, 2);
    assert_eq!(l2.topsep, Skip::new(5.0, 2.5, 1.0));
    assert_eq!(l2.parsep, Skip::new(2.5, 1.0, 1.0));
    assert_eq!(l2.itemsep, Skip::new(2.5, 1.0, 1.0));
    let l3 = list_level(BaseSize::Pt12, 3);
    assert_eq!(l3.topsep, Skip::new(2.5, 1.0, 1.0));
    assert_eq!(l3.parsep, Skip::ZERO);
    assert_eq!(l3.itemsep, Skip::new(2.5, 1.0, 1.0));
    assert_eq!(l3.partopsep, Skip::new(1.0, 0.0, 1.0));
    // Level 4 inherits level-3 skips.
    let l4 = list_level(BaseSize::Pt12, 4);
    assert_eq!(
        (l4.topsep, l4.parsep, l4.itemsep, l4.partopsep),
        (l3.topsep, l3.parsep, l3.itemsep, l3.partopsep)
    );

    // Vertical space around a top-level list: \topsep + \parskip.
    let list = s
        .resolve(&[Block::Document, Block::List(ListKind::Enumerate)])
        .unwrap();
    assert_eq!(list.space_before, Skip::new(10.0, 5.0, 6.0));
    assert_eq!(list.space_after, Skip::new(10.0, 5.0, 6.0));
    // Between items: \itemsep + \parsep.
    let item = s
        .resolve(&[
            Block::Document,
            Block::List(ListKind::Enumerate),
            Block::Item,
        ])
        .unwrap();
    assert_eq!(item.space_before, Skip::new(10.0, 5.0, 2.0));
    // A paragraph inside an item: \parsep, no indent.
    let p = s
        .resolve(&[
            Block::Document,
            Block::List(ListKind::Enumerate),
            Block::Item,
            Block::Paragraph,
        ])
        .unwrap();
    assert_eq!(p.space_before, Skip::new(5.0, 2.5, 1.0));
    assert!(!p.first_line_indent);
    close(p.left_margin.0, 29.3747, 1e-3, "item paragraph margin");
}

#[test]
fn list_margins_at_10pt_and_11pt() {
    let l10 = list_level(BaseSize::Pt10, 1);
    close(l10.leftmargin.0, 25.00003, 1e-4, "10pt leftmargini");
    close(l10.labelwidth.0, 20.00003, 1e-4, "10pt labelwidth");
    assert_eq!(l10.topsep, Skip::new(8.0, 2.0, 4.0));
    assert_eq!(l10.itemsep, Skip::new(4.0, 2.0, 1.0));
    close(
        list_level(BaseSize::Pt10, 3).leftmargin.0,
        18.69997,
        1e-3,
        "10pt leftmarginiii",
    );
    let l11 = list_level(BaseSize::Pt11, 1);
    close(l11.leftmargin.0, 27.37506, 1e-4, "11pt leftmargini");
    assert_eq!(l11.topsep, Skip::new(9.0, 3.0, 5.0));
    assert_eq!(l11.parsep, Skip::new(4.5, 2.0, 1.0));
    close(
        list_level(BaseSize::Pt11, 2).leftmargin.0,
        24.09003,
        1e-3,
        "11pt leftmarginii",
    );
}

#[test]
fn center_environment_uses_trivlist_spacing() {
    let s = sheet(BaseSize::Pt12, Paper::Letter);
    let c = s
        .resolve(&[Block::Document, Block::Align(Alignment::Center)])
        .unwrap();
    assert_eq!(c.alignment, Alignment::Center);
    assert_eq!(c.space_before, Skip::new(10.0, 5.0, 6.0));
    assert_eq!(c.left_margin, Pt::ZERO);
    let p = s
        .resolve(&[
            Block::Document,
            Block::Align(Alignment::Center),
            Block::Paragraph,
        ])
        .unwrap();
    assert_eq!(p.alignment, Alignment::Center);
}

#[test]
fn delta_overlay_inherits_and_overrides() {
    let delta = StyleDelta {
        parskip: Some(Skip::fixed(6.0)),
        rules: vec![
            DeltaRule {
                block: Some(Block::Document),
                alignment: Some(Alignment::Left),
                ..DeltaRule::default()
            },
            DeltaRule {
                block: Some(Block::Heading(1)),
                font_size: Some(Pt(20.0)),
                ..DeltaRule::default()
            },
            DeltaRule {
                block: None,
                italic: Some(false),
                ..DeltaRule::default()
            },
        ],
    };
    let s = sheet(BaseSize::Pt12, Paper::Letter).with_delta(delta);
    let p = s
        .resolve(&[Block::Document, Block::Section(1), Block::Paragraph])
        .unwrap();
    assert_eq!(p.alignment, Alignment::Left, "document rule inherits");
    assert_eq!(p.space_before, Skip::fixed(6.0), "parskip override");
    let h = s.resolve(&[Block::Document, Block::Heading(1)]).unwrap();
    assert_eq!(h.font_size.0, 20.0);
    assert_eq!(
        h.baselineskip.0, 22.0,
        "untouched properties keep class values"
    );
    let e = s
        .resolve(&[
            Block::Document,
            Block::Paragraph,
            Block::Inline(InlineStyle::Emph),
        ])
        .unwrap();
    assert!(!e.italic, "wildcard rule applies at every node");
}

#[test]
fn json_round_trip_and_determinism() {
    let s = sheet(BaseSize::Pt11, Paper::A4)
        .with_geometry(Geometry {
            margin: Some(Pt::cm(2.5)),
            top: Some(Pt::inches(1.0)),
            ..Geometry::default()
        })
        .with_delta(StyleDelta {
            parskip: Some(Skip::new(3.0, 1.0, 0.5)),
            rules: vec![DeltaRule {
                block: Some(Block::Paragraph),
                first_line_indent: Some(false),
                space_after: Some(Skip::fixed(2.0)),
                alignment: Some(Alignment::Justified),
                ..DeltaRule::default()
            }],
        });
    let text = s.to_json();
    let back = Stylesheet::from_json(&text).expect("parse");
    assert_eq!(back, s);
    assert_eq!(back.to_json(), text, "re-serialization is stable");
    assert_eq!(
        sheet(BaseSize::Pt11, Paper::A4).to_json(),
        sheet(BaseSize::Pt11, Paper::A4).to_json()
    );
    assert!(text.starts_with("{\n  \"schema\": \"flashtex-document-style/1\""));

    let v = Value::parse(&text).expect("json");
    let export = v.get("export").expect("export");
    let page = export.get("page").expect("page");
    let tw = page
        .get("latex")
        .unwrap()
        .get("textwidth")
        .unwrap()
        .as_f64()
        .unwrap();
    close(
        tw,
        597.50787 - 2.0 * 71.13189,
        1e-3,
        "exported a4 textwidth with 2.5cm margins",
    );
    let styles = export.get("styles").expect("styles");
    assert!(styles.get("document/heading1").is_some());
    assert!(styles.get("document/itemize/item/itemize").is_some());
    assert!(Stylesheet::from_json("{\"schema\":\"other\"}").is_err());
}

#[test]
fn block_names_round_trip() {
    for b in [
        Block::Document,
        Block::Section(2),
        Block::Heading(4),
        Block::Paragraph,
        Block::ParagraphAfterHeading,
        Block::List(ListKind::Description),
        Block::Item,
        Block::Align(Alignment::Right),
        Block::Inline(InlineStyle::Emph),
        Block::Inline(InlineStyle::Size(SizeName::LARGE3)),
    ] {
        assert_eq!(Block::parse(&b.name()), Some(b));
    }
}

#[test]
fn length_parsing() {
    close(Pt::parse("1in").unwrap().0, 72.27, 1e-9, "in");
    close(Pt::parse("2cm").unwrap().0, 56.9055, 1e-4, "cm");
    close(Pt::parse("25mm").unwrap().0, 71.13189, 1e-4, "mm");
    close(Pt::parse("400pt").unwrap().0, 400.0, 1e-9, "pt");
    close(Pt::parse("72bp").unwrap().0, 72.27, 1e-9, "bp");
    close(Pt::parse("1pc").unwrap().0, 12.0, 1e-9, "pc");
    assert!(Pt::parse("2em").is_err());
    assert!(Pt::parse("12").is_err());
    assert_eq!(Pt(39.8775).settopoint().0, 39.0);
    assert_eq!(Pt(-0.5).settopoint().0, 0.0);
}
