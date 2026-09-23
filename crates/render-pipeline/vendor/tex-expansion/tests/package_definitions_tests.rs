//! What a package file defines (`src/package_defs.rs`): every definer kind
//! recorded with its name, shape and the span of the whole statement, only
//! at the file's outermost level, and the load chain through nested
//! `\RequirePackage`.

use std::collections::HashMap;
use std::rc::Rc;

use flashtex_tex_expansion::{DefinitionKind, Edit, Engine, IncrementalExpander, Limits, OpenedFile, PackageDefinition};

fn engine_with(src: &str, files: &[(&str, &str)]) -> Engine {
    let files: HashMap<String, String> = files.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect();
    let mut engine = Engine::new(src);
    engine.set_package_reader(Rc::new(move |name, ext| files.get(&format!("{name}.{ext}")).cloned()));
    engine
}

/// Run `src` with `files` served, returning the opened files.
fn load(src: &str, files: &[(&str, &str)]) -> Vec<OpenedFile> {
    let mut engine = engine_with(src, files);
    let _ = engine.run();
    let diagnostics: Vec<String> = engine.take_diagnostics().iter().map(|d| d.message.clone()).collect();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    engine.opened_package_files().to_vec()
}

const MAIN: &str = "\\documentclass{article}\n\\usepackage{defs}\n\\begin{document}x\\end{document}\n";

/// `(name, kind, definer, arity, signature, statement text, overrides)`
/// of every definition, the statement text cut from `sty` by the span.
fn rows<'a>(file: &'a OpenedFile, sty: &'a str) -> Vec<(&'a str, DefinitionKind, &'a str, u8, &'a str, &'a str, bool)> {
    file.definitions
        .iter()
        .map(|d| {
            assert_eq!(d.span.source_id, file.source_id, "{d:?}");
            (
                d.name.as_str(),
                d.kind,
                d.definer.as_str(),
                d.arity,
                d.signature.as_str(),
                &sty[d.span.start as usize..d.span.end as usize],
                d.overrides,
            )
        })
        .collect()
}

