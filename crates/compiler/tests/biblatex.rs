//! Basic biblatex support through the project parser.

use flashtex_compiler::parser::{parse_project, Block, Inline, SourceDocument, TextStyle};

const BIB: &str = r#"
@article{beta,
  author = {Beta, Bob},
  title = {Beta article},
  journal = {Journal B},
  year = {2021},
}
@book{alpha,
  author = {Alpha, Alice},
  title = {Alpha book},
  publisher = {Press A},
  year = {2020},
}
"#;

fn parse_article(options: &str, body: &str) -> flashtex_compiler::parser::Parsed {
    parse_document("article", options, body, BIB)
}

fn parse_document(
    class: &str,
    options: &str,
    body: &str,
    bib: &str,
) -> flashtex_compiler::parser::Parsed {
    let main = format!(
        "\\documentclass{{{class}}}\\usepackage[{options}]{{biblatex}}\\addbibresource{{refs.bib}}\\begin{{document}}{body}\\printbibliography\\end{{document}}"
    );
    let documents = [
        SourceDocument {
            path: "main.tex",
            text: &main,
        },
        SourceDocument {
            path: "refs.bib",
            text: bib,
        },
    ];
    parse_project(&documents, "main.tex")
}

fn inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        if let Inline::Text {
            text: value,
            space_before,
            ..
        } = inline
        {
            if *space_before && !text.is_empty() {
                text.push(' ');
            }
            text.push_str(value);
        }
    }
    text
}

fn block_text(block: &Block) -> String {
    match block {
        Block::Paragraph(inlines)
        | Block::Heading {
            content: inlines, ..
        } => inline_text(inlines),
        Block::ListItem { label, content, .. } => {
            let mut text = label
                .as_ref()
                .map(|label| label.0.clone())
                .unwrap_or_default();
            text.push_str(&inline_text(content));
            text
        }
        _ => String::new(),
    }
}

fn all_text(parsed: &flashtex_compiler::parser::Parsed) -> String {
    parsed
        .blocks
        .iter()
        .map(block_text)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn list_items(parsed: &flashtex_compiler::parser::Parsed) -> Vec<(String, String)> {
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { label, content, .. } => Some((
                label
                    .as_ref()
                    .map(|label| label.0.clone())
                    .unwrap_or_default(),
                inline_text(content),
            )),
            _ => None,
        })
        .collect()
}

fn messages(parsed: &flashtex_compiler::parser::Parsed) -> Vec<&str> {
    parsed
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect()
}

#[test]
fn numeric_citations_notes_textcite_and_autocite() {
    let parsed = parse_article(
        "style=numeric,sorting=none,backend=biber",
        r"See \cite[p.~3]{beta}; \parencite{alpha}; \textcite{alpha}; \autocite[p.~5]{beta}.",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", messages(&parsed));
    let text = all_text(&parsed);
    assert!(text.contains("[1]"), "{text}");
    assert!(text.contains("[2]"), "{text}");
    assert!(text.contains("Alpha [2]"), "{text}");
    assert!(text.contains("[1, p. 3]"), "{text}");
    assert!(text.contains("[1, p. 5]"), "{text}");
}

#[test]
fn citeauthor_and_citeyear_use_bib_fields() {
    let parsed = parse_article(
        "style=numeric,sorting=none,backend=biber",
        r"\citeauthor{beta}; \citeyear{alpha}.",
    );
    let body = parsed.blocks.iter().find_map(|block| match block {
        Block::Paragraph(inlines) => Some(inline_text(inlines)),
        _ => None,
    });
    assert_eq!(body.as_deref(), Some("Bob Beta; 2020."));
    assert!(parsed.diagnostics.is_empty(), "{:?}", messages(&parsed));
}

#[test]
fn authoryear_is_minimal_and_alphabetic_warns() {
    let authoryear = parse_article(
        "style=authoryear,sorting=none,backend=biber",
        r"\textcite{alpha}; \parencite{beta}.",
    );
    let body = authoryear.blocks.iter().find_map(|block| match block {
        Block::Paragraph(inlines) => Some(inline_text(inlines)),
        _ => None,
    });
    let body = body.unwrap_or_default();
    assert!(body.contains("Alpha") && body.contains("(2020)"), "{body}");
    assert!(body.contains("(Bob") && body.contains("2021)"), "{body}");
    assert!(
        authoryear.diagnostics.is_empty(),
        "{:?}",
        messages(&authoryear)
    );

    let alphabetic = parse_article(
        "style=alphabetic,sorting=none,backend=biber",
        r"\cite{beta}.",
    );
    assert!(messages(&alphabetic).iter().any(
        |message| message.contains("style=alphabetic") && message.contains("not yet supported")
    ));
}

#[test]
fn nocite_star_prints_every_resource_entry() {
    let parsed = parse_article(
        "style=numeric,sorting=none,backend=biber",
        r"Uncited. \nocite{*}",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", messages(&parsed));
    let items = list_items(&parsed);
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0].0, "[1]");
    assert_eq!(items[1].0, "[2]");
    assert!(items[0].1.contains("Beta article"), "{items:?}");
    assert!(items[1].1.contains("Alpha book"), "{items:?}");
}

