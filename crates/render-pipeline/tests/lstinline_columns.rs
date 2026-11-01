//! listings' column bookkeeping inside a `\lstinline` (`listings::set_inline`,
//! `adapter::ListingMark`): under `flexiblecolumns` every token keeps its
//! natural width, but each character is booked at `\lst@width` (0.45 em of
//! the `basicstyle` face) in `\lst@lostspace`, a dimen that runs negative
//! under a typewriter face and positive under a proportional one. A blank
//! gobbled at the start of the argument or after another blank adds one
//! `\lst@width`, and whatever is positive before the next token is set as
//! a kern box; a token narrower than its columns is padded to them, half on
//! each side. The face is `basicstyle`'s, else the one around the command.
//!
//! Before this the pipeline dropped the gobbled blanks and set every
//! `\lstinline` in typewriter: `| x1  y2 --3 |` came out 7.8 bp narrower
//! than pdflatex's, enough for a line to fit here that breaks there.
//!
//! Oracle: `/Library/TeX/texbin/pdflatex` (pdfTeX 3.141592653-2.6-1.40.29,
//! TeX Live 2026, MacTeX; never in the product path) on exactly the
//! documents below, word origins read from the PDF's content stream with
//! `tools/visual-oracle/pdftext.py` (bp, page top-left origin), and the
//! node lists from `\tracingoutput`. No TeX runs here.

mod common;

use common::*;

const PREAMBLE: &str = "\\documentclass[11pt]{article}\n\\usepackage[T1]{fontenc}\n\\usepackage[margin=1in]{geometry}\n\\usepackage{listings}\n\\lstset{basicstyle=\\ttfamily\\small,breaklines=true}\n\\setlength{\\parskip}{0pt}\n\\begin{document}\n\\pagestyle{empty}\n";
const TAIL: &str = "and the paragraph goes on for a while after that until it ends on the next line.";

/// The first-page word whose text starts with `text` at `x` within 0.05
/// bp, on the line at `baseline` within 0.05 bp.
fn word_at<'a>(words: &'a [Word], text: &str, x: f64, baseline: f64) -> &'a Word {
    words
        .iter()
        .find(|w| w.page == 1 && w.text.starts_with(text) && (w.x - x).abs() < 0.05 && (w.baseline - baseline).abs() < 0.05)
        .unwrap_or_else(|| {
            let seen: Vec<_> = words.iter().filter(|w| w.page == 1).map(|w| format!("{:.3},{:.3} {:?}", w.x, w.baseline, w.text)).collect();
            panic!("no word {text:?} at x={x} y={baseline}; page 1 has {seen:#?}")
        })
}

/// pdflatex, `\noindent\hspace*{16pt}Filler \lstinline| x1  y2 --3 |, and
/// the paragraph ...`, page 1:
///
/// ```text
/// y= 82.959  Filler @87.940  x1 @121.842  y2 @140.670  --3 @156.349  , @177.273 ... next @519.212
/// y= 96.508  line. @72.000
/// ```
///
/// `\tracingoutput`: `\hbox x4.72498` (`\kern 4.72498`, the leading blank's
/// `\lst@width` = 0.45 × 10.49995 pt), `\discretionary`, `\hbox x10.49744`
/// (`x1`), `\discretionary`, `\hbox x5.24872` (`\glue 5.24872`),
/// `\discretionary`, `\hbox x3.15376` (the doubled blank's 4.72498 less the
/// 1.57122 the two boxes before it ran over), `\discretionary`, `y2`, …,
/// `--3` as one box (the digit joins the open run), the trailing blank's
/// box, then the roman `,`. So `y2` is 18.827 bp after `x1` and the `,`
/// 5.229 bp after `--3`'s end; without the two kerns the line is 7.8 bp
/// shorter and `line.` fits on it.
#[test]
fn gobbled_blanks_become_lost_space_kerns_like_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = format!("{PREAMBLE}\\noindent\\hspace*{{16pt}}Filler \\lstinline| x1  y2 --3 |, {TAIL}\n\n\\end{{document}}\n");
    let r = render_one(&doc);
    let overfull: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.code == "overfull_hbox").collect();
    assert!(overfull.is_empty(), "pdflatex reports no overfull box: {overfull:?}");
    let words = words_of(&r);
    let y1 = 82.959;
    word_at(&words, "Filler", 87.940, y1);
    let x1 = word_at(&words, "x1", 121.842, y1);
    let y2 = word_at(&words, "y2", 140.670, y1);
    assert!((y2.x - x1.x - 18.827).abs() < 0.01, "`y2` follows `x1` by 10.49744 + 5.24872 + 3.15376 pt: {:.3}", y2.x - x1.x);
    let dashes = word_at(&words, "--3", 156.349, y1);
    assert!((dashes.width - 15.74616 * 72.0 / 72.27).abs() < 0.01, "`--3` is one box of three characters: {:.3}", dashes.width);
    let comma = word_at(&words, ",", 177.273, y1);
    assert!((comma.x - (dashes.x + dashes.width) - 5.24872 * 72.0 / 72.27).abs() < 0.01, "the trailing blank's box precedes the `,`: {:.3}", comma.x);
    word_at(&words, "next", 519.212, y1);
    word_at(&words, "line.", 72.0, 96.508);
}

