//! An enumitem `label=` value is TeX, not a string to print.
//!
//! enumitem.sty stores the key's value and `\makelabel` typesets it for every
//! `\item` (`\enit@setlabel`, with `\arabic*`-style counters replaced by the
//! item's value), exactly like the argument of `\item[...]`. Before this,
//! every label containing a command was printed as its source:
//! `[label=$\square$]` put the roman word `\square` in front of each item and
//! `[label=\textbf{\arabic*.}]` put `\textbf{1.}` there (the real-world
//! fixture `enumitem-worksheet`, pdflatex: `□`).

use flashtex_compiler::parser::{self, Block, Inline, ItemLabel, Parsed};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass[11pt]{{article}}\n\\usepackage{{amssymb}}\n\\usepackage{{enumitem}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn labels(parsed: &Parsed) -> Vec<ItemLabel> {
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { item, .. } => item.clone(),
            _ => None,
        })
        .collect()
}

fn kinds(label: &ItemLabel) -> Vec<&'static str> {
    let ItemLabel::Explicit { content, .. } = label else {
        panic!("expected typeset label content, got {label:?}");
    };
    content
        .iter()
        .map(|inline| match inline {
            Inline::Text { .. } => "Text",
            Inline::Math { .. } => "Math",
            _ => "other",
        })
        .collect()
}

fn bold_text(label: &ItemLabel) -> bool {
    let ItemLabel::Explicit { content, .. } = label else {
        return false;
    };
    content
        .iter()
        .any(|inline| matches!(inline, Inline::Text { style, .. } if style.bold))
}

#[test]
fn math_and_symbol_labels_are_typeset_not_printed() {
    let parsed = parser::parse(&doc(concat!(
        "\\begin{itemize}[label=$\\square$]\n\\item one\n\\item two\n\\end{itemize}\n",
        "\\begin{itemize}[label=\\textbullet]\n\\item three\n\\end{itemize}\n",
        "\\begin{itemize}[label=$\\triangleright$]\n\\item four\n\\end{itemize}\n",
    )));
    let labels = labels(&parsed);
    assert_eq!(labels.len(), 4);
    for label in &labels {
        assert!(
            !label.text().contains('\\'),
            "label printed as source: {label:?}"
        );
    }
    assert_eq!(kinds(&labels[0]), ["Math"]);
    assert_eq!(labels[0].text(), "\u{25a1}");
    assert_eq!(labels[1].text(), labels[0].text());
    assert_eq!(labels[2].text(), "\u{2022}");
    assert_eq!(kinds(&labels[3]), ["Math"]);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn a_styled_counter_label_keeps_its_counter_and_style() {
    let parsed = parser::parse(&doc(concat!(
        "\\begin{enumerate}[label=\\textbf{\\arabic*.}]\n\\item a\\label{first}\n\\item b\\label{second}\n\\end{enumerate}\n",
        "\\begin{enumerate}[label=\\emph{Case \\roman*}:]\n\\item c\n\\item d\n\\end{enumerate}\n",
        "See \\ref{second}.\n",
    )));
    let labels = labels(&parsed);
    let texts: Vec<&str> = labels.iter().map(ItemLabel::text).collect();
    assert_eq!(texts, ["1.", "2.", "Case i:", "Case ii:"]);
    assert!(bold_text(&labels[0]) && bold_text(&labels[1]));
    // `\ref` resolves to the label's text, not to `\textbf{2.}`.
    let reference = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::ListItem { content, .. } | Block::Paragraph(content) => {
                content.iter().find_map(|inline| match inline {
                    Inline::Label { key, value, .. } if key == "second" => Some(value.clone()),
                    _ => None,
                })
            }
            _ => None,
        })
        .expect("\\label{second} recorded");
    assert_eq!(reference, "2.");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn setlist_labels_and_plain_templates() {
    let parsed = parser::parse(&doc(concat!(
        "\\setlist[itemize,1]{label=$\\circ$}\n",
        "\\begin{itemize}\n\\item a\n\\end{itemize}\n",
        // No command: still the plain template, and still counted.
        "\\begin{enumerate}[label=(\\alph*)]\n\\item b\n\\item c\n\\end{enumerate}\n",
        "\\begin{itemize}[label=--]\n\\item d\n\\end{itemize}\n",
    )));
    let labels = labels(&parsed);
    assert_eq!(kinds(&labels[0]), ["Math"]);
    assert_eq!(labels[0].text(), "\u{2218}");
    assert!(
        matches!(labels[1], ItemLabel::Counter { .. }),
        "{:?}",
        labels[1]
    );
    assert_eq!(labels[2].text(), "(b)");
    assert!(
        matches!(&labels[3], ItemLabel::Template { text } if text == "\u{2013}"),
        "{:?}",
        labels[3]
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}
