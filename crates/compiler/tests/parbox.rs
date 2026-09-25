//! Kernel `\parbox[pos]{width}{text}` is `minipage`'s one-line form.
//!
//! latex.ltx defines `\parbox` through `\@parbox` (`[pos][height][inner-pos]`
//! then `{width}{text}`) and the `minipage` environment through
//! `\@iiiminipage` (the same arguments, as `\begin{minipage}[pos]{width}`).
//! pdflatex sets the same words for both forms (TeX Live 2026: the two
//! one-page PDFs' extracted text is byte-identical, 15523 bytes each), so
//! this compiler must too: the width is the whole line here and `[pos]`
//! only moves the box vertically, so the text is ordinary paragraph
//! material — unlike `\mbox`, which is one unbreakable `\hbox`.
use flashtex_compiler::parser::{parse, Block, Inline};
use flashtex_compiler::vocabulary::{is_known_command, is_listed_as_unimplemented};

fn document(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn messages(source: &str) -> Vec<String> {
    parse(source)
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect()
}

/// Every paragraph's text words, in order.
fn paragraph_words(source: &str) -> Vec<String> {
    let mut words = Vec::new();
    for block in &parse(source).blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { text, .. } = inline {
                    words.push(text.clone());
                }
            }
        }
    }
    words
}

fn paragraph(source: &str) -> Vec<Inline> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut inlines = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(body) = block {
            inlines.extend(body.clone());
        }
    }
    inlines
}

#[test]
fn parbox_is_implemented_so_it_is_known_without_the_unimplemented_list() {
    // `\parbox` has a real dispatch arm (`"parbox" => self.parbox_command(..)`)
    // and a `BUILT_INS` entry, so it stays known for typo suggestions even
    // though it is no longer listed as unimplemented.
    assert!(is_known_command("parbox"));
    // `is_known_command` alone would still pass here even if the
    // `KNOWN_UNIMPLEMENTED_COMMANDS` removal were reverted, since `parbox`
    // is unconditionally known via `BUILT_INS` regardless. Assert the
    // removal directly.
    assert!(
        !is_listed_as_unimplemented("parbox"),
        "parbox must not be listed as unimplemented once it has a real dispatch arm"
    );
    let parsed = parse(&document("\\parbox{5cm}{A paragraph of text.}"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn parbox_sets_its_text_with_no_diagnostic() {
    let source = document("\\parbox{5cm}{A paragraph of text that should wrap at 5cm.}");
    assert!(messages(&source).is_empty(), "{:?}", messages(&source));
    assert_eq!(
        paragraph_words(&source),
        [
            "A",
            "paragraph",
            "of",
            "text",
            "that",
            "should",
            "wrap",
            "at",
            "5cm."
        ],
    );
}

/// `\parbox[pos]{width}{text}` renders the same words as
/// `\begin{minipage}[pos]{width}text\end{minipage}`: the position and the
/// width are the box's own arguments on both forms, never page text.
#[test]
fn parbox_matches_minipage_word_for_word() {
    let text = "A paragraph of text.";
    for pos in ["", "[c]", "[t]", "[b]"] {
        let boxed = document(&format!("\\parbox{pos}{{5cm}}{{{text}}}"));
        let environed = document(&format!(
            "\\begin{{minipage}}{pos}{{5cm}}{text}\\end{{minipage}}"
        ));
        assert!(messages(&boxed).is_empty(), "{pos}: {:?}", messages(&boxed));
        assert_eq!(
            paragraph_words(&boxed),
            paragraph_words(&environed),
            "pos {pos:?}: parbox and minipage differ"
        );
        // Neither form leaks its own arguments onto the page.
        assert_eq!(
            paragraph_words(&boxed),
            ["A", "paragraph", "of", "text."],
            "pos {pos:?}: an argument leaked into the output"
        );
    }
}

/// The full `\@parbox` arity (`[pos][height][inner-pos]{width}{text}`)
/// reads every argument past, like pdflatex does.
#[test]
fn parbox_reads_height_and_inner_pos_past() {
    let source = document("\\parbox[c][10cm][s]{5cm}{Kept.}");
    assert!(messages(&source).is_empty(), "{:?}", messages(&source));
    assert_eq!(paragraph_words(&source), ["Kept."]);
}

/// A paragraph box breaks: its words are ordinary paragraph text, not one
/// unbreakable `\hbox` the way `\mbox` is.
#[test]
fn parbox_content_is_breakable_paragraph_text() {
    let inlines = paragraph(&document("Before \\parbox{5cm}{in box} after"));
    assert!(
        !inlines.iter().any(|i| matches!(i, Inline::HBox(_))),
        "{inlines:?}"
    );
    let words: Vec<&str> = inlines
        .iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(words, ["Before", "in", "box", "after"]);
    // The splice joins the paragraph: the space before `\parbox` separates
    // "Before" from the box's first word.
    let joined = inlines
        .iter()
        .find_map(|i| match i {
            Inline::Text {
                text, space_before, ..
            } if text == "in" => Some(*space_before),
            _ => None,
        })
        .expect("the box's first word");
    assert!(joined, "{inlines:?}");
}
