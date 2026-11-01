//! READ-TRACK (docs/design/engine-v2/READ-TRACK.md): a line the engine has
//! read but not used yet does not stop the jump.
//!
//! A command that peeks at the next line and then typesets pages (`\lipsum`'s
//! trailing optional argument, a `\newcommand` with an optional argument
//! called alone before `\end{document}`) leaves that line in the buffer,
//! below the macro levels, until the pages are done. A keystroke on the line
//! after the first changes only its unread rest: the buffer from the level's
//! `loc` on, `first`, `last`, the level's `limit` (and the dead `limit`
//! copies of the token lists above it, `crate::iso`) and the file stream's
//! record. The convergence test then fails on every page, and the run
//! typesets them all again.
//!
//! * **The watch** (`changes/checkpoint.ch`): at every checkpoint
//!   ([`Globals::dorm_arm`]) `dorm_level` is set one above the innermost
//!   level reading a file, when every level above it is a token list, and
//!   `pop_watch` to the larger of it and S₀'s `ckpt_arm_level`. `pop_input`
//!   below `pop_watch` counts the watched level's return in `dorm_fired`,
//!   inline (a call there costs every caller of `end_token_list`). Every new
//!   input level and terminal line (`flashtex_level_opened`) and every
//!   printed input line ([`Globals::dorm_note_shown`]: `show_context`, the
//!   diagnostics frames) is counted too, in [`Counts`], never restored. A
//!   record ([`Dorm`]) keeps the counts with the run's epoch, new at every
//!   restore, so two records of one epoch are two points of one run.
//!   `dorm_level`, `pop_watch` and `dorm_fired` are dead for the comparisons
//!   (`incr::dead_word`): only the host reads them.
//! * **The test** (`incr::Obs::test`): where the only differences are the
//!   dormant words ([`Level::set_aside`]) and the stream's record, the old
//!   run's last page checkpoint `k′` before it counts anything
//!   ([`quiet_since`]) is where the run goes on live (a barrier's re-run,
//!   DESIGN.md §5.3), with the pages up to `k′` kept.
//! * **The jump** ([`apply`]): the dormant words get the new run's values in
//!   every kept state from the comparison point on (`Arena::set_from`), and
//!   the records the stream's.
//!
//! The Unicode engine (`flashtex-xetex`'s `dormant.rs`) uses [`Dorm`],
//! [`Counts`] and the word tests here with its own state.
//!
//! `FLASHTEX_DORMANT=off` turns the rule off (the watch is not armed).

use crate::checkpoint::ExtRecord;
use crate::generated::consts::{buf_size, macro_, token_list};
use crate::generated::types::in_state_record;
use crate::generated::Globals;
use crate::statediff::{ScalarSlot, WordDiff};
use std::cell::Cell;

/// Whether the rule is on (`FLASHTEX_DORMANT`, default on).
pub fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("FLASHTEX_DORMANT").map_or(true, |v| v != "off"))
}

/// What a checkpoint's record keeps for the rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dorm {
    /// The run that took the checkpoint (0: none known).
    pub epoch: u64,
    /// The run's counts then ([`Counts`]).
    pub fired: u64,
    pub opens: u64,
    pub shown: u64,
    /// The input level the watch was armed on there (an index into
    /// `input_stack`), -1 for none.
    pub level: i32,
    /// `max_buf_stack` there: `\csname`'s scratch above `first`.
    pub max_buf: i32,
}

flashtex_runtime::codec_struct!(Dorm {
    epoch,
    fired,
    opens,
    shown,
    level,
    max_buf
});

/// The run's counts (`checkpoint::Layer::dorm`; the Unicode engine's own).
/// Never restored: a record's epoch says which run's counts it holds (one
/// engine's records are compared only with its own).
#[derive(Default)]
pub struct Counts {
    epochs: Cell<u64>,
    epoch: Cell<u64>,
    opens: Cell<u64>,
    shown: Cell<u64>,
}

fn bump(c: &Cell<u64>) {
    c.set(c.get() + 1);
}

