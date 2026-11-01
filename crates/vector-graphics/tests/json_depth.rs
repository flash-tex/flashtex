//! GH46: nesting depth is bounded by explicit, typed limits instead of the OS
//! thread stack. Every test here ran to a `stack overflow, aborting` (exit 134)
//! or relied on luck before the counters existed.

use flashtex_vector_graphics::Size;
use flashtex_vector_graphics::display_list::{DisplayList, MAX_GROUP_DEPTH, ValidationError};
use flashtex_vector_graphics::item::{Group, Item, ItemId};
use flashtex_vector_graphics::json::{self, JsonError, MAX_JSON_DEPTH, Value};
use std::time::{Duration, Instant};

fn nested_arrays(depth: usize) -> String {
    let mut s = String::with_capacity(depth * 2);
    s.extend(std::iter::repeat_n('[', depth));
    s.extend(std::iter::repeat_n(']', depth));
    s
}

fn nested_objects(depth: usize) -> String {
    let mut s = String::with_capacity(depth * 6 + 4);
    for _ in 0..depth - 1 {
        s.push_str("{\"a\":");
    }
    s.push_str("{}");
    s.extend(std::iter::repeat_n('}', depth - 1));
    s
}

/// Alternating `[{"a":[{"a":…` so both container kinds share one counter.
fn nested_mixed(depth: usize) -> String {
    let mut s = String::new();
    let mut closers = String::new();
    for level in 0..depth {
        if level % 2 == 0 {
            s.push('[');
            closers.insert(0, ']');
        } else {
            s.push_str("{\"a\":");
            closers.insert(0, '}');
        }
    }
    s.push_str("null");
    s.push_str(&closers);
    s
}

/// A well-formed display-list document whose `items` hold `groups` nested
/// groups (each one level deeper than the last) ending in an empty group.
fn nested_groups_doc(groups: usize) -> String {
    let mut s = String::from(
        "{\"format\":\"flashtex-display-list\",\"version\":0,\"page_size\":{\"width_pt\":10,\"height_pt\":10},\"items\":",
    );
    for id in 0..groups {
        s.push_str(&format!(
            "[{{\"kind\":\"group\",\"id\":{id},\"transform\":[1,0,0,1,0,0],\"clip\":null,\"opacity\":1,\"items\":"
        ));
    }
    s.push_str("[]");
    for _ in 0..groups {
        s.push_str("}]");
    }
    s.push('}');
    s
}

/// Builds the same tree in memory, iteratively, so the test itself never recurses.
fn nested_groups_list(groups: usize) -> DisplayList {
    let mut list = DisplayList::new(Size::new(10.0, 10.0));
    let mut innermost: Option<Item> = None;
    for id in (0..groups).rev() {
        let mut g = Group::new(ItemId(id as u64));
        if let Some(child) = innermost.take() {
            g.items.push(child);
        }
        innermost = Some(Item::Group(g));
    }
    if let Some(root) = innermost {
        list.items.push(root);
    }
    list
}

fn max_depth(v: &Value) -> usize {
    // Iterative so the test does not depend on the stack either.
    let mut stack = vec![(v, 1usize)];
    let mut best = 0;
    while let Some((v, d)) = stack.pop() {
        match v {
            Value::Array(a) => {
                best = best.max(d);
                stack.extend(a.iter().map(|c| (c, d + 1)));
            }
            Value::Object(o) => {
                best = best.max(d);
                stack.extend(o.iter().map(|(_, c)| (c, d + 1)));
            }
            _ => {}
        }
    }
    best
}

#[test]
fn limits_are_documented_and_consistent() {
    assert_eq!(MAX_JSON_DEPTH, 256);
    assert_eq!(MAX_GROUP_DEPTH, 64);
    // Every group costs two raw levels (object + `items`), plus two for the
    // document root; the raw limit must leave room for the deepest legal tree.
    const { assert!(MAX_GROUP_DEPTH * 2 + 2 < MAX_JSON_DEPTH) }
}

#[test]
fn nested_arrays_at_limit_parse() {
    let v = json::parse(&nested_arrays(MAX_JSON_DEPTH)).expect("at limit parses");
    assert_eq!(max_depth(&v), MAX_JSON_DEPTH);
}

#[test]
fn nested_arrays_over_limit_return_typed_error() {
    let err = json::parse(&nested_arrays(MAX_JSON_DEPTH + 1)).unwrap_err();
    assert_eq!(
        err,
        JsonError::NestingTooDeep {
            depth: MAX_JSON_DEPTH + 1,
            limit: MAX_JSON_DEPTH
        }
    );
    assert_eq!(
        err.to_string(),
        format!(
            "nesting depth {} exceeds the limit of {}",
            MAX_JSON_DEPTH + 1,
            MAX_JSON_DEPTH
        )
    );
}

