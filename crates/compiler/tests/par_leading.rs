//! `Parsed::block_par_leading`: which `\baselineskip` TeX would have read for
//! each paragraph.
//!
//! TeX reads `\baselineskip` in `append_to_vlist` (§679), once per line, from
//! `post_line_break` — i.e. at `\par` time. The leading of a paragraph is
//! therefore the value in force when the paragraph *ended*, not where its
//! words were typed. Every expectation below was measured with pdfTeX
//! 3.141592653-2.6-1.40.27 (TeX Live 2025) on `\documentclass[11pt]{article}`,
//! reading the baseline-to-baseline distance out of the shipped page (and
//! cross-checked with `\showbox`'s `\glue(\baselineskip)`); `size11.clo` gives
//! `\normalsize` 13.6 pt, `\small` 12 pt, `\footnotesize` 11 pt and `\large`
//! 14 pt.
use flashtex_compiler::parser::{parse, FontSizeLevel, ParLeading};

/// The `ParLeading` of every block that the parser recorded one for, paired
/// with the block's first word so the assertions read like the source.
fn leadings(body: &str) -> Vec<(String, ParLeading)> {
    let source = format!(
        "\\documentclass[11pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    );
    let parsed = parse(&source);
    assert_eq!(
        parsed.block_par_leading.len(),
        parsed.blocks.len(),
        "one ParLeading per block"
    );
    parsed
        .blocks
        .iter()
        .zip(&parsed.block_par_leading)
        .filter_map(|(block, leading)| {
            let inlines = match block {
                flashtex_compiler::parser::Block::Paragraph(inlines)
                | flashtex_compiler::parser::Block::Styled {
                    content: inlines, ..
                }
                | flashtex_compiler::parser::Block::ListItem {
                    content: inlines, ..
                } => inlines,
                _ => return None,
            };
            let first = inlines.iter().find_map(|i| match i {
                flashtex_compiler::parser::Inline::Text { text, .. } => Some(text.clone()),
                _ => None,
            })?;
            Some((first, *leading))
        })
        .collect()
}

/// `{\small ...}` whose `}` comes before the blank line keeps the *body's*
/// leading: the group restores `\baselineskip` before the `\par` that ends the
/// paragraph. pdflatex sets these lines 13.549 bp apart (13.6 pt) even though
/// every glyph on them is `\small`. Getting this wrong is the tempting bug —
/// "the whole paragraph is small, so use small's leading" — and it is what a
/// fix driven by the run sizes would produce.
#[test]
fn a_group_that_closes_before_the_paragraph_ends_does_not_change_the_leading() {
    assert_eq!(
        leadings("{\\small Alpha beta gamma delta.}\n\nEpsilon zeta."),
        vec![("Alpha".to_string(), None), ("Epsilon".to_string(), None),]
    );
}

/// `\par` inside the group: pdflatex sets these lines 11.955 bp apart (12 pt,
/// `\small` at an 11 pt base).
#[test]
fn a_par_inside_the_group_takes_the_declaration_s_leading() {
    assert_eq!(
        leadings("{\\small Alpha beta gamma.\\par}\n\nEpsilon zeta."),
        vec![
            ("Alpha".to_string(), Some(FontSizeLevel::Small)),
            ("Epsilon".to_string(), None),
        ]
    );
}

/// A mid-paragraph switch never moves the leading, in either direction: the
/// register is back to `\normalsize` long before `\par` runs.
#[test]
fn a_mid_paragraph_switch_leaves_the_leading_alone() {
    assert_eq!(
        leadings("Alpha {\\small beta} {\\Large gamma} delta."),
        vec![("Alpha".to_string(), None)]
    );
}

/// `\endtrivlist` runs `\ifhmode\unskip\par\fi` *before* `\end`'s `\endgroup`,
/// so a declaration opened inside the environment is still in force when the
/// last paragraph ends. pdflatex: 11.955 bp inside the `quote`, 13.549 bp for
/// the paragraph after it.
#[test]
fn an_environment_that_pars_before_it_closes_keeps_its_own_declaration() {
    assert_eq!(
        leadings("\\begin{quote}\\small Alpha beta gamma.\\end{quote}\n\nEpsilon zeta."),
        vec![
            ("Alpha".to_string(), Some(FontSizeLevel::Small)),
            ("Epsilon".to_string(), None),
        ]
    );
    assert_eq!(
        leadings("\\begin{center}\\large Alpha beta.\\end{center}\n\nEpsilon."),
        vec![
            ("Alpha".to_string(), Some(FontSizeLevel::Large1)),
            ("Epsilon.".to_string(), None),
        ]
    );
}

/// The same rule through `\list`: every `\item` paragraph ends inside the
/// declaration, and so does the last one, which `\endlist` pars.
#[test]
fn a_declaration_inside_a_list_reaches_every_item() {
    assert_eq!(
        leadings(
            "\\begin{itemize}\\footnotesize\n\\item Alpha beta.\n\\item Gamma delta.\n\\end{itemize}\n\nEpsilon."
        ),
        vec![
            ("Alpha".to_string(), Some(FontSizeLevel::FootnoteSize)),
            ("Gamma".to_string(), Some(FontSizeLevel::FootnoteSize)),
            ("Epsilon.".to_string(), None),
        ]
    );
}

/// A declaration that spans several paragraphs governs each of their `\par`s,
/// and `\normalsize` inside it puts the body leading back.
#[test]
fn a_declaration_spanning_paragraphs_governs_each_par_in_turn() {
    assert_eq!(
        leadings("{\\small Alpha beta.\n\nGamma delta.\n\n{\\normalsize Epsilon zeta.\\par}\\par}\n\nEta."),
        vec![
            ("Alpha".to_string(), Some(FontSizeLevel::Small)),
            ("Gamma".to_string(), Some(FontSizeLevel::Small)),
            ("Epsilon".to_string(), None),
            ("Eta.".to_string(), None),
        ]
    );
}

/// The two per-block vectors are pushed from one place, so they must stay in
/// step through every block kind — headings, `tabular`, `verbatim`, displays
/// and footnotes all push blocks without going through `flush_list_item`. If
/// they ever drift, one paragraph's leading would be read for another.
#[test]
fn every_block_kind_gets_exactly_one_entry() {
    let source = "\\documentclass[11pt]{article}\n\\begin{document}\n\
        \\section{Head}\nText.\n\n\
        \\begin{itemize}\\small\\item One.\\item Two.\\end{itemize}\n\n\
        \\begin{tabular}{ll}a & b \\\\ c & d\\end{tabular}\n\n\
        \\begin{verbatim}\nraw one\nraw two\n\\end{verbatim}\n\n\
        \\[ x = y \\]\n\n\
        \\begin{quote}\\footnotesize Quoted.\\end{quote}\n\n\
        Tail\\footnote{A note.} text.\n\
        \\end{document}\n";
    let parsed = parse(source);
    assert_eq!(
        parsed.block_par_leading.len(),
        parsed.blocks.len(),
        "block_par_leading drifted from blocks: {:?}",
        parsed.blocks.iter().map(block_kind).collect::<Vec<_>>()
    );
    let declared: Vec<(&str, ParLeading)> = parsed
        .blocks
        .iter()
        .zip(&parsed.block_par_leading)
        .filter(|(_, leading)| leading.is_some())
        .map(|(block, leading)| (block_kind(block), *leading))
        .collect();
    assert_eq!(
        declared,
        vec![
            ("ListItem", Some(FontSizeLevel::Small)),
            ("ListItem", Some(FontSizeLevel::Small)),
            ("Styled", Some(FontSizeLevel::FootnoteSize)),
        ]
    );
}

fn block_kind(block: &flashtex_compiler::parser::Block) -> &'static str {
    use flashtex_compiler::parser::Block as B;
    match block {
        B::Paragraph(..) => "Paragraph",
        B::Heading { .. } => "Heading",
        B::FigureCaption { .. } => "FigureCaption",
        B::Styled { .. } => "Styled",
        B::ListItem { .. } => "ListItem",
        _ => "other",
    }
}
