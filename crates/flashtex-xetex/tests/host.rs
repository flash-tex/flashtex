//! `flashtex-host-unicode` over its socket (spec §6), as the app and
//! `flashtex-v3` use it: a xelatex document with fontspec and a cross
//! reference compiles in two passes, its pages arrive as display lists
//! with OpenType fonts, `DONE` names FlashTeX's PDF; an export compile; an
//! edit applied before the next compile; cancels, never lost; no engine
//! outlives the host. Needs TeX Live (the format is FlashTeX's own, built
//! from it); skipped without.

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::json::Json;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn texlive() -> bool {
    flashtex_engine::resolver::discover_texlive().is_some()
}

struct Host {
    child: Child,
    sock: PathBuf,
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.sock);
    }
}

fn start(dir: &Path) -> Host {
    let sock = dir.join("h.sock");
    let child = Command::new(env!("CARGO_BIN_EXE_flashtex-host-unicode"))
        .args([
            "--socket",
            sock.to_str().unwrap(),
            "--once",
            "--accept-timeout",
            "600",
        ])
        .env("SOURCE_DATE_EPOCH", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let t = Instant::now();
    while !sock.exists() {
        assert!(
            t.elapsed() < Duration::from_secs(600),
            "the host did not listen"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    Host { child, sock }
}

/// The events of compile `id` up to its `DONE`.
fn events(c: &mut Client, id: i64) -> (Vec<Event>, Json) {
    let mut ev = vec![];
    loop {
        let e = c.next_event().unwrap().expect("the host closed");
        if let Event::Done(j) = &e {
            if j.int_field("id") == Some(id) {
                return (ev, j.clone());
            }
        }
        ev.push(e);
    }
}

const DOC: &str = r#"\documentclass{article}
\usepackage{fontspec}
\setmainfont{texgyrepagella-regular.otf}
\begin{document}
\section{One}\label{one}
See section~\ref{one} on page~\pageref{one}.
\end{document}
"#;

#[test]
fn the_unicode_host_compiles_a_document() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-unicode-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let proj = dir.join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(proj.join("main.tex"), DOC).unwrap();
    let h = start(&dir);
    let mut c = Client::connect(&h.sock).unwrap();
    assert_eq!(c.hello.str_field("protocol"), Some("display-list-v3"));
    let caps = c
        .hello
        .get("capabilities")
        .and_then(Json::as_array)
        .unwrap();
    assert!(caps.iter().any(|x| x.as_str() == Some("export")));

    // a compile: two passes (the reference), pages with OpenType fonts
    let out = dir.join("out");
    let mut r = CompileRequest::new(1, proj.to_str().unwrap(), "main.tex");
    r.output_dir = Some(out.to_string_lossy().into_owned());
    c.compile(&r).unwrap();
    let (ev, done) = events(&mut c, 1);
    assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
    assert_eq!(done.int_field("passes"), Some(2), "{done:?}");
    assert!(matches!(ev.first(), Some(Event::Started(_))));
    let fonts: Vec<_> = ev
        .iter()
        .filter_map(|e| match e {
            Event::Font(f) => Some(f.info.str_field("ps_name").unwrap_or("").to_string()),
            _ => None,
        })
        .collect();
    assert!(
        fonts.iter().any(|p| p == "TeXGyrePagella-Regular"),
        "{fonts:?}"
    );
    let pages = ev.iter().filter(|e| matches!(e, Event::Page(_))).count();
    assert_eq!(pages, 2, "one page a pass");
    let pdf = PathBuf::from(done.str_field("pdf").unwrap());
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF-"));
    // the second pass resolved the reference
    let log = std::fs::read_to_string(out.join("main.log")).unwrap();
    assert!(!log.contains("undefined references"), "{log}");

    // an edit before the next compile, which then fails with a diagnostic
    let mut r = CompileRequest::new(2, proj.to_str().unwrap(), "main.tex");
    r.output_dir = Some(out.to_string_lossy().into_owned());
    r.buffers = vec![(
        "main.tex".into(),
        DOC.replace("See section", "\\undefinedmacro See"),
    )];
    c.compile(&r).unwrap();
    let (ev, done) = events(&mut c, 2);
    assert_eq!(done.str_field("status"), Some("error"), "{done:?}");
    let diag = ev
        .iter()
        .find_map(|e| match e {
            Event::Diagnostic(j) if j.str_field("severity") == Some("error") => Some(j.clone()),
            _ => None,
        })
        .expect("an error diagnostic");
    assert_eq!(diag.str_field("file"), Some("main.tex"));
    assert_eq!(diag.int_field("line"), Some(6));
    assert!(std::fs::read_to_string(proj.join("main.tex"))
        .unwrap()
        .contains("\\undefinedmacro"));

    // an export compile
    std::fs::write(proj.join("main.tex"), DOC).unwrap();
    let mut r = CompileRequest::new(3, proj.to_str().unwrap(), "main.tex");
    r.output_dir = Some(out.to_string_lossy().into_owned());
    r.export = true;
    c.compile(&r).unwrap();
    let (ev, done) = events(&mut c, 3);
    assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
    assert_eq!(done.str_field("mode"), Some("export"));
    assert!(
        !ev.iter().any(|e| matches!(e, Event::Page(_))),
        "an export sends no pages"
    );

    c.bye().unwrap();
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A document that never ends (TeX loops until it is killed).
const LOOP: &str =
    "\\documentclass{article}\n\\begin{document}\nx\\loop\\iftrue\\repeat\n\\end{document}\n";

/// A project with `main.tex` = `text`, and its output directory.
fn project(dir: &Path, text: &str) -> (PathBuf, String) {
    let proj = dir.join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(proj.join("main.tex"), text).unwrap();
    (proj, dir.join("out").to_string_lossy().into_owned())
}

fn request(id: i64, proj: &Path, out: &str) -> CompileRequest {
    let mut r = CompileRequest::new(id, proj.to_str().unwrap(), "main.tex");
    r.output_dir = Some(out.to_string());
    r
}

/// Kills the host after `secs` unless the returned sender is dropped
/// first: a lost cancel then fails the test (the host closes) instead of
/// hanging it.
fn deadline(pid: u32, secs: u64) -> std::sync::mpsc::Sender<()> {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    std::thread::spawn(move || {
        if rx.recv_timeout(Duration::from_secs(secs))
            == Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        {
            eprintln!("deadline passed: killing the host");
            // SAFETY: kill(2) has no memory-safety preconditions.
            unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        }
    });
    tx
}

/// Every cancel ends its compile, whenever it comes (the review's race:
/// a cancel between the check and the engine's start found nothing to
/// kill, and the engine looped for ever), and a newer COMPILE supersedes
/// a running one the same way.
#[test]
fn cancels_are_never_lost() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-cancel-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (proj, out) = project(&dir, LOOP);
    let h = start(&dir);
    let alarm = deadline(h.child.id(), 600);
    let mut c = Client::connect(&h.sock).unwrap();
    for i in 0..60i64 {
        c.compile(&request(i + 1, &proj, &out)).unwrap();
        // from at once to well after the engine started
        std::thread::sleep(Duration::from_micros((i as u64 % 12) * 1500));
        c.cancel(i + 1).unwrap();
        let (_, done) = events(&mut c, i + 1);
        assert_eq!(
            done.str_field("status"),
            Some("cancelled"),
            "compile {}: {done:?}",
            i + 1
        );
    }
    // superseded: a looping compile, then a newer one that ends
    for i in 0..20i64 {
        let id = 100 + 2 * i;
        std::fs::write(proj.join("main.tex"), LOOP).unwrap();
        c.compile(&request(id, &proj, &out)).unwrap();
        std::thread::sleep(Duration::from_micros((i as u64 % 10) * 2000));
        std::fs::write(
            proj.join("main.tex"),
            "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n",
        )
        .unwrap();
        c.compile(&request(id + 1, &proj, &out)).unwrap();
        let (_, done) = events(&mut c, id);
        assert_eq!(done.str_field("status"), Some("cancelled"), "{done:?}");
        let (_, done) = events(&mut c, id + 1);
        assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
    }
    drop(alarm);
    c.bye().unwrap();
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The host's children (`pgrep -P`).
fn children_of(pid: u32) -> Vec<i32> {
    let o = Command::new("pgrep")
        .args(["-P", &pid.to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout)
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect()
}

fn alive(pid: i32) -> bool {
    // SAFETY: kill(2) with signal 0 only checks.
    unsafe { libc::kill(pid, 0) == 0 }
}

/// However the host ends, its engine does not outlive it: the engine
/// watches the host's lifeline (`host/proc.rs`) and kills its group.
#[test]
fn no_engine_outlives_the_host() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    for (n, sig) in [(1, libc::SIGTERM), (2, libc::SIGKILL)] {
        let dir =
            std::env::temp_dir().join(format!("flashtex-host-orphan-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (proj, out) = project(&dir, LOOP);
        let h = start(&dir);
        let mut c = Client::connect(&h.sock).unwrap();
        c.compile(&request(1, &proj, &out)).unwrap();
        let t = Instant::now();
        let kids = loop {
            let k = children_of(h.child.id());
            if !k.is_empty() {
                break k;
            }
            assert!(t.elapsed() < Duration::from_secs(60), "no engine started");
            std::thread::sleep(Duration::from_millis(20));
        };
        // the engine is in its loop
        std::thread::sleep(Duration::from_millis(300));
        // SAFETY: kill(2) has no memory-safety preconditions.
        unsafe { libc::kill(h.child.id() as i32, sig) };
        let t = Instant::now();
        while kids.iter().any(|&k| alive(k)) {
            assert!(
                t.elapsed() < Duration::from_secs(10),
                "signal {sig}: engine {kids:?} outlived the host"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        drop(c);
        drop(h);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
