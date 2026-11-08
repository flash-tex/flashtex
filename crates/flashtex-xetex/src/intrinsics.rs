//! XeTeX's guarded intrinsics (DESIGN.md §5.6 item 4, decision D9; the
//! pdfTeX engine's `flashtex_engine::intrinsics`, ported): a registered
//! macro without parameters that `big_switch` itself expands is recorded
//! once and afterwards *replayed*, its assignments and groups made directly
//! through TeX's own `eq_define`, `geq_define`, `eq_word_define`,
//! `geq_word_define`, `new_save_level` and `unsave`, for as long as
//! everything the recorded run read still holds the value it read.
//!
//! The target is hyperref's `\pdfstringdefPreHook`, which every
//! `\pdfstringdef` runs (twice a page: the page label and the anchor) and
//! which siunitx fills with a dozen `\DeclareCommandCopy` and a
//! `\cs_set_eq:Nc` for each unit.
//!
//! # Recording (`changes/intrinsics.ch` has every hook)
//!
//! A recording is the normal expansion and execution of the body, observed:
//!
//! * **reads** (the *read set*): the meaning of every control sequence
//!   `get_next` delivers (only "not `\outer`, not `#`" where its caller looks
//!   at the token alone: a parameter text, a body not expanded, a macro's
//!   argument, the control sequence being defined); `\csname`/`\ifcsname`
//!   look-ups, with the name found at the location; the internal quantities
//!   `scan_something_internal` fetches; character codes; the register
//!   `\advance` and kin read; when a token list or name is printed
//!   (`\string`, `\meaning`, `\detokenize`, `\strcmp`), the category code of
//!   each one-character control sequence printed (`print_cs`, read there,
//!   not all 0x110000 of them) and each name printed; `\escapechar`; the
//!   macro's own meaning. The first access to each `eqtb` entry counts: a
//!   read after the recording's own write is not a dependency unless an
//!   `\endgroup` brought back what the entry held before the write.
//! * **writes**, in order, the *operations*: `Def`, `Word`, `Begin`, `End`.
//!   A list made for the definition (`\def`, `\edef`, a braced token
//!   assignment; reference count null) is kept by its tokens and made again
//!   on every replay; a list shared with the control sequence just read
//!   (`\let`, `\futurelet`) as that control sequence, whose meaning a replay
//!   shares as it is then. No `mem` address is ever kept.
//! * **purity**: an allowlist of commands, expandable primitives and
//!   internal quantities, with XeTeX's codes (from `src/generated/consts.rs`;
//!   anything XeTeX-specific not on them abandons the recording), as narrow
//!   as the pdfTeX engine's. Anything else (typesetting, boxes, glue,
//!   dimensions, fonts, files and `\read`, marks, sparse registers,
//!   `\afterassignment`, `\aftergroup`, messages, errors, output of any
//!   kind, a new control sequence, a write to a tracing parameter, reading a
//!   token from below the body's input level, closing a group or conditional
//!   it did not open) abandons it.
//!
//! A recording ends at `big_switch` once every input level from the body's
//! on is used up, and is kept only if the group level, conditionals,
//! `align_state`, strings and hash, errors, mode, nest, tail, scanner,
//! `\afterassignment` and the log and terminal are as it found them.
//!
//! # The guard
//!
//! At every offer each kept variant of the macro (up to [`MAX_VARIANTS`],
//! least recently used dropped) is compared **by value, in full**: every
//! read meaning (a macro or token list by its tokens), integer and name,
//! and the context: the mode, `align_state`, `par_token` and
//! `scanner_status` it was recorded in. Before that the call's own
//! preconditions: no parameters, `\globaldefs=0`, no `\afterassignment`,
//! the tracing parameters that print what a call does all `<= 0`. An
//! `\ifcsname` that finds nothing abandons the recording (no key for
//! absence).
//!
//! # Storage
//!
//! The recordings live in [`Host`](crate::state::Host), outside the word
//! space: an edit's restore rolls the word space back to before the edited
//! page, and its calls must still find them. They stay sound because the
//! guard compares every value. A recording in progress belongs to the
//! state it started in and is abandoned by any restore.
//!
//! # Verification
//!
//! `FLASHTEX_INTRINSICS=verify`: every call the guard admits is checked
//! (`crate::intrinsics_verify`): checkpoint C, replay the admitted variant,
//! capture, restore C, run the real path (recorded, which must stay pure),
//! compare. `FLASHTEX_INTRINSICS_FAULT` breaks the replay or the read set on
//! purpose (`drop-last`, `skip-read`, `no-ref`), to show that verify sees it.

// (TeX's names for its codes, matched as they are spelt in xetex.web)
#![allow(non_upper_case_globals)]
use crate::generated::consts::{
    advance, assign_dimen, assign_int, assign_toks, begin_group, box_ref, call, car_ret,
    cat_code_base, char_def_code, char_given, convert, cs_name, def, def_code, divide, eTeX_expr,
    eTeX_revision_code, eTeX_version_code, end_group, end_match_token, end_template, eqtb_size,
    eqtb_top, escape_char_code, etex_pen_base, expand_after, expanded_code, fi_or_else,
    font_id_base, frozen_control_sequence, global_defs_code, glue_base, glue_ref, hash_base,
    if_case_code, if_cat_code, if_char_code, if_cs_code, if_def_code, if_false_code, if_int_code,
    if_odd_code, if_test, if_true_code, ifx_code, ignore_spaces, int_base, last_item, left_brace,
    let_, lo_mem_stat_max, local_base, long_outer_call, mac_param, matching, math_given,
    max_non_prefixed_command, meaning_code, mem_bot, mmode, multiply, no_expand, null_cs,
    number_code, outer_call, pdf_strcmp_code, prefix, protected_token, register, relax,
    right_brace, roman_numeral_code, semi_simple_group, set_font, shape_ref, shorthand_def,
    show_tokens, simple_group, single_base, spacer, string_code, tab_mark, the, token_list,
    toks_def_code, toks_register, top_bot_mark, tracing_assigns_code, tracing_char_sub_def_code,
    tracing_commands_code, tracing_groups_code, tracing_ifs_code, tracing_lost_chars_code,
    tracing_macros_code, tracing_nesting_code, tracing_online_code, tracing_output_code,
    tracing_pages_code, tracing_paragraphs_code, tracing_restores_code, tracing_scan_tokens_code,
    tracing_stack_levels_code, tracing_stats_code, undefined_control_sequence, undefined_cs,
    unless_code, v_template, vmode, XeTeX_revision_code, XeTeX_version_code,
};
use crate::Globals;
use std::collections::{BTreeMap, HashMap, HashSet};

const NAME: &[u8] = b"pdfstringdefPreHook";
/// Recordings kept per macro (contexts it is called in).
pub const MAX_VARIANTS: usize = 4;
const MAX_OPS: usize = 50_000;
const MAX_READS: usize = 200_000;
/// Tokens one recording may hold (its reads' and its definitions' lists).
const MAX_TOKENS: usize = 4_000_000;
/// Abandoned recordings in a run before a macro is left alone in it.
const ABORT_BUDGET: u32 = 4;
/// Recordings in a run.
const RECORD_BUDGET: u32 = 64;
// xetex.web's `null` is negative, unlike a C null pointer.
pub(crate) const NULL: i32 = -0x0fff_ffff;

/// The tracing parameters that print what a call does (its expansions,
/// commands, assignments, groups, conditionals, restores): a call is
/// offered only while all are `<= 0`, and `\tracingstats` `<= 1` (LaTeX's
/// 1 prints the totals at the end; 2 prints the memory in use at every
/// `\shipout`). The others print only for what a recording refuses
/// (characters, paragraphs, pages, the output routine), and
/// `\tracingonline` only says where.
const TRACING_GUARD: [i32; 10] = [
    tracing_macros_code,
    tracing_commands_code,
    tracing_restores_code,
    tracing_assigns_code,
    tracing_groups_code,
    tracing_ifs_code,
    tracing_scan_tokens_code,
    tracing_nesting_code,
    tracing_stack_levels_code,
    tracing_char_sub_def_code,
];

