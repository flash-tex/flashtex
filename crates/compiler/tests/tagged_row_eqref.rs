//! GitHub issue #823 (review of PR #821): `\tag` rows in `align`/`gather`
//! and `\[ ... \]` displays.
//!
//! Expected values below are pdflatex's (TeX Live 2026), verified with
//! `/Library/TeX/texbin/pdflatex` against the `/tmp/tagprobe` probes:
//! - probe1: align rows `x=1 \tag{A}` / `y=2`, then `z=3` print
//!   `(A)`, `(1)`, `(2)`; `\eqref` reads `(A)`, `(1)`, `(2)`.
//! - probe3: a tagged `equation` also consumes no number (`a=1 \tag{Z}`
//!   then `b=2` prints `(Z)`, `(1)`; tagged `gather` rows behave the same).
//! - probe2: `\[x=1 \tag{B}\label{e:d}\]` prints `(B)` and `\eqref`
//!   reads `(B)`; `\tag{$\alpha$}` prints `(α)` and `\eqref` reads `(α)`.

use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};
use flashtex_compiler::parser::{parse, Block, Inline};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

/// Every `\eqref{key}` resolution in source order.
fn eqref_values(source: &str, output: &CompileOutput) -> Vec<String> {
    let mut values = Vec::new();
    let mut cursor = 0;
    while let Some(found) = source[cursor..].find(r"\eqref{") {
        let start = cursor + found;
        let key_start = start + r"\eqref{".len();
        let key_end = source[key_start..]
            .find('}')
            .map(|offset| key_start + offset)
            .expect("closed eqref");
        let key = &source[key_start..key_end];
        let end = key_end + 1;
        let value = output
            .pages
            .iter()
            .flat_map(|page| page.items.iter())
            .find(|item| {
                item.span.start == start
                    && item.span.end == end
                    && source
                        .get(item.span.start..item.span.end)
                        .is_some_and(|text| text.starts_with(r"\eqref{"))
            })
            .unwrap_or_else(|| panic!("no laid-out item for \\eqref{{{key}}}"))
            .text
            .clone();
        values.push(value);
        cursor = end;
    }
    values
}

/// `Inline::Label` values in document order.
fn label_values(source: &str) -> Vec<(String, String)> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flatten()
        .filter_map(|inline| match inline {
            Inline::Label { key, value, .. } => Some((key.clone(), value.clone())),
            _ => None,
        })
        .collect()
}

/// Per-row numbers of the first multi-row display, in row order.
fn math_row_numbers(source: &str) -> Vec<Option<String>> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flatten()
        .find_map(|inline| match inline {
            Inline::MathRows { rows, .. } => {
                Some(rows.iter().map(|row| row.number.clone()).collect())
            }
            _ => None,
        })
        .expect("multi-row display")
}

/// Numbers of every single-`equation` display, in document order.
fn equation_numbers(source: &str) -> Vec<Option<String>> {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flatten()
        .filter_map(|inline| match inline {
            Inline::Math { number, .. } => Some(number.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn align_tag_sets_the_row_label_value() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{align} x &= 1 \tag{A}\label{e:a} \\ y &= 2 \label{e:b} \end{align}",
        r"See \eqref{e:a} and \eqref{e:b}.",
        r"\end{document}",
    );
    assert_eq!(
        label_values(source),
        [
            ("e:a".to_string(), "A".to_string()),
            ("e:b".to_string(), "1".to_string()),
        ]
    );
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(eqref_values(source, &output), ["(A)", "(1)"]);
}

#[test]
fn align_tag_does_not_step_the_equation_counter() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{align} x &= 1 \tag{A} \\ y &= 2 \end{align}",
        r"\begin{equation} z = 3 \end{equation}",
        r"\end{document}",
    );
    // pdflatex probe1: the tagged row prints (A) with no counter step, so
    // the next row is (1) and the following equation is (2).
    assert_eq!(math_row_numbers(source), [None, Some("1".to_string())]);
    assert_eq!(equation_numbers(source), [Some("2".to_string())]);
}

