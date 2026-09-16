//! `\paragraph` and `\subparagraph` are run-in headings — `\@startsection`
//! with a *negative* after-skip (article.cls lines 406-414):
//!
//! ```tex
//! \newcommand\paragraph{\@startsection{paragraph}{4}{\z@}%
//!   {3.25ex \@plus1ex \@minus.2ex}{-1em}{\normalfont\normalsize\bfseries}}
//! \newcommand\subparagraph{\@startsection{subparagraph}{5}{\parindent}%
//!   {3.25ex \@plus1ex \@minus.2ex}{-1em}{\normalfont\normalsize\bfseries}}
//! ```
//!
//! `\@xsect`'s negative-`#5` branch never sets the head as a vertical block.
//! It arms `\everypar`, which throws away the following paragraph's
//! `\parindent` box, sets `\hskip #3 <head>` in its place and then
//! `\hskip -#5`. So the head *is* the first words of that paragraph: bold,
//! at indent `#3` (0 for `\paragraph`, `\parindent` for `\subparagraph`),
//! followed by 1 em rather than an interword space, with
//! `\addvspace{3.25ex...}` above it.
//!
//! Expected coordinates measured with `tools/visual-oracle/pdftext.py` on
//! the PDFs pdflatex (TeX Live 2025, pdfTeX 3.141592653-2.6-1.40.27,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) produced from exactly these
//! sources; the first is
//! `fixtures/divergence-probes/min-paragraph/main.tex`. The oracle is not
//! run here; pdflatex is an oracle only and never in the product path.
//! Coordinates are bp from the page's top-left corner.

mod common;

use common::*;

const TOL: f64 = 0.3;

/// 10 pt article; `\paragraph` after an ordinary paragraph.
const AFTER_PARAGRAPH: &str = concat!(
    "\\documentclass{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\begin{document}\n\n",
    "\\section{A section}\n\n",
    "Ordinary body text before the run-in heading, long enough that the line above\n",
    "the heading is set the same way on both sides.\n\n",
    "\\paragraph{A run-in heading.} The paragraph heading is set in bold, run into\n",
    "the first line of its paragraph, with a fixed skip above it and a word space\n",
    "after it. Real solutions, proofs and notes use it constantly.\n\n",
    "\\paragraph{A second one.} And the text that follows it.\n",
    "\\end{document}\n",
);

/// 11 pt article; `\paragraph` right after `\end{enumerate}`, the shape
/// every `\paragraph{Solution.}` in `fixtures/real-world/ps-calculus` has.
const AFTER_LIST: &str = concat!(
    "\\documentclass[11pt]{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\begin{document}\n",
    "Plain text before the list here.\n\n",
    "\\begin{enumerate}\n",
    "  \\item First item of the list.\n",
    "  \\item Second item of the list.\n",
    "\\end{enumerate}\n\n",
    "\\paragraph{Solution.} Body text after a run-in heading that follows a list.\n\n",
    "Another plain paragraph here.\n\n",
    "\\paragraph{Second.} Body text after a run-in heading that follows a paragraph.\n",
    "\\end{document}\n",
);

/// 10 pt article; all three forms of the command.
const ALL_FORMS: &str = concat!(
    "\\documentclass{article}\n",
    "\\usepackage[T1]{fontenc}\n",
    "\\usepackage[margin=1in]{geometry}\n",
    "\\begin{document}\n",
    "Ordinary body text before the run-in headings here.\n\n",
    "\\paragraph{A paragraph head.} Body text after the paragraph head.\n\n",
    "\\subparagraph{A subparagraph head.} Body text after the subparagraph head.\n\n",
    "\\paragraph*{A starred head.} Body text after the starred head.\n",
    "\\end{document}\n",
);

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    nth(words, text, 0)
}

/// The `n`th occurrence of `text` in reading order (`A` appears both in the
/// `\section` title and as the run-in head's first word).
fn nth<'a>(words: &'a [Word], text: &str, n: usize) -> &'a Word {
    words.iter().filter(|w| w.text == text).nth(n).unwrap_or_else(|| panic!("no word {text:?} #{n} in {words:?}"))
}

/// Every word on the line with this baseline, left to right.
fn on_line(words: &[Word], baseline: f64) -> Vec<&Word> {
    let mut line: Vec<&Word> = words.iter().filter(|w| (w.baseline - baseline).abs() < 0.05).collect();
    line.sort_by(|a, b| a.x.total_cmp(&b.x));
    line
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!((actual - expected).abs() < TOL, "{what}: {actual} vs pdflatex {expected}");
}