/// Every tracing parameter: a recording that writes one is abandoned
/// (#2141 item 4).
const TRACING_ALL: [i32; 16] = [
    tracing_macros_code,
    tracing_commands_code,
    tracing_restores_code,
    tracing_assigns_code,
    tracing_groups_code,
    tracing_ifs_code,
    tracing_scan_tokens_code,
    tracing_nesting_code,
    tracing_stack_levels_code,
    tracing_char_sub_def_code,
    tracing_stats_code,
    tracing_online_code,
    tracing_output_code,
    tracing_pages_code,
    tracing_paragraphs_code,
    tracing_lost_chars_code,
];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    #[default]
    Off,
    /// Replay what the guard admits.
    On,
    /// Check every call the guard admits against the real path, which the
    /// run goes on from.
    Verify,
}

/// `FLASHTEX_INTRINSICS`: `on`, `verify` (`report`, slice 1's name), else
/// off.
pub fn mode() -> Mode {
    match std::env::var("FLASHTEX_INTRINSICS").as_deref() {
        Ok("on") => Mode::On,
        Ok("verify" | "report") => Mode::Verify,
        _ => Mode::Off,
    }
}

/// Faults injected by `FLASHTEX_INTRINSICS_FAULT`, to show that verify
/// sees them (never set outside those tests).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Fault {
    #[default]
    None,
    /// a replay leaves out the last operation
    DropLast,
    /// a recording leaves out the last macro meaning it read (other than
    /// the call's own)
    SkipRead,
    /// a replayed `\let` shares a list without taking a reference
    NoRef,
}

fn fault() -> Fault {
    match std::env::var("FLASHTEX_INTRINSICS_FAULT").as_deref() {
        Ok("drop-last") => Fault::DropLast,
        Ok("skip-read") => Fault::SkipRead,
        Ok("no-ref") => Fault::NoRef,
        _ => Fault::None,
    }
}

/// One definition or group boundary of a recorded run: `Def(p, t, value,
/// global)`, `Word(p, w, global)`, `new_save_level(c)`, `unsave`.
#[derive(Clone)]
pub(crate) enum Op {
    Def(i32, i32, Val, bool),
    Word(i32, i32, bool),
    Begin(i32),
    End,
}

/// A defined value: a list made for the definition by its tokens, the
/// control sequence whose list a `\let` shares, or any other `equiv`.
#[derive(Clone)]
pub(crate) enum Val {
    Plain(i32),
    Tokens(Vec<i32>),
    Cs(i32),
}

/// One read of the read set.
#[derive(Clone)]
enum Read {
    /// `eqtb[p]` held `eq_type` `t` and this `equiv`.
    Meaning(i32, i32, Equiv),
    /// `eqtb[p]` was neither `\outer` nor a parameter character.
    Plain(i32),
    /// `eqtb[p].int`.
    Word(i32, i32),
    /// The name at location `p` (`None`: none), which `id_lookup` finds
    /// there and only there.
    Name(i32, Option<Vec<i32>>),
    /// `eqtb[p]`, read as a `\let`'s source (or a space before it), was of
    /// this [`let_class`]: a replay's `\let` shares the source's meaning as
    /// it is then (`Val::Cs`), so only the class decides what the run did,
    /// unless it reads the copy later (`Rec::taint`).
    Class(i32, i32),
    /// `\csname` or `\ifcsname` found `eqtb[p]` undefined (or not): all
    /// they look at.
    Defined(i32, bool),
}

/// What a `\let` looks at in its source's meaning (`prefixed_command`):
/// whether it is a space (skipped before the source), a parameter
/// character or `\outer` (what weak reads of the copy look at), or a
/// sparse register (`add_sa_ref`, which a recording refuses).
fn let_class(t: i32, e: i32) -> i32 {
    match t {
        spacer | mac_param | outer_call | long_outer_call => t,
        register | toks_register if !(mem_bot..=lo_mem_stat_max).contains(&e) => -1,
        _ => 0,
    }
}

#[derive(Clone)]
enum Equiv {
    Value(i32),
    /// A list, by its tokens (after the reference count).
    Tokens(Vec<i32>),
}

/// The state a call is recorded in, beyond its read set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Ctx {
    mode: i32,
    /// `align_state`; `None` in a kept variant whose run met no `&`, `\span`
    /// or `\cr` (`tab_mark`, `car_ret`): only those read it (`get_next`'s
    /// "an alignment entry has just ended"), the rest counts braces, which
    /// the run leaves balanced.
    align: Option<i32>,
    par_token: i32,
    scanner: i32,
}

impl Ctx {
    /// Does a kept variant's context hold in `now`?
    fn holds(&self, now: &Ctx) -> bool {
        self.mode == now.mode
            && self.par_token == now.par_token
            && self.scanner == now.scanner
            && self.align.is_none_or(|a| now.align == Some(a))
    }
}

/// A kept recording.
pub(crate) struct Entry {
    id: u64,
    cs: i32,
    ctx: Ctx,
    reads: Vec<Read>,
    pub(crate) ops: Vec<Op>,
    /// The store's clock at its last use (least recently used goes first).
    used: u64,
}

impl Entry {
    /// The `eqtb` entries it reads or defines, and the `\let` sources.
    fn named(&self) -> Vec<i32> {
        let mut v: Vec<i32> = self
            .reads
            .iter()
            .filter_map(|r| match r {
                Read::Meaning(p, ..)
                | Read::Plain(p)
                | Read::Word(p, _)
                | Read::Class(p, _)
                | Read::Defined(p, _) => Some(*p),
                Read::Name(..) => None,
            })
            .collect();
        for o in &self.ops {
            match o {
                Op::Def(p, _, Val::Cs(c), _) => v.extend([*p, *c]),
                Op::Def(p, ..) | Op::Word(p, ..) => v.push(*p),
                _ => {}
            }
        }
        v
    }
}

/// The recordings, kept across restores and runs (the guard checks every
/// value, so a recording made in any state is sound in any other).
#[derive(Default)]
pub struct Store {
    entries: Vec<Entry>,
    clock: u64,
    next_id: u64,
}

#[derive(Clone, Copy)]
enum Seen {
    Dep,
    Weak,
    /// Read first as a `\let`'s source: `Read::Class`.
    LetSrc,
    /// Found first by `\csname` or `\ifcsname`: `Read::Defined`.
    Looked,
    /// Written first; the entry's word before the write.
    Written(u64),
}

