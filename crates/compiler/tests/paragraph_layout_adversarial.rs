//! Moved from `crates/paragraph-layout/tests/adversarial.rs` in old-engine retirement
//! stage S1 (#1236), so that paragraph-layout, an oracle crate DESIGN §10 keeps,
//! no longer dev-depends on flashtex-compiler or flashtex-font-engine. Below,
//! "this crate" means `flashtex-paragraph-layout`. These tests retire with
//! `crates/compiler`.
//!
//! Bounded, adversarial and malformed/Unicode input (FT-030 rev 3, item 5).
//!
//! The hard invariant under test throughout this file: nothing here may
//! ever panic, and nothing here may do unbounded work. Some of these inputs
//! are unusual but perfectly valid (an empty paragraph, an overwide
//! unbreakable word, zero-width/combining/RTL-override/NUL characters) and
//! correctly produce `Ok` — turning them into a manufactured `Err` would
//! misrepresent valid input as invalid, which is its own kind of dishonesty
//! this crate's docs otherwise take pains to avoid. Only conditions that
//! cross a genuinely *declared* bound ([`MAX_DIMEN_PT`], [`MAX_ITEMS`]) are
//! typed `Err`. Each test says which outcome it expects and why.
//!
//! Where the real font-engine metrics/encoding callback matters (the
//! Unicode edge cases), tests route through
//! `flashtex_font_engine::adapters::paragraph::FaceMetrics` wrapping a real
//! `Core14Face`, not the crate's own bundled `Core14Times` table, so the
//! callback actually being consumed is exercised under adversarial input,
//! not just under the golden-path text used elsewhere.

use flashtex_font_engine::adapters::paragraph::FaceMetrics;
use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_paragraph_layout::adapter::{
    try_layout_paragraph, LayoutError, MAX_DIMEN_PT, MAX_ITEMS,
};
use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::hyphenate::NoHyphenation;
use flashtex_paragraph_layout::items::{shape_run, Glue, GlyphRun, Item, ParagraphBuilder};
use flashtex_paragraph_layout::linebreak::{Algorithm, LineBreakParams};

fn params(width: f64) -> LineBreakParams {
    LineBreakParams::article_12pt_letter_1in().with_width(width)
}

fn build(
    text: &str,
    font: &dyn flashtex_paragraph_layout::metrics::FontMetricsSource,
) -> Vec<Item> {
    let h = NoHyphenation;
    let mut b = ParagraphBuilder::new(&h);
    b.text(font, 12.0, text, 0).unwrap();
    b.finish(Glue::fil())
}

// ---------------------------------------------------------------------------
// Valid-but-unusual input: must be Ok, never panic.
// ---------------------------------------------------------------------------

#[test]
fn empty_input_is_ok_not_an_error() {
    // No text at all: `ParagraphBuilder::finish` still appends the
    // mandatory `\penalty10000 \parfillskip \penalty-10000`, so this is
    // never truly an empty item list by the time it reaches the breaker.
    let items = build("", &Core14Times::ROMAN);
    let lines = try_layout_paragraph(&items, &params(300.0)).expect("empty paragraph is valid");
    assert_eq!(lines.lines.len(), 1);
    assert!(lines.lines[0].runs.is_empty());

    // Even a bare empty slice (bypassing the builder entirely) must not
    // panic; `layout_paragraph`/`try_layout_paragraph` synthesize the
    // trailing forced break themselves.
    let lines = try_layout_paragraph(&[], &params(300.0)).expect("empty item list is valid");
    assert_eq!(lines.lines.len(), 1);
}

#[test]
fn zero_width_and_combining_characters_do_not_panic() {
    let times = Core14Face::new(Core14::TimesRoman);
    let m = FaceMetrics::new(&times);
    // U+200B ZERO WIDTH SPACE (not builder-recognised whitespace: only
    // ASCII space/tab/newline/CR split words) and U+0301 COMBINING ACUTE
    // ACCENT stacked after a plain "e".
    let text = "caf\u{0065}\u{0301} zero\u{200B}width word e\u{0301}e\u{0301}e\u{0301}";
    let items = build(text, &m);
    let lines = try_layout_paragraph(&items, &params(400.0)).expect("valid Unicode, just unusual");
    assert!(!lines.lines.is_empty());
}

