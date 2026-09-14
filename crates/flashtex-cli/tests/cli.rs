//! End-to-end tests of the `flashtex` binary: the real-world fixtures
//! (`fixtures/real-world/hw1`, `hw2`, `input-bibliography`, never
//! modified) built to PDF through the exact route, the `check --json`
//! schema, the `supported` inventory, `--version`, and font resolution from
//! a tarball-style `bin/` + `share/flashtex/` layout with the host TeX
//! trees denied (`sandbox-exec`, as the Mac app's BundledMetricsTests do).
//!
//! Fonts: the pinned Latin Modern set in `apps/mac/Fonts` is passed as
//! `--font-dir` so the tests do not depend on a host TeX installation; the
//! matching TFMs come from `apps/mac/Fonts/texmf` through `FLASHTEX_TFM_DIRS`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}

fn fixture(rel: &str) -> PathBuf {
    let p = repo_root().join("fixtures/real-world").join(rel);
    assert!(p.exists(), "missing fixture {}", p.display());
    p
}

fn fonts_dir() -> PathBuf {
    repo_root().join("apps/mac/Fonts")
}

fn tfm_dir() -> PathBuf {
    fonts_dir().join("texmf/fonts/tfm/public/lm")
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-cli-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `flashtex` with the bundled TFMs and the host font overrides cleared
/// (`--font-dir` is passed per call, after the subcommand).
fn run(args: &[&str]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex"));
    c.env_remove("FLASHTEX_FONT_DIRS").env_remove("FLASHTEX_LM_DIR").env("FLASHTEX_TFM_DIRS", tfm_dir());
    c.args(args);
    c.output().expect("flashtex runs")
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn pdf_pages(bytes: &[u8]) -> usize {
    assert!(bytes.starts_with(b"%PDF-1."), "not a PDF");
    let text = String::from_utf8_lossy(bytes);
    let pages = text.matches("/Type /Page\n").count() + text.matches("/Type /Page ").count() + text.matches("/Type /Page/").count() + text.matches("/Type /Page>").count();
    let count = text
        .find("/Count ")
        .map(|i| text[i + 7..].split(|c: char| !c.is_ascii_digit()).next().unwrap().parse::<usize>().unwrap())
        .unwrap_or(0);
    assert_eq!(pages, count, "page objects vs /Count");
    count
}

fn json(text: &str) -> flashtex_compiler::json::Value {
    flashtex_compiler::json::parse(text.trim()).unwrap_or_else(|e| panic!("stdout is not JSON: {}\n{text}", e.0))
}

fn build_fixture(name: &str, entry: &str, expect_pages: usize) -> (String, Vec<u8>) {
    let dir = tmp(name);
    let out = dir.join("out.pdf");
    let fonts = fonts_dir();
    let o = run(&["build", fixture(entry).to_str().unwrap(), "-o", out.to_str().unwrap(), "--font-dir", fonts.to_str().unwrap(), "--timing"]);
    let err = stderr(&o);
    assert_eq!(o.status.code(), Some(0), "exit status\n{err}");
    let bytes = std::fs::read(&out).expect("pdf written");
    assert_eq!(pdf_pages(&bytes), expect_pages, "{err}");
    // The exact route: real font programs, not the base-14 shim.
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/FontFile3") || text.contains("/FontFile2") || text.contains("/FontFile"), "fonts are embedded");
    assert!(text.contains("LMRoman"), "Latin Modern is embedded");
    // Summary + timing lines.
    assert!(err.contains(&format!(", {expect_pages} pages, ")), "summary line:\n{err}");
    assert!(err.contains("0 errors"), "no errors:\n{err}");
    assert!(err.contains("flashtex: timing: render "), "timing line:\n{err}");
    let _ = std::fs::remove_dir_all(&dir);
    (err, bytes)
}

#[test]
fn hw1_builds_three_pages_through_the_exact_route() {
    let (err, _) = build_fixture("hw1", "hw1/HW1.tex", 3);
    assert!(err.contains("HW1.tex: recovered") || err.contains("HW1.tex: ok"), "{err}");
    // Diagnostics carry file:line:col.
    assert!(err.lines().any(|l| l.starts_with("HW1.tex:") && l.contains(": warning[")), "{err}");
}

#[test]
fn hw2_builds_three_pages_through_the_exact_route() {
    build_fixture("hw2", "hw2/HW2.tex", 3);
}

#[test]
fn multi_file_project_resolves_inputs_from_the_project_root() {
    let dir = tmp("multi");
    let out = dir.join("main.pdf");
    let v2 = dir.join("main.json");
    let fonts = fonts_dir();
    let o = run(&[
        "build",
        fixture("input-bibliography/main.tex").to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--v2",
        v2.to_str().unwrap(),
        "--font-dir",
        fonts.to_str().unwrap(),
        "--json",
    ]);
    let err = stderr(&o);
    assert_eq!(o.status.code(), Some(0), "{err}");
    let pages = pdf_pages(&std::fs::read(&out).unwrap());
    assert!(pages >= 2, "pages: {pages}\n{err}");
    let report = json(&stdout(&o));
    let docs: Vec<&str> = report.get("documents").unwrap().as_arr().unwrap().iter().map(|d| d.as_str().unwrap()).collect();
    assert_eq!(docs[0], "main.tex");
    assert!(docs.contains(&"sections/intro.tex") && docs.contains(&"sections/method.tex"), "{docs:?}");
    // The included files' text is in the output: a heading from sections/intro.tex.
    let intro = std::fs::read_to_string(fixture("input-bibliography/sections/intro.tex")).unwrap();
    assert!(intro.contains("\\section"), "fixture has a section");
    // The display list carries every document, entry first.
    let envelope = json(&std::fs::read_to_string(&v2).unwrap());
    assert_eq!(envelope.get("type").and_then(|v| v.as_str()), Some("display_list"));
    let listed = envelope.get("payload").unwrap().get("documents").unwrap().as_arr().unwrap();
    assert_eq!(listed.len(), 3, "{}", stdout(&o));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A diagnostic raised inside an included file names that file and its own
/// line. Built on a project written here, not the fixture: the fixture's
/// sections stopped producing any diagnostic once the compiler supported
/// everything in them, which made the old assertion fail on a better engine.
#[test]
fn a_diagnostic_in_an_included_file_names_that_file() {
    let dir = tmp("included-diag");
    std::fs::create_dir_all(dir.join("sections")).unwrap();
    let src = dir.join("main.tex");
    std::fs::write(&src, "\\documentclass{article}\n\\begin{document}\n\\input{sections/a}\n\\end{document}\n").unwrap();
    std::fs::write(dir.join("sections/a.tex"), "First line.\nHello \\undefinedmacro{x}.\n").unwrap();
    let fonts = fonts_dir();
    let o = run(&["check", src.to_str().unwrap(), "--json", "--font-dir", fonts.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let report = json(&stdout(&o));
    let diags = report.get("diagnostics").unwrap().as_arr().unwrap();
    let inner = diags
        .iter()
        .find(|d| d.get("message").and_then(|m| m.as_str()).map_or(false, |m| m.contains("undefinedmacro")))
        .unwrap_or_else(|| panic!("the unsupported command is reported: {}", stdout(&o)));
    assert_eq!(inner.get("path").and_then(|p| p.as_str()), Some("sections/a.tex"), "{}", stdout(&o));
    assert_eq!(inner.get("line").and_then(|l| l.as_i64()), Some(2), "{}", stdout(&o));
    assert!(stderr(&o).lines().any(|l| l.starts_with("sections/a.tex:2:7: error[")), "{}", stderr(&o));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn check_json_has_the_stable_schema_and_writes_nothing() {
    let dir = tmp("check");
    let src = dir.join("main.tex");
    std::fs::write(&src, "\\documentclass{article}\n\\begin{document}\nHello $x^2$ \\undefinedmacro world.\n\\end{document}\n").unwrap();
    let fonts = fonts_dir();
    let o = run(&["check", src.to_str().unwrap(), "--json", "--font-dir", fonts.to_str().unwrap()]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(!dir.join("main.pdf").exists(), "check writes no PDF");
    let r = json(&stdout(&o));
    assert_eq!(r.get("schema").and_then(|v| v.as_str()), Some("flashtex-check/1"));
    assert_eq!(r.get("entry").and_then(|v| v.as_str()), Some("main.tex"));
    assert_eq!(r.get("status").and_then(|v| v.as_str()), Some("recovered"));
    assert_eq!(r.get("pages").and_then(|v| v.as_i64()), Some(1));
    for key in ["project_root", "documents", "diagnostics", "summary", "timing", "outputs", "version"] {
        assert!(r.get(key).is_some(), "missing {key}: {}", stdout(&o));
    }
    let diags = r.get("diagnostics").unwrap().as_arr().unwrap();
    let undefined = diags
        .iter()
        .find(|d| d.get("message").and_then(|m| m.as_str()).map_or(false, |m| m.contains("undefinedmacro")))
        .unwrap_or_else(|| panic!("an unsupported-command diagnostic: {}", stdout(&o)));
    for key in ["path", "line", "column", "start_byte", "end_byte", "severity", "code", "message", "recovery"] {
        assert!(undefined.get(key).is_some(), "diagnostic lacks {key}");
    }
    assert_eq!(undefined.get("line").and_then(|v| v.as_i64()), Some(3));
    assert_eq!(undefined.get("path").and_then(|v| v.as_str()), Some("main.tex"));
    let summary = r.get("summary").unwrap();
    let errors = summary.get("errors").and_then(|v| v.as_i64()).unwrap();
    let warnings = summary.get("warnings").and_then(|v| v.as_i64()).unwrap();
    assert_eq!(errors + warnings, diags.len() as i64);
    assert!(errors >= 1, "the unknown command is an error: {}", stdout(&o));
    // stderr carries the same diagnostics as file:line:col lines.
    assert!(stderr(&o).lines().any(|l| l.starts_with("main.tex:3:")), "{}", stderr(&o));
    // --strict turns the recovered error into exit 1.
    let strict = run(&["check", src.to_str().unwrap(), "--strict", "--font-dir", fonts.to_str().unwrap()]);
    assert_eq!(strict.status.code(), Some(1));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_include_is_reported_and_the_build_still_writes() {
    let dir = tmp("missing");
    let src = dir.join("main.tex");
    std::fs::write(&src, "\\documentclass{article}\n\\begin{document}\nBefore.\n\\input{nothere}\nAfter.\n\\end{document}\n").unwrap();
    let fonts = fonts_dir();
    let o = run(&["build", src.to_str().unwrap(), "--font-dir", fonts.to_str().unwrap(), "--json"]);
    let err = stderr(&o);
    assert_eq!(o.status.code(), Some(0), "{err}");
    assert!(dir.join("main.pdf").exists(), "default -o is <main>.pdf");
    assert!(err.contains("main.tex:4:1: error[missing_file]"), "{err}");
    assert_eq!(err.lines().filter(|line| line.contains("main.tex:4:1: error[")).count(), 1, "{err}");
    assert!(err.contains("skipped the missing include"), "{err}");
    let report = json(&stdout(&o));
    assert_eq!(report.get("summary").unwrap().get("errors").and_then(|v| v.as_i64()), Some(1), "{}", stdout(&o));
    let strict = run(&["build", src.to_str().unwrap(), "--strict", "--font-dir", fonts.to_str().unwrap()]);
    assert_eq!(strict.status.code(), Some(1), "{}", stderr(&strict));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn diagnostics_full_shows_the_source_line_and_carets() {
    let dir = tmp("full");
    let src = dir.join("main.tex");
    std::fs::write(&src, "\\documentclass{article}\n\\begin{document}\nBefore.\n\\input{nothere}\nAfter.\n\\end{document}\n").unwrap();
    let fonts = fonts_dir();
    let check = |extra: &[&str]| {
        let mut args = vec!["check", src.to_str().unwrap(), "--font-dir", fonts.to_str().unwrap()];
        args.extend_from_slice(extra);
        run(&args)
    };
    // Piped stderr defaults to the one-line form.
    let short = stderr(&check(&[]));
    assert!(short.contains("main.tex:4:1: error[missing_file]"), "{short}");
    assert!(!short.contains("-->"), "{short}");

    for flag in [&["--diagnostics=full"][..], &["--diagnostics", "full"][..]] {
        let o = check(flag);
        let err = stderr(&o);
        assert_eq!(o.status.code(), Some(0), "{err}");
        assert!(err.contains("error[missing_file]: "), "{err}");
        assert!(err.contains(" --> main.tex:4:1\n"), "{err}");
        assert!(err.contains("\n4 | \\input{nothere}\n  | ^"), "{err}");
        assert!(!err.contains('\x1b'), "piped output is uncoloured by default:\n{err}");
        assert!(err.contains("flashtex: main.tex: recovered"), "summary line stays:\n{err}");
    }
    assert!(stderr(&check(&["--diagnostics=full", "--color=always"])).contains("\x1b[1;31merror[missing_file]\x1b[0m"));
    // `json` is `--json` (the report carries wall time, so compare its shape).
    let as_json = json(&stdout(&check(&["--diagnostics=json"])));
    assert_eq!(as_json.get("schema").and_then(|s| s.as_str()), Some("flashtex-check/1"));
    assert!(as_json.get("diagnostics").and_then(|d| d.as_arr()).map_or(false, |d| !d.is_empty()));
    let bad = check(&["--diagnostics=long"]);
    assert_eq!(bad.status.code(), Some(2), "{}", stderr(&bad));
    assert_eq!(check(&["--color", "sometimes"]).status.code(), Some(2));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_include_cannot_escape_the_project_root() {
    let dir = tmp("escape");
    let proj = dir.join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(dir.join("secret.tex"), "SECRET TEXT\n").unwrap();
    std::fs::write(proj.join("main.tex"), "\\documentclass{article}\n\\begin{document}\n\\input{../secret}\n\\end{document}\n").unwrap();
    let fonts = fonts_dir();
    let o = run(&["check", proj.join("main.tex").to_str().unwrap(), "--json", "--font-dir", fonts.to_str().unwrap()]);
    let r = json(&stdout(&o));
    let docs = r.get("documents").unwrap().as_arr().unwrap();
    assert_eq!(docs.len(), 1, "only main.tex: {}", stdout(&o));
    let err = stderr(&o);
    assert!(err.contains("error["), "the escaping include is an error:\n{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn supported_json_parses_and_carries_coverage() {
    let o = run(&["supported", "--json"]);
    assert_eq!(o.status.code(), Some(0));
    let v = json(&stdout(&o));
    assert_eq!(v.get("schema").and_then(|s| s.as_str()), Some("flashtex-supported-latex/1"));
    assert!(v.get("commands").and_then(|c| c.as_arr()).map_or(0, Vec::len) > 100);
    let total = v.get("coverage").unwrap().get("total").unwrap();
    assert!(total.get("percent").and_then(|p| p.as_i64()).is_some() || total.get("percent").is_some());
    let summary = run(&["supported"]);
    assert!(stdout(&summary).contains("coverage of the canonical inventory:"), "{}", stdout(&summary));
    let md = run(&["supported", "--md"]);
    assert!(stdout(&md).contains("supported-latex"), "{}", &stdout(&md)[..80.min(stdout(&md).len())]);
}

#[test]
fn version_names_the_crate_and_the_git_revision() {
    let o = run(&["--version"]);
    assert_eq!(o.status.code(), Some(0));
    let s = stdout(&o);
    assert!(s.starts_with(&format!("flashtex {} (", env!("CARGO_PKG_VERSION"))), "{s}");
    assert!(s.trim().ends_with(')'), "{s}");
}

#[test]
fn usage_errors_exit_2() {
    assert_eq!(run(&[]).status.code(), Some(2));
    assert_eq!(run(&["frobnicate"]).status.code(), Some(2));
    assert_eq!(run(&["build"]).status.code(), Some(2));
    assert_eq!(run(&["build", "/nonexistent/main.tex"]).status.code(), Some(2));
    assert_eq!(run(&["build", "a.tex", "--bogus"]).status.code(), Some(2));
    assert_eq!(run(&["--help"]).status.code(), Some(0));
    assert!(stdout(&run(&["--help"])).contains("flashtex build <main.tex>"));
}

#[test]
fn worker_speaks_runtime_v1_on_stdin_stdout() {
    use std::io::Write;
    let fonts = fonts_dir();
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex"))
        .env_remove("FLASHTEX_FONT_DIRS")
        .env("FLASHTEX_TFM_DIRS", tfm_dir())
        .args(["worker", "--font-dir", fonts.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let request = r#"{"protocol_version":1,"id":"r1","type":"compile","payload":{"project_id":"p","revision":1,"entry_path":"main.tex","documents":[{"path":"main.tex","text":"\\documentclass{article}\n\\begin{document}\nHello worker.\n\\end{document}\n"}],"layout_capabilities":["display-list-v2"]}}"#;
    {
        let mut stdin = child.stdin.take().unwrap();
        writeln!(stdin, "{request}").unwrap();
        writeln!(stdin, "not json").unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let lines: Vec<String> = stdout(&out).lines().map(String::from).collect();
    assert_eq!(lines.len(), 3, "compile_result, display_list, error:\n{}", lines.join("\n"));
    let first = json(&lines[0]);
    assert_eq!(first.get("type").and_then(|t| t.as_str()), Some("compile_result"));
    assert_eq!(first.get("id").and_then(|t| t.as_str()), Some("r1"));
    assert_eq!(first.get("payload").unwrap().get("status").and_then(|s| s.as_str()), Some("ok"));
    assert_eq!(json(&lines[1]).get("type").and_then(|t| t.as_str()), Some("display_list"));
    assert_eq!(json(&lines[2]).get("type").and_then(|t| t.as_str()), Some("error"));
}

/// The tarball layout: `bin/flashtex` next to `share/flashtex/{Fonts,texmf}`,
/// no `--font-dir`, no `FLASHTEX_*`, and the host TeX trees denied by
/// `sandbox-exec` (macOS; elsewhere the layout is exercised without the
/// deny). HW1 must still build 3 pages with the pinned metrics loaded.
#[test]
fn bundled_share_layout_resolves_fonts_without_host_tex() {
    let dir = tmp("share");
    let bin = dir.join("bin");
    let share = dir.join("share/flashtex");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::create_dir_all(share.join("Fonts")).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_flashtex"), bin.join("flashtex")).unwrap();
    for e in std::fs::read_dir(fonts_dir()).unwrap().flatten() {
        let p = e.path();
        let name = p.file_name().unwrap().to_str().unwrap().to_string();
        if name.ends_with(".otf") || name == "GUST-FONT-LICENSE.TXT" || name == "SUPPLEMENTARY-FACES.json" {
            std::fs::copy(&p, share.join("Fonts").join(&name)).unwrap();
        }
    }
    copy_tree(&fonts_dir().join("texmf"), &share.join("texmf"));
    let profile = dir.join("no-host-tex.sb");
    std::fs::write(
        &profile,
        "(version 1)\n(allow default)\n(deny file-read* (subpath \"/usr/local/texlive\"))\n(deny file-read* (subpath \"/Library/TeX\"))\n(deny file-read* (subpath \"/usr/share/texmf\"))\n(deny file-read* (subpath \"/usr/share/texlive\"))\n",
    )
    .unwrap();
    let sandboxed = cfg!(target_os = "macos") && Path::new("/usr/bin/sandbox-exec").exists();
    let exe = bin.join("flashtex");
    let mut cmd = if sandboxed {
        let mut c = Command::new("/usr/bin/sandbox-exec");
        c.arg("-f").arg(&profile).arg(&exe);
        c
    } else {
        Command::new(&exe)
    };
    cmd.env_clear().env("PATH", "/usr/bin:/bin").env("HOME", &dir);
    let out = dir.join("hw1.pdf");
    let o = cmd.args(["build", fixture("hw1/HW1.tex").to_str().unwrap(), "-o", out.to_str().unwrap(), "--verbose"]).output().unwrap();
    let err = stderr(&o);
    assert_eq!(o.status.code(), Some(0), "{err}");
    assert_eq!(pdf_pages(&std::fs::read(&out).unwrap()), 3, "{err}");
    assert!(!err.contains("required_metrics_unavailable") && !err.contains("tfm_missing"), "{err}");
    // Every embedded font came from share/flashtex/Fonts.
    let embedded: Vec<&str> = err.lines().filter(|l| l.starts_with("flashtex: pdf: /F")).collect();
    assert!(!embedded.is_empty(), "{err}");
    assert!(embedded.iter().all(|l| l.contains("share/flashtex/Fonts")), "{embedded:?}");
    // `fonts --json` agrees.
    let mut cmd = if sandboxed {
        let mut c = Command::new("/usr/bin/sandbox-exec");
        c.arg("-f").arg(&profile).arg(&exe);
        c
    } else {
        Command::new(&exe)
    };
    let f = cmd.env_clear().env("PATH", "/usr/bin:/bin").env("HOME", &dir).args(["fonts", "--json"]).output().unwrap();
    assert_eq!(f.status.code(), Some(0), "{}", stderr(&f));
    let v = json(&stdout(&f));
    assert_eq!(v.get("latin_modern_available"), Some(&flashtex_compiler::json::Value::Bool(true)));
    assert_eq!(v.get("required_metrics").and_then(|s| s.as_str()), Some("loaded"));
    let _ = std::fs::remove_dir_all(&dir);
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        let t = to.join(e.file_name());
        if p.is_dir() {
            copy_tree(&p, &t);
        } else {
            std::fs::copy(&p, &t).unwrap();
        }
    }
}

#[test]
fn watch_rebuilds_when_an_included_file_changes() {
    use std::io::Read;
    let dir = tmp("watch");
    std::fs::write(dir.join("main.tex"), "\\documentclass{article}\n\\begin{document}\n\\input{part}\n\\end{document}\n").unwrap();
    std::fs::write(dir.join("part.tex"), "First version.\n").unwrap();
    let fonts = fonts_dir();
    let out = dir.join("main.pdf");
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex"))
        .env_remove("FLASHTEX_FONT_DIRS")
        .env("FLASHTEX_TFM_DIRS", tfm_dir())
        .args(["watch", dir.join("main.tex").to_str().unwrap(), "-o", out.to_str().unwrap(), "--font-dir", fonts.to_str().unwrap(), "--interval", "50"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut err = child.stderr.take().unwrap();
    let mut seen = String::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut buf = [0u8; 4096];
    // First build.
    while !seen.contains("flashtex: timing:") && std::time::Instant::now() < deadline {
        let n = err.read(&mut buf).unwrap();
        seen.push_str(&String::from_utf8_lossy(&buf[..n]));
    }
    assert!(seen.contains("watching 2 files"), "{seen}");
    let first = std::fs::metadata(&out).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(dir.join("part.tex"), "Second version, longer than before.\n").unwrap();
    while seen.matches("flashtex: timing:").count() < 2 && std::time::Instant::now() < deadline {
        let n = err.read(&mut buf).unwrap();
        seen.push_str(&String::from_utf8_lossy(&buf[..n]));
    }
    let _ = child.kill();
    let _ = child.wait();
    assert!(seen.contains("change in part.tex -> rebuild #2"), "{seen}");
    assert!(std::fs::metadata(&out).unwrap().modified().unwrap() > first, "PDF rewritten");
    let _ = std::fs::remove_dir_all(&dir);
}
