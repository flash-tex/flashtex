//! A `tikzpicture` that is the paragraph of a `center` (or `flushleft`,
//! `flushright`, `quote`) opens and closes the environment's `\trivlist`
//! like a paragraph of text there: `\addvspace{\topsep+\partopsep}` above
//! it and `\@endparenv`'s `\addvspace\@topsepadd` below it. The picture used
//! to be a block of its own that took neither, so everything after a
//! centred figure sat 12pt (11pt class: `\topsep` 9pt + `\partopsep` 3pt)
//! too high per skip lost -- 24pt after a paragraph (v0.2.0 gate B7).
//!
//! Oracle: pdflatex 1.40.29 (TeX Live 2026, `SOURCE_DATE_EPOCH=0`), each
//! case a whole document (`article`, 11pt unless noted, `geometry`
//! `margin=1in`, `\pagestyle{empty}`), read with pymupdf: text baselines
//! are span origins, the picture is the bounding box of its one stroked
//! path (`\draw (0,0) -- (4.2,2.5);`, or `-- (1,0.2)` for the small one),
//! whose bottom edge is the picture's baseline. FlashTeX matched every value
//! below to 0.002bp when this was written; the controls (`{\centering ...
//! \par}` and a bare picture paragraph) already matched before.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, PathCmd};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const BIG: &str = "\\begin{tikzpicture}\\draw (0,0) -- (4.2,2.5);\\end{tikzpicture}";
const SMALL: &str = "\\begin{tikzpicture}\\draw (0,0) -- (1,0.2);\\end{tikzpicture}";

struct Page {
    /// `(first word, baseline y)` of every glyph run, in bp from the top.
    runs: Vec<(String, f64)>,
    /// `(x min, y max)` of every path, in bp.
    paths: Vec<(f64, f64)>,
}

fn render_or_skip(class_size: &str, body: &str) -> Option<Page> {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return None;
    }
    let src = format!(
        "\\documentclass[{class_size}]{{article}}\n\\usepackage[margin=1in]{{geometry}}\n\\usepackage{{tikz}}\n\\begin{{document}}\n\\pagestyle{{empty}}\n{body}\\end{{document}}\n"
    );
    let docs = [SourceDocument { path: "main.tex", text: &src }];
    let out = render(&docs, "main.tex", 1, "tikz-center", &fonts, &RenderOptions::default());
    assert_eq!(out.v2.pages.len(), 1);
    let page = &out.v2.pages[0];
    let mut runs = Vec::new();
    let mut paths = Vec::new();
    for item in page.resident_items() {
        match item {
            Item::GlyphRun(r) => {
                if let Some(g) = r.glyphs.first() {
                    runs.push((r.text.clone(), g.baseline_y.to_bp()));
                }
            }
            Item::Path(p) => {
                let pts: Vec<(f64, f64)> = p
                    .commands
                    .iter()
                    .filter_map(|c| match *c {
                        PathCmd::Move(x, y) | PathCmd::Line(x, y) => Some((x.to_bp(), y.to_bp())),
                        _ => None,
                    })
                    .collect();
                let xmin = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
                let ymax = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
                paths.push((xmin, ymax));
            }
            _ => {}
        }
    }
    Some(Page { runs, paths })
}

fn baseline(page: &Page, word: &str) -> f64 {
    let ys: Vec<f64> = page.runs.iter().filter(|(t, _)| t == word).map(|r| r.1).collect();
    assert_eq!(ys.len(), 1, "`{word}` runs: {:?}", page.runs);
    ys[0]
}

fn near(what: &str, got: f64, want: f64) {
    assert!((got - want).abs() <= 0.05, "{what}: FlashTeX {got:.3}bp, pdflatex {want:.3}bp");
}

#[test]
fn centred_picture_at_page_top_keeps_the_closing_skip() {
    let Some(p) = render_or_skip("11pt", &format!("\\begin{{center}}{BIG}\\end{{center}}\n\nBody text here.\n")) else { return };
    // The opening skip is discarded at the top of the page; the closing
    // one is not. Before the fix Body sat at 156.81.
    near("picture bottom", p.paths[0].1, 143.065);
    near("picture x", p.paths[0].0, 246.473);
    near("Body", baseline(&p, "Body"), 168.769);
}

