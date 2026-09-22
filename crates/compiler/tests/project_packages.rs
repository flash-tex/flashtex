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
    // The replacement sites are the two package macros, in the package
    // file (`\emphx`'s `\textbf{` and `}` around its argument are two
    // runs); the pass-through `\documentclass` is no expansion.
    let sites: Vec<(DocumentId, &str)> = parsed
        .expansions
        .iter()
        .map(|site| (site.definition.document, &main[site.invocation.start..site.invocation.end]))
        .collect();
    assert_eq!(sites, [(DocumentId(1), "\\hello"), (DocumentId(1), "\\emphx"), (DocumentId(1), "\\emphx")]);
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

// ---- what a package defined (`crate::package_definitions`) ----------------

use flashtex_compiler::json::{self, Value};
use flashtex_compiler::protocol::handle_line;

/// The `compile_result` payload for `documents`.
fn compile_payload(documents: &[(&str, &str)]) -> Value {
    let docs = documents
        .iter()
        .map(|(path, text)| {
            let mut doc = Value::obj();
            doc.set("path", json::str_(*path));
            doc.set("text", json::str_(*text));
            doc
        })
        .collect();
    let mut payload = Value::obj();
    payload.set("project_id", json::str_("project-packages"));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(docs));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("pkg"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    let reply = json::parse(&handle_line(&json::write(&env))).expect("valid JSON reply");
    reply.get("payload").expect("payload").clone()
}

fn text_of<'a>(span: &Value, documents: &[(&str, &'a str)]) -> &'a str {
    let path = span.get("path").and_then(|v| v.as_str()).expect("path");
    let (start, end) = (span.get("start").and_then(|v| v.as_i64()).unwrap() as usize, span.get("end").and_then(|v| v.as_i64()).unwrap() as usize);
    let text = documents.iter().find(|(p, _)| *p == path).map(|(_, t)| *t).expect("document");
    &text[start..end]
}

/// `Parsed::package_definitions` for the mystyle probe: the file, its
/// `\ProvidesPackage`, the loading command, and the two macros with their
/// shapes and statement spans in `mystyle.sty`.
#[test]
fn mystyle_reports_its_definitions() {
    let main = "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\n\\hello\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "mystyle.sty", text: MYSTYLE }]);
    assert_eq!(parsed.package_definitions.len(), 1);
    let record = &parsed.package_definitions[0];
    assert_eq!((record.document, record.kind), (DocumentId(1), "package"));
    assert_eq!(&main[record.loaded_by.start..record.loaded_by.end], "\\usepackage");
    assert_eq!(record.loaded_by, parsed.package_files[0].1);
    let provides = record.provides.as_ref().expect("provides");
    assert_eq!((provides.name.as_str(), provides.date.as_deref(), provides.span.document), ("mystyle", None, DocumentId(1)));
    assert_eq!(&MYSTYLE[provides.span.start..provides.span.end], "\\ProvidesPackage{mystyle}");
    let rows: Vec<(&str, &str, &str, u8, &str, &str, bool)> = record
        .definitions
        .iter()
        .map(|d| (d.name.as_str(), d.kind, d.definer.as_str(), d.arity, d.signature.as_str(), &MYSTYLE[d.span.start..d.span.end], d.overrides))
        .collect();
    assert_eq!(
        rows,
        [
            ("hello", "macro", "newcommand", 0, "", "\\newcommand{\\hello}{Hello from mystyle}", false),
            ("emphx", "macro", "newcommand", 1, "[1]", "\\newcommand{\\emphx}[1]{\\textbf{#1}}", false),
        ]
    );
    assert!(record.definitions.iter().all(|d| d.span.document == DocumentId(1)));
    // The same records come out of the compile output and stay put across
    // an unchanged recompile.
    let constraints = flashtex_compiler::layout::LayoutConstraints::default();
    let docs = [SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "mystyle.sty", text: MYSTYLE }];
    let output = flashtex_compiler::incremental::compile_full_project(&docs, "main.tex", constraints);
    assert_eq!(output.packages, parsed.package_definitions);
    let mut session = flashtex_compiler::incremental::Session::new();
    session.compile_project(&docs, "main.tex", constraints);
    let again = session.compile_project(&docs, "main.tex", constraints);
    assert_eq!(again.output.packages, parsed.package_definitions);
}

