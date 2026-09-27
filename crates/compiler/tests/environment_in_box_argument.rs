//! A whole environment inside a box argument is ordinary argument material:
//! `\mbox{\begin{tikzpicture}...\end{tikzpicture}}` and
//! `\colorbox{c}{\begin{tabular}...\end{tabular}}` compile in pdflatex
//! (MacTeX 2026) with no error and set the environment inside the box. The
//! runaway-argument scan used to treat every non-math `\begin` as the end of
//! the argument's paragraph, so the box closed empty before the environment
//! ("argument to \mbox is missing its closing brace") and the real `}`
//! was reported as unmatched. A `\begin` whose `\end` is still open when the
//! argument closes stays a boundary (the runaway case).
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{xcolor}}\n\\usepackage{{tikz}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn brace_errors(src: &str) -> Vec<String> {
    parse(src)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .filter(|m| m.contains("missing its closing brace") || m.contains("unmatched '}'"))
        .collect()
}

fn first_paragraph(src: &str) -> Vec<Inline> {
    parse(src)
        .blocks
        .into_iter()
        .find_map(|b| match b {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("a paragraph")
}

fn words(inlines: &[Inline]) -> Vec<String> {
    let mut out = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, .. } => out.push(text.clone()),
            Inline::HBox(b) => out.push(format!("[{}]", words(&b.content).join(" "))),
            Inline::ColorBox(b) => out.push(format!("[{}]", words(&b.content).join(" "))),
            _ => {}
        }
    }
    out
}

#[test]
fn a_tikzpicture_inside_mbox_and_colorbox_stays_inside_the_box() {
    for body in [
        r"Cc \mbox{\begin{tikzpicture}\draw (0,0) rectangle (1,1);\end{tikzpicture}} Dd.",
        r"Cc \colorbox{yellow}{\begin{tikzpicture}\draw (0,0) rectangle (1,1);\end{tikzpicture}} Dd.",
        // Nested environments of the same name, and one of another name.
        r"Cc \mbox{\begin{tikzpicture}\node {\begin{tikzpicture}\draw (0,0) -- (1,0);\end{tikzpicture}};\end{tikzpicture}} Dd.",
        r"Cc \mbox{\begin{tabular}{c}\begin{tikzpicture}\draw (0,0) -- (1,0);\end{tikzpicture}\end{tabular}} Dd.",
    ] {
        let src = doc(body);
        assert_eq!(brace_errors(&src), Vec::<String>::new(), "{body}");
        let got = words(&first_paragraph(&src));
        // The box opens after `Cc` and `Dd.` follows it, outside the box.
        assert_eq!(got.first().map(String::as_str), Some("Cc"), "{body}: {got:?}");
        assert_eq!(got.last().map(String::as_str), Some("Dd."), "{body}: {got:?}");
        assert!(got.len() >= 3 && got[1].starts_with('['), "{body}: {got:?}");
    }
}

#[test]
fn an_environment_still_open_when_the_argument_closes_is_still_a_runaway() {
    // No `}` before the environment's end: the argument still closes at the
    // `\begin` (the list is not swallowed into the bold group).
    let src = doc("Visible \\textbf{Tail \\begin{itemize}\\item One.\\end{itemize}\n\nNext.");
    assert!(
        brace_errors(&src).iter().any(|m| m.contains(r"argument to \textbf is missing its closing brace")),
        "{:?}",
        brace_errors(&src)
    );
}
