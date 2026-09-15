//! Kernel slice B: `\settowidth`/`\settoheight`/`\settodepth` store real
//! box measurements (GH-BOXMEASURER, issues #448-#450). Each test compiles a
//! minimal document and asserts what the command DOES to the output — never
//! just that it compiles.
//!
//! Baseline: the compiler measures at its default body font/size
//! (Times-Roman at 12pt), because the engine's `BoxMeasurer` hook receives
//! no font/size context. Width is real glyph shaping; height/depth are the
//! face's AFM ascender/descender (Times-Roman: 683/-217 per 1000em, i.e.
//! 8.196pt/2.604pt at 12pt), constant regardless of content — per-glyph ink
//! extents are unreachable without new font-engine plumbing.

use flashtex_compiler::parser::{parse, Block, Inline};

/// Joined `Inline::Text` of every paragraph, in order.
fn paragraphs(source: &str) -> Vec<String> {
    parse(source)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => {
                let mut out = String::new();
                for inline in inlines {
                    if let Inline::Text {
                        text, space_before, ..
                    } = inline
                    {
                        if *space_before && !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(text);
                    }
                }
                Some(out)
            }
            _ => None,
        })
        .collect()
}

fn messages(source: &str) -> Vec<String> {
    parse(source)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

#[test]
fn settowidth_stores_the_width_and_grows_with_the_text() {
    // Times-Roman advances at 12pt: H = 722, i = 278 per 1000em, so "Hi" is
    // exactly 1000/1000 * 12 = 12.0pt, rendered by TeX's print_scaled.
    // (The engine wraps the content in a synthetic group; the measurer
    // drops those zero-width grouping braces before shaping.)
    let source =
        "\\newlength{\\mylen}\\settowidth{\\mylen}{Hi}\\begin{document}\\the\\mylen\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["12.0pt"]);

    // A longer string measures wider: the value grows with the text instead
    // of staying at the old 0.0pt placeholder for everything.
    let short = "\\newlength{\\mylen}\\settowidth{\\mylen}{Hi}\\begin{document}\\the\\mylen\\end{document}";
    let long = "\\newlength{\\mylen}\\settowidth{\\mylen}{Hello, world!}\\begin{document}\\the\\mylen\\end{document}";
    let (short_pars, long_pars) = (paragraphs(short), paragraphs(long));
    let parse_pt = |s: &str| s.strip_suffix("pt").unwrap().parse::<f64>().unwrap();
    let (short_pt, long_pt) = (parse_pt(&short_pars[0]), parse_pt(&long_pars[0]));
    assert!(short_pt > 0.0, "{short_pars:?}");
    assert!(long_pt > short_pt, "{short_pars:?} vs {long_pars:?}");
}

#[test]
fn settoheight_stores_the_height_in_a_length_register() {
    // Face ascender 683/1000em at 12pt = 8.196pt, via TeX print_scaled.
    let source =
        "\\newlength{\\mylen}\\settoheight{\\mylen}{Hi}\\begin{document}\\the\\mylen\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["8.196pt"]);
}

#[test]
fn settodepth_stores_the_depth_in_a_length_register() {
    // |Face descender| 217/1000em at 12pt = 2.604pt, via TeX print_scaled.
    let source =
        "\\newlength{\\mylen}\\settodepth{\\mylen}{Hi}\\begin{document}\\the\\mylen\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["2.604pt"]);
}
