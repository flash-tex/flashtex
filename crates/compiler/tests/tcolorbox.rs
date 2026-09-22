//! The `tcolorbox` lane: `\begin{tcolorbox}[colback=..,colframe=..,title=..]`.
//!
//! The environment lowers to the same `Inline::ColorBox` that `\fcolorbox`
//! makes, but with tcolorbox.sty's own reset values (TeX Live 2026,
//! `size=normal`): a `0.5mm` rule, `1mm` padding, `colback=black!5!white`
//! fill and `colframe=black!75!white` frame. The expected operator strings
//! below are the pdfLaTeX oracle: an uncompressed
//! `\begin{tcolorbox}[colback=yellow!10,colframe=black]` probe emits
//! `0 0 0.1 0 k` for the fill and `0 g` for the frame, and a bare box emits
//! `0.95 g` / `0.25 g`.
//!
//! `title=<text>` draws a title bar directly above the box: a second
//! `ColorBox` filled with the frame colour (tcolorbox.sty's default, where
//! `title filled=false` leaves the title on the frame-coloured band)
//! carrying the title in white (`coltitle=white`; `fonttitle` is empty, so
//! no extra face), parsed with the ordinary dispatch so markup works.
//!
//! Shape (b) by design: the content is `box_inlines`' flattened single-line
//! run on its own paragraph, so a body longer than one line overflows rather
//! than re-flowing (the block-level follow-up). Rounded corners, the
//! `left/right/top/bottom` extras, the full-`\linewidth` width and every
//! library stay out of scope and warn instead of being silently dropped.

use flashtex_compiler::parser::{parse, Block, Inline, Parsed};

fn doc(preamble: &str, body: &str) -> Parsed {
    parse(&format!(
        "\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    ))
}

fn walk<'a>(inlines: &'a [Inline], out: &mut Vec<&'a Inline>) {
    for inline in inlines {
        out.push(inline);
        if let Inline::ColorBox(b) = inline {
            walk(&b.content, out);
        }
    }
}

fn inlines(parsed: &Parsed) -> Vec<&Inline> {
    let mut out = Vec::new();
    for block in &parsed.blocks {
        match block {
            Block::Paragraph(content)
            | Block::Styled { content, .. }
            | Block::ListItem { content, .. }
            | Block::Heading { content, .. } => walk(content, &mut out),
            _ => {}
        }
    }
    out
}

fn boxes<'a>(parsed: &'a Parsed) -> Vec<&'a flashtex_compiler::parser::ColorBox> {
    inlines(parsed)
        .into_iter()
        .filter_map(|i| match i {
            Inline::ColorBox(b) => Some(b.as_ref()),
            _ => None,
        })
        .collect()
}

fn messages(parsed: &Parsed) -> Vec<String> {
    parsed.diagnostics.iter().map(|d| d.message.clone()).collect()
}

fn texts(parsed: &Parsed) -> Vec<String> {
    inlines(parsed)
        .into_iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The `(text, style)` runs inside one box, for the title-bar assertions.
fn runs(boxed: &flashtex_compiler::parser::ColorBox) -> Vec<(&str, flashtex_compiler::parser::TextStyle)> {
    boxed
        .content
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, style, .. } => Some((text.as_str(), *style)),
            _ => None,
        })
        .collect()
}

/// tcolorbox.sty's `boxrule=0.5mm`, in TeX points.
const BOXRULE_PT: f64 = 0.5 * 72.27 / 25.4;
/// tcolorbox.sty's `boxsep=1mm`, in TeX points.
const BOXSEP_PT: f64 = 72.27 / 25.4;

#[test]
fn colback_only_uses_yellow_fill_with_default_frame() {
    // No `\usepackage{xcolor}` on purpose: tcolorbox.sty requires xcolor
    // itself, so the colour machinery must already resolve `yellow!10`.
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[colback=yellow!10]Hello world\\end{tcolorbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    let boxed = found[0];
    // pdfLaTeX fills this box with `0 0 0.1 0 k`.
    assert_eq!(boxed.fill.fill_operator(), "0 0 0.1 0 k");
    // Untouched `colframe` keeps tcolorbox's own default: pdfLaTeX `0.25 g`.
    assert_eq!(
        boxed.frame.map(|c| c.fill_operator()).as_deref(),
        Some("0.25 g")
    );
    assert_eq!(boxed.fboxrule_pt, BOXRULE_PT);
    assert_eq!(boxed.fboxsep_pt, BOXSEP_PT);
    let words = texts(&p);
    assert!(words.contains(&"Hello".to_string()), "{words:?}");
    assert!(words.contains(&"world".to_string()), "{words:?}");
}

#[test]
fn colframe_only_keeps_default_fill() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[colframe=red]Hi\\end{tcolorbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("1 0 0 rg")
    );
    // Untouched `colback` keeps tcolorbox's own default: pdfLaTeX `0.95 g`.
    assert_eq!(found[0].fill.fill_operator(), "0.95 g");
}

