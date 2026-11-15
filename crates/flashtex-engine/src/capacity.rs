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
//!   `str_ptr+s > max_strings`. `end_name` (§517, a file name scanned)
//!   stops when `str_ptr+3 > max_strings`, whether it then makes three
//!   strings or one, so a call replays only where
//!   `str_ptr+s+STR_SLACK <= max_strings`;
//! - the pool: `str_room(n)` stops when `pool_ptr+n > pool_size`, and a
//!   print into a full pool drops the character (§58: `if
//!   pool_ptr<pool_size then append_char`; `tokens_to_string` stops at the
//!   top without a word). A call that wrote `p` places replays only where
//!   `pool_ptr+p+POOL_SLACK <= pool_size`: the slack covers a `str_room`
//!   that asks for more than is then written (`str_room(1)` before
//!   `make_string`).
//!
//! **Main memory** (#2309). The call's nodes and tokens come out of `mem`,
//! and what it needs depends on the free lists, which no key holds. So
//! the bound is one that holds whatever they are: the room the call needs
//! between `lo_mem_max` and `hi_mem_min` if it finds no free word at all.
//! - One-word nodes (`get_avail`, tex.web §120): the call's own freed
//!   words are taken again first (`avail` is a stack), so it lowers
//!   `hi_mem_min` by at most the most it has in use at once above its
//!   start: the peak of `dyn_used`, which changes/boxmemo.ch keeps
//!   (`cap_dyn_peak`).
//! - Variable-size nodes (`get_node`, §125): it grows `lo_mem_max` by
//!   1000 words at a time, and only when no free run holds the node. Every
//!   word of a grown block but its bottom remainder has been given out
//!   since, and the remainder is smaller than the node that did not fit.
//!   So the growth is at most twice the words `get_node` gave out
//!   (`cap_var`), plus the largest node (`cap_var_max`), plus one block:
//!   [`Need::mem`] adds a 128th and [`MEM_SLACK`] on top. (Below 1998
//!   free words `get_node` grows by less than 1000; the slack keeps the
//!   normal path out of there.)
//!
//! An offer replays only where `hi_mem_min - lo_mem_max` is at least that.
//! Then the normal path cannot stop with `main memory size`, so leaving it
//! out is exact.
//!
//! **A later `main memory size`** still depends on whether calls were
//! replayed: the normal path grows `lo_mem_max` for good and a replay does
//! not, so a run that replayed stops later than pdfTeX does. Such a run is
//! thrown away and made again without replays (#2309's second half, the
//! Commander's ruling), so a `main memory size` that reaches the log only
//! ever comes from the normal path:
//! - `get_avail` and `get_node` call [`Globals::flashtex_cap_mem_full`]
//!   before `overflow("main memory size", ...)` (changes/boxmemo.ch).
//! - The command-line engine starts over there: the same command line
//!   with `FLASHTEX_BOXMEMO=off FLASHTEX_INTRINSICS=off` ([`rerun_process`]).
//!   Its files are written again from their start. What the first attempt
//!   printed on the terminal stays there.
//! - A resident engine (the hosts) lets the run end and tells its host
//!   ([`take_rerun`]), which runs the document again from the format with
//!   replays switched off ([`set_replays_off`]) until a compile ends
//!   without that overflow.
//!
//! **Near the limit** (#2309's third piece, the Commander's ruling of
//! 2026-11-15). A document pdfTeX stops on with `main memory size` can fit
//! here because replays left it the room: then nothing overflows and the
//! rule above never fires. So a run that replayed is also made again
//! without replays, the same way, when it ends using more than 7/8 of main
//! memory: when `hi_mem_min - lo_mem_max`, the room neither kind of node
//! has taken yet, is below `mem_max/8` ([`Globals::flashtex_cap_run_end`]).
//! That room only shrinks during a run (`get_avail` lowers `hi_mem_min`,
//! tex.web §120, `get_node` raises `lo_mem_max`, §125, and nothing else
//! writes them after `undump`), so its value at the end is the run's
//! peak. The rerun's own result is shipped whatever it is, and replays stay
//! off until a compile ends below 7/8 (a host), so a document that stays
//! near the limit is compiled once per edit, without replays.
//!
//! Not covered (the residual the ruling accepts, BOX-MEMO.md K8 and §18): a
//! document pdfTeX stops on only because the normal path's drift took more
//! than an eighth of main memory, while the replaying run stays below 7/8.
//!
//! Where a test would fail, or the bound is not tight enough to tell, the
//! call runs for real, which is always right. A recording that wrote an
//! array's last place, or came within the slack of the pool's or the string
//! table's top, may have been cut short there, so it is abandoned.

