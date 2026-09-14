//! LaTeX structure recovered on top of the compiler's parse tree: section
//! numbers and `secnumdepth`, `\ref`/`\pageref`, `equation` numbers versus
//! `\[`, the display skips TeX chooses, `\newpage`, and headings under
//! their own `\baselineskip`.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::{Capabilities, V1Item};
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

fn items(text: &str, options: &RenderOptions) -> Vec<(u32, String, f64, f64, usize, usize)> {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 1, "p", &fonts, options);
    let v1 = v1_of(&r, Capabilities::default());
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    v1.pages
        .iter()
        .flat_map(|p| {
            p.items.iter().filter_map(move |it| match it {
                V1Item::Text {
                    text,
                    x_pt,
                    baseline_y_pt,
                    source,
                    ..
                } => Some((p.number, text.clone(), *x_pt, *baseline_y_pt, source.start(), source.end())),
                V1Item::Rule { .. } => None,
            })
        })
        .collect()
}

fn find<'a>(items: &'a [(u32, String, f64, f64, usize, usize)], text: &str) -> &'a (u32, String, f64, f64, usize, usize) {
    items.iter().find(|i| i.1 == text).unwrap_or_else(|| panic!("no item {text:?} in {items:?}"))
}

#[test]
fn section_numbers_follow_secnumdepth_and_point_at_the_command() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let src = "\\begin{document}\\section{Head}\nBody.\\end{document}";
    let numbered = items(src, &RenderOptions::default());
    let one = find(&numbered, "1");
    let head = find(&numbered, "Head");
    assert_eq!((one.4, one.5), (16, 24), "the number's bytes are the \\section command's");
    assert!(head.2 > one.2 + 17.0, "a \\quad of the heading font separates number and title: {numbered:?}");
    assert_eq!(one.3, head.3);

    let unnumbered = items(
        src,
        &RenderOptions {
            default_secnumdepth: 0,
            ..RenderOptions::default()
        },
    );
    assert!(unnumbered.iter().all(|i| i.1 != "1"));
    assert_eq!(find(&unnumbered, "Head").2, 72.0);

    // `\pagestyle{empty}`: the plain footer's page number would be a "1" too.
    let counter = "\\documentclass[12pt]{article}\\pagestyle{empty}\\setcounter{secnumdepth}{0}\\begin{document}\\section{Head} Body.\\end{document}";
    let by_source = items(counter, &RenderOptions::default());
    assert!(by_source.iter().all(|i| i.1 != "1"), "{by_source:?}");
}

#[test]
fn references_resolve_to_label_values_and_pages() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let mut src = String::from("\\begin{document}\\section{A}\\label{s}\n");
    for _ in 0..60 {
        src.push_str("Filler line with enough words to stand on its own line of text here.\\\\\n");
    }
    src.push_str("See \\ref{s} on page \\pageref{s} and \\ref{nope}.\\end{document}");
    let out = items(&src, &RenderOptions::default());
    let see = find(&out, "See");
    assert_eq!(see.0, 2, "sixty forced lines push the reference to page 2");
    let after_see: Vec<&str> = out.iter().skip_while(|i| i.1 != "See").map(|i| i.1.as_str()).take(7).collect();
    assert_eq!(after_see, ["See", "1", "on", "page", "1", "and", "??."]);
    let r = out.iter().skip_while(|i| i.1 != "See").nth(1).unwrap();
    assert_eq!(&src[r.4..r.5], "\\ref{s}");
}

#[test]
fn only_equation_environments_are_numbered_and_the_number_is_flush_right() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let eq = items("\\begin{document}Text.\n\\begin{equation} x = y \\end{equation}\\end{document}", &RenderOptions::default());
    let number = find(&eq, "(1)");
    let x = find(&eq, "x");
    assert_eq!(number.3, x.3, "the equation number sits on the display baseline");
    assert!(number.2 > 520.0 && number.2 < 540.0, "flush right inside the 1in margin: {number:?}");
    let bracket = items("\\begin{document}Text.\n\\[ x = y \\]\\end{document}", &RenderOptions::default());
    assert!(bracket.iter().all(|i| !i.1.starts_with('(')), "\\[ is unnumbered: {bracket:?}");
}

#[test]
fn display_skips_follow_tex_pre_display_size() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // `\[` after a blank line: LaTeX sets an empty .6\linewidth box with
    // \nointerlineskip first, so the long \abovedisplayskip (12pt) applies
    // and the display baseline is 12 + (14.5 - h) + h = 26.5pt below the
    // text line ("Text." has no depth). `equation` after a blank line
    // starts with the bare \parindent box on a normal interline glue
    // (14.5pt) followed by the short skip (0pt): 14.5 + 14.5 = 29pt — the
    // well-known extra blank line LaTeX gives an equation after a blank
    // line. 1pt = 72/72.27 bp.
    let bracket = items("\\begin{document}Text.\n\n\\[ x \\]\\end{document}", &RenderOptions::default());
    let equation = items("\\begin{document}Text.\n\n\\begin{equation} x \\end{equation}\\end{document}", &RenderOptions::default());
    let gap = |v: &[(u32, String, f64, f64, usize, usize)]| find(v, "x").3 - find(v, "Text.").3;
    let (g_bracket, g_equation) = (gap(&bracket), gap(&equation));
    assert!((g_bracket - 26.5 * 72.0 / 72.27).abs() < 0.3, "\\[ gap {g_bracket}");
    assert!((g_equation - 29.0 * 72.0 / 72.27).abs() < 0.3, "equation gap {g_equation}");
}

#[test]
fn newpage_between_paragraphs_ejects_the_page() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let out = items("\\begin{document}First.\n\\newpage\nSecond.\n\\clearpage\n\\section{Third}\\end{document}", &RenderOptions::default());
    assert_eq!(find(&out, "First.").0, 1);
    assert_eq!(find(&out, "Second.").0, 2);
    assert_eq!(find(&out, "Third").0, 3);
    // Skips are discarded at a page top: each page starts at \topskip.
    assert_eq!(find(&out, "First.").3, find(&out, "Second.").3);
}

#[test]
fn a_heading_after_body_text_uses_its_own_baselineskip() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    // 12pt article: \section = \Large on a 22pt baselineskip after
    // \addvspace{3.5ex plus 1ex minus .2ex}; with Latin Modern's ex
    // (0.4306 x 12pt) that is 22 + 18.09 = 40.09pt = 39.94bp from the
    // previous baseline (pdflatex measures the same).
    let out = items(
        "\\begin{document}\\section{Introduction}\nBody text under a heading.\n\\section{Second Section}\nMore.\\end{document}",
        &RenderOptions {
            default_secnumdepth: 0,
            ..RenderOptions::default()
        },
    );
    let gap = find(&out, "Second").3 - find(&out, "Body").3;
    assert!((gap - 39.94).abs() < 0.05, "gap {gap}");
}
