//! GH-497 (GH-TEXTSUPERSCRIPT): `\textsuperscript{text}` and
//! `\textsubscript{text}` errored `unsupported_feature` in text mode.
//! Both are kernel text commands (latex.ltx `ltmisc.dtx`
//! `\@textsuperscript` / `\@textsubscript`): the argument is set at the
//! `\sf@size` of the current size, raised or lowered like a math script
//! of an empty nucleus.
use flashtex_compiler::layout::{layout, layout_with_constraints, LayoutConstraints, Page, TextItem};
use flashtex_compiler::parser::{parse, Block, Inline};

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
            Inline::TextScript(t) => out.push_str(&text_of(&t.content)),
            _ => {}
        }
    }
    out
}

fn scripts(inlines: &[Inline]) -> Vec<(bool, String)> {
    inlines
        .iter()
        .filter_map(|inline| match inline {
            Inline::TextScript(t) => Some((t.superscript, text_of(&t.content))),
            _ => None,
        })
        .collect()
}

/// Issue #497: `E=mc\textsuperscript{2} H\textsubscript{2}O` reported both
/// commands as `unsupported_feature`. Both are kernel commands now.
#[test]
fn text_scripts_typeset_with_no_diagnostics() {
    let source = "E=mc\\textsuperscript{2} H\\textsubscript{2}O";
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "text scripts should not diagnose: {:?}",
        parsed.diagnostics
    );
    let found = scripts(&paragraph_inlines(source));
    assert_eq!(
        found,
        vec![(true, "2".to_string()), (false, "2".to_string())],
        "both commands must parse into raised/lowered wrappers: {found:?}"
    );
}

/// The wrapper keeps its argument visible and the baseline returns to
/// normal afterward: only the argument is wrapped, surrounding text is
/// ordinary `Inline::Text`.
#[test]
fn text_script_wraps_only_its_argument() {
    let source = "a\\textsuperscript{b}c";
    let inlines = paragraph_inlines(source);
    let kinds: Vec<&str> = inlines
        .iter()
        .map(|inline| match inline {
            Inline::TextScript(t) if t.superscript => "super",
            Inline::TextScript(_) => "sub",
            Inline::Text { .. } => "text",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, vec!["text", "super", "text"]);
    assert_eq!(text_of(&inlines), "abc");
}

/// Oracle geometry for the Core 14 layout (pdflatex `\showbox`, GH-507
/// review): running-text scripts are TEXT style at the `\sf@size` of the
/// local size — superscript raised by `sup2` (cmsy10 fontdimen 14,
/// 0.362892em), subscript dropped by `sub1` (fontdimen 16, 0.15em), each
/// times the local size. `ORACLE_TOL` covers the layout's `round2` baseline
/// quantisation, nothing more.
const ORACLE_TOL: f64 = 0.011;
const SUP2_EM: f64 = 0.362892;
const SUB1_EM: f64 = 0.15;

fn laid_out_at(source: &str, body_pt: f64) -> Vec<TextItem> {
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "text scripts should not diagnose: {:?}",
        parsed.diagnostics
    );
    let constraints = LayoutConstraints {
        font_size_pt: body_pt,
        ..LayoutConstraints::default()
    };
    let pages = layout_with_constraints(&parsed.blocks, constraints);
    pages.iter().flat_map(|p| p.items.clone()).collect()
}

fn laid_out(source: &str) -> Vec<TextItem> {
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "text scripts should not diagnose: {:?}",
        parsed.diagnostics
    );
    let pages = layout(&parsed.blocks);
    pages.iter().flat_map(|p| p.items.clone()).collect()
}

fn item<'a>(items: &'a [TextItem], text: &str) -> &'a TextItem {
    items
        .iter()
        .find(|i| i.text == text)
        .unwrap_or_else(|| panic!("expected item {text:?}"))
}

fn assert_near(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < ORACLE_TOL,
        "{what}: got {actual}, want oracle {expected}"
    );
}

/// Body-size oracle row (12pt class): `sup2 × 12 = 4.3547` up,
/// `sub1 × 12 = 1.8` down, both at `\sf@size` 8pt (pdflatex: `-4.3547` /
/// `1.79999`). The old code raised by display-style `sup1` (4.95) and
/// mirrored it for subscripts (4.95 down) — this test fails on both.
#[test]
fn text_scripts_match_text_style_oracle_at_body_size() {
    let items = laid_out("a\\textsuperscript{b}c H\\textsubscript{2}O");
    let (body, raised, lowered) = (
        item(&items, "a").baseline_y_pt,
        item(&items, "b").baseline_y_pt,
        item(&items, "2").baseline_y_pt,
    );
    assert_near(body - raised, SUP2_EM * 12.0, "superscript raise");
    assert_near(lowered - body, SUB1_EM * 12.0, "subscript drop");
    assert_eq!(item(&items, "b").font_size_pt, 8.0);
    assert_eq!(item(&items, "2").font_size_pt, 8.0);
    assert_eq!(
        item(&items, "c").baseline_y_pt,
        body,
        "text after the superscript returns to the baseline"
    );
    assert_eq!(
        item(&items, "c").font_size_pt,
        12.0,
        "text after the superscript returns to the body size"
    );
}