/// A recording in progress.
struct Rec {
    cs: i32,
    /// The input level the body is pushed at.
    base: i32,
    level: i32,
    cond: i32,
    ctx: Ctx,
    /// What a kept call leaves as it found it.
    marks: Vec<(&'static str, i64)>,
    reads: Vec<Read>,
    seen: HashMap<i32, Seen>,
    names: HashSet<i32>,
    ops: Vec<Op>,
    tokens: usize,
    /// `get_next` delivered a `tab_mark` or `car_ret` (`Ctx::align`).
    align_seen: bool,
    /// A `\let`'s copy: the word it made (`eq_type`, `equiv`) and the full
    /// read of the meaning it copied from outside the run (`None`: the
    /// source's meaning is already a dependency, or the run's own). Reading
    /// the copy while it holds that word makes the read a dependency.
    taint: HashMap<i32, ((i32, i32), Option<Read>)>,
    /// The `\let` sources whose taint has been added.
    tainted: HashSet<i32>,
    /// A check of the admitted variant `id`: the replay's state.
    verify: Option<(u64, crate::intrinsics_verify::Pending)>,
}

/// The run's counts, for `FLASHTEX_INTRINSICS_STATS`.
#[derive(Default)]
pub(crate) struct Stats {
    offers: u64,
    admitted: u64,
    replayed: u64,
    replayed_ops: u64,
    replay_ns: u64,
    guard_ns: u64,
    started: u64,
    committed: u64,
    committed_ops: u64,
    pub(crate) checked: u64,
    pub(crate) identical: u64,
    pub(crate) differing: u64,
    rejected: BTreeMap<&'static str, u64>,
    missed: BTreeMap<&'static str, u64>,
    abandoned: BTreeMap<&'static str, u64>,
    pub(crate) classes: BTreeMap<String, u64>,
    unverified: BTreeMap<&'static str, u64>,
    pub(crate) details: Vec<String>,
}

#[derive(Default)]
pub struct Report {
    /// The run's mode, kept outside the word space: a restore puts it back
    /// into `intr_on` (`intr_after_restore`).
    pub mode: Mode,
    fault: Fault,
    store: Store,
    rec: Option<Rec>,
    pub(crate) stats: Stats,
    /// This run records no more (abandoned too often, or for a reason
    /// that will not change).
    norec: bool,
    aborts: u32,
    records: u32,
    /// The verifier's own checkpoints not yet dropped (`drop_checkpoint`'s
    /// precondition).
    pub(crate) verifier_cks: Vec<flashtex_engine::arena::CheckpointId>,
}

impl Report {
    /// Is `id` the verifier's (and no longer so)?
    pub(crate) fn take_verifier_checkpoint(
        &mut self,
        id: flashtex_engine::arena::CheckpointId,
    ) -> bool {
        let n = self.verifier_cks.len();
        self.verifier_cks.retain(|&k| k != id);
        self.verifier_cks.len() < n
    }
}

/// `FLASHTEX_INTRINSICS_DEBUG`: say on stderr why calls are not replayed
/// and recordings are abandoned.
fn debug() -> bool {
    static D: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *D.get_or_init(|| std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some())
}

fn bump<K: Ord>(m: &mut BTreeMap<K, u64>, k: K) {
    *m.entry(k).or_default() += 1;
}

/// Glue, the paragraph shape, e-TeX's penalty shapes, box registers and
/// font identifiers hold pointers a recording does not keep, or are
/// written by routines it does not see.
fn region_ok(p: i32) -> bool {
    p > 0
        && p <= eqtb_top
        && !(font_id_base..=undefined_control_sequence).contains(&p)
        && !(glue_base..=local_base).contains(&p)
        // (penalty shapes, box registers, the current and math fonts)
        && !(etex_pen_base..cat_code_base).contains(&p)
}

/// An integer or dimension (above `eqtb_size` are control sequences again,
/// web2c's `hash_extra`).
fn is_word(p: i32) -> bool {
    (int_base..=eqtb_size).contains(&p)
}

/// A location whose name `id_lookup` finds: the hash and its extension.
fn is_hashed(p: i32) -> bool {
    (hash_base..frozen_control_sequence).contains(&p) || (eqtb_size < p && p <= eqtb_top)
}

impl Globals {
    // -- set-up ----------------------------------------------------------

    /// The run's mode. The recordings stay (a resident engine's runs share
    /// them), unless the intrinsics are off.
    pub fn set_intrinsics(&mut self, mode: Mode) {
        self.intr_abandon("unfinished");
        let r = &mut self.host.intrinsics;
        let store = if mode == Mode::Off {
            Store::default()
        } else {
            std::mem::take(&mut r.store)
        };
        *r = Report {
            mode,
            fault: fault(),
            store,
            ..Report::default()
        };
        self.intr_on = mode != Mode::Off;
        self.intr_rec_on = false;
        self.intr_weak = false;
        self.intr_let = false;
        self.intr_at_switch = false;
    }

    /// After a restore (`Globals::restore`): the restored scalars hold the
    /// flags of the run that took the checkpoint (S₀ may come from an
    /// earlier run or a file), so they are put back, and a recording in
    /// progress belongs to a state that is gone. The kept recordings stay.
    pub fn intr_after_restore(&mut self) {
        self.intr_abandon("restored");
        self.intr_on = self.host.intrinsics.mode != Mode::Off;
        self.intr_rec_on = false;
        self.intr_weak = false;
        self.intr_let = false;
        self.intr_at_switch = false;
    }

    fn intr_abandon(&mut self, why: &'static str) {
        let Some(rec) = self.host.intrinsics.rec.take() else {
            return;
        };
        self.intr_rec_on = false;
        bump(&mut self.host.intrinsics.stats.abandoned, why);
        if let Some((_, p)) = rec.verify {
            bump(&mut self.host.intrinsics.stats.unverified, why);
            self.drop_checkpoint(p.ck());
        }
    }

    // -- the offer -------------------------------------------------------

    /// `macro_call` for a macro `big_switch`'s `get_x_token` expands, before
    /// anything of it is scanned: true when it was replayed.
    pub fn flashtex_intr_call(&mut self) -> bool {
        let cs = self.cur_cs;
        if !self.intr_is_named(cs, NAME) {
            return false;
        }
        if self.intr_rec_on || self.host.intrinsics.rec.is_some() {
            bump(&mut self.host.intrinsics.stats.rejected, "Nested");
            return false;
        }
        self.host.intrinsics.stats.offers += 1;
        if debug() {
            eprintln!(
                "intrinsics: offer {} at line {} mode {} level {}",
                self.host.intrinsics.stats.offers,
                self.line,
                self.cur_list.mode_field,
                self.cur_level
            );
        }
        let mut p = self.mem[self.cur_chr as usize].hh().rh();
        if self.mem[p as usize].hh().lh() == protected_token {
            p = self.mem[p as usize].hh().rh();
        }
        if self.mem[p as usize].hh().lh() != end_match_token {
            bump(&mut self.host.intrinsics.stats.rejected, "Parameters");
            return false;
        }
        let refused = if self.intr_int(global_defs_code) != 0 {
            Some("GlobalDefs")
        } else if self.after_token != 0 {
            Some("AfterAssignment")
        } else if TRACING_GUARD.iter().any(|&c| self.intr_int(c) > 0)
            || self.intr_int(tracing_stats_code) > 1
        {
            Some("Tracing")
        } else {
            None
        };
        if let Some(why) = refused {
            bump(&mut self.host.intrinsics.stats.rejected, why);
            return false;
        }
        let t0 = std::time::Instant::now();
        let hit = self.intr_admit(cs);
        self.host.intrinsics.stats.guard_ns += t0.elapsed().as_nanos() as u64;
        match hit {
            Ok(i) => {
                let r = &mut self.host.intrinsics;
                r.stats.admitted += 1;
                r.store.clock += 1;
                let e = &mut r.store.entries[i];
                e.used = r.store.clock;
                let id = e.id;
                if r.mode == Mode::Verify {
                    let ops = e.ops.clone();
                    let named = e.named();
                    let pending = self.intr_verify_begin(&ops, named);
                    self.intr_rec_start(cs, Some((id, pending)));
                    return false;
                }
                self.intr_replay_entry(i);
                true
            }
            Err(why) => {
                let r = &mut self.host.intrinsics;
                bump(&mut r.stats.missed, why);
                if r.norec || r.records >= RECORD_BUDGET {
                    bump(&mut r.stats.rejected, "NoRecording");
                    return false;
                }
                r.records += 1;
                self.intr_rec_start(cs, None);
                false
            }
        }
    }

    fn intr_ctx(&self) -> Ctx {
        Ctx {
            mode: self.cur_list.mode_field.abs(),
            align: Some(self.align_state),
            par_token: self.par_token,
            scanner: self.scanner_status,
        }
    }

