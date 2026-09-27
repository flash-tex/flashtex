//! `minipage` (latex.ltx `\minipage`, `\@iminipage`, `\@iiminipage`,
//! `\@iiiminipage`) and vertical glue of infinite stretch.
//!
//! The parser reads the environment's `[pos][height][inner-pos]{width}`
//! with latex.ltx's defaults (`\begin{minipage}{w}` is `[c][\relax][s]`, a
//! lone `[p]` is `[p][\relax][s]`, `[p][h]` is `[p][h][p]`), emits the body
//! as the blocks between a `MinipageBegin` and a `MinipageEnd` and puts one
//! `Inline::Minipage` box in the paragraph the environment's `\leavevmode`
//! started or joined; the surrounding paragraph (its `\noindent`, a pending
//! `\item`, a `center`) resumes after `\end{minipage}`. `\vfill`, `\vfil`,
//! `\vss`, `\vspace{\fill}`, `\vspace{\stretch{n}}` and a `\vskip` with an
//! infinite `plus` report their glue order and amount. The render
//! pipeline's `tests/minipage_vfill.rs` measures the laid-out result
//! against pdflatex.

use flashtex_compiler::parser::{parse, Block, Inline, MinipageInner, MinipagePosition};
use flashtex_compiler::text_builtins::{DimenUnit, PhysicalUnit};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn boxes(blocks: &[Block]) -> Vec<&flashtex_compiler::parser::Minipage> {
    blocks
        .iter()
        .flat_map(|b| match b {
            Block::Paragraph(c) | Block::Styled { content: c, .. } | Block::ListItem { content: c, .. } => c.as_slice(),
            _ => &[],
        })
        .filter_map(|i| match i {
            Inline::Minipage(m) => Some(m.as_ref()),
            _ => None,
        })
        .collect()
}

fn messages(parsed: &flashtex_compiler::parser::Parsed) -> Vec<String> {
    parsed.diagnostics.iter().map(|d| d.message.clone()).collect()
}

#[test]
fn the_arguments_take_latex_defaults() {
    let parsed = parse(&doc(
        "\\begin{minipage}{3cm}a\\end{minipage}\n\
         \\begin{minipage}[t]{0.5\\linewidth}b\\end{minipage}\n\
         \\begin{minipage}[b][2in]{\\textwidth}c\\end{minipage}\n\
         \\begin{minipage}[t][3.48in][s]{\\linewidth}d\\end{minipage}\n\
         \\begin{minipage}[z][1in][c]{10pt}e\\end{minipage}",
    ));
    let msgs = messages(&parsed);
    assert!(!msgs.iter().any(|m| m.contains("minipage") || m.contains("linewidth") || m.contains("textwidth")), "{msgs:?}");
    let b = boxes(&parsed.blocks);
    assert_eq!(b.len(), 5);
    let summary: Vec<_> = b.iter().map(|m| (m.position, m.inner, m.height.is_some())).collect();
    assert_eq!(
        summary,
        [
            (MinipagePosition::Center, MinipageInner::Stretch, false),
            (MinipagePosition::Top, MinipageInner::Stretch, false),
            // `[p][h]`: the inner position is `[p]`'s letter.
            (MinipagePosition::Bottom, MinipageInner::Bottom, true),
            (MinipagePosition::Top, MinipageInner::Stretch, true),
            // Any letter but `t`/`b` is `\vcenter`.
            (MinipagePosition::Center, MinipageInner::Center, true),
        ]
    );
    assert_eq!(b[0].width.unit, DimenUnit::Physical(PhysicalUnit::Cm));
    assert_eq!((b[1].width.integer, b[1].width.frac.as_slice(), b[1].width.unit), (0, &[5u8][..], DimenUnit::LineWidth));
    assert_eq!(b[2].width.unit, DimenUnit::TextWidth);
    assert_eq!(b[3].height.as_ref().map(|h| (h.integer, h.frac.clone(), h.unit)), Some((3, vec![4, 8], DimenUnit::Physical(PhysicalUnit::In))));
}

