//! `\listoffigures`/`\listoftables` and `\ref` in a report whose chapters
//! come from `\include`d files: every entry carries the number its caption
//! shows, in the order the `\include` tree is read — not the order the
//! project hands the documents over in — and an `\includeonly`-excluded
//! file's floats are neither set nor listed.
//!
//! Expected data: MacTeX 2026 pdflatex, three runs, from `main.lof`,
//! `main.lot` and the `\newlabel`s of `main.aux`/`two.aux` for the
//! documents below (a fresh directory, so `skipped.aux` never exists). No
//! TeX runs here.

mod common;

use common::*;

const MAIN: &str = r"\documentclass{report}
\includeonly{two,three}
\begin{document}
\listoffigures
\listoftables
\begin{figure}[h]\caption{Before any chapter.}\label{f0}\end{figure}
\chapter{One}
Text.
\begin{figure}[h]\caption{In chapter one.}\label{f11}\end{figure}
\include{two}
\include{skipped}
Text after the includes.
\begin{figure}[h]\caption{Entry after the includes.}\label{f22}\end{figure}
\begin{table}[h]\caption{Entry table.}\label{t21}\end{table}
Refs \ref{f0}, \ref{f11}, \ref{f21}, \ref{f22}, \ref{t21}.
\end{document}
";
const TWO: &str = r"\chapter{Two}
Text.
\begin{figure}[h]\caption{In chapter two.}\label{f21}\end{figure}
";
const SKIPPED: &str = r"\chapter{Skipped}
Text.
\begin{figure}[h]\caption{Never read.}\end{figure}
\begin{table}[h]\caption{Never read.}\end{table}
";

/// Each page's lines (words joined, left to right), top to bottom.
fn lines(r: &flashtex_render_pipeline::Rendered) -> Vec<Vec<String>> {
    let words = words_of(r);
    r.v2
        .pages
        .iter()
        .map(|page| {
            let mut on_page: Vec<&Word> = words.iter().filter(|w| w.page == page.number).collect();
            on_page.sort_by(|a, b| a.baseline.total_cmp(&b.baseline).then(a.x.total_cmp(&b.x)));
            let mut out: Vec<(f64, Vec<&str>)> = Vec::new();
            for w in on_page {
                match out.last_mut() {
                    Some((b, line)) if (w.baseline - *b).abs() < 0.5 => line.push(&w.text),
                    _ => out.push((w.baseline, vec![&w.text])),
                }
            }
            out.into_iter().map(|(_, l)| l.join(" ")).collect()
        })
        .collect()
}

#[test]
fn lists_and_refs_follow_the_include_tree() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    // Handed over neither in reading order nor with the entry first.
    let r = render_docs(&[("skipped.tex", SKIPPED), ("two.tex", TWO), ("main.tex", MAIN)], "main.tex");
    let lines = lines(&r);
    eprintln!("{lines:#?}");
    assert_eq!(r.v2.pages.len(), 5, "page count (pdflatex: 5)");
    let entries = |page: usize| -> Vec<(String, String)> {
        lines[page]
            .iter()
            .filter(|l| l.contains(". ."))
            .map(|l| {
                let (head, page) = l.rsplit_once(' ').unwrap();
                (head.trim_end_matches([' ', '.']).to_string(), page.to_string())
            })
            .collect()
    };
    let pair = |a: &str, b: &str| (a.to_string(), b.to_string());
    // main.lof / main.lot.
    assert_eq!(
        entries(0),
        [pair("1 Before any chapter", "2"), pair("1.1 In chapter one", "3"), pair("2.1 In chapter two", "4"), pair("2.2 Entry after the includes", "5")]
    );
    assert_eq!(entries(1), [pair("2.1 Entry table", "5")]);
    // The captions say the same, on the pages the lists give.
    assert!(lines[1].contains(&"Figure 1: Before any chapter.".to_string()));
    assert!(lines[2].contains(&"Figure 1.1: In chapter one.".to_string()));
    assert!(lines[3].contains(&"Figure 2.1: In chapter two.".to_string()));
    assert!(lines[4].contains(&"Figure 2.2: Entry after the includes.".to_string()));
    assert!(lines[4].contains(&"Table 2.1: Entry table.".to_string()));
    // `\newlabel`s of main.aux and two.aux. The floats sit inside the
    // paragraph (#608: a float on its own lines does not end it), so the
    // refs share the paragraph's one line.
    assert!(lines[4].contains(&"Text after the includes. Refs 1, 1.1, 2.1, 2.2, 2.1.".to_string()), "{:?}", lines[4]);
    assert!(!lines.iter().flatten().any(|l| l.contains("Never") || l.contains("Skipped")), "an excluded file's material was set");
}
