//! The seeded 1-pass loop and its checks (DESIGN.md §15.3; spec §11.9),
//! through the running host.

mod common;

use common::*;
use flashtex_display_list::json::Json;
use flashtex_display_list::kind;

/// A document whose introspection needs iterations: an outline, numbered
/// headings, references and a page counter.
fn doc(word: &str) -> String {
    let mut s = String::from(
        "#set page(width: 9cm, height: 7cm, margin: 6mm, numbering: \"1\")\n\
         #set heading(numbering: \"1.\")\n#outline()\n",
    );
    for i in 1..=6 {
        s.push_str(&format!(
            "= Section {i} <s{i}>\nSee @s{} on page #context counter(page).at(<s{}>).first(). {word} text.\n\n",
            7 - i,
            7 - i
        ));
    }
    s
}

fn buffer(text: &str) -> String {
    format!(
        r#""incremental":true,"font_formats":["opentype"],"buffers":[{{"path":"main.typ","text":{}}}]"#,
        Json::Str(text.into())
    )
}

fn done_of(frames: &[(u8, Vec<u8>)]) -> Json {
    let (k, b) = frames.last().unwrap();
    assert_eq!(*k, kind::DONE);
    json_of(b)
}

/// Every incremental compile after the first is seeded, takes one layout
/// iteration, and (checked on every compile) equals the standard compile.
#[test]
fn seeded_compiles_equal_the_standard_compile() {
    let host = HostProc::start_with("seed-every", &["--verify", "every"]);
    let root = project("seed-every", &doc("first"));
    let mut c = host.connect();
    c.hello(3, 3);
    c.send(
        kind::COMPILE,
        &compile_json(1, &root, "main.typ", &buffer(&doc("first"))),
    );
    let d = done_of(&c.until_done());
    assert_eq!(d.get("seeded").and_then(Json::as_bool), Some(false), "{d}");
    let mut text = doc("first");
    let mut ones = 0;
    for k in 0..24 {
        // Grow a word, then add a section: the outline and pages move.
        if k == 12 {
            text.push_str("= Added <added>\nSee @added and @s1.\n");
        } else {
            text = text.replacen(" text.", "x text.", 1);
        }
        c.send(
            kind::COMPILE,
            &compile_json(2 + k, &root, "main.typ", &buffer(&text)),
        );
        let d = done_of(&c.until_done());
        assert_eq!(d.str_field("status"), Some("ok"), "{d}");
        assert_eq!(d.get("seeded").and_then(Json::as_bool), Some(true), "{d}");
        assert_eq!(d.get("verified").and_then(Json::as_bool), Some(true), "{d}");
        // Mostly one iteration; a new section moves the outline and the
        // references, and the loop iterates, as the standard one would.
        let it = d.int_field("iterations").unwrap();
        assert!((1..=5).contains(&it), "{d}");
        ones += (it == 1) as usize;
    }
    assert!(
        ones >= 20,
        "only {ones} of 24 seeded compiles took one iteration"
    );
    // An export always uses the standard compile.
    let out = scratch("seed-every-out");
    let extra = format!(
        r#""export":true,"output_dir":{:?},"font_formats":["opentype"]"#,
        out.to_string_lossy()
    );
    c.send(kind::COMPILE, &compile_json(99, &root, "main.typ", &extra));
    let d = done_of(&c.until_done());
    assert_eq!(d.get("seeded").and_then(Json::as_bool), Some(false), "{d}");
}

/// A document with two fixed points: a label that exists only when a query
/// finds it. Seeded with an introspector that has it, the loop keeps it;
/// the standard compile, starting empty, never makes it. The checks find
/// the difference and the standard compile's pages win.
const BEFORE: &str = "#context if query(<a>).len() >= 0 [Seen #metadata(1) <a>] else [Unseen]\n";
const AFTER: &str = "#context if query(<a>).len() > 0 [Seen #metadata(1) <a>] else [Unseen]\n";

fn page_count(frames: &[(u8, Vec<u8>)]) -> usize {
    frames.iter().filter(|(k, _)| *k == kind::PAGE).count()
}

