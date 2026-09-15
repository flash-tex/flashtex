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
    let source =
        format!("\\documentclass[11pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
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
                | flashtex_compiler::parser::Block::Styled { content: inlines, .. }
                | flashtex_compiler::parser::Block::ListItem { content: inlines, .. } => inlines,
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
        vec![
            ("Alpha".to_string(), None),
            ("Epsilon".to_string(), None),
        ]
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

#[test]
fn an_underline_argument_does_not_add_a_block_leading() {
    let source = "\\documentclass{article}\n\\begin{document}\n\\underline{under}\n\\end{document}\n";
    let parsed = parse(source);
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.block_par_leading.len(), 1);
}

#[test]
fn a_sout_argument_does_not_add_a_block_leading() {
    let source = "\\documentclass{article}\n\\usepackage{ulem}\n\\begin{document}\n\\sout{struck}\n\\end{document}\n";
    let parsed = parse(source);
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.block_par_leading.len(), 1);
}

#[test]
fn a_uline_argument_does_not_add_a_block_leading() {
    let source = "\\documentclass{article}\n\\usepackage{ulem}\n\\begin{document}\n\\uline{underlined}\n\\end{document}\n";
    let parsed = parse(source);
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.block_par_leading.len(), 1);
}

#[test]
fn a_list_inside_a_table_entry_does_not_add_a_block_leading() {
    let source = "\\documentclass{article}\n\\begin{document}\n\\begin{tabular}{c}\\begin{itemize}\\item x\\end{itemize}\\end{tabular}\n\\end{document}\n";
    let parsed = parse(source);
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.block_par_leading.len(), 1);
}

#[test]
fn the_letter_fixture_has_one_leading_per_block() {
    let parsed = parse(include_str!("../../../fixtures/real-world/letter/main.tex"));
    assert_eq!(parsed.blocks.len(), 13);
    assert_eq!(parsed.block_par_leading.len(), 13);
}

/// Parses `body` in an article that loads `packages` and checks that the
/// parser recorded exactly one `ParLeading` per block, returning the count.
fn blocks_and_leadings(packages: &str, body: &str) -> (usize, usize) {
    let source = format!(
        "\\documentclass{{article}}\n\\usepackage{{{packages}}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    );
    let parsed = parse(&source);
    (parsed.blocks.len(), parsed.block_par_leading.len())
}

/// `\colorbox`'s argument is parsed through `box_inlines`; its temporary
/// paragraph used to leave a leading behind in the outer vector.
#[test]
fn a_colorbox_argument_does_not_add_a_block_leading() {
    assert_eq!(blocks_and_leadings("xcolor", "A \\colorbox{yellow}{Highlighted}."), (1, 1));
}

#[test]
fn an_fcolorbox_argument_does_not_add_a_block_leading() {
    assert_eq!(blocks_and_leadings("xcolor", "A \\fcolorbox{red}{white}{Border}."), (1, 1));
}

/// Paragraph breaks inside a box argument must not leak either: each one
/// flushes a temporary paragraph of its own.
#[test]
fn a_paragraph_break_inside_a_box_argument_does_not_add_block_leadings() {
    let (blocks, leadings) =
        blocks_and_leadings("xcolor", "A \\colorbox{yellow}{a\\par b} \\underline{c\\par d}.\n\nZ.");
    assert_eq!(leadings, blocks);
}

/// `\rotatebox`/`\scalebox`/`\reflectbox`/`\resizebox` and `\footnote` parse
/// their argument through `argument_inlines`, which already truncated; this
/// guards the other sub-parse path with the same shape of input.
#[test]
fn transform_and_footnote_arguments_do_not_add_block_leadings() {
    let (blocks, leadings) = blocks_and_leadings(
        "graphicx",
        "\\rotatebox{30}{a\\par b}\\scalebox{1.5}[0.8]{c}\\reflectbox{d}\\resizebox{2cm}{!}{e\\par f}\\footnote{g\\par h}\n\nZ.",
    );
    assert_eq!(leadings, blocks);
}

/// PR #42's `extended/graphics-transform-color` case, verbatim: a debug build
/// of the render pipeline asserted 7 leadings for 5 blocks on it (the
/// `\colorbox` and `\fcolorbox` arguments each leaked one).
#[test]
fn the_graphics_transform_color_case_has_one_leading_per_block() {
    let source = "\\documentclass{article}\n\\listfiles\n\\usepackage{xcolor,graphicx}\n\n\\begin{document}\n\\definecolor{brand}{RGB}{20,80,170}\n\\textcolor{brand}{Blue text} \\colorbox{yellow}{Highlighted} \\fcolorbox{red}{white}{Border}.\n\\par\\medskip\\noindent\\rotatebox{30}{Rotated text}\\quad\\scalebox{1.5}[0.8]{Scaled}\\quad\\reflectbox{Mirror}.\n\\par\\medskip\\noindent\\begin{picture}(180,70)\\put(0,0){\\framebox(180,70){}}\\put(10,10){\\vector(2,1){100}}\\put(40,40){\\circle{30}}\\put(100,50){Label}\\end{picture}\n\\end{document}\n";
    let parsed = parse(source);
    assert_eq!(parsed.blocks.len(), 5);
    assert_eq!(parsed.block_par_leading.len(), 5);
}

/// Every `.tex` under `fixtures/real-world` and `fixtures/divergence-probes`
/// parses to exactly one `ParLeading` per block, so a future sub-parse that
/// forgets to truncate is caught on real documents.
#[test]
fn every_fixture_has_one_leading_per_block() {
    fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("fixture directory") {
            let path = entry.expect("fixture entry").path();
            if path.is_dir() {
                collect(&path, out);
            } else if path.extension().is_some_and(|e| e == "tex") {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let mut paths = Vec::new();
    collect(&root.join("real-world"), &mut paths);
    collect(&root.join("divergence-probes"), &mut paths);
    paths.sort();
    assert!(paths.len() > 50, "found only {} fixtures", paths.len());
    let mismatches: Vec<String> = paths
        .iter()
        .filter_map(|path| {
            let source = std::fs::read_to_string(path).expect("fixture source");
            let parsed = parse(&source);
            (parsed.blocks.len() != parsed.block_par_leading.len()).then(|| {
                format!(
                    "{}: {} blocks, {} leadings",
                    path.display(),
                    parsed.blocks.len(),
                    parsed.block_par_leading.len()
                )
            })
        })
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
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
