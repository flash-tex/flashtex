//! beamer polish, second pass (issue #944 leftovers 5-9 and the audit's
//! open items): frame-level overlay specifications, action specifications
//! and `\temporal`, nested itemize/enumerate sizes, `[t]`/`[b]` with
//! `[allowframebreaks]`, a `\footnote` inside a column, sans-serif math
//! and `\tableofcontents` in a frame, against pdflatex.
//!
//! Oracle: pdflatex 3.141592653-2.6-1.40.29 (TeX Live 2026), the pinned
//! `reference.pdf` of `fixtures/real-world/beamer-polish` (13 pages) read
//! with `tools/visual-oracle/pdftext.py` (bp from the paper's top-left
//! corner, baseline of the word's first glyph), and the `\showoutput`
//! transcript of the nested-list frame. Every number below is one of
//! those readings; `tools/visual-oracle/rank.py` on the deck reads every
//! page within 0.01 bp except p3, whose sans math carries the OT1 `cmssi`
//! vs `ec-lmsso` italic-correction difference (0.13 bp per `x`, 0.40 bp
//! by the end of the line; see `sans_math_uses_the_sans_text_shapes`).
//!
//! Needs the bundled Latin Modern faces and their metrics
//! (`FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` at TeX Live's
//! `lm`/`ec`/`amsfonts/symbols` TFMs), like every oracle test in this crate.

mod common;

use common::{lm_available, render_one, words_of, Word};
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::Rendered;

fn deck() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-polish/main.tex")).expect("polish deck")
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Some run reading exactly `text` on `page` sits within `tol` bp of
/// `(x, baseline)`.
fn at(words: &[Word], page: u32, text: &str, x: f64, baseline: f64, tol: f64) {
    let same: Vec<&Word> = words.iter().filter(|w| w.page == page && w.text == text).collect();
    assert!(!same.is_empty(), "no word {text:?} on page {page}: {:?}", words.iter().filter(|w| w.page == page).map(|w| &w.text).collect::<Vec<_>>());
    let hit = same.iter().any(|w| near(w.x, x, tol) && near(w.baseline, baseline, tol));
    assert!(hit, "page {page} {text:?}: ours {:?}, pdflatex ({x:.3}, {baseline:.3}), tolerance {tol}", same.iter().map(|w| (w.x, w.baseline)).collect::<Vec<_>>());
}

/// The run reading `text` is absent from `page` (covered or omitted).
fn absent(words: &[Word], page: u32, text: &str) {
    assert!(!words.iter().any(|w| w.page == page && w.text == text), "page {page} shows {text:?}, pdflatex does not");
}

/// The fill colour of the first run reading `text` on `page`.
fn run_paint(r: &Rendered, page: usize, text: &str) -> Option<(f64, f64, f64)> {
    r.v2.pages[page - 1].resident_items().iter().find_map(|it| match it {
        Item::GlyphRun(g) if g.text == text => Some((g.paint.r, g.paint.g, g.paint.b)),
        _ => None,
    })
}

/// Every run reading `text` on `page` as `(x, baseline, size in bp, PostScript name)`.
fn runs_of(r: &Rendered, page: usize, text: &str) -> Vec<(f64, f64, f64, String)> {
    let fonts = &r.v2.fonts;
    r.v2.pages[page - 1]
        .resident_items()
        .iter()
        .filter_map(|it| match it {
            Item::GlyphRun(g) if g.text == text => {
                let name = fonts.iter().find(|f| f.font_id == g.font_id).map(|f| f.postscript_name.clone()).unwrap_or_default();
                let first = g.glyphs.first()?;
                Some((first.origin_x.to_bp(), first.baseline_y.to_bp(), g.font_size.to_bp(), name))
            }
            _ => None,
        })
        .collect()
}

/// The size (bp) and PostScript name of the first run reading `text` on `page`.
fn run_font(r: &Rendered, page: usize, text: &str) -> Option<(f64, String)> {
    runs_of(r, page, text).into_iter().next().map(|(_, _, size, name)| (size, name))
}

/// The deck sets 13 pages: 1 + 1 + 1 + 2 (`\begin{frame}<2->` over items
/// naming slide 3) + 3 + 2 + 2 + 1.
#[test]
fn polish_deck_page_count_and_frame_level_specification() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck());
    assert_eq!(r.v2.pages.len(), 13, "pages");
    let words = words_of(&r);
    // `\begin{frame}<2->`: slide 1 is skipped; p4 is slide 2 (item 3
    // covered), p5 slide 3 (all three items).
    at(&words, 4, "every", 74.282 + 14.735, 116.426, 0.01);
    at(&words, 4, "second", 117.164, 132.964, 0.01);
    absent(&words, 4, "third");
    at(&words, 5, "third", 117.164, 149.502, 0.01);
    at(&words, 5, "specification", 79.825, 21.057, 0.01);
}

