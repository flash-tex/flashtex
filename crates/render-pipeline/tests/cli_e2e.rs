//! `flashtex-render` end to end over JSON Lines: a compile request, a
//! rejected protocol version, an unknown type, capability negotiation,
//! `--v2` output and `--pdf` output.

use std::io::Write;
use std::process::{Command, Stdio};

use flashtex_compiler::json;

/// Whether Latin Modern resolves, for the ~110 `if !lm_available() { return }`
/// guards across this suite.
///
/// Those guards used to make a fontless run *silently green*: the tests did
/// not fail, they simply never executed, and `eprintln!` is captured by
/// libtest, so nothing was printed either. A whole-suite run with no
/// `FLASHTEX_*` set therefore reported success while measuring almost
/// nothing -- the same trap as an oracle harness scoring an OpenType
/// fallback as a pass.
///
/// So a missing Latin Modern is now a loud failure by default. A genuinely
/// fontless environment can still skip, but only by asking for it:
/// `FLASHTEX_ALLOW_FONTLESS_TESTS=1`, which restores the old `false`.
fn lm_available() -> bool {
    if flashtex_render_pipeline::FontSet::with_default_dirs(&[]).latin_modern_available() {
        return true;
    }
    if std::env::var_os("FLASHTEX_ALLOW_FONTLESS_TESTS").is_some() {
        return false;
    }
    panic!(
        "Latin Modern is not resolvable, so this test would have skipped silently \
         and the run would have been green without measuring anything. Point \
         FLASHTEX_FONT_DIRS at a directory of Latin Modern .otf files (the repo \
         bundles apps/mac/Fonts) and FLASHTEX_TFM_DIRS at its metrics, or set \
         FLASHTEX_ALLOW_FONTLESS_TESTS=1 to skip deliberately."
    );
}

fn run(args: &[&str], input: &str) -> (Vec<json::Value>, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-render"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn flashtex-render");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let replies = stdout.lines().map(|l| json::parse(l).expect("reply is JSON")).collect();
    (replies, String::from_utf8_lossy(&out.stderr).into_owned())
}

