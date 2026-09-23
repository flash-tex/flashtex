//! Unused `\documentclass` options (warning parity with pdflatex).
//!
//! Oracle: TeX Live 2026 pdflatex, run 2026-09-23 over minimal files in
//! `/tmp/unusedopt` (kept out of the repo; exact commands in the lane
//! check-in). pdflatex prints, at the end of the preamble,
//!
//! ```text
//! LaTeX Warning: Unused global option(s):
//!     [nosuchoption].
//! ```
//!
//! for `\documentclass[nosuchoption]{article}`, and nothing for
//! `\documentclass[11pt]{article}`. An option the class declares
//! (`article.cls` `\DeclareOption`s) or a loaded package declares is used;
//! anything else is reported once, with `=value` stripped (`fontsize=12pt`
//! reports as `[fontsize]`) and duplicates collapsed (`[foo,foo]` reports
//! as `[foo]`). The check only runs for the four standard classes with
//! every loaded package modelled: a KOMA/AMS/IEEE/memoir class, or a
//! package outside the modelled set (cleveref, siunitx, biblatex, ...),
//! stays silent, because pdflatex may consume the option where this
//! compiler cannot tell (each case below was probed silent). beamer
//! swallows every global option (`\documentclass[foo]{beamer}` is silent),
//! so it never warns.

use flashtex_compiler::parser;

fn unused_warnings(source: &str) -> Vec<String> {
    parser::parse(source)
        .diagnostics
        .into_iter()
        .filter(|d| d.message.contains("Unused global option"))
        .map(|d| d.message)
        .collect()
}

#[test]
fn unknown_class_option_warns_once_listing_it() {
    let source = "\\documentclass[nosuchoption]{article}\n\\begin{document}\nHello.\n\\end{document}\n";
    let warnings = unused_warnings(source);
    assert_eq!(warnings.len(), 1, "{source}: {warnings:?}");
    assert!(
        warnings[0].contains("[nosuchoption]"),
        "warning must list the option: {:?}",
        warnings[0]
    );
    // The minimal repro document has no other diagnostics; the fix adds
    // exactly this one (pdflatex likewise prints just the one warning).
    assert_eq!(
        parser::parse(source).diagnostics.len(),
        1,
        "{:?}",
        parser::parse(source).diagnostics
    );
}

