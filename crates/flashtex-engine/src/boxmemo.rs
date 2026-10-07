//! BOX-MEMO (docs/design/engine-v2/BOX-MEMO.md, lane P6-INFDESC-PAGE):
//! guarded replay of macro calls that typeset only what they throw away.
//!
//! framed.sty's `\fb@sizeofframe` measures a frame by drawing it (on
//! *Infinite Descent*, a whole TikZ picture) around a rule in a box it then
//! discards; it keeps two `\global` dimensions. It is 14-44 % of an edited
//! page's work there (docs/evidence/p6-infdesc-page-2026-10-06/). A call of
//! a registered macro like it is *recorded* once: the normal expansion,
//! observed through `changes/boxmemo.ch`'s hooks, which note what it reads
//! and every assignment and group it makes, and abandon it at anything
//! outside the model (BOX-MEMO.md §4.1). A later call whose **key** holds
//! (§3: the macro and its arguments; every `eqtb` entry outside the control
//! sequences and box registers; the meaning of every control sequence the
//! recording looked at; e-TeX's sparse registers; the font and hyphenation
//! versions; the context) is *replayed*: the body is popped as `get_next`
//! pops a used-up level, the recorded assignments and groups are made again
//! through TeX's own `eq_define` & co., in order, and `last_badness` is set.
//!
//! The recordings live here, outside the word space: an edit's restore
//! rolls the word space back to before the edited window, and the keys are
//! checked by value in full, so they stay sound across restores. No entry
//! holds an address into `mem`; token lists, glue and shapes are kept by
//! value.
//!
//! `FLASHTEX_BOXMEMO=on|verify|off` (default off while the design is under
//! review); `FLASHTEX_BOXMEMO_NAMES=a,b` (default `fb@sizeofframe`);
//! `FLASHTEX_BOXMEMO_STATS=FILE`; `FLASHTEX_BOXMEMO_DEBUG=1`.
//! In `verify` a call the guard admits is expanded normally under a fresh
//! recording, and its assignments, groups and `last_badness` are compared
//! with the replay's (`verify_differences`); `FLASHTEX_BOXMEMO_VERIFY_FAIL=1`
//! makes a difference fatal (exit status 3 at the end of the run).

use crate::generated::consts as k;
use crate::generated::Globals;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

const BOX_BASE: i32 = k::box_base;
const GLUE_BASE: i32 = k::glue_base;
const LOCAL_BASE: i32 = k::local_base;
const INT_BASE: i32 = k::int_base;
const EQTB_SIZE: i32 = k::eqtb_size;
const UNDEFINED_CS: i32 = k::undefined_control_sequence;
const PAR_SHAPE_LOC: i32 = k::par_shape_loc;
const OUTPUT_ROUTINE_LOC: i32 = k::output_routine_loc;
const TOKS_BASE: i32 = k::toks_base;
const ETEX_PEN_BASE: i32 = k::etex_pen_base;
const ETEX_PENS: i32 = k::etex_pens;
const CALL: i32 = k::call;
const LONG_OUTER_CALL: i32 = k::long_outer_call;
const GLUE_REF: i32 = k::glue_ref;
const SHAPE_REF: i32 = k::shape_ref;
const BOX_REF: i32 = k::box_ref;
const REGISTER: i32 = k::register;
const TOKS_REGISTER: i32 = k::toks_register;
const LO_MEM_STAT_MAX: i32 = k::lo_mem_stat_max;
const GLUE_SPEC_SIZE: i32 = k::glue_spec_size;
const LEVEL_ONE: i32 = k::level_one;
const CONTRIB_HEAD: i32 = k::contrib_head;
const PAGE_HEAD: i32 = k::page_head;
const TOKEN_LIST: i32 = k::token_list;
const V_TEMPLATE: i32 = k::v_template;

const INT_VAL: i32 = k::int_val;
const DIMEN_VAL: i32 = k::dimen_val;
const GLUE_VAL: i32 = k::glue_val;
const MU_VAL: i32 = k::mu_val;
const BOX_VAL: i32 = k::box_val;
const TOK_VAL: i32 = k::tok_val;

/// The longest token list or shape kept by value (a longer one abandons).
const MAX_LIST: usize = 1 << 16;
/// Variants kept per macro, least recently used dropped first.
/// (A page of *Infinite Descent* measures a dozen frames, each with its
/// own key, and a typed letter alternates two states of each: 8 thrashed.)
const MAX_VARIANTS: usize = 64;
/// Bytes of recordings kept in all, roughly.
const MAX_BYTES: usize = 64 << 20;
/// Recordings abandoned in a row before a macro is no longer recorded.
const ABORT_BUDGET: u32 = 64;

// ---------------------------------------------------------------------------
// Configuration and statistics
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    Verify,
}

struct Config {
    mode: Mode,
    names: Vec<Vec<u8>>,
    stats_out: Option<String>,
    debug: bool,
    verify_fail: bool,
}

/// The macros offered by default (BOX-MEMO.md §2).
pub const DEFAULT_NAMES: &[&str] = &["fb@sizeofframe", "*@framecommand"];

#[derive(Default, Debug)]
pub struct Stats {
    pub offers: u64,
    pub hits: u64,
    pub replayed_ops: u64,
    pub recordings: u64,
    pub committed: u64,
    pub verified: u64,
    pub verify_differences: u64,
    /// Calls compared on the whole engine state (BOX-MEMO.md §7), and how
    /// many differed.
    pub full_verified: u64,
    pub full_differences: u64,
    pub abandoned: BTreeMap<String, u64>,
    pub misses: BTreeMap<String, u64>,
    pub verify_details: Vec<String>,
    pub entries: usize,
    pub bytes: usize,
}

// ---------------------------------------------------------------------------
// Recordings
// ---------------------------------------------------------------------------

/// A value an assignment stored, kept by value.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Val {
    /// An integer or dimension (`eq_word_define`, `sa_w_def`).
    Word(i32),
    /// An `equiv` that is not a pointer the assignment made.
    Plain(i32),
    /// One of the static glue specifications (`zero_glue` & co.), whose
    /// reference count the assignment takes one of.
    StaticGlue(i32),
    /// A glue specification made for the assignment: width, stretch, shrink,
    /// stretch order, shrink order.
    FreshGlue([i32; 5]),
    /// A token list made for the assignment (`\def`, `\edef`, `\toks`).
    FreshList(Vec<i32>),
    /// The meaning of a control sequence as it is when the assignment is
    /// made (`\let`, `\futurelet`).
    LetCs(i32),
    /// A `\parshape` specification made for the assignment, by its words.
    FreshShape(Vec<i32>),
    /// The glue specification a skip register holds when the assignment is
    /// made (`\skip1=\skip2` shares it): an `eqtb` location, or a sparse
    /// register (type, number) as `(-type, number)`.
    GlueFrom(i32, i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Op {
    Begin(i32),
    End,
    /// `eqtb[p]`: kind 0 `eq_define`, 1 `eq_word_define`, 2 `geq_define`,
    /// 3 `geq_word_define`.
    Def {
        p: i32,
        t: i32,
        v: Val,
        kind: i32,
    },
    /// A sparse register (type, number): kind 0 `sa_def`, 1 `sa_w_def`,
    /// 2 `gsa_def`, 3 `gsa_w_def`.
    Sa {
        t: i32,
        n: i32,
        v: Val,
        kind: i32,
    },
}

/// A register the recording read: an `eqtb` location (`\count` & co. 0-255,
/// and the `\countdef` and kin that name them), or an e-TeX sparse register
/// by (type, number).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum RegId {
    Eqtb(i32),
    Sa(i32, i32),
}