impl Counts {
    /// An input level was opened, or a terminal line read.
    pub fn note_opened(&self) {
        bump(&self.opens);
    }

    /// An input level's line was printed.
    pub fn note_shown(&self) {
        bump(&self.shown);
    }

    /// A restore: what follows is another run.
    pub fn new_epoch(&self) {
        self.epochs.set(self.epochs.get() + 1);
        self.epoch.set(self.epochs.get());
    }

    /// A checkpoint's record, with the level its watch is armed on (-1:
    /// none), its `max_buf_stack` and its `dorm_fired` (the returns of the
    /// watched level, which `pop_input` counts in the word space: dead for
    /// the comparisons, and one run's states in one epoch).
    pub fn record(&self, level: i32, max_buf: i32, fired: i32) -> Dorm {
        if self.epoch.get() == 0 {
            self.new_epoch();
        }
        Dorm {
            epoch: self.epoch.get(),
            fired: fired as u32 as u64,
            opens: self.opens.get(),
            shown: self.shown.get(),
            level,
            max_buf,
        }
    }
}

/// Whether the old run did nothing between its record `b` and its later
/// record `d` that reads the dormant line of input level `j`: one epoch, so
/// one run, and none of its counts moved; and its `\csname` scratch stayed
/// below `cap` (`buf_size`) with `first` grown by `grow`.
pub fn quiet_since(b: &Dorm, d: &Dorm, j: i32, grow: i64, cap: i64) -> bool {
    b.epoch != 0
        && b.level == j
        && (d.epoch, d.fired, d.opens, d.shown, d.level) == (b.epoch, b.fired, b.opens, b.shown, j)
        && (d.max_buf as i64) + grow.max(0) < cap
}

/// Whether the differing bytes `x` of a word of `buffer` (elements `elem`
/// bytes long, the first one `index`) all lie at or after `loc`.
pub fn buffer_rest_only(x: u64, index: usize, elem: usize, loc: i32) -> bool {
    (0..8 / elem).all(|e| {
        let mask = if elem >= 8 {
            u64::MAX
        } else {
            ((1u64 << (8 * elem)) - 1) << (8 * elem * e)
        };
        (index + e) as i64 >= loc as i64 || x & mask == 0
    })
}

/// Whether every byte that differs (`x`) of the word at byte `rel` of an
/// array of input records (`size` bytes each, from 0) lies in the `limit`
/// field (at `limit_off`) of a record `rec` accepts.
pub fn limit_only(
    x: u64,
    rel: usize,
    size: usize,
    limit_off: usize,
    rec: impl Fn(usize) -> bool,
) -> bool {
    let hi = limit_off + std::mem::size_of::<i32>();
    (0..8).all(|b| {
        let p = rel + b;
        x >> (8 * b) & 0xff == 0 || (rec(p / size) && (limit_off..hi).contains(&(p % size)))
    })
}

/// What the jump writes into the kept states: the set-aside words `aside`
/// (offset, the new run's value), and every word of `buffer` (at byte
/// `buf_off` of the space `live`, elements `elem` bytes long) that holds the
/// new run's line from `loc` up to its `first` (`None`: no such region).
///
/// (Why the whole line: between the two checkpoints the old run's
/// `\csname`s use `buffer` from its own `first` on as scratch. Where the new
/// line is longer, that scratch lies over the new line's tail, and a word of
/// it equal at the comparison point is not among the differing words; the
/// new run never writes below its `first`.)
pub fn jump_words(
    aside: impl Iterator<Item = (usize, u64)>,
    arena: &crate::arena::Arena,
    elem: usize,
    loc: i32,
    first: i32,
) -> Option<Vec<(usize, u64)>> {
    let buf_off = arena.regions.iter().find(|r| r.name == "buffer")?.off;
    let live = arena.bytes();
    let mut out: Vec<(usize, u64)> = aside.collect();
    if first > loc && loc >= 0 {
        let lo = (buf_off + loc as usize * elem) & !7;
        let hi = buf_off + first as usize * elem;
        for off in (lo..hi).step_by(8) {
            if off + 8 > live.len() {
                return None;
            }
            if !out.iter().any(|w| w.0 == off) {
                let v = u64::from_ne_bytes(live[off..off + 8].try_into().expect("8 bytes"));
                out.push((off, v));
            }
        }
    }
    Some(out)
}

