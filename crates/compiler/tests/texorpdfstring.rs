//! hyperref's `\texorpdfstring{TeX text}{bookmark string}` (GH-38 stress
//! sweep): the first argument is the document text in a section title, a
//! caption and body text, while the second feeds only the PDF
//! outline/bookmarks and never reaches the page.
//!
//! Ground truth measured with `pdflatex -interaction=nonstopmode`
//! (TeX Live 2026) on a minimal hyperref document: the heading and TOC
//! read "x2 and more" (with x2 as math), the body reads
//! "Body plain z here." (with z as math), the caption reads
//! "Figure 1: Cap y tail", and `Body\texorpdfstring{\LaTeX}{LaTeX}here`
//! sets with no gaps ("BodyLATEXhere."), with no error; under plain
//! article every use is "! Undefined control sequence."

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::parser::{parse, Block, Inline, ItemLabel};

/// The text runs of one inline list, joined the way the words read.
fn joined(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        if let Inline::Text {
            text, space_before, ..
        } = inline
        {
            if *space_before && !out.is_empty() {
                out.push(' ');
            }
            out.push_str(text);
        }
    }
    out
}

fn math_count(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .filter(|inline| matches!(inline, Inline::Math { .. }))
        .count()
}

fn has_text(inlines: &[Inline], needle: &str) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Text { text, .. } => text.contains(needle),
        _ => false,
    })
}

#[test]
fn texorpdfstring_typesets_only_its_first_argument() {
    let parsed = parse(concat!(
        "\\documentclass{article}\\usepackage{hyperref}\\begin{document}",
        "\\section{\\texorpdfstring{$x^2$}{x2} and more}",
        "Body \\texorpdfstring{plain $z$}{other} here.",
        "\\begin{figure}\\caption{\\texorpdfstring{Cap $y$}{Y} tail}\\end{figure}",
        "\\end{document}",
    ));
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    // The section title: the math of the first argument, then "and more";
    // the bookmark string "x2" never reaches the page.
    let heading = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Heading { content, .. } => Some(content),
            _ => None,
        })
        .expect("section heading");
    assert_eq!(math_count(heading), 1, "{heading:#?}");
    assert_eq!(joined(heading), "and more", "{heading:#?}");
    assert!(!has_text(heading, "x2"), "{heading:#?}");
    // The body paragraph: the first argument's words and math, with the
    // command-site space kept (`joined` reads "Body plain here." because it
    // skips math; pdflatex sets "Body plain z here."); the bookmark string
    // "other" never reaches the page.
    let body = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) if has_text(inlines, "Body") => Some(inlines),
            _ => None,
        })
        .expect("body paragraph");
    assert_eq!(math_count(body), 1, "{body:#?}");
    assert_eq!(joined(body), "Body plain here.", "{body:#?}");
    assert!(!has_text(body, "other"), "{body:#?}");
    // The figure caption: the label, then the first argument's words and
    // math; the bookmark string "Y" never reaches the page.
    let caption = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::FigureCaption { content } => Some(content),
            _ => None,
        })
        .expect("figure caption");
    assert_eq!(math_count(caption), 1, "{caption:#?}");
    assert_eq!(joined(caption), "Figure 1: Cap tail", "{caption:#?}");
    assert!(!has_text(caption, "Y"), "{caption:#?}");
}

/// A first argument starting with a non-text inline (the canonical
/// `\texorpdfstring{\LaTeX}{LaTeX}`) splices with the command site's
/// spacing: with no source spaces around it, pdflatex sets no gaps
/// ("BodyLATEXhere."), so the spliced leading inline must carry
/// `space_before = false`, not the `true` a fresh `box_inlines` group
/// starts with. Covers both splice sites (body dispatch and the flattened
/// heading pass).
#[test]
fn texorpdfstring_logo_first_arg_keeps_command_site_spacing() {
    let parsed = parse(concat!(
        "\\documentclass{article}\\usepackage{hyperref}\\begin{document}",
        "\\section{A\\texorpdfstring{\\LaTeX}{LaTeX}B}",
        "Body\\texorpdfstring{\\LaTeX}{LaTeX}here.",
        "\\end{document}",
    ));
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let space_flags = |inlines: &[Inline]| {
        inlines
            .iter()
            .map(|inline| match inline {
                Inline::Text { space_before, .. }
                | Inline::Math { space_before, .. }
                | Inline::Logo { space_before, .. } => Some(*space_before),
                Inline::ColorBox(b) => Some(b.space_before),
                Inline::Underline(u) => Some(u.space_before),
                Inline::HBox(b) => Some(b.space_before),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let body = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) if has_text(inlines, "Body") => Some(inlines),
            _ => None,
        })
        .expect("body paragraph");
    assert_eq!(
        body.iter()
            .filter(|i| matches!(i, Inline::Logo { .. }))
            .count(),
        1,
        "{body:#?}"
    );
    assert!(!has_text(body, "LaTeX"), "{body:#?}");
    // All three run glued, as pdflatex sets "BodyLATEXhere.": the spliced
    // logo and the trailing "here." carry the command site's false.
    assert_eq!(
        space_flags(body),
        vec![Some(false), Some(false), Some(false)],
        "{body:#?}"
    );
    let heading = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Heading { content, .. } => Some(content),
            _ => None,
        })
        .expect("section heading");
    assert_eq!(joined(heading), "AB", "{heading:#?}");
    // "A" opens the flattened argument (`space_before` reads true at index
    // 0); the spliced logo and the glued "B" carry the command site's
    // false — the bug left the logo true, inventing a gap.
    assert_eq!(
        space_flags(heading),
        vec![Some(true), Some(false), Some(false)],
        "{heading:#?}"
    );
}

