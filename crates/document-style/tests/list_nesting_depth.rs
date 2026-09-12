//! Regression tests for GH#44: `Stylesheet::resolve`'s `list_depth: u8`
//! counter used to overflow at 256+ nested `List`/`Align` blocks -- a debug
//! build panicked ("attempt to add with overflow"), and a release build
//! silently wrapped, aliasing a depth-256 list's margin to a depth-0 list's
//! margin. `resolve` now returns a typed [`ListNestingTooDeep`] error once
//! nesting exceeds [`MAX_LIST_NESTING_DEPTH`], well before the counter could
//! ever reach the point where a `u8` would wrap.
//!
//! These tests must pass under both `cargo test` and `cargo test --release`,
//! since the original defect was precisely a case where the two profiles
//! silently disagreed (panic vs. wraparound).

use flashtex_document_style::{
    BaseSize, Block, ClassOptions, ListKind, ListNestingTooDeep, MAX_LIST_NESTING_DEPTH, Paper,
    Stylesheet,
};

fn sheet() -> Stylesheet {
    Stylesheet::article(ClassOptions {
        paper: Paper::Letter,
        size: BaseSize::Pt10,
    })
}

fn nested_list_path(depth: usize) -> Vec<Block> {
    let mut path = vec![Block::Document];
    path.extend(std::iter::repeat_n(Block::List(ListKind::Itemize), depth));
    path
}

/// Exactly at the bound: resolves cleanly, with the correct (unaliased)
/// reported depth.
#[test]
fn resolve_accepts_exactly_max_list_nesting_depth() {
    let s = sheet();
    let path = nested_list_path(MAX_LIST_NESTING_DEPTH as usize);
    let resolved = s.resolve(&path).expect("depth == bound must succeed");
    let list = resolved.list.expect("innermost block is a List");
    assert_eq!(list.depth, MAX_LIST_NESTING_DEPTH);
}

/// One past the bound: a typed error, not a panic and not a wrapped value.
#[test]
fn resolve_rejects_one_past_max_list_nesting_depth_with_typed_error() {
    let s = sheet();
    let path = nested_list_path(MAX_LIST_NESTING_DEPTH as usize + 1);
    let err = s
        .resolve(&path)
        .expect_err("depth > bound must be rejected");
    assert_eq!(
        err,
        ListNestingTooDeep {
            max: MAX_LIST_NESTING_DEPTH
        }
    );
}

/// The exact reproduction from GH#44: a 256-deep path used to overflow the
/// old `u8` counter (debug panic; release silent wraparound to a depth-0
/// alias). It must now be a typed error, identically in debug and release,
/// long before any counter overflow could occur.
#[test]
fn resolve_rejects_the_gh44_256_deep_reproduction() {
    let s = sheet();
    let path = nested_list_path(256);
    assert_eq!(
        s.resolve(&path),
        Err(ListNestingTooDeep {
            max: MAX_LIST_NESTING_DEPTH
        })
    );
}

/// Sibling (breadth) lists must not falsely trip the depth bound -- each
/// `resolve` call starts a fresh counter, and within one call the counter
/// must track current nesting, not a running total across siblings.
#[test]
fn resolve_accepts_many_sibling_calls_past_max_list_nesting_depth() {
    let s = sheet();
    for _ in 0..(MAX_LIST_NESTING_DEPTH as usize) * 3 {
        let path = vec![Block::Document, Block::List(ListKind::Itemize)];
        assert!(s.resolve(&path).is_ok());
    }
}
