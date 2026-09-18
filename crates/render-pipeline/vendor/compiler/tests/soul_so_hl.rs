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
//!   with `xcolor` loaded (`word` and `\hl{word}` are both 21.4167pt wide;
//!   the rule's height is 1.75ex = 7.5347pt and its depth 0.75ex =
//!   3.22914pt, recorded as `SoulHighlightExtents` on the box — the depth
//!   grows the line, the height is consumed by the render-pipeline paint
//!   path, which still paints at content height until the vendored
//!   compiler is re-pinned, see GH-828). Single-line only: real soul's
//!   rule follows each line
//!   fragment, which this compiler does not do; the interword gaps between
//!   the fragments are not painted either (one `FidelityNote` warning per
//!   multi-word `\hl`, see GH-828).
//! - Without soul, `\so`/`\hl` are ordinary undefined names: a user's own
//!   `\newcommand` wins exactly as in real LaTeX. Both compose
//!   (`\hl{\so{..}}`, `\so{\hl{..}}`); both need `\usepackage{soul}` for
//!   the built-in behavior and otherwise diagnose while keeping the text.
//! - soul `\st` (strikethrough) is out of scope (issue #330's ulem-side
//!   work) and must keep its exact `unknown_command` error.
use flashtex_compiler::color::{ColorSpace, DeviceColor};
use flashtex_compiler::diagnostics::DiagnosticCode;
use flashtex_compiler::parser::{parse, Block, Inline, SoulHighlightExtents, UnderlineGeom};
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
/// a text piece, `K` a kern, `H(6.50,..)` soul's explicit replacement
/// word-space glue (natural/stretch/shrink points at 10pt), anything else
/// `OTHER(..)`.
fn sig(inlines: &[Inline]) -> Vec<String> {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text {
                text, space_before, ..
            } => format!("T({text},{space_before})"),
            Inline::Kern { .. } => "K".to_string(),
            Inline::HSpace {
                pt,
                stretch_pt,
                shrink_pt,
                ..
            } => format!("H({pt:.2},{stretch_pt:.2},{shrink_pt:.2})"),
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

fn glue_pts(inlines: &[Inline]) -> Vec<f64> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::HSpace { pt, .. } => Some((pt * 100.0).round() / 100.0),
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
        glue_pts(&inlines),
        vec![5.5, 5.5],
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
/// highlight as an underline-style rule BEHIND the text (with `xcolor`
/// loaded): 10pt `word` and `\hl{word}` are both 21.4167pt wide, but the
/// height grows to 1.75ex (7.5347pt, covering the ascenders) and the depth
/// to 0.75ex (3.22914pt below the baseline). Without `xcolor` soul degrades
/// `\hl` to `\ul` geometry, which is why the old prose — and any probe
/// taken without `xcolor` — wrongly claims the height stays the content's
/// own.
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
    // geometry still extends the fragment to the rule depth. One word is
    // one fragment; several words become one fragment each, breakable
    // between them (round-2 finding 2a).
    assert_eq!(
        hl.content.len(),
        1,
        "one underline per fragment: {inlines:?}"
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
///
/// Slice-1 finding 2: the top is exactly soul's 1.75ex, not 0.75em: at 10pt
/// that is -7.5347pt (not -7.5pt), and at 12pt (x-height 5.16667pt) -9.0417pt
/// over 3.875pt of depth.
#[test]
fn soul_highlight_geom_reaches_three_quarters_ex() {
    let ex_10pt_cmr = 4.30554;
    let (top, extra) =
        UnderlineGeom::SoulHighlight.rule_top_and_depth(0.0, 0.0, 2.5, ex_10pt_cmr);
    assert!(
        (extra - 3.22914).abs() < 0.0001,
        "highlight depth is the measured 3.22914pt: got {extra}"
    );
    assert!(
        (top + 7.5347).abs() < 0.001,
        "highlight top is soul's -1.75ex at 10pt: got {top}"
    );
    let ex_12pt_cmr = 5.16667;
    let (top, extra) =
        UnderlineGeom::SoulHighlight.rule_top_and_depth(0.0, 0.0, 3.0, ex_12pt_cmr);
    assert!(
        (top + 9.0417).abs() < 0.001,
        "highlight top is soul's -1.75ex at 12pt: got {top}"
    );
    assert!(
        (extra - 3.875).abs() < 0.001,
        "highlight depth is 0.75ex at 12pt: got {extra}"
    );
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
            "H(6.50,3.25,2.17)",
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
        ("\\so{ab cd}", 2, vec![6.5]),
        ("\\so{a bb ccc}", 3, vec![6.5, 6.5]),
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
            if matches!(pair[1], Inline::HSpace { .. }) {
                assert!(
                    !matches!(pair[0], Inline::Kern { .. }),
                    "no kern before the word space in {body}: {inlines:?}"
                );
            }
            if matches!(pair[0], Inline::HSpace { .. }) {
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
            glue_pts(&inlines),
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
            "H(5.50,2.75,1.83)",
            "T(a,false)",
            "K",
            "T(b,false)",
            "H(5.50,2.75,1.83)",
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
        glue_pts(&inlines),
        vec![5.5, 5.5, 5.5],
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
            glue_pts(&inlines),
            vec![5.5],
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

fn heading_content(source: &str) -> Vec<Inline> {
    parse(source)
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::Heading { content, .. } => Some(content),
            _ => None,
        })
        .unwrap_or_default()
}

/// Slice-1 finding 1: soul's inner word space must survive Core14 layout.
/// `\so{ab cd}` keeps one `.65em` gap (6.5pt at 10pt); `\so{abcd}` keeps one
/// `.25em` kern (2.5pt) where the gap would be. The `d` position differs by
/// exactly `.65em` minus one letterskip kern = 4.0pt. Pre-fix, Core14 drops
/// the `TextGlue` (it never updates `content_end`, so the next
/// `space_before: false` piece rewinds past it) and the difference is -2.5pt.
#[test]
fn so_inner_space_is_kept_by_core14() {
    fn d_minus_a(body: &str) -> f64 {
        let source = soul_doc(body);
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "{body} silent: {:?}",
            parsed.diagnostics
        );
        // Plain `layout` defaults to 12pt regardless of the document class;
        // pin 10pt so the layout size agrees with the parser's 10pt em.
        let constraints = flashtex_compiler::layout::LayoutConstraints {
            font_size_pt: 10.0,
            ..Default::default()
        };
        let pages =
            flashtex_compiler::layout::layout_with_constraints(&parsed.blocks, constraints);
        let items: Vec<_> = pages.iter().flat_map(|p| &p.items).collect();
        let xa = items
            .iter()
            .find(|item| item.text == "a")
            .expect("item a")
            .x_pt;
        let xd = items
            .iter()
            .find(|item| item.text == "d")
            .expect("item d")
            .x_pt;
        xd - xa
    }
    let gap = d_minus_a("\\so{ab cd}") - d_minus_a("\\so{abcd}");
    assert!(
        (gap - 4.0).abs() < 0.05,
        "inner .65em kept (minus one .25em kern): got {gap}"
    );
}

/// Slice-1 finding 1: soul's inner space is replacement glue, not an extra.
/// It must carry finite stretch/shrink (scaled from the natural interword
/// glue) and its span must cover exactly the replaced source space, so the
/// render pipeline — which reads interword gaps from source bytes — finds no
/// natural space beside it (pre-fix it kept the source space AND the `.65em`,
/// setting the line too wide).
#[test]
fn so_inner_glue_replaces_source_space_with_stretch() {
    let source = soul_doc("\\so{ab cd}");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    let space_at = source.find("ab cd").expect("body bytes") + 2;
    let glue: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::HSpace {
                pt,
                space_before_pt,
                space_after_pt,
                span,
                stretch_pt,
                stretch_fil,
                shrink_pt,
                shrink_fil,
            } => Some((
                *pt, *space_before_pt, *space_after_pt, *span, *stretch_pt, *stretch_fil,
                *shrink_pt, *shrink_fil,
            )),
            _ => None,
        })
        .collect();
    assert_eq!(glue.len(), 1, "one inner glue: {inlines:?}");
    let (pt, before, after, span, stretch, stretch_fil, shrink, shrink_fil) = glue[0];
    assert!((pt - 6.5).abs() < 0.001, "natural .65em at 10pt: got {pt}");
    assert_eq!((before, after), (0.0, 0.0), "no natural glue beside it");
    assert!(
        stretch > 0.0 && shrink > 0.0 && stretch_fil == 0 && shrink_fil == 0,
        "finite stretch/shrink: got {stretch}/{shrink}"
    );
    assert_eq!(
        (span.start, span.end),
        (space_at, space_at + 1),
        "glue covers exactly the replaced source space"
    );
}

