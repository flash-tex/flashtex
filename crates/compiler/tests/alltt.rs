//! `alltt` keeps verbatim whitespace while leaving commands and groups live.
use flashtex_compiler::incremental::{compile_full_project, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline, SourceDocument, TextFamily};

fn compile(source: &str) -> CompileOutput {
    compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: source,
        }],
        "main.tex",
        LayoutConstraints::default(),
    )
}

fn alltt_block(source: &str) -> (Vec<Vec<Inline>>, bool) {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::Alltt { lines, vmode, .. } => Some((lines, vmode)),
            _ => None,
        })
        .expect("alltt block")
}

#[test]
fn alltt_preserves_lines_spaces_and_live_commands() {
    let source = r#"\documentclass[10pt]{article}
\usepackage{alltt}
\begin{document}
\begin{alltt}
  one  two
three \emph{EM} {GROUP}
\end{alltt}
\end{document}
"#;
    let (lines, vmode) = alltt_block(source);
    assert!(vmode);
    assert_eq!(lines.len(), 2);
    let first: Vec<&str> = lines[0]
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(first, [" ", " ", "one", " ", " ", "two"]);
    assert!(lines.iter().flatten().all(|inline| match inline {
        Inline::Text { style, .. } => style.family == TextFamily::Mono,
        _ => true,
    }));
    assert!(lines[1].iter().any(|inline| matches!(
        inline,
        Inline::Text { text, style, .. }
            if text == "EM" && style.family == TextFamily::Mono && style.italic
    )));
    assert!(lines[1].iter().any(|inline| matches!(
        inline,
        Inline::Text { text, style, .. }
            if text == "GROUP" && style.family == TextFamily::Mono
    )));
    assert!(!lines[1].iter().any(|inline| {
        matches!(inline, Inline::Text { text, .. } if text == "{" || text == "}")
    }));
}

fn one_block_source(size: u32, body: &str) -> String {
    format!(
        "\\documentclass[{size}pt]{{article}}\n\\usepackage{{alltt}}\n\\begin{{document}}\n{body}\\end{{document}}\n"
    )
}

fn x_of(output: &CompileOutput, text: &str) -> f64 {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.text == text)
        .unwrap_or_else(|| panic!("missing {text:?}: {:?}", output.pages))
        .x_pt
}

fn all_text_in_order(output: &CompileOutput) -> String {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .map(|item| item.text.as_str())
        .collect::<Vec<_>>()
        .join("")
}

fn baseline(output: &CompileOutput, text: &str) -> f64 {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .find(|item| item.text == text)
        .unwrap_or_else(|| panic!("missing {text:?}: {:?}", output.pages))
        .baseline_y_pt
}

#[test]
fn alltt_vertical_gaps_stay_within_measured_pdflatex_gate_at_each_size() {
    // PyMuPDF glyph-origin measurements from pdflatex 2026, article.cls:
    // consecutive/blank and paragraph-to-alltt gaps in bp are respectively
    // 10pt: 21.917/19.925, 11pt: 25.505/22.516, 12pt: 27.398/24.409.
    const PDFLATEX_CONSECUTIVE_BP: [f64; 3] = [21.917, 25.505, 27.398];
    const PDFLATEX_AFTER_PARAGRAPH_BP: [f64; 3] = [19.925, 22.516, 24.409];
    for (index, size) in [10, 11, 12].into_iter().enumerate() {
        let consecutive = one_block_source(
            size,
            "\\begin{alltt}\nFIRST\n\\end{alltt}\n\\begin{alltt}\nSECOND\n\\end{alltt}\n",
        );
        let output = compile(&consecutive);
        assert!(
            output.diagnostics.is_empty(),
            "{size}pt: {:?}",
            output.diagnostics
        );
        let actual = baseline(&output, "SECOND") - baseline(&output, "FIRST");
        assert!(
            (actual - PDFLATEX_CONSECUTIVE_BP[index]).abs() < 0.5,
            "{size}pt consecutive gap {actual:.3} vs pdflatex {:.3} bp",
            PDFLATEX_CONSECUTIVE_BP[index]
        );

        let after_paragraph = one_block_source(
            size,
            "PRE paragraph.\n\\begin{alltt}\nAFTERPAR\n\\end{alltt}\n",
        );
        let output = compile(&after_paragraph);
        assert!(
            output.diagnostics.is_empty(),
            "{size}pt: {:?}",
            output.diagnostics
        );
        let actual = baseline(&output, "AFTERPAR") - baseline(&output, "PRE");
        assert!(
            (actual - PDFLATEX_AFTER_PARAGRAPH_BP[index]).abs() < 0.5,
            "{size}pt paragraph gap {actual:.3} vs pdflatex {:.3} bp",
            PDFLATEX_AFTER_PARAGRAPH_BP[index]
        );

        let after_blank = one_block_source(
            size,
            "PRE paragraph.\n\n\\begin{alltt}\nAFTERBLANK\n\\end{alltt}\n",
        );
        let output = compile(&after_blank);
        assert!(
            output.diagnostics.is_empty(),
            "{size}pt: {:?}",
            output.diagnostics
        );
        let actual = baseline(&output, "AFTERBLANK") - baseline(&output, "PRE");
        assert!(
            (actual - PDFLATEX_CONSECUTIVE_BP[index]).abs() < 0.5,
            "{size}pt blank-line gap {actual:.3} vs pdflatex {:.3} bp",
            PDFLATEX_CONSECUTIVE_BP[index]
        );
    }
}