/// The inner position when a height is given, as latex.ltx's
/// `\@iiminipage#1[#2]` (the outer letter) and `\@iiiparbox`'s
/// `\csname bm@#3\endcsname` read it: `t`/`l` put the `\vss` below the
/// body, `b`/`r` above it, `c` on both sides, `s` nowhere; any other text
/// is `\bm@c` with LaTeX's "Unexpected alignment" warning -- which is what
/// an outer `[x]` with a height passes on. pdflatex (TeX Live 2026) warns
/// `Unexpected alignment x` for `[x][1in]` and for `[t][1in][x]`, and the
/// render pipeline's `minipage_vfill.rs` measures the resulting positions.
#[test]
fn the_inner_position_defaults_to_the_outer_one_and_falls_back_to_centre() {
    let parsed = parse(&doc(
        "\\begin{minipage}[t][1in]{50pt}a\\end{minipage}\n\
         \\begin{minipage}[b][1in]{50pt}b\\end{minipage}\n\
         \\begin{minipage}[c][1in]{50pt}c\\end{minipage}\n\
         \\begin{minipage}[x][1in]{50pt}d\\end{minipage}\n\
         \\begin{minipage}[s][1in]{50pt}e\\end{minipage}\n\
         \\begin{minipage}[t][1in][x]{50pt}f\\end{minipage}\n\
         \\begin{minipage}[t][1in][l]{50pt}g\\end{minipage}\n\
         \\begin{minipage}[t][1in][r]{50pt}h\\end{minipage}\n\
         \\begin{minipage}[x]{50pt}i\\end{minipage}",
    ));
    let b = boxes(&parsed.blocks);
    let summary: Vec<_> = b.iter().map(|m| (m.position, m.inner)).collect();
    use MinipageInner as I;
    use MinipagePosition as P;
    assert_eq!(
        summary,
        [
            (P::Top, I::Top),
            (P::Bottom, I::Bottom),
            (P::Center, I::Center),
            (P::Center, I::Center),
            (P::Center, I::Stretch),
            (P::Top, I::Center),
            (P::Top, I::Top),
            (P::Top, I::Bottom),
            // `\@iminipage[#1]` alone passes `[s]`: no warning.
            (P::Center, I::Stretch),
        ]
    );
    let warned: Vec<String> = messages(&parsed).into_iter().filter(|m| m.contains("Unexpected alignment")).collect();
    assert_eq!(warned.len(), 2, "{warned:?}");
    assert!(warned.iter().all(|m| m.contains("Unexpected alignment x")), "{warned:?}");
}

/// `mpfootnote` across a nested minipage, as pdflatex numbers it (TeX Live
/// 2026): `\@iiiminipage`'s `\c@mpfootnote\z@` is local to the box's group
/// and `\stepcounter` is global, so after a nested box that stepped the
/// counter the outer box continues from the inner value (`a b [a] b`), and
/// after one that stepped nothing the outer value comes back (`a b [] c`).
#[test]
fn a_nested_minipage_leaves_the_outer_footnote_count_as_latex_does() {
    let src = doc(
        "\\noindent\\begin{minipage}{3in}\nAa\\footnote{One.} Bb\\footnote{Two.}\n\\begin{minipage}{1in}Cc\\footnote{Three.}\\end{minipage}\nDd\\footnote{Four.}\n\\end{minipage}\n\n\
         \\noindent\\begin{minipage}{3in}\nEe\\footnote{Five.} Ff\\footnote{Six.}\n\\begin{minipage}{1in}Gg without notes.\\end{minipage}\nHh\\footnote{Seven.}\n\\end{minipage}",
    );
    let parsed = parse(&src);
    let mut marks: Vec<(usize, String)> = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(c) | Block::Styled { content: c, .. } | Block::ListItem { content: c, .. } = block {
            for i in c {
                if let Inline::Footnote { number, span, .. } = i {
                    marks.push((span.start, number.clone()));
                }
            }
        }
    }
    marks.sort();
    let marks: Vec<&str> = marks.iter().map(|(_, n)| n.as_str()).collect();
    assert_eq!(marks, ["a", "b", "a", "b", "a", "b", "c"]);
}