/// Slice-1 finding 1: the `.55em` edge spaces are replacement glue too, with
/// the outside source spaces' spans.
#[test]
fn so_edge_glue_replaces_outside_spaces() {
    let source = soul_doc("x \\so{ab} y");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    let first_space = source.find("x \\so").expect("leading space") + 1;
    let last_space = source.find("} y").expect("trailing space") + 1;
    let glue: Vec<(f64, usize, usize)> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::HSpace { pt, span, .. } => Some((*pt, span.start, span.end)),
            _ => None,
        })
        .collect();
    assert_eq!(glue.len(), 2, "leading and trailing edge glue: {inlines:?}");
    for (pt, _, _) in &glue {
        assert!((pt - 5.5).abs() < 0.001, "natural .55em at 10pt: got {pt}");
    }
    assert_eq!(
        (glue[0].1, glue[0].2),
        (first_space, first_space + 1),
        "leading glue covers the outside space"
    );
    assert_eq!(
        (glue[1].1, glue[1].2),
        (last_space, last_space + 1),
        "trailing glue covers the outside space"
    );
}

/// Slice-1 finding 8: a trailing space inside the argument is soul's inner
/// space, not lost. `A\so{bc }D` keeps one `.65em` glue with the argument
/// space's span (pre-fix `box_inlines` discarded it: no following inline
/// carried its `space_before`).
#[test]
fn so_trailing_arg_space_becomes_inner_glue() {
    let source = soul_doc("A\\so{bc }D");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    let space_at = source.find("bc }").expect("argument space") + 2;
    let glue: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::HSpace { pt, span, .. } => Some((*pt, *span)),
            _ => None,
        })
        .collect();
    assert_eq!(glue.len(), 1, "the argument space survives: {inlines:?}");
    assert!(
        (glue[0].0 - 6.5).abs() < 0.001,
        "it is soul's .65em inner space: got {:?}",
        glue[0]
    );
    assert_eq!(
        (glue[0].1.start, glue[0].1.end),
        (space_at, space_at + 1),
        "with the argument space's span"
    );
    assert!(
        matches!(inlines.last(), Some(Inline::Text { text, space_before: false, .. }) if text == "D"),
        "D follows with no natural space of its own: {inlines:?}"
    );
}

