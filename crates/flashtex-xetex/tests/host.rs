//! `flashtex-host-unicode` over its socket (spec §6), as the app and
//! the `flashtex` CLI use it: a xelatex document with fontspec and a cross
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

/// A font file whose path is 256 bytes or longer: XDV keeps the path's
/// length in one byte (as XeTeX's), so the writer takes the engine's own
/// path (`page::Metrics::native_path`); before, its glyphs were dropped
/// without a word (a bundle cache under a long home directory).
#[test]
fn a_font_at_a_long_path_is_drawn() {
    let src = Path::new("/System/Library/Fonts/Supplemental/Georgia.ttf");
    if !texlive() || !src.is_file() {
        eprintln!("skipped: no TeX Live or no Georgia.ttf");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-longpath-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut fonts = dir.join("proj");
    while fonts.as_os_str().len() < 270 {
        fonts = fonts.join("a-rather-long-directory-name");
    }
    std::fs::create_dir_all(&fonts).unwrap();
    std::fs::copy(src, fonts.join("Georgia.ttf")).unwrap();
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\
         \\setmainfont{{Georgia.ttf}}[Path={}/]\n\\begin{{document}}\nHello.\n\\end{{document}}\n",
        fonts.display()
    );
    let (proj, out) = project(&dir, &doc);
    let h = start(&dir);
    let mut c = Client::connect(&h.sock).unwrap();
    c.compile(&request(1, &proj, &out)).unwrap();
    let (ev, done) = events(&mut c, 1);
    assert_eq!(done.str_field("status"), Some("ok"), "{done:?}");
    let fonts: Vec<_> = ev
        .iter()
        .filter_map(|e| match e {
            Event::Font(f) => Some(f.info.str_field("ps_name").unwrap_or("").to_string()),
            _ => None,
        })
        .collect();
    assert!(fonts.iter().any(|p| p == "Georgia"), "{fonts:?}");
    c.bye().unwrap();
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

/// bibtex and makeindex are the engine's own ports (`host/proc.rs`,
/// `--run-tool flashtex:...`), so a bibliography and an index need no TeX
/// Live: their outputs equal TeX Live 2026's bibtex and makeindex on the
/// same document (fixtures/notex-tools, which `scripts/notex-host-check.sh`
/// checks for the pdfTeX host), and a follow-up compile reads them.
#[test]
fn bibtex_and_makeindex_are_built_in() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let fx = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/notex-tools");
    let dir = std::env::temp_dir().join(format!("flashtex-host-tools-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let proj = dir.join("proj");
    std::fs::create_dir_all(&proj).unwrap();
    for f in ["main.tex", "refs.bib"] {
        std::fs::copy(fx.join(f), proj.join(f)).unwrap();
    }
    let out = dir.join("out").to_string_lossy().into_owned();
    let h = start(&dir);
    let mut c = Client::connect(&h.sock).unwrap();
    let mut r = request(1, &proj, &out);
    r.external_tools = Some("auto".into());
    c.compile(&r).unwrap();
    // a DONE per compile (the first, then the follow-up the tools' outputs
    // ask for), and the rounds' end
    let mut ev = vec![];
    loop {
        let e = c.next_event().unwrap().expect("the host closed");
        let end = matches!(&e, Event::Tool(j) if j.str_field("event") == Some("settled"));
        if let Event::Done(j) = &e {
            assert_eq!(j.str_field("status"), Some("ok"), "{j:?}");
        }
        ev.push(e);
        if end {
            break;
        }
    }
    assert!(
        ev.iter().filter(|e| matches!(e, Event::Done(_))).count() >= 2,
        "no follow-up compile"
    );
    let ran: Vec<(String, String)> = ev
        .iter()
        .filter_map(|e| match e {
            Event::Tool(j) if j.str_field("event") == Some("done") => Some((
                j.str_field("tool").unwrap_or("").to_string(),
                j.str_field("status").unwrap_or("").to_string(),
            )),
            _ => None,
        })
        .collect();
    for tool in ["bibtex", "makeindex"] {
        assert!(
            ran.iter().any(|(t, s)| t == tool && s == "ok"),
            "{tool} did not run cleanly: {ran:?}"
        );
    }
    for name in ["main.bbl", "main.blg", "main.ind", "main.ilg"] {
        let got = std::fs::read(Path::new(&out).join(name)).unwrap();
        let want = std::fs::read(fx.join("expected").join(name)).unwrap();
        assert!(got == want, "{name} differs from TeX Live's");
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
/// recovers from it) and an edit back; the same in Low Memory, whose
/// checkpoints are thinned to a tight budget (here to nothing but the
/// pinned ones: `FLASHTEX_BUDGET=1`), without S₀ (`FLASHTEX_UNICODE_S0=0`)
/// and without the kept state (`FLASHTEX_UNICODE_FORMAT_POINT=0`).
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
    let tiny = [("FLASHTEX_BUDGET", "1")];
    for (what, envs, profile) in [
        ("in process", &[][..], None),
        ("in process, Low Memory", &tiny[..], Some("low-memory")),
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
        if what == "in process" || what == "in process, Low Memory" {
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

/// S₀ is taken at the real `\begin{document}`, not where a package expanded
/// `\document` into a new definition to patch it (bidi does, for
/// polyglossia's right-to-left languages): an edit in the preamble after
/// the patch runs from the format, a body edit goes on from S₀, and every
/// compile writes what a run of its own writes.
#[test]
fn s0_is_not_taken_where_document_is_patched() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-patch-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let doc = "\\documentclass{article}\n\\makeatletter\n\\expandafter\\def\\expandafter\\document\\expandafter{\\document\\relax}\n\\makeatother\n\\newcommand\\later{late}\n\\begin{document}\nA body, \\later.\n\\end{document}\n";
    let texts = [
        doc.to_string(),
        doc.replace("A body", "The body"),
        doc.replace("{late}", "{later}"),
    ];
    let run = |envs: &[(&str, &str)]| -> Vec<(Vec<u8>, String, String)> {
        let sub = dir.join("run");
        let _ = std::fs::remove_dir_all(&sub);
        let (proj, out) = project(&sub, doc);
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
            let (_, done) = events(&mut c, id);
            let o = Path::new(&out);
            outs.push((
                std::fs::read(o.join("main.pdf")).unwrap_or_default(),
                std::fs::read_to_string(o.join("main.log")).unwrap_or_default(),
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
    let own = run(&[("FLASHTEX_UNICODE_INPROCESS", "0")]);
    let got = run(&[]);
    for (i, (a, b)) in got.iter().zip(&own).enumerate() {
        assert!(a.0 == b.0, "compile {}: the PDFs differ", i + 1);
        assert_eq!(a.1, b.1, "compile {}: the logs differ", i + 1);
    }
    let starts: Vec<&str> = got.iter().map(|o| o.2.as_str()).collect();
    assert_eq!(starts, ["fresh", "s0", "format"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A persisted S₀ (`--s0-cache`, `crate::s0file`): a host saves the
/// document's S₀ after its compile; a new host whose document was edited
/// in its body while closed goes on from it (no format, no preamble), and
/// writes what a from-scratch compile of the same directory writes (the
/// PDF, the log and the `.aux`, byte for byte); a font file the preamble
/// loaded, changed while closed, refuses it (from the start), and the
/// output is again a from-scratch compile's.
#[test]
fn a_reopened_document_goes_on_from_its_persisted_s0() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let find = |n: &str| {
        flashtex_engine::system::find_file(n, flashtex_engine::resolver::Format::OpenType)
            .expect("Latin Modern (TeX Live)")
    };
    let dir = std::env::temp_dir().join(format!("flashtex-host-reopen-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let doc = "\\documentclass{article}\n\\usepackage{fontspec}\n\\font\\mine=\"[myfont.otf]\" at 11pt\n\\begin{document}\n\\section{One}\\label{one}\n{\\mine Our own font, ffi.} See section~\\ref{one}.\n\\end{document}\n";
    let (proj, out) = project(&dir, doc);
    std::fs::copy(find("lmroman10-regular.otf"), proj.join("myfont.otf")).unwrap();
    let cache = dir.join("cache");
    let cache_s = cache.to_string_lossy().into_owned();
    let with_cache = [
        ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
        ("FLASHTEX_S0_CACHE", cache_s.as_str()),
    ];
    let scratch = [
        ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
        ("FLASHTEX_UNICODE_INPROCESS", "0"),
    ];
    let outs = |o: &Path| -> Vec<Vec<u8>> {
        ["main.pdf", "main.log", "main.aux"]
            .iter()
            .map(|n| std::fs::read(o.join(n)).unwrap_or_default())
            .collect()
    };
    // one compile by a new host; the host ends (and its S₀ is written)
    let compile = |envs: &[(&str, &str)], tag: &str| -> (String, String, Vec<Vec<u8>>) {
        let h = start_with(&dir, envs, &dir.join(format!("{tag}.err")));
        let mut c = Client::connect(&h.sock).unwrap();
        c.compile(&request(1, &proj, &out)).unwrap();
        let (_, done) = events(&mut c, 1);
        c.bye().unwrap();
        let mut h = h;
        let t = Instant::now();
        while h.child.try_wait().unwrap().is_none() {
            assert!(
                t.elapsed() < Duration::from_secs(120),
                "the host did not end"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let start = done
            .get("starts")
            .and_then(Json::as_array)
            .and_then(|a| a.first())
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();
        (
            done.str_field("status").unwrap_or("").to_string(),
            start,
            outs(Path::new(&out)),
        )
    };
    // a compile by a host with the cache, and the same from scratch from
    // the same output directory
    let against_scratch = |what: &str| -> String {
        let o = Path::new(&out);
        let before: Vec<(PathBuf, Vec<u8>)> = std::fs::read_dir(o)
            .unwrap()
            .map(|e| e.unwrap().path())
            .map(|p| (p.clone(), std::fs::read(&p).unwrap()))
            .collect();
        let (status, start, got) = compile(&with_cache, what);
        let _ = std::fs::remove_dir_all(o);
        std::fs::create_dir_all(o).unwrap();
        for (p, b) in &before {
            std::fs::write(p, b).unwrap();
        }
        let (rstatus, _, want) = compile(&scratch, &format!("{what}-ref"));
        assert_eq!(status, rstatus, "{what}: the status");
        for (n, (a, b)) in ["PDF", "log", ".aux"].iter().zip(got.iter().zip(&want)) {
            assert!(
                a == b,
                "{what}: the {n} differs from a from-scratch compile's"
            );
        }
        start
    };
    let (status, _, _) = compile(&with_cache, "first");
    assert_eq!(status, "ok");
    let mut saved: Vec<_> = std::fs::read_dir(&cache)
        .expect("the S0 cache")
        .map(|e| e.unwrap().path())
        .collect();
    saved.sort_by_key(|p| p.extension().map(|e| e.to_os_string()));
    let size = |p: &Path| std::fs::metadata(p).unwrap().len();
    // S₀, and the base it is a delta against (the state after the format
    // load, made from the format point)
    assert!(
        saved.len() == 2
            && saved[0].extension().is_some_and(|e| e == "xs0")
            && saved[1].extension().is_some_and(|e| e == "xs0base"),
        "one S0 and its base saved: {saved:?}"
    );
    assert!(
        size(&saved[0]) * 4 < size(&saved[1]),
        "S0 holds what differs from its base: {} and {} bytes",
        size(&saved[0]),
        size(&saved[1])
    );
    // edited in its body while closed: from S₀
    std::fs::write(
        proj.join("main.tex"),
        doc.replace("See section", "Added while closed. See section"),
    )
    .unwrap();
    assert_eq!(against_scratch("body"), "s0", "a body edit goes on from S0");
    // its font file changed while closed: S₀ refused
    std::fs::copy(find("lmroman12-regular.otf"), proj.join("myfont.otf")).unwrap();
    assert_eq!(
        against_scratch("font"),
        "fresh",
        "a changed font file refuses S0"
    );
    // its base gone while closed: S₀ refused; that run makes the base and
    // S₀ again, and the next goes on from them
    std::fs::remove_file(&saved[1]).unwrap();
    assert_eq!(against_scratch("no base"), "fresh", "no base refuses S0");
    assert!(saved[1].is_file(), "the base is made again");
    assert_eq!(against_scratch("base again"), "s0", "the new base serves");
    // a base cut short while closed: S₀ refused; the run's save makes the
    // base again in its place (rather than saving S₀ whole), and collects
    // a base of no S₀ older than a day (not a recent one)
    let stale = cache.join("0000000000000000000000000000dead.xs0base");
    let recent = cache.join("0000000000000000000000000000beef.xs0base");
    for b in [&stale, &recent] {
        std::fs::write(b, b"an old base").unwrap();
    }
    std::fs::File::options()
        .write(true)
        .open(&stale)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - Duration::from_secs(2 * 24 * 3600))
        .unwrap();
    let whole = size(&saved[1]);
    std::fs::File::options()
        .write(true)
        .open(&saved[1])
        .unwrap()
        .set_len(whole / 2)
        .unwrap();
    assert_eq!(
        against_scratch("cut base"),
        "fresh",
        "a cut base refuses S0"
    );
    assert_eq!(size(&saved[1]), whole, "the base is made again");
    assert!(
        size(&saved[0]) * 4 < whole,
        "S0 is saved against the new base, not whole: {} bytes",
        size(&saved[0])
    );
    assert!(!stale.exists(), "the stale base is collected");
    assert!(recent.exists(), "a recent base is kept");
    assert_eq!(
        against_scratch("base made again"),
        "s0",
        "the base made again serves"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A persisted S₀ is saved when the engine is idle (`host::inproc`,
/// `Engine::save_pending`): a keystroke right after a cold compile runs
/// first. The file then written is the one a save made at once writes,
/// byte for byte (what it holds besides S₀'s words is taken when the save
/// is asked for; S₀'s words are a checkpoint's).
#[test]
fn an_s0_saved_after_a_keystroke_is_the_one_saved_at_once() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-s0later-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let doc = "\\documentclass{article}\n\\usepackage{fontspec}\n\\begin{document}\n\\section{One}\\label{one}\nSome words, ffi. See section~\\ref{one}.\n\n\\end{document}\n";
    let (proj, out) = project(&dir, doc);
    // the S₀ file in `cache` after one host's compiles of `texts` (each in
    // the editor's buffer, one right after the other)
    let saved = |cache: &Path, texts: &[&str], tag: &str| -> Vec<u8> {
        let _ = std::fs::remove_dir_all(&out);
        let cache_s = cache.to_string_lossy().into_owned();
        let h = start_with(
            &dir,
            &[
                ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
                ("FLASHTEX_S0_CACHE", cache_s.as_str()),
                // (no save before the second compile: it comes at once)
                ("FLASHTEX_UNICODE_SAVE_IDLE_MS", "3000"),
            ],
            &dir.join(format!("{tag}.err")),
        );
        let mut c = Client::connect(&h.sock).unwrap();
        for (i, t) in texts.iter().enumerate() {
            let id = i as i64 + 1;
            let mut r = request(id, &proj, &out);
            r.buffers = vec![("main.tex".into(), t.to_string())];
            c.compile(&r).unwrap();
            let (_, done) = events(&mut c, id);
            assert_eq!(done.str_field("status"), Some("ok"), "{tag}: compile {id}");
        }
        c.bye().unwrap();
        let mut h = h;
        let t = Instant::now();
        while h.child.try_wait().unwrap().is_none() {
            assert!(
                t.elapsed() < Duration::from_secs(120),
                "the host did not end"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(cache)
            .expect("the S0 cache")
            .map(|e| e.unwrap().path())
            .collect();
        // (the S₀ first, then its base)
        files.sort_by_key(|p| p.extension().is_some_and(|e| e == "xs0base"));
        assert!(
            files.len() == 2
                && files[0].extension().is_some_and(|e| e == "xs0")
                && files[1].extension().is_some_and(|e| e == "xs0base"),
            "{tag}: one S0 and its base saved: {files:?}"
        );
        // (each cache has a base of its own, and a base's identity is a
        // nonce of the file written: S₀ refers to it, so it is masked
        // here; the nonce follows the base's magic)
        let base = std::fs::read(&files[1]).unwrap();
        let magic = b"flashtex xetex S0 base v1";
        let at = base
            .windows(magic.len())
            .position(|w| w == magic)
            .expect("the base's magic")
            + magic.len();
        let nonce = &base[at..at + 16];
        let mut s0 = std::fs::read(&files[0]).unwrap();
        let refs: Vec<usize> = s0
            .windows(16)
            .enumerate()
            .filter(|(_, w)| *w == nonce)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(refs.len(), 1, "{tag}: S0 refers to its base once");
        s0[refs[0]..refs[0] + 16].fill(0);
        s0
    };
    // where two files differ: the header's length, the first differing
    // byte, and the differing 16 KiB blocks after the header
    let diff = |a: &[u8], b: &[u8]| -> String {
        let head = u64::from_le_bytes(a[..8].try_into().unwrap()) as usize + 8;
        let first = a.iter().zip(b).position(|(x, y)| x != y);
        let blocks: Vec<usize> = (0..a.len().min(b.len()).div_ceil(16384))
            .filter(|&k| {
                let r = k * 16384..((k + 1) * 16384).min(a.len()).min(b.len());
                a[r.clone()] != b[r]
            })
            .collect();
        // (the bytes around the first difference, as text)
        let around = |x: &[u8]| -> String {
            first.map_or(String::new(), |f| {
                String::from_utf8_lossy(&x[f.saturating_sub(48)..(f + 48).min(x.len())])
                    .escape_debug()
                    .to_string()
            })
        };
        format!(
            "{} and {} bytes, header {head} bytes, first difference at {first:?}, {} blocks differ: {:?}; around it: {:?} vs {:?}",
            a.len(),
            b.len(),
            blocks.len(),
            &blocks[..blocks.len().min(20)],
            around(a),
            around(b)
        )
    };
    let at_once = saved(&dir.join("cache-a"), &[doc], "at-once");
    // Two hosts that save at once write the same file but for the stat
    // signatures S₀'s key holds of the files it read (`StatSig`: the
    // `.aux` the first pass wrote has times of its own in each host).
    // Those bytes (times and inode numbers, within the header), each with
    // the 7 on either side (a signature's field is 8 bytes), are the only
    // ones a deferred save may differ in.
    let again = saved(&dir.join("cache-c"), &[doc], "at-once-again");
    assert_eq!(at_once.len(), again.len(), "two hosts' S0 files' sizes");
    let head = u64::from_le_bytes(at_once[..8].try_into().unwrap()) as usize + 8;
    let mut volatile = vec![false; at_once.len()];
    let mut differ = 0;
    for p in (0..at_once.len()).filter(|&p| at_once[p] != again[p]) {
        differ += 1;
        volatile[p.saturating_sub(7)..(p + 8).min(at_once.len())].fill(true);
        assert!(
            p < head,
            "two hosts' S0 files differ past the header: {}",
            diff(&at_once, &again)
        );
    }
    assert!(
        differ <= 256,
        "two hosts' S0 files differ in {differ} bytes: {}",
        diff(&at_once, &again)
    );
    let edited = doc.replace("Some words", "Some more words");
    let later = saved(&dir.join("cache-b"), &[doc, &edited], "later");
    assert!(
        at_once.len() == later.len()
            && (0..at_once.len()).all(|p| volatile[p] || at_once[p] == later[p]),
        "the S0 saved after a keystroke differs from the one saved at once: {}",
        diff(&at_once, &later)
    );
    let log = std::fs::read_to_string(dir.join("later.err")).unwrap_or_default();
    assert!(log.contains("\"saved_s0\""), "the save was made: {log}");
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

/// The convergence jump (`crate::conv`): an edit early in a long document
/// with labels; the restarted run converges a few pages on and goes on from
/// the old run's newest page checkpoint. Its PDF, `.aux` and log are a run
/// of its own's, and the next edit (from a carried checkpoint) too.
#[test]
fn a_converged_run_writes_what_a_run_of_its_own_writes() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-host-jump-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let para = "Words and more words that fill the page, again and again, until the \
                paragraph is long enough to break into several lines of text. ";
    let mut body = String::new();
    for i in 1..=30 {
        body.push_str(&format!("\\section{{Part {i}}}\\label{{p{i}}}\n"));
        for _ in 0..4 {
            body.push_str(&para.repeat(3));
            body.push_str("\n\n");
        }
        body.push_str(&format!("See~\\ref{{p{}}}.\n\n", 31 - i));
    }
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n\\begin{{document}}\n{body}\\end{{document}}\n"
    );
    let first = doc.replacen("Words and more", "Wards and more", 1);
    let second = first.replacen("Part 25}", "Part 25 again}", 1);
    let sub = dir.join("run");
    let (proj, out) = project(&sub, &doc);
    let err = sub.join("host.err");
    let o = Path::new(&out);
    let outputs = || {
        (
            std::fs::read(o.join("main.pdf")).unwrap(),
            std::fs::read_to_string(o.join("main.aux")).unwrap(),
            std::fs::read_to_string(o.join("main.log")).unwrap(),
        )
    };
    let compile = |c: &mut Client, id: i64, text: &str| {
        let mut r = request(id, &proj, &out);
        r.buffers = vec![("main.tex".into(), text.to_string())];
        c.compile(&r).unwrap();
        events(c, id).1
    };
    let h = start_with(
        &sub,
        &[
            ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
            ("FLASHTEX_UNICODE_CONV_DEBUG", "1"),
        ],
        &err,
    );
    let mut c = Client::connect(&h.sock).unwrap();
    compile(&mut c, 1, &doc);
    compile(&mut c, 2, &doc);
    compile(&mut c, 3, &first);
    let got1 = outputs();
    let done = compile(&mut c, 4, &second);
    let got2 = outputs();
    c.bye().unwrap();
    drop(h);
    let log = std::fs::read_to_string(&err).unwrap();
    assert!(log.contains("[conv] jumped"), "no jump: {log}");
    assert!(!log.contains("UNSOUND"), "{log}");
    for (text, got) in [(&first, got1), (&second, got2)] {
        let h = start_with(
            &sub,
            &[
                ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
                ("FLASHTEX_UNICODE_INPROCESS", "0"),
            ],
            &sub.join("host2.err"),
        );
        let mut c = Client::connect(&h.sock).unwrap();
        // (twice: the reference's .aux settles as the measured run's did)
        compile(&mut c, 1, text);
        compile(&mut c, 2, text);
        let want = outputs();
        c.bye().unwrap();
        drop(h);
        assert!(got.0 == want.0, "the PDFs differ ({done:?})");
        assert_eq!(got.1, want.1);
        assert_eq!(got.2, want.2);
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// READ-TRACK (`crate::dormant`, docs/design/engine-v2/READ-TRACK.md): a
/// command with an optional argument, alone on its line, peeks at the next
/// line and then typesets pages. Keystrokes at the start of that line after
/// the first change only its unread rest: the run jumps to the old run's
/// last page before it reads the line (here the `\end{document}` line's, and
/// one read before more pages), and every compile's PDF, `.aux` and log are
/// a run of its own's.
#[test]
fn a_dormant_line_is_kept_until_it_is_read() {
    if !texlive() {
        eprintln!("skipped: no TeX Live");
        return;
    }
    let para = |i: usize, w: &str| {
        format!(
            "Paragraph {i} with the word {w}, and enough text to fill a few lines of the \
             page so that the document ships several pages.\n\n"
        )
    };
    let tail: String = (0..160).map(|i| para(i, "tail")).collect();
    let more: String = (0..60).map(|i| para(i, "more")).collect();
    let body: String = (0..40).map(|i| para(i, "body")).collect();
    // (`hyper`: the kept pages make links, destinations and outline entries,
    // which the document taken over at the jump must hold, READ-TRACK-3)
    let parts: String = (0..8)
        .map(|i| {
            format!(
                "\\section{{Part {i}}}\\label{{part{i}}}See part~\\ref{{part{}}}.\n\n{}",
                (i + 3) % 8,
                (0..20).map(|j| para(j, "part")).collect::<String>()
            )
        })
        .collect();
    for (name, line, pre, tail) in [
        ("end", "", "", tail.as_str()),
        ("read", "Zword \\more\n", "", tail.as_str()),
        ("hyper", "", "\\usepackage{hyperref}\n", parts.as_str()),
    ] {
        let dir = std::env::temp_dir().join(format!(
            "flashtex-host-dormant-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let doc = format!(
            "\\documentclass{{article}}\n\\usepackage{{fontspec}}\n{pre}\
             \\newcommand\\tail[1][]{{#1{tail}}}\n\\newcommand\\more{{{more}}}\n\
             \\begin{{document}}\n{body}\\tail\n{line}\\end{{document}}\n"
        );
        let at = doc.find("\\tail\n").unwrap() + "\\tail\n".len();
        let keys: Vec<String> = (1..=3)
            .map(|k| {
                let mut t = doc.clone();
                t.insert_str(at, &"x".repeat(k));
                t
            })
            .collect();
        let sub = dir.join("run");
        let (proj, out) = project(&sub, &doc);
        let err = sub.join("host.err");
        let o = Path::new(&out);
        let outputs = || {
            (
                std::fs::read(o.join("main.pdf")).unwrap(),
                std::fs::read_to_string(o.join("main.aux")).unwrap(),
                std::fs::read_to_string(o.join("main.log")).unwrap(),
            )
        };
        let compile = |c: &mut Client, id: i64, text: &str| {
            let mut r = request(id, &proj, &out);
            r.buffers = vec![("main.tex".into(), text.to_string())];
            c.compile(&r).unwrap();
            events(c, id).1
        };
        let h = start_with(
            &sub,
            &[
                ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
                ("FLASHTEX_UNICODE_CONV_DEBUG", "1"),
            ],
            &err,
        );
        let mut c = Client::connect(&h.sock).unwrap();
        compile(&mut c, 1, &doc);
        compile(&mut c, 2, &doc);
        let mut got = vec![];
        for (k, text) in keys.iter().enumerate() {
            compile(&mut c, 3 + k as i64, text);
            got.push(outputs());
        }
        c.bye().unwrap();
        drop(h);
        let log = std::fs::read_to_string(&err).unwrap();
        assert!(
            log.contains("a dormant line"),
            "{name}: no dormant line: {log}"
        );
        assert!(log.contains("[conv] jumped"), "{name}: no jump: {log}");
        assert!(
            log.contains("pages taken over"),
            "{name}: the old documents were not taken over: {log}"
        );
        assert!(!log.contains("UNSOUND"), "{name}: {log}");
        for (text, got) in keys.iter().zip(got) {
            let h = start_with(
                &sub,
                &[
                    ("FLASHTEX_PIN_CLOCK", "1760000000.5"),
                    ("FLASHTEX_UNICODE_INPROCESS", "0"),
                ],
                &sub.join("host2.err"),
            );
            let mut c = Client::connect(&h.sock).unwrap();
            compile(&mut c, 1, text);
            compile(&mut c, 2, text);
            let want = outputs();
            c.bye().unwrap();
            drop(h);
            assert!(got.0 == want.0, "{name}: the PDFs differ");
            assert_eq!(got.1, want.1, "{name}");
            assert_eq!(got.2, want.2, "{name}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The bundle's fonts by name (#1783) in both kinds of run: in the host's
/// own process (`crate::resident`'s kept catalog, the app's default) and in
/// a process of its own. Opt-in: `FLASHTEX_TEST_FONT_BUNDLE=<digest>@<url>`
/// names a bundle with the font-name index (texbundle-tl2026-4 or later, or
/// one `texbundle.py pack --font-index-tool` made); the resolver is forced
/// to it, so TeX Live's fonts are not what is found.
#[test]
fn bundle_fonts_are_found_by_name_resident_or_not() {
    let Some((digest, url)) = std::env::var("FLASHTEX_TEST_FONT_BUNDLE")
        .ok()
        .and_then(|v| {
            v.split_once('@')
                .map(|(d, u)| (d.to_string(), u.to_string()))
        })
    else {
        eprintln!("skipped: FLASHTEX_TEST_FONT_BUNDLE is not set");
        return;
    };
    let doc = "\\documentclass{article}\n\\usepackage{fontspec}\n\
               \\setmainfont{TeX Gyre Pagella}\n\\begin{document}\nHello, \\textbf{bold}.\n\\end{document}\n";
    // short: the bundle's paths stay well under XDV's 255 bytes
    let cache = PathBuf::from(format!("/tmp/ftxbfi-{}", std::process::id()));
    for inproc in ["1", "0"] {
        let dir =
            std::env::temp_dir().join(format!("flashtex-bfi-{}-{inproc}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (proj, out) = project(&dir, doc);
        let allow = format!("{digest}@{url}");
        let envs = [
            ("FLASHTEX_UNICODE_INPROCESS", inproc),
            ("FLASHTEX_RESOLVER", "bundle"),
            ("FLASHTEX_BUNDLE_URL", url.as_str()),
            ("FLASHTEX_BUNDLE_DIGEST", digest.as_str()),
            ("FLASHTEX_BUNDLE_ALLOW_FETCH", allow.as_str()),
            ("FLASHTEX_BUNDLE_CACHE_DIR", cache.to_str().unwrap()),
        ];
        let h = start_with(&dir, &envs, &dir.join("err.txt"));
        let mut c = Client::connect(&h.sock).unwrap();
        // twice: the second run of a resident engine reuses its kept catalog
        for id in [1, 2] {
            c.compile(&request(id, &proj, &out)).unwrap();
            let (ev, done) = events(&mut c, id);
            assert_eq!(
                done.str_field("status"),
                Some("ok"),
                "inproc={inproc} run {id}: {done:?}"
            );
            let fonts: Vec<_> = ev
                .iter()
                .filter_map(|e| match e {
                    Event::Font(f) => Some(f.info.str_field("ps_name").unwrap_or("").to_string()),
                    _ => None,
                })
                .collect();
            if id == 1 {
                for want in ["TeXGyrePagella-Regular", "TeXGyrePagella-Bold"] {
                    assert!(
                        fonts.iter().any(|p| p == want),
                        "inproc={inproc}: {fonts:?}"
                    );
                }
            }
        }
        c.bye().unwrap();
        drop(h);
        let _ = std::fs::remove_dir_all(&dir);
    }
    let _ = std::fs::remove_dir_all(&cache);
}

/// `--list-fonts` and `--list-math-fonts` (app-parity row A9): the
/// families the host's lookup catalog finds by name, one a line, sorted;
/// `FLASHTEX_FONT_DIRS` counts, as it does for a compile. The names are
/// ones TeX Live's `xetex` found in `apps/mac/Fonts` (the oracle run cited
/// by `fontmgr::catalog`'s `the_families_are_the_names_xelatex_finds`).
/// Needs no TeX.
#[test]
fn the_font_lists_name_the_families_a_lookup_finds() {
    let fonts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts");
    // the catalog's name cache, never the user's
    let cache =
        std::env::temp_dir().join(format!("flashtex-host-font-list-{}", std::process::id()));
    let list = |flag: &str| -> Vec<String> {
        let out = Command::new(env!("CARGO_BIN_EXE_flashtex-host-unicode"))
            .arg(flag)
            .env("FLASHTEX_FONT_DIRS", &fonts)
            .env("FLASHTEX_CACHE_DIR", &cache)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{flag}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(String::from)
            .collect()
    };
    let all = list("--list-fonts");
    for f in [
        "Latin Modern Math",
        "Latin Modern Roman",
        "Latin Modern Sans",
        "TeX Gyre Termes",
    ] {
        assert!(all.iter().any(|x| x == f), "{f} in {all:?}");
    }
    let mut sorted = all.clone();
    sorted.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then(a.cmp(b)));
    sorted.dedup();
    assert_eq!(all, sorted, "sorted, once each");
    let math = list("--list-math-fonts");
    for f in ["Latin Modern Math", "NewComputerModernMath"] {
        assert!(math.iter().any(|x| x == f), "{f} in {math:?}");
    }
    assert!(!math.iter().any(|x| x == "Latin Modern Roman"), "{math:?}");
    assert!(
        math.iter().all(|m| all.contains(m)),
        "every math family is listed"
    );
    let _ = std::fs::remove_dir_all(&cache);
}
