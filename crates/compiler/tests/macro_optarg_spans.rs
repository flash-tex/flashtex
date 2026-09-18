//! GH-925: the optional-argument analogue of `macro_boundary_space.rs`
//! (GH-919, fixed for mandatory-only macros in #923).
//!
//! For `\newcommand{\lb}[2][x]{\uline{#1#2}}` the expansion runs through
//! `\@protected@testopt` / `\@testopt` / `\kernel@ifnextchar`'s `\futurelet`,
//! which used to drop the invocation origin. The `\uline` replacement tokens
//! of `(\lb{a})` then reached the parser with the definition's spans and
//! `maps_to_invocation == false`, so the render pipeline's source-byte gap
//! reader measured preamble bytes as the interword gap (`)` set with a
//! spurious space before it). The compiler side pinned by #923's test was
//! already right; here it is the broken seam, so this test pins the
//! provenance directly: every replacement token carries the invocation span
//! with `maps_to_invocation`, exactly like the mandatory-only control —
//! while argument tokens substituted from the document keep their own
//! spans, as before.
use flashtex_compiler::expansion::{expand_project, ExpandedToken};
use flashtex_compiler::lexer::TokenKind;
use flashtex_compiler::parser::SourceDocument;

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage[normalem]{{ulem}}\n\
         \\newcommand{{\\lb}}[2][x]{{\\uline{{#1#2}}}}\n\
         \\newcommand{{\\lm}}[1]{{\\uline{{x#1}}}}\n\
         \\newcommand{{\\lc}}[2]{{\\uline{{#1#2}}}}\n\
         \\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

/// The expansion tokens of one body line: preamble and document scaffolding
/// removed, `Space`/`ParBreak` dropped.
fn line_tokens(body: &str) -> (String, Vec<ExpandedToken>) {
    let source = doc(body);
    let docs = [SourceDocument { path: "main.tex", text: &source }];
    let expansion = expand_project(&docs, 0);
    assert!(
        expansion.diagnostics.is_empty(),
        "{body}: unexpected diagnostics: {:?}",
        expansion.diagnostics
    );
    let open = source.find(body).expect("body line");
    let close = open + body.len();
    let mut out = Vec::new();
    for token in expansion.tokens.iter() {
        if matches!(token.token.kind, TokenKind::Space | TokenKind::ParBreak) {
            continue;
        }
        if token.token.span.start >= open && token.token.span.end <= close {
            out.push(token.clone());
        }
    }
    assert!(!out.is_empty(), "{body}: no expansion tokens on the line");
    (source, out)
}

fn kind(token: &ExpandedToken) -> String {
    match &token.token.kind {
        TokenKind::Word(w) => format!("W({w})"),
        TokenKind::Command(c) => format!("C({c})"),
        TokenKind::LBrace => "{".to_string(),
        TokenKind::RBrace => "}".to_string(),
        other => format!("OTHER({other:?})"),
    }
}

/// `(name, via_optional_macro, mandatory_analogue, invocation_length)`:
/// each pair must expand to the same kinds with the same provenance, the
/// replacement tokens mapping to their own invocation span.
const PAIRS: [(&str, &str, &str, usize); 2] = [
    ("default", "(\\lb{a})", "(\\lm{a})", "\\lb".len()),
    ("explicit", "(\\lb[y]{a})", "(\\lc{y}{a})", "\\lb".len()),
];

#[test]
fn optional_argument_macros_map_replacements_to_the_invocation() {
    for (name, via_macro, mandatory, inv_len) in PAIRS {
        let (source, tokens) = line_tokens(via_macro);
        let (_, want) = line_tokens(mandatory);
        let kinds: Vec<String> = tokens.iter().map(kind).collect();
        let want_kinds: Vec<String> = want.iter().map(kind).collect();
        assert_eq!(
            kinds, want_kinds,
            "{name}: the optional-argument expansion differs from the mandatory one"
        );
        let open = source.find(via_macro).expect("body line") + 1;
        for token in &tokens {
            if token.maps_to_invocation {
                assert_eq!(
                    (token.token.span.start, token.token.span.end),
                    (open, open + inv_len),
                    "{name}: `{}` maps elsewhere than the invocation",
                    kind(token)
                );
                // ... and the definition span still records where the token
                // was copied from: the preamble, ahead of the invocation.
                let definition = token.definition.expect("{name}: mapped token without a definition span");
                assert!(
                    definition.end <= open,
                    "{name}: `{}` definition span is not in the preamble: {definition:?}",
                    kind(token)
                );
            } else {
                // Argument text substituted from the document (`a`, and the
                // explicitly given `y`) and the surrounding punctuation keep
                // their own source bytes, exactly as on the mandatory path.
                assert!(
                    (token.token.span.start, token.token.span.end) != (open, open + inv_len),
                    "{name}: `{}` unexpectedly maps to the invocation",
                    kind(token)
                );
            }
        }
        // The replacement is really there: the box command maps, the
        // argument word does not.
        assert!(
            tokens.iter().any(|t| matches!(t.token.kind, TokenKind::Command(ref c) if c == "uline") && t.maps_to_invocation),
            "{name}: `\\uline` does not map to the invocation: {kinds:?}"
        );
        assert!(
            tokens.iter().any(|t| matches!(t.token.kind, TokenKind::Word(ref w) if w == "a") && !t.maps_to_invocation),
            "{name}: the argument `a` lost its own span: {kinds:?}"
        );
    }
}

#[test]
fn optional_and_mandatory_provenance_agree_token_for_token() {
    // The full contract: same kinds, same flags; mapped spans are each
    // line's own invocation, while every unmapped span agrees up to the
    // line offset. (The mandatory analogues are named so both invocations
    // are three bytes: `\lm`, `\lc` against `\lb`.)
    for (name, via_macro, mandatory, inv_len) in PAIRS {
        let (source, tokens) = line_tokens(via_macro);
        let (want_source, want) = line_tokens(mandatory);
        let open = source.find(via_macro).expect("body line") + 1;
        let want_open = want_source.find(mandatory).expect("body line") + 1;
        // The mandatory analogue's invocation control word.
        let want_name = mandatory[1..].chars().take_while(|c| *c != '(' && *c != '[' && *c != '{').collect::<String>();
        let want_inv = (want_open, want_open + want_name.len());
        let own_inv = (open, open + inv_len);
        assert_eq!(tokens.len(), want.len(), "{name}: token counts differ");
        for (got, want) in tokens.iter().zip(&want) {
            assert_eq!(kind(got), kind(want), "{name}: kind differs");
            assert_eq!(
                got.maps_to_invocation, want.maps_to_invocation,
                "{name}: `{}` provenance differs",
                kind(got)
            );
            if got.maps_to_invocation {
                assert_eq!(
                    (got.token.span.start, got.token.span.end),
                    own_inv,
                    "{name}: `{}` is not its own invocation span",
                    kind(got)
                );
                assert_eq!(
                    (want.token.span.start, want.token.span.end),
                    want_inv,
                    "{name}: mandatory `{}` is not its own invocation span",
                    kind(want)
                );
            } else {
                // `(` precedes the invocation, so the offset is signed.
                let shift = |at: usize| (at as isize + open as isize - want_open as isize) as usize;
                assert_eq!(
                    (got.token.span.start, got.token.span.end),
                    (shift(want.token.span.start), shift(want.token.span.end)),
                    "{name}: `{}` span differs beyond the line offset",
                    kind(got)
                );
            }
        }
    }
}
