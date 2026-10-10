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

/// A CANCEL that names a compile already finished (or one never sent)
/// is dropped: it does not cancel a later compile that reuses the id.
#[test]
fn a_late_cancel_does_not_cancel_a_later_compile() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-latecancel-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (proj, out) = project(
        &dir,
        "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n",
    );
    let h = start(&dir);
    let alarm = deadline(h.child.id(), 600);
    let mut c = Client::connect(&h.sock).unwrap();
    c.compile(&request(7, &proj, &out)).unwrap();
    let (_, done) = events(&mut c, 7);
    assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
    c.cancel(7).unwrap(); // after its DONE
    c.cancel(99).unwrap(); // never sent
    c.compile(&request(7, &proj, &out)).unwrap();
    let (_, done) = events(&mut c, 7);
    assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
    drop(alarm);
    c.bye().unwrap();
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
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

/// `[fonts]` (PROPOSAL.md §4.5): the manifest's text role replaces the
/// class's font, an `info` line says so (a name TeX cannot take is a
/// warning), and the document's own `\setmainfont` still wins.
#[test]
fn the_manifest_fonts_apply_and_the_document_wins() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-fonts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let proj = dir.join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::write(proj.join(".git"), "").unwrap();
    std::fs::write(
        proj.join("flashtex.toml"),
        "[fonts]\ntext = \"texgyrepagella-regular.otf\"\nsans = \"bad}name\"\n",
    )
    .unwrap();
    let h = start(&dir);
    let mut c = Client::connect(&h.sock).unwrap();
    let out = dir.join("out");
    type Notes = Vec<(String, String)>;
    let mut fonts_of = |id: i64, doc: &str| -> (Vec<String>, Notes) {
        std::fs::write(proj.join("main.tex"), doc).unwrap();
        let mut r = CompileRequest::new(id, proj.to_str().unwrap(), "main.tex");
        r.output_dir = Some(out.to_string_lossy().into_owned());
        c.compile(&r).unwrap();
        let (ev, done) = events(&mut c, id);
        assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
        let mut fonts = vec![];
        let mut notes = vec![];
        for e in ev {
            match e {
                Event::Font(f) => fonts.push(f.info.str_field("ps_name").unwrap_or("").to_string()),
                Event::Diagnostic(j) if j.str_field("file") == Some("flashtex.toml") => {
                    notes.push((
                        j.str_field("severity").unwrap_or("").to_string(),
                        j.str_field("message").unwrap_or("").to_string(),
                    ))
                }
                _ => {}
            }
        }
        (fonts, notes)
    };
    let (fonts, notes) = fonts_of(
        1,
        "\\documentclass{article}\n\\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        fonts.iter().any(|p| p == "TeXGyrePagella-Regular"),
        "{fonts:?}"
    );
    let applied = "FlashTeX [fonts]: \\setmainfont{texgyrepagella-regular.otf}";
    assert!(
        notes.contains(&("info".into(), applied.into())),
        "{notes:?}"
    );
    assert!(
        notes
            .iter()
            .any(|(s, m)| s == "warning" && m.contains("fonts.sans")),
        "{notes:?}"
    );
    let log = std::fs::read_to_string(out.join("main.log")).unwrap();
    assert!(log.contains(applied), "the log says it");
    let (fonts, _) = fonts_of(
        2,
        "\\documentclass{article}\n\\usepackage{fontspec}\n\\setmainfont{texgyretermes-regular.otf}\n\\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        fonts.iter().any(|p| p == "TeXGyreTermes-Regular"),
        "{fonts:?}"
    );
    assert!(
        !fonts.iter().any(|p| p == "TeXGyrePagella-Regular"),
        "{fonts:?}"
    );
    c.bye().unwrap();
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A host with `envs` set, its standard error in `err`.
fn start_with(dir: &Path, envs: &[(&str, &str)], err: &Path) -> Host {
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
        .env("FORCE_SOURCE_DATE", "1")
        .envs(envs.iter().copied())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(err).unwrap())
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

