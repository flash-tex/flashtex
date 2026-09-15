//! Exhaustive FT-006 recovery and incremental-equivalence evidence.

use flashtex_compiler::incremental::{compile_full, Session};
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::protocol::{handle_request_bytes, MAX_LINE_BYTES};
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Clone, Copy)]
struct SourceCase {
    name: &'static str,
    input: &'static str,
    message: &'static str,
}

const SOURCE_CASES: &[SourceCase] = &[
    SourceCase {
        name: "unmatched open brace",
        input: "Visible {tail.",
        message: "unmatched '{'",
    },
    SourceCase {
        name: "stray closing brace",
        input: "Visible } tail.",
        message: "unmatched '}'",
    },
    SourceCase {
        name: "unterminated environment",
        input: r"\begin{document}Visible",
        message: "unterminated environment 'document'",
    },
    SourceCase {
        name: "mismatched end",
        input: r"Visible \begin{tabbing}body\end{itemize} Tail.",
        message: r"does not match \begin{tabbing}",
    },
    SourceCase {
        name: "stray end",
        input: r"Visible \end{tabbing} Tail.",
        message: r"no matching \begin",
    },
    SourceCase {
        name: "unknown command",
        input: r"Visible \frobnicate{argument} Tail.",
        message: r"\frobnicate is not supported",
    },
    SourceCase {
        // \input is implemented now; the recoverable failure is a file the
        // request did not supply. The diagnostic must name what it looked for,
        // because "not found" without the attempted paths is unactionable.
        name: "include of a file the request did not supply",
        input: r"Visible \input{chapter.tex} Tail.",
        message: r"included file not found: looked for 'chapter.tex' and 'chapter.tex.tex'",
    },
    SourceCase {
        name: "math command outside math mode",
        input: r"Visible \frac{a}{b} Tail.",
        message: r"\frac requires math mode",
    },
    SourceCase {
        name: "unsupported preamble command",
        input: r"\tikz \begin{document}Visible\end{document}",
        message: r"\tikz is not supported in the document preamble",
    },
    SourceCase {
        name: "unimplemented environment",
        input: r"Visible \begin{picture}body\end{picture} Tail.",
        message: "environment 'picture' is not implemented",
    },
    SourceCase {
        name: "missing required command argument",
        input: r"Visible \textbf Tail.",
        message: r"\textbf requires a braced argument",
    },
    SourceCase {
        name: "missing required heading argument",
        input: "Visible.\n\n\\section",
        message: r"\section requires a braced argument",
    },
    SourceCase {
        name: "required argument missing closing brace",
        input: r"Visible \textbf{Tail",
        message: r"argument to \textbf is missing its closing brace",
    },
    SourceCase {
        name: "optional argument missing closing bracket",
        input: r"Visible \usepackage[broken",
        message: "optional argument is missing its closing ']'",
    },
    SourceCase {
        name: "empty document class",
        input: r"\documentclass{} Visible.",
        message: r"\documentclass was given an empty argument",
    },
    SourceCase {
        name: "empty package list",
        input: r"\usepackage{} Visible.",
        message: r"\usepackage was given an empty package list",
    },
    SourceCase {
        name: "unsupported package",
        input: r"\usepackage{tikz} Visible.",
        message: "packages tikz are recognised but not implemented",
    },
    SourceCase {
        name: "invalid macro name",
        input: r"Visible \newcommand{oops}{body} Tail.",
        message: "Missing control sequence inserted.",
    },
    SourceCase {
        name: "invalid macro argument count",
        input: r"\newcommand{\x}[10]{x} Visible.",
        message: "You already have nine parameters.",
    },
    SourceCase {
        name: "newcommand redefines existing command",
        input: r"\newcommand{\section}{x} Visible.",
        message: r"LaTeX Error: Command \section already defined.",
    },
    SourceCase {
        name: "renewcommand targets undefined command",
        input: r"\renewcommand{\missing}{x} Visible.",
        message: r"LaTeX Error: Command \missing undefined.",
    },
    SourceCase {
        name: "macro recursion limit",
        input: r"\newcommand{\recurse}{\recurse} Visible \recurse Tail.",
        message: "expansion step limit exceeded",
    },
    SourceCase {
        name: "undeclared macro replacement parameter",
        input: r"\newcommand{\oops}{#1} Visible \oops Tail.",
        message: r"Illegal parameter number in definition of \oops.",
    },
    SourceCase {
        name: "stray display math close",
        input: r"Visible \] Tail.",
        message: r"stray \]",
    },
    SourceCase {
        name: "math script outside math mode",
        input: "Visible ^ Tail.",
        message: "math script marker used outside math mode",
    },
    SourceCase {
        name: "unclosed inline math",
        input: "Visible $x+1",
        message: "inline math is missing its closing '$'",
    },
    SourceCase {
        name: "unclosed dollar display math",
        input: "Visible $$x+1",
        message: "display math is missing its closing delimiter",
    },
    SourceCase {
        name: "unclosed bracket display math",
        input: r"Visible \[x+1",
        message: "display math is missing its closing delimiter",
    },
    SourceCase {
        name: "unclosed inline math ends at its paragraph",
        input: "Visible $x+1\n\nTail $y$.",
        message: "inline math is missing its closing '$'",
    },
    SourceCase {
        name: "unclosed math group in unclosed inline math ends at its paragraph",
        input: "Visible $x^2 + \\frac{a\n\nTail $y$.",
        message: "'{' opened here is not closed before the end of the paragraph",
    },
    SourceCase {
        name: "unclosed display math group ends at its paragraph",
        input: "Visible \\[x+\\frac{a\n\nTail $y$.",
        message: "'{' opened here is not closed before the end of the paragraph",
    },
    SourceCase {
        name: "unclosed inline math ends before a block environment",
        input: "Visible $x\n\\begin{itemize}\\item Tail $y$.\\end{itemize}",
        message: "inline math is missing its closing '$'",
    },
    SourceCase {
        name: "required argument missing closing brace ends at its paragraph",
        input: "Visible \\textbf{Tail\n\nNext $y$.",
        message: r"argument to \textbf is missing its closing brace",
    },
    SourceCase {
        name: "unterminated display environment ends at its paragraph",
        input: "Visible \\begin{align} x\n\nTail $y$.",
        message: "unterminated environment 'align'",
    },
    SourceCase {
        name: "unmatched math closing brace",
        input: "Visible $a}b$ Tail.",
        message: "unmatched '}' in math mode",
    },
    SourceCase {
        name: "duplicate math script",
        input: "Visible $x^a^b$ Tail.",
        message: "duplicate script on a math atom",
    },
    SourceCase {
        name: "unattached math script",
        input: "Visible $^a$ Tail.",
        message: "script marker has no preceding math atom",
    },
    SourceCase {
        name: "math group missing closing brace",
        input: "Visible $x^{a$ Tail.",
        message: "math group is missing its closing brace",
    },
    SourceCase {
        name: "math script missing argument",
        input: "Visible $x^$ Tail.",
        message: "math script is missing its argument",
    },
    SourceCase {
        name: "unexpected nested math delimiter",
        input: r"Visible $a\[b$ Tail.",
        message: "unexpected math delimiter inside math mode",
    },
    SourceCase {
        name: "unknown math command",
        input: r"Visible $x+\bogus$ Tail.",
        message: r"\bogus is not supported in math mode",
    },
    SourceCase {
        // `\frac a{b}` is now valid (TeX takes the next single token as an
        // undelimited argument, so the numerator is just `a`); a stray `^`
        // where the denominator belongs is still a real missing-argument error.
        name: "missing math argument",
        input: r"Visible $x+\frac{a}^2$ Tail.",
        message: r"\frac requires an argument",
    },
];

