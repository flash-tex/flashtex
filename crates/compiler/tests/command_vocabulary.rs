//! `KNOWN_UNIMPLEMENTED_COMMANDS` inventory honesty (GH-TABLE2-UNTESTED-7).
//!
//! That list means "real LaTeX this compiler does not implement". A name with
//! a working dispatch arm must not be in it; a name with no implementation
//! must stay in it (otherwise it is misreported as an unknown-command typo).
use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::parser::parse;
use flashtex_compiler::vocabulary::{
    is_known_command, is_known_environment, is_listed_as_unimplemented,
};

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

/// The tabular row scanner's commands (`parser::tabular`) have no
/// `Parser::command` arm, which is how they ended up listed as unimplemented
/// while working. They are implemented, so the listing must go but the names
/// must stay known (a use outside a table reports unsupported, not a typo).
#[test]
fn table_commands_are_implemented_so_they_are_known_without_the_unimplemented_list() {
    for name in [
        "hline",
        "cline",
        "multicolumn",
        "tabularnewline",
        "toprule",
        "midrule",
        "bottomrule",
        "cmidrule",
        "addlinespace",
        "specialrule",
        "morecmidrules",
        "multirow",
        "rowcolor",
        "cellcolor",
        "columncolor",
        "kill",
        "endfirsthead",
        "endhead",
        "endfoot",
        "endlastfoot",
    ] {
        assert!(is_known_command(name), "{name}");
        assert!(
            !is_listed_as_unimplemented(name),
            "{name} must not be listed as unimplemented once the row scanner handles it"
        );
    }
    assert!(is_known_environment("longtable"));
    let parsed = parse(
        "\\documentclass{article}\
         \\usepackage{booktabs}\\usepackage{multirow}\\usepackage{colortbl}\
         \\begin{document}\
         \\begin{tabular}{cc}\
         \\toprule \\multirow{2}{*}{x}&b\\\\\
         \\cline{1-2}\\rowcolor{red}c&\\cellcolor{red}d\\\\\
         \\multicolumn{2}{c}{wide}\\tabularnewline\
         \\bottomrule\\end{tabular}\
         \\end{document}",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
    let parsed = parse(
        "\\documentclass{article}\
         \\usepackage{longtable}\
         \\begin{document}\
         \\begin{longtable}{cc}a&b\\\\\\endhead c&d\\end{longtable}\
         \\end{document}",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
}

/// The expansion pass executes these, so they were never "unimplemented":
/// counter formats, `\arraystretch`, `\newif` conditionals and `\verb`.
/// Like the table commands above, the listing must go but the names stay.
#[test]
fn expansion_commands_are_known_without_the_unimplemented_list() {
    for name in [
        "arabic",
        "roman",
        "Roman",
        "alph",
        "arraystretch",
        "newif",
        "verb",
    ] {
        assert!(is_known_command(name), "{name}");
        assert!(
            !is_listed_as_unimplemented(name),
            "{name} must not be listed as unimplemented once the expansion pass handles it"
        );
    }
    let parsed = parse(
        "\\documentclass{article}\
         \\newif\\iffoo\\footrue\
         \\renewcommand{\\arraystretch}{1.5}\
         \\begin{document}\
         \\section{S}\\arabic{section} \\iffoo x\\fi\\verb|y|\
         \\begin{tabular}{cc}a&b\\end{tabular}\
         \\end{document}",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn llap_and_rlap_are_known_unimplemented_not_flat_typos() {
    // Issue #835: `\llap`/`\rlap` are real kernel commands (zero-width
    // boxes overhanging left/right), so the typo suggester must not offer
    // `\flat`. They stay listed as unimplemented until a real
    // implementation lands.
    for name in ["llap", "rlap"] {
        assert!(is_known_command(name), "{name}");
        assert!(
            is_listed_as_unimplemented(name),
            "{name} must stay listed as unimplemented until it is implemented"
        );
        let parsed = parse(&format!(
            "\\documentclass{{article}}\\begin{{document}}\\{name}{{x}}y\\end{{document}}",
        ));
        assert_eq!(
            parsed.diagnostics.len(),
            1,
            "{name}: {:?}",
            parsed.diagnostics
        );
        let diag = &parsed.diagnostics[0];
        assert_eq!(diag.code, Some(DiagnosticCode::UnsupportedFeature));
        assert_eq!(
            diag.message,
            format!("\\{name} is not supported by this compiler version")
        );
        assert!(
            diag.help.is_none(),
            "{name}: no misleading suggestion, got {:?}",
            diag.help
        );
    }
}

#[test]
fn addvspace_is_implemented_so_it_is_known_without_the_unimplemented_list() {
    // `\addvspace` has a real dispatch arm (`"addvspace" => ... vertical_command`)
    // and a `BUILT_INS` entry, so it stays known for typo suggestions even
    // though it is no longer listed as unimplemented.
    assert!(is_known_command("addvspace"));
    // `is_known_command` alone would still pass here even if the
    // `KNOWN_UNIMPLEMENTED_COMMANDS` removal were reverted, since `addvspace`
    // is unconditionally known via `BUILT_INS` regardless. Assert the
    // removal directly.
    assert!(
        !is_listed_as_unimplemented("addvspace"),
        "addvspace must not be listed as unimplemented once it has a real dispatch arm"
    );
    let parsed = parse(
        "\\documentclass{article}\\begin{document}Text\\addvspace{1em}More.\\end{document}",
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{:?}",
        parsed.diagnostics
    );
}
