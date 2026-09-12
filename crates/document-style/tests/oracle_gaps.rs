//! Baseline-to-baseline gaps measured from pdflatex output (BasicTeX, TeX Live
//! 2026, pdfTeX 1.40.29) with `\pdfsavepos` marks written at shipout, for the
//! document reproduced in README.md ("Oracle check: baseline gaps"). These are
//! the actual positions pdflatex shipped, in TeX points, not class-file
//! arithmetic; the model must reproduce them.

use flashtex_document_style::*;

fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() <= 1e-3, "{what}: got {a}, expected {b}");
}

/// (size, body->section, section->body, body->subsection, subsection->body,
/// body->subsubsection, subsubsection->body, body->first item / item->item /
/// nested list boundaries / last item->body, body->body)
type Measured = (BaseSize, f64, f64, f64, f64, f64, f64, f64, f64);

const MEASURED: &[Measured] = &[
    (
        BaseSize::Pt10,
        33.0694,
        21.9028,
        27.9930,
        18.4583,
        25.9930,
        18.4583,
        20.0,
        12.0,
    ),
    (
        BaseSize::Pt11,
        34.5010,
        24.4435,
        29.3223,
        20.6719,
        28.9223,
        20.6719,
        22.6,
        13.6,
    ),
    (
        BaseSize::Pt12,
        40.0833,
        26.3833,
        34.7917,
        22.2500,
        31.2917,
        22.2500,
        24.5,
        14.5,
    ),
];

#[test]
fn heading_and_list_baseline_gaps_match_pdflatex() {
    for &(size, h1b, h1a, h2b, h2a, h3b, h3a, list, body) in MEASURED {
        let s = Stylesheet::article(ClassOptions {
            paper: Paper::Letter,
            size,
        });
        let tag = size.name();
        close(
            s.heading_gap_before(1).pt,
            h1b,
            &format!("{tag} body->section"),
        );
        close(
            s.heading_gap_after(1).pt,
            h1a,
            &format!("{tag} section->body"),
        );
        close(
            s.heading_gap_before(2).pt,
            h2b,
            &format!("{tag} body->subsection"),
        );
        close(
            s.heading_gap_after(2).pt,
            h2a,
            &format!("{tag} subsection->body"),
        );
        close(
            s.heading_gap_before(3).pt,
            h3b,
            &format!("{tag} body->subsubsection"),
        );
        close(
            s.heading_gap_after(3).pt,
            h3a,
            &format!("{tag} subsubsection->body"),
        );

        let p = s.resolve(&[Block::Document, Block::Paragraph]).unwrap();
        close(
            p.baselineskip.0 + p.space_before.pt,
            body,
            &format!("{tag} body->body"),
        );

        // Body -> first item: \baselineskip + \topsep + \parskip.
        let l = s
            .resolve(&[Block::Document, Block::List(ListKind::Itemize)])
            .unwrap();
        close(
            p.baselineskip.0 + l.space_before.pt,
            list,
            &format!("{tag} body->item"),
        );
        // Item -> item: \baselineskip + \itemsep + \parsep.
        let item = s
            .resolve(&[Block::Document, Block::List(ListKind::Itemize), Block::Item])
            .unwrap();
        close(
            p.baselineskip.0 + item.space_before.pt,
            list,
            &format!("{tag} item->item"),
        );
        // Item -> nested first item: \baselineskip + \topsep(ii) + \parsep(i)
        // (\parskip inside a list is \parsep of the enclosing level).
        let nested = s
            .resolve(&[
                Block::Document,
                Block::List(ListKind::Itemize),
                Block::Item,
                Block::List(ListKind::Itemize),
            ])
            .unwrap();
        let outer = item.list.unwrap();
        let inner = nested.list.unwrap();
        close(
            p.baselineskip.0 + nested.space_before.pt,
            list,
            &format!("{tag} item->nested item"),
        );
        assert_eq!(nested.space_before, inner.topsep.plus(outer.parsep));
        // Nested last item -> outer next item: \addvspace(\topsep(ii)) then
        // \addvspace(\itemsep(i)) keeps the larger, then \parsep(i).
        close(
            p.baselineskip.0 + inner.topsep.pt.max(outer.itemsep.pt) + outer.parsep.pt,
            list,
            &format!("{tag} nested item->item"),
        );
        // Last item -> body: \baselineskip + \topsep + outer \parskip.
        close(
            p.baselineskip.0 + l.space_after.pt,
            list,
            &format!("{tag} item->body"),
        );
    }
}