/// The body is the blocks between the markers; the paragraph holding the
/// box comes after them and keeps the `\noindent` and the words around the
/// box. The per-block records stay one per block.
#[test]
fn the_body_is_bracketed_and_the_paragraph_resumes() {
    let parsed = parse(&doc(
        "Before.\n\n\\noindent Lead \\begin{minipage}[t]{2in}First.\\par Second.\\vfill Third.\\end{minipage} trail.\n\nAfter.",
    ));
    assert_eq!(parsed.block_par_starts.len(), parsed.blocks.len());
    assert_eq!(parsed.block_par_leading.len(), parsed.blocks.len());
    let kinds: Vec<&str> = parsed
        .blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph(c) if c.iter().any(|i| matches!(i, Inline::Minipage(_))) => "row",
            Block::Paragraph(_) => "par",
            Block::MinipageBegin { .. } => "begin",
            Block::MinipageEnd { .. } => "end",
            Block::VFill { order: 2, stretch, .. } if *stretch == 1.0 => "vfill",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["par", "begin", "par", "par", "vfill", "par", "end", "row", "par"]);
    let row = parsed.blocks.iter().position(|b| matches!(b, Block::Paragraph(c) if c.iter().any(|i| matches!(i, Inline::Minipage(_))))).unwrap();
    assert!(!parsed.block_par_starts[row].indent, "the row keeps its \\noindent");
    let Block::Paragraph(content) = &parsed.blocks[row] else { unreachable!() };
    let texts: Vec<String> = content
        .iter()
        .map(|i| match i {
            Inline::Text { text, .. } => text.clone(),
            Inline::Minipage(_) => "[box]".to_string(),
            _ => String::new(),
        })
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(texts, ["Lead", "[box]", "trail."]);
    // The body's paragraphs are the box's own: none of them is indented
    // by the surrounding paragraph's `\noindent` state.
    let Some(Inline::Minipage(m)) = content.iter().find(|i| matches!(i, Inline::Minipage(_))) else { unreachable!() };
    assert!(m.end.start > m.span.end, "the box ends at \\end{{minipage}}");
}

/// A pending `\item` label waits for the paragraph that holds the box: the
/// body's paragraphs are not items, the row is.
#[test]
fn an_item_label_goes_to_the_row_not_the_body() {
    let parsed = parse(&doc("\\begin{itemize}\\item \\begin{minipage}[t]{2in}Body text.\\end{minipage}\n\\item Next.\\end{itemize}"));
    let body = parsed.blocks.iter().skip_while(|b| !matches!(b, Block::MinipageBegin { .. })).take_while(|b| !matches!(b, Block::MinipageEnd { .. }));
    assert!(body.clone().all(|b| !matches!(b, Block::ListItem { .. })), "{:?}", body.collect::<Vec<_>>());
    let items: Vec<bool> = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::ListItem { label, content, .. } => Some(label.is_some() && content.iter().any(|i| matches!(i, Inline::Minipage(_)))),
            _ => None,
        })
        .collect();
    assert_eq!(items, [true, false]);
}

/// Inside a table entry the body's blocks cannot be set: the body is kept
/// as the entry's text and the box is reported as such.
#[test]
fn a_minipage_in_a_table_entry_is_reported_and_set_as_text() {
    let parsed = parse(&doc("\\begin{tabular}{l}\\begin{minipage}{2in}Cell words.\\end{minipage}\\\\\nnext\n\\end{tabular}"));
    let msgs = messages(&parsed);
    assert!(msgs.iter().any(|m| m.contains("minipage inside a table entry")), "{msgs:?}");
    assert!(!msgs.iter().any(|m| m.contains("block-level content")), "{msgs:?}");
    assert!(format!("{:?}", parsed.blocks).contains("Cell"));
    assert!(!format!("{:?}", parsed.blocks).contains("Minipage("));
}

/// The glue orders: `\vfill` 1fill, `\vfil`/`\vss` 1fil, `\vspace{\fill}` and
/// `\vspace*{\fill}` 1fill, `\vspace{\stretch{2}}` 2fill, and glue with a
/// natural width and an infinite stretch (`\vskip 5pt plus 1filll`,
/// `\vspace{12pt plus 1fill}`) one node carrying both, as TeX keeps it.
#[test]
fn infinite_vertical_glue_keeps_its_order() {
    let parsed = parse(&doc(
        "A.\\par\\vfill\\vfil\\vss\\vspace{\\fill}\\vspace*{\\fill}\\vspace{\\stretch{2}}\\vskip 5pt plus 1filll minus 2pt\\relax\\vspace{12pt plus 1fill}\nB.",
    ));
    let msgs = messages(&parsed);
    assert!(!msgs.iter().any(|m| m.contains("vspace")), "{msgs:?}");
    assert_eq!(parsed.block_par_starts.len(), parsed.blocks.len());
    let glue: Vec<(u8, f64, f64, f64)> = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::VFill { order, stretch, natural_pt, shrink_pt } => Some((*order, *stretch, *natural_pt, *shrink_pt)),
            Block::VSpace { pt, .. } => Some((0, 0.0, *pt, 0.0)),
            _ => None,
        })
        .collect();
    assert_eq!(
        glue,
        [
            (2, 1.0, 0.0, 0.0),
            (1, 1.0, 0.0, 0.0),
            (1, 1.0, 0.0, 0.0),
            (2, 1.0, 0.0, 0.0),
            (2, 1.0, 0.0, 0.0),
            (2, 2.0, 0.0, 0.0),
            (3, 1.0, 5.0, 2.0),
            (2, 1.0, 12.0, 0.0),
        ]
    );
}
