//! The 21-242 proof-practice fixture's constructs other than its fancyhdr
//! head (`fancyhdr_deferred_fields.rs`) and its fixed-height minipages
//! (`fixtures/real-world/proof-practice-21242`). Oracle: pdflatex, TeX Live
//! 2026, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`; the geometry these
//! records produce is measured on the exact route in
//! `crates/render-pipeline/tests/fancyhdr_chrome.rs`.

use flashtex_compiler::parser::{parse, Block, FontSizeLevel, Inline};

fn words(blocks: &[Block]) -> Vec<String> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flatten()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// `\ruled{7}` as the fixture defines it.
const RULED: &str = "\\documentclass[11pt]{article}\n\
\\usepackage{xcolor}\n\
\\setlength{\\parindent}{0pt}\\setlength{\\parskip}{6pt}\n\
\\newcommand{\\ruled}[1]{\\par\\begingroup\\parskip=0pt\\baselineskip=17pt\\count255=0\\loop\\ifnum\\count255<#1\\noindent\\textcolor{black!17}{\\rule{\\linewidth}{.25pt}}\\par\\advance\\count255 by1\\repeat\\endgroup}\n\
\\begin{document}\n\
Before.\n\
\\ruled{7}\n\
After.\n\
\\end{document}\n";

#[test]
fn ruled_group_assignments_set_the_rules_leading_and_parskip_and_end_with_the_group() {
    let parsed = parse(RULED);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // Before, seven rule paragraphs, After.
    let rules: Vec<usize> = parsed
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| matches!(block, Block::Paragraph(inlines) if inlines.iter().any(|i| matches!(i, Inline::Rule { .. }))))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(rules.len(), 7, "{:?}", parsed.blocks);
    for &i in &rules {
        // `\baselineskip=17pt` at 11pt: `\fontsize{10.95}{17}` as far as
        // the leading goes.
        match parsed.block_par_leading[i] {
            Some(FontSizeLevel::Explicit(size)) => assert_eq!(size.baselineskip_sp, 17 * 65536, "{size:?}"),
            other => panic!("rule paragraph {i}: {other:?}"),
        }
        assert_eq!(parsed.block_par_starts[i].parskip_sp, Some((0, 0, 0)));
    }
    // `\endgroup` restores both: `After.` is an ordinary paragraph.
    let after = parsed.blocks.len() - 1;
    assert_eq!(parsed.block_par_leading[after], None);
    assert_eq!(parsed.block_par_starts[after].parskip_sp, None);
    assert_eq!(words(&parsed.blocks), ["Before.", "After."]);
}

#[test]
fn a_size_declaration_after_a_baselineskip_assignment_takes_its_own_leading() {
    // `\small`'s `\selectfont` sets `\baselineskip` from `\f@baselineskip`.
    let parsed = parse(
        "\\documentclass[11pt]{article}\n\\begin{document}\n\
         {\\baselineskip=17pt \\small Text.\\par}\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.block_par_leading.last().copied().flatten(), Some(FontSizeLevel::Small));
}

#[test]
fn pdfbookmark_typesets_nothing_and_needs_hyperref() {
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{hyperref}\n\\begin{document}\n\
         \\pdfbookmark[1]{1. Fields, axioms}{topic1}\n\
         {\\large Heading}\\par\n\
         \\currentpdfbookmark{Two}{two}\\subpdfbookmark{Three}{three}\\belowpdfbookmark{Four}{four}Text.\n\
         \\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(words(&parsed.blocks), ["Heading", "Text."]);

    let parsed = parse("\\documentclass{article}\n\\begin{document}\n\\pdfbookmark{A}{a}Text.\n\\end{document}\n");
    assert!(
        parsed.diagnostics.iter().any(|d| d.message == "\\pdfbookmark needs \\usepackage{hyperref}"),
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(words(&parsed.blocks), ["Text."]);
}

#[test]
fn mathtools_loads_silently_and_its_gaps_report_where_they_are_used() {
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{amsmath,mathtools}\n\\begin{document}\n\
         $a \\coloneqq b$\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // A non-default option still changes output and still warns.
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage[showonlyrefs]{mathtools}\n\\begin{document}\nx\n\\end{document}\n",
    );
    assert!(
        parsed.diagnostics.iter().any(|d| d.message.contains("mathtools are recognised but not implemented")),
        "{:?}",
        parsed.diagnostics
    );
    // An unimplemented command is diagnosed at its own span.
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{mathtools}\n\\begin{document}\n$\\prescript{a}{b}{X}$\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.iter().any(|d| d.message.contains("prescript")), "{:?}", parsed.diagnostics);
}

