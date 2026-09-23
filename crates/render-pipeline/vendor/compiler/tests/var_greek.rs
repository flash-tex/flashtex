//! amsmath's variant capitals `\varGamma`..`\varOmega` (amsmath.sty 385-395).
//!
//! Each is `\DeclareMathSymbol{...}{\mathord}{letters}{"0X}` (slots
//! 0x00-0x0A), so with amsmath loaded it sets one CMMI10 glyph and without it
//! pdflatex answers "Undefined control sequence"
//! (`pdflatex -interaction=nonstopmode` over a minimal article, TeX Live 2026:
//! clean with `\usepackage{amsmath,amssymb}`, `! Undefined control sequence.`
//! for `$\varPsi$` under plain article).

use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::math::{self, MathPackages, Nucleus};

/// (command, painted text) from `math_symbols.rs` (amsmath.sty:385-395).
const VAR_GREEK: &[(&str, &str)] = &[
    ("varGamma", "𝛤"),
    ("varDelta", "𝛥"),
    ("varTheta", "𝛩"),
    ("varLambda", "𝛬"),
    ("varXi", "𝛯"),
    ("varPi", "𝛱"),
    ("varSigma", "𝛴"),
    ("varUpsilon", "𝛶"),
    ("varPhi", "𝛷"),
    ("varPsi", "𝛹"),
    ("varOmega", "𝛺"),
];

fn compile(text: &str) -> flashtex_compiler::incremental::CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn with_amsmath(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage{{amsmath,amssymb}}\n\\begin{{document}}\n{body}\n\\end{{document}}"
    )
}

fn plain_article(body: &str) -> String {
    format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}")
}

#[test]
fn all_var_greek_accepted_with_amsmath() {
    for (command, _) in VAR_GREEK {
        let out = compile(&with_amsmath(&format!("$\\{command}$")));
        assert!(
            out.diagnostics.is_empty(),
            "\\{command} with amsmath diagnostics: {:?}",
            out.diagnostics
        );
    }
}

#[test]
fn var_greek_maps_to_declared_glyph() {
    let mut packages = MathPackages::KERNEL;
    packages.load_package("amsmath");
    for (command, expected) in VAR_GREEK {
        let mut diagnostics = Vec::new();
        let source = format!("\\{command}");
        let tokens = flashtex_compiler::lexer::tokenize(&source);
        let list = math::parse_tokens(&tokens, packages, &mut diagnostics);
        assert!(diagnostics.is_empty(), "\\{command}: {diagnostics:?}");
        assert_eq!(list.atoms.len(), 1, "\\{command}: {:?}", list.atoms);
        match &list.atoms[0].nucleus {
            Nucleus::Symbol(glyph) => assert_eq!(glyph, expected, "\\{command}"),
            other => panic!("\\{command}: expected Symbol nucleus, got {other:?}"),
        }
    }
}

#[test]
fn var_greek_rejected_without_amsmath() {
    for (command, _) in VAR_GREEK {
        let out = compile(&plain_article(&format!("$\\{command}$")));
        assert!(
            !out.diagnostics.is_empty(),
            "\\{command} without amsmath should produce a diagnostic"
        );
        let messages: Vec<&str> = out.diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert!(
            messages.iter().any(|m| m.contains("requires") && m.contains("amsmath")),
            "\\{command}: expected a missing-package diagnostic for amsmath, got: {messages:?}"
        );
    }
}

#[test]
fn task_repro_inline_and_display() {
    let out = compile(&with_amsmath("$\\varPsi(\\mathbf{r},t)$ \\[ \\varPsi \\]"));
    assert!(
        out.diagnostics.is_empty(),
        "task repro diagnostics: {:?}",
        out.diagnostics
    );
}
