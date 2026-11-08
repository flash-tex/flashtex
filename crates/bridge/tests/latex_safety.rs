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
fn caret_notation_cannot_spell_a_denied_control_word() {
    // TeX reduces `^^5c` to `\` and `^^69` to `i` while it reads input,
    // before a control sequence name is scanned, so these are `\input`,
    // `\immediate\write18` and `\end{document}` to the engine (pdflatex
    // runs the `\write` of the second spelling) although the denylist
    // never sees a backslash followed by the denied word.
    rejected(r"^^5cinput{/etc/passwd}");
    rejected(r"^^5cimmediate^^5cwr^^69te18{touch /tmp/pwned}");
    rejected(r"\in^^70ut{/etc/passwd}");
    rejected(r"\end{docu^^6dent}");
    // The one-character form (`^^` and any character, its code ± 64:
    // `^^I` is a tab) and a `^^` in a comment are refused as well; no
    // transcription needs two adjacent superscript characters.
    rejected(r"$x^^I$");
    rejected("% ^^5cinput\n x");
    // Ordinary superscripts are untouched.
    accepted(r"$x^2 + y^{2^k} + {}^1_2 + z^{^2}$");
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
fn denied_control_words_cannot_be_used_as_environment_names() {
    rejected(r"\begin{input}{secret.tex}\end{input}");
    rejected(r"\lowercase{\begin{INPUT}{SECRET.TEX}\end{INPUT}}");
    rejected(r"\uppercase{\begin{input}{secret.tex}\end{input}}");
    rejected(r"\lowercase{SAFE}");
    rejected(r"\uppercase{safe}");
    rejected(r"\begin{ExplSyntaxOn}x\end{ExplSyntaxOn}");
    rejected(r"\begin{makeatletter}x\end{makeatletter}");
    rejected(r"\begin{csname}x\end{csname}");
    rejected(r"\begin{endcsname}x\end{endcsname}");
    rejected(r"\end{input}");
    for name in ["itemize", "figure", "tikzpicture", "align*", "equation"] {
        accepted(&format!(r"\begin{{{name}}}x\end{{{name}}}"));
    }
}

#[test]
fn environment_names_cannot_reenter_begin_or_end() {
    rejected(r"\begin{begin}{input}{secret.tex}\end{begin}");
    rejected(r"\begin{end}{document}\end{end}");
}

#[test]
fn kernel_name_copy_commands_are_rejected() {
    rejected(r"\NewCommandCopy\b\begin{x}\b{input}{secret.tex}\iffalse\end{x}\fi");
    for command in [
        "RenewCommandCopy",
        "DeclareCommandCopy",
        "NewEnvironmentCopy",
        "RenewEnvironmentCopy",
        "DeclareEnvironmentCopy",
        "LetLtxMacro",
    ] {
        rejected(&format!(r"\{command}\b\a"));
    }
}

#[test]
fn truncation_commands_and_environment_names_are_rejected() {
    rejected(r"\enddocument");
    rejected(r"\endinput");
    rejected(r"\begin{enddocument}");
    rejected(r"\begin{endinput}");
    rejected(r"\begin{enddocument}x\end{enddocument}");
    rejected(r"\begin{endinput}x\end{endinput}");
}

#[test]
fn bare_carriage_return_ends_a_comment() {
    rejected("%c\r\\input{secret.tex}");
}

#[test]
fn indirect_file_access_and_object_commands_are_rejected() {
    rejected(r"\InputIfFileExists{secret.tex}{}{}");
    rejected(r"\IfFileExists{secret.tex}{}{}");
    rejected(r"\begin{filecontents}{secret.tex}x\end{filecontents}");
    rejected(r"\begin{filecontents*}{secret.tex}x\end{filecontents*}");
    rejected(r"\pdffiledump offset 0 length 10 {secret.tex}");
    rejected(r"\pdfobj{secret}");
}

#[test]
fn newline_before_denied_environment_name_is_rejected() {
    rejected("\\begin\n{input}{secret.tex}");
}

#[test]
fn comment_before_denied_environment_name_is_rejected() {
    rejected("\\begin%c\n{input}{secret.tex}");
}

#[test]
fn expanded_environment_name_with_interior_command_is_rejected() {
    rejected(r"\begin{in\empty put}{secret.tex}");
}

#[test]
fn expanded_environment_name_with_trailing_command_is_rejected() {
    rejected(r"\begin{input\empty}{secret.tex}");
}

#[test]
fn macro_that_emits_begin_without_a_name_is_rejected() {
    rejected(r"\newcommand\b{\begin}\b{input}{secret.tex}");
}

#[test]
fn make_lowercase_cannot_rewrite_an_environment_name() {
    rejected(r"\MakeLowercase{\begin{INPUT}{SECRET.TEX}}");
    rejected(r"\MakeUppercase{safe}");
}

#[test]
fn environment_name_allows_whitespace_and_comments_before_braces() {
    accepted("\\begin \t\r\n% comment\r\n{align*}x\\end% comment\n{align*}");
}

#[test]
fn malformed_environment_names_are_rejected() {
    for latex in [
        r"\begin",
        r"\begin{}",
        r"\begin{a-b}",
        r"\begin{a_b}",
        r"\end{a\empty}",
        r"\end% no name",
    ] {
        rejected(latex);
    }
}

#[test]
fn provides_expl_commands_cannot_enable_expl3_syntax() {
    rejected(
        r"\ProvidesExplFile{capture.tex}{2026/11/01}{1}{capture}\file_input:n{secret.tex}\ExplSyntaxOff",
    );
    rejected(r"\ProvidesExplPackage{capture}{2026/11/01}{1}{capture}");
    rejected(r"\ProvidesExplClass{capture}{2026/11/01}{1}{capture}");
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
    accepted(r"\begin{tabular}{ll}a & b\end{tabular}");
    accepted(r"\begin{minipage}[t]{0.5\textwidth}x\end{minipage}");
    accepted(r"\begin{figure}[htbp]x\end{figure}");
    accepted(r"\begin{align*}x &= y\end{align*}");
    accepted(r"\begin{multicols*}{2}x\end{multicols*}");
    accepted(r"\begingroup x\endgroup");
}

#[test]
fn control_sequence_names_built_or_rescanned_at_run_time_are_rejected() {
    // Each builds a denied control sequence without spelling it, as
    // `\csname` would; pdflatex runs the first, third and fourth forms
    // (checked against TeX Live 2026 with `\write16` for `\write18`).
    rejected(r"\UseName{immediate}\UseName{write}18{touch /tmp/pwned}");
    rejected(r"\ExpandArgs{c}\relax{input}{/etc/passwd}");
    rejected(r"\expandafter\scantokens\expandafter{\string\inpu t{/etc/passwd}}");
    // expl3 syntax makes `_` and `:` letters: `\sys_shell_now:n` and
    // `\file_input:n` are then single control words the scan cannot see.
    rejected(r"\ExplSyntaxOn \sys_shell_now:n{touch /tmp/pwned} \ExplSyntaxOff");
    // etoolbox's name builders (loaded by biblatex and many classes).
    rejected(r"\csuse{input}{/etc/passwd}");
    rejected(r"\cslet{x}\relax");
    rejected(r"\csdef{x}{y}");
    // eTeX's line read sits with `\read`.
    rejected(r"\readline0 to\x");
    // Lookalikes stay accepted.
    accepted(r"\readlines \scantoken \Usename \csuses");
}
