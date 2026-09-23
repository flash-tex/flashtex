//! beamer Tier 3 (issue #944): blocks, columns, an `\includegraphics` in a
//! `center`, an in-flow `table` with its unnumbered caption, and a `columns`
//! row inside a block, against pdflatex.
//!
//! Oracle: pdflatex 3.141592653-2.6-1.40.29 (TeX Live 2026). The corpus
//! deck `fixtures/real-world/beamer-blocks-columns` is rendered from its
//! own directory (`figure.png` beside it) and compared with the word
//! positions `tools/visual-oracle/pdftext.py` reads off its pinned
//! `reference.pdf` (bp from the paper's top-left, the first glyph's origin),
//! and the image's `cm` operands of the same file. The ladder document below
//! (`[t]`/`[b]`/`[onlytextwidth]`/`totalwidth=` rows, blocks nested in `[T]`
//! columns, a `figure` with a caption, a block after a list) was measured
//! the same way in the session that wrote this (2026-09-18). Every position
//! here failed on `beamer/t0-t1` (the T0+T1 base), where the block
//! environments were "not implemented", `\column` unknown, the image
//! dropped and the table floated to the frame top.
//!
//! Needs the bundled Latin Modern faces and their metrics
//! (`FLASHTEX_FONT_DIRS=apps/mac/Fonts`, `FLASHTEX_TFM_DIRS` at TeX Live's
//! `lm`/`ec`/`amsfonts/symbols` TFMs), like every oracle test in this crate.

mod common;

use common::{lm_available, words_of, Word};
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions, Rendered};
use flashtex_compiler::parser::SourceDocument;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/real-world/beamer-blocks-columns");

/// Renders `text` as `main.tex` of the corpus deck's directory (so
/// `\includegraphics{figure.png}` reads the fixture's file).
fn render_in_fixture(text: &str) -> Rendered {
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(FIXTURE.into()), ..RenderOptions::default() };
    let docs = [SourceDocument { path: "main.tex", text }];
    render(&docs, "main.tex", 1, "beamer-blocks", &fonts, &options)
}

fn corpus_deck() -> Rendered {
    let text = std::fs::read_to_string(format!("{FIXTURE}/main.tex")).expect("fixture main.tex");
    render_in_fixture(&text)
}

fn word_n<'a>(words: &'a [Word], page: u32, text: &str, nth: usize) -> &'a Word {
    words
        .iter()
        .filter(|w| w.page == page && w.text == text)
        .nth(nth)
        .unwrap_or_else(|| panic!("no word {text:?} (#{nth}) on page {page}: {:?}", words.iter().filter(|w| w.page == page).map(|w| &w.text).collect::<Vec<_>>()))
}

/// `(x, baseline)` of the `nth` run reading `text` on `page`, within `tol`
/// bp of pdflatex's.
fn at_n(words: &[Word], page: u32, text: &str, nth: usize, x: f64, baseline: f64, tol: f64) {
    let w = word_n(words, page, text, nth);
    assert!(
        (w.x - x).abs() <= tol && (w.baseline - baseline).abs() <= tol,
        "page {page} {text:?} #{nth}: ours ({:.3}, {:.3}), pdflatex ({x:.3}, {baseline:.3}), tolerance {tol}",
        w.x,
        w.baseline
    );
}

fn at(words: &[Word], page: u32, text: &str, x: f64, baseline: f64, tol: f64) {
    at_n(words, page, text, 0, x, baseline, tol)
}

fn run_paint(r: &Rendered, page: u32, text: &str) -> (f64, f64, f64) {
    for p in &r.v2.pages {
        if p.number != page {
            continue;
        }
        for it in p.resident_items() {
            if let Item::GlyphRun(run) = it {
                if run.text == text {
                    return (run.paint.r, run.paint.g, run.paint.b);
                }
            }
        }
    }
    panic!("no run {text:?} on page {page}");
}

/// `(x, top, width, height)` in bp of the images on `page`.
fn images(r: &Rendered, page: u32) -> Vec<(f64, f64, f64, f64)> {
    let mut out = Vec::new();
    for p in &r.v2.pages {
        if p.number != page {
            continue;
        }
        for it in p.resident_items() {
            if let Item::Image(img) = it {
                out.push((img.x.to_bp(), img.top.to_bp(), img.width.to_bp(), img.height.to_bp()));
            }
        }
    }
    out
}