/// Whether the jump is worth it: with `at` pages shipped at the comparison
/// point, `to` where the run goes on live, and `total` pages in the old run,
/// it keeps at least as many pages as it typesets again.
///
/// (So it never costs more pages than a run without the rule: between the
/// comparison point and `to` the dormant line differs, so no ordinary test
/// passes there either, and a run without the rule typesets at least to
/// `to`. Where a later ordinary test could pass, as after a dormant line in
/// the middle of a document, a jump that went on live from an early `to`
/// would typeset the rest of the document instead.)
pub fn worth(at: usize, to: usize, total: usize) -> bool {
    total.saturating_sub(to) <= to.saturating_sub(at)
}

/// Whether every byte that differs (`x`) of the word at byte `rel` of the
/// scalar region lies in one of the scalars `slots` (offset, size).
pub fn bytes_within(x: u64, rel: usize, slots: &[(usize, usize)]) -> bool {
    (0..8).all(|b| {
        x >> (8 * b) & 0xff == 0
            || slots
                .iter()
                .any(|&(off, size)| (off..off + size).contains(&(rel + b)))
    })
}

/// How far `first` moved, from its word's old and new values (0 when it
/// did not differ).
pub fn first_grow(word: Option<(u64, u64)>) -> i64 {
    word.map_or(0, |(old, new)| {
        (new as u32 as i32) as i64 - (old as u32 as i32) as i64
    })
}

/// The innermost input level reading a file, as an index into
/// `input_stack`, when every level above it is a token list (and the top,
/// `cur_input`, is one): the level a watch can be armed on.
fn watched_level(g: &Globals) -> Option<i32> {
    if g.cur_input.state_field == token_list {
        (0..g.input_ptr.max(0))
            .rev()
            .find(|&k| g.input_stack[k as usize].state_field != token_list)
    } else {
        None
    }
}

impl Globals {
    /// `begin_file_reading` opened an input level, or `term_input` reads a
    /// line (`changes/checkpoint.ch`): both read `first`.
    #[cold]
    #[inline(never)]
    pub fn flashtex_level_opened(&mut self) {
        if let Some(l) = self.layer_ref() {
            l.dorm.note_opened();
        }
    }

    /// An input level's line is printed (`show_context`, a diagnostics
    /// frame).
    pub fn dorm_note_shown(&self) {
        if let Some(l) = self.layer_ref() {
            l.dorm.note_shown();
        }
    }

    /// At a checkpoint, before its state is taken: arm the watch on the
    /// innermost file level (none while S₀'s level is armed).
    pub(crate) fn dorm_arm(&mut self) {
        let j = match watched_level(self) {
            Some(j) if enabled() && self.ckpt_arm_level == 0 => j,
            _ => -1,
        };
        self.dorm_level = j + 1;
        self.pop_watch = self.ckpt_arm_level.max(self.dorm_level);
    }

    /// The record's part (`checkpoint::Globals::capture_ext`).
    pub(crate) fn dorm_record(&mut self) -> Dorm {
        let (level, max_buf, fired) = (self.dorm_level - 1, self.max_buf_stack, self.dorm_fired);
        self.layer().dorm.record(level.max(-1), max_buf, fired)
    }

    /// A restore: what follows is another run.
    pub(crate) fn dorm_new_epoch(&mut self) {
        self.layer().dorm.new_epoch();
    }
}

/// The live state's dormant level, if it can be one: a file level (not the
/// terminal, a `\read` or `\scantokens`) below token lists only, whose line
/// ends at `first - 1`.
#[derive(Clone, Copy, Debug)]
pub struct Level {
    pub j: i32,
    pub start: i32,
    pub loc: i32,
    pub limit: i32,
}

