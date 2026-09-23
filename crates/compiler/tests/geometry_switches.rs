//! `\newgeometry{...}` / `\restoregeometry` (geometry.sty): each runs
//! `\clearpage` and then switches the page frame — the new frame for
//! `\newgeometry`, the preamble frame for `\restoregeometry`. The parser
//! owns the page break (a [`Block::PageBreak`]) plus a switch record on
//! `Parsed::geometry_switches`; the frame itself is the render pipeline's,
//! which reads it from the source at the reported switch, the way it
//! already does for `\pagestyle` and `\twocolumn`. Until the pipeline
//! applies it, each switch keeps the current frame and emits a typed
//! (`UnsupportedFeature`) limitation warning, so the CLI never goes
//! silent about the unmoved margins.
//!
//! Ground truth, pdflatex (TeX Live 2026):
//! `pdflatex -interaction=nonstopmode probe2.tex` in /tmp/geo2 on
//! `\documentclass{article}\usepackage{geometry}\begin{document}`
//! `text \newgeometry{margin=1cm} text \restoregeometry text`
//! `\end{document}` gives `Output written on probe2.pdf (3 pages, ...)`,
//! and `\typeout{\the\textwidth}` after `\newgeometry{margin=1cm}`
//! prints `NEW-TEXTWIDTH=557.38951pt` (back to `430.00462pt` after
//! `\restoregeometry`). Without `\usepackage{geometry}` pdflatex reports
//! two `! Undefined control sequence.` errors (one per command), writes a
//! single page, and typesets the leftover group: `text margin=1cm text
//! text`.
use flashtex_compiler::diagnostics::{Diagnostic, DiagnosticCode, Severity};
use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block};

/// The "margins not applied yet" limitation warnings in a diagnostic list.
fn limitation_warnings(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| {
            d.code == Some(DiagnosticCode::UnsupportedFeature)
                && d.message.contains("not applied yet")
        })
        .collect()
}

fn errors(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn page_words(out: &CompileOutput) -> Vec<Vec<String>> {
    out.pages
        .iter()
        .map(|p| p.items.iter().map(|i| i.text.clone()).collect())
        .collect()
}

const WITH_GEOMETRY: &str = "\\documentclass{article}\n\
     \\usepackage[margin=1in]{geometry}\n\
     \\begin{document}\n\
     text \\newgeometry{margin=1cm} text \\restoregeometry text\n\
     \\end{document}\n";

const WITHOUT_GEOMETRY: &str = "\\documentclass{article}\n\
     \\begin{document}\n\
     text \\newgeometry{margin=1cm} text \\restoregeometry text\n\
     \\end{document}\n";

#[test]
fn both_commands_are_accepted_and_break_the_page() {
    let parsed = parse(WITH_GEOMETRY);
    // Accepted: no errors. Each switch keeps the current frame and warns.
    assert!(errors(&parsed.diagnostics).is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(
        limitation_warnings(&parsed.diagnostics).len(),
        2,
        "{:?}",
        parsed.diagnostics
    );
    let kinds: Vec<&str> = parsed
        .blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph(_) => "para",
            Block::PageBreak => "break",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["para", "break", "para", "break", "para"]);
    assert_eq!(parsed.geometry_switches.len(), 2);
    assert!(!parsed.geometry_switches[0].restore);
    assert!(parsed.geometry_switches[1].restore);
    assert!(
        parsed.geometry_switches.iter().all(|s| !s.preamble),
        "{:?}",
        parsed.geometry_switches
    );
}

#[test]
fn page_count_matches_pdflatex() {
    // pdflatex: 3 pages, one `text` each. The warnings travel with the
    // compiled output; the pages keep the current frame (see
    // `unmoved_margins_warn_with_a_typed_limitation`).
    let out = compile(WITH_GEOMETRY);
    assert!(errors(&out.diagnostics).is_empty(), "{:?}", out.diagnostics);
    assert_eq!(
        limitation_warnings(&out.diagnostics).len(),
        2,
        "{:?}",
        out.diagnostics
    );
    assert_eq!(
        page_words(&out),
        vec![
            vec!["text".to_string()],
            vec!["text".to_string()],
            vec!["text".to_string()],
        ],
        "{:?}",
        out.pages
    );
}

#[test]
fn new_text_frame_matches_pdflatex() {
    let parsed = parse(WITH_GEOMETRY);
    assert!(errors(&parsed.diagnostics).is_empty(), "{:?}", parsed.diagnostics);
    let switch = &parsed.geometry_switches[0];
    assert_eq!(switch.options, "margin=1cm");
    // 1cm in this crate's PDF points (`length_pt`: 72/2.54).
    let one_cm_bp = 72.0 / 2.54;
    for side in [
        switch.left_pt,
        switch.right_pt,
        switch.top_pt,
        switch.bottom_pt,
    ] {
        assert!(
            side.is_some_and(|pt| (pt - one_cm_bp).abs() < 1e-9),
            "{switch:?}"
        );
    }
    // pdflatex measured NEW-TEXTWIDTH=557.38951pt on US Letter
    // (8.5in = 614.295pt); in PDF points that is 555.31bp, and this
    // layout's letter page is 612bp wide, so both agree at 612 - 2cm.
    let pdftex_textwidth_bp = 557.38951 * 72.0 / 72.27;
    let reported =
        612.0 - switch.left_pt.unwrap() - switch.right_pt.unwrap();
    assert!(
        (reported - pdftex_textwidth_bp).abs() <= 0.1,
        "reported {reported}bp vs pdflatex {pdftex_textwidth_bp}bp"
    );
    // `\restoregeometry` takes the preamble frame back: no options, no
    // parsed sides — the pipeline re-reads the preamble from the source.
    let restore = &parsed.geometry_switches[1];
    assert_eq!(restore.options, "");
    assert_eq!(
        [restore.left_pt, restore.right_pt, restore.top_pt, restore.bottom_pt],
        [None, None, None, None]
    );
}

#[test]
fn rejected_without_the_package_like_pdflatex() {
    // pdflatex: two `! Undefined control sequence.` errors, one page, and
    // the unconsumed group typeset as text (`text margin=1cm text text`).
    let parsed = parse(WITHOUT_GEOMETRY);
    let errors: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|d| d.code == Some(DiagnosticCode::UnknownCommand))
        .collect();
    assert_eq!(errors.len(), 2, "{:?}", parsed.diagnostics);
    assert_eq!(parsed.geometry_switches.len(), 0);
    assert!(
        !parsed.blocks.iter().any(|b| matches!(b, Block::PageBreak)),
        "{:?}",
        parsed.blocks
    );
    let out = compile(WITHOUT_GEOMETRY);
    assert_eq!(out.pages.len(), 1, "{:?}", page_words(&out));
    let words: Vec<String> = page_words(&out).into_iter().flatten().collect();
    assert!(
        words.iter().any(|w| w.contains("margin=1cm")),
        "{words:?}"
    );
}

