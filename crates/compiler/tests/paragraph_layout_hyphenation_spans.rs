//! Moved from `crates/paragraph-layout/tests/hyphenation_spans.rs` in old-engine retirement
//! stage S1 (#1236), so that paragraph-layout, an oracle crate DESIGN §10 keeps,
//! no longer dev-depends on flashtex-compiler or flashtex-font-engine. Below,
//! "this crate" means `flashtex-paragraph-layout`. These tests retire with
//! `crates/compiler`.
//!
//! Source-identity honesty at a hyphenation point (FT-030 rev 3, item 4).
//!
//! The hazard: a hyphenated word's two halves must each carry a byte span
//! that genuinely slices the ORIGINAL source back to that half's exact
//! text, and the hyphen glyph inserted between them is generated output —
//! it must carry NO source bytes of its own when it was not written by the
//! author (an automatic/pattern hyphenation point). Getting this wrong means
//! click-to-source silently lands on the wrong character. These tests exist
//! because a silent regression here is not something a rendering diff would
//! ever catch.
//!
//! Two hyphenators are exercised:
//! - `ExplicitDiscretionary` (shipped): honours a literal `\-` the author
//!   typed. Its hyphen's span IS real source bytes — the two-byte `\-`
//!   marker itself — which is correct, not a false span: the author put
//!   those bytes there.
//! - A local `FixedAutomaticPoint`, standing in for the "pattern hyphenator"
//!   the crate documents as pluggable (`src/hyphenate.rs`: "returning
//!   `HyphenationPoint { automatic: true, marker_len: 0, .. }`") but does
//!   not ship. Its hyphen has `marker_len: 0`: no source bytes exist for it
//!   at all, so the honest span is empty, and that emptiness is asserted
//!   directly rather than assumed.
//!
//! Assertions key on item *kind* (box / discretionary penalty), not on a
//! hardcoded item index: `ParagraphBuilder::word` inserts an `Item::Kern`
//! between a fragment and the next one whenever the metrics source reports
//! a nonzero kern across the boundary (documented in `README.md` under
//! "Ligature/kern interaction across a discretionary"), so the exact item
//! count is a function of the font's kerning table, not of the hyphenation
//! logic under test here.

use flashtex_font_engine::adapters::paragraph::FaceMetrics;
use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::hyphenate::{ExplicitDiscretionary, HyphenationPoint, Hyphenator};
use flashtex_paragraph_layout::items::{GlyphRun, Item, ParagraphBuilder, Penalty};

/// A single fixed automatic (pattern-hyphenator-shaped) break point, with no
/// marker bytes, standing in for a real dictionary hyphenator. Only
/// hyphenates the exact word it was built for; every other word is left
/// alone, matching how a real per-word pattern lookup would behave.
struct FixedAutomaticPoint {
    word: &'static str,
    offset: usize,
}

impl Hyphenator for FixedAutomaticPoint {
    fn hyphenate(&self, word: &str) -> Vec<HyphenationPoint> {
        if word == self.word {
            vec![HyphenationPoint {
                offset: self.offset,
                marker_len: 0,
                automatic: true,
            }]
        } else {
            Vec::new()
        }
    }
}

/// Splits a builder's items into the boxes and discretionary penalties, in
/// order, ignoring any `Item::Kern` the builder inserted across a
/// fragment/discretionary boundary (see module docs). Panics if a `Glue`
/// shows up, since `ParagraphBuilder::word` never emits one.
fn boxes_and_penalties(items: &[Item]) -> (Vec<&GlyphRun>, Vec<&Penalty>) {
    let mut boxes = Vec::new();
    let mut penalties = Vec::new();
    for item in items {
        match item {
            Item::Box(run) => boxes.push(run),
            Item::Penalty(p) => penalties.push(p),
            Item::Kern(_) => {}
            Item::Glue(_) => panic!("word() must never emit glue"),
        }
    }
    (boxes, penalties)
}