#[test]
fn non_nfc_unicode_does_not_panic() {
    let times = Core14Face::new(Core14::TimesRoman);
    let m = FaceMetrics::new(&times);
    // "café" spelled two ways: NFC (single U+00E9) and NFD (e + combining
    // acute, U+0065 U+0301) side by side. Neither form is special-cased by
    // this crate (no normalization is performed, and none is claimed).
    let nfc = "caf\u{00E9}";
    let nfd = "cafe\u{0301}";
    assert_ne!(nfc, nfd, "the two encodings are literally different bytes");
    let text = format!("{nfc} {nfd}");
    let items = build(&text, &m);
    let lines = try_layout_paragraph(&items, &params(400.0)).expect("valid Unicode, unnormalized");
    assert!(!lines.lines.is_empty());
}

#[test]
fn rtl_override_characters_do_not_panic() {
    let times = Core14Face::new(Core14::TimesRoman);
    let m = FaceMetrics::new(&times);
    // U+202E RIGHT-TO-LEFT OVERRIDE and U+202C POP DIRECTIONAL FORMATTING,
    // plus real Hebrew text. This crate implements no bidi algorithm
    // (documented in README.md under "Not modelled"); the only contract
    // under test here is that it does not panic on these code points, not
    // that the resulting visual order is correct.
    let text = "before \u{202E}\u{05E9}\u{05DC}\u{05D5}\u{05DD}\u{202C} after";
    let items = build(text, &m);
    let lines =
        try_layout_paragraph(&items, &params(400.0)).expect("valid Unicode, bidi unmodelled");
    assert!(!lines.lines.is_empty());
}

#[test]
fn nul_bytes_in_text_do_not_panic() {
    let times = Core14Face::new(Core14::TimesRoman);
    let m = FaceMetrics::new(&times);
    let text = "before\u{0}after \u{0}\u{0}\u{0} more text";
    let items = build(text, &m);
    let lines =
        try_layout_paragraph(&items, &params(400.0)).expect("NUL is a valid Unicode scalar");
    assert!(!lines.lines.is_empty());
}

#[test]
fn a_single_unbreakable_word_far_wider_than_the_line_is_reported_overfull_not_dropped() {
    // 500 'm's at 12pt in Times-Roman-Bold-shaped width terms is vastly
    // wider than a 10pt line and contains no space at all: there is no
    // legal interior break anywhere in the word.
    let times = Core14Face::new(Core14::TimesRoman);
    let m = FaceMetrics::new(&times);
    let word: String = std::iter::repeat_n('m', 500).collect();
    let items = build(&word, &m);
    let lines =
        try_layout_paragraph(&items, &params(10.0)).expect("overfull is reported, not an error");
    assert_eq!(
        lines.lines.len(),
        1,
        "no legal break exists inside the word"
    );
    assert_eq!(lines.stats.overfull.len(), 1);
    assert!(
        lines.stats.overfull[0].excess > 1000.0,
        "500 'm's at 12pt vastly exceeds a 10pt measure: excess = {}",
        lines.stats.overfull[0].excess
    );
    // The run's source span still covers the whole word: content is kept,
    // not truncated, even though it cannot fit.
    assert_eq!(lines.lines[0].runs[0].source, 0..word.len());
}

#[test]
fn absurdly_long_paragraph_completes_without_panic_and_without_dropping_content() {
    let times = Core14Face::new(Core14::TimesRoman);
    let m = FaceMetrics::new(&times);
    let text = std::iter::repeat_n("word", 8_000)
        .collect::<Vec<_>>()
        .join(" ");
    let total_len = text.len();
    let items = build(&text, &m);
    assert!(
        items.len() < MAX_ITEMS,
        "sanity: this fixture stays under the bound"
    );
    let lines = try_layout_paragraph(&items, &params(300.0)).expect("large but bounded input");
    assert!(
        lines.lines.len() > 100,
        "8000 short words wrap across many lines"
    );
    // Every byte of the original text is covered by exactly one run's span;
    // nothing was dropped or duplicated across the huge input.
    let mut covered = 0usize;
    for line in &lines.lines {
        for run in &line.runs {
            covered += run.source.end - run.source.start;
        }
    }
    // "word" x 8000 = 32000 bytes of actual box content; the spaces between
    // words are glue, not boxes, so they are not run-covered bytes.
    assert_eq!(covered, 4 * 8_000);
    assert!(
        total_len > covered,
        "sanity: spaces exist and are not box-covered"
    );
}

