//! Structured list data on `Block::ListItem`/`Block::Styled`: enclosing
//! list frames, source labels (explicit, counter, symbol), enumitem keys
//! parsed as keys, `start`/`resume`, `description`, `verse` lines and
//! `quote` inside an item. See `src/parser/lists.rs` for the latex.ltx,
//! article.cls and enumitem.sty provenance.

use flashtex_compiler::parser::{
    self, Block, CounterStyle, Inline, ItemLabel, ListEnvironment, ListLength, ListOption,
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
fn text_glued_after_explicit_item_label_is_kept() {
    // `\item[<label>]` never reaches `optional_bracket_argument`: `\item`
    // dispatch calls `item_label_argument`, which has its own tail-rewrite
    // for `[x]TEXT`-style glued text. This pins that behavior next to the
    // other explicit-label coverage.
    let source = doc("\\begin{itemize}\\item[x]TEXT\\end{itemize}");
    let all = items(&source);
    assert_eq!(all.len(), 1, "{:?}", all);
    assert!(matches!(&all[0].1, Some(ItemLabel::Explicit { .. })));
    assert_eq!(label_texts(&source), ["x"]);
    assert_eq!(all[0].2, "TEXT", "text glued after `]` is still typeset");
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
