//! A single `\twocolumn`/`\onecolumn` after the first material.
//!
//! Every number is read off the glyph origins of the PDF
//! `/Library/TeX/texbin/pdflatex` (pdfTeX 3.141592653-2.6-1.40.29, TeX Live
//! 2026) produces for the probe quoted in the test, measured with PyMuPDF.
//! pdflatex is an oracle only and never runs in the product path.
//!
//! The page frame used to be one frame for the whole document: a switch
//! after the first material set every `\if@twocolumn` test but not the
//! column count of the pages, and was reported as `twocolumn_mid_document`
//! (`columns.rs`, `adapter.rs`). The single-switch case — an article that
//! starts one-column for the title/abstract and runs `\twocolumn` once,
//! staying two-column (or the reverse with `\onecolumn`) — now lays out:
//! every page from the switch on uses the post-switch frame, whose first
//! column sits where the old text block did (`\textwidth`, `\parindent`
//! and the margins are the command's, not the class option's, so they do
//! not move). Anything beyond one bare switch at a block boundary keeps
//! the limitation.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[derive(Debug, Clone)]
struct Word {
    text: String,
    x: f64,
    baseline: f64,
    page: usize,
}

fn layout(text: &str) -> (Vec<String>, Vec<Word>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    let mut words = Vec::new();
    for (p, page) in r.v2.pages.iter().enumerate() {
        for it in page.resident_items() {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                words.push(Word {
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    page: p + 1,
                });
            }
        }
    }
    (v1.diagnostics.iter().map(|d| d.code.clone()).collect(), words)
}

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words
        .iter()
        .find(|w| w.text.trim() == text)
        .unwrap_or_else(|| panic!("no word {text:?} in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
}

fn close(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() <= 0.5, "{what}: {got} vs pdflatex {want} ({:+})", got - want);
}

const BODY: &str = "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor \
    incididunt ut labore et dolore magna aliqua Ut enim ad minim veniam quis nostrud \
    exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat Duis aute irure \
    dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur ";

fn front_matter() -> String {
    format!(
        "\\documentclass[10pt]{{article}}\n\\begin{{document}}\n\\title{{A Study of Columns}}\n\
        \\author{{J. Doe}}\n\\date{{January 1, 1970}}\n\\maketitle\n\\begin{{abstract}}\n\
        This abstract spans the full width in one-column mode. {BODY}\n\\end{{abstract}}\n"
    )
}

/// Glyph origins on `page` starting a column at `want` (within rounding):
/// every full line of the column opens exactly there, while short last
/// lines and the centred folio start anywhere around it.
fn column_lines(words: &[Word], page: usize, want: f64) -> usize {
    words.iter().filter(|w| w.page == page && (w.x - want).abs() <= 0.1).count()
}

#[test]
fn a_single_twocolumn_after_front_matter_lays_out_both_columns() {
    let paras = BODY.repeat(8);
    let doc = format!("{}\\twocolumn\n\\section{{First}}\n\n{paras}\n\\section{{Second}}\n\n{paras}\\end{{document}}\n", front_matter());
    let (codes, words) = layout(&doc);
    assert!(
        !codes.iter().any(|c| c == "twocolumn_mid_document"),
        "one bare switch is laid out, not reported: {codes:?}"
    );
    // Three pages: the front matter alone on page 1 ...
    assert_eq!(words.iter().map(|w| w.page).max(), Some(3));
    // ... and the switch starts a new page whose columns are the
    // `\twocolumn` command's: the first where the one-column text block
    // was (133.768 bp), the second `\columnwidth + \columnsep` right
    // (310.605 bp) — not the class option's (72.0 bp).
    assert_eq!(word(&words, "First").page, 2);
    // Full pages set both columns; the closing page still opens the
    // second one (pdflatex puts 8 lines there).
    assert!(column_lines(&words, 2, 133.768) >= 10, "page 2 sets a first column at 133.768 bp");
    assert!(column_lines(&words, 2, 310.605) >= 10, "page 2 sets a second column at 310.605 bp");
    assert!(column_lines(&words, 3, 133.768) >= 10, "page 3 sets a first column at 133.768 bp");
    assert!(column_lines(&words, 3, 310.605) >= 1, "page 3 still opens a second column at 310.605 bp");
    // Later paragraphs sit a `\parindent` (15pt) inside each column: the
    // command keeps the one-column `\parindent`, unlike the class option.
    assert!(column_lines(&words, 2, 148.712) >= 1, "indented lines in the first column");
    assert!(column_lines(&words, 2, 325.549) >= 1, "indented lines in the second column");
}

#[test]
fn a_single_onecolumn_after_front_matter_lays_out_one_column() {
    // The reverse: `[twocolumn]` from the start, then one `\onecolumn`.
    // The command keeps the class's wide `\textwidth`, so the rest of the
    // document is one 469pt column at 72.0 bp.
    let paras = BODY.repeat(4);
    let doc = format!(
        "{}\\onecolumn\n\\section{{First}}\n\n{paras}\\end{{document}}\n",
        front_matter().replacen("[10pt]", "[10pt,twocolumn]", 1)
    );
    let (codes, words) = layout(&doc);
    assert!(
        !codes.iter().any(|c| c == "twocolumn_mid_document"),
        "one bare switch is laid out, not reported: {codes:?}"
    );
    assert_eq!(word(&words, "First").page, 2);
    assert!(
        column_lines(&words, 2, 72.0) >= 10,
        "post-switch page sets one column at 72.0 bp"
    );
    // Full width, not a leftover column: lines run past 500 bp.
    let rightmost = words.iter().filter(|w| w.page == 2).map(|w| w.x).fold(0.0_f64, f64::max);
    assert!(rightmost > 500.0, "one wide column, rightmost {rightmost}");
}
