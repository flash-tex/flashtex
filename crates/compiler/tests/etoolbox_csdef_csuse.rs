//! etoolbox's control-sequence constructors (`\csdef`, `\csgdef`,
//! `\csedef`, `\csxdef`, `\csuse`, `\csletcs`, `\cslet`) run in the
//! expansion pass as host-prelude macros mirroring etoolbox.sty's own
//! definitions (`texdef -t latex -p etoolbox ...` on TeX Live 2026), gated
//! on `\usepackage{etoolbox}` via the engine's `ver@etoolbox.sty` record so
//! they are rejected under plain article exactly like pdflatex rejects them.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

fn compile(path: &str, text: &str) -> Value {
    let mut doc = Value::obj();
    doc.set("path", json::str_(path));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("etoolbox"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(path));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("etoolbox"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply")
}

fn diagnostics(reply: &Value) -> Vec<(String, String, String)> {
    reply
        .get("payload")
        .expect("payload")
        .get("diagnostics")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|d| {
            (
                d.get("severity").and_then(|s| s.as_str()).unwrap_or("?").to_string(),
                d.get("code").and_then(|c| c.as_str()).unwrap_or("?").to_string(),
                d.get("message").and_then(|m| m.as_str()).unwrap_or("?").to_string(),
            )
        })
        .collect()
}

/// The words typeset on every page, in order.
fn page_words(reply: &Value) -> Vec<String> {
    reply
        .get("payload")
        .expect("payload")
        .get("pages")
        .and_then(|p| p.as_arr())
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|pg| {
            pg.get("items").and_then(|i| i.as_arr()).cloned().unwrap_or_default()
        })
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("text"))
        .filter_map(|item| item.get("text").and_then(Value::as_str).map(str::to_string))
        .collect()
}

fn items(reply: &Value) -> Vec<Value> {
    reply
        .get("payload")
        .expect("payload")
        .get("pages")
        .and_then(|p| p.as_arr())
        .cloned()
        .unwrap_or_default()
        .iter()
        .flat_map(|pg| {
            pg.get("items").and_then(|i| i.as_arr()).cloned().unwrap_or_default()
        })
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("text"))
        .collect()
}

