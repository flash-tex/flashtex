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