/// Slice-1 finding 8: a leading space inside the argument likewise survives.
#[test]
fn so_leading_arg_space_becomes_inner_glue() {
    let source = soul_doc("\\so{ bc}");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    let space_at = source.find("{ bc}").expect("argument space") + 1;
    let glue: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::HSpace { pt, span, .. } => Some((*pt, *span)),
            _ => None,
        })
        .collect();
    assert_eq!(glue.len(), 1, "the argument space survives: {inlines:?}");
    assert!(
        (glue[0].0 - 6.5).abs() < 0.001,
        "it is soul's .65em inner space: got {:?}",
        glue[0]
    );
    assert_eq!(
        (glue[0].1.start, glue[0].1.end),
        (space_at, space_at + 1),
        "with the argument space's span"
    );
}

/// Slice-1 finding 8: `\hl{bc }` highlights the argument's trailing space —
/// the highlight content ends with that space's glue (pre-fix the space was
/// discarded before the box was built). Round-2 finding 1 corrects the
/// width to pdflatex's natural interword glue (was soul's `.65em`).
#[test]
fn hl_trailing_arg_space_is_highlighted() {
    let source = soul_doc("\\hl{bc }");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    let space_at = source.find("bc }").expect("argument space") + 2;
    let boxed = inlines.iter().find_map(|inline| match inline {
        Inline::ColorBox(b) => Some(b),
        _ => None,
    });
    let boxed = boxed.expect("highlight box survives");
    let inner = match &boxed.content[0] {
        Inline::Underline(u) => u,
        other => panic!("highlight wraps one underline: {other:?}"),
    };
    let glue = inner
        .content
        .iter()
        .filter_map(|inline| match inline {
            Inline::HSpace { pt, span, .. } => Some((*pt, *span)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        glue.len(),
        1,
        "trailing space inside the highlight: {:?}",
        inner.content
    );
    assert!(
        (glue[0].0 - 10.0 / 3.0).abs() < 0.01,
        "highlighted at the natural interword width: got {:?}",
        glue[0]
    );
    assert_eq!(
        (glue[0].1.start, glue[0].1.end),
        (space_at, space_at + 1),
        "with the argument space's span"
    );
}

/// Round-2 finding 3: `\hl{x}` carries soul's highlight top/overlap
/// extents on the background-paint node. The yellow fill is painted from
/// the `ColorBox` bounds, which cover only the content — without explicit
/// extents the fill never reaches soul's highlight top (-1.75ex, well above
/// the glyphs for `x`). The box now records that top (in ex, resolved
/// font-relatively downstream like the underline geometry) plus soul's
/// side overlap, so the paint path can extend the fill; an ordinary
/// `\colorbox` carries none.
#[test]
fn hl_box_carries_highlight_top_extents() {
    let source = soul_doc("\\hl{x}");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    let boxed = inlines
        .iter()
        .find_map(|inline| match inline {
            Inline::ColorBox(b) => Some(b),
            _ => None,
        })
        .expect("highlight box survives");
    assert_eq!(
        boxed.highlight,
        Some(SoulHighlightExtents {
            top_ex: 1.75,
            side_pt: 0.25,
        }),
        "explicit top/overlap extents on the background-paint node: {inlines:?}"
    );
    // The carried top agrees with the underline geometry: at 10pt cmr
    // (x-height 4.30554pt) the rule top is exactly -1.75ex above the
    // baseline, covering the ascenders the content bounds miss.
    let ex_10pt_cmr = 4.30554;
    let (top, _) =
        UnderlineGeom::SoulHighlight.rule_top_and_depth(0.0, 0.0, 2.5, ex_10pt_cmr);
    let extents = boxed.highlight.expect("highlight extents");
    assert!(
        (top + extents.top_ex * ex_10pt_cmr).abs() < 1e-9,
        "carried top matches the soul geometry: {top} vs -1.75ex"
    );
    // An ordinary xcolor box carries no highlight extents.
    let plain = "\\documentclass{article}\n\\usepackage{xcolor}\n\\begin{document}\n\\colorbox{yellow}{x}\n\\end{document}";
    assert!(
        parse(plain).diagnostics.is_empty(),
        "colorbox silent: {:?}",
        parse(plain).diagnostics
    );
    let plain_box = paragraph_inlines(plain)
        .into_iter()
        .find_map(|inline| match inline {
            Inline::ColorBox(b) => Some(b),
            _ => None,
        })
        .expect("xcolor box survives");
    assert_eq!(
        plain_box.highlight, None,
        "xcolor boxes carry no highlight extents"
    );
}

/// Finding 8 mirror of the trailing-space test: `\hl{ bc}` paints the
/// leading argument space inside the first fragment at the natural width.
#[test]
fn hl_leading_arg_space_is_highlighted() {
    let source = soul_doc("\\hl{ bc}");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    let space_at = source.find("{ bc}").expect("argument space") + 1;
    let boxed = inlines
        .iter()
        .find_map(|inline| match inline {
            Inline::ColorBox(b) => Some(b),
            _ => None,
        })
        .expect("highlight box survives");
    let inner = match &boxed.content[0] {
        Inline::Underline(u) => u,
        other => panic!("highlight wraps one underline: {other:?}"),
    };
    let glue = inner
        .content
        .iter()
        .filter_map(|inline| match inline {
            Inline::HSpace { pt, span, .. } => Some((*pt, *span)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        glue.len(),
        1,
        "leading space inside the highlight: {:?}",
        inner.content
    );
    assert!(
        (glue[0].0 - 10.0 / 3.0).abs() < 0.01,
        "highlighted at the natural interword width: got {:?}",
        glue[0]
    );
    assert_eq!(
        (glue[0].1.start, glue[0].1.end),
        (space_at, space_at + 1),
        "with the argument space's span"
    );
}

/// Slice-1 finding 4: `\section{\hl{word}}` highlights — the flattened
/// heading path (`inlines_from_tokens`) must handle soul like the main
/// stream does (pre-fix it silently emitted plain text).
#[test]
fn flat_section_title_keeps_hl() {
    let source = soul_doc("\\section{\\hl{word}}");
    let diagnostics = parse(&source).diagnostics;
    assert!(
        diagnostics.is_empty(),
        "\\section{{\\hl}} silent: {diagnostics:?}"
    );
    let content = heading_content(&source);
    let boxed = content.iter().find_map(|inline| match inline {
        Inline::ColorBox(b) => Some(b),
        _ => None,
    });
    let boxed = boxed.expect("highlight box in the heading");
    assert_eq!(boxed.fill, yellow(), "soul yellow, not plain text");
    assert_eq!(text_of(&boxed.content), "word");
}

/// Slice-1 finding 4: `\section{\so{ab}}` letterspaces in the heading.
#[test]
fn flat_section_title_keeps_so_spacing() {
    let source = soul_doc("\\section{\\so{ab}}");
    let diagnostics = parse(&source).diagnostics;
    assert!(
        diagnostics.is_empty(),
        "\\section{{\\so}} silent: {diagnostics:?}"
    );
    let content = heading_content(&source);
    let want = TextDimen::parse("0.25em").expect("letterskip parses");
    assert!(
        content
            .iter()
            .any(|inline| matches!(inline, Inline::Kern { amount, .. } if amount == &want)),
        "letterskip kern in the heading: {content:?}"
    );
    assert_eq!(text_of(&content).replace(' ', ""), "ab");
}

/// Slice-1 finding 4: without soul the flattened path diagnoses like the
/// main stream and keeps plain text.
#[test]
fn flat_so_hl_without_soul_diagnose() {
    let source = "\\documentclass{article}\n\\begin{document}\n\\section{\\hl{word} and \\so{ab}}\n\\end{document}";
    let diagnostics = parse(source).diagnostics;
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("\\hl needs \\usepackage{soul}")),
        "hl diagnostic: {diagnostics:?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message.contains("\\so needs \\usepackage{soul}")),
        "so diagnostic: {diagnostics:?}"
    );
    let joined = text_of(&heading_content(source));
    assert!(
        joined.contains("word") && joined.contains("ab"),
        "arguments kept as plain text: {joined:?}"
    );
}

/// Slice-1: a declaration inside an early `\so` segment still applies to
/// later ones — segment boxing threads the text style exactly as boxing the
/// whole argument at once would.
#[test]
fn so_declaration_spans_segments() {
    let source = soul_doc("\\so{\\bfseries a b}");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    let bolds: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } => Some((text.clone(), style.bold)),
            _ => None,
        })
        .collect();
    assert_eq!(
        bolds,
        vec![
            ("a".to_string(), true),
            ("b".to_string(), true),
        ],
        "both segments stay bold: {inlines:?}"
    );
    assert_eq!(glue_pts(&inlines), vec![6.5], "inner glue kept: {inlines:?}");
}

