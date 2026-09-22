//! The package/class kernel (`src/latex_packages.rs`): reading project
//! `.sty`/`.cls` files through a host package reader, ltclass.dtx option
//! processing, and the pass-through of everything the host declines.
//!
//! Expected texts are what pdflatex (MacTeX 2026) typesets or logs for the
//! same files; the oracle was consulted by hand, not run from here.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use flashtex_tex_expansion::{
    is_group_token, tokens_to_display_string, Engine, Severity, Token, TokenKind,
};

/// An engine whose package reader serves `files` (`"name.ext"` -> text).
fn engine_with(src: &str, files: &[(&str, &str)]) -> Engine {
    let files: HashMap<String, String> = files.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect();
    let mut engine = Engine::new(src);
    engine.set_package_reader(Rc::new(move |name, ext| files.get(&format!("{name}.{ext}")).cloned()));
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
    tokens_to_display_string(&content).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn run(src: &str, files: &[(&str, &str)]) -> (String, Vec<String>) {
    let mut engine = engine_with(src, files);
    let tokens = engine.run();
    let diagnostics = engine.take_diagnostics().into_iter().map(|d| format!("{:?}: {}", d.severity, d.message)).collect();
    (text(&tokens), diagnostics)
}

const MYSTYLE: &str = r"\NeedsTeXFormat{LaTeX2e}\ProvidesPackage{mystyle}\newcommand{\hello}{Hello from mystyle}\newcommand{\emphx}[1]{\textbf{#1}}";

#[test]
fn a_project_sty_defines_commands_for_the_document() {
    let (out, diags) = run(
        "\\documentclass{article}\n\\usepackage{mystyle}\n\\begin{document}\n\\hello, \\emphx{world}.\n\\end{document}\n",
        &[("mystyle.sty", MYSTYLE)],
    );
    assert_eq!(out, "\\documentclass article \\document Hello from mystyle, \\textbf world. \\enddocument");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn a_declined_package_passes_through_with_its_own_tokens() {
    let src = "\\documentclass[11pt]{article}\n\\usepackage[fleqn]{amsmath}\n\\begin{document}x\\end{document}";
    let mut engine = engine_with(src, &[]);
    let mut tokens = Vec::new();
    let mut origins = Vec::new();
    while let Some((token, origin)) = engine.next_content_token_with_origin() {
        tokens.push(token);
        origins.push(origin);
    }
    // Every token of the two commands is emitted, with the exact span it
    // was read from and no invocation origin: what the typesetting layer
    // saw before the kernel existed.
    let preamble_end = src.find("\\begin").unwrap() as u32;
    let preamble: Vec<&Token> = tokens
        .iter()
        .zip(&origins)
        .filter(|(t, _)| t.span.source_id == 0 && t.span.start < preamble_end)
        .filter(|(t, _)| !matches!(&t.kind, TokenKind::ControlSequence(n) if n == "relax"))
        .inspect(|(t, origin)| assert_eq!(**origin, None, "{t:?}"))
        .map(|(t, _)| t)
        .collect();
    for t in &preamble {
        let bytes = &src[t.span.start as usize..t.span.end as usize];
        match &t.kind {
            TokenKind::Char(' ', _) => assert!(bytes.trim().is_empty(), "{t:?}"),
            TokenKind::Char(c, _) => assert_eq!(bytes, c.to_string(), "{t:?}"),
            TokenKind::ControlSequence(name) => assert_eq!(bytes, format!("\\{name}"), "{t:?}"),
            other => panic!("{other:?}"),
        }
    }
    // `\documentclass[11pt]{article}` is 16 tokens, `\usepackage[fleqn]{amsmath}`
    // 17, and each of the two newlines one space.
    assert_eq!(preamble.len(), 16 + 17 + 2);
    assert_eq!(&tokens_to_display_string(std::slice::from_ref(preamble[0])), "\\documentclass ");
    assert!(engine.take_diagnostics().is_empty());
    // The pass-through is still recorded as loaded: `\@ifpackageloaded`
    // and `\@ifclasswith` see it.
    let (out, _) = run(
        "\\documentclass[11pt]{article}\\usepackage[fleqn]{amsmath}\\makeatletter\\@ifpackageloaded{amsmath}{A}{-}\\@ifpackagewith{amsmath}{fleqn}{B}{-}\\@ifclasswith{article}{11pt}{C}{-}\\@ifpackageloaded{nope}{-}{D}",
        &[],
    );
    assert!(out.ends_with("ABCD"), "{out}");
}

#[test]
fn require_package_and_load_class_pass_through_as_usepackage_and_documentclass() {
    let (out, diags) = run("\\RequirePackage{amsmath}\\LoadClass[11pt]{article}", &[]);
    assert_eq!(out, "\\usepackage amsmath\\documentclass [11pt]article");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn a_missing_package_passes_through_untouched() {
    let (out, diags) = run("\\usepackage[x]{nothere}\\hello", &[("mystyle.sty", MYSTYLE)]);
    assert_eq!(out, "\\usepackage [x]nothere\\hello");
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn mixed_lists_load_what_exists_and_pass_the_rest_through_in_order() {
    let (out, diags) = run("\\usepackage{amsmath, mystyle,graphicx}\\hello", &[("mystyle.sty", MYSTYLE)]);
    assert_eq!(out, "\\usepackage amsmath\\usepackage graphicxHello from mystyle");
    assert!(diags.is_empty(), "{diags:?}");
    // With options, each declined name gets them; the loaded package
    // reports the option it does not declare, as LaTeX does.
    let (out, diags) = run("\\usepackage[opt]{amsmath,mystyle,graphicx}\\hello", &[("mystyle.sty", MYSTYLE)]);
    assert_eq!(out, "\\usepackage [opt]amsmath\\usepackage [opt]graphicxHello from mystyle");
    assert_eq!(diags, vec!["Error: LaTeX Error: Unknown option `opt' for package `mystyle'."]);
}

#[test]
fn options_are_declared_processed_and_passed() {
    let sty = r"\ProvidesPackage{opts}
\DeclareOption{draft}{\def\isdraft{yes}}
\DeclareOption{final}{\def\isdraft{no}}
\DeclareOption*{\edef\other{\CurrentOption}}
\ExecuteOptions{final}
\ProcessOptions\relax
";
    // The class option `draft` reaches the package (`\@classoptionslist`).
    let (out, diags) = run("\\documentclass[draft]{article}\\usepackage{opts}\\isdraft", &[("opts.sty", sty)]);
    assert_eq!(out, "\\documentclass [draft]article yes");
    assert!(diags.is_empty(), "{diags:?}");
    // `\ExecuteOptions{final}` is the default; the explicit option wins.
    let (out, _) = run("\\documentclass{article}\\usepackage{opts}\\isdraft", &[("opts.sty", sty)]);
    assert_eq!(out, "\\documentclass article no");
    let (out, _) = run("\\documentclass{article}\\usepackage[draft]{opts}\\isdraft", &[("opts.sty", sty)]);
    assert_eq!(out, "\\documentclass article yes");
    // `\DeclareOption*` gets what is not declared; `\PassOptionsToPackage`
    // options come first.
    let (out, _) = run(
        "\\documentclass{article}\\PassOptionsToPackage{twelve}{opts}\\usepackage[draft]{opts}\\other\\isdraft",
        &[("opts.sty", sty)],
    );
    assert_eq!(out, "\\documentclass article twelveyes");
}

#[test]
fn an_undeclared_option_is_a_latex_error() {
    let sty = "\\ProvidesPackage{strict}\\DeclareOption{a}{}\\ProcessOptions\\relax";
    let (_, diags) = run("\\usepackage[a,bogus]{strict}", &[("strict.sty", sty)]);
    assert_eq!(diags, vec!["Error: LaTeX Error: Unknown option `bogus' for package `strict'."]);
    // A package that never processes its options reports every one.
    let (_, diags) = run("\\usepackage[a]{lazy}", &[("lazy.sty", "\\ProvidesPackage{lazy}")]);
    assert_eq!(diags, vec!["Error: LaTeX Error: Unknown option `a' for package `lazy'."]);
    // A class ignores unknown options (`\OptionNotUsed`).
    let (_, diags) = run("\\documentclass[a4paper]{cls}", &[("cls.cls", "\\ProvidesClass{cls}\\ProcessOptions\\relax\\LoadClass{article}")]);
    assert!(diags.is_empty(), "{diags:?}");
}

#[test]
fn a_second_load_is_a_no_op_and_different_options_clash() {
    let sty = "\\ProvidesPackage{once}\\newcommand{\\n}{1}\\DeclareOption{a}{}\\DeclareOption{b}{}\\ProcessOptions\\relax";
    let (out, diags) = run("\\usepackage[a]{once}\\RequirePackage[a]{once}\\usepackage{once}\\n", &[("once.sty", sty)]);
    assert_eq!(out, "1");
    assert!(diags.is_empty(), "{diags:?}");
    let (_, diags) = run("\\usepackage[a]{once}\\usepackage[b]{once}", &[("once.sty", sty)]);
    assert_eq!(diags, vec!["Error: LaTeX Error: Option clash for package once."]);
}

#[test]
fn a_class_loads_its_base_class_with_the_passed_options() {
    let cls = r"\NeedsTeXFormat{LaTeX2e}\ProvidesClass{myclass}
\DeclareOption*{\PassOptionsToClass{\CurrentOption}{article}}
\ProcessOptions\relax
\LoadClass[11pt]{article}
\RequirePackage{mystyle}
\newcommand{\greeting}{Hi}";
    let (out, diags) = run(
        "\\documentclass[a4paper]{myclass}\\begin{document}\\greeting: \\hello\\end{document}",
        &[("myclass.cls", cls), ("mystyle.sty", MYSTYLE)],
    );
    assert_eq!(out, "\\documentclass [a4paper,11pt]article \\document Hi: Hello from mystyle\\enddocument");
    assert!(diags.is_empty(), "{diags:?}");
    // `\@ifclassloaded` sees both the class file and its base.
    let (out, _) = run(
        "\\documentclass{myclass}\\makeatletter\\@ifclassloaded{myclass}{A}{-}\\@ifclassloaded{article}{B}{-}\\@ifclasswith{article}{11pt}{C}{-}",
        &[("myclass.cls", cls), ("mystyle.sty", MYSTYLE)],
    );
    assert!(out.ends_with("ABC"), "{out}");
}

#[test]
fn a_class_without_loadclass_falls_back_to_article_with_one_warning() {
    let (out, diags) = run(
        "\\documentclass[12pt]{thesis}\\chap",
        &[("thesis.cls", "\\ProvidesClass{thesis}\\newcommand{\\chap}{chap}")],
    );
    assert_eq!(out, "\\documentclass [12pt]articlechap");
    assert_eq!(
        diags,
        vec!["Warning: LaTeX Warning: Class `thesis' has no \\LoadClass and loads no standard class; the page layout of `article' is used."]
    );
}

#[test]
fn at_is_a_letter_inside_the_file_and_restored_after() {
    let sty = "\\ProvidesPackage{at}\\def\\at@x{AT}\\newcommand{\\useat}{\\at@x}";
    // `@` is other in the document before and after the load.
    let (out, diags) = run("\\usepackage{at}@\\useat@", &[("at.sty", sty)]);
    assert_eq!(out, "@AT@");
    assert!(diags.is_empty(), "{diags:?}");
    // A document already under `\makeatletter` stays there.
    let (out, _) = run("\\makeatletter\\usepackage{at}\\at@x", &[("at.sty", sty)]);
    assert_eq!(out, "AT");
}

#[test]
fn endinput_stops_the_file() {
    let sty = "\\ProvidesPackage{e}\\def\\a{one}\\endinput\n\\def\\a{two}";
    let (out, _) = run("\\usepackage{e}\\a\\a", &[("e.sty", sty)]);
    assert_eq!(out, "oneone");
}

#[test]
fn hooks_messages_and_queries() {
    let helper = r"\ProvidesPackage{helper}[2024/01/01 v2 helper]
\@ifpackageloaded{outer}{\def\seen{yes}}{\def\seen{no}}
\@ifpackagewith{outer}{draft}{\def\with{yes}}{\def\with{no}}
\AtEndOfPackage{\def\hook{hook}}
\PackageWarning{helper}{Careful with \CurrentOption}
\PackageInfo{helper}{silent}
\ClassWarningNoLine{helper}{Not a class}
\def\hook{early}";
    let outer = "\\ProvidesPackage{outer}\\DeclareOption{draft}{}\\ProcessOptions\\relax\\RequirePackage{helper}";
    let (out, diags) = run(
        "\\usepackage[draft]{outer}\\seen\\with\\hook\\makeatletter\\@ifpackagelater{helper}{2023/12/31}{L}{-}\\@ifpackagelater{helper}{2024/06/01}{-}{E}",
        &[("outer.sty", outer), ("helper.sty", helper)],
    );
    assert_eq!(out, "yesyeshookLE");
    assert_eq!(
        diags,
        vec![
            "Warning: Package helper Warning: Careful with.",
            "Warning: Class helper Warning: Not a class.",
        ]
    );
}

#[test]
fn needs_tex_format_and_provides_checks() {
    let (_, diags) = run("\\usepackage{old}", &[("old.sty", "\\NeedsTeXFormat{LaTeX2e}[2030/01/01]\\ProvidesPackage{old}")]);
    assert_eq!(diags, vec!["Warning: LaTeX Warning: You have requested release `2030/01/01' of LaTeX, but only release `2025-11-01' is available."]);
    let (out, diags) = run("\\usepackage{plainonly}\\x", &[("plainonly.sty", "\\NeedsTeXFormat{plain}\\def\\x{reached}")]);
    assert_eq!(out, "\\x");
    assert_eq!(diags, vec!["Error: LaTeX Error: This file needs format `plain' but this is `LaTeX2e'."]);
    let (_, diags) = run("\\usepackage{a}", &[("a.sty", "\\ProvidesPackage{b}")]);
    assert_eq!(diags, vec!["Warning: LaTeX Warning: You have requested package `a', but the package provides `b'."]);
    let (_, diags) = run("\\usepackage{v}[2030/01/01]", &[("v.sty", "\\ProvidesPackage{v}[2020/01/01]")]);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].contains("but only version `2020/01/01' is available"), "{diags:?}");
}

#[test]
fn opened_files_carry_their_own_source_ids_and_load_sites() {
    let src = "\\documentclass{article}\n\\usepackage{mystyle}\n\\hello";
    let mut engine = engine_with(src, &[("mystyle.sty", MYSTYLE)]);
    let tokens = engine.run();
    let opened = engine.opened_package_files().to_vec();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].name, "mystyle.sty");
    let at = src.find("\\usepackage").unwrap() as u32;
    assert_eq!((opened[0].loaded_at.source_id, opened[0].loaded_at.start, opened[0].loaded_at.end), (0, at, at + 11));
    // The replacement text of `\hello` comes from the package file.
    let hello = tokens.iter().find(|t| matches!(t.kind, TokenKind::Char('H', _))).unwrap();
    assert_eq!(hello.span.source_id, opened[0].source_id);
    assert_eq!(&MYSTYLE[hello.span.start as usize..hello.span.end as usize], "H");
}