#[test]
fn preamble_parskip_keeps_its_glue_on_the_old_path_and_only_the_body_records_one() {
    let parsed = parse(
        "\\documentclass{article}\\setlength{\\parskip}{6pt plus 2pt}\\begin{document}A\n\nB\\end{document}",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.parskip_pt, Some(6.0));
    assert!(parsed.block_par_starts.iter().all(|s| s.parskip_sp.is_none()), "{:?}", parsed.block_par_starts);
    // A fragment (no document environment) never leaves the preamble.
    let parsed = parse("\\setlength{\\parskip}{6pt plus 2pt}A\n\nB");
    assert!(parsed.block_par_starts.iter().all(|s| s.parskip_sp.is_none()), "{:?}", parsed.block_par_starts);
    // A body assignment keeps its stretch and shrink.
    let parsed = parse(
        "\\documentclass{article}\\begin{document}\\setlength{\\parskip}{6pt plus 2pt minus 1pt}A\n\nB\\end{document}",
    );
    assert_eq!(parsed.block_par_starts.last().and_then(|s| s.parskip_sp), Some((6 * 65536, 2 * 65536, 65536)));
}

#[test]
fn a_size_declaration_resets_a_baselineskip_assignment_even_back_to_its_size() {
    // `\small\normalsize` runs `\@setfontsize` twice: `\normalsize`'s own
    // 13.6pt, not the 17pt assigned before it.
    let parsed = parse(
        "\\documentclass[11pt]{article}\n\\begin{document}\n\
         {\\baselineskip=17pt \\small\\normalsize Text.\\par}\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.block_par_leading.last().copied().flatten(), None);
}

#[test]
fn a_parskip_assignment_inside_a_group_counts_for_the_paragraph_it_starts() {
    // `{\parskip=12pt Second}\par`: TeX appends `\parskip` when the
    // paragraph starts, inside the group; the `\par` after the group
    // does not change it.
    let parsed = parse(
        "\\documentclass{article}\\begin{document}First.\n\n{\\parskip=12pt Second.}\\par\nThird.\\end{document}",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let starts: Vec<_> = parsed.block_par_starts.iter().map(|s| s.parskip_sp).collect();
    assert_eq!(starts, [None, Some((12 * 65536, 0, 0)), None]);
}

#[test]
fn fancyhdr_position_letters_are_case_insensitive() {
    // fancyhdr lowercases the bracket (`\f@nch@forc`): `[l]`, `[ce]` and
    // `[LE,ro]` are the same as their uppercase spellings.
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\\fancyhf{}\n\
         \\fancyhead[l]{Left}\\fancyhead[ce,co]{Centre}\\fancyfoot[LE,ro]{Foot}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let text = |inlines: &[Inline]| -> String {
        inlines
            .iter()
            .filter_map(|i| match i {
                Inline::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    };
    assert_eq!(text(&parsed.fancy.head[0]), "Left");
    assert_eq!(text(&parsed.fancy.head[1]), "Centre");
    assert_eq!(text(&parsed.fancy.foot[2]), "Foot");
}

#[test]
fn counter_references_in_fields_wait_for_the_page() {
    // `\thesection` and `\arabic{section}` are read when the page ships:
    // the fields carry placeholders the page chrome fills
    // (`parser::FANCY_COUNTER`), not the engine's value at `\begin{document}`.
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\\fancyhf{}\n\
         \\fancyhead[R]{Section \\thesection}\\fancyhead[L]{No. \\arabic{section}}\n\
         \\begin{document}\n\\section{One}\nText.\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let texts = |inlines: &[Inline]| -> Vec<String> {
        inlines
            .iter()
            .filter_map(|i| match i {
                Inline::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    };
    let c = flashtex_compiler::parser::FANCY_COUNTER;
    assert_eq!(texts(&parsed.fancy.head[2]), ["Section".to_string(), format!("{c}the:section{c}")]);
    assert_eq!(texts(&parsed.fancy.head[0]), ["No.".to_string(), format!("{c}arabic:section{c}")]);
}

#[test]
fn thepage_in_a_field_carries_the_style_in_force() {
    // `\small\thepage`: pdflatex sets the number in cmr9 (render-pipeline's
    // fancyhdr_chrome.rs pins the positions).
    let parsed = parse(
        "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\\fancyhf{}\\fancyfoot[C]{\\small\\thepage}\n\\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    match parsed.fancy.foot[1].as_slice() {
        [Inline::Text { text, style, .. }] => {
            assert!(text.contains("the:page"), "{text:?}");
            assert_eq!(style.size, Some(FontSizeLevel::Small));
        }
        other => panic!("{other:?}"),
    }
}