    /// The guard: the most recently used variant of `cs` whose context and
    /// read set hold now, or why none.
    fn intr_admit(&self, cs: i32) -> Result<usize, &'static str> {
        let ctx = self.intr_ctx();
        let entries = &self.host.intrinsics.store.entries;
        let mut order: Vec<usize> = (0..entries.len())
            .filter(|&i| entries[i].cs == cs)
            .collect();
        order.sort_unstable_by_key(|&i| std::cmp::Reverse(entries[i].used));
        let mut why = "NotRecorded";
        for i in order {
            let e = &entries[i];
            if !e.ctx.holds(&ctx) {
                if debug() {
                    eprintln!(
                        "intrinsics: variant {} context {:?}, now {ctx:?}",
                        e.id, e.ctx
                    );
                }
                why = "Context";
                continue;
            }
            if let Err((w, p)) = self.intr_reads_hold(&e.reads) {
                if debug() {
                    eprintln!(
                        "intrinsics: variant {} {w} of \\{} ({p})",
                        e.id,
                        self.intr_cs_name(p)
                    );
                }
                why = w;
                continue;
            }
            // expanding the armed control sequence arms a checkpoint
            // (changes/checkpoint.ch), which a replay would not
            let arm = self.ckpt_arm_cs;
            if arm != 0
                && e.reads
                    .iter()
                    .any(|r| matches!(r, Read::Meaning(p, ..) if *p == arm))
            {
                why = "Checkpoint";
                continue;
            }
            return Ok(i);
        }
        Err(why)
    }

    /// Every read of `reads` holds. (The macro bodies are compared after the
    /// rest, several lists a step at a time: each is a chain of dependent
    /// loads, and walking them side by side overlaps their cache misses.)
    fn intr_reads_hold(&self, reads: &[Read]) -> Result<(), (&'static str, i32)> {
        let mut lists: Vec<(i32, i32, &[i32])> = Vec::new();
        for r in reads {
            match r {
                Read::Meaning(p, t, v) => {
                    let w = self.eqtb[(p - 1) as usize];
                    let same = w.hh_b0() == *t
                        && match v {
                            Equiv::Value(e) => w.hh_rh() == *e,
                            Equiv::Tokens(ts) => {
                                lists.push((*p, w.hh_rh(), ts));
                                true
                            }
                        };
                    if !same {
                        return Err(("Meaning", *p));
                    }
                }
                Read::Plain(p) => {
                    let t = self.eqtb[(p - 1) as usize].hh_b0();
                    if t == mac_param || t == outer_call || t == long_outer_call {
                        return Err(("Meaning", *p));
                    }
                }
                Read::Word(p, w) => {
                    if self.eqtb[(p - 1) as usize].int() != *w {
                        return Err(("Word", *p));
                    }
                }
                Read::Defined(p, d) => {
                    if (self.eqtb[(p - 1) as usize].hh_b0() != undefined_cs) != *d {
                        return Err(("Meaning", *p));
                    }
                }
                Read::Class(p, k) => {
                    let w = self.eqtb[(p - 1) as usize];
                    if let_class(w.hh_b0(), w.hh_rh()) != *k {
                        return Err(("Meaning", *p));
                    }
                }
                Read::Name(p, n) => {
                    let same = match (self.intr_str(*p), n) {
                        (None, None) => true,
                        (Some(r), Some(n)) => {
                            r.len() == n.len() && r.zip(n).all(|(i, &c)| self.str_pool[i] == c)
                        }
                        _ => false,
                    };
                    if !same {
                        return Err(("Name", *p));
                    }
                }
            }
        }
        self.intr_lists_are(&lists).map_err(|p| ("Meaning", p))
    }

    /// Does each list `e` (a reference-count node) of `lists` hold its
    /// tokens? `Err`: the location of one that does not.
    fn intr_lists_are(&self, lists: &[(i32, i32, &[i32])]) -> Result<(), i32> {
        const LANES: usize = 16;
        let len = self.mem.len();
        let ok = |q: i32| q >= 0 && (q as usize) < len;
        for group in lists.chunks(LANES) {
            // (position in the list, next node) per lane
            let mut at = [(0usize, NULL); LANES];
            for (k, &(p, e, _)) in group.iter().enumerate() {
                if !ok(e) {
                    return Err(p);
                }
                at[k] = (0, self.mem[e as usize].hh().rh());
            }
            loop {
                let mut busy = false;
                for (k, &(p, _, ts)) in group.iter().enumerate() {
                    let (i, q) = at[k];
                    if i == ts.len() {
                        continue;
                    }
                    if !ok(q) {
                        return Err(p);
                    }
                    let h = self.mem[q as usize].hh();
                    if h.lh() != ts[i] {
                        return Err(p);
                    }
                    at[k] = (i + 1, h.rh());
                    busy = true;
                }
                if !busy {
                    break;
                }
            }
            for (k, &(p, _, _)) in group.iter().enumerate() {
                if at[k].1 != NULL {
                    return Err(p);
                }
            }
        }
        Ok(())
    }

    // -- replay ----------------------------------------------------------

    fn intr_replay_entry(&mut self, i: usize) {
        let t0 = std::time::Instant::now();
        let ops = std::mem::take(&mut self.host.intrinsics.store.entries[i].ops);
        self.intr_replay(&ops);
        let n = ops.len() as u64;
        self.host.intrinsics.store.entries[i].ops = ops;
        let s = &mut self.host.intrinsics.stats;
        s.replayed += 1;
        s.replayed_ops += n;
        s.replay_ns += t0.elapsed().as_nanos() as u64;
    }

    /// Make the recorded operations through TeX's own routines.
    pub(crate) fn intr_replay(&mut self, ops: &[Op]) {
        let fault = self.host.intrinsics.fault;
        for (k, op) in ops.iter().enumerate() {
            if fault == Fault::DropLast && k + 1 == ops.len() {
                continue;
            }
            match op {
                Op::Def(p, t, e, global) => {
                    let (t, e) = match e {
                        Val::Plain(v) => (*t, *v),
                        Val::Tokens(ts) => (*t, self.intr_new_list(ts)),
                        Val::Cs(c) => {
                            // `\let`: the source's meaning now, one more reference
                            let w = self.eqtb[(c - 1) as usize];
                            let (t, e) = (w.hh_b0(), w.hh_rh());
                            if self.intr_is_list(t, e) && fault != Fault::NoRef {
                                let r = self.mem[e as usize].hh().lh();
                                self.mem[e as usize].set_hh_lh(r + 1);
                            }
                            (t, e)
                        }
                    };
                    if *global {
                        self.geq_define(*p, t, e)
                    } else {
                        self.eq_define(*p, t, e)
                    }
                }
                Op::Word(p, w, global) => {
                    if *global {
                        self.geq_word_define(*p, *w)
                    } else {
                        self.eq_word_define(*p, *w)
                    }
                }
                Op::Begin(c) => self.new_save_level(*c),
                Op::End => self.unsave(),
            }
        }
    }

    /// A fresh token list (reference count `null`) holding `ts`, one
    /// `get_avail` per node as `scan_toks` makes it (never a node the
    /// display list places: `dl_token_begin`).
    fn intr_new_list(&mut self, ts: &[i32]) -> i32 {
        self.dl_token_begin();
        let h = self.get_avail();
        self.mem[h as usize].set_hh_lh(NULL);
        let mut q = h;
        for &t in ts {
            let r = self.get_avail();
            self.mem[r as usize].set_hh_lh(t);
            self.mem[q as usize].set_hh_rh(r);
            q = r;
        }
        self.dl_token_end();
        h
    }

    // -- recording -------------------------------------------------------

    fn intr_rec_start(
        &mut self,
        cs: i32,
        verify: Option<(u64, crate::intrinsics_verify::Pending)>,
    ) {
        self.host.intrinsics.stats.started += 1;
        let marks = self.intr_marks();
        self.host.intrinsics.rec = Some(Rec {
            cs,
            base: self.intr_body_level(),
            level: self.cur_level,
            cond: self.cond_ptr,
            ctx: self.intr_ctx(),
            marks,
            reads: vec![],
            seen: HashMap::new(),
            names: HashSet::new(),
            ops: vec![],
            tokens: 0,
            align_seen: false,
            taint: HashMap::new(),
            tainted: HashSet::new(),
            verify,
        });
        self.intr_rec_on = true;
        // the macro's own meaning, and \escapechar (printing)
        self.intr_rec_read(cs);
        self.intr_rec_read(int_base + escape_char_code);
    }

