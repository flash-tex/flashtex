//! Without amsthm, `\newtheorem` is the LaTeX kernel's, and its head is not
//! amsthm's. latex.ltx `\@begintheorem`/`\@opargbegintheorem`:
//!
//! ```tex
//! \item[\hskip \labelsep{\bfseries #1\ #2}]\itshape
//! \item[\hskip \labelsep{\bfseries #1\ #2\ (#3)}]\itshape
//! ```
//!
//! so the note is bold like the name and number, and there is no head
//! punctuation. pdflatex (TeX Live 2026, oracle only, 11pt article, no
//! amsthm) sets `\begin{theorem}[Lock cleanup explicitly]` as `Theorem 1
//! (Lock cleanup explicitly)` all in `\OT1/cmr/bx/n/10.95` and the body in
//! `\OT1/cmr/m/it/10.95`. The parser used to give every document amsthm's
//! `Theorem 1 (Lock cleanup explicitly).` with an upright medium note.
//!
//! The render pipeline tells the two heads apart by amsthm's `.` run on the
//! `\begin` span (`render-pipeline/src/amsthm.rs`), so the tests pin that
//! run's presence and absence too.
use flashtex_compiler::parser::{parse, Block, Inline};

fn head(preamble: &str, body: &str) -> Vec<(String, bool, bool, bool)> {
    let source = format!(
        "\\documentclass[11pt]{{article}}\n{preamble}\\newtheorem{{theorem}}{{Theorem}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let begin = source.find("\\begin{theorem}").expect("a theorem");
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(
                inlines
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Text { text, style, span, .. } => {
                            Some((text.clone(), style.bold, style.italic, span.start == begin))
                        }
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .expect("a paragraph")
}

fn texts(runs: &[(String, bool, bool, bool)]) -> Vec<&str> {
    runs.iter().map(|(text, ..)| text.as_str()).collect()
}

#[test]
fn a_kernel_head_has_a_bold_note_and_no_punctuation() {
    let runs = head("", "\\begin{theorem}[Lock cleanup]\nunlocks on drop.\n\\end{theorem}");
    assert_eq!(texts(&runs), ["Theorem 1", " ", "(", "Lock", "cleanup", ")", "unlocks", "on", "drop."]);
    for (text, bold, italic, _) in &runs[..6] {
        assert!(*bold && !*italic, "`{text}` is `\\bfseries` upright: {runs:?}");
    }
    let (_, bold, italic, _) = &runs[6];
    assert!(!*bold && *italic, "the body is `\\itshape`: {runs:?}");
    assert!(
        !runs.iter().any(|(text, _, _, on_begin)| text == "." && *on_begin),
        "no `\\thm@headpunct` run: {runs:?}"
    );
}

#[test]
fn a_kernel_head_without_a_note_is_name_and_number() {
    let runs = head("", "\\begin{theorem}\nBody.\n\\end{theorem}");
    assert_eq!(texts(&runs), ["Theorem 1", "Body."]);
    assert!(runs[0].1 && runs[1].2, "{runs:?}");
}

/// The kernel theorem is a plain `\trivlist`: the first block records the
/// mode `\begin` was read in (`\@trivlist` adds `\partopsep` in vertical
/// mode), and `\@endparenv`'s `\@endpe` leaves text straight after `\end`
/// unindented. pdflatex (TeX Live 2026, 11pt, no amsthm): a theorem after a
/// blank line sits 3pt further from the text above and below than one
/// opened straight after text, and `Echo` after `\end{theorem}` starts at
/// the margin (amsthm's `\@endtheorem` adds `\@endpefalse`, so there it is
/// indented).
#[test]
fn a_kernel_theorem_is_a_plain_trivlist() {
    for (preamble, kernel) in [("", true), ("\\usepackage{amsthm}\n", false)] {
        let source = format!(
            "\\documentclass[11pt]{{article}}\n{preamble}\\newtheorem{{theorem}}{{Theorem}}\n\\begin{{document}}\nAlpha.\n\n\\begin{{theorem}}\nBravo.\n\\end{{theorem}}\nCharlie.\n\\begin{{theorem}}\nDelta.\n\\end{{theorem}}\nEcho.\n\\end{{document}}\n"
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let starts: Vec<_> = parsed
            .blocks
            .iter()
            .zip(&parsed.block_par_starts)
            .filter_map(|(block, start)| match block {
                Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                    Inline::Text { text, .. } if text.ends_with('.') && text.len() > 1 => Some((text.clone(), *start)),
                    _ => None,
                }),
                _ => None,
            })
            .collect();
        let of = |word: &str| starts.iter().find(|(text, _)| text == word).map(|(_, s)| *s).unwrap_or_else(|| panic!("no {word}: {starts:?}"));
        let bravo = of("Bravo.").trivlist.map(|t| t.vmode);
        let delta = of("Delta.").trivlist.map(|t| t.vmode);
        let echo = of("Echo.");
        if kernel {
            assert_eq!((bravo, delta), (Some(true), Some(false)), "{starts:?}");
            assert!(!echo.indent, "`\\@endpe`: {starts:?}");
        } else {
            assert_eq!((bravo, delta), (None, None), "amsthm assigns its own skips: {starts:?}");
            assert!(echo.indent, "amsthm's `\\@endpefalse`: {starts:?}");
        }
    }
}

#[test]
fn amsthm_keeps_its_own_head() {
    let runs = head("\\usepackage{amsthm}\n", "\\begin{theorem}[Lock cleanup]\nunlocks on drop.\n\\end{theorem}");
    assert_eq!(texts(&runs), ["Theorem 1", " ", "(", "Lock", "cleanup", ")", ".", "unlocks", "on", "drop."]);
    assert!(!runs[3].1, "amsthm's `\\thm@notefont` is medium: {runs:?}");
    assert!(runs[6].3, "amsthm's `\\thm@headpunct` is a `.` run on the `\\begin` span: {runs:?}");
}
