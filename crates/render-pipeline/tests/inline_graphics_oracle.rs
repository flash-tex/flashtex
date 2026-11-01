//! GH-762: inline (non-float) `\includegraphics` must reserve its box height.
//!
//! A graphic standing alone between paragraphs is one `\hbox` on its own
//! line (graphicx's `\Gin@setfile` + `\leavevmode`); the line's height is
//! the graphic's height and the text after it starts below it. The expected
//! `xxx` baselines below are real pdflatex (TeX Live 2026) oracle values --
//! glyph span origins extracted from the PDF (not `\pdfsavepos` in vertical
//! mode, which reports the previous line) -- against the fixture image
//! `images/pic-72x36.pdf` (a 72 x 36 bp PDF page). Glyph gate: 0.5 bp.
//!
//! Probe oracle (`xxx` baseline, bp from the page top):
//! `inl-real` 206.501, `inl-realnat` 142.869, `dw100` 206.501,
//! `dw200` 306.134, `dw300` 405.830, `dnat` 178.869, `dsc2` 250.869.

mod common;

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::Item;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const GATE: f64 = 0.5;
const IMAGE_GATE: f64 = 1.0;

const PRE: &str = r"\documentclass[10pt]{article}
\usepackage[margin=1in]{geometry}
\usepackage{graphicx}
\pagestyle{empty}
\setlength{\parindent}{0pt}
\begin{document}
Alpha alpha alpha alpha.

Bravo bravo bravo bravo.
";

const POST: &str = r"
xxx xxx xxx xxx.
\end{document}
";

struct Probe {
    baseline: f64,
    images: Vec<(f64, f64, f64, f64)>,
    diags: Vec<String>,
}

fn render_probe(graphic: &str, project_root: &str) -> Probe {
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions { project_root: Some(project_root.into()), ..RenderOptions::default() };
    let tex = format!("{PRE}\n{graphic}\n{POST}");
    let sources = [SourceDocument { path: "main.tex", text: &tex }];
    let r = render(&sources, "main.tex", 1, "inline-graphics", &fonts, &options);
    let diags: Vec<String> = r.v2.diagnostics.iter().map(|d| format!("{}: {}", d.code, d.message)).collect();
    let words = common::words_of(&r);
    let baseline = words
        .iter()
        .find(|w| w.page == 1 && w.text.starts_with("xxx"))
        .unwrap_or_else(|| panic!("no xxx line in {words:?}\n{diags:?}"))
        .baseline;
    let mut images = Vec::new();
    for page in &r.v2.pages {
        for it in page.resident_items() {
            if let Item::Image(i) = it {
                images.push((i.x.to_bp(), i.top.to_bp(), i.width.to_bp(), i.height.to_bp()));
            }
        }
    }
    Probe { baseline, images, diags }
}

#[test]
fn inline_graphics_reserve_height() {
    if !common::lm_available() {
        return;
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/inline-graphics");
    // (probe, graphic line, oracle xxx baseline in bp from the page top)
    let probes: &[(&str, &str, f64)] = &[
        ("inl-real", r"\includegraphics[width=200pt]{images/pic-72x36.pdf}", 206.501),
        ("inl-realnat", r"\includegraphics{images/pic-72x36.pdf}", 142.869),
        ("dw100", r"\includegraphics[draft,width=100pt]{nofile-zzz.png}", 206.501),
        ("dw200", r"\includegraphics[draft,width=200pt]{nofile-zzz.png}", 306.134),
        ("dw300", r"\includegraphics[draft,width=300pt]{nofile-zzz.png}", 405.830),
        ("dnat", r"\includegraphics[draft]{nofile-zzz.png}", 178.869),
        ("dsc2", r"\includegraphics[draft,scale=2]{nofile-zzz.png}", 250.869),
    ];
    let mut failures = Vec::new();
    for (name, graphic, want) in probes {
        let p = render_probe(graphic, dir);
        if (p.baseline - want).abs() > GATE {
            failures.push(format!("{name}: xxx baseline {:.3}bp vs oracle {want:.3}bp (dy={:.3}bp)\ndiags: {:?}", p.baseline, p.baseline - want, p.diags));
        }
        // No error diagnostics; a missing file under `draft` warns (its 1 in
        // natural size is kept, as pdfTeX does) and is expected here.
        for d in &p.diags {
            if d.starts_with("image_unavailable") && graphic.contains("draft") {
                continue;
            }
            failures.push(format!("{name}: unexpected diagnostic {d}"));
        }
    }
    // Image gate on the real file: 200pt x 100pt = 199.253 x 99.626 bp.
    let p = render_probe(r"\includegraphics[width=200pt]{images/pic-72x36.pdf}", dir);
    match p.images.as_slice() {
        [(x, top, w, h)] => {
            for (got, want, what) in [(*x, 72.0, "x"), (*top, 94.914, "top"), (*w, 199.253, "width"), (*h, 99.626, "height")] {
                if (got - want).abs() > IMAGE_GATE {
                    failures.push(format!("inl-real: image {what} {got:.3}bp vs oracle {want:.3}bp"));
                }
            }
        }
        other => failures.push(format!("inl-real: expected 1 image, got {}", other.len())),
    }
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