/// Round-2 finding 1: `\hl{a b}` keeps pdflatex's NATURAL interword glue,
/// not soul's `.65em` `\so` replacement space (`\hl{a b}` was ~3.17pt too
/// wide at 10pt: 6.5pt instead of 3.33333pt). The gap between the words is
/// therefore an ordinary breakable space — no explicit `HSpace` at all —
/// while each word keeps its own yellow fragment (round-2 finding 2a).
#[test]
fn hl_multiword_inner_space_is_natural() {
    let source = soul_doc("\\hl{a b}");
    // Round-3 finding 3: the unpainted interword gap warns (one
    // `FidelityNote` per `\\hl`), so this is no longer silent. The fragment
    // structure below is unchanged.
    let diagnostics = parse(&source).diagnostics;
    assert_eq!(
        diagnostics.len(),
        1,
        "one gap warning: {diagnostics:?}"
    );
    assert_eq!(diagnostics[0].code, Some(DiagnosticCode::FidelityNote));
    assert!(
        diagnostics[0].message.contains("2 words")
            && diagnostics[0].message.contains("1 interword gap"),
        "warning names the unpainted gap: {:?}",
        diagnostics[0].message
    );
    let inlines = paragraph_inlines(&source);
    let boxes: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::ColorBox(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(
        boxes.len(),
        2,
        "one highlight fragment per word, breakable between them: {inlines:?}"
    );
    assert_eq!(text_of(&boxes[0].content), "a");
    assert_eq!(text_of(&boxes[1].content), "b");
    assert!(
        boxes[1].space_before,
        "the inter-word gap is natural glue, not explicit: {inlines:?}"
    );
    assert!(
        !inlines.iter().any(|i| matches!(
            i,
            Inline::HSpace { pt, .. } if (pt - 6.5).abs() < 0.01
        )),
        "no soul .65em space anywhere in \\hl: {inlines:?}"
    );
    // Width proof in this layout: the a->b advance inside `\hl{a b}` is
    // exactly the advance of a plain `a b` (pre-fix it was wider by
    // .65em minus one natural space).
    assert!(
        (ab_gap("\\hl{a b}") - ab_gap("a b")).abs() < 0.05,
        "\\hl{{a b}} advances like plain `a b`"
    );
}

/// Advance from the `a` item to the `b` item at 10pt in this layout.
fn ab_gap(body: &str) -> f64 {
    let source = soul_doc(body);
    let parsed = parse(&source);
    // Round-3 finding 3's one-shot gap warning is expected for multi-word
    // `\\hl`; anything else must stay silent.
    let unexpected: Vec<_> = parsed
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            !(diagnostic.code == Some(DiagnosticCode::FidelityNote)
                && diagnostic.message.starts_with("\\hl spans"))
        })
        .collect();
    assert!(
        unexpected.is_empty(),
        "{body} otherwise silent: {:?}",
        parsed.diagnostics
    );
    let constraints = flashtex_compiler::layout::LayoutConstraints {
        font_size_pt: 10.0,
        ..Default::default()
    };
    let pages =
        flashtex_compiler::layout::layout_with_constraints(&parsed.blocks, constraints);
    let items: Vec<_> = pages.iter().flat_map(|p| &p.items).collect();
    let xa = items
        .iter()
        .find(|item| item.text == "a")
        .expect("item a")
        .x_pt;
    let xb = items
        .iter()
        .find(|item| item.text == "b")
        .expect("item b")
        .x_pt;
    xb - xa
}