#[test]
fn a_seeded_compile_that_differs_is_replaced_by_the_standard_one() {
    // Checked on every compile: the standard pages are sent, verified false.
    let host = HostProc::start_with("seed-differ", &["--verify", "every"]);
    let root = project("seed-differ", BEFORE);
    let mut c = host.connect();
    c.hello(3, 3);
    c.send(
        kind::COMPILE,
        &compile_json(1, &root, "main.typ", &buffer(BEFORE)),
    );
    c.until_done();
    c.send(
        kind::COMPILE,
        &compile_json(2, &root, "main.typ", &buffer(AFTER)),
    );
    let f = c.until_done();
    let d = done_of(&f);
    assert_eq!(
        d.get("verified").and_then(Json::as_bool),
        Some(false),
        "{d}"
    );
    assert_eq!(d.get("seeded").and_then(Json::as_bool), Some(false), "{d}");
    assert_eq!(
        page_count(&f),
        1,
        "the standard page replaces the shown one"
    );
}

#[test]
fn the_idle_check_resends_what_differs() {
    let host = HostProc::start_with("seed-idle", &["--verify", "idle:100"]);
    let root = project("seed-idle", BEFORE);
    let mut c = host.connect();
    c.hello(3, 3);
    c.send(
        kind::COMPILE,
        &compile_json(1, &root, "main.typ", &buffer(BEFORE)),
    );
    c.until_done();
    c.send(
        kind::COMPILE,
        &compile_json(2, &root, "main.typ", &buffer(AFTER)),
    );
    let d = done_of(&c.until_done());
    assert_eq!(d.get("seeded").and_then(Json::as_bool), Some(true), "{d}");
    assert_eq!(d.get("verified"), Some(&Json::Null), "{d}");
    // Idle: the host compiles again by itself, cause "verify", same id.
    let f = c.until_done();
    let (k, started) = &f[0];
    assert_eq!(*k, kind::STARTED);
    let started = json_of(started);
    assert_eq!(started.str_field("cause"), Some("verify"), "{started}");
    assert_eq!(started.int_field("id"), Some(2));
    let d = done_of(&f);
    assert_eq!(d.str_field("cause"), Some("verify"), "{d}");
    assert_eq!(
        d.get("verified").and_then(Json::as_bool),
        Some(false),
        "{d}"
    );
    assert_eq!(page_count(&f), 1, "the page that differs is sent again");
    // And when the seeded compile was right, the idle check sends nothing.
    let text = format!("{AFTER}More.\n");
    c.send(
        kind::COMPILE,
        &compile_json(3, &root, "main.typ", &buffer(&text)),
    );
    let d = done_of(&c.until_done());
    assert_eq!(d.get("seeded").and_then(Json::as_bool), Some(true), "{d}");
    std::thread::sleep(std::time::Duration::from_millis(600));
    c.send(
        kind::COMPILE,
        &compile_json(4, &root, "main.typ", &buffer(&text)),
    );
    let f = c.until_done();
    let started = json_of(&f[0].1);
    assert_eq!(
        started.int_field("id"),
        Some(4),
        "no follow-up came first: {started}"
    );
}

/// When the standard compile fails where the seeded one succeeded, that is
/// a mismatch too: the idle check's follow-up carries the error.
#[test]
fn a_standard_error_after_a_seeded_success_is_resent() {
    let after =
        "#context if query(<a>).len() > 0 [Seen #metadata(1) <a>] else [#panic(\"no anchor\")]\n";
    let host = HostProc::start_with("seed-err", &["--verify", "idle:100"]);
    let root = project("seed-err", BEFORE);
    let mut c = host.connect();
    c.hello(3, 3);
    c.send(
        kind::COMPILE,
        &compile_json(1, &root, "main.typ", &buffer(BEFORE)),
    );
    c.until_done();
    c.send(
        kind::COMPILE,
        &compile_json(2, &root, "main.typ", &buffer(after)),
    );
    let d = done_of(&c.until_done());
    assert_eq!(d.str_field("status"), Some("ok"), "{d}");
    assert_eq!(d.get("seeded").and_then(Json::as_bool), Some(true), "{d}");
    let f = c.until_done();
    let d = done_of(&f);
    assert_eq!(d.str_field("cause"), Some("verify"), "{d}");
    assert_eq!(d.str_field("status"), Some("error"), "{d}");
    assert!(
        f.iter()
            .any(|(k, b)| *k == kind::DIAGNOSTIC && json_of(b).to_string().contains("no anchor")),
        "the standard compile's error is sent"
    );
}
