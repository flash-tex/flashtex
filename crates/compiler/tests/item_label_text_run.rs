//! `\item[<label>]` is an inline **text run** (GH-676).
//!
//! latex.ltx `\@item` passes the optional argument to `\makelabel`, which
//! sets it in ordinary text mode — so `$…$` inside it re-enters math, the
//! text-style commands apply, and a command that is only legal in math is an
//! error there exactly as it is in running text. Before this, the label was
//! read by a flattened token pass that kept `Word` tokens and dropped
//! everything else without a word: `\item[this is $2x$]` became the two words
//! "this" and "is2x", and `\item[$\alpha$]` produced no label at all and no
//! diagnostic.
//!
//! Two properties are pinned here, both from the issue:
//!   * nothing in `[...]` is dropped in silence — every label carries content,
//!     or a diagnostic says why it does not;
//!   * every piece's span covers the bracket text it came from, never the
//!     `\item` token, so hover and hit-testing on a label land on the label.

use flashtex_compiler::parser::{self, Block, Inline, ItemLabel};

fn doc(body: &str) -> String {
    format!("\\documentclass[10pt]{{article}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n")
}

/// Every `\item`'s label, in document order.
fn labels(parsed: &parser::Parsed) -> Vec<ItemLabel> {
    parsed
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

/// The source each piece's span points at.
fn spanned<'a>(source: &'a str, content: &[Inline]) -> Vec<&'a str> {
    content
        .iter()
        .map(|inline| {
            let span = match inline {
                Inline::Text { span, .. } | Inline::Math { span, .. } => *span,
                other => panic!("unexpected label piece {other:?}"),
            };
            &source[span.start..span.end]
        })
        .collect()
}

/// The issue's five-item repro, end to end: a label run for all five items,
/// the two formulas set as math rather than as roman words, and not one
/// diagnostic — nothing here is unsupported.
#[test]
fn the_issue_repro_keeps_every_label_and_reports_nothing() {
    let source = doc(concat!(
        "\\begin{itemize}\n",
        "\\item[this is $2x$] body one\n",
        "\\item[\\textbf{bold}] body two\n",
        "\\item[(a)] body three\n",
        "\\end{itemize}\n",
        "\\begin{enumerate}\n",
        "\\item[$\\alpha$] alpha body\n",
        "\\item plain\n",
        "\\end{enumerate}\n",
        "\\begin{description}\n",
        "\\item[Term $x^2$] definition\n",
        "\\end{description}",
    ));
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    assert_eq!(labels.len(), 6, "five bracketed items and one plain one");

    // Not one label is empty: that is the "0 silent drops" gate.
    for label in &labels {
        assert!(
            !label.text().is_empty(),
            "a label lost all of its content: {label:?}"
        );
    }

    let (content, text) = explicit(&labels[0]);
    assert_eq!(text, "this is 2x");
    assert_eq!(kinds(content), ["Text", "Text", "Math"]);

    let (_, text) = explicit(&labels[1]);
    assert_eq!(text, "bold");

    let (_, text) = explicit(&labels[2]);
    assert_eq!(text, "(a)");

    // The label that used to vanish outright.
    let (content, text) = explicit(&labels[3]);
    assert_eq!(text, "\u{3b1}");
    assert_eq!(kinds(content), ["Math"]);

    // The bracketed `\item` does not step the counter, so the plain item is
    // still "1." (latex.ltx `\@noitemargtrue`); unchanged, pinned here so the
    // label work cannot quietly move it.
    assert_eq!(labels[4].text(), "1.");

    let (content, text) = explicit(&labels[5]);
    assert_eq!(text, "Term x2");
    assert_eq!(kinds(content), ["Text", "Math"]);

    assert!(
        parsed.diagnostics.is_empty(),
        "nothing in the repro is unsupported, got {:?}",
        parsed.diagnostics
    );
}

/// `\descriptionlabel` is `\normalfont\bfseries #1`: the run starts bold, and
/// a style command inside the brackets composes with it rather than replacing
/// the run.
#[test]
fn description_labels_start_bold_and_styles_compose() {
    let source = doc(
        "\\begin{description}\\item[Term \\emph{it} \\texttt{tt}] d\\end{description}\
         \\begin{itemize}\\item[Term \\emph{it}] i\\end{itemize}",
    );
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    let styles = |label: &ItemLabel| -> Vec<(bool, bool)> {
        explicit(label)
            .0
            .iter()
            .filter_map(|inline| match inline {
                Inline::Text { style, .. } => Some((style.bold, style.italic)),
                _ => None,
            })
            .collect()
    };
    // description: bold throughout, italic only on the `\emph` piece.
    assert_eq!(styles(&labels[0]), [(true, false), (true, true), (true, false)]);
    // itemize: `\makelabel` adds no weight of its own.
    assert_eq!(styles(&labels[1]), [(false, false), (false, true)]);
}

