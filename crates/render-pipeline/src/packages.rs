//! Packages the pipeline implements on the compiler's behalf.
//!
//! The compiler reports `packages X, Y are recognised but not implemented`
//! for every `\usepackage` it has no model for (`parser::package_matches_layout`).
//! Several of those packages are set by this crate, not the compiler: the
//! Latin Modern text and math faces (`lmodern`, `amsfonts`), the amsmath
//! displays and amssymb glyph tables (`mathtex`, `style::cmex_designs`),
//! `microtype` protrusion and expansion (`adapter::microtype_setup`), the
//! `geometry` page frame (`flashtex_class_geometry` through
//! `adapter::document_setup`), `graphicx` image items (`floats::ImageCache`)
//! and `tikz` pictures (`tikz`). A consumer of this crate's display list would
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
        "amsmath" | "amssymb" | "amsfonts" | "lmodern" | "microtype" | "geometry" | "graphicx" | "tikz"
    )
}

/// Rewrites a compiler `packages … are recognised but not implemented`
/// message without the packages this crate sets; `None` drops the
/// diagnostic because every listed package is implemented here, and a
/// message of any other shape comes back unchanged.
pub fn supersede_message(message: &str) -> Option<String> {
    let Some(list) = message.strip_prefix(PREFIX).and_then(|m| m.strip_suffix(SUFFIX)) else {
        return Some(message.to_string());
    };
    let remaining: Vec<&str> = list
        .split(", ")
        .filter(|package| !implemented_by_pipeline(package))
        .collect();
    if remaining.is_empty() {
        None
    } else if remaining.len() == list.split(", ").count() {
        Some(message.to_string())
    } else {
        Some(format!("{PREFIX}{}{SUFFIX}", remaining.join(", ")))
    }
}

/// The preamble commands `adapter::document_setup` reads for a document with
/// an explicit `\documentclass`; the compiler's "not supported in the
/// document preamble" error for them is superseded.
pub fn preamble_command_superseded(message: &str, has_class: bool) -> bool {
    has_class && message == "\\pagestyle is not supported in the document preamble"
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
}
