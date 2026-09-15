//! `\today` renders the date the *caller* supplied, and nothing else reads a clock.
//!
//! The rule this file pins is the one that makes both halves possible at once:
//! runtime-v1 requires byte-identical output for byte-identical input, and a
//! compiler that reads the wall clock is not a function of its inputs at all —
//! so the clock is read by the caller and the date arrives as an ordinary
//! request field (`payload.date`,
//! `protocol/proposals/runtime-v1-request-date.md`). Output stays a pure
//! function of `(documents, entry_path, layout_capabilities, date)`.
//!
//! Oracle for every rendered string and every vertical measurement below:
//! pdflatex 3.141592653-2.6-1.40.27 (TeX Live 2025), `\documentclass{article}`,
//! no `babel`. `\meaning\today` there is
//!
//! ```text
//! \ifcase\month\or January\or ... \or December\fi \space\number\day, \number\year
//! ```
//!
//! i.e. `<MonthName> <day>, <year>` with no zero padding on either number.

use flashtex_compiler::date::TodayDate;
use flashtex_compiler::json::{self, Value};
use flashtex_compiler::parser::{parse_project_with, Block, ParseOptions, SourceDocument};
use flashtex_compiler::protocol::handle_line;

/// A runtime-v1 `compile` envelope; `date` is omitted entirely when `None`, so
/// these tests exercise the absent-field path as well as the supplied one.
fn compile_line(text: &str, date: Option<&str>) -> String {
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("demo"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    if let Some(date) = date {
        payload.set("date", json::str_(date));
    }
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("today-1"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env)
}

fn reply(text: &str, date: Option<&str>) -> Value {
    json::parse(&handle_line(&compile_line(text, date))).expect("reply must be valid JSON")
}

fn status(v: &Value) -> String {
    v.get("payload")
        .and_then(|p| p.get("status"))
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Every painted glyph run on every page, concatenated in paint order.
fn rendered_text(v: &Value) -> String {
    v.get("payload")
        .and_then(|p| p.get("pages"))
        .and_then(|p| p.as_arr())
        .map(|pages| {
            pages
                .iter()
                .filter_map(|page| page.get("items").and_then(|i| i.as_arr()).cloned())
                .flatten()
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()).map(String::from))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn diagnostics(v: &Value) -> Vec<String> {
    v.get("payload")
        .and_then(|p| p.get("diagnostics"))
        .and_then(|d| d.as_arr())
        .map(|ds| {
            ds.iter()
                .filter_map(|d| d.get("message").and_then(|m| m.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn doc(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}{preamble}\\begin{{document}}{body}\\end{{document}}")
}

// ---------------------------------------------------------------------------
// A real date reaches the page, over the wire.
// ---------------------------------------------------------------------------

#[test]
fn a_real_date_supplied_by_the_caller_reaches_the_page() {
    let source = doc("", "\\today");
    let out = reply(&source, Some("2026-09-13"));
    assert_eq!(status(&out), "ok", "{:?}", diagnostics(&out));
    assert!(
        rendered_text(&out).contains("September 13, 2026"),
        "expected the request's date; got {:?}",
        rendered_text(&out)
    );
    // The bug this fixes: the epoch must no longer be hard-coded anywhere.
    assert!(!rendered_text(&out).contains("1970"));
}

#[test]
fn date_today_in_maketitle_reaches_the_page() {
    let source = doc("\\title{T}\\author{A}\\date{\\today}", "\\maketitle");
    let out = reply(&source, Some("2026-09-13"));
    assert_eq!(status(&out), "ok", "{:?}", diagnostics(&out));
    assert!(
        rendered_text(&out).contains("September 13, 2026"),
        "got {:?}",
        rendered_text(&out)
    );
}

/// Different dates in, different bytes out — and identical dates in, identical
/// bytes out. That second half is the determinism contract, restated precisely:
/// the output is a pure function of its inputs, `date` now being one of them.
#[test]
fn identical_requests_render_identically_and_different_dates_do_not() {
    let source = doc("", "\\today");
    let a = handle_line(&compile_line(&source, Some("2026-09-13")));
    let b = handle_line(&compile_line(&source, Some("2026-09-13")));
    let c = handle_line(&compile_line(&source, Some("2026-09-14")));
    assert_eq!(a, b, "byte-identical input must give byte-identical output");
    assert_ne!(a, c, "a different date must change the output");
}

/// The same warm worker, same project, same entry path, two days running. The
/// session cache keys on the date, so the second request must not be served
/// yesterday's pages.
#[test]
fn a_warm_session_does_not_reuse_yesterdays_date() {
    let source = doc("", "\\today");
    let first = reply(&source, Some("2026-09-13"));
    let second = reply(&source, Some("2026-09-14"));
    assert!(rendered_text(&first).contains("September 13, 2026"));
    assert!(
        rendered_text(&second).contains("September 14, 2026"),
        "warm session served a stale date: {:?}",
        rendered_text(&second)
    );
}

// ---------------------------------------------------------------------------
// The epoch override pins it.
// ---------------------------------------------------------------------------

/// An absent `date` is the Unix epoch — exactly what this compiler printed
/// before the field existed. This is what keeps every old client and every
/// committed fixture byte-identical, and it is why no oracle reference data
/// needed regenerating for this change.
#[test]
fn an_absent_date_is_the_epoch() {
    let out = reply(&doc("", "\\today"), None);
    assert_eq!(status(&out), "ok", "{:?}", diagnostics(&out));
    assert!(
        rendered_text(&out).contains("January 1, 1970"),
        "got {:?}",
        rendered_text(&out)
    );
}

/// Harnesses with committed expectations pin the epoch explicitly rather than
/// relying on the default, so their reference data survives any later change to
/// what "no date supplied" means.
#[test]
fn an_explicit_epoch_pins_the_same_bytes_as_omitting_the_field() {
    let source = doc("\\title{T}\\author{A}", "\\maketitle \\today");
    assert_eq!(
        handle_line(&compile_line(&source, Some("1970-01-01"))),
        handle_line(&compile_line(&source, None)),
        "an explicit epoch must be indistinguishable from an absent date"
    );
}

// ---------------------------------------------------------------------------
// A bad date is refused, never guessed.
// ---------------------------------------------------------------------------

#[test]
fn a_malformed_date_fails_the_compile_instead_of_silently_using_another_one() {
    for bad in [
        "2026-9-13",
        "13-09-2026",
        "2026/09/13",
        "2026-09-13T12:00:00Z",
        "yesterday",
        "",
    ] {
        let out = reply(&doc("", "\\today"), Some(bad));
        assert_eq!(status(&out), "failed", "{bad:?} was accepted");
        assert!(
            diagnostics(&out).iter().any(|m| m.contains("date")),
            "{bad:?} produced no date diagnostic: {:?}",
            diagnostics(&out)
        );
        assert!(
            !rendered_text(&out).contains("1970"),
            "{bad:?} silently fell back to the epoch"
        );
    }
}

#[test]
fn a_day_that_does_not_exist_is_refused() {
    // 2026 is not a leap year; 2024 is.
    let out = reply(&doc("", "\\today"), Some("2026-02-29"));
    assert_eq!(status(&out), "failed", "{:?}", diagnostics(&out));
    let ok = reply(&doc("", "\\today"), Some("2024-02-29"));
    assert_eq!(status(&ok), "ok", "{:?}", diagnostics(&ok));
    assert!(rendered_text(&ok).contains("February 29, 2024"));
}

#[test]
fn a_non_string_date_is_refused() {
    let mut doc_value = Value::obj();
    doc_value.set("path", json::str_("main.tex"));
    doc_value.set("text", json::str_(&doc("", "\\today")));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("demo"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc_value]));
    payload.set("date", Value::Num(1_757_721_600.0));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("today-num"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    let out = json::parse(&handle_line(&json::write(&env))).unwrap();
    assert_eq!(status(&out), "failed");
    assert!(diagnostics(&out).iter().any(|m| m.contains("date")));
}

// ---------------------------------------------------------------------------
// `\date{}` vs an absent `\date`, against the oracle.
// ---------------------------------------------------------------------------

fn title_date(source: &str, today: TodayDate) -> Option<String> {
    let parsed = parse_project_with(
        &[SourceDocument {
            path: "main.tex",
            text: source,
        }],
        "main.tex",
        &ParseOptions { today },
    );
    parsed
        .blocks
        .into_iter()
        .find_map(|b| match b {
            Block::TitleBlock { date, .. } => Some(date),
            _ => None,
        })
        .expect("a title block")
        .map(|inlines| format!("{inlines:?}"))
}

/// latex.ltx 17225: `\gdef\@date{\today}`. A document with no `\date` is
/// therefore *identical* to `\date{\today}`, not merely similar. pdflatex
/// (TeX Live 2025) agrees to the scaled point: `\the\pagetotal` immediately
/// after `\maketitle` is 116.86673pt in both documents.
#[test]
fn an_absent_date_renders_exactly_like_date_today() {
    let today = TodayDate::parse_iso("2026-09-13").unwrap();
    let absent = title_date(&doc("\\title{T}\\author{A}", "\\maketitle"), today);
    let explicit = title_date(
        &doc("\\title{T}\\author{A}\\date{\\today}", "\\maketitle"),
        today,
    );
    let absent = absent.expect("absent \\date still typesets a date line");
    let explicit = explicit.expect("\\date{\\today} typesets a date line");
    assert!(absent.contains("September 13, 2026"), "{absent}");
    assert!(explicit.contains("September 13, 2026"), "{explicit}");
}

/// `\date{}` leaves `\@date` empty. article.cls still emits `\vskip 1em` and
/// opens `{\large \@date}`, but an empty group typesets no material, so the
/// paragraph contributes no line and no `\baselineskip`. pdflatex (TeX Live
/// 2025) `\the\pagetotal` after `\maketitle`: 95.2001pt for `\date{}` against
/// 114.4001pt for `\date{Zz}` — a difference of exactly one `\large`
/// baselineskip (19.2pt). The 1em *stays*; only the line goes.
#[test]
fn an_empty_date_suppresses_the_line_and_is_not_the_epoch() {
    let today = TodayDate::parse_iso("2026-09-13").unwrap();
    let empty = title_date(&doc("\\title{T}\\author{A}\\date{}", "\\maketitle"), today);
    assert!(
        empty.is_none(),
        "\\date{{}} must suppress the date line, got {empty:?}"
    );
}

#[test]
fn an_explicit_date_is_used_verbatim_and_never_replaced_by_the_request_date() {
    let today = TodayDate::parse_iso("2026-09-13").unwrap();
    let explicit = title_date(
        &doc("\\title{T}\\author{A}\\date{Spring 2026}", "\\maketitle"),
        today,
    )
    .expect("a date line");
    assert!(explicit.contains("Spring"), "{explicit}");
    assert!(
        !explicit.contains("September"),
        "the request date overwrote an explicit \\date: {explicit}"
    );
}

// ---------------------------------------------------------------------------
// `\today` outside a title argument.
// ---------------------------------------------------------------------------

/// `\today` in ordinary prose had no dispatch arm and fell through to
/// `P::unsupported`, whose `debug_assert!(!BUILT_INS.contains(&name))` fires
/// because `today` *is* a built-in — so a debug build panicked on a document
/// that merely wrote the date in a sentence. `tests/supported_latex.rs`'s
/// `canonical_names_outside_the_inventory_are_diagnosed` was failing on main
/// for exactly this reason.
#[test]
fn today_in_body_text_renders_instead_of_panicking() {
    let out = reply(&doc("", "Written on \\today."), Some("2026-09-13"));
    assert_eq!(status(&out), "ok", "{:?}", diagnostics(&out));
    let text = rendered_text(&out);
    assert!(text.contains("September 13, 2026"), "{text}");
    assert!(
        !diagnostics(&out)
            .iter()
            .any(|m| m.contains("not supported")),
        "{:?}",
        diagnostics(&out)
    );
}

#[test]
fn today_renders_the_same_in_prose_and_in_a_title() {
    let today = TodayDate::parse_iso("2026-01-05").unwrap();
    // \number\day: no zero padding on a single-digit day.
    assert_eq!(today.latex_today(), "January 5, 2026");
    let in_title = title_date(
        &doc("\\title{T}\\author{A}\\date{\\today}", "\\maketitle"),
        today,
    )
    .expect("a date line");
    assert!(in_title.contains("January 5, 2026"), "{in_title}");

    let in_prose = reply(&doc("", "\\today"), Some("2026-01-05"));
    assert!(rendered_text(&in_prose).contains("January 5, 2026"));
}


// ---------------------------------------------------------------------------
// The same latent panic, in the neighbouring title-only built-ins.
// ---------------------------------------------------------------------------

/// `\thanks` and `\and` had the identical bug `\today` did: no dispatch arm, so
/// they reached `P::unsupported` and tripped its `BUILT_INS` `debug_assert`.
/// `supported.rs` already claimed they were "diagnosed on their own"; now they
/// really are.
#[test]
fn title_only_builtins_diagnose_instead_of_panicking() {
    for (source, needle) in [
        ("\\thanks{Supported by a grant}", "\\thanks"),
        ("\\and", "\\and"),
    ] {
        let out = reply(&doc("", source), Some("2026-09-13"));
        let diags = diagnostics(&out);
        assert!(
            diags.iter().any(|m| m.contains(needle)),
            "{source} produced no diagnostic naming it: {diags:?}"
        );
    }
}

/// This compiler has no footnote implementation, so `\thanks` must drop its
/// note rather than typeset it inline as running prose.
#[test]
fn thanks_does_not_leak_its_note_into_the_page() {
    let out = reply(
        &doc("", "Body \\thanks{SECRETNOTE} text."),
        Some("2026-09-13"),
    );
    assert!(
        !rendered_text(&out).contains("SECRETNOTE"),
        "note text leaked onto the page: {:?}",
        rendered_text(&out)
    );
}
