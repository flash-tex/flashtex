//! `\twocolumn[<material>]`: `\@topnewpage`'s full-width banner.
//!
//! Every number is a glyph origin read off the PDF
//! `/Library/TeX/texbin/pdflatex` (pdfTeX 3.141592653-2.6-1.40.29, TeX Live
//! 2026) produces for the probe quoted in the test, measured with PyMuPDF.
//! pdflatex is an oracle only and never runs in the product path.
//!
//! #746 landed `\twocolumn` as document state and left the optional
//! argument set *in the column, brackets included*. LaTeX sets it in a box
//! of its own (latex.ltx 20466-20505):
//!
//! ```text
//! \long\def \@topnewpage [#1]{%
//!   \global \setbox\@currbox \vbox{\hsize\textwidth \@parboxrestore
//!                                  \col@number\@ne #1 \vskip -\dbltextfloatsep}%
//!   \ifdim \ht\@currbox>\textheight \ht\@currbox \textheight \fi
//!   \@tempdima -\ht\@currbox \advance \@tempdima -\dbltextfloatsep
//!   \global \advance \@colht \@tempdima ...
//! ```
//!
//! The trailing `\vskip -\dbltextfloatsep` makes `\ht\@currbox` the
//! material's natural height *less* `\dbltextfloatsep` (and its depth zero,
//! the last list item being glue); `\@colht` then loses `\ht\@currbox +
//! \dbltextfloatsep`, and `\@combinedblfloats` (21019) stacks the box,
//! `\vskip\dbltextfloatsep` and the two-column box in a `\vbox
//! to\textheight`. **The `\dbltextfloatsep` cancels**: the columns begin
//! exactly the material's natural height below the top of the text area,
//! and lose exactly that much height. Measured at 10 pt, a one-line banner
//! whose last line has no descender: banner baseline 131.720, columns from
//! 141.683 where a plain `\twocolumn` starts at 134.765 — a shift of 6.918
//! bp = 6.9444 pt, which is cmr10's ascender height, the whole box.
//!
//! `\@outputpage` restores `\global\@colht\textheight`, so **only the page
//! the command starts carries the box**: page 2 of a document whose columns
//! overflow begins at the plain two-column top (134.765 at 10 pt), measured.

mod common;

use common::*;
use flashtex_compiler::parser::SourceDocument;
use flashtex_render_pipeline::v1::Capabilities;
use flashtex_render_pipeline::{render, FontSet, RenderOptions};

#[derive(Debug, Clone)]
struct Word {
    text: String,
    x: f64,
    baseline: f64,
    page: usize,
}

fn layout(text: &str) -> (Vec<String>, Vec<Word>) {
    let fonts = FontSet::with_default_dirs(&[]);
    let docs = [SourceDocument { path: "main.tex", text }];
    let r = render(&docs, "main.tex", 4, "p", &fonts, &RenderOptions::default());
    let v1 = v1_of(&r, Capabilities { rules: true, font_hints: true, ..Capabilities::default() });
    assert_ne!(v1.status, "failed", "{:?}", v1.diagnostics);
    let mut words = Vec::new();
    for (p, page) in r.v2.pages.iter().enumerate() {
        for it in page.resident_items() {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let Some(first) = run.glyphs.first() else { continue };
                words.push(Word {
                    text: run.text.clone(),
                    x: first.origin_x.to_bp(),
                    baseline: first.baseline_y.to_bp(),
                    page: p + 1,
                });
            }
        }
    }
    (v1.diagnostics.iter().map(|d| d.code.clone()).collect(), words)
}

fn close(got: f64, want: f64, what: &str) {
    assert!((got - want).abs() <= 0.5, "{what}: {got} vs pdflatex {want} ({:+})", got - want);
}

/// A paragraph of short words: nothing hyphenates and nothing ligates, so a
/// difference in the probe is a difference in the page, not in the line
/// breaker.
const FILL: &str = "aaa bbb ccc ddd eee ggg hhh iii jjj kkk lll mmm nnn ooo ppp qqq rrr sss ttt uuu vvv www xxx yyy zzz. ";

