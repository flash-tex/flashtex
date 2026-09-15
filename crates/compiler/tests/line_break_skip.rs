//! `\\[<dimen>]` reports its optional argument on `Inline::LineBreak`.
//!
//! latex.ltx's `\\` is `\@normalcr`: it ends the line and then `\@xnewline`
//! issues `\vspace{<dimen>}`, so the bracketed argument is a real vertical
//! skip after the broken line. The parser has always *consumed* it (it must
//! never reach the page as text) but used to throw the value away, which left
//! every consumer to re-read the `[...]` out of the source bytes following the
//! node's span.
//!
//! That re-reading is only correct for a `\\` written literally in the
//! document. Expanded from a macro body, the node's span is the *invocation*
//! (`expansion::Converter::place` places a replacement-text token at the call
//! site), so the bytes after it are the call's own arguments — the `{Education}`
//! of `\cvsection{Education}` — and the skip cannot be recovered at all. The
//! pdflatex oracle for the layout consequence is
//! `crates/render-pipeline/tests/line_break_skip_macro.rs`.

use flashtex_compiler::parser::{parse, Block, Inline};

/// Every `Inline::LineBreak` in the document, in order, as its reported skip.
fn line_break_skips(text: &str) -> Vec<Option<f64>> {
    let parsed = parse(text);
    let mut out = Vec::new();
    collect(&parsed.blocks, &mut out);
    out
}

fn collect(blocks: &[Block], out: &mut Vec<Option<f64>>) {
    for block in blocks {
        let inlines = match block {
            Block::Paragraph(inlines)
            | Block::Styled {
                content: inlines, ..
            }
            | Block::ListItem {
                content: inlines, ..
            } => inlines,
            _ => continue,
        };
        for inline in inlines {
            if let Inline::LineBreak { skip_pt, .. } = inline {
                out.push(*skip_pt);
            }
        }
    }
}

const PREAMBLE: &str = r"\documentclass[11pt]{article}
\setlength{\parindent}{0pt}
";

#[test]
fn a_literal_line_break_reports_its_optional_argument() {
    let text = format!(
        "{PREAMBLE}{}",
        r"\begin{document}
First\\[-6pt]
Second
\end{document}
"
    );
    assert_eq!(line_break_skips(&text), vec![Some(-6.0)]);
}

#[test]
fn a_line_break_without_an_argument_reports_none() {
    let text = format!(
        "{PREAMBLE}{}",
        r"\begin{document}
First\\
Second
\end{document}
"
    );
    assert_eq!(line_break_skips(&text), vec![None]);
}

/// The regression this file exists for: the `\\[-6pt]` lives in a
/// `\newcommand` body, so the node's span is `\cvsection{Education}` and no
/// consumer reading source bytes after that span can see the `-6pt`.
#[test]
fn a_line_break_inside_a_macro_body_reports_its_argument() {
    let text = format!(
        "{PREAMBLE}{}",
        r"\newcommand{\cvsection}[1]{{\large\bfseries #1}\\[-6pt]\rule{\textwidth}{0.6pt}}
\begin{document}
Top.

\cvsection{Education}

Body.
\end{document}
"
    );
    assert_eq!(line_break_skips(&text), vec![Some(-6.0)]);
}

/// Two calls of the same macro each report the skip; a second, argument-less
/// `\\` in the same body stays `None`, so the value is per node and not a
/// per-macro lookup.
#[test]
fn each_expansion_reports_its_own_skip() {
    let text = format!(
        "{PREAMBLE}{}",
        r"\newcommand{\head}[1]{#1\\[-6pt]rule\\here}
\begin{document}
\head{One}

\head{Two}
\end{document}
"
    );
    assert_eq!(
        line_break_skips(&text),
        vec![Some(-6.0), None, Some(-6.0), None]
    );
}

/// Units other than `pt` are converted, and the argument is still never
/// typeset: no page item may contain the bracketed text.
#[test]
fn the_argument_is_converted_and_never_typeset() {
    let text = format!(
        "{PREAMBLE}{}",
        r"\newcommand{\gap}{\\[1in]}
\begin{document}
First\gap Second
\end{document}
"
    );
    assert_eq!(line_break_skips(&text), vec![Some(72.27)]);
    let parsed = parse(&text);
    let mut words = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { text, .. } = inline {
                    words.push(text.clone());
                }
            }
        }
    }
    assert!(
        !words.iter().any(|w| w.contains('[') || w.contains("1in")),
        "the optional argument reached the page as text: {words:?}"
    );
}
