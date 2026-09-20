//! The optional note of a theorem head (`\begin{theorem}[B\'ezout]`) is
//! ordinary text: amsthm's `\thmnote{ {\the\thm@notefont(#3)}}` typesets
//! `#3` as it would any argument, so `\'e` is `é`. pdflatex (TeX Live 2026,
//! 11pt `[T1]{fontenc}` article, amsthm): the head of
//! fixtures/real-world/lecture-notes page 2 reads `Theorem 3.4 (Bézout).`
//! with `(Bézout).` 44.895 bp wide at x 144.945 and `For` at 194.815. The
//! note used to reach the head as raw text, so `\'e` degraded to the
//! apostrophe-then-`e` `(B’ezout)` — one quoteright (3.03 bp) wider — and
//! every word of the first line sat up to 2.83 bp right of pdflatex.
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage{{amsthm}}\n\\newtheorem{{theorem}}{{Theorem}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn head_texts(body: &str) -> Vec<(String, bool, bool)> {
    let source = doc(body);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(
                inlines
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Text { text, style, space_before, .. } => {
                            Some((text.clone(), style.bold, *space_before))
                        }
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .expect("a paragraph")
}

#[test]
fn the_note_is_set_as_text_with_its_accents() {
    let body = "\\begin{theorem}[B\\'ezout]\nFor all.\n\\end{theorem}";
    let texts = head_texts(body);
    // `Theorem 1` (bold), the head-font space, `(`, the note's pieces, `)`,
    // the bold period, then the body. The accent reaches the render
    // pipeline the way body text does: the mark as a one-character text
    // whose two-byte span is the `\'` of the source (the pipeline's
    // `accent()` composes `é` from it), never as a literal apostrophe glued
    // into the word.
    let spelled: Vec<String> = texts.iter().map(|(t, bold, sp)| format!("{}{}{t}", if *sp { "+" } else { "" }, if *bold { "B:" } else { "" })).collect();
    assert_eq!(spelled[..7], ["+B:Theorem 1", "B: ", "(", "B", "’", "ezout", ")"], "{spelled:?}");
    assert_eq!(spelled[7], "B:.", "the head punctuation stays in the head font");
    let source = doc(body);
    let parsed = parse(&source);
    let mark = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::Text { text, span, .. } if text == "’" => Some(*span),
                _ => None,
            }),
            _ => None,
        })
        .expect("the accent mark");
    assert_eq!(&source[mark.start..mark.end], "\\'");
}

#[test]
fn the_parentheses_stand_on_the_brackets_own_bytes() {
    let body = "\\begin{theorem}[Euclid]\nFor all.\n\\end{theorem}";
    let source = doc(body);
    let parsed = parse(&source);
    let inlines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .unwrap();
    let span_of = |want: &str| {
        inlines
            .iter()
            .find_map(|inline| match inline {
                Inline::Text { text, span, .. } if text == want => Some(*span),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no {want:?}"))
    };
    let (open, note, close) = (span_of("("), span_of("Euclid"), span_of(")"));
    assert_eq!(&source[open.start..open.end], "[");
    assert_eq!(&source[note.start..note.end], "Euclid");
    assert_eq!(&source[close.start..close.end], "]");
    assert_eq!(open.end, note.start);
    assert_eq!(note.end, close.start);
}
