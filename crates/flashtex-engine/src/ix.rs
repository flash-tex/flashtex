//! Array subscripts of the translated engine (L6, DESIGN.md §5.6 item 1).
//!
//! `tools/web2rust` wraps every subscript of `src/generated/` in [`U`]
//! (`--index-type crate::ix::U` in `web2rust-default.args`), so that these
//! impls, not the arrays' own, decide how an element is reached:
//!
//! * **Reads** skip the bounds check in optimised builds. Measured on the
//!   benchmark documents (docs/evidence/l6-optimizations-2026-09-29/): the
//!   checks were 12-14% of the instructions and 5-10% of the cycles, most of
//!   them on `mem` and `eqtb`, whose `len` the compiler must reload after
//!   every write through `&mut Globals`.
//! * **Writes** stay checked: they go through the arrays' own `IndexMut`,
//!   which is also the checkpoint write barrier (`crate::arena`).
//!
//! Why a read can go unchecked: every subscript the generated code computes
//! is one pdfTeX's own logic keeps in range, as web2c's C (which has no
//! checks at all) relies on; `mem` and `eqtb` indices are pointers TeX
//! allocated or codes it range-checked when it scanned them. That is an
//! invariant of TeX's program, not something this module can prove. So:
//!
//! * debug builds (`debug_assertions`, every `cargo test` without
//!   `--release`) and the `checked-arrays` feature keep the check, and turn
//!   a violation into the same panic as before;
//! * an out-of-range read in an unchecked build reads whatever lies there;
//!   it never writes. The arrays are regions of one mapping
//!   (`crate::arena`), so a small overshoot reads a neighbouring array.
//!
//! The trip test's `tex.web` translation (feature `tex82`) does not wrap its
//! subscripts (`web2rust-trip.args` has no `--index-type`), so it leaves this
//! module out.

use crate::arena::Arr;
use std::ops::{Index, IndexMut};

/// A subscript of the generated code (see the module documentation).
#[derive(Clone, Copy, Debug)]
pub struct U(pub usize);

/// Whether reads are checked in this build.
pub const CHECKED: bool = cfg!(any(debug_assertions, feature = "checked-arrays"));

#[inline(always)]
fn read<T>(a: &[T], i: usize) -> &T {
    if CHECKED {
        &a[i]
    } else {
        // SAFETY: none in the type system: the generated code only computes
        // subscripts pdfTeX keeps in range (module documentation). Builds
        // that check them (`CHECKED`) run every test suite.
        unsafe { a.get_unchecked(i) }
    }
}

impl<T> Index<U> for Arr<T> {
    type Output = T;
    #[inline(always)]
    fn index(&self, i: U) -> &T {
        read(self, i.0)
    }
}

impl<T> IndexMut<U> for Arr<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: U) -> &mut T {
        &mut self[i.0]
    }
}

impl<T, const N: usize> Index<U> for [T; N] {
    type Output = T;
    #[inline(always)]
    fn index(&self, i: U) -> &T {
        read(self, i.0)
    }
}

impl<T, const N: usize> IndexMut<U> for [T; N] {
    #[inline(always)]
    fn index_mut(&mut self, i: U) -> &mut T {
        &mut self[i.0]
    }
}

impl<T> Index<U> for Vec<T> {
    type Output = T;
    #[inline(always)]
    fn index(&self, i: U) -> &T {
        read(self, i.0)
    }
}

impl<T> IndexMut<U> for Vec<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: U) -> &mut T {
        &mut self[i.0]
    }
}