/// A control sequence's meaning as the key keeps it.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Meaning {
    Plain(i32, i32),
    Macro(i32, Vec<i32>),
    /// A `\countdef` (and kin) of an e-TeX sparse register: the command,
    /// the register's type and number.
    Sparse(i32, i32, i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Context {
    mode: i32,
    interaction: i32,
    align_state: i32,
    scanner_status: i32,
    par_token: i32,
    font_version: i32,
    hyph_version: i32,
}

/// A hash of K1, the context, K2 and K4: a variant is compared in full only
/// when its hash matches (a filter; the comparison decides).
fn key_hash(k1: &[i32], ctx: &Context, k2: &[i32], k4: &[i32]) -> u64 {
    let mut h: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut mix = |x: i32| {
        h = (h ^ x as u32 as u64)
            .wrapping_mul(0x0100_0000_01b3)
            .rotate_left(23);
    };
    for &x in k1.iter().chain(k2).chain(k4) {
        mix(x);
    }
    for x in [
        ctx.mode,
        ctx.interaction,
        ctx.align_state,
        ctx.scanner_status,
        ctx.par_token,
        ctx.font_version,
        ctx.hyph_version,
        k1.len() as i32,
        k2.len() as i32,
        k4.len() as i32,
    ] {
        mix(x);
    }
    h
}

/// A variant whose K1, context, K2 and K4 hold: its index, and the K3
/// meanings, absent names and void boxes left to check.
type Candidate = (
    usize,
    Vec<(i32, Meaning)>,
    Vec<Vec<i32>>,
    Vec<i32>,
    Vec<(RegId, Vec<i32>)>,
    Vec<(i32, Vec<i32>)>,
);

struct Entry {
    hash: u64,
    /// K1: the macro's body and its arguments.
    k1: Vec<i32>,
    ctx: Context,
    /// K2 and K4, as one stream of values.
    k2: Vec<i32>,
    k4: Vec<i32>,
    /// K3: the meanings looked at, by location.
    k3: Vec<(i32, Meaning)>,
    /// Names looked up (`\ifcsname`) and not found.
    absent: Vec<Vec<i32>>,
    /// KR: the registers read, by value, first read first.
    kr: Vec<(RegId, Vec<i32>)>,
    /// Box registers read before written, void when read (LaTeX's
    /// `\voidb@x`).
    voids: Vec<i32>,
    /// `line` when the recording read `\inputlineno` (LaTeX's `\begin`).
    line: Option<i32>,
    /// Foreign box registers read for their dimensions or kind, and those
    /// moved into the output (the holes, in order): by `box_key`.
    kb: Vec<(i32, Vec<i32>)>,
    holes: Vec<(i32, Vec<i32>)>,
    /// A draw call's output: the nodes it appended to the current list,
    /// and the list's `aux` after them.
    frag: Option<crate::boxfrag::Frag>,
    aux: u64,
    ops: Vec<Op>,
    last_badness: i32,
    last_used: u64,
    hits: u64,
    bytes: usize,
}

/// The state a recording compares at its end with its start.
#[derive(PartialEq, Eq, Debug, Clone)]
struct Frame {
    cur_level: i32,
    save_ptr: i32,
    cond: (i32, i32, i32, i32),
    align_state: i32,
    strings: (i32, i32, i32),
    errors: (i32, i32),
    nest_ptr: i32,
    mode: i32,
    tail: i32,
    page: Vec<i64>,
    fonts: (i32, i32),
    font_glue: Vec<i32>,
    marks: Vec<i32>,
    obj_ptr: i32,
    interaction: i32,
    arm: (i32, i32),
    outputs: (i32, i32, u64, usize, usize),
    /// The line-shift journal (`crate::lineshift::journal_mark`).
    lines: (u64, Vec<(i32, i32)>, bool),
}

struct Rec {
    cs: i32,
    /// The input level of the body.
    base: i32,
    /// `save_scanner_status`: the status the body runs with.
    scanner: i32,
    start: Frame,
    k1: Vec<i32>,
    ctx: Context,
    k2: Vec<i32>,
    k4: Vec<i32>,
    k3: Vec<(i32, Meaning)>,
    absent: Vec<Vec<i32>>,
    voids: Vec<i32>,
    kr: Vec<(RegId, Vec<i32>)>,
    /// Registers and control sequences the call assigned globally: read
    /// after that, they hold what the call put there.
    gwritten: HashSet<RegId>,
    line_read: bool,
    boxes: HashSet<i32>,
    /// Box registers assigned globally: each must be void when the call
    /// ends, and the replay makes it so (`\global\setbox` to void, at the
    /// end of the log).
    gboxes: Vec<i32>,
    /// A foreign box register just fetched, waiting for what is done with
    /// it (`flashtex_bm_box_use`): (register, box).
    pending_box: Option<(i32, i32)>,
    kb: Vec<(i32, Vec<i32>)>,
    /// (register, box node, key) of each box moved out of a foreign register
    holes: Vec<(i32, i32, Vec<i32>)>,
    ops: Vec<Op>,
    /// Verifying the entry of this index (of `store[cs]`).
    verify: Option<usize>,
}

#[derive(Default)]
struct State {
    store: HashMap<i32, Vec<Entry>>,
    aborts: HashMap<i32, u32>,
    rec: Option<Rec>,
    clock: u64,
    bytes: usize,
    stats: Stats,
    stats_dirty: u64,
}

/// The recording's hot state, outside `ST`'s `RefCell` borrow: the body's
/// input level, and which control sequences it has looked at (`seen[p] ==
/// stamp`), so that `get_next`'s hook costs a load and a compare.
#[derive(Default)]
struct Fast {
    base: i32,
    stamp: u32,
    seen: Vec<u32>,
}

thread_local! {
    /// Debugging misses: the key by location of the last recording, per macro.
    static LAST_KEY: RefCell<HashMap<i32, Vec<(i32, Vec<i32>)>>> = RefCell::new(HashMap::new());
    static FAST: RefCell<Fast> = RefCell::new(Fast::default());
    static CONFIG: RefCell<Option<Config>> = const { RefCell::new(None) };
    static ST: RefCell<State> = RefCell::new(State::default());
}

fn with_config<R>(f: impl FnOnce(&Config) -> R) -> R {
    CONFIG.with(|c| {
        let mut c = c.borrow_mut();
        if c.is_none() {
            let mode = match std::env::var("FLASHTEX_BOXMEMO").as_deref() {
                Ok("on") | Ok("1") => Mode::On,
                Ok("verify") => Mode::Verify,
                _ => Mode::Off,
            };
            let names = match std::env::var("FLASHTEX_BOXMEMO_NAMES") {
                Ok(v) => v
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.as_bytes().to_vec())
                    .collect(),
                Err(_) => DEFAULT_NAMES
                    .iter()
                    .map(|s| s.as_bytes().to_vec())
                    .collect(),
            };
            *c = Some(Config {
                mode,
                names,
                stats_out: std::env::var("FLASHTEX_BOXMEMO_STATS").ok(),
                debug: std::env::var_os("FLASHTEX_BOXMEMO_DEBUG").is_some(),
                verify_fail: std::env::var_os("FLASHTEX_BOXMEMO_VERIFY_FAIL").is_some(),
            });
        }
        f(c.as_ref().unwrap())
    })
}

pub fn mode() -> Mode {
    with_config(|c| c.mode)
}

fn debug() -> bool {
    with_config(|c| c.debug)
}

/// The statistics so far.
pub fn stats() -> String {
    ST.with(|s| {
        let s = s.borrow();
        let mut st = format!("{:#?}", s.stats);
        st.push('\n');
        st
    })
}

/// With `FLASHTEX_BOXMEMO_VERIFY_FAIL`, a difference ends the process at
/// once with status 3 (a host too: the gates see it as a crash).
fn fail_now() {
    if fail_on_difference() {
        write_stats(true);
        eprintln!("boxmemo: a verification found a difference; exiting with status 3");
        std::process::exit(3);
    }
}

/// `FLASHTEX_BOXMEMO_VERIFY_FAIL` is set and a verification found a
/// difference: the run's exit status becomes 3.
pub fn fail_on_difference() -> bool {
    mode() == Mode::Verify && with_config(|c| c.verify_fail) && differences() > 0
}

/// Did any verification find a difference?
pub fn differences() -> u64 {
    ST.with(|s| s.borrow().stats.verify_differences)
}

fn write_stats(force: bool) {
    let Some(out) = with_config(|c| c.stats_out.clone()) else {
        return;
    };
    let text = ST.with(|s| {
        let mut s = s.borrow_mut();
        s.stats_dirty += 1;
        if !force && s.stats_dirty % 32 != 0 {
            return None;
        }
        s.stats.entries = s.store.values().map(|v| v.len()).sum();
        s.stats.bytes = s.bytes;
        Some(format!("{:#?}\n", s.stats))
    });
    if let Some(t) = text {
        let _ = std::fs::write(out, t);
    }
}