/// Slice-1: nested styling across a top-level `\so` space keeps working —
/// `\so{a\textbf{b c}d}` boxes each segment with its own groups intact.
#[test]
fn so_nested_group_across_space() {
    let source = soul_doc("\\so{a\\textbf{b c}d}");
    assert!(
        parse(&source).diagnostics.is_empty(),
        "silent: {:?}",
        parse(&source).diagnostics
    );
    let inlines = paragraph_inlines(&source);
    assert_eq!(text_of(&inlines).replace(' ', ""), "abcd");
    assert_eq!(glue_pts(&inlines), vec![6.5], "one inner glue: {inlines:?}");
    let bolds: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::Text { text, style, .. } if style.bold => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(bolds, vec!["b", "c"], "group styling intact: {inlines:?}");
}

fn caption_content(source: &str) -> Vec<Inline> {
    parse(source)
        .blocks
        .into_iter()
        .find_map(|block| match block {
            Block::FigureCaption { content } => Some(content),
            _ => None,
        })
        .unwrap_or_default()
}

/// Round-2 finding 2 (Commander ruling, option (a)): a long `\\hl{...}` is
/// genuinely breakable at the compiler level — one highlight box per word
/// with natural glue between the boxes, so the line breaker can split
/// them. Only the render-pipeline painting of a line-broken highlight
/// (yellow across the line gap) stays a known follow-up.
#[test]
fn hl_long_highlight_breaks_between_word_fragments() {
    let body = "\\hl{aa bb cc dd ee ff}";
    let source = soul_doc(body);
    // Round-3 finding 3: five unpainted gaps, still one diagnostic.
    let diagnostics = parse(&source).diagnostics;
    assert_eq!(
        diagnostics.len(),
        1,
        "one gap warning for five gaps: {diagnostics:?}"
    );
    assert_eq!(diagnostics[0].code, Some(DiagnosticCode::FidelityNote));
    assert!(
        diagnostics[0].message.contains("6 words")
            && diagnostics[0].message.contains("5 interword gaps"),
        "warning names the unpainted gaps: {:?}",
        diagnostics[0].message
    );
    let inlines = paragraph_inlines(&source);
    let boxes: Vec<_> = inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::ColorBox(b) => Some(b),
            _ => None,
        })
        .collect();
    assert_eq!(
        boxes.len(),
        6,
        "one breakable fragment per word, not one overfull box: {inlines:?}"
    );
    for fragment in &boxes[1..] {
        assert!(
            fragment.space_before,
            "natural breakable glue between fragments: {inlines:?}"
        );
    }
    // Behavior: at a narrow measure the fragments wrap onto several lines
    // instead of forming one overfull box.
    let constraints = flashtex_compiler::layout::LayoutConstraints {
        font_size_pt: 10.0,
        measure_pt: 30.0,
        ..Default::default()
    };
    let parsed = parse(&source);
    let pages =
        flashtex_compiler::layout::layout_with_constraints(&parsed.blocks, constraints);
    let mut lines: Vec<f64> = pages
        .iter()
        .flat_map(|p| &p.items)
        .map(|item| item.baseline_y_pt)
        .collect();
    lines.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    lines.dedup();
    assert!(
        lines.len() >= 2,
        "highlight wraps at a narrow measure: {lines:?}"
    );
}

