//! Payload-by-payload coverage for the LaTeX content-safety gate: model
//! output (`proposal.latex`) is untrusted text that reaches a real compiler
//! process (`validation::CompilerValidator`) and, once a human approves it,
//! the user's own source file. Each test here drives that gate directly
//! (`Proposal::validate` / `validation::scan_latex`) with a payload built by
//! hand — no network, no API key, no external compiler.
use flashtex_bridge::{validation::scan_latex, Proposal};

fn proposal(latex: &str) -> Proposal {
    Proposal {
        latex: latex.into(),
        ambiguities: vec![],
        required_dependencies: vec![],
    }
}
fn rejected(latex: &str) {
    let err = proposal(latex).validate().unwrap_err();
    assert_eq!(err.code, "invalid_proposal", "payload: {latex:?}");
}
fn accepted(latex: &str) {
    proposal(latex)
        .validate()
        .unwrap_or_else(|e| panic!("expected acceptance of {latex:?}, got {e:?}"));
}

#[test]
fn shell_escape_is_rejected() {
    rejected(r"\write18{touch /tmp/pwned}");
    rejected(r"\immediate\write18{rm -rf ~}");
    // Numeric-register obfuscation of the stream number does not help: the
    // control word `write` itself is denied, not the literal text "write18".
    rejected(r"\write\numexpr 9*2\relax{echo hi}");
}

#[test]
fn file_io_primitives_are_rejected() {
    rejected(r"\input{/etc/passwd}");
    rejected(r"\include{secret}");
    rejected(r"\openin0=/etc/passwd ");
    rejected(r"\read0 to\x");
    rejected(r"\openout3=/tmp/evil ");
}

#[test]
fn catcode_and_primitive_redefinition_are_rejected() {
    rejected(r"\catcode`\@=11");
    rejected(r"\def\documentclass{fake}");
    rejected(r"\let\input\relax");
    rejected(r"\edef\x{y}");
    rejected(r"\gdef\x{y}");
    rejected(r"\xdef\x{y}");
    rejected(r"\makeatletter\@input{x}\makeatother");
    rejected(r"\documentclass{article}");
    rejected(r"\usepackage{shellesc}");
}

#[test]
fn csname_obfuscation_is_rejected() {
    // \csname ... \endcsname builds a control sequence name at expansion
    // time; it is denied outright rather than trying to evaluate what name
    // it would build.
    rejected(r"\csname write18\endcsname{ls}");
    rejected(r"\csname input\endcsname{/etc/passwd}");
}

#[test]
fn unbalanced_braces_and_environments_are_rejected() {
    rejected("{unbalanced open");
    rejected("unbalanced close}");
    rejected(r"\begin{align}x");
    rejected(r"\begin{align}x\end{itemize}");
    rejected(r"\end{align}"); // stray close with nothing open in this snippet
}

#[test]
fn end_document_truncation_is_rejected_however_it_is_spelled() {
    rejected(r"\end{document}");
    rejected(r"$x$ \end{document} everything after this is silently discarded");
    rejected(r"\begin{document}hi\end{document}");
}

#[test]
fn oversized_latex_was_already_rejected_before_this_change() {
    let huge = "x".repeat(70 * 1024);
    rejected(&huge);
}

#[test]
fn null_byte_was_already_rejected_before_this_change() {
    rejected("x\0y");
}

#[test]
fn deeply_nested_groups_beyond_the_hard_cap_are_rejected() {
    let mut s = String::new();
    for _ in 0..2500 {
        s.push('{');
    }
    for _ in 0..2500 {
        s.push('}');
    }
    rejected(&s);
}

#[test]
fn moderate_nesting_is_allowed_but_surfaced_as_an_advisory() {
    let mut latex = String::new();
    for _ in 0..25 {
        latex.push('{');
    }
    latex.push('x');
    for _ in 0..25 {
        latex.push('}');
    }
    accepted(&latex);
    let scan = scan_latex(&latex).unwrap();
    assert!(
        scan.advisories.iter().any(|a| a.contains("nests grouping")),
        "expected a nesting advisory, got {:?}",
        scan.advisories
    );
}

#[test]
fn loop_and_repeat_are_allowed_but_surfaced_as_an_advisory() {
    let latex = r"\loop \iterate \repeat";
    accepted(latex);
    let scan = scan_latex(latex).unwrap();
    assert!(scan.advisories.iter().any(|a| a.contains(r"\loop")));
    assert!(scan.advisories.iter().any(|a| a.contains(r"\repeat")));
}

