//! GH-SOUL-SO-HL (issue #502): soul `\so{text}` (letterspacing) and
//! `\hl{text}` (highlight) errored `unknown_command`.
//!
//! Real-TeX targets (10pt article, pdflatex), all pinned below:
//! - `\so` inserts soul's `.25em` letterskip kern between every two adjacent
//!   letters of its argument (`ab` = 10.55559pt, `\so{ab}` = 13.05559pt),
//!   reusing the existing text-kern machinery; inner word spaces become
//!   `.65em` and the spaces just outside become `.55em` (`ab cd` =
//!   23.88893pt vs `\so{ab cd}` = 32.05554pt; `x ab y` = 27.77785pt vs
//!   `x \so{ab} y` = 34.61125pt).
//! - `\hl` is a yellow behind-text rule at the argument's natural width
//!   (`word` and `\hl{word}` are both 21.4167pt, same height; only the
//!   depth changes, to 3.22914pt = 0.75ex). Single-line only: real soul's
//!   rule follows each line fragment, which this compiler does not do.
//! - Without soul, `\so`/`\hl` are ordinary undefined names: a user's own
//!   `\newcommand` wins exactly as in real LaTeX. Both compose
//!   (`\hl{\so{..}}`, `\so{\hl{..}}`); both need `\usepackage{soul}` for
//!   the built-in behavior and otherwise diagnose while keeping the text.
//! - soul `\st` (strikethrough) is out of scope (issue #330's ulem-side
//!   work) and must keep its exact `unknown_command` error.
use flashtex_compiler::color::{ColorSpace, DeviceColor};
use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::parser::{parse, Block, Inline, UnderlineGeom};
use flashtex_compiler::text_builtins::TextDimen;

fn soul_doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage{{soul}}\n\\begin{{document}}\n{body}\n\\end{{document}}"
    )
}

fn paragraph_inlines(source: &str) -> Vec<Inline> {
    parse(source)
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .unwrap_or_default()
}

fn text_of(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, .. } => out.push_str(text),
            Inline::ColorBox(b) => out.push_str(&text_of(&b.content)),
            Inline::Underline(u) => out.push_str(&text_of(&u.content)),
            _ => {}
        }
    }
    out
}

/// Compact signature of a paragraph for exact-sequence pins: `T(x,true)` is
/// a text piece, `K` a kern, `G(0.65)`/`G(0.55)` soul's explicit word-space
/// glue, anything else `OTHER(..)`.
fn sig(inlines: &[Inline]) -> Vec<String> {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text {
                text, space_before, ..
            } => format!("T({text},{space_before})"),
            Inline::Kern { .. } => "K".to_string(),
            Inline::TextGlue { em, .. } => format!("G({em})"),
            other => format!("OTHER({other:?})"),
        })
        .collect()
}

fn kern_amounts(inlines: &[Inline]) -> Vec<TextDimen> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Kern { amount, .. } => Some(amount.clone()),
            _ => None,
        })
        .collect()
}

fn glue_ems(inlines: &[Inline]) -> Vec<f64> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::TextGlue { em, .. } => Some(*em),
            _ => None,
        })
        .collect()
}

fn yellow() -> DeviceColor {
    DeviceColor::from_billionths(ColorSpace::Cmyk, &[0, 0, 1_000_000_000, 0])
        .expect("soul highlight yellow parses")
}

