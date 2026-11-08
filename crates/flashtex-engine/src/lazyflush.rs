//! LAZY-FLUSH: `flush_list` without walking the list it frees (lane
//! CONTIG-TOKENS; `changes/lazyflush.ch` has the change and its proof,
//! docs/design/engine-v2/CONTIG-TOKENS.md the design and the measurements).
//!
//! The free list of one-word nodes is a stack of segments: `avail`'s chain,
//! then the chains of `seg_stack[seg_ptr-1]`, ..., `seg_stack[0]`, each
//! ending in a null link. As a sequence it is always pdftex.web's free list.
//! These are the two routines the change file calls; `spill_scalars`
//! (src/checkpoint.rs) calls [`flashtex_merge_free`](Globals::flashtex_merge_free)
//! before every reading of the word space, so the hosts' checkpoints,
//! restores, hashes and convergence tests see the free list as one chain.
//! `tests/lazyflush_segments.rs` checks the segments against pdftex.web's
//! free list node for node, through a full stack (`seg_size` pending: the
//! walking fallback), the pops, and the merge.

use crate::generated::Globals;

impl Globals {
    /// The free one-word nodes on the chain that starts at `p`.
    fn lf_chain_len(&self, mut p: i32) -> i32 {
        let mut n = 0i32;
        while p != 0 {
            n = n.wrapping_add(1);
            p = self.mem[p as usize].hh().rh();
        }
        n
    }

    /// pdfTeX's `dyn_used` for the state now: every hi-memory word that is
    /// not on the free list is in use. pdftex.web changes `dyn_used` by one
    /// at every allocation and every node freed, so its count always equals
    /// this (section 1311 recomputes it so for the dump; a leaked node is
    /// in use for both). The engine keeps no running count
    /// (`changes/lazyflush.ch` (2)).
    pub fn lf_dyn_used(&self) -> i32 {
        let mut free = self.lf_chain_len(self.avail);
        for k in 0..self.seg_ptr.max(0) {
            free = free.wrapping_add(self.lf_chain_len(self.seg_stack[k as usize]));
        }
        (self.mem_end + 1 - self.hi_mem_min).wrapping_sub(free)
    }

    /// [`lf_dyn_used`](Self::lf_dyn_used), for the memory-usage lines of
    /// `\tracingstats>1`.
    pub fn flashtex_dyn_used(&mut self) -> i32 {
        self.lf_dyn_used()
    }

    /// Makes the free list one chain again, as pdftex.web keeps it: each
    /// pending segment's last node is linked to the next segment, in the
    /// order `get_avail` would take them, and the stack's entries are
    /// cleared (`seg_stack` is null above `seg_ptr`). Then every word is the
    /// one the walking `flush_list` would have left. Each node it walks was
    /// freed since the previous merge (the bottom segment, which holds the
    /// rest, is not walked), so it costs at most the walks it replaced.
    pub fn flashtex_merge_free(&mut self) {
        if self.seg_ptr <= 0 {
            return;
        }
        if self.avail == 0 {
            self.seg_ptr -= 1;
            let k = self.seg_ptr as usize;
            self.avail = self.seg_stack[k];
            self.seg_stack[k] = 0;
        }
        let mut q = self.avail;
        while self.seg_ptr > 0 {
            loop {
                let r = self.mem[q as usize].hh().rh();
                if r == 0 {
                    break;
                }
                q = r;
            }
            self.seg_ptr -= 1;
            let k = self.seg_ptr as usize;
            let s = self.seg_stack[k];
            self.seg_stack[k] = 0;
            self.mem[q as usize].set_hh_rh(s);
            q = s;
        }
    }
}
