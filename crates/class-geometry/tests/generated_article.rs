//! Drift test for the generated `article` class tables.
//!
//! The expected values below are hand-written from documented LaTeX facts
//! (`classes.dtx` / `size1x.clo`: nominal `\textwidth` 345/360/390pt,
//! `\baselineskip` 12/13.6/14.5pt, `\topskip` 10/11/12pt, article's
//! `\headheight`/`\headsep`/`\footskip` 12/25/30pt, `\parskip` 0pt plus 1pt,
//! `\skip\footins` 9/10/10.8pt plus 4pt minus 2pt, `\footnotesep`
//! 6.65/7.7/8.4pt; default US Letter paper 8.5in x 11in), plus small
//! hand derivations noted inline (line counts, margin splits). They are
//! NOT copied from `generated.rs`, so this test fails if the generator
//! mis-parses, swaps sizes, or truncates -- it does not echo the
//! generator's output back at itself. No TeX runs here.

use flashtex_class_geometry::class::{BaseSize, Glue};
use flashtex_class_geometry::generated::{
    article_layout, ARTICLE_10PT, ARTICLE_11PT, ARTICLE_12PT, ARTICLE_TABLES,
};
use flashtex_class_geometry::Sp;

fn sp(s: &str) -> Sp {
    Sp::parse(s).expect("test constant parses")
}

#[test]
fn tables_cover_three_sizes_in_order() {
    assert_eq!(ARTICLE_TABLES.len(), 3);
    assert_eq!(ARTICLE_TABLES[0].size, BaseSize::Pt10);
    assert_eq!(ARTICLE_TABLES[1].size, BaseSize::Pt11);
    assert_eq!(ARTICLE_TABLES[2].size, BaseSize::Pt12);
    assert_eq!(article_layout(BaseSize::Pt10).size, BaseSize::Pt10);
    assert_eq!(article_layout(BaseSize::Pt11).size, BaseSize::Pt11);
    assert_eq!(article_layout(BaseSize::Pt12).size, BaseSize::Pt12);
}

#[test]
fn paper_is_letter_by_default() {
    for t in ARTICLE_TABLES {
        assert_eq!(t.paperwidth, sp("8.5in"), "paperwidth");
        assert_eq!(t.paperheight, sp("11in"), "paperheight");
        // Standard classes never set the PDF media; offsets stay zero.
        assert_eq!(t.hoffset, Sp::ZERO);
        assert_eq!(t.voffset, Sp::ZERO);
    }
}

#[test]
fn nominal_textwidths_per_clo() {
    // size1x.clo: min(paperwidth - 2in, nominal); Letter is wide enough
    // that the nominal 345/360/390pt wins at every size.
    assert_eq!(ARTICLE_10PT.textwidth, Sp::pt(345));
    assert_eq!(ARTICLE_11PT.textwidth, Sp::pt(360));
    assert_eq!(ARTICLE_12PT.textwidth, Sp::pt(390));
}

#[test]
fn body_baselines_and_topskip() {
    assert_eq!(ARTICLE_10PT.baselineskip, sp("12pt"));
    assert_eq!(ARTICLE_11PT.baselineskip, sp("13.6pt"));
    assert_eq!(ARTICLE_12PT.baselineskip, sp("14.5pt"));
    assert_eq!(ARTICLE_10PT.topskip, Sp::pt(10));
    assert_eq!(ARTICLE_11PT.topskip, Sp::pt(11));
    assert_eq!(ARTICLE_12PT.topskip, Sp::pt(12));
    // \maxdepth is .5\topskip (size1x.clo).
    assert_eq!(ARTICLE_10PT.maxdepth, Sp::pt(5));
    assert_eq!(ARTICLE_11PT.maxdepth, sp("5.5pt"));
    assert_eq!(ARTICLE_12PT.maxdepth, Sp::pt(6));
}

#[test]
fn textheight_line_counts() {
    // size1x.clo: whole \baselineskip lines in \paperheight - 3.5in,
    // plus \topskip. 11in - 3.5in = 7.5in holds 45/39/37 lines at
    // 12/13.6/14.5pt (hand division, floor).
    assert_eq!(ARTICLE_10PT.textheight, Sp(786432 * 45 + 655360));
    assert_eq!(ARTICLE_11PT.textheight, Sp(891290 * 39 + 720896));
    assert_eq!(ARTICLE_12PT.textheight, Sp(950272 * 37 + 786432));
    assert_eq!(ARTICLE_10PT.textheight, Sp::pt(550));
}

#[test]
fn article_head_and_foot() {
    // article.cls: \headheight 12pt, \headsep 25pt, \footskip 30pt.
    for t in ARTICLE_TABLES {
        assert_eq!(t.headheight, Sp::pt(12));
        assert_eq!(t.headsep, Sp::pt(25));
        assert_eq!(t.footskip, Sp::pt(30));
    }
}

