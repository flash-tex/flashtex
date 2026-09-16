//! Margin notes: `\marginpar{note}` sets the note in the right margin at
//! `\footnotesize`, `marginparwidth` wide and `marginparsep` past the text
//! block, its top aligned with the calling line and clipped to the page.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, Tick, TICKS_PER_BP};
use flashtex_render_pipeline::Rendered;

/// article 10pt on Letter, one-sided (class-geometry, exact to the sp):
/// text block at 134.27pt (1in + `\oddsidemargin` 62pt), `\textwidth` 345pt,
/// `\marginparsep` 11pt, `\marginparwidth` 65pt. Display ticks are big
/// points, so expectations convert TeX points at 72/72.27.
const BP: f64 = 72.0 / 72.27;
const TEXT_RIGHT_TEXPT: f64 = 134.27 + 345.0;
const NOTE_X_TEXPT: f64 = TEXT_RIGHT_TEXPT + 11.0;
const NOTE_WIDTH_TEXPT: f64 = 65.0;

fn body(text: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{text}\n\\end{{document}}\n")
}

struct Run {
    text: String,
    x: f64,
    baseline: f64,
    size: Tick,
    width: f64,
}

fn runs(r: &Rendered) -> Vec<Run> {
    let mut out = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let Item::GlyphRun(g) = it {
                let Some(first) = g.glyphs.first() else { continue };
                let last = g.glyphs.last().expect("non-empty run");
                out.push(Run {
                    text: g.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    size: g.font_size,
                    width: (last.origin_x.0 + last.advance_x.0 - first.origin_x.0) as f64 / TICKS_PER_BP,
                });
            }
        }
    }
    out
}

fn note_runs(runs: &[Run]) -> Vec<&Run> {
    runs.iter().filter(|r| r.text == "side" || r.text == "note").collect()
}

#[test]
fn margin_note_sits_marginparsep_past_the_text_block_within_marginparwidth() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&body("First words\\marginpar{side note} and the rest of the paragraph continue here."));
    assert!(
        r.v2.diagnostics.iter().all(|d| !d.message.contains("marginpar") && d.code != "unsupported_block"),
        "unexpected diagnostics: {:?}",
        r.v2.diagnostics.iter().map(|d| (d.code.clone(), d.message.clone())).collect::<Vec<_>>()
    );
    let all = runs(&r);
    let notes = note_runs(&all);
    assert_eq!(notes.len(), 2, "note words missing: {:?}", all.iter().map(|r| r.text.clone()).collect::<Vec<_>>());
    // The note starts one `\marginparsep` past the text block ...
    assert!((notes[0].x - NOTE_X_TEXPT * BP).abs() < 0.5, "first note word x={:.3}, want {:.3}", notes[0].x, NOTE_X_TEXPT * BP);
    assert!(notes[1].x > notes[0].x, "second note word sits after the first");
    // ... and ends inside `\marginparwidth`.
    let right = notes.iter().map(|n| n.x + n.width).fold(0.0, f64::max);
    assert!(right <= (NOTE_X_TEXPT + NOTE_WIDTH_TEXPT) * BP + 0.5, "note right edge {right:.3}");
    // The running text stays out of the margin (the page number excepted,
    // which is centred, not marginal).
    for b in all.iter().filter(|r| r.text != "side" && r.text != "note") {
        assert!(b.x + b.width <= TEXT_RIGHT_TEXPT * BP + 0.5, "body run in the margin: {:?}", (b.text.clone(), b.x));
    }
}

#[test]
fn margin_note_is_set_in_footnotesize_near_the_calling_line() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&body("First words\\marginpar{side note} and the rest of the paragraph continue here."));
    let all = runs(&r);
    let notes = note_runs(&all);
    assert_eq!(notes.len(), 2);
    // `\footnotesize` at 10pt is 8pt; the body is 10pt.
    for n in &notes {
        assert_eq!(n.size, Tick::from_tex_pt(8.0), "note size: {:?}", n.size);
    }
    let body_size = all.iter().find(|r| r.text == "First").expect("body text").size;
    assert_eq!(body_size, Tick::from_tex_pt(10.0));
    // The note's top aligns with the calling line's top, so its first
    // baseline sits within one `\baselineskip` (12pt) of that baseline.
    let call = all.iter().find(|r| r.text == "and").expect("calling line");
    assert!((notes[0].baseline - call.baseline).abs() < 12.0 * BP, "note baseline {:.3} vs calling line {:.3}", notes[0].baseline, call.baseline);
}

#[test]
fn margin_note_follows_a_later_calling_line_not_the_paragraph_top() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // Enough words to break the paragraph, with `\marginpar` on a later line.
    let filler = (0..40).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ");
    let r = render_one(&body(&format!("{filler} anchor\\marginpar{{side note}} tail tail tail tail.")));
    let all = runs(&r);
    let notes = note_runs(&all);
    assert_eq!(notes.len(), 2, "note words missing");
    let anchor = all.iter().find(|r| r.text == "anchor").expect("anchor word");
    let first = all.iter().find(|r| r.text == "word0").expect("first line");
    assert!(
        (anchor.baseline - first.baseline).abs() > 12.0 * BP,
        "fixture did not put the call on a later line: anchor {:.3} vs first {:.3}",
        anchor.baseline,
        first.baseline
    );
    assert!((notes[0].baseline - anchor.baseline).abs() < 12.0 * BP, "note baseline {:.3} vs calling line {:.3}", notes[0].baseline, anchor.baseline);
    assert!((notes[0].baseline - first.baseline).abs() > 12.0 * BP, "note stuck at paragraph top: {:.3}", notes[0].baseline);
}
