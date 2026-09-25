//! Kernel `\fbox`, `\framebox` and `\makebox` in text mode.
//!
//! The student convention `\fbox{42}` (a boxed final numeric answer) used to
//! report `error[unsupported_feature] \fbox is not supported by this compiler
//! version`; its math-mode sibling `\boxed{...}` already drew a frame.
//!
//! ## Oracle (every value measured, not assumed)
//!
//! TeX Live 2026 (`/Library/TeX/texbin/pdflatex
//! -interaction=nonstopmode /tmp/fbox-oracle/fbox.tex`, exit code 0, zero
//! `!` errors; `texdef -t latex` for the macro shapes):
//! - `\fbox#1` is `\leavevmode\setbox\@tempboxa\hbox{\kern\fboxsep{#1}\kern
//!   \fboxsep}\@frameb@x\relax` with `\the\fboxsep` = `3.0pt` and
//!   `\the\fboxrule` = `0.4pt` (both from `\typeout` lines in `fbox.log`).
//! - `\framebox` is `\@ifnextchar[\@framebox\fbox`: without optionals it IS
//!   `\fbox`. `\@framebox[width]` defaults the position to `c`.
//! - `\makebox` is `\@ifnextchar[\@makebox\mbox`: without optionals it IS
//!   `\mbox`. `\@imakebox[width][pos]` adds no `\fboxsep` (unlike
//!   `\@iframebox`, which kerns `\fboxsep` on both sides).
//! - An unknown position warns (`LaTeX Warning: Unexpected alignment 'z' ..
//!   .`) and centers (`\bm@c`).
//! - PyMuPDF glyph-advance-left measurements from the oracle PDF (bp):
//!   `\framebox{42}` sets `42` at exactly the advance-left of `\fbox{42}`
//!   (delta 0.000bp), and `\makebox{42}` at exactly that of `\mbox{42}`
//!   (delta 0.000bp). The compiler's Core 14 layout splices every box's
//!   content inline, so these tests pin the same identity there: the two
//!   variants' content starts at exactly the same `x_pt` (tolerance 0,
//!   tighter than the 0.1bp the task allows).
//!
//! Fixed `[width]` geometry (the box exactly `[width]` wide with the content
//! at `[pos]`) is layout work beyond the compiler's node shapes (the Core 14
//! layout has no box model: it splices box content inline, like `\colorbox`
//! and `\mbox`); the optionals are parsed and validated, and the box is set
//! at its natural width with its frame, exactly as the sibling
//! `\colorbox`/`\frame`-environment nodes already render downstream.
use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};
use flashtex_compiler::parser::{parse, Block, ColorBox, HBox, Inline, Parsed};
use flashtex_compiler::vocabulary::{is_known_command, is_listed_as_unimplemented};

/// Plain article, no packages: all three commands are kernel (latex.ltx),
/// so they must work here exactly as under pdflatex.
fn document(body: &str) -> String {
    document_pre("", body)
}

fn document_pre(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn parse_body(body: &str) -> Parsed {
    parse(&document(body))
}

fn messages(parsed: &Parsed) -> Vec<String> {
    parsed.diagnostics.iter().map(|d| d.message.clone()).collect()
}

fn paragraph_inlines(source: &str) -> Vec<Inline> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    paragraph_of(&parsed, source)
}

fn paragraph_of(parsed: &Parsed, source: &str) -> Vec<Inline> {
    let mut out = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            out.extend(inlines.clone());
        }
    }
    assert!(!out.is_empty(), "no paragraph in {source:?}");
    out
}

fn colorboxes(inlines: &[Inline]) -> Vec<&ColorBox> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::ColorBox(boxed) => Some(boxed.as_ref()),
            _ => None,
        })
        .collect()
}

fn hboxes(inlines: &[Inline]) -> Vec<&HBox> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::HBox(boxed) => Some(boxed.as_ref()),
            _ => None,
        })
        .collect()
}

fn texts(content: &[Inline]) -> Vec<&str> {
    content
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

fn compile(source: &str) -> CompileOutput {
    let output = compile_full(source, LayoutConstraints::default());
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    output
}

/// The `x_pt` of the `text` item: the compiler splices box content inline,
/// so this is where the box's content starts on the line.
fn item_x(output: &CompileOutput, text: &str) -> f64 {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.text == text)
        .unwrap_or_else(|| panic!("missing {text:?}: {:?}", output.pages))
        .x_pt
}

