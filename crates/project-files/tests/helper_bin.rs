//! The `flashtex-project-files --root DIR` JSON Lines host, driven as a child
//! process the way the Mac shell drives it. Temp directories only.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use flashtex_project_files::json::Json;
use flashtex_project_files::sha256_hex;

mod common;

fn run(root: &std::path::Path, requests: &[&str]) -> Vec<Json> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-project-files"))
        .arg("--root")
        .arg(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn helper");
    {
        let mut stdin = child.stdin.take().unwrap();
        for r in requests {
            writeln!(stdin, "{r}").unwrap();
        }
    }
    let stdout = child.stdout.take().unwrap();
    let replies: Vec<Json> = BufReader::new(stdout)
        .lines()
        .map(|l| Json::parse(&l.unwrap()).expect("reply is JSON"))
        .collect();
    let status = child.wait().unwrap();
    assert!(status.success(), "helper exited {status:?}");
    replies
}

fn payload<'a>(reply: &'a Json, id: &str) -> &'a Json {
    assert_eq!(reply.get("id").and_then(Json::as_str), Some(id));
    reply
        .get("payload")
        .unwrap_or_else(|| panic!("expected payload, got {}", reply.to_string_compact()))
}

fn error_code<'a>(reply: &'a Json, id: &str) -> &'a str {
    assert_eq!(reply.get("id").and_then(Json::as_str), Some(id));
    reply
        .get("error")
        .and_then(|e| e.get("code"))
        .and_then(Json::as_str)
        .unwrap_or_else(|| panic!("expected error, got {}", reply.to_string_compact()))
}

#[test]
fn ping_read_status_save_and_conflict_over_the_wire() {
    let tmp = common::TempDir::new("helper-bin");
    let root = tmp.root();
    std::fs::write(root.join("a.tex"), "hello\n").unwrap();
    let h1 = sha256_hex(b"hello\n");
    let h2 = sha256_hex(b"hello\nworld\n");
    let replies = run(
        root,
        &[
            r#"{"id":"1","operation":"ping"}"#,
            r#"{"id":"2","operation":"read","path":"a.tex"}"#,
            &format!(
                r#"{{"id":"3","operation":"status","path":"a.tex","expected_sha256":"{h1}"}}"#
            ),
            &format!(
                r#"{{"id":"4","operation":"save","path":"a.tex","text":"hello\nworld\n","expected":"{h1}"}}"#
            ),
            &format!(
                r#"{{"id":"5","operation":"save","path":"a.tex","text":"again\n","expected":"{h1}"}}"#
            ),
            &format!(
                r#"{{"id":"6","operation":"status","path":"a.tex","expected_sha256":"{h1}"}}"#
            ),
            r#"{"id":"7","operation":"read","path":"missing.tex"}"#,
            r#"{"id":"8","operation":"status","path":"missing.tex","expected_sha256":null}"#,
            r#"{"id":"9","operation":"save","path":"new/dir/n.tex","text":"n","expected":"new"}"#,
            r#"{"id":"10","operation":"save","path":"new/dir/n.tex","text":"n2","expected":"new"}"#,
            &format!(
                r#"{{"id":"11","operation":"save","path":"a.tex","text":"forced\n","expected":"{h1}","force":true}}"#
            ),
        ],
    );
    assert_eq!(replies.len(), 11);
    let p = payload(&replies[0], "1");
    assert_eq!(
        p.get("protocol").and_then(Json::as_str),
        Some("project-files-v1")
    );
    let p = payload(&replies[1], "2");
    assert_eq!(p.get("text").and_then(Json::as_str), Some("hello\n"));
    assert_eq!(p.get("sha256").and_then(Json::as_str), Some(h1.as_str()));
    assert_eq!(
        payload(&replies[2], "3")
            .get("state")
            .and_then(Json::as_str),
        Some("unchanged")
    );
    let p = payload(&replies[3], "4");
    assert_eq!(p.get("outcome").and_then(Json::as_str), Some("saved"));
    assert_eq!(
        p.get("receipt")
            .and_then(|r| r.get("sha256"))
            .and_then(Json::as_str),
        Some(h2.as_str())
    );
    let p = payload(&replies[4], "5");
    assert_eq!(p.get("outcome").and_then(Json::as_str), Some("conflict"));
    let c = p.get("conflict").unwrap();
    assert_eq!(
        c.get("kind").and_then(Json::as_str),
        Some("modified_externally")
    );
    assert_eq!(c.get("ours").and_then(Json::as_str), Some(h1.as_str()));
    assert_eq!(c.get("theirs").and_then(Json::as_str), Some(h2.as_str()));
    assert_eq!(
        payload(&replies[5], "6")
            .get("state")
            .and_then(Json::as_str),
        Some("modified")
    );
    assert_eq!(
        payload(&replies[6], "7").get("exists"),
        Some(&Json::Bool(false))
    );
    assert_eq!(
        payload(&replies[7], "8")
            .get("state")
            .and_then(Json::as_str),
        Some("unchanged")
    );
    assert_eq!(
        payload(&replies[8], "9")
            .get("outcome")
            .and_then(Json::as_str),
        Some("saved")
    );
    let c = payload(&replies[9], "10").get("conflict").unwrap();
    assert_eq!(c.get("kind").and_then(Json::as_str), Some("already_exists"));
    assert_eq!(
        payload(&replies[10], "11")
            .get("outcome")
            .and_then(Json::as_str),
        Some("saved")
    );
    assert_eq!(
        std::fs::read_to_string(root.join("a.tex")).unwrap(),
        "forced\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("new/dir/n.tex")).unwrap(),
        "n"
    );
}