fn compile_line(id: &str, text: &str, caps: Option<&[&str]>) -> String {
    let mut doc = json::Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = json::Value::obj();
    payload.set("project_id", json::str_("cli"));
    payload.set("revision", json::num(3.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", json::Value::Arr(vec![doc]));
    if let Some(c) = caps {
        payload.set("layout_capabilities", json::Value::Arr(c.iter().map(|s| json::str_(*s)).collect()));
    }
    let mut v = json::Value::obj();
    v.set("protocol_version", json::num(1.0));
    v.set("id", json::str_(id));
    v.set("type", json::str_("compile"));
    v.set("payload", payload);
    json::write(&v) + "\n"
}

#[test]
fn worker_answers_each_line_and_fails_closed_on_unknown_versions() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-render-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let v2_path = dir.join("out.v2.json");
    let pdf_path = dir.join("out.pdf");
    let mut input = String::new();
    input.push_str(&compile_line("a", "\\begin{document}Hello $\\frac{1}{2}$ wörld.\\end{document}", Some(&["rules-v1", "nope"])));
    input.push_str("{\"protocol_version\":2,\"id\":\"b\",\"type\":\"compile\",\"payload\":{}}\n");
    input.push_str("{\"protocol_version\":1,\"id\":\"c\",\"type\":\"render\",\"payload\":{}}\n");
    input.push_str("not json\n");
    input.push_str(&compile_line("d", "\\begin{document}Second request.\\end{document}", None));
    let (replies, stderr) = run(&["--v2", v2_path.to_str().unwrap(), "--pdf", pdf_path.to_str().unwrap(), "--timing"], &input);
    assert_eq!(replies.len(), 5, "one reply per line: {stderr}");

    let a = &replies[0];
    assert_eq!(a.get("id").and_then(|v| v.as_str()), Some("a"));
    assert_eq!(a.get("type").and_then(|v| v.as_str()), Some("compile_result"));
    let pa = a.get("payload").unwrap();
    assert_eq!(pa.get("status").and_then(|v| v.as_str()), Some("ok"));
    let caps = pa.get("layout_capabilities").and_then(|v| v.as_arr()).unwrap();
    assert_eq!(caps.len(), 1);
    assert_eq!(caps[0].as_str(), Some("rules-v1"));
    let items = pa.get("pages").and_then(|v| v.as_arr()).unwrap()[0].get("items").and_then(|v| v.as_arr()).unwrap();
    assert!(items.iter().any(|i| i.get("kind").and_then(|v| v.as_str()) == Some("rule")));
    assert!(items.iter().all(|i| i.get("font").is_none()), "font hints were not requested");
    let w = items.iter().find(|i| i.get("text").and_then(|v| v.as_str()) == Some("wörld.")).expect("wörld. item");
    let src = w.get("source").unwrap();
    assert_eq!(src.get("path").and_then(|v| v.as_str()), Some("main.tex"));
    let (s, e) = (src.get("start_byte").unwrap().as_i64().unwrap() as usize, src.get("end_byte").unwrap().as_i64().unwrap() as usize);
    assert_eq!(&"\\begin{document}Hello $\\frac{1}{2}$ wörld.\\end{document}"[s..e], "wörld.");

    let b = &replies[1];
    assert_eq!(b.get("type").and_then(|v| v.as_str()), Some("error"));
    assert_eq!(b.get("payload").unwrap().get("code").and_then(|v| v.as_str()), Some("unsupported_protocol_version"));
    let c = &replies[2];
    assert_eq!(c.get("payload").unwrap().get("code").and_then(|v| v.as_str()), Some("unsupported_type"));
    let n = &replies[3];
    assert_eq!(n.get("payload").unwrap().get("code").and_then(|v| v.as_str()), Some("malformed_json"));
    let d = &replies[4];
    assert_eq!(d.get("id").and_then(|v| v.as_str()), Some("d"));
    assert!(d.get("payload").unwrap().get("layout_capabilities").is_none(), "omitted when not requested");

    // --v2 holds the LAST successful request; --pdf too.
    let v2 = json::parse(&std::fs::read_to_string(&v2_path).unwrap()).unwrap();
    assert_eq!(v2.get("id").and_then(|v| v.as_str()), Some("d"));
    assert_eq!(v2.get("protocol_version").and_then(|v| v.as_i64()), Some(2));
    let pdf = std::fs::read(&pdf_path).unwrap();
    assert!(pdf.starts_with(b"%PDF-1."));
    assert!(stderr.contains("rendered in"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn malformed_capability_lists_fail_the_request_not_the_worker() {
    if !lm_available() {
        return;
    }
    let mut input = String::new();
    input.push_str("{\"protocol_version\":1,\"id\":\"x\",\"type\":\"compile\",\"payload\":{\"project_id\":\"p\",\"revision\":1,\"entry_path\":\"main.tex\",\"documents\":[{\"path\":\"main.tex\",\"text\":\"a\"}],\"layout_capabilities\":[\"rules-v1\",\"rules-v1\"]}}\n");
    input.push_str(&compile_line("y", "\\begin{document}ok\\end{document}", Some(&[])));
    let (replies, _) = run(&[], &input);
    assert_eq!(replies[0].get("payload").unwrap().get("status").and_then(|v| v.as_str()), Some("failed"));
    let py = replies[1].get("payload").unwrap();
    assert_eq!(py.get("status").and_then(|v| v.as_str()), Some("ok"));
    assert_eq!(py.get("layout_capabilities").and_then(|v| v.as_arr()).map(Vec::len), Some(0));
}

fn run_env(args: &[&str], input: &str, env: &[(&str, &str)]) -> (Vec<json::Value>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flashtex-render"));
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("spawn flashtex-render");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    let replies = stdout.lines().map(|l| json::parse(l).expect("reply is JSON")).collect();
    (replies, String::from_utf8_lossy(&out.stderr).into_owned())
}

/// `docs/contracts/runtime-v1-display-list-v2.md` producer gate: (a) not
/// requested -> no v2 line; (b) requested + ok -> echoed and exactly one
/// `display_list` line right after, same id/project/revision and the request
/// text's digest; (c) failed -> no line; (d) oversize -> declined with the
/// `display-list-v2 declined:` warning and no line.
#[test]
fn display_list_v2_is_a_sibling_line_only_when_negotiated() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let text = "\\begin{document}Hello $\\frac{1}{2}$ wörld.\\end{document}";
    let mut input = String::new();
    input.push_str(&compile_line("plain", text, Some(&["rules-v1"])));
    input.push_str(&compile_line("v2", text, Some(&["rules-v1", "display-list-v2"])));
    input.push_str("{\"protocol_version\":1,\"id\":\"bad\",\"type\":\"compile\",\"payload\":{\"project_id\":\"cli\",\"revision\":3,\"entry_path\":\"../x.tex\",\"documents\":[{\"path\":\"../x.tex\",\"text\":\"a\"}],\"layout_capabilities\":[\"display-list-v2\"]}}\n");
    input.push_str(&compile_line("after", text, None));
    let (replies, _) = run(&[], &input);
    let ids: Vec<(Option<&str>, Option<&str>, Option<i64>)> = replies
        .iter()
        .map(|r| (r.get("id").and_then(|v| v.as_str()), r.get("type").and_then(|v| v.as_str()), r.get("protocol_version").and_then(|v| v.as_i64())))
        .collect();
    assert_eq!(
        ids,
        vec![
            (Some("plain"), Some("compile_result"), Some(1)),
            (Some("v2"), Some("compile_result"), Some(1)),
            (Some("v2"), Some("display_list"), Some(2)),
            (Some("bad"), Some("compile_result"), Some(1)),
            (Some("after"), Some("compile_result"), Some(1)),
        ],
        "one v2 line, only for the request that asked and succeeded"
    );
    let p = replies[1].get("payload").unwrap();
    let caps: Vec<&str> = p.get("layout_capabilities").and_then(|v| v.as_arr()).unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(caps, vec!["rules-v1", "display-list-v2"]);
    let dl = replies[2].get("payload").unwrap();
    assert_eq!(dl.get("render_format").and_then(|v| v.as_str()), Some("display-list-v2"));
    assert_eq!(dl.get("project_id").and_then(|v| v.as_str()), Some("cli"));
    assert_eq!(dl.get("revision").and_then(|v| v.as_i64()), Some(3));
    let doc = &dl.get("documents").and_then(|v| v.as_arr()).unwrap()[0];
    assert_eq!(doc.get("revision").and_then(|v| v.as_i64()), Some(3));
    assert_eq!(doc.get("byte_length").and_then(|v| v.as_i64()), Some(text.len() as i64));
    let digest = flashtex_font_engine::sha256::hex(&flashtex_font_engine::sha256::digest(text.as_bytes()));
    assert_eq!(doc.get("sha256").and_then(|v| v.as_str()), Some(digest.as_str()));
    assert!(dl.get("pages").and_then(|v| v.as_arr()).is_some_and(|p| !p.is_empty()));
    let bad = replies[3].get("payload").unwrap();
    assert_eq!(bad.get("status").and_then(|v| v.as_str()), Some("failed"));
    let bad_caps = bad.get("layout_capabilities").and_then(|v| v.as_arr()).unwrap();
    assert_eq!(bad_caps.len(), 1, "a failed request still echoes what it accepted, and sends no v2 line");

    // (d) oversize: a tiny reply limit declines the capability for that request.
    let (replies, _) = run_env(&[], &compile_line("big", text, Some(&["display-list-v2"])), &[("FLASHTEX_MAX_REPLY_BYTES", "6000")]);
    assert_eq!(replies.len(), 1, "no display_list line when declined");
    assert_eq!(replies[0].get("type").and_then(|v| v.as_str()), Some("compile_result"));
    let p = replies[0].get("payload").unwrap();
    assert_eq!(p.get("layout_capabilities").and_then(|v| v.as_arr()).map(Vec::len), Some(0), "display-list-v2 not echoed when declined");
    let diags = p.get("diagnostics").and_then(|v| v.as_arr()).unwrap();
    assert!(
        diags.iter().any(|d| d.get("message").and_then(|v| v.as_str()).is_some_and(|m| m.starts_with("display-list-v2 declined:"))),
        "{diags:?}"
    );
    assert_eq!(p.get("status").and_then(|v| v.as_str()), Some("recovered"));
}

/// `--tex FILE` convenience mode: no JSON on either side. The file's basename
/// is the project path, `--pdf`/`--v2` are written, diagnostics reach stderr
/// as `severity[code] message (line:col)`, and the exit code follows the
/// status (0 ok/recovered, 1 failed, 2 unreadable input).
#[test]
fn tex_file_mode_renders_without_json_and_reports_readably() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-render-tex-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let tex = dir.join("hello.tex");
    let pdf = dir.join("hello.pdf");
    let v2 = dir.join("hello.v2.json");
    std::fs::write(&tex, "\\begin{document}\nHello $\\frac{1}{2}$ wörld.\n\\end{document}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-render"))
        .args(["--tex", tex.to_str().unwrap(), "--pdf", pdf.to_str().unwrap(), "--v2", v2.to_str().unwrap()])
        .stdin(Stdio::null())
        .output()
        .expect("spawn flashtex-render --tex");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "exit 0 for an ok document: {stderr}");
    assert!(out.stdout.is_empty(), "--tex writes no JSON to stdout: {}", String::from_utf8_lossy(&out.stdout));
    assert!(stderr.contains("hello.tex: ok, 1 page"), "{stderr}");
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF-1."));
    let v2 = json::parse(&std::fs::read_to_string(&v2).unwrap()).unwrap();
    assert_eq!(v2.get("id").and_then(|v| v.as_str()), Some("hello.tex"), "the request id is the basename");
    let doc = &v2.get("payload").unwrap().get("documents").and_then(|v| v.as_arr()).unwrap()[0];
    assert_eq!(doc.get("path").and_then(|v| v.as_str()), Some("hello.tex"));

    // A document with a diagnostic on line 2 reports it at (line:col) and
    // keeps exit 0 when the pipeline recovered.
    let bad = dir.join("bad.tex");
    std::fs::write(&bad, "\\begin{document}\nHello } world.\n\\end{document}\n").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-render"))
        .args(["--tex", bad.to_str().unwrap()])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let diag = stderr.lines().find(|l| l.starts_with("error[") || l.starts_with("warning[")).unwrap_or_else(|| panic!("a readable diagnostic line: {stderr}"));
    assert!(diag.contains("] ") && diag.ends_with(')'), "severity[code] message (line:col): {diag}");
    assert!(diag.contains("(2:"), "the diagnostic is on line 2: {diag}");
    let status = stderr.lines().find(|l| l.starts_with("flashtex-render: bad.tex: ")).unwrap_or_else(|| panic!("summary line: {stderr}"));
    if status.contains(": failed") {
        assert_eq!(out.status.code(), Some(1), "{stderr}");
    } else {
        assert!(out.status.success(), "{stderr}");
    }

    // Unreadable input: exit 2, nothing written.
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-render"))
        .args(["--tex", dir.join("missing.tex").to_str().unwrap()])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot read"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The product bug `display-list-v2-window` exists for: a document too big for
/// the reply limit gets **no reply at all**, and a window gives it one.
///
/// The real case is the 500 KB corpus document -- 385 pages, a 20 339 674-byte
/// `compile_result` and a 152 MB `display_list` against the 16 MiB limit, so
/// both lines are refused and the request ends `failed`. Laying out 385 pages
/// in a debug build would cost minutes, and the failure is a *ratio* between
/// the reply and the limit rather than an absolute size, so this puts a much
/// smaller document into the same ratio through `FLASHTEX_MAX_REPLY_BYTES` --
/// the knob that exists for exactly this. Same code path, same refusals.
///
/// Nothing here is hard-coded to a page size: the limit is derived from a
/// measured window, and the whole-document size is read back out of the
/// producer's own refusal, so the test keeps testing the relation it is about
/// when page sizes drift.
#[test]
fn a_document_over_the_reply_limit_has_a_reply_when_a_window_is_negotiated() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    const WINDOW: i64 = 2;
    let mut text = String::from("\\begin{document}\n");
    for i in 0..280 {
        text.push_str(&format!(
            "\\section{{Part {i}}}\nSome prose for part {i}, with enough words in it that the \
             paragraph breaks over several lines of the measure and the page fills up rather \
             than holding one short line. More words follow, and then more again.\n\n"
        ));
    }
    text.push_str("\\end{document}\n");
    let plain = &["rules-v1", "display-list-v2"];
    let with_window = &["rules-v1", "display-list-v2", "display-list-v2-only", "display-list-v2-window"];

    // (0) Measure one window, with the real limit in place: this document is
    // already past the point where the whole thing fits, which is the
    // condition under test, so the window is the only way to see a page of it.
    let line = with_window_field(&compile_line("measure", &text, Some(with_window)), 1, WINDOW);
    let (replies, _) = run_env(&[], &line, &[]);
    assert_eq!(replies.len(), 2, "a window of this document fits the real limit");
    let measured = json::write(&replies[1]).len();
    let total_pages = replies[1].get("payload").unwrap().get("pages").and_then(|v| v.as_arr()).unwrap().len();

    // A limit a little over one window: enough to carry it, nowhere near
    // enough to carry the document.
    let limit_bytes = measured + measured / (2 * WINDOW as usize);
    let limit = [("FLASHTEX_MAX_REPLY_BYTES", limit_bytes.to_string())];
    let limit: Vec<(&str, &str)> = limit.iter().map(|(k, v)| (*k, v.as_str())).collect();

    // (a) Today's consumer: the v2 sibling is over the limit so it is declined
    // without being serialised, and the v1 `compile_result` that would have
    // carried the pages instead is over the limit too. The request ends
    // `failed`. Not slow -- failed.
    let (replies, _) = run_env(&[], &compile_line("today", &text, Some(plain)), &limit);
    assert_eq!(replies.len(), 1, "the declined sibling is not sent");
    let p = replies[0].get("payload").unwrap();
    assert_eq!(p.get("status").and_then(|v| v.as_str()), Some("failed"), "the bug: no reply for a long document");
    assert_eq!(p.get("pages").and_then(|v| v.as_arr()).map(Vec::len), Some(0), "a failed reply carries no pages");
    // The producer's refusal states the size it could not send; read it back,
    // so "the document does not fit" is a checked fact and not an assumption
    // about how big a page is.
    let message = p
        .get("diagnostics")
        .and_then(|v| v.as_arr())
        .and_then(|d| d.first())
        .and_then(|d| d.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let whole_bytes: usize = message
        .split_whitespace()
        .find_map(|w| w.parse::<usize>().ok())
        .unwrap_or_else(|| panic!("expected a sized refusal, got {message:?}"));
    assert!(
        whole_bytes > limit_bytes,
        "this document no longer reproduces the 500 KB case's ratio ({whole_bytes} B over \
         {total_pages} pages against a {limit_bytes} B limit) -- lengthen it"
    );

    // (b) The same document, the same limit, a consumer that also negotiates
    // `-only` and `-window`: a reply, carrying the window it asked for.
    let line = with_window_field(&compile_line("win", &text, Some(with_window)), 1, WINDOW);
    let (replies, _) = run_env(&[], &line, &limit);
    assert_eq!(replies.len(), 2, "a compile_result and its display_list sibling");
    let p = replies[0].get("payload").unwrap();
    assert_ne!(p.get("status").and_then(|v| v.as_str()), Some("failed"), "the window is the fix");
    let echoed: Vec<&str> = p.get("layout_capabilities").and_then(|v| v.as_arr()).unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(echoed.contains(&"display-list-v2-window"), "the window is echoed when served: {echoed:?}");
    assert!(echoed.contains(&"display-list-v2-only"), "{echoed:?}");
    assert_eq!(p.get("pages").and_then(|v| v.as_arr()).map(Vec::len), Some(0), "-only elided the v1 pages");

    let dl = replies[1].get("payload").unwrap();
    assert_eq!(replies[1].get("type").and_then(|v| v.as_str()), Some("display_list"));
    let pages = dl.get("pages").and_then(|v| v.as_arr()).unwrap();
    assert_eq!(pages.len(), total_pages, "every page of the document is still present");
    let resident: Vec<&json::Value> = pages.iter().filter(|p| p.get("items").is_some()).collect();
    assert_eq!(resident.len(), WINDOW as usize, "exactly the window's pages carry items");
    assert!(!resident[0].get("items").and_then(|v| v.as_arr()).unwrap().is_empty(), "a resident page is painted");
    let win = dl.get("window").expect("the sibling states its own coverage");
    assert_eq!(win.get("first_page").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(win.get("page_count").and_then(|v| v.as_i64()), Some(WINDOW));
    assert_eq!(win.get("document_page_count").and_then(|v| v.as_i64()), Some(total_pages as i64));
    for page in pages.iter().filter(|p| p.get("items").is_none()) {
        assert_eq!(json::write(page.get("resident").expect("resident flag")), "false");
        assert!(page.get("number").is_some() && page.get("width").is_some() && page.get("height").is_some());
    }

    // (c) A window the limit still cannot carry is narrowed to what fits and
    // served, rather than declined into the `failed` of (a).
    let wide = with_window_field(&compile_line("wide", &text, Some(with_window)), 1, 64);
    let (replies, _) = run_env(&[], &wide, &limit);
    assert_eq!(replies.len(), 2, "narrowed, not declined");
    let served = replies[1].get("payload").unwrap().get("window").expect("window");
    let count = served.get("page_count").and_then(|v| v.as_i64()).unwrap();
    assert!(count < 64, "the window should have been narrowed to what fits, not served at 64");
    assert!(json::write(&replies[1]).len() <= limit_bytes, "the narrowed line is within the limit");

    // (d) The capability listed with no window field is an unwindowed reply
    // (proposal §4) -- the consumer has to say where the viewer is -- so the
    // long document still fails, and the name is absent from the echo.
    let (replies, _) = run_env(&[], &compile_line("nofield", &text, Some(with_window)), &limit);
    assert_eq!(replies.len(), 1);
    let p = replies[0].get("payload").unwrap();
    assert_eq!(p.get("status").and_then(|v| v.as_str()), Some("failed"));
    let echoed: Vec<&str> = p.get("layout_capabilities").and_then(|v| v.as_arr()).unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(!echoed.contains(&"display-list-v2-window"), "not echoed when no window was served: {echoed:?}");
}

/// `display-list-v2-window` and `display-list-v2-delta` are mutually exclusive
/// in r1 (§7): a request listing both is answered with the window, and the echo
/// says so, because a delta binds a digest for every page and a windowed
/// producer has none for a page it never materialised.
#[test]
fn a_window_declines_a_delta_and_needs_the_display_list() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let text = "\\begin{document}\nA short document.\n\\end{document}\n";

    // Both requested: the window is accepted, `-delta` is not echoed.
    let both = &["display-list-v2", "display-list-v2-delta", "display-list-v2-window"];
    let line = with_window_field(&compile_line("both", text, Some(both)), 1, 1);
    let (replies, _) = run_env(&[], &line, &[]);
    let echoed: Vec<&str> = replies[0]
        .get("payload")
        .unwrap()
        .get("layout_capabilities")
        .and_then(|v| v.as_arr())
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(echoed.contains(&"display-list-v2-window"), "{echoed:?}");
    assert!(!echoed.contains(&"display-list-v2-delta"), "the window wins in r1: {echoed:?}");
    assert_eq!(replies[1].get("type").and_then(|v| v.as_str()), Some("display_list"), "a full line, never a delta");

    // Without `display-list-v2` the name means nothing and is not accepted.
    let alone = &["rules-v1", "display-list-v2-window"];
    let line = with_window_field(&compile_line("alone", text, Some(alone)), 1, 1);
    let (replies, _) = run_env(&[], &line, &[]);
    assert_eq!(replies.len(), 1, "no sibling without display-list-v2");
    let echoed: Vec<&str> = replies[0]
        .get("payload")
        .unwrap()
        .get("layout_capabilities")
        .and_then(|v| v.as_arr())
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(echoed, vec!["rules-v1"], "{echoed:?}");

    // A degenerate window is not an error: the reply is unwindowed and the
    // name is absent from the echo, which is the consumer's only signal (§8).
    let caps = &["display-list-v2", "display-list-v2-window"];
    for (first, count) in [(0, 4), (1, 0)] {
        let line = with_window_field(&compile_line("bad", text, Some(caps)), first, count);
        let (replies, _) = run_env(&[], &line, &[]);
        assert_eq!(replies.len(), 2, "still a reply");
        assert!(replies[1].get("payload").unwrap().get("window").is_none(), "{first}:{count} is not a window");
        let echoed: Vec<&str> = replies[0]
            .get("payload")
            .unwrap()
            .get("layout_capabilities")
            .and_then(|v| v.as_arr())
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(echoed, vec!["display-list-v2"], "{first}:{count} should not echo the window");
    }
}

/// Inserts `display_list_window` into a request line built by `compile_line`.
fn with_window_field(line: &str, first_page: i64, page_count: i64) -> String {
    let mut v = json::parse(line.trim()).expect("request is JSON");
    let mut payload = v.get("payload").expect("payload").clone();
    let mut w = json::Value::obj();
    w.set("first_page", json::num(first_page as f64));
    w.set("page_count", json::num(page_count as f64));
    payload.set("display_list_window", w);
    v.set("payload", payload);
    json::write(&v) + "\n"
}