/// `\fbox{42}` is one framed box: an `\fboxrule` frame in the current colour
/// around `\fboxsep`-padded content, with the page colour as its (invisible)
/// fill — the same node `\boxed`, `\colorbox` and the `frame` environment
/// already produce, so the existing renderer draws it.
#[test]
fn fbox_boxes_its_argument_with_a_frame() {
    let inlines = paragraph_inlines(&document("The answer is \\fbox{42}."));
    let found = colorboxes(&inlines);
    assert_eq!(found.len(), 1, "{inlines:?}");
    let framed = found[0];
    // Oracle `fbox.log`: `\the\fboxsep` = 3.0pt, `\the\fboxrule` = 0.4pt.
    assert_eq!(framed.fboxsep_pt, 3.0);
    assert_eq!(framed.fboxrule_pt, 0.4);
    // Black frame on a default page, like `\fbox` under pdflatex.
    assert_eq!(
        framed.frame.map(|c| c.fill_operator()).as_deref(),
        Some("0 g")
    );
    assert_eq!(framed.fill.fill_operator(), "1 g");
    assert_eq!(texts(&framed.content), ["42"]);
    // The box spans the whole command, like `\mbox`'s does.
    let source = document("The answer is \\fbox{42}.");
    assert_eq!(&source[found[0].span.start..found[0].span.end], "\\fbox{42}");
}

/// `\setlength{\fboxsep}/\fboxrule` reach the box, as they do for `\colorbox`
/// and the `frame` environment.
#[test]
fn fbox_honours_set_fboxsep_and_fboxrule() {
    let inlines = paragraph_inlines(&document(
        "\\setlength{\\fboxsep}{5pt}\\setlength{\\fboxrule}{2pt}\\fbox{x}",
    ));
    let found = colorboxes(&inlines);
    assert_eq!(found.len(), 1, "{inlines:?}");
    assert_eq!(found[0].fboxsep_pt, 5.0);
    assert_eq!(found[0].fboxrule_pt, 2.0);
}

/// The frame follows `\color` and the fill follows `\pagecolor`, exactly
/// like the `frame` environment's box.
#[test]
fn fbox_uses_current_colour_and_page_colour() {
    let inlines = paragraph_inlines(&document_pre(
        "\\usepackage{xcolor}\\pagecolor{yellow!20}",
        "{\\color{red}\\fbox{x}}",
    ));
    let found = colorboxes(&inlines);
    assert_eq!(found.len(), 1, "{inlines:?}");
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("1 0 0 rg")
    );
}

/// `\framebox{42}` without optionals IS `\fbox{42}` (oracle: `\@framebox`
/// falls through to `\fbox`): the same framed node, and the content at
/// exactly the same `x_pt` — the oracle PDF sets both `42`s at the same
/// advance-left (delta 0.000bp).
#[test]
fn fbox_framebox_without_width_matches_fbox_exactly() {
    let plain = paragraph_inlines(&document("A \\fbox{42} B"));
    let optional = paragraph_inlines(&document("A \\framebox{42} B"));
    let (Some(plain), Some(optional)) = (
        colorboxes(&plain).into_iter().next(),
        colorboxes(&optional).into_iter().next(),
    ) else {
        panic!("both forms box their argument: {plain:?} {optional:?}");
    };
    assert_eq!(plain.fboxsep_pt, optional.fboxsep_pt);
    assert_eq!(plain.fboxrule_pt, optional.fboxrule_pt);
    assert_eq!(plain.frame, optional.frame);
    assert_eq!(texts(&plain.content), texts(&optional.content));
    let plain_x = item_x(&compile(&document("A \\fbox{42} B")), "42");
    let optional_x = item_x(&compile(&document("A \\framebox{42} B")), "42");
    assert_eq!(plain_x, optional_x);
}

/// `\makebox{42}` without optionals IS `\mbox{42}` (oracle: `\@makebox`
/// falls through to `\mbox`): one unframed `HBox`, and the content at
/// exactly the same `x_pt` — the oracle PDF sets both `42`s at the same
/// advance-left (delta 0.000bp).
#[test]
fn fbox_makebox_without_width_is_an_hbox_like_mbox() {
    let inlines = paragraph_inlines(&document("A \\makebox{42} B"));
    assert!(colorboxes(&inlines).is_empty(), "{inlines:?}");
    let found = hboxes(&inlines);
    assert_eq!(found.len(), 1, "{inlines:?}");
    assert_eq!(texts(&found[0].content), ["42"]);
    let plain_x = item_x(&compile(&document("A \\mbox{42} B")), "42");
    let optional_x = item_x(&compile(&document("A \\makebox{42} B")), "42");
    assert_eq!(plain_x, optional_x);
}