/// Issue #502: `\so{text}` is letterspaced, not unknown. Real soul.sty
/// (`soul-ori.sty:670`, `\sodef\textso{}{.25em}{...}`) uses a .25em
/// letterskip: 10pt `ab` = 10.55559pt, `\so{ab}` = 13.05559pt, one gap of
/// exactly 2.5pt.
#[test]
fn so_with_soul_kerns_between_letters() {
    let source = soul_doc("Text \\so{text} here.");
    let diagnostics = parse(&source).diagnostics;
    assert!(
        diagnostics.is_empty(),
        "\\so with soul loaded should be silent: {diagnostics:?}"
    );
    let inlines = paragraph_inlines(&source);
    let joined = text_of(&inlines);
    assert!(
        joined.contains("text"),
        "argument must stay visible: {inlines:?}"
    );
    let want = TextDimen::parse("0.25em").expect("letterskip parses");
    let mut letters = 0;
    let mut kerns = 0;
    for inline in &inlines {
        match inline {
            Inline::Text { text, .. } if text == "t" || text == "e" || text == "x" => {
                letters += 1
            }
            Inline::Kern { amount, .. } => {
                assert_eq!(amount, &want, "soul letterskip is 0.25em: {inlines:?}");
                kerns += 1;
            }
            _ => {}
        }
    }
    assert!(letters >= 4, "letters split apart: {inlines:?}");
    assert_eq!(kerns, 3, "one kern between every two letters: {inlines:?}");
    // The surrounding spaces are soul's .55em edge spaces (finding 3), not
    // natural glue: `x ab y` = 27.77785pt vs `x \so{ab} y` = 34.61125pt.
    assert_eq!(
        glue_ems(&inlines),
        vec![0.55, 0.55],
        "one .55em edge space on each side: {inlines:?}"
    );
}

/// `\so` without soul diagnoses (like ulem without ulem) and keeps text.
#[test]
fn so_without_soul_diagnoses_and_keeps_text() {
    let source = "\\documentclass{article}\n\\begin{document}\nText \\so{text} here.\n\\end{document}";
    let diagnostics = parse(source).diagnostics;
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("\\so needs \\usepackage{soul}")),
        "missing-package diagnostic: {diagnostics:?}"
    );
    assert!(
        !diagnostics.iter().any(|d| d.code == Some(DiagnosticCode::UnknownCommand)),
        "\\so is known once implemented: {diagnostics:?}"
    );
    let joined = text_of(&paragraph_inlines(source));
    assert!(joined.contains("text"), "argument kept: {joined:?}");
}

/// Issue #502: `\hl{text}` is a yellow behind-text rule at the argument's
/// natural width — not unknown, and not a padded box. Real soul draws the
/// highlight as an underline-style rule BEHIND the text: 10pt `word` and
/// `\hl{word}` are both 21.4167pt wide with the same height; only the depth
/// changes (to 3.22914pt = 0.75ex for the rule below the baseline).
#[test]
fn hl_with_soul_is_a_natural_width_highlight() {
    let source = soul_doc("Text \\hl{word} here.");
    let diagnostics = parse(&source).diagnostics;
    assert!(
        diagnostics.is_empty(),
        "\\hl with soul loaded should be silent: {diagnostics:?}"
    );
    let inlines = paragraph_inlines(&source);
    let boxes: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::ColorBox(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(boxes.len(), 1, "one highlight box: {inlines:?}");
    let hl = boxes[0];
    // Yellow fill, no frame, and — the width fix — ZERO separation: the box
    // adds no padding on any side, so `\hl{word}` lays out exactly as wide
    // as `word` (the old `\fboxsep` padding made it ~6pt wider).
    assert_eq!(hl.fill, yellow(), "highlight fill is yellow");
    assert_eq!(hl.frame, None, "highlight has no frame");
    assert_eq!(
        hl.fboxsep_pt, 0.0,
        "no padding: width is the content's own: {inlines:?}"
    );
    assert_eq!(hl.fboxrule_pt, 0.0, "no frame rule: {inlines:?}");
    // The depth comes from the zero-thickness highlight underline inside:
    // thickness 0 draws no rule in either layout, while the SoulHighlight
    // geometry still extends the fragment to the rule depth. Single
    // unbreakable fragment (real soul's rule follows each line fragment
    // instead — documented limitation, not silently ignored).
    assert_eq!(
        hl.content.len(),
        1,
        "single unbreakable fragment: {inlines:?}"
    );
    let inner = match &hl.content[0] {
        Inline::Underline(u) => u,
        other => panic!("highlight wraps one underline: {other:?}"),
    };
    assert_eq!(inner.thickness_pt, 0.0, "no over-bar: {inlines:?}");
    assert!(
        matches!(inner.geom, UnderlineGeom::SoulHighlight),
        "highlight geometry: {inlines:?}"
    );
    assert_eq!(text_of(&inner.content), "word");
}

/// Finding 4's measured target: the highlight geometry extends the depth to
/// 0.75ex — at 10pt cmr (x-height 4.30554pt) that is 3.22914pt, exactly real
/// pdflatex's `\hl{word}` depth — while the rule top sits above the baseline
/// (behind the glyphs). Width/height are the content's own by construction
/// (zero padding, fragment depth-only extension), pinned structurally above.
#[test]
fn soul_highlight_geom_reaches_three_quarters_ex() {
    let ex_10pt_cmr = 4.30554;
    let (top, extra) =
        UnderlineGeom::SoulHighlight.rule_top_and_depth(0.0, 0.0, 2.5, ex_10pt_cmr);
    assert!(
        (extra - 3.22914).abs() < 0.0001,
        "highlight depth is the measured 3.22914pt: got {extra}"
    );
    assert!(top < 0.0, "rule top sits above the baseline: got {top}");
}

/// `\hl` without soul diagnoses and keeps text.
#[test]
fn hl_without_soul_diagnoses_and_keeps_text() {
    let source = "\\documentclass{article}\n\\begin{document}\nText \\hl{text} here.\n\\end{document}";
    let diagnostics = parse(source).diagnostics;
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("\\hl needs \\usepackage{soul}")),
        "missing-package diagnostic: {diagnostics:?}"
    );
    let joined = text_of(&paragraph_inlines(source));
    assert!(joined.contains("text"), "argument kept: {joined:?}");
}

