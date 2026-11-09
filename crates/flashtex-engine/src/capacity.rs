//! How much of TeX's buffer, string pool and string table a recorded call
//! uses (lane AUDIT-OVERFLOW, #2308): BOX-MEMO (`src/boxmemo.rs`) and D9
//! (`src/intrinsics.rs`) replay a call only where its normal path would not
//! stop with `! TeX capacity exceeded`.
//!
//! A replay does not run the call. Whatever the call put in the buffer above
//! the current line (`\csname`'s name, tex.web §374, and e-TeX's
//! `\ifcsname`), in the pool for a moment (`\the`, `\meaning`, `\string`
//! and the rest print into a new string, §464-§470; pdfTeX's
//! `tokens_to_string`, so `\pdfstrcmp`), or in the string table for a
//! moment (`make_string`, §43), it does not. The key holds no position in
//! any of them, so a call recorded with room to spare was replayed where its
//! normal path overflows: pdfTeX stops, and the replay went on.
//!
//! **What a recording measures.** Above each pointer (`first`, `pool_ptr`,
//! `str_ptr`) the call writes a run of places with no gap, from the pointer
//! up: §374's loop from `first`; `append_char` (§42) at `pool_ptr`, and
//! `id_lookup`'s move of the current string (§260) within what `str_room`
//! allowed; `make_string`'s `str_start[str_ptr]` as it raises `str_ptr`.
//! So the highest place it reaches is below the first place it did not
//! write. At its start a [`Probe`] puts a *marker*, a value TeX never stores
//! there ([`MARK`]: the buffer and the pool hold character codes,
//! `str_start` pool positions), at offsets 0, 1, 2, 4, 8, ... above each
//! pointer and at the array's last place: a few dozen writes. At the end,
//! the offset of the first marker still there bounds the places the call
//! used, within a factor of two. The places are dead memory: TeX reads none
//! of them before it writes them, and the convergence test does not compare
//! them (`incr::live_len`).
//!
//! **What an offer checks** ([`Need`]): each overflow test of the normal
//! path, with that bound put on top of the pointers as they are now.
//! - the buffer: §374 stops when the name reaches `buffer[buf_size-1]`, so
//!   a call that wrote `b` places above `first` overflows when
//!   `first+b >= buf_size`;
//! - the string table: `make_string` stops when `str_ptr=max_strings`, so
//!   a call that had `s` strings of its own at once overflows when
//!   `str_ptr+s > max_strings`;
//! - the pool: `str_room(n)` stops when `pool_ptr+n > pool_size`, and a
//!   print into a full pool drops the character (§58: `if
//!   pool_ptr<pool_size then append_char`; `tokens_to_string` stops at the
//!   top without a word). A call that wrote `p` places replays only where
//!   `pool_ptr+p+POOL_SLACK <= pool_size`: the slack covers a `str_room`
//!   that asks for more than is then written (`str_room(1)` before
//!   `make_string`).
//!
//! Where a test would fail, or the bound is not tight enough to tell, the
//! call runs for real, which is always right. A recording that wrote an
//! array's last place, or came within the slack of the pool's top, may have
//! been cut short there, so it is abandoned.

use crate::generated::consts::{buf_size, max_strings, pool_size};
use crate::generated::Globals;

/// Never a character code (`buffer`, `str_pool`) or a pool position
/// (`str_start`).
const MARK: i32 = -0x5a5a_5a5b;
/// More room than the call wrote, for a `str_room(n)` that asks for more
/// than it then writes.
const POOL_SLACK: i32 = 64;

/// Where a recording started: `first`, `pool_ptr`, `str_ptr`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Probe {
    first: i32,
    pool: i32,
    str_ptr: i32,
}

/// How far above each pointer a call goes, at most: the places of the
/// buffer above `first`, the characters above `pool_ptr`, the strings
/// above `str_ptr`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Need {
    pub buf: i32,
    pub pool: i32,
    pub strings: i32,
}

/// The marked places of an array whose first free place is `lo` and whose
/// last is `top`, as offsets above `lo`: 0, 1, 2, 4, ..., and `top-lo`.
fn marks(lo: i32, top: i32) -> impl Iterator<Item = i32> {
    let span = top - lo;
    std::iter::once(0)
        .chain((0..31).map(|k| 1i32 << k))
        .take_while(move |&o| o < span)
        .chain((span >= 0).then_some(span))
}

impl Probe {
    /// The probe of a recording that started with these pointers (D9 keeps
    /// them in the word space, so that a restore inside a recording finds
    /// its markers again).
    pub(crate) fn at(first: i32, pool_ptr: i32, str_ptr: i32) -> Probe {
        Probe {
            first,
            pool: pool_ptr,
            str_ptr,
        }
    }

    /// (lo, top) of the buffer, the pool and `str_start`: `make_string`
    /// writes `str_start[str_ptr]` after raising it, so `str_start`'s
    /// first free place is `str_ptr+1`.
    fn spans(&self) -> [(i32, i32); 3] {
        [
            (self.first, buf_size - 1),
            (self.pool, pool_size - 1),
            (self.str_ptr + 1, max_strings),
        ]
    }
}

impl Globals {
    fn cap_at(&self, a: usize, i: i32) -> i32 {
        let i = i as usize;
        match a {
            0 => self.buffer[i],
            1 => self.str_pool[i],
            _ => self.str_start[i],
        }
    }

    fn cap_mark(&mut self, a: usize, i: i32) {
        let i = i as usize;
        match a {
            0 => self.buffer[i] = MARK,
            1 => self.str_pool[i] = MARK,
            _ => self.str_start[i] = MARK,
        }
    }

    /// Start measuring (a recording's start): put the markers.
    pub(crate) fn cap_probe(&mut self) -> Probe {
        let p = Probe::at(self.first, self.pool_ptr, self.str_ptr);
        for (a, (lo, top)) in p.spans().into_iter().enumerate() {
            for o in marks(lo, top) {
                self.cap_mark(a, lo + o);
            }
        }
        p
    }

    /// What the call used since `p` (a recording's end), or `None` when it
    /// may have been cut short: it wrote an array's last place, came within
    /// the slack of the pool's top, or moved `first`.
    pub(crate) fn cap_need(&self, p: &Probe) -> Option<Need> {
        if self.first != p.first {
            return None;
        }
        let mut n = [0i32; 3];
        for (a, (lo, top)) in p.spans().into_iter().enumerate() {
            // (the first marker still there: the call wrote below it only)
            n[a] = marks(lo, top).find(|&o| self.cap_at(a, lo + o) == MARK)?;
        }
        let need = Need {
            buf: n[0],
            pool: n[1],
            strings: n[2],
        };
        if p.pool + need.pool + POOL_SLACK > pool_size {
            return None;
        }
        Some(need)
    }

    /// Whether a call that uses `n` would pass every one of these overflow
    /// tests if it ran now.
    pub(crate) fn cap_room_ok(&self, n: &Need) -> bool {
        (n.buf == 0 || self.first + n.buf < buf_size)
            && self.pool_ptr + n.pool + POOL_SLACK <= pool_size
            && self.str_ptr + n.strings <= max_strings
    }
}
