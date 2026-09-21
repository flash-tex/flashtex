//! Page breaks around lists and at a forced break, against pdflatex.
//!
//! * `\@item` puts `\addpenalty\@itempenalty` before every later `\item`,
//!   `\@beginparpenalty` before the first (unless `\@nobreak`), and
//!   `\@endparenv` `\@endparpenalty` after the list -- each
//!   `-\@lowpenalty` = -51 in article. With the list's own glue on the page
//!   the badness of a break one line short is well under 51, so pdflatex
//!   ends the page *between* two items rather than inside the next one
//!   (`\tracingpages`: `b=... p=-51 c=...#` wins over the later `p=0`
//!   breaks). Without the penalties every such page broke one to three
//!   lines later, inside the item.
//! * `\newpage` is `\vfil\penalty-\@M`: the `\vfil` puts the last line's
//!   depth into `page_total` before the penalty is weighed, so a page that
//!   its shrink only just holds without that depth is overfull at the
//!   eject and ends at the best earlier break. pdflatex `\tracingpages` on
//!   the second document: `t=665.00027 plus 47.0 minus 16.0 ... b=75`,
//!   then `t=667.1289 plus 47.0 plus 1.0fil minus 16.0 g=650.43001 b=*
//!   p=-10000`; the page ends at `t=637.80026 ... c=2`, two lines of the
//!   last paragraph go to page 2. The pipeline weighed the eject without
//!   the depth and kept both lines on page 1, below the text block.
//!
//! Oracle: MacTeX 2026 pdflatex (`SOURCE_DATE_EPOCH=0`, two runs), page
//! contents from `pdftotext`. No TeX runs here.

mod common;

use common::{lm_available, render_one, words_of, Word};

const PARA: &str = "Item text that runs across several lines so that the page break can fall inside it rather than between the items of the list here. ";

fn doc(body: &str) -> String {
    format!("\\documentclass[11pt]{{article}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage[margin=1in]{{geometry}}\n\\begin{{document}}\n{body}\\end{{document}}\n")
}

/// The runs of each page's first line, joined by blanks.
fn first_lines(words: &[Word]) -> Vec<String> {
    let pages = words.iter().map(|w| w.page).max().unwrap_or(0);
    (1..=pages)
        .map(|page| {
            let on: Vec<&Word> = words.iter().filter(|w| w.page == page).collect();
            let top = on.iter().map(|w| w.baseline).fold(f64::INFINITY, f64::min);
            let mut line: Vec<&&Word> = on.iter().filter(|w| (w.baseline - top).abs() < 0.05).collect();
            line.sort_by(|a, b| a.x.total_cmp(&b.x));
            line.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
        })
        .collect()
}

#[test]
fn a_list_offers_minus_51_between_items_and_the_page_takes_it() {
    if !lm_available() {
        return;
    }
    let mut body = String::new();
    for v in 0..6 {
        body.push_str("\\begin{itemize}\n");
        for i in 0..20 + v {
            body.push_str(&format!("\\item Short item {v}.{i}.\n"));
        }
        for i in 0..6 {
            body.push_str(&format!("\\item Long {v}.{i}. {}\n", PARA.repeat(4)));
        }
        body.push_str("\\end{itemize}\n\\newpage\n");
    }
    let r = render_one(&doc(&body));
    let firsts = first_lines(&words_of(&r));
    // pdflatex: every variant's second page opens with an item.
    let want = ["Short item 0.0.", "Long 0.2.", "Short item 1.0.", "Long 1.2.", "Short item 2.0.", "Long 2.2.", "Short item 3.0.", "Long 3.1.", "Short item 4.0.", "Long 4.1.", "Short item 5.0.", "Long 5.1."];
    assert_eq!(firsts.len(), want.len(), "pages: {firsts:#?}");
    let misses: Vec<String> = firsts
        .iter()
        .zip(want)
        .enumerate()
        .filter(|(_, (got, want))| !got.replace(' ', "").contains(&want.replace(' ', "")))
        .map(|(i, (got, want))| format!("page {}: first line {got:?}, pdflatex opens with {want:?}", i + 1))
        .collect();
    assert!(misses.is_empty(), "{}", misses.join("\n"));
}

#[test]
fn newpage_weighs_the_last_depth_and_an_overfull_page_breaks_earlier() {
    if !lm_available() {
        return;
    }
    let para = "Paragraph text that runs across several lines so that the page break can fall inside it rather than at the list boundary here. ";
    let mut body = String::new();
    for i in 0..30 {
        body.push_str(&format!("Short paragraph 0.{i}.\n\n"));
    }
    body.push_str("\\begin{itemize}\n");
    for i in 0..3 {
        body.push_str(&format!("\\item Item 0.{i}. {}\n", para.repeat(2)));
    }
    body.push_str(&format!("\\end{{itemize}}\nAfter 0. {}\n\n\\newpage\nNext.\n", para.repeat(5)));
    let r = render_one(&doc(&body));
    let firsts = first_lines(&words_of(&r));
    // pdflatex: three pages; the second opens with the last two lines of
    // the `After` paragraph ("here. Paragraph text that runs ...").
    assert_eq!(firsts.len(), 3, "pages: {firsts:#?}");
    assert!(firsts[1].starts_with("here. Paragraph"), "page 2 opens with {:?}", firsts[1]);
    assert_eq!(firsts[2], "Next.");
}

/// The other side of the same rule: the `\vfil` of `\end{document}`'s
/// `\clearpage` is itself a breakpoint, weighed before its glue adds the
/// depth, so a last page that ends exactly full keeps its last line even
/// though the eject behind it is overfull. pdflatex (12pt, 1in margins):
/// `t=650.0 plus 7.0 g=650.43001 b=0 p=0 c=0#`, then `t=652.33331 plus 7.0
/// plus 1.0fil ... b=* p=-10000`; four pages, the fourth ending "the quiet
/// river below."
#[test]
fn a_page_that_ends_exactly_full_keeps_its_last_line_at_the_final_eject() {
    if !lm_available() {
        return;
    }
    let para = "The quick brown fox jumps over the lazy dog while the patient owl watches from an old oak branch and counts every leaf that falls into the quiet river below. ";
    let mut src = String::from("\\documentclass[12pt]{article}\n\\usepackage[margin=1in]{geometry}\n\\begin{document}");
    for _ in 0..30 {
        src.push_str(&para.repeat(3));
        src.push_str("\n\n");
    }
    src.push_str("\\end{document}");
    let r = render_one(&src);
    assert_eq!(r.v2.pages.len(), 4, "pdflatex sets four pages");
}
