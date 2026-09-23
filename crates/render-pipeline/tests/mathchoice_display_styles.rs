//! `\mathchoice{D}{T}{S}{SS}` selects one branch by the current math style:
//! display, text, script or scriptscript (TeX's mlist-to-hlist dispatch).
//!
//! Oracle (measured, not assumed): TeX Live 2026 pdfTeX, `article.cls` from
//! `kpsewhich article.cls` =
//! `/usr/local/texlive/2026/texmf-dist/tex/latex/base/article.cls`,
//! `pdflatex -interaction=nonstopmode mc.tex` on
//! ```tex
//! \documentclass{article}
//! \pagestyle{empty}
//! \begin{document}
//! Inline $\mathchoice{D}{T}{S}{Q}$ here.
//! \[
//! \mathchoice{D}{T}{S}{Q}
//! \]
//! \end{document}
//! ```
//! then `pdftotext -layout mc.pdf` gives `Inline T here.` for the `$...$`
//! formula (text branch) and a lone `D` for the `\[...\]` formula (display
//! branch).
//!
//! The pipeline does not implement `\mathchoice` yet: the compiler reports
//! `unknown_command` for it (`crates/compiler/src/math.rs`, issue #846 arm)
//! and the four braced groups then parse as ordinary groups, so every
//! branch's content is laid out in both styles. These tests pin the correct
//! dispatch and fail until the fix lands.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

fn doc(body: &str) -> String {
    format!("\\begin{{document}}{body}\\end{{document}}")
}

/// The source character behind every math glyph on page 1.
fn math_chars(body: &str) -> Vec<String> {
    let r = render_one(&doc(body));
    let mut out = Vec::new();
    for item in r.v2.pages[0].resident_items() {
        if let Item::GlyphRun(run) = item {
            if run.role != RunRole::Math {
                continue;
            }
            let mut chars = run
                .clusters
                .iter()
                .map(|c| run.text[c.text_start_byte as usize..c.text_end_byte as usize].to_string());
            for _ in &run.glyphs {
                out.push(chars.next().unwrap_or_default());
            }
        }
    }
    out
}

#[test]
fn inline_mathchoice_selects_the_text_branch() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let chars = math_chars("Inline $\\mathchoice{D}{T}{S}{Q}$ here.");
    eprintln!("inline math chars: {chars:?}");
    assert!(chars.iter().any(|c| c == "T"), "inline `\\mathchoice` must set the text branch `T`, got {chars:?}");
    for branch in ["D", "S", "Q"] {
        assert!(
            !chars.iter().any(|c| c == branch),
            "inline `\\mathchoice` must not set the `{branch}` branch, got {chars:?}"
        );
    }
}

#[test]
fn display_mathchoice_selects_the_display_branch() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    let chars = math_chars("Before.\n\n\\[\\mathchoice{D}{T}{S}{Q}\\]");
    eprintln!("display math chars: {chars:?}");
    assert!(chars.iter().any(|c| c == "D"), "display `\\mathchoice` must set the display branch `D`, got {chars:?}");
    for branch in ["T", "S", "Q"] {
        assert!(
            !chars.iter().any(|c| c == branch),
            "display `\\mathchoice` must not set the `{branch}` branch, got {chars:?}"
        );
    }
}

/// Script and scriptscript (added with the engine fix). pdflatex, same setup,
/// `$x^{\mathchoice{D}{T}{S}{Q}}$` sets `S` at 6.97 pt and
/// `$x^{y^{\mathchoice{D}{T}{S}{Q}}}$` sets `Q` at 4.98 pt (PyMuPDF span
/// sizes of the TeX Live 2026 PDF).
#[test]
fn script_and_scriptscript_mathchoice_select_their_branches() {
    if !lm_available() {
        eprintln!("skipped: Latin Modern not available");
        return;
    }
    for (body, want) in [("$x^{\\mathchoice{D}{T}{S}{Q}}$", "S"), ("$x^{y^{\\mathchoice{D}{T}{S}{Q}}}$", "Q")] {
        let chars = math_chars(body);
        let branches: Vec<_> = chars.iter().filter(|c| ["D", "T", "S", "Q"].contains(&c.as_str())).collect();
        assert_eq!(branches, [want], "{body}: {chars:?}");
    }
}