    /// Abandon the recording: the call stays an ordinary call. A check's
    /// real path that is not pure is a call the guard should not have
    /// admitted: a difference.
    fn intr_rec_abort(&mut self, why: &'static str) {
        let Some(rec) = self.host.intrinsics.rec.take() else {
            return;
        };
        self.intr_rec_on = false;
        if debug() {
            eprintln!(
                "intrinsics: recording of \\{} abandoned: {why} (cmd {} chr {} cs {})",
                self.intr_cs_name(rec.cs),
                self.cur_cmd,
                self.cur_chr,
                if self.cur_cs > 0 {
                    self.intr_cs_name(self.cur_cs)
                } else {
                    String::new()
                }
            );
        }
        if let Some((id, p)) = rec.verify {
            self.drop_checkpoint(p.ck());
            self.intr_check_failed(id, vec![format!("impure.{why}")], vec![]);
            return;
        }
        let r = &mut self.host.intrinsics;
        bump(&mut r.stats.abandoned, why);
        r.aborts += 1;
        // a first run may enter control sequences (`\csname`), a checkpoint
        // may interrupt one: anything else does not change
        let retry = matches!(why, "NewCs" | "Checkpoint");
        if !retry || r.aborts >= ABORT_BUDGET {
            r.norec = true;
        }
    }

    /// A checked variant differed (or its real path was not pure): count
    /// it and drop the variant.
    pub(crate) fn intr_check_failed(
        &mut self,
        id: u64,
        classes: Vec<String>,
        details: Vec<String>,
    ) {
        let r = &mut self.host.intrinsics;
        r.stats.checked += 1;
        r.stats.differing += 1;
        if debug() {
            eprintln!("intrinsics: check differs: {classes:?} {details:?}");
        }
        for d in details.into_iter().take(3) {
            if r.stats.details.len() < 8 {
                r.stats.details.push(d);
            }
        }
        for c in classes {
            bump(&mut r.stats.classes, c);
        }
        r.store.entries.retain(|e| e.id != id);
    }

    /// The recorded run reads `eqtb[p]`.
    fn intr_rec_read(&mut self, p: i32) {
        let Some(rec) = self.host.intrinsics.rec.as_ref() else {
            return;
        };
        let w = self.eqtb[(p - 1) as usize];
        match rec.seen.get(&p) {
            Some(Seen::Dep) => return,
            Some(Seen::Weak | Seen::LetSrc | Seen::Looked) => {}
            Some(Seen::Written(pre)) => {
                let pre = *pre;
                self.intr_rec_taint(p);
                // derived from the run's own change, unless an `\endgroup`
                // brought back what the entry held before it
                let pre = crate::generated::types::memory_word::from_bits(pre);
                let same = if is_word(p) {
                    pre.int() == w.int()
                } else {
                    pre.hh_b0() == w.hh_b0() && pre.hh_rh() == w.hh_rh()
                };
                if !same {
                    return;
                }
            }
            None => {
                if !region_ok(p) {
                    return self.intr_rec_abort("Region");
                }
            }
        }
        let read = if is_word(p) {
            Read::Word(p, w.int())
        } else {
            let (t, e) = (w.hh_b0(), w.hh_rh());
            if t == glue_ref || t == shape_ref || t == box_ref {
                return self.intr_rec_abort("PointerType");
            }
            if (t == register || t == toks_register) && !(mem_bot..=lo_mem_stat_max).contains(&e) {
                return self.intr_rec_abort("Sparse");
            }
            if self.intr_is_list(t, e) {
                Read::Meaning(
                    p,
                    t,
                    Equiv::Tokens(self.intr_tokens(self.mem[e as usize].hh().rh())),
                )
            } else {
                Read::Meaning(p, t, Equiv::Value(e))
            }
        };
        let n = match &read {
            Read::Meaning(_, _, Equiv::Tokens(ts)) => ts.len(),
            _ => 0,
        };
        let rec = self.host.intrinsics.rec.as_mut().expect("checked");
        rec.seen.insert(p, Seen::Dep);
        rec.reads.push(read);
        rec.tokens += n;
        if rec.reads.len() > MAX_READS || rec.tokens > MAX_TOKENS {
            self.intr_rec_abort("Capacity");
        }
    }

    /// The recorded run reads the token `eqtb[p]` names but looks only at
    /// the token: its meaning matters only if it is `\outer` (an error) or
    /// a parameter character.
    fn intr_rec_weak(&mut self, p: i32) {
        let Some(rec) = self.host.intrinsics.rec.as_ref() else {
            return;
        };
        if !matches!(rec.seen.get(&p), None | Some(Seen::Looked)) || is_word(p) {
            return;
        }
        if !region_ok(p) {
            return self.intr_rec_abort("Region");
        }
        let t = self.eqtb[(p - 1) as usize].hh_b0();
        if t == mac_param || t == outer_call || t == long_outer_call {
            return self.intr_rec_abort("Command");
        }
        let rec = self.host.intrinsics.rec.as_mut().expect("checked");
        rec.seen.insert(p, Seen::Weak);
        rec.reads.push(Read::Plain(p));
    }

    /// The recorded run is about to change `eqtb[p]`. What it held is not a
    /// dependency (only `eq_define`'s bookkeeping looks at it, and a replay
    /// calls `eq_define`), unless the run reads it back later.
    fn intr_rec_write(&mut self, p: i32) {
        let Some(rec) = self.host.intrinsics.rec.as_ref() else {
            return;
        };
        if matches!(rec.seen.get(&p), Some(Seen::Dep | Seen::Written(_))) {
            return;
        }
        if !region_ok(p) {
            return self.intr_rec_abort("Region");
        }
        let w = self.eqtb[(p - 1) as usize].to_bits();
        let rec = self.host.intrinsics.rec.as_mut().expect("checked");
        rec.seen.insert(p, Seen::Written(w));
    }

    fn intr_seen(&self, p: i32, f: impl Fn(Option<&Seen>) -> bool) -> bool {
        self.host
            .intrinsics
            .rec
            .as_ref()
            .is_some_and(|r| f(r.seen.get(&p)))
    }

    /// A token a `\let` reads for its source (`intr_let`, the spaces before
    /// it too): its class only, where its meaning comes from outside the
    /// run (`Read::Class`); where it is the run's own copy of an outside
    /// meaning, or an `\endgroup` brought one back, as any read.
    fn intr_rec_let_src(&mut self, p: i32) {
        let Some(rec) = self.host.intrinsics.rec.as_ref() else {
            return;
        };
        match rec.seen.get(&p) {
            Some(Seen::Dep | Seen::LetSrc) => {}
            Some(Seen::Weak | Seen::Written(_)) => self.intr_rec_read(p),
            None | Some(Seen::Looked) => {
                if !region_ok(p) || is_word(p) {
                    return self.intr_rec_abort("Region");
                }
                let w = self.eqtb[(p - 1) as usize];
                let k = let_class(w.hh_b0(), w.hh_rh());
                if k < 0 {
                    return self.intr_rec_abort("Sparse");
                }
                let rec = self.host.intrinsics.rec.as_mut().expect("checked");
                rec.seen.insert(p, Seen::LetSrc);
                rec.reads.push(Read::Class(p, k));
            }
        }
    }

