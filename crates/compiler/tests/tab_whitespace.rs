//! A literal TAB (U+0009) has catcode 10 in TeX -- it is whitespace, exactly
//! like a space, in both text and math mode. pdflatex silently treats it as
//! a space (the corpus oracle logs report nothing for tabs).
//!
//! Before the fix, a tab lexed as a printable "other" character, reached
//! shaping, and produced `has no glyph for '\t' (U+0009)` warnings (plus
//! `math_limitation` empty boxes for tabs in math mode downstream).

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

/// Messages of diagnostics that mention the tab character itself.
fn tab_diagnostics(source: &str) -> Vec<String> {
    compile(source)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .filter(|message| message.contains('\u{9}') || message.contains("U+0009"))
        .collect()
}

/// All laid-out text of the compiled pages, in order.
fn page_texts(output: &CompileOutput) -> Vec<String> {
    output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| item.text.clone())
        .collect()
}

fn assert_same_as_space(tabbed: &str, spaced: &str) {
    let tabbed_out = compile(tabbed);
    let spaced_out = compile(spaced);
    let tabbed_diags: Vec<_> = tabbed_out
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    let spaced_diags: Vec<_> = spaced_out
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.clone())
        .collect();
    assert_eq!(
        tabbed_diags, spaced_diags,
        "tab vs space diagnostics differ for {tabbed:?}"
    );
    assert_eq!(
        page_texts(&tabbed_out),
        page_texts(&spaced_out),
        "tab vs space laid-out text differs for {tabbed:?}"
    );
}

#[test]
fn tab_is_whitespace_in_text_mode() {
    assert_eq!(tab_diagnostics("a\tb"), Vec::<String>::new());
    assert_same_as_space("a\tb", "a b");
}

#[test]
fn tab_is_whitespace_in_math_mode() {
    assert_eq!(tab_diagnostics("$a\tb$"), Vec::<String>::new());
    assert_same_as_space("$a\tb$", "$a b$");
}

#[test]
fn tab_at_start_of_line_collapses_like_space() {
    assert_same_as_space("\ta", "a");
    assert_same_as_space("a\n\tb", "a\nb");
}

#[test]
fn run_of_tabs_collapses_to_one_space() {
    assert_same_as_space("a\t\t\tb", "a b");
    assert_same_as_space("a\t \tb", "a b");
}

#[test]
fn tab_at_argument_edges_and_empty_input() {
    // Tabs at either edge of a group behave like spaces there.
    assert_same_as_space("{a\t}", "{a }");
    assert_same_as_space("{\ta}", "{ a}");
    // A document holding only a tab compiles like one holding only a space.
    assert_same_as_space("\t", " ");
    // A trailing tab is harmless: no tab diagnostic either way.
    assert_eq!(tab_diagnostics("a\t"), Vec::<String>::new());
}

/// The behaviour #839 is actually about: a line whose only content is
/// whitespace ends the paragraph, exactly as an empty line does.
///
/// The `assert_same_as_space` tests above cannot catch a regression here --
/// they only compare tab against space, so a change that broke paragraph
/// breaking for BOTH would keep every one of them green. This pins the
/// absolute behaviour against the pdflatex oracle, which puts "Para two."
/// on a new baseline for a tab-only line and for an empty line alike
/// (measured with `pdftotext -bbox`; a single space joins them instead).
#[test]
fn whitespace_only_line_ends_the_paragraph() {
    let empty_line = compile("Para one.\n\nPara two.\n");
    let tab_line = compile("Para one.\n\t\nPara two.\n");
    let space_run = compile("Para one.\n \nPara two.\n");

    assert_eq!(
        empty_line.blocks.len(),
        2,
        "control: an empty line must end the paragraph"
    );
    assert_eq!(
        tab_line.blocks.len(),
        empty_line.blocks.len(),
        "a tab-only line must end the paragraph exactly like an empty line"
    );
    assert_eq!(
        space_run.blocks.len(),
        empty_line.blocks.len(),
        "a space-only line must end the paragraph too"
    );
}