#[test]
fn every_definer_kind_is_recorded_with_its_statement_span() {
    let sty = "\\ProvidesPackage{defs}[2024/01/02 v1.3 my macros]\n\
               \\newcommand{\\hello}{Hello}\n\
               \\newcommand*\\emphx[2][x]{\\textbf{#1#2}}\n\
               \\renewcommand{\\hello}{Bye}\n\
               \\providecommand{\\hello}{never}\n\
               \\providecommand{\\fresh}{yes}\n\
               \\DeclareRobustCommand{\\robust}[1]{#1}\n\
               \\long\\def\\body#1\\stop{#1}\n\
               \\edef\\once{a}\\gdef\\twice{b}\\global\\let\\alias\\emphx\n\
               \\let\\hello=\\relax\n\
               \\newenvironment{crate}[1][o]{begin}{end}\n\
               \\renewenvironment{crate}{b}{e}\n\
               \\newif\\ifdraft\n\
               \\newcounter{thing}[section]\n\
               \\newlength{\\gap}\n\
               \\newcount\\reg\n\
               \\newtheorem{thm}{Theorem}[section]\n\
               \\newtheorem{lem}[thm]{Lemma}\n\
               \\DeclareMathOperator*{\\argmax}{arg\\,max}\n\
               \\DeclareOption{draft}{\\def\\isdraft{yes}}\\DeclareOption*{\\OptionNotUsed}\\ProcessOptions\\relax\n\
               \\NewDocumentCommand{\\xp}{O{x} m}{#1#2}\n\
               \\NewDocumentEnvironment{xenv}{s o}{b}{e}\n";
    let files = load(MAIN, &[("defs.sty", sty)]);
    assert_eq!(files.len(), 1);
    let file = &files[0];
    let provides = file.provides.as_ref().expect("\\ProvidesPackage");
    assert_eq!(provides.name, "defs");
    assert_eq!(provides.date.as_deref(), Some("2024/01/02"));
    assert_eq!(provides.version.as_deref(), Some("v1.3"));
    assert_eq!(provides.description.as_deref(), Some("my macros"));
    assert_eq!(&sty[provides.span.start as usize..provides.span.end as usize], "\\ProvidesPackage{defs}[2024/01/02 v1.3 my macros]");
    assert_eq!(
        file.options.iter().map(|o| (o.name.as_str(), &sty[o.span.start as usize..o.span.end as usize])).collect::<Vec<_>>(),
        [("draft", "\\DeclareOption{draft}{\\def\\isdraft{yes}}"), ("*", "\\DeclareOption*{\\OptionNotUsed}")]
    );
    use DefinitionKind::*;
    assert_eq!(
        rows(file, sty),
        [
            ("hello", Macro, "newcommand", 0, "", "\\newcommand{\\hello}{Hello}", false),
            ("emphx", Macro, "newcommand", 2, "[2][x]", "\\newcommand*\\emphx[2][x]{\\textbf{#1#2}}", false),
            ("hello", Macro, "renewcommand", 0, "", "\\renewcommand{\\hello}{Bye}", true),
            // `\providecommand` of a defined name defines nothing.
            ("fresh", Macro, "providecommand", 0, "", "\\providecommand{\\fresh}{yes}", false),
            ("robust", Macro, "DeclareRobustCommand", 1, "[1]", "\\DeclareRobustCommand{\\robust}[1]{#1}", false),
            ("body", Macro, "def", 1, "#1\\stop", "\\long\\def\\body#1\\stop{#1}", false),
            ("once", Macro, "edef", 0, "", "\\edef\\once{a}", false),
            ("twice", Macro, "gdef", 0, "", "\\gdef\\twice{b}", false),
            // `\let` reports the copied macro's shape.
            ("alias", Macro, "let", 2, "[2][x]", "\\global\\let\\alias\\emphx", false),
            ("hello", Macro, "let", 0, "", "\\let\\hello=\\relax", true),
            ("crate", Environment, "newenvironment", 1, "[1][o]", "\\newenvironment{crate}[1][o]{begin}{end}", false),
            ("crate", Environment, "renewenvironment", 0, "", "\\renewenvironment{crate}{b}{e}", true),
            ("ifdraft", Conditional, "newif", 0, "", "\\newif\\ifdraft", false),
            ("drafttrue", Conditional, "newif", 0, "", "\\newif\\ifdraft", false),
            ("draftfalse", Conditional, "newif", 0, "", "\\newif\\ifdraft", false),
            ("thing", Counter, "newcounter", 0, "", "\\newcounter{thing}[section]", false),
            ("gap", Length, "newlength", 0, "", "\\newlength{\\gap}", false),
            ("reg", Register, "newcount", 0, "", "\\newcount\\reg", false),
            ("thm", Theorem, "newtheorem", 0, "", "\\newtheorem{thm}{Theorem}[section]", false),
            ("lem", Theorem, "newtheorem", 0, "[thm]", "\\newtheorem{lem}[thm]{Lemma}", false),
            ("argmax", MathOperator, "DeclareMathOperator", 0, "*{arg\\,max}", "\\DeclareMathOperator*{\\argmax}{arg\\,max}", false),
            ("xp", Macro, "NewDocumentCommand", 2, "O{x} m", "\\NewDocumentCommand{\\xp}{O{x} m}{#1#2}", false),
            ("xenv", Environment, "NewDocumentEnvironment", 2, "s o", "\\NewDocumentEnvironment{xenv}{s o}{b}{e}", false),
        ]
    );
    // `\let\alias\emphx` copies a command with an optional argument: the
    // outer `\emphx` macro takes none itself, but reports the LaTeX shape.
    let emphx = &file.definitions[1];
    assert_eq!(emphx.optional_default.as_deref(), Some("x"));
    let alias = file.definitions.iter().find(|d| d.name == "alias").unwrap();
    assert_eq!(alias.arity, 2, "{alias:?}");
    let thing = file.definitions.iter().find(|d| d.name == "thing").unwrap();
    assert_eq!(thing.within.as_deref(), Some("section"));
    let thm = file.definitions.iter().find(|d| d.name == "thm").unwrap();
    assert_eq!((thm.title.as_deref(), thm.within.as_deref()), (Some("Theorem"), Some("section")));
    let lem = file.definitions.iter().find(|d| d.name == "lem").unwrap();
    assert_eq!((lem.title.as_deref(), lem.within.as_deref()), (Some("Lemma"), None));
    let boxes: Vec<_> = file.definitions.iter().filter(|d| d.name == "crate").collect();
    assert_eq!(boxes[0].optional_default.as_deref(), Some("o"));
}

/// The document's own definitions, a macro body's (`\helper` is recorded,
/// what its body defines is not), an option's code run by
/// `\ProcessOptions`, an `\AtEndOfPackage` hook and text after `\endinput`
/// are not the file's outermost-level statements.
#[test]
fn only_outermost_level_statements_of_the_file_are_recorded() {
    let sty = "\\ProvidesPackage{defs}\n\
               \\DeclareOption{draft}{\\newcommand{\\fromoption}{o}}\\ProcessOptions\\relax\n\
               \\def\\helper{\\newcommand{\\frombody}{b}}\\helper\n\
               \\AtEndOfPackage{\\newcommand{\\fromhook}{h}}\n\
               \\ifx\\relax\\relax\\newcommand{\\fromif}{i}\\fi\n\
               \\begingroup\\def\\fromgroup{g}\\endgroup\n\
               \\expandafter\\newcommand\\csname fromcsname\\endcsname{c}\n\
               \\newcommand{\\last}{l}\\endinput\n\
               \\newcommand{\\unreachable}{u}\n";
    let main = "\\documentclass{article}\n\\usepackage[draft]{defs}\n\\newcommand{\\indoc}{d}\n\\begin{document}x\\end{document}\n";
    let files = load(main, &[("defs.sty", sty)]);
    let names: Vec<&str> = files[0].definitions.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["helper", "fromif", "fromgroup", "fromcsname", "last"]);
    let last = files[0].definitions.last().unwrap();
    assert_eq!(&sty[last.span.start as usize..last.span.end as usize], "\\newcommand{\\last}{l}");
}

