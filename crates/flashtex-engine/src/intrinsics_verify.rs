//! Both paths, diffed (DESIGN.md §5.6 item 4: "in CI both paths run and the
//! full state change is diffed"). Placeholder; see below.

use crate::generated::Globals;
use crate::intrinsics::Why;

pub(crate) fn begin(g: &mut Globals, slot: usize) {
    let _ = (g, slot);
}

pub(crate) fn note_op(k: i32, a: i32, b: i32, c: i32) {
    let _ = (k, a, b, c);
}

pub(crate) fn normal_path_done(g: &mut Globals, slot: usize) {
    let _ = (g, slot);
}

pub(crate) fn verify_aborted(g: &mut Globals, slot: usize, why: Why) {
    let _ = (g, slot, why);
}