/// 10pt-class oracle row (pdflatex `\showbox`: `-3.62892` / `1.49998`).
#[test]
fn text_scripts_match_text_style_oracle_at_10pt() {
    let items = laid_out_at("a\\textsuperscript{b}c H\\textsubscript{2}O", 10.0);
    let body = item(&items, "a").baseline_y_pt;
    assert_near(
        body - item(&items, "b").baseline_y_pt,
        SUP2_EM * 10.0,
        "raise@10pt",
    );
    assert_near(
        item(&items, "2").baseline_y_pt - item(&items, "H").baseline_y_pt,
        SUB1_EM * 10.0,
        "drop@10pt",
    );
    assert_eq!(item(&items, "b").font_size_pt, 7.0);
    assert_eq!(item(&items, "2").font_size_pt, 7.0);
}

/// Tall-subscript branch (TeXbook Appendix G rule 18): the drop is
/// `max(sub1, h − ⅘·x-height)`, not a fixed `sub1`. An explicit declaration
/// inside the argument still takes effect (`\mbox` semantics), so
/// `\textsubscript{\large (g)}` at 10pt sets a 12pt box: with this layout's
/// own Times metrics (ascender 0.683em, x-height 0.45em) the drop is
/// `0.683·12 − 0.8·0.45·10 = 4.596`, well past `sub1·10 = 1.5`.
///
/// NOTE on the GH-507 oracle (`shifted 1.80557` for `x\textsubscript{(g)}`
/// at 10pt): that value is Computer Modern's — CM paren ink reaches 0.75em
/// of the 7pt mark size while its x-height is only 0.4306em of the local
/// 10pt, so the tall branch governs there. Times paren ink (0.683em) is
/// shorter against its fatter x-height (0.45em), so a bare `(g)` stays on
/// `sub1` here (pinned below); the tall branch needs a genuinely tall box.
#[test]
fn tall_subscript_uses_box_height_over_sub1() {
    let items = laid_out_at("x\\textsubscript{\\large (g)}", 10.0);
    let body = item(&items, "x").baseline_y_pt;
    let tall = item(&items, "(g)");
    assert_eq!(tall.font_size_pt, 12.0, "inner declaration still applies");
    let drop = tall.baseline_y_pt - body;
    assert!(
        drop > SUB1_EM * 10.0,
        "tall box must drop past sub1, got {drop}"
    );
    assert_near(drop, 0.683 * 12.0 - 0.8 * 0.45 * 10.0, "h − ⅘·x-height");
}

/// ...while a bare `(g)` is short in Times metrics and stays on `sub1`.
#[test]
fn short_subscript_stays_on_sub1() {
    let items = laid_out_at("x\\textsubscript{(g)}", 10.0);
    assert_eq!(item(&items, "(g)").font_size_pt, 7.0);
    assert_near(
        item(&items, "(g)").baseline_y_pt - item(&items, "x").baseline_y_pt,
        SUB1_EM * 10.0,
        "short subscript drop",
    );
}

/// A local size declaration drives both the script size and the shifts
/// (GH-507 finding 3): `{\large ...}` at the 12pt class means local 14.4pt,
/// whose `\sf@size` is 10pt with `sup2·14.4 = 5.2257` / `sub1·14.4 = 2.16`.
/// The old code set `b` at 14.4pt (the declaration leaking through the
/// `\mbox`) with body-size shifts — this test fails on both halves.
#[test]
fn local_size_declaration_drives_script_size_and_shifts() {
    let items = laid_out("{\\large a\\textsuperscript{b}c}");
    let local = item(&items, "a").baseline_y_pt;
    assert_eq!(item(&items, "a").font_size_pt, 14.4);
    assert_eq!(item(&items, "b").font_size_pt, 10.0, "\\sf@size of 14.4");
    assert_near(
        local - item(&items, "b").baseline_y_pt,
        SUP2_EM * 14.4,
        "raise@large",
    );
    assert_eq!(item(&items, "c").baseline_y_pt, local);
    assert_eq!(item(&items, "c").font_size_pt, 14.4);

    let items = laid_out("{\\large x\\textsubscript{2}}");
    let local = item(&items, "x").baseline_y_pt;
    assert_eq!(item(&items, "2").font_size_pt, 10.0, "\\sf@size of 14.4");
    assert_near(
        item(&items, "2").baseline_y_pt - local,
        SUB1_EM * 14.4,
        "drop@large",
    );
}

/// The shared constant (GH-507 finding 2): `\footnotemark` is
/// `\@makefnmark`, i.e. a `\textsuperscript`, so the running-text mark sits
/// at `sup2` too — the `sup1`→`sup2` fix must move marks down to it, not
/// regress them.
#[test]
fn footnote_mark_uses_text_style_sup2() {
    let items = laid_out("w\\footnote{N} after");
    let mark = items
        .iter()
        .find(|i| i.text == "1" && i.font_size_pt == 8.0)
        .expect("running-text footnote mark at \\sf@size");
    assert_near(
        item(&items, "w").baseline_y_pt - mark.baseline_y_pt,
        SUP2_EM * 12.0,
        "footnote mark raise",
    );
}
