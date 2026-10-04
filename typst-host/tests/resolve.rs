//! On-demand source mapping through the host process (display-list-v3
//! spec §11.6, E8): RESOLVE (a click on a page → file, line, column, byte)
//! and LOCATE (a source position → where it is drawn), on a page with
//! bleed so that page space and Typst's frame differ; and the stale and
//! unknown-file answers.

mod common;

use common::*;
use flashtex_display_list::json::Json;
use flashtex_display_list::kind;
use flashtex_display_list::page::{Item, Page, StreamKind};

const SRC: &str = "#set page(width: 200pt, height: 120pt, margin: 20pt, bleed: 7pt)\n\
                   #set text(size: 12pt)\n\
                   Hello world\n\
                   \n\
                   Second paragraph here.\n";

fn int(j: &Json, k: &str) -> i64 {
    match j.get(k) {
        Some(Json::Int(v)) => *v,
        v => panic!("{k}: {v:?} in {j}"),
    }
}

#[test]
fn resolve_and_locate_answer_from_the_shown_pages() {
    let host = HostProc::start("resolve");
    let root = project("resolve", SRC);
    let main = root.join("main.typ").to_string_lossy().into_owned();
    let mut c = host.connect();
    let (_, hello) = c.hello_caps(3, 3, &[]);
    assert!(hello.to_string().contains("\"resolve-v1\""), "{hello}");
    c.send(
        kind::COMPILE,
        &compile_json(1, &root, "main.typ", r#""font_formats":["opentype"]"#),
    );
    let frames = c.until_done();
    let page = frames
        .iter()
        .find(|(k, _)| *k == kind::PAGE)
        .map(|(_, b)| Page::decode(StreamKind::Page, b).unwrap())
        .unwrap();
    // The glyphs, in painting order: "Hello", "world", "Second", ...
    let glyphs: Vec<(i32, i32)> = page
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Glyph { x, y, .. } => Some((*x, *y)),
            _ => None,
        })
        .collect();
    assert!(glyphs.len() > 25);
    let sp = |pt: f64| (pt * 65_781.76) as i64;
    let resolve = |c: &mut Raw, id: i64, compile: i64, (x, y): (i32, i32)| {
        // Inside the glyph: 1 pt right of its origin, 3 pt above the baseline.
        c.send(
            kind::RESOLVE,
            &format!(
                r#"{{"id":{id},"compile":{compile},"page":0,"x":{},"y":{}}}"#,
                x as i64 + sp(1.0),
                y as i64 - sp(3.0)
            ),
        );
        let (k, b) = c.frame().unwrap();
        assert_eq!(k, kind::RESOLVED);
        json_of(&b)
    };
    // The second glyph of "Hello": line 3 of the file, byte column 1.
    let r = resolve(&mut c, 10, 1, glyphs[1]);
    assert_eq!(int(&r, "id"), 10);
    assert_eq!(r.str_field("file"), Some(main.as_str()), "{r}");
    assert_eq!(int(&r, "line"), 3, "{r}");
    let hello_at = SRC.find("Hello").unwrap() as i64;
    assert!((hello_at..hello_at + 5).contains(&int(&r, "byte")), "{r}");
    assert_eq!(int(&r, "column"), int(&r, "byte") - hello_at, "{r}");
    // A glyph of "Second": line 5.
    let second = glyphs.len() - 20;
    let r = resolve(&mut c, 11, 1, glyphs[second]);
    assert_eq!(int(&r, "line"), 5, "{r}");
    // Far outside the text: nothing.
    let r = resolve(&mut c, 12, 1, (sp(1.0) as i32, sp(4.0) as i32));
    assert_eq!(r.get("none").and_then(Json::as_bool), Some(true), "{r}");
    // Another compile than the shown one: stale.
    let r = resolve(&mut c, 13, 7, glyphs[1]);
    assert_eq!(r.get("stale").and_then(Json::as_bool), Some(true), "{r}");

    // LOCATE inside "paragraph": a position on page 0 among the glyphs of
    // the second line, near where they are drawn.
    let byte = SRC.find("paragraph").unwrap() + 3;
    c.send(
        kind::LOCATE,
        &format!(r#"{{"id":20,"compile":1,"file":{main:?},"byte":{byte}}}"#),
    );
    let (k, b) = c.frame().unwrap();
    assert_eq!(k, kind::LOCATED);
    let l = json_of(&b);
    let pos = match l.get("positions") {
        Some(Json::Arr(a)) => a.clone(),
        v => panic!("{v:?}"),
    };
    assert!(!pos.is_empty(), "{l}");
    let p = match &pos[0] {
        Json::Arr(p) => p
            .iter()
            .map(|v| match v {
                Json::Int(i) => *i,
                v => panic!("{v:?}"),
            })
            .collect::<Vec<_>>(),
        v => panic!("{v:?}"),
    };
    assert_eq!(p[0], 0);
    // Typst places a text node at its first glyph's origin (the
    // baseline): that is a GLYPH of the page, bleed included, within the
    // frame-vs-PDF difference (krilla's f32, a few sp).
    let near = glyphs
        .iter()
        .map(|g| ((p[1] - g.0 as i64).abs().max((p[2] - g.1 as i64).abs()), g))
        .min()
        .unwrap();
    assert!(
        near.0 <= 10,
        "{p:?}: the nearest glyph {:?} is {} sp away",
        near.1,
        near.0
    );
    assert!(
        (near.1 .1 - glyphs[second].1).abs() <= 10,
        "not on the second line: {p:?}"
    );
    // A file that is not the project's: nothing.
    c.send(
        kind::LOCATE,
        r#"{"id":21,"compile":1,"file":"/nowhere/else.typ","byte":3}"#,
    );
    let (_, b) = c.frame().unwrap();
    assert_eq!(json_of(&b).get("positions"), Some(&Json::Arr(vec![])));
}
