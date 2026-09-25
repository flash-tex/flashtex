//! Slice 1 of the `tcolorbox` lane: `\begin{tcolorbox}[colback=..,colframe=..]`.
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
//! Shape (b) by design: the content is `box_inlines`' flattened single-line
//! run on its own paragraph, so a body longer than one line overflows rather
//! than re-flowing (the block-level follow-up). `title`, rounded corners,
//! the `left/right/top/bottom` extras, the full-`\linewidth` width and every
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
        "\\begin{tcolorbox}[colback=red,title={a, b},sharp corners]Hi\\end{tcolorbox}",
    );
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    // The honoured key still applies; the box is rendered, not dropped.
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0.25 g")
    );
    assert_eq!(found[0].fill.fill_operator(), "1 0 0 rg");
    // One diagnostic names both ignored keys; `title={a, b}` stays one key
    // (the comma inside braces must not split the option list), and nothing
    // is silently dropped.
    let warned: Vec<_> = messages(&p)
        .into_iter()
        .filter(|m| m.contains("tcolorbox keys"))
        .collect();
    assert_eq!(warned.len(), 1, "{:?}", messages(&p));
    assert!(warned[0].contains("title"), "{}", warned[0]);
    assert!(warned[0].contains("sharp corners"), "{}", warned[0]);
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
fn newtcolorbox_zero_args_renders_white_framed_box() {
    // The wave-builder wb-1 repro: `\newtcolorbox{mybox}{colback=white}` in
    // the preamble, `\begin{mybox}body text\end{mybox}` in the body.
    // pdflatex (TeX Live 2026, article 10pt letter, `probe.tex`) reports 0
    // errors and paints a white (`1 g`) fill inside the default
    // `black!75!white` (`0.25 g`) frame, with the body starting at x=149.36.
    let p = doc(
        "\\usepackage{tcolorbox}\n\\newtcolorbox{mybox}{colback=white}",
        "\\begin{mybox}body text\\end{mybox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    assert_eq!(found[0].fill.fill_operator(), "1 g");
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0.25 g")
    );
    assert_eq!(found[0].fboxrule_pt, BOXRULE_PT);
    assert_eq!(found[0].fboxsep_pt, BOXSEP_PT);
    let words = texts(&p);
    assert!(words.contains(&"body".to_string()), "{words:?}");
    assert!(words.contains(&"text".to_string()), "{words:?}");
}

#[test]
fn newtcolorbox_with_argument_substitutes_into_options() {
    // pdflatex `args.tex`: `\newtcolorbox{cbox}[1]{colback=#1}` with
    // `\begin{cbox}{red}` paints an RGB `1 0 0` fill inside the default
    // frame, with zero errors.
    let p = doc(
        "\\usepackage{tcolorbox}\n\\newtcolorbox{cbox}[1]{colback=#1}",
        "\\begin{cbox}{red}reddish\\end{cbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    assert_eq!(found[0].fill.fill_operator(), "1 0 0 rg");
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0.25 g")
    );
    assert!(texts(&p).contains(&"reddish".to_string()));
}

#[test]
fn newtcolorbox_optional_first_argument_uses_default_or_given() {
    // pdflatex `args.tex`: `\newtcolorbox{dbox}[2][yellow]{colback=#1,colframe=#2}`.
    // `\begin{dbox}{blue}` leaves `#1` at its default, so the box fills
    // default yellow (`0 0 1 0 k`) inside a blue (`0 0 1 rg`) frame;
    // `\begin{dbox}[green]{black}` fills green (`0 1 0 rg`) inside black
    // (`0 g`). (Order verified in the content stream: tcolorbox paints the
    // frame path first, then the fill.)
    let p = doc(
        "\\usepackage{tcolorbox}\n\\newtcolorbox{dbox}[2][yellow]{colback=#1,colframe=#2}",
        "\\begin{dbox}{blue}default frame\\end{dbox}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    assert_eq!(found[0].fill.fill_operator(), "0 0 1 0 k");
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0 0 1 rg")
    );
    let q = doc(
        "\\usepackage{tcolorbox}\n\\newtcolorbox{dbox}[2][yellow]{colback=#1,colframe=#2}",
        "\\begin{dbox}[green]{black}extra opt\\end{dbox}",
    );
    assert!(q.diagnostics.is_empty(), "{:?}", messages(&q));
    let found = boxes(&q);
    assert_eq!(found.len(), 1, "{:?}", texts(&q));
    assert_eq!(found[0].fill.fill_operator(), "0 1 0 rg");
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("0 g")
    );
}

#[test]
fn newtcolorbox_without_the_package_is_rejected_like_pdflatex() {
    // pdflatex `errs.tex` (plain article, no tcolorbox): `! Undefined
    // control sequence.` for `\newtcolorbox`, then `Environment mybox
    // undefined`. FlashTeX must diagnose both and render no box.
    let p = doc(
        "",
        "\\begin{mybox}body text\\end{mybox}",
    );
    assert!(
        boxes(&p).is_empty(),
        "no box without a definition: {:?}",
        texts(&p)
    );
    let q = doc(
        "\\newtcolorbox{mybox}{colback=white}",
        "\\begin{mybox}body text\\end{mybox}",
    );
    assert!(
        messages(&q)
            .iter()
            .any(|m| m.contains("\\newtcolorbox needs \\usepackage{tcolorbox}")),
        "{:?}",
        messages(&q)
    );
    assert!(boxes(&q).is_empty(), "{:?}", texts(&q));
}

#[test]
fn newtcolorbox_redefinition_rules_match_pdflatex() {
    // pdflatex `redef.tex`: a second `\newtcolorbox{mybox}` reports `!
    // LaTeX Error: Command \mybox already defined.`, `\renewtcolorbox` of
    // an unknown name reports `! LaTeX Error: Environment nobox
    // undefined.`, and a valid `\renewtcolorbox` wins (final fill red).
    let p = doc(
        "\\usepackage{tcolorbox}\n\\newtcolorbox{mybox}{colback=white}\n\\newtcolorbox{mybox}{colback=red}",
        "\\begin{mybox}body text\\end{mybox}",
    );
    assert!(
        messages(&p)
            .iter()
            .any(|m| m.contains("environment 'mybox' is already defined")),
        "{:?}",
        messages(&p)
    );
    // The failed redefinition keeps the first options: still white.
    assert_eq!(boxes(&p)[0].fill.fill_operator(), "1 g");
    let q = doc(
        "\\usepackage{tcolorbox}\n\\renewtcolorbox{nobox}{colback=red}",
        "plain",
    );
    assert!(
        messages(&q)
            .iter()
            .any(|m| m.contains("environment 'nobox' is undefined")),
        "{:?}",
        messages(&q)
    );
    let r = doc(
        "\\usepackage{tcolorbox}\n\\newtcolorbox{mybox}{colback=white}\n\\renewtcolorbox{mybox}{colback=red}",
        "\\begin{mybox}body text\\end{mybox}",
    );
    assert!(r.diagnostics.is_empty(), "{:?}", messages(&r));
    assert_eq!(boxes(&r)[0].fill.fill_operator(), "1 0 0 rg");
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