// ---------------------------------------------------------------------------
// Declared bounds: at and one past must be a typed Err.
// ---------------------------------------------------------------------------

#[test]
fn item_count_one_past_max_items_is_a_typed_error() {
    let items: Vec<Item> = (0..=MAX_ITEMS).map(|_| Item::kern(0.0)).collect();
    let err = try_layout_paragraph(&items, &params(300.0)).unwrap_err();
    assert_eq!(
        err,
        LayoutError::TooManyItems {
            count: MAX_ITEMS + 1,
            limit: MAX_ITEMS
        }
    );
}

#[test]
fn a_box_dimension_at_max_dimen_from_a_real_metrics_source_is_rejected() {
    // A pathological metrics source with a real font id but an advance so
    // large it sits exactly at MAX_DIMEN once scaled — the kind of value a
    // corrupt or adversarial font file could report through the exact same
    // `FontMetricsSource` callback real fonts use.
    struct HugeAdvance;
    impl flashtex_paragraph_layout::metrics::FontMetricsSource for HugeAdvance {
        fn font_id(&self) -> flashtex_paragraph_layout::metrics::FontId {
            flashtex_paragraph_layout::metrics::FontId::from_label("test:huge")
        }
        fn units_per_em(&self) -> f64 {
            1.0
        }
        fn advance(&self, _ch: char) -> f64 {
            MAX_DIMEN_PT
        }
        fn kern(&self, _l: char, _r: char) -> f64 {
            0.0
        }
        fn glyph_id(&self, _ch: char) -> u32 {
            0
        }
        fn ascender(&self) -> f64 {
            10.0
        }
        fn descender(&self) -> f64 {
            -3.0
        }
        fn line_gap(&self) -> f64 {
            0.0
        }
        fn space(&self) -> f64 {
            2.0
        }
    }
    // size / units_per_em == 1.0, so the glyph's scaled advance equals
    // MAX_DIMEN_PT exactly: at the bound, not one past it.
    let run: GlyphRun = shape_run(&HugeAdvance, 1.0, "x", 0);
    assert_eq!(run.width, MAX_DIMEN_PT);
    let items = vec![
        Item::Box(run),
        Item::penalty(flashtex_paragraph_layout::items::FORCED_BREAK),
    ];
    let err = try_layout_paragraph(&items, &params(300.0)).unwrap_err();
    // `validate_items` checks a box's own `width` before its per-glyph
    // fields, so that is the context reported here — still a genuine
    // rejection of the value this metrics callback produced, not a
    // fabricated one.
    assert_eq!(
        err,
        LayoutError::DimensionOverflow {
            value: MAX_DIMEN_PT,
            context: "box.width"
        }
    );
}

// ---------------------------------------------------------------------------
// Penalty-value bounds (`INFINITE_PENALTY` forbids, `FORCED_BREAK` forces):
// exact boundary and one past it.
// ---------------------------------------------------------------------------

use flashtex_paragraph_layout::items::{Penalty, FORCED_BREAK, INFINITE_PENALTY};

fn box_of(text: &str) -> Item {
    Item::Box(shape_run(&Core14Times::ROMAN, 12.0, text, 0))
}

fn penalty(value: i32) -> Item {
    Item::Penalty(Penalty {
        value,
        flagged: false,
        pre_break: None,
        automatic: false,
        post_break: None,
        replace_count: 0,
    })
}

