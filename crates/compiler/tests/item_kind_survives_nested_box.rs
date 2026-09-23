//! A `\uline`/`\sout`/`\underline`/`\colorbox` argument inside an `\item`
//! is parsed in a nested paragraph. That nested parse saved and restored
//! `pending_item_label` but not `pending_item`, so its own `flush_paragraph`
//! consumed the item's kind and the `ListItem` block reached layout with a
//! label text but `item: None` — the pipeline then set `\textbullet` as a
//! roman word 2.77 bp wide of pdflatex's (GH-924).

use flashtex_compiler::parser::{parse, Block, ItemLabel};

fn item_kinds(src: &str) -> Vec<Option<&'static str>> {
    let parsed = parse(src);
    parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::ListItem { item, .. } => Some(item.as_ref().map(|i| match i {
                ItemLabel::Symbol { .. } => "symbol",
                ItemLabel::Counter { .. } => "counter",
                ItemLabel::Explicit { .. } => "explicit",
                ItemLabel::Template { .. } => "template",
                ItemLabel::Empty { .. } => "empty",
            })),
            _ => None,
        })
        .collect()
}

#[test]
fn a_box_command_in_an_item_keeps_the_items_label_kind() {
    for body in [r"\uline{unit}", r"\sout{unit}", r"\underline{unit}", r"\colorbox{yellow}{unit}"] {
        let src = format!(
            "\\documentclass{{article}}\\usepackage[normalem]{{ulem}}\\usepackage{{xcolor}}\\begin{{document}}\\begin{{itemize}}\\item \"{body}\"---one\\item plain\\end{{itemize}}\\end{{document}}"
        );
        assert_eq!(item_kinds(&src), vec![Some("symbol"), Some("symbol")], "{body}");
    }
}

#[test]
fn an_explicit_label_survives_a_box_command_in_the_item() {
    let src = r"\documentclass{article}\usepackage[normalem]{ulem}\begin{document}\begin{itemize}\item[$\alpha$] \uline{x}\end{itemize}\end{document}";
    assert_eq!(item_kinds(src), vec![Some("explicit")]);
}
