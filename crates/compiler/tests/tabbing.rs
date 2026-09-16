//! GH-TABBING, slice 1: `\=`, `\>`, `\\` and `\kill` in `tabbing`.
//!
//! `tabbing` is a plain LaTeX2e kernel environment (not a package) that
//! aligns rows at dynamically recorded tab stops — genuinely different from
//! `tabular`'s column-spec model. Slice 1 covers the common case: fixed
//! stops set on a first `\kill`-terminated setup row, then content rows
//! using `\>` to jump between them. `\<`, `\+` and `\-` are a follow-up
//! slice (they warn and are ignored here).

use flashtex_compiler::incremental::compile_full;
use flashtex_compiler::layout::{self, Font, LayoutConstraints, TextItem, BODY_SIZE_PT, MARGIN_PT};
use flashtex_compiler::parser::{self, Block, Inline};

/// Setup row (`\kill`) plus two content rows using `\>` jumps.
const SETUP: &str = "\\begin{tabbing}\nLongest \\= middle \\= short \\kill\na \\> b \\> c \\\\\n dd \\> ee \\> ff\n\\end{tabbing}\n";

fn compiled(text: &str) -> flashtex_compiler::incremental::CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

fn items_of(text: &str) -> Vec<TextItem> {
    compiled(text)
        .pages
        .iter()
        .flat_map(|page| page.items.clone())
        .collect()
}

fn item_with<'a>(items: &'a [TextItem], text: &str, occurrence: usize) -> &'a TextItem {
    items
        .iter()
        .filter(|item| item.text == text)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("no {occurrence}-th item {text:?} in {items:#?}"))
}

fn width(text: &str) -> f64 {
    layout::text_width(text, BODY_SIZE_PT, Font::TimesRoman)
}

fn space() -> f64 {
    layout::word_space(BODY_SIZE_PT, Font::TimesRoman)
}

fn close(got: f64, want: f64, what: &str) {
    assert!(
        (got - want).abs() < 0.01,
        "{what}: got {got}, want {want} (delta {})",
        got - want
    );
}

#[test]
fn rows_collect_into_a_tabbing_block_with_no_warning() {
    let parsed = parser::parse(SETUP);
    assert!(
        parsed
            .diagnostics
            .iter()
            .all(|d| !d.message.contains("not implemented")),
        "{:#?}",
        parsed.diagnostics
    );
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    assert_eq!(parsed.blocks.len(), 1);
    let Block::Tabbing { lines, .. } = &parsed.blocks[0] else {
        panic!("expected Block::Tabbing, got {:#?}", parsed.blocks[0]);
    };
    assert_eq!(lines.len(), 3);
    assert_eq!(
        lines.iter().map(|line| line.killed).collect::<Vec<_>>(),
        vec![true, false, false]
    );
    // Setup row: text with two `\=` stops.
    assert_eq!(
        lines[0]
            .content
            .iter()
            .filter(|inline| matches!(inline, Inline::TabStop { .. }))
            .count(),
        2
    );
    // Content rows: two `\>` jumps each.
    for line in &lines[1..] {
        assert_eq!(
            line.content
                .iter()
                .filter(|inline| matches!(inline, Inline::TabJump { .. }))
                .count(),
            2,
            "{line:#?}"
        );
    }
}

#[test]
fn jumps_land_on_the_recorded_stops() {
    let items = items_of(SETUP);
    let stop1 = MARGIN_PT + width("Longest");
    // The setup row sets "Longest middle" with its ordinary interword
    // space, so the second stop sits past that space.
    let stop2 = stop1 + space() + width("middle");
    close(item_with(&items, "b", 0).x_pt, stop1, "first jump");
    close(item_with(&items, "c", 0).x_pt, stop2, "second jump");
    // The second content row jumps to the same stops.
    close(item_with(&items, "ee", 0).x_pt, stop1, "second-row first jump");
    close(item_with(&items, "ff", 0).x_pt, stop2, "second-row second jump");
}

#[test]
fn kill_row_produces_no_output_but_registers_its_stops() {
    let items = items_of(SETUP);
    for killed in ["Longest", "middle", "short"] {
        assert!(
            items.iter().all(|item| item.text != killed),
            "killed row leaked {killed:?} into {items:#?}"
        );
    }
    // The jumps above landing on the stops proves the killed row's `\=`
    // stops registered; here the stops visibly come from nowhere else.
}

