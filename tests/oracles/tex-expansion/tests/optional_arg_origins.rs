//! Replacement-text tokens of a `\newcommand` with an optional argument
//! (`\@protected@testopt` -> `\@ifnextchar` -> `\futurelet`) report the
//! document invocation as their origin, exactly as a mandatory-only
//! command's do (#925). A host places replacement tokens at the
//! invocation from that origin; with a prelude span there instead it fell
//! back to the definition's bytes and read preamble text as interword gaps.

use flashtex_tex_expansion::{Engine, Span, Token, TokenKind};

/// `(token, origin)` for every content token of `src`.
fn expand(src: &str) -> Vec<(Token, Option<Span>)> {
    let mut e = Engine::new(src);
    let mut out = Vec::new();
    while let Some(pair) = e.next_content_token_with_origin() {
        out.push(pair);
    }
    assert!(e.diagnostics().is_empty(), "{src:?}: {:?}", e.diagnostics());
    out
}

fn is_doc_span(s: Span) -> bool {
    !s.is_synthetic() && s.source_id == 0
}

/// Every content token after the definition either lies inside or after
/// the invocation `call` (a token read from the document: the arguments
/// and what follows) or carries `call`'s span as its origin (replacement
/// text, default included).
fn check(src: &str, call: &str, expected_text: &str) {
    let start = src.rfind(call).expect("invocation in source") as u32;
    let invocation = Span { source_id: 0, start, end: start + call.len() as u32 };
    let toks = expand(src);
    let text: String = toks
        .iter()
        .filter_map(|(t, _)| match &t.kind {
            TokenKind::Char(c, _) => Some(*c),
            TokenKind::ControlSequence(cs) => Some(cs.chars().next().unwrap()),
            _ => None,
        })
        .collect();
    assert_eq!(text, expected_text, "{src:?}");
    for (tok, origin) in &toks {
        // Replacement tokens keep the definition's span (the host maps
        // them through the origin); a token read from the document after
        // the call has no origin.
        match origin {
            None => assert!(
                is_doc_span(tok.span) && tok.span.start >= start,
                "{src:?}: {tok:?} has no origin yet is not a document token after the call"
            ),
            Some(o) => assert_eq!(*o, invocation, "{src:?}: origin of {tok:?} is not the invocation"),
        }
    }
}

#[test]
fn optional_default_replacement_originates_at_the_invocation() {
    check(r"\newcommand{\lb}[2][x]{(#1:#2)}\lb{a}", r"\lb", "(x:a)");
}

#[test]
fn optional_given_replacement_originates_at_the_invocation() {
    check(r"\newcommand{\lb}[2][x]{(#1:#2)}\lb[y]{a}", r"\lb", "(y:a)");
}

#[test]
fn optional_after_space_originates_at_the_invocation() {
    // `\lb {a}`: `\@ifnch` sees `\@sptoken`, `\@xifnch` re-peeks.
    check(r"\newcommand{\lb}[2][x]{(#1:#2)}\lb {a}", r"\lb", "(x:a)");
}

#[test]
fn optional_with_no_mandatory_originates_at_the_invocation() {
    check(r"\newcommand{\lb}[1][x]{(#1)}\lb.", r"\lb", "(x).");
}

#[test]
fn mandatory_only_replacement_originates_at_the_invocation() {
    // The reference behaviour (#923): nothing new here.
    check(r"\newcommand{\lb}[1]{(#1)}\lb{a}", r"\lb", "(a)");
}
