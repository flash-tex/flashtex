//! `diag-v1` (lane P5-DIAGNOSTICS; docs/protocol/display-list-v3.md §6.7)
//! on the installed TeX Live's LaTeX. Skips where there is no TeX Live.
//!
//! * `corpus_positions_equal_pdflatex`: every case of
//!   `tests/diagnostics/cases/` compiled by `flashtex-host iserve` as the
//!   socket host compiles it; each report's file, line and column must be
//!   the ones pdflatex's own error context shows (`expected.json`, made from
//!   pdflatex's log by `tools/diag-oracle/oracle.py`, never by hand): the
//!   `l.<n>` line and the split of the context display, a warning's column
//!   from the oracle's probe run, a box report's line range.
//! * `incremental_diagnostics_equal_scratch`: the diagnostics of every
//!   compile of an editing session (edits before, inside and after the
//!   pages with errors; a fix, a new error, a line insertion, reverts; a
//!   persisted S₀ opened in a new host) equal those of a fresh host's
//!   compile of the same files: the notes of pages an incremental compile
//!   kept are re-emitted as a run from scratch has them.
//! * `socket_negotiates_diag_v1`: `HELLO.accept` through the socket host.
#![cfg(feature = "kpathsea")]

use flashtex_display_list::json::Json;
use flashtex_engine::resolver::find_texlive_bin;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Env {
    dir: PathBuf,
    fmt: PathBuf,
    pool: PathBuf,
}

/// A pdflatex format made by this engine, once per test binary.
fn env() -> Option<Env> {
    static MADE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    find_texlive_bin()?;
    let _once = MADE.lock().unwrap_or_else(|p| p.into_inner());
    let initex = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let base = std::env::temp_dir().join(format!("flashtex-diag-{}", std::process::id()));
    let fmt = base.join("fmt");
    std::fs::create_dir_all(&fmt).unwrap();
    let pdftex = fmt.join("pdftex");
    let _ = std::os::unix::fs::symlink(initex, &pdftex);
    if !fmt.join("pdflatex.fmt").exists() {
        let st = Command::new(&pdftex)
            .args([
                "-ini",
                "-jobname=pdflatex",
                "-progname=pdflatex",
                "-translate-file=cp227.tcx",
                "*pdflatex.ini",
            ])
            .current_dir(&fmt)
            .env("FLASHTEX_POOL", &pool)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(st.success() && fmt.join("pdflatex.fmt").exists());
    }
    Some(Env {
        dir: base,
        fmt,
        pool,
    })
}

struct Host {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Host {
    /// `flashtex-host iserve` in `dir` on `main`, as the socket host runs
    /// the engine (nonstopmode, file:line:error, a display list).
    fn start(e: &Env, dir: &Path, main: &str) -> Host {
        let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-host"));
        c.arg("iserve")
            .arg("--")
            .args([
                "-fmt=pdflatex",
                "-interaction=nonstopmode",
                "-file-line-error",
                main,
            ])
            .current_dir(dir)
            .env("FLASHTEX_POOL", &e.pool)
            .env("FLASHTEX_FORMATS", &e.fmt)
            .env("SOURCE_DATE_EPOCH", "1700000000")
            .env("FORCE_SOURCE_DATE", "1")
            .env("FLASHTEX_PIN_CLOCK", "1700000000.123456")
            .env("FLASHTEX_DISPLAY_LIST", "/dev/null")
            .env_remove("FLASHTEX_NO_DIAGNOSTICS");
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Host {
            child,
            stdin,
            stdout,
        }
    }

    fn cmd(&mut self, c: &str) -> String {
        writeln!(self.stdin, "{c}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert!(!line.starts_with("{\"error\""), "{c}: {line}");
        line
    }

    /// The last compile's `DIAG`s.
    fn diagnostics(&mut self) -> Vec<Json> {
        let l = self.cmd("diagnostics");
        let j = Json::parse(l.trim()).unwrap();
        j.get("diagnostics")
            .and_then(Json::as_array)
            .unwrap()
            .to_vec()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.child.wait();
    }
}

fn kind_of(d: &Json) -> &'static str {
    let code = d.str_field("code").unwrap_or("");
    let sev = d.str_field("severity").unwrap_or("");
    if code == "tex/show" {
        "show"
    } else if code.starts_with("tex/") && (code.ends_with("-hbox") || code.ends_with("-vbox")) {
        "box"
    } else if d.str_field("origin") == Some("pdftex") && sev == "warning" {
        "pdfwarning"
    } else if sev == "error" {
        "error"
    } else if sev == "warning" {
        "warning"
    } else {
        "other"
    }
}

