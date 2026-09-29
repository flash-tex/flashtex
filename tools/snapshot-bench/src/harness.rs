//! The measured loops, shared by every phase.

use std::hint::black_box;
use std::time::Instant;

use crate::backend::Backend;
use crate::workload::Op;

/// Dependent integer work standing in for the engine's own per-access cost (token
/// dispatch, `divide_scaled`, glue arithmetic). Each round is a multiply, a rotate and an
/// xor, all on the same register, so the rounds cannot overlap.
#[inline(always)]
fn engine_work<const ROUNDS: u32>(mut x: u64) -> u64 {
    let mut k = 0;
    while k < ROUNDS {
        x = x
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .rotate_left(23)
            ^ 0xD1B5_4A32_D192_ED03;
        k += 1;
    }
    x
}

/// Replay one page's op stream against `b`.
///
/// Every value read feeds the accumulator, and every value written derives from it, so
/// the chain of loads and stores cannot be reordered away or elided. `ROUNDS` adds the
/// synthetic engine work; `ROUNDS = 0` is the bare replay loop.
#[inline(never)]
pub fn run_page<B: Backend, const ROUNDS: u32>(b: &mut B, ops: &[Op], seed: u64) -> u64 {
    let mut acc = seed;
    for &op in ops {
        let i = op.index();
        if op.is_write() {
            acc = acc.wrapping_add(i as u64).rotate_left(11);
            b.set(i, acc);
        } else {
            acc = acc.wrapping_add(b.get(i)).rotate_left(7) ^ i as u64;
        }
        if ROUNDS > 0 {
            acc = engine_work::<ROUNDS>(acc);
        }
    }
    acc
}

pub struct PageTiming {
    pub ns: f64,
    pub acc: u64,
}

/// Time one page, returning the elapsed nanoseconds and the accumulator (black-boxed so
/// the optimiser cannot drop the work).
#[inline(never)]
pub fn time_page<B: Backend, const ROUNDS: u32>(b: &mut B, ops: &[Op], seed: u64) -> PageTiming {
    let ops = black_box(ops);
    let t = Instant::now();
    let acc = run_page::<B, ROUNDS>(b, ops, seed);
    let ns = t.elapsed().as_nanos() as f64;
    PageTiming {
        ns,
        acc: black_box(acc),
    }
}

/// Rounds of synthetic engine work used for the `engine` hot-loop flavour.
pub const ENGINE_ROUNDS: u32 = 8;

/// Page-time budgets from DESIGN Appendix B.1, used to express a barrier's absolute cost
/// as a fraction of a real engine's hot loop.
pub const BODY_PAGE_NS: f64 = 2_400_000.0; // 2.4 ms, the 183-page review document
pub const PLOT_PAGE_NS: f64 = 90_000_000.0; // 90 ms, a heavy pgfplots page
