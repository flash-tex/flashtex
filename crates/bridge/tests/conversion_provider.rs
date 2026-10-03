//! `--conversion-provider` through the real compiled `flashtex-bridge` binary,
//! for both network providers, against a loopback HTTP stub that replays
//! checked-in provider reply fixtures (`tests/fixtures/providers/`).
//!
//! Nothing here reaches a real provider: every base URL is `http://127.0.0.1`
//! on a port this test bound, the key is a literal fixture string, and every
//! inherited credential/model/proxy variable is removed from the child's
//! environment before it starts.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Cursor, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

const FIXTURE_KEY: &str = "fixture-key-not-a-secret";
const XAI_REPLY: &str = include_str!("fixtures/providers/xai-responses.json");
const OPENAI_REPLY: &str = include_str!("fixtures/providers/openai-chat-completions.json");

/// Variables that must never leak from the developer's shell into the child.
const SCRUBBED: &[&str] = &[
    "XAI_API_KEY",
    "FLASHTEX_AI_API_KEY",
    "FLASHTEX_GROK_API_KEY",
    "OPENAI_API_KEY",
    "FLASHTEX_CONVERSION_MODEL",
    "FLASHTEX_GROK_MODEL",
    "FLASHTEX_CONVERSION_BASE_URL",
    "FLASHTEX_CONVERSION_PROVIDER",
    "FLASHTEX_CONVERSION_MAX_ATTEMPTS",
    "FLASHTEX_GROK_MAX_ATTEMPTS",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "ALL_PROXY",
    "http_proxy",
    "https_proxy",
    "all_proxy",
];

#[derive(Debug, Clone)]
struct Seen {
    request_line: String,
    authorization: Option<String>,
    body: Value,
}

/// A one-thread HTTP/1.1 stub on 127.0.0.1 that answers every request with
/// `status` and `reply`, recording what it received.
struct Stub {
    base: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}
impl Stub {
    fn start(status: u16, reply: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!(
            "http://127.0.0.1:{}/v1",
            listener.local_addr().unwrap().port()
        );
        let seen = Arc::new(Mutex::new(Vec::new()));
        let record = seen.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let mut reader = BufReader::new(stream);
                let mut request_line = String::new();
                if reader.read_line(&mut request_line).is_err() {
                    continue;
                }
                let mut length = 0usize;
                let mut authorization = None;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    let (name, value) = line.split_once(':').unwrap_or((&line, ""));
                    let value = value.trim().to_string();
                    match name.to_ascii_lowercase().as_str() {
                        "content-length" => length = value.parse().unwrap_or(0),
                        "authorization" => authorization = Some(value),
                        _ => {}
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                record.lock().unwrap().push(Seen {
                    request_line: request_line.trim().to_string(),
                    authorization,
                    body: serde_json::from_slice(&body).unwrap_or(Value::Null),
                });
                let mut stream = reader.into_inner();
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        Self { base, seen }
    }
    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().unwrap().clone()
    }
}

fn request(id: &str, kind: &str, payload: Value) -> String {
    format!(
        "{}\n",
        json!({"protocol_version":1,"id":id,"type":kind,"payload":payload})
    )
}

fn capture_flow() -> String {
    let mut image = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(2, 2)
        .write_to(&mut image, image::ImageFormat::Png)
        .unwrap();
    [
        request(
            "open",
            "document_open",
            json!({"project_id":"proj","path":"main.tex","revision":1,"text":"Let $y$ = .\n"}),
        ),
        request(
            "pin",
            "destination_pin",
            json!({"destination_id":"anchor","project_id":"proj","path":"main.tex","revision":1,"start_byte":10,"end_byte":10}),
        ),
        request(
            "submit",
            "capture_submit",
            json!({"capture_id":"cap-provider","destination_id":"anchor","base_revision":1,
                   "image":{"mime_type":"image/png","data_base64":STANDARD.encode(image.into_inner())},
                   "instructions":"transcribe"}),
        ),
        request("convert", "capture_convert", json!({"capture_id":"cap-provider"})),
        request("status", "capture_status", json!({"capture_id":"cap-provider"})),
    ]
    .concat()
}

struct Run {
    success: bool,
    replies: Vec<Value>,
    stderr: String,
}

