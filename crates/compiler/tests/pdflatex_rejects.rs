//! LaTeX that pdflatex refuses must not compile silently here.
//!
//! The caret-context lane (issue #2, PR #350) needed pdflatex as its oracle
//! because this compiler was reported to accept every broken insertion it
//! produced. Re-measuring found two separate things:
//!
//! * The report's zero-diagnostic figure came from reading `diagnostics` at
//!   the top level of the `--v2` file, which is an envelope — they live under
//!   `payload`. The compiler already diagnosed 13 of the fixture's 15
//!   pdflatex-rejected cases.
//! * The remaining cases were real gaps, and so were nine more in the same
//!   families. This file pins them.
//!
//! Every expectation below was produced by compiling the same source with
//! pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025) under
//! `-interaction=nonstopmode`, and recording whether it answered with a `!`
//! error. Nothing here is an assumption about what LaTeX "should" do — three
//! of the cases in `ACCEPTED` were written expecting an error and turned out
//! to compile cleanly, which is exactly why they are pinned as controls.
//!
//! Note on severity: pdflatex reports every one of these as an error and then
//! recovers, producing a PDF anyway (none is fatal outside `-halt-on-error`).
//! That is the behaviour matched here — an error diagnostic *and* the page
//! still built — so `recovery` is asserted on every case.

use flashtex_compiler::diagnostics::{DiagnosticCode, Severity};
use flashtex_compiler::parser;

struct Case {
    /// What pdflatex answered, verbatim from its log.
    pdflatex: &'static str,
    body: &'static str,
    /// A distinctive fragment of the diagnostic this compiler must report.
    expect: &'static str,
}

/// Preamble-free bodies: `parse` wraps them in a document itself.
const REJECTED: &[Case] = &[
    // ---- nested math shifts -------------------------------------------
    Case {
        pdflatex: "Missing $ inserted.",
        body: r"Let $a + $x^2$ + b$ hold.",
        expect: "math script marker used outside math mode",
    },
    Case {
        pdflatex: "LaTeX Error: Bad math environment delimiter.",
        body: r"Let $a + \( x^2 \) + b$ hold.",
        expect: "unexpected math delimiter inside math mode",
    },
    Case {
        pdflatex: "LaTeX Error: Bad math environment delimiter.",
        body: r"\[ a + \[ x^2 \] + b \]",
        expect: "unexpected math delimiter inside math mode",
    },
    Case {
        pdflatex: "Display math should end with $$.",
        body: r"Prose \[ a + $x$ + b \] more.",
        expect: "unexpected math delimiter inside math mode",
    },
    // ---- unbalanced and stray delimiters ------------------------------
    Case {
        pdflatex: "Missing $ inserted.",
        body: r"Some prose $x^2 + y and then the paragraph ends.",
        expect: "inline math is missing its closing '$'",
    },
    Case {
        pdflatex: "Missing $ inserted.",
        body: r"Prose \( x^2 + y and then more prose.",
        expect: r"inline math is missing its closing '\)'",
    },
    Case {
        pdflatex: "LaTeX Error: Bad math environment delimiter.",
        body: r"Prose \) more prose.",
        expect: r"stray \) has no matching \(",
    },
    Case {
        pdflatex: "LaTeX Error: Bad math environment delimiter.",
        body: r"Prose \] more prose.",
        expect: r"stray \] has no matching \[",
    },
    // ---- display math in restricted horizontal mode -------------------
    Case {
        pdflatex: "Missing $ inserted.",
        body: "\\begin{tabular}{cc}\n a & \\[ x^2 \\] \\\\\n c & d\n\\end{tabular}",
        expect: "restricted horizontal mode",
    },
    Case {
        pdflatex: "Missing $ inserted.",
        body: "\\begin{tabular}{cc}\n a & \\begin{equation} x^2 \\end{equation} \\\\\n c & d\n\\end{tabular}",
        expect: "restricted horizontal mode",
    },
    Case {
        pdflatex: "Missing $ inserted.",
        body: "\\begin{figure}\n\\caption{A caption \\[ x^2 \\] here}\n\\end{figure}",
        expect: "restricted horizontal mode",
    },
    // ---- \text without amsmath ----------------------------------------
    Case {
        pdflatex: "Undefined control sequence.",
        body: r"Let $x + \text{y}$ hold.",
        expect: r"\text requires \usepackage{amsmath}",
    },
    // ---- & outside an alignment ---------------------------------------
    Case {
        pdflatex: "Misplaced alignment tab character &.",
        body: "Some prose a & b more prose.",
        expect: "misplaced alignment tab character &",
    },
    Case {
        pdflatex: "Misplaced alignment tab character &.",
        body: "AT&T here.",
        expect: "misplaced alignment tab character &",
    },
    Case {
        pdflatex: "Misplaced alignment tab character &.",
        body: "\\begin{tabular}{cc}\na & b\n\\end{tabular}\nthen c & d more.",
        expect: "misplaced alignment tab character &",
    },
    // ---- \\ with no line to end ----------------------------------------
    Case {
        pdflatex: "LaTeX Error: There's no line here to end.",
        body: "Prose.\n\n\\\\\nMore prose.",
        expect: "there's no line here to end",
    },
    Case {
        pdflatex: "LaTeX Error: There's no line here to end.",
        body: "\\begin{center}\n\\\\ line\n\\end{center}",
        expect: "there's no line here to end",
    },
];