/// The `[width][pos]` forms parse their optionals and set the content with
/// no diagnostic: `\framebox` framed, `\makebox` unframed. The default
/// position is `c`, like `\@framebox`/`\@makebox`.
#[test]
fn fbox_framebox_and_makebox_width_and_pos_forms_set_content() {
    for (body, framed, word) in [
        ("\\framebox[2cm][l]{x}", true, "x"),
        ("\\framebox[2cm][c]{y}", true, "y"),
        ("\\framebox[2cm][r]{z}", true, "z"),
        ("\\framebox[2cm]{w}", true, "w"),
        ("\\makebox[3cm][l]{x}", false, "x"),
        ("\\makebox[3cm][c]{y}", false, "y"),
        ("\\makebox[3cm][r]{z}", false, "z"),
        ("\\makebox[3cm]{w}", false, "w"),
    ] {
        let parsed = parse_body(body);
        assert!(
            parsed.diagnostics.is_empty(),
            "{body:?}: {:?}",
            parsed.diagnostics
        );
        let inlines = paragraph_of(&parsed, body);
        assert_eq!(colorboxes(&inlines).len() == 1, framed, "{body:?}: {inlines:?}");
        assert_eq!(hboxes(&inlines).len() == 1, !framed, "{body:?}: {inlines:?}");
        let content = if framed {
            &colorboxes(&inlines)[0].content
        } else {
            &hboxes(&inlines)[0].content
        };
        assert_eq!(texts(content), [word], "{body:?}: {inlines:?}");
    }
}

/// An unknown position warns and centers, like pdflatex's
/// `LaTeX Warning: Unexpected alignment` (`\@iframebox`/`\@imakebox` fall
/// back to `\bm@c`); the content is still set.
#[test]
fn fbox_framebox_unexpected_alignment_warns_like_pdflatex() {
    for body in ["\\framebox[2cm][z]{x}", "\\makebox[2cm][z]{x}"] {
        let parsed = parse_body(body);
        assert_eq!(parsed.diagnostics.len(), 1, "{body:?}: {:?}", parsed.diagnostics);
        let diag = &parsed.diagnostics[0];
        assert!(diag.message.contains("nexpected alignment"), "{body:?}: {diag:?}");
        assert!(diag.message.contains('z'), "{body:?}: {diag:?}");
    }
    // The warning is a warning: the box and its content still reach the page.
    let parsed = parse_body("\\framebox[2cm][z]{x}");
    let inlines = paragraph_of(&parsed, "\\framebox[2cm][z]{x}");
    assert_eq!(colorboxes(&inlines).len(), 1, "{inlines:?}");
}

/// A malformed width is diagnosed (pdflatex hard-errors on a bad length),
/// but the prose is never eaten: the box is still set at natural width.
#[test]
fn fbox_makebox_unrecognised_width_keeps_prose() {
    let parsed = parse_body("\\makebox[abc]{hello}");
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    assert!(
        parsed.diagnostics[0].message.contains("abc"),
        "{:?}",
        parsed.diagnostics
    );
    let output = compile_full(&document("\\makebox[abc]{hello}"), LayoutConstraints::default());
    assert_eq!(output.diagnostics.len(), 1);
    let words: Vec<&str> = output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| item.text.as_str())
        .collect();
    assert!(words.contains(&"hello"), "{words:?}");
}

/// An explicit `$...$` keeps its formula inside `\fbox`, like `\boxed`.
#[test]
fn fbox_keeps_explicit_math_formula() {
    let inlines = paragraph_inlines(&document("The answer is \\fbox{$f(x) = 2x$.}"));
    let found = colorboxes(&inlines);
    assert_eq!(found.len(), 1, "{inlines:?}");
    assert!(found[0].frame.is_some(), "frame must be drawn");
    assert!(
        found[0]
            .content
            .iter()
            .any(|inline| matches!(inline, Inline::Math { .. })),
        "explicit $...$ keeps its formula: {inlines:?}"
    );
}

/// Like `\mbox`, the box commands are body commands: preamble use is
/// diagnosed instead of typeset.
#[test]
fn fbox_in_the_preamble_is_diagnosed_like_mbox() {
    for body in ["\\fbox{x}", "\\framebox{x}", "\\makebox{x}"] {
        let source = format!(
            "\\documentclass{{article}}\n{body}\n\\begin{{document}}\nText.\n\\end{{document}}\n"
        );
        let parsed = parse(&source);
        assert!(!parsed.diagnostics.is_empty(), "{body}: silent");
        assert!(
            parsed.diagnostics.iter().all(|d| d.message.contains("preamble")),
            "{body}: {:?}",
            parsed.diagnostics
        );
    }
}

/// All three names are implemented kernel vocabulary now: known, and no
/// longer listed as unimplemented.
#[test]
fn fbox_commands_are_known_and_implemented() {
    for name in ["fbox", "framebox", "makebox"] {
        assert!(is_known_command(name), "{name}");
        assert!(
            !is_listed_as_unimplemented(name),
            "{name} must not be listed as unimplemented once it has a real dispatch arm"
        );
    }
}
