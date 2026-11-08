//! Missing coverage: `canonical` (no tests at all), `decode_event`'s kinds,
//! `CompileRequest`'s optional fields, JSON parser failures, frame
//! fail-closed paths, `Font` decode errors, and `Page::decode` reference
//! validation. Expected hashes below are independent oracles:
//! `shasum -a 256` (`"{}"`) and the FIPS 180-4 vectors (`""`, `"abc"`).

use flashtex_display_list::canonical::canonical;
use flashtex_display_list::client::{decode_event, CompileRequest, Edit, Event};
use flashtex_display_list::frame::{complete_frames, read_frame, write_frame, Cursor, MAX_FRAME};
use flashtex_display_list::json::{obj, s, Json};
use flashtex_display_list::kind;
use flashtex_display_list::page::{alternate, Color, ColorSpace, Item, Page, Path, StreamKind};
use flashtex_display_list::resource::{Font, ImageData, Sources};

const SHA_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const SHA_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const SHA_CURLY: &str = "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";

#[test]
fn canonical_minimal_page_is_exact() {
    let body = Page::new(StreamKind::Page, 0).encode();
    let zeros64 = "0".repeat(64);
    let expect = format!(
        "page 0 0 0 0 0 0 0 0 0 0 0 0 0 0 \
         0000000000000000 0000000000000000 0000000000000000 0000000000000000 {zeros64}\n"
    );
    assert_eq!(canonical(kind::PAGE, body.clone()).unwrap(), expect);
    assert!(canonical(kind::FORM, body).unwrap().starts_with("form "));
}

#[test]
fn canonical_font_sources_and_imagedata() {
    let font = Font {
        id: 1,
        key: [0; 32],
        info: obj([("format", s("none"))]),
        program: vec![],
    };
    assert_eq!(
        canonical(kind::FONT, font.encode()).unwrap(),
        format!("font 1 {} 0 {SHA_EMPTY} none\n", "0".repeat(64))
    );
    // With an encoding the `enc` line names code 0 and dashes the gaps.
    let enc = Font {
        id: 2,
        key: [0; 32],
        info: obj([
            ("format", s("type1")),
            ("encoding", Json::Arr(vec![s("A"), Json::Int(5)])),
        ]),
        program: vec![],
    };
    let text = canonical(kind::FONT, enc.encode()).unwrap();
    let mut lines = text.lines();
    assert!(lines.next().unwrap().starts_with("font 2 "));
    assert!(lines.next().unwrap().starts_with("enc A - "));
    assert_eq!(text.lines().count(), 2);

    let src = Sources {
        files: vec![(1, String::new())],
        spans: vec![(1, 1, 12)],
    };
    assert_eq!(
        canonical(kind::SOURCES, src.to_json().to_string().into_bytes()).unwrap(),
        "sources\nfile 1 -\nspan 1 1 12\n"
    );
    let data = ImageData {
        id: 3,
        parts: vec![b"abc".to_vec(), vec![]],
    };
    assert_eq!(
        canonical(kind::IMAGE_DATA, data.encode()).unwrap(),
        format!("imagedata 3 2 {SHA_ABC} {SHA_EMPTY}\n")
    );
}