/// Hot spares (`host/spare.rs`): each run after the first takes the run
/// started ahead for it and writes what a run started then writes (the PDF
/// and the log, byte for byte, against a host without spares); a main file
/// whose first line becomes a `%&` line ends the spare instead, and a fresh
/// run does the compile.
#[test]
fn spares_stand_for_runs_only_while_what_they_read_is_unchanged() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-spares-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // the PDF and the log after each of three compiles, and what the host said
    let run = |spares: &str| -> (Vec<(Vec<u8>, String)>, String) {
        // the same paths both times: the log names them
        let sub = dir.join("run");
        let _ = std::fs::remove_dir_all(&sub);
        let (proj, out) = project(&sub, DOC);
        let err = sub.join("host.err");
        let h = start_with(
            &sub,
            &[
                ("FLASHTEX_UNICODE_SPARES", spares),
                ("FLASHTEX_UNICODE_SPARE_DEBUG", "1"),
            ],
            &err,
        );
        let mut c = Client::connect(&h.sock).unwrap();
        let mut outs = vec![];
        for (id, text) in [
            (1, DOC.to_string()),
            (2, DOC.replace("See section", "Look at section")),
            (3, format!("%&xelatex\n{DOC}")),
        ] {
            let mut r = request(id, &proj, &out);
            r.buffers = vec![("main.tex".into(), text)];
            c.compile(&r).unwrap();
            let (_, done) = events(&mut c, id);
            assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
            let o = Path::new(&out);
            outs.push((
                std::fs::read(o.join("main.pdf")).unwrap(),
                std::fs::read_to_string(o.join("main.log")).unwrap(),
            ));
        }
        c.bye().unwrap();
        drop(h);
        let said = std::fs::read_to_string(&err).unwrap();
        (outs, said)
    };
    let (with, said) = run("1");
    let (without, said_without) = run("0");
    let fates: Vec<&str> = said
        .lines()
        .filter_map(|l| l.split_once(": spare "))
        .filter_map(|(_, f)| f.split_once(": ").map(|(_, why)| why))
        .collect();
    // compile 1's second pass and compile 2 take theirs; compile 3's main
    // file has a `%&` first line, so it runs afresh (one pass: the `.aux` is
    // unchanged)
    assert_eq!(
        fates,
        [
            "taken",
            "taken",
            "the format or the main file's first line changed",
        ],
        "{said}"
    );
    assert!(!said_without.contains(": spare "), "{said_without}");
    for (i, (a, b)) in with.iter().zip(&without).enumerate() {
        assert!(a.0 == b.0, "compile {}: the PDFs differ", i + 1);
        assert!(a.1 == b.1, "compile {}: the logs differ", i + 1);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Spares follow the performance mode (protocol §6.9): none in Low Memory
/// (a spare holds about 200 MB), kept in Balanced, and a switch to Low
/// Memory ends the one there is.
#[test]
fn low_memory_keeps_no_spare() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-lowmem-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (proj, out) = project(&dir, DOC);
    let err = dir.join("host.err");
    let h = start_with(&dir, &[("FLASHTEX_UNICODE_SPARE_DEBUG", "1")], &err);
    let mut c = Client::connect_with(&h.sock, &[], Some("low-memory")).unwrap();
    let knobs = c.hello.get("profile").cloned().unwrap();
    assert_eq!(knobs.str_field("mode"), Some("low-memory"));
    assert_eq!(knobs.get("spare").and_then(Json::as_bool), Some(false));
    let compile = |c: &mut Client, id: i64| {
        c.compile(&request(id, &proj, &out)).unwrap();
        let (_, done) = events(c, id);
        assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
    };
    compile(&mut c, 1);
    compile(&mut c, 2);
    let said = |e: &Path| std::fs::read_to_string(e).unwrap();
    assert!(!said(&err).contains(": spare "), "{}", said(&err));
    assert!(children_of(h.child.id()).is_empty(), "no spare waits");

    let switch = |c: &mut Client, mode: &str| -> Json {
        c.set_profile(mode).unwrap();
        loop {
            if let Event::Profile(j) = c.next_event().unwrap().expect("the host closed") {
                return j.get("profile").cloned().unwrap();
            }
        }
    };
    let knobs = switch(&mut c, "balanced");
    assert_eq!(knobs.get("spare").and_then(Json::as_bool), Some(true));
    compile(&mut c, 3);
    compile(&mut c, 4);
    assert!(said(&err).contains(": taken"), "{}", said(&err));
    assert_eq!(children_of(h.child.id()).len(), 1, "one spare waits");

    let knobs = switch(&mut c, "low-memory");
    assert_eq!(knobs.get("spare").and_then(Json::as_bool), Some(false));
    let t = Instant::now();
    while !children_of(h.child.id()).is_empty() {
        assert!(
            t.elapsed() < Duration::from_secs(10),
            "the spare outlived the switch to Low Memory"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    c.bye().unwrap();
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}
