//! The arrays `src/generated/` declares (`crate::arena::Arr`), in the
//! interface web2rust emits for the engines' word space
//! (`crates/flashtex-engine/src/arena.rs`), without what BibTeX does not need:
//! there are no checkpoints, so every array is its own growable vector.
//!
//! web2c's run-time arrays map as web2rust's README says: `xmalloc_array(T,
//! n)` is [`Arr::alloc_len`] of `n + 1` elements, `xrealloc_array(p, T, n)`
//! is [`Arr::resize_len`] (the old elements kept). Fresh elements are zero,
//! where C's `xmalloc` leaves them undefined; BibTeX never reads one before
//! writing it (the comparison with TeX Live's binary checks that).

use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

/// Bytes of one scalar global (for `SCALAR_BYTES`, which only the engines'
/// checkpoints use).
pub const fn slot<T>() -> usize {
    std::mem::size_of::<T>()
}

/// web2rust's layout plan; here only the initial lengths matter.
pub struct Plan;

/// A reservation in the plan: the array's starting capacity.
pub struct Region<T> {
    cap: usize,
    _t: PhantomData<T>,
}

impl Plan {
    pub fn new(_scalar_bytes: usize) -> Plan {
        Plan
    }

    pub fn reserve<T>(&mut self, _name: &'static str, cap: usize) -> Region<T> {
        Region {
            cap,
            _t: PhantomData,
        }
    }

    pub fn build(self) -> Arena {
        Arena
    }
}

/// The word space: nothing, since each [`Arr`] owns its elements.
pub struct Arena;

impl Arena {
    pub fn arr<T: Copy + Default>(&self, r: Region<T>, len: usize) -> Arr<T> {
        let mut v = Vec::with_capacity(r.cap.max(len));
        v.resize(len, T::default());
        Arr(v)
    }
}

/// One array global: indexing is checked, so a read past the end that the
/// C program would make (undefined behaviour there) stops the run here.
///
/// It dereferences to its `Vec`, which is what a routine's `var` parameter
/// of a pointer type (`var buf: buf_type`) is in the translation: the call
/// moves the array out of its global for the call, as web2rust does for
/// every `var` argument.
#[derive(Default)]
pub struct Arr<T>(Vec<T>);

impl<T> std::ops::Deref for Arr<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Arr<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.0
    }
}

impl<T: Copy + Default> Arr<T> {
    /// `xmalloc_array`: `n` elements, all new.
    pub fn alloc_len(&mut self, n: usize) {
        self.0.clear();
        self.0.resize(n, T::default());
    }

    /// `xrealloc_array`: `n` elements, the first ones kept.
    pub fn resize_len(&mut self, n: usize) {
        self.0.resize(n, T::default());
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T> Index<usize> for Arr<T> {
    type Output = T;
    #[inline(always)]
    fn index(&self, i: usize) -> &T {
        &self.0[i]
    }
}

impl<T> IndexMut<usize> for Arr<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.0[i]
    }
}

/// web2rust's scalar visitor (the engines' checkpoint spill and fill);
/// BibTeX's `Globals::visit_scalars` exists but nothing calls it.
pub trait Visit {
    fn pod<T: Copy>(&mut self, x: &mut T);
    fn arr_len<T>(&mut self, _a: &mut Arr<T>) {}
}
