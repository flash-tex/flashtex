//! A `tikzpicture` in a document becomes display-list-v2 path items plus
//! glyph runs for its node text; the compiler's "unknown environment" text
//! and diagnostics for the picture body are gone, the surrounding text stays.

use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::display::{Item, PathCmd, PathPaintOp, TICKS_PER_BP};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

const SRC: &str = "Before text.\n\n\\begin{tikzpicture}\n\\draw[->] (0,0) -- (2,1) node[midway,above] {hi};\n\\node[draw] at (0,0) {box};\n\\clip (0,0) circle (1);\n\\fill[red] (0,0) rectangle (1,1);\n\\end{tikzpicture}\n\nAfter text.\n";

#[test]
fn tikzpicture_becomes_paths_and_glyph_runs() {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let docs = [SourceDocument { path: "main.tex", text: SRC }];
    let out = render(&docs, "main.tex", 1, "tikz", &fonts, &RenderOptions::default());
    let page = &out.v2.pages[0];
    let texts: Vec<&str> = page
        .resident_items()
        .iter()
        .filter_map(|i| if let Item::GlyphRun(r) = i { Some(r.text.as_str()) } else { None })
        .collect();
    assert_eq!(texts, ["Before", "text.", "hi", "box", "After", "text."], "{texts:?}");

    let paths: Vec<_> = page.resident_items().iter().filter_map(|i| if let Item::Path(p) = i { Some(p) } else { None }).collect();
    let strokes = paths.iter().filter(|p| matches!(p.op, PathPaintOp::Stroke(_))).count();
    let fills = paths.iter().filter(|p| matches!(p.op, PathPaintOp::Fill { .. })).count();
    assert_eq!(strokes, 3, "shaft, arrow tip, node border");
    assert_eq!(fills, 1);
    let clipped = paths.iter().find(|p| matches!(p.op, PathPaintOp::Fill { .. })).unwrap();
    assert_eq!(clipped.clips.len(), 1);
    assert_eq!(clipped.paint.r, 1.0);

    // The picture sits between the two paragraphs, its box starting at the
    // margin plus `\parindent` (this body-only document uses the default
    // `12pt`, 1in margins, `\parindent 0pt`, so the box is at the margin).
    // This used to be a one-sided `min_x >= 72bp` bound ("at the left
    // margin") which passed with the picture glued to the margin even when
    // an indent was due -- it encoded the bug. The locked value below is
    // the margin plus this picture's own left extent (the drawn node's
    // border); indented placement against the pdflatex oracle is covered by
    // the tests below.
    let ys = |i: &Item| -> Vec<i64> {
        match i {
            Item::Path(p) => p
                .commands
                .iter()
                .filter_map(|c| match *c {
                    PathCmd::Move(_, y) | PathCmd::Line(_, y) => Some(y.0),
                    _ => None,
                })
                .collect(),
            Item::GlyphRun(r) => r.glyphs.iter().map(|g| g.baseline_y.0).collect(),
            _ => vec![],
        }
    };
    let before_y = ys(&page.resident_items()[0])[0];
    let after_y = *ys(page.resident_items().iter().rev().find(|i| matches!(i, Item::GlyphRun(_))).unwrap()).last().unwrap();
    for p in page.resident_items() {
        if let Item::Path(_) = p {
            for y in ys(p) {
                assert!(y > before_y && y < after_y, "path y {y} outside ({before_y}, {after_y})");
            }
        }
    }
    let min_x = paths
        .iter()
        .flat_map(|p| p.commands.iter())
        .filter_map(|c| if let PathCmd::Move(x, _) | PathCmd::Line(x, _) = *c { Some(x.0) } else { None })
        .min()
        .unwrap();
    let min_x_bp = min_x as f64 / TICKS_PER_BP;
    assert!((min_x_bp - 87.094).abs() <= 0.5, "path min_x: {min_x_bp}bp");

    // No compiler complaints about \draw or the unknown environment.
    for d in &out.v2.diagnostics {
        assert!(
            !matches!(
                d.code.as_str(),
                "compiler" | "unknown_command" | "unsupported_feature" | "syntax_error" | "export_limitation" | "fidelity_note" | "recovered_input"
            ),
            "{d:?}"
        );
    }
    let features = out.v2.required_features();
    assert!(features.contains(&"path_stroke") && features.contains(&"path_fill") && features.contains(&"clip"), "{features:?}");
    let json = flashtex_compiler::json::write(&out.v2.to_json("x"));
    assert!(json.contains("\"kind\":\"path_stroke\"") && json.contains("\"clips\""));
}