fn pair(j: Option<&Json>) -> Option<(i64, i64)> {
    let a = j?.as_array()?;
    Some((a.first()?.as_i64()?, a.get(1)?.as_i64()?))
}

fn basename(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

/// Compare one case's `DIAG`s with pdflatex's reports: `Err` lists the
/// differences.
fn check_case(name: &str, expected: &[Json], got: &[Json]) -> Result<usize, Vec<String>> {
    let mut bad = vec![];
    let mut seen: std::collections::HashMap<&str, usize> = Default::default();
    let mut checked = 0;
    for it in expected {
        let k = it.str_field("kind").unwrap();
        let idx = *seen.entry(k).and_modify(|x| *x += 1).or_insert(1) - 1;
        let mine: Vec<&Json> = got.iter().filter(|d| kind_of(d) == k).collect();
        let msg = it.str_field("message").unwrap_or("");
        let Some(o) = mine.get(idx) else {
            bad.push(format!("{name}: {k} #{idx} missing: {msg}"));
            continue;
        };
        if k == "box" {
            if let Some(l) = pair(it.get("lines")) {
                checked += 1;
                if pair(o.get("lines")) != Some(l) {
                    bad.push(format!(
                        "{name}: {msg}: lines {:?}, pdflatex {l:?}",
                        pair(o.get("lines"))
                    ));
                }
            }
            continue;
        }
        let Some(line) = it.int_field("line") else {
            continue;
        };
        checked += 1;
        if let Some(f) = it.str_field("file") {
            let of = o.str_field("file").unwrap_or("");
            if basename(of) != basename(f) {
                bad.push(format!("{name}: {msg}: file {of}, pdflatex {f}"));
            }
        }
        let col = it.int_field("col");
        if o.int_field("line") != Some(line) || (col.is_some() && o.int_field("col") != col) {
            bad.push(format!(
                "{name}: {msg}: {:?}:{:?}, pdflatex {line}:{col:?}",
                o.int_field("line"),
                o.int_field("col")
            ));
        }
        if o.get("exact").and_then(Json::as_bool) != Some(true) {
            bad.push(format!("{name}: {msg}: not from the engine's record"));
        }
    }
    if bad.is_empty() {
        Ok(checked)
    } else {
        Err(bad)
    }
}

#[test]
fn corpus_positions_equal_pdflatex() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/diagnostics");
    let exp = Json::parse(&std::fs::read_to_string(here.join("expected.json")).unwrap()).unwrap();
    let cases: Vec<(String, Vec<Json>)> = match exp.get("cases") {
        Some(Json::Obj(kv)) => kv
            .iter()
            .map(|(k, v)| (k.clone(), v.as_array().unwrap().to_vec()))
            .collect(),
        _ => panic!("expected.json has no cases"),
    };
    assert!(cases.len() >= 60, "the corpus has {} cases", cases.len());
    let work = e.dir.join("corpus");
    let _ = std::fs::remove_dir_all(&work);
    let e = std::sync::Arc::new(e);
    let queue = std::sync::Arc::new(std::sync::Mutex::new(cases));
    let results = std::sync::Arc::new(std::sync::Mutex::new((0usize, vec![])));
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let (e, queue, results, here, work) = (
                e.clone(),
                queue.clone(),
                results.clone(),
                here.clone(),
                work.clone(),
            );
            std::thread::spawn(move || loop {
                let Some((name, items)) = queue.lock().unwrap().pop() else {
                    break;
                };
                let d = work.join(&name);
                std::fs::create_dir_all(&d).unwrap();
                let main = format!("{name}.tex");
                std::fs::copy(here.join("cases").join(&main), d.join(&main)).unwrap();
                let mut h = Host::start(&e, &d, &main);
                h.cmd("compile");
                let got = h.diagnostics();
                let r = check_case(&name, &items, &got);
                let mut res = results.lock().unwrap();
                match r {
                    Ok(n) => res.0 += n,
                    Err(b) => res.1.extend(b),
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let res = results.lock().unwrap();
    assert!(
        res.1.is_empty(),
        "{} differences:\n{}",
        res.1.len(),
        res.1.join("\n")
    );
    eprintln!("{} pdflatex positions matched", res.0);
    assert!(res.0 >= 100);
}

/// The DIAGs without what differs between two directories and two
/// processes: the directory in paths, span ids (names per process).
fn normalised(v: &[Json], dir: &Path) -> String {
    fn strip(j: &Json, dir: &str) -> Json {
        match j {
            Json::Obj(kv) => Json::Obj(
                kv.iter()
                    .filter(|(k, _)| k != "span")
                    .map(|(k, v)| (k.clone(), strip(v, dir)))
                    .collect(),
            ),
            Json::Arr(a) => Json::Arr(a.iter().map(|x| strip(x, dir)).collect()),
            Json::Str(s) => Json::Str(s.replace(dir, "<DIR>")),
            x => x.clone(),
        }
    }
    let d = std::fs::canonicalize(dir).unwrap();
    let ds = d.to_string_lossy().into_owned();
    v.iter()
        .map(|x| strip(&strip(x, &ds), &dir.to_string_lossy()).to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn copy_dir(from: &Path, to: &Path) {
    let _ = std::fs::remove_dir_all(to);
    std::fs::create_dir_all(to).unwrap();
    for f in std::fs::read_dir(from).unwrap() {
        let f = f.unwrap();
        if f.file_type().unwrap().is_file() {
            std::fs::copy(f.path(), to.join(f.file_name())).unwrap();
        }
    }
}

fn session_doc() -> String {
    let mut s = String::from(
        "\\documentclass{article}\n\
         \\newcommand{\\mycmd}[1]{\\textbf{#1}\\nothere}\n\
         \\begin{document}\n\
         \\section{One}\\label{one}\n",
    );
    for i in 0..60 {
        s.push_str(&format!(
            "Paragraph {i} with enough words to fill a few lines of the page so that \
             the document ships several pages; see \\ref{{one}} and math $a^{i}+b$.\n\n"
        ));
        match i {
            8 => s.push_str("Here is an error \\foo{} on an early page.\n\n"),
            20 => s.push_str("A macro call \\mycmd{x} with an error inside.\n\n"),
            30 => s.push_str("\\hbox to 20pt{An overfull box in the middle}\n\n"),
            40 => s.push_str("An undefined reference \\ref{nolabel} late.\n\n"),
            50 => s.push_str("Text with a math error x^2 near the end.\n\n"),
            _ => {}
        }
    }
    s.push_str("\\end{document}\n");
    s
}

#[test]
fn incremental_diagnostics_equal_scratch() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("session");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = session_doc();
    std::fs::write(dir.join("doc.tex"), &doc).unwrap();
    let mut h = Host::start(&e, &dir, "doc.tex");
    let mut compiles = 0;
    let mut check = |h: &mut Host, text: &str, what: &str| {
        std::fs::write(dir.join("doc.tex"), text).unwrap();
        let reference = dir.with_extension("ref");
        copy_dir(&dir, &reference);
        let report = h.cmd("compile");
        let got = h.diagnostics();
        let mut r = Host::start(&e, &reference, "doc.tex");
        r.cmd("compile");
        let want = r.diagnostics();
        assert!(!want.is_empty(), "{what}: the document has diagnostics");
        assert_eq!(
            normalised(&got, &dir),
            normalised(&want, &reference),
            "{what}: the incremental compile's diagnostics differ from a scratch compile's\n{report}"
        );
        compiles += 1;
        report
    };
    check(&mut h, &doc, "first compile");
    check(&mut h, &doc, "unchanged");
    let edits: Vec<(String, &str)> = vec![
        (
            doc.replacen("Paragraph 3 with", "Paragraph 3 wiht", 1),
            "an edit before every error",
        ),
        (doc.clone(), "its revert"),
        (
            doc.replacen("Paragraph 25 with", "Paragraph 25 wiht", 1),
            "an edit between errors",
        ),
        (doc.clone(), "its revert"),
        (
            doc.replacen("Paragraph 55 with", "Paragraph 55 wiht", 1),
            "an edit after every error",
        ),
        (doc.clone(), "its revert"),
        (
            doc.replacen("an error \\foo{} on", "an error \\textbf{} on", 1),
            "an error fixed",
        ),
        (doc.clone(), "the error back"),
        (
            doc.replacen("Paragraph 35 with", "Paragraph 35 \\bar{} with", 1),
            "a new error",
        ),
        (doc.clone(), "its revert"),
        (
            doc.replacen("Paragraph 12 with", "Paragraph 12\nwith", 1),
            "a line inserted",
        ),
        (doc.clone(), "its revert"),
        (
            doc.replacen("\\textbf{#1}\\nothere", "\\textit{#1}\\nothere", 1),
            "the macro's definition edited",
        ),
        (doc.clone(), "its revert"),
    ];
    for (text, what) in &edits {
        check(&mut h, text, what);
    }
    // A persisted S₀ opened by another host reports what a full run does.
    let s0 = e.dir.join("session.s0");
    h.cmd(&format!("save {}", s0.display()));
    drop(h);
    let mut o = Host::start(&e, &dir, "doc.tex");
    o.cmd(&format!("open {}", s0.display()));
    let got = o.diagnostics();
    let reference = dir.with_extension("ref");
    copy_dir(&dir, &reference);
    let mut r = Host::start(&e, &reference, "doc.tex");
    r.cmd("compile");
    assert_eq!(
        normalised(&got, &dir),
        normalised(&r.diagnostics(), &reference),
        "a reopened S0's diagnostics differ from a scratch compile's"
    );
    eprintln!("{compiles} compiles checked");
}

/// Through the socket: a client that accepts `diag-v1` gets `DIAG`s (with
/// the spans they name declared first) and no `DIAGNOSTIC`; one that does
/// not gets the 3.1 `DIAGNOSTIC`s only.
#[test]
fn socket_negotiates_diag_v1() {
    use flashtex_display_list::client::{Client, CompileRequest, Event};
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("socket");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("main.tex"),
        "\\documentclass{article}\n\\begin{document}\nHello \\foo{} world.\n\nSee \\ref{x}.\n\\end{document}\n",
    )
    .unwrap();
    let sock = PathBuf::from(format!("/tmp/ftdiag-{}.sock", std::process::id()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-host"))
        .args(["--socket", sock.to_str().unwrap(), "--no-warm"])
        .env("FLASHTEX_POOL", &e.pool)
        .env("FLASHTEX_FORMATS", &e.fmt)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("FLASHTEX_S0_CACHE")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut out = BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert!(out.read_line(&mut line).unwrap() > 0, "the host exited");
        if line.contains("listening") {
            break;
        }
    }
    std::thread::spawn(move || for _ in out.lines() {});
    let run = |accept: &[&str]| {
        let mut c = Client::connect_accepting(&sock, accept).unwrap();
        let caps = c.hello.get("capabilities").unwrap().to_string();
        assert!(caps.contains("\"diag-v1\""), "{caps}");
        let req = CompileRequest::new(1, dir.to_str().unwrap(), "main.tex");
        c.compile(&req).unwrap();
        let mut diags = vec![];
        let mut legacy = 0;
        let mut spans = std::collections::HashSet::new();
        loop {
            match c.next_event().unwrap().unwrap() {
                Event::Diag(d) => {
                    if let Some(s) = d.span {
                        assert!(spans.contains(&(s as u32)), "span {s} not declared first");
                    }
                    diags.push(d)
                }
                Event::Diagnostic(_) => legacy += 1,
                Event::Sources(s) => spans.extend(s.spans.iter().map(|x| x.0)),
                Event::Done(d) => {
                    let n = d.int_field("diagnostics").unwrap();
                    assert_eq!(n as usize, diags.len() + legacy);
                    break;
                }
                _ => {}
            }
        }
        (diags, legacy)
    };
    let (d, legacy) = run(&[flashtex_display_list::diag::CAPABILITY]);
    assert_eq!(legacy, 0);
    let codes: Vec<&str> = d.iter().map(|x| x.code.as_str()).collect();
    assert!(
        codes.contains(&"tex/undefined-control-sequence"),
        "{codes:?}"
    );
    assert!(codes.contains(&"latex/undefined-reference"), "{codes:?}");
    let err = d
        .iter()
        .find(|x| x.code == "tex/undefined-control-sequence")
        .unwrap();
    assert_eq!(
        (err.line, err.col, err.range),
        (Some(3), Some(10), Some((6, 10)))
    );
    assert!(err.file.as_deref().unwrap().ends_with("/main.tex"));
    assert!(err.span.is_some() && err.exact && !err.help.is_empty());
    let (d, legacy) = run(&[]);
    assert!(
        d.is_empty() && legacy >= 2,
        "{} DIAGs, {legacy} DIAGNOSTICs",
        d.len()
    );
    let _ = child.kill();
    let _ = child.wait();
    let _ = std::fs::remove_file(&sock);
}