/// `\def` over a taken name, `\let` over one, and a `\newcommand` that
/// errors on a taken name (nothing defined, nothing recorded).
#[test]
fn overrides_is_whether_the_name_had_a_meaning() {
    let sty = "\\def\\a{1}\\def\\a{2}\\let\\b\\a\\let\\b\\relax\\newcommand{\\a}{3}\\newcommand{\\c}{4}\\renewcommand{\\c}{5}\n";
    let mut engine = engine_with(MAIN, &[("defs.sty", sty)]);
    let _ = engine.run();
    let messages: Vec<String> = engine.take_diagnostics().iter().map(|d| d.message.clone()).collect();
    assert_eq!(messages, ["LaTeX Error: Command \\a already defined."]);
    let files = engine.opened_package_files().to_vec();
    let rows: Vec<(&str, &str, bool)> = files[0].definitions.iter().map(|d| (d.name.as_str(), d.definer.as_str(), d.overrides)).collect();
    assert_eq!(rows, [("a", "def", false), ("a", "def", true), ("b", "let", false), ("b", "let", true), ("c", "newcommand", false), ("c", "renewcommand", true)]);
}

/// A statement's span ends at its last token even when the definer looked
/// one token further (`\newcounter` looking for `[`), and a comment or
/// blank line after it is not part of it.
#[test]
fn statement_spans_stop_at_the_last_argument() {
    let sty = "\\newcounter{one}%\n\\newcounter{two}\n\n\\newcommand{\\x}{1} % trailing\n\\let\\y\\x\n\\newlength{\\z}\n";
    let files = load(MAIN, &[("defs.sty", sty)]);
    let texts: Vec<&str> = files[0].definitions.iter().map(|d| &sty[d.span.start as usize..d.span.end as usize]).collect();
    assert_eq!(texts, ["\\newcounter{one}", "\\newcounter{two}", "\\newcommand{\\x}{1}", "\\let\\y\\x", "\\newlength{\\z}"]);
}

/// `main.tex:\usepackage{a}` -> `a.sty:\RequirePackage{b}` -> `b.sty`: each
/// file's `loaded_at` is in its loader, and each file's definitions are
/// its own.
#[test]
fn nested_loads_record_the_chain() {
    let a = "\\ProvidesPackage{a}\\RequirePackage{b}\\newcommand{\\froma}{a}\n";
    let b = "\\ProvidesPackage{b}\\newcommand{\\fromb}{b}\n";
    let files = load(MAIN.replace("{defs}", "{a}").as_str(), &[("a.sty", a), ("b.sty", b)]);
    assert_eq!(files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["a.sty", "b.sty"]);
    let main = MAIN.replace("{defs}", "{a}");
    assert_eq!(files[0].loaded_at.source_id, 0);
    assert_eq!(&main[files[0].loaded_at.start as usize..files[0].loaded_at.end as usize], "\\usepackage");
    assert_eq!(files[1].loaded_at.source_id, files[0].source_id);
    assert_eq!(&a[files[1].loaded_at.start as usize..files[1].loaded_at.end as usize], "\\RequirePackage");
    assert_eq!(files[0].definitions.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["froma"]);
    assert_eq!(files[1].definitions.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["fromb"]);
    assert_eq!(files[0].provides.as_ref().map(|p| p.name.as_str()), Some("a"));
    assert_eq!(files[1].provides.as_ref().map(|p| p.name.as_str()), Some("b"));
}

/// The incremental expander carries the records: an edit after the load
/// keeps them, an edit before it re-reads the file and records it again.
#[test]
fn the_incremental_expander_keeps_the_records() {
    let sty = "\\ProvidesPackage{defs}\\newcommand{\\hello}{Hello}\n";
    let files: HashMap<String, String> = HashMap::from([("defs.sty".to_string(), sty.to_string())]);
    let init: Rc<dyn Fn(&mut Engine)> = Rc::new(move |engine| {
        let files = files.clone();
        engine.set_package_reader(Rc::new(move |name, ext| files.get(&format!("{name}.{ext}")).cloned()));
    });
    let src = "\\documentclass{article}\n\\usepackage{defs}\n\\begin{document}\nfirst paragraph\n\nsecond\\end{document}\n";
    let mut expander = IncrementalExpander::with_host(src, Limits::default(), 8, init);
    let defs = |expander: &IncrementalExpander| -> Vec<PackageDefinition> {
        expander.opened_package_files().flat_map(|f| f.definitions.clone()).collect()
    };
    let before = defs(&expander);
    assert_eq!(before.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["hello"]);
    let at = src.find("second").unwrap();
    let stats = expander.edit(&Edit { start: at, end: at, replacement: "another ".into() });
    assert!(stats.restarted_from > 0, "{stats:?}");
    assert_eq!(defs(&expander), before, "an edit after the load keeps the record");
    let stats = expander.edit(&Edit { start: 0, end: 0, replacement: "% comment\n".into() });
    assert_eq!(stats.restarted_from, 0, "{stats:?}");
    assert_eq!(defs(&expander), before, "an edit before the load records the same file again");
}
