#!/usr/bin/env python3
"""Measurement-only experiment (never committed): a read barrier on every eqtb read of the
Classic engine. Each `eqtb[i]` read through `Index` sets bit i of a side bitmap, which is what
a generic "record every eqtb read" barrier costs at minimum (a per-segment epoch store costs the
same: one load-modify-store per read). Writes (`IndexMut`) are unchanged."""
import sys
root = sys.argv[1]
a = f'{root}/crates/flashtex-engine/src/arena.rs'
s = open(a).read()
s += r'''

/// READ-TRACK experiment: the read bitmap of eqtb (measurement only).
pub static mut RT_BITS: [u64; 1 << 14] = [0; 1 << 14];

/// READ-TRACK experiment: eqtb with a read barrier (measurement only).
pub struct RArr<T>(pub Arr<T>);

impl<T> Deref for RArr<T> {
    type Target = Arr<T>;
    #[inline(always)]
    fn deref(&self) -> &Arr<T> {
        &self.0
    }
}

impl<T> std::ops::DerefMut for RArr<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Arr<T> {
        &mut self.0
    }
}

impl<T> Index<usize> for RArr<T> {
    type Output = T;
    #[inline(always)]
    fn index(&self, i: usize) -> &T {
        // SAFETY: masked into the array; single engine thread (measurement only).
        unsafe {
            let b = &mut *std::ptr::addr_of_mut!(RT_BITS);
            *b.get_unchecked_mut((i >> 6) & ((1 << 14) - 1)) |= 1u64 << (i & 63);
        }
        &self.0[i]
    }
}

impl<T> IndexMut<usize> for RArr<T> {
    #[inline(always)]
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.0[i]
    }
}
'''
open(a, 'w').write(s)
g = f'{root}/crates/flashtex-engine/src/generated/globals.rs'
s = open(g).read()
s = s.replace('pub eqtb: crate::arena::Arr<memory_word>,', 'pub eqtb: crate::arena::RArr<memory_word>,', 1)
s = s.replace('eqtb: __arena.arr(__r_eqtb, 630192),', 'eqtb: crate::arena::RArr(__arena.arr(__r_eqtb, 630192)),', 1)
open(g, 'w').write(s)
print('patched')