/// A fresh version number, never used before in this process.
fn fresh_version() -> i32 {
    use std::sync::atomic::{AtomicI32, Ordering};
    static NEXT: AtomicI32 = AtomicI32::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

thread_local! {
    /// Where BOX-MEMO may record (replays are allowed everywhere): `None`
    /// (the command line) anywhere; in the host, only in an edit's window,
    /// from the engine's resumption to the edited page's shipout -- the
    /// calls the next keystroke runs again. A cold pass's calls never hit
    /// (pgf's picture serial is in their key, BOX-MEMO.md §9), so
    /// recording them would only cost.
    static WINDOW: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

/// The host: an edit's window opens (`true`) or closes.
pub fn record_window(open: bool) {
    WINDOW.with(|w| w.set(Some(open)));
}

/// The engine restored a checkpoint (`Globals::fill_scalars`): a recording
/// in progress, in this timeline or in the one left, cannot continue.
/// A full verification in progress (command line only, BOX-MEMO.md §7):
/// the checkpoint taken where the call was admitted, and the warning index
/// `macro_call` restores at its exit.
struct FullVerify {
    ck: crate::arena::CheckpointId,
    warn: i32,
}

thread_local! {
    static FULL: RefCell<Option<FullVerify>> = const { RefCell::new(None) };
}

/// The scalars a command reads only after writing them (D9's verifier's
/// `EXCLUDED_SCALARS`, which every convergence point also has equal): the
/// replay leaves them as the normal path did.
#[derive(Clone)]
struct Scratch {
    // scratch arrays, written before they are read (D9's list)
    dig: Vec<i32>,
    trick_buf: Vec<i32>,
    pstack: Vec<i32>,
    // `hpack`'s and `vpack`'s glue totals: each pack zeroes all four orders
    // before it adds to them (tex.web §650, §668) and reads them only after
    total_stretch: Vec<i32>,
    total_shrink: Vec<i32>,
    cur_cmd: i32,
    cur_chr: i32,
    cur_cs: i32,
    cur_tok: i32,
    cur_val: i32,
    cur_val_level: i32,
    radix: i32,
    cur_order: i32,
    def_ref: i32,
    long_state: i32,
    // `x_over_n`'s and `xn_over_d`'s remainder (tex.web §106, §107): read
    // only right after the call that sets it (§458, §461, §716, §717)
    remainder: i32,
    // pseudo-printing scratch (D9's list)
    tally: i32,
    first_count: i32,
    trick_count: i32,
    base_ptr: i32,
    // `big_switch`'s flags, set before each `get_x_token`
    intr_at_switch: bool,
    bm_at_switch: bool,
}

impl Scratch {
    fn take(g: &Globals) -> Scratch {
        Scratch {
            dig: (0..g.dig.len()).map(|i| g.dig[i]).collect(),
            trick_buf: (0..g.trick_buf.len()).map(|i| g.trick_buf[i]).collect(),
            pstack: (0..g.pstack.len()).map(|i| g.pstack[i]).collect(),
            total_stretch: (0..g.total_stretch.len()).map(|i| g.total_stretch[i]).collect(),
            total_shrink: (0..g.total_shrink.len()).map(|i| g.total_shrink[i]).collect(),
            cur_cmd: g.cur_cmd,
            cur_chr: g.cur_chr,
            cur_cs: g.cur_cs,
            cur_tok: g.cur_tok,
            cur_val: g.cur_val,
            cur_val_level: g.cur_val_level,
            radix: g.radix,
            cur_order: g.cur_order,
            def_ref: g.def_ref,
            long_state: g.long_state,
            remainder: g.remainder,
            tally: g.tally,
            first_count: g.first_count,
            trick_count: g.trick_count,
            base_ptr: g.base_ptr,
            intr_at_switch: g.intr_at_switch,
            bm_at_switch: g.bm_at_switch,
        }
    }
    fn put(self, g: &mut Globals) {
        for (i, &v) in self.dig.iter().enumerate() {
            if g.dig[i] != v {
                g.dig[i] = v;
            }
        }
        for (i, &v) in self.trick_buf.iter().enumerate() {
            if g.trick_buf[i] != v {
                g.trick_buf[i] = v;
            }
        }
        for (i, &v) in self.pstack.iter().enumerate() {
            if g.pstack[i] != v {
                g.pstack[i] = v;
            }
        }
        for (i, &v) in self.total_stretch.iter().enumerate() {
            if g.total_stretch[i] != v {
                g.total_stretch[i] = v;
            }
        }
        for (i, &v) in self.total_shrink.iter().enumerate() {
            if g.total_shrink[i] != v {
                g.total_shrink[i] = v;
            }
        }
        g.cur_cmd = self.cur_cmd;
        g.cur_chr = self.cur_chr;
        g.cur_cs = self.cur_cs;
        g.cur_tok = self.cur_tok;
        g.cur_val = self.cur_val;
        g.cur_val_level = self.cur_val_level;
        g.radix = self.radix;
        g.cur_order = self.cur_order;
        g.def_ref = self.def_ref;
        g.long_state = self.long_state;
        g.remainder = self.remainder;
        g.tally = self.tally;
        g.first_count = self.first_count;
        g.trick_count = self.trick_count;
        g.base_ptr = self.base_ptr;
        g.intr_at_switch = self.intr_at_switch;
        g.bm_at_switch = self.bm_at_switch;
    }
}

/// Leave the used-up token lists at the top of the input stack, as the
/// next `get_next` would.
fn pop_used_up(g: &mut Globals) {
    while g.cur_input.state_field == TOKEN_LIST
        && g.cur_input.loc_field == 0
        && g.cur_input.index_field != V_TEMPLATE
        && g.input_ptr > 0
    {
        g.end_token_list();
    }
}

pub fn after_restore(g: &mut Globals) {
    if g.bm_rec_on {
        g.bm_rec_on = false;
    }
    ST.with(|s| {
        let mut s = s.borrow_mut();
        if s.rec.take().is_some() {
            *s.stats.abandoned.entry("Restore".into()).or_default() += 1;
        }
    });
}

// ---------------------------------------------------------------------------
// Reading the state
// ---------------------------------------------------------------------------

impl Globals {
    #[inline]
    fn bm_eq(&self, p: i32) -> crate::generated::types::memory_word {
        self.eqtb[(p - 1) as usize]
    }
    #[inline]
    fn bm_link(&self, p: i32) -> i32 {
        self.mem[p as usize].hh().rh()
    }
    #[inline]
    fn bm_info(&self, p: i32) -> i32 {
        self.mem[p as usize].hh().lh()
    }

    /// The tokens of the list after `p` (a reference count or a body).
    fn bm_tokens(&self, mut p: i32, out: &mut Vec<i32>) -> bool {
        let start = out.len();
        while p != 0 {
            if out.len() - start > MAX_LIST {
                return false;
            }
            out.push(self.bm_info(p));
            p = self.bm_link(p);
        }
        true
    }

    /// A token list value (its reference count node `e`), by its tokens.
    fn bm_push_list(&self, e: i32, out: &mut Vec<i32>) {
        if e == 0 {
            out.push(-1);
            return;
        }
        let at = out.len();
        out.push(0);
        if !self.bm_tokens(self.bm_link(e), out) {
            out.truncate(at);
            out.push(-2);
            return;
        }
        out[at] = (out.len() - at - 1) as i32;
    }

    fn bm_glue_fields(&self, e: i32) -> [i32; 5] {
        [
            self.mem[(e + 1) as usize].int(),
            self.mem[(e + 2) as usize].int(),
            self.mem[(e + 3) as usize].int(),
            self.mem[e as usize].hh().b0(),
            self.mem[e as usize].hh().b1(),
        ]
    }

    fn bm_shape_words(&self, e: i32) -> Option<Vec<i32>> {
        if e == 0 {
            return Some(vec![]);
        }
        let n = self.bm_info(e);
        if n < 0 || (2 * n + 1) as usize > MAX_LIST {
            return None;
        }
        // info(p) = n, then the indentations and lengths (`sc`)
        Some(
            std::iter::once(n)
                .chain((1..=2 * n).map(|i| self.mem[(e + i) as usize].int()))
                .collect(),
        )
    }

    /// K2: every `eqtb` entry from `glue_base` to `eqtb_size` but the box
    /// registers, pointers by what they point to.
    fn bm_k2(&self) -> Vec<i32> {
        let mut v = Vec::with_capacity(8192);
        for p in GLUE_BASE..=EQTB_SIZE {
            if (BOX_BASE..BOX_BASE + 256).contains(&p) || Self::bm_is_register(p) {
                continue;
            }
            let w = self.bm_eq(p);
            if p < LOCAL_BASE {
                // glue parameters
                let e = w.hh().rh();
                if e <= LO_MEM_STAT_MAX {
                    v.push(-1);
                    v.push(e);
                } else {
                    v.extend_from_slice(&self.bm_glue_fields(e));
                }
            } else if p == PAR_SHAPE_LOC || (ETEX_PEN_BASE..ETEX_PENS).contains(&p) {
                let e = w.hh().rh();
                match self.bm_shape_words(e) {
                    Some(s) => {
                        v.push(s.len() as i32);
                        v.extend(s);
                    }
                    None => v.push(-2),
                }
            } else if (OUTPUT_ROUTINE_LOC..TOKS_BASE + 256).contains(&p) {
                self.bm_push_list(w.hh().rh(), &mut v);
            } else if p < INT_BASE {
                v.push(w.hh().rh());
            } else {
                v.push(w.int());
            }
        }
        v
    }

    /// Debugging misses: K2 and K4 by location (`eqtb` location, or
    /// `-(type*65536+number)` for a sparse register), each value as K2/K4
    /// keep it.
    fn bm_key_by_loc(&self) -> Vec<(i32, Vec<i32>)> {
        let mut out = vec![];
        for p in GLUE_BASE..=EQTB_SIZE {
            if (BOX_BASE..BOX_BASE + 256).contains(&p) {
                continue;
            }
            let w = self.bm_eq(p);
            let mut v = vec![];
            if p < LOCAL_BASE {
                let e = w.hh().rh();
                if e <= LO_MEM_STAT_MAX {
                    v.push(e);
                } else {
                    v.extend_from_slice(&self.bm_glue_fields(e));
                }
            } else if (OUTPUT_ROUTINE_LOC..TOKS_BASE + 256).contains(&p) {
                self.bm_push_list(w.hh().rh(), &mut v);
            } else if p < INT_BASE {
                v.push(w.hh().rh());
            } else {
                v.push(w.int());
            }
            out.push((p, v));
        }
        for t in [INT_VAL, DIMEN_VAL, GLUE_VAL, MU_VAL, TOK_VAL] {
            let mut items: Vec<(i32, i32)> = vec![];
            self.bm_sa_walk(t, &mut |n, e| items.push((n, e)));
            for (n, e) in items {
                let v = match t {
                    INT_VAL | DIMEN_VAL => vec![self.mem[(e + 2) as usize].int()],
                    GLUE_VAL | MU_VAL => {
                        let q = self.bm_link(e + 1);
                        if q <= LO_MEM_STAT_MAX {
                            vec![q]
                        } else {
                            self.bm_glue_fields(q).to_vec()
                        }
                    }
                    _ => {
                        let mut v = vec![];
                        self.bm_push_list(self.bm_link(e + 1), &mut v);
                        v
                    }
                };
                out.push((-(t * 65536 + n), v));
            }
        }
        out
    }

    fn bm_loc_name(&self, p: i32) -> String {
        if p < 0 {
            let (t, n) = ((-p) / 65536, (-p) % 65536);
            let kind = ["count", "dimen", "skip", "muskip", "box", "toks"][t as usize];
            return format!("\\{kind}{n}");
        }
        let regs = [
            (k::count_base, "count"),
            (k::scaled_base, "dimen"),
            (k::skip_base, "skip"),
            (k::mu_skip_base, "muskip"),
            (TOKS_BASE, "toks"),
        ];
        for (b, n) in regs {
            if (b..b + 256).contains(&p) {
                return format!("\\{n}{}", p - b);
            }
        }
        format!("eqtb[{p}]")
    }

    /// The sparse array elements of type `t`, in order: (number, element).
    fn bm_sa_walk(&self, t: i32, f: &mut dyn FnMut(i32, i32)) {
        let root = self.sa_root[t as usize];
        if root == 0 {
            return;
        }
        let get = |q: i32, i: i32| -> i32 {
            let w = self.mem[(q + i / 2 + 1) as usize].hh();
            if i % 2 == 1 {
                w.rh()
            } else {
                w.lh()
            }
        };
        for d1 in 0..16 {
            let q1 = get(root, d1);
            if q1 == 0 {
                continue;
            }
            for d2 in 0..16 {
                let q2 = get(q1, d2);
                if q2 == 0 {
                    continue;
                }
                for d3 in 0..16 {
                    let q3 = get(q2, d3);
                    if q3 == 0 {
                        continue;
                    }
                    for d4 in 0..16 {
                        let e = get(q3, d4);
                        if e != 0 {
                            f(((d1 * 16 + d2) * 16 + d3) * 16 + d4, e);
                        }
                    }
                }
            }
        }
    }

    /// Is `eqtb` location `p` a register (`\count`, `\dimen`, `\skip`,
    /// `\muskip`, `\toks` 0-255)? Registers are read only explicitly,
    /// never by a primitive behind the scenes, so the key holds those the
    /// recording read (KR) rather than all of them.
    fn bm_is_register(p: i32) -> bool {
        [k::count_base, k::scaled_base, k::skip_base, k::mu_skip_base, TOKS_BASE]
            .iter()
            .any(|&b| (b..b + 256).contains(&p))
    }

    /// A register's value as KR keeps it.
    fn bm_reg_value(&self, r: RegId) -> Vec<i32> {
        let mut v = vec![];
        match r {
            RegId::Eqtb(p) => {
                let w = self.bm_eq(p);
                if (k::skip_base..k::mu_skip_base + 256).contains(&p) {
                    let e = w.hh().rh();
                    if e <= LO_MEM_STAT_MAX {
                        v.push(-1);
                        v.push(e);
                    } else {
                        v.extend_from_slice(&self.bm_glue_fields(e));
                    }
                } else if (TOKS_BASE..TOKS_BASE + 256).contains(&p) {
                    self.bm_push_list(w.hh().rh(), &mut v);
                } else {
                    v.push(w.int());
                }
            }
            RegId::Sa(t, n) => {
                let e = self.bm_sa_find(t, n);
                match t {
                    INT_VAL | DIMEN_VAL => v.push(if e == 0 { 0 } else { self.mem[(e + 2) as usize].int() }),
                    GLUE_VAL | MU_VAL => {
                        let q = if e == 0 { k::zero_glue } else { self.bm_link(e + 1) };
                        if q <= LO_MEM_STAT_MAX {
                            v.push(-1);
                            v.push(q);
                        } else {
                            v.extend_from_slice(&self.bm_glue_fields(q));
                        }
                    }
                    _ => self.bm_push_list(if e == 0 { 0 } else { self.bm_link(e + 1) }, &mut v),
                }
            }
        }
        v
    }

    /// The sparse array element of type `t` and number `n`, or 0, found as
    /// `find_sa_element(t,n,false)` finds it, without changing anything.
    fn bm_sa_find(&self, t: i32, n: i32) -> i32 {
        let get = |q: i32, i: i32| -> i32 {
            let w = self.mem[(q + i / 2 + 1) as usize].hh();
            if i % 2 == 1 {
                w.rh()
            } else {
                w.lh()
            }
        };
        let mut q = self.sa_root[t as usize];
        for i in [n / 4096, (n / 256) % 16, (n / 16) % 16, n % 16] {
            if q == 0 {
                return 0;
            }
            q = get(q, i);
        }
        q
    }

    /// The recording read register `r` (a register it assigned globally
    /// before holds the call's own value: not an input).
    fn bm_note_reg(&mut self, r: RegId) {
        let known = self
            .bm_with_rec(|rec| rec.gwritten.contains(&r) || rec.kr.iter().any(|(x, _)| *x == r))
            .unwrap_or(true);
        if known {
            return;
        }
        let v = self.bm_reg_value(r);
        self.bm_with_rec(|rec| rec.kr.push((r, v)));
    }

    /// `\count`-like register `n` of type `t` (`int_val` .. `tok_val`)
    /// is read by number.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_reg_read(&mut self, t: i32, n: i32) {
        if n > 255 {
            return self.bm_note_reg(RegId::Sa(t, n));
        }
        let base = match t {
            INT_VAL => k::count_base,
            DIMEN_VAL => k::scaled_base,
            GLUE_VAL => k::skip_base,
            MU_VAL => k::mu_skip_base,
            _ => TOKS_BASE,
        };
        self.bm_note_reg(RegId::Eqtb(base + n));
    }

    /// A sparse register element is read through a `\countdef` (and kin).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_sa_read(&mut self, p: i32) {
        let t = self.mem[p as usize].hh().b0() / 16;
        let n = if t <= DIMEN_VAL {
            self.bm_link(p + 1)
        } else {
            self.bm_sa_number(p)
        };
        self.bm_note_reg(RegId::Sa(t, n));
    }

    /// `\advance`, `\multiply`, `\divide` read their register: `eqtb`
    /// location `l`, or the sparse element `l` when `e`.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_reg_loc(&mut self, l: i32, e: bool) {
        if e {
            self.flashtex_bm_sa_read(l)
        } else if Self::bm_is_register(l) {
            self.bm_note_reg(RegId::Eqtb(l))
        }
    }

    /// Sparse registers are keyed precisely (KR): K4 is empty.
    fn bm_k4(&self) -> Vec<i32> {
        vec![]
    }

    /// The register number of sparse element `p` (from its place in the
    /// tree: each index node keeps its four-bit digit in `sa_index`).
    fn bm_sa_number(&self, p: i32) -> i32 {
        let mut n = self.mem[p as usize].hh().b0() % 16;
        let mut q = self.bm_link(p);
        let mut shift = 4;
        for _ in 0..3 {
            n += (self.mem[q as usize].hh().b0() % 16) << shift;
            shift += 4;
            q = self.bm_link(q);
        }
        n
    }

    /// K3: the meaning of control sequence `p`, or `None` if it cannot be
    /// kept by value (a sparse register's `\countdef`).
    fn bm_meaning(&self, p: i32) -> Option<Meaning> {
        let w = self.bm_eq(p).hh();
        let (t, e) = (w.b0(), w.rh());
        if (CALL..=LONG_OUTER_CALL).contains(&t) && e != 0 {
            let mut v = vec![];
            if !self.bm_tokens(self.bm_link(e), &mut v) {
                return None;
            }
            return Some(Meaning::Macro(t, v));
        }
        if (t == REGISTER || t == TOKS_REGISTER) && e > LO_MEM_STAT_MAX {
            let st = self.mem[e as usize].hh().b0() / 16;
            let n = if st <= DIMEN_VAL {
                self.bm_link(e + 1)
            } else {
                self.bm_sa_number(e)
            };
            return Some(Meaning::Sparse(t, st, n));
        }
        Some(Meaning::Plain(t, e))
    }

    /// The control sequence named `name` (internal codes, two or more), as
    /// `id_lookup` finds it, without entering it.
    fn bm_find_cs(&self, name: &[i32]) -> Option<i32> {
        if name.len() < 2 {
            return None;
        }
        let prime = k::hash_prime as i64;
        let mut h = name[0] as i64;
        for &c in &name[1..] {
            h = h + h + c as i64;
            while h >= prime {
                h -= prime;
            }
        }
        let hb = k::hash_base;
        let mut p = h as i32 + hb;
        loop {
            let w = self.hash[(p - hb) as usize];
            let t = w.rh();
            if t > 0 && t < self.str_ptr {
                let (a, b) = (
                    self.str_start[t as usize] as usize,
                    self.str_start[t as usize + 1] as usize,
                );
                if b - a == name.len()
                    && self.str_pool[a..b].iter().zip(name).all(|(&c, &n)| c == n)
                {
                    return Some(p);
                }
            }
            let next = w.lh();
            if next == 0 {
                return None;
            }
            p = next;
        }
    }

    fn bm_is_cs(p: i32) -> bool {
        (1..GLUE_BASE).contains(&p) || p > EQTB_SIZE
    }

    fn bm_k1(&self, cs: i32, n: i32) -> Vec<i32> {
        let mut v = vec![];
        let w = self.bm_eq(cs).hh();
        v.push(w.b0());
        self.bm_push_list(w.rh(), &mut v);
        for i in 0..n {
            let l = self.param_stack[(self.param_ptr - n + i) as usize];
            v.push(-3);
            let at = v.len();
            v.push(0);
            if !self.bm_tokens(l, &mut v) {
                v.truncate(at);
                v.push(-2);
                continue;
            }
            v[at] = (v.len() - at - 1) as i32;
        }
        v
    }

    fn bm_ctx(&self, scanner: i32) -> Context {
        Context {
            mode: self.cur_list.mode_field,
            interaction: self.interaction,
            align_state: self.align_state,
            scanner_status: scanner,
            par_token: self.par_token,
            font_version: self.bm_font_version,
            hyph_version: self.bm_hyph_version,
        }
    }

    fn bm_log_len(&mut self) -> u64 {
        if !self.log_opened {
            return 0;
        }
        match self.log_file.snapshot() {
            Ok(s) => match s.stream {
                crate::system::Stream::Out { len, .. } => len,
                _ => 0,
            },
            Err(_) => u64::MAX,
        }
    }

    fn bm_frame(&mut self) -> Frame {
        let page: Vec<i64> = (0..8)
            .map(|i| self.page_so_far[i] as i64)
            .chain([
                self.page_tail as i64,
                self.page_contents as i64,
                self.best_page_break as i64,
                self.least_page_cost as i64,
                self.best_size as i64,
                self.page_max_depth as i64,
                self.last_glue as i64,
                self.last_penalty as i64,
                self.last_kern as i64,
                self.insert_penalties as i64,
                self.output_active as i64,
                self.dead_cycles as i64,
                self.bm_link(CONTRIB_HEAD) as i64,
                self.bm_link(PAGE_HEAD) as i64,
            ])
            .collect();
        let font_glue = (0..=self.font_ptr)
            .map(|f| self.font_glue[f as usize])
            .collect();
        let mut marks: Vec<i32> = (0..5).map(|i| self.cur_mark[i]).collect();
        marks.extend((0..2).map(|i| self.disc_ptr[i + 1]));
        marks.push(self.sa_root[k::mark_val as usize]);
        let log = self.bm_log_len();
        Frame {
            cur_level: self.cur_level,
            save_ptr: self.save_ptr,
            cond: (self.cond_ptr, self.if_limit, self.cur_if, self.if_line),
            align_state: self.align_state,
            strings: (self.str_ptr, self.pool_ptr, self.hash_used),
            errors: (self.error_count, self.history),
            nest_ptr: self.nest_ptr,
            mode: self.cur_list.mode_field,
            tail: self.cur_list.tail_field,
            page,
            fonts: (self.font_ptr, self.fmem_ptr),
            font_glue,
            marks,
            obj_ptr: self.obj_ptr,
            interaction: self.interaction,
            arm: (self.ckpt_arm_cs, self.ckpt_arm_level),
            outputs: (
                self.file_offset,
                self.term_offset,
                log,
                crate::system::terminal_len(),
                crate::system::external_effects_len(),
            ),
            lines: crate::lineshift::journal_mark(),
        }
    }
}

