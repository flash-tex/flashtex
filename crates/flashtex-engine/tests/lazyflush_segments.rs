//! LAZY-FLUSH (`changes/lazyflush.ch`, `src/lazyflush.rs`): the free list
//! of one-word nodes as a stack of segments, checked against pdftex.web's
//! own free list (sections 120-123, 138-141), which is modelled here as a
//! plain sequence: `get_avail` takes its first node (or `hi_mem_min-1` when
//! it is empty) and `flush_list` puts a whole list in front of it.
//!
//! The case this file is for is the fallback: once `seg_size` segments are
//! pending, `flush_list` walks the list and links its last node to `avail`,
//! as pdftex.web always does. Real documents rarely get there (every
//! allocation that empties a segment pops one), so the test builds the
//! state on purpose: many lists of two or more nodes freed in a row, each
//! while `avail` is not empty, so that every one pushes a segment. It
//! checks that the stack was full and that the fallback ran, and after
//! every operation that the free list (`avail`'s chain, then each pending
//! segment from the top) is pdftex.web's, node for node; then that every
//! allocation returns pdftex.web's node, that `lf_dyn_used` is pdftex.web's
//! running count, and that `flashtex_merge_free` leaves exactly the words
//! the walking `flush_list` would have left. A fallback that dropped,
//! reordered or lost a node, or pushed past the stack, fails it.

use flashtex_engine::generated::consts::{mem_max, null, seg_size};
use flashtex_engine::Globals;
use std::collections::VecDeque;

/// pdftex.web's free list and `dyn_used`, as a sequence and a count.
struct Model {
    free: VecDeque<i32>, // free[0] is what `get_avail` returns next
    hi_mem_min: i32,
    dyn_used: i32,
}

impl Model {
    fn alloc(&mut self) -> i32 {
        self.dyn_used += 1;
        if self.free.is_empty() {
            self.hi_mem_min -= 1;
            self.hi_mem_min
        } else {
            self.free.pop_front().unwrap()
        }
    }
    fn flush(&mut self, list: &[i32]) {
        self.dyn_used -= list.len() as i32;
        for &p in list.iter().rev() {
            self.free.push_front(p);
        }
    }
}

fn link(g: &Globals, p: i32) -> i32 {
    g.mem[p as usize].hh().rh()
}

/// The engine's free list as one sequence: `avail`'s chain, then the
/// pending segments from the top of the stack down (each ends in null).
fn free_seq(g: &Globals) -> Vec<i32> {
    let mut v = vec![];
    let mut walk = |mut p: i32| {
        while p != null {
            v.push(p);
            p = link(g, p);
        }
    };
    walk(g.avail);
    for k in (0..g.seg_ptr.max(0)).rev() {
        walk(g.seg_stack[k as usize]);
    }
    v
}

/// A fresh word space as pdftex.web's initialisation leaves hi memory:
/// nothing free, every one-word node still untouched below `hi_mem_min`.
fn fresh() -> (Box<Globals>, Model) {
    let mut g = Globals::new();
    g.mem_end = mem_max;
    g.hi_mem_min = mem_max - 14;
    g.lo_mem_max = 1000;
    g.avail = null;
    g.seg_ptr = 0;
    let m = Model {
        free: VecDeque::new(),
        hi_mem_min: g.hi_mem_min,
        dyn_used: 15,
    };
    (g, m)
}

/// Allocates a list of `n` nodes through `get_avail` (and the model), linked
/// in allocation order; returns its nodes.
fn make_list(g: &mut Globals, m: &mut Model, n: usize) -> Vec<i32> {
    let mut nodes = vec![];
    for _ in 0..n {
        let p = g.get_avail();
        assert_eq!(
            p,
            m.alloc(),
            "get_avail returned another node than pdfTeX's"
        );
        nodes.push(p);
    }
    for w in nodes.windows(2) {
        g.mem[w[0] as usize].set_hh_rh(w[1]);
    }
    nodes
}

fn check(g: &Globals, m: &Model, at: &str) {
    assert_eq!(g.hi_mem_min, m.hi_mem_min, "{at}: hi_mem_min");
    assert!(
        free_seq(g).iter().eq(m.free.iter()),
        "{at}: the free list is not pdfTeX's"
    );
    assert_eq!(g.lf_dyn_used(), m.dyn_used, "{at}: dyn_used");
}

