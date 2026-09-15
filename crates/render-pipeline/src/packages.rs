//! Packages the pipeline implements on the compiler's behalf.
//!
//! The compiler reports `packages X, Y are recognised but not implemented`
//! for every `\usepackage` it has no model for (`parser::package_matches_layout`).
//! Several of those packages are set by this crate, not the compiler: the
//! Latin Modern text and math faces (`lmodern`, `amsfonts`), the amsmath
//! displays and amssymb glyph tables (`mathtex`, `style::cmex_designs`),
//! `microtype` protrusion and expansion (`adapter::microtype_setup`), the
//! `geometry` page frame (`flashtex_class_geometry` through
//! `adapter::document_setup`), `graphicx` image items (`floats::ImageCache`),
//! `float`'s `[H]` placement (`floats::prepare`) and `tikz` pictures (`tikz`). A consumer of this crate's display list would
//! otherwise be told that what it is looking at was not typeset.
//!
//! The rewrite keeps the compiler's exact phrasing so the Mac app's
//! diagnostic categories (which key off the message) keep matching.
//!
//! `\pagestyle` in the preamble is the same shape of gap: the compiler
//! rejects it as unsupported preamble material, while
//! `adapter::document_setup` reads it through `DocumentSetup::from_preamble`
//! whenever the document declares a standard class.

const PREFIX: &str = "packages ";
const SUFFIX: &str = " are recognised but not implemented";

/// Packages whose effect on the output this crate produces.
pub fn implemented_by_pipeline(package: &str) -> bool {
    matches!(
        package,
        "amsmath" | "amssymb" | "amsfonts" | "lmodern" | "microtype" | "geometry" | "graphicx" | "tikz" | "float"
    )
}