/// Paragraph-indent placement of a `tikzpicture` against the pdflatex oracle.
///
/// Preamble for every document below: `\documentclass{article}` plus
/// `\usepackage{tikz}` (default 10pt), so the class geometry (1in +
/// `\oddsidemargin`, `\parindent 15pt`) is live. The oracle numbers are
/// node-text xMin in PDF user units (bp), measured with TeX Live 2026
/// pdflatex 1.40.29 (`/Library/TeX/texbin/pdflatex`) and char boxes read
/// with pymupdf; the tikz package is confirmed loaded because the node
/// text is set as its own line (an unloaded tikz would leave the body as
/// paragraph text). A centred `\node at (0,0) {hi}` starts its text one
/// inner sep inside the picture box whatever the text is, so xMin is the
/// picture box x plus that sep:
///
/// ```text
/// paragraph-initial  152.033  (= indented text 148.712 + inner sep ~3.32)
/// \noindent          137.089  (= margin 133.768 + inner sep)
/// two consecutive    152.033, 152.033 (blank line between the pictures)
/// \parindent=0pt     137.089
/// inside itemize     node 161.996, bullet 148.714 (hang, no parindent)
/// inside center      301.473 (centred, unchanged)
/// ```
const INDENT_PREAMBLE: &str = "\\documentclass{article}\n\\usepackage{tikz}\n\\begin{document}\n";
const INDENT_PIC: &str = "\\begin{tikzpicture}\n\\node at (0,0) {hi};\n\\end{tikzpicture}\n";

/// Left edges, in bp, of every glyph run with text `want`, in order.
fn run_xmins_bp(page: &flashtex_render_pipeline::display::Page, want: &str) -> Vec<f64> {
    page.resident_items()
        .iter()
        .filter_map(|i| match i {
            Item::GlyphRun(r) if r.text == want => Some(r.glyphs.iter().map(|g| g.origin_x.to_bp()).fold(f64::INFINITY, f64::min)),
            _ => None,
        })
        .collect()
}

/// Renders one preamble-fixed document, or `None` when Latin Modern is
/// missing (skip, as in the test above).
fn render_body_or_skip(body: &str) -> Option<flashtex_render_pipeline::Rendered> {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return None;
    }
    let src = format!("{INDENT_PREAMBLE}{body}\\end{{document}}\n");
    let docs = [SourceDocument { path: "main.tex", text: &src }];
    Some(render(&docs, "main.tex", 1, "tikz", &fonts, &RenderOptions::default()))
}

#[test]
fn picture_paragraph_indent_matches_pdflatex() {
    let Some(out) = render_body_or_skip(&format!("Before text.\n\n{INDENT_PIC}\nAfter text.\n")) else { return };
    assert_eq!(out.v2.pages.len(), 1);
    let page = &out.v2.pages[0];
    // Oracle: 152.033.
    let hi = run_xmins_bp(page, "hi");
    assert_eq!(hi.len(), 1, "{hi:?}");
    assert!((hi[0] - 152.033).abs() <= 0.5, "node-text xMin: {}bp", hi[0]);
    // Controls: ordinary text must not move (oracle 148.712, 0.0 delta).
    for (want, xs) in [("Before", run_xmins_bp(page, "Before")), ("After", run_xmins_bp(page, "After"))] {
        assert_eq!(xs.len(), 1, "{want}: {xs:?}");
        assert!((xs[0] - 148.712).abs() <= 0.05, "{want} xMin: {}bp", xs[0]);
    }
}

#[test]
fn picture_noindent_matches_pdflatex() {
    let Some(out) = render_body_or_skip(&format!("Before text.\n\n\\noindent{INDENT_PIC}\nAfter text.\n")) else { return };
    assert_eq!(out.v2.pages.len(), 1);
    let page = &out.v2.pages[0];
    // Oracle: 137.089; ordinary text stays at 148.712.
    let hi = run_xmins_bp(page, "hi");
    assert_eq!(hi.len(), 1, "{hi:?}");
    assert!((hi[0] - 137.089).abs() <= 0.5, "node-text xMin: {}bp", hi[0]);
    let before = run_xmins_bp(page, "Before");
    assert_eq!(before.len(), 1, "{before:?}");
    assert!((before[0] - 148.712).abs() <= 0.05, "Before xMin: {}bp", before[0]);
}

