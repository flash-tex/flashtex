//! `\paragraph` and `\subparagraph`: `\@startsection` levels 4 and 5.
//!
//! ```tex
//! \newcommand\paragraph{\@startsection{paragraph}{4}{\z@}%
//!     {3.25ex \@plus1ex \@minus.2ex}{-1em}{\normalfont\normalsize\bfseries}}
//! \newcommand\subparagraph{\@startsection{subparagraph}{5}{\parindent}%
//!     {3.25ex \@plus1ex \@minus.2ex}{-1em}{\normalfont\normalsize\bfseries}}
//! ```
//!
//! `#5` is not positive, so `\@sect` stores the heading in `\@svsechd`
//! instead of setting it and `\@xsect` installs an `\everypar` that, on the
//! *next* paragraph, drops that paragraph's `\parindent` box
//! (`{\setbox\z@\lastbox}`), unboxes the heading and adds `\hskip -#5` =
//! `\hskip 1em`. Above it stands only `\addvspace{#4}` = `3.25ex`.
//!
//! **Requires a `vendor/compiler` re-pinned past the compiler-side PR**
//! (`\paragraph`/`\subparagraph` as `Block::Heading` levels 4 and 5); the
//! pipeline arm is inert against an older pin, which never emits them. Hence
//! the `run-in-headings` feature — see `Cargo.toml`.
//!
//! Every expected coordinate is the pdflatex geometry of
//! `fixtures/divergence-probes/min-paragraph` (TeX Live 2025,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) read with
//! `tools/visual-oracle/pdftext.py`; the oracle is not run here. Coordinates
//! are bp from the page's top-left corner.
#![cfg(feature = "run-in-headings")]

mod common;

use common::*;

const TOL: f64 = 0.05;

const SRC: &str = "\\documentclass{article}\n\\usepackage[T1]{fontenc}\n\\usepackage[margin=1in]{geometry}\n\
\\begin{document}\n\n\\section{A section}\n\n\
Ordinary body text before the run-in heading, long enough that the line above\n\
the heading is set the same way on both sides.\n\n\
\\paragraph{A run-in heading.} The paragraph heading is set in bold, run into\n\
the first line of its paragraph, with a fixed skip above it and a word space\n\
after it. Real solutions, proofs and notes use it constantly.\n\n\
\\paragraph{A second one.} And the text that follows it.\n\\end{document}\n";

fn word<'a>(words: &'a [Word], text: &str) -> &'a Word {
    words.iter().find(|w| w.text == text).unwrap_or_else(|| panic!("no word {text:?} in {words:?}"))
}

/// The leftmost word of the line `text` stands on.
fn line_start<'a>(words: &'a [Word], text: &str) -> &'a Word {
    let baseline = word(words, text).baseline;
    words
        .iter()
        .filter(|w| (w.baseline - baseline).abs() < 0.05)
        .min_by(|a, b| a.x.partial_cmp(&b.x).expect("finite"))
        .expect("the line has a word")
}

#[test]
fn a_paragraph_heading_opens_the_next_paragraphs_first_line() {
    if !lm_available() {
        return;
    }
    let r = render_one(SRC);
    assert!(r.v2.diagnostics.iter().all(|d| d.severity != flashtex_render_pipeline::display::Severity::Error), "{:?}", r.v2.diagnostics);
    let words = words_of(&r);

    // `\paragraph{A run-in heading.}`: "heading." is on its own line only
    // there ("heading," and "heading" in the body carry other punctuation).
    let head = line_start(&words, "heading.");
    // `{\setbox\z@\lastbox}`: no `\parindent` before the heading.
    assert!((head.x - 72.000).abs() < TOL, "{head:?}");
    // `\addvspace{3.25ex}` above it: the body line before ends at 115.736.
    assert!((head.baseline - 141.629).abs() < TOL, "{head:?}");
    // `\hskip 1em` of the body font after the heading box.
    assert!((word(&words, "The").x - 171.438).abs() < TOL, "{:?}", word(&words, "The"));
    assert_eq!(word(&words, "The").baseline, head.baseline, "the paragraph runs into the heading's line");
}

#[test]
fn a_run_in_heading_is_set_in_the_bold_body_face() {
    if !lm_available() {
        return;
    }
    let fonts = flashtex_render_pipeline::FontSet::with_default_dirs(&[]);
    let r = render_one_with(SRC, &fonts);
    let mut head = None;
    let mut body = None;
    for page in &r.v2.pages {
        for it in &page.items {
            if let flashtex_render_pipeline::display::Item::GlyphRun(run) = it {
                let name = fonts.by_font_id(&run.font_id).map(|f| f.name.clone()).unwrap_or_default();
                let size = run.font_size.to_bp();
                if run.text == "heading." {
                    head = Some((name, size));
                } else if run.text == "paragraph" {
                    body = Some((name, size));
                }
            }
        }
    }
    let (head, head_size) = head.expect("the heading is set");
    let (body, body_size) = body.expect("the paragraph is set");
    assert!(head.to_lowercase().contains("bold") || head.contains("bx"), "`\\bfseries`: {head}");
    assert!(!(body.to_lowercase().contains("bold") || body.contains("bx")), "the body is not: {body}");
    // `\normalsize`: the heading is the body size, unlike levels 1 and 2.
    assert!((head_size - body_size).abs() < 1e-6, "{head_size} vs {body_size}");
}

#[test]
fn a_subparagraph_starts_one_parindent_in() {
    if !lm_available() {
        return;
    }
    let src = SRC.replace("\\paragraph{A second one.}", "\\subparagraph{A second one.}");
    let words = words_of(&render_one(&src));
    // `\@startsection`'s `#3` is `\parindent` for level 5 (15 pt at a 10 pt
    // body = 14.944 bp), `\z@` for level 4.
    let second = line_start(&words, "second");
    assert!((second.x - 86.944).abs() < TOL, "{second:?}");
}

#[test]
fn a_run_in_heading_with_nothing_after_it_is_still_set() {
    if !lm_available() {
        return;
    }
    let src = SRC.replace("\\paragraph{A second one.} And the text that follows it.", "\\paragraph{Last of all.}");
    let words = words_of(&render_one(&src));
    let last = word(&words, "Last");
    assert!((last.x - 72.000).abs() < TOL, "{last:?}");
    assert!(words.iter().any(|w| w.text == "all."), "{words:?}");
}
