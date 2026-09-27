//! Tests for the cancel package: `\cancel`, `\bcancel`, `\xcancel`, `\cancelto`.

use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::LayoutConstraints;
use flashtex_compiler::math::{self, Frame, MathPackages, Nucleus};

fn compile(text: &str) -> flashtex_compiler::incremental::CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn preamble(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage{{cancel}}\n\\begin{{document}}\n{body}\n\\end{{document}}"
    )
}

#[test]
fn cancel_parses_without_error() {
    let out = compile(&preamble("$\\cancel{x}$"));
    assert!(
        out.diagnostics.is_empty(),
        "\\cancel{{x}} diagnostics: {:?}",
        out.diagnostics
    );
}

#[test]
fn bcancel_parses_without_error() {
    let out = compile(&preamble("$\\bcancel{x+y}$"));
    assert!(
        out.diagnostics.is_empty(),
        "\\bcancel{{x+y}} diagnostics: {:?}",
        out.diagnostics
    );
}

#[test]
fn xcancel_parses_without_error() {
    let out = compile(&preamble("$\\xcancel{\\frac{a}{b}}$"));
    assert!(
        out.diagnostics.is_empty(),
        "\\xcancel{{\\frac{{a}}{{b}}}} diagnostics: {:?}",
        out.diagnostics
    );
}

#[test]
fn nested_cancel() {
    let out = compile(&preamble("$\\cancel{\\bcancel{x}}$"));
    assert!(
        out.diagnostics.is_empty(),
        "nested cancel diagnostics: {:?}",
        out.diagnostics
    );
}

#[test]
fn cancel_without_package_reports_missing_package() {
    let out = compile("\\documentclass{article}\n\\begin{document}\n$\\cancel{x}$\n\\end{document}");
    assert!(
        !out.diagnostics.is_empty(),
        "\\cancel without \\usepackage{{cancel}} should produce a diagnostic"
    );
    let messages: Vec<&str> = out.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages.iter().any(|m| m.contains("requires") && m.contains("cancel")),
        "expected a missing-package diagnostic for cancel, got: {messages:?}"
    );
}

#[test]
fn cancelto_parses_without_error() {
    let out = compile(&preamble("$\\cancelto{0}{x}$"));
    assert!(
        out.diagnostics.is_empty(),
        "\\cancelto{{0}}{{x}} diagnostics: {:?}",
        out.diagnostics
    );
}

#[test]
fn cancelto_without_package_reports_missing_package() {
    let out = compile("\\documentclass{article}\n\\begin{document}\n$\\cancelto{0}{x}$\n\\end{document}");
    assert!(
        !out.diagnostics.is_empty(),
        "\\cancelto without \\usepackage{{cancel}} should produce a diagnostic"
    );
    let messages: Vec<&str> = out.diagnostics.iter().map(|d| d.message.as_str()).collect();
    assert!(
        messages.iter().any(|m| m.contains("requires") && m.contains("cancel")),
        "expected a missing-package diagnostic for cancelto, got: {messages:?}"
    );
}

#[test]
fn cancelto_produces_cancel_frame_with_value_superscript() {
    let mut packages = MathPackages::KERNEL;
    packages.load_package("cancel");

    let mut diagnostics = Vec::new();
    let source = r"\cancelto{0}{x}";
    let tokens = flashtex_compiler::lexer::tokenize(source);
    let list = math::parse_tokens(&tokens, packages, &mut diagnostics);
    assert!(diagnostics.is_empty(), "\\cancelto: {diagnostics:?}");
    assert_eq!(list.atoms.len(), 1, "\\cancelto: {:?}", list.atoms);
    let atom = &list.atoms[0];
    // `\cancelto` is a closed box (`\@cancelto` is an `\hbox`): the outer
    // atom is a one-atom group with free script slots, and the cancelled
    // atom with its value label sits inside.
    assert!(
        atom.superscript.is_none() && atom.subscript.is_none(),
        "\\cancelto: outer atom must keep free script slots, got {atom:?}"
    );
    let inner = match &atom.nucleus {
        Nucleus::Group(body) => {
            assert_eq!(body.atoms.len(), 1, "\\cancelto box: {body:?}");
            &body.atoms[0]
        }
        other => panic!("\\cancelto: expected closed Group nucleus, got {other:?}"),
    };
    match &inner.nucleus {
        Nucleus::Framed { body, frame } => {
            assert_eq!(*frame, Frame::Cancel, "\\cancelto frame");
            assert_eq!(body.atoms.len(), 1, "\\cancelto body: {:?}", body.atoms);
            assert_eq!(
                body.atoms[0].nucleus,
                Nucleus::Symbol("x".into()),
                "\\cancelto body"
            );
        }
        other => panic!("\\cancelto: expected Framed nucleus inside, got {other:?}"),
    }
    let value = inner
        .superscript
        .as_ref()
        .expect("\\cancelto value annotation attached as superscript");
    assert_eq!(value.atoms.len(), 1, "\\cancelto value: {value:?}");
    assert_eq!(
        value.atoms[0].nucleus,
        Nucleus::Symbol("0".into()),
        "\\cancelto value"
    );
}

