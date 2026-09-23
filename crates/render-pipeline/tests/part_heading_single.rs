//! `\part{Title}` renders its heading text exactly once.
//!
//! The compiler used to emit a drawable `Block::Heading` (level 0) for
//! `\part` while the adapter's pre-existing `BodyKind::Part` path drew the
//! same heading as a `Block::Part`, so every part title appeared twice.
//! `\chapter` avoids this: the compiler emits only a plain paragraph for
//! the title (which `strip_command_text` removes, its bytes lying inside
//! the `\chapter` command range) and the adapter does the actual drawing.
//! `\part` must follow that same split.
//!
//! Expected data: MacTeX 2026 (TeX Live 2026) pdflatex, two runs, of
//! `\documentclass{article}\begin{document}\part{Title}Hello.\end{document}`:
//! one page reading `Part I`, `Title`, `Hello.` — `Title` exactly once.
//! No TeX runs here.

mod common;

use common::*;

#[test]
fn part_title_renders_exactly_once() {
    if !lm_available() {
        eprintln!("SKIPPED: Latin Modern not available");
        return;
    }
    let r = render_one("\\documentclass{article}\\begin{document}\\part{Title}Hello.\\end{document}");
    let words = words_of(&r);
    let text: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
    eprintln!("pages: {}, words: {text:?}", r.v2.pages.len());
    // pdflatex: a single page.
    assert_eq!(r.v2.pages.len(), 1, "page count (pdflatex: 1)");
    // pdflatex: `Part I` above `Title`, each word once.
    for want in ["Part", "I", "Title", "Hello."] {
        let n = text.iter().filter(|w| **w == want).count();
        assert_eq!(n, 1, "word {want:?} occurs {n} times, pdflatex renders it once (words: {text:?})");
    }
}
