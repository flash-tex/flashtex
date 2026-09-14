//! siunitx `S`/`s` table columns: the `S[<options>]` specification is one
//! centred column (siunitx loads array.sty), numeric entries are typeset as
//! `\num` would set them, and braced entries stay text.
use flashtex_compiler::parser::{parse, Block, Inline};
use flashtex_compiler::tabular::{Align, Entry, Tabular};

fn doc(packages: &str, body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n{packages}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn first_tabular(source: &str) -> (Tabular, Vec<String>) {
    let parsed = parse(source);
    let diagnostics = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    let tabular = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::Tabular(tabular) => Some((**tabular).clone()),
                _ => None,
            }),
            _ => None,
        })
        .expect("a tabular");
    (tabular, diagnostics)
}

/// Each row's cells as `M` (a formula) or `T` (anything else).
fn kinds(tabular: &Tabular) -> Vec<String> {
    tabular
        .entries
        .iter()
        .filter_map(|entry| match entry {
            Entry::Row(row) => Some(
                row.cells
                    .iter()
                    .map(|cell| {
                        if cell
                            .content
                            .iter()
                            .any(|i| matches!(i, Inline::Math { .. }))
                        {
                            'M'
                        } else {
                            'T'
                        }
                    })
                    .collect(),
            ),
            _ => None,
        })
        .collect()
}

#[test]
fn table_format_columns_parse_as_centred_number_columns() {
    let (tabular, diagnostics) = first_tabular(&doc(
        "\\usepackage{siunitx}",
        "\\begin{tabular}{l S[table-format=1.3] S[table-format=3.1]}\n{Batch} & {Mean} & {Max} \\\\\nA1 & 0.842 & 142.3 \\\\\n\\end{tabular}",
    ));
    assert!(
        diagnostics
            .iter()
            .all(|m| !m.contains("column specification")),
        "{diagnostics:?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|m| m.contains("decimal marker"))
            .count(),
        1,
        "{diagnostics:?}"
    );
    assert!(
        diagnostics.iter().all(|m| !m.contains("table-format")),
        "{diagnostics:?}"
    );
    let aligns: Vec<Align> = tabular.columns.iter().map(|c| c.align).collect();
    assert_eq!(aligns, vec![Align::Left, Align::Center, Align::Center]);
    assert!(tabular.array_package);
    assert_eq!(kinds(&tabular), vec!["TTT", "TMM"]);
}

#[test]
fn braced_blank_and_non_numeric_entries_stay_text() {
    let (tabular, diagnostics) = first_tabular(&doc(
        "\\usepackage{siunitx}",
        "\\begin{tabular}{S S}\n{12} & -- \\\\\n & \\textbf{1.5} \\\\\n3.5 \\pm 0.2 & 1e3 \\\\\n\\end{tabular}",
    ));
    assert_eq!(kinds(&tabular), vec!["TT", "TT", "MM"], "{diagnostics:?}");
}

#[test]
fn number_options_reach_the_formatter_and_table_keys_do_not() {
    let (_, diagnostics) = first_tabular(&doc(
        "\\usepackage{siunitx}",
        "\\begin{tabular}{S[table-format=1.2,output-decimal-marker={,},unknown-key=1]}\n1.25 \\\\\n\\end{tabular}",
    ));
    let not_implemented: Vec<&String> = diagnostics
        .iter()
        .filter(|m| m.contains("is not implemented"))
        .collect();
    assert!(
        not_implemented.iter().any(|m| m.contains("unknown-key")),
        "{diagnostics:?}"
    );
    assert!(
        not_implemented
            .iter()
            .all(|m| !m.contains("table-format") && !m.contains("output-decimal-marker")),
        "{diagnostics:?}"
    );
}

#[test]
fn unit_columns_typeset_units() {
    let (tabular, diagnostics) = first_tabular(&doc(
        "\\usepackage{siunitx}",
        "\\begin{tabular}{l s}\nmass & \\kilo\\gram \\\\\n\\end{tabular}",
    ));
    assert!(
        diagnostics
            .iter()
            .all(|m| !m.contains("column specification")),
        "{diagnostics:?}"
    );
    assert_eq!(kinds(&tabular), vec!["TM"]);
}

#[test]
fn without_siunitx_s_is_still_an_illegal_column_character() {
    let (_, diagnostics) = first_tabular(&doc(
        "",
        "\\begin{tabular}{l S}\nA & 1.0 \\\\\n\\end{tabular}",
    ));
    assert!(
        diagnostics
            .iter()
            .any(|m| m == "illegal character 'S' in the column specification"),
        "{diagnostics:?}"
    );
}
