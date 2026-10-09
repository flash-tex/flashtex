//! `flashtex-host-unicode` over its socket (spec §6), as the app and
//! `flashtex-v3` use it: a xelatex document with fontspec and a cross
//! reference compiles in two passes, its pages arrive as display lists
//! with OpenType fonts, `DONE` names FlashTeX's PDF; an export compile; an
//! edit applied before the next compile; a cancelled compile. Needs TeX
//! Live (the format is FlashTeX's own, built from it); skipped without.

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

    // a compile cancelled while it runs
    let mut r = CompileRequest::new(4, proj.to_str().unwrap(), "main.tex");
    r.output_dir = Some(out.to_string_lossy().into_owned());
    c.compile(&r).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    c.cancel(4).unwrap();
    let (_, done) = events(&mut c, 4);
    assert!(
        matches!(done.str_field("status"), Some("cancelled" | "ok")),
        "{done:?}"
    );
    c.bye().unwrap();
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}
