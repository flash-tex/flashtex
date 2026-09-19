//! Project `.sty`/`.cls` files (`crate::packages`, S1 of
//! `docs/proposals/packages-fonts-manifest.md`): `\usepackage{mystyle}`
//! reads `mystyle.sty` from the project documents and runs it through the
//! expansion engine; built-in packages keep their models even when a file
//! of that name is in the project; a class file's `\LoadClass` gives the
//! document the base class's model.
use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::parser::{parse_project, Block, Inline, Parsed, SourceDocument};
use flashtex_compiler::DocumentId;

const MYSTYLE: &str = "\\NeedsTeXFormat{LaTeX2e}\\ProvidesPackage{mystyle}\\newcommand{\\hello}{Hello from mystyle}\\newcommand{\\emphx}[1]{\\textbf{#1}}\n";
const MYCLASS: &str = "\\NeedsTeXFormat{LaTeX2e}\\ProvidesClass{myclass}\\DeclareOption*{\\PassOptionsToClass{\\CurrentOption}{article}}\\ProcessOptions\\relax\\LoadClass[11pt]{article}\\RequirePackage{mystyle}\\newcommand{\\greeting}{Hi}\n";

/// The paragraph words with their bold flag.
fn words(parsed: &Parsed) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    for block in &parsed.blocks {
        let Block::Paragraph(inlines) = block else {
            continue;
        };
        for inline in inlines {
            if let Inline::Text { text, style, .. } = inline {
                out.push((text.clone(), style.bold));
            }
        }
    }
    out
}

fn messages(parsed: &Parsed) -> Vec<String> {
    parsed.diagnostics.iter().map(|d| format!("{}: {}", d.severity.as_str(), d.message)).collect()
}

fn parse<'a>(documents: &[SourceDocument<'a>]) -> Parsed {
    parse_project(documents, "main.tex")
}

#[test]
fn mystyle_defines_commands_for_the_document() {
    let main = "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\n\\hello, \\emphx{world}.\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "mystyle.sty", text: MYSTYLE }]);
    assert_eq!(messages(&parsed), Vec::<String>::new());
    assert_eq!(
        words(&parsed),
        [("Hello", false), ("from", false), ("mystyle", false), (",", false), ("world", true), (".", false)]
            .map(|(w, b)| (w.to_string(), b))
    );
    assert_eq!(parsed.package_files.len(), 1);
    assert_eq!(parsed.package_files[0].0, DocumentId(1));
    let at = parsed.package_files[0].1;
    assert_eq!((at.document, &main[at.start..at.end]), (DocumentId(0), "\\usepackage"));
    assert_eq!(parsed.class_file, None);
    assert_eq!(parsed.packages, Vec::<String>::new(), "a project package is not a parser package");
}

/// The same document without the file: the warning names the search.
#[test]
fn a_missing_package_says_where_it_was_looked_for() {
    let main = "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\n\\hello\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }]);
    let warning = parsed
        .diagnostics
        .iter()
        .find(|d| d.message == "packages mystyle are recognised but not implemented")
        .expect("the pass-through warning");
    assert_eq!(warning.notes, ["no project file found: looked for mystyle.sty next to main.tex"]);
    assert!(parsed.diagnostics.iter().any(|d| d.message.contains("\\hello is not supported")));
    // A built-in package with unsupported options keeps the plain warning.
    let main = "\\documentclass{article}\n\\usepackage[fleqn]{amsmath}\n\\begin{document}\nx\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }]);
    let warning = parsed.diagnostics.iter().find(|d| d.message.starts_with("packages amsmath")).expect("warning");
    assert!(warning.notes.is_empty(), "{:?}", warning.notes);
}

#[test]
fn a_built_in_package_ignores_a_project_file_of_its_name() {
    let main = "\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\n$\\dfrac{1}{2}$ \\hello\n\\end{document}\n";
    let stray = "\\ProvidesPackage{amsmath}\\newcommand{\\hello}{from the stray file}\\hbox{}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "amsmath.sty", text: stray }]);
    assert!(parsed.package_files.is_empty());
    assert_eq!(parsed.packages, ["amsmath"]);
    assert!(parsed.diagnostics.iter().any(|d| d.message.contains("\\hello is not supported")), "{:?}", messages(&parsed));
    assert!(!parsed.diagnostics.iter().any(|d| d.message.contains("amsmath")), "{:?}", messages(&parsed));
}

