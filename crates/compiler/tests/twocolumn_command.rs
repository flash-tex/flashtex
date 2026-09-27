//! `\twocolumn[<material>]` (latex.ltx 20256-20275): the parser has no
//! `\@topnewpage` box model, but a renderer that wants to build one (the
//! render-pipeline crate's `columns` module) needs the bracket's content to
//! actually appear in this IR, at its own byte positions, rather than be
//! consumed and discarded the way `optional_bracket_argument` (an
//! options-string reader) would. See `crates/compiler/src/parser.rs`'s
//! `column_command`.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn words(out: &CompileOutput) -> Vec<String> {
    out.pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| i.text.clone())
        .collect()
}

/// The bracket's content must be typeset, not dropped: before the fix,
/// `optional_bracket_argument` consumed the tokens as an opaque string and
/// nothing of "Banner" or "text" ever reached a page.
#[test]
fn the_optional_arguments_material_is_typeset() {
    let src = "\\documentclass{article}\n\\begin{document}\nBanner text.\n\\twocolumn[Second column marker.]\nAfter the switch.\n\\end{document}\n";
    let out = compile(src);
    let w = words(&out);
    assert!(w.iter().any(|t| t.contains("Second")), "the bracket's own material must be typeset: {w:?}");
    assert!(w.iter().any(|t| t.contains("column")), "the rest of the bracket's material too: {w:?}");
    assert!(w.iter().any(|t| t.contains("After")), "and the text after \\twocolumn is unaffected: {w:?}");
}

/// `\twocolumn` is a known command (`BUILT_INS`): no "not supported by this
/// compiler version" diagnostic, whether or not it takes a bracket.
#[test]
fn twocolumn_is_not_reported_as_an_unknown_command() {
    for src in [
        "\\documentclass{article}\n\\begin{document}\nBody.\n\\twocolumn\nAfter.\n\\end{document}\n",
        "\\documentclass{article}\n\\begin{document}\nBody.\n\\twocolumn[Banner.]\nAfter.\n\\end{document}\n",
    ] {
        let out = compile(src);
        assert!(
            !out.diagnostics.iter().any(|d| d.message.contains("is not supported by this compiler version")),
            "{src:?}: {:?}",
            out.diagnostics
        );
    }
}

/// The positioning itself (`\@topnewpage`'s box above both columns) is still
/// honestly reported as not implemented here, distinct from the (now false)
/// claim that the argument was dropped.
#[test]
fn the_positioning_limitation_is_still_reported() {
    let src = "\\documentclass{article}\n\\begin{document}\nBody.\n\\twocolumn[Banner.]\nAfter.\n\\end{document}\n";
    let out = compile(&src);
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("\\@topnewpage") && d.message.contains("not implemented")),
        "{:?}",
        out.diagnostics
    );
    assert!(
        !out.diagnostics.iter().any(|d| d.message.contains("dropped")),
        "the argument is typeset now, not dropped: {:?}",
        out.diagnostics
    );
}

/// A bare `\twocolumn` (no bracket) gets no such diagnostic at all.
#[test]
fn no_bracket_no_diagnostic() {
    let src = "\\documentclass{article}\n\\begin{document}\nBody.\n\\twocolumn\nAfter.\n\\end{document}\n";
    let out = compile(&src);
    assert!(
        !out.diagnostics.iter().any(|d| d.message.contains("\\@topnewpage")),
        "{:?}",
        out.diagnostics
    );
}

/// PLAN1 site 37: every switch the document *runs* is reported on
/// `Parsed::column_switches`, wherever its bytes are. The render pipeline
/// read them out of the entry source instead, which cannot see one a macro
/// or a project `.sty` performed, and had to skip definition bodies by hand
/// so that an uncalled one did not count.
#[test]
fn every_switch_the_document_runs_is_reported() {
    let parse = flashtex_compiler::parser::parse;
    let flat = |src: &str| {
        parse(src)
            .column_switches
            .iter()
            .map(|c| (c.two, c.preamble, c.first_material))
            .collect::<Vec<_>>()
    };

    // Written directly and produced by a macro: the same switch.
    let direct = "\\documentclass{article}\n\\begin{document}\n\\twocolumn Text.\n\\end{document}\n";
    let via_macro = "\\documentclass{article}\n\\newcommand\\tc{\\twocolumn}\n\\begin{document}\n\\tc Text.\n\\end{document}\n";
    assert_eq!(flat(direct), [(true, false, true)]);
    assert_eq!(flat(via_macro), [(true, false, true)]);

    // A preamble switch, then one after material: `preamble` and
    // `first_material` tell the three positions apart.
    let mixed = "\\documentclass{article}\n\\twocolumn\n\\begin{document}\nIntro.\n\\onecolumn\nMore.\n\\end{document}\n";
    assert_eq!(flat(mixed), [(true, true, false), (false, false, false)]);

    // A switch inside a definition that is never called never ran.
    let uncalled = "\\documentclass{article}\n\\newcommand\\tc{\\twocolumn}\n\\begin{document}\nText.\n\\end{document}\n";
    assert_eq!(flat(uncalled), []);

    // The class option is not a switch: it is the starting value.
    let option = "\\documentclass[twocolumn]{article}\n\\begin{document}\nText.\n\\end{document}\n";
    assert_eq!(flat(option), []);
}

/// A preamble `\pagestyle{empty}` leaves a zero-width marker pending in the
/// open paragraph. It sets nothing on the page, so a `\twocolumn` right
/// after `\begin{document}` is still the document's first material -- which
/// is what decides whether `\@topnewpage` gets its banner box.
#[test]
fn a_pending_whatsit_does_not_end_the_first_material() {
    let src = "\\documentclass{article}\n\\pagestyle{empty}\n\\begin{document}\n\\twocolumn[Banner]\nText.\n\\end{document}\n";
    let switches = flashtex_compiler::parser::parse(src).column_switches;
    assert_eq!(switches.len(), 1, "{switches:?}");
    assert!(switches[0].first_material, "{switches:?}");
    assert!(!switches[0].preamble, "{switches:?}");
}
