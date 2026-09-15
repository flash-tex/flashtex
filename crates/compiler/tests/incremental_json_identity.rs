//! Issue #65: warm incremental compiles must serialise to exactly the bytes of a
//! fresh compile.
//!
//! Each document is replayed through realistic edits (typing a character,
//! deleting a line, restoring it) on one warm `compile_result` session. After
//! every step the reply must equal, byte for byte, the reply of a brand-new
//! session given the same text, and the incremental `CompileOutput` must equal
//! the authoritative clean `compile_full_project` build.

use flashtex_compiler::incremental::{compile_full_project, Session};
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::SourceDocument;
use flashtex_compiler::protocol::handle_line;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

static FRESH: AtomicUsize = AtomicUsize::new(0);

fn request(project: &str, entry: &str, documents: &[(String, String)]) -> String {
    let docs = documents
        .iter()
        .map(|(path, text)| {
            let mut doc = Value::obj();
            doc.set("path", json::str_(path.as_str()));
            doc.set("text", json::str_(text.as_str()));
            doc
        })
        .collect();
    let mut payload = Value::obj();
    payload.set("project_id", json::str_(project));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(entry));
    payload.set("documents", Value::Arr(docs));
    payload.set(
        "layout_capabilities",
        Value::Arr(vec![json::str_("font-hints-v1"), json::str_("rules-v1")]),
    );
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("identity"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env)
}

/// Texts of the entry document: base, then each edit applied to the base.
fn edits(base: &str) -> Vec<String> {
    let mut middle = base.len() / 2;
    while !base.is_char_boundary(middle) {
        middle -= 1;
    }
    let line_start = base[middle..]
        .find('\n')
        .map_or(base.len(), |offset| middle + offset + 1);
    let line_end = base[line_start..]
        .find('\n')
        .map_or(base.len(), |offset| line_start + offset + 1);
    let mut typed = base.to_string();
    typed.insert(line_start, 'x');
    let mut typed_twice = typed.clone();
    typed_twice.insert(line_start + 1, ' ');
    let mut deleted = base.to_string();
    deleted.replace_range(line_start..line_end, "");
    vec![
        base.to_string(),
        typed,
        typed_twice,
        base.to_string(),
        deleted,
        base.to_string(),
    ]
}

fn assert_replay_identical(label: &str, entry: &str, documents: Vec<(String, String)>) {
    let entry_index = documents
        .iter()
        .position(|(path, _)| path == entry)
        .unwrap_or(0);
    let warm_project = format!("identity-warm-{label}");
    let constraints = LayoutConstraints::default();
    let mut session = Session::new();
    for (step, text) in edits(&documents[entry_index].1).into_iter().enumerate() {
        let mut current = documents.clone();
        current[entry_index].1 = text;

        let warm = handle_line(&request(&warm_project, entry, &current));
        let fresh_project = format!("identity-fresh-{}", FRESH.fetch_add(1, Ordering::Relaxed));
        let fresh = handle_line(&request(&fresh_project, entry, &current)).replace(
            &format!("\"project_id\":\"{fresh_project}\""),
            &format!("\"project_id\":\"{warm_project}\""),
        );
        assert!(
            warm == fresh,
            "{label} step {step}: warm compile_result JSON differs from a fresh compile"
        );

        let sources: Vec<SourceDocument<'_>> = current
            .iter()
            .map(|(path, text)| SourceDocument { path, text })
            .collect();
        let incremental = session.compile_project(&sources, entry, constraints);
        let clean = compile_full_project(&sources, entry, constraints);
        assert!(
            format!("{:#?}", incremental.output) == format!("{clean:#?}"),
            "{label} step {step}: incremental output differs from a clean build"
        );
    }
}

fn repo_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn hw1_edits_serialise_identically_to_fresh_compiles() {
    let text = std::fs::read_to_string(repo_root().join("fixtures/real-world/hw1/HW1.tex"))
        .expect("HW1 fixture");
    assert_replay_identical("hw1", "main.tex", vec![("main.tex".into(), text)]);
}

#[test]
fn corpus_edits_serialise_identically_to_fresh_compiles() {
    let corpus = repo_root().join("tests/tex-corpus");
    let manifest = json::parse(&std::fs::read_to_string(corpus.join("manifest.json")).unwrap())
        .expect("corpus manifest");
    let cases: Vec<Value> = match manifest.get("cases") {
        Some(Value::Arr(cases)) => cases.clone(),
        Some(Value::Obj(cases)) => cases.values().cloned().collect(),
        _ => panic!("corpus manifest has no cases"),
    };
    assert!(!cases.is_empty());
    for case in &cases {
        let id = case.get("id").and_then(Value::as_str).expect("case id");
        let entry = case.get("entry_path").and_then(Value::as_str).unwrap();
        let documents = case
            .get("documents")
            .and_then(Value::as_arr)
            .expect("case documents")
            .iter()
            .map(|document| {
                let path = document.as_str().expect("document path");
                let text = std::fs::read_to_string(corpus.join("cases").join(id).join(path))
                    .unwrap_or_else(|error| panic!("{id}/{path}: {error}"));
                (path.to_string(), text)
            })
            .collect();
        assert_replay_identical(id, entry, documents);
    }
}

