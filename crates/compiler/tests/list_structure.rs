//! Structured list data on `Block::ListItem`/`Block::Styled`: enclosing
//! list frames, source labels (explicit, counter, symbol), enumitem keys
//! parsed as keys, `start`/`resume`, `description`, `verse` lines and
//! `quote` inside an item. See `src/parser/lists.rs` for the latex.ltx,
//! article.cls and enumitem.sty provenance.

use flashtex_compiler::math::Nucleus;
use flashtex_compiler::parser::{
    self, Block, CounterStyle, FontSizeLevel, Inline, ItemLabel, ListEnvironment, ListLength,
    ListOption, TextFamily,
};

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\n\\usepackage{{enumitem}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn items(source: &str) -> Vec<(Vec<ListEnvironment>, Option<ItemLabel>, String)> {
    let parsed = parser::parse(source);
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem {
                lists,
                item,
                content,
                ..
            } => Some((
                lists.iter().map(|f| f.environment).collect(),
                item.clone(),
                content
                    .iter()
                    .filter_map(|i| match i {
                        Inline::Text { text, .. } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            )),
            _ => None,
        })
        .collect()
}

fn label_texts(source: &str) -> Vec<String> {
    items(source)
        .into_iter()
        .filter_map(|(_, item, _)| item.map(|i| i.text().to_string()))
        .collect()
}

#[test]
fn nested_labels_follow_article_per_kind_depth() {
    let source = doc(
        "\\begin{enumerate}\\item A\\begin{enumerate}\\item B\\begin{itemize}\\item C\\begin{itemize}\\item D\\end{itemize}\\end{itemize}\\item E\\begin{enumerate}\\item F\\end{enumerate}\\end{enumerate}\\end{enumerate}",
    );
    assert_eq!(label_texts(&source), ["1.", "(a)", "•", "–", "(b)", "i."]);
    let all = items(&source);
    assert!(matches!(
        &all[1].1,
        Some(ItemLabel::Counter { value: 1, style: CounterStyle::Alph, prefix, suffix, .. }) if prefix == "(" && suffix == ")"
    ));
    assert!(
        matches!(&all[3].1, Some(ItemLabel::Symbol { bold: true, command, .. }) if command == "textendash")
    );
    assert_eq!(
        all[3].0,
        [
            ListEnvironment::Enumerate,
            ListEnvironment::Enumerate,
            ListEnvironment::Itemize,
            ListEnvironment::Itemize
        ]
    );
}

#[test]
fn four_itemize_levels_show_all_kernel_markers() {
    let source = doc(
        "\\begin{itemize}\\item A\\begin{itemize}\\item B\\begin{itemize}\\item C\\begin{itemize}\\item D\\end{itemize}\\end{itemize}\\end{itemize}\\end{itemize}",
    );
    assert_eq!(label_texts(&source), ["•", "–", "∗", "·"]);
    let all = items(&source);
    let markers = [
        ("textbullet", false),
        ("textendash", true),
        ("textasteriskcentered", false),
        ("textperiodcentered", false),
    ];
    assert_eq!(all.len(), markers.len());
    for ((_, item, _), (command, bold)) in all.iter().zip(markers) {
        assert!(
            matches!(&item, Some(ItemLabel::Symbol { command: c, bold: b, .. }) if c == command && *b == bold),
            "{item:?}"
        );
    }
}

#[test]
fn labelitem_commands_typeset_the_kernel_markers() {
    let source = doc("A\\labelitemi B\\labelitemii C\\labelitemiii D\\labelitemiv E");
    let parsed = parser::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut words: Vec<(String, bool)> = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(content) = block {
            for inline in content {
                if let Inline::Text { text, style, .. } = inline {
                    words.push((text.as_str().to_owned(), style.bold));
                }
            }
        }
    }
    assert_eq!(
        words.iter().map(|(text, _)| text.as_str()).collect::<Vec<_>>().join(" "),
        "A • B – C ∗ D · E"
    );
    // Level 2 keeps its `\bfseries`; the other markers are regular.
    assert_eq!(
        words.iter().map(|(_, bold)| *bold).collect::<Vec<_>>(),
        [false, false, false, true, false, false, false, false, false]
    );
}

