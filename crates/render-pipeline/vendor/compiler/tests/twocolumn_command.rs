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