    /// `eqtb[p]` is about to become (`t`, `e`) as `v`: a `\let` of `c`'s
    /// list (`Val::Cs(c)`) taints the copy with `c`'s meaning when that came
    /// from outside the run; any other definition clears the taint.
    fn intr_rec_copied(&mut self, p: i32, t: i32, e: i32, v: &Val) {
        let Val::Cs(c) = *v else {
            if let Some(rec) = self.host.intrinsics.rec.as_mut() {
                rec.taint.remove(&p);
            }
            return;
        };
        let Some(rec) = self.host.intrinsics.rec.as_ref() else {
            return;
        };
        let w = self.eqtb[(c - 1) as usize];
        let outside = match rec.seen.get(&c) {
            Some(Seen::Dep) => false,
            Some(Seen::Written(pre)) => {
                let pre = crate::generated::types::memory_word::from_bits(*pre);
                pre.hh_b0() == w.hh_b0() && pre.hh_rh() == w.hh_rh()
            }
            _ => true,
        };
        let taint = if outside {
            Some(Read::Meaning(
                c,
                w.hh_b0(),
                Equiv::Tokens(self.intr_tokens(self.mem[w.hh_rh() as usize].hh().rh())),
            ))
        } else {
            // the run's own copy of a copy: its taint, while it holds it
            rec.taint
                .get(&c)
                .filter(|(word, _)| *word == (w.hh_b0(), w.hh_rh()))
                .and_then(|(_, r)| r.clone())
        };
        let n = match &taint {
            Some(Read::Meaning(_, _, Equiv::Tokens(ts))) => ts.len(),
            _ => 0,
        };
        let rec = self.host.intrinsics.rec.as_mut().expect("checked");
        rec.tokens += n;
        rec.taint.insert(p, ((t, e), taint));
    }

    /// The run reads `p`, which it wrote: if `p` still holds a `\let`'s
    /// copy of an outside meaning, that meaning is a dependency.
    fn intr_rec_taint(&mut self, p: i32) {
        let w = self.eqtb[(p - 1) as usize];
        let Some(rec) = self.host.intrinsics.rec.as_mut() else {
            return;
        };
        let Some(((t, e), Some(read))) = rec.taint.get(&p) else {
            return;
        };
        if (*t, *e) != (w.hh_b0(), w.hh_rh()) {
            return;
        }
        let Read::Meaning(c, ..) = read else {
            return;
        };
        if rec.tainted.insert(*c) {
            let read = read.clone();
            rec.reads.push(read);
        }
    }

    /// `changes/intrinsics.ch`: a definition (kind 1, 2; plus 8 when
    /// global) or group boundary (3, 4) the recorded run makes; 6 (`\wd`
    /// and kin write a box node) and 7 (`\immediate` output) abandon it.
    pub fn flashtex_intr_op(&mut self, k: i32, p: i32, t: i32, e: i32) {
        if self.host.intrinsics.rec.is_none() {
            return;
        }
        let global = k & 8 != 0;
        let op = match k & 7 {
            1 => {
                self.intr_rec_write(p);
                if !self.intr_rec_on {
                    return;
                }
                if t == glue_ref || t == shape_ref || t == box_ref {
                    return self.intr_rec_abort("PointerType");
                }
                if (t == register || t == toks_register)
                    && !(mem_bot..=lo_mem_stat_max).contains(&e)
                {
                    return self.intr_rec_abort("Sparse");
                }
                let c = self.cur_cs;
                let v = if !self.intr_is_list(t, e) {
                    // a `\let` of a value that is not a list keeps the value:
                    // its source's meaning is a dependency
                    if c > 0 && self.intr_seen(c, |s| matches!(s, Some(Seen::LetSrc))) {
                        self.intr_rec_read(c);
                        if !self.intr_rec_on {
                            return;
                        }
                    }
                    Val::Plain(e)
                } else if self.mem[e as usize].hh().lh() == NULL {
                    let ts = self.intr_tokens(self.mem[e as usize].hh().rh());
                    self.host.intrinsics.rec.as_mut().expect("checked").tokens += ts.len();
                    Val::Tokens(ts)
                } else {
                    // a list shared with the control sequence just read
                    let c = self.cur_cs;
                    let w = (c > 0 && c <= eqtb_top).then(|| self.eqtb[(c - 1) as usize]);
                    if !w.is_some_and(|w| w.hh_b0() == t && w.hh_rh() == e) {
                        return self.intr_rec_abort("UnpinnedList");
                    }
                    Val::Cs(c)
                };
                self.intr_rec_copied(p, t, e, &v);
                if !self.intr_rec_on {
                    return;
                }
                Op::Def(p, t, v, global)
            }
            2 => {
                if TRACING_ALL.iter().any(|&c| p == int_base + c) {
                    return self.intr_rec_abort("Tracing");
                }
                self.intr_rec_write(p);
                if !self.intr_rec_on {
                    return;
                }
                Op::Word(p, t, global)
            }
            3 => {
                if p != simple_group && p != semi_simple_group {
                    return self.intr_rec_abort("Group");
                }
                Op::Begin(p)
            }
            4 => {
                let level = self.host.intrinsics.rec.as_ref().expect("checked").level;
                if self.cur_level <= level {
                    return self.intr_rec_abort("Group");
                }
                Op::End
            }
            6 => return self.intr_rec_abort("BoxDimen"),
            _ => return self.intr_rec_abort("FileOutput"),
        };
        let rec = self.host.intrinsics.rec.as_mut().expect("checked");
        rec.ops.push(op);
        if rec.ops.len() > MAX_OPS || rec.tokens > MAX_TOKENS {
            self.intr_rec_abort("Capacity");
        }
    }

    /// The end of `get_next`, during a recording.
    pub fn flashtex_intr_next(&mut self) {
        let Some(base) = self.host.intrinsics.rec.as_ref().map(|r| r.base) else {
            return;
        };
        if self.input_ptr < base {
            return self.intr_rec_abort("Level");
        }
        if self.cur_cmd == tab_mark || self.cur_cmd == car_ret {
            self.host
                .intrinsics
                .rec
                .as_mut()
                .expect("checked")
                .align_seen = true;
        }
        let cs = self.cur_cs;
        if cs == 0 {
            return;
        }
        if self.cur_cmd == top_bot_mark {
            return self.intr_rec_abort("Mark");
        }
        if self.ckpt_arm_cs != 0 && cs == self.ckpt_arm_cs {
            return self.intr_rec_abort("Checkpoint");
        }
        if self.intr_weak || self.scanner_status == matching {
            self.intr_rec_weak(cs)
        } else if self.intr_let {
            self.intr_rec_let_src(cs)
        } else {
            self.intr_rec_read(cs)
        }
    }

    /// A character code, the register `\advance` reads, or a
    /// one-character control sequence's category code (`print_cs`).
    pub fn flashtex_intr_read(&mut self, p: i32) {
        self.intr_rec_read(p);
    }

    /// `\csname` and `\ifcsname` found `p`: its meaning, and the name at
    /// it. An `\ifcsname` that found nothing is not recorded.
    pub fn flashtex_intr_lookup(&mut self, p: i32) {
        if p == undefined_control_sequence {
            return self.intr_rec_abort("Absent");
        }
        self.flashtex_intr_name(p);
        let Some(rec) = self.host.intrinsics.rec.as_ref() else {
            return;
        };
        let first = match rec.seen.get(&p) {
            // the whole meaning is a dependency already, or this is
            Some(Seen::Dep | Seen::Looked) => return,
            // the run's own, unless restored or a copy (`intr_rec_read`)
            Some(Seen::Written(_)) => return self.intr_rec_read(p),
            // a weak or a `\let` source's read says nothing of it
            Some(Seen::Weak | Seen::LetSrc) => false,
            None => true,
        };
        if !region_ok(p) || is_word(p) {
            return self.intr_rec_abort("Region");
        }
        let d = self.eqtb[(p - 1) as usize].hh_b0() != undefined_cs;
        let rec = self.host.intrinsics.rec.as_mut().expect("checked");
        if first {
            rec.seen.insert(p, Seen::Looked);
        }
        rec.reads.push(Read::Defined(p, d));
    }