// ---------------------------------------------------------------------------
// The engine's hooks (changes/boxmemo.ch)
// ---------------------------------------------------------------------------

impl Globals {
    /// `Set init`: is BOX-MEMO switched on?
    pub fn flashtex_bm_enabled(&mut self) -> bool {
        mode() != Mode::Off
    }

    /// After a format is loaded: mark the registered macros it defines.
    pub fn flashtex_bm_loaded(&mut self) {
        let names = with_config(|c| c.names.clone());
        for p in 1..=k::eqtb_top {
            if !Self::bm_is_cs(p) || p < k::hash_base {
                continue;
            }
            if self.bm_name_is(p, &names) {
                self.bm_cand[p as usize] = true;
            }
        }
    }

    fn bm_name_is(&self, p: i32, names: &[Vec<u8>]) -> bool {
        let i = p - k::hash_base;
        if i < 0 || i as usize >= self.hash.len() {
            return false;
        }
        let t = self.hash[i as usize].rh();
        if t <= 0 || t >= self.str_ptr {
            return false;
        }
        let (a, b) = (
            self.str_start[t as usize] as usize,
            self.str_start[t as usize + 1] as usize,
        );
        names.iter().any(|n| {
            // `*suffix`: every name that ends with it (ntheorem's
            // `\<env>@framecommand`)
            if let Some(suf) = n.strip_prefix(b"*") {
                return b - a >= suf.len()
                    && self.str_pool[b - suf.len()..b]
                        .iter()
                        .zip(suf)
                        .all(|(&c, &d)| c == d as i32);
            }
            b - a == n.len()
                && self.str_pool[a..b]
                    .iter()
                    .zip(n)
                    .all(|(&c, &d)| c == d as i32)
        })
    }

    /// A control sequence was just entered into the hash.
    pub fn flashtex_bm_new_cs(&mut self, p: i32) {
        let names = with_config(|c| c.names.clone());
        if self.bm_name_is(p, &names) {
            self.bm_cand[p as usize] = true;
        }
    }

    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_font_changed(&mut self) {
        self.bm_font_version = fresh_version();
    }

    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_hyph_changed(&mut self) {
        self.bm_hyph_version = fresh_version();
    }

