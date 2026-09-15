//! Regression tests for the old `u8` list-depth counter in
//! `Stylesheet::resolve`, which wrapped past 255 nested levels (panicking in
//! debug builds, aliasing depth 1 in release). Deep nesting must clamp to
//! the deepest defined level instead.
use flashtex_document_style::*;

fn sheet() -> Stylesheet {
    Stylesheet::article(ClassOptions {
        paper: Paper::Letter,
        size: BaseSize::Pt12,
    })
}

fn nested_lists(n: usize) -> Vec<Block> {
    let mut path = vec![Block::Document];
    for _ in 0..n {
        path.push(Block::List(ListKind::Itemize));
    }
    path
}

#[test]
fn extreme_depths_do_not_alias_depth_1() {
    let s = sheet();
    let d1 = s.resolve(&nested_lists(1));
    let l1 = d1.list.expect("depth-1 list style");
    let deepest = list_level(BaseSize::Pt12, MAX_LIST_NESTING_DEPTH as u8);
    for n in [255, 256, 257, 1000] {
        let deep = s.resolve(&nested_lists(n));
        let list = deep.list.expect("deep list style");
        // Clamped to the deepest defined level, never wrapped to depth 1.
        assert_eq!(
            list.depth, MAX_LIST_NESTING_DEPTH as u8,
            "depth field saturates at the max for n={n}"
        );
        assert_eq!(list.leftmargin, deepest.leftmargin, "n={n}");
        assert_eq!(list.labelwidth, deepest.labelwidth, "n={n}");
        assert_eq!(list.topsep, deepest.topsep, "n={n}");
        assert_eq!(list.parsep, deepest.parsep, "n={n}");
        assert_eq!(list.itemsep, deepest.itemsep, "n={n}");
        // The core regression: extreme depth must not look like depth 1.
        assert_ne!(
            (list.depth, list.leftmargin),
            (l1.depth, l1.leftmargin),
            "depth {n} aliases depth 1"
        );
        assert_ne!(
            deep.left_margin, d1.left_margin,
            "cumulative margin at depth {n} aliases depth 1"
        );
    }
}

#[test]
fn clamped_depth_keeps_accumulating_deepest_margin() {
    // Each level past the max still adds the deepest level's own margin,
    // so cumulative indentation keeps growing instead of collapsing.
    let s = sheet();
    let m257 = s.resolve(&nested_lists(257)).left_margin.0;
    let m1000 = s.resolve(&nested_lists(1000)).left_margin.0;
    assert!(
        m1000 > m257,
        "1000 deep: {m1000} should exceed 257 deep: {m257}"
    );
}

#[test]
fn try_resolve_ok_at_and_under_max() {
    let s = sheet();
    for n in 0..=MAX_LIST_NESTING_DEPTH {
        let got = s.try_resolve(&nested_lists(n));
        assert!(got.is_ok(), "depth {n} should resolve");
        assert_eq!(got.unwrap(), s.resolve(&nested_lists(n)));
    }
}

#[test]
fn try_resolve_err_past_max() {
    let s = sheet();
    for n in [MAX_LIST_NESTING_DEPTH + 1, 100, 256, 257, 1000] {
        match s.try_resolve(&nested_lists(n)) {
            Err(e) => assert_eq!(e.depth, n, "error reports requested depth"),
            Ok(_) => panic!("depth {n} should exceed the max"),
        }
    }
}

#[test]
fn try_resolve_counts_alignments_as_nesting() {
    let s = sheet();
    let mut path = vec![Block::Document];
    for _ in 0..=MAX_LIST_NESTING_DEPTH {
        path.push(Block::Align(Alignment::Center));
    }
    let depth = MAX_LIST_NESTING_DEPTH + 1;
    assert_eq!(
        s.try_resolve(&path),
        Err(ListNestingTooDeep { depth }),
        "trivlist alignments count toward the limit"
    );
    // Mixed list/alignment nesting counts together.
    let mut mixed = vec![Block::Document];
    for i in 0..depth {
        if i % 2 == 0 {
            mixed.push(Block::List(ListKind::Enumerate));
        } else {
            mixed.push(Block::Align(Alignment::Left));
        }
    }
    assert_eq!(
        s.try_resolve(&mixed),
        Err(ListNestingTooDeep { depth }),
        "mixed nesting counts together"
    );
    // But resolve still clamps instead of failing: the mixed path ends in
    // a list whose per-level style saturates at the deepest level.
    let clamped = s.resolve(&mixed);
    assert_eq!(
        clamped.list.expect("list style").depth,
        MAX_LIST_NESTING_DEPTH as u8
    );
}