    /// The name at `p` is looked up or printed.
    pub fn flashtex_intr_name(&mut self, p: i32) {
        if !is_hashed(p) {
            return;
        }
        let Some(rec) = self.host.intrinsics.rec.as_ref() else {
            return;
        };
        if rec.names.contains(&p) {
            return;
        }
        let name = self
            .intr_str(p)
            .map(|r| r.map(|i| self.str_pool[i]).collect::<Vec<i32>>());
        let rec = self.host.intrinsics.rec.as_mut().expect("checked");
        rec.tokens += name.as_ref().map_or(0, |n| n.len());
        rec.names.insert(p);
        rec.reads.push(Read::Name(p, name));
    }

    /// `print_meaning`: `print_cmd_chr` reads a font's name and size, and a
    /// sparse register's number, which the read set does not hold.
    pub fn flashtex_intr_meaning(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        if c == set_font
            || c == top_bot_mark
            || ((c == register || c == toks_register)
                && !(mem_bot..=lo_mem_stat_max).contains(&chr))
        {
            self.intr_rec_abort("Meaning");
        }
    }

    /// `expand`, at `reswitch`: the expandable primitives a recording may run.
    pub fn flashtex_intr_expand(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        let ok = match c {
            c if (call..end_template).contains(&c) => true,
            expand_after | fi_or_else | cs_name => true,
            // \noexpand (not \primitive)
            no_expand => chr == 0,
            // \if \ifcat \ifnum \ifodd \ifx \iftrue \iffalse \ifcase
            // \ifdefined \ifcsname (and \unless of them)
            if_test => matches!(
                chr % unless_code,
                if_char_code
                    | if_cat_code
                    | if_int_code
                    | if_odd_code
                    | ifx_code
                    | if_true_code
                    | if_false_code
                    | if_case_code
                    | if_def_code
                    | if_cs_code
            ),
            // \number \romannumeral \string \meaning \eTeXrevision
            // \expanded \strcmp \XeTeXrevision
            convert => matches!(
                chr,
                number_code
                    | roman_numeral_code
                    | string_code
                    | meaning_code
                    | eTeX_revision_code
                    | expanded_code
                    | pdf_strcmp_code
                    | XeTeX_revision_code
            ),
            // \the \unexpanded \detokenize
            the => chr == 0 || chr == 1 || chr == show_tokens,
            _ => false,
        };
        if !ok {
            self.intr_rec_abort("Expandable");
        }
    }

    /// `scan_something_internal`: the quantities a recording may fetch.
    pub fn flashtex_intr_internal(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        match c {
            // (the code itself is read where it is fetched)
            def_code | char_given | math_given => {}
            assign_toks | assign_int | assign_dimen => self.intr_rec_read(chr),
            // \eTeXversion \XeTeXversion \numexpr
            last_item
                if chr == eTeX_version_code || chr == XeTeX_version_code || chr == eTeX_expr => {}
            _ => self.intr_rec_abort("Internal"),
        }
    }

    /// `scan_dimen` (1), a token register copied (2), a sparse array (3).
    pub fn flashtex_intr_abort(&mut self, r: i32) {
        self.intr_rec_abort(match r {
            1 => "Dimension",
            2 => "TokenCopy",
            _ => "Sparse",
        })
    }

    /// Popping the condition stack: not below the recording's own.
    pub fn flashtex_intr_pop_cond(&mut self) {
        let Some(cond) = self.host.intrinsics.rec.as_ref().map(|r| r.cond) else {
            return;
        };
        if self.cond_ptr == cond {
            self.intr_rec_abort("Cond");
        }
    }

    /// `main_control` at `reswitch`, and `prefixed_command` once the
    /// prefixes are read: the commands a recording may run.
    pub fn flashtex_intr_command(&mut self) {
        let Some(level) = self.host.intrinsics.rec.as_ref().map(|r| r.level) else {
            return;
        };
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        let m = self.cur_list.mode_field.abs();
        let inside = self.cur_level > level;
        let ok = if c > max_non_prefixed_command {
            match c {
                toks_register | assign_toks | assign_int | def_code | register | advance
                | multiply | divide | prefix | let_ | def => true,
                // \chardef \mathchardef \countdef \dimendef \skipdef
                // \muskipdef \toksdef (not XeTeX's \Umathchardef and kin)
                shorthand_def => (char_def_code..=toks_def_code).contains(&chr),
                _ => false,
            }
        } else {
            match c {
                relax | begin_group => true,
                left_brace => m != mmode,
                right_brace => self.cur_group == simple_group && inside,
                end_group => self.cur_group == semi_simple_group && inside,
                spacer => m == vmode || m == mmode,
                // \ignorespaces (not \primitive)
                ignore_spaces => chr == 0,
                _ => false,
            }
        };
        if !ok {
            self.intr_rec_abort("Command");
        }
    }

    /// `big_switch`: end the recording once the body is used up.
    pub fn flashtex_intr_switch(&mut self) {
        let Some(base) = self.host.intrinsics.rec.as_ref().map(|r| r.base) else {
            return;
        };
        if !self.intr_used_up(base) {
            return;
        }
        let mut rec = self.host.intrinsics.rec.take().expect("checked");
        self.intr_rec_on = false;
        let now = self.intr_marks();
        let why = if self.after_token != 0 {
            Some("State")
        } else {
            rec.marks
                .iter()
                .zip(&now)
                .find(|(a, b)| a != b)
                .map(|(a, _)| a.0)
        };
        match (why, rec.verify.take()) {
            (Some(w), Some((id, p))) => {
                self.drop_checkpoint(p.ck());
                self.intr_check_failed(id, vec![format!("impure.{w}")], vec![]);
            }
            (Some(w), None) => {
                // (put back for the abort's accounting)
                self.host.intrinsics.rec = Some(rec);
                self.intr_rec_abort(w);
            }
            (None, Some((id, p))) => {
                if let Err((classes, details)) = self.intr_verify_end(p) {
                    self.intr_check_failed(id, classes, details);
                }
            }
            (None, None) => self.intr_commit(rec),
        }
    }

    fn intr_commit(&mut self, mut rec: Rec) {
        if debug() {
            let mut big: Vec<(usize, i32)> = rec
                .reads
                .iter()
                .filter_map(|r| match r {
                    Read::Meaning(p, _, Equiv::Tokens(ts)) => Some((ts.len(), *p)),
                    _ => None,
                })
                .collect();
            let total: usize = big.iter().map(|b| b.0).sum();
            big.sort_unstable_by(|a, b| b.cmp(a));
            let kinds = rec.reads.iter().fold([0usize; 5], |mut k, r| {
                k[match r {
                    Read::Meaning(..) => 0,
                    Read::Plain(_) => 1,
                    Read::Word(..) => 2,
                    Read::Name(..) => 3,
                    Read::Class(..) | Read::Defined(..) => 4,
                }] += 1;
                k
            });
            let top: Vec<String> = big
                .iter()
                .take(12)
                .map(|&(n, p)| format!("\\{} {n}", self.intr_cs_name(p)))
                .collect();
            eprintln!(
                "intrinsics: kept \\{}: {} ops, reads meaning/plain/word/name/class {kinds:?}, {total} tokens; largest {top:?}",
                self.intr_cs_name(rec.cs),
                rec.ops.len()
            );
        }
        let r = &mut self.host.intrinsics;
        if r.fault == Fault::SkipRead {
            let cs = rec.cs;
            if let Some(k) = rec
                .reads
                .iter()
                .rposition(|x| matches!(x, Read::Meaning(p, _, Equiv::Tokens(_)) if *p != cs))
            {
                rec.reads.remove(k);
            }
        }
        let store = &mut r.store;
        let mine: Vec<usize> = (0..store.entries.len())
            .filter(|&i| store.entries[i].cs == rec.cs)
            .collect();
        if mine.len() >= MAX_VARIANTS {
            let lru = *mine
                .iter()
                .min_by_key(|&&i| store.entries[i].used)
                .expect("some");
            store.entries.remove(lru);
        }
        store.clock += 1;
        store.next_id += 1;
        r.stats.committed += 1;
        r.stats.committed_ops += rec.ops.len() as u64;
        store.entries.push(Entry {
            id: store.next_id,
            cs: rec.cs,
            ctx: Ctx {
                align: rec.ctx.align.filter(|_| rec.align_seen),
                ..rec.ctx
            },
            reads: rec.reads,
            ops: rec.ops,
            used: store.clock,
        });
    }

