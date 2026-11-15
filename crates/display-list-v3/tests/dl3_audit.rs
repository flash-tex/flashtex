//! Self-tests of `examples/dl3-audit.rs` against scripted hosts (no engine):
//! a step passes only when both compiles end `ok` with the same page count
//! above 0 and the pages compare equal. Compiles that fail, an `error`
//! status, a document of no pages, or a client page that differs from the
//! scratch one must each fail the audit; a compile that ends before the
//! CANCEL must not hang it.
#![cfg(unix)]

#[path = "../examples/dl3-audit.rs"]
#[allow(dead_code)]
mod audit;

use flashtex_display_list::client::Event;
use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::held::Model;
use flashtex_display_list::json::{obj, s, Json};
use flashtex_display_list::page::{Item, Page, StreamKind};
use flashtex_display_list::resource::{Font, Sources};
use flashtex_display_list::transport::{Listener, Stream};
use flashtex_display_list::{kind, PROTOCOL};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// One page, `DONE` `ok`, in both hosts.
    Ok,
    /// Every compile dies: `DONE` `failed`, no `pages`.
    Failed,
    /// One page, but `DONE` `error`.
    Error,
    /// `DONE` `ok` with 0 pages.
    NoPages,
    /// The incremental compile's page differs from the scratch one.
    ClientDiffers,
    /// After the first compile on a connection, `DONE` `ok` `unchanged`
    /// without a page (the client holds it).
    Unchanged,
}

fn send(c: &mut Stream, k: u8, j: Json) {
    write_frame(c, k, j.to_string().as_bytes()).unwrap();
}

fn connection(mut c: Stream, mode: Mode) {
    let Ok(Some((k, _))) = read_frame(&mut c) else {
        return;
    };
    assert_eq!(k, kind::C_HELLO);
    send(
        &mut c,
        kind::HELLO,
        obj([
            ("protocol", s(PROTOCOL)),
            ("version", Json::Arr(vec![Json::Int(3), Json::Int(2)])),
        ]),
    );
    let mut compiles = 0;
    loop {
        let Ok(Some((k, body))) = read_frame(&mut c) else {
            return;
        };
        let j = Json::parse(std::str::from_utf8(&body).unwrap()).unwrap();
        let id = j.int_field("id").unwrap_or(0);
        match k {
            kind::COMPILE => {
                let incr = j.get("incremental").and_then(Json::as_bool) == Some(true);
                let keep = incr && compiles > 0;
                compiles += 1;
                send(
                    &mut c,
                    kind::STARTED,
                    obj([
                        ("id", Json::Int(id)),
                        ("mode", s("resident")),
                        ("keep", Json::Bool(keep)),
                    ]),
                );
                let done = |st: &str, pages: Option<i64>, m: &str| {
                    let mut kv = vec![
                        ("id".to_string(), Json::Int(id)),
                        ("status".to_string(), s(st)),
                        ("mode".to_string(), s(m)),
                    ];
                    if let Some(p) = pages {
                        kv.push(("pages".to_string(), Json::Int(p)));
                    }
                    Json::Obj(kv)
                };
                match mode {
                    Mode::Failed => {
                        send(&mut c, kind::DONE, done("failed", None, "cold"));
                        continue;
                    }
                    Mode::NoPages => {
                        send(&mut c, kind::DONE, done("ok", Some(0), "cold"));
                        continue;
                    }
                    Mode::Unchanged if keep => {
                        send(&mut c, kind::DONE, done("ok", Some(1), "unchanged"));
                        continue;
                    }
                    _ => {}
                }
                let font = Font {
                    id: 1,
                    key: [7; 32],
                    info: obj([("pdf_name", s("F1")), ("format", s("type1"))]),
                    program: b"%!PS-AdobeFont-1.0".to_vec(),
                };
                write_frame(&mut c, kind::FONT, &font.encode()).unwrap();
                let root = j.str_field("root").unwrap_or("/p").to_string();
                let src = Sources {
                    files: vec![(1, format!("{root}/main.tex"))],
                    spans: vec![(1, 1, 12)],
                };
                send(&mut c, kind::SOURCES, src.to_json());
                let mut p = Page::new(StreamKind::Page, 0);
                p.matrices.push([10.0, 0.0, 0.0, 10.0, 0.0, 0.0]);
                let code = if mode == Mode::ClientDiffers && incr { 73 } else { 72 };
                p.items = vec![
                    Item::Span(1),
                    Item::Matrix(1),
                    Item::Glyph {
                        font: 1,
                        code,
                        x: 100,
                        y: 200,
                        col: 0,
                    },
                ];
                p.hash = p.content_hash(&|_| [7; 32], &|_| [0; 32]);
                write_frame(&mut c, kind::PAGE, &p.encode()).unwrap();
                let st = if mode == Mode::Error { "error" } else { "ok" };
                let m = if keep { "incremental" } else { "cold" };
                send(&mut c, kind::DONE, done(st, Some(1), m));
            }
            // every compile here has ended before a CANCEL can arrive: ignored
            kind::CANCEL => {}
            kind::BYE => return,
            _ => {}
        }
        c.flush().unwrap();
    }
}