#[test]
fn line_break_returns_to_the_left_margin_not_a_stop() {
    let items = items_of(SETUP);
    let a = item_with(&items, "a", 0);
    let dd = item_with(&items, "dd", 0);
    close(a.x_pt, MARGIN_PT, "first row starts at the margin");
    close(dd.x_pt, MARGIN_PT, "row after \\\\ starts at the margin");
    assert!(
        item_with(&items, "b", 0).x_pt > a.x_pt,
        "the jump moved right"
    );
}

#[test]
fn kill_row_takes_no_vertical_space() {
    // The first content row after a killed setup row sits on the first
    // baseline, exactly as if the setup row were not there at all.
    let with_kill = items_of(SETUP);
    let without = items_of("\\begin{tabbing}\na\n\\end{tabbing}\n");
    close(
        item_with(&with_kill, "a", 0).baseline_y_pt,
        item_with(&without, "a", 0).baseline_y_pt,
        "killed setup row takes no space",
    );
}

#[test]
fn live_row_stops_are_visible_to_later_rows() {
    let items = items_of("\\begin{tabbing}\naa \\= bb \\\\\nx \\> y\n\\end{tabbing}\n");
    close(
        item_with(&items, "y", 0).x_pt,
        MARGIN_PT + width("aa"),
        "stop set on a live row",
    );
}

#[test]
fn jump_without_a_stop_warns_and_stays() {
    let out = compiled("\\begin{tabbing}\na \\> b\n\\end{tabbing}\n");
    assert_eq!(out.diagnostics.len(), 1, "{:#?}", out.diagnostics);
    assert!(
        out.diagnostics[0].message.contains("no tab stop"),
        "{:#?}",
        out.diagnostics
    );
    let items: Vec<TextItem> = out
        .pages
        .iter()
        .flat_map(|page| page.items.clone())
        .collect();
    let a = item_with(&items, "a", 0);
    // No stop: `b` follows `a` with ordinary interword spacing, as if the
    // `\>` were not there.
    close(
        item_with(&items, "b", 0).x_pt,
        a.x_pt + width("a") + space(),
        "jump with no stop stays put",
    );
}

#[test]
fn deferred_commands_warn_and_are_ignored() {
    // `\<`, `\+`, `\-` belong to a follow-up slice: they must warn
    // explicitly rather than typesetting as text, and the row still lays
    // out around them.
    let out = compiled("\\begin{tabbing}\na \\< b \\+ c \\- d\n\\end{tabbing}\n");
    assert_eq!(out.diagnostics.len(), 3, "{:#?}", out.diagnostics);
    for (diagnostic, needle) in out.diagnostics.iter().zip(["\\<", "\\+", "\\-"]) {
        assert!(
            diagnostic.message.contains("not implemented yet")
                && diagnostic.message.contains(needle),
            "{diagnostic:#?}"
        );
    }
    let items: Vec<TextItem> = out
        .pages
        .iter()
        .flat_map(|page| page.items.clone())
        .collect();
    let texts: Vec<&str> = items.iter().map(|item| item.text.as_str()).collect();
    assert_eq!(texts, vec!["a", "b", "c", "d"]);
    assert!(
        items.windows(2).all(|pair| pair[0].x_pt < pair[1].x_pt
            && pair[0].baseline_y_pt == pair[1].baseline_y_pt),
        "one laid-out row: {items:#?}"
    );
}

#[test]
fn footnote_in_kill_row_leaves_no_footnote() {
    // The killed row's mark rewinds with its items and its queued text
    // rewinds with the footnote queues: nothing reaches the page.
    let out = compiled(
        "\\begin{tabbing}\nsetup \\= row \\footnote{note} \\kill\na \\> b\n\\end{tabbing}\n",
    );
    let texts: Vec<String> = out
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
        .collect();
    assert!(
        !texts.iter().any(|text| text.contains("note")),
        "killed row leaked a footnote into {texts:?}"
    );
    assert!(
        out.diagnostics.is_empty(),
        "{:#?}",
        out.diagnostics
    );
}

#[test]
fn kill_outside_tabbing_is_still_unsupported() {
    let out = compiled("a \\kill b");
    assert!(
        out.diagnostics.iter().any(|d| d.message.contains("\\kill")),
        "{:#?}",
        out.diagnostics
    );
}

#[test]
fn control_symbols_outside_tabbing_are_unchanged() {
    // `\=`-family symbols keep their ordinary meaning outside `tabbing`:
    // `=` is text and `\>` is a medspace kern (no item, no warning).
    let out = compiled("a = b \\> c");
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    let items: Vec<TextItem> = out
        .pages
        .iter()
        .flat_map(|page| page.items.clone())
        .collect();
    let texts: Vec<&str> = items.iter().map(|item| item.text.as_str()).collect();
    assert_eq!(texts, vec!["a", "=", "b", "c"]);
}