#[test]
fn colback_and_colframe_together() {
    let p = doc(
        "\\usepackage{xcolor}\n\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[colback=yellow!10,colframe=black]Hi\\end{tcolorbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    // pdfLaTeX paints exactly these two operators for this option pair.
    assert_eq!(found[0].fill.fill_operator(), "0 0 0.1 0 k");
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0 g")
    );
}

#[test]
fn bare_box_uses_tcolorbox_defaults() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}Plain box\\end{tcolorbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    // pdfLaTeX fills the no-frills box with `0.95 g` inside `0.25 g`.
    assert_eq!(found[0].fill.fill_operator(), "0.95 g");
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0.25 g")
    );
    assert_eq!(found[0].fboxrule_pt, BOXRULE_PT);
    assert_eq!(found[0].fboxsep_pt, BOXSEP_PT);
}

#[test]
fn unrecognised_keys_warn_and_keep_colback_colframe() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[colback=red,watermark text={a, b},sharp corners]Hi\\end{tcolorbox}",
    );
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    // The honoured key still applies; the box is rendered, not dropped.
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0.25 g")
    );
    assert_eq!(found[0].fill.fill_operator(), "1 0 0 rg");
    // One diagnostic names both ignored keys in option order;
    // `watermark text={a, b}` stays one key (the comma inside braces must
    // not split the option list into a stray `b`), and nothing is silently
    // dropped.
    let warned: Vec<_> = messages(&p)
        .into_iter()
        .filter(|m| m.contains("tcolorbox keys"))
        .collect();
    assert_eq!(warned.len(), 1, "{:?}", messages(&p));
    assert!(
        warned[0].contains("watermark text, sharp corners"),
        "{}",
        warned[0]
    );
}

#[test]
fn title_renders_bar_above_body_without_warning() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[colback=red,title=Example]Body words\\end{tcolorbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 2, "{:?}", texts(&p));
    // The bar comes first, filled with the frame colour (tcolorbox.sty's
    // default, where `title filled=false` leaves the title on the
    // frame-coloured band); the untouched `colframe` default paints
    // pdfLaTeX `0.25 g`.
    let bar = found[0];
    assert_eq!(bar.fill.fill_operator(), "0.25 g");
    assert_eq!(
        bar.frame.map(|c| c.fill_operator()).as_deref(),
        Some("0.25 g")
    );
    assert_eq!(bar.fboxrule_pt, BOXRULE_PT);
    assert_eq!(bar.fboxsep_pt, BOXSEP_PT);
    let heading = runs(bar);
    assert!(
        heading.iter().any(|(text, _)| *text == "Example"),
        "{heading:?}"
    );
    // `coltitle=white`: every title run paints pdfLaTeX `1 g`, with no
    // extra face (`fonttitle` is empty by default).
    assert!(!heading.is_empty());
    for (text, style) in &heading {
        assert_eq!(
            style.color.map(|c| c.fill_operator()).as_deref(),
            Some("1 g"),
            "{text}"
        );
        assert!(!style.bold && !style.italic, "{text}");
    }
    // The body box is unchanged: red fill, default frame, body text only.
    let body = found[1];
    assert_eq!(body.fill.fill_operator(), "1 0 0 rg");
    assert_eq!(
        body.frame.map(|c| c.fill_operator()).as_deref(),
        Some("0.25 g")
    );
    let body_runs = runs(body);
    assert!(
        body_runs.iter().any(|(text, _)| *text == "Body"),
        "{body_runs:?}"
    );
    assert!(
        body_runs.iter().all(|(text, _)| *text != "Example"),
        "{body_runs:?}"
    );
}

#[test]
fn title_with_comma_in_braces_is_one_title() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[title={a, b}]Hi\\end{tcolorbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 2, "{:?}", texts(&p));
    // The comma stays inside the one title: no warning, and both words land
    // in the bar rather than becoming stray keys.
    let heading = runs(found[0]);
    assert!(heading.iter().any(|(text, _)| *text == "a,"), "{heading:?}");
    assert!(heading.iter().any(|(text, _)| *text == "b"), "{heading:?}");
}