/// The runtime-v1 `metadata.packages` section for the mystyle probe, and
/// its absence for a project without package files.
#[test]
fn compile_result_carries_metadata_packages() {
    let main = "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\n\\hello\n\\end{document}\n";
    let documents = [("main.tex", main), ("mystyle.sty", MYSTYLE)];
    let payload = compile_payload(&documents);
    assert_eq!(payload.get("status").and_then(|v| v.as_str()), Some("ok"));
    let packages = payload.get("metadata").and_then(|m| m.get("packages")).and_then(|p| p.as_arr()).expect("metadata.packages");
    assert_eq!(packages.len(), 1);
    let package = &packages[0];
    assert_eq!(package.get("path").and_then(|v| v.as_str()), Some("mystyle.sty"));
    assert_eq!(package.get("kind").and_then(|v| v.as_str()), Some("package"));
    let provides = package.get("provides").expect("provides");
    assert_eq!(provides.get("name").and_then(|v| v.as_str()), Some("mystyle"));
    assert_eq!(provides.get("date"), Some(&Value::Null));
    assert_eq!(text_of(package.get("loaded_by").unwrap(), &documents), "\\usepackage");
    assert_eq!(package.get("loaded_by").and_then(|s| s.get("path")).and_then(|v| v.as_str()), Some("main.tex"));
    assert_eq!(package.get("options_declared").and_then(|v| v.as_arr()).map(|a| a.len()), Some(0));
    let definitions = package.get("definitions").and_then(|v| v.as_arr()).expect("definitions");
    let rows: Vec<(String, String, String, i64, Value, String, String, bool)> = definitions
        .iter()
        .map(|d| {
            (
                d.get("name").unwrap().as_str().unwrap().to_string(),
                d.get("kind").unwrap().as_str().unwrap().to_string(),
                d.get("definer").unwrap().as_str().unwrap().to_string(),
                d.get("arity").unwrap().as_i64().unwrap(),
                d.get("optional_default").unwrap().clone(),
                d.get("signature").unwrap().as_str().unwrap().to_string(),
                text_of(d.get("span").unwrap(), &documents).to_string(),
                matches!(d.get("overrides"), Some(Value::Bool(true))),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("hello".into(), "macro".into(), "newcommand".into(), 0, Value::Null, "".into(), "\\newcommand{\\hello}{Hello from mystyle}".into(), false),
            ("emphx".into(), "macro".into(), "newcommand".into(), 1, Value::Null, "[1]".into(), "\\newcommand{\\emphx}[1]{\\textbf{#1}}".into(), false),
        ]
    );
    // The exact wire text, for the consumer's decoder.
    let wire = json::write(package.get("definitions").unwrap());
    assert_eq!(
        wire,
        "[{\"arity\":0,\"definer\":\"newcommand\",\"kind\":\"macro\",\"name\":\"hello\",\"optional_default\":null,\"overrides\":false,\"signature\":\"\",\"span\":{\"end\":88,\"path\":\"mystyle.sty\",\"start\":49}},\
          {\"arity\":1,\"definer\":\"newcommand\",\"kind\":\"macro\",\"name\":\"emphx\",\"optional_default\":null,\"overrides\":false,\"signature\":\"[1]\",\"span\":{\"end\":123,\"path\":\"mystyle.sty\",\"start\":88}}]"
    );
    // No package files: no `metadata` key at all.
    let plain = compile_payload(&[("main.tex", "\\documentclass{article}\n\\begin{document}\nx\n\\end{document}\n")]);
    assert_eq!(plain.get("metadata"), None);
}

/// The myclass probe: the class and the package it `\RequirePackage`s,
/// each loaded by the command in its loader, the class's `\DeclareOption*`
/// and `\newcommand`.
#[test]
fn myclass_reports_the_class_and_its_package() {
    let main = "\\documentclass{myclass}\n\\begin{document}\n\\greeting\n\\end{document}\n";
    let documents = [("main.tex", main), ("myclass.cls", MYCLASS), ("mystyle.sty", MYSTYLE)];
    let payload = compile_payload(&documents);
    let packages = payload.get("metadata").and_then(|m| m.get("packages")).and_then(|p| p.as_arr()).expect("metadata.packages");
    let summary: Vec<(String, String, String, String, Vec<String>, Vec<String>)> = packages
        .iter()
        .map(|p| {
            (
                p.get("path").unwrap().as_str().unwrap().to_string(),
                p.get("kind").unwrap().as_str().unwrap().to_string(),
                p.get("loaded_by").unwrap().get("path").unwrap().as_str().unwrap().to_string(),
                text_of(p.get("loaded_by").unwrap(), &documents).to_string(),
                p.get("options_declared").unwrap().as_arr().unwrap().iter().map(|o| o.get("name").unwrap().as_str().unwrap().to_string()).collect(),
                p.get("definitions").unwrap().as_arr().unwrap().iter().map(|d| d.get("name").unwrap().as_str().unwrap().to_string()).collect(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            ("myclass.cls".into(), "class".into(), "main.tex".into(), "\\documentclass".into(), vec!["*".into()], vec!["greeting".into()]),
            ("mystyle.sty".into(), "package".into(), "myclass.cls".into(), "\\RequirePackage".into(), vec![], vec!["hello".into(), "emphx".into()]),
        ]
    );
    let class = &packages[0];
    assert_eq!(class.get("provides").unwrap().get("name").and_then(|v| v.as_str()), Some("myclass"));
    let option = &class.get("options_declared").unwrap().as_arr().unwrap()[0];
    assert_eq!(text_of(option.get("span").unwrap(), &documents), "\\DeclareOption*{\\PassOptionsToClass{\\CurrentOption}{article}}");
}

/// `appendix` is the first package flipped from a built-in no-op to the
/// real vendored file (`tex-expansion/vendor-packages/appendix.sty`, plan4):
/// `\usepackage{appendix}` is consumed by the engine (no "recognised but
/// not implemented" warning) and the package is really executed, so its
/// `\appendixname` expands to `Appendix` instead of being diagnosed.
#[test]
fn appendix_runs_the_vendored_package() {
    let main = "\\documentclass{article}\n\\usepackage{appendix}\n\\begin{document}\n\\appendixname\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }]);
    assert!(
        parsed.diagnostics.iter().all(|d| !d.message.contains("recognised but not implemented")),
        "{:?}",
        messages(&parsed)
    );
    assert!(
        parsed.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "{:?}",
        messages(&parsed)
    );
    assert_eq!(words(&parsed), [("Appendix".to_string(), false)]);
}

/// The real file's option declarations run: `[toc]` is accepted (no
/// "Unknown option" error) and the load stays silent apart from the
/// package's own diagnostics.
#[test]
fn appendix_options_are_processed_by_the_real_file() {
    let main = "\\documentclass{article}\n\\usepackage[toc]{appendix}\n\\begin{document}\nx\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }]);
    assert!(
        parsed.diagnostics.iter().all(|d| !d.message.contains("recognised but not implemented") && !d.message.contains("Unknown option")),
        "{:?}",
        messages(&parsed)
    );
    let main = "\\documentclass{article}\n\\usepackage[bogus]{appendix}\n\\begin{document}\nx\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }]);
    assert!(
        parsed.diagnostics.iter().any(|d| d.message.contains("Unknown option `bogus'")),
        "{:?}",
        messages(&parsed)
    );
}

/// A project file still wins over the vendored one: the vendored
/// `appendix.sty` never loads, so its `\appendixname` stays undefined.
#[test]
fn a_project_appendix_sty_wins_over_the_vendored_one() {
    let main = "\\documentclass{article}\n\\usepackage{appendix}\n\\begin{document}\n\\fromproject\n\\end{document}\n";
    let sty = "\\ProvidesPackage{appendix}\\newcommand{\\fromproject}{project appendix}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "appendix.sty", text: sty }]);
    assert_eq!(messages(&parsed), Vec::<String>::new());
    assert_eq!(words(&parsed).iter().map(|(w, _)| w.as_str()).collect::<Vec<_>>(), ["project", "appendix"]);
}

/// A diagnostic inside `b.sty`, loaded by `a.sty`, loaded by `main.tex`:
/// both loaders are labelled, innermost first.
#[test]
fn diagnostics_in_a_nested_package_name_the_whole_load_chain() {
    let a = "\\ProvidesPackage{a}\n\\RequirePackage{b}\n";
    let b = "\\ProvidesPackage{b}\n\\newcommand{\\dup}{x}\n\\newcommand{\\dup}{y}\n";
    let main = "\\documentclass{article}\n\\usepackage{a}\n\\begin{document}\n\\dup\n\\end{document}\n";
    let parsed = parse(&[SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "a.sty", text: a }, SourceDocument { path: "b.sty", text: b }]);
    let redefined = parsed.diagnostics.iter().find(|d| d.message == "LaTeX Error: Command \\dup already defined.").expect("the engine's error");
    assert_eq!(redefined.span.map(|s| s.document), Some(DocumentId(2)));
    let labels: Vec<(String, DocumentId, &str)> = redefined
        .labels
        .iter()
        .map(|l| {
            let text = match l.span.document {
                DocumentId(0) => &main[l.span.start..l.span.end],
                DocumentId(1) => &a[l.span.start..l.span.end],
                _ => "",
            };
            (l.text.clone(), l.span.document, text)
        })
        .collect();
    assert_eq!(labels, [("b.sty is loaded here".to_string(), DocumentId(1), "\\RequirePackage"), ("a.sty is loaded here".to_string(), DocumentId(0), "\\usepackage")]);
    // The records agree: `b.sty` is loaded from `a.sty`.
    let chain: Vec<(DocumentId, DocumentId)> = parsed.package_definitions.iter().map(|r| (r.document, r.loaded_by.document)).collect();
    assert_eq!(chain, [(DocumentId(1), DocumentId(0)), (DocumentId(2), DocumentId(1))]);
}