#[test]
fn blocks_of_the_three_kinds_stack_at_the_measured_pitch() {
    if !lm_available() {
        return;
    }
    let r = corpus_deck();
    assert_eq!(r.v2.pages.len(), 6);
    let w = words_of(&r);
    // p2: titles 49.40bp apart, bodies 13.33bp under a depthless title,
    // 15.66bp under "Example" (its `p`). The whole body sits 0.12bp low
    // because `\texttt{\textbackslash}` is set in the typewriter face here
    // where OT1's `\textbackslash` is cmsy10's `\backslash` (a taller,
    // deeper, narrower glyph): a text-symbol matter outside this tier.
    at(&w, 2, "Definition", 28.346, 84.577, 0.15);
    at(&w, 2, "A", 28.346, 97.910, 0.15);
    at(&w, 2, "frame.", 28.346, 111.460, 0.15);
    at(&w, 2, "Caution", 28.346, 133.975, 0.15);
    at(&w, 2, "The", 28.346, 147.309, 0.15);
    at(&w, 2, "has", 28.346, 160.858, 0.15);
    at(&w, 2, "Example", 28.346, 183.373, 0.15);
    at_n(&w, 2, "A", 1, 28.346, 199.031, 0.15);
    // Title colours: structure, `alerted text` (red), `example text`
    // (green!50!black); the title size is `\large` (12pt = 11.96bp).
    assert_eq!(run_paint(&r, 2, "Definition"), (0.2, 0.2, 0.7));
    assert_eq!(run_paint(&r, 2, "Caution"), (1.0, 0.0, 0.0));
    assert_eq!(run_paint(&r, 2, "Example"), (0.0, 0.5, 0.0));
    assert_eq!(run_paint(&r, 2, "frame."), (0.0, 0.0, 0.0));
}

#[test]
fn columns_t_row_sits_on_the_paper_wide_hbox() {
    if !lm_available() {
        return;
    }
    let w = words_of(&corpus_deck());
    // p3: `[T]` columns of `.5\textwidth` at x = 18.90 / 190.87bp (a
    // `\hfill` of 18.968pt before, between and after), first baselines
    // aligned at 100.357 (`-1ex` above a 7.604pt-high line under the row).
    at(&w, 3, "Left", 18.898, 100.357, 0.05);
    at(&w, 3, "Right", 190.866, 100.357, 0.05);
    at(&w, 3, "This", 18.898, 113.906, 0.05);
    at(&w, 3, "inside", 18.898, 127.455, 0.05);
    at(&w, 3, "measured.", 118.079, 141.005, 0.05);
    // The list inside the left column: `\leftmargini` from the column's
    // edge, `\itemsep` 3pt; the block inside the right column takes the
    // column's width.
    at(&w, 3, "first", 40.716, 157.543, 0.05);
    at(&w, 3, "second", 40.716, 174.081, 0.05);
    at(&w, 3, "In", 190.866, 133.433, 0.05);
    at(&w, 3, "Blocks", 190.866, 146.766, 0.05);
    at_n(&w, 3, "column", 3, 190.866, 160.316, 0.05);
    at(&w, 3, "them.", 290.928, 160.316, 0.05);
}

#[test]
fn includegraphics_in_a_centred_paragraph_is_placed_and_sized() {
    if !lm_available() {
        return;
    }
    let r = corpus_deck();
    let w = words_of(&r);
    // p4: the image's XObject in the reference is drawn at `1 0 0 1 89.574
    // 87.865 cm` (PDF space) under a 1.08795 scale of a 168.84 x 102.96 bp
    // box: 183.69 x 112.02 bp, top 72.245 from the paper top. `center`
    // takes the 9pt `\topsep` of `size11.clo`, not beamer's 3pt list one.
    let imgs = images(&r, 4);
    assert_eq!(imgs.len(), 1, "{imgs:?}");
    let (x, top, width, height) = imgs[0];
    assert!((x - 89.574).abs() <= 0.05 && (top - 72.245).abs() <= 0.05, "image at ({x:.3}, {top:.3}), pdflatex (89.574, 72.245)");
    assert!((width - 183.690).abs() <= 0.05 && (height - 112.016).abs() <= 0.05, "image {width:.3} x {height:.3}, pdflatex 183.690 x 112.016");
    at(&w, 4, "The", 28.346, 206.777, 0.05);
    at(&w, 4, "centred.", 283.012, 206.777, 0.05);
}