#[test]
fn sorting_none_keeps_resource_order_and_nyt_sorts_by_author() {
    let none = parse_article("style=numeric,sorting=none,backend=biber", r"\nocite{*}");
    let nyt = parse_article("style=numeric,sorting=nyt,backend=biber", r"\nocite{*}");
    let none_items = list_items(&none);
    let nyt_items = list_items(&nyt);
    assert_eq!(none_items[0].0, "[1]");
    assert!(none_items[0].1.contains("Beta article"), "{none_items:?}");
    assert!(
        nyt_items[0].0 == "[1]" && nyt_items[0].1.contains("Alpha book"),
        "{nyt_items:?}"
    );
    assert!(
        nyt_items[1].0 == "[2]" && nyt_items[1].1.contains("Beta article"),
        "{nyt_items:?}"
    );
}

#[test]
fn undefined_citation_is_warned_and_key_is_bold() {
    let parsed = parse_article(
        "style=numeric,sorting=none,backend=biber",
        r"Missing \cite{missing}.",
    );
    assert!(messages(&parsed)
        .iter()
        .any(|message| message.contains("undefined") && message.contains("missing")));
    assert!(parsed.blocks.iter().any(|block| match block {
        Block::Paragraph(inlines) => inlines.iter().any(|inline| matches!(
            inline,
            Inline::Text { text, style, .. }
                if text == "missing" && *style == TextStyle::BOLD
        )),
        _ => false,
    }));
}

#[test]
fn printbibliography_heading_depends_on_document_class_and_accepts_title() {
    for (class, expected) in [
        ("article", "References"),
        ("report", "Bibliography"),
        ("book", "Bibliography"),
    ] {
        let main = format!(
            "\\documentclass{{{class}}}\\usepackage[style=numeric]{{biblatex}}\\addbibresource{{refs.bib}}\\begin{{document}}\\printbibliography[title={{Sources}}]\\end{{document}}"
        );
        let documents = [
            SourceDocument {
                path: "main.tex",
                text: &main,
            },
            SourceDocument {
                path: "refs.bib",
                text: BIB,
            },
        ];
        let parsed = parse_project(&documents, "main.tex");
        assert!(parsed.diagnostics.is_empty(), "{:?}", messages(&parsed));
        let heading = parsed.blocks.iter().find_map(|block| match block {
            Block::Heading { content, .. } => Some(inline_text(content)),
            _ => None,
        });
        assert_eq!(heading.as_deref(), Some("Sources"), "{class}");

        let default_main = format!(
            "\\documentclass{{{class}}}\\usepackage[style=numeric]{{biblatex}}\\addbibresource{{refs.bib}}\\begin{{document}}\\printbibliography\\end{{document}}"
        );
        let default_documents = [
            SourceDocument {
                path: "main.tex",
                text: &default_main,
            },
            SourceDocument {
                path: "refs.bib",
                text: BIB,
            },
        ];
        let default_parsed = parse_project(&default_documents, "main.tex");
        let default_heading = default_parsed.blocks.iter().find_map(|block| match block {
            Block::Heading { content, .. } => Some(inline_text(content)),
            _ => None,
        });
        assert_eq!(default_heading.as_deref(), Some(expected), "{class}");
    }
}
