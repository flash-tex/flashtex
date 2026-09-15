//! FT-002 acceptance tests, stated against docs/contracts/runtime-v1.md.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::{
    handle_line, read_request_line, RequestLine, MAX_LINE_BYTES, MAX_RESULT_BYTES,
};
use std::io::{BufReader, Cursor};

fn compile_line(id: &str, revision: i64, path: &str, text: &str) -> String {
    let mut doc = Value::obj();
    doc.set("path", json::str_(path));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("demo"));
    payload.set("revision", Value::Num(revision as f64));
    payload.set("entry_path", json::str_(path));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_(id));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env)
}

fn reply(line: &str) -> Value {
    json::parse(&handle_line(line)).expect("reply must be valid JSON")
}

fn items(v: &Value) -> Vec<Value> {
    v.get("payload")
        .and_then(|p| p.get("pages"))
        .and_then(|p| p.as_arr())
        .map(|pages| {
            pages
                .iter()
                .flat_map(|pg| {
                    pg.get("items")
                        .and_then(|i| i.as_arr())
                        .cloned()
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

fn status(v: &Value) -> String {
    v.get("payload")
        .and_then(|p| p.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string()
}

#[test]
fn realistic_preamble_is_informational_and_only_body_is_positioned() {
    let text = r"\documentclass[draft]{article}
\usepackage[demo]{amsmath}
\begin{document}Body only\end{document}trailer";
    let response = reply(&compile_line("preamble", 1, "main.tex", text));

    assert_eq!(status(&response), "recovered");
    let diagnostics = response
        .get("payload")
        .unwrap()
        .get("diagnostics")
        .unwrap()
        .as_arr()
        .unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].get("severity").unwrap().as_str(),
        Some("warning")
    );
    assert!(diagnostics[0]
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("amsmath"));

    assert_eq!(
        items(&response)
            .iter()
            .map(|item| item.get("text").unwrap().as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Body", "only"]
    );
}

#[test]
fn unicode_spans_are_utf8_bytes_that_slice_back_exactly() {
    let text = "héllo — naïve café world.\n";
    let r = reply(&compile_line("u1", 1, "main.tex", text));
    assert_eq!(r.get("type").unwrap().as_str(), Some("compile_result"));
    assert_eq!(r.get("id").unwrap().as_str(), Some("u1"));

    let its = items(&r);
    assert!(!its.is_empty(), "expected positioned text");

    for it in &its {
        let src = it.get("source").unwrap();
        let a = src.get("start_byte").unwrap().as_i64().unwrap() as usize;
        let b = src.get("end_byte").unwrap().as_i64().unwrap() as usize;
        let word = it.get("text").unwrap().as_str().unwrap();
        // Panics if the span is not on UTF-8 boundaries, which is the point.
        assert_eq!(&text[a..b], word, "span must slice back to the item's text");
    }

    // The multi-byte word must be found at its true byte offset, not a char index.
    let cafe = its
        .iter()
        .find(|i| i.get("text").unwrap().as_str() == Some("café"))
        .expect("café must be typeset");
    let start = cafe
        .get("source")
        .unwrap()
        .get("start_byte")
        .unwrap()
        .as_i64()
        .unwrap() as usize;
    assert_eq!(start, text.find("café").unwrap());
    assert_ne!(
        start,
        text.chars().position(|c| c == 'c').unwrap(),
        "byte offset must differ from char index here"
    );
}

#[test]
fn malformed_input_recovers_with_diagnostics_and_still_produces_output() {
    let text = "Good text {unclosed and \\nosuchcommand here.\n";
    let r = reply(&compile_line("m1", 1, "main.tex", text));
    assert_eq!(status(&r), "recovered");

    let diags = r
        .get("payload")
        .unwrap()
        .get("diagnostics")
        .unwrap()
        .as_arr()
        .unwrap()
        .clone();
    assert!(!diags.is_empty(), "malformed input must be reported");

    let messages: Vec<String> = diags
        .iter()
        .map(|d| d.get("message").unwrap().as_str().unwrap().to_string())
        .collect();
    assert!(
        messages.iter().any(|m| m.contains("unmatched '{'")),
        "got {:?}",
        messages
    );
    assert!(
        messages.iter().any(|m| m.contains("nosuchcommand")),
        "got {:?}",
        messages
    );

    for d in &diags {
        assert!(d.get("recovery").is_some(), "recovery field is required");
        assert!(d.get("severity").is_some());
    }
    assert!(
        !items(&r).is_empty(),
        "recovery must still typeset the readable text"
    );
}

#[test]
fn stray_closing_brace_is_reported_not_swallowed() {
    let r = reply(&compile_line("m2", 1, "main.tex", "text } more\n"));
    let diags = r
        .get("payload")
        .unwrap()
        .get("diagnostics")
        .unwrap()
        .as_arr()
        .unwrap()
        .clone();
    assert!(diags.iter().any(|d| d
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("unmatched '}'")));
    assert_eq!(status(&r), "recovered");
}

#[test]
fn repository_fixture_roundtrips() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../protocol/fixtures/compile-request.json"
    );
    let line = std::fs::read_to_string(fixture).expect("fixture must exist");
    let out = handle_line(line.trim());

    // The reply must itself be parseable: serialize -> parse -> serialize is stable.
    let parsed = json::parse(&out).expect("reply parses");
    let reparsed = json::parse(&json::write(&parsed)).expect("reply reparses");
    assert_eq!(parsed, reparsed);

    assert_eq!(
        parsed.get("id").unwrap().as_str(),
        Some("fixture-compile-1")
    );
    assert_eq!(parsed.get("type").unwrap().as_str(), Some("compile_result"));
    let payload = parsed.get("payload").unwrap();
    assert_eq!(payload.get("project_id").unwrap().as_str(), Some("demo"));
    assert_eq!(payload.get("revision").unwrap().as_i64(), Some(1));
    assert_eq!(payload.get("pdf_path").unwrap(), &Value::Null);
    assert_eq!(status(&parsed), "ok");

    let its = items(&parsed);
    assert!(its
        .iter()
        .any(|i| i.get("text").unwrap().as_str() == Some("Hello")));
    for it in &its {
        for key in [
            "kind",
            "text",
            "x_pt",
            "baseline_y_pt",
            "font_size_pt",
            "source",
        ] {
            assert!(it.get(key).is_some(), "item is missing {}", key);
        }
    }
}

#[test]
fn a_minimal_document_and_a_second_edited_revision_both_lay_out() {
    let v1 = "\\section{Intro}\nFirst version of the document.\n";
    let v2 = "\\section{Intro}\nFirst version of the edited document, now longer.\n";

    let r1 = reply(&compile_line("r1", 1, "main.tex", v1));
    let r2 = reply(&compile_line("r2", 2, "main.tex", v2));

    assert_eq!(
        r1.get("payload").unwrap().get("revision").unwrap().as_i64(),
        Some(1)
    );
    assert_eq!(
        r2.get("payload").unwrap().get("revision").unwrap().as_i64(),
        Some(2)
    );

    let i1 = items(&r1);
    let i2 = items(&r2);
    assert!(!i1.is_empty() && !i2.is_empty());
    assert!(
        i2.len() > i1.len(),
        "the longer revision should place more words"
    );

    // Spans must track the edited text, not the old text.
    //
    // Not every visible item is literal source text, and pretending otherwise
    // was too strong an invariant. Three categories exist, and all three must
    // point at bytes the author actually typed:
    //
    //   1. literal text  — the span slices back to exactly that text;
    //   2. substituted glyphs (\alpha renders as a Greek letter) — the span
    //      covers the command that produced it;
    //   3. generated values (section numbers, resolved \ref) — the span
    //      identifies the producing command.
    //
    // What is asserted for every item, without exception, is that the span
    // slices at all: it is in range and on character boundaries, so navigating
    // to it can never panic. Literal items are additionally required to match
    // their own text exactly, which is the guarantee this test was written for.
    for (r, text) in [(&r1, v1), (&r2, v2)] {
        for it in items(r) {
            let src = it.get("source").unwrap();
            let a = src.get("start_byte").unwrap().as_i64().unwrap() as usize;
            let b = src.get("end_byte").unwrap().as_i64().unwrap() as usize;
            let sliced = text
                .get(a..b)
                .unwrap_or_else(|| panic!("span {a}..{b} does not slice the source"));
            let rendered = it.get("text").unwrap().as_str().unwrap();
            if sliced != rendered {
                // Generated or substituted: the span must still name a real
                // command in the source, never invented bytes.
                assert!(
                    sliced.starts_with('\\'),
                    "item {rendered:?} is neither its own source text nor \
                     attributable to a command; span {a}..{b} is {sliced:?}"
                );
            }
        }
    }

    // The heading is typeset larger than body text.
    let heading = i2
        .iter()
        .find(|i| i.get("text").unwrap().as_str() == Some("Intro"))
        .unwrap();
    let body = i2
        .iter()
        .find(|i| i.get("text").unwrap().as_str() == Some("First"))
        .unwrap();
    assert!(
        heading.get("font_size_pt").unwrap().as_i64().unwrap()
            > body.get("font_size_pt").unwrap().as_i64().unwrap()
    );
}

#[test]
fn unknown_protocol_version_and_type_produce_error_envelopes() {
    let bad_version = r#"{"protocol_version":99,"id":"e1","type":"compile","payload":{}}"#;
    let r = reply(bad_version);
    assert_eq!(r.get("type").unwrap().as_str(), Some("error"));
    assert_eq!(r.get("id").unwrap().as_str(), Some("e1"));

    let bad_type = r#"{"protocol_version":1,"id":"e2","type":"no_such_type","payload":{}}"#;
    let r = reply(bad_type);
    assert_eq!(r.get("type").unwrap().as_str(), Some("error"));
    assert_eq!(r.get("id").unwrap().as_str(), Some("e2"));

    let malformed = reply("{not json");
    assert_eq!(malformed.get("type").unwrap().as_str(), Some("error"));
}

#[test]
fn unsafe_paths_are_rejected() {
    for path in ["/etc/passwd", "../outside.tex", "a/../../b.tex"] {
        let r = reply(&compile_line("p", 1, path, "x\n"));
        assert_eq!(status(&r), "failed", "path {} must be rejected", path);
        let diags = r
            .get("payload")
            .unwrap()
            .get("diagnostics")
            .unwrap()
            .as_arr()
            .unwrap()
            .clone();
        assert!(diags.iter().any(|d| d
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("rejected")));
    }
}

#[test]
fn math_and_unsupported_commands_are_reported_never_silent() {
    let r = reply(&compile_line(
        "x1",
        1,
        "main.tex",
        "text $x^2$ and \\tikz{a}\n",
    ));
    let diags = r
        .get("payload")
        .unwrap()
        .get("diagnostics")
        .unwrap()
        .as_arr()
        .unwrap()
        .clone();
    let messages: Vec<String> = diags
        .iter()
        .map(|d| d.get("message").unwrap().as_str().unwrap().to_string())
        .collect();
    // `$x^2$` is implemented and reports nothing. This used to assert a
    // "math mode is not implemented" message, which was satisfied only by
    // the same suffix wrongly appended to the unsupported *text* command.
    assert!(
        !messages.iter().any(|m| m.contains("math mode")),
        "got {:?}",
        messages
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("\\tikz is not supported")),
        "got {:?}",
        messages
    );
}

#[test]
fn bounded_reader_rejects_an_oversized_line_and_recovers_for_the_next_request() {
    let valid = compile_line("after-large", 7, "main.tex", "still works\n");
    let mut input = vec![b'x'; MAX_LINE_BYTES + 1];
    input.push(b'\n');
    input.extend_from_slice(valid.as_bytes());
    input.push(b'\n');

    // A deliberately small BufReader exercises the chunked path rather than
    // exposing the whole test input in one fill_buf call.
    let mut reader = BufReader::with_capacity(31, Cursor::new(input));
    assert_eq!(
        read_request_line(&mut reader).unwrap(),
        Some(RequestLine::TooLarge)
    );

    let next = read_request_line(&mut reader)
        .unwrap()
        .expect("next request remains readable");
    let bytes = match next {
        RequestLine::Data(bytes) => bytes,
        RequestLine::TooLarge => panic!("valid request was incorrectly rejected"),
    };
    let response = reply(std::str::from_utf8(&bytes).unwrap());
    assert_eq!(response.get("id").unwrap().as_str(), Some("after-large"));
    assert_eq!(
        response.get("type").unwrap().as_str(),
        Some("compile_result")
    );
    assert_eq!(
        response
            .get("payload")
            .unwrap()
            .get("revision")
            .unwrap()
            .as_i64(),
        Some(7)
    );
}

#[test]
fn warm_session_cache_is_bounded_and_stays_correct_after_eviction() {
    // Compile more distinct documents than the worker keeps warm, then return to
    // the first one. Eviction may cost it its reuse, but never its correctness:
    // the reply must still be the right answer for that document's text.
    let first = "Alpha paragraph one.\n\nAlpha paragraph two.\n";
    let r = reply(&compile_line("s0", 1, "a.tex", first));
    let before = items(&r);
    assert!(!before.is_empty());

    for n in 1..=12 {
        let text = format!("Filler document {n} with enough words to lay out.\n");
        let line = {
            let mut doc = Value::obj();
            doc.set("path", json::str_("a.tex"));
            doc.set("text", json::str_(text.clone()));
            let mut payload = Value::obj();
            payload.set("project_id", json::str_(format!("proj-{n}")));
            payload.set("revision", Value::Num(1.0));
            payload.set("entry_path", json::str_("a.tex"));
            payload.set("documents", Value::Arr(vec![doc]));
            let mut env = Value::obj();
            env.set("protocol_version", Value::Num(1.0));
            env.set("id", json::str_(format!("f{n}")));
            env.set("type", json::str_("compile"));
            env.set("payload", payload);
            json::write(&env)
        };
        let fr = reply(&line);
        assert_eq!(status(&fr), "ok", "filler document {n} must still compile");
    }

    // The original document was almost certainly evicted by now.
    let again = reply(&compile_line("s0", 2, "a.tex", first));
    let after = items(&again);
    assert_eq!(
        after.len(),
        before.len(),
        "eviction must not change how many items a document produces"
    );
    for (a, b) in after.iter().zip(before.iter()) {
        assert_eq!(a.get("text").unwrap(), b.get("text").unwrap());
        assert_eq!(
            a.get("source").unwrap().get("start_byte").unwrap(),
            b.get("source").unwrap().get("start_byte").unwrap(),
            "spans must survive eviction unchanged"
        );
    }
}

/// Issue #21: a valid large project must not produce a reply the consumer will
/// reject. It may deliver fewer pages, but only while saying so explicitly.
#[test]
fn a_large_project_reply_fits_the_transport_frame_and_says_what_it_dropped() {
    let mut text = String::from("\\documentclass{article}\n\\begin{document}\n");
    while text.len() < 500_000 {
        text.push_str("Paragraph with a reasonable number of words that wrap across a line.\n\n");
    }
    text.push_str("\\end{document}\n");

    let line = compile_line("oversized", 1, "main.tex", &text);
    let raw = handle_line(&line);
    assert!(
        raw.len() <= MAX_RESULT_BYTES,
        "reply of {} bytes exceeds the {MAX_RESULT_BYTES}-byte transport frame; \
         the consumer would reject it as malformed",
        raw.len()
    );

    let response = json::parse(&raw).expect("reply must still be valid JSON");
    let pages = response
        .get("payload")
        .and_then(|p| p.get("pages"))
        .and_then(|p| p.as_arr())
        .cloned()
        .unwrap_or_default();
    assert!(
        !pages.is_empty(),
        "a valid document must still deliver pages"
    );

    // Dropping pages silently is what the issue explicitly rules out.
    let diagnostics = response
        .get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap_or_default();
    let announced = diagnostics.iter().any(|d| {
        d.get("message")
            .and_then(|m| m.as_str())
            .is_some_and(|m| m.contains("transport frame") && m.contains("were not delivered"))
    });
    assert!(
        announced,
        "pages were truncated without an explicit diagnostic saying so"
    );
}

#[test]
fn punctuation_after_a_style_group_survives_macro_expansion() {
    // GH-MACRO-BODY-COMMA: `\newcommand{\w}{\textit{leaf}, then}\w` must
    // typeset exactly like the directly-typed `\textit{leaf}, then` — the
    // comma is ordinary text, not the `\,` kern control symbol. The scope is
    // general to every kern control symbol (`,`, `;`, `:`, `!`), not
    // comma-specific; `.` and `?` were never affected.
    fn texts(text: &str) -> Vec<String> {
        items(&reply(&compile_line("punct", 1, "main.tex", text)))
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            })
            .collect()
    }
    assert_eq!(texts("\\textit{leaf}, then"), ["leaf", ",", "then"]);
    assert_eq!(
        texts("\\newcommand{\\w}{\\textit{leaf}, then}\\w"),
        ["leaf", ",", "then"]
    );
    for punct in [";", ":", "!"] {
        let body = format!("\\textit{{leaf}}{punct} then");
        assert_eq!(
            texts(&format!("\\newcommand{{\\w}}{{{body}}}\\w")),
            ["leaf", punct, "then"],
            "punctuation {punct:?} after a style group must survive macro expansion"
        );
    }
    // Untouched witnesses: `.` and `?` are not kern control symbols.
    assert_eq!(
        texts("\\newcommand{\\w}{\\textit{leaf}. then}\\w"),
        ["leaf", ".", "then"]
    );
    assert_eq!(
        texts("\\newcommand{\\w}{\\textit{leaf}? then}\\w"),
        ["leaf", "?", "then"]
    );
    // Flip side: a genuine `\,` inside a longer-named macro is still a kern —
    // it typesets no "," text item, exactly like the direct `\,`.
    assert!(!texts("\\,").contains(&",".to_string()));
    assert!(!texts("\\newcommand{\\ww}{\\,}\\ww").contains(&",".to_string()));
}