#[test]
fn centred_picture_after_a_paragraph_takes_both_skips() {
    let Some(p) = render_or_skip("11pt", &format!("Above text.\n\n\\begin{{center}}{BIG}\\end{{center}}\n\nBody text here.\n")) else { return };
    // Before the fix: picture 155.02, Body 168.77 (both skips lost).
    near("Above", baseline(&p, "Above"), 82.959);
    near("picture bottom", p.paths[0].1, 166.976);
    near("Body", baseline(&p, "Body"), 192.679);
}

#[test]
fn small_centred_picture_keeps_the_closing_skip() {
    let Some(p) = render_or_skip("11pt", &format!("\\begin{{center}}{SMALL}\\end{{center}}\n\nBody text here.\n")) else { return };
    near("picture bottom", p.paths[0].1, 82.76);
    near("Body", baseline(&p, "Body"), 108.463);
}

#[test]
fn centred_picture_opens_the_environment_for_the_text_after_it() {
    // The picture opens the `\trivlist` in vertical mode, so the closing
    // skip after the caption keeps `\partopsep` too (it was 3pt short).
    let Some(p) = render_or_skip("11pt", &format!("Above text.\n\n\\begin{{center}}{BIG}\n\nCaption after\n\\end{{center}}\nBody text here.\n")) else {
        return;
    };
    near("picture bottom", p.paths[0].1, 166.976);
    near("Caption", baseline(&p, "Caption"), 180.724);
    near("Body", baseline(&p, "Body"), 206.229);
}

#[test]
fn two_centred_pictures_close_after_the_second() {
    let Some(p) = render_or_skip("11pt", &format!("Above text.\n\n\\begin{{center}}{BIG}\n\n{SMALL}\\end{{center}}\n\nBody text here.\n")) else { return };
    assert_eq!(p.paths.len(), 2, "{:?}", p.paths);
    near("first picture bottom", p.paths[0].1, 166.976);
    near("second picture bottom", p.paths[1].1, 180.525);
    near("Body", baseline(&p, "Body"), 206.229);
}

#[test]
fn centre_begun_in_horizontal_mode_has_no_partopsep() {
    let Some(p) = render_or_skip("11pt", &format!("Above text.\n\\begin{{center}}{BIG}\\end{{center}}Body text here.\n")) else { return };
    near("picture bottom", p.paths[0].1, 163.987);
    near("Body", baseline(&p, "Body"), 186.702);
}

#[test]
fn centred_picture_at_ten_point() {
    let Some(p) = render_or_skip("10pt", &format!("Above text.\n\n\\begin{{center}}{BIG}\\end{{center}}\n\nBody text here.\n")) else { return };
    near("picture bottom", p.paths[0].1, 163.987);
    near("Body", baseline(&p, "Body"), 186.104);
}

#[test]
fn flush_and_quote_pictures_are_placed_and_spaced_like_text() {
    for (env, x) in [("flushleft", 72.199), ("flushright", 420.746), ("quote", 99.472)] {
        let Some(p) = render_or_skip("11pt", &format!("Above text.\n\n\\begin{{{env}}}{BIG}\\end{{{env}}}\n\nBody text here.\n")) else { return };
        near(&format!("{env} picture x"), p.paths[0].0, x);
        near(&format!("{env} picture bottom"), p.paths[0].1, 166.976);
        near(&format!("{env} Body"), baseline(&p, "Body"), 192.679);
    }
}

#[test]
fn centering_declaration_and_bare_picture_are_unchanged() {
    // Controls: `\centering` opens no `\trivlist`, and a picture paragraph
    // outside any environment takes only `\parskip`.
    let Some(p) = render_or_skip("11pt", &format!("Above text.\n\n{{\\centering {BIG}\\par}}\n\nBody text here.\n")) else { return };
    near("centering picture x", p.paths[0].0, 246.473);
    near("centering picture bottom", p.paths[0].1, 155.021);
    near("centering Body", baseline(&p, "Body"), 168.769);
    let Some(p) = render_or_skip("11pt", &format!("Above text.\n\n{BIG}\n\nBody text here.\n")) else { return };
    near("bare picture x", p.paths[0].0, 89.136);
    near("bare picture bottom", p.paths[0].1, 155.021);
    near("bare Body", baseline(&p, "Body"), 168.769);
}