fn render(src: &str) -> Vec<Word> {
    let r = render_one(src);
    assert!(!r.v2.pages.is_empty(), "{:?}", r.v2.diagnostics);
    words_of(&r)
}

#[test]
fn paragraph_runs_into_its_body_flush_bold_and_one_em_clear() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let w = render(AFTER_PARAGRAPH);

    // `#3` is `\z@`: the head is flush at the margin, *not* indented by
    // `\parindent` — `\@xsect` discarded that box.
    // `A` also opens the `\section` title, so take the second one.
    let head = nth(&w, "A", 1);
    close(head.x, 72.000, "run-in head x");
    // `\addvspace{3.25ex}` above it: the body line before ends at 115.736
    // and one `\baselineskip` alone would put this at 127.691.
    close(head.baseline, 141.629, "run-in head baseline");

    // The head's own words, then `\hskip 1em` before the body text. The
    // interword glue *inside* the head is the bold face's, not the body's.
    // Both `run-in` and `heading` also occur in the body above, so read the
    // head's own line rather than the first match.
    let line = on_line(&w, head.baseline);
    assert_eq!(line[..4].iter().map(|x| x.text.as_str()).collect::<Vec<_>>(), ["A", "run-in", "heading.", "The"]);
    close(line[1].x, 84.664, "second word of the head");
    close(line[2].x, 119.476, "third word of the head");
    close(line[3].x, 171.438, "body text, 1 em after the head");

    // The second head, same shape.
    let head2 = nth(&w, "A", 2);
    close(head2.x, 72.000, "second head, first word");
    let line2 = on_line(&w, head2.baseline);
    assert_eq!(line2[..4].iter().map(|x| x.text.as_str()).collect::<Vec<_>>(), ["A", "second", "one.", "And"]);
    close(line2[1].x, 84.664, "second head, second word");
    close(line2[3].x, 152.089, "second head's body text");
}

#[test]
fn paragraph_after_a_list_does_not_add_to_the_lists_own_addvspace() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let w = render(AFTER_LIST);

    // `\endtrivlist` already did `\addvspace\@topsepadd`; `\@startsection`'s
    // `\addvspace{3.25ex...}` then keeps whichever is larger
    // (`\@xaddvskip`), it does not add to it. Summing the two puts this head
    // ~1.5 bp low and drags the rest of the page with it.
    let head = word(&w, "Solution.");
    close(head.x, 72.000, "head after a list, x");
    close(head.baseline, 159.789, "head after a list, baseline");
    close(word(&w, "Body").x, 131.313, "body text 1 em after the head");

    // The list above it is unmoved, and so is the paragraph below.
    close(word(&w, "1.").baseline, 108.463, "list item 1");
    close(word(&w, "2.").baseline, 130.979, "list item 2");
    close(word(&w, "Another").baseline, 173.339, "paragraph after the head's paragraph");
    close(word(&w, "Another").x, 88.936, "that paragraph keeps its \\parindent");

    // A head after an ordinary paragraph gets the full skip.
    close(word(&w, "Second.").baseline, 202.149, "head after a paragraph, baseline");
    close(word(&w, "Second.").x, 72.000, "head after a paragraph, x");
}

#[test]
fn subparagraph_indents_by_parindent_and_the_starred_form_sets_no_star() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let w = render(ALL_FORMS);

    // `\paragraph`'s `#3` is `\z@`; `\subparagraph`'s is `\parindent`
    // (15 pt = 14.944 bp at 10 pt), so its head is *not* flush.
    let para = nth(&w, "A", 0);
    close(para.x, 72.000, "\\paragraph head x");
    close(para.baseline, 107.855, "\\paragraph head baseline");

    let sub = nth(&w, "A", 1);
    close(sub.x, 86.944, "\\subparagraph head x (\\parindent)");
    close(sub.baseline, 133.748, "\\subparagraph head baseline");
    close(on_line(&w, sub.baseline)[1].x, 99.419, "\\subparagraph second word");

    // `\paragraph*` sets a head with no number — and no star. The compiler
    // does not consume the `*`, so without dropping it the head would carry
    // a literal asterisk and push the whole line 5.73 bp right.
    let starred = nth(&w, "A", 2);
    close(starred.x, 72.000, "\\paragraph* head x");
    close(starred.baseline, 159.640, "\\paragraph* head baseline");
    let line = on_line(&w, starred.baseline);
    assert_eq!(line[0].text, "A", "no star on the head: {line:?}");
    close(line[1].x, 84.475, "\\paragraph* second word");
}