#[test]
fn picture_two_consecutive_paragraphs_match_pdflatex() {
    let Some(out) = render_body_or_skip(&format!("Before text.\n\n{INDENT_PIC}\n{INDENT_PIC}\nAfter text.\n")) else { return };
    assert_eq!(out.v2.pages.len(), 1);
    let page = &out.v2.pages[0];
    // Oracle: both 152.033.
    let hi = run_xmins_bp(page, "hi");
    assert_eq!(hi.len(), 2, "{hi:?}");
    for (i, x) in hi.iter().enumerate() {
        assert!((x - 152.033).abs() <= 0.5, "picture {i} node-text xMin: {x}bp");
    }
}

#[test]
fn picture_zero_parindent_matches_pdflatex() {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let src = format!(
        "\\documentclass{{article}}\n\\usepackage{{tikz}}\n\\setlength{{\\parindent}}{{0pt}}\n\\begin{{document}}\nBefore text.\n\n{INDENT_PIC}\nAfter text.\n\\end{{document}}\n"
    );
    let docs = [SourceDocument { path: "main.tex", text: &src }];
    let out = render(&docs, "main.tex", 1, "tikz", &fonts, &RenderOptions::default());
    assert_eq!(out.v2.pages.len(), 1);
    let page = &out.v2.pages[0];
    // Oracle: node 137.089, text at the unindented margin 133.768.
    let hi = run_xmins_bp(page, "hi");
    assert_eq!(hi.len(), 1, "{hi:?}");
    assert!((hi[0] - 137.089).abs() <= 0.5, "node-text xMin: {}bp", hi[0]);
    let before = run_xmins_bp(page, "Before");
    assert_eq!(before.len(), 1, "{before:?}");
    assert!((before[0] - 133.768).abs() <= 0.05, "Before xMin: {}bp", before[0]);
}

#[test]
fn picture_list_item_hang_matches_pdflatex() {
    let Some(out) = render_body_or_skip(&format!("\\begin{{itemize}}\n\\item {INDENT_PIC}\\end{{itemize}}\n")) else { return };
    assert_eq!(out.v2.pages.len(), 1);
    let page = &out.v2.pages[0];
    // Oracle: node 161.996 (item text at the hanging indent plus inner
    // sep), no `\parindent`.
    let hi = run_xmins_bp(page, "hi");
    assert_eq!(hi.len(), 1, "{hi:?}");
    assert!((hi[0] - 161.996).abs() <= 0.5, "node-text xMin: {}bp", hi[0]);
}

#[test]
fn picture_center_placement_is_unchanged() {
    let Some(out) = render_body_or_skip(&format!("\\begin{{center}}\n{INDENT_PIC}\\end{{center}}\n")) else { return };
    assert_eq!(out.v2.pages.len(), 1);
    let page = &out.v2.pages[0];
    // Oracle: 301.473; the centred branch is untouched by the indent fix.
    let hi = run_xmins_bp(page, "hi");
    assert_eq!(hi.len(), 1, "{hi:?}");
    assert!((hi[0] - 301.473).abs() <= 0.5, "node-text xMin: {}bp", hi[0]);
}

#[cfg(feature = "tikz-patterns")]
#[test]
fn tikz_pattern_is_emitted_in_the_v2_json_shape() {
    let fonts = FontSet::with_default_dirs(&[]);
    if !fonts.latin_modern_available() {
        eprintln!("SKIP: Latin Modern fonts are not installed");
        return;
    }
    let src = r"\begin{tikzpicture}
\fill[pattern=grid,pattern color=red!20] (0,0) rectangle (1,1);
\end{tikzpicture}";
    let docs = [SourceDocument { path: "pattern.tex", text: src }];
    let out = render(&docs, "pattern.tex", 1, "patterns", &fonts, &RenderOptions::default());
    let path = out.v2.pages.iter().flat_map(|p| &p.items).find_map(|item| match item {
        Item::Path(path) if matches!(path.op, PathPaintOp::Fill { .. }) => Some(path),
        _ => None,
    }).expect("pattern fill");
    let pattern = path.pattern.as_ref().expect("feature-gated pattern");
    assert_eq!(pattern.name, "grid");
    assert_eq!(pattern.color, [1.0, 0.8, 0.8]);

    let text = out.v2.write_json("patterns");
    assert!(text.contains("\"pattern\":{\"color\":{\"b\":0.8,\"g\":0.8,\"r\":1},\"name\":\"grid\"}"), "{text}");
    assert_eq!(text, flashtex_compiler::json::write(&out.v2.to_json("patterns")));
}
