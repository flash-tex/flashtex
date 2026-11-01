//! `payload.date` on the runtime-v1 compile request.
//!
//! This pipeline never reads the wall clock — `docs/contracts/runtime-v1.md`
//! requires byte-identical output for byte-identical input, and a renderer that
//! reads the clock is not a function of its inputs. The caller reads it and
//! sends the answer (`protocol/proposals/runtime-v1-request-date.md`).
//!
//! What is verified here is the **worker's half of the contract**: the field is
//! accepted, validated, and refused when malformed, and a request that omits it
//! is byte-identical to one that pins the epoch.
//!
//! The substitution into `\today` is behind the `request-date` cargo feature,
//! which is **on by default** and whose precondition — the compiler's
//! `parser::parse_project_with` — is met, because this crate links
//! `crates/compiler` live rather than a frozen mirror. The gap the feature
//! guarded is closed; `a_supplied_date_is_accepted_but_does_not_yet_reach_today`
//! still asserts both arms so that a `--no-default-features` build is held to
//! the behaviour it actually has.

use flashtex_render_pipeline::date::TodayDate;
use flashtex_render_pipeline::{FontSet, RenderOptions};

fn compile_line(text: &str, date: Option<&str>) -> String {
    let doc = format!(
        r#"{{"path":"main.tex","text":{}}}"#,
        json_string(text)
    );
    let date_field = match date {
        Some(d) => format!(r#","date":{}"#, json_string(d)),
        None => String::new(),
    };
    format!(
        r#"{{"protocol_version":1,"id":"d1","type":"compile","payload":{{"project_id":"p","revision":1,"entry_path":"main.tex","documents":[{doc}]{date_field}}}}}"#
    )
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn handle(text: &str, date: Option<&str>) -> String {
    let fonts = FontSet::with_default_dirs(&[]);
    let options = RenderOptions::default();
    flashtex_render_pipeline::protocol::handle_line(
        &compile_line(text, date),
        &fonts,
        &options,
        None,
    )
    .line
}

fn body(text: &str) -> String {
    format!("\\documentclass{{article}}\\begin{{document}}{text}\\end{{document}}")
}

/// `\today` reached through `\date{\today}\maketitle` rather than written
/// bare in the body.
///
/// Not a stylistic choice originally: bare `\today` in body text had **no
/// dispatch arm** in the frozen `vendor/compiler` this crate used to link, so
/// it fell to `P::unsupported`, whose
/// `debug_assert!(!BUILT_INS.contains(&name))` fired and aborted a debug build.
/// PR #220 fixed that in `crates/compiler`, which is now linked live, so the
/// hazard is gone; the title route is kept because it exercises the same
/// substitution site and every committed expectation here was recorded through
/// it.
fn today_via_title() -> String {
    body("\\title{T}\\author{A}\\date{\\today}\\maketitle")
}

#[test]
fn an_absent_date_is_byte_identical_to_an_explicit_epoch() {
    let src = today_via_title();
    assert_eq!(
        handle(&src, None),
        handle(&src, Some("1970-01-01")),
        "an explicit epoch must be indistinguishable from an absent date, \
         so harnesses can pin it without changing committed reference data"
    );
}

#[test]
fn identical_requests_are_byte_identical() {
    let src = today_via_title();
    assert_eq!(handle(&src, Some("2026-09-13")), handle(&src, Some("2026-09-13")));
}

#[test]
fn a_malformed_date_fails_the_compile_instead_of_being_ignored() {
    let src = body("hello");
    for bad in [
        "2026-9-13",
        "13-09-2026",
        "2026/09/13",
        "2026-09-13T12:00:00Z",
        "2026-02-29",
        "",
        "tomorrow",
    ] {
        let out = handle(&src, Some(bad));
        assert!(
            out.contains("\"status\":\"failed\"") || out.contains("\"status\": \"failed\""),
            "{bad:?} was accepted: {out}"
        );
        assert!(out.contains("date"), "{bad:?} produced no date diagnostic: {out}");
    }
}

#[test]
fn a_well_formed_date_is_accepted() {
    let out = handle(&body("hello"), Some("2026-09-13"));
    assert!(
        !out.contains("\"status\":\"failed\""),
        "a valid date was refused: {out}"
    );
}

/// Both arms of the `request-date` feature. With it on -- the default, and the
/// only configuration the crate still builds in -- the request's date reaches
/// `\today`. With it off the date is validated and carried but `\today`
/// renders the epoch. The test asserts whichever arm the build selected, so
/// neither can rot unnoticed.
#[test]
fn a_supplied_date_is_accepted_but_does_not_yet_reach_today() {
    let out = handle(&today_via_title(), Some("2026-09-13"));
    if cfg!(feature = "request-date") {
        assert!(
            out.contains("September"),
            "request-date is on, so the request's date must reach \\today: {out}"
        );
    } else {
        assert!(
            out.contains("January") || out.contains("1970"),
            "request-date is off, so \\today still renders the epoch: {out}"
        );
    }
}

#[test]
fn the_pipelines_date_type_formats_like_the_latex_kernel() {
    // pdflatex 3.141592653-2.6-1.40.27 (TeX Live 2025): `\meaning\today` is
    // `\ifcase\month\or January\or ... \fi \space\number\day, \number\year`.
    assert_eq!(TodayDate::EPOCH.latex_today(), "January 1, 1970");
    assert_eq!(
        TodayDate::parse_iso("2026-09-13").unwrap().latex_today(),
        "September 13, 2026"
    );
}
