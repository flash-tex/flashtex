//! enumitem `label=` templates with TeX markup are set, not printed.
//!
//! `\@item` typesets the template, so `\textbf{Q\arabic*.}` is bold "Q1.",
//! `$\square$` is math, `\emph{Step \arabic*}:` is italic "Step 1:", and
//! `--` is an en dash. Before this, the template was flattened to text and
//! the markup printed literally (`\textbf{Q1.}` at x 97.225bp,
//! `\square` at 120.677bp). Templates without markup stay byte-identical
//! (`ItemLabel::Counter`).
//!
//! Oracle: `pdflatex -interaction=nonstopmode` (TeX Live 2026, article 10pt,
//! `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1`) over minimal documents with
//! each template below; `pdftotext -bbox` xMin values are quoted per test.
//! `$\square$` needs `\usepackage{amssymb}` there too: without it pdflatex
//! stops with `! Undefined control sequence. <recently read> \square` and
//! sets no label at all.

use flashtex_compiler::parser::{self, Block, CounterStyle, Inline, ItemLabel};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass[10pt]{{article}}\n\\usepackage{{enumitem}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn doc_amssymb(body: &str) -> String {
    format!(
        "\\documentclass[10pt]{{article}}\n\\usepackage{{enumitem}}\n\\usepackage{{amssymb}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

/// Every `\item`'s label, in document order.
fn labels(source: &str) -> Vec<ItemLabel> {
    parser::parse(source)
        .blocks
        .iter()
        .filter_map(|block| match block {
            Block::ListItem { item, .. } => item.clone(),
            _ => None,
        })
        .collect()
}

fn explicit(label: &ItemLabel) -> (&[Inline], &str) {
    match label {
        ItemLabel::Explicit { content, text, .. } => (content, text.as_str()),
        other => panic!("expected an explicit label, got {other:?}"),
    }
}

/// `Text`/`Math` for each piece, so a test can say what kind of run it is
/// without depending on the math list's shape.
fn kinds(content: &[Inline]) -> Vec<&'static str> {
    content
        .iter()
        .map(|inline| match inline {
            Inline::Text { .. } => "Text",
            Inline::Math { .. } => "Math",
            _ => "other",
        })
        .collect()
}

/// (text, bold, italic) for each text piece.
fn runs(content: &[Inline]) -> Vec<(String, bool, bool)> {
    content
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } => Some((text.clone(), style.bold, style.italic)),
            _ => None,
        })
        .collect()
}

/// Oracle (`pdftotext -bbox` xMin): bold "Q1." at 136.176bp, "Step" at
/// 122.334bp, the en dash at 148.712bp. Each label is set content with the
/// counter substituted — never the literal source.
#[test]
fn markup_templates_are_set_as_explicit_content() {
    let source = doc(concat!(
        "\\begin{enumerate}[label=\\textbf{Q\\arabic*.}]\n",
        "\\item Alpha\n",
        "\\item Beta\n",
        "\\end{enumerate}\n",
        "\\begin{enumerate}[label=\\emph{Step \\arabic*}:]\n",
        "\\item Gamma\n",
        "\\end{enumerate}\n",
        "\\begin{enumerate}[label=--]\n",
        "\\item Delta\n",
        "\\end{enumerate}\n",
    ));
    let parsed = parser::parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "markup labels set silently: {:?}",
        parsed.diagnostics
    );
    let all = labels(&source);
    assert_eq!(all.len(), 4);

    // Bold, with the counter stepped per item.
    let (content, text) = explicit(&all[0]);
    assert_eq!(text, "Q1.");
    assert_eq!(kinds(content), ["Text"]);
    assert!(runs(content).iter().all(|(_, bold, _)| *bold));

    let (content, text) = explicit(&all[1]);
    assert_eq!(text, "Q2.");
    assert!(runs(content).iter().all(|(_, bold, _)| *bold));

    // Italic body with an upright colon after the group.
    let (content, text) = explicit(&all[2]);
    assert_eq!(text, "Step 1:");
    let runs = runs(content);
    assert!(
        runs.iter().any(|(_, _, italic)| *italic),
        "the body is italic: {runs:?}"
    );
    assert!(
        matches!(runs.last(), Some((text, false, false)) if text == ":"),
        "the colon after the group is upright: {runs:?}"
    );

    // The ligature sets as an en dash, like `\item[--]` does. No markup, so
    // it stays a plain template (adapted by flashtex-2a/opus-23: the text,
    // not the label's shape, is what pdflatex pins).
    assert_eq!(all[3].text(), "–");
}