/// `\item<1-| alert@2>` / `\item<2-| alert@3>` / `\temporal<2>{..}{..}{..}`
/// (p6-p8): the alerted item's text and its label are red on the named
/// slide only; `\temporal` shows one of its three arguments per slide at
/// the same baseline (160.620).
#[test]
fn action_specifications_and_temporal() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck());
    let words = words_of(&r);
    let red = Some((1.0, 0.0, 0.0));
    let structure = Some((0.2, 0.2, 0.7));
    // p6 (slide 1): item 1 shown, not alerted; items 2 and 3 covered.
    at(&words, 6, "Alerted", 50.165, 111.006, 0.01);
    assert_ne!(run_paint(&r, 6, "Alerted"), red);
    absent(&words, 6, "From");
    at(&words, 6, "Before", 28.346, 160.620, 0.01);
    // p7 (slide 2): item 1 alerted (label included), item 2 shown.
    assert_eq!(run_paint(&r, 7, "Alerted"), red);
    assert_eq!(run_paint(&r, 7, "\u{25B6}"), red, "the alerted item's label");
    at(&words, 7, "From", 50.165, 127.544, 0.01);
    assert_ne!(run_paint(&r, 7, "From"), red);
    at(&words, 7, "On", 28.346, 160.620, 0.01);
    absent(&words, 7, "Before");
    // p8 (slide 3): item 2 alerted, item 3 shown, `after` text.
    assert_ne!(run_paint(&r, 8, "Alerted"), red);
    assert_eq!(run_paint(&r, 8, "From"), red);
    assert_eq!(run_paint(&r, 8, "\u{25B6}"), structure, "item 1's label is back in the structure colour");
    at(&words, 8, "third", 95.620, 144.082, 0.01);
    at(&words, 8, "After", 28.346, 160.620, 0.01);
}

/// Nested lists (p2, `\showoutput` transcript): level 2 in `\small`
/// (label at 58.780, text at 71.983 on the 102.120 baseline, 13pt under
/// level 1 because level 3's `\footnotesize` was in force at its `\par`),
/// level 3 in `\footnotesize` (81.373 / 93.801 at 115.071), the level-2
/// label `\raise1.5pt`; nested enumerate labels `1.1`/`1.2` at 53.798 with
/// their text at 71.988.
#[test]
fn nested_lists_take_the_sub_body_sizes() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck());
    let words = words_of(&r);
    at(&words, 2, "First", 50.165, 89.168, 0.01);
    at(&words, 2, "Second", 71.983, 102.120, 0.01);
    at(&words, 2, "small.", 176.924, 102.120, 0.01);
    at(&words, 2, "Third", 93.801, 115.071, 0.01);
    at(&words, 2, "footnotesize.", 184.586, 115.071, 0.01);
    at(&words, 2, "Another", 71.983, 129.019, 0.01);
    at(&words, 2, "Back", 50.165, 145.557, 0.01);
    at(&words, 2, "1.", 36.225, 163.490, 0.01);
    at(&words, 2, "1.1", 53.798, 177.437, 0.01);
    at(&words, 2, "1.2", 53.798, 189.392, 0.01);
    at(&words, 2, "2.", 36.225, 205.930, 0.01);
    // The labels: level 1 at the body size, level 2 at `\small`, level 3
    // at `\footnotesize`.
    let labels = runs_of(&r, 2, "\u{25B6}");
    assert_eq!(labels.len(), 5, "{labels:?}");
    let sizes: Vec<f64> = labels.iter().map(|l| (l.2 * 100.0).round() / 100.0).collect();
    assert_eq!(sizes, vec![10.91, 9.96, 8.97, 9.96, 10.91]);
    assert!(near(labels[1].0, 58.780, 0.01) && near(labels[1].1, 100.625, 0.01), "{:?}", labels[1]);
    assert!(near(labels[2].0, 81.373, 0.01) && near(labels[2].1, 113.577, 0.01), "{:?}", labels[2]);
}

