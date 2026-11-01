//! cite.sty (Arseneau, v5.5) numeric citations against pdflatex: the
//! package sorts the keys, compresses three or more consecutive numbers
//! into `first--last` (`\citedash`, an en dash and `\penalty 1000`), and
//! separates entries with `\citepunct` = `,\penalty\@m\hskip.13em plus.1em
//! minus.1em` (cite.sty lines 46-47) -- not the kernel's `,\penalty\@m\ `
//! interword space, which set `[1, 2]` 2.03pt wider and moved every later
//! word of article-twocolumn page 1's line by up to 1.20 bp.
//!
//! Oracle: MacTeX 2026 pdflatex (`SOURCE_DATE_EPOCH=0`, two runs), word
//! origins from `tools/visual-oracle/pdftext.py`; `\showbox` of
//! `\hbox{block~\cite{a,b}. The}` in this document: `[` `1` `,` `\penalty
//! 1000` `\glue 1.29973 plus 0.9998 minus 0.9998` `2` `]` `.` `\glue
//! 4.44336 plus 4.99878 minus 0.37027` `T` `h` `e`, 70.44951pt wide; and
//! of `\hbox{block \cite{a,b,c} The}`: `[` `1` `\hbox(4.3045+0.0)x4.99878`
//! (the `--` ligature) `\penalty 1000` `3` `]`. No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of, Word};

const TOL: f64 = 0.02;

const DOC: &str = r"\documentclass[10pt]{article}
\usepackage[T1]{fontenc}
\usepackage[margin=1in]{geometry}
\usepackage{cite}
\begin{document}
\noindent block~\cite{a,b}. The

\noindent block \cite{a,b,c} The

\noindent block \cite{c,a} The

\noindent block \cite{d,b,a} The

\begin{thebibliography}{9}
\bibitem{a}A.
\bibitem{b}B.
\bibitem{c}C.
\bibitem{d}D.
\end{thebibliography}
\end{document}
";

/// Per line: the citation pdflatex set, the x of its `[` and the x of the
/// `The` after it (bp from the page's left edge, `pdftext.py`).
const EXPECTED: [(&str, f64, f64); 4] = [
    ("[1,2].", 98.281, 125.037),
    ("[1--3]", 98.281, 122.073),
    ("[1,3]", 98.281, 121.154),
    ("[1,2,4]", 98.281, 130.197),
];

/// The runs on the first page's baselines, left to right.
fn lines(words: &[Word]) -> Vec<Vec<&Word>> {
    let mut out: Vec<(f64, Vec<&Word>)> = Vec::new();
    let mut sorted: Vec<&Word> = words.iter().filter(|w| w.page == 1).collect();
    sorted.sort_by(|a, b| a.baseline.total_cmp(&b.baseline).then(a.x.total_cmp(&b.x)));
    for w in sorted {
        match out.last_mut() {
            Some((b, ws)) if (*b - w.baseline).abs() < 0.05 => ws.push(w),
            _ => out.push((w.baseline, vec![w])),
        }
    }
    out.into_iter().map(|(_, ws)| ws).collect()
}

#[test]
fn cite_package_sorts_compresses_and_sets_its_thin_separator() {
    if !lm_available() {
        return;
    }
    let r = render_one(DOC);
    let words = words_of(&r);
    let lines = lines(&words);
    let mut misses = Vec::new();
    for (i, (cite, open_x, the_x)) in EXPECTED.iter().enumerate() {
        let line = &lines[i];
        let text: String = line.iter().map(|w| w.text.as_str()).collect();
        let body = text.replace('\u{2013}', "--");
        if !body.starts_with("block[") || !body.ends_with("The") || !body.contains(cite) {
            misses.push(format!("line {i}: runs read {text:?}, pdflatex set `block {cite} The`"));
            continue;
        }
        let open = line.iter().find(|w| w.text.starts_with('[')).expect("[");
        let the = line.iter().rev().find(|w| w.text == "The").expect("The");
        if (open.x - open_x).abs() > TOL {
            misses.push(format!("line {i}: `[` at x {:.3}, pdflatex {open_x:.3}", open.x));
        }
        if (the.x - the_x).abs() > TOL {
            misses.push(format!("line {i}: `The` at x {:.3}, pdflatex {the_x:.3}", the.x));
        }
    }
    assert!(misses.is_empty(), "cite.sty citations off by more than {TOL} bp:\n{}", misses.join("\n"));
}