impl Level {
    pub fn of(g: &Globals) -> Option<Level> {
        let j = watched_level(g)?;
        let r = g.input_stack[j as usize];
        if r.name_field <= 19 || g.first != r.limit_field + 1 || r.loc_field < r.start_field {
            return None;
        }
        Some(Level {
            j,
            start: r.start_field,
            loc: r.loc_field,
            limit: r.limit_field,
        })
    }

    /// Whether word `w` of the comparison's differences is a dormant one:
    /// - a word of `buffer` that differs only at or after `loc`;
    /// - `first` or `last`;
    /// - a word of an input level's record that differs only in `limit`:
    ///   level `j`'s, or that of a token list above it other than a macro's
    ///   (`begin_token_list` never sets it there, and only a macro's is read,
    ///   as `param_start`: tex.web §323, §324). `cur_input` is the top one.
    pub fn set_aside(&self, g: &Globals, layout: &[ScalarSlot], w: &WordDiff) -> bool {
        let x = w.old ^ w.new;
        match (w.region, w.scalar) {
            (_, Some("first" | "last")) => {
                // (only those two scalars' bytes: a word may hold another)
                let in_word: Vec<(usize, usize)> = layout
                    .iter()
                    .filter(|s| matches!(s.name, "first" | "last"))
                    .map(|s| (s.off, s.size))
                    .collect();
                bytes_within(x, w.index, &in_word)
            }
            ("buffer", None) => {
                buffer_rest_only(x, w.index, std::mem::size_of_val(&g.buffer[0]), self.loc)
            }
            ("input_stack", None) => {
                let (_, rel) = g.arena.region_at(w.off);
                let size = std::mem::size_of::<in_state_record>();
                limit_only(x, rel, size, LIMIT, |k| {
                    let k = k as i32;
                    k >= self.j
                        && k < g.input_ptr
                        && (k == self.j || dead_limit(&g.input_stack[k as usize]))
                })
            }
            (_, Some("cur_input")) => {
                let (_, rel) = g.arena.region_at(w.off);
                let Some(s) = layout.iter().find(|s| s.name == "cur_input") else {
                    return false;
                };
                rel >= s.off
                    && dead_limit(&g.cur_input)
                    && limit_only(x, rel - s.off, s.size.div_ceil(8) * 8, LIMIT, |k| k == 0)
            }
            _ => false,
        }
    }
}

/// `limit`'s place in an input record.
const LIMIT: usize = std::mem::offset_of!(in_state_record, limit_field);

/// A token list other than a macro's: its `limit` is a stale copy.
fn dead_limit(r: &in_state_record) -> bool {
    r.state_field == token_list && r.index_field != macro_
}

/// The convergence test of a dormant line (`incr::same_words`): the level,
/// and the differing words it set aside.
pub struct Aside {
    pub level: Level,
    pub words: Vec<WordDiff>,
}

impl Aside {
    /// How far `first` moves (the new run's minus the old run's).
    pub fn grow(&self) -> i64 {
        first_grow(
            self.words
                .iter()
                .find(|w| w.scalar == Some("first"))
                .map(|w| (w.old, w.new)),
        )
    }
}

/// What the jump puts into the kept states (`incr::Obs::test`).
#[derive(Clone, Debug)]
pub struct Jump {
    /// The dormant words' offsets and the new run's values.
    pub words: Vec<(usize, u64)>,
    /// The stream whose record differs, and the new run's record of it.
    pub stream: Option<(usize, crate::system::FileSnap)>,
    /// The old page checkpoint the run goes on from.
    pub to: crate::arena::CheckpointId,
}

/// The old run's last page checkpoint after `old` (record `o`), among
/// `pages` (its page checkpoints), before it reads the dormant line of
/// level `j` ([`quiet_since`]). `None`: there is none after `old`, or `o`
/// was not watching level `j`.
pub fn jump_point(
    g: &Globals,
    old: crate::arena::CheckpointId,
    o: &ExtRecord,
    pages: &std::collections::HashSet<crate::arena::CheckpointId>,
    j: i32,
    grow: i64,
) -> Option<crate::arena::CheckpointId> {
    let b = o.dorm;
    let ids = g.pending_ids();
    let at = ids.iter().position(|&i| i == old)?;
    let mut best = None;
    for &c in &ids[at + 1..] {
        let Some(rc) = g.pending_record(c) else { break };
        let d = rc.dorm;
        if !quiet_since(&b, &d, j, grow, buf_size as i64) {
            if std::env::var_os("FLASHTEX_INCR_DEBUG").is_some() {
                eprintln!("[dormant] from {old} ({b:?}): stops at {c} ({d:?})");
            }
            break;
        }
        if pages.contains(&c) {
            best = Some(c);
        }
    }
    best
}

