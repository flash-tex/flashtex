//! `Inline::Math::size`: the size declaration in force where a formula
//! starts, which LaTeX's `\check@mathfonts` sets its math fonts from.

use flashtex_compiler::parser::{parse, Block, FontSizeLevel, Inline};

fn math_sizes(body: &str) -> Vec<Option<FontSizeLevel>> {
    let parsed = parse(&format!("\\documentclass[12pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let (Block::Paragraph(content) | Block::Styled { content, .. }) = block else { continue };
        for inline in content {
            if let Inline::Math { size, .. } = inline {
                out.push(*size);
            }
        }
    }
    out
}

#[test]
fn a_formula_records_the_size_declaration_around_it() {
    assert_eq!(
        math_sizes("$a$ {\\small $b$ {\\footnotesize $c$}} {\\large $d$} $e$"),
        [None, Some(FontSizeLevel::Small), Some(FontSizeLevel::FootnoteSize), Some(FontSizeLevel::Large1), None]
    );
}

#[test]
fn a_size_environment_and_a_declaration_to_the_group_end_count_too() {
    assert_eq!(
        math_sizes("\\begin{small}$a$\\end{small} $b$ {\\scriptsize x \\normalsize $c$}"),
        [Some(FontSizeLevel::Small), None, None]
    );
}