use crate::generated::consts::{buf_size, max_strings, mem_max, pool_size};
use crate::generated::Globals;

/// Never a character code (`buffer`, `str_pool`) or a pool position
/// (`str_start`).
const MARK: i32 = -0x5a5a_5a5b;
/// More room than the call wrote, for a `str_room(n)` that asks for more
/// than it then writes.
const POOL_SLACK: i32 = 64;

/// More strings than the call had at once, for `end_name`'s test, which
/// asks for three whatever it then makes.
const STR_SLACK: i32 = 3;

/// Room between `lo_mem_max` and `hi_mem_min` on top of what the call
/// needs: one block `get_node` grows by, and the 1998 words below which
/// it grows by less.
const MEM_SLACK: i64 = 4096;
/// Where `cap_var` stops counting (changes/boxmemo.ch).
const VAR_TOP: i32 = 1 << 30;

/// Where a recording started: `first`, `pool_ptr`, `str_ptr`, and
/// `dyn_used`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Probe {
    first: i32,
    pool: i32,
    str_ptr: i32,
    dyn0: i32,
}

/// How far above each pointer a call goes, at most: the places of the
/// buffer above `first`, the characters above `pool_ptr`, the strings
/// above `str_ptr`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Need {
    pub buf: i32,
    pub pool: i32,
    pub strings: i32,
    /// The words of main memory between `lo_mem_max` and `hi_mem_min`
    /// (module documentation, "Main memory").
    pub mem: i32,
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

thread_local! {
    /// A run of this thread stopped with `main memory size`, or ended within
    /// an eighth of it ([`Globals::flashtex_cap_run_end`]).
    static MEM_FULL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Calls replayed (BOX-MEMO and D9, not their verify modes) by the
    /// engines of this thread since [`new_run`]. Never restored with a
    /// checkpoint: it counts the replays of abandoned runs too, and a state
    /// opened from disk counts as one ([`unknown_history`]), which only
    /// errs towards running again.
    static REPLAYS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Replays and recordings are switched off (`set_replays_off`).
    static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A call was replayed.
pub(crate) fn note_replay() {
    REPLAYS.with(|r| r.set(r.get() + 1));
}

/// A run from the format begins: no state it leaves has a replay behind it
/// yet.
pub fn new_run() {
    REPLAYS.with(|r| r.set(0));
    MEM_FULL.with(|f| f.set(false));
}

/// The engine goes on from a state another process left (S₀, S₁): whether
/// a replay is behind it is not known.
pub fn unknown_history() {
    note_replay();
}

/// Whether offers are refused: BOX-MEMO and D9 neither replay nor record.
pub fn replays_off() -> bool {
    OFF.with(|o| o.get())
}

/// Switch replays and recordings off or back on, for every engine of this
/// thread. Only between runs.
pub fn set_replays_off(off: bool) {
    OFF.with(|o| o.set(off));
}

/// Whether the rule is on (`FLASHTEX_MEM_RERUN=0` turns it off, to see the
/// run it replaces).
fn rerun_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("FLASHTEX_MEM_RERUN").map_or(true, |v| v != "0"))
}

/// What a host does after a run of its engine ended (not one it stopped):
/// `true` when the run stopped with `main memory size`, or ended within an
/// eighth of it, after replays, and replays are now off: the host runs the
/// document again from the format. A run that ended below that switches
/// replays back on.
pub fn take_rerun() -> bool {
    let full = MEM_FULL.with(|f| f.replace(false));
    if !rerun_enabled() {
        return false;
    }
    if !full {
        set_replays_off(false);
        return false;
    }
    if replays_off() || REPLAYS.with(|r| r.get()) == 0 {
        // (the normal path's own overflow: pdfTeX's)
        return false;
    }
    set_replays_off(true);
    true
}

