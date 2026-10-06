//! Moved from `crates/paragraph-layout/tests/mismatch_fixtures.rs` in old-engine retirement
//! stage S1 (#1236), so that paragraph-layout, an oracle crate DESIGN §10 keeps,
//! no longer dev-depends on flashtex-compiler or flashtex-font-engine. Below,
//! "this crate" means `flashtex-paragraph-layout`. These tests retire with
//! `crates/compiler`.
//!
//! Pinned paragraph break/position mismatch fixtures (FT-030 rev 3, items 3
//! and 7): reproducible, exact evidence of where this crate's line breaking
//! differs from what the real consumer — `crates/compiler` — currently
//! produces, on the same 14-document corpus, the same font (Times-Roman,
//! Adobe Core 14, consumed through the real, already-published
//! `flashtex_font_engine::adapters::paragraph::FaceMetrics` callback — see
//! `crates/font-engine/src/adapters/paragraph.rs:26`, not this crate's own
//! bundled `Core14Times` table), the same size (12pt) and the same measure
//! (468pt = the compiler's `PAGE_WIDTH_PT - 2*MARGIN_PT`,
//! `crates/compiler/src/layout.rs:24-27`).
//!
//! This is a claim about ALGORITHM, not about the font stack: hyphenation
//! is off on both sides (`NoHyphenation`; the compiler has no hyphenation
//! at all — `crates/compiler/src/layout.rs:6`, `"breaking here is greedy"`),
//! so every difference below comes from this crate's Knuth-Plass total-fit
//! optimizing globally across a whole paragraph versus the compiler's
//! greedy `x + w > right_edge` word-at-a-time wrap
//! (`crates/compiler/src/layout.rs:322`). Neither breaker is TeX itself, and
//! nothing here claims parity with TeX or with the compiler: it pins the
//! actual, current numeric divergence between this crate and its real
//! consumer, byte-for-byte, so a future change to either breaker is caught
//! by a failing assertion instead of a vague "it looks different".
//!
//! ## Method
//!
//! - `compiler_line_starts`: parses each document with `flashtex_compiler`'s
//!   real parser and lays it out with its real (greedy) `layout::layout`,
//!   then reads off the byte offset of the first word of every line — a
//!   compiler `TextItem` starts a line exactly when its `x_pt` equals the
//!   left margin (`crates/compiler/src/layout.rs`: `x` is reset to
//!   `MARGIN_PT` only by `newline`), so `x_pt == 72.0` is that predicate
//!   here.
//! - `mine_line_starts`: builds the same text into this crate's `Item`s via
//!   `ParagraphBuilder` (real font-engine metrics, `NoHyphenation`), runs
//!   `layout_paragraph` at the same size/measure, and reads the byte offset
//!   of the first run of every `Line`.
//! - A "differing break position" is an index `i` (0-based line number)
//!   where the two engines' line-start byte offsets disagree, including
//!   where one has a line the other doesn't. Every document below produces
//!   the same NUMBER of lines from both engines (this corpus does not
//!   happen to exercise a case where the line COUNT itself differs), so
//!   every disagreement here is a shifted break point, not an added or
//!   dropped line.
//!
//! Both engines are already documented and tested deterministic
//! (`tests/golden.rs::layout_is_deterministic` on this crate's side); this
//! file adds no randomness or timing of its own, so a rerun reproduces
//! every number below exactly.

use flashtex_compiler::layout as clayout;
use flashtex_compiler::parser;
use flashtex_font_engine::adapters::paragraph::FaceMetrics;
use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_paragraph_layout::hyphenate::NoHyphenation;
use flashtex_paragraph_layout::items::{Glue, ParagraphBuilder};
use flashtex_paragraph_layout::linebreak::{layout_paragraph, LineBreakParams, Lines};

/// `crates/compiler/src/layout.rs:23`.
const MARGIN_PT: f64 = 72.0;
/// `crates/compiler/src/layout.rs:22,24`: `PAGE_WIDTH_PT - 2.0 * MARGIN_PT`.
const MEASURE_PT: f64 = 612.0 - 2.0 * MARGIN_PT;
/// `crates/compiler/src/layout.rs:26`.
const BODY_SIZE_PT: f64 = 12.0;

