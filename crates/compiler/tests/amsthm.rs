//! amsthm coverage: `\newtheorem` (plain/starred/shared-counter/`[section]`
//! forms), `\theoremstyle`, and `proof`. See `src/theorems.rs` for the style
//! rules this checks: `plain` bolds the head and italicises the body,
//! `definition` bolds the head and leaves the body upright, `remark`
//! italicises the head and leaves the body upright.

use flashtex_compiler::parser::{self, Block, Inline, TextStyle};

fn messages(source: &str) -> Vec<String> {
    parser::parse(source)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

/// Every `Inline::Text` run across every `Block::Paragraph`, in document
/// order, as `(text, style)` pairs — plain enough to assert head/body fonts
/// and exact head text (including the number) against.
fn text_runs(source: &str) -> Vec<(String, TextStyle)> {
    parser::parse(source)
        .blocks
        .into_iter()
        .flat_map(|block| match block {
            Block::Paragraph(inlines) => inlines,
            _ => Vec::new(),
        })
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } => Some((text, style)),
            _ => None,
        })
        .collect()
}

fn plain_texts(source: &str) -> Vec<String> {
    text_runs(source)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

const ITALIC: TextStyle = TextStyle {
    bold: false,
    italic: true,
    family: flashtex_compiler::parser::TextFamily::Roman,
    size: None,
    color: None,
};

#[test]
fn plain_style_bolds_head_and_italicises_body() {
    let source = r"\newtheorem{theorem}{Theorem}
\begin{theorem}
Every prime greater than two is odd.
\end{theorem}";
    let runs = text_runs(source);
    let (head_text, head_style) = &runs[0];
    assert_eq!(head_text, "Theorem 1");
    assert_eq!(*head_style, TextStyle::BOLD);
    assert_eq!(runs[1].0, ".");
    assert_eq!(
        runs[1].1,
        TextStyle::BOLD,
        "\\the\\thm@headpunct is typeset inside \\the\\thm@headfont's group: \
         pdflatex traces a `plain` head's period as `\\T1/cmr/bx/n/10.95 .`"
    );
    let body: Vec<&String> = runs[2..].iter().map(|(text, _)| text).collect();
    assert!(body.contains(&&"Every".to_string()));
    for (_, style) in &runs[2..] {
        assert_eq!(*style, ITALIC, "body text must be italic in plain style");
    }
}

#[test]
fn successive_theorems_number_sequentially() {
    let source = r"\newtheorem{theorem}{Theorem}
\begin{theorem}
First.
\end{theorem}
\begin{theorem}
Second.
\end{theorem}";
    let texts = plain_texts(source);
    assert!(texts.contains(&"Theorem 1".to_string()));
    assert!(texts.contains(&"Theorem 2".to_string()));
}

#[test]
fn optional_note_is_upright_and_parenthesized() {
    let source = r"\newtheorem{theorem}{Theorem}
\begin{theorem}[Fermat]
Statement.
\end{theorem}";
    let runs = text_runs(source);
    assert_eq!(runs[0].0, "Theorem 1");
    // `\thmnote{ {\the\thm@notefont(#3)}}`: the space token is outside the
    // `\thm@notefont` group, so pdflatex sets it from the head font — the
    // fixture's `Definition 1.1 (Divides).` traces the *bold* interword
    // glue `4.17043 plus 2.08443 minus 1.3896` there, not the body face's
    // `3.63054 plus 1.81337 minus 1.20892`.
    assert_eq!(runs[1].0, " ");
    assert_eq!(
        runs[1].1,
        TextStyle::BOLD,
        "the space before the note is a head-font space"
    );
    assert_eq!(runs[2].0, "(Fermat)");
    assert_eq!(
        runs[2].1,
        TextStyle::default(),
        "note must be upright, not italic"
    );
    assert_eq!(runs[3].0, ".");
    assert_eq!(
        runs[3].1,
        TextStyle::BOLD,
        "the head punctuation follows the note, in the head font"
    );
}

#[test]
fn theoremstyle_definition_keeps_body_upright() {
    let source = r"\theoremstyle{definition}
\newtheorem{definition}{Definition}
\begin{definition}
A number is even if it is divisible by two.
\end{definition}";
    let runs = text_runs(source);
    assert_eq!(runs[0].0, "Definition 1");
    assert_eq!(
        runs[0].1,
        TextStyle::BOLD,
        "definition style still bolds the head"
    );
    for (_, style) in &runs[2..] {
        assert_eq!(
            *style,
            TextStyle::default(),
            "definition style leaves the body upright"
        );
    }
}

#[test]
fn theoremstyle_remark_italicises_head_and_keeps_body_upright() {
    let source = r"\theoremstyle{remark}
\newtheorem{remark}{Remark}
\begin{remark}
This generalizes to any ring.
\end{remark}";
    let runs = text_runs(source);
    // `\thmnumber{\@ifnotempty{#1}{ }\@upn{#2}}`: `\@upn` is `\textup`, so
    // the number is upright inside an italic head. pdflatex traces
    // `\OT1/cmr/m/it/10.95 R…k`, `\glue 3.91763 plus 1.67899 minus 1.11932`
    // (italic), `\OT1/cmr/m/n/10.95 1`, `\OT1/cmr/m/it/10.95 .` — so the
    // name, the space and the punctuation are italic and only the number
    // is not.
    assert_eq!(runs[0].0, "Remark");
    assert_eq!(runs[0].1, ITALIC, "remark style italicises the head");
    assert_eq!(runs[1].0, " ");
    assert_eq!(
        runs[1].1, ITALIC,
        "the space before the number is italic too"
    );
    assert_eq!(runs[2].0, "1");
    assert_eq!(
        runs[2].1,
        TextStyle::default(),
        "\\@upn sets the number upright"
    );
    assert_eq!(runs[3].0, ".");
    assert_eq!(
        runs[3].1, ITALIC,
        "the head punctuation is in the italic head font"
    );
    for (_, style) in &runs[4..] {
        assert_eq!(
            *style,
            TextStyle::default(),
            "remark style leaves the body upright"
        );
    }
}

#[test]
fn theoremstyle_switches_back_and_forth() {
    let source = r"\newtheorem{theorem}{Theorem}
\theoremstyle{definition}
\newtheorem{definition}{Definition}
\theoremstyle{plain}
\newtheorem{lemma}{Lemma}
\begin{theorem}
T.
\end{theorem}
\begin{definition}
D.
\end{definition}
\begin{lemma}
L.
\end{lemma}";
    let runs = text_runs(source);
    // theorem: plain (italic body); definition: definition (upright body);
    // lemma: plain again (italic body). Each body is a single one-word run.
    let body_of = |head: &str| -> TextStyle {
        runs.iter()
            .position(|(text, _)| text == head)
            .and_then(|i| runs.get(i + 2))
            .map(|(_, style)| *style)
            .unwrap()
    };
    assert_eq!(body_of("Theorem 1"), ITALIC);
    assert_eq!(body_of("Definition 1"), TextStyle::default());
    assert_eq!(body_of("Lemma 1"), ITALIC);
}

#[test]
fn starred_newtheorem_is_unnumbered_and_does_not_advance_any_counter() {
    let source = r"\newtheorem*{remarkstar}{Remark}
\begin{remarkstar}
No number here.
\end{remarkstar}
\begin{remarkstar}
Still no number.
\end{remarkstar}";
    let texts = plain_texts(source);
    assert!(texts.contains(&"Remark".to_string()));
    assert!(!texts.iter().any(|t| t.starts_with("Remark ")));
}

#[test]
fn shared_counter_interleaves_with_the_theorem_it_shares() {
    let source = r"\newtheorem{theorem}{Theorem}
\newtheorem{lemma}[theorem]{Lemma}
\begin{theorem}
T1.
\end{theorem}
\begin{lemma}
L1.
\end{lemma}
\begin{theorem}
T2.
\end{theorem}";
    let texts = plain_texts(source);
    assert!(texts.contains(&"Theorem 1".to_string()));
    assert!(texts.contains(&"Lemma 2".to_string()));
    assert!(texts.contains(&"Theorem 3".to_string()));
}

#[test]
fn undefined_shared_counter_is_an_honest_error() {
    let source = r"\newtheorem{lemma}[undefinedname]{Lemma}";
    let msgs = messages(source);
    assert!(
        msgs.iter()
            .any(|m| m.contains("undefinedname") && m.contains("undefined")),
        "{msgs:?}"
    );
}

#[test]
fn within_section_resets_per_section_and_prints_section_dot_number() {
    let source = r"\documentclass{article}
\newtheorem{theorem}{Theorem}[section]
\begin{document}
\section{One}
\begin{theorem}
A.
\end{theorem}
\begin{theorem}
B.
\end{theorem}
\section{Two}
\begin{theorem}
C.
\end{theorem}
\end{document}";
    let texts = plain_texts(source);
    assert!(texts.contains(&"Theorem 1.1".to_string()), "{texts:?}");
    assert!(texts.contains(&"Theorem 1.2".to_string()), "{texts:?}");
    assert!(texts.contains(&"Theorem 2.1".to_string()), "{texts:?}");
}

#[test]
fn unsupported_within_counter_warns_and_falls_back_to_a_plain_counter() {
    let source = r"\newtheorem{theorem}{Theorem}[chapter]
\begin{theorem}
A.
\end{theorem}
\begin{theorem}
B.
\end{theorem}";
    let msgs = messages(source);
    assert!(
        msgs.iter()
            .any(|m| m.contains("chapter") && m.contains("recognised but not implemented")),
        "{msgs:?}"
    );
    let texts = plain_texts(source);
    assert!(texts.contains(&"Theorem 1".to_string()));
    assert!(texts.contains(&"Theorem 2".to_string()));
}

#[test]
fn unknown_theoremstyle_name_is_an_honest_error() {
    let msgs = messages(r"\theoremstyle{fancy}");
    assert!(msgs.iter().any(|m| m.contains("fancy")), "{msgs:?}");
}

#[test]
fn proof_has_italic_head_and_right_flushed_qed_symbol() {
    let source = r"\begin{proof}
This follows directly.
\end{proof}";
    let runs = text_runs(source);
    assert_eq!(runs[0].0, "Proof.");
    assert_eq!(runs[0].1, ITALIC);
    for (text, style) in &runs[1..runs.len() - 1] {
        if text != "∎" {
            assert_eq!(*style, TextStyle::default(), "proof body must be upright");
        }
    }
    let (last_text, last_style) = runs.last().unwrap();
    assert_eq!(last_text, "∎");
    assert_eq!(*last_style, TextStyle::default());

    let blocks = parser::parse(source).blocks;
    let has_hfill_before_qed = blocks.iter().any(|block| {
        if let Block::Paragraph(inlines) = block {
            inlines.windows(2).any(|pair| {
                matches!(pair[0], Inline::HFill { .. })
                    && matches!(&pair[1], Inline::Text { text, .. } if text == "∎")
            })
        } else {
            false
        }
    });
    assert!(has_hfill_before_qed, "{blocks:?}");
}

#[test]
fn proof_custom_heading_replaces_default_but_keeps_the_period() {
    let source = r"\begin{proof}[Proof of Lemma 2]
Body.
\end{proof}";
    let runs = text_runs(source);
    assert_eq!(runs[0].0, "Proof of Lemma 2.");
    assert_eq!(runs[0].1, ITALIC);
}

#[test]
fn amsthm_alone_is_silent() {
    let msgs =
        messages(r"\documentclass{article}\usepackage{amsthm}\begin{document}x\end{document}");
    assert!(msgs.is_empty(), "{msgs:?}");
}

/// `\usepackage{amsmath,amssymb,amsthm}` -- HW1's and HW2's line -- is silent
/// now that all three are implemented, not only amsthm: `crate::math` sets the
/// amsmath constructs and gates the amssymb inventory, and the constructs that
/// are still missing report themselves where they are used rather than as a
/// claim about the package. A package that really is only recognised still
/// warns from the same `\usepackage`, and only names itself.
#[test]
fn the_ams_trio_is_silent_and_an_unimplemented_package_still_warns() {
    let msgs = messages(
        r"\documentclass{article}\usepackage{amsmath,amssymb,amsthm}\begin{document}x\end{document}",
    );
    assert!(msgs.is_empty(), "{msgs:?}");

    let msgs = messages(
        r"\documentclass{article}\usepackage{amsmath,amssymb,amsthm,fancyhdr}\begin{document}x\end{document}",
    );
    let package_msgs: Vec<&String> = msgs
        .iter()
        .filter(|m| m.contains("recognised but not implemented"))
        .collect();
    assert_eq!(package_msgs.len(), 1, "{msgs:?}");
    assert!(package_msgs[0].contains("fancyhdr"), "{package_msgs:?}");
    for implemented in ["amsmath", "amssymb", "amsthm"] {
        assert!(
            !package_msgs[0].contains(implemented),
            "{implemented} must not be blamed: {package_msgs:?}"
        );
    }
}

#[test]
fn unregistered_environment_name_still_reports_the_generic_gap() {
    // A name that was never `\newtheorem`-declared is not silently treated
    // as a theorem: it still gets the honest generic diagnostic.
    let msgs = messages(r"\begin{claim}Not declared.\end{claim}");
    assert!(
        msgs.iter()
            .any(|m| m.contains("claim") && m.contains("not implemented")),
        "{msgs:?}"
    );
}

/// `fixtures/real-world/lecture-notes`' first head, run against pdflatex
/// (TeX Live 2025, `\documentclass[11pt]{article}` + `[T1]{fontenc}`,
/// `\tracingoutput=1`). The shipped page holds, in order:
///
/// ```text
/// \T1/cmr/bx/n/10.95 D e ^^\ (ligature fi) n i t i o n
/// \kern 0.0
/// \glue 4.17043 plus 2.08443 minus 1.3896        % the *bold* space
/// \T1/cmr/bx/n/10.95 1 . 1
/// \kern 0.0
/// \glue 4.17043 plus 2.08443 minus 1.3896        % bold again
/// \T1/cmr/m/n/10.95 ( D i v i d e s )            % \thm@notefont
/// \T1/cmr/bx/n/10.95 .                           % \the\thm@headpunct
/// \glue 5.0 plus 1.0 minus 1.0                   % \hskip\thm@headsep
/// \T1/cmr/m/n/10.95 L e t                        % the body
/// ```
///
/// Everything but `(Divides)` is in the head font. The `\glue 5.0` head
/// separator is the render pipeline's to emit (this crate's `Inline` list
/// carries no rubber horizontal glue); what the compiler owns is which
/// piece is set in which font.
#[test]
fn definition_head_with_a_note_matches_the_oracle_font_for_every_piece() {
    let source = r"\theoremstyle{definition}
\newtheorem{definition}{Definition}[section]
\section{Divisibility}
\begin{definition}[Divides]
Let $a$ divide $b$.
\end{definition}";
    let runs = text_runs(source);
    let head: Vec<(&str, TextStyle)> = runs
        .iter()
        .take(4)
        .map(|(text, style)| (text.as_str(), *style))
        .collect();
    assert_eq!(
        head,
        vec![
            ("Definition 1.1", TextStyle::BOLD),
            (" ", TextStyle::BOLD),
            ("(Divides)", TextStyle::default()),
            (".", TextStyle::BOLD),
        ]
    );
}

/// `proof`'s head is `\item[\hskip\labelsep \itshape #1\@addpunct{.}]`:
/// name and period alike come from the italic face, and the body is
/// upright. pdflatex traces the label box as `\glue 5.475` (`\labelsep` at
/// 11pt) then `\OT1/cmr/m/it/10.95 P r o o f … .`.
#[test]
fn proof_head_and_its_period_are_italic() {
    let source = r"\begin{proof}[Proof sketch]
Run the Euclidean algorithm.
\end{proof}";
    let runs = text_runs(source);
    assert_eq!(runs[0].0, "Proof sketch.");
    assert_eq!(
        runs[0].1,
        TextStyle {
            italic: true,
            ..TextStyle::default()
        }
    );
    assert_eq!(runs[1].1, TextStyle::default(), "the proof body is upright");
}