/// Oracle: the MSAM10 square at xMin 145.945bp. Needs `amssymb`, exactly
/// like the document body does.
#[test]
fn square_label_is_math_with_amssymb() {
    let source = doc_amssymb("\\begin{itemize}[label=$\\square$]\n\\item Beta\n\\end{itemize}");
    let parsed = parser::parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "the symbol sets silently: {:?}",
        parsed.diagnostics
    );
    let all = labels(&source);
    assert_eq!(all.len(), 1);
    let (content, text) = explicit(&all[0]);
    assert_eq!(kinds(content), ["Math"]);
    assert_eq!(text, "□");
}

/// Without `amssymb`, pdflatex stops with `! Undefined control sequence.
/// <recently read> \square` and sets no label at all: the label is
/// rejected here too, naming `\square` — never printed literally.
#[test]
fn square_label_is_rejected_without_amssymb() {
    let source = doc("\\begin{itemize}[label=$\\square$]\n\\item Beta\n\\end{itemize}");
    let parsed = parser::parse(&source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message.contains("\\square")),
        "expected a diagnostic naming \\square, got {:?}",
        parsed.diagnostics
    );
    let all = labels(&source);
    assert_eq!(all.len(), 1);
    assert!(
        !all[0].text().contains("\\square"),
        "the source is never printed literally: {:?}",
        all[0]
    );
}

/// No markup: byte-identical `ItemLabel::Counter`s, stepping as before.
/// Oracle xMins in the minimal probe above: `(i)` at 143.178bp and `a)`
/// at 144.838bp.
#[test]
fn plain_templates_stay_counter() {
    let source = doc(concat!(
        "\\begin{enumerate}[label=(\\roman*)]\n",
        "\\item Epsilon\n",
        "\\item Zeta\n",
        "\\end{enumerate}\n",
        "\\begin{enumerate}[label=\\alph*)]\n",
        "\\item Eta\n",
        "\\end{enumerate}\n",
        "\\begin{enumerate}[label=\\arabic*.\\alph*]\n",
        "\\item Theta\n",
        "\\end{enumerate}\n",
    ));
    let parsed = parser::parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "plain templates stay silent: {:?}",
        parsed.diagnostics
    );
    let all = labels(&source);
    assert_eq!(all.len(), 4);
    assert!(matches!(
        &all[0],
        ItemLabel::Counter { value: 1, style: CounterStyle::Roman, prefix, suffix, text }
        if prefix == "(" && suffix == ")" && text == "(i)"
    ));
    assert!(matches!(
        &all[1],
        ItemLabel::Counter { value: 2, style: CounterStyle::Roman, text, .. }
        if text == "(ii)"
    ));
    assert!(matches!(
        &all[2],
        ItemLabel::Counter { value: 1, style: CounterStyle::Alph, prefix, suffix, text }
        if prefix.is_empty() && suffix == ")" && text == "a)"
    ));
    // Several counters without markup keep the flattened template text.
    assert!(matches!(
        &all[3],
        ItemLabel::Template { text } if text == "1.a"
    ));
}

/// Review slice 2 WARNING (`parser.rs` re-lexed spans): every re-lexed
/// label token shares the `\item` span, so `label_plain_text` must not
/// slice composite-math fallback text from the document bytes. Passing the
/// document text there made `label=$\frac{1}{2}$` slice the `\item` bytes
/// for its fallback; with no source text the fallback composes `(1/2)`.
/// Oracle (`pdftotext -bbox` xMin): the stacked 1/2 at 148.527bp, Eta at
/// 158.675bp.
#[test]
fn composite_math_label_text_never_slices_item_bytes() {
    let source = doc("\\begin{enumerate}[label=$\\frac{1}{2}$]\n\\item Alpha\n\\end{enumerate}");
    let parsed = parser::parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "the fraction sets silently: {:?}",
        parsed.diagnostics
    );
    let all = labels(&source);
    assert_eq!(all.len(), 1);
    let (content, text) = explicit(&all[0]);
    assert_eq!(kinds(content), ["Math"]);
    assert_eq!(text, "(1/2)");
    assert!(
        !text.contains("Alpha"),
        "fallback text is composed, never the \\item bytes: {text:?}"
    );
}

