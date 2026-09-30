//! Array subscripts of the translated engine (L6, DESIGN.md §5.6 item 1).
//!
//! `tools/web2rust` wraps every subscript of `src/generated/` in [`U`]
//! (`--index-type crate::ix::U` in `web2rust-default.args`), so that these
//! impls, not the arrays' own, decide how an element is reached:
//!
//! * **Reads are bounds-checked in every build we ship** (release, the PGO
//!   build of `scripts/build-engine-dist.sh`, the resident host), as the
//!   arrays' own `Index` checks them. A violated range is a panic, never a
//!   silent wrong page (Commander decision, 2026-09-30: parity over speed).
//! * **Writes** are checked too: they go through the arrays' own `IndexMut`,
//!   which is also the checkpoint write barrier (`crate::arena`).
//!
//! **Benchmarking only: the `unchecked-reads` feature.** With it (and
//! without `debug_assertions`) reads skip the check. Measured
//! (docs/evidence/l6-optimizations-2026-09-29/): the checks cost 5-8% of the
//! cycles and 11% of the instructions on Apple M5 (the compiler must reload
//! an array's `len` after every write through `&mut Globals`), 1-3% on x86.
//! The subscripts are ones pdfTeX's own logic keeps in range, as web2c's C
//! (which checks nothing) relies on, but that is an invariant of TeX's
//! program, not something this module can prove, and a port bug would turn
//! into a read of a neighbouring array (one mapping, `crate::arena`) and a
//! silently wrong result on an untrusted document. So the feature exists to
//! price the checks, and no shipped build may enable it.
//!
//! The trip test's `tex.web` translation (feature `tex82`) does not wrap its
//! subscripts (`web2rust-trip.args` has no `--index-type`), so it leaves this
//! module out.

use crate::arena::Arr;
use std::ops::{Index, IndexMut};

/// A subscript of the generated code (see the module documentation).
#[derive(Clone, Copy, Debug)]
pub struct U(pub usize);

/// Whether reads are checked in this build: always, except in a
/// benchmarking build with `unchecked-reads` and without debug assertions.
pub const CHECKED: bool = cfg!(any(debug_assertions, not(feature = "unchecked-reads")));

#[inline(always)]
fn read<T>(a: &[T], i: usize) -> &T {
    if CHECKED {
        &a[i]
    } else {
        // SAFETY: none in the type system: the generated code only computes
        // subscripts pdfTeX keeps in range (module documentation). Only the
        // benchmarking feature `unchecked-reads` gets here; every shipped and
        // tested build checks (`CHECKED`).
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