fn xs_of(reply: &Value) -> Vec<(String, f64)> {
    items(reply)
        .iter()
        .map(|item| {
            let text = item
                .get("text")
                .and_then(Value::as_str)
                .expect("text item")
                .to_string();
            let x = item
                .get("x_pt")
                .and_then(|v| match v {
                    Value::Num(n) => Some(*n),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("no x_pt for {text:?}"));
            (text, x)
        })
        .collect()
}

/// Rendered words with harness item splits glued back: text the expansion
/// pass produced from a macro (`\csuse{qux}` -> `Foo`) ends its item at the
/// macro boundary, so a following `.`/`!` is its own item while pdflatex —
/// and a literal FlashTeX run — set `Foo.`/`IN!` as one word.
fn rendered_text(reply: &Value) -> String {
    page_words(reply).join(" ").replace(" .", ".").replace(" !", "!")
}

fn with_etoolbox(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{etoolbox}}\n{preamble}\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

fn plain_article(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

#[test]
fn csdef_csgdef_csuse_match_pdflatex() {
    // Oracle: `pdflatex -interaction=nonstopmode repro.tex` (article 10pt,
    // letter, TeX Live 2026) exits with 0 errors; `pdftotext repro.pdf`
    // prints `A Foo Bar B.`; `pdftotext -bbox` puts `A` at xMin=148.712
    // (= the row's 133.77 text-left + 14.94 parindent), `Foo` at 159.501,
    // `Bar` at 178.737, `B.` at 197.995.
    let reply = compile(
        "csuse-repro.tex",
        &with_etoolbox(
            "\\csdef{foo}{Foo}\\csgdef{bar}{Bar}\n",
            "A \\csuse{foo} \\csuse{bar} \\csuse{undefined}B.",
        ),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(rendered_text(&reply), "A Foo Bar B.");
    // Positions: this pipeline lays out from its fixed 72pt margin model
    // (`layout::MARGIN_PT`), without the article left margin and parindent
    // pdflatex's 148.712 carries, so no absolute assertion can reach the
    // measured value here. The tight check available is identity: every
    // word the macros produced must sit exactly where the same literal
    // text sits (0.1 tolerance, observed 0.0).
    let reference = compile("csuse-literal.tex", &plain_article("A Foo Bar B."));
    assert!(diagnostics(&reference).is_empty());
    let (got, want) = (xs_of(&reply), xs_of(&reference));
    assert_eq!(got.len(), want.len(), "word count: {got:?} vs {want:?}");
    for ((g_text, g_x), (w_text, w_x)) in got.iter().zip(want.iter()) {
        assert_eq!(g_text, w_text, "word mismatch: {got:?} vs {want:?}");
        assert!(
            (g_x - w_x).abs() <= 0.1,
            "{g_text}: macro-expanded x {g_x} must match literal x {w_x}"
        );
    }
}

#[test]
fn csedef_expands_at_definition_and_csxdef_is_global() {
    // Oracle: `pdflatex -interaction=nonstopmode oracle2.tex` (same setup,
    // 0 errors) prints `E IN! G G L Foo Foo U done.`: `\csedef` bakes the
    // expansion in, and the `\csxdef` inside a group is still visible after.
    let reply = compile(
        "csedef-global.tex",
        &with_etoolbox(
            "\\def\\inner{IN}\\csedef{egreet}{\\inner!}\n{\\csxdef{gglob}{G}}\n",
            "E \\csuse{egreet} G \\csuse{gglob}.",
        ),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(rendered_text(&reply), "E IN! G G.");
}

#[test]
fn csletcs_and_cslet_alias_existing_commands() {
    // Same oracle2 run: `L \csuse{baz} \csuse{qux}` prints `L Foo Foo`.
    let reply = compile(
        "cslet-alias.tex",
        &with_etoolbox(
            "\\csdef{foo}{Foo}\\csletcs{baz}{foo}\\cslet{qux}\\foo\n",
            "L \\csuse{baz} \\csuse{qux}.",
        ),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(rendered_text(&reply), "L Foo Foo.");
}

#[test]
fn csletcs_of_an_undefined_name_undefines_its_target() {
    // Same oracle2 run: after `\csletcs{t}{nosuchname}`, `\csuse{t}` prints
    // nothing (`U \csuse{t}done.` renders as `U done.`).
    let reply = compile(
        "csletcs-undef.tex",
        &with_etoolbox(
            "\\csdef{t}{T}\\csletcs{t}{nosuchname}\n",
            "U \\csuse{t}done.",
        ),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(rendered_text(&reply), "U done.");
}

#[test]
fn without_etoolbox_csuse_is_rejected_like_pdflatex() {
    // Oracle: `pdflatex -interaction=nonstopmode noetb.tex` (plain article)
    // reports `! Undefined control sequence.` at `\csuse` and still prints
    // `A foo B.` (the leftover `{foo}` group typesets as text).
    let reply = compile("csuse-noetb.tex", &plain_article("A \\csuse{foo} B."));
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an undefined-command error without etoolbox (all: {found:?})"
    );
    assert_eq!(rendered_text(&reply), "A foo B.");
}

#[test]
fn without_etoolbox_csdef_is_rejected_like_pdflatex() {
    // Oracle: adding `\csdef{foo}{Foo}` to a plain-article preamble makes
    // pdflatex report `! Undefined control sequence.` there too.
    let reply = compile(
        "csdef-noetb.tex",
        "\\documentclass{article}\n\\csdef{foo}{Foo}\n\\begin{document}\nx\n\\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an undefined-command error for \\csdef without etoolbox (all: {found:?})"
    );
}