#[test]
fn cancelto_followed_by_superscript_keeps_label_and_takes_own_script() {
    // PR #982 review finding 1: pdflatex sets the cancelled `x` with its
    // `0` label and then `^2` on the finished box (`\@cancelto` is an
    // `\hbox`, `cancel.sty`), so no "duplicate script" diagnostic may fire,
    // the `0` label must survive, and `2` must land on the whole atom.
    let out = compile(&preamble("$\\cancelto{0}{x}^2$"));
    assert!(
        out.diagnostics.is_empty(),
        "\\cancelto{{0}}{{x}}^2 diagnostics: {:?}",
        out.diagnostics
    );

    let mut packages = MathPackages::KERNEL;
    packages.load_package("cancel");
    let mut diagnostics = Vec::new();
    let source = r"\cancelto{0}{x}^2";
    let tokens = flashtex_compiler::lexer::tokenize(source);
    let list = math::parse_tokens(&tokens, packages, &mut diagnostics);
    assert!(diagnostics.is_empty(), "\\cancelto^2: {diagnostics:?}");
    assert_eq!(list.atoms.len(), 1, "\\cancelto^2: {:?}", list.atoms);
    let atom = &list.atoms[0];
    // The outer `2` rides on the finished (closed) atom.
    let outer = atom
        .superscript
        .as_ref()
        .expect("\\cancelto^2: expected superscript 2 on the result");
    assert_eq!(outer.atoms.len(), 1, "\\cancelto^2 superscript: {outer:?}");
    assert_eq!(
        outer.atoms[0].nucleus,
        Nucleus::Symbol("2".into()),
        "\\cancelto^2 superscript"
    );
    // The `0` label survives inside, on the cancelled atom.
    let inner = match &atom.nucleus {
        Nucleus::Group(body) => {
            assert_eq!(body.atoms.len(), 1, "\\cancelto^2 box: {body:?}");
            &body.atoms[0]
        }
        other => panic!("\\cancelto^2: expected closed Group nucleus, got {other:?}"),
    };
    match &inner.nucleus {
        Nucleus::Framed { body, frame } => {
            assert_eq!(*frame, Frame::Cancel, "\\cancelto^2 frame");
            assert_eq!(body.atoms.len(), 1, "\\cancelto^2 body: {:?}", body.atoms);
            assert_eq!(
                body.atoms[0].nucleus,
                Nucleus::Symbol("x".into()),
                "\\cancelto^2 body"
            );
        }
        other => panic!("\\cancelto^2: expected Framed nucleus inside, got {other:?}"),
    }
    let value = inner
        .superscript
        .as_ref()
        .expect("\\cancelto^2: expected label 0 kept on the cancelled atom");
    assert_eq!(value.atoms.len(), 1, "\\cancelto^2 label: {value:?}");
    assert_eq!(
        value.atoms[0].nucleus,
        Nucleus::Symbol("0".into()),
        "\\cancelto^2 label"
    );
}

#[test]
fn cancel_produces_framed_nucleus_with_correct_frame() {
    let mut packages = MathPackages::KERNEL;
    packages.load_package("cancel");

    for (command, expected_frame) in [
        ("cancel", Frame::Cancel),
        ("bcancel", Frame::BCancel),
        ("xcancel", Frame::XCancel),
    ] {
        let mut diagnostics = Vec::new();
        let source = format!(r"\{command}{{x}}");
        let tokens = flashtex_compiler::lexer::tokenize(&source);
        let list = math::parse_tokens(&tokens, packages, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{command}: {diagnostics:?}");
        assert_eq!(list.atoms.len(), 1, "{command}: {:?}", list.atoms);
        match &list.atoms[0].nucleus {
            Nucleus::Framed { body, frame } => {
                assert_eq!(*frame, expected_frame, "{command}");
                assert_eq!(body.atoms.len(), 1, "{command}: body {:?}", body.atoms);
                assert_eq!(
                    body.atoms[0].nucleus,
                    Nucleus::Symbol("x".into()),
                    "{command}"
                );
            }
            other => panic!("{command}: expected Framed nucleus, got {other:?}"),
        }
    }
}