fn run_bridge(store: &Path, args: &[&str], env: &[(&str, &str)], input: &str) -> Run {
    let mut command = Command::new(env!("CARGO_BIN_EXE_flashtex-bridge"));
    command.arg("--store").arg(store).args(args);
    for name in SCRUBBED {
        command.env_remove(name);
    }
    command
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("FLASHTEX_CONVERSION_MAX_ATTEMPTS", "1");
    for (name, value) in env {
        command.env(name, value);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    Run {
        success: output.status.success(),
        replies: String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            // `--help` prints plain text; every protocol reply is JSON.
            .filter_map(|s| serde_json::from_str(s).ok())
            .collect(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn convert_reply(run: &Run) -> &Value {
    assert!(run.success, "{}", run.stderr);
    assert_eq!(run.replies.len(), 5, "{:?}", run.replies);
    assert_eq!(
        run.replies[2]["type"], "capture_received",
        "{:?}",
        run.replies[2]
    );
    &run.replies[3]
}

#[test]
fn xai_provider_converts_via_responses_fixture_and_reports_evidence() {
    let stub = Stub::start(200, XAI_REPLY);
    let store = tempfile::tempdir().unwrap();
    let run = run_bridge(
        store.path(),
        &["--conversion-provider", "xai"],
        &[
            ("FLASHTEX_AI_API_KEY", FIXTURE_KEY),
            ("FLASHTEX_CONVERSION_MODEL", "grok-fixture-requested"),
            ("FLASHTEX_CONVERSION_BASE_URL", &stub.base),
        ],
        &capture_flow(),
    );
    let reply = convert_reply(&run);
    assert_eq!(reply["type"], "capture_proposal", "{reply}");
    assert_eq!(reply["payload"]["latex"], "$\\frac{a}{b}$");
    assert_eq!(reply["payload"]["insertion_blocked"], false);
    let evidence = &reply["payload"]["provider_evidence"];
    assert_eq!(
        evidence,
        &json!({"provider":"xai","model":"grok-fixture-vision","response_id":"resp_fixture_xai_0001",
                "usage":{"input_tokens":1834,"output_tokens":41,"total_tokens":1875}})
    );
    // Persisted: capture_status (a pure journal read) reports the same evidence.
    assert_eq!(&run.replies[4]["payload"]["provider_evidence"], evidence);

    let seen = stub.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].request_line, "POST /v1/responses HTTP/1.1");
    assert_eq!(
        seen[0].authorization.as_deref(),
        Some(format!("Bearer {FIXTURE_KEY}").as_str())
    );
    assert_eq!(seen[0].body["model"], "grok-fixture-requested");
    assert_eq!(seen[0].body["store"], false);
    assert_eq!(seen[0].body["text"]["format"]["strict"], true);
    assert!(!run.stderr.contains(FIXTURE_KEY));
    let journal = std::fs::read_dir(store.path())
        .unwrap()
        .filter_map(|e| std::fs::read_to_string(e.unwrap().path()).ok())
        .collect::<String>();
    assert!(journal.contains("resp_fixture_xai_0001"));
    assert!(
        !journal.contains(FIXTURE_KEY),
        "the key must never be journaled"
    );
}

#[test]
fn enable_grok_alias_still_works_with_legacy_environment_names() {
    let stub = Stub::start(200, XAI_REPLY);
    let store = tempfile::tempdir().unwrap();
    let run = run_bridge(
        store.path(),
        &["--enable-grok"],
        &[
            ("XAI_API_KEY", FIXTURE_KEY),
            ("FLASHTEX_GROK_MODEL", "grok-legacy-model"),
            ("FLASHTEX_CONVERSION_BASE_URL", &stub.base),
        ],
        &capture_flow(),
    );
    let reply = convert_reply(&run);
    assert_eq!(reply["type"], "capture_proposal", "{reply}");
    assert_eq!(reply["payload"]["provider_evidence"]["provider"], "xai");
    let seen = stub.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].body["model"], "grok-legacy-model");
    assert_eq!(
        seen[0].authorization.as_deref(),
        Some(format!("Bearer {FIXTURE_KEY}").as_str())
    );
}

#[test]
fn openai_compatible_provider_converts_via_chat_completions_fixture() {
    let stub = Stub::start(200, OPENAI_REPLY);
    let store = tempfile::tempdir().unwrap();
    let run = run_bridge(
        store.path(),
        &["--conversion-provider", "openai-compatible"],
        &[
            ("FLASHTEX_AI_API_KEY", FIXTURE_KEY),
            ("FLASHTEX_CONVERSION_MODEL", "local-vlm-requested"),
            ("FLASHTEX_CONVERSION_BASE_URL", &stub.base),
            // Legacy xAI names must not leak into another provider.
            ("FLASHTEX_GROK_MODEL", "grok-should-be-ignored"),
        ],
        &capture_flow(),
    );
    let reply = convert_reply(&run);
    assert_eq!(reply["type"], "capture_proposal", "{reply}");
    assert_eq!(reply["payload"]["latex"], "$x^{2}+1$");
    let evidence = &reply["payload"]["provider_evidence"];
    assert_eq!(
        evidence,
        &json!({"provider":"openai-compatible","model":"local-vlm-fixture","response_id":"chatcmpl-fixture-0001",
                "usage":{"input_tokens":912,"output_tokens":27,"total_tokens":939}})
    );
    assert_eq!(&run.replies[4]["payload"]["provider_evidence"], evidence);
    let seen = stub.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].request_line, "POST /v1/chat/completions HTTP/1.1");
    assert_eq!(
        seen[0].authorization.as_deref(),
        Some(format!("Bearer {FIXTURE_KEY}").as_str())
    );
    assert_eq!(seen[0].body["model"], "local-vlm-requested");
    assert_eq!(
        seen[0].body["response_format"]["json_schema"]["strict"],
        true
    );
    assert!(
        seen[0].body["messages"][1]["content"][1]["image_url"]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
}

