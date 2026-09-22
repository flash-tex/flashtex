//! NFSS `\fontsize{<size>}{<skip>}\selectfont`: the expansion engine
//! resolves `\f@size`/`\f@baselineskip` and hands both commands back; the
//! parser puts the exact size on the style (`FontSizeLevel::Explicit`) at
//! `\selectfont`, scoped like `\Large`, with the size of the font the
//! family's `.fd` loads for it (`ExplicitSize::font_sp`).
use flashtex_compiler::parser::{parse, Block, ExplicitSize, FontSizeLevel, Inline};

fn sizes(source: &str) -> Vec<(String, Option<FontSizeLevel>)> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.iter().all(|d| !d.message.contains("fontsize") && !d.message.contains("selectfont") && !d.message.contains("baselineskip")), "{:?}", parsed.diagnostics);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let inlines = match block {
            Block::Paragraph(inlines) | Block::Heading { content: inlines, .. } => inlines,
            _ => continue,
        };
        for inline in inlines {
            if let Inline::Text { text, style, .. } = inline {
                out.push((text.clone(), style.size));
            }
        }
    }
    out
}

fn explicit(size: f64, font: f64, skip: f64) -> Option<FontSizeLevel> {
    let sp = |pt: f64| (pt * 65536.0).round() as i32;
    Some(FontSizeLevel::Explicit(ExplicitSize { size_sp: sp(size), font_sp: sp(font), baselineskip_sp: sp(skip) }))
}

#[test]
fn the_size_applies_at_selectfont_and_ends_with_the_group() {
    let source = "\\documentclass{article}\n\\begin{document}\nA {\\fontsize{13}{15}B \\selectfont C} D\n\\end{document}\n";
    // OT1 `cmr` declares no 13pt: LaTeX substitutes `cmr12` (pdflatex:
    // "size <13> not available, size <12> substituted").
    assert_eq!(
        sizes(source),
        [("A".to_string(), None), ("B".to_string(), None), ("C".to_string(), explicit(13.0, 12.0, 15.0)), ("D".to_string(), None)]
    );
}

#[test]
fn latin_modern_scales_to_the_exact_size() {
    let source = "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n\\begin{document}\n{\\fontsize{9.5}{11.5}\\selectfont x}\n\\end{document}\n";
    assert_eq!(sizes(source), [("x".to_string(), explicit(9.5, 9.5, 11.5))]);
}

#[test]
fn a_heading_argument_takes_it_too() {
    let source = "\\documentclass{article}\n\\begin{document}\n\\section{\\fontsize{13}{15}\\selectfont Head}\nx\n\\end{document}\n";
    let got = sizes(source);
    assert!(got.contains(&("Head".to_string(), explicit(13.0, 12.0, 15.0))), "{got:?}");
    assert!(!got.iter().any(|(t, _)| t.contains("pt")), "{got:?}");
}
