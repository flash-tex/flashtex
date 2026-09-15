//! Kernel slice A: one observable-output test per previously untested
//! kernel command (audit `docs/dev/kernel-inventory-audit.md`, slice A).
//! Each test compiles a minimal document and asserts what the command DOES
//! to the output — never just that it compiles.

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};
use flashtex_compiler::layout::Font;
use flashtex_compiler::parser::{parse, Block, Inline};

fn messages(source: &str) -> Vec<String> {
    parse(source)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

fn blocks_debug(source: &str) -> String {
    format!("{:?}", parse(source).blocks)
}

/// Joined `Inline::Text` of every paragraph, in order (a run with
/// `space_before` contributes its separating space, as words would).
fn paragraphs(source: &str) -> Vec<String> {
    parse(source)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => {
                let mut out = String::new();
                for inline in inlines {
                    if let Inline::Text {
                        text, space_before, ..
                    } = inline
                    {
                        if *space_before && !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(text);
                    }
                }
                Some(out)
            }
            _ => None,
        })
        .collect()
}

/// `(text, space_before)` of the first paragraph's text runs.
fn first_paragraph_runs(source: &str) -> Vec<(String, bool)> {
    parse(source)
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(
                inlines
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Text {
                            text, space_before, ..
                        } => Some((text.clone(), *space_before)),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

/// Every laid-out word, in order.
fn page_texts(source: &str) -> Vec<String> {
    compile(source)
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
        .collect()
}

/// GH-ATBEGINDOC (issue #458): `\AtBeginDocument` hook text used to be
/// spliced before the `\begin{document}` marker and dropped as preamble
/// instead of being typeset ahead of the body. The engine emits the hook
/// ahead of the real `\begin{document}` re-emission (kernel-faithful); the
/// compiler holds hook output (marked by the host prelude) back and
/// re-emits it right after `\begin{document}` closes.
/// Minimal repro: `\AtBeginDocument{HOOKA}\begin{document}Body\end{document}`
/// typesets `["HOOKABody"]`, mirroring `\AtEndDocument`, whose test below
/// asserts the hook is appended (`["BodyTAILZ"]`).
#[test]
fn at_begin_document_hook_content_is_typeset_before_the_body() {
    let source = "\\AtBeginDocument{HOOKA}\\begin{document}Body\\end{document}";
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(paragraphs(source), ["HOOKABody"]);
}

#[test]
fn at_end_document_hook_content_is_typeset_after_the_body() {
    let source = "\\begin{document}Body\\AtEndDocument{TAILZ}\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["BodyTAILZ"]);
}

#[test]
fn at_begin_document_multiple_hooks_run_in_registration_order() {
    let source = "\\AtBeginDocument{A}\\AtBeginDocument{B}\\begin{document}C\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["ABC"]);
}

#[test]
fn atbegindoc_hook_uses_the_definition_current_at_begin_document() {
    let out = paragraphs(
        r"\newcommand{\deferred}{FIRST}\AtBeginDocument{\deferred}\renewcommand{\deferred}{SECOND}\begin{document}Body\end{document}",
    );
    assert_eq!(out, vec!["SECONDBody".to_string()]);
}

#[test]
fn atbegindoc_hook_can_use_a_macro_defined_after_registration() {
    let source = r"\AtBeginDocument{\late}\newcommand{\late}{LATE}\begin{document}Body\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), vec!["LATEBody".to_string()]);
}

#[test]
fn atbegindoc_multiple_hooks_defer_and_keep_registration_order() {
    let source = r"\newcommand{\myv}{1}\AtBeginDocument{A\myv}\AtBeginDocument{B\myv}\renewcommand{\myv}{2}\begin{document}C\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), vec!["A2B2C".to_string()]);
}

#[test]
fn at_begin_document_hook_with_a_space_keeps_word_separation() {
    let source = "\\AtBeginDocument{HOOK A}\\begin{document}Body\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["HOOK ABody"]);
}

#[test]
fn at_begin_and_end_document_hooks_combine() {
    let source =
        "\\AtBeginDocument{HOOKA}\\begin{document}Body\\AtEndDocument{TAILZ}\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["HOOKABodyTAILZ"]);
}

#[test]
fn at_begin_document_after_begin_runs_immediately() {
    let source = "\\begin{document}Body\\AtBeginDocument{LATE}\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["BodyLATE"]);
}

#[test]
fn ordinary_preamble_text_is_still_dropped_silently() {
    let source = "Preamble junk \\begin{document}Body\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["Body"]);
}

#[test]
fn declare_robust_command_defines_a_macro_that_expands() {
    let source = "\\DeclareRobustCommand{\\foo}{BAR}\\begin{document}\\foo\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["BAR"]);
    let args = "\\DeclareRobustCommand{\\greet}[1]{Hi #1}\\begin{document}\\greet{you}\\end{document}";
    assert_eq!(paragraphs(args), ["Hi you"]);
}

#[test]
fn addtocounter_adds_globally_including_inside_a_group() {
    let source = "\\newcounter{foo}{\\addtocounter{foo}{4}}\\addtocounter{foo}{-1}\\begin{document}\\arabic{foo}\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    // 0 + 4 (inside the group: global) - 1.
    assert_eq!(paragraphs(source), ["3"]);
}

#[test]
fn addtolength_grows_a_length_read_back_with_the() {
    let source = "\\newlength{\\mylen}\\mylen=5pt\\addtolength{\\mylen}{3pt}\\begin{document}\\the\\mylen\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["8.0pt"]);
}

#[test]
fn bibliographystyle_warns_it_has_no_effect_and_keeps_the_body() {
    let source = "\\begin{document}Body\\bibliographystyle{plain}\\end{document}";
    assert_eq!(
        messages(source),
        ["\\bibliographystyle has no effect without BibTeX/biblatex .bib support"]
    );
    let blocks = blocks_debug(source);
    assert!(blocks.contains("\"Body\""), "{blocks}");
    assert!(!blocks.contains("plain"), "style argument leaked as text: {blocks}");
}

#[test]
fn bigskip_ends_the_paragraph_with_twelve_points_of_space() {
    let source = "\\begin{document}One\\bigskip Two\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert!(blocks_debug(source).contains("VSpace { pt: 12.0 }"), "{}", blocks_debug(source));
    // The space is real in layout: bigskip gaps the baselines 6pt more than medskip.
    let gap = |command: &str| {
        let out = compile(&format!("\\begin{{document}}One{command} Two\\end{{document}}"));
        let y = |word: &str| {
            out.pages[0]
                .items
                .iter()
                .find(|item| item.text == word)
                .map(|item| item.baseline_y_pt)
                .unwrap()
        };
        y("Two") - y("One")
    };
    assert!((gap("\\bigskip") - gap("\\medskip") - 6.0).abs() < 0.01);
}

#[test]
fn columnwidth_names_the_measure_as_a_rule_width() {
    let source = "\\begin{document}\\rule{\\columnwidth}{4pt}\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert!(blocks_debug(source).contains("unit: ColumnWidth"), "{}", blocks_debug(source));
    let out = compile(source);
    let rule = out.pages[0]
        .items
        .iter()
        .find(|item| item.rule.is_some())
        .expect("rule item");
    // Resolved against the real measure, like an explicit width would be.
    assert_eq!(rule.rule.unwrap().width_pt, LayoutConstraints::default().measure_pt);
    assert_eq!(rule.rule.unwrap().height_pt, 4.0);
}

#[test]
fn displaystyle_is_accepted_and_leaves_its_neighbours_alone() {
    let source = "\\begin{document}$a\\displaystyle b$\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    let blocks = blocks_debug(source);
    assert!(blocks.contains("Symbol(\"a\")") && blocks.contains("Symbol(\"b\")"), "{blocks}");
    assert!(blocks.contains("Space { em: 0.0"), "displaystyle must add no visible atom: {blocks}");
}

#[test]
fn fboxrule_sets_the_fcolorbox_frame_thickness() {
    let with = "\\usepackage{xcolor}\\setlength{\\fboxrule}{2pt}\\begin{document}\\fcolorbox{red}{yellow}{x}\\end{document}";
    assert!(messages(with).is_empty(), "{:?}", messages(with));
    assert!(blocks_debug(with).contains("fboxrule_pt: 2.0"), "{}", blocks_debug(with));
    let default = "\\usepackage{xcolor}\\begin{document}\\fcolorbox{red}{yellow}{x}\\end{document}";
    assert!(blocks_debug(default).contains("fboxrule_pt: 0.4"), "{}", blocks_debug(default));
}

#[test]
fn hsize_is_a_text_width_relative_length_in_tabular_widths() {
    // A single-column `tabular*` stretches nothing observable into layout,
    // so use a fill table: `\extracolsep{\fill}` pushes the trailing
    // column's right edge out to the resolved target width, and its `x`
    // pins that width in the laid-out result.
    let table = |width: &str| {
        format!(
            "\\begin{{document}}\\begin{{tabular*}}{{{width}}}{{@{{\\extracolsep{{\\fill}}}}lr}}A & B\\end{{tabular*}}\\end{{document}}"
        )
    };
    let trailing_x = |width: &str| {
        let source = table(width);
        assert!(messages(&source).is_empty(), "{:?}", messages(&source));
        compile(&source).pages[0]
            .items
            .iter()
            .find(|item| item.text == "B")
            .map(|item| item.x_pt)
            .expect("trailing cell")
    };
    let from_hsize = trailing_x("0.5\\hsize");
    // `\hsize` resolves exactly like `\textwidth` ...
    assert_eq!(from_hsize, trailing_x("0.5\\textwidth"));
    // ... and to half the text measure in laid-out points.
    let half_measure_pt = format!("{}pt", 0.5 * LayoutConstraints::default().measure_pt);
    assert_eq!(from_hsize, trailing_x(&half_measure_pt));
}

#[test]
fn ignorespaces_eats_following_spaces_mid_line() {
    let source = "\\begin{document}A\\ignorespaces   B\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(
        first_paragraph_runs(source),
        [("A".to_string(), false), ("B".to_string(), false)]
    );
    // Without it the word space survives.
    assert_eq!(
        first_paragraph_runs("\\begin{document}A   B\\end{document}"),
        [("A".to_string(), false), ("B".to_string(), true)]
    );
}

#[test]
fn jobname_expands_to_texput() {
    let source = "\\begin{document}\\jobname\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["texput"]);
}

#[test]
fn makeatother_restores_other_catcode_for_at() {
    let back_to_other = "\\makeatletter\\makeatother\\begin{document}\\the\\catcode64\\end{document}";
    assert!(messages(back_to_other).is_empty(), "{:?}", messages(back_to_other));
    assert_eq!(paragraphs(back_to_other), ["12"]);
    let still_letter = "\\makeatletter\\begin{document}\\the\\catcode64\\end{document}";
    assert_eq!(paragraphs(still_letter), ["11"]);
}

#[test]
fn mathnormal_keeps_its_content_as_math() {
    let source = "\\begin{document}$\\mathnormal{AB}$\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    let blocks = blocks_debug(source);
    assert!(blocks.contains("Symbol(\"A\")") && blocks.contains("Symbol(\"B\")"), "{blocks}");
}

#[test]
fn mdseries_switches_back_to_medium_weight() {
    let out = compile("\\begin{document}{\\bfseries bold \\mdseries plain}\\end{document}");
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let font_of = |word: &str| {
        out.pages[0]
            .items
            .iter()
            .find(|item| item.text == word)
            .map(|item| item.font)
            .unwrap()
    };
    assert_eq!(font_of("bold"), Font::TimesBold);
    assert_eq!(font_of("plain"), Font::TimesRoman);
}

#[test]
fn medskip_ends_the_paragraph_with_six_points_of_space() {
    let source = "\\begin{document}One\\medskip Two\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert!(blocks_debug(source).contains("VSpace { pt: 6.0 }"), "{}", blocks_debug(source));
}

#[test]
fn newcounter_allocates_a_zero_counter_printed_as_arabic() {
    let source = "\\newcounter{foo}\\begin{document}\\thefoo\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["0"]);
    let dup = "\\newcounter{foo}\\newcounter{foo}\\begin{document}A\\end{document}";
    assert!(
        messages(dup).iter().any(|m| m.contains("Command \\c@foo already defined")),
        "{:?}",
        messages(dup)
    );
    // `[within]` allocation works and starts at zero too.
    let within = "\\newcounter{parent}\\newcounter{child}[parent]\\begin{document}\\thechild\\end{document}";
    assert!(messages(within).is_empty(), "{:?}", messages(within));
    assert_eq!(paragraphs(within), ["0"]);
}

#[test]
fn newcounter_within_kernel_counter_compiles_and_counts() {
    // GH-NEWCOUNTER-WITHIN: `section` (like every class counter) is a valid
    // `[within]` parent without an explicit `\newcounter{section}`.
    let source = "\\section{First}\\newcounter{c}[section]\\stepcounter{c}\\arabic{c}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["1"]);
    // Another kernel counter as the `[within]` target.
    let sub = "\\newcounter{d}[subsection]\\stepcounter{d}\\arabic{d}";
    assert!(messages(sub).is_empty(), "{:?}", messages(sub));
    assert_eq!(paragraphs(sub), ["1"]);
}

#[test]
fn newlength_allocates_a_skip_register_set_and_read_in_pt() {
    let source = "\\newlength{\\mylen}\\mylen=7pt\\begin{document}\\the\\mylen\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["7.0pt"]);
    let dup = "\\newlength{\\mylen}\\newlength{\\mylen}\\begin{document}A\\end{document}";
    assert!(
        messages(dup).iter().any(|m| m.contains("Command \\mylen already defined")),
        "{:?}",
        messages(dup)
    );
}

#[test]
fn noindent_is_a_silent_no_op() {
    let source = "\\begin{document}\\noindent hello\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["hello"]);
    assert_eq!(page_texts(source), page_texts("\\begin{document}hello\\end{document}"));
}

#[test]
fn protected_keeps_a_macro_unexpanded_inside_edef() {
    let source = "\\protected\\def\\foo{BAR}\\edef\\baz{\\foo}\\begin{document}\\meaning\\baz\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    assert_eq!(paragraphs(source), ["macro:->\\foo"]);
    // Without the prefix the same `\edef` expands fully.
    let plain = "\\def\\foo{BAR}\\edef\\baz{\\foo}\\begin{document}\\meaning\\baz\\end{document}";
    assert_eq!(paragraphs(plain), ["macro:->BAR"]);
}
