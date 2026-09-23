//! Project `.sty`/`.cls` files end to end (S1 of
//! `docs/proposals/packages-fonts-manifest.md`): `\usepackage{mystyle}`
//! renders the package's macros, and a `myclass.cls` that `\LoadClass[11pt]
//! {article}` renders with article's 11pt page model -- the same words at
//! the same positions as `\documentclass[11pt]{article}` -- while the class
//! file's own `\setlength`s apply on top.
//!
//! No TeX runs here; the control documents are this pipeline's own
//! renderings of the equivalent plain-`article` sources.

mod common;

use common::*;

const MYSTYLE: &str = "\\NeedsTeXFormat{LaTeX2e}\\ProvidesPackage{mystyle}\\newcommand{\\hello}{Hello from mystyle}\\newcommand{\\emphx}[1]{\\textbf{#1}}\n";
const MYCLASS: &str = "\\NeedsTeXFormat{LaTeX2e}\\ProvidesClass{myclass}\n\\DeclareOption*{\\PassOptionsToClass{\\CurrentOption}{article}}\n\\ProcessOptions\\relax\n\\LoadClass[11pt]{article}\n\\RequirePackage{mystyle}\n\\newcommand{\\greeting}{Hi}\n";

fn positioned(r: &flashtex_render_pipeline::Rendered) -> Vec<(u32, String, i64, i64)> {
    words_of(r)
        .iter()
        .map(|w| (w.page, w.text.clone(), (w.x * 100.0).round() as i64, (w.baseline * 100.0).round() as i64))
        .collect()
}

#[test]
fn mystyle_renders_its_macros_with_world_in_bold() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\n\\hello, \\emphx{world}.\n\\end{document}\n";
    let r = render_docs(&[("main.tex", main), ("mystyle.sty", MYSTYLE)], "main.tex");
    let text: Vec<String> = words_of(&r).iter().map(|w| w.text.clone()).collect();
    assert_eq!(text.join(" "), "Hello from mystyle, world . 1", "{text:?}");
    // The control: the same body written out, with `\textbf{world}`.
    let control = "\\documentclass{article}\n\\begin{document}\nHello from mystyle, \\textbf{world}.\n\\end{document}\n";
    let c = render_docs(&[("main.tex", control)], "main.tex");
    assert_eq!(positioned(&r), positioned(&c));
    assert!(!r.v2.diagnostics.iter().any(|d| d.message.contains("mystyle") || d.message.contains("\\hello")), "{:?}", r.v2.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>());
}

#[test]
fn myclass_renders_with_articles_11pt_model_and_its_own_lengths() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let body = "Hi: Hello from mystyle, \\textbf{world}. A second sentence long enough to need a second line of text on the page, and a third one after it.\n\nA second paragraph, indented (or not) by the class.\n";
    let main = format!("\\documentclass{{myclass}}\n\\begin{{document}}\n{}\\end{{document}}\n", body.replace("Hi: Hello from mystyle, \\textbf{world}.", "\\greeting: \\hello, \\emphx{world}."));
    let r = render_docs(&[("main.tex", main.as_str()), ("myclass.cls", MYCLASS), ("mystyle.sty", MYSTYLE)], "main.tex");
    let control = format!("\\documentclass[11pt]{{article}}\n\\begin{{document}}\n{body}\\end{{document}}\n");
    let c = render_docs(&[("main.tex", control.as_str())], "main.tex");
    assert_eq!(positioned(&r), positioned(&c), "the 11pt article model through the class file");
    // Not the 10pt model.
    let ten = format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\\end{{document}}\n");
    let t = render_docs(&[("main.tex", ten.as_str())], "main.tex");
    assert_ne!(positioned(&r), positioned(&t), "11pt differs from 10pt");
    // The class file's `\setlength{\parindent}{0pt}` applies on top.
    let flat_class = format!("{MYCLASS}\\setlength{{\\parindent}}{{0pt}}\n");
    let f = render_docs(&[("main.tex", main.as_str()), ("myclass.cls", flat_class.as_str()), ("mystyle.sty", MYSTYLE)], "main.tex");
    let flat_control = format!("\\documentclass[11pt]{{article}}\n\\setlength{{\\parindent}}{{0pt}}\n\\begin{{document}}\n{body}\\end{{document}}\n");
    let fc = render_docs(&[("main.tex", flat_control.as_str())], "main.tex");
    assert_eq!(positioned(&f), positioned(&fc), "class-file \\setlength");
    assert_ne!(positioned(&f), positioned(&r), "\\parindent 0pt moved the second paragraph");
}

#[test]
fn a_class_without_loadclass_renders_as_article_with_one_warning() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let main = "\\documentclass[12pt]{thesis}\n\\begin{document}\n\\chap\n\\end{document}\n";
    let r = render_docs(&[("main.tex", main), ("thesis.cls", "\\ProvidesClass{thesis}\\newcommand{\\chap}{Chapter}\n")], "main.tex");
    let c = render_docs(&[("main.tex", "\\documentclass[12pt]{article}\n\\begin{document}\nChapter\n\\end{document}\n")], "main.tex");
    assert_eq!(positioned(&r), positioned(&c));
    let warnings: Vec<&str> = r.v2.diagnostics.iter().map(|d| d.message.as_str()).filter(|m| m.contains("thesis")).collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("LaTeX Warning: Class `thesis' has no \\LoadClass"), "{warnings:?}");
}
