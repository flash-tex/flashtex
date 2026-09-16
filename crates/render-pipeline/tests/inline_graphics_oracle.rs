//! GH-762: an `\includegraphics` **outside** a float reserves the vertical
//! space pdfTeX reserves.
//!
//! Before this gate the pipeline dropped the box entirely — no width, no
//! height, nothing on the horizontal list — so the first baseline after the
//! graphic sat the graphic's whole height too high (−88.673 bp at 10 pt for
//! a 200 pt-wide image, −200.996 bp for one wider than `\textwidth`).
//!
//! Every number below is the first baseline of the `zzz` paragraph after the
//! graphic, in bp from the page top, measured with
//! `/Library/TeX/texbin/pdflatex` (pdfTeX 1.40.29, TeX Live 2026) on exactly
//! the sources built here, and read back with PyMuPDF glyph origins. The
//! image is `fixtures/float-graphics/images/objstm.pdf`, natural 150 x 60 bp.
//!
//! `float` is the control: the same graphic inside a `figure[h!]` was
//! already correct before the fix, and must not move. `draft` is the
//! missing-ink form of the in-text case; it shares the sub-gate
//! `\Gscale@div` quantisation of the float path and is held to the same
//! 0.5 bp glyph gate, not to exact equality.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

/// The project's glyph-position gate.
const GATE: f64 = 0.5;

/// `(name, body, [10 pt, 11 pt, 12 pt] pdflatex baseline of `zzz`, bp)`.
const CASES: &[(&str, &str, [f64; 3])] = &[
    // An image alone in a paragraph, at a requested width.
    ("width", "\\includegraphics[width=200pt]{images/objstm.pdf}", [186.578, 190.762, 193.551]),
    // The same image at its natural size.
    ("nat", "\\includegraphics{images/objstm.pdf}", [166.869, 171.053, 173.843]),
    // `height=` rather than `width=`.
    ("height", "\\includegraphics[height=50pt]{images/objstm.pdf}", [156.682, 160.867, 163.656]),
    // `scale=` rather than `width=`.
    ("scale", "\\includegraphics[scale=2]{images/objstm.pdf}", [226.869, 231.053, 233.843]),
    // Inline with text on the same line: the image raises that line, so the
    // next baseline moves by the image's height minus what `\baselineskip`
    // already held.
    ("inline", "Charlie \\includegraphics[width=40pt]{images/objstm.pdf} delta.", [122.809, 126.994, 129.783]),
    // Wider than `\textwidth` (an overfull `\hbox`): pdfTeX still sets the
    // box, and the line still grows by its height.
    ("over", "\\includegraphics[width=600pt]{images/objstm.pdf}", [346.071, 350.255, 353.045]),
    // Already correct before the fix, and must stay correct: the same
    // graphic inside a float.
    ("float", "\\begin{figure}[h!]\n\\includegraphics[width=200pt]{images/objstm.pdf}\n\\end{figure}", [209.492, 213.676, 220.451]),
    // `draft` reserves the same space and paints a frame instead of the ink.
    ("draft", "\\includegraphics[draft,width=200pt]{images/objstm.pdf}", [186.578, 190.762, 193.551]),
];

fn source(size: u32, body: &str) -> String {
    format!(
        "\\documentclass[{size}pt]{{article}}\n\
         \\usepackage[margin=1in]{{geometry}}\n\
         \\usepackage{{graphicx}}\n\
         \\pagestyle{{empty}}\n\
         \\setlength{{\\parindent}}{{0pt}}\n\
         \\begin{{document}}\n\
         Alpha alpha alpha alpha.\n\n\
         Bravo bravo bravo bravo.\n\n\
         {body}\n\n\
         zzz zzz zzz zzz.\n\
         \\end{{document}}\n"
    )
}

/// The first baseline, in bp from the page top, of a run whose text starts
/// the `zzz` paragraph, with the page it is on.
fn zzz_baseline(r: &flashtex_render_pipeline::Rendered) -> Option<(u32, f64)> {
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let Item::GlyphRun(run) = it {
                if run.text.starts_with("zzz") {
                    return run.glyphs.first().map(|g| (page.number, g.baseline_y.to_bp()));
                }
            }
        }
    }
    None
}

#[test]
fn inline_graphics_reserve_pdflatex_space() {
    if !common::lm_available() {
        return;
    }
    // The image lives with the float-graphics fixtures; only the sources
    // here are new, so there is one copy of the file in the tree.
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/float-graphics");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(root.into()), ..RenderOptions::default() };
    let mut failures = Vec::new();
    for (name, body, refs) in CASES {
        for (i, size) in [10u32, 11, 12].into_iter().enumerate() {
            let tex = source(size, body);
            let r = render(&[SourceDocument { path: "main.tex", text: &tex }], "main.tex", 1, "inline-graphics", &fonts, &options);
            for d in &r.v2.diagnostics {
                if d.code.starts_with("image") || d.code.starts_with("graphics") {
                    failures.push(format!("{name}-{size}: diagnostic {}: {}", d.code, d.message));
                }
            }
            match zzz_baseline(&r) {
                None => failures.push(format!("{name}-{size}: no `zzz` run in the display list")),
                Some((page, y)) => {
                    if page != 1 {
                        failures.push(format!("{name}-{size}: `zzz` on page {page}, pdflatex has it on page 1"));
                    }
                    let dy = y - refs[i];
                    if dy.abs() > GATE {
                        failures.push(format!("{name}-{size}: baseline {y:.3} bp, pdflatex {:.3} bp (dy {dy:+.3}, gate {GATE})", refs[i]));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The ink is drawn, not only reserved: an in-text graphic puts an image
/// item on the page whose box is the graphicx box (200 pt wide, 80 pt tall
/// for a 150 x 60 bp image), sitting on the line it is in.
#[test]
fn inline_graphic_paints_its_image() {
    if !common::lm_available() {
        return;
    }
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/float-graphics");
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(root.into()), ..RenderOptions::default() };
    let tex = source(10, "\\includegraphics[width=200pt]{images/objstm.pdf}");
    let r = render(&[SourceDocument { path: "main.tex", text: &tex }], "main.tex", 1, "inline-graphics", &fonts, &options);
    let images: Vec<_> = r
        .v2
        .pages
        .iter()
        .flat_map(|p| {
            p.resident_items().iter().filter_map(|it| match it {
                Item::Image(i) => Some((i.x.to_bp(), i.top.to_bp(), i.width.to_bp(), i.height.to_bp())),
                _ => None,
            })
        })
        .collect();
    assert_eq!(images.len(), 1, "one image expected, got {images:?}");
    let (x, top, w, h) = images[0];
    // 200 TeX pt = 199.272 bp wide; the aspect ratio is kept, so 79.709 bp
    // tall. pdflatex paints it at x = 72 bp (the 1 in margin) with its top
    // edge 94.913 bp down the page (reference rect 72, 94.913, 271.272,
    // 174.622). The 1 bp image gate applies.
    assert!((x - 72.0).abs() < 1.0, "x {x}");
    assert!((w - 199.272).abs() < 1.0, "width {w}");
    assert!((h - 79.709).abs() < 1.0, "height {h}");
    assert!((top - 94.913).abs() < 1.0, "top {top}");
}