/// The document: `head` right after `\begin{document}`, then `n` paragraphs
/// each opening with the marker word `Zulu`.
fn doc(size: &str, head: &str, n: usize) -> String {
    let mut s = format!("\\documentclass[{size}]{{article}}\n\\pagestyle{{empty}}\n\\begin{{document}}\n{head}\n");
    for _ in 0..n {
        s.push_str("Zulu ");
        s.push_str(FILL);
        s.push_str(FILL);
        s.push_str("\n\n");
    }
    s.push_str("\\end{document}\n");
    s
}

/// The banner's marker word and the first `Zulu`: the banner's own first
/// line and the first line of the columns under it.
fn probe(source: &str, banner_word: &str) -> (Vec<String>, Word, Word) {
    let (codes, words) = layout(source);
    let first = words
        .iter()
        .find(|w| w.text.trim_start().starts_with(banner_word))
        .unwrap_or_else(|| panic!("no banner word {banner_word:?} in {:?}", words.iter().map(|w| &w.text).collect::<Vec<_>>()))
        .clone();
    let zulu = words.iter().find(|w| w.text.trim_end().ends_with("Zulu")).expect("the Zulu marker").clone();
    (codes, first, zulu)
}

/// `(size, banner baseline, banner x, first column baseline, column x)`.
type Row = (&'static str, f64, f64, f64, f64);

fn check(head: &str, paragraphs: usize, banner_word: &str, rows: [Row; 3]) {
    for (size, by, bx, cy, cx) in rows {
        let (codes, first, zulu) = probe(&doc(size, head, paragraphs), banner_word);
        // `math_resource_profile` is a font-loading note, not a page fact.
        assert!(codes.iter().all(|c| c == "math_resource_profile"), "{size}: {codes:?}");
        assert_eq!(first.page, 1);
        assert_eq!(zulu.page, 1);
        close(first.baseline, by, &format!("{size} banner baseline"));
        close(first.x, bx, &format!("{size} banner x"));
        close(zulu.baseline, cy, &format!("{size} first column baseline"));
        close(zulu.x, cx, &format!("{size} first column x"));
    }
}

/// A one-line banner. The columns start the box's natural height below the
/// text top -- here one `cmr` ascender, no descender in the line -- and the
/// banner itself is set flush left at the full `\textwidth`
/// (`\@parboxrestore` zeroes `\parindent`), while the column's first
/// paragraph is indented as usual.
#[test]
fn a_one_line_banner_is_set_at_the_text_width_and_the_columns_start_under_it() {
    check(
        "\\twocolumn[Short banner line]",
        3,
        "Short",
        [
            ("10pt", 131.720, 133.768, 141.683, 148.712),
            ("11pt", 137.359, 125.798, 148.318, 142.735),
            ("12pt", 134.100, 110.854, 146.056, 128.413),
        ],
    );
}

/// The same probe with a descender in the banner's only line. TeX gives the
/// `\vbox` a height that includes the last box's depth, because the
/// `\vskip -\dbltextfloatsep` after it is glue -- so the columns move down
/// by the descender too: 1.937/2.121/2.324 bp, which is `cmr`'s 0.19444 em
/// at 10/10.95/12 pt.
#[test]
fn the_last_lines_depth_counts_toward_the_box() {
    check(
        "\\twocolumn[Short banner line with a descender: jumpy.]",
        3,
        "Short",
        [
            ("10pt", 131.720, 133.768, 143.620, 148.712),
            ("11pt", 137.359, 125.798, 150.439, 142.735),
            ("12pt", 134.100, 110.854, 148.380, 128.413),
        ],
    );
}

/// Two paragraphs of banner: the columns follow both of them.
#[test]
fn the_columns_follow_a_two_paragraph_banner() {
    check(
        "\\twocolumn[Short banner line\\par Second banner line]",
        3,
        "Short",
        [
            ("10pt", 131.720, 133.768, 153.638, 148.712),
            ("11pt", 137.359, 125.798, 161.867, 142.735),
            ("12pt", 134.100, 110.854, 160.501, 128.413),
        ],
    );
}

/// A banner long enough to wrap: it is broken to `\textwidth`, not to
/// `\columnwidth`, and the columns start under however many lines that
/// takes (five at 10 and 12 pt, six at 11 pt).
#[test]
fn the_columns_start_under_a_banner_that_wraps() {
    let wide = format!(
        "\\twocolumn[Wide banner material {}]",
        "uuu vvv www xxx yyy zzz aaa bbb ccc ddd eee ggg hhh iii jjj kkk lll mmm nnn ooo ppp qqq rrr sss ttt. ".repeat(6)
    );
    check(
        &wide,
        3,
        "Wide",
        [
            ("10pt", 131.720, 133.768, 239.262, 148.712),
            ("11pt", 137.359, 125.798, 258.833, 142.735),
            ("12pt", 134.100, 110.854, 263.947, 128.413),
        ],
    );
}

/// `\@topnewpage`'s `\vbox` begins in vertical mode, so an environment that
/// opens the material is a `\begin` read there: `\@topsepadd` keeps
/// `\partopsep`, and `\@endparenv` keeps it again at the close. Reading the
/// flag off the source instead (what precedes the `\begin` is `\twocolumn[`)
/// lost 2 x `\partopsep` -- 4.04/6.04/6.05 bp.
#[test]
fn an_environment_that_opens_the_box_begins_in_vertical_mode() {
    check(
        "\\twocolumn[\\begin{center}Centred banner\\end{center}]",
        3,
        "Centred",
        [
            ("10pt", 141.683, 271.696, 161.608, 148.712),
            ("11pt", 149.314, 267.974, 172.228, 142.735),
            ("12pt", 147.052, 265.293, 171.958, 128.413),
        ],
    );
}

/// A size declaration in the banner: the box is as tall as the material it
/// holds, whatever that material's leading is.
#[test]
fn a_size_declaration_in_the_banner_sets_the_boxs_height() {
    check(
        "\\twocolumn[{\\Large Large banner line}]",
        3,
        "Large",
        [
            ("10pt", 134.765, 133.768, 147.517, 148.712),
            ("11pt", 139.746, 125.798, 153.494, 142.735),
            ("12pt", 137.753, 110.854, 153.057, 128.413),
        ],
    );
}

/// Inline math in the banner: its height is the line's, so the box and the
/// columns follow it.
#[test]
fn math_in_the_banner_sets_the_boxs_height() {
    check(
        "\\twocolumn[Banner with math $x^2+y^2=z^2$ inside.]",
        3,
        "Banner",
        [
            ("10pt", 132.912, 133.768, 144.811, 148.712),
            ("11pt", 138.878, 125.798, 151.959, 142.735),
            ("12pt", 135.273, 110.854, 149.553, 128.413),
        ],
    );
}

/// `\@outputpage` restores `\global\@colht\textheight`, so the box belongs
/// to the page the command starts and to no other: page 2 has no banner and
/// its columns begin at the plain two-column top.
#[test]
fn the_banner_is_on_its_own_page_only() {
    for (size, banner, page2_top) in [("10pt", 131.720, 134.765), ("11pt", 137.359, 140.742), ("12pt", 134.100, 137.753)] {
        let (codes, words) = layout(&doc(size, "\\twocolumn[Short banner line]", 16));
        assert!(codes.is_empty(), "{size}: {codes:?}");
        assert!(words.iter().any(|w| w.page == 2), "{size}: the probe must overflow onto page 2");
        let banner_words: Vec<&Word> = words.iter().filter(|w| w.text.trim_start().starts_with("Short")).collect();
        assert_eq!(banner_words.len(), 1, "{size}: the banner must not repeat");
        close(banner_words[0].baseline, banner, &format!("{size} banner baseline"));
        assert_eq!(banner_words[0].page, 1);
        let top = words
            .iter()
            .filter(|w| w.page == 2)
            .map(|w| w.baseline)
            .fold(f64::INFINITY, f64::min);
        close(top, page2_top, &format!("{size} page 2 top"));
    }
}

/// `\twocolumn[]`. `\ht\@currbox` is `-\dbltextfloatsep`, which the `\vskip
/// \dbltextfloatsep` under it gives straight back, so `\@colht` loses
/// nothing: measured, the page is the one a plain `\twocolumn` sets.
#[test]
fn an_empty_optional_argument_leaves_the_page_alone() {
    for (size, top) in [("10pt", 134.765), ("11pt", 140.742), ("12pt", 137.753)] {
        let (codes, empty) = layout(&doc(size, "\\twocolumn[]", 3));
        assert!(codes.is_empty(), "{size}: {codes:?}");
        let (_, plain) = layout(&doc(size, "\\twocolumn", 3));
        assert_eq!(empty.len(), plain.len(), "{size}");
        for (a, b) in empty.iter().zip(&plain) {
            assert_eq!((a.page, a.text.clone()), (b.page, b.text.clone()), "{size}");
            close(a.baseline, b.baseline, &format!("{size} baseline vs plain \\twocolumn"));
            close(a.x, b.x, &format!("{size} x vs plain \\twocolumn"));
        }
        close(empty[0].baseline, top, &format!("{size} first baseline"));
    }
}

/// A sectioning command in the argument is page-level material that
/// `Context::box_blocks` does not set in a box. Rather than drop it, the
/// split is refused and the argument stays where #746 left it, reported.
#[test]
fn a_sectioning_command_in_the_argument_is_still_reported() {
    let (codes, _) = layout(&doc("10pt", "\\twocolumn[\\section*{Head}]", 1));
    assert!(codes.iter().any(|c| c == "twocolumn_top_material"), "{codes:?}");
}

/// Material after the banner that carries no entry-document range — a
/// `tikzpicture` here, an `\input` file's blocks, a `longtable` or the
/// `\tableofcontents` — must not refuse the whole split. It stands past the
/// `]`, so it joins the kept blocks without needing a source range of its
/// own, and the banner is still set at the full `\textwidth`.
#[test]
fn trailing_unpositioned_material_does_not_refuse_the_box() {
    let src = "\\documentclass[10pt]{article}\n\\pagestyle{empty}\n\\begin{document}\n\
        \\twocolumn[Short banner line]\nZulu aaa bbb ccc ddd eee ggg hhh iii jjj kkk lll mmm.\n\n\
        \\begin{tikzpicture}\n\\draw (0,0) -- (2,1);\n\\end{tikzpicture}\n\n\
        Tail words here.\n\\end{document}\n";
    let (codes, first, zulu) = probe(src, "Short");
    assert!(!codes.iter().any(|c| c == "twocolumn_top_material"), "{codes:?}");
    close(first.baseline, 131.720, "banner baseline");
    close(first.x, 133.768, "banner x");
    close(zulu.baseline, 141.683, "first column baseline");
}

/// A blank line between `\twocolumn` and `[x]` is a `\par`, not a space
/// token, so `\@ifnextchar [` never sees the bracket: real pdflatex typesets
/// `[x]` as ordinary text, and there is no `twocolumn_top_material` to
/// report.
#[test]
fn a_blank_line_before_the_bracket_is_plain_text_not_a_warning() {
    let src = "\\documentclass[10pt]{article}\n\\begin{document}\n\\twocolumn\n\n[x]\nAaa\n\\end{document}\n";
    let (codes, words) = layout(src);
    assert!(!codes.iter().any(|c| c == "twocolumn_top_material"), "{codes:?}");
    assert!(words.iter().any(|w| w.text.contains('[')), "the bracket is typeset as ordinary text: {words:?}");
}

/// A `\twocolumn` after material cannot open the box (`\@topnewpage` runs
/// `\@nodocument`, and the frame cannot change the column count mid-document
/// either), so its argument is left alone and reported as before.
#[test]
fn a_twocolumn_after_material_does_not_open_a_box() {
    let src = "\\documentclass[10pt]{article}\n\\begin{document}\nAaa\n\n\\twocolumn[Banner]\nBbb\n\\end{document}\n";
    let (codes, _) = layout(src);
    assert!(codes.iter().any(|c| c == "twocolumn_mid_document"), "{codes:?}");
    assert!(codes.iter().any(|c| c == "twocolumn_top_material"), "{codes:?}");
}
