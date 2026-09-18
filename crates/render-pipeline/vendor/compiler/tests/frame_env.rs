//! The bordered-box `frame` environment: an `\fbox` in environment form.
//! The body is one `Inline::ColorBox` with an `\fboxrule` frame, sized to
//! its content, and the environment no longer reports `unsupported_feature`.

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

#[test]
fn frame_boxes_its_body_with_an_fboxrule_frame() {
    let p = doc("", "Before \\begin{frame}Hello \\textbf{world}\\end{frame} after.");
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1, "{:?}", texts(&p));
    let framed = found[0];
    // `\fboxsep` padding and an `\fboxrule` frame, like `\fbox`.
    assert_eq!(framed.fboxsep_pt, 3.0);
    assert_eq!(framed.fboxrule_pt, 0.4);
    // `\fbox` draws its rule in the current colour: black here.
    assert_eq!(
        framed.frame.map(|c| c.fill_operator()).as_deref(),
        Some("0 g")
    );
    // `\fbox` has no fill of its own: white on a default page.
    assert_eq!(framed.fill.fill_operator(), "1 g");
    // The body's normal typesetting survives inside the box.
    let words = texts(&p);
    assert!(words.contains(&"Hello".to_string()), "{words:?}");
    assert!(words.contains(&"world".to_string()), "{words:?}");
    assert!(words.contains(&"Before".to_string()), "{words:?}");
    assert!(words.contains(&"after.".to_string()), "{words:?}");
}

#[test]
fn frame_uses_the_current_colour_and_page_colour() {
    let p = doc(
        "\\usepackage{xcolor}\\pagecolor{yellow!20}",
        "{\\color{red}\\begin{frame}x\\end{frame}}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[0].frame.map(|c| c.fill_operator()).as_deref(),
        Some("1 0 0 rg")
    );
    // No fill of its own: the box takes the page colour.
    assert_eq!(found[0].fill, p.page_color.expect("page colour parsed"));
}

#[test]
fn frame_honours_fboxsep_and_fboxrule() {
    let p = doc(
        "",
        "\\setlength{\\fboxsep}{5pt}\\setlength{\\fboxrule}{2pt}\\begin{frame}x\\end{frame}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fboxsep_pt, 5.0);
    assert_eq!(found[0].fboxrule_pt, 2.0);
}

#[test]
fn nested_frames_nest_the_boxes() {
    let p = doc("", "\\begin{frame}a \\begin{frame}b\\end{frame} c\\end{frame}");
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let found = boxes(&p);
    assert_eq!(found.len(), 2);
    assert_eq!(found[1].content.len(), 1);
}

#[test]
fn unterminated_frame_is_diagnosed_and_boxes_the_rest() {
    let p = doc("", "\\begin{frame}abc");
    let m = messages(&p);
    assert!(
        m.iter().any(|m| m.contains("unterminated environment 'frame'")),
        "{m:?}"
    );
    assert_eq!(boxes(&p).len(), 1);
    assert!(texts(&p).contains(&"abc".to_string()));
}

#[test]
fn boxes_keep_the_one_leading_per_block_invariant() {
    // `box_inlines` parses into local blocks: the box's paragraphs must not
    // leak entries into the per-block `block_par_leading` vector.
    for body in [
        "\\colorbox{yellow}{M}",
        "\\fcolorbox{red}{blue}{N}",
        "\\begin{frame}Hi \\textbf{Yo}\\end{frame}",
        "A \\begin{frame}x\\end{frame} B",
    ] {
        let p = doc("\\usepackage{xcolor}", body);
        assert_eq!(
            p.block_par_leading.len(),
            p.blocks.len(),
            "{body:?}: {} leadings vs {} blocks",
            p.block_par_leading.len(),
            p.blocks.len()
        );
    }
}

#[test]
fn frame_never_reports_unsupported_feature() {
    for body in [
        "\\begin{frame}a\\end{frame}",
        "\\begin{frame}a\n\nb\\end{frame}",
        "\\begin{frame}$x^2$ \\textcolor{red}{c}\\end{frame}",
    ] {
        let p = doc("\\usepackage{xcolor}", body);
        let m = messages(&p);
        assert!(
            m.iter()
                .all(|m| !m.contains("not implemented") && !m.contains("not supported")),
            "{body:?}: {m:?}"
        );
        assert!(!boxes(&p).is_empty(), "{body:?}: no box");
    }
}