/// Issue #502: `\hl{\so{..}}` and `\so{\hl{..}}` both work.
#[test]
fn so_and_hl_compose_both_ways() {
    let outer_hl = soul_doc("Text \\hl{\\so{ab}} here.");
    let diagnostics = parse(&outer_hl).diagnostics;
    assert!(diagnostics.is_empty(), "\\hl{{\\so}} silent: {diagnostics:?}");
    let inlines = paragraph_inlines(&outer_hl);
    let boxed = inlines.iter().find_map(|inline| match inline {
        Inline::ColorBox(b) => Some(b),
        _ => None,
    });
    let boxed = boxed.expect("highlight box survives");
    assert_eq!(boxed.fill, yellow());
    assert_eq!(
        boxed.fboxsep_pt, 0.0,
        "no padding on the outer highlight either: {inlines:?}"
    );
    let inner = match &boxed.content[0] {
        Inline::Underline(u) => u,
        other => panic!("highlight wraps one underline: {other:?}"),
    };
    assert!(
        inner
            .content
            .iter()
            .any(|i| matches!(i, Inline::Kern { .. })),
        "letterspacing inside the highlight: {:?}",
        inner.content
    );

    let outer_so = soul_doc("Text \\so{\\hl{ab}} here.");
    let diagnostics = parse(&outer_so).diagnostics;
    assert!(diagnostics.is_empty(), "\\so{{\\hl}} silent: {diagnostics:?}");
    let inlines = paragraph_inlines(&outer_so);
    assert!(
        inlines.iter().any(|i| matches!(i, Inline::ColorBox(_))),
        "highlight box inside the spacing: {inlines:?}"
    );
    assert_eq!(text_of(&inlines).replace(' ', ""), "Textabhere.");
}

