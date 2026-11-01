//! Beamer Tier 3 (issue #944), the parse side: what the parser hands the
//! render pipeline for `block`/`alertblock`/`exampleblock`, `columns` with
//! `\column`/`column`, and beamer's in-flow `figure`/`table` with
//! `\caption`. The geometry (block skips, the paper-wide column row, the
//! caption line) is the render pipeline's, tested in
//! `crates/render-pipeline/tests/beamer_blocks.rs` against pdflatex.
//!
//! Oracle for the shapes asserted here: `beamerbaselocalstructure.sty`
//! 107-134 (blocks) and 550-601 (floats, `\abovecaptionskip` =
//! `\belowcaptionskip` = 7pt), `beamerbaseframecomponents.sty` 204-291
//! (columns keys, `\column`).

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{
    beamer_columns_options, parse, BeamerBlockKind, BeamerColumnAlign, BeamerFloatKind, Block,
    Inline, ParagraphStyle,
};

fn deck(body: &str) -> String {
    format!("\\documentclass{{beamer}}\n\\begin{{document}}\n\\begin{{frame}}\n{body}\n\\end{{frame}}\n\\end{{document}}\n")
}

fn blocks(text: &str) -> Vec<Block> {
    parse(text).blocks
}

fn messages(text: &str) -> Vec<String> {
    parse(text).diagnostics.iter().map(|d| d.message.clone()).collect()
}

