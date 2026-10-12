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
/// a running one the same way: runs in the host's own process (they stop
/// at their next poll, `crate::resident`) and runs in processes of their
/// own (killed).
#[test]
fn cancels_are_never_lost() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    for inproc in ["1", "0"] {
        cancels_are_never_lost_in(inproc);
    }
}

fn cancels_are_never_lost_in(inproc: &str) {
    let dir = std::env::temp_dir().join(format!(
        "flashtex-host-cancel-{}-{inproc}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let (proj, out) = project(&dir, LOOP);
    let h = start_with(
        &dir,
        &[("FLASHTEX_UNICODE_INPROCESS", inproc)],
        &dir.join("host.err"),
    );
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
/// watches the host's lifeline (`host/proc.rs`) and kills its group. (Runs
/// in processes of their own: an engine in the host's own process ends
/// with it.)
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
        let h = start_with(
            &dir,
            &[("FLASHTEX_UNICODE_INPROCESS", "0")],
            &dir.join("host.err"),
        );
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
                ("FLASHTEX_UNICODE_INPROCESS", "0"),
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
    let h = start_with(
        &dir,
        &[
            ("FLASHTEX_UNICODE_INPROCESS", "0"),
            ("FLASHTEX_UNICODE_SPARE_DEBUG", "1"),
        ],
        &err,
    );
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

/// Compiles in the host's own process (`crate::resident`; from the second
/// run on, from the state kept where the format is loaded, and an edit after
/// the preamble from S₀) write what runs in processes of their own write,
/// byte for byte: the PDF, the log and the `.aux`, over two passes, an edit
/// in the body, a preamble edit, an error, a fatal error (the engine
/// recovers from it) and an edit back; the same in Low Memory, which keeps
/// no engine between compiles, without S₀ (`FLASHTEX_UNICODE_S0=0`) and
/// without the kept state (`FLASHTEX_UNICODE_FORMAT_POINT=0`).
#[test]
fn compiles_in_the_host_process_write_what_runs_of_their_own_write() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-inproc-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let texts = [
        DOC.to_string(),
        DOC.replace("See section", "Look at section"),
        DOC.replace("See section", "Look at section").replace(
            "\\begin{document}",
            "\\newcommand\\later{x}\n\\begin{document}",
        ),
        DOC.replace("See section", "\\undefinedmacro See"),
        DOC.replace("See section", "\\input{no-such-file} See"),
        DOC.to_string(),
    ];
    type Outs = Vec<(String, Vec<u8>, String, String, String)>;
    let run = |envs: &[(&str, &str)], profile: Option<&str>| -> Outs {
        // the same paths every time: the log names them
        let sub = dir.join("run");
        let _ = std::fs::remove_dir_all(&sub);
        let (proj, out) = project(&sub, DOC);
        let mut e = vec![("FLASHTEX_PIN_CLOCK", "1760000000.5")];
        e.extend_from_slice(envs);
        let h = start_with(&sub, &e, &sub.join("host.err"));
        let mut c = Client::connect_with(&h.sock, &[], profile).unwrap();
        let mut outs = vec![];
        for (i, text) in texts.iter().enumerate() {
            let id = i as i64 + 1;
            let mut r = request(id, &proj, &out);
            r.buffers = vec![("main.tex".into(), text.clone())];
            c.compile(&r).unwrap();
            let (_, done) = events(&mut c, id);
            let o = Path::new(&out);
            outs.push((
                done.str_field("status").unwrap_or("").to_string(),
                std::fs::read(o.join("main.pdf")).unwrap_or_default(),
                std::fs::read_to_string(o.join("main.log")).unwrap(),
                std::fs::read_to_string(o.join("main.aux")).unwrap_or_default(),
                // where the compile's first run started
                done.get("starts")
                    .and_then(Json::as_array)
                    .and_then(|a| a.first())
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
            ));
        }
        c.bye().unwrap();
        drop(h);
        outs
    };
    let own = run(&[("FLASHTEX_UNICODE_INPROCESS", "0")], None);
    let statuses: Vec<&str> = own.iter().map(|o| o.0.as_str()).collect();
    assert_eq!(statuses, ["ok", "ok", "ok", "error", "error", "ok"]);
    let no_point = [("FLASHTEX_UNICODE_FORMAT_POINT", "0")];
    let no_s0 = [("FLASHTEX_UNICODE_S0", "0")];
    for (what, envs, profile) in [
        ("in process", &[][..], None),
        ("in process, Low Memory", &[][..], Some("low-memory")),
        ("in process, no S0", &no_s0[..], None),
        ("in process, no kept state", &no_point[..], None),
    ] {
        let got = run(envs, profile);
        for (i, (a, b)) in got.iter().zip(&own).enumerate() {
            assert_eq!(a.0, b.0, "{what}: compile {}: the status", i + 1);
            assert!(a.1 == b.1, "{what}: compile {}: the PDFs differ", i + 1);
            assert_eq!(a.2, b.2, "{what}: compile {}: the logs differ", i + 1);
            assert_eq!(a.3, b.3, "{what}: compile {}: the .aux differs", i + 1);
        }
        let starts: Vec<&str> = got.iter().map(|o| o.4.as_str()).collect();
        if what == "in process" {
            // the body edits go on from S₀; the preamble edits (one, then
            // its removal) run from the format and take a new S₀; the fatal
            // error's second pass read the `.aux` its first pass left, and
            // its S₀ stands for the edit back
            assert_eq!(
                starts,
                ["fresh", "s0", "format", "format", "s0", "s0"],
                "{what}"
            );
        } else if what == "in process, no S0" {
            assert_eq!(
                starts,
                ["fresh", "format", "format", "format", "format", "format"],
                "{what}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Page checkpoints (`crate::ckpt::PageCk`): an edit on a document's last
/// page restarts after the pages before it, which are sent again from the
/// checkpoint (`DONE.starts` `page`); an edit on its first page goes on
/// from S₀; the output files, display list and all, are byte for byte a
/// run of its own's.
#[test]
fn an_edit_late_in_a_long_document_restarts_near_it() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-pages-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let para = "Words and more words that fill the page, again and again, until the \
                paragraph is long enough to break into several lines of text. ";
    let mut body = String::new();
    for i in 1..=6 {
        body.push_str(&format!("\\section{{Part {i}}}\\label{{p{i}}}\n"));
        for _ in 0..8 {
            body.push_str(&para.repeat(3));
            body.push_str("\n\n");
        }
    }
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\begin{{document}}\n{body}See~\\ref{{p2}}.\n\\end{{document}}\n"
    );
    let texts = [
        doc.clone(),
        doc.replace("See~", "Look at~"),
        doc.replacen("Words and more", "Wards and more", 1),
        doc.clone(),
    ];
    type Outs = Vec<(String, Vec<u8>, String, String, Vec<Vec<u8>>)>;
    let run = |envs: &[(&str, &str)]| -> Outs {
        let sub = dir.join("run");
        let _ = std::fs::remove_dir_all(&sub);
        let (proj, out) = project(&sub, &doc);
        let mut e = vec![("FLASHTEX_PIN_CLOCK", "1760000000.5")];
        e.extend_from_slice(envs);
        let h = start_with(&sub, &e, &sub.join("host.err"));
        let mut c = Client::connect(&h.sock).unwrap();
        let mut outs = vec![];
        for (i, text) in texts.iter().enumerate() {
            let id = i as i64 + 1;
            let mut r = request(id, &proj, &out);
            r.buffers = vec![("main.tex".into(), text.clone())];
            c.compile(&r).unwrap();
            let (ev, done) = events(&mut c, id);
            let pages: Vec<Vec<u8>> = ev
                .iter()
                .filter_map(|e| match e {
                    Event::Page(p) => Some(p.encode()),
                    _ => None,
                })
                .collect();
            let starts = done
                .get("starts")
                .and_then(Json::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let o = Path::new(&out);
            outs.push((
                starts,
                std::fs::read(o.join("main.pdf")).unwrap_or_default(),
                std::fs::read_to_string(o.join("main.log")).unwrap(),
                std::fs::read_to_string(o.join("main.aux")).unwrap_or_default(),
                pages,
            ));
        }
        c.bye().unwrap();
        drop(h);
        outs
    };
    let own = run(&[("FLASHTEX_UNICODE_INPROCESS", "0")]);
    let got = run(&[]);
    let starts: Vec<&str> = got.iter().map(|o| o.0.as_str()).collect();
    assert!(starts[1].starts_with("page"), "{starts:?}");
    assert!(starts[2].starts_with("s0"), "{starts:?}");
    for (i, (a, b)) in got.iter().zip(&own).enumerate() {
        assert!(a.1 == b.1, "compile {}: the PDFs differ", i + 1);
        assert_eq!(a.2, b.2, "compile {}: the logs differ", i + 1);
        assert_eq!(a.3, b.3, "compile {}: the .aux differs", i + 1);
        assert!(a.4.len() > 4, "compile {}: {} pages", i + 1, a.4.len());
        assert!(a.4 == b.4, "compile {}: the pages differ", i + 1);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A compile cancelled while it runs from S₀ or a page leaves the output
/// files as the last complete run left them (`resident::Resident::guard`):
/// the next compile still goes on from S₀ or a page (its `.aux` was cut
/// back by the cancelled run, and S₀'s key reads it), and writes what a run
/// of its own writes.
#[test]
fn a_cancelled_compile_leaves_the_next_one_incremental() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-cancelled-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let para = "Words and more words that fill the page, again and again, until the \
                paragraph is long enough to break into several lines of text. ";
    let mut body = String::new();
    for i in 1..=40 {
        body.push_str(&format!("\\section{{Part {i}}}\\label{{p{i}}}\n"));
        for _ in 0..4 {
            body.push_str(&para.repeat(3));
            body.push_str("\n\n");
        }
    }
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\begin{{document}}\n{body}See~\\ref{{p2}}.\n\\end{{document}}\n"
    );
    let first_edit = doc.replacen("Words and more", "Wards and more", 1);
    let second_edit = doc.replacen("Words and more", "Wordz and more", 1);
    let sub = dir.join("run");
    let (proj, out) = project(&sub, &doc);
    let h = start_with(
        &sub,
        &[("FLASHTEX_PIN_CLOCK", "1760000000.5")],
        &sub.join("host.err"),
    );
    let mut c = Client::connect(&h.sock).unwrap();
    let compile = |c: &mut Client, id: i64, text: &str, cancel_after: Option<u64>| -> Json {
        let mut r = request(id, &proj, &out);
        r.buffers = vec![("main.tex".into(), text.to_string())];
        c.compile(&r).unwrap();
        if let Some(ms) = cancel_after {
            std::thread::sleep(Duration::from_millis(ms));
            c.cancel(id).unwrap();
        }
        events(c, id).1
    };
    compile(&mut c, 1, &doc, None);
    compile(&mut c, 2, &doc, None);
    let cancelled = compile(&mut c, 3, &first_edit, Some(40));
    let done = compile(&mut c, 4, &second_edit, None);
    let start = done
        .get("starts")
        .and_then(Json::as_array)
        .and_then(|a| a.first())
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    assert!(
        start == "s0" || start == "page",
        "after {cancelled:?}: {done:?}"
    );
    let o = Path::new(&out);
    let got = (
        std::fs::read(o.join("main.pdf")).unwrap(),
        std::fs::read_to_string(o.join("main.aux")).unwrap(),
    );
    c.bye().unwrap();
    drop(h);
    // a run of its own on the same text
    let h = start_with(
        &sub,
        &[
            ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
            ("FLASHTEX_UNICODE_INPROCESS", "0"),
        ],
        &sub.join("host2.err"),
    );
    let mut c = Client::connect(&h.sock).unwrap();
    compile(&mut c, 1, &second_edit, None);
    let want = (
        std::fs::read(o.join("main.pdf")).unwrap(),
        std::fs::read_to_string(o.join("main.aux")).unwrap(),
    );
    c.bye().unwrap();
    drop(h);
    assert!(got.0 == want.0, "the PDFs differ");
    assert_eq!(got.1, want.1);
    let _ = std::fs::remove_dir_all(&dir);
}

/// An edit saved while a compile runs, and that compile cancelled: the next
/// compile sees the edit (`resident`'s `texts` are the files as they were
/// before a run read them, and a cancelled run puts back the last complete
/// run's). The edit is early in the document, before the page checkpoint
/// the cancelled run restarted from; the save comes while that run pauses
/// between looking for the edit and restoring its checkpoint
/// (`FLASHTEX_UNICODE_TEST_START_PAUSE_MS`), at several points of the pause.
#[test]
fn an_edit_saved_while_a_cancelled_compile_runs_is_seen() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-saved-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let para = "Words and more words that fill the page, again and again, until the \
                paragraph is long enough to break into several lines of text. ";
    let mut body = String::new();
    for i in 1..=40 {
        body.push_str(&format!("\\section{{Part {i}}}\\label{{p{i}}}\n"));
        for _ in 0..4 {
            body.push_str(&para.repeat(3));
            body.push_str("\n\n");
        }
    }
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\begin{{document}}\n{body}See~\\ref{{p2}}.\n\\end{{document}}\n"
    );
    // the cancelled compile's edit, late; the one saved while it runs, early
    let late = doc.replace("See~\\ref{p2}.", "See~\\ref{p2} too.");
    let saved = late.replacen("Words and more", "Wordz and more, early,", 1);
    let sub = dir.join("run");
    let (proj, out) = project(&sub, &doc);
    let h = start_with(
        &sub,
        &[
            ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
            ("FLASHTEX_UNICODE_TEST_START_PAUSE_MS", "60"),
        ],
        &sub.join("host.err"),
    );
    let mut c = Client::connect(&h.sock).unwrap();
    let compile = |c: &mut Client, id: i64, text: &str, save_then_cancel: Option<u64>| -> Json {
        let mut r = request(id, &proj, &out);
        r.buffers = vec![("main.tex".into(), text.to_string())];
        c.compile(&r).unwrap();
        if let Some(ms) = save_then_cancel {
            std::thread::sleep(Duration::from_millis(ms));
            std::fs::write(proj.join("main.tex"), &saved).unwrap();
            std::thread::sleep(Duration::from_millis(60));
            c.cancel(id).unwrap();
        }
        events(c, id).1
    };
    let o = Path::new(&out);
    let mut got = vec![];
    let mut id = 1;
    for ms in [20u64, 40] {
        compile(&mut c, id, &doc, None);
        compile(&mut c, id + 1, &doc, None);
        compile(&mut c, id + 2, &late, Some(ms));
        let done = compile(&mut c, id + 3, &saved, None);
        got.push((
            ms,
            done,
            std::fs::read(o.join("main.pdf")).unwrap(),
            std::fs::read_to_string(o.join("main.aux")).unwrap(),
        ));
        id += 4;
    }
    c.bye().unwrap();
    drop(h);
    // a run of its own on the saved text
    let h = start_with(
        &sub,
        &[
            ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
            ("FLASHTEX_UNICODE_INPROCESS", "0"),
        ],
        &sub.join("host2.err"),
    );
    let mut c = Client::connect(&h.sock).unwrap();
    compile(&mut c, 1, &saved, None);
    let want_pdf = std::fs::read(o.join("main.pdf")).unwrap();
    let want_aux = std::fs::read_to_string(o.join("main.aux")).unwrap();
    c.bye().unwrap();
    drop(h);
    for (ms, done, pdf, aux) in got {
        assert!(
            pdf == want_pdf,
            "saved after {ms} ms: the PDFs differ ({done:?})"
        );
        assert_eq!(aux, want_aux, "saved after {ms} ms");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Convergence, measurement only (`crate::conv`, step 1): an edit early in
/// a long document; the restarted run finds a page after which its state
/// is the last complete run's, and every page after it is the old run's
/// (the measurement checks its claim against the run's own pages).
#[test]
fn the_convergence_measurement_finds_a_point_that_holds() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-conv-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let para = "Words and more words that fill the page, again and again, until the \
                paragraph is long enough to break into several lines of text. ";
    let mut body = String::new();
    for i in 1..=30 {
        body.push_str(&format!("\\section{{Part {i}}}\n"));
        for _ in 0..4 {
            body.push_str(&para.repeat(3));
            body.push_str("\n\n");
        }
    }
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\begin{{document}}\n{body}\\end{{document}}\n"
    );
    let edited = doc.replacen("Words and more", "Wards and more", 1);
    let sub = dir.join("run");
    let (proj, out) = project(&sub, &doc);
    let err = sub.join("host.err");
    let h = start_with(
        &sub,
        &[
            ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
            ("FLASHTEX_UNICODE_CONV_DEBUG", "1"),
        ],
        &err,
    );
    let mut c = Client::connect(&h.sock).unwrap();
    for (id, text) in [(1, &doc), (2, &doc), (3, &edited)] {
        let mut r = request(id, &proj, &out);
        r.buffers = vec![("main.tex".into(), text.to_string())];
        c.compile(&r).unwrap();
        events(&mut c, id);
    }
    c.bye().unwrap();
    drop(h);
    let log = std::fs::read_to_string(&err).unwrap();
    let runs: Vec<&str> = log
        .lines()
        .filter(|l| l.starts_with("[conv] run:"))
        .collect();
    assert!(!log.contains("UNSOUND"), "{log}");
    assert!(
        runs.iter().any(|l| l.contains("holds")),
        "no claim that holds: {runs:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
