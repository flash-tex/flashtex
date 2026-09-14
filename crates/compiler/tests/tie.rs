//! The tie, and the control symbols that share its failure mode.
//!
//! LaTeX has three spellings of one thing: `~` (latex.ltx 9413 makes it
//! catcode 13 and expands it to `\nobreakspace`), `\nobreakspace` itself
//! (latex.ltx 9411, `\leavevmode\nobreak\ `), and a non-breaking space typed
//! straight into a UTF-8 source, which `inputenc` maps onto the same
//! command. All three reach the text stream as `lexer::NO_BREAK_SPACE`, so
//! a consumer recognises the tie by the character it is looking at and
//! never by reading the source bytes back.
//!
//! That distinction is the bug these tests pin. The lexer records a control
//! symbol's escape only in the *width* of its span, and a token copied out
//! of a macro's replacement text carries the span of the *invocation*, so
//! any decision made from `token.span` there is made from unrelated bytes.
//! `\newcommand{\fig}{Figure~7}` printed a literal tilde and
//! `\newcommand{\ab}{A\,B}` printed a literal comma, while the identical
//! source outside a macro body was correct.

use flashtex_compiler::lexer::NO_BREAK_SPACE;
use flashtex_compiler::parser::{self, Block, Inline};

fn body(source: &str) -> Vec<Inline> {
    let parsed = parser::parse(source);
    let mut out = Vec::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            out.extend(inlines.iter().cloned());
        }
    }
    out
}

/// Every `Inline::Text` of the first paragraph, concatenated.
fn text_of(source: &str) -> String {
    body(source)
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn diagnostics(source: &str) -> Vec<String> {
    parser::parse(source).diagnostics.iter().map(|d| d.message.clone()).collect()
}

fn doc(inner: &str) -> String {
    format!(r"\documentclass{{article}}\begin{{document}}{inner}\end{{document}}")
}

fn with_preamble(preamble: &str, inner: &str) -> String {
    format!(r"\documentclass{{article}}{preamble}\begin{{document}}{inner}\end{{document}}")
}

#[test]
fn a_typed_tie_is_a_no_break_space_not_a_tilde() {
    assert_eq!(text_of(&doc("Figure~7")), format!("Figure{NO_BREAK_SPACE}7"));
    assert!(diagnostics(&doc("Figure~7")).is_empty());
}

/// The defect: replacement text carries the invocation's span, so nothing
/// downstream could tell this `~` from `\textasciitilde` by looking at the
/// source. `\def` takes the same path as `\newcommand`.
#[test]
fn a_tie_inside_a_macro_body_is_still_a_tie() {
    let expected = format!("Figure{NO_BREAK_SPACE}7");
    assert_eq!(text_of(&with_preamble(r"\newcommand{\fig}{Figure~7}", r"\fig")), expected);
    assert_eq!(text_of(&with_preamble(r"\newcommand{\fig}[1]{Figure~#1}", r"\fig{7}")), expected);
    assert_eq!(text_of(&with_preamble(r"\def\fig{Figure~7}", r"\fig")), expected);
}

/// `~` written at the call site rather than in the body was never broken --
/// argument text is the source's own bytes -- and must stay that way.
#[test]
fn a_tie_in_a_macro_argument_is_a_tie() {
    let source = with_preamble(r"\newcommand{\wrap}[1]{#1}", r"\wrap{Figure~7}");
    assert_eq!(text_of(&source), format!("Figure{NO_BREAK_SPACE}7"));
}

/// `\textasciitilde` is the only way to ask for the character, and it must
/// keep producing it -- including inside a macro body, where the span is
/// just as unreliable in the other direction.
#[test]
fn textasciitilde_is_the_character_everywhere() {
    assert_eq!(text_of(&doc(r"A\textasciitilde B")), "A~B");
    let in_body = with_preamble(r"\newcommand{\ab}{A\textasciitilde B}", r"\ab");
    assert_eq!(text_of(&in_body), "A~B");
}

/// `\nobreakspace` is `\DeclareRobustCommand` in latex.ltx, not a
/// `\DeclareTextCommand`: it exists in every encoding. Resolving it through
/// the encoding tables reported `unavailable in encoding OT1` and typeset
/// nothing, silently deleting the space.
#[test]
fn nobreakspace_is_the_tie_and_raises_no_diagnostic() {
    for source in [doc(r"Figure\nobreakspace 7"), doc(r"Figure\nobreakspace{}7")] {
        assert_eq!(text_of(&source), format!("Figure{NO_BREAK_SPACE}7"), "{source}");
        assert_eq!(diagnostics(&source), Vec::<String>::new(), "{source}");
    }
}

/// A non-breaking space typed into the source is the same tie
/// (`inputenc`'s `\DeclareUnicodeCharacter{00A0}{\nobreakspace}`), so it
/// must survive the text stream unchanged rather than being mistaken for
/// ordinary whitespace.
#[test]
fn a_typed_no_break_space_survives() {
    let source = doc(&format!("Figure{NO_BREAK_SPACE}7"));
    assert_eq!(text_of(&source), format!("Figure{NO_BREAK_SPACE}7"));
}

/// The same span confusion, one bug away: `\,` is lexed as the word `,`
/// whose span happens to be two bytes wide, so replacement text printed the
/// comma. Text mode uses `Inline::Kern`; the math hand-off is a separate
/// lane's fix.
#[test]
fn a_control_symbol_kern_survives_a_macro_body() {
    let kerns = |source: &str| {
        body(source)
            .iter()
            .filter(|inline| matches!(inline, Inline::Kern { .. }))
            .count()
    };
    for (symbol, spelled) in [(r"\,", ","), (r"\!", "!"), (r"\:", ":"), (r"\;", ";"), (r"\>", ">")] {
        let source = with_preamble(&format!(r"\newcommand{{\ab}}{{A{symbol}B}}"), r"\ab");
        assert_eq!(kerns(&source), 1, "{symbol} in a macro body is a kern");
        assert_eq!(text_of(&source), "AB", "{symbol} in a macro body sets no {spelled}");
    }
}

/// The control symbol's twin: a genuine comma in a macro body is still a
/// comma. The fix reads the token's own bytes, so it must not start
/// swallowing ordinary punctuation.
#[test]
fn an_ordinary_comma_in_a_macro_body_is_still_a_comma() {
    let source = with_preamble(r"\newcommand{\ab}{A,B}", r"\ab");
    assert_eq!(text_of(&source), "A,B");
}

/// `\verb` and `verbatim` are where pdflatex stops treating `~` as active,
/// and they never reach the ligature pass, so the character stays.
#[test]
fn verbatim_keeps_the_literal_tilde() {
    let verbatim: String = body(&doc(r"\verb|~/.local|"))
        .iter()
        .filter_map(|inline| match inline {
            Inline::Verbatim { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(verbatim, "~/.local");
}