/// `[t,allowframebreaks]` (p9/p10) and `[b,allowframebreaks]` (p11/p12):
/// the split falls after item 14 in both (the 14th item's bottom is
/// 1.47bp past `0.95\textheight`, within the `\itemsep` shrink, §974),
/// the `[t]` pages keep the natural item pitch (the bottom `1fill` takes
/// the free height) and the `[b]` pages sit on the frame's bottom.
#[test]
fn top_and_bottom_aligned_frames_break_after_item_fourteen() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck());
    let words = words_of(&r);
    at(&words, 9, "I", 122.961, 21.057, 0.01);
    at(&words, 9, "one", 74.282, 44.995, 0.01);
    at(&words, 9, "fourteen.", 74.282, 259.989, 0.01);
    absent(&words, 9, "fifteen.");
    at(&words, 10, "II", 122.961, 21.057, 0.01);
    at(&words, 10, "fifteen.", 74.282, 42.006, 0.01);
    at(&words, 10, "twenty,", 74.282, 124.696, 0.01);
    at(&words, 11, "one", 74.282, 53.147, 0.01);
    at(&words, 11, "fourteen.", 74.282, 268.141, 0.01);
    at(&words, 12, "fifteen.", 74.282, 180.341, 0.01);
    at(&words, 12, "twenty,", 74.282, 263.031, 0.01);
}

/// A `\footnote` in a `\column` is a minipage footnote (p13): the mark is
/// `\thempfootnote`, an italic `a` at `\sf@size` (CMSSI8 at 7.97pt in the
/// text, 5.98pt in the note), the note sits at the column's foot under
/// `\skip\@mpfootins` and the `\footnoterule` (baseline 143.706, 17.634bp
/// under the column's last line), and the row centres with it.
#[test]
fn column_footnotes_sit_at_the_column_foot() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck());
    let words = words_of(&r);
    at(&words, 13, "The", 18.898, 112.523, 0.01);
    at(&words, 13, "footnote", 18.898, 126.072, 0.01);
    at(&words, 13, "a", 57.261, 122.113, 0.01);
    at(&words, 13, "in", 65.464, 126.072, 0.01);
    at(&words, 13, "a", 31.936, 139.897, 0.01);
    at(&words, 13, "Set", 35.485, 143.706, 0.01);
    at(&words, 13, "bottom", 77.184, 143.706, 0.01);
    at(&words, 13, "right", 212.408, 112.523, 0.01);
    at(&words, 13, "column.", 60.418, 154.665, 0.01);
    let marks: Vec<_> = runs_of(&r, 13, "a").into_iter().filter(|m| m.3.contains("Oblique")).collect();
    assert_eq!(marks.len(), 2, "{marks:?}");
    assert!(near(marks[0].2, 7.97, 0.01) && near(marks[1].2, 5.98, 0.01), "{marks:?}");
    assert!(marks.iter().all(|m| m.3.contains("Oblique")), "{marks:?}");
    assert!(!r.v2.diagnostics.iter().any(|d| d.code == "beamer_column_footnote"), "{:?}", r.v2.diagnostics);
}

/// Sans-serif math (p3): letters in the sans oblique shape, digits,
/// `+`, `=`, `(`, `)` and the operator names in the sans upright one,
/// `\alpha`/`\beta` in the math italic font. The `x` at 56.343 and `f` at
/// 131.403 sit exactly; the words after each `x` carry OT1 `cmssi10`'s
/// smaller italic correction (0.09169 em against `ec-lmsso10`'s 0.104444),
/// 0.13 bp each. The display is set with amsfonts' scaled `cmex10 at
/// 10.95pt` (`\int` limits 21.586bp apart), which beamer's own `amssymb`
/// brings, and the frame centres at the reference baselines.
#[test]
fn sans_math_uses_the_sans_text_shapes() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck());
    let words = words_of(&r);
    at(&words, 3, "Inline", 28.346, 103.798, 0.01);
    at(&words, 3, "x", 56.343, 103.798, 0.01);
    at(&words, 3, "+", 64.799, 103.798, 0.15);
    at(&words, 3, "y", 75.706, 103.798, 0.15);
    at(&words, 3, "=2", 84.958, 103.798, 0.15);
    at(&words, 3, "f", 131.403, 103.798, 0.15);
    at(&words, 3, "(", 137.104, 103.798, 0.15);
    at(&words, 3, "Operators", 28.346, 164.828, 0.01);
    at(&words, 3, "sin", 95.499, 164.828, 0.01);
    at(&words, 3, "x", 109.745, 164.828, 0.01);
    at(&words, 3, "log", 139.561, 164.828, 0.15);
    at(&words, 3, "a", 201.944, 128.025, 0.3);
    at(&words, 3, "2", 211.543, 142.888, 0.3);
    // The `\int` limits are one run (`10`) whose first glyph is the `1`.
    // The display is centred, so its 0.57bp of italic-correction
    // difference (`f(x)`, `dx`, `a`, `b`) shifts it 0.29bp left.
    at(&words, 3, "10", 145.951, 123.711, 0.3);
    let (size, name) = run_font(&r, 3, "x").expect("x run");
    assert!(name.contains("Sans") && name.contains("Oblique") && near(size, 10.91, 0.01), "{name} {size}");
    let (_, name) = run_font(&r, 3, "sin").expect("sin run");
    assert!(name.contains("Sans") && !name.contains("Oblique"), "{name}");
    let (_, name) = run_font(&r, 3, "+").expect("+ run");
    assert!(name.contains("Sans"), "{name}");
    let (_, name) = run_font(&r, 3, "\u{3B1}").expect("alpha run");
    assert!(name.contains("Math"), "{name}");
}

