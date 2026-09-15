//! Structured diagnostic codes (issue #76): typos are `unknown_command` with a
//! did-you-mean suggestion, real-but-unimplemented LaTeX is
//! `unsupported_feature`, and both survive into runtime-v1 JSON.

use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::protocol::handle_line;

fn diagnostics(text: &str) -> Vec<(String, Option<DiagnosticCode>, Option<String>)> {
    compile_full(text, LayoutConstraints::default())
        .diagnostics
        .into_iter()
        .map(|d| (d.message, d.code, d.suggestion))
        .collect()
}

fn only(text: &str, needle: &str) -> (Option<DiagnosticCode>, Option<String>) {
    let all = diagnostics(text);
    let matching: Vec<_> = all.iter().filter(|(m, ..)| m.contains(needle)).collect();
    assert_eq!(matching.len(), 1, "{text}: {all:?}");
    (matching[0].1, matching[0].2.clone())
}

#[test]
fn typos_are_unknown_commands_with_suggestions() {
    for (text, needle, suggestion) in [
        (r"Text \alpah here.", r"\alpah", r"\alpha"),
        (r"Math $x + \alpah$ here.", r"\alpah", r"\alpha"),
        (r"Text \textbff{bold} here.", r"\textbff", r"\textbf"),
        (
            "\\documentclass{article}\n\\usepackge{amsmath}\n\\begin{document}Body\\end{document}",
            r"\usepackge",
            r"\usepackage",
        ),
    ] {
        assert_eq!(
            only(text, needle),
            (
                Some(DiagnosticCode::UnknownCommand),
                Some(suggestion.to_string())
            ),
            "{text}"
        );
    }
    assert_eq!(
        only(r"Text \frobnicate{x} here.", r"\frobnicate"),
        (Some(DiagnosticCode::UnknownCommand), None)
    );
}

#[test]
fn real_unimplemented_latex_is_an_unsupported_feature() {
    for (text, needle) in [
        (r"Text \tikz here.", r"\tikz"),
        // `\mathcal` is supported since FT-060 (New Computer Modern Math).
        // `\mathfrak` is supported too (Unicode mathematical fraktur).
        (r"Math $\mathscr{A}$ here.", r"\mathscr"),
        (
            r"\tikz \begin{document}Visible\end{document}",
            r"\tikz is not supported in the document preamble",
        ),
        (
            r"Visible \begin{tabbing}body\end{tabbing} Tail.",
            "environment 'tabbing'",
        ),
    ] {
        assert_eq!(
            only(text, needle),
            (Some(DiagnosticCode::UnsupportedFeature), None),
            "{text}"
        );
    }
    assert_eq!(
        only(
            r"Visible \begin{itemze}body\end{itemze} Tail.",
            "environment 'itemze'"
        )
        .0,
        Some(DiagnosticCode::UnknownCommand)
    );
}

#[test]
fn malformed_input_is_a_syntax_error() {
    for (text, needle) in [
        ("Visible {tail.", "unmatched '{'"),
        ("Visible } tail.", "unmatched '}'"),
        (r"\begin{document}Visible", "unterminated environment"),
        ("Math $a & b$ here.", "misplaced alignment tab"),
        (r"Visible \frac{a}{b} Tail.", "requires math mode"),
    ] {
        assert_eq!(
            only(text, needle).0,
            Some(DiagnosticCode::SyntaxError),
            "{text}"
        );
    }
    assert_eq!(
        only(r"See \ref{missing}.", "undefined").0,
        Some(DiagnosticCode::RecoveredInput)
    );
}

#[test]
fn every_compile_diagnostic_in_a_mixed_document_has_a_code() {
    let text = "\\documentclass{article}\n\\usepackage{tikz}\n\\setlength{\\parindent}{0pt}\n\
                \\begin{document}\n\\section{A} {open \\alpah \\tikz $\\bogus x^ & \\hat{ab}$ \
                \\ref{nope} \\label{k}\\label{k} \\input{missing} \\includegraphics{x} \
                \\begin{tabbing}t\\end{tabbing} \\end{itemize}\n\\end{document}\n";
    let all = diagnostics(text);
    assert!(all.len() >= 8, "{all:?}");
    for (message, code, _) in &all {
        assert!(code.is_some(), "no code for {message:?}");
    }
}