fn compiled(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn text_of(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn shape(b: &[Block]) -> Vec<&'static str> {
    b.iter()
        .map(|b| match b {
            Block::BeamerFrameBegin { .. } => "frame",
            Block::BeamerFrameEnd { .. } => "/frame",
            Block::BeamerBlockBegin { .. } => "block",
            Block::BeamerBlockEnd { .. } => "/block",
            Block::BeamerColumnsBegin { .. } => "columns",
            Block::BeamerColumn { .. } => "column",
            Block::BeamerColumnsEnd { .. } => "/columns",
            Block::BeamerCaption { .. } => "caption",
            Block::VSpace { .. } => "vspace",
            Block::Paragraph(_) => "para",
            Block::Styled { .. } => "styled",
            Block::ListItem { .. } => "item",
            _ => "other",
        })
        .collect()
}

#[test]
fn the_three_block_kinds_bracket_their_bodies() {
    let b = blocks(&deck(concat!(
        "\\begin{block}{Definition}\nA frame.\n\\end{block}\n",
        "\\begin{alertblock}{Caution}\nCounters differ.\n\\end{alertblock}\n",
        "\\begin{exampleblock}{Example}\nThree slides.\n\\end{exampleblock}",
    )));
    assert_eq!(
        shape(&b),
        ["frame", "block", "para", "/block", "block", "para", "/block", "block", "para", "/block", "/frame"]
    );
    let heads: Vec<(BeamerBlockKind, String)> = b
        .iter()
        .filter_map(|b| match b {
            Block::BeamerBlockBegin { kind, title, .. } => Some((*kind, text_of(title))),
            _ => None,
        })
        .collect();
    assert_eq!(
        heads,
        [
            (BeamerBlockKind::Plain, "Definition".to_string()),
            (BeamerBlockKind::Alert, "Caution".to_string()),
            (BeamerBlockKind::Example, "Example".to_string())
        ]
    );
    // The body text is the block's, not the title's.
    assert!(matches!(&b[2], Block::Paragraph(p) if text_of(p) == "A frame."));
}

#[test]
fn a_block_overlay_spec_is_read_past_and_lists_nest_inside() {
    let b = blocks(&deck("\\begin{block}<2->{T}\n\\begin{itemize}\n\\item one\n\\item two\n\\end{itemize}\n\\end{block}"));
    assert_eq!(shape(&b), ["frame", "block", "item", "item", "/block", "/frame"]);
    let src = deck("\\begin{block}<2->{T}\nx\n\\end{block}");
    assert!(messages(&src).iter().all(|m| !m.contains("block")), "{:?}", messages(&src));
}

#[test]
fn columns_row_with_command_and_environment_columns() {
    let b = blocks(&deck(concat!(
        "\\begin{columns}[T]\n",
        "\\column{.5\\textwidth}\nLeft.\n",
        "\\begin{column}[b]{4cm}\nRight.\n\\end{column}\n",
        "\\end{columns}",
    )));
    assert_eq!(shape(&b), ["frame", "columns", "column", "para", "column", "para", "/columns", "/frame"]);
    let Block::BeamerColumnsBegin { options, .. } = &b[1] else { panic!("{:?}", b[1]) };
    assert_eq!(options.align, BeamerColumnAlign::TopBaseline);
    assert!(!options.only_text_width && options.total_width.is_none());
    let cols: Vec<(String, Option<BeamerColumnAlign>)> = b
        .iter()
        .filter_map(|b| match b {
            Block::BeamerColumn { width, align, .. } => Some((width.clone(), *align)),
            _ => None,
        })
        .collect();
    assert_eq!(cols, [(".5\\textwidth".to_string(), None), ("4cm".to_string(), Some(BeamerColumnAlign::Bottom))]);
    // `\textwidth` inside the width argument is not an unsupported command.
    let src = deck("\\begin{columns}\\column{.5\\textwidth}x\\end{columns}");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

#[test]
fn columns_options_keys() {
    let (o, a) = beamer_columns_options("onlytextwidth,t");
    assert!(o.only_text_width && o.align == BeamerColumnAlign::Top && a == Some(BeamerColumnAlign::Top));
    let (o, a) = beamer_columns_options("totalwidth=\\textwidth");
    assert_eq!(o.total_width.as_deref(), Some("\\textwidth"));
    assert!(a.is_none() && o.align == BeamerColumnAlign::Center);
    let (o, _) = beamer_columns_options("c, height=3cm");
    assert_eq!(o.align, BeamerColumnAlign::Center);
}

#[test]
fn column_outside_columns_is_diagnosed_not_marked() {
    let src = deck("\\column{.5\\textwidth}\nText.");
    let b = blocks(&src);
    assert_eq!(shape(&b), ["frame", "para", "/frame"]);
    assert!(messages(&src).iter().any(|m| m.contains("\\column outside a columns environment")), "{:?}", messages(&src));
}

#[test]
fn beamer_table_is_a_centred_box_with_an_unnumbered_small_caption() {
    let b = blocks(&deck(concat!(
        "\\begin{table}\n\\centering\n\\begin{tabular}{lr}\na & 1 \\\\\n\\end{tabular}\n",
        "\\caption{Time per stage.}\n\\label{tab:t}\n\\end{table}\nSee Table~\\ref{tab:t}.",
    )));
    assert_eq!(shape(&b), ["frame", "styled", "vspace", "caption", "vspace", "para", "/frame"]);
    assert!(matches!(&b[1], Block::Styled { style: ParagraphStyle::Center, .. }));
    assert!(matches!(&b[2], Block::VSpace { pt, .. } if *pt == 7.0));
    let Block::BeamerCaption { kind, content, .. } = &b[3] else { panic!("{:?}", b[3]) };
    assert_eq!(*kind, BeamerFloatKind::Table);
    assert_eq!(text_of(content), "Time per stage.");
    assert!(content.iter().all(|i| !matches!(i, Inline::Text { style, .. } if style.size.is_none())), "the caption is \\small");
    // `\refstepcounter` still numbers the table for `\ref`.
    let out = compiled(&deck("\\begin{table}\n\\caption{c}\n\\label{tab:t}\n\\end{table}\nSee \\ref{tab:t}."));
    let text: String = out.pages.iter().flat_map(|p| p.items.iter().map(|i| i.text.as_str())).collect::<Vec<_>>().join(" ");
    assert!(text.contains("See 1"), "{text}");
}

#[test]
fn beamer_figure_takes_the_same_path() {
    let b = blocks(&deck("\\begin{figure}\n\\caption{A picture.}\n\\end{figure}"));
    assert_eq!(shape(&b), ["frame", "vspace", "caption", "vspace", "/frame"]);
    assert!(matches!(&b[2], Block::BeamerCaption { kind: BeamerFloatKind::Figure, .. }));
    let src = deck("\\begin{figure}\n\\caption{A picture.}\n\\end{figure}");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}

#[test]
fn outside_beamer_the_block_environments_name_the_class() {
    let src = "\\documentclass{article}\n\\begin{document}\n\\begin{block}{T}\nx\n\\end{block}\n\\end{document}\n";
    let m = messages(src);
    assert!(m.iter().any(|m| m.contains("environment 'block' is defined by the beamer document class; this document is article")), "{m:?}");
    assert!(!m.iter().any(|m| m.contains("not implemented")), "{m:?}");
    // article's `table` keeps its numbered caption.
    let src = "\\documentclass{article}\n\\begin{document}\n\\begin{table}\n\\caption{c}\n\\end{table}\n\\end{document}\n";
    let b = blocks(src);
    assert!(b.iter().any(|b| matches!(b, Block::FigureCaption { content } if text_of(content).starts_with("Table 1:"))), "{b:?}");
}

/// An environment opening directly after `\begin{frame}` (or after a real
/// `{title}`) is the body, not a title: the expansion pass emits the
/// environment's group brace ahead of every `\begin{...}` (#950), and the
/// head's optional `{title}{subtitle}` scan must not read that brace as a
/// literal one. Likewise `\onslide<2->` directly before an environment
/// stays the bare running-marker form.
#[test]
fn environment_group_is_not_an_optional_title_argument() {
    let src = deck("\\begin{itemize}\n\\item A\n\\end{itemize}");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
    let b = blocks(&src);
    let Block::BeamerFrameBegin { title, subtitle, .. } = &b[0] else { panic!("{:?}", b[0]) };
    assert!(title.is_empty() && subtitle.is_empty(), "{title:?} {subtitle:?}");
    assert_eq!(shape(&b), ["frame", "item", "/frame"]);

    // A real title, then the environment as the body.
    let src = "\\documentclass{beamer}\n\\begin{document}\n\\begin{frame}{Title}\\begin{block}{B}\nx\n\\end{block}\n\\end{frame}\n\\end{document}\n";
    assert!(messages(src).is_empty(), "{:?}", messages(src));
    let b = blocks(src);
    let Block::BeamerFrameBegin { title, subtitle, .. } = &b[0] else { panic!("{:?}", b[0]) };
    assert_eq!(text_of(title), "Title");
    assert!(subtitle.is_empty(), "{subtitle:?}");
    assert_eq!(shape(&b), ["frame", "block", "para", "/block", "/frame"]);

    // `\onslide<2->` ahead of an environment: the marker form, no
    // "missing closing brace" diagnostic.
    let src = deck("\\onslide<2->\\begin{itemize}\n\\item A\n\\end{itemize}");
    assert!(messages(&src).is_empty(), "{:?}", messages(&src));
}