/// Slice-1 finding 4: `\caption{\so{word}}` letterspaces too — captions
/// share the flattened path with headings.
#[test]
fn flat_caption_keeps_so_spacing() {
    let source = soul_doc("\\begin{figure}\\caption{\\so{ab}}\\end{figure}");
    let diagnostics = parse(&source).diagnostics;
    assert!(
        diagnostics.is_empty(),
        "caption \\so silent: {diagnostics:?}"
    );
    let content = caption_content(&source);
    let want = TextDimen::parse("0.25em").expect("letterskip parses");
    assert!(
        content
            .iter()
            .any(|inline| matches!(inline, Inline::Kern { amount, .. } if amount == &want)),
        "letterskip kern in the caption: {content:?}"
    );
}

/// Round-3 finding 1, end to end at 10pt (with `xcolor` loaded): a `\\hl{x}`
/// line carries the oracle depth below the baseline. The next paragraph's
/// first baseline therefore sits 3.22916pt (0.75ex) of descent below a
/// highlight line, versus 2.0pt of nominal descent below a plain `x` line —
/// a 1.22916pt shift. (The 1.75ex top cannot move a line: it is
/// 0.75347em of the fragment's own size, and the line's nominal text
/// ascent is never smaller than that size. It is carried on the box as
/// `SoulHighlightExtents` for the render-pipeline paint path instead,
/// which still paints the highlight box at content height until the
/// vendored compiler is re-pinned — see GH-828.)
#[test]
fn hl_line_carries_oracle_depth_in_layout() {
    fn second_baseline(first: &str) -> f64 {
        let source = soul_doc(&format!("{first}\n\nnext"));
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "{first} silent: {:?}",
            parsed.diagnostics
        );
        let constraints = flashtex_compiler::layout::LayoutConstraints {
            font_size_pt: 10.0,
            ..Default::default()
        };
        let pages =
            flashtex_compiler::layout::layout_with_constraints(&parsed.blocks, constraints);
        pages
            .iter()
            .flat_map(|page| &page.items)
            .find(|item| item.text == "next")
            .expect("second paragraph laid out")
            .baseline_y_pt
    }
    let shift = second_baseline("\\hl{x}") - second_baseline("x");
    assert!(
        (shift - (3.22916 - 2.0)).abs() < 0.05,
        "highlight line is 0.75ex deep vs 2.0pt nominal: shift {shift}"
    );
}

