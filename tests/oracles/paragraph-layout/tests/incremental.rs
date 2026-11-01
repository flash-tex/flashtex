//! Property: `relayout` after any edit equals a clean `layout_document`.

use flashtex_paragraph_layout::core14::Core14Times;
use flashtex_paragraph_layout::document::{
    DocumentLayout, DocumentSpec, layout_document, relayout,
};
use flashtex_paragraph_layout::*;

const BASE: &str = "Reproducibility and verification are the characteristic responsibilities of any typographical documentation effort, and the international community expects deliberately measured compilation results.\n\nDeliberately unbalanced paragraphs demonstrate emergency stretchability: extraordinarily incomprehensible terminology complicates justification considerably.\n\nShort last paragraph with fine coffee and an em dash \u{2014} done.\n\nA fourth paragraph so that edits in the middle leave paragraphs on both sides untouched, with re\\-pro\\-ducible words.";

fn spec<'a>(text: &'a str, hyph: &'a LiangHyphenator) -> DocumentSpec<'a> {
    DocumentSpec {
        text,
        font: &Core14Times::ROMAN,
        size: 12.0,
        hyphenator: hyph,
        line: LineBreakParams::article_12pt_letter_1in().with_width(200.0),
        page: PageParams::article_12pt_letter_1in_tex_pt(),
    }
}

/// Deterministic LCG so the property is reproducible.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

fn same(a: &DocumentLayout, b: &DocumentLayout) -> bool {
    a.paragraphs == b.paragraphs && a.pages == b.pages
}

/// Picks a char-boundary-aligned edit and a replacement drawn from a small
/// alphabet that includes spaces, newlines and a discretionary marker.
fn random_edit(rng: &mut Lcg, text: &str) -> (std::ops::Range<usize>, String) {
    let boundaries: Vec<usize> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .collect();
    let a = boundaries[rng.below(boundaries.len())];
    let span = rng.below(12);
    let b_idx = boundaries.iter().position(|&x| x >= a).unwrap();
    let b = boundaries[(b_idx + span).min(boundaries.len() - 1)];
    let pieces = [
        "",
        "a",
        "verification ",
        " ",
        "\n\n",
        "\n",
        "xyz",
        "counterproductive",
        "\\-",
        "é",
        ", ",
        "hyphenation ",
    ];
    let mut rep = String::new();
    for _ in 0..rng.below(3) {
        rep.push_str(pieces[rng.below(pieces.len())]);
    }
    (a..b, rep)
}

#[test]
fn relayout_equals_clean_layout_over_random_edits() {
    let hyph = LiangHyphenator::en_us_subset();
    let mut rng = Lcg(0x5eed_2026);
    let mut text = BASE.to_string();
    let mut previous = layout_document(&spec(&text, &hyph));
    let mut reuse_events = 0;
    for step in 0..300 {
        let (edit, rep) = random_edit(&mut rng, &text);
        let mut next = text.clone();
        next.replace_range(edit.clone(), &rep);
        let s = spec(&next, &hyph);
        let incremental = relayout(&previous, &s, edit.clone(), rep.len());
        let clean = layout_document(&s);
        assert!(
            same(&incremental, &clean),
            "step {step}: edit {edit:?} -> {rep:?} produced a different layout\n{:?}\nvs\n{:?}",
            incremental.stats,
            clean.stats
        );
        assert_eq!(
            incremental.stats.reused_before
                + incremental.stats.reused_after
                + incremental.stats.relaid,
            clean.paragraphs.len()
        );
        if incremental.stats.reused_before + incremental.stats.reused_after > 0 {
            reuse_events += 1;
        }
        text = next;
        previous = incremental;
    }
    assert!(
        reuse_events > 200,
        "reuse happened in only {reuse_events} of 300 steps"
    );
}

/// An edit inside one paragraph re-breaks only that paragraph; the others are
/// reused (the ones after it with shifted offsets), and every source range in
/// the reused paragraphs points at the same text as before.
#[test]
fn single_paragraph_edit_relays_one_paragraph() {
    let hyph = LiangHyphenator::en_us_subset();
    let first = layout_document(&spec(BASE, &hyph));
    assert_eq!(first.paragraphs.len(), 4);
    let at = BASE.find("emergency").unwrap();
    let mut edited = BASE.to_string();
    edited.replace_range(at..at + 9, "very urgent");
    let s = spec(&edited, &hyph);
    let inc = relayout(&first, &s, at..at + 9, 11);
    assert_eq!(
        (
            inc.stats.reused_before,
            inc.stats.relaid,
            inc.stats.reused_after
        ),
        (1, 1, 2)
    );
    assert!(same(&inc, &layout_document(&s)));
    for p in &inc.paragraphs[2..] {
        assert_eq!(&edited[p.source.clone()], p.text);
        for line in &p.lines.lines {
            for run in &line.runs {
                if !run.is_hyphen {
                    assert_eq!(
                        &edited[run.source.clone()],
                        &p.text[run.source.start - p.source.start..run.source.end - p.source.start]
                    );
                }
            }
        }
    }
}