/// Issue point 2: a label piece's span is the bracket text it came from. The
/// `\item` token is never the answer — hover and caret mapping on a label
/// used to land there for every cluster.
#[test]
fn label_spans_cover_the_bracket_text_not_the_item_token() {
    let source =
        doc("\\begin{itemize}\\item[this is $2x$] body one\\end{itemize}");
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    let (content, _) = explicit(&labels[0]);
    assert_eq!(spanned(&source, content), ["this", "is", "$2x$"]);

    let ItemLabel::Explicit { span, .. } = &labels[0] else {
        unreachable!()
    };
    // The argument's own span is the brackets, and every piece sits inside it.
    assert_eq!(&source[span.start..span.end], "[this is $2x$]");
    for inline in content {
        let piece = match inline {
            Inline::Text { span, .. } | Inline::Math { span, .. } => *span,
            other => panic!("unexpected label piece {other:?}"),
        };
        assert!(
            piece.start >= span.start && piece.end <= span.end,
            "{piece:?} is outside the bracketed argument {span:?}"
        );
    }
}

/// A command a label cannot set is reported at its own span and the rest of
/// the label survives — the same two messages running text raises for a
/// math-only command and for an unknown one.
#[test]
fn an_unsettable_label_piece_is_diagnosed_not_dropped() {
    let source = doc("\\begin{itemize}\\item[\\alpha keeps \\frobnicate going] b\\end{itemize}");
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    let (_, text) = explicit(&labels[0]);
    // The words either side of both commands survive. There is no space
    // between them because TeX's lexer eats the space after a control word,
    // and running text loses it the same way when the command sets nothing —
    // this label is byte-for-byte what `keeps \frobnicate going` produces in
    // a paragraph, which is the parity GH-676 asks for.
    assert_eq!(text, "keepsgoing", "the words around the commands stay");
    let messages: Vec<_> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        messages,
        [
            "\\alpha is not supported by this compiler version",
            "\\frobnicate is not supported by this compiler version",
        ]
    );
    for diagnostic in &parsed.diagnostics {
        let span = diagnostic.span.expect("a span on each");
        assert!(
            source[span.start..span.end].starts_with('\\'),
            "the diagnostic points at the command, not the item"
        );
    }
}

/// A script marker and a stray `\)` in a label get running text's own
/// messages rather than vanishing.
#[test]
fn stray_math_markers_in_a_label_are_diagnosed() {
    let source = doc("\\begin{itemize}\\item[a^2] b\\item[$x$ \\)] c\\end{itemize}");
    let parsed = parser::parse(&source);
    let messages: Vec<_> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        messages,
        [
            "math script marker used outside math mode",
            "stray \\) has no matching \\(",
        ]
    );
    // `a^2` still contributes both characters.
    assert_eq!(labels(&parsed)[0].text(), "a2");
}

/// `\[..\]` (display math) inside a label is rejected, matching real
/// pdflatex's "Bad math environment delimiter" for `\[` in restricted
/// horizontal mode -- not silently accepted and dropped.
#[test]
fn display_math_in_a_label_is_diagnosed_not_dropped() {
    let source = doc("\\begin{itemize}\\item[\\[x\\]]body\\end{itemize}");
    let parsed = parser::parse(&source);
    let messages: Vec<_> = parsed.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert_eq!(messages, ["Bad math environment delimiter", "Bad math environment delimiter"]);
}

/// `\(…\)` is the other inline-math spelling and reads the same way.
#[test]
fn paren_math_in_a_label_is_math_too() {
    let source = doc("\\begin{itemize}\\item[at \\(y_1\\)] b\\end{itemize}");
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    let (content, text) = explicit(&labels[0]);
    assert_eq!(kinds(content), ["Text", "Math"]);
    assert_eq!(text, "at y1");
    assert_eq!(spanned(&source, content), ["at", "\\(y_1\\)"]);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

/// Unterminated math in a label ends with the label, and says so — it does
/// not run on into the item's body.
#[test]
fn unterminated_label_math_is_reported_and_closed() {
    let source = doc("\\begin{itemize}\\item[$x] body\\end{itemize}");
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    let (content, _) = explicit(&labels[0]);
    assert_eq!(kinds(content), ["Math"]);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.message == "inline math is missing its closing '$'"),
        "{:?}",
        parsed.diagnostics
    );
}

/// `\cite` in a label sets the same run running text builds — `[?]` plus
/// the undefined-citation warning — never the false "not supported" error.
#[test]
fn cite_in_a_label_matches_running_text() {
    let source = doc("\\begin{itemize}\\item[\\cite{k}] b\\end{itemize}");
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    let (_, text) = explicit(&labels[0]);
    assert_eq!(text, "[?]");
    let messages: Vec<_> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(messages, ["citation 'k' is undefined"]);
}