#[test]
fn gather_tag_sets_the_row_label_value_without_stepping_the_counter() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{gather} c = 3 \tag{G}\label{e:g} \\ d = 4 \label{e:h} \end{gather}",
        r"\begin{equation} e = 5 \end{equation}",
        r"See \eqref{e:g} and \eqref{e:h}.",
        r"\end{document}",
    );
    // pdflatex probe3 (same shape, shifted by its earlier displays):
    // the tagged gather row consumes no number, so the rows are (G), (1)
    // and the following equation is (2).
    assert_eq!(math_row_numbers(source), [None, Some("1".to_string())]);
    assert_eq!(equation_numbers(source), [Some("2".to_string())]);
    assert_eq!(
        label_values(source),
        [
            ("e:g".to_string(), "G".to_string()),
            ("e:h".to_string(), "1".to_string()),
        ]
    );
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(eqref_values(source, &output), ["(G)", "(1)"]);
}

#[test]
fn equation_tag_does_not_step_the_counter() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{equation} a = 1 \tag{Z}\label{e:z} \end{equation}",
        r"\begin{equation} b = 2 \label{e:next} \end{equation}",
        r"See \eqref{e:z} and \eqref{e:next}.",
        r"\end{document}",
    );
    // pdflatex probe3: (Z), (1) — a tagged single equation consumes no
    // number either.
    assert_eq!(equation_numbers(source), [None, Some("1".to_string())]);
    assert_eq!(
        label_values(source),
        [
            ("e:z".to_string(), "Z".to_string()),
            ("e:next".to_string(), "1".to_string()),
        ]
    );
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(eqref_values(source, &output), ["(Z)", "(1)"]);
}

#[test]
fn bracket_display_tag_and_label_resolve() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\[x = 1 \tag{B}\label{e:d}\]",
        r"See \eqref{e:d}.",
        r"\end{document}",
    );
    // pdflatex probe2: the display prints (B) and the reference reads (B).
    assert_eq!(label_values(source), [("e:d".to_string(), "B".to_string())]);
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(eqref_values(source, &output), ["(B)"]);
}

#[test]
fn multline_tag_on_first_row_suppresses_the_environment_number() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{multline} a + a \tag{M}\label{m1} \\ b = c \end{multline}",
        r"\begin{equation} d = 1 \label{after} \end{equation}",
        r"See \eqref{m1} and \eqref{after}.",
        r"\end{document}",
    );
    // pdflatex oracle (review probe): `a+a` / `b = c  (M)` / `d=1  (1)`,
    // with `.aux` recording `\newlabel{m1}{{{M}}…}` and
    // `\newlabel{after}{{1}…}` — the multline consumed NO number, so the
    // following equation is still (1).
    assert_eq!(math_row_numbers(source), [None, None]);
    assert_eq!(equation_numbers(source), [Some("1".to_string())]);
    assert_eq!(
        label_values(source),
        [
            ("m1".to_string(), "M".to_string()),
            ("after".to_string(), "1".to_string()),
        ]
    );
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(eqref_values(source, &output), ["(M)", "(1)"]);
}

#[test]
fn multline_tag_on_last_row_suppresses_the_environment_number() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{multline} a + a \\ b = c \tag{M}\label{m1} \end{multline}",
        r"\begin{equation} d = 1 \label{after} \end{equation}",
        r"See \eqref{m1} and \eqref{after}.",
        r"\end{document}",
    );
    // pdflatex: the tag owns the environment's single (last-line) number
    // slot wherever it sits, so this prints `a+a` / `b = c  (M)` /
    // `d=1  (1)` exactly like the first-row-tag case.
    assert_eq!(math_row_numbers(source), [None, None]);
    assert_eq!(equation_numbers(source), [Some("1".to_string())]);
    assert_eq!(
        label_values(source),
        [
            ("m1".to_string(), "M".to_string()),
            ("after".to_string(), "1".to_string()),
        ]
    );
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(eqref_values(source, &output), ["(M)", "(1)"]);
}

#[test]
fn multline_without_tag_numbers_the_last_row() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{multline} a + a \\ b = c \end{multline}",
        r"\begin{equation} d = 1 \label{after} \end{equation}",
        r"\end{document}",
    );
    // pdflatex control: with no `\tag`, the multline takes (1) on its
    // last line and the following equation is (2).
    assert_eq!(
        math_row_numbers(source),
        [None, Some("1".to_string())]
    );
    assert_eq!(equation_numbers(source), [Some("2".to_string())]);
}