/// After the jump to the old run (`Globals::redo_to_remapped_with`, `old`
/// its convergence point): the dormant words are the new run's in every kept
/// state from `old` on, and so is the stream's record.
pub fn apply(g: &mut Globals, old: crate::arena::CheckpointId, dj: &Jump) -> Result<(), String> {
    g.arena.set_from(old, &dj.words)?;
    if let Some((i, snap)) = &dj.stream {
        g.patch_records_after(old, &mut |r: &mut ExtRecord| {
            if let Some(f) = r.files.get_mut(*i) {
                *f = snap.clone();
            }
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_buffer_word_differs_only_after_loc() {
        // elements 10 and 11 of 4 bytes: the second one differs
        let x = 0xff_u64 << 32;
        assert!(buffer_rest_only(x, 10, 4, 11));
        assert!(!buffer_rest_only(x, 10, 4, 12));
        assert!(buffer_rest_only(0, 10, 4, 99));
    }

    #[test]
    fn only_the_limit_field_may_differ() {
        let size = std::mem::size_of::<in_state_record>();
        // a word at the start of record 1 that covers its `limit`
        let base = size;
        let rel = (base + LIMIT) & !7;
        let at = base + LIMIT - rel;
        let x = 0xff_u64 << (8 * at);
        assert!(limit_only(x, rel, size, LIMIT, |k| k == 1));
        assert!(!limit_only(x, rel, size, LIMIT, |k| k == 0));
        // a byte outside the field
        let other = (base + std::mem::offset_of!(in_state_record, loc_field)) & !7;
        let y = 0xff_u64 << (8 * (base + std::mem::offset_of!(in_state_record, loc_field) - other));
        assert!(!limit_only(y, other, size, LIMIT, |_| true));
    }

    #[test]
    fn a_quiet_record_is_one_of_the_same_run_with_nothing_counted() {
        let b = Dorm {
            epoch: 7,
            fired: 3,
            opens: 2,
            shown: 1,
            level: 0,
            max_buf: 100,
        };
        assert!(quiet_since(&b, &b, 0, 5, 200));
        assert!(!quiet_since(&b, &b, 1, 5, 200));
        assert!(!quiet_since(&b, &Dorm { epoch: 8, ..b }, 0, 5, 200));
        assert!(!quiet_since(&b, &Dorm { fired: 4, ..b }, 0, 5, 200));
        assert!(!quiet_since(&b, &Dorm { opens: 3, ..b }, 0, 5, 200));
        assert!(!quiet_since(&b, &Dorm { shown: 2, ..b }, 0, 5, 200));
        assert!(!quiet_since(&b, &b, 0, 100, 200));
        assert!(!quiet_since(
            &Dorm { epoch: 0, ..b },
            &Dorm { epoch: 0, ..b },
            0,
            5,
            200
        ));
        // b4: compared at 12 of 19 pages, live from 18; a line in the middle
        // of c1-lipsum: compared at 4, live from 6
        assert!(worth(12, 18, 19));
        assert!(!worth(4, 6, 19));
        assert!(worth(4, 6, 8));
        // a word holding `first` (bytes 8..12) and another scalar (12..16)
        assert!(bytes_within(0xff, 8, &[(8, 4)]));
        assert!(!bytes_within(0xff << 32, 8, &[(8, 4)]));
        assert!(bytes_within(0xff << 32, 8, &[(8, 4), (12, 4)]));
        assert_eq!(first_grow(Some((30, 31))), 1);
        assert_eq!(first_grow(None), 0);
    }
}