/// Round-3 finding 3 (diagnostic half): the one-shot warning fires exactly
/// once per multi-word `\\hl`, names the unpainted gaps, and stays silent
/// everywhere the fill is complete.
#[test]
fn hl_gap_warning_is_one_shot_and_names_gaps() {
    // Two words, one gap: singular wording, span over the whole `\\hl`.
    let source = soul_doc("Text \\hl{aa bb} here.");
    let parsed = parse(&source);
    assert_eq!(
        parsed.diagnostics.len(),
        1,
        "exactly one warning: {:?}",
        parsed.diagnostics
    );
    let warning = &parsed.diagnostics[0];
    assert_eq!(warning.code, Some(DiagnosticCode::FidelityNote));
    assert!(
        warning.message.contains("2 words")
            && warning.message.contains("the 1 interword gap")
            && warning.message.contains("is left unpainted")
            && warning.message.contains("GH-828"),
        "names the single unpainted gap: {:?}",
        warning.message
    );
    let span = warning.span.expect("warning points at the highlight");
    assert_eq!(
        &source[span.start..span.end],
        "\\hl{aa bb}",
        "span covers the whole multi-word highlight"
    );
    // Three words, two gaps: plural wording, still exactly one warning.
    let source = soul_doc("\\hl{aa bb cc}");
    let diagnostics = parse(&source).diagnostics;
    assert_eq!(
        diagnostics.len(),
        1,
        "one warning for two gaps: {diagnostics:?}"
    );
    assert!(
        diagnostics[0].message.contains("3 words")
            && diagnostics[0].message.contains("2 interword gaps")
            && diagnostics[0].message.contains("are left unpainted"),
        "plural wording: {:?}",
        diagnostics[0].message
    );
    // Complete fills stay silent: a single word, painted argument-edge
    // spaces, and the all-space argument.
    for body in ["\\hl{word}", "\\hl{bc }", "\\hl{ bc}", "\\hl{ }"] {
        let source = soul_doc(body);
        assert!(
            parse(&source).diagnostics.is_empty(),
            "{body} paints fully, so silent: {:?}",
            parse(&source).diagnostics
        );
    }
}