#[test]
fn diagnostics_inside_a_package_point_into_the_package() {
    let sty = "\\ProvidesPackage{bad}\n\\newcommand{\\relax}{x}\n";
    let mut engine = engine_with("\\usepackage{bad}", &[("bad.sty", sty)]);
    let _ = engine.run();
    let diags = engine.take_diagnostics();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, Severity::Error);
    assert_eq!(diags[0].span.source_id, engine.opened_package_files()[0].source_id);
    assert_eq!(&sty[diags[0].span.start as usize..diags[0].span.end as usize], "\\newcommand");
}

#[test]
fn the_reader_is_asked_by_name_and_extension() {
    let asked = Rc::new(RefCell::new(Vec::new()));
    let log = asked.clone();
    let mut engine = Engine::new("\\documentclass{article}\\usepackage{a,b}\\RequirePackage{c}\\LoadClass{d}");
    engine.set_package_reader(Rc::new(move |name, ext| {
        log.borrow_mut().push(format!("{name}.{ext}"));
        None
    }));
    let _ = engine.run();
    assert_eq!(asked.borrow().as_slice(), ["article.cls", "a.sty", "b.sty", "c.sty", "d.cls"]);
}

#[test]
fn without_a_reader_everything_passes_through() {
    let (out, diags) = run("\\documentclass[11pt]{article}\\usepackage[T1]{fontenc}\\usepackage{mystyle}\\hello", &[]);
    assert_eq!(out, "\\documentclass [11pt]article\\usepackage [T1]fontenc\\usepackage mystyle\\hello");
    assert!(diags.is_empty(), "{diags:?}");
}