/// The command-line engine starts over without replays (module
/// documentation). Returns only if it could not.
fn rerun_process() {
    use std::io::Write;
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut args = std::env::args_os();
    let arg0 = args.next();
    eprintln!(
        "flashtex: main memory ran out, or nearly, after replayed calls; running again without replays"
    );
    let _ = std::io::stdout().flush();
    let mut c = std::process::Command::new(exe);
    c.args(args)
        .env("FLASHTEX_BOXMEMO", "off")
        .env("FLASHTEX_INTRINSICS", "off");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // (the name it was called by chooses the format: `pdflatex`)
        if let Some(a) = arg0 {
            c.arg0(a);
        }
        let e = c.exec();
        eprintln!("flashtex: could not run again: {e}");
    }
    #[cfg(not(unix))]
    {
        let _ = arg0;
        if let Ok(st) = c.status() {
            std::process::exit(st.code().unwrap_or(1));
        }
    }
}

impl Probe {
    /// The probe of a recording that started with these pointers (D9 keeps
    /// them in the word space, so that a restore inside a recording finds
    /// its markers again).
    pub(crate) fn at(first: i32, pool_ptr: i32, str_ptr: i32, dyn_used: i32) -> Probe {
        Probe {
            first,
            pool: pool_ptr,
            str_ptr,
            dyn0: dyn_used,
        }
    }

    /// `dyn_used` at the start (D9 keeps it in the word space).
    pub(crate) fn dyn0(&self) -> i32 {
        self.dyn0
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
    /// `get_avail` and `get_node`, about to stop with `main memory size`
    /// (changes/boxmemo.ch; module documentation, "A later `main memory
    /// size`").
    /// A run ended, or stopped (`exit_process` for the command-line engine,
    /// `catch_exit` for a resident one, not a run its host paused): if it
    /// came within an eighth of main memory's end, it is treated as one that
    /// ran out (module documentation, "Near the limit").
    pub fn flashtex_cap_run_end(&mut self) {
        if self.hi_mem_min - self.lo_mem_max < mem_max / 8 {
            self.flashtex_cap_mem_full();
        }
    }

    pub fn flashtex_cap_mem_full(&mut self) {
        if crate::system::is_resident() {
            MEM_FULL.with(|f| f.set(true));
        } else if rerun_enabled() && !replays_off() && REPLAYS.with(|r| r.get()) > 0 {
            rerun_process();
        }
    }

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

    /// Start measuring (a recording's start): put the markers, and start
    /// the counts of main memory.
    pub(crate) fn cap_probe(&mut self) -> Probe {
        let p = Probe::at(self.first, self.pool_ptr, self.str_ptr, self.dyn_used);
        self.cap_dyn_peak = self.dyn_used;
        self.cap_var = 0;
        self.cap_var_max = 0;
        for (a, (lo, top)) in p.spans().into_iter().enumerate() {
            for o in marks(lo, top) {
                self.cap_mark(a, lo + o);
            }
        }
        p
    }

    /// What the call used since `p` (a recording's end), or `None` when it
    /// may have been cut short: it wrote an array's last place, came within
    /// the slack of the pool's or the string table's top, or moved `first`.
    pub(crate) fn cap_need(&self, p: &Probe) -> Option<Need> {
        if self.first != p.first {
            return None;
        }
        let mut n = [0i32; 3];
        for (a, (lo, top)) in p.spans().into_iter().enumerate() {
            // (the first marker still there: the call wrote below it only)
            n[a] = marks(lo, top).find(|&o| self.cap_at(a, lo + o) == MARK)?;
        }
        // (main memory: `cap_var` stopped counting at its top)
        if self.cap_var >= VAR_TOP {
            return None;
        }
        let (var, dynp) = (
            self.cap_var as i64,
            (self.cap_dyn_peak - p.dyn0).max(0) as i64,
        );
        let mem = dynp + 2 * var + var / 128 + self.cap_var_max as i64 + MEM_SLACK;
        let need = Need {
            buf: n[0],
            pool: n[1],
            strings: n[2],
            mem: mem.min(i32::MAX as i64) as i32,
        };
        if p.pool + need.pool + POOL_SLACK > pool_size
            || p.str_ptr + need.strings + STR_SLACK > max_strings
        {
            return None;
        }
        Some(need)
    }

    /// Whether a call that uses `n` would pass every one of these overflow
    /// tests if it ran now.
    pub(crate) fn cap_room_ok(&self, n: &Need) -> bool {
        (n.buf == 0 || self.first + n.buf < buf_size)
            && self.pool_ptr + n.pool + POOL_SLACK <= pool_size
            && self.str_ptr + n.strings + STR_SLACK <= max_strings
            && self.hi_mem_min - self.lo_mem_max >= n.mem
    }
}