/// Splits a message into its body and any trailing bracketed clauses.
///
/// `display::Diagnostic::from_compiler` appends the compiler's structured
/// `labels`/`notes`/`help` (#346/#389) to `message` as ` [note: …]` /
/// ` [help: …]`, because display-list-v2's diagnostic object has no field of
/// its own for them. Everything in this module matches on the compiler's
/// *phrasing*, so it has to see the body that was phrased rather than the
/// body plus whatever was appended: with the clause on the end,
/// `supersede_message`'s `strip_suffix` fails and every superseded
/// `\usepackage` diagnostic leaks through — for `amsmath`, `microtype`,
/// `geometry` and the rest of [`implemented_by_pipeline`] alike.
///
/// This recognises the clause *shape* — ` [<label>: <text>]` with a plain
/// word for the label — rather than the two labels that exist today, so a
/// third kind appended later keeps working without another fix here. Bracket
/// depth is tracked, so clause text may itself contain brackets, and a body
/// that merely ends in `]` is not mistaken for a clause.
///
/// Returns `(body, clauses)` with the leading space kept on `clauses`, so the
/// original is exactly `format!("{body}{clauses}")`.
fn split_clauses(message: &str) -> (&str, &str) {
    let mut cut = message.len();
    loop {
        let head = &message[..cut];
        if !head.ends_with(']') {
            break;
        }
        // Walk back to the `[` opening this clause, counting depth so a
        // bracket inside the clause text does not end the scan early.
        let mut depth = 0usize;
        let mut open = None;
        for (i, c) in head.char_indices().rev() {
            match c {
                ']' => depth += 1,
                '[' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    if depth == 0 {
                        open = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        // A clause is ` [<label>: <text>]`: preceded by a space, opening with
        // a plain-word label.
        let Some(open) = open.filter(|i| *i > 0 && head.as_bytes()[i - 1] == b' ') else {
            break;
        };
        let Some((label, _)) = head[open + 1..cut - 1].split_once(": ") else {
            break;
        };
        if label.is_empty()
            || !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            break;
        }
        cut = open - 1;
    }
    message.split_at(cut)
}

/// Rewrites a compiler `packages … are recognised but not implemented`
/// message without the packages this crate sets; `None` drops the
/// diagnostic because every listed package is implemented here, and a
/// message of any other shape comes back unchanged.
///
/// Trailing `[note: …]`/`[help: …]` clauses are split off before matching and
/// re-attached to whatever survives, so an appended clause can never stop a
/// diagnostic being superseded (see [`split_clauses`]).
pub fn supersede_message(message: &str) -> Option<String> {
    let (body, clauses) = split_clauses(message);
    let Some(list) = body.strip_prefix(PREFIX).and_then(|m| m.strip_suffix(SUFFIX)) else {
        return Some(message.to_string());
    };
    let listed = list.split(", ").count();
    let remaining: Vec<&str> = list
        .split(", ")
        .filter(|package| !implemented_by_pipeline(package))
        .collect();
    if remaining.is_empty() {
        // The whole diagnostic goes, any clause with it.
        None
    } else if remaining.len() == listed {
        Some(message.to_string())
    } else {
        // The survivors really are unimplemented, so the compiler's help
        // ("remove that \usepackage …") still applies to them.
        Some(format!("{PREFIX}{}{SUFFIX}{clauses}", remaining.join(", ")))
    }
}

/// The preamble commands `adapter::document_setup` reads for a document with
/// an explicit `\documentclass`; the compiler's "not supported in the
/// document preamble" error for them is superseded.
///
/// `lib.rs` calls this on the compiler's own diagnostic, before
/// `from_compiler` appends any clause, so unlike `supersede_message` this one
/// does not leak today — but it compares the whole message for equality, so
/// it would the moment that call moved after the conversion. It reads the
/// body for the same reason.
pub fn preamble_command_superseded(message: &str, has_class: bool) -> bool {
    has_class && split_clauses(message).0 == "\\pagestyle is not supported in the document preamble"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packages_the_pipeline_sets_leave_the_message() {
        assert_eq!(supersede_message("packages amsmath, amssymb are recognised but not implemented"), None);
        assert_eq!(supersede_message("packages microtype are recognised but not implemented"), None);
        assert_eq!(
            supersede_message("packages amsmath, booktabs, tikz are recognised but not implemented").as_deref(),
            Some("packages booktabs are recognised but not implemented")
        );
    }

    #[test]
    fn pagestyle_in_a_classed_preamble_is_the_pipelines() {
        let m = "\\pagestyle is not supported in the document preamble";
        assert!(preamble_command_superseded(m, true));
        assert!(!preamble_command_superseded(m, false));
        assert!(!preamble_command_superseded("\\foo is not supported in the document preamble", true));
    }

    #[test]
    fn other_packages_and_other_messages_are_untouched() {
        let cite = "packages cite, booktabs are recognised but not implemented";
        assert_eq!(supersede_message(cite).as_deref(), Some(cite));
        let other = "\\setlist keys x are recognised but not implemented";
        assert_eq!(supersede_message(other).as_deref(), Some(other));
    }

    /// `display::Diagnostic::from_compiler` appends the compiler's structured
    /// `help`/`notes` after the message. Every test above uses a bare message,
    /// which is exactly why the clause once slipped past `strip_suffix` and
    /// leaked the superseded diagnostics into `flashtex-render`'s output.
    const HELP: &str =
        " [help: remove that \\usepackage if you do not need it; its commands are still diagnosed when used]";

    #[test]
    fn an_appended_help_clause_does_not_stop_a_supersede() {
        assert_eq!(supersede_message(&format!("packages amsmath, amssymb are recognised but not implemented{HELP}")), None);
        assert_eq!(supersede_message(&format!("packages microtype are recognised but not implemented{HELP}")), None);
        // Every package this crate sets, one at a time.
        for p in ["amsmath", "amssymb", "amsfonts", "lmodern", "microtype", "geometry", "graphicx", "tikz", "float"] {
            let m = format!("packages {p} are recognised but not implemented{HELP}");
            assert_eq!(supersede_message(&m), None, "{p} leaked");
        }
    }

    #[test]
    fn a_mixed_list_with_a_clause_is_trimmed_and_keeps_the_clause() {
        // The survivors are still unimplemented, so the help still applies.
        assert_eq!(
            supersede_message(&format!("packages amsmath, booktabs, tikz are recognised but not implemented{HELP}")).as_deref(),
            Some(&*format!("packages booktabs are recognised but not implemented{HELP}"))
        );
        // Nothing of ours in the list: unchanged, clause included.
        let cite = format!("packages cite, booktabs are recognised but not implemented{HELP}");
        assert_eq!(supersede_message(&cite).as_deref(), Some(&*cite));
    }

    #[test]
    fn several_clauses_and_clauses_holding_brackets_are_still_split_off() {
        let many = "packages amsmath are recognised but not implemented \
                    [note: this command] [note: \\tilde is a math accent] [help: wrap it in math: \\(x\\)]";
        assert_eq!(supersede_message(many), None);
        // Clause text may itself contain brackets.
        let bracketed = "packages amsmath, booktabs are recognised but not implemented [help: use \\cite[p. 1]{k}]";
        assert_eq!(
            supersede_message(bracketed).as_deref(),
            Some("packages booktabs are recognised but not implemented [help: use \\cite[p. 1]{k}]")
        );
    }

    #[test]
    fn a_body_that_merely_ends_in_a_bracket_is_not_read_as_a_clause() {
        for body in [
            "packages amsmath are recognised but not implemented [not a clause]",
            "packages amsmath are recognised but not implemented [help without a colon]",
            "some other message ending in a list [a, b]",
        ] {
            assert_eq!(split_clauses(body), (body, ""), "{body}");
        }
        // No trailing bracket at all, and a bare `[` that opens nothing.
        assert_eq!(split_clauses("plain message"), ("plain message", ""));
        assert_eq!(split_clauses("unbalanced ] here"), ("unbalanced ] here", ""));
    }

    #[test]
    fn split_clauses_round_trips() {
        let m = format!("packages amsmath are recognised but not implemented{HELP}");
        let (body, clauses) = split_clauses(&m);
        assert_eq!(body, "packages amsmath are recognised but not implemented");
        assert_eq!(clauses, HELP);
        assert_eq!(format!("{body}{clauses}"), m);
    }

    #[test]
    fn a_pagestyle_error_is_superseded_with_a_clause_too() {
        let m = format!("\\pagestyle is not supported in the document preamble{HELP}");
        assert!(preamble_command_superseded(&m, true));
        assert!(!preamble_command_superseded(&m, false));
        assert!(!preamble_command_superseded(&format!("\\foo is not supported in the document preamble{HELP}"), true));
    }
}