#[test]
fn a_full_segment_stack_frees_by_walking_and_keeps_pdftex_free_list() {
    let (mut g, mut m) = fresh();
    let seg = seg_size as usize;
    // Enough lists that, freed one after another, they fill the stack and
    // then go on through the fallback for a few thousand more; lengths 2..5,
    // so the walk has something to walk.
    let total = seg + 1 + 3000;
    let lists: Vec<Vec<i32>> = (0..total)
        .map(|i| make_list(&mut g, &mut m, 2 + i % 4))
        .collect();
    check(&g, &m, "after allocating");
    let mut fallbacks = 0;
    let mut full_seen = false;
    for (i, l) in lists.iter().enumerate() {
        let before = g.seg_ptr;
        let full = before == seg_size;
        full_seen |= full;
        g.flush_list(l[0]);
        m.flush(l);
        if full {
            // the fallback: nothing pushed, the list walked and linked
            assert_eq!(g.seg_ptr, seg_size, "list {i}: pushed past a full stack");
            assert_eq!(g.avail, l[0], "list {i}: avail");
            assert_eq!(
                link(&g, *l.last().unwrap()),
                m.free[l.len()],
                "list {i}: the walked list's last node is not linked to the old avail"
            );
            fallbacks += 1;
        } else if before > 0 || i > 0 {
            assert_eq!(g.seg_ptr, before + 1, "list {i}: no segment pushed");
        }
        // one-node frees between them, as `free_avail` and backed-up tokens do
        if i % 1000 == 999 {
            let one = make_list(&mut g, &mut m, 1);
            g.flush_list(one[0]);
            m.flush(&one);
        }
        if i % 4096 == 0 || (full && fallbacks % 500 == 1) {
            check(&g, &m, &format!("after freeing list {i}"));
        }
    }
    assert!(
        full_seen,
        "the stack never filled: the fallback was not reached"
    );
    assert_eq!(fallbacks, total - seg - 1, "fallbacks taken");
    check(&g, &m, "after freeing every list");

    // Allocate half of it back: every node is pdfTeX's, in pdfTeX's order,
    // through the segments as they pop.
    let half = m.free.len() / 2;
    for k in 0..half {
        let p = g.get_avail();
        assert_eq!(p, m.alloc(), "allocation {k} after the frees");
        assert_eq!(link(&g, p), null, "allocation {k}: link not cleared");
    }
    check(&g, &m, "after allocating half back");

    // The merge the hosts do before reading the word space: one chain,
    // pdfTeX's sequence, the stack empty and cleared.
    g.flashtex_merge_free();
    assert_eq!(g.seg_ptr, 0, "segments left after the merge");
    assert!(
        (0..=seg).all(|k| g.seg_stack[k] == null),
        "seg_stack not cleared by the merge"
    );
    check(&g, &m, "after the merge");
    let last = *m.free.back().unwrap();
    assert_eq!(
        link(&g, last),
        null,
        "the merged chain does not end in null"
    );

    // And the rest, down to an empty free list and one fresh node.
    while !m.free.is_empty() {
        let p = g.get_avail();
        assert_eq!(p, m.alloc(), "allocation after the merge");
    }
    let p = g.get_avail();
    assert_eq!(p, m.alloc(), "the first node below hi_mem_min");
    check(&g, &m, "at the end");
}

#[test]
fn a_stack_that_fills_and_drains_repeatedly_stays_pdftex_free_list() {
    // Fill the stack past full, drain it by allocation, and do it again,
    // with frees of every length in between; the sequence is checked after
    // every round and the hosts' merge is taken at an odd moment.
    let (mut g, mut m) = fresh();
    let seg = seg_size as usize;
    for round in 0..3 {
        let lists: Vec<Vec<i32>> = (0..2 * seg)
            .map(|i| make_list(&mut g, &mut m, 1 + (i * 7 + round) % 6))
            .collect();
        for l in &lists {
            g.flush_list(l[0]);
            m.flush(l);
        }
        assert!(
            round > 0 || g.seg_ptr == seg_size,
            "round {round}: the stack did not fill ({} pending)",
            g.seg_ptr
        );
        check(&g, &m, &format!("round {round}: after the frees"));
        let take = m.free.len() * 2 / 3;
        for _ in 0..take {
            let p = g.get_avail();
            assert_eq!(p, m.alloc(), "round {round}: allocation");
        }
        check(&g, &m, &format!("round {round}: after the allocations"));
        if round == 1 {
            g.flashtex_merge_free();
            check(&g, &m, "after the merge in round 1");
        }
    }
}