#[test]
fn table_in_a_frame_is_in_flow_with_an_unnumbered_small_caption() {
    if !lm_available() {
        return;
    }
    let r = corpus_deck();
    let w = words_of(&r);
    // p5: the `tabular` centred in the `[c]` body, the caption 7pt +
    // `\baselineskip` glue below it in `\small`, "Table:" (no number) in
    // the structure colour.
    at(&w, 5, "stage", 120.962, 95.221, 0.05);
    at(&w, 5, "share", 218.541, 95.221, 0.05);
    at(&w, 5, "parse", 120.962, 114.407, 0.05);
    at(&w, 5, "export", 120.962, 155.055, 0.05);
    at(&w, 5, "Table:", 81.071, 176.820, 0.05);
    at(&w, 5, "Time", 109.853, 176.820, 0.05);
    at(&w, 5, "document.", 237.928, 176.820, 0.05);
    assert_eq!(run_paint(&r, 5, "Table:"), (0.2, 0.2, 0.7));
    assert_eq!(run_paint(&r, 5, "Time"), (0.0, 0.0, 0.0));
    assert!(!w.iter().any(|x| x.page == 5 && x.text == "1:"), "beamer's caption carries no number");
}

#[test]
fn columns_c_row_inside_a_block_and_the_frame_footnote() {
    if !lm_available() {
        return;
    }
    let w = words_of(&corpus_deck());
    // p6: the `[c]` row (`\vcenter` on the 2.7375pt math axis) inside the
    // block body: "narrow" (one x-height-high line) 0.304pt above the
    // row's baseline, the two-line column 0.4pt below it; the frame
    // footnote stays at the foot (T1) with the body above it.
    at(&w, 6, "Beamer", 28.346, 101.000, 0.05);
    at_n(&w, 6, "bottom", 1, 28.346, 114.549, 0.05);
    at(&w, 6, "Blocks", 28.346, 134.076, 0.05);
    at(&w, 6, "narrow", 18.898, 151.228, 0.05);
    at(&w, 6, "wide,", 160.250, 144.756, 0.05);
    at(&w, 6, "column", 160.250, 158.305, 0.05);
    at(&w, 6, "wraps", 220.189, 158.305, 0.05);
    at(&w, 6, "Above", 44.934, 268.141, 0.05);
}

#[test]
fn title_page_sets_the_empty_institute_box() {
    if !lm_available() {
        return;
    }
    let w = words_of(&corpus_deck());
    // p1: no `\institute`, yet the template's `institute` colour box is a
    // 16pt `\hbox` under its `\lineskip`: the title sits 7.53bp higher and
    // the date 9.41bp lower than with the box dropped.
    at(&w, 1, "Blocks", 121.820, 96.143, 0.05);
    at(&w, 1, "S.", 162.629, 131.565, 0.05);
    at(&w, 1, "March", 154.342, 175.135, 0.05);
}

/// The ladder: `[t]` and `[b]` rows, `[onlytextwidth]` and `totalwidth=`
/// rows, blocks nested in `[T]` columns, a `figure` with an image and a
/// caption, a block after a list.
fn ladder() -> String {
    concat!(
        "\\documentclass{beamer}\n\\usepackage{graphicx}\n\\setbeamertemplate{navigation symbols}{}\n\\begin{document}\n",
        "\\begin{frame}\n\\frametitle{Top and bottom}\n",
        "\\begin{columns}[t]\n\\column{.4\\textwidth}\nAlpha line one.\\\\Alpha line two.\n\\column{.4\\textwidth}\nBeta one.\n\\end{columns}\n",
        "\\begin{columns}[b]\n\\column{.4\\textwidth}\nGamma line one.\\\\Gamma line two.\n\\column{.4\\textwidth}\nDelta one.\n\\end{columns}\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Only text width}\n",
        "\\begin{columns}[onlytextwidth]\n\\column{.3\\textwidth}\nLeft text.\n\\column{.3\\textwidth}\nRight text.\n\\end{columns}\n",
        "\\begin{columns}[totalwidth=6cm]\n\\column{2cm}\nNarrow.\n\\column{3cm}\nWider.\n\\end{columns}\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Nested}\n\\begin{columns}[T]\n\\column{.5\\textwidth}\n",
        "\\begin{alertblock}{Alert}\n\\begin{itemize}\n\\item one\n\\item two\n\\end{itemize}\n\\end{alertblock}\n",
        "\\column{.5\\textwidth}\n\\begin{block}{Two titles}\nBody.\n\\end{block}\n\\begin{exampleblock}{Example}\nMore.\n\\end{exampleblock}\n\\end{columns}\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{Figure}\n\\begin{figure}\n\\centering\n\\includegraphics[width=3cm]{figure.png}\n\\caption{A caption.}\n\\end{figure}\nAfter the figure.\n\\end{frame}\n",
        "\\begin{frame}\n\\frametitle{List then block}\n\\begin{itemize}\n\\item item\n\\end{itemize}\n\\begin{block}{After a list}\nText.\n\\end{block}\n\\end{frame}\n",
        "\\end{document}\n",
    )
    .to_string()
}