/// Round-3 finding 2: the user-facing inventory records `\\hl`'s real
/// `xcolor` geometry (1.75ex above, 0.75ex below — not the old `\\ul`
/// "0.75ex deeper" claim) and the unpainted-gap limitation.
#[test]
fn hl_inventory_description_records_xcolor_geometry() {
    let inventory = flashtex_compiler::supported::inventory();
    let hl = inventory
        .commands
        .iter()
        .find(|command| command.name == "hl")
        .expect("hl inventory entry");
    assert!(
        hl.description.contains("1.75ex"),
        "height is 1.75ex: {:?}",
        hl.description
    );
    assert!(
        hl.description.contains("0.75ex"),
        "depth is 0.75ex: {:?}",
        hl.description
    );
    assert!(
        !hl.description.contains("same height"),
        "no stale \\ul-geometry claim: {:?}",
        hl.description
    );
    assert!(
        hl.description.contains("not painted") && hl.description.contains("GH-828"),
        "gap limitation recorded: {:?}",
        hl.description
    );
    let soul = inventory
        .packages
        .iter()
        .find(|package| package.name == "soul")
        .expect("soul package entry");
    assert!(
        soul.description.contains("1.75ex") && soul.description.contains("not painted"),
        "package entry agrees: {:?}",
        soul.description
    );
}
