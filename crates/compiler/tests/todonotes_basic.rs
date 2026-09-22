//! todonotes `\todo{text}` (basic slice): the plain form is a margin note
//! through the shared `\marginpar` machinery (`Inline::Marginpar`), so the
//! render pipeline positions it exactly like an equivalent `\marginpar`.
//!
//! Out of scope here: todonotes options (`\todo[color=...]{...}`),
//! `\listoftodos` and `\missingfigure`, which stay diagnosed/unrecognised.
use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{todonotes}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn bare_doc(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn margin_notes(blocks: &[Block]) -> Vec<&Vec<Inline>> {
    blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flat_map(|inlines| {
            inlines.iter().filter_map(|i| match i {
                Inline::Marginpar { text, .. } => Some(text),
                _ => None,
            })
        })
        .collect()
}

fn note_words(text: &[Inline]) -> Vec<String> {
    text.iter()
        .filter_map(|i| match i {
            Inline::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Acceptance: `\todo{note}` produces the same margin-note inline as an
/// equivalent `\marginpar{note}` (same `Inline::Marginpar` content, no
/// mark in the running text), so the shared adapter/typeset path places
/// both identically.
#[test]
fn todo_note_matches_marginpar_note() {
    let todo = parse(&doc("Text\\todo{note} more."));
    assert!(
        todo.diagnostics.is_empty(),
        "{:?}",
        todo.diagnostics
    );
    let marginpar = parse(&doc("Text\\marginpar{note} more."));
    assert!(
        marginpar.diagnostics.is_empty(),
        "{:?}",
        marginpar.diagnostics
    );
    let todo_notes = margin_notes(&todo.blocks);
    let marginpar_notes = margin_notes(&marginpar.blocks);
    assert_eq!(todo_notes.len(), 1, "{:?}", todo.blocks);
    assert_eq!(marginpar_notes.len(), 1, "{:?}", marginpar.blocks);
    assert_eq!(
        note_words(todo_notes[0]),
        note_words(marginpar_notes[0]),
        "todo and marginpar notes must carry the same text"
    );
    assert_eq!(note_words(todo_notes[0]), vec!["note".to_string()]);
    // The note text stays out of the running prose in both cases.
    for (name, parsed) in [("todo", &todo), ("marginpar", &marginpar)] {
        let prose: String = parsed
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph(inlines) => Some(inlines),
                _ => None,
            })
            .flat_map(|inlines| {
                inlines.iter().filter_map(|i| match i {
                    Inline::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            !prose.contains("note"),
            "{name}: note leaked into prose: {prose:?}"
        );
    }
}

/// Without todonotes, `\todo` diagnoses the missing package (like soul's
/// `\so`/`\hl`) and keeps the argument as plain text; it is a known name,
/// never an unknown-command typo.
#[test]
fn todo_without_todonotes_diagnoses_and_keeps_text() {
    let source = bare_doc("Text \\todo{note} here.");
    let parsed = parse(&source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\todo needs \\usepackage{todonotes}")),
        "missing-package diagnostic: {:?}",
        parsed.diagnostics
    );
    assert!(
        !parsed
            .diagnostics
            .iter()
            .any(|d| d.code == Some(DiagnosticCode::UnknownCommand)),
        "\\todo is known once implemented: {:?}",
        parsed.diagnostics
    );
    assert!(
        margin_notes(&parsed.blocks).is_empty(),
        "no margin note without the package: {:?}",
        parsed.blocks
    );
    let prose: String = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flat_map(|inlines| {
            inlines.iter().filter_map(|i| match i {
                Inline::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert!(prose.contains("note"), "argument kept: {prose:?}");
}

/// Without todonotes, a user's own `\newcommand{\todo}` wins, exactly as
/// in real LaTeX (same rule as soul's `\so`/`\hl`).
#[test]
fn user_todo_macro_wins_without_todonotes() {
    let source = "\\documentclass{article}\n\\newcommand{\\todo}[1]{[#1]}\n\\begin{document}\nA \\todo{bc} d.\n\\end{document}";
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "user \\todo must win without todonotes: {:?}",
        parsed.diagnostics
    );
    let prose: String = parsed
        .blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flat_map(|inlines| {
            inlines.iter().filter_map(|i| match i {
                Inline::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
        })
        .collect();
    assert!(
        prose.contains("[bc]"),
        "user \\todo expansion must survive: {prose:?}"
    );
}

/// `\usepackage{todonotes}` loads silently (no "not implemented" warning).
#[test]
fn todonotes_package_load_is_silent() {
    let parsed = parse(&doc("Text here."));
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
}

/// todonotes options are out of scope: `\todo[color=red]{note}` still sets
/// the plain note, with the options reported as ignored.
#[test]
fn todo_options_are_reported_and_ignored() {
    let parsed = parse(&doc("Text\\todo[color=red]{note} more."));
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("[options]")),
        "options diagnostic: {:?}",
        parsed.diagnostics
    );
    let notes = margin_notes(&parsed.blocks);
    assert_eq!(notes.len(), 1, "{:?}", parsed.blocks);
    assert_eq!(note_words(notes[0]), vec!["note".to_string()]);
}

/// `\listoftodos` and `\missingfigure` are out of scope and stay
/// unrecognised (unknown-command error), even with todonotes loaded.
#[test]
fn listoftodos_and_missingfigure_stay_unrecognised() {
    for name in ["listoftodos", "missingfigure"] {
        let parsed = parse(&doc(&format!("Text\\{name} more.")));
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|d| d.code == Some(DiagnosticCode::UnknownCommand)
                    && d.message.contains(&format!("\\{name}"))),
            "{name} must stay unknown: {:?}",
            parsed.diagnostics
        );
        assert!(
            margin_notes(&parsed.blocks).is_empty(),
            "{name} must not produce a margin note: {:?}",
            parsed.blocks
        );
    }
}
