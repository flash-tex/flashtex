//! GH-CLASS-OPTIONS: key=value class and package options through FlashTeX's
//! own `xkeyval`/`kvoptions` subsets, and a project class's `\LoadClass`
//! options resolved before the typesetting layer sees them.
//!
//! Every expected value is pdflatex's (pdfTeX 1.40.29, TeX Live 2026,
//! xkeyval 2.9, kvoptions 3.15) on the same files, read back from the PDF.
//! The documents print `OK`/`BAD` words from `\ifx` comparisons, so the
//! check does not depend on how the layout splits text into items.

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{LayoutConstraints, Session};
use flashtex_compiler::parser::{parse_project, SourceDocument};

fn run(files: &[(&str, &str)]) -> (String, Vec<String>) {
    let docs: Vec<SourceDocument> = files.iter().map(|(path, text)| SourceDocument { path, text }).collect();
    let result = Session::new().compile_project(&docs, "main.tex", LayoutConstraints::default());
    let problems = result
        .output
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error || d.message.contains("xkeyval") || d.message.contains("kvoptions"))
        .map(|d| d.message.clone())
        .collect();
    let text = result
        .output
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.as_str()))
        .flat_map(|t| t.chars().filter(|c| !c.is_whitespace()))
        .collect();
    (text, problems)
}

