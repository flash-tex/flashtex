//! natbib's author list is a group, and TeX's lig/kern program stops at a
//! brace: `\NAT@nmfmt{\NAT@nm}` is `{\NAT@up#1}` (natbib.sty line 287), so
//! in `\citep{hobby1986}` the `y` of "Hobby" and the `,` of `\NAT@aysep`
//! after it are set with no kern between them, where a typed `y,` gets
//! cmr's -0.0833 em (-0.91251 pt at 10.95 pt). Setting the citation as one
//! string kerned it, and every later word of natbib-review's citation
//! lines sat 0.91 bp left of pdflatex's (pages 1 and 2).
//!
//! Oracle: MacTeX 2026 pdflatex (`SOURCE_DATE_EPOCH=0`, two runs), word
//! origins from `tools/visual-oracle/pdftext.py`; `\showbox` of
//! `\hbox{criterion \citep{hobby1986,frank1990}. X}` in this preamble:
//! `H` `o` `b` `b` `\kern-0.30418` `y` `,` `\glue 3.65 plus 1.825 minus
//! 1.21666` `1` `9` `8` `6` `;` ... -- the `b`-`y` kern inside the name
//! stays, the `y`-`,` one is absent. No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of, Word};

const TOL: f64 = 0.02;

const DOC: &str = r"\documentclass[11pt]{article}
\usepackage[margin=1in]{geometry}
\usepackage{amsmath}
\usepackage[authoryear,round]{natbib}
\begin{document}
\noindent optimality criterion \citep{hobby1986,frank1990}. X

\noindent curves \citep{hobby1986}, and non-Latin

\noindent line breaking \citep{plass1981,hobby1986,frank1990}. We conclude

\noindent whole-page breaking was addressed by \citet{frank1990}, who adapted
\begin{thebibliography}{9}
\bibitem[Hobby(1986)]{hobby1986} H.
\bibitem[Frank(1990)]{frank1990} F.
\bibitem[Plass(1981)]{plass1981} P.
\end{thebibliography}
\end{document}
";

/// Per citation line, the words after the citation and their x as
/// pdflatex set them (bp from the page's left edge, `pdftext.py`).
const EXPECTED: [&[(&str, f64)]; 4] = [
    &[("X", 307.398)],
    &[("and", 180.270), ("non-Latin", 201.490)],
    &[("We", 335.817), ("conclude", 354.605)],
    &[("who", 327.271), ("adapted", 350.309)],
];

/// The runs on the first page's baselines, top to bottom.
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
fn natbib_author_list_is_not_kerned_into_the_separator() {
    if !lm_available() {
        return;
    }
    let r = render_one(DOC);
    let words = words_of(&r);
    let lines = lines(&words);
    let mut misses = Vec::new();
    for (i, expected) in EXPECTED.iter().enumerate() {
        let line = &lines[i];
        for (text, x) in expected.iter() {
            match line.iter().find(|w| w.text == *text) {
                Some(w) if (w.x - x).abs() > TOL => misses.push(format!("line {i}: {text:?} at x {:.3}, pdflatex {x:.3}", w.x)),
                Some(_) => {}
                None => misses.push(format!("line {i}: no run {text:?} in {:?}", line.iter().map(|w| w.text.as_str()).collect::<Vec<_>>())),
            }
        }
    }
    assert!(misses.is_empty(), "natbib citation lines off by more than {TOL} bp:\n{}", misses.join("\n"));
}
