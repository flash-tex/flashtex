//! `algorithm`, `algorithmic` and `algpseudocode` through the parser: no
//! unsupported diagnostics for the packages' commands, `Algorithm N`
//! captions and `\label` values, and one left-aligned paragraph per
//! statement with bold keywords (the package model itself is unit-tested in
//! `src/algorithmic.rs`).

use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument, TextStyle};

fn parse(text: &str) -> flashtex_compiler::parser::Parsed {
    parse_project(
        &[SourceDocument {
            path: "main.tex",
            text,
        }],
        "main.tex",
    )
}

fn words(content: &[Inline]) -> Vec<(String, bool)> {
    content
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, style, .. } => Some((text.clone(), style.bold)),
            _ => None,
        })
        .collect()
}

#[test]
fn algpseudocode_float_parses_without_diagnostics() {
    let src = r"\documentclass{article}
\usepackage{algorithm}
\usepackage[noend]{algpseudocode}
\renewcommand{\algorithmicrequire}{\textbf{Input:}}
\algrenewcommand\algorithmicensure{\textbf{Output:}}
\algnewcommand\Global{\item[\textbf{Global:}]}
\algrenewcommand\algorithmicindent{1em}
\floatname{algorithm}{Procedure}
\begin{document}
\begin{algorithm}[t]
\caption{Euclid}\label{alg:euclid}
\begin{algorithmic}[1]
\Require $a, b$
\Global $g$
\Procedure{Euclid}{$a,b$}\Comment{gcd}
\While{$b \neq 0$}
\State \Call{Swap}{$a, b$}
\EndWhile
\State \Return $a$
\EndProcedure
\end{algorithmic}
\end{algorithm}
See Procedure~\ref{alg:euclid}.
\end{document}
";
    let parsed = parse(src);
    let unexpected: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|d| !d.message.contains("PDF export") && !d.message.contains("has no glyph"))
        .map(|d| d.message.clone())
        .collect();
    assert!(unexpected.is_empty(), "{unexpected:?}");
    let caption = parsed
        .blocks
        .iter()
        .find_map(|b| match b {
            Block::FigureCaption { content } => Some(words(content)),
            _ => None,
        })
        .expect("caption");
    assert_eq!(
        caption,
        vec![
            ("Procedure".into(), true),
            ("1".into(), true),
            ("Euclid".into(), false)
        ]
    );
    let label = parsed.blocks.iter().find_map(|b| match b {
        Block::Paragraph(content)
        | Block::Styled { content, .. }
        | Block::FigureCaption { content } => content.iter().find_map(|i| match i {
            Inline::Label { key, value, .. } if key == "alg:euclid" => Some(value.clone()),
            _ => None,
        }),
        _ => None,
    });
    assert_eq!(label.as_deref(), Some("1"));
    let lines: Vec<Vec<(String, bool)>> = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Styled { content, .. } => Some(words(content)),
            _ => None,
        })
        .collect();
    // `noend`: `\EndWhile`/`\EndProcedure` print nothing and set no line.
    assert_eq!(lines.len(), 6, "{lines:?}");
    assert_eq!(lines[0][0], ("Input:".into(), true));
    assert_eq!(lines[1][0], ("Global:".into(), true));
    assert_eq!(
        lines[2][..3],
        [
            ("1:".into(), false),
            ("procedure".into(), true),
            ("Euclid".into(), false)
        ]
    );
    assert!(lines[2].contains(&("▷".into(), false)) && lines[2].contains(&("gcd".into(), false)));
    assert!(lines[3].contains(&("while".into(), true)) && lines[3].contains(&("do".into(), true)));
    assert_eq!(
        lines[4][..3],
        [
            ("3:".into(), false),
            ("Swap".into(), false),
            ("(".into(), false)
        ]
    );
    assert_eq!(
        lines[5][..2],
        [("4:".into(), false), ("return".into(), true)]
    );
    let number_style = parsed.blocks.iter().find_map(|b| match b {
        Block::Styled { content, .. } => content.iter().find_map(|i| match i {
            Inline::Text { text, style, .. } if text == "1:" => Some(*style),
            _ => None,
        }),
        _ => None,
    });
    assert_eq!(
        number_style.map(|s: TextStyle| s.size),
        Some(Some(flashtex_compiler::parser::FontSizeLevel::FootnoteSize))
    );
}

#[test]
fn algorithmic_float_with_uppercase_commands() {
    let src = r"\documentclass{article}
\usepackage{algorithm}
\usepackage{algorithmic}
\begin{document}
\begin{algorithm}
\caption{Loop}
\begin{algorithmic}
\REQUIRE $n \ge 0$
\FOR{$i = 1$ \TO $n$}
\IF[odd]{$i$ is odd}
\STATE print $i$ \COMMENT{output}
\ELSIF{$i = 2$}
\RETURN \TRUE
\ENDIF
\ENDFOR
\end{algorithmic}
\end{algorithm}
\end{document}
";
    let parsed = parse(src);
    let messages: Vec<_> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(messages.is_empty(), "{messages:?}");
    let lines: Vec<String> = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Styled { content, .. } => Some(
                words(content)
                    .into_iter()
                    .map(|(t, _)| t)
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            "Require: n 0",
            "for i = 1 to n do",
            "if i is odd then { odd }",
            "print i { output }",
            "else if i = 2 then",
            "return true",
            "end if",
            "end for",
        ]
    );
}
