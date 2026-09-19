//! `fontspec` under a pdflatex-equivalent engine (issues #837, #905).
//!
//! Oracle: pdfTeX 3.141592653-2.6-1.40.29 (TeX Live 2026):
//! - guarded (`\usepackage{iftex}` + `\ifxetex ... \fi` around the fontspec
//!   setup): exit 0, the block skipped, no errors;
//! - unguarded (`\usepackage{fontspec}` + `\setmainfont{...}` with no guard):
//!   fatal error "The fontspec package requires either XeTeX or LuaTeX."
//!
//! FlashTeX's engine model is pdflatex-equivalent, so a guarded block must be
//! skipped (the `\ifxetex`/`\ifluatex` switches are false, as `iftex.sty`
//! sets them under pdflatex) while unguarded fontspec use must diagnose
//! loudly under this compiler's warn-and-continue convention for
//! unimplemented packages -- never silently mis-render, never crash.

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

fn compile(path: &str, text: &str) -> Vec<Value> {
    let mut doc = Value::obj();
    doc.set("path", json::str_(path));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("fontspec-guard"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_(path));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("fontspec-guard"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    let reply = json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply");
    reply
        .get("payload")
        .expect("payload")
        .get("diagnostics")
        .and_then(|v| v.as_arr())
        .cloned()
        .unwrap_or_default()
}

fn field(diag: &Value, key: &str) -> String {
    diag.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn help_of(diag: &Value) -> String {
    diag.get("help")
        .and_then(|h| h.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or("")
        .to_string()
}

#[test]
fn guarded_fontspec_block_is_skipped_like_pdflatex() {
    // The common safe-guard pattern: pdflatex exits 0 on this with no errors.
    let diags = compile(
        "guarded.tex",
        "\\documentclass{article}\n\
         \\usepackage{iftex}\n\
         \\ifxetex\n\
         \\usepackage{fontspec}\n\
         \\setmainfont{Times New Roman}\n\
         \\fi\n\
         \\begin{document}\n\
         Hello guarded world.\n\
         \\end{document}\n",
    );
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| field(d, "severity") == "error")
        .collect();
    assert!(errors.is_empty(), "error diagnostics remain: {errors:?}");
    let unsupported: Vec<_> = diags
        .iter()
        .filter(|d| field(d, "code") == "unsupported_feature")
        .collect();
    assert!(
        unsupported.is_empty(),
        "unsupported_feature diagnostics remain: {unsupported:?} (all: {diags:?})"
    );
}

#[test]
fn luatex_guard_takes_the_else_branch() {
    // `\ifluatex` is false under pdflatex, so the true branch -- poisoned
    // here with `\setmainfont`, which would diagnose if taken -- must be
    // skipped while the `\else` text survives. Mirrors the evan.sty corpus
    // pattern (`\ifluatex ... \else ... \fi`).
    let diags = compile(
        "luatex-guard.tex",
        "\\documentclass{article}\n\
         \\usepackage{iftex}\n\
         \\begin{document}\n\
         \\ifluatex\n\
         \\setmainfont{LuaFont}\n\
         \\else\n\
         Plain pdf text.\n\
         \\fi\n\
         \\end{document}\n",
    );
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| field(d, "severity") == "error")
        .collect();
    assert!(errors.is_empty(), "error diagnostics remain: {errors:?}");
    let unsupported: Vec<_> = diags
        .iter()
        .filter(|d| field(d, "code") == "unsupported_feature")
        .collect();
    assert!(
        unsupported.is_empty(),
        "unsupported_feature diagnostics remain: {unsupported:?} (all: {diags:?})"
    );
}

#[test]
fn capitalized_engine_switches_are_aliases() {
    // `iftex.sty` lets `\ifXeTeX`/`\ifLuaTeX` to the same switches (used by
    // e.g. the lshort corpus document); both are false under pdflatex.
    let diags = compile(
        "capital-guard.tex",
        "\\documentclass{article}\n\
         \\usepackage{iftex}\n\
         \\begin{document}\n\
         \\ifXeTeX\n\
         \\setmainfont{XeFont}\n\
         \\else\n\
         Plain pdf text.\n\
         \\fi\n\
         \\end{document}\n",
    );
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| field(d, "severity") == "error")
        .collect();
    assert!(errors.is_empty(), "error diagnostics remain: {errors:?}");
    let unsupported: Vec<_> = diags
        .iter()
        .filter(|d| field(d, "code") == "unsupported_feature")
        .collect();
    assert!(
        unsupported.is_empty(),
        "unsupported_feature diagnostics remain: {unsupported:?} (all: {diags:?})"
    );
}