#[test]
fn math_mode_tag_reports_the_text_font_gap_downstream() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{equation} w = 2 \tag{$\alpha$}\label{e:alpha} \end{equation}",
        r"See \eqref{e:alpha}.",
        r"\end{document}",
    );
    // pdflatex probe2 prints (α) for both the tag and the reference. The
    // parser-side value pipeline already agrees: the label value is "α",
    // the display lays out "α" from the math Symbol font, and the
    // reference item text is "(α)". What is still missing is downstream:
    // references are shaped in the text face (Times-Roman), which has no
    // α glyph, so the face's .notdef is emitted — visually "()" — with a
    // warning. Closing that gap needs math-font fallback in the
    // layout/font engine, outside this parser-only slice, so this test
    // pins the diagnosed state instead of forcing it.
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let label = parsed
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .flatten()
        .find_map(|inline| match inline {
            Inline::Label { key, value, .. } if key == "e:alpha" => Some(value.clone()),
            _ => None,
        })
        .expect("alpha label");
    assert_eq!(label, "α");

    let output = compile(source);
    let errors: Vec<_> = output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("has no glyph for '\u{3b1}'")),
        "{:?}",
        output.diagnostics
    );
    let texts: Vec<_> = output
        .pages
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| item.text.as_str())
        .collect();
    assert!(texts.contains(&"α"), "{texts:?}");
    assert_eq!(eqref_values(source, &output), ["(α)"]);
}

#[test]
fn multline_middle_row_tag_resolves_every_label_to_the_tag() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{multline} a + a \label{m:first} \\ b + b \tag{B}\label{m:mid} \\ c + c \label{m:last} \end{multline}",
        r"\begin{equation} d = 1 \label{after} \end{equation}",
        r"See \eqref{m:first}, \eqref{m:mid}, \eqref{m:last} and \eqref{after}.",
        r"\end{document}",
    );
    // pdflatex oracle (TeX Live 2026 `/Library/TeX/texbin/pdflatex`,
    // preamble `\documentclass{article}\usepackage{amsmath}`, two passes):
    // the middle-row tag prints on the LAST row (`c+c   (B)` at the last
    // row's baseline), the following equation is (1), and `.aux` records
    // the middle-row label as `{{B}}` — every label in the environment
    // means the environment's single tag, whichever row it was typed on.
    // (Strict pdflatex drops all but the last of several `\label`s with a
    // `Multiple \label's` error; this engine keeps every label, so each
    // one must still read the tag.)
    assert_eq!(math_row_numbers(source), [None, None, None]);
    assert_eq!(equation_numbers(source), [Some("1".to_string())]);
    assert_eq!(
        label_values(source),
        [
            ("m:first".to_string(), "B".to_string()),
            ("m:mid".to_string(), "B".to_string()),
            ("m:last".to_string(), "B".to_string()),
            ("after".to_string(), "1".to_string()),
        ]
    );
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(eqref_values(source, &output), ["(B)", "(B)", "(B)", "(1)"]);
}

#[test]
fn multline_label_on_early_row_reads_the_last_row_number() {
    let source = concat!(
        r"\documentclass{article}",
        r"\usepackage{amsmath}",
        r"\begin{document}",
        r"\begin{multline} a + a \label{m:first} \\ b + b \\ c + c \end{multline}",
        r"\begin{equation} d = 1 \label{after} \end{equation}",
        r"See \eqref{m:first} and \eqref{after}.",
        r"\end{document}",
    );
    // pdflatex oracle (same engine and preamble, two passes): with no
    // `\tag` the last row takes (1), the following equation is (2), and
    // `.aux` records the FIRST-row label as `{1}` — a label on any row
    // reads the environment's single number.
    assert_eq!(
        math_row_numbers(source),
        [None, None, Some("1".to_string())]
    );
    assert_eq!(equation_numbers(source), [Some("2".to_string())]);
    assert_eq!(
        label_values(source),
        [
            ("m:first".to_string(), "1".to_string()),
            ("after".to_string(), "2".to_string()),
        ]
    );
    let output = compile(source);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(eqref_values(source, &output), ["(1)", "(2)"]);
}