/// Multi-word `\so` kerns only WITHIN words, and the inner word space is
/// soul's `.65em` — never the natural glue, never kerned across. Measured:
/// `ab cd` = 23.88893pt, `\so{ab cd}` = 32.05554pt (two .25em gaps = 5pt,
/// plus the wider space: 6.5pt vs 3.33333pt natural = +3.16667pt).
#[test]
fn so_multiword_inner_spaces_are_wider() {
    // Exact sequence pin for `\so{ab cd}` at the document start (no leading
    // edge glue there: paragraph-start space is neutralized by the layouts,
    // so none is emitted): one .25em kern inside each word, one .65em glue
    // across the word space, and the pieces around it carry no natural
    // space of their own.
    let source = soul_doc("\\so{ab cd}");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "\\so{{ab cd}} silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    assert_eq!(
        sig(&inlines),
        vec![
            "T(a,true)",
            "K",
            "T(b,false)",
            "G(0.65)",
            "T(c,false)",
            "K",
            "T(d,false)"
        ],
        "inner space is .65em glue, no kern across it: {inlines:?}"
    );
    let quarter = TextDimen::parse("0.25em").expect("letterskip parses");
    assert!(
        kern_amounts(&inlines).iter().all(|k| k == &quarter),
        "both kerns are .25em: {inlines:?}"
    );

    // Invariant + kern/glue counts across word shapes: two short words, three
    // words of different lengths, and the single-word case (3 kerns, no
    // glue). Every word gap is exactly one .65em glue with a kern on neither
    // side; no piece keeps a natural `space_before` past the first.
    for (body, want_kerns, want_glues) in [
        ("\\so{ab cd}", 2, vec![0.65]),
        ("\\so{a bb ccc}", 3, vec![0.65, 0.65]),
        ("\\so{text}", 3, vec![]),
    ] {
        let source = soul_doc(body);
        assert!(
            parse(&source).diagnostics.is_empty(),
            "{body} silent: {:?}",
            parse(&source).diagnostics
        );
        let inlines = paragraph_inlines(&source);
        for pair in inlines.windows(2) {
            if matches!(pair[1], Inline::TextGlue { .. }) {
                assert!(
                    !matches!(pair[0], Inline::Kern { .. }),
                    "no kern before the word space in {body}: {inlines:?}"
                );
            }
            if matches!(pair[0], Inline::TextGlue { .. }) {
                assert!(
                    !matches!(pair[1], Inline::Kern { .. }),
                    "no kern after the word space in {body}: {inlines:?}"
                );
            }
        }
        assert!(
            inlines
                .iter()
                .skip(1)
                .filter_map(|inline| match inline {
                    Inline::Text { space_before, .. } => Some(*space_before),
                    _ => None,
                })
                .all(|space_before| !space_before),
            "no natural space survives past the first piece in {body}: {inlines:?}"
        );
        assert_eq!(
            kern_amounts(&inlines).len(),
            want_kerns,
            "kern count for {body}: {inlines:?}"
        );
        assert_eq!(
            glue_ems(&inlines),
            want_glues,
            "word-space glue for {body}: {inlines:?}"
        );
        assert_eq!(
            text_of(&inlines).replace(' ', ""),
            body
                .trim_start_matches("\\so{")
                .trim_end_matches('}')
                .replace(' ', ""),
            "letters preserved for {body}"
        );
    }
}

/// Spaces just outside `\so{...}` become soul's `.55em` edge space instead
/// of the natural glue. Measured: `x ab y` = 27.77785pt,
/// `x \so{ab} y` = 34.61125pt (one .25em gap = 2.5pt, plus two widened
/// spaces: 2 * (5.5pt - 3.33333pt) = +4.33334pt).
#[test]
fn so_adjacent_spaces_widen_to_half_em() {
    let source = soul_doc("x \\so{ab} y");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "edge spaces silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    assert_eq!(
        sig(&inlines),
        vec![
            "T(x,true)",
            "G(0.55)",
            "T(a,false)",
            "K",
            "T(b,false)",
            "G(0.55)",
            "T(y,false)"
        ],
        ".55em on each side, nothing doubled: {inlines:?}"
    );

    // One source space between two groups stays one widened gap: the first
    // `\so` consumes it as its trailing edge, so the second sees no
    // preceding space and emits no leading glue of its own.
    let source = soul_doc("x \\so{ab} \\so{cd} y");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "adjacent groups silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    assert_eq!(
        glue_ems(&inlines),
        vec![0.55, 0.55, 0.55],
        "leading, middle, trailing — exactly one widened gap each: {inlines:?}"
    );
    assert_eq!(
        text_of(&inlines).replace(' ', ""),
        "xabcdy",
        "letters preserved: {inlines:?}"
    );
}