#[test]
fn penalty_at_infinite_penalty_forbids_the_only_candidate_break() {
    // Two long, space-free runs with a bare penalty between them and NO
    // glue anywhere: the penalty is the ONLY place a break could legally
    // happen. A width that fits either half alone but not both together
    // makes "was this break legal" directly observable as
    // one-overfull-line vs two-clean-lines.
    let half = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"; // 41 chars
    let half_width = shape_run(&Core14Times::ROMAN, 12.0, half, 0).width;
    // Set the measure to exactly one half's width: a line ending at the
    // penalty (or at the paragraph's own forced end) is then either an
    // exact-fit (ratio 0, badness 0) or grossly overfull, with nothing in
    // between to blur the legal/forbidden distinction.
    let width = half_width;

    for forbidden in [INFINITE_PENALTY, INFINITE_PENALTY + 1] {
        let items = vec![
            box_of(half),
            penalty(forbidden),
            box_of(half),
            Item::penalty(FORCED_BREAK),
        ];
        let lines = try_layout_paragraph(&items, &params(width)).unwrap();
        assert_eq!(
            lines.lines.len(),
            1,
            "penalty {forbidden} must not be a legal break point"
        );
        assert_eq!(
            lines.stats.overfull.len(),
            1,
            "forced onto one overfull line"
        );
    }

    let items = vec![
        box_of(half),
        penalty(INFINITE_PENALTY - 1),
        box_of(half),
        Item::penalty(FORCED_BREAK),
    ];
    let lines = try_layout_paragraph(&items, &params(width)).unwrap();
    assert_eq!(
        lines.lines.len(),
        2,
        "penalty 9999 is one under the bound: it is a legal break"
    );
    assert!(
        lines.stats.overfull.is_empty(),
        "clean break, nothing overfull"
    );
}

#[test]
fn penalty_at_forced_break_always_splits_even_when_it_fits() {
    let a = "aaaa";
    let b = "bbbb";
    let combined_width = shape_run(&Core14Times::ROMAN, 12.0, &format!("{a}{b}"), 0).width;
    let generous_width = combined_width * 4.0; // both fit on one line easily

    for forced in [FORCED_BREAK, FORCED_BREAK - 1] {
        let items = vec![
            box_of(a),
            penalty(forced),
            box_of(b),
            Item::penalty(FORCED_BREAK),
        ];
        let lines = try_layout_paragraph(&items, &params(generous_width)).unwrap();
        assert_eq!(
            lines.lines.len(),
            2,
            "penalty {forced} forces a break even though the content fits on one line"
        );
    }

    // An ordinary, non-forcing, non-flagged zero-cost penalty in the same
    // spot: nothing compels a split, and splitting a paragraph that fits
    // easily into two short, badly-filled lines is strictly worse under
    // total-fit's demerits, so the breaker keeps it on one line.
    let items = vec![
        box_of(a),
        penalty(0),
        box_of(b),
        Item::penalty(FORCED_BREAK),
    ];
    let lines = try_layout_paragraph(&items, &params(generous_width)).unwrap();
    assert_eq!(
        lines.lines.len(),
        1,
        "an ordinary penalty of 0 does not force an unneeded split"
    );
}

#[test]
fn first_fit_also_respects_the_penalty_bounds() {
    // The declared bounds are properties of `is_legal_break`/`penalty_value`
    // shared by both algorithms, not just total-fit; spot-check first-fit.
    let half = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    let half_width = shape_run(&Core14Times::ROMAN, 12.0, half, 0).width;
    let width = half_width * 1.5;
    let mut p = params(width);
    p.algorithm = Algorithm::FirstFit;

    let items = vec![
        box_of(half),
        penalty(INFINITE_PENALTY),
        box_of(half),
        Item::penalty(FORCED_BREAK),
    ];
    let lines = try_layout_paragraph(&items, &p).unwrap();
    assert_eq!(lines.lines.len(), 1);
    assert_eq!(lines.stats.overfull.len(), 1);

    let items = vec![
        box_of(half),
        penalty(INFINITE_PENALTY - 1),
        box_of(half),
        Item::penalty(FORCED_BREAK),
    ];
    let lines = try_layout_paragraph(&items, &p).unwrap();
    assert_eq!(lines.lines.len(), 2);
}
