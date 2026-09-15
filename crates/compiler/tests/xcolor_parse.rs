//! xcolor/color.sty in the parser: which colour every text run, formula and
//! box carries, and how `\color` is scoped. Operator values themselves are
//! pinned against pdfTeX in `color_oracle.rs`.

use flashtex_compiler::parser::{parse, Block, Inline, Parsed};

fn doc(preamble: &str, body: &str) -> Parsed {
    parse(&format!("\\documentclass{{article}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"))
}

fn walk<'a>(inlines: &'a [Inline], out: &mut Vec<&'a Inline>) {
    for inline in inlines {
        out.push(inline);
        if let Inline::ColorBox(b) = inline {
            walk(&b.content, out);
        }
    }
}

fn inlines(parsed: &Parsed) -> Vec<&Inline> {
    let mut out = Vec::new();
    for block in &parsed.blocks {
        match block {
            Block::Paragraph(content)
            | Block::Styled { content, .. }
            | Block::ListItem { content, .. }
            | Block::Heading { content, .. } => walk(content, &mut out),
            _ => {}
        }
    }
    out
}

/// The fill operator of the text run `word` (`None`: default colour).
fn color_of(parsed: &Parsed, word: &str) -> Option<String> {
    inlines(parsed)
        .into_iter()
        .find_map(|i| match i {
            Inline::Text { text, style, .. } if text == word => Some(style.color.map(|c| c.fill_operator())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text run {word:?}"))
}

fn messages(parsed: &Parsed) -> Vec<String> {
    parsed.diagnostics.iter().map(|d| d.message.clone()).collect()
}

#[test]
fn textcolor_and_color_are_scoped_by_groups() {
    let p = doc(
        "\\usepackage{xcolor}",
        "A \\textcolor{red}{B} C {\\color{blue!50} D \\textcolor{-red}{E} F} G",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    assert_eq!(color_of(&p, "A"), None);
    assert_eq!(color_of(&p, "B").as_deref(), Some("1 0 0 rg"));
    assert_eq!(color_of(&p, "C"), None);
    assert_eq!(color_of(&p, "D").as_deref(), Some("0.5 0.5 1 rg"));
    assert_eq!(color_of(&p, "E").as_deref(), Some("0 1 1 rg"));
    assert_eq!(color_of(&p, "F").as_deref(), Some("0.5 0.5 1 rg"));
    assert_eq!(color_of(&p, "G"), None);
}

#[test]
fn definitions_and_expressions_keep_pdftex_values() {
    let p = doc(
        "\\usepackage{xcolor}\\definecolor{mc}{cmyk}{.1,.2,.3,.4}\\colorlet{half}{mc!50}\\definecolor{web}{HTML}{FF8800}",
        "\\textcolor{half}{X} \\textcolor{web}{Y} \\textcolor[RGB]{12,200,33}{Z} {\\color{web}\\textcolor{.!50}{W}}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    assert_eq!(color_of(&p, "X").as_deref(), Some("0.05 0.09999 0.15001 0.2 k"));
    assert_eq!(color_of(&p, "Y").as_deref(), Some("1 0.53333 0 rg"));
    assert_eq!(color_of(&p, "Z").as_deref(), Some("0.04706 0.7843 0.12941 rg"));
    assert_eq!(color_of(&p, "W").as_deref(), Some("1 0.76666 0.5 rg"));
}

#[test]
fn color_persists_across_paragraphs_but_not_environments() {
    let p = doc(
        "\\usepackage{xcolor}",
        "{\\color{red} a\n\nb}\n\nc \\begin{center}\\color{blue} d\\end{center} e",
    );
    assert_eq!(color_of(&p, "a").as_deref(), Some("1 0 0 rg"));
    assert_eq!(color_of(&p, "b").as_deref(), Some("1 0 0 rg"));
    assert_eq!(color_of(&p, "c"), None);
    assert_eq!(color_of(&p, "d").as_deref(), Some("0 0 1 rg"));
    assert_eq!(color_of(&p, "e"), None);
}

#[test]
fn font_changes_keep_the_colour_and_macros_carry_it() {
    let p = doc(
        "\\usepackage[dvipsnames]{xcolor}\\newcommand{\\hl}[1]{\\textcolor{RoyalBlue}{#1}}",
        "{\\color{red}\\bfseries a \\normalfont b} \\hl{word} \\section{Head \\textcolor{Maroon}{Tail}}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    assert_eq!(color_of(&p, "a").as_deref(), Some("1 0 0 rg"));
    assert_eq!(color_of(&p, "b").as_deref(), Some("1 0 0 rg"));
    assert_eq!(color_of(&p, "word").as_deref(), Some("1 0.5 0 0 k"));
    assert_eq!(color_of(&p, "Head"), None);
    assert_eq!(color_of(&p, "Tail").as_deref(), Some("0 0.87 0.68 0.32 k"));
}

#[test]
fn math_takes_the_surrounding_colour_and_records_inner_ranges() {
    let p = doc("\\usepackage{xcolor}", "{\\color{blue} $a$} $x \\textcolor{red}{y} z \\color{green} w$");
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let maths: Vec<_> = inlines(&p)
        .into_iter()
        .filter_map(|i| match i {
            Inline::Math { color, color_ranges, span, .. } => Some((*color, color_ranges.clone(), *span)),
            _ => None,
        })
        .collect();
    assert_eq!(maths.len(), 2);
    assert_eq!(maths[0].0.map(|c| c.fill_operator()).as_deref(), Some("0 0 1 rg"));
    assert!(maths[0].1.is_empty());
    assert_eq!(maths[1].0, None);
    let source = format!(
        "\\documentclass{{article}}\n\\usepackage{{xcolor}}\n\\begin{{document}}\n{}\n\\end{{document}}\n",
        "{\\color{blue} $a$} $x \\textcolor{red}{y} z \\color{green} w$"
    );
    let ranges: Vec<(String, String)> = maths[1]
        .1
        .iter()
        .map(|(s, c)| (source[s.start..s.end].to_string(), c.fill_operator()))
        .collect();
    assert_eq!(ranges, vec![("y".to_string(), "1 0 0 rg".to_string()), ("w".to_string(), "0 1 0 rg".to_string())]);
}

#[test]
fn colorbox_and_fcolorbox_capture_fill_frame_and_fboxsep() {
    let p = doc(
        "\\usepackage{xcolor}",
        "\\colorbox{yellow}{M} \\setlength{\\fboxsep}{5pt}\\fcolorbox{red}{blue!10}{N \\textbf{O}}",
    );
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    let boxes: Vec<_> = inlines(&p)
        .into_iter()
        .filter_map(|i| match i {
            Inline::ColorBox(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes[0].fill.fill_operator(), "0 0 1 0 k");
    assert_eq!(boxes[0].frame, None);
    assert_eq!(boxes[0].fboxsep_pt, 3.0);
    assert_eq!(boxes[1].frame.map(|c| c.fill_operator()).as_deref(), Some("1 0 0 rg"));
    assert_eq!(boxes[1].fill.fill_operator(), "0.9 0.9 1 rg");
    assert_eq!(boxes[1].fboxsep_pt, 5.0);
    assert_eq!(boxes[1].content.len(), 2);
    assert_eq!(color_of(&p, "M"), None);
}

#[test]
fn page_color_and_target_models() {
    let p = doc("\\usepackage[rgb]{xcolor}\\pagecolor{yellow!20}", "\\textcolor{cyan}{X}");
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    assert_eq!(p.page_color.map(|c| c.fill_operator()).as_deref(), Some("1 1 0.8 rg"));
    assert_eq!(color_of(&p, "X").as_deref(), Some("0 1 1 rg"));
    assert_eq!(p.default_color.map(|c| c.fill_operator()).as_deref(), Some("0 0 0 rg"));
    let p = doc("\\usepackage{color}", "\\textcolor{blue}{X}");
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
    assert_eq!(color_of(&p, "X").as_deref(), Some("0 0 1 rg"));
    assert_eq!(p.default_color, None);
}

#[test]
fn errors_are_diagnosed_with_xcolor_recovery() {
    let p = doc("", "\\textcolor{red}{a}");
    assert!(messages(&p).iter().any(|m| m.contains("needs \\usepackage{xcolor}")), "{:?}", messages(&p));
    assert_eq!(color_of(&p, "a").as_deref(), Some("1 0 0 rg"));
    let p = doc("\\usepackage{xcolor}", "{\\color{red} \\textcolor{nosuch}{b} \\textcolor[hsb]{0.5,1,1}{c}}");
    let m = messages(&p);
    assert!(m.iter().any(|m| m.contains("undefined colour `nosuch`")), "{m:?}");
    assert!(m.iter().any(|m| m.contains("`hsb` is not supported")), "{m:?}");
    assert_eq!(color_of(&p, "b").as_deref(), Some("0 g"));
    assert_eq!(color_of(&p, "c").as_deref(), Some("1 0 0 rg"));
    let p = doc("\\usepackage[dvipsnames,svgnames,x11names,table]{xcolor}", "x");
    assert!(p.diagnostics.is_empty(), "{:?}", messages(&p));
}