/// ... including when the key resolves: `[1]` with no diagnostic at all.
#[test]
fn cite_in_a_label_resolves_like_running_text() {
    let source = doc(concat!(
        "\\begin{thebibliography}{9}\n",
        "\\bibitem{k} Someone. Title. 2020.\n",
        "\\end{thebibliography}\n",
        "\\begin{itemize}\\item[\\cite{k}] b\\end{itemize}",
    ));
    let parsed = parser::parse(&source);
    // The bibliography's own `\bibitem` is the second list item; the
    // `\cite` label is the last one.
    let labels = labels(&parsed);
    let (_, text) = explicit(&labels[labels.len() - 1]);
    assert_eq!(text, "[1]");
    assert!(
        parsed.diagnostics.is_empty(),
        "a resolvable cite sets silently: {:?}",
        parsed.diagnostics
    );
}

/// `\index` and `\protect` set nothing visible, so a label swallows them
/// the way a heading does: `[\index{i}a]` keeps `a` and reports nothing.
/// `\label`, unlike those two, has a real side effect: it registers the key
/// exactly like `\label` in running text does, so a later `\ref` to it
/// resolves instead of printing "??".
#[test]
fn index_and_protect_are_silent_in_a_label() {
    let source = doc(concat!(
        "\\begin{itemize}\n",
        "\\item[\\index{i}a] c\n",
        "\\item[\\protect\\cite{k}] d\n",
        "\\end{itemize}",
    ));
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    assert_eq!(labels.len(), 2);
    let (_, text) = explicit(&labels[0]);
    assert_eq!(text, "a");
    // `\protect` vanishes; the `\cite` still sets (and still warns).
    let (_, text) = explicit(&labels[1]);
    assert_eq!(text, "[?]");
    let messages: Vec<_> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(messages, ["citation 'k' is undefined"]);
}

/// `\label{it:a}` inside an `\item` label registers the key in the same
/// place ordinary running text's `\label` does -- proven here by a second,
/// ordinary `\label{it:a}` colliding with it and firing the ordinary
/// "duplicate \label" diagnostic (before this fix, the item-bracket
/// `\label` was silently consumed without registering anything, so the
/// second one had nothing to collide with, and a `\ref{it:a}` anywhere in
/// the document would print "??" instead of the item's number).
#[test]
fn label_inside_an_item_label_registers_like_running_texts_label() {
    let source = doc(concat!(
        "\\begin{itemize}\n",
        "\\item[(a)\\label{it:a}] b\n",
        "\\end{itemize}\n",
        "\\label{it:a}",
    ));
    let parsed = parser::parse(&source);
    let labels = labels(&parsed);
    assert_eq!(labels.len(), 1);
    let (_, text) = explicit(&labels[0]);
    assert_eq!(text, "(a)");
    let messages: Vec<_> = parsed.diagnostics.iter().map(|d| d.message.clone()).collect();
    assert_eq!(messages, ["duplicate \\label{it:a}; the second definition wins"]);
}

/// Built-ins this pass cannot set yet (`\footnote`, `\underline`, `\url`,
/// ...) get an honest warning naming the command — never the false claim
/// that the compiler does not support them.
#[test]
fn unsettable_builtins_in_a_label_get_an_honest_warning() {
    let source = doc(concat!(
        "\\begin{itemize}\n",
        "\\item[\\footnote{f}] b\n",
        "\\item[\\underline{u}] c\n",
        "\\item[\\url{https://x.y}] d\n",
        "\\end{itemize}",
    ));
    let parsed = parser::parse(&source);
    let messages: Vec<_> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(
        messages,
        [
            "\\footnote inside an \\item label is not set yet",
            "\\underline inside an \\item label is not set yet",
            "\\url inside an \\item label is not set yet",
        ]
    );
    // The words around each command still reach the page. (`\url`'s
    // verbatim argument never arrives here as text — headings drop it
    // the same way, a pre-existing gap outside this slice — so that
    // label is empty, but diagnosed above rather than silently dropped.)
    let labels = labels(&parsed);
    let texts: Vec<_> = labels.iter().map(|label| label.text().to_string()).collect();
    assert_eq!(texts, ["f", "u", ""]);
}

/// The same flattened pass serves headings, captions and style arguments.
/// Math there is read now too — it was stripped to roman words before — but
/// those stay lenient about commands they cannot set, because they routinely
/// carry `\label`, `\protect` and friends that are correctly ignored. A flood
/// of diagnostics from every heading is not what GH-676 asked for.
#[test]
fn headings_read_nested_math_but_stay_lenient_about_commands() {
    let source = doc("\\section{A $2x$ B \\frobnicate}\nText.");
    let parsed = parser::parse(&source);
    let heading = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Heading { content, .. } => Some(content.clone()),
            _ => None,
        })
        .expect("one heading");
    assert_eq!(kinds(&heading), ["Text", "Math", "Text"]);
    assert_eq!(spanned(&source, &heading), ["A", "$2x$", "B"]);
    assert!(
        parsed.diagnostics.is_empty(),
        "a heading does not report what it skips: {:?}",
        parsed.diagnostics
    );
}