#[test]
fn control_characters_and_bidi_overrides_are_rejected() {
    rejected("x\x01y"); // C0 control
    rejected("x\x1By"); // ESC
    rejected("x\x7Fy"); // DEL
    rejected("safe\u{202E}evil"); // right-to-left override (Trojan Source)
    rejected("safe\u{202A}evil"); // left-to-right embedding
    rejected("safe\u{2066}evil"); // left-to-right isolate
    rejected("safe\u{200E}evil"); // left-to-right mark
}

#[test]
fn invisible_zero_width_characters_are_allowed_but_surfaced_as_an_advisory() {
    let latex = "a\u{200B}b";
    accepted(latex);
    let scan = scan_latex(latex).unwrap();
    assert!(scan.advisories.iter().any(|a| a.contains("invisible")));
}

#[test]
fn invalid_utf8_cannot_reach_proposal_by_construction() {
    // `Proposal::latex` is a `String`, which the Rust standard library
    // guarantees is always valid UTF-8; there is no safe way to construct
    // one holding invalid UTF-8. The one place bytes from the model
    // provider are turned into a `String` is `serde_json::from_slice` in
    // `GrokClient::convert` (crates/bridge/src/grok.rs), which itself
    // requires its input to be valid UTF-8 JSON text and fails closed
    // (`provider_invalid_response`) otherwise. This test documents that
    // invariant (via a non-literal byte source, so the compiler can't just
    // fold the known-bad literal at compile time) rather than exercising a
    // reachable code path.
    let bytes: Vec<u8> = [0xFF, 0xFE, 0xFD].into_iter().collect();
    assert!(std::str::from_utf8(&bytes).is_err());
}

#[test]
fn ordinary_math_and_lookalike_control_words_are_unaffected() {
    // A denylist keyed on exact, maximal-munch control-sequence names (not
    // substrings) must not reject legitimate commands that merely start
    // with a denied word's letters.
    accepted(r"$\frac{a}{b} + \sum_{i=1}^{n} x_i^2$");
    accepted(r"\begin{align} x &= y \\ z &= w \end{align}");
    accepted(r"\begin{bmatrix} 1 & 0 \\ 0 & 1 \end{bmatrix}");
    accepted(r"100\% correct \{literal braces\} and \\ a line break");
    accepted(r"\definecolor{x}{RGB}{1,2,3}"); // starts with "def", is not \def
    accepted(r"\inputencoding{utf8}"); // starts with "input", is not \input
    accepted(r"\foreach \x in {1,...,5} { \draw (\x,0) circle (1pt); }");
}

#[test]
fn file_readers_may_not_name_a_path_outside_the_project() {
    // Absolute, home-relative, variable-expanded or `..` paths reach
    // files outside the project; a name built by a macro cannot be
    // checked here, so it is refused too.
    rejected(r"\includegraphics{/etc/passwd}");
    rejected(r"\includegraphics[width=\linewidth]{../../secret.png}");
    rejected(r"\includegraphics{ {/Users}/me/.ssh/id_rsa}");
    rejected(r"\includegraphics{~/Pictures/x}");
    rejected(r"\includegraphics{$HOME/x}");
    rejected(r"\includegraphics{figs/../../x}");
    rejected(r"\includegraphics{\string/etc/passwd}");
    rejected(r"\newcommand\p{/etc/passwd}\includegraphics\p");
    rejected(r"\pdfximage width 2pt {/etc/passwd}");
    rejected(r"\pdfobj file {/etc/passwd}");
    rejected(r"\pdfmdfivesum file {../x}");
    rejected(r"\verbatiminput{/etc/hosts}");
    rejected(r"\lstinputlisting[language=C]{/etc/hosts}");
    rejected(r"\inputminted{c}{/etc/hosts}");
    rejected(r"\import{/etc/}{passwd}");
    rejected(r"\InputIfFileExists{/etc/passwd}{}{}");
    rejected(r"\includepdf[pages=-]{../other.pdf}");
    // Project-relative paths stay accepted; \newcommand is allowed by
    // policy (Commander ruling).
    accepted(r"\includegraphics[width=0.5\linewidth]{figs/plot.pdf}");
    accepted(r"\includegraphics{plot}");
    accepted(r"\inputminted{c}{src/main.c}");
    accepted(r"\import{chapters/}{intro}");
    accepted(r"\newcommand{\R}{\mathbb{R}} \renewcommand{\vec}[1]{\mathbf{#1}}");
}