/// `\tableofcontents` in a frame (p1): the five sections in the structure
/// colour at x 28.346, 35.086bp apart from 73.956 -- beamer's `\vfill`s
/// (`plus 1fill`) sharing the frame's free height with its `[c]` skips --
/// and no `Contents` heading.
#[test]
fn table_of_contents_lists_the_sections_between_fills() {
    if !lm_available() {
        return;
    }
    let r = render_one(&deck());
    let words = words_of(&r);
    at(&words, 1, "Outline", 8.504, 21.057, 0.01);
    for (i, title) in ["Lists", "Mathematics", "Overlays", "Breaks", "Columns"].iter().enumerate() {
        at(&words, 1, title, 28.346, 73.956 + 35.086 * i as f64, 0.01);
        assert_eq!(run_paint(&r, 1, title), Some((0.2, 0.2, 0.7)), "{title}");
    }
    absent(&words, 1, "Contents");
    assert_eq!(words.iter().filter(|w| w.page == 1).count(), 6);
}

/// Sans math's other families (beamerbasefont.sty 205-238): the uppercase
/// Greek letters are `operators` characters, so `OT1/cmss/m/n`; `\mathbf`
/// is `cmss` `bx/n`; `\mathrm` stays `\rmdefault`. pdflatex (TeX Live
/// 2026) on the probe below: `\Gamma` .. `\Omega` in CMSS10 at 56.343,
/// 62.253, 71.343, 79.828, 86.495, 93.768, 101.495, 109.373, 117.858,
/// 125.737, 134.222; `\mathbf{x}`, `\mathbf{A}`, `\mathbf{0}` in CMSSBX10
/// at 246.279, 265.063, 287.613; `\mathrm{d}` in CMR10 at 317.395; the
/// display's `\Delta` CMSS10 at 152.158 and `\mathbf{u}` CMSSBX10 at
/// 161.249. The frame sits 0.104bp lower than pdflatex's (the display's
/// `\sum` limits), so the baselines are held to 0.15.
#[test]
fn sans_math_greek_capitals_bold_and_roman() {
    if !lm_available() {
        return;
    }
    let src = "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}\n  \\frametitle{Greek}\n  Inline $\\Gamma \\Delta \\Theta \\Lambda \\Xi \\Pi \\Sigma \\Upsilon \\Phi \\Psi \\Omega$ and $\\alpha + \\beta = \\gamma$.\n  Bold $\\mathbf{x} + \\mathbf{A} = \\mathbf{0}$ and $\\mathrm{d}x$.\n  \\[ \\Delta \\mathbf{u} = \\sum_{i=1}^n \\Phi_i \\]\n\\end{frame}\n\\end{document}\n";
    let r = render_one(src);
    let words = words_of(&r);
    // One run of the eleven letters (TeX's are one `operators` run too),
    // from 56.343 to the end of `\Omega` (134.222 + 7.879).
    let greek = "\u{393}\u{394}\u{398}\u{39B}\u{39E}\u{3A0}\u{3A3}\u{3A5}\u{3A6}\u{3A8}\u{3A9}";
    at(&words, 1, greek, 56.343, 105.988, 0.15);
    let run = words.iter().find(|w| w.page == 1 && w.text == greek).expect("greek run");
    assert!(near(run.width, 142.101 - 56.343, 0.05), "{}", run.width);
    let (_, name) = run_font(&r, 1, greek).expect("greek run");
    assert!(name.contains("Sans") && !name.contains("Oblique"), "{name}");
    for (text, x) in [("x", 246.279), ("A", 265.063), ("0", 287.613)] {
        at(&words, 1, text, x, 105.988, 0.15);
    }
    for text in ["A", "0", "u"] {
        let (_, name) = run_font(&r, 1, text).expect("bold run");
        assert!(name.contains("Sans") && name.contains("Bold"), "{text}: {name}");
    }
    at(&words, 1, "d", 317.395, 105.988, 0.15);
    let (_, name) = run_font(&r, 1, "d").expect("mathrm run");
    assert!(name.contains("Roman") && !name.contains("Bold"), "{name}");
    at(&words, 1, "u", 161.249, 138.334, 0.15);
}