#[test]
fn preamble_switch_is_recorded_without_a_break() {
    let src = "\\documentclass{article}\n\
         \\usepackage[margin=1in]{geometry}\n\
         \\newgeometry{margin=1cm}\n\
         \\begin{document}\nText.\n\\end{document}\n";
    let parsed = parse(src);
    assert!(errors(&parsed.diagnostics).is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(
        limitation_warnings(&parsed.diagnostics).len(),
        1,
        "{:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.geometry_switches.len(), 1);
    assert!(parsed.geometry_switches[0].preamble);
    assert!(
        !parsed.blocks.iter().any(|b| matches!(b, Block::PageBreak)),
        "a preamble \\clearpage has nothing to ship: {:?}",
        parsed.blocks
    );
}

#[test]
fn unmodelled_keys_warn_but_keep_the_switch_and_the_break() {
    let src = "\\documentclass{article}\n\
         \\usepackage[margin=1in]{geometry}\n\
         \\begin{document}\n\
         text \\newgeometry{margin=1cm,landscape} text\n\
         \\end{document}\n";
    let parsed = parse(src);
    assert_eq!(parsed.geometry_switches.len(), 1);
    assert_eq!(
        parsed
            .blocks
            .iter()
            .filter(|b| matches!(b, Block::PageBreak))
            .count(),
        1,
        "{:?}",
        parsed.blocks
    );
    assert!(
        parsed.diagnostics.iter().any(|d| d.message.contains("landscape")),
        "{:?}",
        parsed.diagnostics
    );
    // The unmodelled-key warning is separate from the limitation warning.
    assert_eq!(
        limitation_warnings(&parsed.diagnostics).len(),
        1,
        "{:?}",
        parsed.diagnostics
    );
}

#[test]
fn unmoved_margins_warn_with_a_typed_limitation() {
    // Minimum fix for the review finding: render-pipeline ignores the
    // switch, so the page keeps the current frame. Each switch must say
    // so with a typed warning instead of going silent.
    let parsed = parse(WITH_GEOMETRY);
    let warnings = limitation_warnings(&parsed.diagnostics);
    assert_eq!(warnings.len(), 2, "{:?}", parsed.diagnostics);
    assert!(
        warnings[0].message.contains("\\newgeometry")
            && warnings[0].message.contains("margins are not applied yet"),
        "{:?}",
        warnings[0]
    );
    assert!(
        warnings[1].message.contains("\\restoregeometry")
            && warnings[1].message.contains("not applied yet"),
        "{:?}",
        warnings[1]
    );
    for warning in &warnings {
        assert_eq!(warning.severity, Severity::Warning, "{warning:?}");
        assert_eq!(
            warning.code,
            Some(DiagnosticCode::UnsupportedFeature),
            "{warning:?}"
        );
        assert!(warning.span.is_some(), "{warning:?}");
    }
    // Render follow-up row: the compiled pages still keep the current
    // frame — applying it is render-pipeline work. The warnings above are
    // the honest signal until then.
    let out = compile(WITH_GEOMETRY);
    assert_eq!(out.pages.len(), 3, "{:?}", page_words(&out));
    assert_eq!(
        limitation_warnings(&out.diagnostics).len(),
        2,
        "{:?}",
        out.diagnostics
    );
}