#[test]
fn oneside_margins_letter() {
    // Oneside: oddsidemargin = (paperwidth - textwidth)/2 - 1in,
    // truncated to whole points; evensidemargin mirrors it.
    // 10pt: (614.295 - 345)/2 - 72.27 = 62.38 -> 62pt, even 62pt.
    assert_eq!(ARTICLE_10PT.oddsidemargin, Sp::pt(62));
    assert_eq!(ARTICLE_10PT.evensidemargin, Sp::pt(62));
    // 11pt: (614.295 - 360)/2 - 72.27 = 54.88 -> 54pt; even 55pt.
    assert_eq!(ARTICLE_11PT.oddsidemargin, Sp::pt(54));
    assert_eq!(ARTICLE_11PT.evensidemargin, Sp::pt(55));
    // 12pt: (614.295 - 390)/2 - 72.27 = 39.88 -> 39pt; even 40pt.
    assert_eq!(ARTICLE_12PT.oddsidemargin, Sp::pt(39));
    assert_eq!(ARTICLE_12PT.evensidemargin, Sp::pt(40));
    // Vertical split of the leftover space, truncated: 16/21/17pt.
    assert_eq!(ARTICLE_10PT.topmargin, Sp::pt(16));
    assert_eq!(ARTICLE_11PT.topmargin, Sp::pt(21));
    assert_eq!(ARTICLE_12PT.topmargin, Sp::pt(17));
}

#[test]
fn parindent_parskip_and_lists() {
    // size1x.clo: 15pt / 17pt / 1.5em (12pt em is 11.74988pt).
    assert_eq!(ARTICLE_10PT.parindent, Sp::pt(15));
    assert_eq!(ARTICLE_11PT.parindent, Sp::pt(17));
    assert_eq!(ARTICLE_12PT.parindent, sp("17.62482pt"));
    // article.cls: \parskip 0pt plus 1pt; \columnsep 10pt, no rule.
    for t in ARTICLE_TABLES {
        assert_eq!(t.parskip, Glue::new("0pt", "1pt", "0pt"));
        assert_eq!(t.columnsep, Sp::pt(10));
        assert_eq!(t.columnseprule, Sp::ZERO);
        assert_eq!(t.overfullrule, Sp::ZERO);
    }
}

#[test]
fn footnote_glue_per_clo() {
    assert_eq!(ARTICLE_10PT.footnotesep, sp("6.65pt"));
    assert_eq!(ARTICLE_11PT.footnotesep, sp("7.7pt"));
    assert_eq!(ARTICLE_12PT.footnotesep, sp("8.4pt"));
    assert_eq!(
        ARTICLE_10PT.skip_footins,
        Glue::new("9pt", "4pt", "2pt")
    );
    assert_eq!(
        ARTICLE_11PT.skip_footins,
        Glue::new("10pt", "4pt", "2pt")
    );
    assert_eq!(
        ARTICLE_12PT.skip_footins,
        Glue::new("10.8pt", "4pt", "2pt")
    );
}

#[test]
fn margin_notes_per_clo() {
    // size1x.clo: \marginparsep 11pt at 10pt else 10pt (oneside, non-book);
    // \marginparpush 7pt at 12pt else 5pt.
    assert_eq!(ARTICLE_10PT.marginparsep, Sp::pt(11));
    assert_eq!(ARTICLE_11PT.marginparsep, Sp::pt(10));
    assert_eq!(ARTICLE_12PT.marginparsep, Sp::pt(10));
    assert_eq!(ARTICLE_10PT.marginparpush, Sp::pt(5));
    assert_eq!(ARTICLE_11PT.marginparpush, Sp::pt(5));
    assert_eq!(ARTICLE_12PT.marginparpush, Sp::pt(7));
}

#[test]
fn body_font_em_ex() {
    // cmr10 at 10pt, cmr10 at 10.95pt, cmr12 at 12pt.
    assert_eq!(ARTICLE_10PT.em, sp("10.00002pt"));
    assert_eq!(ARTICLE_10PT.ex, sp("4.30554pt"));
    assert_eq!(ARTICLE_11PT.em, sp("10.95003pt"));
    assert_eq!(ARTICLE_11PT.ex, sp("4.71457pt"));
    assert_eq!(ARTICLE_12PT.em, sp("11.74988pt"));
    assert_eq!(ARTICLE_12PT.ex, sp("5.16667pt"));
    // Spot-check an em-derived length: 10pt \leftmargini is 2.5em.
    assert_eq!(ARTICLE_10PT.leftmargini, sp("25.00003pt"));
    assert_eq!(ARTICLE_10PT.labelsep, sp("5.0pt"));
}

#[test]
fn no_mathindent_without_fleqn() {
    // Default article never defines \mathindent (fleqn only).
    for t in ARTICLE_TABLES {
        assert_eq!(t.mathindent, None);
    }
}
