//! Margin notes: `\marginpar{note}` sets the note in the right margin at
//! `\normalsize`, `marginparwidth` wide and `marginparsep` past the text
//! block, its first baseline aligned with the calling line's baseline.

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
fn margin_note_is_set_in_normalsize_near_the_calling_line() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let r = render_one(&body("First words\\marginpar{side note} and the rest of the paragraph continue here."));
    let all = runs(&r);
    let notes = note_runs(&all);
    assert_eq!(notes.len(), 2);
    // `\@marginparreset` resets to `\normalsize`, same as the body: 10pt.
    for n in &notes {
        assert_eq!(n.size, Tick::from_tex_pt(10.0), "note size: {:?}", n.size);
    }
    let body_size = all.iter().find(|r| r.text == "First").expect("body text").size;
    assert_eq!(body_size, Tick::from_tex_pt(10.0));
    // The note is a `\vtop`: its first baseline equals the calling line's
    // own baseline, exactly (up to floating-point noise).
    let call = all.iter().find(|r| r.text == "and").expect("calling line");
    assert!((notes[0].baseline - call.baseline).abs() < 0.1 * BP, "note baseline {:.3} vs calling line {:.3}", notes[0].baseline, call.baseline);
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
    assert!((notes[0].baseline - anchor.baseline).abs() < 0.1 * BP, "note baseline {:.3} vs calling line {:.3}", notes[0].baseline, anchor.baseline);
    assert!((notes[0].baseline - first.baseline).abs() > 12.0 * BP, "note stuck at paragraph top: {:.3}", notes[0].baseline);
}

#[test]
fn marginpars_on_nearby_lines_are_kept_marginparpush_apart() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // Two notes anchored on adjacent lines: without `\marginparpush`
    // stacking, both would align to their own calling line and overlap.
    // Distinct note text (rather than `note_runs`'s fixed "side note") lets
    // the two notes' runs be told apart.
    let filler = (0..40).map(|i| format!("word{i}")).collect::<Vec<_>>().join(" ");
    let r = render_one(&body(&format!(
        "{filler} first\\marginpar{{alpha beta}} more more more more more more more more second\\marginpar{{gamma delta}} tail."
    )));
    let all = runs(&r);
    let first_note: Vec<&Run> = all.iter().filter(|r| r.text == "alpha" || r.text == "beta").collect();
    let second_note: Vec<&Run> = all.iter().filter(|r| r.text == "gamma" || r.text == "delta").collect();
    assert_eq!(first_note.len(), 2, "first note words missing: {:?}", all.iter().map(|r| r.text.clone()).collect::<Vec<_>>());
    assert_eq!(second_note.len(), 2, "second note words missing: {:?}", all.iter().map(|r| r.text.clone()).collect::<Vec<_>>());
    // Exact pdflatex oracle for this fixture (`pdftotext -bbox`, baseline =
    // yMin + 6.914bp): alpha/beta's baseline is 170.630bp, gamma/delta's is
    // 184.467bp (= alpha's baseline + the first note's real last-line depth
    // (no strut) + `\marginparpush` (5pt) + the second note's first-line
    // height). Round-2 review finding: a phantom `0.3\baselineskip` strut
    // depth on every note (copied from the footnote builder, which really
    // has one) made this drift low, cumulatively over more pushed notes;
    // the loose inequality this test used to have could not detect it.
    for n in &first_note {
        assert!((n.baseline - 170.630).abs() < 0.1, "first note baseline {:.3}, want 170.630", n.baseline);
    }
    for n in &second_note {
        assert!((n.baseline - 184.467).abs() < 0.1, "second note baseline {:.3}, want 184.467", n.baseline);
    }
}

#[test]
fn marginpar_inside_multicols_is_reported_and_not_placed() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass{article}\n\\usepackage{multicol}\n\\begin{document}\n\
        \\begin{multicols}{2}\nFirst words\\marginpar{side note} and the rest of the paragraph continue here.\n\\end{multicols}\n\
        \\end{document}\n";
    let r = render_one(src);
    assert!(
        r.v2.diagnostics.iter().any(|d| d.message.contains("marginpars") && d.message.contains("multicols")),
        "expected a multicols/marginpar warning: {:?}",
        r.v2.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
    );
    let all = runs(&r);
    assert!(note_runs(&all).is_empty(), "the note was placed despite being inside multicols");
}

#[test]
fn marginpar_in_the_left_column_of_a_twocolumn_document_goes_in_the_left_margin() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\documentclass[10pt,twocolumn]{article}\n\\begin{document}\n\
        First words\\marginpar{side note} and the rest of the paragraph continue here.\n\
        \\end{document}\n";
    let r = render_one(src);
    let all = runs(&r);
    let notes = note_runs(&all);
    assert_eq!(notes.len(), 2, "note words missing: {:?}", all.iter().map(|r| r.text.clone()).collect::<Vec<_>>());
    // The call is early enough to fall in the first (left) column.
    let body_left = all.iter().find(|r| r.text == "First").expect("body text").x;
    let note_left = notes.iter().map(|n| n.x).fold(f64::MAX, f64::min);
    let note_right = notes.iter().map(|n| n.x + n.width).fold(0.0, f64::max);
    assert!(note_right < body_left, "left-column note not in the left margin: note right {note_right:.3} vs body left {body_left:.3}");
    // Exact pdflatex oracle (round-2 review): the left margin's own left
    // edge, `column left - marginparsep - marginparwidth` = 58.05bp.
    assert!((note_left - 58.05).abs() < 0.5, "left-column note x {note_left:.3}, want 58.05");
}