#[test]
fn alltt_closing_gap_to_paragraph_matches_pdflatex_both_entry_modes() {
    // TeX's `\@endparenv` closing skip is `\@topsepadd`: `topsep` plus
    // `partopsep` when the environment was entered in vertical mode (after a
    // blank line), `topsep` alone mid-paragraph. PyMuPDF glyph-origin deltas
    // from pdflatex 2026, alltt body baseline to following paragraph baseline:
    // vmode 10/11/12pt: 21.917/25.505/27.398; hmode: 19.925/22.516/24.409.
    const PDFLATEX_VMODE_BP: [f64; 3] = [21.917, 25.505, 27.398];
    const PDFLATEX_HMODE_BP: [f64; 3] = [19.925, 22.516, 24.409];
    for (index, size) in [10, 11, 12].into_iter().enumerate() {
        let vmode = one_block_source(
            size,
            "PRE paragraph.\n\n\\begin{alltt}\nVFIRST\n\\end{alltt}\nVAFTER paragraph here.\n",
        );
        let output = compile(&vmode);
        assert!(
            output.diagnostics.is_empty(),
            "{size}pt: {:?}",
            output.diagnostics
        );
        let actual = baseline(&output, "VAFTER") - baseline(&output, "VFIRST");
        assert!(
            (actual - PDFLATEX_VMODE_BP[index]).abs() < 0.5,
            "{size}pt vmode closing gap {actual:.3} vs pdflatex {:.3} bp",
            PDFLATEX_VMODE_BP[index]
        );

        let hmode = one_block_source(
            size,
            "PRE paragraph.\n\\begin{alltt}\nHFIRST\n\\end{alltt}\nHAFTER paragraph here.\n",
        );
        let output = compile(&hmode);
        assert!(
            output.diagnostics.is_empty(),
            "{size}pt: {:?}",
            output.diagnostics
        );
        let actual = baseline(&output, "HAFTER") - baseline(&output, "HFIRST");
        assert!(
            (actual - PDFLATEX_HMODE_BP[index]).abs() < 0.5,
            "{size}pt hmode closing gap {actual:.3} vs pdflatex {:.3} bp",
            PDFLATEX_HMODE_BP[index]
        );
    }
}

#[test]
fn alltt_inside_quote_and_itemize_matches_sibling_x() {
    // `alltt.sty` sets `\leftskip\@totalleftmargin`, so the body sits at the
    // enclosing list's margin: the same x as a sibling paragraph there.
    let quote = one_block_source(
        10,
        "\\begin{quote}\nQUOTEPARA tail words here.\n\n\\begin{alltt}\nQALLTT\n\\end{alltt}\n\\end{quote}\n",
    );
    let output = compile(&quote);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(x_of(&output, "QALLTT"), x_of(&output, "QUOTEPARA"));

    let itemize = one_block_source(
        10,
        "\\begin{itemize}\n\\item ITEMTEXT tail words here.\n\\begin{alltt}\nIALLTT\n\\end{alltt}\n\\end{itemize}\n",
    );
    let output = compile(&itemize);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(x_of(&output, "IALLTT"), x_of(&output, "ITEMTEXT"));
}

#[test]
fn alltt_percent_is_literal_inside_and_comment_after() {
    let source = one_block_source(
        10,
        "\\begin{alltt}\nA%B\n\\end{alltt}\nTAIL % comment here\nNEXT line.\n",
    );
    let output = compile(&source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let text = all_text_in_order(&output);
    assert!(text.contains("A%B"), "% is literal inside alltt: {text:?}");
    assert!(
        !text.contains("comment"),
        "% resumes starting a comment after alltt: {text:?}"
    );
    assert!(
        text.contains("TAIL") && text.contains("NEXT"),
        "surrounding text intact: {text:?}"
    );
}

#[test]
fn alltt_special_chars_are_literal() {
    let source = one_block_source(10, "\\begin{alltt}\nP$Q#R&S^T_U~V\n\\end{alltt}\n");
    let output = compile(&source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let text = all_text_in_order(&output);
    for special in ["$", "#", "&", "^", "_", "~"] {
        assert!(
            text.contains(special),
            "{special:?} is literal inside alltt: {text:?}"
        );
    }
}

#[test]
fn alltt_nests_inside_quote_without_diagnostics() {
    let source = one_block_source(
        10,
        "\\begin{quote}\nQPARA words here.\n\\begin{alltt}\nNESTED\n\\end{alltt}\n\\end{quote}\n",
    );
    let output = compile(&source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    let kinds: Vec<&str> = output
        .blocks
        .iter()
        .map(|block| match block {
            Block::Styled { .. } => "Styled",
            Block::Alltt { .. } => "Alltt",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["Styled", "Alltt"]);
    let nested: Vec<String> = output
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Alltt { lines, .. } => Some(
                lines
                    .iter()
                    .flat_map(|line| line.iter())
                    .filter_map(|inline| match inline {
                        Inline::Text { text, .. } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<String>(),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(nested, ["NESTED"]);
}

#[test]
fn alltt_unterminated_still_typesets_its_lines() {
    let source = one_block_source(10, "\\begin{alltt}\nLONE line here\n");
    let output = compile(&source);
    let text = all_text_in_order(&output);
    assert!(
        text.contains("LONE") && text.contains("line") && text.contains("here"),
        "unterminated alltt keeps its lines: {text:?} {:?}",
        output.diagnostics
    );
}