/// The `scaling_bench` generator: sections, inline math, display math, macros.
fn scaling_document(target_bytes: usize) -> String {
    let mut out = String::from(
        "\\documentclass{article}\n\\newcommand{\\proj}{FlashTeX}\n\\begin{document}\n",
    );
    let mut n = 0usize;
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    while out.len() < target_bytes {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        match (seed >> 33) % 4 {
            0 => out.push_str(&format!("\\section{{Section {n}}}\n")),
            1 => out.push_str(&format!(
                "Paragraph {n} of \\proj{{}} with inline $x^{{{n}}} + \\alpha$ maths.\n\n"
            )),
            2 => out.push_str(&format!(
                "Paragraph {n} discusses results in some detail, with enough words to wrap \
                 across a line and exercise the paragraph breaker properly.\n\n"
            )),
            _ => out.push_str(&format!("Displayed: $$\\frac{{a_{{{n}}}}}{{b}}$$\n\n")),
        }
        n += 1;
    }
    out.push_str("\\end{document}\n");
    out
}

#[test]
fn scaling_document_edits_serialise_identically_to_fresh_compiles() {
    assert_replay_identical(
        "scaling",
        "main.tex",
        vec![("main.tex".into(), scaling_document(500_000))],
    );
}

/// `export::unrepresentable` skips printable ASCII without consulting the
/// tables; that is only sound while every such character maps as encodable.
#[test]
fn printable_ascii_is_always_exportable() {
    use flashtex_compiler::export::{map_char, unrepresentable, Glyph};
    for c in ' '..='~' {
        assert!(
            matches!(map_char(c), Glyph::Encodable { .. }),
            "{c:?} is no longer exportable; remove the ASCII fast path"
        );
    }
    let all: String = (' '..='~').collect();
    assert!(unrepresentable(&all).is_empty());
}

/// The reply writers' integer and no-escape fast paths keep the exact bytes.
#[test]
fn json_fast_paths_match_the_formatting_machinery() {
    for n in [
        0.0,
        -0.0,
        1.0,
        -1.0,
        9.0,
        10.0,
        612.0,
        -792.0,
        123_456_789.0,
        999_999_999_999_999.0,
        -999_999_999_999_999.0,
        1e15,
        12.34,
        -0.5,
    ] {
        let mut fast = String::new();
        json::write_number_into(n, &mut fast);
        let expected = if n == n.trunc() && n.abs() < 1e15 {
            format!("{}", n as i64)
        } else {
            format!("{n}")
        };
        assert_eq!(fast, expected, "{n}");
    }
    // Exhaustive over the hundredths fast path's whole domain and just past it.
    for k in -1_000_100..=1_000_100_i64 {
        let n = k as f64 / 100.0;
        let mut fast = String::new();
        json::write_number_into(n, &mut fast);
        let expected = if k % 100 == 0 {
            format!("{}", n as i64)
        } else {
            format!("{n}")
        };
        assert_eq!(fast, expected, "{k} hundredths");
    }
    for text in [
        "plain",
        "",
        "caf\u{e9} \u{2211}",
        "q\"uote",
        "back\\slash",
        "tab\tnl\n",
        "\u{1}",
    ] {
        let mut fast = String::new();
        json::write_string_into(text, &mut fast);
        let mut slow = String::from('"');
        for c in text.chars() {
            match c {
                '"' => slow.push_str("\\\""),
                '\\' => slow.push_str("\\\\"),
                '\n' => slow.push_str("\\n"),
                '\r' => slow.push_str("\\r"),
                '\t' => slow.push_str("\\t"),
                c if (c as u32) < 0x20 => slow.push_str(&format!("\\u{:04x}", c as u32)),
                c => slow.push(c),
            }
        }
        slow.push('"');
        assert_eq!(fast, slow);
    }
}

/// Bounded diagnostics (`diagnostics::limit_repeats`, and the engine's and
/// parser's early dropping of repeats) are a function of the whole revision,
/// so a warm session must report exactly what a fresh one does: repeats at
/// one invocation, a per-code budget with its summary, and a runaway loop.
#[test]
fn bounded_diagnostics_serialise_identically_to_fresh_compiles() {
    let mut body = String::from(
        "\\documentclass{article}\n\\newcommand{\\bad}{\\ifnum\\relax<1 \\fi\\efcode\\efcode\\ifnum\\relax<1 \\fi}\n\\begin{document}\n",
    );
    for i in 0..1100 {
        body.push_str(&format!("Line {i} \\bad{{}} and \\efcode{i}.\n"));
        if i % 50 == 0 {
            body.push('\n');
        }
    }
    body.push_str("\\end{document}\n");
    assert_replay_identical("bounded", "main.tex", vec![("main.tex".into(), body.clone())]);
    let runaway = body.replace("Line 550 ", "Line 550 \\def\\a{\\n\\a}\\a ");
    assert_replay_identical("bounded-runaway", "main.tex", vec![("main.tex".into(), runaway)]);
}
