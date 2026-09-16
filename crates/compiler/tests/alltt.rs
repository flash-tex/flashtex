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
