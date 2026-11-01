//! READ-TRACK for the Unicode engine (docs/design/engine-v2/READ-TRACK.md;
//! the pdfTeX engine's `flashtex_engine::dormant`, whose records, counts and
//! word tests this uses): a line the engine has read but not used yet does
//! not stop the jump.
//!
//! * **The watch** (`changes/checkpoint.ch`): at every page checkpoint
//!   ([`Globals::dorm_arm`], `crate::ckpt`'s `take_page`) `dorm_level` is set
//!   one above the innermost level reading a file, when every level above it
//!   is a token list; `pop_input` below `pop_watch` counts its return in
//!   `dorm_fired` (inline; dead for the comparisons). New input levels,
//!   terminal lines and the lines `show_context` prints are counted too, in
//!   `Host::dorm`; each page checkpoint keeps the counts (`PageCk::dorm`).
//! * **The test** (`crate::conv`'s `conv_check`): where the only differences
//!   are the dormant words ([`Level::set_aside`]), the jump's target is the
//!   old run's last page checkpoint before it counts anything ([`target`]).
//! * **The jump** (`conv_jump`): the dormant words get the new run's values
//!   in every kept state from the comparison point on (`Arena::set_from`).
//!   (The reader of the edited file keeps no copy of the line: its record is
//!   compared as for any convergence.)
//!
//! `FLASHTEX_DORMANT=off` turns the rule off.

use crate::generated::consts::{buf_size, macro_, token_list};
use crate::generated::types::in_state_record;
use crate::generated::Globals;
use crate::statediff::{ScalarSlot, WordDiff};
use flashtex_engine::dormant::{
    buffer_rest_only, bytes_within, first_grow, limit_only, quiet_since,
};

/// The innermost input level reading a file, as an index into
/// `input_stack`, when every level above it is a token list (and the top,
/// `cur_input`, is one).
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
    /// line: both read `first`.
    #[cold]
    #[inline(never)]
    pub fn flashtex_level_opened(&mut self) {
        self.host.dorm.note_opened();
    }

    /// `show_context` prints a file level's line.
    #[cold]
    #[inline(never)]
    pub fn flashtex_level_shown(&mut self) {
        self.host.dorm.note_shown();
    }

    /// At a page checkpoint, before its state is taken: arm the watch on
    /// the innermost file level (none while S₀'s level is armed).
    pub(crate) fn dorm_arm(&mut self) {
        let j = match watched_level(self) {
            Some(j) if flashtex_engine::dormant::enabled() && self.ckpt_arm_level == 0 => j,
            _ => -1,
        };
        self.dorm_level = j + 1;
        self.pop_watch = self.ckpt_arm_level.max(self.dorm_level);
    }

    /// The page checkpoint's record of the counts.
    pub(crate) fn dorm_record(&self) -> flashtex_engine::dormant::Dorm {
        self.host.dorm.record(
            (self.dorm_level - 1).max(-1),
            self.max_buf_stack,
            self.dorm_fired,
        )
    }
}

/// The live state's dormant level, if it can be one: a file level (not the
/// terminal, a `\read` or `\scantokens`) below token lists only, whose line
/// ends at `first - 1`.
#[derive(Clone, Copy, Debug)]
pub struct Level {
    pub j: i32,
    pub loc: i32,
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
            loc: r.loc_field,
        })
    }

    /// Whether word `w` of the comparison's differences is a dormant one
    /// (the pdfTeX engine's `Level::set_aside`): a word of `buffer` that
    /// differs only at or after `loc`; `first` or `last`; a word of an input
    /// level's record that differs only in `limit`, level `j`'s or that of a
    /// token list above it other than a macro's (`cur_input` the top one).
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

/// How far `first` moves among the set-aside words (new minus old).
pub fn grow(words: &[WordDiff]) -> i64 {
    first_grow(
        words
            .iter()
            .find(|w| w.scalar == Some("first"))
            .map(|w| (w.old, w.new)),
    )
}

/// What the jump puts into the kept states (`crate::conv`).
#[derive(Clone, Debug)]
pub struct Jump {
    /// The dormant words' offsets and the new run's values.
    pub words: Vec<(usize, u64)>,
    /// The old page checkpoint (an index into `Old::pages`) the run goes on
    /// from.
    pub to: usize,
}

/// The last of the old run's page checkpoints `pages` after `i` before it
/// reads the dormant line of level `j` (`quiet_since`), `first` grown by
/// `grow`.
pub fn target(pages: &[crate::conv::OldPage], i: usize, j: i32, grow: i64) -> Option<usize> {
    let b = pages[i].dorm;
    let mut best = None;
    for (t, p) in pages.iter().enumerate().skip(i + 1) {
        if !quiet_since(&b, &p.dorm, j, grow, buf_size as i64) {
            break;
        }
        best = Some(t);
    }
    best
}