#[test]
fn title_bar_follows_colframe() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[colframe=blue,title=T]Hi\\end{tcolorbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 2, "{:?}", texts(&p));
    // The bar uses the chosen frame colour, like the body's frame.
    assert_eq!(found[0].fill.fill_operator(), "0 0 1 rg");
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0 0 1 rg")
    );
    assert_eq!(
        found[1].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0 0 1 rg")
    );
}

#[test]
fn empty_title_shows_no_bar_and_no_warning() {
    for options in ["title=", "title={}"] {
        let p = doc(
            "\\usepackage{tcolorbox}",
            &format!("\\begin{{tcolorbox}}[{options}]Hi\\end{{tcolorbox}}"),
        );
        assert!(p.diagnostics.is_empty(), "{options}: {:?}", messages(&p));
        assert_eq!(boxes(&p).len(), 1, "{options}: {:?}", texts(&p));
    }
}

#[test]
fn title_markup_is_parsed_not_literal() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[title=Hi \\textit{there}]Hi\\end{tcolorbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 2, "{:?}", texts(&p));
    let heading = runs(found[0]);
    // The command runs through the ordinary dispatch: `there` keeps its
    // italic face (and the white title colour), and no literal backslash
    // text leaks into the bar.
    assert!(
        heading
            .iter()
            .any(|(text, style)| *text == "there" && style.italic),
        "{heading:?}"
    );
    assert!(
        heading.iter().all(|(text, _)| !text.contains('\\')),
        "{heading:?}"
    );
}

#[test]
fn bare_title_key_warns() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}[title]Hi\\end{tcolorbox}",
    );
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    let warned: Vec<_> = messages(&p)
        .into_iter()
        .filter(|m| m.contains("tcolorbox keys"))
        .collect();
    assert_eq!(warned.len(), 1, "{:?}", messages(&p));
    assert!(warned[0].contains("title"), "{}", warned[0]);
}

#[test]
fn use_without_the_package_warns_but_still_renders() {
    let p = doc("", "\\begin{tcolorbox}[colback=red]Hi\\end{tcolorbox}");
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    assert_eq!(found[0].fill.fill_operator(), "1 0 0 rg");
    assert!(
        messages(&p)
            .iter()
            .any(|m| m.contains("\\begin{tcolorbox} needs \\usepackage{tcolorbox}")),
        "{:?}",
        messages(&p)
    );
}

#[test]
fn box_is_its_own_paragraph_with_single_line_content() {
    let p = doc(
        "\\usepackage{tcolorbox}",
        "Before.\n\\begin{tcolorbox}[colback=red]Box words here\\end{tcolorbox}\nAfter.",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    // The display shape: surrounding paragraphs close before and open after.
    let paragraphs: Vec<_> = p
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(content) => Some(content),
            _ => None,
        })
        .collect();
    assert_eq!(paragraphs.len(), 3, "{:?}", texts(&p));
    assert_eq!(boxes(&p).len(), 1);
    // A blank line inside the body joins the paragraphs into the one box
    // rather than breaking it (like `\fbox` forbids `\par` outright): the
    // words survive, but no paragraph boundary does — beyond one line the
    // box overflows instead of re-flowing (shape (b), documented).
    let q = doc(
        "\\usepackage{tcolorbox}",
        "\\begin{tcolorbox}one\n\ntwo\\end{tcolorbox}",
    );
    assert!(q.diagnostics.is_empty(), "{:?}", messages(&q));
    assert_eq!(boxes(&q).len(), 1, "{:?}", texts(&q));
    let words = texts(&q);
    assert!(words.contains(&"one".to_string()), "{words:?}");
    assert!(words.contains(&"two".to_string()), "{words:?}");
}

#[test]
fn package_load_is_silent_bare_but_warns_for_libraries() {
    let bare = doc("\\usepackage{tcolorbox}", "x");
    assert!(
        bare.diagnostics
            .iter()
            .all(|d| !d.message.contains("recognised but not implemented")),
        "{:?}",
        messages(&bare)
    );
    let libs = doc("\\usepackage[most]{tcolorbox}", "x");
    assert!(
        messages(&libs)
            .iter()
            .any(|m| m.contains("tcolorbox") && m.contains("recognised but not implemented")),
        "{:?}",
        messages(&libs)
    );
}