/// The incremental expander loads packages exactly as a full run does,
/// through every edit, restart and convergence: the reader travels with
/// the checkpoints.
#[test]
fn incremental_expansion_reads_packages_like_a_full_run() {
    use flashtex_tex_expansion::{Edit, IncrementalExpander, Limits};
    let files: HashMap<String, String> = [
        ("mystyle.sty".to_string(), MYSTYLE.to_string()),
        (
            "opts.sty".to_string(),
            "\\ProvidesPackage{opts}\\DeclareOption{a}{\\def\\opt{A}}\\DeclareOption{b}{\\def\\opt{B}}\\ProcessOptions\\relax".to_string(),
        ),
    ]
    .into_iter()
    .collect();
    let reader: flashtex_tex_expansion::PackageReader = {
        let files = files.clone();
        Rc::new(move |name, ext| files.get(&format!("{name}.{ext}")).cloned())
    };
    let init: Rc<dyn Fn(&mut Engine)> = {
        let reader = reader.clone();
        Rc::new(move |engine: &mut Engine| engine.set_package_reader(reader.clone()))
    };
    let mut src = String::from("\\documentclass[a]{article}\n\\usepackage{mystyle}\n\\usepackage{opts}\n\\begin{document}\n");
    for i in 0..60 {
        src.push_str(&format!("Line {i}: \\hello, \\emphx{{x}} \\opt.\n\n"));
    }
    src.push_str("\\end{document}\n");
    let mut inc = IncrementalExpander::with_host(&src, Limits::default(), 64, init);
    let full = |src: &str| {
        let mut engine = engine_with(src, &files.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>());
        let tokens = engine.run();
        (tokens, engine.take_diagnostics())
    };
    let edits = [
        (src.find("Line 30").unwrap(), 0, "\\hello "),
        (src.find("[a]").unwrap() + 1, 1, "b"),
        (src.find("\\usepackage{opts}").unwrap(), 0, "\\usepackage{nothere}"),
        (src.find("Line 59").unwrap(), 4, "Last"),
    ];
    let mut current = src.clone();
    for (start, len, replacement) in edits {
        inc.edit(&Edit { start, end: start + len, replacement: replacement.to_string() });
        current.replace_range(start..start + len, replacement);
        let (tokens, diagnostics) = full(&current);
        assert_eq!(inc.tokens(), tokens.as_slice(), "tokens after edit at {start}");
        assert_eq!(inc.diagnostics(), diagnostics.as_slice(), "diagnostics after edit at {start}");
        let opened: Vec<&str> = inc.opened_package_files().map(|f| f.name.as_str()).collect();
        assert_eq!(opened, ["mystyle.sty", "opts.sty"]);
    }
}

