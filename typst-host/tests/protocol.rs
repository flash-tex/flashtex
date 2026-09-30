//! Protocol round trips against the running host (spec
//! `docs/protocol/display-list-v3.md` §6, §7; DESIGN.md §15.4): handshake and
//! version negotiation, message order, resources before use, the 3.2 draft
//! sections, the 3.1 fallback, held fonts, incremental compiles, request
//! errors, diagnostics, confinement and packages. Every frame is decoded by
//! the MIT reference decoder (`flashtex-display-list`).

mod common;

use common::*;
use flashtex_display_list::client::{Client, CompileRequest, Event};
use flashtex_display_list::json::Json;
use flashtex_display_list::kind;
use flashtex_display_list::page::Item;
use flashtex_display_list::sha256::hex;
use flashtex_typst_host::v32;

const DOC: &str = "#set page(width: 8cm, height: 5cm, margin: 5mm)\n#set text(font: \"Libertinus Serif\")\nHello, world!\n#pagebreak()\nSecond *page*.\n#pagebreak()\nThird page.\n";

fn strs(j: &Json, k: &str) -> Vec<String> {
    j.get(k)
        .and_then(Json::as_array)
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn hello_negotiates_the_minor_version() {
    let host = HostProc::start("hello");
    assert!(
        host.ready.contains("\"typst\":\"0.15.1\""),
        "{}",
        host.ready
    );
    let mut c = host.connect();
    let (k, j) = c.hello(3, 2);
    assert_eq!(k, kind::HELLO);
    assert_eq!(j.str_field("protocol"), Some("display-list-v3"));
    assert_eq!(j.get("version").unwrap().to_string(), "[3,2]");
    assert_eq!(j.str_field("engine"), Some("Typst 0.15.1"));
    let caps = strs(&j, "capabilities");
    for cap in [
        "compile",
        "font-formats",
        "have-fonts",
        "incremental",
        "origins-f64",
        "page-meta",
        "opentype-glyphs",
    ] {
        assert!(
            caps.contains(&cap.to_string()),
            "{cap} missing from {caps:?}"
        );
    }
    assert!(
        !caps.contains(&"cancel".to_string()),
        "a Typst compile cannot be cancelled"
    );
    drop(c);

    // A later minor is answered with ours; the reference client says [3, 1].
    let mut c = host.connect();
    assert_eq!(c.hello(3, 9).1.get("version").unwrap().to_string(), "[3,2]");
    drop(c);
    let r = Client::connect(&host.socket).unwrap();
    assert_eq!(r.hello.get("version").unwrap().to_string(), "[3,1]");
    assert!(!strs(&r.hello, "capabilities").contains(&"origins-f64".to_string()));
}

#[test]
fn another_major_or_no_hello_is_refused() {
    let host = HostProc::start("refuse");
    let mut c = host.connect();
    let (k, j) = c.hello(4, 0);
    assert_eq!(k, kind::ERROR);
    assert_eq!(j.str_field("code"), Some("version"));
    assert!(c.frame().is_none(), "the host closes after a version error");

    let mut c = host.connect();
    c.send(kind::COMPILE, "{}");
    let (k, b) = c.frame().unwrap();
    assert_eq!(k, kind::ERROR);
    assert_eq!(json_of(&b).str_field("code"), Some("protocol"));
    assert!(c.frame().is_none());

    let mut c = host.connect();
    c.send(
        kind::C_HELLO,
        r#"{"protocol":"display-list-v2","version":[3,1]}"#,
    );
    let (k, b) = c.frame().unwrap();
    assert_eq!(
        (k, json_of(&b).str_field("code")),
        (kind::ERROR, Some("protocol"))
    );
}

/// A full compile for a 3.2 client: order, resources before use, sections,
/// hashes, positions.
#[test]
fn compile_round_trip_v32() {
    let host = HostProc::start("rt32");
    let root = project("rt32", DOC);
    let mut c = host.connect();
    c.hello(3, 2);
    c.send(
        kind::COMPILE,
        &compile_json(7, &root, "main.typ", r#""font_formats":["opentype"]"#),
    );
    let frames = c.until_done();
    let evs = events(&frames);
    assert!(
        matches!(&evs[0], Event::Started(j) if j.int_field("id") == Some(7) && j.str_field("mode") == Some("resident"))
    );
    let Event::Done(done) = evs.last().unwrap() else {
        panic!("DONE last")
    };
    assert_eq!(done.int_field("id"), Some(7));
    assert_eq!(done.str_field("status"), Some("ok"));
    assert_eq!(done.int_field("pages"), Some(3));
    assert_eq!(done.str_field("mode"), Some("cold"));
    assert_eq!(
        evs.iter().filter(|e| matches!(e, Event::Done(_))).count(),
        1
    );

    let mut fonts = std::collections::HashMap::new();
    let mut spans = std::collections::HashSet::new();
    let mut files = std::collections::HashSet::new();
    let mut page_no = 0;
    for ((_, body), e) in frames.iter().zip(&evs) {
        match e {
            Event::Font(f) => {
                assert_eq!(f.info.str_field("format"), Some("opentype"));
                assert!(
                    !f.program.is_empty(),
                    "a client taking opentype gets the program"
                );
                assert_eq!(
                    f.info.int_field("program_bytes"),
                    Some(f.program.len() as i64)
                );
                assert_eq!(
                    f.info.str_field("program_sha256").unwrap(),
                    hex(&flashtex_display_list::sha256::sha256(&f.program))
                );
                assert!(f
                    .info
                    .str_field("file")
                    .unwrap()
                    .starts_with(font_dir().to_str().unwrap()));
                fonts.insert(f.id, f.key);
            }
            Event::Sources(s) => {
                for (id, path) in &s.files {
                    assert_eq!(path, root.join("main.typ").to_str().unwrap());
                    files.insert(*id);
                }
                for (id, file, line) in &s.spans {
                    assert!(files.contains(file));
                    assert!((1..=7).contains(line));
                    spans.insert(*id);
                }
            }
            Event::Page(p) => {
                assert_eq!(p.index, page_no);
                page_no += 1;
                assert_eq!(p.flags, 0, "{:?}", p.unsupported);
                assert_eq!(p.counts, [0; 10]);
                let mut glyphs = vec![];
                for it in &p.items {
                    match it {
                        Item::Glyph {
                            font, x, y, col, ..
                        } => {
                            assert!(fonts.contains_key(font), "FONT before its first use");
                            assert_ne!(*col, 0xFFFF, "prose glyphs have a column");
                            glyphs.push((*x, *y));
                        }
                        Item::Span(s) => assert!(
                            *s == 0 || spans.contains(s),
                            "SOURCES before the span's use"
                        ),
                        _ => {}
                    }
                }
                assert!(!glyphs.is_empty());
                // 3.2 sections: origins agree with the rounded positions.
                let secs = v32::sections(body).unwrap();
                let o = v32::decode_origins(
                    secs.iter()
                        .find(|(t, _)| *t == v32::tag::ORIGINS_F64)
                        .unwrap()
                        .1,
                )
                .unwrap();
                assert_eq!(o.len(), glyphs.len());
                let h = p.pdf_box[3];
                for ((x, y), (ox, oy)) in glyphs.iter().zip(&o) {
                    assert_eq!(*x, (ox * 65_781.76).round() as i32);
                    assert_eq!(*y, ((h - oy) * 65_781.76).round() as i32);
                }
                let meta = json_of(
                    secs.iter()
                        .find(|(t, _)| *t == v32::tag::PAGE_META)
                        .unwrap()
                        .1,
                );
                assert_eq!(meta.str_field("engine"), Some("typst"));
                assert_eq!(meta.int_field("number"), Some(page_no as i64));
                // The hash: v3's over what is drawn, extended by the 3.2 sections.
                let v3 = p.content_hash(&|id| fonts[&id], &|_| [0; 32]);
                let ext: Vec<(u32, Vec<u8>)> = secs
                    .iter()
                    .filter(|(t, _)| *t >= 7)
                    .map(|(t, d)| (*t, d.to_vec()))
                    .collect();
                assert_eq!(p.hash, v32::extended_hash(v3, &ext));
            }
            _ => {}
        }
    }
    assert_eq!(page_no, 3);
    // Every page is drawable, so no PDF is written unless asked for.
    assert!(done.str_field("pdf").is_none());
    c.send(
        kind::COMPILE,
        &compile_json(
            8,
            &root,
            "main.typ",
            r#""font_formats":["opentype"],"export":true"#,
        ),
    );
    let Event::Done(done) = events(&c.until_done()).pop().unwrap() else {
        panic!()
    };
    assert!(std::path::Path::new(done.str_field("pdf").unwrap()).is_file());
}

/// A 3.1 client (the reference `Client`) cannot draw OpenType glyph ids: it
/// gets every page INCOMPLETE, no 3.2 sections, no font programs, and falls
/// back to DONE.pdf (DESIGN.md §15.4).
#[test]
fn a_31_client_gets_incomplete_pages() {
    let host = HostProc::start("v31");
    let root = project("v31", DOC);
    let mut c = Client::connect(&host.socket).unwrap();
    let mut req = CompileRequest::new(1, root.to_str().unwrap(), "main.typ");
    req.format = "typst".into();
    c.compile(&req).unwrap();
    let mut pages = 0;
    loop {
        match c.next_event().unwrap().unwrap() {
            Event::Font(f) => assert!(f.program.is_empty()),
            Event::Page(p) => {
                pages += 1;
                assert_eq!(p.flags & 1, 1);
                assert!(p.unsupported.iter().any(|u| u.contains("opentype")));
            }
            Event::Done(d) => {
                assert_eq!(d.str_field("status"), Some("ok"));
                assert!(d.str_field("pdf").is_some());
                break;
            }
            _ => {}
        }
    }
    assert_eq!(pages, 3);
    drop(c); // the host serves one connection at a time
             // No 3.2 section reaches a 3.1 client: re-read raw to check.
    let mut raw = host.connect();
    raw.hello(3, 1);
    raw.send(
        kind::COMPILE,
        &compile_json(2, &root, "main.typ", r#""font_formats":["opentype"]"#),
    );
    for (k, b) in raw.until_done() {
        if k == kind::PAGE {
            assert!(v32::sections(&b).unwrap().iter().all(|(t, _)| *t <= 6));
        }
    }
}

#[test]
fn held_fonts_are_sent_without_programs() {
    let host = HostProc::start("held");
    let root = project("held", DOC);
    let mut c = host.connect();
    c.hello(3, 2);
    c.send(
        kind::COMPILE,
        &compile_json(1, &root, "main.typ", r#""font_formats":["opentype"]"#),
    );
    let keys: Vec<String> = events(&c.until_done())
        .iter()
        .filter_map(|e| {
            if let Event::Font(f) = e {
                Some(hex(&f.key))
            } else {
                None
            }
        })
        .collect();
    assert!(!keys.is_empty());
    let have = keys
        .iter()
        .map(|k| format!("{k:?}"))
        .collect::<Vec<_>>()
        .join(",");
    c.send(
        kind::COMPILE,
        &compile_json(
            2,
            &root,
            "main.typ",
            &format!(r#""font_formats":["opentype"],"have_fonts":[{have}]"#),
        ),
    );
    for e in events(&c.until_done()) {
        if let Event::Font(f) = e {
            assert!(f.program.is_empty(), "held font resent with its program");
            assert!(f.info.int_field("program_bytes").unwrap() > 0);
        }
    }
}

/// `incremental`: the second compile keeps ids, sends only the page that
/// changed, and says so in `PAGES` and `DONE`.
#[test]
fn incremental_compiles_send_only_changed_pages() {
    let host = HostProc::start("incr");
    let root = project("incr", DOC);
    let mut c = host.connect();
    c.hello(3, 2);
    let opts = r#""font_formats":["opentype"],"incremental":true"#;
    c.send(kind::COMPILE, &compile_json(1, &root, "main.typ", opts));
    let first = events(&c.until_done());
    assert_eq!(
        first.iter().filter(|e| matches!(e, Event::Page(_))).count(),
        3
    );
    let Event::Started(s) = &first[0] else {
        panic!()
    };
    assert_eq!(s.get("keep").and_then(Json::as_bool), Some(false));

    // Edit "Second" -> "Changed" on page 2 by a byte splice.
    let off = DOC.find("Second").unwrap();
    let edit = format!(
        r#"{opts},"edits":[{{"path":"main.typ","offset":{off},"delete":6,"insert":"Changed"}}]"#
    );
    c.send(kind::COMPILE, &compile_json(2, &root, "main.typ", &edit));
    let second = events(&c.until_done());
    let Event::Started(s) = &second[0] else {
        panic!()
    };
    assert_eq!(s.get("keep").and_then(Json::as_bool), Some(true));
    let pages: Vec<u32> = second
        .iter()
        .filter_map(|e| {
            if let Event::Page(p) = e {
                Some(p.index)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(pages, vec![1], "only the edited page is sent");
    assert!(
        !second
            .iter()
            .any(|e| matches!(e, Event::Font(f) if f.id == 0)),
        "font ids are kept, not resent"
    );
    let pages_msg = second
        .iter()
        .find_map(|e| {
            if let Event::Pages(j) = e {
                Some(j.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(pages_msg.int_field("count"), Some(3));
    assert_eq!(
        pages_msg.get("complete").and_then(Json::as_bool),
        Some(true)
    );
    let Event::Done(d) = second.last().unwrap() else {
        panic!()
    };
    assert_eq!(
        (d.str_field("mode"), d.int_field("typeset_pages")),
        (Some("incremental"), Some(1))
    );
    assert!(std::fs::read_to_string(root.join("main.typ"))
        .unwrap()
        .contains("Changed *page*"));

    // `buffers` replace a file's text.
    let buf = format!(
        r#"{opts},"buffers":[{{"path":"main.typ","text":{}}}]"#,
        Json::Str(DOC.replace("Third", "3rd"))
    );
    c.send(kind::COMPILE, &compile_json(3, &root, "main.typ", &buf));
    let third = events(&c.until_done());
    let pages: Vec<u32> = third
        .iter()
        .filter_map(|e| {
            if let Event::Page(p) = e {
                Some(p.index)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(pages, vec![1, 2], "page 2 reverts, page 3 changes");
}

#[test]
fn bad_requests_are_refused_and_the_connection_stays_usable() {
    let host = HostProc::start("bad");
    let root = project("bad", DOC);
    let mut c = host.connect();
    c.hello(3, 2);
    for (body, what) in [
        (r#"{"root":"/tmp","main":"main.typ"}"#.to_string(), "no id"),
        (r#"{"id":1,"main":"main.typ"}"#.to_string(), "no root"),
        (
            r#"{"id":2,"root":"relative","main":"main.typ"}"#.to_string(),
            "relative root",
        ),
        (
            compile_json(3, &root, "../main.typ", ""),
            "main outside root",
        ),
        (
            compile_json(
                4,
                &root,
                "main.typ",
                r#""edits":[{"path":"main.typ","offset":100000,"delete":1,"insert":""}]"#,
            ),
            "edit out of range",
        ),
        (
            compile_json(
                5,
                &root,
                "main.typ",
                r#""buffers":[{"path":"../x.typ","text":""}]"#,
            ),
            "buffer outside root",
        ),
    ] {
        c.send(kind::COMPILE, &body);
        let (k, b) = c.frame().unwrap();
        assert_eq!(k, kind::ERROR, "{what}");
        assert_eq!(json_of(&b).str_field("code"), Some("request"), "{what}");
    }
    c.send(kind::COMPILE, &compile_json(6, &root, "main.typ", ""));
    let Event::Done(d) = events(&c.until_done()).pop().unwrap() else {
        panic!()
    };
    assert_eq!(d.str_field("status"), Some("ok"));
}

#[test]
fn typst_errors_are_diagnostics_not_protocol_errors() {
    let host = HostProc::start("diag");
    let root = project("diag", "Some text.\n\n#let x = (1, 2\n");
    let mut c = host.connect();
    c.hello(3, 2);
    c.send(kind::COMPILE, &compile_json(1, &root, "main.typ", ""));
    let evs = events(&c.until_done());
    let diags: Vec<&Json> = evs
        .iter()
        .filter_map(|e| {
            if let Event::Diagnostic(j) = e {
                Some(j)
            } else {
                None
            }
        })
        .collect();
    assert!(!diags.is_empty());
    let d = diags[0];
    assert_eq!(d.str_field("severity"), Some("error"));
    assert_eq!(
        d.str_field("file"),
        Some(root.join("main.typ").to_str().unwrap())
    );
    assert_eq!(d.int_field("line"), Some(3));
    assert!(d.int_field("column").is_some());
    let Event::Done(done) = evs.last().unwrap() else {
        panic!()
    };
    assert_eq!(
        (done.str_field("status"), done.int_field("pages")),
        (Some("error"), Some(0))
    );
    assert!(
        !evs.iter().any(|e| matches!(e, Event::Page(_))),
        "a failed Typst compile yields no document"
    );
}

#[test]
fn files_outside_the_root_and_packages_are_refused() {
    let host = HostProc::start("confine");
    let root = project("confine", "#read(\"../secret.txt\")\n");
    std::fs::write(
        root.parent().unwrap().join("secret.txt"),
        "TOP-SECRET-CONTENT",
    )
    .unwrap();
    std::os::unix::fs::symlink(
        root.parent().unwrap().join("secret.txt"),
        root.join("link.txt"),
    )
    .unwrap();
    let mut c = host.connect();
    c.hello(3, 2);
    for (i, src) in [
        "#read(\"../secret.txt\")\n",
        "#read(\"link.txt\")\n",
        "#import \"@preview/example:0.1.0\": *\n",
    ]
    .iter()
    .enumerate()
    {
        std::fs::write(root.join("main.typ"), src).unwrap();
        c.send(
            kind::COMPILE,
            &compile_json(i as i64 + 1, &root, "main.typ", ""),
        );
        let evs = events(&c.until_done());
        let Event::Done(done) = evs.last().unwrap() else {
            panic!()
        };
        assert_eq!(done.str_field("status"), Some("error"), "{src}");
        let msgs: Vec<String> = evs
            .iter()
            .filter_map(|e| {
                if let Event::Diagnostic(j) = e {
                    j.str_field("message").map(String::from)
                } else {
                    None
                }
            })
            .collect();
        assert!(
            !msgs.is_empty() && !msgs.iter().any(|m| m.contains("TOP-SECRET")),
            "{msgs:?}"
        );
        if src.contains("@preview") {
            assert!(msgs.iter().any(|m| m.contains("package")), "{msgs:?}");
        }
    }
}

#[test]
fn unsupported_constructs_flag_the_page_incomplete() {
    let host = HostProc::start("unsup");
    let root = project("unsup", &fixture("unsupported.typ"));
    let mut c = host.connect();
    c.hello(3, 2);
    c.send(
        kind::COMPILE,
        &compile_json(1, &root, "main.typ", r#""font_formats":["opentype"]"#),
    );
    let evs = events(&c.until_done());
    let pages: Vec<_> = evs
        .iter()
        .filter_map(|e| {
            if let Event::Page(p) = e {
                Some(p)
            } else {
                None
            }
        })
        .collect();
    assert!(!pages.is_empty());
    let all: Vec<&String> = pages.iter().flat_map(|p| &p.unsupported).collect();
    for p in &pages {
        assert_eq!(
            p.flags & 1,
            1,
            "every page of the fixture uses something v3 cannot draw"
        );
        assert!(p.items.iter().any(|i| matches!(i, Item::Unsupported(_))));
    }
    for what in ["gradient", "alpha", "stroked text"] {
        assert!(
            all.iter().any(|u| u.contains(what)),
            "{what} not flagged: {all:?}"
        );
    }
    let Event::Done(d) = evs.last().unwrap() else {
        panic!()
    };
    assert_eq!(
        d.str_field("status"),
        Some("ok"),
        "INCOMPLETE is not an error: DONE.pdf is exact"
    );
    assert!(
        d.str_field("pdf").is_some(),
        "an INCOMPLETE page needs DONE.pdf"
    );
}

#[test]
fn every_compile_gets_one_done_in_order_and_bye_closes() {
    let host = HostProc::start("order");
    let root = project("order", DOC);
    let mut c = host.connect();
    c.hello(3, 2);
    // Two compiles back to back: the first may be superseded (DONE
    // cancelled) or run; either way each gets exactly one DONE, in order.
    let a = compile_json(1, &root, "main.typ", "");
    let b = compile_json(2, &root, "main.typ", "");
    c.send(kind::COMPILE, &a);
    c.send(kind::COMPILE, &b);
    let mut dones = vec![];
    while dones.len() < 2 {
        let (k, body) = c.frame().unwrap();
        if k == kind::DONE {
            let j = json_of(&body);
            dones.push((
                j.int_field("id").unwrap(),
                j.str_field("status").unwrap().to_string(),
            ));
        }
    }
    assert_eq!(dones[0].0, 1);
    assert!(dones[0].1 == "ok" || dones[0].1 == "cancelled");
    assert_eq!(dones[1], (2, "ok".to_string()));
    c.send(kind::BYE, "{}");
    assert!(c.frame().is_none(), "BYE closes the connection");
    // The host serves the next connection.
    let mut c = host.connect();
    assert_eq!(c.hello(3, 2).0, kind::HELLO);
}

/// Review fix: a buffer or edit never writes through a symlink, dangling or
/// not, final component or directory.
#[test]
fn buffers_and_edits_never_write_through_symlinks() {
    let host = HostProc::start("wsym");
    let root = project("wsym", DOC);
    let outside = root.parent().unwrap().join("wsym-outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("victim.typ"), "keep").unwrap();
    std::os::unix::fs::symlink(outside.join("escaped.txt"), root.join("notes.typ")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("out")).unwrap();
    std::os::unix::fs::symlink(outside.join("victim.typ"), root.join("victim.typ")).unwrap();
    let mut c = host.connect();
    c.hello(3, 2);
    let cases = [
        r#""buffers":[{"path":"notes.typ","text":"escaped"}]"#,
        r#""buffers":[{"path":"out/new.typ","text":"escaped"}]"#,
        r#""buffers":[{"path":"victim.typ","text":"overwritten"}]"#,
        r#""edits":[{"path":"victim.typ","offset":0,"delete":1,"insert":"X"}]"#,
        r#""edits":[{"path":"out/victim.typ","offset":0,"delete":1,"insert":"X"}]"#,
    ];
    for (i, extra) in cases.iter().enumerate() {
        c.send(
            kind::COMPILE,
            &compile_json(i as i64 + 1, &root, "main.typ", extra),
        );
        let (k, b) = c.frame().unwrap();
        assert_eq!(k, kind::ERROR, "{extra} was not refused");
        assert_eq!(json_of(&b).str_field("code"), Some("request"));
    }
    assert!(
        !outside.join("escaped.txt").exists(),
        "wrote through a dangling symlink"
    );
    assert!(
        !outside.join("new.typ").exists(),
        "wrote through a symlinked directory"
    );
    assert_eq!(
        std::fs::read_to_string(outside.join("victim.typ")).unwrap(),
        "keep"
    );
    // A plain buffer inside the root still works on the same connection.
    let ok = r#""buffers":[{"path":"main.typ","text":"Fine."}]"#;
    c.send(kind::COMPILE, &compile_json(9, &root, "main.typ", ok));
    let Event::Done(d) = events(&c.until_done()).pop().unwrap() else {
        panic!()
    };
    assert_eq!(d.str_field("status"), Some("ok"));
}

/// Review fix: one font program, however many variation instances use it.
/// 400 `wght` values used to send 400 copies of the program (135 MB).
#[test]
fn variation_instances_share_one_program() {
    let host = HostProc::start("vars");
    let src = "#set page(width: 20cm, height: auto, margin: 5mm)\n#set text(font: \"Libertinus Serif\")\n#for i in range(400) [#text(variations: (wght: 100 + i))[a] ]\n";
    let root = project("vars", src);
    let mut c = host.connect();
    c.hello(3, 2);
    c.send(
        kind::COMPILE,
        &compile_json(1, &root, "main.typ", r#""font_formats":["opentype"]"#),
    );
    let frames = c.until_done();
    let Event::Done(d) = events(&frames).pop().unwrap() else {
        panic!()
    };
    assert_eq!(d.str_field("status"), Some("ok"), "{d}");
    let fonts: Vec<_> = events(&frames)
        .into_iter()
        .filter_map(|e| {
            if let Event::Font(f) = e {
                Some(f)
            } else {
                None
            }
        })
        .collect();
    let font_bytes: usize = frames
        .iter()
        .filter(|(k, _)| *k == kind::FONT)
        .map(|(_, b)| b.len())
        .sum();
    let with_program: Vec<_> = fonts.iter().filter(|f| !f.program.is_empty()).collect();
    let mut shas: Vec<&str> = fonts
        .iter()
        .map(|f| f.info.str_field("program_sha256").unwrap())
        .collect();
    shas.sort();
    shas.dedup();
    assert!(
        fonts.len() >= 400,
        "one FONT per variation instance: {}",
        fonts.len()
    );
    assert_eq!(
        with_program.len(),
        shas.len(),
        "each program is sent exactly once"
    );
    let programs: usize = with_program.iter().map(|f| f.program.len()).sum();
    assert!(
        font_bytes < programs + fonts.len() * 2048,
        "{font_bytes} bytes of FONT frames for {programs} bytes of programs"
    );
    // Every program-less instance names the frame that carried its program.
    let carriers: std::collections::HashMap<&str, u16> = with_program
        .iter()
        .map(|f| (f.info.str_field("program_sha256").unwrap(), f.id))
        .collect();
    for f in fonts.iter().filter(|f| f.program.is_empty()) {
        let from = f.info.int_field("program_from").expect("program_from") as u16;
        assert_eq!(carriers[f.info.str_field("program_sha256").unwrap()], from);
    }
    eprintln!(
        "400 variations: {} FONT frames, {} programs, {font_bytes} bytes",
        fonts.len(),
        with_program.len()
    );
}
