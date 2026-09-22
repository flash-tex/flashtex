//! scrlayer-scrpage basic layer (slice 1): `\pagestyle{scrheadings}` plus
//! `\ihead`/`\chead`/`\ohead` and `\ifoot`/`\cfoot`/`\ofoot` as a thin
//! adapter over fancyhdr's six running-head slots.
//!
//! Oracle: real scrlayer-scrpage, where on odd (one-sided: every) pages
//! inner is left and outer is right, so `\ihead` fills the same field as
//! fancyhdr's `\lhead` and `\ohead` the same as `\rhead`. This layout is
//! always one-sided (see `fancy_position_slots`), so the adapter maps
//! inner to slot 0, centre to slot 1, outer to slot 2, and pages shipping
//! under `scrheadings` stamp the same chrome as `fancy` pages.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::{LayoutConstraints, TextItem};
use flashtex_compiler::parser::parse;

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

/// One page-0 item reduced to what layout means: text, slot geometry and
/// rules. Equal vectors mean equal rendered layouts.
fn layout_key(item: &TextItem) -> (String, u64, u64, bool) {
    (
        item.text.clone(),
        item.x_pt.to_bits(),
        item.baseline_y_pt.to_bits(),
        item.rule.is_some(),
    )
}

fn page_keys(out: &CompileOutput) -> Vec<(String, u64, u64, bool)> {
    out.pages
        .first()
        .map(|page| page.items.iter().map(layout_key).collect())
        .unwrap_or_default()
}

fn words(out: &CompileOutput) -> Vec<String> {
    out.pages
        .first()
        .map(|page| {
            page.items
                .iter()
                .filter(|item| !item.text.is_empty())
                .map(|item| item.text.clone())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn scrheadings_header_matches_fancyhdr_header_layout() {
    // The acceptance case: inner/centre/outer under `scrheadings` lays out
    // exactly like left/centre/right under `fancy`.
    let scr = "\\documentclass{article}\n\
         \\usepackage{scrlayer-scrpage}\n\
         \\pagestyle{scrheadings}\n\
         \\ihead{Left Field}\n\
         \\chead{Centre Field}\n\
         \\ohead{Right Field}\n\
         \\begin{document}\n\
         Body text on page one.\n\
         \\end{document}\n";
    let fancy = "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\lhead{Left Field}\n\
         \\chead{Centre Field}\n\
         \\rhead{Right Field}\n\
         \\begin{document}\n\
         Body text on page one.\n\
         \\end{document}\n";
    let scr_out = compile(scr);
    assert!(
        scr_out.diagnostics.is_empty(),
        "scrheadings setup must be silent: {:?}",
        scr_out.diagnostics
    );
    let fancy_out = compile(fancy);
    assert!(
        fancy_out.diagnostics.is_empty(),
        "fancyhdr setup must be silent: {:?}",
        fancy_out.diagnostics
    );
    assert_eq!(scr_out.pages.len(), 1);
    assert_eq!(fancy_out.pages.len(), 1);
    // Content-stream order: header fields, then the body.
    assert_eq!(
        words(&scr_out),
        [
            "Left", "Field", "Centre", "Field", "Right", "Field", "Body", "text", "on", "page",
            "one."
        ],
    );
    // Same layout down to slot coordinates: inner sat in the left slot,
    // outer in the right slot, centre in the centre slot.
    assert_eq!(
        page_keys(&scr_out),
        page_keys(&fancy_out),
        "scrheadings \\ihead/\\chead/\\ohead must lay out like fancyhdr \\lhead/\\chead/\\rhead"
    );
}

#[test]
fn scrheadings_footer_matches_fancyhdr_footer_layout() {
    let scr = "\\documentclass{article}\n\
         \\usepackage{scrlayer-scrpage}\n\
         \\pagestyle{scrheadings}\n\
         \\ifoot{Left}\n\
         \\cfoot{Centre}\n\
         \\ofoot{Right}\n\
         \\begin{document}\n\
         Body.\n\
         \\end{document}\n";
    let fancy = "\\documentclass{article}\n\
         \\usepackage{fancyhdr}\n\
         \\pagestyle{fancy}\n\
         \\lfoot{Left}\n\
         \\cfoot{Centre}\n\
         \\rfoot{Right}\n\
         \\begin{document}\n\
         Body.\n\
         \\end{document}\n";
    let scr_out = compile(scr);
    assert!(
        scr_out.diagnostics.is_empty(),
        "scrheadings footer setup must be silent: {:?}",
        scr_out.diagnostics
    );
    let fancy_out = compile(fancy);
    assert!(
        fancy_out.diagnostics.is_empty(),
        "fancyhdr footer setup must be silent: {:?}",
        fancy_out.diagnostics
    );
    assert_eq!(words(&scr_out), ["Body.", "Left", "Centre", "Right"]);
    assert_eq!(
        page_keys(&scr_out),
        page_keys(&fancy_out),
        "scrheadings \\ifoot/\\cfoot/\\ofoot must lay out like fancyhdr \\lfoot/\\cfoot/\\rfoot"
    );
}

#[test]
fn commands_need_their_own_package() {
    // Bare load is silent, exactly like fancyhdr's.
    let parsed = parse(
        "\\documentclass{article}\n\
         \\usepackage{scrlayer-scrpage}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);

    // Inner/outer without the package name what is missing, as fancyhdr's
    // commands do without fancyhdr.
    let parsed = parse(
        "\\documentclass{article}\n\
         \\ihead{Lost}\n\
         \\begin{document}\nText.\n\\end{document}\n",
    );
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\ihead") && d.message.contains("scrlayer-scrpage")),
        "\\ihead without the package must name it: {:?}",
        parsed.diagnostics
    );

    // fancyhdr alone does not provide inner/outer (real LaTeX: undefined
    // control sequence), and scrlayer-scrpage alone does not provide
    // left/right.
    for (preamble, command) in [
        ("\\usepackage{fancyhdr}", "\\ihead"),
        ("\\usepackage{scrlayer-scrpage}", "\\lhead"),
    ] {
        let parsed = parse(&format!(
            "\\documentclass{{article}}\n{preamble}\n{command}{{Lost}}\n\
             \\begin{{document}}\nText.\n\\end{{document}}\n"
        ));
        assert!(
            !parsed.diagnostics.is_empty(),
            "{command} under {preamble} must be diagnosed: {:?}",
            parsed.diagnostics
        );
    }

    // The shared centre pair works under either package alone.
    for preamble in ["\\usepackage{fancyhdr}", "\\usepackage{scrlayer-scrpage}"] {
        let parsed = parse(&format!(
            "\\documentclass{{article}}\n{preamble}\n\\chead{{C}}\\cfoot{{F}}\n\
             \\begin{{document}}\nText.\n\\end{{document}}\n"
        ));
        assert!(
            parsed.diagnostics.is_empty(),
            "\\chead/\\cfoot under {preamble}: {:?}",
            parsed.diagnostics
        );
    }
}
