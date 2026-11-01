//! `changes/throughput.ch` [10]: `push_input`'s `input_stack[input_ptr]:=
//! cur_input`, the record stacked a field at a time.
//!
//! `push_input` runs for every new input level (`begin_token_list`, so
//! every macro call, parameter, `back_input` and inserted list). Just
//! before it, the program has usually stored one of `cur_input`'s fields on
//! its own (`loc` after reading a token, most often). Copied as a whole,
//! the 28-byte record is read with two 16-byte loads, each covering several
//! fields, and a load that spans a store still in flight cannot take its
//! value from that store: it waits for the store to reach the cache. On the
//! arXiv set's NixOS PC (Zen 4) `perf annotate` put a quarter of
//! `get_next_slow`'s samples on that load (lane THROUGHPUT). Read a field
//! at a time, each load matches the store that last wrote its field and
//! is forwarded from it.
//!
//! The reads are volatile so that the compiler does not merge them back
//! into the wide loads; the record stored is the same record, field for
//! field, through the same bounds check and write barrier as the
//! assignment's (`Arr`'s `IndexMut`).

use crate::generated::types::in_state_record;
use crate::generated::Globals;

impl Globals {
    /// `input_stack[input_ptr]:=cur_input`.
    #[inline(always)]
    pub fn flashtex_stack_cur_input(&mut self) {
        let c = &self.cur_input;
        // SAFETY: each pointer is a reference to a field of `self.cur_input`,
        // valid and aligned for the read.
        let r = unsafe {
            in_state_record {
                state_field: std::ptr::read_volatile(&c.state_field),
                index_field: std::ptr::read_volatile(&c.index_field),
                start_field: std::ptr::read_volatile(&c.start_field),
                loc_field: std::ptr::read_volatile(&c.loc_field),
                limit_field: std::ptr::read_volatile(&c.limit_field),
                name_field: std::ptr::read_volatile(&c.name_field),
                synctex_tag_field: std::ptr::read_volatile(&c.synctex_tag_field),
            }
        };
        let i = self.input_ptr;
        self.input_stack[crate::ix::U(i as usize)] = r;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The record stacked is `cur_input`, every field of it.
    #[test]
    fn stacks_the_whole_record() {
        let mut g = Globals::new();
        g.input_ptr = 3;
        g.cur_input = in_state_record {
            state_field: 1,
            index_field: 2,
            start_field: 3,
            loc_field: 4,
            limit_field: 5,
            name_field: 6,
            synctex_tag_field: 7,
        };
        g.flashtex_stack_cur_input();
        assert_eq!(g.input_stack[3], g.cur_input);
    }
}