#[derive(Clone)]
struct WireCase {
    name: &'static str,
    input_description: String,
    bytes: Vec<u8>,
    expected: &'static str,
    message: &'static str,
    source_text: Option<&'static str>,
    source_mappable: bool,
    expect_text: bool,
}

fn compile_request(path: &str, text: &str) -> String {
    compile_request_with(path, vec![(path, text)])
}

fn compile_request_with(entry: &str, documents: Vec<(&str, &str)>) -> String {
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("recovery-evidence"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(entry));
    payload.set(
        "documents",
        Value::Arr(
            documents
                .into_iter()
                .map(|(path, text)| {
                    let mut document = Value::obj();
                    document.set("path", json::str_(path));
                    document.set("text", json::str_(text));
                    document
                })
                .collect(),
        ),
    );
    let mut envelope = Value::obj();
    envelope.set("protocol_version", Value::Num(1.0));
    envelope.set("id", json::str_("recovery"));
    envelope.set("type", json::str_("compile"));
    envelope.set("payload", payload);
    json::write(&envelope)
}

fn wire_cases() -> Vec<WireCase> {
    // Supplying an unreferenced extra document is no longer a failure: multi-file
    // projects compile. The recoverable wire failure is now an \input naming a
    // file the request did not supply, with the other document present to prove
    // the compiler looked and did not merely ignore the whole set.
    let multi = compile_request_with(
        "main.tex",
        vec![
            ("main.tex", "Visible main document. \\input{absent}"),
            ("chapter.tex", "Other."),
        ],
    );
    let absolute = compile_request("/absolute.tex", "Visible.");
    let traversal = compile_request_with("../outside.tex", vec![("main.tex", "Visible.")]);
    let no_documents = compile_request_with("main.tex", Vec::new());
    let oversized = vec![b'x'; MAX_LINE_BYTES + 1];
    vec![
        WireCase {
            name: "include naming a document the request did not supply",
            input_description: multi.clone(),
            bytes: multi.into_bytes(),
            expected: "recovered",
            message: "included file not found",
            source_text: Some(r"Visible main document. \input{absent}"),
            source_mappable: false,
            expect_text: true,
        },
        WireCase {
            name: "absolute document path",
            input_description: absolute.clone(),
            bytes: absolute.into_bytes(),
            expected: "failed",
            message: "/absolute.tex",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "parent traversal entry path",
            input_description: traversal.clone(),
            bytes: traversal.into_bytes(),
            expected: "failed",
            message: "../outside.tex",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "no documents",
            input_description: no_documents.clone(),
            bytes: no_documents.into_bytes(),
            expected: "failed",
            message: "no documents supplied",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "malformed JSON request",
            input_description: "{not json".into(),
            bytes: b"{not json".to_vec(),
            expected: "error",
            message: "expected",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "unknown protocol version",
            input_description:
                r#"{"protocol_version":99,"id":"bad-version","type":"compile","payload":{}}"#.into(),
            bytes: br#"{"protocol_version":99,"id":"bad-version","type":"compile","payload":{}}"#
                .to_vec(),
            expected: "error",
            message: "protocol version 99",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "missing protocol version",
            input_description: r#"{"id":"missing-version","type":"compile","payload":{}}"#.into(),
            bytes: br#"{"id":"missing-version","type":"compile","payload":{}}"#.to_vec(),
            expected: "error",
            message: "protocol_version is required",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "unknown message type",
            input_description: r#"{"protocol_version":1,"id":"bad-type","type":"mystery"}"#.into(),
            bytes: br#"{"protocol_version":1,"id":"bad-type","type":"mystery"}"#.to_vec(),
            expected: "error",
            message: "message type 'mystery'",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "missing message type",
            input_description: r#"{"protocol_version":1,"id":"missing-type"}"#.into(),
            bytes: br#"{"protocol_version":1,"id":"missing-type"}"#.to_vec(),
            expected: "error",
            message: "type is required",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "missing compile payload",
            input_description: r#"{"protocol_version":1,"id":"missing-payload","type":"compile"}"#
                .into(),
            bytes: br#"{"protocol_version":1,"id":"missing-payload","type":"compile"}"#.to_vec(),
            expected: "error",
            message: "compile requires a payload",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "oversized request line",
            input_description: format!("<exactly {} ASCII 'x' bytes>", MAX_LINE_BYTES + 1),
            bytes: oversized,
            expected: "error",
            message: "8388608-byte limit",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
        WireCase {
            name: "invalid UTF-8 request line",
            input_description: "<hex ff>".into(),
            bytes: vec![0xff],
            expected: "error",
            message: "not valid UTF-8",
            source_text: None,
            source_mappable: false,
            expect_text: false,
        },
    ]
}

fn response_status(response: &Value) -> &str {
    if response.get("type").and_then(Value::as_str) == Some("error") {
        "error"
    } else {
        response
            .get("payload")
            .and_then(|payload| payload.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("")
    }
}

fn diagnostics(response: &Value) -> &[Value] {
    response
        .get("payload")
        .and_then(|payload| payload.get("diagnostics"))
        .and_then(Value::as_arr)
        .map_or(&[], Vec::as_slice)
}

fn items(response: &Value) -> Vec<&Value> {
    response
        .get("payload")
        .and_then(|payload| payload.get("pages"))
        .and_then(Value::as_arr)
        .into_iter()
        .flatten()
        .flat_map(|page| {
            page.get("items")
                .and_then(Value::as_arr)
                .into_iter()
                .flatten()
        })
        .collect()
}

fn assert_diagnostics(
    name: &str,
    response: &Value,
    needle: &str,
    source_text: Option<&str>,
    source_mappable: bool,
) {
    let found = diagnostics(response);
    assert!(!found.is_empty(), "{name}: compiler emitted no diagnostic");
    assert!(
        found.iter().any(|diagnostic| diagnostic
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains(needle))),
        "{name}: no diagnostic named {needle:?}; got {found:?}"
    );
    for diagnostic in found {
        let recovery = diagnostic.get("recovery").unwrap_or_else(|| {
            panic!("{name}: diagnostic is missing its recovery field: {diagnostic:?}")
        });
        assert!(
            recovery == &Value::Null
                || recovery
                    .as_str()
                    .is_some_and(|description| !description.is_empty()),
            "{name}: recovery must be null or a real description: {diagnostic:?}"
        );
        let source = diagnostic.get("source").unwrap_or_else(|| {
            panic!("{name}: diagnostic is missing its source field: {diagnostic:?}")
        });
        if source_mappable {
            assert_ne!(
                source,
                &Value::Null,
                "{name}: diagnostic is not source-mapped"
            );
        }
        if source != &Value::Null {
            let text = source_text.unwrap_or_else(|| panic!("{name}: no source text for range"));
            let start = source.get("start_byte").and_then(Value::as_i64).unwrap() as usize;
            let end = source.get("end_byte").and_then(Value::as_i64).unwrap() as usize;
            assert!(
                text.get(start..end).is_some(),
                "{name}: diagnostic range {start}..{end} does not slice input of {} bytes",
                text.len()
            );
        }
    }
}

fn run(bytes: &[u8], name: &str) -> Value {
    let reply = std::panic::catch_unwind(|| handle_request_bytes(bytes))
        .unwrap_or_else(|_| panic!("{name}: compiler panicked instead of replying"));
    assert!(!reply.is_empty(), "{name}: compiler returned no reply");
    json::parse(&reply).unwrap_or_else(|error| panic!("{name}: invalid reply JSON: {}", error.0))
}

#[test]
fn every_source_recovery_class_replies_with_positioned_output_and_mapped_diagnostics() {
    for case in SOURCE_CASES {
        let request = compile_request("main.tex", case.input);
        let response = run(request.as_bytes(), case.name);
        assert_eq!(
            response_status(&response),
            "recovered",
            "{}: wrong status",
            case.name
        );
        assert_diagnostics(case.name, &response, case.message, Some(case.input), true);
        assert!(
            !items(&response).is_empty(),
            "{}: no positioned text survived recovery",
            case.name
        );
    }
}

#[test]
fn every_wire_failure_replies_with_exact_status_and_explicit_diagnostics() {
    for case in wire_cases() {
        let response = run(&case.bytes, case.name);
        assert_eq!(
            response_status(&response),
            case.expected,
            "{}: wrong status",
            case.name
        );
        assert_diagnostics(
            case.name,
            &response,
            case.message,
            case.source_text,
            case.source_mappable,
        );
        assert_eq!(
            !items(&response).is_empty(),
            case.expect_text,
            "{}: wrong positioned-output presence",
            case.name
        );
        if case.expected == "recovered" {
            let text = case
                .source_text
                .unwrap_or_else(|| panic!("{}: recoverable case has no source text", case.name));
            let mut session = Session::new();
            assert_session_equivalent(&mut session, case.name, text);
            assert_session_equivalent(&mut session, case.name, text);
        }
    }
}

fn output_bytes(text: &str) -> Vec<u8> {
    format!("{:#?}", compile_full(text, LayoutConstraints::default())).into_bytes()
}

fn assert_session_equivalent(session: &mut Session, name: &str, text: &str) {
    let incremental = session.compile(text, LayoutConstraints::default());
    assert_eq!(
        format!("{:#?}", incremental.output).into_bytes(),
        output_bytes(text),
        "{name}: incremental output differs byte-for-byte from compile_full"
    );
}

#[test]
fn every_source_recovery_is_byte_identical_incrementally() {
    for case in SOURCE_CASES {
        let mut session = Session::new();
        assert_session_equivalent(&mut session, case.name, case.input);
        assert_session_equivalent(&mut session, case.name, case.input);
    }
}

#[test]
fn broken_and_fixed_edits_remain_byte_identical_in_both_directions() {
    let transitions = [
        ("brace", "Lead {broken\n\nTail.", "Lead {fixed}\n\nTail."),
        (
            "command",
            r"Lead \oops{shown}\n\nTail.",
            r"Lead \textbf{shown}\n\nTail.",
        ),
        ("inline math", "Lead $x+1\n\nTail.", "Lead $x+1$\n\nTail."),
        (
            "math group",
            "Lead $x^2 + \\frac{a\n\nTail $y$.",
            "Lead $x^2 + \\frac{a}{b}$\n\nTail $y$.",
        ),
        (
            "paragraph argument",
            "Lead \\textbf{abc\n\nTail.",
            "Lead \\textbf{abc}\n\nTail.",
        ),
        (
            "environment",
            r"\begin{document}Body",
            r"\begin{document}Body\end{document}",
        ),
        ("required argument", r"Lead \textbf", r"Lead \textbf{fixed}"),
    ];
    for (name, broken, fixed) in transitions {
        for revisions in [[broken, fixed, broken], [fixed, broken, fixed]] {
            let mut session = Session::new();
            for text in revisions {
                assert_session_equivalent(&mut session, name, text);
            }
        }
    }
}

fn markdown_section(out: &mut String, name: &str, input: &str, response: &Value) {
    writeln!(out, "## {name}\n").unwrap();
    writeln!(out, "Input:\n\n```text\n{input}\n```\n").unwrap();
    writeln!(out, "Status: `{}`\n", response_status(response)).unwrap();
    writeln!(out, "Diagnostics:\n").unwrap();
    for diagnostic in diagnostics(response) {
        let message = diagnostic.get("message").and_then(Value::as_str).unwrap();
        let recovery = match diagnostic.get("recovery").unwrap() {
            Value::Null => "null",
            value => value.as_str().unwrap(),
        };
        let range = match diagnostic.get("source").unwrap() {
            Value::Null => "null".to_string(),
            source => format!(
                "{}..{}",
                source.get("start_byte").and_then(Value::as_i64).unwrap(),
                source.get("end_byte").and_then(Value::as_i64).unwrap()
            ),
        };
        writeln!(
            out,
            "- `{message}` — recovery: `{recovery}`; byte range: `{range}`"
        )
        .unwrap();
    }
    writeln!(out, "\nPositioned text items:\n").unwrap();
    let positioned = items(response);
    if positioned.is_empty() {
        writeln!(out, "- none\n").unwrap();
    } else {
        for item in positioned {
            let text = item.get("text").and_then(Value::as_str).unwrap();
            let source = item.get("source").unwrap();
            let start = source.get("start_byte").and_then(Value::as_i64).unwrap();
            let end = source.get("end_byte").and_then(Value::as_i64).unwrap();
            writeln!(out, "- `{text}` — byte range `{start}..{end}`").unwrap();
        }
        out.push('\n');
    }
}

fn evidence_body() -> String {
    let mut out = String::from("# FlashTeX recovery evidence\n\n");
    out.push_str("Generated from the real compiler by the command above. Status, diagnostics, recovery notes, ranges, and positioned items are observed rather than handwritten.\n\n");
    out.push_str("Locality policy: recovery stays inside the paragraph that contains the error, mirroring TeX's runaway-argument and `Missing $ inserted` behaviour. An unterminated `$`, `$$` or `\\[` math span, an unterminated display environment (`equation`, `align`, ...) and an unclosed command argument (`\\section{`, `\\label{`, or a `\\textbf{` with no matching `}` anywhere later) are closed at the end of their paragraph: a blank line, `\\par`, `\\item`, `\\section`/`\\subsection`, or `\\begin`/`\\end` of an environment that is not typeset inside math. A math group left open inside math closes at the math delimiter, or with the math at the end of the paragraph; either way one primary diagnostic names the innermost unclosed `{`, and arguments that could not be read because that group swallowed the rest of the math are not reported again. Arguments of `\\newcommand` macros and macro bodies are long, so they may span paragraphs; only when never closed at all are they closed at the end of their first paragraph. A bare `{` group still extends to its matching `}` or the end of input, because groups spanning paragraphs are valid TeX and only scope style and macro definitions. Everything after the paragraph lays out exactly as in the balanced document (`tests/local_recovery.rs`).\n\n");
    for case in SOURCE_CASES {
        let request = compile_request("main.tex", case.input);
        markdown_section(
            &mut out,
            case.name,
            case.input,
            &run(request.as_bytes(), case.name),
        );
    }
    for case in wire_cases() {
        markdown_section(
            &mut out,
            case.name,
            &case.input_description,
            &run(&case.bytes, case.name),
        );
    }
    out
}

fn evidence_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("RECOVERY.md")
}

#[test]
fn recovery_evidence_is_current() {
    let actual = std::fs::read_to_string(evidence_path()).expect("RECOVERY.md must be committed");
    let (_, body) = actual
        .split_once('\n')
        .expect("RECOVERY.md provenance line");
    assert_eq!(body.trim_start(), evidence_body());
}

#[test]
#[ignore = "regenerates the committed recovery evidence"]
fn generate_recovery_evidence() {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("git rev-parse must run");
    assert!(commit.status.success(), "git rev-parse HEAD failed");
    let commit = String::from_utf8(commit.stdout).expect("commit SHA is UTF-8");
    let command = "cargo test --test recovery generate_recovery_evidence -- --ignored --exact";
    let contents = format!(
        "Generated from commit `{}` by `{}`.\n\n{}",
        commit.trim(),
        command,
        evidence_body()
    );
    std::fs::write(evidence_path(), contents).expect("write RECOVERY.md");
}
