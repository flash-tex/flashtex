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
    // `\thepage` is a counter placeholder carrying the style in force.
    assert_eq!(text(&parsed.fancy.foot[2]), format!("{0}the:page{0}", flashtex_compiler::parser::FANCY_COUNTER));
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

fn markers(parsed: &flashtex_compiler::parser::Parsed) -> Vec<Box<flashtex_compiler::parser::FancyHdr>> {
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flatten()
        .filter_map(|inline| match inline {
            Inline::FancyFields { fields, .. } => Some(fields.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_field_follows_a_macro_it_reaches_through_another_one() {
    // pdflatex: page 1 `Topic: First`, page 2 `Topic: Second`.
    let src = "\\documentclass{article}\n\\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\\fancyhf{}\\fancyhead[R]{\\myhead}\n\
         \\newcommand{\\topicshort}{First}\\newcommand{\\myhead}{Topic: \\topicshort}\n\
         \\begin{document}\nOne.\n\\newpage\n\\renewcommand{\\topicshort}{Second}\nTwo.\n\\end{document}\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(text(&parsed.fancy.head[2]), "Topic: First");
    let markers = markers(&parsed);
    assert_eq!(markers.len(), 1, "{markers:?}");
    assert_eq!(text(&markers[0].head[2]), "Topic: Second");
    let out = compile(src);
    assert_eq!(words(&out, 0), ["Topic:", "First", "One."]);
    assert_eq!(words(&out, 1), ["Topic:", "Second", "Two."]);
}

#[test]
fn a_field_command_in_a_group_ends_with_the_group() {
    // pdflatex: page 1 (shipped inside the group) `Inner`, pages 2 and 3
    // `Outer`: fancyhdr's field definitions are local.
    let src = "\\documentclass{article}\n\\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\\fancyhf{}\\fancyhead[C]{Outer}\n\
         \\begin{document}\n{\\fancyhead[C]{Inner}One.\\newpage}\nTwo.\n\\newpage\nThree.\n\\end{document}\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // The group's `\fancyhead` runs before any material, so it is the
    // body's starting fields; its end restores `Outer` with a marker.
    assert_eq!(text(&parsed.fancy.head[1]), "Inner");
    let heads: Vec<String> = markers(&parsed).iter().map(|m| text(&m.head[1])).collect();
    assert_eq!(heads, ["Outer"]);
    let out = compile(src);
    assert_eq!(words(&out, 0), ["Inner", "One."]);
    assert_eq!(words(&out, 1), ["Outer", "Two."]);
    assert_eq!(words(&out, 2), ["Outer", "Three."]);
}

#[test]
fn a_document_level_redefinition_does_not_fire_again_at_the_end() {
    // The document body is one group in LaTeX; a redefinition at its top
    // level lasts through `\end{document}`, so it leaves one marker.
    let parsed = parse(REPRO);
    assert_eq!(markers(&parsed).len(), 1);
}

#[test]
fn fancyhdr_defaults_are_the_marks_and_a_centred_page_number() {
    // fancyhdr.sty `\f@nch@initialise`, one-sided: `\fancyhead[l]{\slshape
    // \rightmark}`, `\fancyhead[r]{\slshape\leftmark}`, `\fancyfoot[c]
    // {\rmfamily\thepage}`. pdflatex prints the centred page number under a
    // bare `\pagestyle{fancy}`.
    let src = "\\documentclass{article}\n\\usepackage{fancyhdr}\n\\pagestyle{fancy}\n\
         \\begin{document}\nText.\n\\end{document}\n";
    let parsed = parse(src);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    // `\rmfamily\thepage`: the page-number placeholder, upright roman.
    match parsed.fancy.foot[1].as_slice() {
        [Inline::Text { text, style, .. }] => {
            assert_eq!(*text, format!("{0}the:page{0}", flashtex_compiler::parser::FANCY_COUNTER));
            assert!(!style.slanted && !style.bold, "{style:?}");
        }
        other => panic!("{other:?}"),
    }
    let marks: Vec<(String, bool)> = [&parsed.fancy.head[0], &parsed.fancy.head[2]]
        .iter()
        .flat_map(|field| field.iter())
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } => Some((text.clone(), style.slanted)),
            _ => None,
        })
        .collect();
    assert_eq!(
        marks,
        [
            (flashtex_compiler::parser::FANCY_RIGHT_MARK.to_string(), true),
            (flashtex_compiler::parser::FANCY_LEFT_MARK.to_string(), true)
        ]
    );
    let out = compile(src);
    assert_eq!(words(&out, 0), ["Text.", "1"]);
}