/// Asserts every glyph cluster in `items` (boxes and discretionary hyphens)
/// lands on a UTF-8 boundary of `doc` — i.e. every claimed span is at least
/// well-formed, re-derived from the glyphs themselves rather than trusted
/// from the builder's bookkeeping.
fn assert_no_glyph_cluster_lies(doc: &str, items: &[Item]) {
    for item in items {
        let clusters: Vec<std::ops::Range<usize>> = match item {
            Item::Box(run) => run.glyphs.iter().map(|g| g.cluster.clone()).collect(),
            Item::Penalty(p) => p
                .pre_break
                .as_ref()
                .map(|hy| hy.glyphs.iter().map(|g| g.cluster.clone()).collect())
                .unwrap_or_default(),
            Item::Glue(_) | Item::Kern(_) => Vec::new(),
        };
        for cluster in clusters {
            assert!(
                doc.get(cluster.clone()).is_some(),
                "cluster {cluster:?} does not land on a UTF-8 boundary of the document"
            );
        }
    }
}

#[test]
fn explicit_discretionary_halves_slice_back_to_the_original_word() {
    let doc = "See the un\\-believ\\-able results below.";
    let word = "un\\-believ\\-able";
    let word_start = doc.find(word).unwrap();

    let h = ExplicitDiscretionary;
    let mut b = ParagraphBuilder::new(&h);
    b.word(&Core14Times::ROMAN, 12.0, word, word_start).unwrap();
    let items = b.items().to_vec();
    let (boxes, penalties) = boxes_and_penalties(&items);

    // Three fragments ("un", "believ", "able"), each slicing back exactly.
    assert_eq!(boxes.len(), 3);
    for (b, text) in boxes.iter().zip(["un", "believ", "able"]) {
        assert_eq!(&doc[b.source.clone()], text);
    }

    // Two discretionaries. Unlike the automatic case, the marker IS real
    // source text (the author wrote `\-`), so its span is the two marker
    // bytes, not empty — the correct, honest span here.
    assert_eq!(penalties.len(), 2);
    let expected_marker_starts = [word_start + 2, word_start + 2 + 6 + 2];
    for (p, expected_start) in penalties.iter().zip(expected_marker_starts) {
        assert!(p.flagged);
        assert!(!p.automatic);
        let hy = p
            .pre_break
            .as_ref()
            .expect("discretionary carries a hyphen");
        assert_eq!(hy.glyphs.len(), 1, "the hyphen is one glyph");
        let cluster = hy.glyphs[0].cluster.clone();
        assert_eq!(cluster, expected_start..expected_start + 2);
        assert_eq!(
            &doc[cluster], "\\-",
            "marker span must slice to the literal marker bytes"
        );
    }

    assert_no_glyph_cluster_lies(doc, &items);

    // The fragments plus the two 2-byte markers reconstruct the word
    // exactly: no gap, no overlap, nothing invented.
    let word_end = word_start + word.len();
    assert_eq!(&doc[word_start..word_end], word);
    assert_eq!(
        boxes[0].source.end, expected_marker_starts[0],
        "\"un\" ends exactly where the first marker begins"
    );
    assert_eq!(
        expected_marker_starts[0] + 2,
        boxes[1].source.start,
        "\"believ\" begins exactly where the first marker ends"
    );
    assert_eq!(
        boxes[1].source.end, expected_marker_starts[1],
        "\"believ\" ends exactly where the second marker begins"
    );
    assert_eq!(
        expected_marker_starts[1] + 2,
        boxes[2].source.start,
        "\"able\" begins exactly where the second marker ends"
    );
    assert_eq!(boxes[2].source.end, word_end);
}