#[test]
fn renewcommand_of_a_labelitem_redirects_later_uses() {
    let source = doc("\\renewcommand{\\labelitemii}{OK}A\\labelitemii B");
    let parsed = parser::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut words = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(content) = block {
            for inline in content {
                if let Inline::Text { text, .. } = inline {
                    words.push(text.as_str().to_owned());
                }
            }
        }
    }
    assert_eq!(words.join(" "), "A OK B");
}

#[test]
fn explicit_labels_are_content_and_do_not_step_the_counter() {
    let source = doc(
        "\\begin{enumerate}\\item[--] One\\item Two\\item [(x)] Three\\item Four\\end{enumerate}",
    );
    let all = items(&source);
    assert_eq!(label_texts(&source), ["–", "1.", "(x)", "2."]);
    assert!(matches!(&all[0].1, Some(ItemLabel::Explicit { .. })));
    assert_eq!(all[0].2, "One");
    assert_eq!(all[2].2, "Three");
}

#[test]
fn description_terms_are_bold_explicit_labels() {
    let source =
        doc("\\begin{description}\\item[Second label] Body text.\\item Bare\\end{description}");
    let parsed = parser::parse(&source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(|d| !d.message.contains("description")),
        "{:?}",
        parsed.diagnostics
    );
    let all = items(&source);
    match &all[0].1 {
        Some(ItemLabel::Explicit {
            content,
            text,
            span,
        }) => {
            assert_eq!(text, "Second label");
            assert!(content
                .iter()
                .all(|i| matches!(i, Inline::Text { style, .. } if style.bold)));
            assert_eq!(&source[span.start..span.end], "[Second label]");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(all[0].2, "Body text.");
    assert_eq!(all[1].1, Some(ItemLabel::Empty));
    assert_eq!(all[0].0, [ListEnvironment::Description]);
}

#[test]
fn enumitem_options_are_keys_not_label_text() {
    let source = doc("\\begin{enumerate}[noitemsep]\\item A\\item B\\end{enumerate}");
    assert_eq!(label_texts(&source), ["1.", "2."]);
    let parsed = parser::parse(&source);
    let Some(Block::ListItem { lists, .. }) = parsed
        .blocks
        .iter()
        .find(|b| matches!(b, Block::ListItem { .. }))
    else {
        panic!()
    };
    assert_eq!(lists[0].options, [ListOption::NoItemSep]);
}

#[test]
fn setlist_keys_apply_in_order_before_begin_keys() {
    let source = format!(
        "\\documentclass[10pt]{{article}}\n\\usepackage{{enumitem}}\n\\setlist{{nosep}}\n\\setlist[enumerate]{{leftmargin=*}}\n\\setlist[enumerate,2]{{label=\\roman*)}}\n\\begin{{document}}\n{}\n\\end{{document}}\n",
        "\\begin{itemize}\\item A\\end{itemize}\\begin{enumerate}[labelsep=1em]\\item B\\begin{enumerate}\\item C\\end{enumerate}\\end{enumerate}"
    );
    let parsed = parser::parse(&source);
    let frames: Vec<_> = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::ListItem { lists, .. } => Some(lists.last().unwrap().options.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(frames[0], [ListOption::NoSep]);
    assert_eq!(
        frames[1],
        [
            ListOption::NoSep,
            ListOption::LeftMargin(ListLength::Star),
            ListOption::LabelSep(ListLength::Pt(10.0))
        ]
    );
    assert_eq!(
        frames[2],
        [
            ListOption::NoSep,
            ListOption::LeftMargin(ListLength::Star),
            ListOption::Label("\\roman*)".into())
        ]
    );
    assert_eq!(label_texts(&source), ["•", "1.", "i)"]);
}

#[test]
fn start_and_resume_continue_the_counter() {
    let source = doc(
        "\\begin{enumerate}[start=3]\\item A\\item B\\end{enumerate}\nMiddle.\n\\begin{enumerate}[resume]\\item C\\item D\\end{enumerate}\\begin{enumerate}[label=(\\alph*), series=s]\\item E\\end{enumerate}\\begin{enumerate}[resume*=s]\\item F\\end{enumerate}",
    );
    assert_eq!(label_texts(&source), ["3.", "4.", "5.", "6.", "(a)", "(b)"]);
}

#[test]
fn label_star_prefixes_the_enclosing_label() {
    let source = doc("\\begin{enumerate}\\item A\\begin{enumerate}[label*=\\arabic*.]\\item B\\end{enumerate}\\end{enumerate}");
    assert_eq!(label_texts(&source), ["1.", "1.1."]);
}

#[test]
fn verse_lines_are_paragraphs_started_by_the_line_break() {
    let source = doc(
        "\\begin{verse}\nFirst line\\\\\nSecond line\\\\[3pt]\nThird\n\nNew stanza\n\\end{verse}",
    );
    let parsed = parser::parse(&source);
    let styled: Vec<_> = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Styled {
                lists,
                line_break_before,
                content,
                ..
            } => Some((
                lists.iter().map(|f| f.environment).collect::<Vec<_>>(),
                line_break_before.as_ref().map(|l| l.skip_pt),
                content.len(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(styled.len(), 4, "{:?}", parsed.blocks);
    assert!(styled
        .iter()
        .all(|(lists, _, _)| lists == &[ListEnvironment::Verse]));
    assert_eq!(
        styled.iter().map(|s| s.1).collect::<Vec<_>>(),
        [None, Some(None), Some(Some(3.0)), None]
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(|d| !d.message.contains("verse")),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn quote_inside_an_item_is_styled_with_both_frames() {
    let source = doc(
        "\\begin{itemize}\\item Short.\\begin{quote}Quoted.\\end{quote}\\item Next.\\end{itemize}",
    );
    let parsed = parser::parse(&source);
    let kinds: Vec<_> = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::ListItem { lists, .. } => Some(("item", lists.len())),
            Block::Styled { lists, .. } => Some(("styled", lists.len())),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, [("item", 1), ("styled", 2), ("item", 1)]);
}

#[test]
fn labelitem_command_survives_as_a_nested_explicit_label() {
    // `\item[\labelitemi]`: the restricted nested-content dispatcher used
    // for explicit `[...]` labels previously recognised only
    // `text_builtins::TEXT_SYMBOLS`, so a `\labelitem<i>` inside it
    // silently vanished instead of typesetting the marker.
    let source = doc("\\begin{itemize}\\item[\\labelitemi] A\\item[\\labelitemiii] B\\end{itemize}");
    assert_eq!(label_texts(&source), ["•", "∗"]);
}

#[test]
fn labelitem_command_resets_style_rather_than_inheriting_it() {
    // The article default for level 2 is `\normalfont\bfseries\textendash`:
    // the marker's own style, not whatever face happens to be active where
    // `\labelitemii` is written. `\itshape\labelitemii` must NOT come out
    // italic-and-bold.
    let source = doc("\\itshape\\labelitemii");
    let parsed = parser::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut found = false;
    for block in &parsed.blocks {
        if let Block::Paragraph(content) = block {
            for inline in content {
                if let Inline::Text { text, style, .. } = inline {
                    if text == "–" {
                        found = true;
                        assert!(style.bold, "level 2's marker keeps its own \\bfseries");
                        assert!(!style.italic, "the marker must reset \\itshape, not inherit it");
                    }
                }
            }
        }
    }
    assert!(found, "expected the \\labelitemii marker in the output");
}

#[test]
fn labelitem_command_keeps_size_and_colour_but_resets_face() {
    // `\labelitemfont` is `\normalfont` (article.cls:355-359): it resets
    // family, series and shape only, so an active size or colour survives
    // the marker while the surrounding face does not.
    let source = "\\documentclass[10pt]{article}\n\\usepackage{xcolor}\n\\begin{document}\n{\\Large\\labelitemi} {\\sffamily\\itshape\\color{red}\\labelitemii}\n\\end{document}\n";
    let parsed = parser::parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut markers = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(content) = block {
            for inline in content {
                if let Inline::Text { text, style, .. } = inline {
                    if text == "•" || text == "–" {
                        markers.push((text.clone(), *style));
                    }
                }
            }
        }
    }
    assert_eq!(markers.len(), 2, "{markers:?}");
    // `\Large` survives the reset; level 1 stays upright roman.
    assert_eq!(markers[0].1.size, Some(FontSizeLevel::Large2));
    assert!(!markers[0].1.bold);
    assert!(!markers[0].1.italic);
    assert_eq!(markers[0].1.family, TextFamily::Roman);
    // Level 2 keeps its own `\bfseries`, drops sans+italic, keeps red.
    assert!(markers[1].1.bold);
    assert!(!markers[1].1.italic);
    assert_eq!(markers[1].1.family, TextFamily::Roman);
    assert_eq!(markers[1].1.size, None);
    assert_eq!(
        markers[1].1.color.map(|c| c.fill_operator()),
        Some("1 0 0 rg".to_string())
    );
}

#[test]
fn renewcommand_of_a_labelitem_changes_the_itemize_default_too() {
    let source = doc("\\renewcommand{\\labelitemi}{X}\\begin{itemize}\\item A\\end{itemize}");
    assert_eq!(label_texts(&source), ["X"], "a redefined \\labelitemi should change itemize's own default marker too");
}

#[test]
fn renewcommand_of_a_deeper_labelitem_applies_only_at_that_nesting_level() {
    // The expansion pass captures all four levels at each `\begin{itemize}`;
    // the parser selects by its own `kind_depth`, so only the renewed level
    // changes.
    let source = doc("\\renewcommand{\\labelitemii}{Y}\\begin{itemize}\\item A\\begin{itemize}\\item B\\end{itemize}\\item C\\end{itemize}");
    assert_eq!(label_texts(&source), ["•", "Y", "•"]);
}

#[test]
fn labelitem_override_textbf_body_is_parsed_as_bold_content() {
    // The captured `\labelitem<i>` body is inline LaTeX, not plain text:
    // `\textbf{X}` must typeset bold `X`, not the literal `\textbf{X}`.
    let source = doc("\\renewcommand{\\labelitemi}{\\textbf{X}}\\begin{itemize}\\item A\\end{itemize}");
    let parsed = parser::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(label_texts(&source), ["X"]);
    let all = items(&source);
    match &all[0].1 {
        Some(ItemLabel::Explicit { content, text, .. }) => {
            assert_eq!(text, "X");
            assert!(
                matches!(&content[..], [Inline::Text { text, style, .. }] if text == "X" && style.bold),
                "{content:?}"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn labelitem_override_math_body_becomes_a_math_nucleus() {
    // `$\star$`: the marker is a math nucleus, not the literal `$\star$`
    // text. (`\star` itself is unsupported in math mode in this compiler
    // version; running text reports the same diagnostic, so the test pins
    // parity with running text rather than clean compilation.)
    let source = doc("\\renewcommand{\\labelitemi}{$\\star$}\\begin{itemize}\\item A\\end{itemize}");
    let parsed = parser::parse(&source);
    let running = parser::parse(&doc("A $\\star$ B"));
    assert_eq!(
        parsed.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
        running.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
        "the override body must diagnose exactly like the same source in running text"
    );
    assert_eq!(label_texts(&source), [""]);
    let all = items(&source);
    match &all[0].1 {
        Some(ItemLabel::Explicit { content, .. }) => {
            assert!(
                matches!(&content[..], [Inline::Math { list, .. }] if list.atoms.iter().any(|a| matches!(&a.nucleus, Nucleus::Symbol(s) if s == "\\star"))),
                "{content:?}"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn labelitem_override_textendash_body_gets_the_usual_diagnostic() {
    // `\textendash` is not a supported text-symbol command in this compiler
    // version (running text reports the same error), so the marker carries
    // the usual diagnostic instead of the literal `\textendash` text.
    let source = doc("\\renewcommand{\\labelitemi}{\\textendash}\\begin{itemize}\\item A\\end{itemize}");
    let parsed = parser::parse(&source);
    let running = parser::parse(&doc("A \\textendash B"));
    assert_eq!(
        parsed.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
        running.diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>(),
        "the override body must diagnose exactly like the same source in running text"
    );
    assert!(
        parsed.diagnostics.iter().any(|d| d.message.contains("\\textendash")),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(label_texts(&source), [""]);
    let all = items(&source);
    assert!(matches!(&all[0].1, Some(ItemLabel::Explicit { .. })), "{:?}", all[0].1);
}
