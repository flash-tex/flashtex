//! Box registers as token captures (`\newsavebox`/`\sbox`/`\setbox`/
//! `lrbox`, replayed by `\usebox`/`\box`/`\copy`), the mode the main loop
//! tracks for `\ifvmode`/`\ifhmode`/`\ifmmode`, and the `\everypar` text
//! it inserts where a paragraph starts (tex.web §1090 `new_graf`).
//!
//! Expected texts are what pdflatex (MacTeX 2026) typesets for the same
//! input; the oracle was consulted by hand, not run from here.

use flashtex_tex_expansion::{is_group_token, tokens_to_display_string, Engine, Token, TokenKind};

fn text(tokens: &[Token]) -> String {
    let content: Vec<Token> = tokens
        .iter()
        .filter(|t| !is_group_token(t))
        .filter(|t| !matches!(&t.kind, TokenKind::ControlSequence(n) if n == "relax"))
        .cloned()
        .collect();
    tokens_to_display_string(&content).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn run(src: &str) -> (String, Vec<String>) {
    let mut engine = Engine::new(src);
    let tokens = engine.run();
    let diagnostics = engine.take_diagnostics().into_iter().map(|d| format!("{:?}: {}", d.severity, d.message)).collect();
    (text(&tokens), diagnostics)
}

#[test]
fn sbox_captures_and_usebox_replays() {
    let (out, diags) = run(r"\newsavebox{\mybox}\sbox\mybox{Hello \textbf{bold}}Before \usebox\mybox\ after \usebox{\mybox}.");
    assert_eq!(out, "Before Hello \\textbf bold\\ after Hello \\textbf bold.", "{diags:?}");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn box_voids_the_register_but_copy_keeps_it() {
    let (out, diags) = run(r"\newsavebox{\b}\sbox\b{X}[\copy\b][\box\b][\box\b]");
    assert_eq!(out, "[X][X][]", "{diags:?}");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn lrbox_environment_and_global_sbox() {
    let (out, diags) = run(
        r"\newsavebox{\b}\begin{lrbox}{\b}World\end{lrbox}(\usebox\b){\global\sbox\b{Global}}(\usebox\b){\sbox\b{Local}}(\usebox\b)",
    );
    assert_eq!(out, "(World)(Global)(Global)", "{diags:?}");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn setbox_hbox_to_width_and_nested_groups() {
    let (out, diags) = run(r"\newbox\b\setbox\b\hbox to 2cm{a{b}c}\box\b|\setbox\b=\vbox{v}\unvbox\b");
    assert_eq!(out, "abc|v", "{diags:?}");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn box_dimensions_read_the_measurer_and_accept_assignments() {
    // The default measurer reports zero; an assigned `\wd` shadows it.
    let (out, diags) = run(r"\newsavebox{\b}\sbox\b{wide}\newlength{\l}\setlength{\l}{\wd\b}[\the\l]\wd\b=3pt\setlength{\l}{\wd\b}[\the\l]");
    assert_eq!(out, "[0.0pt][3.0pt]", "{diags:?}");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn everypar_is_inserted_where_each_paragraph_starts() {
    let (out, diags) = run(r"\everypar{[N]}First.\par Second line.\par\noindent Third.\par\textbf{Fourth}.\everypar{}Fifth.");
    assert_eq!(out, "[N]First.\\par [N]Second line.\\par \\noindent [N]Third.\\par [N]\\textbf Fourth.Fifth.", "{diags:?}");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn everypar_does_not_fire_inside_an_hbox_but_usebox_starts_a_paragraph() {
    let (out, diags) = run(r"\newsavebox{\b}\everypar{[N]}\sbox\b{boxed}\usebox\b\par\sbox\b{again}x\usebox\b");
    assert_eq!(out, "[N]boxed\\par [N]xagain", "{diags:?}");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn mode_conditionals_follow_the_main_loop() {
    let (out, diags) = run(r"\ifvmode V\fi Text\ifhmode H\fi $\ifmmode M\fi$\ifhmode H2\fi\par\ifvmode V2\fi \[\ifmmode D\fi\]\ifhmode H3\fi");
    assert_eq!(out, "VTextH$M$H2\\par V2\\[D\\]H3", "{diags:?}");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn a_bare_box_register_in_main_control_is_an_error() {
    let (_, diags) = run(r"\newsavebox{\b}\b");
    assert_eq!(diags, vec!["Error: A <box> was supposed to be here.".to_string()]);
}
