//! T1 guillemets and base quotes.
//!
//! Under `\usepackage[T1]{fontenc}`, `\guillemotleft`, `\guillemotright`,
//! `\guilsinglleft`, `\guilsinglright`, `\quotedblbase` and
//! `\quotesinglbase` map to the T1/ec font's dedicated glyph slots
//! (t1enc.def slots 19, 20, 14, 15, 18, 13). Before this fix the compiler
//! reported them as not supported and set nothing. Under OT1 pdflatex
//! reports them unavailable in that encoding and typesets nothing, which
//! the shared `text_symbol` path already does.
//!
//! Oracle reproduced locally: `pdflatex -interaction=nonstopmode` on the
//! source below (article 10pt) gives `B` x0=179.148, `C` x0=206.943 and
//! `D` x0=235.436 (PyMuPDF). The compiler's own Core 14 layout sets the
//! same characters from its own metrics, so these tests pin silence plus
//! identity with the same characters typed directly.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline};

fn t1_document(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage[T1]{{fontenc}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn messages(o: &CompileOutput) -> Vec<String> {
    o.diagnostics.iter().map(|d| d.message.clone()).collect()
}

fn runs(o: &CompileOutput) -> Vec<(String, i64)> {
    o.pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| (i.text.clone(), (i.x_pt * 100.0).round() as i64))
        .collect()
}

fn paragraph_text(source: &str) -> String {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut out = String::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { text, .. } = inline {
                    out.push_str(text);
                }
            }
        }
    }
    out
}

#[test]
fn t1_quotes_typeset_their_characters_silently() {
    let body = "A \\guillemotleft x\\guillemotright{} B \\guilsinglleft y\\guilsinglright{} C \\quotedblbase z \\quotesinglbase{} D next.";
    let out = compile(&t1_document(body));
    assert!(out.diagnostics.is_empty(), "{:?}", out.diagnostics);
    let text = paragraph_text(&t1_document(body));
    assert!(text.contains('«'), "{text:?}");
    assert!(text.contains('»'), "{text:?}");
    assert!(text.contains('‹'), "{text:?}");
    assert!(text.contains('›'), "{text:?}");
    assert!(text.contains('„'), "{text:?}");
    assert!(text.contains('‚'), "{text:?}");
}

#[test]
fn t1_quotes_lay_out_like_the_typed_characters() {
    // Commands arrive as one run each, so the run split differs from the
    // typed text; positions and the concatenated line are identical.
    let commands = "A \\guillemotleft x\\guillemotright{} B \\guilsinglleft y\\guilsinglright{} C \\quotedblbase z \\quotesinglbase{} D next.";
    let typed = "A «x» B ‹y› C „z ‚ D next.";
    let (a, b) = (compile(&t1_document(commands)), compile(&t1_document(typed)));
    assert_eq!(messages(&a), messages(&b), "{commands:?} vs {typed:?}");
    let (ra, rb) = (runs(&a), runs(&b));
    assert_eq!(
        ra.iter().map(|(t, _)| t.as_str()).collect::<String>(),
        rb.iter().map(|(t, _)| t.as_str()).collect::<String>(),
        "{commands:?} vs {typed:?}"
    );
    // The oracle-named words sit at the same x either way.
    let at = |runs: &[(String, i64)], word: &str| {
        runs.iter()
            .find(|(t, _)| t.starts_with(word))
            .map(|(_, x)| *x)
            .unwrap_or_else(|| panic!("{word} missing in {runs:?}"))
    };
    for word in ["B", "C", "D", "next."] {
        assert_eq!(at(&ra, word), at(&rb, word), "{word}: {commands:?} vs {typed:?}");
    }
}

#[test]
fn ot1_quotes_are_unavailable_like_pdflatex() {
    // pdflatex: `! LaTeX Error: Command \guillemotleft unavailable in
    // encoding OT1.` and nothing typeset. The compiler must say the same,
    // not "not supported".
    let body = "A \\guillemotleft x\\guillemotright{} B next.";
    let source =
        format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
    let parsed = parse(&source);
    let errors: Vec<&str> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        errors.iter().any(|m| m.contains("unavailable in encoding OT1")),
        "{errors:?}"
    );
    assert!(
        errors.iter().all(|m| !m.contains("not supported")),
        "{errors:?}"
    );
}