/// In the document itself the four declarations are the host parser's
/// (inert metadata); only inside a package or class file do they run.
#[test]
fn declarations_in_a_document_pass_through() {
    let (out, diags) = run(
        "\\NeedsTeXFormat{LaTeX2e}[2022/06/01]\\ProvidesClass{article}\\ProvidesPackage{hyperref}[2023-11-26 v7.01d]\\ProvidesFile{foo.cfg}\\NeedsTeXFormat",
        &[],
    );
    assert_eq!(
        out,
        "\\NeedsTeXFormat LaTeX2e[2022/06/01]\\ProvidesClass article\\ProvidesPackage hyperref[2023-11-26 v7.01d]\\ProvidesFile foo.cfg\\NeedsTeXFormat"
    );
    assert!(diags.is_empty(), "{diags:?}");
    // `\@ifpackageloaded` is untouched by a document-level `\ProvidesPackage`.
    let (out, _) = run("\\ProvidesPackage{x}\\makeatletter\\@ifpackageloaded{x}{-}{N}", &[]);
    assert!(out.ends_with('N'), "{out}");
}


#[test]
fn a_project_class_uses_the_kernel_switches_font_defaults_and_setfontsize() {
    // The opening lines of IEEEtran.cls/mnras.cls/aastex.cls, reduced: a
    // `\if@compatibility\else` guard around options, the times.sty font
    // defaults, a `\@setfontsize`-based `\normalsize` and `\skip\footins`.
    const CLS: &str = r"\NeedsTeXFormat{LaTeX2e}\ProvidesClass{mycls}
\if@compatibility\else\DeclareOption{a4paper}{\def\paper{a4}}\fi
\DeclareOption*{}\ProcessOptions\relax
\renewcommand{\sfdefault}{phv}\renewcommand{\rmdefault}{ptm}
\def\normalsize{\@setfontsize\normalsize\@xpt\@xiipt}
\normalsize
\skip\footins 12pt plus 12pt
\@twosidetrue
\def\hello{\if@twoside two\else one\fi/\rmdefault/\paper}";
    let (out, diags) = run(
        "\\documentclass[a4paper]{mycls}\n\\begin{document}\\hello\\end{document}\n",
        &[("mycls.cls", CLS)],
    );
    let unexpected: Vec<&String> = diags.iter().filter(|d| !d.contains("has no \\LoadClass")).collect();
    assert!(unexpected.is_empty(), "{diags:?}");
    // `\fontsize`/`\selectfont` run in the engine and come back to the
    // host as `\flashtexfontsizedone{10}{12.0pt}`/`\flashtexselectfontdone`.
    assert_eq!(
        out,
        "\\flashtexfontsizedone 1012.0pt\\flashtexselectfontdone \\documentclass [a4paper]article \\document two/ptm/a4\\enddocument"
    );
}
