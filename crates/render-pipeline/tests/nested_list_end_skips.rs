//! The skips where nested lists end.
//!
//! `\endtrivlist` is `\addvspace\@topsepadd`, and `\@topsepadd` is the
//! `\topsep` *of the level that closes* (plus that list's `\partopsep` when
//! its `\begin` was read in vertical mode). Several lists closing together,
//! and a following `\item`'s `\addvspace\itemsep`, keep the largest natural
//! skip, not the sum. The pipeline used the outermost level's `\topsep`
//! (8 pt at 10 pt, not `\@listii`'s 4 pt) and added the `\itemsep` on top,
//! so display-placement fixture 20's `2. Outer again` was 7.97 bp low.
//!
//! A paragraph right after `\end{itemize}` (no blank line) is not indented
//! (`\@endparenv` sets `\@endpetrue`), as after `center` or `quote`.
//!
//! ## Oracle
//!
//! pdfTeX 1.40 (MacTeX 2026), pdftotext word origins (`oracle.py refs`
//! conventions) of the probes below, 10 pt T1 Latin Modern, in bp: the
//! baseline differences asserted here are `\baselineskip` 12 pt plus
//!
//! * `(b) D` after level 3 ends: max(`\topsep` iii 2 pt, `\itemsep` ii 2 pt)
//!   + `\parsep` ii 2 pt = 4 pt -> 15.940 bp;
//! * `2. E` after level 2 ends: max(4 pt, `\itemsep` i 4 pt) + `\parsep` i
//!   4 pt = 8 pt -> 19.925 bp;
//! * `After` after levels 2 and 1 end: max(4 pt, 8 pt + `\partopsep` 2 pt)
//!   + `\parskip` 0 pt = 10 pt -> 21.918 bp.
//!
//! pdflatex is an oracle only and never runs in the product path.

mod common;

const TOL: f64 = 0.5;

const HEAD: &str = "\\documentclass[10pt]{article}\n\\usepackage[T1]{fontenc}\n\\usepackage{lmodern}\n\\pagestyle{empty}\n\\begin{document}\n";

fn words(body: &str) -> Vec<common::Word> {
    common::words_of(&common::render_one(&format!("{HEAD}{body}\\end{{document}}\n")))
}

fn at<'a>(words: &'a [common::Word], text: &str) -> &'a common::Word {
    words
        .iter()
        .find(|w| w.text.trim() == text)
        .unwrap_or_else(|| panic!("no run `{text}` in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
}

fn check(label: &str, got: f64, expect: f64) {
    assert!((got - expect).abs() <= TOL, "{label}: {got:.3} bp, pdflatex {expect:.3} bp ({:+.3})", got - expect);
}

#[test]
fn a_closing_level_uses_its_own_topsep_and_addvspace_keeps_the_larger_skip() {
    if !common::lm_available() {
        eprintln!("SKIP nested_list_end_skips: Latin Modern not installed");
        return;
    }
    let w = words(
        "\\begin{enumerate}\n\\item A\n\\begin{enumerate}\n\\item B\n\\begin{enumerate}\n\\item C\n\\end{enumerate}\n\\item D\n\\end{enumerate}\n\\item E\n\\end{enumerate}\n",
    );
    check("C -> D (level 3 closes)", at(&w, "D").baseline - at(&w, "C").baseline, 186.57 - 170.63);
    check("D -> E (level 2 closes)", at(&w, "E").baseline - at(&w, "D").baseline, 206.496 - 186.57);
}

#[test]
fn two_lists_closing_together_keep_the_outer_topsep_and_partopsep() {
    if !common::lm_available() {
        eprintln!("SKIP nested_list_end_skips: Latin Modern not installed");
        return;
    }
    let w = words("\\begin{itemize}\n\\item Outer item.\n\\begin{itemize}\n\\item Inner.\n\\end{itemize}\n\\end{itemize}\nAfter list.\n");
    check("Inner -> After", at(&w, "After").baseline - at(&w, "Inner.").baseline, 176.608 - 154.69);
    // `\@endpe`: flush left, like the page's own left margin.
    check("After is not indented", at(&w, "After").x, 133.768);
}

#[test]
fn text_after_an_inner_list_continues_the_outer_item() {
    if !common::lm_available() {
        eprintln!("SKIP nested_list_end_skips: Latin Modern not installed");
        return;
    }
    let w = words("\\begin{itemize}\n\\item Outer item.\n\\begin{itemize}\n\\item Inner.\n\\end{itemize}\ncontinued outer text.\n\\item Next.\n\\end{itemize}\nAfter list.\n");
    check("Inner -> continued", at(&w, "continued").baseline - at(&w, "Inner.").baseline, 174.615 - 154.69);
    check("Next -> After", at(&w, "After").baseline - at(&w, "Next.").baseline, 216.458 - 194.54);
}
