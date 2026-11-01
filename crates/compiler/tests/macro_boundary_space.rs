//! GH-919: a user macro expanding to a text command (`\newcommand{\ul}[1]
//! {\uline{#1}}`) must glue to its neighbours exactly as the command
//! written directly does. `"\ul{a b}" x` set `a` +3.32 bp and `x` +6.64 bp
//! right of pdflatex through the render pipeline: one interword space was
//! invented on each side of the expansion at a `"`, `(` or `---` boundary.
//!
//! The compiler's side of that seam is pinned here: after expansion the
//! parse tree of the macro line is the direct line's parse tree with the
//! spans of the replacement tokens rewritten to the invocation (`\ul`), and
//! the compiler's own layout — which reads only `space_before`, never a
//! span — places every word at the same x. The render pipeline, which
//! re-reads the source bytes between spans for its interword gaps, had the
//! actual fault (`adapter::token_gap`); its pdflatex-measured pin is
//! `crates/render-pipeline/tests/macro_boundary_space.rs`.
use flashtex_compiler::incremental::compile_full_project;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::parser::{parse, Block, Inline, SourceDocument};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage[normalem]{{ulem}}\n\
         \\newcommand{{\\ul}}[1]{{\\uline{{#1}}}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

/// The paragraph's inlines with every span blanked, so the macro and direct
/// forms compare on structure and `space_before` alone.
fn shape(body: &str) -> Vec<String> {
    let source = doc(body);
    let parsed = parse(&source);
    let para = parsed
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("one paragraph");
    fn sig(inlines: &[Inline], out: &mut Vec<String>) {
        for inline in inlines {
            match inline {
                Inline::Text {
                    text, space_before, ..
                } => out.push(format!("T({text},{space_before})")),
                Inline::Underline(u) => {
                    out.push(format!("U({},{:?}[", u.space_before, u.geom));
                    sig(&u.content, out);
                    out.push("])".to_string());
                }
                other => out.push(format!("OTHER({:?})", std::mem::discriminant(other))),
            }
        }
    }
    let mut out = Vec::new();
    sig(&para, &mut out);
    out
}

/// Every text item's `(text, x)` on the first page of the compiler's own
/// layout, in order.
fn placed(body: &str) -> Vec<(String, f64)> {
    let source = doc(body);
    let out = compile_full_project(
        &[SourceDocument {
            path: "main.tex",
            text: &source,
        }],
        "main.tex",
        LayoutConstraints::default(),
    );
    out.pages[0]
        .items
        .iter()
        .filter(|item| item.rule.is_none())
        .map(|item| (item.text.clone(), item.x_pt))
        .collect()
}

const BOUNDARIES: [(&str, &str, &str); 3] = [
    (
        "quotes",
        "\"\\ul{a b}\" x \"\\ul{c d}\" y.",
        "\"\\uline{a b}\" x \"\\uline{c d}\" y.",
    ),
    (
        "parens",
        "(\\ul{a b}) x (\\ul{c d}) y.",
        "(\\uline{a b}) x (\\uline{c d}) y.",
    ),
    (
        "em dash",
        "p---\\ul{a b}---x q---\\ul{c d}---y.",
        "p---\\uline{a b}---x q---\\uline{c d}---y.",
    ),
];

#[test]
fn macro_and_direct_parse_to_the_same_shape() {
    for (name, via_macro, direct) in BOUNDARIES {
        assert_eq!(
            shape(via_macro),
            shape(direct),
            "{name}: the expanded parse differs from the direct one"
        );
    }
    // The quotes glue to the underline on both sides; only `x` and `y.`
    // follow a real blank. (The underline's first word carries the argument
    // group's leading position, which layout ignores after the rewind.)
    assert_eq!(
        shape(BOUNDARIES[0].1),
        [
            "T(\",true)",
            "U(false,UlemDescender[",
            "T(a,true)",
            "T(b,true)",
            "])",
            "T(\",false)",
            "T(x,true)",
            "T(\",true)",
            "U(false,UlemDescender[",
            "T(c,true)",
            "T(d,true)",
            "])",
            "T(\",false)",
            "T(y.,true)",
        ]
    );
}

#[test]
fn macro_and_direct_place_every_word_at_the_same_x() {
    for (name, via_macro, direct) in BOUNDARIES {
        let expanded = placed(via_macro);
        let plain = placed(direct);
        assert_eq!(
            expanded.len(),
            plain.len(),
            "{name}: item counts differ\n macro: {expanded:?}\n direct: {plain:?}"
        );
        for ((text, x), (want_text, want_x)) in expanded.iter().zip(&plain) {
            assert_eq!(text, want_text, "{name}: item order differs");
            assert!(
                (x - want_x).abs() < 0.005,
                "{name}: `{text}` at {x} via the macro, {want_x} directly"
            );
        }
    }
}
