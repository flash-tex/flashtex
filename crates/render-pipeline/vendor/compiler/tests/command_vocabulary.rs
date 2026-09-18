//! `KNOWN_UNIMPLEMENTED_COMMANDS` inventory honesty (GH-TABLE2-UNTESTED-7).
//!
//! That list means "real LaTeX this compiler does not implement". A name with
//! a working dispatch arm must not be in it; a name with no implementation
//! must stay in it (otherwise it is misreported as an unknown-command typo).
use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::parser::parse;
use flashtex_compiler::vocabulary::{is_known_command, is_listed_as_unimplemented};

#[test]
fn marginpar_is_implemented_so_it_is_known_without_the_unimplemented_list() {
    // `\marginpar` has a real dispatch arm (`"marginpar" => self.marginpar(..)`)
    // and a `BUILT_INS` entry, so it stays known for typo suggestions even
    // though it is no longer listed as unimplemented.
    assert!(is_known_command("marginpar"));
    // `is_known_command` alone would still pass here even if the
    // `KNOWN_UNIMPLEMENTED_COMMANDS` removal were reverted, since `marginpar`
    // is unconditionally known via `BUILT_INS` regardless. Assert the
    // removal directly.
    assert!(
        !is_listed_as_unimplemented("marginpar"),
        "marginpar must not be listed as unimplemented once it has a real dispatch arm"
    );
    let parsed = parse("\\documentclass{article}\\begin{document}Text\\marginpar{note}\\end{document}");
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn addvspace_is_reported_as_unimplemented_not_unknown() {
    // `\addvspace` has no dispatch arm and no `BUILT_INS` entry in this
    // crate: real usage reports `unsupported_feature`, and the name stays
    // known (not an unknown-command typo) via `KNOWN_UNIMPLEMENTED_COMMANDS`.
    // That entry must stay until a real implementation lands.
    assert!(is_known_command("addvspace"));
    let parsed = parse(
        "\\documentclass{article}\\begin{document}Text\\addvspace{1em}More.\\end{document}",
    );
    assert_eq!(parsed.diagnostics.len(), 1, "{:?}", parsed.diagnostics);
    let diag = &parsed.diagnostics[0];
    assert_eq!(diag.code, Some(DiagnosticCode::UnsupportedFeature));
    assert_eq!(
        diag.message,
        "\\addvspace is not supported by this compiler version"
    );
}