/// A host listening at a fresh socket, answering every connection in `mode`.
fn host(name: &str, mode: Mode) -> PathBuf {
    let path = std::env::temp_dir().join(format!("dl3a-{name}-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let l = Listener::bind(&path).unwrap();
    std::thread::spawn(move || {
        while let Ok((c, _)) = l.accept() {
            std::thread::spawn(move || connection(c, mode));
        }
    });
    path
}

/// Run the audit's `steps` against a client host in `client` and a scratch
/// host in `scratch`; the number of failures.
fn audit(name: &str, client: Mode, scratch: Mode, steps: &str) -> usize {
    let main = host(&format!("{name}-c"), client);
    let scr = host(&format!("{name}-s"), scratch);
    let work = std::env::temp_dir().join(format!("dl3a-{name}-{}", std::process::id()));
    let n = audit::run(audit::Opts {
        hosts: audit::Hosts::Connect { main, scratch: scr },
        work: work.clone(),
        format: None,
        images: vec![],
        steps: Some(steps.into()),
        xetex: false,
        timeout: Duration::from_secs(120),
    });
    let _ = std::fs::remove_dir_all(&work);
    n
}

#[test]
fn hosts_that_agree_pass() {
    // the positive control: the scripted hosts are good enough to pass
    assert_eq!(audit("ok", Mode::Ok, Mode::Ok, "open,same,comment"), 0);
}

#[test]
fn failed_compiles_fail_the_audit() {
    // S-REVIEW MAJOR 1 on #2397: both compiles failed, both counts 0, and
    // every step printed OK
    assert_eq!(audit("failed", Mode::Failed, Mode::Failed, "open,same"), 3);
    // one side failing is enough
    assert!(audit("failed-c", Mode::Failed, Mode::Ok, "open") > 0);
    assert!(audit("failed-s", Mode::Ok, Mode::Failed, "open") > 0);
}

#[test]
fn an_error_status_fails_the_audit() {
    assert!(audit("error", Mode::Error, Mode::Error, "open") > 0);
}

#[test]
fn a_document_of_no_pages_compares_nothing_and_fails() {
    assert_eq!(audit("nopages", Mode::NoPages, Mode::NoPages, "open"), 2);
}

#[test]
fn a_page_that_differs_from_scratch_fails() {
    // the negative control for the comparison itself
    assert!(audit("differs", Mode::ClientDiffers, Mode::Ok, "open") > 0);
}

#[test]
fn a_host_that_is_not_there_fails_the_audit() {
    let work = std::env::temp_dir().join(format!("dl3a-nohost-{}", std::process::id()));
    let n = audit::run(audit::Opts {
        hosts: audit::Hosts::Spawn {
            exe: "/nonexistent/flashtex-host".into(),
            args: vec![],
        },
        work: work.clone(),
        format: None,
        images: vec![],
        steps: Some("open".into()),
        xetex: false,
        timeout: Duration::from_secs(120),
    });
    let _ = std::fs::remove_dir_all(&work);
    assert!(n > 0);
}

#[test]
fn a_compile_that_ends_before_its_cancel_does_not_hang() {
    // S-REVIEW MINOR 2: an `unchanged` compile ships no page, so the CANCEL
    // finds nothing running; the audit must go on (with the timeout as the
    // backstop, a hang would end the test process)
    assert_eq!(audit("unchanged", Mode::Unchanged, Mode::Ok, "open,cancel,cancel"), 0);
}

#[test]
fn the_noimga_step_needs_two_images() {
    // without images the step equals `regrow`: it must not pass testing nothing
    assert_eq!(audit("noimg", Mode::Ok, Mode::Ok, "open,noimgA"), 1);
}

#[test]
fn verdict_requires_ok_and_pages() {
    let done = |st: &str, pages: Option<i64>| {
        let mut kv = vec![("id".to_string(), Json::Int(1)), ("status".to_string(), s(st))];
        if let Some(p) = pages {
            kv.push(("pages".to_string(), Json::Int(p)));
        }
        Json::Obj(kv)
    };
    let model = |d: &Json| {
        let mut m = Model::default();
        m.apply(Event::Done(d.clone()));
        m
    };
    let failed = done("failed", None);
    let (bad, n) = audit::verdict(
        &model(&failed),
        Some(&failed),
        &model(&failed),
        Some(&failed),
        "/o1",
        "/o2",
    );
    assert_eq!(n, 0);
    assert!(bad.iter().any(|b| b.starts_with("client: DONE status")), "{bad:?}");
    assert!(bad.iter().any(|b| b.starts_with("scratch: DONE status")), "{bad:?}");
    assert!(bad.iter().any(|b| b.starts_with("nothing to compare")), "{bad:?}");
    let (bad, _) = audit::verdict(&Model::default(), None, &Model::default(), None, "/o1", "/o2");
    assert!(bad.iter().any(|b| b == "client: no DONE"), "{bad:?}");
    assert!(bad.iter().any(|b| b == "scratch: no DONE"), "{bad:?}");
}
