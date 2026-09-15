//! Regression coverage for font-relative dimensions and parser-owned globals.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::{self, Font, LayoutConstraints};
use flashtex_compiler::parser::{parse, Block, Inline, SourceDocument};

fn document(class: &str, body: &str) -> String {
    format!("\\documentclass[{class}pt]{{article}}\\begin{{document}}{body}\\end{{document}}")
}

fn hspace(source: &str) -> (f64, f64, f64) {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::HSpace {
                    pt,
                    space_before_pt,
                    space_after_pt,
                    ..
                } => Some((*pt, *space_before_pt, *space_after_pt)),
                _ => None,
            }),
            _ => None,
        })
        .expect("the document has no hspace")
}

fn vspace(source: &str) -> f64 {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::VSpace { pt } => Some(*pt),
            _ => None,
        })
        .expect("the document has no vspace")
}

fn paragraph_text(source: &str) -> String {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    paragraph_text_from(&parsed)
}

fn paragraph_text_from(parsed: &flashtex_compiler::parser::Parsed) -> String {
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(
                inlines
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .expect("the document has no paragraph")
}

#[test]
fn em_ex_use_class_size_local_size_and_font_family() {
    // Measured with TeX Live 2026's `/Library/TeX/texbin/pdflatex` using
    // `\typeout{\the\fontdimen6\font}` and `\typeout{\the\fontdimen5\font}`:
    // body 10pt CMR = 10.00002pt/4.30554pt, `\small` = 9.24994pt/3.87498pt,
    // and `\Large` = 14.09984pt/6.2pt. FlashTeX's compiler layout uses its
    // active Core14 face, so its Times values are 10/4.5, 9/4.05, and
    // 14.4/6.48; Helvetica's x-height is 5.23pt at 10pt.
    let body = document("10", "A\\hspace{2em} B");
    let (pt, before, after) = hspace(&body);
    assert_eq!(pt, 20.0);
    assert_eq!(before, 0.0);
    assert!((after - 10.0 * 0.25).abs() < 0.01);
    let out = compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: &body,
        }],
        "main.tex",
        LayoutConstraints::default(),
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let items = &out.pages[0].items;
    let a = items.iter().find(|item| item.text == "A").expect("A");
    let b = items.iter().find(|item| item.text == "B").expect("B");
    let expected_delta = layout::text_width("A", 10.0, Font::TimesRoman)
        + 20.0
        + layout::word_space(10.0, Font::TimesRoman);
    assert!((b.x_pt - a.x_pt - expected_delta).abs() < 0.02);
    assert!((vspace(&document("10", "A\\vspace{1ex}B")) - 4.5).abs() < 0.001);

    assert_eq!(hspace(&document("10", "{\\small\\hspace{2em}} ")).0, 18.0);
    assert!((vspace(&document("10", "{\\small\\vspace{1ex}}")) - 4.05).abs() < 0.001);
    assert_eq!(hspace(&document("10", "{\\Large\\hspace{1em}} ")).0, 14.4);
    assert!((vspace(&document("10", "{\\Large\\vspace{1ex}}")) - 6.48).abs() < 0.001);
    assert!((vspace(&document("10", "{\\sffamily\\vspace{1ex}}")) - 5.23).abs() < 0.001);
    assert_eq!(hspace(&document("10", "\\textsf{\\hspace{1em}}")).0, 10.0);
}

#[test]
fn setlength_em_uses_the_document_class_size() {
    for (class, expected) in [("10", 30.0), ("11", 33.0), ("12", 36.0)] {
        let source = format!(
            "\\documentclass[{class}pt]{{article}}\\setlength{{\\parskip}}{{3em}}\\begin{{document}}x\\end{{document}}"
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.parskip_pt, Some(expected));
    }
}

#[test]
fn engine_length_registers_use_the_document_class_em() {
    for (class, expected) in [("10", 30.0), ("12", 36.0)] {
        let source = format!(
            "\\documentclass[{class}pt]{{article}}\\newlength{{\\x}}\\setlength{{\\x}}{{3em}}\\begin{{document}}\\the\\x\\end{{document}}"
        );
        assert_eq!(paragraph_text(&source), format!("{expected:.1}pt"));
    }
}

#[test]
fn local_length_restores_and_global_length_survives_group_end() {
    let local = parse(r"\documentclass{article}{\parskip=1pt}\begin{document}x\end{document}");
    assert!(local.diagnostics.is_empty(), "{:?}", local.diagnostics);
    assert_eq!(local.parskip_pt, None);

    let global =
        parse(r"\documentclass{article}{\global\parskip=1pt}\begin{document}x\end{document}");
    assert!(global.diagnostics.is_empty(), "{:?}", global.diagnostics);
    assert_eq!(global.parskip_pt, Some(1.0));

    let table = parse(
        r"\documentclass{article}{\tabcolsep=1pt}\global\tabcolsep=2pt\begin{document}x\end{document}",
    );
    assert!(table.diagnostics.is_empty(), "{:?}", table.diagnostics);
}

#[test]
fn expansion_register_global_survives_group_end() {
    let parsed = parse(
        r"\documentclass{article}\newcount\localcount\localcount=1{\advance\localcount by 2}\newcount\globalcount\globalcount=1{\global\advance\globalcount by 2}\begin{document}\the\localcount,\the\globalcount\end{document}",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(paragraph_text_from(&parsed), "1,3");
}

#[test]
fn global_setlength_remains_invalid() {
    // pdflatex reports `You can't use a prefix with '\\setlength'.`; the
    // expansion engine is responsible for retaining that invalid-prefix error.
    let parsed = parse(
        r"\documentclass{article}\global\setlength{\parskip}{1pt}\begin{document}x\end{document}",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("You can't use a prefix")),
        "{:?}",
        parsed.diagnostics
    );
}