/// pdflatex, `\noindent Filler \lstinline[basicstyle=\small]|il il  fi(a) x|,
/// and the paragraph ...` — a roman `basicstyle`, so `SFRM1000` — page 1:
///
/// ```text
/// y= 82.959  Filler @72.000  il @102.286  il @115.729  fi @133.656  (a) @141.212  x, @158.831 ... line. @520.111
/// ```
///
/// `\tracingoutput`: `\hbox x8.99774` holding `\hbox x1.72177` (`\kern`),
/// `\hbox x5.5542` (`^^\ (ligature fi)`), `\hbox x1.72177` — a token of
/// two columns (2 × 4.49887) padded around its 5.5542 pt; `\hbox x11.38611`
/// (`T`, `\kern-0.83313`, `o`). Here: each `il` (5.5542) padded to 8.99774
/// pt, so the two are 8.99774 + 4.49887 (the blank, itself padded from
/// 3.33252) = 13.49661 pt = 13.443 bp apart; `fi` a further 8.99774 +
/// 4.49887 + 4.49887 (the doubled blank's kern) = 17.996 pt = 17.927 bp;
/// `(` 7.584 pt after `fi`'s origin (8.99774 − 1.72177 + 0.30768, half of
/// `(`'s own padding from 3.8835 to 4.49887).
#[test]
fn a_proportional_basicstyle_pads_narrow_tokens_like_pdflatex() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = format!("{PREAMBLE}\\noindent Filler \\lstinline[basicstyle=\\small]|il il  fi(a) x|, {TAIL}\n\n\\end{{document}}\n");
    let r = render_one(&doc);
    let words = words_of(&r);
    let y1 = 82.959;
    word_at(&words, "Filler", 72.0, y1);
    let il1 = word_at(&words, "il", 102.286, y1);
    let il2 = word_at(&words, "il", 115.729, y1);
    assert!((il2.x - il1.x - 13.49661 * 72.0 / 72.27).abs() < 0.01, "`il il`: {:.3}", il2.x - il1.x);
    let fi = word_at(&words, "fi", 133.656, y1);
    assert!((fi.x - il2.x - 17.99548 * 72.0 / 72.27).abs() < 0.01, "`il  fi`: {:.3}", fi.x - il2.x);
    assert!((fi.width - 5.5542 * 72.0 / 72.27).abs() < 0.01, "`fi` ligates: {:.3}", fi.width);
    // `(`, `a` and `)` are three boxes; pdftext reads them as one word.
    let paren = word_at(&words, "(", 141.212, y1);
    assert!((paren.x - fi.x - 7.58365 * 72.0 / 72.27).abs() < 0.01, "`fi(`: {:.3}", paren.x - fi.x);
    word_at(&words, "x", 158.831, y1);
    word_at(&words, "and", 170.550, y1);
    word_at(&words, "line.", 520.111, y1);
}

/// pdflatex, `\noindent\hspace*{254pt}Filler words here and some more of
/// them, ten in all, \lstinline[breaklines=false]|one  two three four
/// five|, and the paragraph ...`, page 1:
///
/// ```text
/// y= 82.959  Filler @325.051 ... ten @524.930
/// y= 96.508  in @72.000  all, @84.751  one @102.945  two @126.474  three @147.393  four @178.760  five, @204.908 ... on @528.548
/// y=110.057  the @72.000  next @90.679  line. @115.093
/// ```
///
/// Without `breaklines` no `\lst@discretionary` follows the boxes, so the
/// argument moves to the next line whole (`ten` ends the first at 524.930,
/// where 469.755 pt of text width end at 539.755 bp); the doubled blank's
/// kern is still set: `two` is 15.74616 + 5.24872 + 2.63003 pt = 23.529 bp
/// after `one`.
#[test]
fn without_breaklines_the_argument_moves_whole_and_keeps_its_kerns() {
    if !lm_available() {
        eprintln!("skipping: Latin Modern not installed");
        return;
    }
    let doc = format!("{PREAMBLE}\\noindent\\hspace*{{254pt}}Filler words here and some more of them, ten in all, \\lstinline[breaklines=false]|one  two three four five|, {TAIL}\n\n\\end{{document}}\n");
    let r = render_one(&doc);
    let overfull: Vec<_> = r.v2.diagnostics.iter().filter(|d| d.code == "overfull_hbox").collect();
    assert!(overfull.is_empty(), "pdflatex reports no overfull box: {overfull:?}");
    let words = words_of(&r);
    word_at(&words, "ten", 524.930, 82.959);
    let y2 = 96.508;
    word_at(&words, "in", 72.0, y2);
    let one = word_at(&words, "one", 102.945, y2);
    let two = word_at(&words, "two", 126.474, y2);
    assert!((two.x - one.x - 23.62491 * 72.0 / 72.27).abs() < 0.01, "`one  two`: {:.3}", two.x - one.x);
    word_at(&words, "five", 204.908, y2);
    word_at(&words, "on", 528.548, y2);
    word_at(&words, "the", 72.0, 110.057);
    word_at(&words, "line.", 115.093, 110.057);
}
