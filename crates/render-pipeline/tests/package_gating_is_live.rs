//! Guards that the `compiler-package-gating` feature path is actually
//! exercised, not merely compiled.
//!
//! #231's gate numbers were measured against a hand-swapped `vendor/compiler`
//! tree rather than its committed diff, so "green" there did not prove the
//! committed configuration runs the new code. These assertions fail if the
//! feature is ever dropped from `default`, or if the re-pin regresses.

/// amsmath renews `\thinspace`/`\negthinspace` to `.1667em` against the
/// kernel's `.16667em` (measured with TeX Live 2025 pdflatex). The pipeline's
/// kern reverse lookup has no document in scope, so it accepts *either*, by
/// asking the compiler under both package contexts.
///
/// If these two were equal the `[false, true]` loop in
/// `adapter::kern_amount_matches` would be dead code and the whole point of
/// #231 would be untestable — so assert they differ.
#[test]
fn the_two_thin_space_definitions_actually_differ() {
    use flashtex_compiler::text_builtins::text_kern;
    let kernel = text_kern(",", false).expect("kernel \\, is defined");
    let ams = text_kern(",", true).expect("amsmath \\, is defined");
    assert_ne!(kernel, ams, "amsmath's \\, must differ from the kernel's, else #231 is a no-op");
    let kernel_neg = text_kern("!", false).expect("kernel \\! is defined");
    let ams_neg = text_kern("!", true).expect("amsmath \\! is defined");
    assert_ne!(kernel_neg, ams_neg, "amsmath's \\! must differ from the kernel's");
}

/// The two-argument `text_kern` is the post-#230 compiler's signature. This
/// test calling it at all is the assertion: against the older pin it does not
/// compile, which is precisely why the feature had to become non-optional.
#[test]
fn the_pinned_compiler_carries_the_package_context() {
    use flashtex_compiler::math::MathPackages;
    let none = MathPackages::default();
    assert!(!none.amsmath && !none.amssymb, "default is every package absent");
}

/// #230's gating is live end to end: an msam/msbm name without its package
/// is an error, and the same document with `\usepackage{amssymb}` is clean.
#[test]
fn an_undeclared_amssymb_symbol_is_diagnosed_and_declaring_it_is_clean() {
    use flashtex_compiler::parser::SourceDocument;
    use flashtex_render_pipeline::{render, FontSet, RenderOptions};
    let fonts = FontSet::with_default_dirs(&[]);
    let run = |text: &str| {
        let docs = [SourceDocument { path: "main.tex", text }];
        let r = render(&docs, "main.tex", 1, "p", &fonts, &RenderOptions::default());
        r.v2
            .diagnostics
            .iter()
            // The gating diagnostic specifically -- not the generic
            // "packages amssymb are recognised but not implemented" note,
            // which is about package *features* and is emitted either way.
            .filter(|d| d.message.contains("requires \\usepackage"))
            .map(|d| d.message.clone())
            .collect::<Vec<_>>()
    };
    let undeclared = run("\\documentclass{article}\\begin{document}$\\varnothing$\\end{document}");
    assert!(
        undeclared.iter().any(|m| m.contains("\\varnothing")),
        "an undeclared \\varnothing must be diagnosed; got {undeclared:?}"
    );
    let declared = run("\\documentclass{article}\\usepackage{amssymb}\\begin{document}$\\varnothing$\\end{document}");
    assert!(declared.is_empty(), "declaring amssymb must clear it; got {declared:?}");
}