#[test]
fn codes_and_suggestions_are_serialized_in_runtime_v1_json() {
    let mut document = Value::obj();
    document.set("path", json::str_("main.tex"));
    document.set("text", json::str_(r"Text \alpah and \tikz here."));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("codes"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![document]));
    let mut envelope = Value::obj();
    envelope.set("protocol_version", Value::Num(1.0));
    envelope.set("id", json::str_("codes"));
    envelope.set("type", json::str_("compile"));
    envelope.set("payload", payload);

    let response = json::parse(&handle_line(&json::write(&envelope))).expect("valid JSON");
    let diagnostics = response
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(Value::as_arr)
        .expect("diagnostics array");
    let field = |d: &Value, key: &str| d.get(key).and_then(Value::as_str).map(str::to_string);
    let rows: Vec<_> = diagnostics
        .iter()
        .map(|d| (field(d, "code"), field(d, "suggestion")))
        .collect();
    assert_eq!(
        rows,
        vec![
            (Some("unknown_command".into()), Some(r"\alpha".into())),
            (Some("unsupported_feature".into()), None),
        ]
    );
    // Absent, not null, when there is no suggestion: older decoders never see
    // a new null-valued key.
    assert!(diagnostics[1].get("suggestion").is_none());
    assert!(diagnostics[0].get("help").is_some(), "{:?}", diagnostics[0]);
    assert!(diagnostics[1].get("help").is_some(), "{:?}", diagnostics[1]);
    let help = diagnostics[0].get("help").expect("help");
    assert_eq!(
        help.get("message").and_then(Value::as_str),
        Some("did you mean \\alpha?")
    );
    let replacement = help.get("replacement").expect("replacement");
    assert_eq!(
        replacement.get("text").and_then(Value::as_str),
        Some(r"\alpha")
    );
    let source = replacement.get("source").expect("source");
    assert_eq!(source.get("path").and_then(Value::as_str), Some("main.tex"));
    let start = source.get("start_byte").and_then(Value::as_i64).unwrap() as usize;
    let end = source.get("end_byte").and_then(Value::as_i64).unwrap() as usize;
    let text = r"Text \alpah and \tikz here.";
    assert_eq!(&text[start..end], r"\alpah", "{start}..{end} in {text:?}");
}

#[test]
fn high_frequency_messages_carry_help() {
    let cases = [
        (
            r"Visible \frobnicate{argument} Tail.",
            r"\frobnicate is not supported",
        ),
        (r"Visible \tikz Tail.", r"\tikz is not supported"),
        (
            r"\usepackage{tikz} Visible.",
            "recognised but not implemented",
        ),
        ("Visible $x+1", "missing its closing '$'"),
        ("Visible {tail.", "unmatched '{'"),
        (r"Visible \usepackage[broken", "missing its closing ']'"),
        (
            "Visible ^ Tail.",
            "math script marker used outside math mode",
        ),
        (r"Visible \frac{a}{b} Tail.", r"\frac requires math mode"),
        (r"See \ref{missing}.", "undefined"),
        (
            r"Visible \input{chapter.tex} Tail.",
            "included file not found",
        ),
        (
            r"Visible \begin{tikzpicture}body\end{tikzpicture} Tail.",
            "environment 'tikzpicture'",
        ),
        (
            "\\documentclass{article}\\title{T}\\begin{document}\\maketitle\\end{document}",
            r"No \author given",
        ),
        (
            r"\tikz \begin{document}Visible\end{document}",
            "not supported in the document preamble",
        ),
    ];
    for (text, needle) in cases {
        let matching: Vec<_> = compile_full(text, LayoutConstraints::default())
            .diagnostics
            .into_iter()
            .filter(|d| d.message.contains(needle))
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "{text}: {:?}",
            matching.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
        assert!(
            matching[0].help.is_some(),
            "{needle} has no help: {:?}",
            matching[0].message
        );
    }
}

#[test]
fn math_mode_help_is_real_or_absent() {
    let text_cmd = compile_full(r"Math $\centering$ here.", LayoutConstraints::default());
    let centering: Vec<_> = text_cmd
        .diagnostics
        .iter()
        .filter(|d| {
            d.message
                .contains(r"\centering is not supported in math mode")
        })
        .collect();
    assert_eq!(centering.len(), 1, "{:?}", text_cmd.diagnostics);
    assert_eq!(
        centering[0].help.as_ref().map(|h| h.message.as_str()),
        Some(r"\centering is a text command; use it outside math or inside \text{...}")
    );

    let unknown = compile_full(r"Math $\bogusxyz$ here.", LayoutConstraints::default());
    let bogus: Vec<_> = unknown
        .diagnostics
        .iter()
        .filter(|d| {
            d.message
                .contains(r"\bogusxyz is not supported in math mode")
        })
        .collect();
    assert_eq!(bogus.len(), 1, "{:?}", unknown.diagnostics);
    assert!(
        bogus[0].help.is_none(),
        "restating help: {:?}",
        bogus[0].help
    );

    let tabbing = compile_full(
        r"Visible \begin{tabbing}body\end{tabbing} Tail.",
        LayoutConstraints::default(),
    );
    let env: Vec<_> = tabbing
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("environment 'tabbing'"))
        .collect();
    assert_eq!(env.len(), 1, "{:?}", tabbing.diagnostics);
    assert!(env[0].help.is_none(), "restating help: {:?}", env[0].help);
}

#[test]
fn undefined_ref_help_points_at_the_ref_key() {
    let near = compile_full(r"See \ref{s2}. \label{s1}", LayoutConstraints::default());
    let matching: Vec<_> = near
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("undefined"))
        .collect();
    assert_eq!(matching.len(), 1, "{:?}", near.diagnostics);
    assert_eq!(
        matching[0].help.as_ref().map(|h| h.message.as_str()),
        Some("a label `s1` exists; did you mean \\ref{s1}?")
    );

    let none = compile_full(r"See \ref{missing}.", LayoutConstraints::default());
    let matching: Vec<_> = none
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("undefined"))
        .collect();
    assert_eq!(matching.len(), 1, "{:?}", none.diagnostics);
    assert_eq!(
        matching[0].help.as_ref().map(|h| h.message.as_str()),
        Some("add a matching \\label{...} or fix the key; undefined references render as ??")
    );
}