#[test]
fn myclass_loads_article_at_11pt_and_its_packages() {
    let main = "\\documentclass{myclass}\n\\begin{document}\n\\greeting: \\hello, \\emphx{world}.\n\\end{document}\n";
    let parsed = parse(&[
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "myclass.cls", text: MYCLASS },
        SourceDocument { path: "mystyle.sty", text: MYSTYLE },
    ]);
    assert_eq!(messages(&parsed), Vec::<String>::new());
    assert_eq!(parsed.document_class.as_deref(), Some("article"));
    assert_eq!(parsed.class_options.as_deref(), Some("11pt"));
    assert_eq!(parsed.class_size_pt, Some(11.0));
    assert_eq!(parsed.class_file, Some(DocumentId(1)));
    assert_eq!(parsed.package_files.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [DocumentId(1), DocumentId(2)]);
    assert_eq!(
        words(&parsed),
        [("Hi", false), (":", false), ("Hello", false), ("from", false), ("mystyle", false), (",", false), ("world", true), (".", false)]
            .map(|(w, b)| (w.to_string(), b))
    );
    // The same body under `\documentclass[11pt]{article}` sets the same
    // size: the class file changes nothing but where the size comes from.
    let plain = "\\documentclass[11pt]{article}\n\\usepackage{mystyle}\n\\begin{document}\nHi: \\hello, \\emphx{world}.\n\\end{document}\n";
    let reference = parse(&[SourceDocument { path: "main.tex", text: plain }, SourceDocument { path: "mystyle.sty", text: MYSTYLE }]);
    assert_eq!(reference.class_size_pt, parsed.class_size_pt);
    let constraints = flashtex_compiler::layout::LayoutConstraints::default();
    assert_eq!(reference.preamble_constraints(constraints).font_size_pt, parsed.preamble_constraints(constraints).font_size_pt);
    // (`Hi:` is one word in the reference, `Hi` + `:` here: the colon is a
    // document token after a macro-expanded word.)
    let joined = |parsed: &Parsed| words(parsed).iter().map(|(w, b)| format!("{w}{}", if *b { "*" } else { "" })).collect::<String>();
    assert_eq!(joined(&reference), joined(&parsed));
    // The document's own class options are passed on by `\DeclareOption*`.
    let main = "\\documentclass[12pt]{myclass}\n\\begin{document}\nx\n\\end{document}\n";
    let parsed = parse(&[
        SourceDocument { path: "main.tex", text: main },
        SourceDocument { path: "myclass.cls", text: MYCLASS },
        SourceDocument { path: "mystyle.sty", text: MYSTYLE },
    ]);
    assert_eq!(parsed.class_options.as_deref(), Some("12pt,11pt"));
}

#[test]
fn a_class_without_loadclass_gets_article_and_one_warning() {
    let main = "\\documentclass[12pt]{thesis}\n\\begin{document}\n\\chap\n\\end{document}\n";
    let thesis = "\\ProvidesClass{thesis}\\newcommand{\\chap}{Chapter}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "thesis.cls", text: thesis }]);
    assert_eq!(
        messages(&parsed),
        ["warning: LaTeX Warning: Class `thesis' has no \\LoadClass and loads no standard class; the page layout of `article' is used."]
    );
    assert_eq!(parsed.document_class.as_deref(), Some("article"));
    assert_eq!(parsed.class_options.as_deref(), Some("12pt"));
    assert_eq!(parsed.class_size_pt, Some(12.0));
    assert_eq!(parsed.class_file, Some(DocumentId(1)));
    assert_eq!(words(&parsed), [("Chapter".to_string(), false)]);
}

#[test]
fn package_options_at_catcode_and_endinput() {
    let sty = "\\ProvidesPackage{opts}\n\\DeclareOption{draft}{\\def\\isdraft{yes}}\\DeclareOption{final}{\\def\\isdraft{no}}\n\\ExecuteOptions{final}\\ProcessOptions\\relax\n\\def\\at@x{AT}\\newcommand{\\useat}{\\at@x}\n\\endinput\n\\def\\isdraft{unreachable}\n";
    let main = "\\documentclass{article}\n\\usepackage[draft]{opts}\n\\begin{document}\n\\isdraft \\useat @\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "opts.sty", text: sty }]);
    assert_eq!(messages(&parsed), Vec::<String>::new());
    assert_eq!(words(&parsed).iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>(), ["yes", "AT", "@"]);
    // The class option reaches the package too.
    let main = "\\documentclass[draft]{article}\n\\usepackage{opts}\n\\begin{document}\n\\isdraft\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "opts.sty", text: sty }]);
    assert_eq!(words(&parsed).iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>(), ["yes"]);
    // An undeclared option is LaTeX's error, at the package.
    let main = "\\documentclass{article}\n\\usepackage[bogus]{opts}\n\\begin{document}\nx\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "opts.sty", text: sty }]);
    assert_eq!(messages(&parsed), ["error: LaTeX Error: Unknown option `bogus' for package `opts'."]);
}

#[test]
fn require_package_of_a_loaded_package_is_a_no_op() {
    let main = "\\documentclass{article}\n\\usepackage{mystyle}\n\\RequirePackage{mystyle}\n\\usepackage{mystyle}\n\\begin{document}\n\\hello\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "mystyle.sty", text: MYSTYLE }]);
    assert_eq!(messages(&parsed), Vec::<String>::new());
    assert_eq!(parsed.package_files.len(), 1);
    assert_eq!(words(&parsed).len(), 3);
}

/// A diagnostic inside a package is reported at the package file, with
/// the `\usepackage` site as a label.
#[test]
fn diagnostics_inside_a_package_point_into_it_and_name_the_load_site() {
    let sty = "\\ProvidesPackage{bad}\n\\newcommand{\\hello}{x}\n\\newcommand{\\hello}{y}\n\\def\\body{\\nosuchcommand}\n";
    let main = "\\documentclass{article}\n\\usepackage{bad}\n\\begin{document}\n\\body\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "bad.sty", text: sty }]);
    let redefined = parsed
        .diagnostics
        .iter()
        .find(|d| d.message == "LaTeX Error: Command \\hello already defined.")
        .expect("the engine's error");
    let span = redefined.span.expect("span");
    assert_eq!(span.document, DocumentId(1));
    assert_eq!(&sty[span.start..span.end], "\\newcommand");
    assert_eq!(redefined.labels.len(), 1);
    assert_eq!(redefined.labels[0].text, "bad.sty is loaded here");
    let at = redefined.labels[0].span;
    assert_eq!((at.document, &main[at.start..at.end]), (DocumentId(0), "\\usepackage"));
    // A parser diagnostic for a token expanded from the package: the
    // invocation is in the document, so no label.
    let unknown = parsed.diagnostics.iter().find(|d| d.message.contains("\\nosuchcommand")).expect("unknown command");
    assert_eq!(unknown.span.map(|s| s.document), Some(DocumentId(0)));
    assert_eq!(unknown.severity, Severity::Error);
}
