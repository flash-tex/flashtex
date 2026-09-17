//! `\shoveleft`/`\shoveright` in `multline`: the override moves that one row
//! flush against its margin, leaves the other rows on the display default,
//! and reports no `unknown_command` diagnostic.
use flashtex_compiler::layout::{MARGIN_PT, layout};
use flashtex_compiler::parser::parse;
use std::collections::BTreeMap;

fn doc(body: &str) -> String {
    format!("\\documentclass{{article}}\\begin{{document}}{body}\\end{{document}}")
}

fn diagnostics(body: &str) -> Vec<String> {
    parse(&doc(body))
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

/// Minimum content x per display row, top to bottom. The equation number
/// shares its row's baseline but sits at the right margin, so the row
/// minimum is always display content, never the number.
fn row_min_x(body: &str) -> Vec<f64> {
    let parsed = parse(&doc(body));
    assert!(
        parsed.diagnostics.is_empty(),
        "{body}: {:?}",
        parsed.diagnostics
    );
    let pages = layout(&parsed.blocks);
    let items: Vec<_> = pages.iter().flat_map(|p| p.items.iter()).collect();
    assert!(!items.is_empty(), "{body} laid out nothing");
    let mut rows: BTreeMap<i64, f64> = BTreeMap::new();
    for item in items {
        // Baselines are rounded to 2dp at layout, so the scaled key is exact.
        let key = (item.baseline_y_pt * 100.0).round() as i64;
        let min = rows.entry(key).or_insert(f64::INFINITY);
        *min = min.min(item.x_pt);
    }
    rows.into_values().collect()
}

const ROWS: &str = "\\begin{multline} a \\\\ b + c + d \\\\ e = f \\end{multline}";

#[test]
fn shove_commands_are_not_unknown_in_multline() {
    for body in [
        "\\begin{multline} \\shoveleft{a} \\\\ b \\end{multline}",
        "\\begin{multline} a \\\\ \\shoveright{b} \\end{multline}",
        "\\begin{multline*} \\shoveleft{a} \\\\ b \\end{multline*}",
    ] {
        let found = diagnostics(body);
        assert!(
            !found.iter().any(|m| m.contains("shove")),
            "{body} must not report a shove gap: {found:?}"
        );
    }
}

#[test]
fn shoveleft_moves_only_its_row_flush_left() {
    let plain = row_min_x(ROWS);
    assert_eq!(plain.len(), 3, "{plain:?}");
    let shoved = row_min_x("\\begin{multline} \\shoveleft{a} \\\\ b + c + d \\\\ e = f \\end{multline}");
    assert_eq!(shoved.len(), 3, "{shoved:?}");
    // The narrow first row centres well inside the measure; shoved, it sits
    // against the left margin.
    assert!(
        shoved[0] < plain[0] - 10.0,
        "shoved {shoved:?} vs default {plain:?}"
    );
    assert_eq!(shoved[0], MARGIN_PT, "{shoved:?}");
    // The other rows keep the default placement exactly.
    assert_eq!(shoved[1], plain[1], "{shoved:?} vs {plain:?}");
    assert_eq!(shoved[2], plain[2], "{shoved:?} vs {plain:?}");
}

#[test]
fn shoveright_moves_only_its_row_flush_right() {
    let plain = row_min_x(ROWS);
    let shoved = row_min_x("\\begin{multline} a \\\\ b + c + d \\\\ \\shoveright{e = f} \\end{multline}");
    assert_eq!(shoved.len(), 3, "{shoved:?}");
    // A shoved-right row starts further right than its centred default...
    assert!(
        shoved[2] > plain[2] + 10.0,
        "shoved {shoved:?} vs default {plain:?}"
    );
    // ...while the other rows keep the default placement exactly.
    assert_eq!(shoved[0], plain[0], "{shoved:?} vs {plain:?}");
    assert_eq!(shoved[1], plain[1], "{shoved:?} vs {plain:?}");
}

#[test]
fn shove_content_still_typesets() {
    // The override's braced content stays in place as an ordinary group: the
    // row keeps all of its glyphs, only their x-position changes.
    let parsed = parse(&doc(
        "\\begin{multline} \\shoveleft{a + b} \\\\ c \\end{multline}",
    ));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let texts: String = layout(&parsed.blocks)
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|item| item.text.as_str())
        .collect();
    for glyph in ["a", "+", "b", "c"] {
        assert!(texts.contains(glyph), "lost {glyph}: {texts:?}");
    }
}
