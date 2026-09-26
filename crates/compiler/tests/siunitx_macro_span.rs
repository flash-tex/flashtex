//! GitHub issue #760: a control symbol's identity must come from its own
//! source bytes, not from the span width of whatever macro expanded to it.
//!
//! Root cause: `siunitx::raw_text` rebuilt the raw `\num`/`\unit`/`\qty`
//! argument by re-adding the backslash to a one-character word whose span
//! was one byte wider than the word (`\,` written directly spans 2 bytes).
//! A token copied out of a macro's replacement text carries the
//! *invocation's* span, so the width test really measured how long a name
//! the user happened to give the macro. The failure ran in both directions:
//!
//!   - `\,` inside a four-byte `\tss` lost its backslash and parsed as the
//!     decimal comma, exactly like a directly typed `,` (raw `1,2` instead
//!     of raw `1\,2`);
//!   - a plain `,` inside a two-byte `\q` spuriously gained a backslash and
//!     failed to parse, exactly like a directly typed `\,` (raw `1\,2`
//!     with an "invalid number" diagnostic instead of raw `1,2`).
//!
//! The fix keys the backslash off the lexer's `control_symbol` mark — the
//! identity signal already preserved through expansion — instead of span
//! length, the same idiom #756 established for math spacing.
//!
//! pdflatex ground truth (TeX Live 2026, siunitx v3,
//! `pdflatex -interaction=nonstopmode probe.tex`, exit 0, no diagnostics;
//! `pdftotext probe.pdf`):
//!
//!   - `\num{1,2}` sets "1.2" (comma is a decimal marker);
//!   - `\num{1\,2}` sets "12" with no error (`\,` is dropped);
//!   - `\unit{\metre\,\second}` sets "m s" (inter-unit product, no error).
//!
//! FlashTeX already matches pdflatex on the first case, directly and (after
//! this fix) through macros. The directly typed `\,` cases still diverge
//! from pdflatex (`\num{1\,2}` raises "invalid number"; see the guard test
//! below): that is a separate pre-existing siunitx `\,`-input gap, identical
//! with and without macros, and out of scope here. What this file pins is
//! the identity property: a macro must behave *exactly* like what it
//! expands to, in atoms and in diagnostics.

use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage{{siunitx}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

/// The first paragraph's math-atom nuclei (debug-spelled, spans excluded —
/// spans legitimately point at different source bytes for a macro
/// invocation versus direct source) plus the diagnostic messages: equal
/// pairs typeset identically.
fn shape(body: &str) -> (Vec<String>, Vec<String>) {
    let parsed = parse(&doc(body));
    let diagnostics = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    let inlines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("a paragraph");
    let mut spelled = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Math { list, .. } => {
                for atom in &list.atoms {
                    spelled.push(format!("{:?}", atom.nucleus));
                }
            }
            other => spelled.push(format!("{other:?}")),
        }
    }
    (spelled, diagnostics)
}

#[test]
fn escaped_comma_through_a_long_macro_matches_direct() {
    // `\tss` is four bytes, so the old width test missed the `\,` it
    // expands to: `\num{1\tss2}` parsed as `1,2` (decimal comma, no
    // diagnostic) while direct `\num{1\,2}` keeps the backslash (raw
    // `1\,2`, "invalid number" diagnostic). After the fix both are the
    // latter — identical atoms and identical diagnostics.
    let direct = shape("\\num{1\\,2}");
    assert_eq!(direct.1, ["siunitx: invalid number '1\\,2'"], "{direct:?}");
    let via_macro = shape("\\newcommand{\\tss}{\\,}\\num{1\\tss2}");
    assert_eq!(via_macro, direct, "macro \\, must match direct \\,");
}

#[test]
fn plain_comma_through_a_two_byte_macro_is_not_an_escape() {
    // The silent direction: `\q` is two bytes, so the `,` it expands to
    // used to look exactly like `\,` — `\num{1\q2}` failed with "invalid
    // number" while direct `\num{1,2}` sets 1.2. After the fix both set 1.2
    // with no diagnostics, matching pdflatex ("1.2", exit 0, no diagnostics).
    let direct = shape("\\num{1,2}");
    assert!(direct.1.is_empty(), "{direct:?}");
    assert_eq!(
        direct.0,
        ["Symbol(\"1\")", "Symbol(\".\")", "Symbol(\"2\")"],
        "{direct:?}"
    );
    let via_macro = shape("\\newcommand{\\q}{,}\\num{1\\q2}");
    assert_eq!(via_macro, direct, "macro `,` must match direct `,`");
}

#[test]
fn direct_inputs_unchanged_guards() {
    // Guard against over-correction now that detection no longer depends on
    // span length: directly typed inputs must keep their current behaviour.
    // `\num{1,2}` matches pdflatex ("1.2", no diagnostics). Direct
    // `\num{1\,2}` still raises "invalid number" (pdflatex silently sets
    // "12" — a separate pre-existing siunitx `\,`-input gap, identical with
    // and without macros, deliberately not changed by this fix).
    let (nuclei, diagnostics) = shape("\\num{1,2}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(
        nuclei,
        ["Symbol(\"1\")", "Symbol(\".\")", "Symbol(\"2\")"],
        "{nuclei:?}"
    );
    let (nuclei, diagnostics) = shape("\\num{1\\,2}");
    assert_eq!(
        diagnostics,
        ["siunitx: invalid number '1\\,2'"],
        "{nuclei:?}"
    );
    // (Nucleus's derived Debug escapes the backslash, hence `\\`.)
    assert_eq!(nuclei, ["Text(\"1\\\\,2\")"], "{nuclei:?}");
}