/// jstrieb/homework-template and dev-aditya/LaTeX-template's pattern.
const XKV_CLASS: &str = r"\NeedsTeXFormat{LaTeX2e}\ProvidesClass{myc}
\LoadClass{article}
\RequirePackage{xkeyval}
\DeclareOptionX{name}[]{\def\myname{#1}}
\DeclareOptionX{num}[7]{\def\mynum{#1}}
\DeclareOptionX{flag}{\def\myflag{set}}
\def\myrest{}
\DeclareOptionX*{\edef\myrest{\myrest[\CurrentOption]}}
\ProcessOptionsX\relax
";

#[test]
fn xkeyval_class_options_take_values_defaults_and_the_star_handler() {
    let main = r"\documentclass[name=Jacob Strieb, num, flag, 11pt, foo=bar]{myc}
\makeatletter
\begin{document}
\def\w{JacobStrieb}\ifx\myname\w NAMEOK\else NAMEBAD\fi
\def\w{7}\ifx\mynum\w NUMOK\else NUMBAD\fi
\def\w{set}\ifx\myflag\w FLAGOK\else FLAGBAD\fi
\def\w{[11pt][foo=bar]}\ifx\myrest\w RESTOK\else RESTBAD\fi
\end{document}
";
    let (text, problems) = run(&[("main.tex", main), ("myc.cls", XKV_CLASS)]);
    assert_eq!(text, "NAMEOKNUMOKFLAGOKRESTOK", "{problems:?}");
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_loadclass_without_options_does_not_inherit_the_documentclass_options() {
    // pdflatex: article sees none of `11pt, foo=bar` (a class reads only
    // the options it was loaded with), so the base class gets no options.
    let main = "\\documentclass[name=A, 11pt, foo=bar]{myc}\n\\begin{document}\nx\n\\end{document}\n";
    let docs = [SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "myc.cls", text: XKV_CLASS }];
    let parsed = parse_project(&docs, "main.tex");
    assert_eq!(parsed.document_class.as_deref(), Some("article"));
    assert_eq!(parsed.class_options, None);
}

/// artemmavrin/latex-homework's pattern: a void size option sets a macro
/// that `\LoadClass` reads; everything else goes to the default handler.
const KVO_CLASS: &str = r"\NeedsTeXFormat{LaTeX2e}\ProvidesClass{kvc}
\RequirePackage{kvoptions}
\SetupKeyvalOptions{family=HW,prefix=HW@}
\def\@fontsize{12pt}
\DeclareVoidOption{11pt}{\renewcommand{\@fontsize}{11pt}}
\DeclareBoolOption{qed}
\DeclareBoolOption[true]{boxes}
\DeclareStringOption[dflt]{title}[given]
\DeclareComplementaryOption{noqed}{qed}
\def\hwrest{}
\DeclareDefaultOption{\edef\hwrest{\hwrest[\CurrentOptionKey|\CurrentOptionValue]}}
\ProcessKeyvalOptions*
\LoadClass[\@fontsize]{article}
";

#[test]
fn kvoptions_bool_string_void_and_default_options() {
    let main = r"\documentclass[11pt, qed, boxes=false, title=My Title, x=y]{kvc}
\makeatletter
\begin{document}
\ifHW@qed QEDOK\else QEDBAD\fi
\ifHW@boxes BOXESBAD\else BOXESOK\fi
\def\w{MyTitle}\ifx\HW@title\w TITLEOK\else TITLEBAD\fi
\def\w{[x|y]}\ifx\hwrest\w RESTOK\else RESTBAD\fi
\end{document}
";
    let (text, problems) = run(&[("main.tex", main), ("kvc.cls", KVO_CLASS)]);
    assert_eq!(text, "QEDOKBOXESOKTITLEOKRESTOK", "{problems:?}");
    assert!(problems.is_empty(), "{problems:?}");
    let docs = [SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "kvc.cls", text: KVO_CLASS }];
    assert_eq!(parse_project(&docs, "main.tex").class_options.as_deref(), Some("11pt"));
}

#[test]
fn loadclass_options_from_a_macro_reach_the_typesetter_expanded() {
    // pdflatex sets artemmavrin's homework at 12pt: `\LoadClass[\@fontsize]`
    // with `\@fontsize` = 12pt. Before, the typesetter got `\@fontsize`.
    let main = "\\documentclass{kvc}\n\\makeatletter\n\\begin{document}\n\\ifHW@qed QEDBAD\\else QEDOK\\fi\n\\def\\w{dflt}\\ifx\\HW@title\\w TITLEOK\\else TITLEBAD\\fi\n\\end{document}\n";
    let (text, problems) = run(&[("main.tex", main), ("kvc.cls", KVO_CLASS)]);
    assert_eq!(text, "QEDOKTITLEOK", "{problems:?}");
    let docs = [SourceDocument { path: "main.tex", text: main }, SourceDocument { path: "kvc.cls", text: KVO_CLASS }];
    assert_eq!(parse_project(&docs, "main.tex").class_options.as_deref(), Some("12pt"));
}

const XKV_PACKAGE: &str = r"\ProvidesPackage{mypkg}
\RequirePackage{xkeyval}
\DeclareOptionX{color}[red]{\def\pkgcolor{#1}}
\DeclareOptionX{mydraft}{\def\pkgdraft{yes}}
\ExecuteOptionsX{color=blue}
\ProcessOptionsX\relax
\define@cmdkey{ref}{first}[]{}
\define@cmdkey{ref}{second}[dflt]{}
\newcommand\refs[1]{\setkeys{ref}{#1}%
  \key@ifundefined[cmdKV]{ref}{first}{NOFIRST}{FIRST=\cmdKV@ref@first}%
  \key@ifundefined[cmdKV]{ref}{second}{NOSECOND}{SECOND=\cmdKV@ref@second}}
";

#[test]
fn xkeyval_package_options_cmdkeys_and_global_options() {
    // pdflatex: `C=green. D=none. NOFIRSTSECOND=dflt FIRST=AnnNOSECOND`;
    // unstarred \ProcessOptionsX leaves the global `mydraft` alone.
    let main = r"\documentclass[mydraft,color=pink]{article}
\usepackage[color=green]{mypkg}
\makeatletter
\begin{document}
C=\pkgcolor. D=\@ifundefined{pkgdraft}{none}{\pkgdraft}.
{\refs{second}} {\refs{first=Ann}}
\end{document}
";
    let (text, problems) = run(&[("main.tex", main), ("mypkg.sty", XKV_PACKAGE)]);
    assert_eq!(text, "C=green.D=none.NOFIRSTSECOND=dfltFIRST=AnnNOSECOND", "{problems:?}");
    assert!(problems.is_empty(), "{problems:?}");
    // `\ProcessOptionsX*` takes the global options that are its keys first:
    // pdflatex `C=green. D=yes.`
    let starred = XKV_PACKAGE.replace(r"\ProcessOptionsX\relax", r"\ProcessOptionsX*\relax");
    let (text, problems) = run(&[("main.tex", main), ("mypkg.sty", &starred)]);
    assert!(text.starts_with("C=green.D=yes."), "{text} {problems:?}");
}

#[test]
fn kvoptions_package_takes_global_options_that_are_its_keys() {
    // pdflatex: `KD=T. KC=pink.`
    let pkg = r"\ProvidesPackage{kvp}
\RequirePackage{kvoptions}
\SetupKeyvalOptions{family=KP,prefix=KP@}
\DeclareBoolOption{mydraft}
\DeclareStringOption[none]{color}
\ProcessKeyvalOptions*
";
    let main = "\\documentclass[mydraft,color=pink]{article}\n\\usepackage{kvp}\n\\makeatletter\n\\begin{document}\nKD=\\ifKP@mydraft T\\else F\\fi. KC=\\KP@color.\n\\end{document}\n";
    let (text, problems) = run(&[("main.tex", main), ("kvp.sty", pkg)]);
    assert_eq!(text, "KD=T.KC=pink.", "{problems:?}");
}

#[test]
fn an_undeclared_package_option_is_the_xkeyval_error() {
    let main = "\\documentclass{article}\n\\usepackage[nosuch]{mypkg}\n\\begin{document}\nx\n\\end{document}\n";
    let (_, problems) = run(&[("main.tex", main), ("mypkg.sty", XKV_PACKAGE)]);
    assert!(problems.iter().any(|p| p.contains("nosuch") && p.contains("undefined")), "{problems:?}");
}

#[test]
fn a_project_file_of_the_same_name_still_wins() {
    let own = "\\ProvidesPackage{xkeyval}\\def\\ownxkv{OWN}\n";
    let main = "\\documentclass{article}\n\\usepackage{xkeyval}\n\\begin{document}\n\\ownxkv\n\\end{document}\n";
    let (text, _) = run(&[("main.tex", main), ("xkeyval.sty", own)]);
    assert!(text.starts_with("OWN"), "{text}");
}