#[test]
fn openai_compatible_loopback_server_may_run_without_a_key() {
    let stub = Stub::start(200, OPENAI_REPLY);
    let store = tempfile::tempdir().unwrap();
    let run = run_bridge(
        store.path(),
        &["--conversion-provider", "openai-compatible"],
        &[
            ("FLASHTEX_CONVERSION_MODEL", "local-vlm"),
            ("FLASHTEX_CONVERSION_BASE_URL", &stub.base),
        ],
        &capture_flow(),
    );
    assert_eq!(convert_reply(&run)["type"], "capture_proposal");
    let seen = stub.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].authorization, None);
}

#[test]
fn disabled_or_misconfigured_providers_refuse_without_any_request() {
    let stub = Stub::start(200, XAI_REPLY);
    // (arguments, environment, expected error code)
    type Case<'a> = (Vec<&'a str>, Vec<(&'a str, &'a str)>, &'a str);
    let cases: Vec<Case> = vec![
        (
            vec![],
            vec![("FLASHTEX_AI_API_KEY", FIXTURE_KEY)],
            "provider_disabled",
        ),
        (
            vec!["--conversion-provider", "none"],
            vec![("FLASHTEX_AI_API_KEY", FIXTURE_KEY)],
            "provider_disabled",
        ),
        (
            vec!["--conversion-provider", "xai"],
            vec![("FLASHTEX_CONVERSION_BASE_URL", stub.base.as_str())],
            "provider_auth_missing",
        ),
        (
            vec!["--conversion-provider", "openai-compatible"],
            vec![
                ("FLASHTEX_AI_API_KEY", FIXTURE_KEY),
                ("FLASHTEX_CONVERSION_MODEL", "m"),
            ],
            "provider_base_url_missing",
        ),
        (
            vec!["--conversion-provider", "openai-compatible"],
            vec![
                ("FLASHTEX_AI_API_KEY", FIXTURE_KEY),
                ("FLASHTEX_CONVERSION_BASE_URL", stub.base.as_str()),
            ],
            "invalid_model",
        ),
        (
            vec!["--conversion-provider", "openai-compatible"],
            vec![
                ("FLASHTEX_AI_API_KEY", FIXTURE_KEY),
                ("FLASHTEX_CONVERSION_MODEL", "m"),
                ("FLASHTEX_CONVERSION_BASE_URL", "http://llm.example/v1"),
            ],
            "invalid_base_url",
        ),
    ];
    for (args, env, code) in cases {
        let store = tempfile::tempdir().unwrap();
        let run = run_bridge(store.path(), &args, &env, &capture_flow());
        let reply = convert_reply(&run);
        assert_eq!(reply["type"], "error", "{args:?}: {reply}");
        assert_eq!(reply["payload"]["code"], code, "{args:?}: {reply}");
        assert_eq!(run.replies[4]["payload"]["provider_evidence"], Value::Null);
    }
    assert!(stub.seen().is_empty(), "no case may contact the endpoint");
}

#[test]
fn provider_http_errors_keep_their_codes_for_both_providers() {
    for (provider, reply) in [("xai", XAI_REPLY), ("openai-compatible", OPENAI_REPLY)] {
        let stub = Stub::start(401, reply);
        let store = tempfile::tempdir().unwrap();
        let run = run_bridge(
            store.path(),
            &["--conversion-provider", provider],
            &[
                ("FLASHTEX_AI_API_KEY", FIXTURE_KEY),
                ("FLASHTEX_CONVERSION_MODEL", "m"),
                ("FLASHTEX_CONVERSION_BASE_URL", &stub.base),
            ],
            &capture_flow(),
        );
        let reply = convert_reply(&run);
        assert_eq!(
            reply["payload"]["code"], "provider_auth_error",
            "{provider}: {reply}"
        );
        assert!(reply["payload"]["message"]
            .as_str()
            .unwrap()
            .contains("(attempt 1/1)"));
        assert_eq!(stub.seen().len(), 1, "401 is never retried");
    }
}

#[test]
fn invalid_or_conflicting_provider_arguments_fail_at_startup() {
    for args in [
        vec!["--conversion-provider", "gemini"],
        vec!["--conversion-provider"],
        vec![
            "--conversion-provider",
            "openai-compatible",
            "--enable-grok",
        ],
        vec!["--enable-grok", "--conversion-provider", "none"],
    ] {
        let store = tempfile::tempdir().unwrap();
        let run = run_bridge(store.path(), &args, &[], "");
        assert!(!run.success, "{args:?} should be rejected");
        assert!(
            run.stderr.contains("invalid_arguments"),
            "{args:?}: {}",
            run.stderr
        );
    }
    let store = tempfile::tempdir().unwrap();
    let consistent = run_bridge(
        store.path(),
        &["--enable-grok", "--conversion-provider", "xai"],
        &[],
        "",
    );
    assert!(consistent.success, "{}", consistent.stderr);
    let help = run_bridge(store.path(), &["--help"], &[], "");
    assert!(help.success);
}