/// No trailing edge space where real TeX drops the glue: before a paragraph
/// break or `\end`, the space after `\so{...}` vanishes instead of widening.
#[test]
fn so_trailing_space_dropped_at_paragraph_end() {
    for body in ["End \\so{ab}", "Trail \\so{ab}\n\nTail."] {
        let source = soul_doc(body);
        assert!(
            parse(&source).diagnostics.is_empty(),
            "{body:?} silent: {:?}",
            parse(&source).diagnostics
        );
        let inlines = paragraph_inlines(&source);
        assert_eq!(
            glue_ems(&inlines),
            vec![0.55],
            "only the leading edge space in {body:?}: {inlines:?}"
        );
        assert!(
            matches!(
                inlines.last(),
                Some(Inline::Text { text, .. }) if text == "b"
            ),
            "paragraph ends on the last letter in {body:?}: {inlines:?}"
        );
    }
}

/// Out of scope: soul `\st` keeps its exact `unknown_command` error, with
/// or without the package.
#[test]
fn st_still_errors_unknown_command() {
    for source in [
        soul_doc("Text \\st{text} here."),
        "\\documentclass{article}\n\\begin{document}\nText \\st{text} here.\n\\end{document}".to_string(),
    ] {
        let diagnostics = parse(&source).diagnostics;
        assert!(
            diagnostics.iter().any(|d| d.code == Some(DiagnosticCode::UnknownCommand)
                && d.message.contains("\\st")),
            "\\st must stay unknown_command: {diagnostics:?}"
        );
    }
}

/// Finding 1 (slice-3 review): without soul, `\so`/`\hl` are ordinary
/// undefined names, so a user's own `\newcommand` wins exactly as it would
/// for any other non-kernel name (real pdflatex accepts
/// `\newcommand{\hl}[1]{\textcolor{RoyalBlue}{#1}}` when soul is absent).
/// The built-in soul behavior only kicks in with `\usepackage{soul}`.
#[test]
fn user_hl_macro_wins_without_soul() {
    let source = "\\documentclass{article}\n\\usepackage[dvipsnames]{xcolor}\n\\newcommand{\\hl}[1]{\\textcolor{RoyalBlue}{#1}}\n\\begin{document}\nBlue \\hl{word} here.\n\\end{document}";
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "user \\hl must win without soul: {:?}",
        parsed.diagnostics
    );
    let inlines = paragraph_inlines(source);
    let royal: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } if text == "word" => style.color,
            _ => None,
        })
        .collect();
    assert_eq!(
        royal.len(),
        1,
        "the user macro's RoyalBlue argument must survive: {inlines:?}"
    );
    assert_eq!(
        royal[0].fill_operator(),
        "1 0.5 0 0 k",
        "RoyalBlue, not soul yellow: {inlines:?}"
    );
    assert!(
        !inlines.iter().any(|i| matches!(i, Inline::ColorBox(_))),
        "no soul highlight box without soul: {inlines:?}"
    );
}

/// Same as above for `\so`: a user-defined `\so` without soul expands.
#[test]
fn user_so_macro_wins_without_soul() {
    let source = "\\documentclass{article}\n\\newcommand{\\so}[1]{[#1]}\n\\begin{document}\nA \\so{bc} d.\n\\end{document}";
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "user \\so must win without soul: {:?}",
        parsed.diagnostics
    );
    let joined = text_of(&paragraph_inlines(source));
    assert!(
        joined.contains("[bc]"),
        "user \\so expansion must survive: {joined:?}"
    );
}

/// `\usepackage{soul}` loads silently (no "not implemented" warning).
#[test]
fn soul_package_load_is_silent() {
    let source = soul_doc("Text here.");
    let diagnostics = parse(&source).diagnostics;
    assert!(
        !diagnostics
            .iter()
            .any(|d| d.message.contains("not implemented")),
        "soul is implemented: {diagnostics:?}"
    );
}