#[test]
fn engine_test_packages_load_silently() {
    // `iftex`/`ifxetex`/`ifluatex` take no options; like `ifthen`, loading
    // them is silent because their conditionals run in the expansion pass.
    let diags = compile(
        "engines.tex",
        "\\documentclass{article}\n\
         \\usepackage{iftex,ifxetex,ifluatex}\n\
         \\begin{document}\n\
         x\n\
         \\end{document}\n",
    );
    let unsupported: Vec<_> = diags
        .iter()
        .filter(|d| field(d, "code") == "unsupported_feature")
        .collect();
    assert!(
        unsupported.is_empty(),
        "engine-test package warning remains: {unsupported:?} (all: {diags:?})"
    );
}

#[test]
fn engine_test_package_options_still_warn() {
    // `iftex.sty` takes no options; an unknown one keeps the package warning.
    let diags = compile(
        "iftex-opt.tex",
        "\\documentclass{article}\n\\usepackage[foo]{iftex}\n\\begin{document}\nx\n\\end{document}\n",
    );
    assert!(
        diags.iter().any(|d| field(d, "code") == "unsupported_feature"
            && field(d, "message").contains("iftex")),
        "expected the iftex package warning for an unknown option (all: {diags:?})"
    );
}

#[test]
fn unguarded_fontspec_package_load_warns() {
    // pdflatex fatals here ("requires either XeTeX or LuaTeX"); this
    // compiler warns and continues, as for every other unimplemented
    // package -- but it must say so, not stay silent.
    let diags = compile(
        "unguarded.tex",
        "\\documentclass{article}\n\
         \\usepackage{fontspec}\n\
         \\begin{document}\n\
         Hello unguarded world.\n\
         \\end{document}\n",
    );
    assert!(
        diags.iter().any(|d| field(d, "code") == "unsupported_feature"
            && field(d, "message").contains("fontspec")),
        "expected the fontspec package warning (all: {diags:?})"
    );
}

#[test]
fn unguarded_setmainfont_is_unsupported_not_unknown() {
    // `\setmainfont` is real fontspec vocabulary: it must report
    // `unsupported_feature` with a fontspec help line (not an
    // `unknown_command` typo hunt), and its font-name argument must be
    // consumed as a parameter so "Times New Roman" never reaches the page.
    let diags = compile(
        "setmainfont.tex",
        "\\documentclass{article}\n\
         \\usepackage{fontspec}\n\
         \\begin{document}\n\
         \\setmainfont{Times New Roman}\n\
         Done.\n\
         \\end{document}\n",
    );
    let target: Vec<_> = diags
        .iter()
        .filter(|d| field(d, "message").contains("setmainfont"))
        .collect();
    assert_eq!(target.len(), 1, "expected one setmainfont diagnostic (all: {diags:?})");
    assert_eq!(field(target[0], "code"), "unsupported_feature");
    assert!(
        help_of(target[0]).contains("fontspec"),
        "help should name the fontspec package (all: {diags:?})"
    );
    assert!(
        field(target[0], "recovery").contains("skipped the command and its argument"),
        "the font name must be consumed, not typeset (all: {diags:?})"
    );
}