#[test]
fn refusals_and_bad_requests_are_errors_and_write_nothing() {
    let tmp = common::TempDir::new("helper-bin-refuse");
    let root = tmp.root();
    let outside = common::TempDir::new("helper-bin-outside");
    let secret = outside.root().join("secret.tex");
    std::fs::write(&secret, "secret\n").unwrap();
    std::os::unix::fs::symlink(&secret, root.join("link.tex")).unwrap();
    let replies = run(
        root,
        &[
            r#"{"id":"1","operation":"save","path":"link.tex","text":"x","expected":"any","force":true}"#,
            r#"{"id":"2","operation":"read","path":"link.tex"}"#,
            r#"{"id":"3","operation":"read","path":"../secret.tex"}"#,
            r#"{"id":"4","operation":"read","path":"/etc/hosts"}"#,
            r#"{"id":"5","operation":"nope"}"#,
            r#"{"id":"6","operation":"save","path":"a.tex","text":"x","expected":"zz"}"#,
            r#"{"id":"7","operation":"save","path":"a.tex","text":"x","expected":"new","force":"yes"}"#,
            r#"not json"#,
            r#"{"id":"8","operation":"read"}"#,
        ],
    );
    assert_eq!(replies.len(), 9);
    assert_eq!(error_code(&replies[0], "1"), "refused");
    assert_eq!(error_code(&replies[1], "2"), "refused");
    assert_eq!(error_code(&replies[2], "3"), "invalid_path");
    assert_eq!(error_code(&replies[3], "4"), "invalid_path");
    assert_eq!(error_code(&replies[4], "5"), "unsupported_operation");
    assert_eq!(error_code(&replies[5], "6"), "invalid_request");
    assert_eq!(error_code(&replies[6], "7"), "invalid_request");
    assert_eq!(replies[7].get("id"), Some(&Json::Null));
    assert_eq!(
        replies[7]
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(Json::as_str),
        Some("invalid_request")
    );
    assert_eq!(error_code(&replies[8], "8"), "invalid_request");
    assert_eq!(std::fs::read_to_string(&secret).unwrap(), "secret\n");
    assert!(!root.join("a.tex").exists());
}

#[test]
fn symlinked_root_is_refused_at_startup() {
    let real = common::TempDir::new("helper-bin-realroot");
    let holder = common::TempDir::new("helper-bin-holder");
    let link = holder.root().join("root-link");
    std::os::unix::fs::symlink(real.root(), &link).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-project-files"))
        .arg("--root")
        .arg(&link)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cannot open root"));
}

