//! Two spaces with a command that sets nothing between them are two
//! interword glues, as in TeX, unless the command is one of latex.ltx's
//! `\@bsphack`..`\@esphack` pairs (issue #1124).
//!
//! pdflatex, TeX Live 2026, `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`,
//! `\showbox` of `\hbox{<case>}` in an `article` (the glue nodes, counting
//! `\@esphack`'s zero `\hskip\z@skip` out):
//!
//! * two glues: `a \nothing{x} b` (`\newcommand{\nothing}[1]{}`), `a {} b`,
//!   `a \setcounter{footnote}{1} b`, `a \pagestyle{empty} b`,
//!   `a \label{x} \label{y} b` (the first `\@esphack` leaves its zero skip
//!   as the last node, so the second label skips nothing), and
//!   `a \nothing{x} \label{y} b`;
//! * three: `a \nothing{x} \nothing{y} b`;
//! * one: `a \label{x} b`, `a \index{x} b`, `a \nocite{x} b`,
//!   `a \label{y} \nothing{x} b` (`\ignorespaces` expands `\nothing` away
//!   too), `a \color{red} b` (color.sty's own `\ignorespaces`).
//!
//! In a paragraph the words after the command move by the same amount:
//! pdflatex sets `in` of `Some text \nothing{x} in the middle of a
//! paragraph.` at x 199.35bp, one interword space (3.32bp) right of
//! `Some text \label{x} in ...`'s 196.04bp; before this FlashTeX set both at
//! 196.04bp.
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(preamble: &str, body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n{preamble}\\newcommand{{\\nothing}}[1]{{}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

/// The first paragraph's interword glues (`glue_before` of its runs).
fn glues_in(preamble: &str, body: &str) -> usize {
    let parsed = parse(&doc(preamble, body));
    let inlines = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("a paragraph");
    inlines
        .iter()
        .filter(|inline| {
            matches!(inline, Inline::Text { glue_before: Some(_), .. } | Inline::Math { glue_before: Some(_), .. })
        })
        .count()
}

fn glues(body: &str) -> usize {
    glues_in("", body)
}

#[test]
fn a_command_that_sets_nothing_leaves_both_spaces() {
    assert_eq!(glues("a \\nothing{x} b"), 2);
    assert_eq!(glues("a \\nothing{x} \\nothing{y} b"), 3);
    assert_eq!(glues("a {} b"), 2);
    assert_eq!(glues("a \\setcounter{footnote}{1} b"), 2);
    assert_eq!(glues("a \\refstepcounter{footnote} b"), 2);
    assert_eq!(glues("a \\pagestyle{empty} b"), 2);
    assert_eq!(glues("a \\setlength{\\parskip}{0pt} b"), 2);
    // One space and a comment: still one glue.
    assert_eq!(glues("a \\nothing{x}%\n b"), 1);
    assert_eq!(glues("a b"), 1);
}

#[test]
fn bsphack_esphack_commands_do_not_double_the_space() {
    assert_eq!(glues("a \\label{x} b"), 1);
    assert_eq!(glues("a\\label{x} b"), 1);
    assert_eq!(glues("a \\label{x}b"), 1);
    assert_eq!(glues("a \\nocite{x} b"), 1);
    assert_eq!(glues("a \\enlargethispage{1pt} b"), 1);
    assert_eq!(glues_in("\\usepackage{makeidx}\\makeindex\n", "a \\index{x} b"), 1);
    assert_eq!(glues("a \\label{y} \\nothing{x} b"), 1);
    assert_eq!(glues_in("\\usepackage{xcolor}\n", "a \\color{red} b"), 1);
}

#[test]
fn esphack_skips_nothing_after_its_own_zero_skip() {
    assert_eq!(glues("a \\label{x} \\label{y} b"), 2);
    assert_eq!(glues("a \\nothing{x} \\label{y} b"), 2);
}

/// The first paragraph's inlines.
fn first_paragraph(preamble: &str, body: &str) -> Vec<Inline> {
    parse(&doc(preamble, body))
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("a paragraph")
}

fn empty_glue_runs(inlines: &[Inline]) -> usize {
    inlines
        .iter()
        .filter(|inline| matches!(inline, Inline::Text { text, glue_before: Some(_), .. } if text.is_empty()))
        .count()
}

/// `CJK*` drops a blank after a CJK character (`\CJK@ignorespaces`), which
/// the renderer decides from the run the glue follows, so such a blank is
/// not an empty run of its own. pdflatex sets `y` of
/// `\begin{CJK*}{UTF8}{min}東 \end{CJK*} y` one interword space after `東`
/// (render-pipeline `tests/cjk_env.rs`); an empty run in between hid `東`
/// and cost that space (3.32bp at 10pt).
#[test]
fn a_cjk_star_blank_is_not_promoted_to_its_own_glue() {
    let inlines = first_paragraph("\\usepackage{CJK}\n", "\\begin{CJK*}{UTF8}{min}東 \\end{CJK*} y and");
    assert_eq!(empty_glue_runs(&inlines), 0, "{inlines:?}");
    assert!(
        inlines.iter().any(|i| matches!(i, Inline::Text { text, glue_before: Some(_), .. } if text == "y")),
        "{inlines:?}"
    );
}

/// `\par` removes the glue a paragraph ends with; a space read after an
/// `\end` the parser does not track (`abstract`, set by the renderer) is
/// vertical mode. Neither leaves an empty glue run at the paragraph's end
/// (render-pipeline `tests/vmode_boundary_skips.rs`: `abstract -> center`
/// moved 1.99bp with one).
#[test]
fn a_paragraph_does_not_end_with_an_empty_glue_run() {
    let inlines = first_paragraph("", "\\begin{abstract}\nalpha\n\\end{abstract}\n\\begin{center}bravo\\end{center}");
    assert_eq!(empty_glue_runs(&inlines), 0, "{inlines:?}");
    let inlines = first_paragraph("", "alpha \\nothing{x} \n\nbravo");
    assert_eq!(empty_glue_runs(&inlines), 0, "{inlines:?}");
    // Mid-paragraph the second glue stays.
    let inlines = first_paragraph("", "alpha \\nothing{x} bravo");
    assert_eq!(empty_glue_runs(&inlines), 1, "{inlines:?}");
}