#[test]
fn ladder_rows_by_alignment_and_width_key() {
    if !lm_available() {
        return;
    }
    let r = render_in_fixture(&ladder());
    assert_eq!(r.v2.pages.len(), 5);
    let w = words_of(&r);
    // `[t]`: first baselines on the row; `[b]`: last baselines. The rows
    // are `.4\textwidth` columns of the paper-wide box.
    at(&w, 1, "Alpha", 39.308, 113.942, 0.05);
    at_n(&w, 1, "Alpha", 1, 39.308, 127.491, 0.05);
    at(&w, 1, "Beta", 201.072, 113.942, 0.05);
    at(&w, 1, "Gamma", 39.308, 138.184, 0.05);
    at_n(&w, 1, "Gamma", 1, 39.308, 151.734, 0.05);
    at(&w, 1, "Delta", 201.072, 151.734, 0.05);
    // `[onlytextwidth]`: the first column at the text edge, one `\hfill`
    // between (the trailing one is `\unskip`ped); `[c]` centres each
    // column on the math axis, so a line with a descender sits higher.
    at(&w, 2, "Left", 28.346, 124.275, 0.05);
    at(&w, 2, "Right", 242.645, 123.215, 0.05);
    // `totalwidth=6cm` with 2cm + 3cm columns: 1cm of fill between.
    at(&w, 2, "Narrow.", 28.346, 137.825, 0.05);
    at(&w, 2, "Wider.", 113.386, 137.825, 0.05);
}

#[test]
fn ladder_blocks_nested_in_t_columns_and_after_a_list() {
    if !lm_available() {
        return;
    }
    let r = render_in_fixture(&ladder());
    let w = words_of(&r);
    // A `[T]` column whose body starts with a block: the `\leavevmode`
    // line, `-1ex`, `\nointerlineskip`, then `\medskipamount` and the
    // title box; the list inside the block takes its 3pt `\topsep`.
    at(&w, 3, "Alert", 18.898, 114.028, 0.05);
    at(&w, 3, "one", 40.716, 130.350, 0.05);
    at(&w, 3, "two", 40.716, 146.888, 0.05);
    at(&w, 3, "Two", 190.866, 114.028, 0.05);
    at(&w, 3, "Body.", 190.866, 127.578, 0.05);
    at(&w, 3, "Example", 190.866, 150.093, 0.05);
    at(&w, 3, "More.", 190.866, 165.751, 0.05);
    assert_eq!(run_paint(&r, 3, "Alert"), (1.0, 0.0, 0.0));
    // A block right after a list: `\@endparenv`'s 3pt then `\medskipamount`.
    at(&w, 5, "item", 50.165, 115.230, 0.05);
    at(&w, 5, "After", 28.346, 137.746, 0.05);
    at(&w, 5, "Text.", 28.346, 151.295, 0.05);
}

#[test]
fn ladder_figure_with_image_and_caption() {
    if !lm_available() {
        return;
    }
    let r = render_in_fixture(&ladder());
    let w = words_of(&r);
    // `figure` is `center`: 9pt `\topsep`, the 3cm image, `\vskip 7pt`,
    // the caption under the body's `\baselineskip` (13.6 - 6.944), 7pt,
    // the closing 9pt, then the paragraph.
    let imgs = images(&r, 4);
    assert_eq!(imgs.len(), 1, "{imgs:?}");
    let (x, top, width, height) = imgs[0];
    assert!((x - 138.898).abs() <= 0.05 && (top - 85.309).abs() <= 0.05, "image at ({x:.3}, {top:.3}), pdflatex (138.898, 85.309)");
    assert!((width - 85.041).abs() <= 0.05 && (height - 51.859).abs() <= 0.05, "image {width:.3} x {height:.3}, pdflatex 85.041 x 51.859");
    at(&w, 4, "Figure:", 143.767, 157.691, 0.05);
    at(&w, 4, "caption.", 185.835, 157.691, 0.05);
    at(&w, 4, "After", 28.346, 187.181, 0.05);
}