/// The `manifest` operation: the governing `flashtex.toml` (walked up to
/// from the root), the classified `texinputs`, the package inputs with
/// their text, the template for the given entry, and the defaults when
/// there is no manifest at all.
#[test]
fn manifest_operation_serves_the_manifest_its_inputs_and_the_template() {
    let tmp = common::TempDir::new("helper-manifest");
    let root = tmp.root();
    std::fs::create_dir_all(root.join("styles")).unwrap();
    std::fs::create_dir_all(root.join("../helper-manifest-shared")).unwrap();
    std::fs::write(root.join("mystyle.sty"), "\\def\\x{1}\n").unwrap();
    std::fs::write(root.join("styles/a.cls"), "class\n").unwrap();
    std::fs::write(
        root.join("flashtex.toml"),
        "[project]\nentry = \"paper.tex\"\ntexinputs = [\"styles\", \"/abs\"]\n[fonts]\nserif = \"x\"\n",
    )
    .unwrap();
    let replies = run(
        root,
        &[
            r#"{"id":"1","operation":"manifest","entry":"paper.tex"}"#,
            r#"{"id":"2","operation":"manifest","entry":3}"#,
        ],
    );
    let p = payload(&replies[0], "1");
    assert_eq!(p.get("exists"), Some(&Json::Bool(true)));
    assert_eq!(
        p.get("path").and_then(Json::as_str),
        Some(root.join("flashtex.toml").to_str().unwrap())
    );
    let project = p.get("manifest").unwrap().get("project").unwrap();
    assert_eq!(project.get("entry").and_then(Json::as_str), Some("paper.tex"));
    assert_eq!(
        p.get("manifest").unwrap().get("packages").unwrap().get("fetch").and_then(Json::as_str),
        Some("ask")
    );
    let warnings = match p.get("warnings") {
        Some(Json::Array(w)) => w,
        other => panic!("warnings: {other:?}"),
    };
    assert_eq!(warnings[0].get("key").and_then(Json::as_str), Some("fonts.serif"));
    let texinputs = match p.get("texinputs") {
        Some(Json::Array(t)) => t,
        other => panic!("texinputs: {other:?}"),
    };
    assert_eq!(texinputs[0].get("location").and_then(Json::as_str), Some("inside"));
    assert_eq!(texinputs[1].get("location").and_then(Json::as_str), Some("invalid"));
    let files = match p.get("files") {
        Some(Json::Array(f)) => f,
        other => panic!("files: {other:?}"),
    };
    let listed: Vec<(&str, &str)> = files
        .iter()
        .map(|f| (f.get("path").unwrap().as_str().unwrap(), f.get("kind").unwrap().as_str().unwrap()))
        .collect();
    assert_eq!(listed, [("mystyle.sty", "package"), ("styles/a.cls", "class")]);
    assert_eq!(files[0].get("text").and_then(Json::as_str), Some("\\def\\x{1}\n"));
    assert_eq!(files[0].get("texinput"), Some(&Json::Null));
    assert_eq!(files[1].get("texinput").and_then(Json::as_u64), Some(0));
    let diagnostics = match p.get("diagnostics") {
        Some(Json::Array(d)) => d,
        other => panic!("diagnostics: {other:?}"),
    };
    assert_eq!(diagnostics[0].get("key").and_then(Json::as_str), Some("project.texinputs[1]"));
    let template = p.get("template").and_then(Json::as_str).unwrap();
    assert!(template.contains("entry = \"paper.tex\""), "{template}");
    assert_eq!(error_code(&replies[1], "2"), "invalid_request");

    // No manifest anywhere up to the filesystem root: defaults, and the
    // root's own package files are still the package inputs.
    let bare = common::TempDir::new("helper-no-manifest");
    std::fs::write(bare.root().join("local.sty"), "s").unwrap();
    let replies = run(bare.root(), &[r#"{"id":"1","operation":"manifest"}"#]);
    let p = payload(&replies[0], "1");
    assert_eq!(p.get("exists"), Some(&Json::Bool(false)));
    assert_eq!(p.get("path"), Some(&Json::Null));
    assert_eq!(
        p.get("manifest").unwrap().get("project").unwrap().get("entry"),
        Some(&Json::Null)
    );
    let files = match p.get("files") {
        Some(Json::Array(f)) => f,
        other => panic!("files: {other:?}"),
    };
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].get("path").and_then(Json::as_str), Some("local.sty"));
    assert!(p.get("template").and_then(Json::as_str).unwrap().contains("entry = \"main.tex\""));
}

