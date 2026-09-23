//! latex.ltx `\@startsection` (17231-17315): a class or package that
//! redefines `\section` & co. with its own indent, skips and style. The
//! expansion engine runs `\@startsection`/`\@sect`/`\@ssect`
//! (`expansion::HOST_PRELUDE`) and the parser's `startsection_marker`
//! sets the heading: a `Block::Heading` with the class's skips expressed
//! as the difference from the standard-class skips of its level, or a
//! run-in head at the start of the following paragraph when the
//! after-skip is negative (`\@xsect`'s `\everypar` branch).
//!
//! Probe: `fixtures/divergence-probes/min-startsection` (pdflatex reference).
use flashtex_compiler::parser::{parse_project, Block, Inline, Parsed, SourceDocument};

const STYLE: &str = "\\NeedsTeXFormat{LaTeX2e}\\ProvidesPackage{secstyle}\n\
\\renewcommand\\section{\\@startsection{section}{1}{\\z@}{-2.0ex \\@plus -0.5ex \\@minus -0.2ex}{1.5ex \\@plus 0.3ex \\@minus 0.2ex}{\\large\\bf\\raggedright}}\n\
\\renewcommand\\subsection{\\@startsection{subsection}{2}{\\z@}{-1.8ex \\@plus -0.5ex \\@minus -0.2ex}{0.8ex \\@plus 0.2ex}{\\normalsize\\bf\\raggedright}}\n\
\\renewcommand\\paragraph{\\@startsection{paragraph}{4}{\\z@}{1.5ex \\@plus 0.5ex \\@minus 0.2ex}{-1em}{\\normalsize\\bf}}\n";

fn parse(main: &str) -> Parsed {
    parse_project(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "secstyle.sty", text: STYLE }], "main.tex")
}

fn errors(parsed: &Parsed) -> Vec<String> {
    parsed.diagnostics.iter().filter(|d| d.severity.as_str() == "error").map(|d| d.message.clone()).collect()
}

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
fn class_defined_display_headings_are_heading_blocks_with_the_class_skips() {
    let main = "\\documentclass{article}\n\\usepackage{secstyle}\n\\begin{document}\nBefore.\n\\section{First heading}\nAfter.\n\\subsection{A subsection}\nText.\n\\section*{Unnumbered}\nMore.\n\\end{document}\n";
    let parsed = parse(main);
    assert_eq!(errors(&parsed), Vec::<String>::new());
    let blocks: Vec<String> = parsed
        .blocks
        .iter()
        .map(|block| match block {
            Block::Paragraph(inlines) => format!("para {}", text(inlines)),
            Block::Heading { level, number, content, .. } => {
                let bold = content.iter().all(|i| matches!(i, Inline::Text { style, .. } if style.bold));
                format!("heading L{level} [{number}] {} bold={bold}", text(content))
            }
            // The class's skips are smaller than article's at every
            // level, so every delta is negative; the magnitudes are
            // checked against the probe's pdflatex reference
            // (`fixtures/divergence-probes/min-startsection`), not here:
            // the engine's `\the\@tempskipa` for `2.0ex` written in a
            // package's macro body currently comes out about 0.65x of
            // cmr10's ex (5.6pt, not 8.61pt) while `\hspace{2ex}` in the
            // body resolves correctly -- an open discrepancy this test
            // must not encode as expected.
            Block::VSpace { pt, .. } => format!("vspace {}", if *pt < 0.0 { "negative" } else { "non-negative" }),
            other => format!("{other:?}"),
        })
        .collect();
    // `\section`: `\addvspace{2.0ex}` is less than article's 3.5ex and
    // `\vskip 1.5ex` less than its 2.3ex; `\subsection`: 1.8ex vs 3.25ex
    // before, 0.8ex vs 1.5ex after. The starred form is unnumbered but
    // keeps the same skips.
    let expect = [
        "para Before.",
        "vspace negative",
        "heading L1 [1] First heading bold=true",
        "vspace negative",
        "para After.",
        "vspace negative",
        "heading L2 [1.1] A subsection bold=true",
        "vspace negative",
        "para Text.",
        "vspace negative",
        "heading L1 [] Unnumbered bold=true",
        "vspace negative",
        "para More.",
    ];
    assert_eq!(blocks, expect);
}

#[test]
fn the_style_argument_sets_the_title_face_and_size() {
    use flashtex_compiler::parser::FontSizeLevel;
    let main = "\\documentclass{article}\n\\usepackage{secstyle}\n\\begin{document}\n\\section{Head}\n\\subsection{Sub}\n\\end{document}\n";
    let parsed = parse(main);
    let sizes: Vec<(u8, Option<FontSizeLevel>, bool)> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Heading { level, content, .. } => content.iter().find_map(|i| match i {
                Inline::Text { style, .. } => Some((*level, style.size, style.bold)),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    // `\large\bf` and `\normalsize\bf`: the class's own sizes, not
    // article's `\Large`/`\large`.
    assert_eq!(sizes, [(1, Some(FontSizeLevel::Large1), true), (2, None, true)]);
}

#[test]
fn a_negative_after_skip_is_a_run_in_head_in_the_following_paragraph() {
    let main = "\\documentclass{article}\n\\usepackage{secstyle}\n\\begin{document}\nBefore.\n\\paragraph{Run-in head} continues here.\n\\end{document}\n";
    let parsed = parse(main);
    assert_eq!(errors(&parsed), Vec::<String>::new());
    let paragraphs: Vec<&Vec<Inline>> = parsed.blocks.iter().filter_map(|b| if let Block::Paragraph(p) = b { Some(p) } else { None }).collect();
    assert_eq!(paragraphs.len(), 2, "{:?}", parsed.blocks);
    let head = paragraphs[1];
    // `\@svsechd`: the bold title, then `\hskip -(-1em)` = 1em of the
    // heading font (cmr10's quad, 10pt), then the paragraph's own text.
    let words: Vec<(String, bool)> = head.iter().filter_map(|i| match i { Inline::Text { text, style, .. } => Some((text.clone(), style.bold)), _ => None }).collect();
    assert_eq!(words, [("Run-in".to_string(), true), ("head".to_string(), true), ("continues".to_string(), false), ("here.".to_string(), false)]);
    let skips: Vec<f64> = head.iter().filter_map(|i| match i { Inline::HSpace { pt, .. } => Some(*pt), _ => None }).collect();
    // cmr10's quad is 10.00002pt (TFM), which `\the` prints as such.
    assert_eq!(skips.len(), 1, "{skips:?}");
    assert!((skips[0] - 10.00002).abs() < 1e-4, "{skips:?}");
    assert!(!parsed.diagnostics.iter().any(|d| d.message.contains("@startsection")), "{:?}", parsed.diagnostics);
}

#[test]
fn secnumdepth_and_kernel_helpers_are_honoured() {
    // `\setcounter{secnumdepth}{0}`: `\ifnum 1>\c@secnumdepth` leaves the
    // section unnumbered but the counter is not stepped either (`\@sect`
    // skips `\refstepcounter`). `\secdef` and `\@dblarg` are the kernel's.
    let main = "\\documentclass{article}\n\\usepackage{secstyle}\n\\setcounter{secnumdepth}{0}\n\\begin{document}\n\\section{Head}\n\\end{document}\n";
    let parsed = parse(main);
    let numbers: Vec<String> = parsed.blocks.iter().filter_map(|b| if let Block::Heading { number, .. } = b { Some(number.clone()) } else { None }).collect();
    assert_eq!(numbers, [String::new()]);
}
