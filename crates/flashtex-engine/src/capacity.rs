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
//! **What a recording measures.** At its start, a [`Probe`] fills a window
//! of each array above its pointer with a value TeX never stores there
//! ([`SENTINEL`]: the buffer and the pool hold character codes, `str_start`
//! pool positions). What the call writes there it writes in order from the
//! pointer up (`append_char`, §42, `make_string`'s `str_start[str_ptr]`,
//! §374's loop from `first`), so at the end the highest place no longer
//! holding the sentinel is how far up the call went. The windows are dead
//! memory: TeX reads none of the three above its pointer before it writes
//! it, and the convergence test does not compare them (`incr::live_len`).
//!
//! **What an offer checks** ([`Need`]): each overflow test of the normal
//! path, with the call's use put on top of the pointers as they are now.
//! - the buffer: §374 stops when the name reaches `buffer[buf_size-1]`, so
//!   a call that wrote `b` places above `first` overflows exactly when
//!   `first+b >= buf_size`;
//! - the string table: `make_string` stops when `str_ptr=max_strings`, so
//!   a call that had `s` strings of its own at once overflows exactly when
//!   `str_ptr+s > max_strings`;
//! - the pool: `str_room(n)` stops when `pool_ptr+n > pool_size`, and a
//!   print into a full pool drops the character (§58: `if
//!   pool_ptr<pool_size then append_char`). A call that wrote `p` places
//!   replays only where `pool_ptr+p+POOL_SLACK <= pool_size`: the slack
//!   covers a `str_room` that asks for more than is then written
//!   (`str_room(1)` before `make_string`, `str_room(12)` for a font's
//!   name). Closer to the top the call runs for real, which is always
//!   right.
//!
//! A recording that wrote its window's last place, or came within the slack
//! of the pool's top, is abandoned: it cannot say how much it used.

use crate::generated::consts::{buf_size, max_strings, pool_size};
use crate::generated::Globals;

/// Never a character code (`buffer`, `str_pool`) or a pool position
/// (`str_start`).
const SENTINEL: i32 = -0x5a5a_5a5b;
/// Places watched above `first`: longer `\csname` names are not recorded.
const BUF_WINDOW: i32 = 4096;
/// Characters watched above `pool_ptr`.
const POOL_WINDOW: i32 = 65536;
/// Strings watched above `str_ptr`.
const STR_WINDOW: i32 = 4096;
/// More room than the call wrote, for a `str_room(n)` that asks for more
/// than it then writes.
const POOL_SLACK: i32 = 64;

/// The windows a recording watches: where each starts and ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Probe {
    first: i32,
    buf_end: i32,
    pool: i32,
    pool_end: i32,
    str_ptr: i32,
    str_end: i32,
}

/// How far above each pointer a call goes: the places of the buffer above
/// `first`, the characters above `pool_ptr`, the strings above `str_ptr`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Need {
    pub buf: i32,
    pub pool: i32,
    pub strings: i32,
}

impl Probe {
    /// The windows of a recording that started with these pointers (D9
    /// keeps the pointers in the word space, so that a restore inside a
    /// recording finds its windows again).
    pub(crate) fn at(first: i32, pool_ptr: i32, str_ptr: i32) -> Probe {
        Probe {
            first,
            buf_end: (first + BUF_WINDOW).min(buf_size),
            pool: pool_ptr,
            pool_end: (pool_ptr + POOL_WINDOW).min(pool_size),
            // (`make_string` writes `str_start[str_ptr]` after raising it)
            str_ptr,
            str_end: (str_ptr + 1 + STR_WINDOW).min(max_strings + 1),
        }
    }
}

impl Globals {
    /// Start watching the three arrays (a recording's start).
    pub(crate) fn cap_probe(&mut self) -> Probe {
        let p = Probe::at(self.first, self.pool_ptr, self.str_ptr);
        for i in p.first..p.buf_end {
            self.buffer[i as usize] = SENTINEL;
        }
        for i in p.pool..p.pool_end {
            self.str_pool[i as usize] = SENTINEL;
        }
        for i in p.str_ptr + 1..p.str_end {
            self.str_start[i as usize] = SENTINEL;
        }
        p
    }

    /// What the call used since `p` (a recording's end), or `None` when it
    /// cannot tell: the call wrote a window's last place, came within the
    /// slack of the pool's top, or moved `first`.
    pub(crate) fn cap_need(&self, p: &Probe) -> Option<Need> {
        if self.first != p.first {
            return None;
        }
        // (the number of places written: the highest one, plus one)
        let used = |lo: i32, hi: i32, at: &dyn Fn(i32) -> i32| -> Option<i32> {
            if hi > lo && at(hi - 1) != SENTINEL {
                return None;
            }
            Some(
                (lo..hi)
                    .rev()
                    .find(|&i| at(i) != SENTINEL)
                    .map_or(0, |i| i + 1 - lo),
            )
        };
        let buf = used(p.first, p.buf_end, &|i| self.buffer[i as usize])?;
        let pool = used(p.pool, p.pool_end, &|i| self.str_pool[i as usize])?;
        let strings = used(p.str_ptr + 1, p.str_end, &|i| self.str_start[i as usize])?;
        if p.pool + pool + POOL_SLACK > pool_size {
            return None;
        }
        Some(Need { buf, pool, strings })
    }

    /// Whether a call that uses `n` would pass every one of these overflow
    /// tests if it ran now.
    pub(crate) fn cap_room_ok(&self, n: &Need) -> bool {
        (n.buf == 0 || self.first + n.buf < buf_size)
            && self.pool_ptr + n.pool + POOL_SLACK <= pool_size
            && self.str_ptr + n.strings <= max_strings
    }
}
