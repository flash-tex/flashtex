//! A control symbol whose character is multi-byte UTF-8 (`\é`, `\€`, `\😀`),
//! or a backslash at the very end of the source, must not panic the source
//! scanners that step past a backslash.
//!
//! On main `70ca914bd`, `flashtex build` of
//! `\begin{multicols}{2}\é\end{multicols}` panicked at `multicol.rs:278`
//! ("start byte index 75 is not a char boundary; it is inside 'é'"), and a
//! float body holding `\é` panicked at `floats.rs:429` the same way. Both
//! scans stepped a fixed two bytes past the backslash. The multicol case was
//! found by #627's fuzzer (`multicol_scan_after_a_backslash_before_a_multibyte_character`).
//! pdflatex reads `\é` as an undefined control symbol and carries on, so
//! the render has to finish and keep the text around it.

mod common;

use flashtex_render_pipeline::{pdf, FontSet};

fn render_text(body: &str) -> String {
    let r = common::render_one(body);
    let fonts = FontSet::with_default_dirs(&[]);
    // The exact PDF route `flashtex build` writes must not fail on it either.
    let _ = pdf::write_pdf_exact(&r.v2, fonts.dirs(), None);
    common::words_of(&r).iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
}

const SYMBOLS: [&str; 3] = ["é", "€", "😀"];

#[test]
fn multicols_body_with_a_multibyte_control_symbol() {
    for sym in SYMBOLS {
        let text = render_text(&format!(
            "\\documentclass{{article}}\n\\usepackage{{multicol}}\n\\begin{{document}}\nbefore\n\\begin{{multicols}}{{2}}\\{sym} left \\columnbreak right\\end{{multicols}}\nafter\n\\end{{document}}\n"
        ));
        for w in ["before", "left", "right", "after"] {
            assert!(text.contains(w), "{sym}: {w} missing from {text:?}");
        }
    }
}

#[test]
fn multicols_mentioned_in_text_before_a_multibyte_control_symbol() {
    for sym in SYMBOLS {
        let text = render_text(&format!("\\documentclass{{article}}\n\\begin{{document}}\nmulticols \\{sym} after\n\\end{{document}}\n"));
        assert!(text.contains("after"), "{sym}: {text:?}");
    }
}

#[test]
fn multicols_source_ending_in_a_backslash() {
    // No `\end{document}`: the compiler reports the open environment; the
    // scan must not step past the end of the source.
    let text = render_text("\\documentclass{article}\n\\usepackage{multicol}\n\\begin{document}\nbefore\n\\begin{multicols}{2}x\\end{multicols}\\");
    assert!(text.contains("before"), "{text:?}");
}

#[test]
fn float_body_with_a_multibyte_control_symbol() {
    for sym in SYMBOLS {
        let text = render_text(&format!(
            "\\documentclass{{article}}\n\\begin{{document}}\nbefore\n\\begin{{figure}}\\centering\\{sym} inside\\caption{{Cap}}\\end{{figure}}\nafter\n\\end{{document}}\n"
        ));
        for w in ["before", "inside", "Cap", "after"] {
            assert!(text.contains(w), "{sym}: {w} missing from {text:?}");
        }
    }
}
