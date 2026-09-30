//! The client against a scripted host (no engine): the handshake, a compile's
//! message sequence, cancellation, version refusal, and fail-closed decoding.
//! The end-to-end test against the real engine host is
//! crates/flashtex-engine/tests/display_list_host.rs (it needs the engine's
//! binaries, which only that crate builds).

use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::json::{obj, s, Json};
use flashtex_display_list::page::{Item, Page, StreamKind};
use flashtex_display_list::resource::{Font, Sources};
use flashtex_display_list::{kind, PROTOCOL};
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

fn sock(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("dl3-{name}-{}.sock", std::process::id()))
}

fn hello(major: i64) -> Vec<u8> {
    obj([
        ("protocol", s(PROTOCOL)),
        ("version", Json::Arr(vec![Json::Int(major), Json::Int(0)])),
    ])
    .to_string()
    .into_bytes()
}

/// A host that answers one compile with a font, sources, one page and DONE,
/// and a CANCEL with DONE(cancelled).
fn scripted_host(listener: UnixListener) {
    let (mut c, _) = listener.accept().unwrap();
    let (k, _) = read_frame(&mut c).unwrap().unwrap();
    assert_eq!(k, kind::C_HELLO);
    write_frame(&mut c, kind::HELLO, &hello(3)).unwrap();
    loop {
        let Some((k, body)) = read_frame(&mut c).unwrap() else {
            return;
        };
        let j = Json::parse(std::str::from_utf8(&body).unwrap()).unwrap();
        let id = j.int_field("id").unwrap_or(0);
        match k {
            kind::COMPILE => {
                assert_eq!(j.str_field("main"), Some("main.tex"));
                write_frame(
                    &mut c,
                    kind::STARTED,
                    obj([("id", Json::Int(id))]).to_string().as_bytes(),
                )
                .unwrap();
                let font = Font {
                    id: 41,
                    key: [7; 32],
                    info: obj([("pdf_name", s("F41")), ("format", s("type1"))]),
                    program: b"%!PS-AdobeFont-1.0".to_vec(),
                };
                write_frame(&mut c, kind::FONT, &font.encode()).unwrap();
                let src = Sources {
                    files: vec![(1, "/p/main.tex".into())],
                    spans: vec![(1, 1, 12)],
                };
                write_frame(&mut c, kind::SOURCES, src.to_json().to_string().as_bytes()).unwrap();
                let mut p = Page::new(StreamKind::Page, 0);
                p.matrices.push([10.0, 0.0, 0.0, 10.0, 0.0, 0.0]);
                p.items = vec![
                    Item::Span(1),
                    Item::Matrix(1),
                    Item::Glyph {
                        font: 41,
                        code: 72,
                        x: 100,
                        y: 200,
                        col: 0,
                    },
                ];
                p.hash = p.content_hash(&|_| [7; 32], &|_| [0; 32]);
                write_frame(&mut c, kind::PAGE, &p.encode()).unwrap();
                write_frame(
                    &mut c,
                    kind::DONE,
                    obj([
                        ("id", Json::Int(id)),
                        ("status", s("ok")),
                        ("pages", Json::Int(1)),
                    ])
                    .to_string()
                    .as_bytes(),
                )
                .unwrap();
            }
            kind::CANCEL => {
                write_frame(
                    &mut c,
                    kind::DONE,
                    obj([("id", Json::Int(id)), ("status", s("cancelled"))])
                        .to_string()
                        .as_bytes(),
                )
                .unwrap();
            }
            kind::BYE => return,
            _ => panic!("unexpected kind {k}"),
        }
        c.flush().unwrap();
    }
}

#[test]
fn compile_sequence_and_cancel() {
    let path = sock("seq");
    let _ = std::fs::remove_file(&path);
    let l = UnixListener::bind(&path).unwrap();
    let host = std::thread::spawn(move || scripted_host(l));
    let mut c = Client::connect(&path).unwrap();
    assert_eq!(c.hello.str_field("protocol"), Some(PROTOCOL));
    c.compile(&CompileRequest::new(1, "/p", "main.tex"))
        .unwrap();
    let mut seen = vec![];
    let mut fonts = std::collections::HashMap::new();
    loop {
        match c.next_event().unwrap().unwrap() {
            Event::Started(_) => seen.push("started"),
            Event::Font(f) => {
                fonts.insert(f.id, f.key);
                seen.push("font")
            }
            Event::Sources(src) => {
                assert_eq!(src.spans, vec![(1, 1, 12)]);
                seen.push("sources")
            }
            Event::Page(p) => {
                // Every glyph's font arrived before the page.
                for it in &p.items {
                    if let Item::Glyph { font, .. } = it {
                        assert!(fonts.contains_key(font));
                    }
                }
                assert_eq!(p.hash, p.content_hash(&|f| fonts[&f], &|_| [0; 32]));
                seen.push("page")
            }
            Event::Done(d) => {
                assert_eq!(d.str_field("status"), Some("ok"));
                seen.push("done");
                break;
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(seen, ["started", "font", "sources", "page", "done"]);
    let mut k = c.canceller().unwrap();
    k.cancel(1).unwrap();
    match c.next_event().unwrap().unwrap() {
        Event::Done(d) => assert_eq!(d.str_field("status"), Some("cancelled")),
        other => panic!("unexpected {other:?}"),
    }
    c.bye().unwrap();
    host.join().unwrap();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn refuses_another_major_version() {
    let path = sock("ver");
    let _ = std::fs::remove_file(&path);
    let l = UnixListener::bind(&path).unwrap();
    let host = std::thread::spawn(move || {
        let (mut c, _) = l.accept().unwrap();
        read_frame(&mut c).unwrap().unwrap();
        write_frame(&mut c, kind::HELLO, &hello(4)).unwrap();
    });
    let err = Client::connect(&path)
        .err()
        .expect("a version-4 host is refused");
    assert!(err.to_string().contains("version"), "{err}");
    host.join().unwrap();
    let _ = std::fs::remove_file(&path);
}

#[test]
fn corrupt_page_fails_closed() {
    let (a, mut b) = UnixStream::pair().unwrap();
    let t = std::thread::spawn(move || {
        let mut a2 = a.try_clone().unwrap();
        read_frame(&mut a2).unwrap().unwrap();
        write_frame(&mut a2, kind::HELLO, &hello(3)).unwrap();
        // The fixed header (120 bytes), then one ITEMS section holding an
        // opcode no version defines.
        let mut p = Page::new(StreamKind::Page, 0).encode();
        p.truncate(120);
        p.extend_from_slice(&1u32.to_le_bytes());
        p.extend_from_slice(&3u32.to_le_bytes());
        p.extend_from_slice(&1u32.to_le_bytes());
        p.push(0xEE);
        write_frame(&mut a2, kind::PAGE, &p).unwrap();
    });
    let mut c = Client::over(b.try_clone().unwrap()).unwrap();
    let e = c.next_event().expect_err("an unknown opcode is an error");
    assert!(e.to_string().contains("opcode"), "{e}");
    t.join().unwrap();
    let _ = b.flush();
}