#[test]
fn automatic_hyphenation_point_leaves_a_hyphen_with_no_source_span() {
    let doc = "The word believable appears exactly once here.";
    let word = "believable";
    let word_start = doc.find(word).unwrap();
    let break_offset_in_word = "believ".len(); // break after "believ", before "able"

    let h = FixedAutomaticPoint {
        word,
        offset: break_offset_in_word,
    };
    let mut b = ParagraphBuilder::new(&h);
    b.word(&Core14Times::ROMAN, 12.0, word, word_start).unwrap();
    let items = b.items().to_vec();
    let (boxes, penalties) = boxes_and_penalties(&items);

    assert_eq!(boxes.len(), 2, "\"believ\" and \"able\"");
    assert_eq!(&doc[boxes[0].source.clone()], "believ");
    assert_eq!(&doc[boxes[1].source.clone()], "able");

    assert_eq!(penalties.len(), 1);
    let p = penalties[0];
    assert!(p.automatic);

    // The hyphen itself: automatic, marker_len 0, so it must carry an EMPTY
    // span — no source bytes at all, because the author never wrote a
    // hyphen here. This is the exact property the hazard is about: a
    // generated glyph must never claim real source bytes it doesn't own.
    let hy = p
        .pre_break
        .as_ref()
        .expect("automatic point still carries a hyphen glyph");
    assert_eq!(hy.glyphs.len(), 1);
    let cluster = hy.glyphs[0].cluster.clone();
    let expected_point = word_start + break_offset_in_word;
    assert_eq!(
        cluster,
        expected_point..expected_point,
        "automatic hyphen span must be empty, not a false claim on real bytes"
    );
    assert_eq!(
        &doc[cluster], "",
        "an empty span slices to the empty string: no false span"
    );

    assert_no_glyph_cluster_lies(doc, &items);

    // The two halves' spans are exactly adjacent (no gap, no overlap) at the
    // break point, and together reconstruct the whole word with nothing
    // inserted or lost.
    assert_eq!(
        boxes[0].source.end, boxes[1].source.start,
        "halves are contiguous"
    );
    assert_eq!(
        &doc[boxes[0].source.start..boxes[1].source.end],
        word,
        "the two halves concatenate back to the exact original word"
    );
}

/// Same automatic-hyphen honesty property, but measured through the real,
/// already-published font-engine metrics/encoding callback
/// (`flashtex_font_engine::adapters::paragraph::FaceMetrics` implementing
/// this crate's `FontMetricsSource`, see
/// `crates/font-engine/src/adapters/paragraph.rs`) instead of the crate's
/// own bundled `Core14Times` table — the span logic lives entirely in
/// `ParagraphBuilder::word` and does not depend on which metrics source is
/// behind it, and this proves that directly rather than assuming it.
#[test]
fn automatic_hyphenation_span_honesty_holds_through_the_real_font_engine_callback() {
    let doc = "Consider unbreakable words carefully.";
    let word = "unbreakable";
    let word_start = doc.find(word).unwrap();
    let break_offset_in_word = "un".len();

    let times = Core14Face::new(Core14::TimesRoman);
    let metrics = FaceMetrics::new(&times);

    let h = FixedAutomaticPoint {
        word,
        offset: break_offset_in_word,
    };
    let mut b = ParagraphBuilder::new(&h);
    b.word(&metrics, 12.0, word, word_start).unwrap();
    let items = b.items().to_vec();
    let (boxes, penalties) = boxes_and_penalties(&items);

    assert_eq!(boxes.len(), 2);
    assert_eq!(&doc[boxes[0].source.clone()], "un");
    assert_eq!(&doc[boxes[1].source.clone()], "breakable");

    assert_eq!(penalties.len(), 1);
    let hy = penalties[0].pre_break.as_ref().unwrap();
    let cluster = hy.glyphs[0].cluster.clone();
    let point = word_start + break_offset_in_word;
    assert_eq!(cluster, point..point);
    assert_eq!(&doc[cluster], "");

    assert_no_glyph_cluster_lies(doc, &items);
}