/// The `set_fonts` operation: the governing manifest's `[fonts]` table
/// rewritten (everything else kept), the template when there is none,
/// nothing to write when there is none and nothing is named, and the
/// refusals. It never writes: the consumer saves the text it returns.
#[test]
fn set_fonts_rewrites_the_fonts_table_and_writes_nothing_itself() {
    let tmp = common::TempDir::new("helper-set-fonts");
    let root = tmp.root();
    // No manifest and nothing named: nothing to write.
    let replies = run(root, &[r#"{"id":"1","operation":"set_fonts","fonts":{}}"#]);
    let p = payload(&replies[0], "1");
    assert_eq!(p.get("exists"), Some(&Json::Bool(false)));
    assert_eq!(p.get("changed"), Some(&Json::Bool(false)));
    assert!(p.get("text").is_none());
    assert_eq!(p.get("path").and_then(Json::as_str), Some(root.join("flashtex.toml").to_str().unwrap()));
    // No manifest and a family named: the template for `entry`, with the table.
    let replies = run(root, &[r#"{"id":"2","operation":"set_fonts","entry":"paper.tex","fonts":{"text":"Georgia","math":null}}"#]);
    let p = payload(&replies[0], "2");
    assert_eq!(p.get("changed"), Some(&Json::Bool(true)));
    let text = p.get("text").and_then(Json::as_str).unwrap();
    assert!(text.contains("entry = \"paper.tex\"") && text.contains("[fonts]") && text.contains("\ntext = \"Georgia\"\n"), "{text}");
    assert!(!root.join("flashtex.toml").exists(), "set_fonts writes nothing");
    // An existing manifest: its other content byte for byte, the table replaced.
    let original = "# mine\n[project]\nentry = \"paper.tex\"\n\n[fonts]\ntext = \"Old\"\nsans = \"Old Sans\"\n\n[packages]\nfetch = \"never\"\n";
    std::fs::write(root.join("flashtex.toml"), original).unwrap();
    let replies = run(
        root,
        &[
            r#"{"id":"3","operation":"set_fonts","fonts":{"text":"Georgia","mono":"Menlo"}}"#,
            r#"{"id":"4","operation":"set_fonts","fonts":{"text":3}}"#,
            r#"{"id":"5","operation":"set_fonts","fonts":{"serif":"x"}}"#,
            r#"{"id":"6","operation":"set_fonts","fonts":"Georgia"}"#,
            r#"{"id":"7","operation":"set_fonts","fonts":{"text":"Old","sans":"Old Sans"}}"#,
        ],
    );
    let p = payload(&replies[0], "3");
    assert_eq!(p.get("exists"), Some(&Json::Bool(true)));
    assert_eq!(p.get("changed"), Some(&Json::Bool(true)));
    assert_eq!(
        p.get("text").and_then(Json::as_str),
        Some("# mine\n[project]\nentry = \"paper.tex\"\n\n[fonts]\ntext = \"Georgia\"\nmono = \"Menlo\"\n\n[packages]\nfetch = \"never\"\n")
    );
    assert_eq!(std::fs::read_to_string(root.join("flashtex.toml")).unwrap(), original, "untouched on disk");
    assert_eq!(error_code(&replies[1], "4"), "invalid_request");
    assert_eq!(error_code(&replies[2], "5"), "invalid_request");
    assert_eq!(error_code(&replies[3], "6"), "invalid_request");
    // The same table as on disk: nothing changed.
    let p = payload(&replies[4], "7");
    assert_eq!(p.get("changed"), Some(&Json::Bool(false)));
    assert_eq!(p.get("text").and_then(Json::as_str), Some(original));
}
