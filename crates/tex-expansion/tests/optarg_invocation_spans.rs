//! GH-925: a `\newcommand` with an optional argument
//! (`\newcommand{\lb}[2][x]{...}`, expanded via `\@protected@testopt` /
//! `\@testopt` / `\kernel@ifnextchar`'s `\futurelet`) must stamp the
//! invocation origin onto its replacement tokens, exactly like the
//! mandatory-only path (GH-919, fixed for that path in #923).
//!
//! `\futurelet` used to push its two lookahead tokens back with
//! `push_tokens`, stamping both with the peeked token's origin. Inside an
//! optional-argument expansion that origin is the document token after the
//! call, so the `\@testopt` / `\@ifnch` chain expanded with no invocation
//! origin and the replacement tokens reached the parser carrying the
//! definition's spans instead of the invocation's. The render pipeline's
//! source-byte gap reader then measured preamble bytes as the interword gap:
//! `(\lb{a})` rendered with a spurious space before the `)`.
use flashtex_tex_expansion::{Engine, Span, TokenKind};

/// `(text, span, origin)` of every content token of `src`.
fn stream(src: &str) -> Vec<(String, Span, Option<Span>)> {
    let mut engine = Engine::new(src);
    let mut out = Vec::new();
    while let Some((tok, origin)) = engine.next_content_token_with_origin() {
        let text = match &tok.kind {
            TokenKind::Char(c, _) => c.to_string(),
            TokenKind::ControlSequence(name) => format!("\\{name}"),
            TokenKind::ActiveChar(c) => c.to_string(),
            TokenKind::Param(n) => format!("#{n}"),
            TokenKind::Eof => "<eof>".to_string(),
        };
        out.push((text, tok.span, origin));
    }
    assert!(
        engine.take_diagnostics().is_empty(),
        "unexpected diagnostics for {src:?}"
    );
    out
}

/// The invocation control word's span: `name` (`\lb`, `\m`) right after the
/// body `(`. (The preamble's own `\newcommand{\lb}` must not match.)
fn invocation(src: &str, name: &str) -> Span {
    let at = src.find(&format!("({name}")).expect("body invocation") + 1;
    Span::new(0, at as u32, (at + name.len()) as u32)
}

fn check(src: &str, name: &str, middle: &[&str]) {
    let got = stream(src);
    let texts: Vec<&str> = got.iter().map(|(t, _, _)| t.as_str()).collect();
    let mut expected = vec!["("];
    expected.extend(middle.iter().copied());
    expected.push(")");
    assert_eq!(texts, expected, "expansion text of {src:?}");
    let inv = invocation(src, name);
    // The surrounding document tokens are not part of any expansion.
    assert_eq!(got.first().map(|(_, _, o)| *o), Some(None));
    assert_eq!(got.last().map(|(_, _, o)| *o), Some(None));
    // Every replacement token carries the invocation origin — the
    // mandatory-path contract — rather than none (or a prelude span, which
    // is what the `\futurelet` lookahead left behind before the fix).
    for (text, _, origin) in &got[1..got.len() - 1] {
        assert_eq!(*origin, Some(inv), "`{text}` of {src:?} lost the invocation origin");
    }
}

#[test]
fn optional_argument_defaults_to_invocation_origin() {
    check(r"\newcommand{\lb}[2][x]{[#1#2]}(\lb{a})", "\\lb", &["[", "x", "a", "]"]);
}

#[test]
fn explicit_optional_argument_keeps_invocation_origin() {
    check(r"\newcommand{\lb}[2][x]{[#1#2]}(\lb[y]{a})", "\\lb", &["[", "y", "a", "]"]);
}

#[test]
fn mandatory_only_control_still_stamps_invocation_origin() {
    // The #923-fixed path, unchanged: the optional-argument cases above
    // must behave exactly like this one.
    check(r"\newcommand{\m}[1]{[#1]}(\m{a})", "\\m", &["[", "a", "]"]);
}
