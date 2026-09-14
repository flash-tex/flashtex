//! A hyphenated word whose text carries a character with no T1 slot must
//! still be set as one contiguous word.
//!
//! `typeset::word_items` cuts a hyphenatable word at its Liang points and
//! shapes each fragment on its own, keeping the font kern the whole-word
//! shaping had across the point as an explicit kern after the
//! discretionary, so an unbroken word keeps exactly its width. That kern is
//! *whole minus parts*, and each of those three measurements comes back in
//! the units of whichever shaping produced it: `shape` runs the TFM
//! ligature/kern program (2^20 per em) for a string whose characters all
//! have T1 slots and falls back to the font program (the face's own units
//! per em, 1000 for Latin Modern) for one that does not. Two fragments of
//! the same word can therefore arrive in different units.
//!
//! The kern used to divide all three by the units of the *whole* word.
//! `ellipsis\dots` shapes as `ellipsis…;`, so the whole word and the tail
//! `sis…;` came back in the face's 1000 units while the head `lip` came
//! back in 2^20: `3126 − 1158757 − 2014 = −1157645`, divided by 1000 and
//! multiplied by the 10.95 pt size, is a kern of −12676 pt. The line's x
//! cursor took it, so the last five glyphs of `ellipsis` and every word
//! after them on that line were painted at x ≈ −12321 bp — 12300 points off
//! the left edge of the page, where a reader sees nothing at all. It is the
//! `fixtures/real-world/unicode-accents` page's largest defect.
//!
//! The characters that trigger it are not `\dots`-specific: any character
//! with no T1 slot does (U+2026 from `\dots`/`\ldots`/`\textellipsis`,
//! `\dag`, Greek, Cyrillic, CJK), in any hyphenatable word, as long as one
//! fragment is pure T1. Latin-1 letters like `é` are in T1 and never were.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

/// The paragraph of `fixtures/real-world/unicode-accents/main.tex` that
/// caught this, with `%%WORD%%` replaced per case. Two lines long, so the
/// second Liang point of the last long word really is set, not just
/// considered.
const DOC: &str = r"\documentclass[11pt]{article}
\usepackage[T1]{fontenc}
\usepackage[utf8]{inputenc}
\usepackage[margin=1in]{geometry}
\pagestyle{empty}
\begin{document}
\noindent
Five fluffy waffles offer efficient office affluence --- ff, fi, fl, ffi, ffl.
En dash: pages 12--15; em dash --- like this; ``double quotes'' and `single
quotes'; %%WORD%% 25\,\% and \S\,4; \copyright~2026; \pounds 75.
\end{document}
";

/// Every glyph of every run, in order, with the run it belongs to.
fn check(case: &str, word: &str) {
    let r = render_one(&DOC.replace("%%WORD%%", word));
    let mut runs = 0;
    for page in &r.v2.pages {
        let width = page.width.to_bp();
        for it in &page.items {
            let Item::GlyphRun(run) = it else { continue };
            runs += 1;
            for (i, g) in run.glyphs.iter().enumerate() {
                let x = g.origin_x.to_bp();
                assert!(
                    (-1.0..=width + 1.0).contains(&x),
                    "{case}: glyph {i} of run {:?} is painted at x = {x:.2} bp, off a {width:.2} bp page",
                    run.text
                );
                // Fragments of one word are contiguous: the only gaps
                // inside a run are the kern at a hyphenation point and a
                // dropped .notdef, both a fraction of the em.
                if i > 0 {
                    let prev = &run.glyphs[i - 1];
                    let gap = x - prev.origin_x.to_bp() - prev.advance_x.to_bp();
                    assert!(
                        gap.abs() < 12.0,
                        "{case}: run {:?} jumps {gap:.2} bp between glyph {} and {i}",
                        run.text,
                        i - 1
                    );
                }
            }
        }
    }
    assert!(runs > 30, "{case}: only {runs} runs — the paragraph did not typeset");
}

#[test]
fn word_with_a_non_t1_character_stays_on_the_page() {
    if !lm_available() {
        return;
    }
    // `\dots` and its aliases: the construct the defect was found through.
    check("dots", r"ellipsis\dots;");
    check("ldots", r"ellipsis\ldots;");
    check("textellipsis", r"ellipsis\textellipsis;");
    check("leading ellipsis", r"\dots ellipsis;");
    // …and the same defect reached by other characters with no T1 slot,
    // and by other words: the magnitude was the head fragment's TFM width,
    // so it differed per word (`typesetting` gave −13954 bp).
    check("dagger", r"ellipsis\dag;");
    check("greek", "ellipsisα;");
    check("cyrillic", "ellipsisж;");
    check("cjk", "ellipsis東;");
    check("mid-word", "ellip…sisation;");
    check("typesetting", r"typesetting\dots;");
    check("transliteration", r"transliteration\dots;");
    // Controls: all-T1 words, and a word whose ellipsis is separated by a
    // space, were never affected and must stay that way.
    check("control plain", "ellipsist;");
    check("control latin-1", "ellipsisé;");
    check("control spaced", r"ellipsis \dots;");
    check("control periods", "ellipsis...;");
}
