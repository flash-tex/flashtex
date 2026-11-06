//! The stack high-water marks a recording reaches, and a replay raises to
//! (`changes/stackpeak.ch`).
//!
//! The end-of-job statistics print `max_in_stack`, `max_nest_stack`,
//! `max_param_stack`, `max_buf_stack` and `max_save_stack` (tex.web §1334,
//! "stack positions"). A replayed macro -- an intrinsic
//! (`crate::intrinsics`) or a BOX-MEMO call (`crate::boxmemo`) -- does not
//! push what its normal expansion pushes, so a replay in a deeper place than
//! its recording left the marks lower than pdfTeX's.
//!
//! Each mark has a shadow `pk_*`, raised by the generated code wherever the
//! mark is (`pk <= max` always). A recording lowers the shadows to 0 when it
//! begins ([`Globals::peaks_begin`]); when it ends
//! ([`Globals::peaks_end`]) they hold the peaks its normal path reached,
//! which it keeps relative to the levels it began at, and are raised back
//! over what they held before (so an enclosing recording's tracking goes
//! on). A replay at given levels raises the marks and the shadows to where
//! the normal path's peaks would have taken them ([`Globals::peaks_raise`]),
//! after checking the normal path would not have overflowed a stack there
//! ([`Globals::peaks_overflow`]).

use crate::generated::consts::{buf_size, nest_size, param_size, save_size, stack_size};
use crate::generated::Globals;

/// The five marks, or levels, in §1334's order: input stack, semantic
/// nest, parameter stack, buffer, save stack.
pub type Marks = [i32; 5];

/// A peak the recording never reached: its normal path raised no mark.
pub const NONE: i32 = -1;

/// No peak at all.
pub const NO_PEAKS: Marks = [NONE; 5];

impl Globals {
    fn pk_get(&self) -> Marks {
        [
            self.pk_in_stack,
            self.pk_nest_stack,
            self.pk_param_stack,
            self.pk_buf_stack,
            self.pk_save_stack,
        ]
    }

    fn pk_set(&mut self, m: Marks) {
        self.pk_in_stack = m[0];
        self.pk_nest_stack = m[1];
        self.pk_param_stack = m[2];
        self.pk_buf_stack = m[3];
        self.pk_save_stack = m[4];
    }

    fn max_get(&self) -> Marks {
        [
            self.max_in_stack,
            self.max_nest_stack,
            self.max_param_stack,
            self.max_buf_stack,
            self.max_save_stack,
        ]
    }

    fn max_set(&mut self, m: Marks) {
        self.max_in_stack = m[0];
        self.max_nest_stack = m[1];
        self.max_param_stack = m[2];
        self.max_buf_stack = m[3];
        self.max_save_stack = m[4];
    }

    /// The levels the marks are taken of: `input_ptr`, `nest_ptr`,
    /// `param_ptr`, `first` (the buffer's scratch begins there) and
    /// `save_ptr`.
    pub fn stack_levels(&self) -> Marks {
        [
            self.input_ptr,
            self.nest_ptr,
            self.param_ptr,
            self.first,
            self.save_ptr,
        ]
    }

    /// A recording begins: the shadows are lowered, and what they held is
    /// returned, for [`Globals::peaks_end`].
    pub fn peaks_begin(&mut self) -> Marks {
        let saved = self.pk_get();
        self.pk_set([0; 5]);
        saved
    }

    /// The recording that began at `base` ends: the peaks its normal path
    /// reached, relative to `base` ([`NONE`] where it raised no mark), and
    /// the shadows raised back over `saved`.
    pub fn peaks_end(&mut self, saved: Marks, base: Marks) -> Marks {
        let cur = self.pk_get();
        let mut rel = NO_PEAKS;
        let mut back = saved;
        for i in 0..5 {
            // (every value a mark is raised to is at least the level the
            // recording began at: its pushes are above its own levels)
            if cur[i] > 0 && cur[i] >= base[i] {
                rel[i] = cur[i] - base[i];
            }
            back[i] = saved[i].max(cur[i]);
        }
        self.pk_set(back);
        rel
    }

    /// Would the normal path, at `base`, overflow a stack where its peaks
    /// `rel` take it? Each test is the one the generated code makes when it
    /// raises that mark (tex.web §216, §321, §374, §390, §273).
    pub fn peaks_overflow(&self, base: Marks, rel: Marks) -> bool {
        let v = |i: usize| (rel[i] != NONE).then(|| base[i] + rel[i]);
        v(0).is_some_and(|x| x >= stack_size)
            || v(1).is_some_and(|x| x >= nest_size)
            || v(2).is_some_and(|x| x > param_size)
            || v(3).is_some_and(|x| x >= buf_size)
            || v(4).is_some_and(|x| x > save_size - 7)
    }

    /// A replay at `base`: the marks (and the shadows) where the normal
    /// path's peaks `rel` would have raised them.
    pub fn peaks_raise(&mut self, base: Marks, rel: Marks) {
        let (mut m, mut p) = (self.max_get(), self.pk_get());
        for i in 0..5 {
            if rel[i] != NONE {
                let v = base[i] + rel[i];
                m[i] = m[i].max(v);
                p[i] = p[i].max(v);
            }
        }
        self.max_set(m);
        self.pk_set(p);
    }
}