    /// The input level `macro_call` will push the body at: above the
    /// used-up token lists it pops first ("conserve stack space").
    fn intr_body_level(&self) -> i32 {
        let (mut k, mut l) = (self.input_ptr, self.cur_input);
        while k > 0
            && l.state_field == token_list
            && l.loc_field == NULL
            && l.index_field != v_template
        {
            k -= 1;
            l = self.input_stack[k as usize];
        }
        k + 1
    }

    /// Every input level from `base` on is a used-up token list (or gone):
    /// `main_control` has executed all of the body.
    fn intr_used_up(&self, base: i32) -> bool {
        (base..=self.input_ptr).all(|k| {
            let l = if k == self.input_ptr {
                self.cur_input
            } else {
                self.input_stack[k as usize]
            };
            l.state_field == token_list && l.loc_field == NULL && l.index_field != v_template
        })
    }

    /// The state a kept call leaves as it found it, by reason.
    fn intr_marks(&mut self) -> Vec<(&'static str, i64)> {
        let log = self.intr_log_len() as i64;
        let c = &self.cur_list;
        [
            ("Group", self.cur_level),
            ("Cond", self.cond_ptr),
            ("Cond", self.if_limit),
            ("Cond", self.cur_if),
            ("Cond", self.if_line),
            ("Unbalanced", self.align_state),
            ("NewCs", self.str_ptr),
            ("NewCs", self.pool_ptr),
            ("NewCs", self.hash_used),
            ("NewCs", self.hash_high),
            ("Error", self.error_count),
            ("Error", self.history),
            ("State", c.mode_field.abs()),
            ("State", self.nest_ptr),
            ("State", c.tail_field),
            ("State", self.scanner_status),
            ("Output", self.file_offset),
            ("Output", self.term_offset),
        ]
        .into_iter()
        .map(|(w, v)| (w, v as i64))
        .chain([("Output", log)])
        .collect()
    }

    pub(crate) fn intr_is_list(&self, t: i32, e: i32) -> bool {
        (call..=long_outer_call).contains(&t) && e >= 0 && (e as usize) < self.mem.len()
    }

    /// The tokens of the list starting at `p` (after its reference count).
    pub(crate) fn intr_tokens(&self, mut p: i32) -> Vec<i32> {
        let mut v = vec![];
        while p >= 0 && (p as usize) < self.mem.len() && v.len() < MAX_TOKENS {
            let w = self.mem[p as usize].hh();
            v.push(w.lh());
            p = w.rh();
        }
        v
    }

    fn intr_int(&self, code: i32) -> i32 {
        self.eqtb[(int_base + code - 1) as usize].int()
    }

    /// The pool range of the name at `cs`, if it has one.
    fn intr_str(&self, cs: i32) -> Option<std::ops::Range<usize>> {
        if cs < hash_base || cs >= hash_base + self.hash.len() as i32 {
            return None;
        }
        let s = self.hash[(cs - hash_base) as usize].rh();
        if !(65536..self.str_ptr).contains(&s) {
            return None;
        }
        let a = self.str_start[(s - 65536) as usize] as usize;
        let b = self.str_start[(s + 1 - 65536) as usize] as usize;
        Some(a..b)
    }

    fn intr_is_named(&self, cs: i32, name: &[u8]) -> bool {
        self.intr_str(cs).is_some_and(|r| {
            r.len() == name.len() && r.zip(name).all(|(i, &c)| self.str_pool[i] == c as i32)
        })
    }

    /// A control sequence's name, for the report (UTF-16 code units).
    pub(crate) fn intr_cs_name(&self, cs: i32) -> String {
        match self.intr_str(cs) {
            Some(r) => {
                let u: Vec<u16> = r.map(|i| self.str_pool[i] as u16).collect();
                String::from_utf16_lossy(&u)
            }
            None => match char::from_u32((cs - single_base) as u32) {
                Some(c) if (single_base..null_cs).contains(&cs) => c.to_string(),
                _ => format!("eqtb[{cs}]"),
            },
        }
    }

    /// `FLASHTEX_INTRINSICS_VERIFY_FAIL=1` and a check differed: the
    /// driver exits with status 3.
    pub fn intr_verify_failed(&self) -> bool {
        std::env::var("FLASHTEX_INTRINSICS_VERIFY_FAIL").as_deref() == Ok("1")
            && self.host.intrinsics.stats.differing > 0
    }

    /// A separate report file (`FLASHTEX_INTRINSICS_STATS`) keeps TeX's log
    /// and output byte for byte the same in every mode.
    pub fn write_intrinsics_report(&mut self) {
        if self.host.intrinsics.mode == Mode::Off {
            return;
        }
        self.intr_abandon("unfinished");
        if self.intr_verify_failed() {
            eprintln!(
                "XeTeX intrinsics verify: {} of {} checked calls differ",
                self.host.intrinsics.stats.differing, self.host.intrinsics.stats.checked
            );
        }
        let Some(path) = std::env::var_os("FLASHTEX_INTRINSICS_STATS") else {
            return;
        };
        let r = &self.host.intrinsics;
        let s = &r.stats;
        // (the verifier's checkpoints are gone: a driver run has none)
        let checkpoints = self.arena.checkpoint_ids().len();
        let mode = match r.mode {
            Mode::On => "on",
            _ => "verify",
        };
        let mut out = format!(
            "mode: {mode}\nfault: {:?}\ncheckpoints: {checkpoints}\nentries: {}\noffers: {}\nadmitted: {}\nreplayed: {}\nreplayed_ops: {}\nreplay_us: {}\nguard_us: {}\nrecordings.started: {}\nrecordings.committed: {}\nrecordings.committed_ops: {}\nverify.checked: {}\nverify.identical: {}\nverify.differing: {}\n",
            r.fault,
            r.store.entries.len(),
            s.offers,
            s.admitted,
            s.replayed,
            s.replayed_ops,
            s.replay_ns / 1000,
            s.guard_ns / 1000,
            s.started,
            s.committed,
            s.committed_ops,
            s.checked,
            s.identical,
            s.differing,
        );
        for (group, map) in [
            ("rejected", &s.rejected),
            ("missed", &s.missed),
            ("abandoned", &s.abandoned),
            ("verify.unverified", &s.unverified),
        ] {
            for (&k, &n) in map {
                out.push_str(&format!("{group}.{k}: {n}\n"));
            }
        }
        for (k, n) in &s.classes {
            out.push_str(&format!("verify.differing.{k}: {n}\n"));
        }
        for d in &s.details {
            out.push_str(&format!("verify.detail: {d}\n"));
        }
        if let Err(e) = std::fs::write(&path, out) {
            eprintln!("XeTeX intrinsics report {}: {e}", path.to_string_lossy());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Mode;
    use crate::Globals;

    /// A checkpoint taken while the intrinsics were off (an earlier run's
    /// S₀, or one read from a file) does not turn them off in a run that
    /// restores it, nor on in one where they are off.
    #[test]
    fn a_restore_keeps_the_runs_mode() {
        let mut g = Globals::new();
        let s0 = g.checkpoint();
        assert!(!g.intr_on);
        g.set_intrinsics(Mode::Verify);
        g.restore(s0).unwrap();
        assert!(g.intr_on);
        let s0 = g.checkpoint();
        g.set_intrinsics(Mode::Off);
        g.restore(s0).unwrap();
        assert!(!g.intr_on);
    }
}
