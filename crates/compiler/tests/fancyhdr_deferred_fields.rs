//! fancyhdr fields expand at shipout, not at `\fancyhead` (the 21-242
//! proof-practice fixture, `fixtures/real-world/proof-practice-21242`).
//!
//! fancyhdr.sty stores a field unexpanded and `\@outputpage` expands it when
//! the page ships. So `\fancyhead[R]{\topicshort}` may come before
//! `\newcommand{\topicshort}`, and a `\renewcommand{\topicshort}` after a
//! `\newpage` changes the head of the page that follows and of no earlier
//! one. Oracle: pdflatex, TeX Live 2026
//! (`/usr/local/texlive/2026/bin/universal-darwin/pdflatex`,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) on `REPRO` prints, by
//! `pdftotext` page:
//!
//! ```text
//! page 1: HEAD / First topic / Body one. / 1
//! page 2: HEAD / Second topic / Body two. / 2
//! page 3: HEAD / Second topic / Body three. / 3
//! ```
//!
//! and no diagnostic: `\topicshort` is defined by the time any page ships.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline};

const REPRO: &str = "\\documentclass{article}\n\
     \\usepackage{fancyhdr}\n\
     \\pagestyle{fancy}\\fancyhf{}\\fancyhead[L]{HEAD}\\fancyhead[R]{\\topicshort}\\fancyfoot[R]{\\thepage}\n\
     \\newcommand{\\topicshort}{First topic}\n\
     \\begin{document}\n\
     Body one.\n\
     \\newpage\n\
     \\renewcommand{\\topicshort}{Second topic}\n\
     Body two.\n\
     \\newpage\n\
     Body three.\n\
     \\end{document}\n";

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn words(out: &CompileOutput, page: usize) -> Vec<String> {
    out.pages[page]
        .items
        .iter()
        .filter(|item| !item.text.is_empty())
        .map(|item| item.text.clone())
        .collect()
}

fn text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn a_field_macro_defined_after_the_head_is_silent_and_takes_its_definition() {
    let parsed = parse(REPRO);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // The document's fields at `\begin{document}`: the macro's first
    // definition, although the `\fancyhead` came before the `\newcommand`.
    assert_eq!(text(&parsed.fancy.head[0]), "HEAD");
    assert_eq!(text(&parsed.fancy.head[2]), "First topic");
    assert!(matches!(parsed.fancy.foot[2].as_slice(), [Inline::ThePage { .. }]));
}

#[test]
fn a_body_renewcommand_leaves_one_marker_with_the_new_fields() {
    let parsed = parse(REPRO);
    let markers: Vec<&Inline> = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flatten()
        .filter(|inline| matches!(inline, Inline::FancyFields { .. }))
        .collect();
    assert_eq!(markers.len(), 1, "{markers:?}");
    let Inline::FancyFields { fields, span } = markers[0] else { unreachable!() };
    assert_eq!(text(&fields.head[0]), "HEAD");
    assert_eq!(text(&fields.head[2]), "Second topic");
    // The marker stands at the redefinition: after the first `\newpage`,
    // before `Body two.`.
    let renew = REPRO.find("\\renewcommand").unwrap();
    let body_two = REPRO.find("Body two.").unwrap();
    assert!(span.start >= renew && span.start < body_two, "{span:?}");
}

#[test]
fn each_page_ships_with_the_definition_in_force_there() {
    let out = compile(REPRO);
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(out.pages.len(), 3);
    assert_eq!(words(&out, 0), ["HEAD", "First", "topic", "Body", "one.", "1"]);
    assert_eq!(words(&out, 1), ["HEAD", "Second", "topic", "Body", "two.", "2"]);
    assert_eq!(words(&out, 2), ["HEAD", "Second", "topic", "Body", "three.", "3"]);
}

#[test]
fn an_undefined_field_macro_is_still_reported_once_the_body_starts() {
    // Never defined: pdflatex stops at the first shipout with
    // `! Undefined control sequence.`; here the unknown command is reported
    // once, at the field, and not in the preamble where fancyhdr only
    // stores it.
    let parsed = parse(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\\fancyhead[R]{\\nevermade}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    let unknown: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("nevermade"))
        .collect();
    assert_eq!(unknown.len(), 1, "{:?}", parsed.diagnostics);
}

#[test]
fn a_body_fancyhead_changes_the_fields_from_its_page_on() {
    let out = compile(
        "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\\fancyhf{}\\fancyhead[C]{Before}\n\
         \\begin{document}\n\
         One.\n\
         \\newpage\n\
         \\fancyhead[C]{After}\n\
         Two.\n\
         \\end{document}\n",
    );
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    assert_eq!(words(&out, 0), ["Before", "One."]);
    assert_eq!(words(&out, 1), ["After", "Two."]);
}

#[test]
fn without_fancyhdr_the_missing_package_is_still_reported() {
    let parsed = parse(
        "\\documentclass{article}\n\
         \\fancyhead[L]{X}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("needs \\usepackage{fancyhdr}")),
        "{:?}",
        parsed.diagnostics
    );
}