/// Review slice 2 WARNING (`token_source` dropped `MathShift`):
/// `\setlist[itemize]{label=$\square$}` arrived at the itemize arm as
/// `label=\square` — a math-only command outside math — while the `\begin`
/// path kept its shifts. The shifts survive now, so both paths set math.
/// Oracle (`pdftotext -bbox` xMin): the setlist square at 145.945bp,
/// byte-identical to the `\begin` path's square; Theta at 158.675bp.
#[test]
fn setlist_math_label_keeps_its_shifts() {
    let source = doc_amssymb(
        "\\setlist[itemize]{label=$\\square$}\n\\begin{itemize}\n\\item Beta\n\\end{itemize}",
    );
    let parsed = parser::parse(&source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(|d| !d.message.contains("\\square")),
        "no diagnostic names the symbol: {:?}",
        parsed.diagnostics
    );
    let all = labels(&source);
    assert_eq!(all.len(), 1);
    let (content, text) = explicit(&all[0]);
    assert_eq!(kinds(content), ["Math"]);
    assert_eq!(text, "□");
}

/// Review slice 3 BLOCKER (`set_enumitem_label` ligatured the whole
/// source): `apply_text_ligatures` is documented for genuine text-mode
/// words only, but it ran over the raw template before re-lexing, so
/// math bytes were rewritten before the lexer saw them — `label=$a'$`
/// set `Math` atoms `[Symbol("a"), Symbol("’")]` with text `"a’"`
/// instead of a prime, and `label=$a--b$` set `"a–b"` instead of two
/// math minuses. The label now re-lexes the original TeX (ligatures
/// still apply to text words through `word_text`), while text-mode
/// `--` still sets as an en dash.
/// Oracle (`pdftotext -layout` codepoints, TL2026): `$a'$ sets as `a` +
/// U+2032 PRIME, `$a--b$` as `a` + U+2212 + U+2212 + `b` — no U+2019 or
/// U+2013 in either label; `--` in text is the en dash (U+2013) at xMin
/// 148.712bp (see `markup_templates_are_set_as_explicit_content`).
#[test]
fn math_ligature_chars_survive_in_math_labels() {
    let source = doc(concat!(
        "\\begin{enumerate}[label=$a'$]\n",
        "\\item Alpha\n",
        "\\end{enumerate}\n",
        "\\begin{enumerate}[label=$a--b$]\n",
        "\\item Beta\n",
        "\\end{enumerate}\n",
    ));
    let parsed = parser::parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "math labels set silently: {:?}",
        parsed.diagnostics
    );
    let all = labels(&source);
    assert_eq!(all.len(), 2);

    let (content, text) = explicit(&all[0]);
    assert_eq!(kinds(content), ["Math"]);
    // A math prime (U+2032), never the ligature's right quote (U+2019).
    assert_eq!(text, "a^\u{2032}");

    let (content, text) = explicit(&all[1]);
    assert_eq!(kinds(content), ["Math"]);
    assert_eq!(text, "a--b");
}

/// `label*` with markup: the enclosing enumerate's current label comes
/// first as plain text, the marked-up star template is set after it.
#[test]
fn starred_markup_templates_prefix_the_enclosing_label() {
    let source = doc(concat!(
        "\\begin{enumerate}\n",
        "\\item A\n",
        "\\begin{enumerate}[label*=\\textbf{\\arabic*.}]\n",
        "\\item B\n",
        "\\end{enumerate}\n",
        "\\end{enumerate}\n",
    ));
    let parsed = parser::parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "starred labels set silently: {:?}",
        parsed.diagnostics
    );
    let all = labels(&source);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].text(), "1.");
    let (content, text) = explicit(&all[1]);
    assert_eq!(text, "1.1.");
    assert_eq!(
        runs(content),
        [
            ("1.".to_string(), false, false),
            ("1.".to_string(), true, false)
        ]
    );
}
