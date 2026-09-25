//! Paragraph-layout conformance harness, slice 1 of #27 (measurement only).
//!
//! The compiler breaks paragraphs greedily (`layout.rs`: "breaking here is
//! greedy": each word wraps with `x + w > right_edge`), while
//! `crates/paragraph-layout` offers TeX-optimal Knuth-Plass total-fit
//! (`layout_paragraph`). Adopting it in production changes every line break,
//! so this slice only MEASURES: it lays out a small fixed corpus of plain
//! prose with the real compiler pipeline (greedy) and with
//! `layout_paragraph` (total-fit), and diffs the chosen break points. It
//! touches no rendering code path and no existing fixture.
//!
//! Metrics parity: the optimal side rebuilds each document's horizontal list
//! with `ParagraphBuilder` over `Core14Times::ROMAN` at 12pt. Those advances
//! were transcribed from the same Adobe AFM data as the compiler's own
//! tables (`core14.rs` provenance note), and both sides apply kerns and the
//! `fi`/`fl` ligatures; hyphenation is off on both sides (`NoHyphenation`;
//! the compiler has none). The corpus avoids apostrophes, the one known
//! deliberate metrics deviation (`quotesingle` 180 vs `quoteright` 333).
//! Measure is the compiler's text column (`PAGE_WIDTH_PT - 2*MARGIN_PT`).

use flashtex_compiler::layout::{self as clayout, BODY_SIZE_PT, MARGIN_PT, PAGE_WIDTH_PT};
use flashtex_compiler::parser;
use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::{
    Glue, LineBreakParams, Lines, NoHyphenation, ParagraphBuilder, layout_paragraph,
};

/// Compiler text-column width in pt.
const MEASURE_PT: f64 = PAGE_WIDTH_PT - 2.0 * MARGIN_PT;

/// Small fixed corpus of plain prose (no LaTeX-special characters, so the
/// compiler parses each as one `Block::Paragraph` of `Inline::Text`, exactly
/// what `ParagraphBuilder::text` consumes).
const CORPUS: &[&str] = &[
    "In 1969 the Apollo 11 mission landed the first humans on the Moon, a milestone that reshaped how people imagined the future of exploration, science, and the long, uncertain relationship between ambition and the fragile machinery that carries it forward.",
    "Supercalifragilisticexpialidocious words occasionally appear in otherwise ordinary paragraphs and they change how a line must wrap, forcing the breaker to reconsider every candidate point rather than simply falling back on the nearest convenient space between two shorter words.",
    "The committee reviewed the quarterly report, noted several discrepancies in the projected revenue figures, and requested a full audit before the next meeting, citing concerns that had already been raised twice in prior sessions without any concrete resolution being reached.",
    "The old lighthouse keeper climbed the spiral staircase every evening to light the lamp that guided ships safely past the rocky coastline, a routine he had kept for thirty years without missing a single night, rain or shine, storm or calm.",
];

/// Line-start byte offsets from the real greedy pipeline. A compiler
/// `TextItem` starts a line exactly when its `x_pt` is the left margin (`x`
/// is reset to `MARGIN_PT` only by `newline`).
fn greedy_line_starts(text: &str) -> Vec<usize> {
    let parsed = parser::parse(text);
    let pages = clayout::layout(&parsed.blocks);
    let mut starts = Vec::new();
    for page in &pages {
        for item in &page.items {
            if (item.x_pt - MARGIN_PT).abs() < 1e-9 {
                starts.push(item.span.start);
            }
        }
    }
    starts
}

fn optimal_layout(text: &str) -> Lines {
    let m = Core14Times::ROMAN;
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    b.text(&m, BODY_SIZE_PT, text, 0).unwrap();
    let items = b.finish(Glue::fil());
    let params = LineBreakParams::article_12pt_letter_1in().with_width(MEASURE_PT);
    layout_paragraph(&items, &params).unwrap()
}

/// Line-start byte offsets from `layout_paragraph` (total-fit): the source
/// offset of the first run of every line.
fn optimal_line_starts(text: &str) -> Vec<usize> {
    optimal_layout(text)
        .lines
        .iter()
        .filter_map(|l| l.runs.first().map(|r| r.source.start))
        .collect()
}

struct Pinned {
    greedy: &'static [usize],
    optimal: &'static [usize],
}

/// Pinned per-document line-start byte offsets for BOTH engines. Exact
/// output, captured by running this harness — the test recomputes them from
/// the two real engines every run and asserts equality against these
/// literals, so a change to either breaker fails here with the new numbers.
const PINNED: [Pinned; 4] = [
    Pinned {
        greedy: &[0, 93, 186],
        optimal: &[0, 97, 194],
    },
    Pinned {
        greedy: &[0, 94, 188],
        optimal: &[0, 98, 195],
    },
    Pinned {
        greedy: &[0, 90, 184],
        optimal: &[0, 98, 197],
    },
    Pinned {
        greedy: &[0, 99, 195],
        optimal: &[0, 99, 202],
    },
];

/// Indices (into [`CORPUS`]) of the documents where greedy and total-fit
/// choose at least one different break point: every document in this corpus.
const DOCS_THAT_DIFFER: &[usize] = &[0, 1, 2, 3];

/// Total count of differing line-index positions across the corpus: docs 0,
/// 1 and 2 disagree at 2 positions each, doc 3 at 1.
const TOTAL_DIFFERING_POSITIONS: usize = 7;

#[test]
fn greedy_vs_total_fit_breaks_pinned() {
    assert_eq!(CORPUS.len(), PINNED.len());
    assert_eq!(MEASURE_PT, 468.0);
    assert_eq!(BODY_SIZE_PT, 12.0);
    let mut docs_differing = Vec::new();
    let mut total_differing_positions = 0usize;

    for (i, text) in CORPUS.iter().enumerate() {
        let g_starts = greedy_line_starts(text);
        let o_starts = optimal_line_starts(text);
        println!("doc {i}: greedy  {g_starts:?}");
        println!("doc {i}: optimal {o_starts:?}");

        assert_eq!(g_starts, PINNED[i].greedy, "doc {i}: greedy breaks changed");
        assert_eq!(
            o_starts, PINNED[i].optimal,
            "doc {i}: total-fit breaks changed"
        );

        let max_len = g_starts.len().max(o_starts.len());
        let diff = (0..max_len)
            .filter(|&idx| g_starts.get(idx) != o_starts.get(idx))
            .count();
        total_differing_positions += diff;
        if diff > 0 {
            docs_differing.push(i);
        }
    }

    println!("docs that differ: {docs_differing:?}");
    println!("differing positions: {total_differing_positions}");
    assert_eq!(docs_differing, DOCS_THAT_DIFFER);
    assert_eq!(total_differing_positions, TOTAL_DIFFERING_POSITIONS);
    assert!(
        !docs_differing.is_empty(),
        "bug #27 no longer reproduces: greedy and total-fit agree everywhere"
    );
}
