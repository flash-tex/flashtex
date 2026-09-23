//! The `abstract` environment is *not* set by this crate, on purpose
//! (issue #953). The render pipeline owns its typesetting
//! (`crates/render-pipeline/src/abstractenv.rs`: the `\small` centred head
//! and `quotation` body of article/report, the `titlepage` page, the
//! two-column `\section*` form) and derives it from the source plus the
//! plain paragraphs the compiler produces for the body. It supersedes the
//! compiler's "not implemented" warning where it sets the environment and
//! keeps the warning where the class has no `abstract` (`book`) or the form
//! is not set. A compiler-side head block, paragraph style or `\small` here
//! reached the pipeline through `vendor/compiler` and was typeset twice
//! (d5cedcb0a: 17 oracle tests failed), so this test pins the contract.

use flashtex_compiler::parser::{self, Block, Inline};

const DOC: &str = r"\documentclass{article}
\begin{document}
Before.

\begin{abstract}
First body paragraph.

Second body paragraph.
\end{abstract}

After.
\end{document}";

fn text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn the_body_is_plain_paragraphs_with_no_head_block_and_no_paragraph_style() {
    let parsed = parser::parse(DOC);
    let paragraphs: Vec<String> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(text(inlines)),
            _ => None,
        })
        .collect();
    assert_eq!(
        paragraphs,
        ["Before.", "First body paragraph.", "Second body paragraph.", "After."],
        "{:?}",
        parsed.blocks
    );
    assert!(
        !parsed.blocks.iter().any(|block| matches!(block, Block::Styled { .. })),
        "no `Styled` head or `quotation` body block for the pipeline to see twice: {:?}",
        parsed.blocks
    );
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { style, text, .. } = inline {
                    assert_eq!(style.size, None, "the pipeline sets `\\small` itself: {text:?}");
                    assert!(!style.bold, "{text:?}");
                }
            }
        }
    }
}

#[test]
fn the_environment_keeps_its_not_implemented_warning_for_the_pipeline_to_supersede() {
    let parsed = parser::parse(DOC);
    let warnings: Vec<&str> = parsed.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert_eq!(
        warnings,
        ["environment 'abstract' is not implemented; its body is typeset as plain text"],
        "{:?}",
        parsed.diagnostics
    );
    let span = parsed.diagnostics[0].span.expect("the warning points at the command");
    assert_eq!(&DOC[span.start..span.end], r"\begin");
}
