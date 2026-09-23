//! Lane etoolbox-appto-preto: etoolbox's macro patchers (`\appto`/`\gappto`/
//! `\eappto` append, `\preto`/`\gpreto` prepend, `\csappto`/`\cspreto` by
//! csname) run in the expansion pass over etoolbox.sty's own
//! `\edef`/`\xdef` + `\expandonce`/`\unexpanded` bodies, gated on
//! `\@ifpackageloaded{etoolbox}` so plain-article use errors the way
//! pdflatex errors. Expected words are the measured pdflatex (TeX Live
//! 2026, `pdflatex -interaction=nonstopmode`, 0 `!` lines) values for the
//! same inputs, read back with `pdftotext`.

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

fn etoolbox_document(preamble: &str, body: &str) -> String {
    format!("\\documentclass{{article}}\n\\usepackage{{etoolbox}}\n{preamble}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// Probe text with all whitespace removed. Each token spliced in by
/// `\appto`/`\preto`/`\gappto` keeps the patch command's invocation span,
/// so the typesetter emits one text run per letter (`C`, `A`, `B`, `D`)
/// instead of one run per word; joining runs with `" "` would invent
/// spaces pdflatex never typesets. A dumped reply for the repro shows the
/// runs adjacent on one baseline (`C` x=105.66, `A` 113.66, `B` 122.33,
/// `D` 130.33, `.` 139, no space items between), i.e. the rendered word
/// is `CABD` exactly as pdflatex's `Value: CABD.` — so comparing
/// whitespace-insensitive text is the faithful check.
fn squashed(words: &[String]) -> String {
    words.join("").replace(' ', "")
}

#[test]
fn appto_preto_gappto_repro_matches_pdflatex() {
    // Slice repro: pdflatex typesets "Value: CABD." with 0 errors.
    let reply = compile(
        "appto-repro.tex",
        &etoolbox_document(
            "\\newcommand\\x{A}\\appto\\x{B}\\preto\\x{C}\\gappto\\x{D}",
            "Value: \\x.",
        ),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(squashed(&page_words(&reply)), "Value:CABD.");
}

#[test]
fn all_seven_patchers_match_pdflatex() {
    // pdflatex typesets "A:Ab. G:Gg. E:pre-Ab. P:qP. GP:qP. CS:C. CSQ:ZQ.
    // N:N." with 0 errors (`\csappto` and `\appto` also define undefined
    // macros; `\eappto` fully expands the appended code).
    let reply = compile(
        "appto-all.tex",
        &etoolbox_document(
            "\\newcommand\\maa{A}\\appto\\maa{b}\
             \\newcommand\\mgg{G}\\gappto\\mgg{g}\
             \\newcommand\\mee{pre-}\\eappto\\mee{\\maa}\
             \\newcommand\\mpp{P}\\preto\\mpp{q}\
             \\newcommand\\mgp{P}\\gpreto\\mgp{q}\
             \\csappto{csmacro}{C}\
             \\newcommand\\csq{Q}\\cspreto{csq}{Z}\
             \\appto\\newmacro{N}",
            "A:\\maa. G:\\mgg. E:\\mee. P:\\mpp. GP:\\mgp. CS:\\csmacro. CSQ:\\csq. N:\\newmacro.",
        ),
    );
    let found = diagnostics(&reply);
    assert!(found.is_empty(), "unexpected diagnostics: {found:?}");
    assert_eq!(
        squashed(&page_words(&reply)),
        "A:Ab.G:Gg.E:pre-Ab.P:qP.GP:qP.CS:C.CSQ:ZQ.N:N."
    );
}

#[test]
fn patchers_are_rejected_without_etoolbox() {
    // pdflatex under plain article errors (`! Undefined control sequence.`
    // for `\appto`); the patcher must not silently apply.
    let reply = compile(
        "appto-noetb.tex",
        "\\documentclass{article}\n\\newcommand\\x{A}\\appto\\x{B}\n\\begin{document}\nValue: \\x.\n\\end{document}\n",
    );
    let found = diagnostics(&reply);
    assert!(
        found.iter().any(|(sev, code, _)| sev == "error" && code == "unknown_command"),
        "expected an error for \\appto without etoolbox (all: {found:?})"
    );
    assert!(
        !page_words(&reply).join(" ").contains("CABD")
            && !page_words(&reply).join(" ").contains("AB."),
        "unpatched \\x must not gain the appended code: {:?}",
        page_words(&reply)
    );
}