/// Valid LaTeX in the same families. pdflatex compiles each of these without
/// a single error, so a diagnostic here would be a false positive — the
/// failure mode that makes a diagnostic worse than useless.
///
/// The first three were written expecting pdflatex to reject them and did
/// not: a `\section` title and a `\footnote` both set their argument as a
/// paragraph, so a display is legal in either, and `$$...$$` inside `$...$`
/// parses as alternating inline formulas rather than as nesting.
const ACCEPTED: &[&str] = &[
    r"\section{A title \[ x^2 \] here}",
    r"Prose\footnote{A note \[ x^2 \] here} more.",
    r"Let $a + $$x^2$$ + b$ hold.",
    r"Prose \( x^2 \) more.",
    r"Let $x^2 + y^2$ hold.",
    "Prose.\n\\[ x^2 + y^2 = z^2 \\]\nMore prose.",
    "\\begin{equation}\nx^2 + y^2 = z^2\n\\end{equation}",
    "\\begin{tabular}{cc}\na & $x^2$ \\\\\nc & d\n\\end{tabular}",
    r"It costs \$5 today.",
    r"Tom \& Jerry and AT\&T.",
    "% price is $5 here\nProse here.",
    "\\begin{verbatim}\ncost $5\n\\end{verbatim}\nProse.",
    "\\begin{center}\nline one \\\\\nline two\n\\end{center}",
    r"Some prose \\ more prose.",
    r"\noindent \\ text",
    "Prose.\n$$ x^2 + y^2 $$\nMore.",
    r"\section{A title $x^2$ here}",
];

fn errors(body: &str) -> Vec<flashtex_compiler::diagnostics::Diagnostic> {
    let source = format!("\\documentclass{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
    parser::parse(&source)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

#[test]
fn latex_pdflatex_rejects_is_diagnosed_here_too() {
    let mut missing = Vec::new();
    for case in REJECTED {
        let found = errors(case.body);
        if !found.iter().any(|d| d.message.contains(case.expect)) {
            missing.push(format!(
                "{:?}\n    pdflatex: ! {}\n    expected a diagnostic containing {:?}, got {:?}",
                case.body,
                case.pdflatex,
                case.expect,
                found.iter().map(|d| &d.message).collect::<Vec<_>>()
            ));
        }
    }
    assert!(missing.is_empty(), "silently accepted:\n{}", missing.join("\n"));
}

#[test]
fn every_such_diagnostic_names_its_recovery_and_carries_a_code() {
    // The engine's value is that it keeps rendering. An error that does not
    // say what was rendered instead leaves the author with nothing to act on,
    // and one without a code cannot be classified by a consumer — the
    // runtime-v1 contract says to classify by `code`, never by wording.
    for case in REJECTED {
        for d in errors(case.body).iter().filter(|d| d.message.contains(case.expect)) {
            assert!(d.recovery.is_some(), "{:?}: {} has no recovery", case.body, d.message);
            assert!(d.code.is_some(), "{:?}: {} has no code", case.body, d.message);
            assert!(d.span.is_some(), "{:?}: {} has no span", case.body, d.message);
        }
    }
}

#[test]
fn misplaced_syntax_is_a_syntax_error_not_an_unsupported_feature() {
    // `\text` is the one exception: the command is real LaTeX this compiler
    // does implement, and the fault is the missing `\usepackage`, so it is
    // classified like every other package-gated command (amssymb's symbols
    // take the same route).
    for case in REJECTED.iter().filter(|c| !c.expect.contains("amsmath")) {
        for d in errors(case.body).iter().filter(|d| d.message.contains(case.expect)) {
            assert_eq!(
                d.code,
                Some(DiagnosticCode::SyntaxError),
                "{:?}: {} should be a syntax_error",
                case.body,
                d.message
            );
        }
    }
}

#[test]
fn valid_latex_in_the_same_families_stays_clean() {
    let mut noisy = Vec::new();
    for body in ACCEPTED {
        let found = errors(body);
        if !found.is_empty() {
            noisy.push(format!(
                "{body:?}\n    pdflatex compiles this without an error; we reported {:?}",
                found.iter().map(|d| &d.message).collect::<Vec<_>>()
            ));
        }
    }
    assert!(noisy.is_empty(), "false positives:\n{}", noisy.join("\n"));
}

#[test]
fn a_diagnosed_document_still_produces_content() {
    // Reporting the error must not cost the preview. Every rejected case
    // still has to lay out: pdflatex itself recovers and emits a PDF for all
    // of them, and a preview that goes blank on a half-typed formula is worse
    // than one that shows the author's best-effort page.
    for case in REJECTED {
        let source = format!(
            "\\documentclass{{article}}\n\\begin{{document}}\n{}\n\\end{{document}}\n",
            case.body
        );
        let parsed = parser::parse(&source);
        assert!(
            !parsed.blocks.is_empty(),
            "{:?} recovered to nothing at all",
            case.body
        );
    }
}

/// `\(...\)` is ordinary LaTeX that this compiler used to drop on the floor:
/// the delimiters lexed as literal parentheses, so `\(x^2\)` set "(x2)" as
/// text with no diagnostic. Diagnosing a misplaced one is only half the fix.
#[test]
fn paren_delimiters_set_real_inline_math() {
    let parsed = parser::parse(
        "\\documentclass{article}\n\\begin{document}\nProse \\( x^2 \\) more.\n\\end{document}\n",
    );
    assert!(
        parsed.diagnostics.iter().all(|d| d.severity != Severity::Error),
        "{:?}",
        parsed.diagnostics
    );
    let rendered = format!("{:?}", parsed.blocks);
    assert!(rendered.contains("Math"), "\\(...\\) did not produce a math inline: {rendered}");
    assert!(
        !rendered.contains("\"(\""),
        "\\( was still set as a literal parenthesis: {rendered}"
    );
}