    /// `macro_call` fed the body of a registered macro (`warning_index`)
    /// and its `n` arguments; the body runs with scanner status `scanner`.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_call(&mut self, n: i32, scanner: i32, warn: i32) {
        // Never inside a recording, BOX-MEMO's or the intrinsics' (the
        // boundary with MACRO-REPLAY, BOX-MEMO.md §1).
        if self.bm_rec_on || self.intr_rec_on {
            return;
        }
        let cs = self.warning_index;
        // the body must be the top level, as `Feed` left it
        if self.cur_input.state_field != TOKEN_LIST || self.cur_input.index_field != k::macro_ {
            return;
        }
        if cs == self.ckpt_arm_cs || self.ckpt_arm_level == self.input_ptr {
            return;
        }
        let m = mode();
        ST.with(|s| s.borrow_mut().stats.offers += 1);
        // preconditions (K6)
        let globaldefs = self.bm_eq(INT_BASE + k::global_defs_code).int();
        if globaldefs != 0 || self.after_token != 0 {
            return self.bm_miss("Preconditions");
        }
        let k1 = self.bm_k1(cs, n);
        let ctx = self.bm_ctx(scanner);
        let k2 = self.bm_k2();
        let k4 = self.bm_k4();
        let hash = key_hash(&k1, &ctx, &k2, &k4);
        // the variants whose K1, context, K2 and K4 hold (most recent first)
        let candidates: Vec<Candidate> = ST.with(|s| {
            let s = s.borrow();
            let Some(v) = s.store.get(&cs) else {
                return vec![];
            };
            let mut c: Vec<_> = v
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    e.hash == hash
                        && e.k1 == k1
                        && e.ctx == ctx
                        && e.k2 == k2
                        && e.k4 == k4
                        && e.line.is_none_or(|l| l == self.line)
                })
                .map(|(i, e)| {
                    (
                        e.last_used,
                        i,
                        e.k3.clone(),
                        e.absent.clone(),
                        e.voids.clone(),
                        e.kr.clone(),
                        e.kb.iter().chain(&e.holes).cloned().collect::<Vec<_>>(),
                    )
                })
                .collect();
            c.sort_by_key(|x| std::cmp::Reverse(x.0));
            c.into_iter()
                .map(|(_, i, a, b, d, r, x)| (i, a, b, d, r, x))
                .collect()
        });
        let mut why = "NotRecorded";
        // ... and of those, the first whose K3 and K7 hold
        for (i, k3, absent, voids, kr, kb) in candidates {
            let absent_ok = absent.iter().all(|n| match self.bm_find_cs(n) {
                None => true,
                Some(p) => self.bm_eq(p).hh().b0() == k::undefined_cs,
            }) && voids.iter().all(|&n| self.bm_box_value(n) == 0)
                && kr.iter().all(|(r, v)| self.bm_reg_value(*r) == *v)
                && kb
                    .iter()
                    .all(|(n, v)| self.bm_box_key(self.bm_box_value(*n)) == *v);
            if absent_ok
                && k3
                    .iter()
                    .all(|(p, m)| self.bm_meaning(*p).as_ref() == Some(m))
            {
                if m == Mode::Verify {
                    // the full verifier. In the host its restore replaces
                    // the edit's pending branch, so a compile with a
                    // verified call does not converge: verify is a gate
                    // mode, and the output must still equal a full run's
                    {
                        if let Ok(ck) = self.checkpoint() {
                            FULL.with(|f| *f.borrow_mut() = Some(FullVerify { ck, warn }));
                        }
                    }
                    return self.bm_start(cs, n, scanner, k1, ctx, k2, k4, Some(i));
                }
                return self.bm_replay(cs, i);
            }
            why = "Meanings";
            if debug() {
                let bad: Vec<String> = k3
                    .iter()
                    .filter(|(p, m)| self.bm_meaning(*p).as_ref() != Some(m))
                    .take(4)
                    .map(|(p, m)| {
                        format!(
                            "\\{} was {m:?} now {:?}",
                            self.cs_name_string(*p),
                            self.bm_meaning(*p)
                        )
                    })
                    .collect();
                let regs: Vec<String> = kr
                    .iter()
                    .filter(|(r, v)| self.bm_reg_value(*r) != *v)
                    .take(6)
                    .map(|(r, v)| format!("{r:?} was {v:?} now {:?}", self.bm_reg_value(*r)))
                    .collect();
                eprintln!(
                    "boxmemo: \\{} key differs: absent ok {absent_ok}; {bad:?}; registers {regs:?}",
                    self.cs_name_string(cs)
                );
            }
        }
        self.bm_miss(why);
        if debug() && why == "NotRecorded" {
            let now = self.bm_key_by_loc();
            let diff: Vec<String> = LAST_KEY.with(|l| {
                l.borrow().get(&cs).map_or(vec![], |old| {
                    let om: HashMap<i32, &Vec<i32>> = old.iter().map(|(p, v)| (*p, v)).collect();
                    now.iter()
                        .filter(|(p, v)| om.get(p).is_none_or(|o| *o != v))
                        .map(|(p, v)| format!("{}={:?}", self.bm_loc_name(*p), &v[..v.len().min(3)]))
                        .take(12)
                        .collect()
                })
            });
            if !diff.is_empty() {
                eprintln!("boxmemo: \\{} miss, key differs at {diff:?}", self.cs_name_string(cs));
            }
            LAST_KEY.with(|l| l.borrow_mut().insert(cs, now));
        }
        if WINDOW.with(|w| w.get()) == Some(false) {
            return;
        }
        let refused = ST.with(|s| s.borrow().aborts.get(&cs).copied().unwrap_or(0) >= ABORT_BUDGET);
        if refused {
            return;
        }
        self.bm_start(cs, n, scanner, k1, ctx, k2, k4, None);
    }

    fn bm_miss(&mut self, why: &str) {
        ST.with(|s| {
            *s.borrow_mut().stats.misses.entry(why.into()).or_default() += 1;
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn bm_start(
        &mut self,
        cs: i32,
        _n: i32,
        scanner: i32,
        k1: Vec<i32>,
        ctx: Context,
        k2: Vec<i32>,
        k4: Vec<i32>,
        verify: Option<usize>,
    ) {
        let start = self.bm_frame();
        let rec = Rec {
            cs,
            base: self.input_ptr,
            scanner,
            start,
            k1,
            ctx,
            k2,
            k4,
            k3: vec![],
            absent: vec![],
            voids: vec![],
            kr: vec![],
            gwritten: HashSet::new(),
            line_read: false,
            boxes: HashSet::new(),
            gboxes: vec![],
            pending_box: None,
            kb: vec![],
            holes: vec![],
            ops: vec![],
            verify,
        };
        // the macro's own meaning is part of K1; mark it seen
        let base = self.input_ptr;
        let size = (k::eqtb_top + 1) as usize;
        FAST.with(|f| {
            let mut f = f.borrow_mut();
            f.base = base;
            f.stamp = f.stamp.wrapping_add(1);
            if f.stamp == 0 || f.seen.len() < size {
                f.seen = vec![0; size];
                f.stamp = 1;
            }
            let st = f.stamp;
            f.seen[cs as usize] = st;
        });
        ST.with(|s| {
            let mut s = s.borrow_mut();
            s.stats.recordings += 1;
            s.rec = Some(rec);
        });
        self.bm_rec_on = true;
    }

    fn bm_abort(&mut self, why: &str) {
        if !self.bm_rec_on {
            return;
        }
        self.bm_rec_on = false;
        let dbg = debug();
        let rec = ST.with(|s| s.borrow_mut().rec.take());
        let Some(rec) = rec else { return };
        if rec.verify.is_some() {
            if let Some(fv) = FULL.with(|f| f.borrow_mut().take()) {
                self.retain_checkpoints(&|id| id != fv.ck);
            }
        }
        if dbg {
            eprintln!(
                "boxmemo: recording of \\{} abandoned: {why} (cmd {} chr {} cs {})",
                self.cs_name_string(rec.cs),
                self.cur_cmd,
                self.cur_chr,
                if self.cur_cs > 0 {
                    self.cs_name_string(self.cur_cs)
                } else {
                    String::new()
                }
            );
        }
        ST.with(|s| {
            let mut s = s.borrow_mut();
            *s.stats.abandoned.entry(why.into()).or_default() += 1;
            if rec.verify.is_some() {
                s.stats.verified += 1;
                s.stats.verify_differences += 1;
                s.stats.full_differences += 1;
                if s.stats.verify_details.len() < 20 {
                    s.stats.verify_details.push(format!(
                        "normal path not pure after the guard passed: {why}"
                    ));
                }
                // drop the entry: the guard let through a call it should not have
                if let Some(v) = s.store.get_mut(&rec.cs) {
                    if let Some(i) = rec.verify {
                        if i < v.len() {
                            let e = v.remove(i);
                            s.bytes = s.bytes.saturating_sub(e.bytes);
                        }
                    }
                }
            } else {
                *s.aborts.entry(rec.cs).or_default() += 1;
            }
        });
        write_stats(false);
        fail_now();
    }

    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_abort(&mut self, r: i32) {
        match r {
            1 => self.bm_abort("Mag"),
            _ => self.bm_abort("Other"),
        }
    }

    fn bm_with_rec<R>(&self, f: impl FnOnce(&mut Rec) -> R) -> Option<R> {
        ST.with(|s| s.borrow_mut().rec.as_mut().map(f))
    }

    /// First access to control sequence `p` (read, or write before it).
    fn bm_note_cs(&mut self, p: i32) {
        if !Self::bm_is_cs(p) {
            return;
        }
        let seen = FAST.with(|f| {
            let mut f = f.borrow_mut();
            let st = f.stamp;
            let e = &mut f.seen[p as usize];
            let was = *e == st;
            *e = st;
            was
        });
        if seen {
            return;
        }
        match self.bm_meaning(p) {
            Some(m) => {
                self.bm_with_rec(|r| {
                    r.k3.push((p, m));
                });
            }
            None => self.bm_abort("SparseMeaning"),
        }
    }

    /// The end of `get_next`, during a recording.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_next(&mut self) {
        let base = FAST.with(|f| f.borrow().base);
        if self.input_ptr < base {
            return self.bm_abort("Level");
        }
        if self.cur_input.state_field != TOKEN_LIST {
            return self.bm_abort("File");
        }
        if self.cur_cs != 0 {
            if self.cur_cmd == k::top_bot_mark {
                return self.bm_abort("Mark");
            }
            self.bm_note_cs(self.cur_cs);
        }
    }

    /// `\csname` and `\ifcsname` looked `p` up.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_read(&mut self, p: i32) {
        // a name not found is noted by `flashtex_bm_id`
        if p != UNDEFINED_CS {
            self.bm_note_cs(p);
        }
    }

    /// `id_lookup` looked `buffer[j..j+l)` up and returns `p`.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_id(&mut self, j: i32, l: i32, p: i32) {
        if p != UNDEFINED_CS {
            return self.bm_note_cs(p);
        }
        let name: Vec<i32> = (j..j + l).map(|i| self.buffer[i as usize]).collect();
        self.bm_with_rec(|r| {
            if !r.absent.contains(&name) {
                r.absent.push(name);
            }
        });
    }

    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_group(&mut self, c: i32) {
        self.bm_with_rec(|r| r.ops.push(Op::Begin(c)));
    }

    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_unsave(&mut self) {
        let level = self.bm_with_rec(|r| r.start.cur_level).unwrap_or(0);
        if self.cur_level <= level {
            return self.bm_abort("Group");
        }
        self.bm_with_rec(|r| r.ops.push(Op::End));
    }

    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_pop_cond(&mut self) {
        let c = self.bm_with_rec(|r| r.start.cond.0).unwrap_or(-1);
        if self.cond_ptr == c {
            self.bm_abort("Cond");
        }
    }

    /// The value assignment `(t, e)` stores, kept by value.
    fn bm_val(&mut self, p: i32, t: i32, e: i32) -> Result<Val, &'static str> {
        if t == GLUE_REF {
            if e <= LO_MEM_STAT_MAX {
                return Ok(Val::StaticGlue(e));
            }
            if self.bm_link(e) != 0 {
                return self.bm_glue_source(e).ok_or("SharedGlue");
            }
            return Ok(Val::FreshGlue(self.bm_glue_fields(e)));
        }
        if t == SHAPE_REF {
            if (ETEX_PEN_BASE..ETEX_PENS).contains(&p) {
                return Err("PenaltyShape");
            }
            return match self.bm_shape_words(e) {
                Some(w) if !w.is_empty() => Ok(Val::FreshShape(w)),
                Some(_) => Ok(Val::Plain(0)),
                None => Err("LongShape"),
            };
        }
        if (CALL..=LONG_OUTER_CALL).contains(&t) && e != 0 {
            if self.bm_info(e) == 0 {
                let mut v = vec![];
                if !self.bm_tokens(self.bm_link(e), &mut v) {
                    return Err("LongList");
                }
                return Ok(Val::FreshList(v));
            }
            let src = self.cur_cs;
            if src != 0 {
                let w = self.bm_eq(src).hh();
                if w.b0() == t && w.rh() == e {
                    return Ok(Val::LetCs(src));
                }
            }
            return Err("SharedList");
        }
        if (t == REGISTER || t == TOKS_REGISTER) && e > LO_MEM_STAT_MAX {
            // `\let` of a sparse register's `\countdef` (and kin): shares
            // the element, as `\let` does (`add_sa_ref`)
            let src = self.cur_cs;
            if src != 0 {
                let w = self.bm_eq(src).hh();
                if w.b0() == t && w.rh() == e {
                    return Ok(Val::LetCs(src));
                }
            }
            return Err("SparseRef");
        }
        Ok(Val::Plain(e))
    }

    /// A shared glue specification `e` being assigned: the one register
    /// that holds it (the one `scan_glue` just read, which took a reference
    /// to it), or `None` when it is not exactly one.
    fn bm_glue_source(&self, e: i32) -> Option<Val> {
        // The register `scan_glue` just read is still the current token's
        // (`\skipdef`'d: `assign_glue` with its location; e-TeX's sparse one:
        // `register` with its element).
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        if (c == k::assign_glue || c == k::assign_mu_glue)
            && (GLUE_BASE..LOCAL_BASE).contains(&chr)
            && self.bm_eq(chr).hh().rh() == e
        {
            return Some(Val::GlueFrom(chr, 0));
        }
        if c == REGISTER && chr > LO_MEM_STAT_MAX && self.bm_link(chr + 1) == e {
            let t = self.mem[chr as usize].hh().b0() / 16;
            if t == GLUE_VAL || t == MU_VAL {
                return Some(Val::GlueFrom(-t, self.bm_sa_number(chr)));
            }
        }
        let mut found: Vec<Val> = vec![];
        for p in GLUE_BASE..LOCAL_BASE {
            if self.bm_eq(p).hh().rh() == e {
                found.push(Val::GlueFrom(p, 0));
            }
        }
        for t in [GLUE_VAL, MU_VAL] {
            self.bm_sa_walk(t, &mut |n, q| {
                if self.bm_link(q + 1) == e {
                    found.push(Val::GlueFrom(-t, n));
                }
            });
        }
        if found.len() == 1 {
            found.pop()
        } else {
            None
        }
    }

    /// `eq_define` (0), `eq_word_define` (1), `geq_define` (2),
    /// `geq_word_define` (3), before they change `eqtb[p]`.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_def(&mut self, p: i32, t: i32, e: i32, kind: i32) {
        let level = self.bm_with_rec(|r| r.start.cur_level).unwrap_or(0);
        let global = kind >= 2;
        if !global && self.cur_level == level && self.cur_level > LEVEL_ONE {
            if debug() {
                eprintln!(
                    "boxmemo: local assignment at the call's own level to {} ({p})",
                    if Self::bm_is_cs(p) { self.cs_name_string(p) } else { self.bm_loc_name(p) }
                );
            }
            return self.bm_abort("OuterLocal");
        }
        if (BOX_BASE..BOX_BASE + 256).contains(&p) {
            // A box made inside the call stays inside it (its local
            // assignments are undone by its groups); a global one may only
            // make the register void (TikZ's `\tikz@figbox`), replayed.
            if global {
                self.bm_with_rec(|r| {
                    if !r.gboxes.contains(&(p - BOX_BASE)) {
                        r.gboxes.push(p - BOX_BASE)
                    }
                });
            }
            self.bm_with_rec(|r| r.boxes.insert(p - BOX_BASE));
            return;
        }
        if t == BOX_REF {
            return self.bm_abort("BoxRef");
        }
        // The old value of what an assignment overwrites is not an input:
        // the replay's own `eq_define` & co. treat whatever is there as the
        // normal path did. A global assignment makes later reads the call's
        // own; a local one is undone when its group ends, and a later read
        // then sees the old value, so that value is keyed now.
        if global {
            if Self::bm_is_register(p) {
                self.bm_with_rec(|r| r.gwritten.insert(RegId::Eqtb(p)));
            } else if Self::bm_is_cs(p) {
                FAST.with(|f| {
                    let mut f = f.borrow_mut();
                    let st = f.stamp;
                    f.seen[p as usize] = st;
                });
            }
        } else if Self::bm_is_register(p) {
            self.bm_note_reg(RegId::Eqtb(p));
        } else {
            self.bm_note_cs(p);
        }
        if !self.bm_rec_on {
            return;
        }
        let v = if kind == 1 || kind == 3 {
            Val::Word(e)
        } else {
            match self.bm_val(p, t, e) {
                Ok(v) => v,
                Err(w) => return self.bm_abort(w),
            }
        };
        self.bm_with_rec(|r| r.ops.push(Op::Def { p, t, v, kind }));
    }

    /// e-TeX's `sa_def` (0), `sa_w_def` (1), `gsa_def` (2), `gsa_w_def` (3).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_sa_def(&mut self, p: i32, e: i32, kind: i32) {
        let level = self.bm_with_rec(|r| r.start.cur_level).unwrap_or(0);
        let global = kind >= 2;
        if !global && self.cur_level == level && self.cur_level > LEVEL_ONE {
            return self.bm_abort("OuterLocal");
        }
        let t = self.mem[p as usize].hh().b0() / 16;
        let n = if t <= DIMEN_VAL {
            self.bm_link(p + 1)
        } else {
            self.bm_sa_number(p)
        };
        if t == BOX_VAL {
            if global {
                self.bm_with_rec(|r| {
                    if !r.gboxes.contains(&n) {
                        r.gboxes.push(n)
                    }
                });
            }
            self.bm_with_rec(|r| r.boxes.insert(n));
            return;
        }
        if global {
            self.bm_with_rec(|r| r.gwritten.insert(RegId::Sa(t, n)));
        } else {
            self.bm_note_reg(RegId::Sa(t, n));
        }
        let v = if kind == 1 || kind == 3 {
            Val::Word(e)
        } else {
            let ty = if t == TOK_VAL { CALL } else { GLUE_REF };
            match self.bm_val(0, ty, e) {
                Ok(v) => v,
                Err(w) => return self.bm_abort(w),
            }
        };
        self.bm_with_rec(|r| r.ops.push(Op::Sa { t, n, v, kind }));
    }

    /// `fetch_box`: box register `n` is read. One the call wrote is its
    /// own; one it did not is part of the key while it is void (LaTeX's
    /// `\voidb@x`), else the recording is abandoned.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_box(&mut self, n: i32) {
        let pending = self.bm_with_rec(|r| r.pending_box.is_some()).unwrap_or(false);
        if pending {
            // the previous fetch was not a dimension, a test or a move: the
            // box's contents were read (`\copy`, `\unhbox`, `\vsplit`, ...)
            return self.bm_abort("BoxContent");
        }
        let written = self.bm_with_rec(|r| r.boxes.contains(&n)).unwrap_or(false);
        if written {
            return;
        }
        let p = self.bm_box_value(n);
        if p == 0 {
            self.bm_with_rec(|r| {
                if !r.voids.contains(&n) {
                    r.voids.push(n);
                }
            });
            return;
        }
        self.bm_with_rec(|r| r.pending_box = Some((n, p)));
    }

    /// The box register a recording just fetched is used: `k` 1 for a
    /// dimension (`\wd` & co.), 2 for a test (`\ifvoid`, `\ifhbox`), 3 for a
    /// move (`\box`, which voids the register).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_box_use(&mut self, n: i32, k: i32) {
        let Some(Some((m, p))) = self.bm_with_rec(|r| r.pending_box.take()) else {
            return;
        };
        if m != n {
            return self.bm_abort("BoxContent");
        }
        let key = self.bm_box_key(p);
        self.bm_with_rec(|r| {
            if k == 3 {
                r.holes.push((n, p, key));
                // void now, and the call's own from here on
                r.boxes.insert(n);
            } else if !r.kb.iter().any(|(x, _)| *x == n) {
                r.kb.push((n, key));
            }
        });
    }

    /// What a box's surroundings can see of it without reading its
    /// contents (`hpack` and `vpack` read a box node's width, height, depth
    /// and shift, tex.web §653, §669; `\ifhbox` its type).
    fn bm_box_key(&self, p: i32) -> Vec<i32> {
        if p == 0 {
            return vec![-1];
        }
        let m = |o: i32| self.mem[(p + o) as usize].int();
        vec![self.mem[p as usize].hh().b0(), m(1), m(2), m(3), m(4)]
    }

    /// `change_box(null)`: box register `n` becomes void at its level
    /// (tex.web §1079, e-TeX's `set_sa_box`).
    fn bm_void_box(&mut self, n: i32) {
        if n < 256 {
            let i = (BOX_BASE + n - 1) as usize;
            self.eqtb[i].set_hh_rh(0);
        } else {
            self.find_sa_element(BOX_VAL, n, false);
            let q = self.cur_ptr;
            if q != 0 {
                self.mem[(q + 1) as usize].set_hh_rh(0);
                let r = self.mem[(q + 1) as usize].hh().lh() + 1;
                self.mem[(q + 1) as usize].set_hh_lh(r);
                self.delete_sa_ref(q);
            }
        }
    }

    /// `box(n)` (a sparse one looked up without creating anything).
    fn bm_box_value(&self, n: i32) -> i32 {
        if n < 256 {
            return self.bm_eq(BOX_BASE + n).hh().rh();
        }
        let get = |q: i32, i: i32| -> i32 {
            let w = self.mem[(q + i / 2 + 1) as usize].hh();
            if i % 2 == 1 {
                w.rh()
            } else {
                w.lh()
            }
        };
        let mut q = self.sa_root[BOX_VAL as usize];
        for i in [n / 4096, (n / 256) % 16, (n / 16) % 16, n % 16] {
            if q == 0 {
                return 0;
            }
            q = get(q, i);
        }
        if q == 0 {
            0
        } else {
            self.bm_link(q + 1)
        }
    }

    /// `expand`, at `reswitch`, during a recording.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_expand(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        let ok = match c {
            c if (CALL..k::end_template).contains(&c) => true,
            c if c == k::end_template => true,
            c if c == k::expand_after || c == k::no_expand || c == k::cs_name => true,
            c if c == k::fi_or_else || c == k::the => true,
            c if c == k::if_test => chr % k::unless_code != k::if_eof_code,
            c if c == k::convert => [
                k::number_code,
                k::roman_numeral_code,
                k::string_code,
                k::meaning_code,
                k::font_name_code,
                k::eTeX_revision_code,
                k::pdftex_revision_code,
                k::pdftex_banner_code,
                k::pdf_strcmp_code,
                k::pdf_escape_string_code,
                k::pdf_escape_name_code,
                k::pdf_escape_hex_code,
                k::pdf_unescape_hex_code,
                k::expanded_code,
                k::job_name_code,
            ]
            .contains(&chr),
            _ => false,
        };
        if !ok {
            self.bm_abort("Expandable");
        }
    }

    /// `scan_something_internal`, during a recording.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_internal(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        let outer = self.bm_with_rec(|r| r.start.nest_ptr).unwrap_or(i32::MAX);
        if c == k::last_item {
            let ok = match chr {
                // \lastpenalty \lastkern \lastskip \lastnodetype: of a list
                // the call made, never the one it was called in
                0..=3 => self.nest_ptr > outer,
                // \pdftexversion, \eTeXversion, \gluestretchorder,
                // \glueshrinkorder, \fontchar.., \parshape..,
                // \gluestretch, \glueshrink, \mutoglue, \gluetomu, the
                // \..expr scanners
                6 | 20 | 26..=42 => true,
                // \inputlineno: `line` joins the key (the call reads no
                // file, so it is the same throughout)
                4 => {
                    self.bm_with_rec(|r| r.line_read = true);
                    true
                }
                _ => false,
            };
            if !ok {
                self.bm_abort("LastItem");
            }
        } else if (c == k::assign_int
            || c == k::assign_dimen
            || c == k::assign_glue
            || c == k::assign_mu_glue
            || c == k::assign_toks)
            && Self::bm_is_register(chr)
        {
            self.bm_note_reg(RegId::Eqtb(chr));
        } else if c == k::set_page_dimen || c == k::set_page_int || c == k::set_prev_graf {
            self.bm_abort("Page");
        } else if c == k::set_aux && self.nest_ptr <= outer {
            self.bm_abort("OuterList");
        }
    }

    /// `main_control` at `reswitch`, and `prefixed_command`, during a
    /// recording.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_command(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        let outer = self.bm_with_rec(|r| r.start.nest_ptr).unwrap_or(i32::MAX);
        let ok = if c == k::extension {
            [
                k::open_node,
                k::write_node,
                k::close_node,
                k::special_node,
                k::set_language_code,
                k::pdf_literal_node,
                k::pdf_colorstack_node,
                k::pdf_setmatrix_node,
                k::pdf_save_node,
                k::pdf_restore_node,
                k::pdf_save_pos_node,
            ]
            .contains(&chr)
        } else if [
            k::stop,
            k::in_stream,
            k::read_to_cs,
            k::set_interaction,
            k::hyph_data,
            k::assign_font_dimen,
            k::assign_font_int,
            k::message,
            k::xray,
            k::set_page_dimen,
            k::set_page_int,
            k::set_prev_graf,
        ]
        .contains(&c)
        {
            false
        } else if c == k::leader_ship {
            chr != k::a_leaders - 1
        } else if c == k::remove_item || c == k::set_aux {
            self.nest_ptr > outer
        } else if c == k::make_box {
            if chr == k::vsplit_code {
                false
            } else if chr == k::last_box_code {
                self.nest_ptr > outer
            } else {
                true
            }
        } else {
            true
        };
        if !ok {
            self.bm_abort("Command");
        }
    }

    /// Is the recorded body used up (every input level from the body's on
    /// a token list with nothing left)?
    fn bm_exhausted(&self, base: i32) -> bool {
        if self.input_ptr < base {
            return true;
        }
        (base..=self.input_ptr).all(|j| {
            let l = if j == self.input_ptr {
                self.cur_input
            } else {
                self.input_stack[j as usize]
            };
            l.state_field == TOKEN_LIST && l.loc_field == 0 && l.index_field != V_TEMPLATE
        })
    }

    /// `big_switch`, during a recording: finish it when the body is done.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_switch(&mut self) {
        let Some(base) = self.bm_with_rec(|r| r.base) else {
            self.bm_rec_on = false;
            return;
        };
        if !self.bm_exhausted(base) {
            return;
        }
        let end = self.bm_frame();
        let (start, scanner) = self.bm_with_rec(|r| (r.start.clone(), r.scanner)).unwrap();
        let why = if end.cur_level != start.cur_level || end.save_ptr != start.save_ptr {
            Some("Group")
        } else if end.cond != start.cond {
            Some("Cond")
        } else if end.align_state != start.align_state {
            Some("Unbalanced")
        } else if end.strings != start.strings {
            Some("NewCs")
        } else if end.errors != start.errors {
            Some("Error")
        } else if self.after_token != 0 || self.scanner_status != scanner {
            Some("State")
        } else if end.nest_ptr != start.nest_ptr || end.mode != start.mode {
            Some("OuterList")
        } else if end.tail != start.tail && start.mode != -k::hmode {
            // only in restricted horizontal mode may a call append: no
            // paragraph, no page builder
            Some("OuterList")
        } else if self.bm_with_rec(|r| r.pending_box.is_some()).unwrap_or(false) {
            Some("BoxContent")
        } else if end.page != start.page {
            Some("Page")
        } else if end.fonts != start.fonts || end.font_glue != start.font_glue {
            Some("Fonts")
        } else if end.marks != start.marks {
            Some("Marks")
        } else if end.obj_ptr != start.obj_ptr {
            Some("Objects")
        } else if end.interaction != start.interaction || end.arm != start.arm {
            Some("State")
        } else if end.outputs != start.outputs {
            Some("Output")
        } else if end.lines != start.lines {
            Some("LineShift")
        } else {
            None
        };
        if let Some(w) = why {
            return self.bm_abort(w);
        }
        let gboxes = self.bm_with_rec(|r| r.gboxes.clone()).unwrap_or_default();
        if gboxes.iter().any(|&n| self.bm_box_value(n) != 0) {
            return self.bm_abort("GlobalBox");
        }
        self.bm_rec_on = false;
        let mut rec = ST.with(|s| s.borrow_mut().rec.take()).unwrap();
        for n in gboxes {
            if n < 256 {
                rec.ops.push(Op::Def {
                    p: BOX_BASE + n,
                    t: BOX_REF,
                    v: Val::Plain(0),
                    kind: 2,
                });
            } else {
                rec.ops.push(Op::Sa {
                    t: BOX_VAL,
                    n,
                    v: Val::Plain(0),
                    kind: 2,
                });
            }
        }
        // a draw call's output, with its holes
        let mut frag = None;
        if end.tail != start.tail || !rec.holes.is_empty() {
            let holes: Vec<i32> = rec.holes.iter().map(|h| h.1).collect();
            if end.tail == start.tail {
                return self.bm_abort_taken(rec, "Hole");
            }
            let first = self.bm_link(start.tail);
            match self.frag_take(first, 0, &holes) {
                Ok(f) if self.cur_list.tail_field == end.tail => frag = Some(f),
                Ok(_) => return self.bm_abort_taken(rec, "Tail"),
                Err(w) => return self.bm_abort_taken(rec, w),
            }
        }
        let aux = self.cur_list.aux_field.0;
        let last_badness = self.last_badness;
        if let Some(i) = rec.verify {
            let (cs, scanner) = (rec.cs, rec.scanner);
            self.bm_verified(rec, i, last_badness, &frag);
            return self.bm_verify_full(cs, i, scanner);
        }
        self.bm_commit(rec, last_badness, frag, aux);
    }

    /// Abandon a recording already taken out of the state (its end check).
    fn bm_abort_taken(&mut self, rec: Rec, why: &str) {
        ST.with(|s| s.borrow_mut().rec = Some(rec));
        self.bm_rec_on = true;
        self.bm_abort(why);
    }

    fn bm_commit(
        &mut self,
        rec: Rec,
        last_badness: i32,
        frag: Option<crate::boxfrag::Frag>,
        aux: u64,
    ) {
        let line = self.line;
        let bytes = 4 * (rec.k1.len() + rec.k2.len() + rec.k4.len())
            + rec
                .k3
                .iter()
                .map(|(_, m)| match m {
                    Meaning::Macro(_, v) => 16 + 4 * v.len(),
                    _ => 16,
                })
                .sum::<usize>()
            + rec
                .ops
                .iter()
                .map(|o| match o {
                    Op::Def {
                        v: Val::FreshList(l),
                        ..
                    }
                    | Op::Sa {
                        v: Val::FreshList(l),
                        ..
                    } => 32 + 4 * l.len(),
                    Op::Def {
                        v: Val::FreshShape(l),
                        ..
                    } => 32 + 4 * l.len(),
                    _ => 32,
                })
                .sum::<usize>();
        let name = if debug() {
            Some(self.cs_name_string(rec.cs))
        } else {
            None
        };
        ST.with(|s| {
            let mut s = s.borrow_mut();
            s.clock += 1;
            let now = s.clock;
            s.aborts.remove(&rec.cs);
            let entry = Entry {
                hash: key_hash(&rec.k1, &rec.ctx, &rec.k2, &rec.k4),
                k1: rec.k1,
                ctx: rec.ctx,
                k2: rec.k2,
                k4: rec.k4,
                k3: rec.k3,
                absent: rec.absent,
                kr: rec.kr,
                kb: rec.kb,
                holes: rec.holes.iter().map(|h| (h.0, h.2.clone())).collect(),
                frag,
                aux,
                voids: rec.voids,
                line: rec.line_read.then_some(line),
                ops: rec.ops,
                last_badness,
                last_used: now,
                hits: 0,
                bytes,
            };
            if let Some(n) = &name {
                eprintln!(
                    "boxmemo: recorded \\{n}: {} ops, {} meanings, {} bytes",
                    entry.ops.len(),
                    entry.k3.len(),
                    bytes
                );
            }
            s.bytes += bytes;
            let v = s.store.entry(rec.cs).or_default();
            if v.len() >= MAX_VARIANTS {
                let (i, _) = v
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, e)| e.last_used)
                    .unwrap();
                let old = v.remove(i);
                s.bytes = s.bytes.saturating_sub(old.bytes);
            }
            s.store.entry(rec.cs).or_default().push(entry);
            s.stats.committed += 1;
            // over the budget: drop the least recently used entries
            while s.bytes > MAX_BYTES {
                let victim = s
                    .store
                    .iter()
                    .flat_map(|(&cs, v)| {
                        v.iter().enumerate().map(move |(i, e)| (e.last_used, cs, i))
                    })
                    .min();
                let Some((_, cs, i)) = victim else { break };
                let e = s.store.get_mut(&cs).unwrap().remove(i);
                s.bytes = s.bytes.saturating_sub(e.bytes);
            }
        });
        write_stats(false);
    }

    /// The normal path of a call the guard admitted, re-recorded: compare.
    fn bm_verified(
        &mut self,
        rec: Rec,
        i: usize,
        last_badness: i32,
        frag: &Option<crate::boxfrag::Frag>,
    ) {
        let name = self.cs_name_string(rec.cs);
        let fail = with_config(|c| c.verify_fail);
        ST.with(|s| {
            let mut s = s.borrow_mut();
            s.stats.verified += 1;
            let Some(e) = s.store.get(&rec.cs).and_then(|v| v.get(i)) else {
                return;
            };
            let mut diffs = vec![];
            if e.ops.len() != rec.ops.len() {
                diffs.push(format!(
                    "operations: recorded {} normal path {}",
                    e.ops.len(),
                    rec.ops.len()
                ));
            }
            for (j, (a, b)) in e.ops.iter().zip(&rec.ops).enumerate() {
                if a != b {
                    diffs.push(format!("op {j}: recorded {a:?} normal path {b:?}"));
                    break;
                }
            }
            if e.frag != *frag {
                diffs.push("the output nodes differ".to_string());
            }
            if e.last_badness != last_badness {
                diffs.push(format!(
                    "last_badness: recorded {} normal path {last_badness}",
                    e.last_badness
                ));
            }
            if !diffs.is_empty() {
                s.stats.verify_differences += 1;
                eprintln!("boxmemo verify: \\{name}: {diffs:?}");
                if s.stats.verify_details.len() < 20 {
                    s.stats
                        .verify_details
                        .push(format!("\\{name}: {:?}", &diffs[..diffs.len().min(4)]));
                }
                if fail {
                    eprintln!("boxmemo verify: FLASHTEX_BOXMEMO_VERIFY_FAIL is set: exit 3");
                }
            }
        });
        write_stats(false);
        fail_now();
    }

    fn bm_null_hold_head(&mut self) {
        let h = k::hold_head as usize;
        if self.mem[h].hh().rh() != 0 {
            self.mem[h].set_hh_rh(0);
        }
    }

    /// Debugging a full-state difference: where the lists the message names
    /// hang (the static heads, the box registers, the semantic nest).
    fn bm_locate_debug(&self, msg: &str) {
        let ptrs: Vec<i32> = msg
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|t| t.parse::<i32>().ok())
            .filter(|&p| p > 1000)
            .collect();
        let heads = [
            ("contrib_head", k::contrib_head),
            ("page_head", k::page_head),
            ("temp_head", k::temp_head),
            ("hold_head", k::hold_head),
            ("adjust_head", k::adjust_head),
            ("align_head", k::align_head),
            ("backup_head", k::backup_head),
        ];
        let mut out = vec![];
        for (n, h) in heads {
            out.push(format!("{n}->{}", self.bm_link(h)));
        }
        for &p in &ptrs {
            for b in 0..256 {
                if self.bm_eq(BOX_BASE + b).hh().rh() == p {
                    out.push(format!("{p} = box{b}"));
                }
            }
            for j in 0..=self.nest_ptr {
                let l = if j == self.nest_ptr {
                    self.cur_list
                } else {
                    self.nest[j as usize]
                };
                if self.bm_link(l.head_field) == p || l.head_field == p {
                    out.push(format!("{p} = nest[{j}] list"));
                }
            }
        }
        eprintln!("boxmemo verify debug: {out:?}");
    }

    /// The full verifier (BOX-MEMO.md §7): the normal path is done; take
    /// its state N, restore the admission point C, replay, and compare the
    /// replay's state with N by the convergence test's comparison. The run
    /// goes on from the replay.
    fn bm_verify_full(&mut self, cs: i32, i: usize, scanner: i32) {
        let Some(fv) = FULL.with(|f| f.borrow_mut().take()) else {
            return;
        };
        let name = self.cs_name_string(cs);
        pop_used_up(self);
        let scratch = Scratch::take(self);
        // `link(hold_head)` is written before every read within a command
        // (pdftex.web: the alignment preamble scan §779-§784 and `init_span`
        // §789 set it first; `fin_col` §808; `reconstitute` §905 and the
        // hyphenation loop §913-§918 set it to null first), so between two
        // commands it is a dangling pointer to cells either path may have
        // reused: made null on both paths (the convergence test's
        // `temp_head` and `backup_head` rule, iso.rs, for one more head)
        self.bm_null_hold_head();
        // The page builder's best break so far: a call never moves it (the
        // recording's end check compares it with its start), so both paths
        // hold the same pointer; but between the page builder's runs it may
        // point at a node `iso.rs` does not reach from its roots (it is
        // compared only where pages ship, where it is dead). Left out of
        // the comparison, and put back.
        let bpb = self.best_page_break;
        self.best_page_break = 0;
        let result = (|| -> Result<usize, String> {
            let n = self.checkpoint()?;
            self.restore(fv.ck)?;
            // `macro_call`'s exit, which the normal path ran
            self.bm_replay(cs, i);
            self.scanner_status = scanner;
            self.warning_index = fv.warn;
            pop_used_up(self);
            scratch.put(self);
            self.bm_null_hold_head();
            self.best_page_break = 0;
            let r = crate::incr::same_state(self, n);
            if r.is_err() && debug() {
                self.bm_locate_debug(r.as_ref().err().unwrap());
            }
            self.abandon_pending();
            r
        })();
        self.best_page_break = bpb;
        let ck = fv.ck;
        self.retain_checkpoints(&|id| id != ck);
        let fail = with_config(|c| c.verify_fail);
        ST.with(|s| {
            let mut s = s.borrow_mut();
            s.stats.full_verified += 1;
            if let Err(e) = &result {
                s.stats.full_differences += 1;
                s.stats.verify_differences += 1;
                eprintln!("boxmemo verify (full state): \\{name}: {e}");
                if s.stats.verify_details.len() < 20 {
                    s.stats
                        .verify_details
                        .push(format!("full state \\{name}: {e}"));
                }
                if fail {
                    eprintln!("boxmemo verify: FLASHTEX_BOXMEMO_VERIFY_FAIL is set: exit 3");
                }
            }
        });
        write_stats(false);
        fail_now();
    }

    // -- replay --------------------------------------------------------------

    fn bm_make_list(&mut self, toks: &[i32]) -> i32 {
        let head = self.get_avail();
        self.mem[head as usize].set_hh_lh(0);
        let mut q = head;
        for &t in toks {
            let n = self.get_avail();
            self.mem[n as usize].set_hh_lh(t);
            self.mem[q as usize].set_hh_rh(n);
            q = n;
        }
        self.mem[q as usize].set_hh_rh(0);
        head
    }

    /// The `equiv` a recorded value stands for now.
    fn bm_value(&mut self, v: &Val) -> (Option<i32>, i32) {
        match v {
            Val::Word(w) | Val::Plain(w) => (None, *w),
            Val::StaticGlue(e) => {
                let r = self.bm_link(*e) + 1;
                self.mem[*e as usize].set_hh_rh(r);
                (None, *e)
            }
            Val::FreshGlue(f) => {
                let q = self.get_node(GLUE_SPEC_SIZE);
                self.mem[q as usize].set_hh_rh(0);
                self.mem[q as usize].set_hh_b0(f[3]);
                self.mem[q as usize].set_hh_b1(f[4]);
                self.mem[(q + 1) as usize].set_int(f[0]);
                self.mem[(q + 2) as usize].set_int(f[1]);
                self.mem[(q + 3) as usize].set_int(f[2]);
                (None, q)
            }
            Val::FreshList(l) => (None, self.bm_make_list(l)),
            Val::GlueFrom(a, b) => {
                let e = if *a > 0 {
                    self.bm_eq(*a).hh().rh()
                } else {
                    self.find_sa_element(-*a, *b, false);
                    let q = self.cur_ptr;
                    if q == 0 {
                        k::zero_glue
                    } else {
                        self.bm_link(q + 1)
                    }
                };
                let r = self.bm_link(e) + 1;
                self.mem[e as usize].set_hh_rh(r);
                (None, e)
            }
            Val::LetCs(src) => {
                let w = self.bm_eq(*src).hh();
                let (t, e) = (w.b0(), w.rh());
                if (CALL..=LONG_OUTER_CALL).contains(&t) && e != 0 {
                    let r = self.bm_info(e) + 1;
                    self.mem[e as usize].set_hh_lh(r);
                } else if (t == REGISTER || t == TOKS_REGISTER) && e > LO_MEM_STAT_MAX {
                    // sa_ref(e) = info(e+1)
                    let r = self.bm_info(e + 1) + 1;
                    self.mem[(e + 1) as usize].set_hh_lh(r);
                }
                (Some(t), e)
            }
            Val::FreshShape(words) => {
                let q = self.get_node(words.len() as i32);
                self.mem[q as usize].set_hh_lh(words[0]);
                for (i, &w) in words.iter().enumerate().skip(1) {
                    self.mem[q as usize + i].set_int(w);
                }
                (None, q)
            }
        }
    }

    fn bm_replay(&mut self, cs: i32, i: usize) {
        let t0 = std::time::Instant::now();
        // Take the entry's operations (an entry is never replayed inside
        // itself: a replay runs no recorded hook).
        let taken = ST.with(|s| {
            let mut s = s.borrow_mut();
            s.clock += 1;
            let now = s.clock;
            let e = s.store.get_mut(&cs)?.get_mut(i)?;
            e.last_used = now;
            e.hits += 1;
            Some((
                std::mem::take(&mut e.ops),
                e.last_badness,
                e.k3.iter().map(|(p, _)| *p).collect::<Vec<i32>>(),
            ))
        });
        let Some((ops, last_badness, k3)) = taken else {
            return;
        };
        // the body, used up: as `get_next` pops it
        self.end_token_list();
        // the L5 read-set (DESIGN.md §5.5), as D9's `intr_report_reads`
        if self.rs_on {
            let mut locs = k3;
            locs.push(cs);
            for o in &ops {
                if let Op::Def { p, v, .. } = o {
                    locs.push(*p);
                    if let Val::LetCs(s) = v {
                        locs.push(*s);
                    }
                }
            }
            for p in locs {
                if Self::bm_is_cs(p) && !self.rs_seen[p as usize] {
                    self.flashtex_cs_read(p);
                }
            }
        }
        for o in &ops {
            match o {
                Op::Begin(c) => self.new_save_level(*c),
                Op::End => self.unsave(),
                Op::Def { p, t, v, kind } => {
                    let (lt, e) = self.bm_value(v);
                    let t = lt.unwrap_or(*t);
                    match kind {
                        0 => self.eq_define(*p, t, e),
                        1 => self.eq_word_define(*p, e),
                        2 => self.geq_define(*p, t, e),
                        _ => self.geq_word_define(*p, e),
                    }
                }
                Op::Sa { t, n, v, kind } => {
                    self.find_sa_element(*t, *n, true);
                    let q = self.cur_ptr;
                    let (_, e) = self.bm_value(v);
                    match kind {
                        0 => self.sa_def(q, e),
                        1 => self.sa_w_def(q, e),
                        2 => self.gsa_def(q, e),
                        _ => self.gsa_w_def(q, e),
                    }
                }
            }
        }
        self.last_badness = last_badness;
        // a draw call: the boxes it moved out of their registers, then its
        // output appended to the current list
        let draw = ST.with(|s| {
            let s = s.borrow();
            let e = s.store.get(&cs)?.get(i)?;
            Some((e.holes.clone(), e.frag.clone(), e.aux))
        });
        if let Some((holes, Some(frag), aux)) = draw {
            let mut nodes = vec![];
            for (n, _) in &holes {
                nodes.push(self.bm_box_value(*n));
                self.bm_void_box(*n);
            }
            let save = self.scanner_status;
            self.scanner_status = 0;
            let (first, last) = self.frag_make(&frag, &nodes);
            self.scanner_status = save;
            if first != 0 {
                let t = self.cur_list.tail_field;
                self.mem[t as usize].set_hh_rh(first);
                self.cur_list.tail_field = last;
            }
            self.cur_list.aux_field = crate::generated::types::memory_word(aux);
        }
        let n = ops.len() as u64;
        ST.with(|s| {
            let mut s = s.borrow_mut();
            if let Some(e) = s.store.get_mut(&cs).and_then(|v| v.get_mut(i)) {
                e.ops = ops;
            }
            s.stats.hits += 1;
            s.stats.replayed_ops += n;
        });
        if debug() {
            eprintln!(
                "boxmemo: replayed \\{} ({n} ops) in {:?}",
                self.cs_name_string(cs),
                t0.elapsed()
            );
        }
        write_stats(false);
    }

    /// The end of the run: the statistics.
    pub fn flashtex_bm_finish(&mut self) {
        write_stats(true);
    }
}
