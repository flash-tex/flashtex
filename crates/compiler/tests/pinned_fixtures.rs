//! Raw compile-result evidence. These comparisons deliberately do not parse,
//! normalize, round, reorder, or otherwise rewrite either byte stream.
//!
//! Regenerate with:
//! `cargo test --test pinned_fixtures regenerate_pinned_fixtures -- --ignored --exact`

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const REGEN_COMMAND: &str =
    "cargo test --test pinned_fixtures regenerate_pinned_fixtures -- --ignored --exact";

struct Fixture {
    id: &'static str,
    entry: &'static str,
    documents: &'static [(&'static str, &'static str)],
}

const FIXTURES: &[Fixture] = &[
    Fixture {
        id: "plain-text",
        entry: "main.tex",
        documents: &[("main.tex", "Plain text in a paragraph.\n")],
    },
    Fixture {
        id: "heading",
        entry: "main.tex",
        documents: &[("main.tex", "\\section{Shaped Heading}\nBody text.\n")],
    },
    Fixture {
        id: "inline-display-math",
        entry: "main.tex",
        documents: &[(
            "main.tex",
            "Inline $x^2 + \\alpha$. Display: \\[\\frac{a}{b}=\\sqrt{x}\\]\n",
        )],
    },
    Fixture {
        id: "macro",
        entry: "main.tex",
        documents: &[(
            "main.tex",
            "\\newcommand{\\pair}[2]{#1 meets #2}\n\\pair{left}{right}\n",
        )],
    },
    Fixture {
        id: "include",
        entry: "main.tex",
        documents: &[
            ("main.tex", "Before. \\input{part} After.\n"),
            ("part.tex", "Included café.\n"),
        ],
    },
    Fixture {
        id: "forward-reference",
        entry: "main.tex",
        documents: &[(
            "main.tex",
            "See Section \\ref{later}.\n\n\\section{Later}\\label{later}\n",
        )],
    },
    Fixture {
        id: "kerning-pairs",
        entry: "main.tex",
        documents: &[("main.tex", "AV Wa To Ty\n")],
    },
];

fn request(fixture: &Fixture, capabilities: bool) -> String {
    let mut documents = Vec::new();
    for (path, text) in fixture.documents {
        let mut document = Value::obj();
        document.set("path", json::str_(*path));
        document.set("text", json::str_(*text));
        documents.push(document);
    }
    let mode = if capabilities { "caps" } else { "base" };
    let mut payload = Value::obj();
    payload.set(
        "project_id",
        json::str_(format!("pinned-{mode}-{}", fixture.id)),
    );
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(fixture.entry));
    payload.set("documents", Value::Arr(documents));
    if capabilities {
        payload.set(
            "layout_capabilities",
            Value::Arr(vec![json::str_("rules-v1"), json::str_("font-hints-v1")]),
        );
    }
    let mut envelope = Value::obj();
    envelope.set("protocol_version", Value::Num(1.0));
    envelope.set("id", json::str_(format!("{mode}-{}", fixture.id)));
    envelope.set("type", json::str_("compile"));
    envelope.set("payload", payload);
    json::write(&envelope)
}

fn evidence(capabilities: bool) -> Vec<u8> {
    let mode = if capabilities {
        "rules-v1 + font-hints-v1"
    } else {
        "no capabilities"
    };
    let mut output =
        format!("# Full compile_result JSON bytes; mode: {mode}\n# Regenerate: {REGEN_COMMAND}\n");
    for fixture in FIXTURES {
        let response = handle_line(&request(fixture, capabilities));
        writeln!(output, "{response}").expect("write to String");
    }
    output.into_bytes()
}

fn evidence_path(capabilities: bool) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(if capabilities {
        "tests/pinned/layout-capabilities.compile-results.jsonl"
    } else {
        "tests/pinned/no-capabilities.compile-results.jsonl"
    })
}

fn context(bytes: &[u8], offset: usize) -> String {
    let start = offset.saturating_sub(16);
    let end = bytes.len().min(offset.saturating_add(17));
    String::from_utf8_lossy(&bytes[start..end])
        .escape_debug()
        .to_string()
}

fn assert_exact_bytes(label: &str, expected: &[u8], actual: &[u8]) {
    if expected == actual {
        return;
    }
    let mut report = format!(
        "{label}: raw evidence differs (expected {} bytes, actual {} bytes); every differing byte offset follows:\n",
        expected.len(),
        actual.len()
    );
    for offset in 0..expected.len().max(actual.len()) {
        let expected_byte = expected.get(offset).copied();
        let actual_byte = actual.get(offset).copied();
        if expected_byte != actual_byte {
            writeln!(
                report,
                "offset {offset}: expected={expected_byte:?} actual={actual_byte:?}; expected context=\"{}\"; actual context=\"{}\"",
                context(expected, offset),
                context(actual, offset)
            )
            .expect("write to String");
        }
    }
    panic!("{report}");
}

#[test]
fn pinned_compile_results_are_byte_exact_without_normalization() {
    for capabilities in [false, true] {
        let path = evidence_path(capabilities);
        let expected = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("{} must be committed: {error}", path.display()));
        let actual = evidence(capabilities);
        assert_exact_bytes(path.to_string_lossy().as_ref(), &expected, &actual);
    }
}

#[test]
#[ignore = "regenerates committed raw compile-result evidence"]
fn regenerate_pinned_fixtures() {
    for capabilities in [false, true] {
        let path = evidence_path(capabilities);
        std::fs::create_dir_all(path.parent().expect("evidence parent"))
            .expect("create evidence dir");
        std::fs::write(&path, evidence(capabilities)).expect("write pinned evidence");
    }
}
