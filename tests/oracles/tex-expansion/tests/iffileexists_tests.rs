//! `\IfFileExists{file}{true}{false}` (ltfiles.dtx): expands to the true
//! branch when `file` is in the project closure and to the false branch
//! when it is not -- never an error, so packages can guard optional
//! features with it.

use std::collections::HashMap;
use std::rc::Rc;

use flashtex_tex_expansion::{
    is_group_token, tokens_to_display_string, Engine, Token, TokenKind,
};

/// The bytes of a real fixture file: the file reader below serves them as
/// `present.tex`, so the "present" case exercises genuine file content.
const PRESENT_TEX: &str = include_str!("fixtures/HW1.tex");

/// An engine whose file reader serves `present.tex` and whose package
/// reader serves `mystyle.sty`; everything else is absent.
fn engine_with(src: &str) -> Engine {
    let mut engine = Engine::new(src);
    engine.set_file_reader(Rc::new(|name: &str| {
        (name == "present.tex").then(|| PRESENT_TEX.to_string())
    }));
    let packages: HashMap<String, String> =
        [("mystyle.sty".to_string(), "\\ProvidesPackage{mystyle}".to_string())]
            .into_iter()
            .collect();
    engine.set_package_reader(Rc::new(move |name, ext| {
        packages.get(&format!("{name}.{ext}")).cloned()
    }));
    engine
}

/// Content text: grouping tokens and executed `\relax`es dropped, spaces
/// collapsed -- what the typesetter would set.
fn text(tokens: &[Token]) -> String {
    let content: Vec<Token> = tokens
        .iter()
        .filter(|t| !is_group_token(t))
        .filter(|t| !matches!(&t.kind, TokenKind::ControlSequence(n) if n == "relax"))
        .cloned()
        .collect();
    tokens_to_display_string(&content)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn run(src: &str) -> (String, Vec<String>) {
    let mut engine = engine_with(src);
    let tokens = engine.run();
    let diagnostics = engine
        .take_diagnostics()
        .into_iter()
        .map(|d| format!("{:?}: {}", d.severity, d.message))
        .collect();
    (text(&tokens), diagnostics)
}

#[test]
fn missing_file_expands_to_false_branch() {
    // The task's repro: no such file in the project closure.
    let (out, diags) = run("\\IfFileExists{nope.tex}{yes}{no}");
    assert_eq!(out, "no");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn present_fixture_file_expands_to_true_branch() {
    let (out, diags) = run("\\IfFileExists{present.tex}{yes}{no}");
    assert_eq!(out, "yes");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn present_package_file_expands_to_true_branch() {
    let (out, diags) = run("\\IfFileExists{mystyle.sty}{yes}{no}");
    assert_eq!(out, "yes");
    assert!(diags.is_empty(), "{diags:?}");
    let (out, diags) = run("\\IfFileExists{other.sty}{yes}{no}");
    assert_eq!(out, "no");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn filename_is_expanded_and_chosen_branch_expands() {
    let (out, diags) = run("\\def\\whichfile{nope.tex}\\IfFileExists{\\whichfile}{yes}{no}");
    assert_eq!(out, "no");
    assert!(diags.is_empty(), "{diags:?}");
    let (out, diags) = run("\\def\\y{YES}\\IfFileExists{present.tex}{\\y}{no}");
    assert_eq!(out, "YES");
    assert!(diags.is_empty(), "{diags:?}");
    let (out, diags) = run("\\def\\n{NO}\\IfFileExists{nope.tex}{yes}{\\n}");
    assert_eq!(out, "NO");
    assert!(diags.is_empty(), "{diags:?}");
}
