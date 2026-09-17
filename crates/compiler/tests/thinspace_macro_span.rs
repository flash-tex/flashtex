//! GitHub issue #756: `\,`/`\:`/`\;`/`\!` inside a macro body must typeset
//! thin/medium/thick math spacing, not a literal punctuation glyph.
//!
//! Root cause: math spacing was detected by span byte-length (`\,` written
//! directly spans 2 bytes), but a macro-expanded `\,` carries the
//! *invocation's* span (e.g. `\dd` is 3 bytes), so longer-named macros fell
//! through to an ordinary symbol. The fix keys spacing off the lexer's
//! control-symbol identity instead of span length.

use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::LayoutConstraints;

fn compile(text: &str) -> flashtex_compiler::incremental::CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn texts(text: &str) -> Vec<String> {
    compile(text)
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| i.text.clone())
        .collect()
}

/// Positioned items (text plus rounded geometry): equal layouts mean the
/// same glyphs at the same positions, so a thin space is really a space of
/// the right width rather than a swallowed glyph.
fn laid_out(text: &str) -> Vec<(String, i64, i64)> {
    compile(text)
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| {
            (
                i.text.clone(),
                (i.x_pt * 100.0).round() as i64,
                (i.baseline_y_pt * 100.0).round() as i64,
            )
        })
        .collect()
}

#[test]
fn thinspace_in_macro_body_is_space_not_a_literal_comma() {
    // The issue's falsification cases: a 3-byte name (`\dd`), a 2-byte name
    // that only worked by span-length coincidence (`\e`), and another
    // 3-byte name (`\ee`). All three must behave exactly like `\,` typed
    // directly: no "," glyph, and the identical laid-out items.
    let direct = laid_out("$a\\,d$.\\n");
    assert!(!direct.iter().any(|(t, _, _)| t == ","), "{direct:?}");
    for (name, body) in [("dd", "\\,d"), ("e", "\\,d"), ("ee", "\\,d")] {
        let doc = format!("\\newcommand{{\\{name}}}{{{body}}}$a\\{name}$.\\n");
        let expanded = laid_out(&doc);
        assert!(
            !expanded.iter().any(|(t, _, _)| t == ","),
            "`\\{name}` with body `{body}` typeset a literal comma: {expanded:?}"
        );
        assert_eq!(
            expanded, direct,
            "`\\{name}` must lay out exactly like direct `\\,`"
        );
    }
    // The `\,\mathrm{d}` shape from the issue report.
    let dd_direct = laid_out("$a\\,\\mathrm{d}$.\\n");
    let dd_macro = laid_out("\\newcommand{\\dd}{\\,\\mathrm{d}}$a\\dd$.\\n");
    assert!(!dd_macro.iter().any(|(t, _, _)| t == ","), "{dd_macro:?}");
    assert_eq!(dd_macro, dd_direct);
}

#[test]
fn all_math_spacing_commands_survive_macro_expansion() {
    // `\,` `\:` `\;` `\!` (and `\|`, which shares the detection arm) via a
    // longer-named macro must match the direct source exactly.
    for (cmd, literal) in [("\\,", ","), ("\\:", ":"), ("\\;", ";"), ("\\!", "!")] {
        let direct = laid_out(&format!("$a{cmd}b$.\\n"));
        assert!(
            !direct.iter().any(|(t, _, _)| t == literal),
            "direct {cmd} typeset a literal {literal}: {direct:?}"
        );
        let doc = format!("\\newcommand{{\\sp}}{{a{cmd}b}}$\\sp$.\\n");
        assert_eq!(laid_out(&doc), direct, "{cmd} via macro must match direct");
    }
}

#[test]
fn direct_spacing_commands_add_width() {
    // A thin space is really laid out (not swallowed): `$a\,b$` is wider
    // than `$ab$`, with no literal comma either way.
    let plain = laid_out("$ab$.\\n");
    let spaced = laid_out("$a\\,b$.\\n");
    assert!(!spaced.iter().any(|(t, _, _)| t == ","), "{spaced:?}");
    assert_ne!(spaced, plain, "thin space must add width");
}

#[test]
fn literal_comma_in_math_stays_a_literal_comma() {
    // Guard against over-correction now that detection no longer depends on
    // span length: an ordinary `,` typed in math — directly or via a macro
    // body — must still render as punctuation. The 2-byte `\w` case is the
    // mirror of the bug: the old span heuristic mistook the macro's own
    // 2-byte span for a `\,` and swallowed the comma as a thin space.
    assert!(texts("$1,2$.\\n").contains(&",".to_string()));
    assert!(texts("\\newcommand{\\ww}{1,2}$\\ww$.\\n").contains(&",".to_string()));
    assert!(texts("\\newcommand{\\w}{1,2}$\\w$.\\n").contains(&",".to_string()));
}

#[test]
fn other_control_symbols_still_render_literally() {
    // `\%` `\&` `\$` share the lexer's control-symbol arm but are not
    // spacing commands: they must keep rendering as literal characters in
    // text, directly and via macro expansion. (`\&` in math is a separate
    // pre-existing limitation — "misplaced alignment tab" — untouched by
    // this fix, so only `\%` and `\$` are asserted in math mode.)
    for (cmd, literal) in [("\\%", "%"), ("\\&", "&"), ("\\$", "$")] {
        assert!(
            texts(&format!("a{cmd}b.\\n")).contains(&literal.to_string()),
            "text-mode {cmd}"
        );
        let doc = format!("\\newcommand{{\\cs}}{{a{cmd}b}}\\cs.\\n");
        assert!(texts(&doc).contains(&literal.to_string()), "{cmd} via macro");
    }
    for (cmd, literal) in [("\\%", "%"), ("\\$", "$")] {
        assert!(
            texts(&format!("$a{cmd}b$.\\n")).contains(&literal.to_string()),
            "math-mode {cmd}"
        );
        let doc = format!("\\newcommand{{\\cs}}{{a{cmd}b}}$\\cs$.\\n");
        assert!(
            texts(&doc).contains(&literal.to_string()),
            "{cmd} via macro in math"
        );
    }
}