#[test]
fn canonical_json_other_and_decode_errors() {
    assert_eq!(
        canonical(kind::DONE, b"{}".to_vec()).unwrap(),
        format!("json done {SHA_CURLY}\n")
    );
    assert_eq!(canonical(0xFF, vec![0; 5]).unwrap(), "other 255 5\n");
    assert!(canonical(kind::PAGE, b"short".to_vec()).is_err());
    assert!(canonical(kind::FONT, b"x".to_vec()).is_err());
    assert!(canonical(kind::DIAG, br#"{"no":"message"}"#.to_vec()).is_err());
}

fn event_name(e: &Event) -> &'static str {
    match e {
        Event::Started(_) => "started",
        Event::Image(_) => "image",
        Event::Diagnostic(_) => "diagnostic",
        Event::Done(_) => "done",
        Event::Error(_) => "error",
        Event::Pages(_) => "pages",
        Event::Tool(_) => "tool",
        Event::Diag(_) => "diag",
        Event::Resolved(_) => "resolved",
        Event::Located(_) => "located",
        Event::Package(_) => "package",
        Event::Profile(_) => "profile",
        Event::Form(_) => "form",
        Event::ImageData(_) => "image-data",
        _ => "other",
    }
}

#[test]
fn decode_event_covers_every_kind() {
    for (k, name) in [
        (kind::STARTED, "started"),
        (kind::IMAGE, "image"),
        (kind::DIAGNOSTIC, "diagnostic"),
        (kind::DONE, "done"),
        (kind::ERROR, "error"),
        (kind::PAGES, "pages"),
        (kind::TOOL, "tool"),
        (kind::RESOLVED, "resolved"),
        (kind::LOCATED, "located"),
        (kind::PACKAGE, "package"),
        (kind::PROFILE, "profile"),
    ] {
        let e = decode_event(k, b"{}".to_vec()).unwrap();
        assert_eq!(event_name(&e), name);
    }
    let form = Page::new(StreamKind::Form, 9).encode();
    assert_eq!(event_name(&decode_event(kind::FORM, form).unwrap()), "form");
    let data = ImageData {
        id: 1,
        parts: vec![vec![7]],
    }
    .encode();
    match decode_event(kind::IMAGE_DATA, data).unwrap() {
        Event::ImageData(d) => assert_eq!((d.id, d.parts), (1, vec![vec![7]])),
        e => panic!("wrong variant: {}", event_name(&e)),
    }
    match decode_event(kind::DIAG, br#"{"message":"x"}"#.to_vec()).unwrap() {
        Event::Diag(d) => assert_eq!(d.message, "x"),
        e => panic!("wrong variant: {}", event_name(&e)),
    }
    assert!(decode_event(kind::PAGE, b"junk".to_vec()).is_err());
    assert!(decode_event(kind::FONT, b"junk".to_vec()).is_err());
    assert!(decode_event(kind::SOURCES, br#"{"files":[1]}"#.to_vec()).is_err());
    assert!(decode_event(kind::DIAG, b"{}".to_vec()).is_err());
    // Unknown kinds pass through untouched for a later minor version.
    match decode_event(0x7F, vec![1, 2, 3]).unwrap() {
        Event::Other(k, b) => assert_eq!((k, b), (0x7F, vec![1, 2, 3])),
        e => panic!("wrong variant: {}", event_name(&e)),
    }
}

#[test]
fn compile_request_json_defaults_and_options() {
    let d = CompileRequest::new(1, "/p", "main.tex").to_json();
    assert_eq!(d.int_field("id"), Some(1));
    assert_eq!(d.str_field("format"), Some("pdflatex"));
    for absent in [
        "output_dir",
        "jobname",
        "have_fonts",
        "incremental",
        "viewport",
        "external_tools",
        "includeonly",
        "font_formats",
        "buffers",
        "edits",
        "export",
    ] {
        assert!(d.get(absent).is_none(), "{absent} sent by default");
    }
    let mut r = CompileRequest::new(2, "/p", "m.tex");
    r.output_dir = Some("out".into());
    r.jobname = Some("job".into());
    r.have_fonts = vec!["a".into(), "b".into()];
    r.incremental = true;
    r.viewport = Some(7);
    r.external_tools = Some("auto".into());
    r.includeonly = Some(vec!["ch1".into()]);
    r.font_formats = vec!["type1".into()];
    r.buffers = vec![("f.tex".into(), "hi".into())];
    r.edits = vec![Edit {
        path: "e.tex".into(),
        offset: 3,
        delete: 4,
        insert: "x".into(),
    }];
    r.export = true;
    let j = r.to_json();
    assert_eq!(j.str_field("output_dir"), Some("out"));
    assert_eq!(j.str_field("jobname"), Some("job"));
    assert_eq!(
        j.get("have_fonts").and_then(Json::as_array).unwrap().len(),
        2
    );
    assert_eq!(j.get("incremental").and_then(Json::as_bool), Some(true));
    assert_eq!(j.int_field("viewport"), Some(7));
    assert_eq!(j.str_field("external_tools"), Some("auto"));
    assert_eq!(
        j.get("includeonly").and_then(Json::as_array).unwrap().len(),
        1
    );
    assert_eq!(
        j.get("font_formats")
            .and_then(Json::as_array)
            .unwrap()
            .len(),
        1
    );
    let b = &j.get("buffers").and_then(Json::as_array).unwrap()[0];
    assert_eq!(
        (b.str_field("path"), b.str_field("text")),
        (Some("f.tex"), Some("hi"))
    );
    let e = &j.get("edits").and_then(Json::as_array).unwrap()[0];
    assert_eq!(
        (
            e.str_field("path"),
            e.int_field("offset"),
            e.int_field("delete")
        ),
        (Some("e.tex"), Some(3), Some(4))
    );
    assert_eq!(j.get("export").and_then(Json::as_bool), Some(true));
}

#[test]
fn json_parser_errors_and_edge_values() {
    for bad in [
        "",
        "  ",
        "nul",
        "tru",
        "[1,",
        "{\"a\"}",
        "{\"a\":1,}",
        "\"\\q\"",
        "\"abc",
        "\"a\x01b\"",
        "\"\\u12\"",
        "\"\\u12xy\"",
        "-",
        "1e",
        "[1] x",
        "{} {}",
    ] {
        assert!(Json::parse(bad).is_err(), "{bad:?} should fail");
    }
    let deep = "[".repeat(200);
    assert!(Json::parse(&deep).unwrap_err().contains("nesting too deep"));
    assert_eq!(
        Json::parse(r#""\ud83d\ude00""#).unwrap().as_str(),
        Some("😀")
    );
    assert_eq!(
        Json::parse(r#""\/\b\f""#).unwrap().as_str(),
        Some("/\u{8}\u{c}")
    );
    assert_eq!(Json::parse("1e3").unwrap().as_i64(), Some(1000));
    assert_eq!(Json::parse("1.5").unwrap().as_i64(), None);
    assert!(matches!(
        Json::parse("9223372036854775808").unwrap(),
        Json::Num(_)
    ));
    assert_eq!(Json::Num(f64::NAN).to_string(), "null");
    assert_eq!(Json::Num(f64::INFINITY).to_string(), "null");
    assert_eq!(Json::Int(-3).to_string(), "-3");
    let mut e = String::new();
    flashtex_display_list::json::write_str(&mut e, "\x01\"\\\n");
    assert_eq!(e, "\"\\u0001\\\"\\\\\\n\"");
}

#[test]
fn frames_fail_closed() {
    let huge = vec![0u8; MAX_FRAME as usize];
    let err = write_frame(&mut Vec::new(), 1, &huge).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    for len in [0u32, MAX_FRAME + 1] {
        let mut head = len.to_le_bytes().to_vec();
        head.push(1);
        let mut r = &head[..];
        let err = read_frame(&mut r).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "len {len}");
        assert!(complete_frames(&head).is_err());
    }
    let mut short = 10u32.to_le_bytes().to_vec();
    short.push(1);
    short.extend_from_slice(b"abc");
    let mut r = &short[..];
    assert_eq!(
        read_frame(&mut r).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
    assert_eq!(complete_frames(&[]).unwrap(), 0);
    assert_eq!(complete_frames(&[1, 2, 3]).unwrap(), 0);
    let mut c = Cursor::new(&[1, 2]);
    assert_eq!(c.left(), 2);
    assert!(c.u32().is_err());
    assert!(c.f64().is_err());
    let mut c = Cursor::new(&[100, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert!(c.count(4).unwrap_err().contains("exceeds"));
    let mut c = Cursor::new(&[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!((c.count(4).unwrap(), c.left()), (2, 8));
}

#[test]
fn font_decode_errors_and_glyph_names() {
    let font = Font {
        id: 41,
        key: [7; 32],
        info: obj([
            ("format", s("type1")),
            ("encoding", Json::Arr(vec![s("A"), Json::Int(5)])),
        ]),
        program: vec![1, 2, 3],
    };
    assert_eq!(Font::decode(&font.encode()).unwrap(), font);
    let mut bad_id = font.encode();
    bad_id[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Font::decode(&bad_id).unwrap_err().contains("out of range"));
    assert!(Font::decode(&font.encode()[..10]).is_err());
    let mut bad_info = font.encode();
    bad_info[40] = 0xFF; // inside the JSON info: not UTF-8, not JSON
    assert!(Font::decode(&bad_info).is_err());
    assert_eq!(font.glyph_name(0), Some("A"));
    assert_eq!(font.glyph_name(1), None);
    assert_eq!(font.glyph_name(200), None);
    let plain = Font {
        info: obj([("format", s("none"))]),
        ..font.clone()
    };
    assert_eq!(plain.glyph_name(0), None);
}

#[test]
fn page_decode_dangling_refs_fail_closed() {
    // An item or path naming an entry that does not exist is refused,
    // naming the dangling reference.
    let err = |items: Vec<Item>| -> String {
        let mut p = Page::new(StreamKind::Page, 0);
        p.items = items;
        Page::decode(StreamKind::Page, &p.encode()).unwrap_err()
    };
    assert_eq!(err(vec![Item::Path(0)]), "item names path 0 of 0");
    assert_eq!(err(vec![Item::Clip(1)]), "item names path 1 of 0");
    assert_eq!(err(vec![Item::Matrix(1)]), "item names matrix 1 of 0");
    assert_eq!(
        err(vec![Item::Image { id: 7, matrix: 2 }]),
        "item names matrix 2 of 0"
    );
    assert_eq!(
        err(vec![Item::Form { id: 7, matrix: 1 }]),
        "item names matrix 1 of 0"
    );
    assert_eq!(
        err(vec![Item::Unsupported(0)]),
        "item names unsupported entry 0"
    );
    let mut p = Page::new(StreamKind::Page, 0);
    p.paths.push(Path {
        paint: 0,
        matrix: 1,
        stroke: None,
        segs: vec![],
    });
    assert_eq!(
        Page::decode(StreamKind::Page, &p.encode()).unwrap_err(),
        "path names matrix 1"
    );
    // Boundary-valid references still decode: matrix 0 is the identity,
    // path 0 of one path and matrix 1 of one matrix resolve.
    let mut p = Page::new(StreamKind::Page, 0);
    p.matrices.push([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    p.paths.push(Path {
        paint: 0,
        matrix: 1,
        stroke: None,
        segs: vec![],
    });
    p.unsupported.push("sh".into());
    p.items = vec![
        Item::Matrix(1),
        Item::Matrix(0),
        Item::Path(0),
        Item::Clip(0),
        Item::Image { id: 1, matrix: 1 },
        Item::Form { id: 2, matrix: 0 },
        Item::Unsupported(0),
    ];
    assert_eq!(Page::decode(StreamKind::Page, &p.encode()).unwrap(), p);
}

#[test]
fn page_decode_bad_separation_alternates_fail_closed() {
    let sep = |alternate: u32, n: usize| ColorSpace::Separation {
        name: "spot".into(),
        alternate,
        c0: vec![0.0; n],
        c1: vec![1.0; n],
        e: 1.0,
    };
    let err = |spaces: Vec<ColorSpace>| -> String {
        let mut p = Page::new(StreamKind::Page, 0);
        p.colorspaces = spaces;
        Page::decode(StreamKind::Page, &p.encode()).unwrap_err()
    };
    // Not a space at all.
    assert_eq!(
        err(vec![sep(9, 1)]),
        "Separation alternate 0x9 is not a space"
    );
    // A later entry: the alternate must come before the Separation.
    assert_eq!(
        err(vec![
            sep(2, 1),
            ColorSpace::Icc {
                n: 1,
                profile: vec![]
            }
        ]),
        "Separation alternate 0x2 is not a space"
    );
    // A Separation is not a valid alternate; only Device or an earlier ICC.
    assert_eq!(
        err(vec![sep(alternate::DEVICE_GRAY, 1), sep(1, 1)]),
        "Separation alternate 0x1 is not a space"
    );
    // The tint transform must have the alternate's component count.
    assert_eq!(
        err(vec![sep(alternate::DEVICE_GRAY, 3)]),
        "Separation tint of 3 components for 1"
    );
    // A Device alternate with the matching tint decodes.
    let mut p = Page::new(StreamKind::Page, 0);
    p.colorspaces = vec![sep(alternate::DEVICE_RGB, 3)];
    p.items = vec![Item::FillColorCs {
        cs: 1,
        color: Color(vec![0.5]),
    }];
    assert_eq!(Page::decode(StreamKind::Page, &p.encode()).unwrap(), p);
}