/// A first argument starting with an image (`Inline::Graphic`) or a
/// graphics transform (`Inline::Transform`, e.g. `\scalebox`) splices with
/// the command site's spacing too: with no source spaces around it, no
/// interword glue is invented where the source has none. Covers both splice
/// sites (body dispatch and the flattened heading pass).
#[test]
fn texorpdfstring_graphic_first_arg_keeps_command_site_spacing() {
    let parsed = parse(concat!(
        "\\documentclass{article}\\usepackage{hyperref}\\begin{document}",
        "\\section{A\\texorpdfstring{\\includegraphics{foo}}{bar}B}",
        "Body\\texorpdfstring{\\includegraphics{foo}}{bar}here.\n\n",
        "T\\texorpdfstring{\\scalebox{2}{w}}{s}ail.",
        "\\end{document}",
    ));
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let space_flags = |inlines: &[Inline]| {
        inlines
            .iter()
            .map(|inline| match inline {
                Inline::Text { space_before, .. } => Some(*space_before),
                Inline::Graphic(b) => Some(b.space_before),
                Inline::Transform(b) => Some(b.space_before),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let paragraphs: Vec<&Vec<Inline>> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .collect();
    // All three run glued: the spliced image and the trailing "here."
    // carry the command site's false — the bug left the image true.
    let body = paragraphs
        .iter()
        .find(|inlines| has_text(inlines, "Body"))
        .expect("body paragraph");
    assert_eq!(
        body.iter()
            .filter(|i| matches!(i, Inline::Graphic(_)))
            .count(),
        1,
        "{body:#?}"
    );
    assert!(!has_text(body, "bar"), "{body:#?}");
    assert_eq!(joined(body), "Bodyhere.", "{body:#?}");
    assert_eq!(
        space_flags(body),
        vec![Some(false), Some(false), Some(false)],
        "{body:#?}"
    );
    // Same splice through a transform box.
    let scaled = paragraphs
        .iter()
        .find(|inlines| has_text(inlines, "ail"))
        .expect("scaled paragraph");
    assert_eq!(
        scaled
            .iter()
            .filter(|i| matches!(i, Inline::Transform(_)))
            .count(),
        1,
        "{scaled:#?}"
    );
    assert_eq!(joined(scaled), "Tail.", "{scaled:#?}");
    // The paragraph-initial "T" reflects the blank line above it (line-start
    // glue is discarded, so the flag is harmless there); the spliced
    // transform and the trailing "ail." carry the command site's false.
    assert_eq!(
        space_flags(scaled),
        vec![Some(true), Some(false), Some(false)],
        "{scaled:#?}"
    );
    let heading = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Heading { content, .. } => Some(content),
            _ => None,
        })
        .expect("section heading");
    assert_eq!(joined(heading), "AB", "{heading:#?}");
    // "A" opens the flattened argument (`space_before` reads true at index
    // 0); the spliced image and the glued "B" carry the command site's
    // false.
    assert_eq!(
        space_flags(heading),
        vec![Some(true), Some(false), Some(false)],
        "{heading:#?}"
    );
}

/// Without hyperref the command does not exist inside an `\item` label
/// either: the label pass never silently swallows, so it reports exactly as
/// it did before the flattened heading/caption arm existed.
#[test]
fn texorpdfstring_without_hyperref_in_item_label_is_reported() {
    let parsed = parse(concat!(
        "\\documentclass{article}\\begin{document}",
        "\\begin{itemize}\\item[\\texorpdfstring{A}{B} tail] body\\end{itemize}",
        "\\end{document}",
    ));
    assert_eq!(
        parsed.diagnostics.len(),
        1,
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let diagnostic = &parsed.diagnostics[0];
    assert!(
        diagnostic.message.contains("\\texorpdfstring"),
        "{}",
        diagnostic.message
    );
}

/// With hyperref an `\item` label splices the first argument like a heading
/// does, with no diagnostic; the bookmark string never reaches the page.
#[test]
fn texorpdfstring_with_hyperref_in_item_label_splices_first_arg() {
    let parsed = parse(concat!(
        "\\documentclass{article}\\usepackage{hyperref}\\begin{document}",
        "\\begin{itemize}\\item[\\texorpdfstring{A}{B} tail] body\\end{itemize}",
        "\\end{document}",
    ));
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let label = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::ListItem { item, .. } => Some(item),
            _ => None,
        })
        .expect("list item");
    let content = match label {
        Some(ItemLabel::Explicit { content, .. }) => content,
        other => panic!("expected an explicit label, got {other:?}"),
    };
    assert!(has_text(content, "A"), "{content:#?}");
    assert!(has_text(content, "tail"), "{content:#?}");
    assert!(!has_text(content, "B"), "{content:#?}");
}

#[test]
fn texorpdfstring_without_hyperref_is_rejected_like_pdflatex() {
    // pdflatex under plain article: "! Undefined control sequence." This
    // compiler's equivalent is the unknown-command error naming it.
    let parsed = parse(concat!(
        "\\documentclass{article}\\begin{document}",
        "Body \\texorpdfstring{plain}{other} here.",
        "\\end{document}",
    ));
    assert_eq!(
        parsed.diagnostics.len(),
        1,
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let diagnostic = &parsed.diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(
        diagnostic.message.contains("\\texorpdfstring"),
        "{}",
        diagnostic.message
    );
}