#[test]
fn several_unknown_options_share_one_warning() {
    let warnings = unused_warnings(
        "\\documentclass[nosuchoption,otherbad]{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("nosuchoption"), "{:?}", warnings[0]);
    assert!(warnings[0].contains("otherbad"), "{:?}", warnings[0]);
}

#[test]
fn used_class_options_do_not_warn() {
    for options in [
        "11pt",
        "10pt",
        "12pt",
        "a4paper",
        "letterpaper",
        "landscape",
        "twoside",
        "draft",
        "final",
        "titlepage",
        "notitlepage",
        "twocolumn",
        "onecolumn",
        "leqno",
        "fleqn",
        "openbib",
    ] {
        let source = format!(
            "\\documentclass[{options}]{{article}}\n\\begin{{document}}\nHello.\n\\end{{document}}\n"
        );
        assert!(
            unused_warnings(&source).is_empty(),
            "{options}: {:?}",
            unused_warnings(&source)
        );
    }
}

#[test]
fn mixed_known_and_unknown_lists_only_the_unknown() {
    let warnings = unused_warnings(
        "\\documentclass[11pt,nosuchoption]{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("nosuchoption"), "{:?}", warnings[0]);
    assert!(!warnings[0].contains("11pt"), "{:?}", warnings[0]);
}

#[test]
fn option_declared_by_a_package_does_not_warn() {
    // pdflatex is silent for both: natbib declares `round`, xcolor takes
    // `table`.
    for source in [
        "\\documentclass[round]{article}\n\\usepackage{natbib}\n\\begin{document}\nHello.\n\\end{document}\n",
        "\\documentclass[table]{article}\n\\usepackage{xcolor}\n\\begin{document}\nHello.\n\\end{document}\n",
    ] {
        assert!(
            unused_warnings(source).is_empty(),
            "{source}: {:?}",
            unused_warnings(source)
        );
    }
}

#[test]
fn option_no_modelled_package_declares_still_warns() {
    // pdflatex still warns: cite does not declare `foo`, and a bare
    // `\usepackage{inputenc}` does not consume a global `utf8` (both
    // probed). (cite is modelled; an unmodelled package such as cleveref
    // stays silent — see `unmodelled_package_stays_silent`.)
    for (source, listed) in [
        (
            "\\documentclass[foo]{article}\n\\usepackage{cite}\n\\begin{document}\nHello.\n\\end{document}\n",
            "[foo]",
        ),
        (
            "\\documentclass[utf8]{article}\n\\usepackage{inputenc}\n\\begin{document}\nHello.\n\\end{document}\n",
            "[utf8]",
        ),
    ] {
        let warnings = unused_warnings(source);
        assert_eq!(warnings.len(), 1, "{source}: {warnings:?}");
        assert!(warnings[0].contains(listed), "{:?}", warnings[0]);
    }
}

#[test]
fn reviewer_silent_cases_give_zero_warnings() {
    // Each of these is silent under TeX Live 2026 pdflatex (probed
    // 2026-09-23 in /tmp/unusedopt): the class or the loaded package
    // consumes the global option.
    for source in [
        "\\documentclass[english]{article}\n\\usepackage{babel}\n\\begin{document}\nHello.\n\\end{document}\n",
        "\\documentclass[hidelinks]{article}\n\\usepackage{hyperref}\n\\begin{document}\nHello.\n\\end{document}\n",
        "\\documentclass[final]{article}\n\\usepackage{microtype}\n\\begin{document}\nHello.\n\\end{document}\n",
        "\\documentclass[dvipsnames]{article}\n\\usepackage{xcolor}\n\\begin{document}\nHello.\n\\end{document}\n",
        "\\documentclass[draft]{article}\n\\usepackage{graphicx}\n\\begin{document}\nHello.\n\\end{document}\n",
        "\\documentclass[round]{article}\n\\usepackage{natbib}\n\\begin{document}\nHello.\n\\end{document}\n",
    ] {
        assert!(
            unused_warnings(source).is_empty(),
            "{source}: {:?}",
            unused_warnings(source)
        );
    }
}

#[test]
fn unmodelled_class_stays_silent() {
    // pdflatex is silent for both (probed): scrartcl declares `fontsize`
    // itself, and IEEEtran declares `conference`. The lane does not model
    // those option sets, so it stays silent rather than warning falsely.
    for source in [
        "\\documentclass[fontsize=12pt]{scrartcl}\n\\begin{document}\nHello.\n\\end{document}\n",
        "\\documentclass[conference]{IEEEtran}\n\\begin{document}\nHello.\n\\end{document}\n",
    ] {
        assert!(
            unused_warnings(source).is_empty(),
            "{source}: {:?}",
            unused_warnings(source)
        );
    }
}

#[test]
fn unmodelled_package_stays_silent() {
    // pdflatex may consume `foo` inside a package this compiler does not
    // model (cleveref, siunitx, biblatex, ...), so a loaded unmodelled
    // package silences the check rather than risking a false warning.
    for source in [
        "\\documentclass[foo]{article}\n\\usepackage{cleveref}\n\\begin{document}\nHello.\n\\end{document}\n",
        "\\documentclass[foo]{article}\n\\usepackage{siunitx}\n\\begin{document}\nHello.\n\\end{document}\n",
    ] {
        assert!(
            unused_warnings(source).is_empty(),
            "{source}: {:?}",
            unused_warnings(source)
        );
    }
}

#[test]
fn key_value_option_reports_its_key() {
    // pdflatex reports `[fontsize]` for `[fontsize=12pt]`.
    let warnings = unused_warnings(
        "\\documentclass[fontsize=12pt]{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("fontsize"), "{:?}", warnings[0]);
}

#[test]
fn duplicate_option_is_listed_once() {
    // pdflatex reports `[foo]` for `[foo,foo]`.
    let warnings = unused_warnings(
        "\\documentclass[foo,foo]{article}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("[foo]"), "{:?}", warnings[0]);
}

#[test]
fn star_handler_option_needs_an_explicit_pass_to_count() {
    // fontenc's `*` handler only fires for explicitly passed options
    // (latex.ltx `\@process@pti@ns`): a bare `\usepackage{fontenc}`
    // leaves a global `T1` unused (probed warns), while
    // `\usepackage[T1]{fontenc}` consumes it (probed silent).
    let bare = unused_warnings(
        "\\documentclass[T1]{article}\n\\usepackage{fontenc}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    assert_eq!(bare.len(), 1, "{bare:?}");
    assert!(bare[0].contains("[T1]"), "{:?}", bare[0]);
    let explicit = unused_warnings(
        "\\documentclass[T1]{article}\n\\usepackage[T1]{fontenc}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    assert!(explicit.is_empty(), "{explicit:?}");
}

#[test]
fn beamer_consumes_every_global_option() {
    // pdflatex is silent for `\documentclass[foo]{beamer}`.
    let warnings = unused_warnings(
        "\\documentclass[foo]{beamer}\n\\begin{document}\nHello.\n\\end{document}\n",
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}
