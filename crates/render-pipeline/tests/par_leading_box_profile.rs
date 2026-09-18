//! A box argument's interior size must not become the surrounding
//! paragraph's `\baselineskip`, and the pipeline must lay the same document
//! out identically in a debug and a release build (#667).
//!
//! `\colorbox`/`\fcolorbox` parse their body through the compiler's
//! `P::box_inlines`. Before crates/compiler #517 that left one stray entry in
//! `Parsed::block_par_leading` per box — the leading of a paragraph that
//! never reached `Parsed::blocks` — and a stray entry is pushed *before* the
//! block whose paragraph holds the box, so from the first box onwards entry
//! *i* stopped naming block *i*.
//!
//! The pipeline zipped the two lists regardless, behind a `debug_assert_eq!`.
//! Against a `vendor/compiler` pinned before #517 that meant a debug build
//! panicked inside `\fcolorbox` rendering (`tests/frame_env.rs`) while a
//! release build silently gave a paragraph the leading of some box's
//! interior: the same input, two different outcomes, decided by the
//! optimisation level.
//!
//! These tests measure the property in *either* profile, because CI only ever
//! runs `--release`.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const WORDS: &str = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi \
omicron pi rho sigma tau upsilon phi chi psi omega alpha beta gamma delta epsilon zeta eta theta \
iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega";

/// The tightest `\baselineskip` the document's body paragraphs were set on,
/// in ticks.
///
/// Only runs at the document's largest font size count, so a `\small` or
/// `\tiny` run *inside* a box does not perturb the measurement — the point is
/// what the surrounding body text does. A cluster's `hit_rect.top` depends on
/// which letters the run holds (ascenders), so the tops of one line span a
/// few million ticks; `LINE` separates that spread from a real line break,
/// and the first top of each line is the one compared. A
/// paragraph-to-paragraph gap carries `\parskip` on top and is larger, so the
/// minimum over the document is the tightest paragraph's leading — exactly
/// what a stray `block_par_leading` entry moves.
fn line_pitch(src: &str) -> i64 {
    const LINE: i64 = 5_000_000;
    let r = render_one(src);
    let mut runs: Vec<(i64, i64)> = Vec::new();
    for page in &r.v2.pages {
        for item in page.resident_items() {
            if let Item::GlyphRun(run) = item {
                if let Some(c) = run.clusters.first() {
                    runs.push((run.font_size.0, c.hit_rect.top.0));
                }
            }
        }
    }
    let body = runs.iter().map(|(size, _)| *size).max().expect("no glyphs rendered");
    let mut tops: Vec<i64> = runs.iter().filter(|(size, _)| *size == body).map(|(_, top)| *top).collect();
    tops.sort_unstable();
    tops.dedup();
    let mut line_tops = vec![tops[0]];
    for w in tops.windows(2) {
        if w[1] - w[0] > LINE {
            line_tops.push(w[1]);
        }
    }
    let mut pitches: Vec<i64> = line_tops.windows(2).map(|w| w[1] - w[0]).collect();
    pitches.sort_unstable();
    let pitch = *pitches.first().unwrap_or_else(|| panic!("no wrapped paragraph in this document: {tops:?}"));
    // Consecutive line tops round to within a tick of each other, so "the
    // same leading" is a tick's tolerance, not bit equality.
    assert!(
        pitches.iter().filter(|p| (**p - pitch).abs() <= 2).count() >= 2,
        "expected a paragraph of at least three lines set on one leading: {pitches:?}"
    );
    pitch
}

/// Two documents' body paragraphs are set on the same leading.
fn same_pitch(what: &str, a: &str, b: &str) {
    let (pa, pb) = (line_pitch(a), line_pitch(b));
    assert!((pa - pb).abs() <= 2, "{what}: line pitch {pb} is not the body's {pa}");
}

/// `\colorbox{white}{\small x}` inside a body paragraph leaves the
/// paragraph's line pitch alone. TeX reads `\baselineskip` at `\par`, in the
/// outer paragraph's own size; a box argument is a group that has already
/// closed by then, and the box never reaches the vertical list at all.
///
/// Before this fix: fails in a release build (the paragraph picks up the box
/// interior's leading) and panics in a debug build. After it: passes in both.
#[test]
fn a_box_interiors_size_is_not_the_paragraphs_leading() {
    if !lm_available() {
        return;
    }
    let plain = format!("\\usepackage{{xcolor}}\\begin{{document}}x {WORDS}\\end{{document}}");
    let boxed = format!("\\usepackage{{xcolor}}\\begin{{document}}\\colorbox{{white}}{{\\small x}} {WORDS}\\end{{document}}");
    let framed =
        format!("\\usepackage{{xcolor}}\\begin{{document}}\\fcolorbox{{black}}{{white}}{{\\footnotesize x}} {WORDS}\\end{{document}}");
    same_pitch("\\colorbox of \\small", &plain, &boxed);
    same_pitch("\\fcolorbox of \\footnotesize", &plain, &framed);
}

/// The misalignment shifts every *later* block too, not just the one holding
/// the box: a box in the second paragraph used to hand its interior's leading
/// to that paragraph while the first kept the body's.
#[test]
fn a_box_does_not_shift_the_leadings_of_the_blocks_after_it() {
    if !lm_available() {
        return;
    }
    let plain = format!("\\usepackage{{xcolor}}\\begin{{document}}x {WORDS}\n\ny {WORDS}\\end{{document}}");
    let boxed = format!(
        "\\usepackage{{xcolor}}\\begin{{document}}x {WORDS}\n\n\\colorbox{{white}}{{\\tiny y}} {WORDS}\\end{{document}}"
    );
    same_pitch("a \\colorbox of \\tiny in the second paragraph", &plain, &boxed);
}