fn compiler_line_starts(text: &str) -> Vec<usize> {
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

fn mine_layout(text: &str) -> Lines {
    let times = Core14Face::new(Core14::TimesRoman);
    let m = FaceMetrics::new(&times);
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    b.text(&m, BODY_SIZE_PT, text, 0).unwrap();
    let items = b.finish(Glue::fil());
    let params = LineBreakParams::article_12pt_letter_1in().with_width(MEASURE_PT);
    layout_paragraph(&items, &params).unwrap()
}

fn mine_line_starts(text: &str) -> Vec<usize> {
    mine_layout(text)
        .lines
        .iter()
        .filter_map(|l| l.runs.first().map(|r| r.source.start))
        .collect()
}

/// 14 documents: a mix of short one-liners, ordinary multi-sentence prose,
/// long compound words, and dense short-word runs, all plain ASCII/Unicode
/// prose with no LaTeX-special characters (so the compiler's parser treats
/// every one of them as a single `Block::Paragraph` of plain `Inline::Text`
/// words, exactly like this crate's `ParagraphBuilder::text`).
const CORPUS: &[&str] = &[
    "The quick brown fox jumps over the lazy dog near the riverbank every single morning before the sun rises completely, leaving long shadows across the wet grass and the gravel path that winds down toward the old stone bridge.",
    "In 1969 the Apollo 11 mission landed the first humans on the Moon, a milestone that reshaped how people imagined the future of exploration, science, and the long, uncertain relationship between ambition and the fragile machinery that carries it forward.",
    "Short line here.",
    "Supercalifragilisticexpialidocious words occasionally appear in otherwise ordinary paragraphs and they change how a line must wrap, forcing the breaker to reconsider every candidate point rather than simply falling back on the nearest convenient space between two shorter words.",
    "A watched pot never boils, or so the old saying goes, yet every kitchen in every home keeps at least one person waiting anyway, tapping a spoon against the counter, glancing at the clock, and wondering whether patience is really a virtue or just a habit nobody questions.",
    "The committee reviewed the quarterly report, noted several discrepancies in the projected revenue figures, and requested a full audit before the next meeting, citing concerns that had already been raised twice in prior sessions without any concrete resolution being reached.",
    "Rain fell steadily through the afternoon while commuters hurried along the crowded sidewalks searching for shelter under awnings and bus stops, their umbrellas colliding in the narrow spaces between storefronts and the slow-moving line of taxis idling at the curb.",
    "Whales communicate across vast distances using low frequency calls that can travel for hundreds of miles through the deep ocean water, a phenomenon that researchers only began to document seriously once underwater listening equipment became sensitive enough to register it.",
    "One two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twenty-one twenty-two twenty-three twenty-four twenty-five twenty-six twenty-seven twenty-eight twenty-nine thirty.",
    "The old lighthouse keeper climbed the spiral staircase every evening to light the lamp that guided ships safely past the rocky coastline, a routine he had kept for thirty years without missing a single night, rain or shine, storm or calm.",
    "Extraordinarily long compound words like internationalization and disestablishmentarianism can force a justified line to stretch or shrink dramatically, which is exactly the kind of stress case a line breaker's stretch and shrink parameters are meant to absorb gracefully.",
    "She sold seashells by the seashore while the tide slowly crept up across the warm afternoon sand toward the abandoned pier, scattering gulls that had settled near the old wooden posts left over from a boardwalk nobody remembered building.",
    "Machine learning models require large amounts of carefully labeled training data before they can make reliable predictions on new examples, and the cost of collecting that data honestly, without shortcuts, is usually far higher than teams initially budget for.",
    "A single word.",
];

/// Pinned per-document line-start byte offsets for BOTH engines, and the
/// count of differing positions within that document (0 when the two
/// engines agree on every break). Index matches [`CORPUS`]. These are exact
/// output, captured by running this fixture once and transcribed verbatim —
/// not independently re-derived by hand — which is why the test below
/// recomputes them from the two real engines every run and asserts equality
/// against these literals, rather than trusting the literals on their own.
struct Pinned {
    compiler: &'static [usize],
    mine: &'static [usize],
}

const PINNED: [Pinned; 14] = [
    Pinned {
        compiler: &[0, 95, 190],
        mine: &[0, 95, 190],
    },
    Pinned {
        compiler: &[0, 93, 186],
        mine: &[0, 97, 194],
    },
    Pinned {
        compiler: &[0],
        mine: &[0],
    },
    Pinned {
        compiler: &[0, 94, 188],
        mine: &[0, 98, 195],
    },
    Pinned {
        compiler: &[0, 95, 192],
        mine: &[0, 95, 192],
    },
    Pinned {
        compiler: &[0, 90, 184],
        mine: &[0, 98, 197],
    },
    Pinned {
        compiler: &[0, 93, 191],
        mine: &[0, 93, 191],
    },
    Pinned {
        compiler: &[0, 96, 188],
        mine: &[0, 96, 188],
    },
    Pinned {
        compiler: &[0, 97, 191],
        mine: &[0, 97, 191],
    },
    Pinned {
        compiler: &[0, 99, 195],
        mine: &[0, 99, 202],
    },
    Pinned {
        compiler: &[0, 95, 196],
        mine: &[0, 95, 201],
    },
    Pinned {
        compiler: &[0, 97, 194],
        mine: &[0, 97, 194],
    },
    Pinned {
        compiler: &[0, 97, 195],
        mine: &[0, 97, 195],
    },
    Pinned {
        compiler: &[0],
        mine: &[0],
    },
];

/// The exact set of documents (0-based index into [`CORPUS`]) where the two
/// engines choose at least one different break point.
const DOCS_THAT_DIFFER: [usize; 5] = [1, 3, 5, 9, 10];

/// Sum, across all 14 documents, of the per-document count of line indices
/// where the two engines' line-start byte offsets disagree. Reported as a
/// count, not an adjective: **5 of 14 documents break differently, 8
/// differing break positions total** (documents 1, 3, 5: 2 differing
/// positions each; documents 9, 10: 1 each; the other 9 documents: 0).
const TOTAL_DIFFERING_POSITIONS: usize = 8;

#[test]
fn pinned_break_positions_match_both_engines_exactly() {
    assert_eq!(CORPUS.len(), PINNED.len());
    let mut docs_differing = Vec::new();
    let mut total_differing_positions = 0usize;

    for (i, text) in CORPUS.iter().enumerate() {
        let c_starts = compiler_line_starts(text);
        let m_starts = mine_line_starts(text);

        // Pin the exact arrays: a change in either engine's output for this
        // corpus fails here, not just in the aggregate counts below.
        assert_eq!(
            c_starts, PINNED[i].compiler,
            "doc {i}: compiler line starts changed"
        );
        assert_eq!(
            m_starts, PINNED[i].mine,
            "doc {i}: this crate's line starts changed"
        );

        let max_len = c_starts.len().max(m_starts.len());
        let diff_positions = (0..max_len)
            .filter(|&idx| c_starts.get(idx) != m_starts.get(idx))
            .count();
        total_differing_positions += diff_positions;
        if diff_positions > 0 {
            docs_differing.push(i);
        }
    }

    assert_eq!(
        docs_differing, DOCS_THAT_DIFFER,
        "the set of documents that break differently changed"
    );
    assert_eq!(
        docs_differing.len(),
        5,
        "5 of {} documents break differently",
        CORPUS.len()
    );
    assert_eq!(total_differing_positions, TOTAL_DIFFERING_POSITIONS);
}

/// A worked glyph-position mismatch (not just a break index), pinned to
/// exact numbers in points, for document 1's word "how" (source bytes
/// 93..96): the compiler places it as the FIRST word of its own line 2 (so
/// at its left margin), while this crate keeps it as the LAST word of line
/// 1 (justified interword shrink lets total-fit fit one more word than the
/// compiler's fixed-width greedy wrap does).
///
/// Coordinate frames: `compiler`'s `x_pt`/`baseline_y_pt` are already
/// page-absolute. `layout_paragraph`'s `PositionedRun.x` is paragraph-frame
/// (from the left edge of the measure, `left_skip = 0pt` here, which is the
/// same left edge as the compiler's text column), so `MARGIN_PT + run.x` is
/// the directly comparable page-absolute x. Baseline `y` is deliberately
/// NOT converted or compared: the compiler places lines with a fixed 1.2x
/// leading (`LINE_SPACING`, `crates/compiler/src/layout.rs:26`), while this
/// crate places them with TeX's baselineskip/lineskip/lineskip-limit
/// rule (`src/linebreak.rs` module docs) — two different, non-analogous
/// vertical models, not a frame offset. Comparing them numerically would be
/// exactly the "implicit approximation" this task's acceptance criteria
/// rules out, so this fixture states plainly that vertical position is not
/// compared here rather than manufacturing a number for it.
#[test]
fn pinned_glyph_position_mismatch_for_doc1_word_how() {
    let text = CORPUS[1];
    let word_span = 93..96;
    assert_eq!(&text[word_span.clone()], "how");

    // Compiler: "how" starts its own line, at the page's left margin.
    let parsed = parser::parse(text);
    let pages = clayout::layout(&parsed.blocks);
    let compiler_item = pages
        .iter()
        .flat_map(|p| &p.items)
        .find(|item| item.span.start == word_span.start)
        .expect("\"how\" is placed exactly once");
    assert_eq!(compiler_item.text, "how");
    assert_eq!(compiler_item.x_pt, 72.0);
    assert_eq!(compiler_item.baseline_y_pt, 98.4);

    // This crate: "how" is the last run on line 0 (index 0), not a line
    // start at all.
    let lines = mine_layout(text);
    let (line_index, run) = lines
        .lines
        .iter()
        .enumerate()
        .find_map(|(i, l)| {
            l.runs
                .iter()
                .find(|r| r.source == word_span)
                .map(|r| (i, r))
        })
        .expect("\"how\" is placed exactly once");
    assert_eq!(
        line_index, 0,
        "\"how\" is on this crate's first line, not its second"
    );
    assert_eq!(run.x, 447.6360000000001);
    assert_eq!(run.baseline_y, 8.196);

    // Directly comparable (page-absolute) horizontal positions: the same
    // source bytes land 447.6360000000001pt apart horizontally, and on
    // different lines entirely.
    let compiler_page_x = compiler_item.x_pt;
    let mine_page_x = MARGIN_PT + run.x;
    assert_eq!(compiler_page_x, 72.0);
    assert_eq!(mine_page_x, 519.6360000000001);
    assert_eq!(mine_page_x - compiler_page_x, 447.6360000000001);
}