#[test]
fn nested_objects_at_limit_and_over() {
    let v = json::parse(&nested_objects(MAX_JSON_DEPTH)).expect("at limit parses");
    assert_eq!(max_depth(&v), MAX_JSON_DEPTH);
    assert_eq!(
        json::parse(&nested_objects(MAX_JSON_DEPTH + 1)).unwrap_err(),
        JsonError::NestingTooDeep {
            depth: MAX_JSON_DEPTH + 1,
            limit: MAX_JSON_DEPTH
        }
    );
}

#[test]
fn arrays_and_objects_share_one_depth_counter() {
    assert!(json::parse(&nested_mixed(MAX_JSON_DEPTH)).is_ok());
    assert!(matches!(
        json::parse(&nested_mixed(MAX_JSON_DEPTH + 1)),
        Err(JsonError::NestingTooDeep { .. })
    ));
}

#[test]
fn depth_counter_resets_between_siblings() {
    // Two siblings each at the limit are fine: depth is nesting, not a total.
    let inner = nested_arrays(MAX_JSON_DEPTH - 1);
    let doc = format!("[{inner},{inner}]");
    assert!(json::parse(&doc).is_ok());
}

#[test]
fn nested_groups_at_limit_read_and_validate() {
    let list =
        json::read_display_list(&nested_groups_doc(MAX_GROUP_DEPTH)).expect("at limit reads");
    assert!(list.validate().is_empty());
    assert_eq!(list, nested_groups_list(MAX_GROUP_DEPTH));
    // Writing it back stays within the raw limit and round-trips.
    let text = json::write_display_list(&list);
    assert_eq!(json::read_display_list(&text).unwrap(), list);
}

#[test]
fn nested_groups_over_limit_return_typed_error() {
    let err = json::read_display_list(&nested_groups_doc(MAX_GROUP_DEPTH + 1)).unwrap_err();
    assert_eq!(
        err,
        JsonError::NestingTooDeep {
            depth: MAX_GROUP_DEPTH + 1,
            limit: MAX_GROUP_DEPTH
        }
    );
    // The document itself is well within the raw JSON limit, so this is the
    // schema reader's own bound, not the parser's.
    assert!(json::parse(&nested_groups_doc(MAX_GROUP_DEPTH + 1)).is_ok());
}

#[test]
fn validate_reports_in_memory_trees_over_the_group_limit() {
    let ok = nested_groups_list(MAX_GROUP_DEPTH);
    assert!(ok.validate().is_empty());
    let deep = nested_groups_list(MAX_GROUP_DEPTH + 1);
    let errors = deep.validate();
    assert_eq!(
        errors,
        vec![ValidationError::NestingTooDeep {
            id: ItemId(MAX_GROUP_DEPTH as u64),
            depth: MAX_GROUP_DEPTH + 1,
            limit: MAX_GROUP_DEPTH,
        }]
    );
    assert_eq!(
        errors[0].to_string(),
        format!(
            "item {} is nested {} groups deep, over the limit of {}",
            MAX_GROUP_DEPTH,
            MAX_GROUP_DEPTH + 1,
            MAX_GROUP_DEPTH
        )
    );
}

#[test]
fn hundred_thousand_deep_arrays_return_error_in_bounded_time() {
    // GH46's reproduction aborted the process at 50 000; 100 000 is 2x that.
    let doc = nested_arrays(100_000);
    let start = Instant::now();
    let err = json::parse(&doc).unwrap_err();
    let elapsed = start.elapsed();
    assert!(matches!(err, JsonError::NestingTooDeep { limit, .. } if limit == MAX_JSON_DEPTH));
    // The parser stops at the 257th bracket; even a heavily loaded machine
    // finishes far inside this bound.
    assert!(elapsed < Duration::from_secs(2), "took {elapsed:?}");
}

#[test]
fn hundred_thousand_deep_groups_return_error_in_bounded_time() {
    let doc = nested_groups_doc(100_000);
    let start = Instant::now();
    let err = json::read_display_list(&doc).unwrap_err();
    let elapsed = start.elapsed();
    assert!(matches!(err, JsonError::NestingTooDeep { .. }), "{err}");
    assert!(elapsed < Duration::from_secs(2), "took {elapsed:?}");
}

#[test]
fn unterminated_deep_input_is_rejected_by_depth_not_end_of_input() {
    let mut s = String::new();
    s.extend(std::iter::repeat_n('[', MAX_JSON_DEPTH + 1));
    assert!(matches!(
        json::parse(&s),
        Err(JsonError::NestingTooDeep { .. })
    ));
}
