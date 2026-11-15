//! The public behaviour of `memstat` that needs no feature: the tag
//! scopes, the tag names, the JSON members, `rss` and the `mincore`
//! residency. (The counting allocator itself needs `mem-stats`.)

use flashtex_runtime::memstat;

/// `scope` marks the thread's allocations until the guard drops; scopes
/// nest, and each puts the outer state back.
#[test]
fn scope_marks_log_allocations_and_restores() {
    assert!(!memstat::in_log_scope());
    let a = memstat::scope(memstat::tag::LOG);
    assert!(memstat::in_log_scope());
    {
        let _b = memstat::scope(memstat::tag::ENGINE);
        assert!(!memstat::in_log_scope());
    }
    assert!(memstat::in_log_scope());
    drop(a);
    assert!(!memstat::in_log_scope());
}
