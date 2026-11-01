//! LAZY-FLUSH: `flush_list` without walking the list it frees (lane
//! CONTIG-TOKENS; `changes/lazyflush.ch` has the change and its proof,
//! docs/design/engine-v2/CONTIG-TOKENS.md the design and the measurements).
//!
//! Off unless the command-line driver switches it on
//! (`FLASHTEX_LAZY_FLUSH=1`, src/main.rs). The hosts never switch it on, so
//! their checkpoints and convergence tests (`crate::iso`, `crate::incr`),
//! which read the free list as one chain from `avail`, see what they always
//! saw: `seg_ptr` stays 0.

use crate::generated::Globals;
use std::sync::atomic::{AtomicBool, Ordering};

static ON: AtomicBool = AtomicBool::new(false);

/// The command line's switch, `FLASHTEX_LAZY_FLUSH=1`, read before the run
/// starts (`flashtex_lazy_flush_enabled` reads it at `Set initial values`).
pub fn enable_from_env() {
    if std::env::var_os("FLASHTEX_LAZY_FLUSH").is_some_and(|v| v == "1") {
        ON.store(true, Ordering::Relaxed);
    }
}

impl Globals {
    /// Whether `flush_list` pushes free-list segments (`lazy_flush`).
    pub fn flashtex_lazy_flush_enabled(&mut self) -> bool {
        ON.load(Ordering::Relaxed)
    }

    /// The free one-word nodes on the chain that starts at `p`.
    fn lf_chain_len(&self, mut p: i32) -> i32 {
        let mut n = 0i32;
        while p != 0 {
            n = n.wrapping_add(1);
            p = self.mem[p as usize].hh().rh();
        }
        n
    }

    /// pdfTeX's `dyn_used`, for the memory-usage lines. Without the switch
    /// it is `dyn_used`. With it, `flush_list` does not count, so it is
    /// worked out as pdftex.web's §1311 does: every hi-memory word that is
    /// not on the free list is in use (pdftex.web changes `dyn_used` by one
    /// at every allocation and every node freed, so the two agree at all
    /// times; leaked nodes are counted as used by both).
    pub fn flashtex_dyn_used(&mut self) -> i32 {
        if !self.lazy_flush {
            return self.dyn_used;
        }
        let mut free = self.lf_chain_len(self.avail);
        for k in 0..self.seg_ptr.max(0) {
            free = free.wrapping_add(self.lf_chain_len(self.seg_stack[k as usize]));
        }
        (self.mem_end + 1 - self.hi_mem_min).wrapping_sub(free)
    }

    /// Makes the free list one chain again, as pdftex.web keeps it: each
    /// pending segment's last node is linked to the next segment, in the
    /// order `get_avail` would take them (before the format's dump).
    pub fn flashtex_merge_free(&mut self) {
        if self.seg_ptr <= 0 {
            return;
        }
        if self.avail == 0 {
            self.seg_ptr -= 1;
            self.avail = self.seg_stack[self.seg_ptr as usize];
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
            let s = self.seg_stack[self.seg_ptr as usize];
            self.mem[q as usize].set_hh_rh(s);
        }
    }
}

