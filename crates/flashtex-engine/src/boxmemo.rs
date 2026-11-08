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
const MAX_VARIANTS: usize = 1024;
/// Bytes of recordings kept in all, roughly.
const MAX_BYTES: usize = 32 << 20;
/// The most one span's entry may take of it (§17.5).
const MAX_SPAN_BYTES: usize = MAX_BYTES / 4;
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
    /// The environments offered as spans (§17): `FLASHTEX_BOXMEMO_SPANS`,
    /// none by default.
    spans: Vec<Vec<u8>>,
    stats_out: Option<String>,
    debug: bool,
    verify_fail: bool,
    /// `FLASHTEX_BOXMEMO_FULL_VERIFY_HOST`: measurement only (§17.12), the
    /// whole-state verifier in the host too.
    full_in_host: bool,
}

/// The macros offered by default (BOX-MEMO.md §2, §11): framed.sty's frame
/// measurement and ntheorem's frame commands; xcolor's `\colorlet` body,
/// which beamer's `\usebeamercolor` calls hundreds of times a slide; pgf's
/// length and point computations; LaTeX's page head and foot (beamer's
/// headline, sidebars, navigation symbols and footline, as draw calls);
/// beamer's head and foot measurement and its frame title.
pub const DEFAULT_NAMES: &[&str] = &[
    "fb@sizeofframe",
    "*@framecommand",
    "XC@col@rlet",
    "pgfmathsetlength",
    "pgf@process",
    "pgfpoint",
    "@oddhead",
    "@oddfoot",
    "beamer@typesetheadorfoot",
    "beamer@@tmpl@frametitle",
    // (lane T7-LAST12) pgf's shading functions, parsed again at each
    // declaration: Madrid's block shadow declares four shadings at every
    // block (its `\pdfxform`s run for real, after the parse); beamer's
    // colour templates; a path's use (its PDF literals are a draw call's
    // fragment)
    "pgf@parsefunc",
    "beamer@thc@docolor",
    "beamer@thc@usecolor",
    "pgfusepath",
    // hyperref's per-`\pdfstringdef` hook, which siunitx fills (about 25 M
    // instructions a call): recorded anywhere and taken from the intrinsics
    // (`RECORD_ANYWHERE`, §12.2)
    "pdfstringdefPreHook",
    // (lane FANCYHDR) fancyhdr's head and foot: LaTeX's `\@thehead` is
    // `\let` to `\@oddhead` or `\@evenhead`, which fancyhdr makes call
    // these (a draw call each, whose `\normalsize` leaves `\strutbox` set
    // at the call's level, §16)
    "f@nch@head",
    "f@nch@foot",
    // cleveref's counter prefix, made again at every `\refstepcounter` and
    // `\label` (the reset lists walked by `\cref@isinresetlist`): only
    // assignments, 1-3 M instructions a call
    "cref@constructprefix",
];

/// Whether BOX-MEMO is on when `FLASHTEX_BOXMEMO` does not say: in the
/// engine host (`set_default_on`, where it records only in an edit's
/// window), not on the command line.
static DEFAULT_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The engine host: BOX-MEMO is on unless `FLASHTEX_BOXMEMO=off`. Call
/// before the first engine starts.
pub fn set_default_on() {
    DEFAULT_ON.store(true, std::sync::atomic::Ordering::Relaxed);
}

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
    /// Control sequences a replay entered again (`Op::NewCs`, §12.3).
    pub entered: u64,
    /// K3's macro bodies compared token by token, the comparisons §12.4's
    /// entries answered, and the times the entries were emptied.
    pub meanings_walked: u64,
    pub meanings_seen: u64,
    pub seen_cleared: u64,
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
    /// A span's compressed log (§17.5): the token list the log's operation
    /// `.0` made, which a `\let` may have shared; a replay makes it at the
    /// first operation with this `.0` and shares it at the others, so the
    /// lists are shared as on the normal path (their reference counts).
    Shared(u32, Vec<i32>),
    /// The meaning of a control sequence as it is when the assignment is
    /// made (`\let`, `\futurelet`).
    LetCs(i32),
    /// A `\parshape` specification made for the assignment, by its words.
    FreshShape(Vec<i32>),
    /// The glue specification a skip register holds when the assignment is
    /// made (`\skip1=\skip2` shares it): an `eqtb` location, or a sparse
    /// register (type, number) as `(-type, number)`, or font `f`'s
    /// interword glue as `(0, f)` (beamer's `\beamer@lastskip=\lastskip`
    /// after an interword space).
    GlueFrom(i32, i32),
    /// A box a local `\setbox` at the call's own level put in a register
    /// (fancyhdr's `\normalsize`: `\size@update` sets `\strutbox`), by value:
    /// the box the register holds when the call ends, which nothing outside
    /// the call made or shares (§16). A replay makes it fresh, as the
    /// normal path's `box_end` made it.
    Box(std::rc::Rc<crate::boxfrag::Frag>),
    /// While recording, such a box by its node; `bm_finish` takes it by
    /// value ([`Val::Box`]) or abandons the recording. Never in an entry.
    BoxNode(i32),
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
    /// A control sequence entered (`id_lookup` making `name`, which a
    /// `\csname` looked up for the first time) at `p` (§12.3): a replay
    /// enters it again through `id_lookup`, where an offer has worked out it
    /// lands at `p` (`bm_new_cs_placeable`).
    NewCs {
        p: i32,
        name: Vec<i32>,
    },
    /// A span's `\ht`/`\wd`/`\dp` assignment to box register `n`, which it
    /// did not make (amsmath's `\resetMathstrut@`): word `c` of the box
    /// node becomes `v`, as `alter_box_dimen` does; no group takes it back
    /// (§17.6).
    BoxDim {
        n: i32,
        c: i32,
        v: i32,
    },
}

thread_local! {
    /// The macro bodies the kept keys read (K3), each once: a beamer slide's
    /// `\colorlet` calls read the same few hundred xcolor and beamer macros,
    /// so the variants share them. A body no entry holds any more is
    /// dropped when the pool is pruned (`insert_entry`).
    static TOKENS: RefCell<HashSet<std::rc::Rc<[i32]>>> = RefCell::new(HashSet::new());
    /// The pool's size in bytes, roughly (part of `MAX_BYTES`).
    static TOKEN_BYTES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Is the recording's span's F a token list (§17.10)?
    static SPAN_TOKS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// During a replay, the lists its `Val::Shared` operations made, by
    /// the recorded operation that made each.
    static SHARED: RefCell<HashMap<u32, i32>> = RefCell::new(HashMap::new());
}

/// `v`, shared with the pool's equal body if there is one.
fn intern_tokens(v: Vec<i32>) -> std::rc::Rc<[i32]> {
    TOKENS.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(r) = t.get(&v[..]) {
            return r.clone();
        }
        let r: std::rc::Rc<[i32]> = v.into();
        TOKEN_BYTES.with(|b| b.set(b.get() + 32 + 4 * r.len()));
        t.insert(r.clone());
        r
    })
}

/// Drop the pooled bodies no entry holds.
fn prune_tokens() {
    TOKENS.with(|t| {
        let mut t = t.borrow_mut();
        t.retain(|r| std::rc::Rc::strong_count(r) > 1);
        let n: usize = t.iter().map(|r| 32 + 4 * r.len()).sum();
        TOKEN_BYTES.with(|b| b.set(n));
    });
}

/// Roughly what a log costs in the store's budget.
fn ops_bytes(ops: &[Op]) -> usize {
    ops.iter()
        .map(|o| match o {
            Op::Def {
                v: Val::FreshList(l) | Val::Shared(_, l),
                ..
            }
            | Op::Sa {
                v: Val::FreshList(l) | Val::Shared(_, l),
                ..
            } => 32 + 4 * l.len(),
            Op::Def {
                v: Val::FreshShape(l),
                ..
            } => 32 + 4 * l.len(),
            // (and once more in `Entry::new_cs`)
            Op::NewCs { name, .. } => 2 * (32 + 4 * name.len()),
            Op::Def { v: Val::Box(f), .. } => 32 + f.bytes(),
            _ => 32,
        })
        .sum()
}

/// Keep `entry` among `cs`'s variants: the least recently used is dropped
/// past `MAX_VARIANTS`, and the least recently used of all past
/// `MAX_BYTES`.
fn insert_entry(s: &mut State, cs: i32, entry: Entry) {
    s.bytes += entry.bytes;
    s.stale.insert(cs);
    let v = s.store.entry(cs).or_default();
    if v.len() >= MAX_VARIANTS {
        let (i, _) = v
            .iter()
            .enumerate()
            .min_by_key(|(_, e)| e.last_used)
            .unwrap();
        let old = v.remove(i);
        s.bytes = s.bytes.saturating_sub(old.bytes);
    }
    s.store.entry(cs).or_default().push(entry);
    // over the budget: drop the least recently used entries (the pooled
    // bodies first pruned of the ones only dropped entries held)
    if s.bytes + TOKEN_BYTES.with(|b| b.get()) > MAX_BYTES {
        prune_tokens();
    }
    while s.bytes + TOKEN_BYTES.with(|b| b.get()) > MAX_BYTES {
        let victim = s
            .store
            .iter()
            .flat_map(|(&cs, v)| v.iter().enumerate().map(move |(i, e)| (e.last_used, cs, i)))
            .min();
        let Some((_, cs, i)) = victim else { break };
        s.stale.insert(cs);
        let e = s.store.get_mut(&cs).unwrap().remove(i);
        s.bytes = s.bytes.saturating_sub(e.bytes);
        drop(e);
        prune_tokens();
    }
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
    /// The control sequence a `\csname` (`id_lookup`) found by this name:
    /// the name must still lead to this location. A name's place in the
    /// hash depends on the order names were made in (a colliding name takes
    /// the next free slot, tex.web §260), which two runs can differ in --
    /// the same `\csname` then names another location, which the meaning
    /// at the recorded location cannot show (sweep C, framed-tikz-theorems,
    /// verify mode).
    Name(Vec<i32>),
    /// A macro's type and its tokens, shared by every variant that read
    /// the same body (`intern_tokens`).
    Macro(i32, std::rc::Rc<[i32]>),
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
    /// The current list's `aux` (`\spacefactor` and `clang`, or
    /// `\prevdepth`) at the call: a draw call's first spaces and lines
    /// depend on it.
    aux: u64,
}

/// How K2 keeps the value at one `eqtb` location.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum K2Kind {
    Glue,
    Shape,
    List,
    Equiv,
    Int,
}

/// K2's locations, `glue_base..=eqtb_size` in order, as runs `[a, b)` of
/// one kind ([`Globals::bm_k2_kind`]), the locations it leaves out
/// skipped.
fn k2_plan() -> &'static [(i32, i32, K2Kind)] {
    static PLAN: std::sync::OnceLock<Vec<(i32, i32, K2Kind)>> = std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        let mut v: Vec<(i32, i32, K2Kind)> = vec![];
        for p in GLUE_BASE..=EQTB_SIZE {
            let Some(k) = Globals::bm_k2_kind(p) else {
                continue;
            };
            match v.last_mut() {
                Some((_, b, kk)) if *b == p && *kk == k => *b = p + 1,
                _ => v.push((p, p + 1, k)),
            }
        }
        v
    })
}

/// Whether each word's low half (`hh().rh()`, `int()`) is the value at
/// its place in `want`: the comparison of K2's runs of equivalents and
/// integers, a few thousand words at every offer (beamer-30's slide: 6 %
/// of the edited page's typesetting compared one by one), made 16 words at
/// a time with no exit inside a block (lane T7-LETTERSTART).
fn low_halves_eq(words: &[crate::generated::types::memory_word], want: &[i32]) -> bool {
    if words.len() != want.len() {
        return false;
    }
    let (wb, wt) = words.as_chunks::<16>();
    let (vb, vt) = want.as_chunks::<16>();
    for (a, b) in wb.iter().zip(vb) {
        let mut d = 0u32;
        for k in 0..16 {
            d |= (a[k].0 as u32) ^ (b[k] as u32);
        }
        if d != 0 {
            return false;
        }
    }
    wt.iter().zip(vt).all(|(x, &y)| x.0 as u32 == y as u32)
}

/// [`Globals::bm_k2_hash`] of a K2 kept as values.
fn k2_hash_of(k2: &[i32]) -> u64 {
    let mut h: u64 = 0x243f_6a88_85a3_08d3;
    for &x in k2 {
        h = (h ^ x as u32 as u64)
            .wrapping_mul(0x0100_0000_01b3)
            .rotate_left(23);
    }
    h
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
        ctx.aux as i32,
        (ctx.aux >> 32) as i32,
        k1.len() as i32,
        k2.len() as i32,
        k4.len() as i32,
    ] {
        mix(x);
    }
    h
}

struct Entry {
    /// A recording of this key was abandoned (`bm_abort`): the call reads
    /// the same values up to the same refusal, so a call that matches it is
    /// expanded for real without recording again, and the calls inside it
    /// are offered (a recording would refuse them). Its operations are
    /// empty.
    doomed: bool,
    hash: u64,
    /// `bm_k2_hash` of K2.
    k2hash: u64,
    /// K1: the macro's body and its arguments.
    k1: Vec<i32>,
    ctx: Context,
    /// K2 and K4, as one stream of values. K2 is shared by the variants
    /// of a macro recorded in the same state (a typed letter's two states,
    /// recorded at each call), so it is kept once and compared once per
    /// offer.
    k2: std::rc::Rc<[i32]>,
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
    /// `cur_level` and `cur_group` at the call when the recording read
    /// `\currentgrouplevel` or `\currentgrouptype`.
    group: Option<(i32, i32)>,
    /// Foreign box registers read for their dimensions or kind, and those
    /// moved into the output (the holes, in order): by `box_key`.
    kb: Vec<(i32, Vec<i32>)>,
    /// Foreign box registers a span copied, by contents (§17.6).
    kc: Vec<(i32, crate::boxfrag::Frag, Positions)>,
    holes: Vec<(i32, Vec<i32>)>,
    /// A draw call's output: the nodes it appended to the current list,
    /// and the list's `aux` after them.
    frag: Option<crate::boxfrag::Frag>,
    aux: u64,
    /// A last `\\ignorespaces` (its token), run after the replay.
    trailer: Option<i32>,
    ops: Vec<Op>,
    /// What a replay makes: `ops` without the groups that leave nothing
    /// behind or are known by their effect ([`compress_groups`]).
    fast: Vec<Op>,
    /// The control sequences the call entered (`Op::NewCs`: name,
    /// location), in order: an offer checks they would land where they
    /// did (`bm_new_cs_placeable`).
    new_cs: Vec<(Vec<i32>, i32)>,
    /// Existing forms and images the call referred to (`\pdfrefxform`,
    /// `\pdfrefximage`): each must still be one of its type with the same
    /// dimensions (part of the key).
    obj_refs: Vec<ObjRef>,
    /// Objects the call made (a `\pdfstartlink`'s), made again by a replay.
    new_objs: usize,
    /// Fonts whose interword glue the call shares (an assignment's
    /// `GlueFrom(0, f)`, a fragment's `Spec::Font`), with its fields: each
    /// must be there with the same fields (part of the key).
    font_glues: Vec<(i32, [i32; 5])>,
    /// `\pdflastlink` after the call, when the call set it: the `i`-th of
    /// its new objects.
    last_link: Option<usize>,
    /// The control sequences a replay reports to the L5 read-set: the
    /// macro, K3's, and every one `ops` assigns or `\let`s from.
    rs_locs: Vec<i32>,
    /// `last_badness` after the call, when the call packed a box: a call that
    /// packs none leaves it as it was, which a replay must then do too (it
    /// is the state's, not the call's).
    last_badness: Option<i32>,
    /// A span's file and what it left (§17).
    span: Option<Box<SpanEntry>>,
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
    /// `obj_ptr` and `sys_obj_ptr` (pdftex.web: an object stream's number
    /// is taken from `sys_obj_ptr` with `obj_ptr` left behind it, "not
    /// accessible to user", until the next object makes them equal again).
    obj_ptr: (i32, i32),
    interaction: i32,
    arm: (i32, i32),
    outputs: (i32, i32, u64, usize, usize),
    /// The line-shift journal (`crate::lineshift::journal_mark`).
    lines: (u64, Vec<(i32, i32)>, bool),
}

/// An existing object a recording referred to: its type, its number, and
/// its width, height and depth (`obj_xform_width` & co.).
#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjRef {
    t: i32,
    n: i32,
    dims: [i32; 3],
}

/// What a span (§17) read of its file: the lines `input_ln` read, each raw
/// with the stream's offset after it, counted from the offset at the call.
/// Part of the key: the file must hold the same lines there now.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SpanFile {
    /// The file level's file, as the run opened it.
    path: String,
    lines: Vec<(Vec<u8>, u64)>,
}

/// Where a span left its file level (§17), counted from its `start`: when
/// the span read a line, the line in the buffer (`buffer[start..=limit]`;
/// else empty: the buffer is as it was); `loc`, `limit`, the state; `first`
/// and, when the span read a line, `last` and the buffer's high mark;
/// `line`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SpanEnd {
    buf: Vec<i32>,
    loc: i32,
    limit: i32,
    state: i32,
    first: i32,
    last: Option<i32>,
    max_buf: Option<i32>,
    /// `max_buf_stack` at the call: with `max_buf` `None` the span's own
    /// high mark is at most this, so a replay needs `max_buf_stack` at
    /// least this (a restored checkpoint can leave it lower); comparisons
    /// compare the high mark each leaves instead ([`SpanEnd::same`]).
    max_buf0: i32,
    line: i32,
    /// The line-shift journal's entries the span made (`\inputlineno`
    /// reads of its file level's lines, tikz-cd's `\tikzcd@lineno`), each
    /// as its line's distance from the span's first.
    journal: Vec<i32>,
}

impl SpanEnd {
    /// The buffer's high mark a replay of `self` leaves after a call at
    /// which it was `at` (both counted from the line's start).
    fn high_mark(&self, at: i32) -> i32 {
        self.max_buf.map_or(at, |m| m.max(at))
    }

    /// Does a replay of the recorded `self` leave the state the normal
    /// path's `b` (of a call at the same place) left? Equal but for the
    /// high mark, where each leaves the same one from `b`'s call.
    fn same(&self, b: &SpanEnd) -> bool {
        let strip = |e: &SpanEnd| SpanEnd {
            max_buf: None,
            max_buf0: 0,
            ..e.clone()
        };
        strip(self) == strip(b)
            && (self.max_buf.is_some() || self.max_buf0 <= b.max_buf0)
            && self.high_mark(b.max_buf0) == b.high_mark(b.max_buf0)
    }
}

/// A span begun in vertical mode inside a box (§17.3): it started a
/// paragraph (`new_graf`), which is still open at its end. The nodes it
/// appended to the box's list before (the paragraph skip), the paragraph's
/// level (`mode_line`, `prev_graf`, `aux`) and its list. (On the main
/// vertical list, `outer_h`: the skips' natural widths, each at least 0,
/// for [`Globals::bm_page_quiet`].)
#[derive(Clone, Debug, PartialEq, Eq)]
struct Graf {
    outer: crate::boxfrag::Frag,
    inner: crate::boxfrag::Frag,
    ml: i32,
    pg: i32,
    aux: u64,
    outer_h: i64,
}

/// The source positions of a span's output nodes (§17.4), in the order
/// [`Globals::bm_node_walk`] visits them: the outer fragment's (or the draw
/// fragment's), then the paragraph's list head and its fragment's.
type Positions = Vec<crate::displaylist::SidePos>;

/// What a span keeps besides a call's entry (§17).
#[derive(Clone, Debug)]
struct SpanEntry {
    file: SpanFile,
    end: SpanEnd,
    graf: Option<Graf>,
    pos: Positions,
    /// The foreign box registers whose `\copy` its output holds
    /// (`Node::Copy`, in order, §17.6).
    copies: Vec<i32>,
    /// A token list's F (§17.10): the tokens the span read from it.
    toks: Option<Vec<i32>>,
}

/// A paragraph skip a span's `new_graf` appended to the main vertical
/// list (§17.3), as it was then: the node, its glue specification and that
/// specification's words, and its source position. The page builder took
/// it at once (and may have thrown it away), so it is kept from here.
#[derive(Clone)]
struct GlueRec {
    p: i32,
    words: [u64; 2],
    spec: i32,
    spec_words: [u64; 4],
    pos: crate::displaylist::SidePos,
}

/// What a span's recording noted of the paragraphs `new_graf` began on the
/// main vertical list (§17.3). LaTeX begins every paragraph twice (ltpara:
/// `\everypar` ends the paragraph `new_graf` began, empty, and begins it
/// again with `\noindent`), and each time the page builder runs: the
/// paragraph skips, in order, the outer list's tail after the page builder's
/// last run, and the page's state then with the parameters it read (each
/// run's the same).
struct GrafNote {
    glues: Vec<GlueRec>,
    tail: i32,
    page: Option<(Vec<i64>, [i64; 7])>,
}

/// A span's recording: the file level's input-stack index and file.
struct SpanRec {
    f: i32,
    index: i32,
    file: SpanFile,
    /// A token list's F (§17.10): its list (`start`) and `loc` at the call.
    toks: Option<(i32, i32)>,
    /// The stream's offset at the call, and `max_buf_stack`.
    at: u64,
    max_buf: i32,
    graf: Option<GrafNote>,
    /// The current list's `eTeX_aux` at the call.
    etex_aux: i32,
    /// `line` at the call, and the line-shift journal's count of entries.
    line0: i32,
    journal: u64,
}

struct Rec {
    cs: i32,
    /// The macro called: `cs` itself, or for a span `\begin` (`cs` is then
    /// the environment's control sequence, the store's key).
    call: i32,
    span: Option<SpanRec>,
    obj_refs: Vec<ObjRef>,
    new_objs: Vec<i32>,
    last_link: i32,
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
    group_read: bool,
    /// `cur_level` and `cur_group` at the call.
    group0: (i32, i32),
    /// (register, box) for each box the call put in a register (or made
    /// void by moving its box out, box 0): read, a register holding one of
    /// them is the call's own, not an input; a register holding a box the
    /// call did not put there (a group ended) is foreign.
    boxes: HashSet<(i32, i32)>,
    /// `boxes` as it was where each open group of the call began: the
    /// group's `unsave` puts back every register it saved, and with it
    /// whether the register is the call's (`flashtex_bm_unsave`).
    box_stack: Vec<HashSet<(i32, i32)>>,
    /// Box registers assigned globally: each must be void when the call
    /// ends, and the replay makes it so (`\global\setbox` to void, at the
    /// end of the log).
    gboxes: Vec<i32>,
    /// A foreign box register just fetched, waiting for what is done with
    /// it (`flashtex_bm_box_use`): (register, box).
    pending_box: Option<(i32, i32)>,
    /// A foreign box register whose dimension a span is assigning, until
    /// the value is scanned (`flashtex_bm_box_dimen`).
    alter: Option<i32>,
    kb: Vec<(i32, Vec<i32>)>,
    /// Foreign box registers a span copied (`\copy`), by contents (§17.6).
    kc: Vec<(i32, crate::boxfrag::Frag, Positions)>,
    /// (register, copy) of each `\copy` of a foreign register a span made,
    /// and the register of the one being made (§17.6).
    copies: Vec<(i32, i32)>,
    copying: Option<i32>,
    /// (register, box node, key) of each box moved out of a foreign register
    holes: Vec<(i32, i32, Vec<i32>)>,
    ops: Vec<Op>,
    /// The control sequences the call entered (`Op::NewCs`), in order.
    created: Vec<i32>,
    /// The call packed a box (`hpack`, `vpackage`), so it set `last_badness`.
    packed: bool,
    /// Verifying the entry of this index (of `store[cs]`).
    verify: Option<usize>,
}

#[derive(Default)]
struct State {
    store: HashMap<i32, Vec<Entry>>,
    /// Per macro, its variants' indices by the hash of K1 and the context
    /// (`Entry::hash`), made again from `store` after a variant is added
    /// or dropped (`stale`).
    index: HashMap<i32, HashMap<u64, Vec<usize>>>,
    stale: HashSet<i32>,
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
    /// A span's file level (`base - 1`), else -1: the one level below the
    /// body the recording may read (§17).
    span_f: i32,
    stamp: u32,
    seen: Vec<u32>,
}

/// A key by location (`bm_key_by_loc`): each location and its values.
type KeyByLoc = Vec<(i32, Vec<i32>)>;

thread_local! {
    /// Debugging misses: the key by location of the last recording, per macro.
    static LAST_KEY: RefCell<HashMap<i32, KeyByLoc>> = RefCell::new(HashMap::new());
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
                Ok("off") | Ok("0") => Mode::Off,
                _ if DEFAULT_ON.load(std::sync::atomic::Ordering::Relaxed) => Mode::On,
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
            let spans = std::env::var("FLASHTEX_BOXMEMO_SPANS")
                .map(|v| {
                    v.split(',')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.as_bytes().to_vec())
                        .collect()
                })
                .unwrap_or_default();
            *c = Some(Config {
                mode,
                names,
                spans,
                stats_out: std::env::var("FLASHTEX_BOXMEMO_STATS").ok(),
                debug: std::env::var_os("FLASHTEX_BOXMEMO_DEBUG").is_some(),
                verify_fail: std::env::var_os("FLASHTEX_BOXMEMO_VERIFY_FAIL").is_some(),
                full_in_host: std::env::var_os("FLASHTEX_BOXMEMO_FULL_VERIFY_HOST").is_some(),
            });
        }
        f(c.as_ref().unwrap())
    })
}

pub fn mode() -> Mode {
    with_config(|c| c.mode)
}

/// Macros BOX-MEMO records anywhere, not only in an edit's window (§6), and
/// takes from the intrinsics wherever it is on (`owns`): hyperref's
/// `\pdfstringdefPreHook` (§12.2). Its calls hit on every later page, a
/// cold pass's included, and its entries survive the restores that take
/// the intrinsics' recordings away. And pgf's `\pgf@parsefunc` (§14): a
/// shading declaration around it is refused (its `\pdfxform`), and a call
/// is never offered inside another's recording, so in the window it was
/// first recorded at the second key on a frame; recorded where the open's
/// cold run first parses each function, it hits from the first.
pub const RECORD_ANYWHERE: &[&str] = &["pdfstringdefPreHook", "pgf@parsefunc"];

fn record_anywhere() -> &'static [Vec<u8>] {
    static R: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
    R.get_or_init(|| {
        RECORD_ANYWHERE
            .iter()
            .map(|s| s.as_bytes().to_vec())
            .collect()
    })
}

/// Whether BOX-MEMO, on and offering `name`, takes it from the intrinsics
/// (`intrinsics::config`): one of [`RECORD_ANYWHERE`]. One owner per macro
/// in a session, so that every run of the session leaves the intrinsics'
/// state alike (the convergence test compares it, §12.2).
pub fn owns(name: &[u8]) -> bool {
    RECORD_ANYWHERE.iter().any(|n| n.as_bytes() == name)
        && with_config(|c| c.mode != Mode::Off && c.names.iter().any(|x| x == name))
}

fn debug() -> bool {
    with_config(|c| c.debug)
}

/// Counts over the session, for the host's `DONE` (`stages.boxmemo`):
/// offers, hits, recordings committed, control sequences a replay entered.
pub fn counts() -> [u64; 4] {
    ST.with(|s| {
        let s = s.borrow();
        [
            s.stats.offers,
            s.stats.hits,
            s.stats.committed,
            s.stats.entered,
        ]
    })
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
        // (nor from a thread that never offered a call: the host's other
        // threads, which would write over the engine thread's counts)
        if (!force && s.stats_dirty % 32 != 0) || s.stats.offers == 0 {
            return None;
        }
        s.stats.entries = s.store.values().map(|v| v.len()).sum();
        s.stats.bytes = s.bytes;
        [
            s.stats.meanings_walked,
            s.stats.meanings_seen,
            s.stats.seen_cleared,
        ] = seen_counts();
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

/// The version that names format file `path` as it is now: its path and
/// signature (length, modification time, identity), interned; a fresh one
/// when there is no path or the signature is racy (it then proves nothing,
/// `StatSig::racy`).
fn format_version(path: Option<&str>) -> i32 {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static TABLE: Mutex<Option<HashMap<String, i32>>> = Mutex::new(None);
    let Some(p) = path else {
        return fresh_version();
    };
    let Some(s) = crate::system::StatSig::of(p) else {
        return fresh_version();
    };
    if s.racy {
        return fresh_version();
    }
    let key = format!("{p}\t{}\t{}\t{}", s.len, s.mtime_ns, s.ino);
    let mut t = TABLE.lock().unwrap_or_else(|e| e.into_inner());
    *t.get_or_insert_with(HashMap::new)
        .entry(key)
        .or_insert_with(fresh_version)
}

/// The fonts' version after the change `change` (what changed, and the
/// value it got) to version `parent`: the same number for the same parent
/// and change, a fresh one otherwise. A version thus names the sequence of
/// changes since the engine's start (whose version is fresh), so two runs
/// that make the same changes -- an edit's window typed in again, which
/// loads the same font and sets its `\hyphenchar` again -- reach the same
/// version, and equal versions name equal font arrays. (A fresh number at
/// every change, as revision 1 had it, kept every call after a font load
/// in the window from ever hitting.)
fn next_version(parent: i32, change: [i32; 4]) -> i32 {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type Versions = HashMap<(i32, [i32; 4]), i32>;
    static TABLE: Mutex<Option<Versions>> = Mutex::new(None);
    let mut t = TABLE.lock().unwrap_or_else(|e| e.into_inner());
    *t.get_or_insert_with(HashMap::new)
        .entry((parent, change))
        .or_insert_with(fresh_version)
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
    // pdfTeX's protrusion: `char_pw` sets the character it looked at
    // (`left_pw`, `right_pw`), and both are read only right after, in the
    // same line break (`total_pw` then try_break's "Calculate variations of
    // marginal kerns", post_line_break's `new_margin_kern`). fancyhdr's head
    // breaks its `\parbox`es with microtype's protrusion on (a replay of
    // `\@oddhead`, proof-practice-21242 in sweep A's verify mode)
    last_leftmost_char: i32,
    last_rightmost_char: i32,
    // pdfTeX's `divide_scaled` remainder, read only right after a call of it
    // (`adv_char_width`, `pdf_set_origin`, `pdf_set_textmatrix`,
    // `pdf_use_font`, `pdf_begin_string` after their `divide_scaled` or
    // `pdf_print_bp`); font expansion's packing sets it
    scaled_out: i32,
    // font expansion's ratio: `hpack(cal_expand_ratio)` zeroes and computes
    // it, and only the `hpack(subst_ex_font)` it then calls reads it
    font_expand_ratio: i32,
    // a paragraph's expansion limits and step: `line_break` sets all three
    // as it starts (pdftex.web "Get ready to start line breaking", when
    // `pdf_adjust_spacing > 1`), and only that line break reads them
    // (`check_expand_pars`, try_break's shortfall), under the same test
    max_stretch_ratio: i32,
    max_shrink_ratio: i32,
    cur_font_step: i32,
    // pseudo-printing scratch (D9's list)
    tally: i32,
    first_count: i32,
    trick_count: i32,
    base_ptr: i32,
    // `big_switch`'s flags, set before each `get_x_token`
    intr_at_switch: bool,
    bm_at_switch: bool,
    // `mlist_penalties`: every caller of `mlist_to_hlist` (`clean_box`, its
    // own sub-list case, `after_math`'s three) sets it just before the
    // call, which reads it on entry and nowhere else (pdftex.web; found by
    // a span's full verification, BOX-MEMO.md §17)
    mlist_penalties: bool,
    // `cur_style`, `cur_size`, `cur_mu`: likewise every caller of
    // `mlist_to_hlist` sets `cur_style` before the call, which sets the
    // other two from it; only it and its helpers read them (pdftex.web,
    // `clean_box` to `make_scripts`; a span in display math, §17)
    cur_style: i32,
    cur_size: i32,
    cur_mu: i32,
    // The current list's head's `info`: `push_nest`'s `get_avail` leaves
    // what the cell held, and nothing reads it (pdftex.web reads a list's
    // head only by `link` and as `tail=head`; `\lastpenalty` & co. test it
    // with `is_char_node`). A span that began a paragraph (§17) got its
    // head from another cell than its replay.
    head_info: i32,
}

impl Scratch {
    fn take(g: &Globals) -> Scratch {
        Scratch {
            dig: (0..g.dig.len()).map(|i| g.dig[i]).collect(),
            trick_buf: (0..g.trick_buf.len()).map(|i| g.trick_buf[i]).collect(),
            pstack: (0..g.pstack.len()).map(|i| g.pstack[i]).collect(),
            total_stretch: (0..g.total_stretch.len())
                .map(|i| g.total_stretch[i])
                .collect(),
            total_shrink: (0..g.total_shrink.len())
                .map(|i| g.total_shrink[i])
                .collect(),
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
            last_leftmost_char: g.last_leftmost_char,
            last_rightmost_char: g.last_rightmost_char,
            scaled_out: g.scaled_out,
            font_expand_ratio: g.font_expand_ratio,
            max_stretch_ratio: g.max_stretch_ratio,
            max_shrink_ratio: g.max_shrink_ratio,
            cur_font_step: g.cur_font_step,
            tally: g.tally,
            first_count: g.first_count,
            trick_count: g.trick_count,
            base_ptr: g.base_ptr,
            intr_at_switch: g.intr_at_switch,
            bm_at_switch: g.bm_at_switch,
            mlist_penalties: g.mlist_penalties,
            cur_style: g.cur_style,
            cur_size: g.cur_size,
            cur_mu: g.cur_mu,
            head_info: g.mem[g.cur_list.head_field as usize].hh().lh(),
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
        g.last_leftmost_char = self.last_leftmost_char;
        g.last_rightmost_char = self.last_rightmost_char;
        g.scaled_out = self.scaled_out;
        g.font_expand_ratio = self.font_expand_ratio;
        g.max_stretch_ratio = self.max_stretch_ratio;
        g.max_shrink_ratio = self.max_shrink_ratio;
        g.cur_font_step = self.cur_font_step;
        g.tally = self.tally;
        g.first_count = self.first_count;
        g.trick_count = self.trick_count;
        g.base_ptr = self.base_ptr;
        g.intr_at_switch = self.intr_at_switch;
        g.bm_at_switch = self.bm_at_switch;
        g.mlist_penalties = self.mlist_penalties;
        g.cur_style = self.cur_style;
        g.cur_size = self.cur_size;
        g.cur_mu = self.cur_mu;
        let h = g.cur_list.head_field;
        if h >= g.hi_mem_min && g.mem[h as usize].hh().lh() != self.head_info {
            g.mem[h as usize].set_hh_lh(self.head_info);
        }
    }
}

/// Whether two logged operations are the same: a box by value (§16) in
/// the bits the program reads (`boxfrag::frag_diff`), the rest exactly.
fn same_op(a: &Op, b: &Op) -> bool {
    match (a, b) {
        (
            Op::Def {
                p,
                t,
                v: Val::Box(x),
                kind,
            },
            Op::Def {
                p: q,
                t: u,
                v: Val::Box(y),
                kind: l,
            },
        ) => p == q && t == u && kind == l && crate::boxfrag::frag_diff(x, y).is_none(),
        _ => a == b,
    }
}

/// A value whose word does not depend on the state where it is made: the
/// assignment makes the same `equiv` (or word) wherever it is replayed.
/// `\let` (`LetCs`, the meaning there) and a shared glue (`GlueFrom`, the
/// specification there) take theirs from the state.
fn by_value(v: &Val) -> bool {
    !matches!(v, Val::LetCs(_) | Val::GlueFrom(..))
}

/// A span's log with its entered control sequences first, in their order
/// (§17.5), so that [`compress_groups`] can take the groups holding them by
/// their effect. `id_lookup` making a name changes the hash and the string
/// pool and nothing a later assignment or group reads: no `unsave` takes a
/// name away, no assignment reads the hash (a `\let`'s meaning is read from
/// `eqtb`, which entering a name leaves as it was, undefined), and a name is
/// assigned only after it is entered. Only the names' order among
/// themselves decides where each lands, and that is kept. A pgfplots axis
/// enters a few dozen names in the outermost group and makes some 10^5
/// assignments to a few hundred locations.
fn new_cs_first(ops: &[Op]) -> Vec<Op> {
    let mut v: Vec<Op> = ops
        .iter()
        .filter(|o| matches!(o, Op::NewCs { .. }))
        .cloned()
        .collect();
    v.extend(
        ops.iter()
            .filter(|o| !matches!(o, Op::NewCs { .. }))
            .cloned(),
    );
    v
}

/// The operations a replay makes: `ops` with every group (`Begin` .. its
/// `End`) whose net effect can be told from its operations alone replaced
/// by that effect (BOX-MEMO.md §11.2, §12.5).
///
/// At its `unsave` a group puts back every location it assigned locally:
/// `eq_save` kept the word the location held, `unsave` restores it and
/// `eq_destroy`s the group's value (tex.web §283; e-TeX's `restore_sa`
/// likewise), and a reassignment of the value a location already holds
/// (e-TeX's "reassigning") saves nothing and changes nothing. A location
/// the group assigned globally (`geq_define`, `geq_word_define`: level one)
/// is kept ("retaining"), with the value of the group's *last* global
/// assignment to it: a local assignment after that one saved the global
/// value, which `unsave` puts back. So the group's net effect on `eqtb`
/// and the save stack is one global assignment per location it assigned
/// globally, the last one, made where the group was; a group with none
/// leaves nothing (revision 2's elision). What it changes besides is memory
/// the program does not read back: the free lists and the usage statistics
/// (`var_used`, `dyn_used`, `max_save_stack`), which DESIGN.md §1.1 counts
/// as accounting and the convergence test treats as dead. The recording
/// refuses everything else a group could leave (an `\aftergroup` token,
/// output, tracing).
///
/// A global value from the state ([`by_value`]: a `\\let`'s meaning, a
/// shared glue) is the one where the group was when nothing in the group
/// assigned what it reads before it; it is then made there, before what
/// the group assigns after it. A group is replayed as recorded (its own
/// groups replaced in turn) when such a value reads what the group
/// assigned (inside the group the state is the group's own), or a font's
/// glue or a sparse register, when it assigns a sparse register globally,
/// or when it enters a control sequence (no `unsave` takes it away); less
/// its local assignments to every location no such value in it reads,
/// which its `unsave` takes back unread.
fn compress_groups(ops: &[Op]) -> Vec<Op> {
    compress_groups_with(ops, false)
}

/// The location of sparse register (type `t`, number `n`) among
/// [`compress_groups_with`]'s, below every `eqtb` location.
fn sa_loc(t: i32, n: i32) -> i32 {
    -(t * 65536 + n) - 1
}

/// [`compress_groups`]; with `sparse`, a span's (§17.5), a group that
/// assigns e-TeX's sparse registers globally by value is taken by its
/// effect too: e-TeX's `sa_save`/`sa_restore` keep a location of level one
/// at the group's end and put back the others, as `eq_save` and `unsave`
/// do for `eqtb` (etex.ch, "retaining"/"restoring"), so a sparse register
/// is one more location, its net effect its last global assignment. (pgf's
/// counters above 255 are sparse: every picture's serial is one.)
fn compress_groups_with(ops: &[Op], sparse: bool) -> Vec<Op> {
    // (each `Begin`'s `End`)
    let n = ops.len();
    let mut end = vec![usize::MAX; n];
    let mut stack = vec![];
    for (i, o) in ops.iter().enumerate() {
        match o {
            Op::Begin(_) => stack.push(i),
            Op::End => {
                if let Some(b) = stack.pop() {
                    end[b] = i;
                }
            }
            _ => {}
        }
    }
    // The net global assignments of the group ops[b] begins, in the order
    // of their last assignment, and every location it assigned; `None`
    // when it is replayed as recorded.
    fn net(ops: &[Op], end: &[usize], b: usize, sparse: bool) -> Option<(Vec<Op>, HashSet<i32>)> {
        let e = end[b];
        if e == usize::MAX {
            return None;
        }
        // (in the order of each location's last assignment: a location's
        // earlier one is left as `None`, found by `at`; a pgfplots group
        // makes 10^5 assignments to a few thousand locations)
        let mut last: Vec<Option<Op>> = vec![];
        let mut at: HashMap<i32, usize> = HashMap::new();
        // (a value from the state is the group's entry state's when nothing
        // in the group assigned what it reads before it: it is then made
        // where the group was, before what the group assigns after it)
        let mut touched: HashSet<i32> = HashSet::new();
        let from_entry = |o: &Op, touched: &HashSet<i32>| match o {
            Op::Def {
                v: Val::LetCs(q), ..
            } => !touched.contains(q),
            Op::Def {
                v: Val::GlueFrom(a, _),
                ..
            } => *a > 0 && !touched.contains(a),
            _ => true,
        };
        let put = |p: i32, o: &Op, last: &mut Vec<Option<Op>>, at: &mut HashMap<i32, usize>| {
            if let Some(i) = at.insert(p, last.len()) {
                last[i] = None;
            }
            last.push(Some(o.clone()));
        };
        // (a span's, §17.5: `cur` holds what each control sequence the
        // group assigned holds now, as a value; a `\let` from one of them
        // takes that value, as the normal path's `\let` takes the meaning
        // it has then. Two locations that shared a token list then hold
        // equal lists: a memory difference only, DESIGN.md §1.1. pgfplots'
        // `\global\let`s of macros its group defined kept every plot's
        // outermost group whole)
        let mut cur: HashMap<i32, Op> = HashMap::new();
        let resolve = |o: &Op, touched: &HashSet<i32>, cur: &HashMap<i32, Op>| -> Option<Op> {
            if from_entry(o, touched) {
                return Some(o.clone());
            }
            let Op::Def {
                p,
                kind,
                v: Val::LetCs(q),
                ..
            } = o
            else {
                return None;
            };
            if !sparse {
                return None;
            }
            let Some(Op::Def { t, v, .. }) = cur.get(q) else {
                return None;
            };
            let r = Op::Def {
                p: *p,
                t: *t,
                v: v.clone(),
                kind: *kind,
            };
            from_entry(&r, touched).then_some(r)
        };
        // (does the group assign box register `n`? a `BoxDim` changes the
        // box it holds then)
        let assigns_box = |n: i32| {
            ops[b + 1..e].iter().any(|o| match o {
                Op::Def { p, .. } => n < 256 && *p == BOX_BASE + n,
                Op::Sa { t, n: m, .. } => *t == BOX_VAL && *m == n,
                _ => false,
            })
        };
        let mut i = b + 1;
        while i < e {
            match &ops[i] {
                Op::Begin(_) => {
                    let (v, t) = net(ops, end, i, sparse)?;
                    for o in v {
                        let o = resolve(&o, &touched, &cur)?;
                        let p = match &o {
                            Op::Def { p, .. } => {
                                cur.insert(*p, o.clone());
                                *p
                            }
                            Op::Sa { t, n, .. } => sa_loc(*t, *n),
                            Op::BoxDim { n, .. } => {
                                if assigns_box(*n) {
                                    return None;
                                }
                                last.push(Some(o.clone()));
                                continue;
                            }
                            _ => unreachable!(),
                        };
                        put(p, &o, &mut last, &mut at);
                    }
                    // (a location the inner group assigned only locally
                    // holds again what it held before it)
                    touched.extend(t);
                    i = end[i] + 1;
                    continue;
                }
                Op::Def { p, kind, t, v } => {
                    // (a list made here: named by where, for its sharers)
                    let made;
                    let o = match v {
                        Val::FreshList(l) if sparse => {
                            made = Op::Def {
                                p: *p,
                                t: *t,
                                v: Val::Shared(i as u32, l.clone()),
                                kind: *kind,
                            };
                            &made
                        }
                        _ => &ops[i],
                    };
                    let r = resolve(o, &touched, &cur);
                    if *kind > 1 {
                        put(*p, r.as_ref()?, &mut last, &mut at);
                    }
                    match r {
                        Some(r) if sparse => {
                            cur.insert(*p, r);
                        }
                        _ => {
                            cur.remove(p);
                        }
                    }
                    touched.insert(*p);
                }
                Op::Sa { t, n, v, kind } if sparse => {
                    if *kind > 1 {
                        if !by_value(v) {
                            return None;
                        }
                        put(sa_loc(*t, *n), &ops[i], &mut last, &mut at);
                    }
                    touched.insert(sa_loc(*t, *n));
                }
                Op::Sa { kind, .. } if *kind > 1 => return None,
                Op::Sa { .. } => {}
                Op::BoxDim { n, .. } => {
                    // (no group takes it back: it stays, in its order
                    // among the others, unless the group assigns the
                    // register, whose box it changes)
                    if assigns_box(*n) {
                        return None;
                    }
                    last.push(Some(ops[i].clone()));
                }
                Op::NewCs { .. } | Op::End => return None,
            }
            i += 1;
        }
        Some((last.into_iter().flatten().collect(), touched))
    }
    // `kept`: inside a group replayed as recorded, the locations a value
    // from the state reads anywhere in it (`None` at the call's level).
    // There a local assignment to any other location is left out: the
    // group's `unsave` takes it back, and nothing in the group reads it.
    fn walk(
        ops: &[Op],
        end: &[usize],
        from: usize,
        to: usize,
        kept: Option<&HashSet<i32>>,
        out: &mut Vec<Op>,
        sparse: bool,
    ) {
        let mut i = from;
        while i < to {
            match &ops[i] {
                Op::Begin(_) if end[i] != usize::MAX => {
                    match net(ops, end, i, sparse) {
                        Some((v, _)) => out.extend(v),
                        None => {
                            let read: HashSet<i32> = ops[i + 1..end[i]]
                                .iter()
                                .filter_map(|o| match o {
                                    Op::Def {
                                        v: Val::LetCs(q), ..
                                    } => Some(*q),
                                    Op::Def {
                                        v: Val::GlueFrom(a, _),
                                        ..
                                    } if *a > 0 => Some(*a),
                                    _ => None,
                                })
                                .collect();
                            out.push(ops[i].clone());
                            walk(ops, end, i + 1, end[i], Some(&read), out, sparse);
                            out.push(Op::End);
                        }
                    }
                    i = end[i] + 1;
                    continue;
                }
                Op::Def { p, kind, .. } if *kind <= 1 && kept.is_some_and(|k| !k.contains(p)) => {}
                o => out.push(o.clone()),
            }
            i += 1;
        }
    }
    let mut out = Vec::with_capacity(n);
    walk(ops, &end, 0, n, None, &mut out, sparse);
    if sparse {
        // (a list only one operation holds is its own)
        let mut uses: HashMap<u32, u32> = HashMap::new();
        for o in &out {
            if let Op::Def {
                v: Val::Shared(id, _),
                ..
            } = o
            {
                *uses.entry(*id).or_default() += 1;
            }
        }
        for o in &mut out {
            if let Op::Def { v, .. } = o {
                if let Val::Shared(id, l) = v {
                    if uses[id] == 1 {
                        *v = Val::FreshList(std::mem::take(l));
                    }
                }
            }
        }
    }
    out
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

/// Whether a span is being recorded (§17.4). The host holds a timed
/// checkpoint back until it ends: a checkpoint's record takes the
/// line-shift journal's entries, after which the span cannot tell its own
/// (`journal_since`) and is abandoned (`LineShift`), and, verified, its
/// normal path then disagrees with the guard that let a replay through.
///
/// A replay's fast-forward over the span's lines counts them as the normal
/// path's reads did, with the checkpoint held back as there
/// (`SPAN_FORWARD`): else the replay alone asks for one, a difference of
/// the whole-state verifier's (`ckpt_request`, §17.12).
pub fn span_recording(g: &Globals) -> bool {
    (g.bm_rec_on && FAST.with(|f| f.borrow().span_f >= 0)) || SPAN_FORWARD.with(|c| c.get())
}

thread_local! {
    /// A span's replay is moving the file past its lines (`bm_span_forward`).
    static SPAN_FORWARD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn after_restore(g: &mut Globals) {
    // (the word space is another state: §12.4's entries say nothing of it)
    seen_clear();
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
// Meanings found equal (§12.4)
// ---------------------------------------------------------------------------

type IntHash = std::hash::BuildHasherDefault<crate::iso::FastHasher>;

/// The macro bodies K3 found equal since the last restore (BOX-MEMO.md
/// §12.4): location `p` held type `t` and the list at reference count `e`,
/// whose tokens were the kept body. A macro's list does not change while it
/// is referenced (TeX makes a new one for every definition), and a list
/// freed (`delete_token_ref` reaching zero, `flashtex_bm_freed`) takes the
/// entries naming it with it, so while `eqtb[p]` still holds `t` and `e` its
/// tokens are still that body and the walk is not made again. Emptied at
/// every restore of the word space (`after_restore`), at a format load, and
/// when another engine on this thread notes one (`owner`).
#[derive(Default)]
struct Seen {
    owner: usize,
    at: HashMap<i32, (i32, i32, std::rc::Rc<[i32]>), IntHash>,
    /// The locations whose entry names list `e`.
    by_list: HashMap<i32, Vec<i32>, IntHash>,
    /// The lists some entry names, a bit each by `mem` index: the free
    /// hook's test.
    bits: Vec<u64>,
    hits: u64,
    walks: u64,
    clears: u64,
}

impl Seen {
    fn clear(&mut self) {
        self.clears += 1;
        for &e in self.by_list.keys() {
            if let Some(w) = self.bits.get_mut(e as usize / 64) {
                *w = 0;
            }
        }
        self.at.clear();
        self.by_list.clear();
    }

    fn has_bit(&self, e: i32) -> bool {
        self.bits
            .get(e as usize / 64)
            .is_some_and(|w| w & (1u64 << (e as usize % 64)) != 0)
    }

    /// `p`'s entry, which names list `e`, goes.
    fn forget_at(&mut self, p: i32, e: i32) {
        self.at.remove(&p);
        if let Some(l) = self.by_list.get_mut(&e) {
            l.retain(|&q| q != p);
            if l.is_empty() {
                self.by_list.remove(&e);
                if let Some(w) = self.bits.get_mut(e as usize / 64) {
                    *w &= !(1u64 << (e as usize % 64));
                }
            }
        }
    }
}

thread_local! {
    static SEEN: RefCell<Seen> = RefCell::new(Seen::default());
    /// Some entry is kept (the free hook's first test).
    static SEEN_ANY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn owner_of(g: &Globals) -> usize {
    g as *const Globals as usize
}

/// Whether `p` holding `t` and `e` was found equal to `toks` since.
fn seen_is(g: &Globals, p: i32, t: i32, e: i32, toks: &std::rc::Rc<[i32]>) -> bool {
    if !SEEN_ANY.with(|a| a.get()) {
        return false;
    }
    SEEN.with(|s| {
        let mut s = s.borrow_mut();
        if s.owner != owner_of(g) {
            return false;
        }
        let ok =
            s.at.get(&p)
                .is_some_and(|(t0, e0, b)| *t0 == t && *e0 == e && std::rc::Rc::ptr_eq(b, toks));
        if ok {
            s.hits += 1;
        }
        ok
    })
}

/// `p` holding `t` and `e` was just found equal to `toks`.
fn seen_note(g: &Globals, p: i32, t: i32, e: i32, toks: &std::rc::Rc<[i32]>) {
    SEEN.with(|s| {
        let mut s = s.borrow_mut();
        if s.owner != owner_of(g) {
            s.clear();
            s.owner = owner_of(g);
        }
        if let Some(&(_, e0, _)) = s.at.get(&p) {
            s.forget_at(p, e0);
        }
        let w = e as usize / 64;
        if s.bits.len() <= w {
            let n = (w + 1).max(g.mem.len() / 64 + 1);
            s.bits.resize(n, 0);
        }
        s.bits[w] |= 1u64 << (e as usize % 64);
        s.by_list.entry(e).or_default().push(p);
        s.at.insert(p, (t, e, toks.clone()));
    });
    SEEN_ANY.with(|a| a.set(true));
}

/// Every entry goes (a restore, a format load).
fn seen_clear() {
    if SEEN_ANY.with(|a| a.replace(false)) {
        SEEN.with(|s| s.borrow_mut().clear());
    }
}

/// K3's walks, the comparisons the entries answered, and the times they
/// were emptied (`Stats`).
fn seen_counts() -> [u64; 3] {
    SEEN.with(|s| {
        let s = s.borrow();
        [s.walks, s.hits, s.clears]
    })
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
        self.bm_k2_each(|x| {
            v.push(x);
            true
        });
        v
    }

    /// Whether K2 now is `want`, compared as it is made (no copy of the
    /// state is built), stopping at the first difference.
    fn bm_k2_is(&self, want: &[i32]) -> bool {
        // (`bm_k2_each`'s walk, a run of fixed-width values compared as one
        // slice)
        let mut at = 0usize;
        let mut take = |n: usize| -> Option<&[i32]> {
            let s = want.get(at..at + n)?;
            at += n;
            Some(s)
        };
        for &(a, b, kind) in k2_plan() {
            let words = &self.eqtb[(a - 1) as usize..(b - 1) as usize];
            match kind {
                // (`rh` and `int` are both a word's low half)
                K2Kind::Equiv | K2Kind::Int => match take(words.len()) {
                    Some(w) if low_halves_eq(words, w) => {}
                    _ => return false,
                },
                K2Kind::Glue => {
                    for x in words {
                        let e = x.hh().rh();
                        let ok = if e <= LO_MEM_STAT_MAX {
                            take(2) == Some(&[-1, e][..])
                        } else {
                            take(5) == Some(&self.bm_glue_fields(e)[..])
                        };
                        if !ok {
                            return false;
                        }
                    }
                }
                K2Kind::Shape => {
                    for x in words {
                        let ok = match self.bm_shape_words(x.hh().rh()) {
                            Some(v) => {
                                take(1) == Some(&[v.len() as i32][..])
                                    && take(v.len()) == Some(&v[..])
                            }
                            None => take(1) == Some(&[-2][..]),
                        };
                        if !ok {
                            return false;
                        }
                    }
                }
                K2Kind::List => {
                    for x in words {
                        let e = x.hh().rh();
                        if e == 0 {
                            if take(1) != Some(&[-1][..]) {
                                return false;
                            }
                            continue;
                        }
                        let mut q = self.bm_link(e);
                        while q != 0 {
                            if take(1) != Some(&[self.bm_info(q)][..]) {
                                return false;
                            }
                            q = self.bm_link(q);
                        }
                        if take(1) != Some(&[-3][..]) {
                            return false;
                        }
                    }
                }
            }
        }
        at == want.len()
    }

    /// K2's values in order, each handed to `f` until it returns false;
    /// false then. A token list is its tokens and the end marker -3 (a
    /// token is never negative), the null list -1; a glue specification
    /// -1 and the pointer when it is static, else its five fields. The
    /// locations are walked by runs of one kind ([`k2_plan`]).
    fn bm_k2_each(&self, mut f: impl FnMut(i32) -> bool) -> bool {
        for &(a, b, kind) in k2_plan() {
            let words = &self.eqtb[(a - 1) as usize..(b - 1) as usize];
            match kind {
                K2Kind::Glue => {
                    for w in words {
                        let e = w.hh().rh();
                        if e <= LO_MEM_STAT_MAX {
                            if !(f(-1) && f(e)) {
                                return false;
                            }
                        } else if !self.bm_glue_fields(e).into_iter().all(&mut f) {
                            return false;
                        }
                    }
                }
                K2Kind::Shape => {
                    for w in words {
                        let ok = match self.bm_shape_words(w.hh().rh()) {
                            Some(v) => f(v.len() as i32) && v.into_iter().all(&mut f),
                            None => f(-2),
                        };
                        if !ok {
                            return false;
                        }
                    }
                }
                K2Kind::List => {
                    for w in words {
                        let e = w.hh().rh();
                        if e == 0 {
                            if !f(-1) {
                                return false;
                            }
                            continue;
                        }
                        let mut q = self.bm_link(e);
                        while q != 0 {
                            if !f(self.bm_info(q)) {
                                return false;
                            }
                            q = self.bm_link(q);
                        }
                        if !f(-3) {
                            return false;
                        }
                    }
                }
                K2Kind::Equiv => {
                    for w in words {
                        if !f(w.hh().rh()) {
                            return false;
                        }
                    }
                }
                K2Kind::Int => {
                    for w in words {
                        if !f(w.int()) {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    /// The kind of K2 value at `eqtb` location `p`, `None` for a location
    /// K2 leaves out (the box registers, and the registers KR keys
    /// precisely, BOX-MEMO.md §10.1).
    fn bm_k2_kind(p: i32) -> Option<K2Kind> {
        if (BOX_BASE..BOX_BASE + 256).contains(&p) || Self::bm_is_register(p) {
            None
        } else if p < LOCAL_BASE {
            Some(K2Kind::Glue)
        } else if p == PAR_SHAPE_LOC || (ETEX_PEN_BASE..ETEX_PENS).contains(&p) {
            Some(K2Kind::Shape)
        } else if (OUTPUT_ROUTINE_LOC..TOKS_BASE + 256).contains(&p) {
            Some(K2Kind::List)
        } else if p < INT_BASE {
            Some(K2Kind::Equiv)
        } else {
            Some(K2Kind::Int)
        }
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
    pub(crate) fn bm_sa_walk(&self, t: i32, f: &mut dyn FnMut(i32, i32)) {
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
    #[inline]
    fn bm_is_register(p: i32) -> bool {
        (k::count_base..k::count_base + 256).contains(&p)
            || (k::scaled_base..k::scaled_base + 256).contains(&p)
            || (k::skip_base..k::skip_base + 256).contains(&p)
            || (k::mu_skip_base..k::mu_skip_base + 256).contains(&p)
            || (TOKS_BASE..TOKS_BASE + 256).contains(&p)
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
                    INT_VAL | DIMEN_VAL => v.push(if e == 0 {
                        0
                    } else {
                        self.mem[(e + 2) as usize].int()
                    }),
                    GLUE_VAL | MU_VAL => {
                        let q = if e == 0 {
                            k::zero_glue
                        } else {
                            self.bm_link(e + 1)
                        };
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
            return Some(Meaning::Macro(t, intern_tokens(v)));
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

    /// Whether the meaning of `p` is `m` ([`bm_meaning`](Self::bm_meaning)
    /// compared in place, without copying a macro's tokens).
    fn bm_meaning_is(&self, p: i32, m: &Meaning) -> bool {
        if let Meaning::Name(n) = m {
            // A location of the hash's chains holds the name `id_lookup`
            // finds there: it enters a name only where it found none, in
            // that name's chain (tex.web §259-260), and a location's text
            // stays (the frozen locations, whose texts repeat names, are
            // looked up as before).
            let chains = (k::hash_base..k::frozen_control_sequence).contains(&p)
                || (k::eqtb_size < p && p <= k::hash_top);
            if chains && n.len() >= 2 {
                let t = self.hash[(p - k::hash_base) as usize].rh();
                if t <= 0 || t >= self.str_ptr {
                    return false;
                }
                let (a, b) = (
                    self.str_start[t as usize] as usize,
                    self.str_start[t as usize + 1] as usize,
                );
                return self.str_pool[a..b] == n[..];
            }
            return self.bm_find_cs(n) == Some(p);
        }
        let w = self.bm_eq(p).hh();
        let (t, e) = (w.b0(), w.rh());
        if (CALL..=LONG_OUTER_CALL).contains(&t) && e != 0 {
            let Meaning::Macro(mt, toks) = m else {
                return false;
            };
            if *mt != t {
                return false;
            }
            if seen_is(self, p, t, e, toks) {
                return true;
            }
            SEEN.with(|s| s.borrow_mut().walks += 1);
            let mut q = self.bm_link(e);
            for &x in toks.iter() {
                if q == 0 || self.bm_info(q) != x {
                    return false;
                }
                q = self.bm_link(q);
            }
            if q == 0 {
                seen_note(self, p, t, e, toks);
            }
            return q == 0;
        }
        self.bm_meaning(p).as_ref() == Some(m)
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
            aux: self.cur_list.aux_field.0,
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
            obj_ptr: (self.obj_ptr, self.sys_obj_ptr),
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
        seen_clear();
        // The fonts' and the exceptions' first versions name the format
        // file loaded (`format_version`): the same unchanged file gives the
        // same versions, so two loads leave the same state (COLD-FIXED's
        // image stands for the second), and another format other versions.
        let v = format_version(self.fmt_file.path());
        self.bm_font_version = v;
        self.bm_hyph_version = v;
        let names = with_config(|c| c.names.clone());
        let spans = with_config(|c| !c.spans.is_empty());
        self.bm_span_cs = 0;
        for p in 1..=k::eqtb_top {
            if !Self::bm_is_cs(p) || p < k::hash_base {
                continue;
            }
            if self.bm_name_is(p, &names) {
                self.bm_cand[p as usize] = true;
            }
            // (§17: LaTeX's `\begin` offers the environments named)
            if spans && self.bm_name_is(p, &[b"begin".to_vec()]) {
                self.bm_cand[p as usize] = true;
                self.bm_span_cs = p;
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

    /// `delete_token_ref` frees the token list at reference count `p`: the
    /// meanings found equal to it go (§12.4).
    #[inline]
    pub fn flashtex_bm_freed(&mut self, p: i32) {
        if !SEEN_ANY.with(|a| a.get()) {
            return;
        }
        SEEN.with(|s| {
            let mut s = s.borrow_mut();
            if !s.has_bit(p) {
                return;
            }
            for q in s.by_list.get(&p).cloned().unwrap_or_default() {
                s.forget_at(q, p);
            }
        });
    }

    /// A control sequence was just entered into the hash.
    pub fn flashtex_bm_new_cs(&mut self, p: i32) {
        // (the names compared where they are kept: siunitx's first
        // `\pdfstringdef` makes some 250 control sequences)
        if with_config(|c| self.bm_name_is(p, &c.names)) {
            self.bm_cand[p as usize] = true;
        }
        if self.bm_span_cs == 0
            && with_config(|c| !c.spans.is_empty())
            && self.bm_name_is(p, &[b"begin".to_vec()])
        {
            self.bm_cand[p as usize] = true;
            self.bm_span_cs = p;
        }
        if self.bm_rec_on {
            self.bm_note_new_cs(p);
        }
    }

    /// A recording entered control sequence `p` (§12.3): an operation, if
    /// no string was being built (`id_lookup` moves one up past the new
    /// name, and a replay could not tell where it stood).
    #[cold]
    #[inline(never)]
    fn bm_note_new_cs(&mut self, p: i32) {
        if self.pool_ptr != self.str_start[self.str_ptr as usize] {
            return self.bm_abort("NewCs");
        }
        let t = self.hash[(p - k::hash_base) as usize].rh();
        let (a, b) = (
            self.str_start[t as usize] as usize,
            self.str_start[t as usize + 1] as usize,
        );
        let name: Vec<i32> = self.str_pool[a..b].to_vec();
        self.bm_with_rec(|r| {
            r.created.push(p);
            r.ops.push(Op::NewCs { p, name });
        });
    }

    /// At a recording's end, with the string or hash counters moved: whether
    /// all the call left in the pool is the names of the control sequences
    /// it entered, one string each, in order, with no string being built at
    /// its start or end. `hash_used` and `hash_high` may have moved only for
    /// those; a replay enters them again (`Op::NewCs`).
    fn bm_strings_are_new_names(&self, start: &Frame, end: &Frame) -> bool {
        let (s0, p0, _) = start.strings;
        let (s1, p1, _) = end.strings;
        let Some(created) = self.bm_with_rec(|r| r.created.clone()) else {
            return false;
        };
        if created.is_empty()
            || s1 - s0 != created.len() as i32
            || self.str_start[s0 as usize] != p0
            || self.str_start[s1 as usize] != p1
        {
            return false;
        }
        created
            .iter()
            .enumerate()
            .all(|(i, &p)| self.hash[(p - k::hash_base) as usize].rh() == s0 + i as i32)
    }

    /// Whether the control sequences an entry enters (`Op::NewCs`, in order)
    /// are entered at their recorded locations if entered now: each name is
    /// absent, and `id_lookup`'s search for a free location (tex.web §259,
    /// web2c's `hash_extra`) ends where the recording's did, given the ones
    /// before it. Nothing is changed.
    fn bm_new_cs_placeable(&self, made: &[(Vec<i32>, i32)]) -> bool {
        if made.is_empty() {
            return true;
        }
        let hb = k::hash_base;
        let mut taken: HashMap<i32, usize> = HashMap::new();
        let mut links: HashMap<i32, i32> = HashMap::new();
        let (mut used, mut high) = (self.hash_used, self.hash_high);
        let text = |q: i32| self.hash[(q - hb) as usize].rh();
        let occupied = |taken: &HashMap<i32, usize>, q: i32| taken.contains_key(&q) || text(q) > 0;
        for (i, (name, want)) in made.iter().enumerate() {
            if name.is_empty() || self.first + name.len() as i32 >= k::buf_size {
                return false;
            }
            // tex.web §261
            let prime = k::hash_prime as i64;
            let mut h = name[0] as i64;
            for &c in &name[1..] {
                h = h + h + c as i64;
                while h >= prime {
                    h -= prime;
                }
            }
            let mut p = h as i32 + hb;
            loop {
                // (an earlier one of these, or a name in the table)
                match taken.get(&p) {
                    Some(&j) if made[j].0 == *name => return false,
                    Some(_) => {}
                    None => {
                        let t = text(p);
                        if t > 0 && t < self.str_ptr {
                            let (a, b) = (
                                self.str_start[t as usize] as usize,
                                self.str_start[t as usize + 1] as usize,
                            );
                            if b - a == name.len()
                                && self.str_pool[a..b].iter().zip(name).all(|(&c, &n)| c == n)
                            {
                                return false;
                            }
                        }
                    }
                }
                let next = links
                    .get(&p)
                    .copied()
                    .unwrap_or_else(|| self.hash[(p - hb) as usize].lh());
                if next != 0 {
                    p = next;
                    continue;
                }
                // §260: where it goes
                let at = if occupied(&taken, p) {
                    let q = if high < k::hash_extra {
                        high += 1;
                        high + k::eqtb_size
                    } else {
                        loop {
                            if used == hb {
                                return false;
                            }
                            used -= 1;
                            if !occupied(&taken, used) {
                                break used;
                            }
                        }
                    };
                    links.insert(p, q);
                    q
                } else {
                    p
                };
                if at != *want {
                    return false;
                }
                taken.insert(at, i);
                break;
            }
        }
        true
    }

    /// A replay enters `name` as the normal path's `\csname` did: through
    /// `id_lookup` on the buffer above the current line (tex.web §374),
    /// where `bm_new_cs_placeable` worked out it lands at `p`.
    fn bm_make_cs(&mut self, p: i32, name: &[i32]) {
        let first = self.first;
        for (i, &c) in name.iter().enumerate() {
            self.buffer[(first + i as i32) as usize] = c;
        }
        let j = first + name.len() as i32;
        if j > self.max_buf_stack {
            self.max_buf_stack = j;
        }
        let save = self.no_new_control_sequence;
        self.no_new_control_sequence = false;
        let q = self.id_lookup(first, name.len() as i32);
        self.no_new_control_sequence = save;
        // (an offer checked it: anything else is a broken invariant, and
        // going on would leave the state wrong)
        assert_eq!(
            q, p,
            "boxmemo: a replayed control sequence landed elsewhere"
        );
        ST.with(|s| s.borrow_mut().stats.entered += 1);
    }

    /// The dimensions of object `n` (a form or an image) as pdfTeX keeps
    /// them (`obj_xform_width` & co.: `pdf_mem` at its `obj_data_ptr`).
    fn bm_obj_dims(&self, n: i32) -> [i32; 3] {
        let d = self.obj_tab[n as usize].int4;
        [
            self.pdf_mem[d as usize],
            self.pdf_mem[(d + 1) as usize],
            self.pdf_mem[(d + 2) as usize],
        ]
    }

    /// Whether object `n` is one of type `t` (`pdf_check_obj`'s walk).
    fn bm_obj_is(&self, t: i32, n: i32) -> bool {
        if n <= 0 || n > self.obj_ptr {
            return false;
        }
        let mut k = self.head_tab[(t - 1) as usize];
        while k != 0 && k != n {
            k = self.obj_tab[k as usize].int1;
        }
        k == n
    }

    /// A recording referred to existing object `n` of type `t` (pdftex.web's
    /// `\pdfrefxform`, `\pdfrefximage`; changes/boxmemo.ch): the node
    /// copied its dimensions, so the object joins the key.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_obj_ref(&mut self, t: i32, n: i32) {
        let r = ObjRef {
            t,
            n,
            dims: self.bm_obj_dims(n),
        };
        self.bm_with_rec(|rec| {
            if !rec.obj_refs.contains(&r) {
                rec.obj_refs.push(r);
            }
        });
    }

    /// A recording packed a box (changes/boxmemo.ch: `hpack`, `vpackage`).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_packed(&mut self) {
        self.bm_with_rec(|r| r.packed = true);
    }

    /// A recording made object `n` (a `\pdfstartlink`'s `pdf_new_objnum`;
    /// changes/boxmemo.ch): a replay makes its own.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_new_obj(&mut self, n: i32) {
        self.bm_with_rec(|rec| rec.new_objs.push(n));
    }

    /// A font parameter was set (changes/boxmemo.ch): `n` -1 for
    /// `\fontdimen` (`a` its place in `font_info`), else `assign_font_int`'s
    /// code (`a` the font, `b` the character), and `v` the value.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_font_set(&mut self, n: i32, a: i32, b: i32, v: i32) {
        self.bm_font_version = next_version(self.bm_font_version, [n, a, b, v]);
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
        let call = self.warning_index;
        // the body must be the top level, as `Feed` left it
        if self.cur_input.state_field != TOKEN_LIST || self.cur_input.index_field != k::macro_ {
            return;
        }
        if call == self.ckpt_arm_cs || self.ckpt_arm_level == self.input_ptr {
            return;
        }
        // A span (§17): `\begin` of an environment named, read from a
        // file; its entries are kept under the environment's control
        // sequence.
        let mut cs = call;
        let mut span_key: Option<Vec<i32>> = None;
        if call == self.bm_span_cs && call != 0 {
            let r = self.bm_span_offer(n);
            if debug() {
                if let Err(Some(w)) = &r {
                    eprintln!("boxmemo: span not offered: {w} (line {})", self.line);
                } else if r.is_ok() {
                    eprintln!("boxmemo: span offered at line {}", self.line);
                }
            }
            match r {
                Ok((env, key)) => {
                    cs = env;
                    span_key = Some(key);
                }
                Err(None) => return,
                Err(Some(why)) => {
                    ST.with(|s| s.borrow_mut().stats.offers += 1);
                    return self.bm_miss(why);
                }
            }
        }
        let m = mode();
        ST.with(|s| s.borrow_mut().stats.offers += 1);
        // preconditions (K6)
        let globaldefs = self.bm_eq(INT_BASE + k::global_defs_code).int();
        if globaldefs != 0 || self.after_token != 0 {
            return self.bm_miss("Preconditions");
        }
        let mut k1 = self.bm_k1(call, n);
        if let Some(sk) = &span_key {
            k1.extend_from_slice(sk);
        }
        let ctx = self.bm_ctx(scanner);
        // K1 and the context select the variants (by their hash, then in
        // full); K2 and the rest are compared in place, without a copy of
        // the state: an offer that hits builds no key.
        let hash = key_hash(&k1, &ctx, &[], &[]);
        let k4_now = self.bm_k4();
        // (the most recent variant whose whole key holds; whether one held
        // up to K3 or K7 but not them)
        ST.with(|s| {
            let mut s = s.borrow_mut();
            if s.stale.remove(&cs) {
                let mut ix: HashMap<u64, Vec<usize>> = HashMap::new();
                for (i, e) in s.store.get(&cs).into_iter().flatten().enumerate() {
                    ix.entry(e.hash).or_default().push(i);
                }
                s.index.insert(cs, ix);
            }
        });
        let spanning = span_key.is_some();
        let (found, partial, fits) = ST.with(|s| {
            let s = s.borrow();
            let (Some(v), Some(ix)) = (
                s.store.get(&cs),
                s.index.get(&cs).and_then(|ix| ix.get(&hash)),
            ) else {
                return (None, None, vec![]);
            };
            let mut best: Option<(u64, usize)> = None;
            let mut partial = None;
            // (a span's variants whose key holds but for the file, which is
            // read after: §17)
            let mut fits: Vec<(u64, usize)> = vec![];
            let mut k2_seen: Vec<(*const i32, bool)> = vec![];
            for (i, e) in ix.iter().map(|&i| (i, &v[i])) {
                if !spanning && best.is_some_and(|(u, _)| u >= e.last_used) {
                    continue;
                }
                if e.hash != hash
                    || e.k1 != k1
                    || e.ctx != ctx
                    || e.k4 != k4_now
                    || !e.line.is_none_or(|l| l == self.line)
                    || !e
                        .group
                        .is_none_or(|g| g == (self.cur_level, self.cur_group))
                {
                    continue;
                }
                // (K2 in place, once per kept K2)
                let k2p = std::rc::Rc::as_ptr(&e.k2) as *const i32;
                let k2_ok = match k2_seen.iter().find(|(q, _)| *q == k2p) {
                    Some(&(_, ok)) => ok,
                    None => {
                        let ok = self.bm_k2_is(&e.k2);
                        k2_seen.push((k2p, ok));
                        ok
                    }
                };
                if !k2_ok {
                    continue;
                }
                let rest_ok = e.absent.iter().all(|n| match self.bm_find_cs(n) {
                    None => true,
                    Some(p) => self.bm_eq(p).hh().b0() == k::undefined_cs,
                }) && e.voids.iter().all(|&n| self.bm_box_value(n) == 0)
                    && e.kr.iter().all(|(r, v)| self.bm_reg_value(*r) == *v)
                    && e.kb
                        .iter()
                        .chain(&e.holes)
                        .all(|(n, v)| self.bm_box_key(self.bm_box_value(*n)) == *v)
                    && e.k3.iter().all(|(p, m)| self.bm_meaning_is(*p, m))
                    && e.kc.iter().all(|(n, f, pos)| {
                        let b = self.bm_box_value(*n);
                        b != 0
                            && self.bm_box_contents(b).is_ok_and(|(g, ps)| {
                                crate::boxfrag::frag_diff(f, &g).is_none() && ps == *pos
                            })
                    })
                    && e.obj_refs
                        .iter()
                        .all(|r| self.bm_obj_is(r.t, r.n) && self.bm_obj_dims(r.n) == r.dims)
                    && e.font_glues.iter().all(|(f, w)| {
                        let q = self.font_glue[*f as usize];
                        q > LO_MEM_STAT_MAX && self.bm_glue_fields(q) == *w
                    })
                    && self.bm_new_cs_placeable(&e.new_cs);
                if rest_ok && spanning {
                    fits.push((e.last_used, i));
                } else if rest_ok {
                    best = Some((e.last_used, i));
                } else if partial.is_none_or(|(u, _)| u < e.last_used) {
                    partial = Some((e.last_used, i));
                }
            }
            (best.map(|b| b.1), partial.map(|p| p.1), fits)
        });
        let (found, partial) = if spanning {
            match self.bm_span_pick(cs, fits) {
                Ok(i) => (Some(i), partial),
                Err(p) => (None, p.or(partial)),
            }
        } else {
            (found, partial)
        };
        let mut why = "NotRecorded";
        let doomed = found
            .is_some_and(|i| ST.with(|s| s.borrow().store.get(&cs).is_some_and(|v| v[i].doomed)));
        if doomed {
            return self.bm_miss("Doomed");
        }
        if let Some(i) = found {
            if m == Mode::Verify {
                // the full verifier, on the command line only. Its
                // admission checkpoint stays the base of the state the run
                // goes on from, so it cannot be dropped after the
                // comparison; in the host it was left among the restart
                // points with no page count (the verify sweep: "restart
                // point without a page count" on beamer-fragile, a compile
                // interrupted by the next edit), and its restore replaced
                // the edit's pending branch. The host verifies each
                // admitted call's operation log and output (`bm_verified`).
                // `FLASHTEX_BOXMEMO_FULL_VERIFY_HOST` (measurement only,
                // §17.12) runs it in the host anyway, restart points at risk.
                if WINDOW.with(|w| w.get()).is_none() || with_config(|c| c.full_in_host) {
                    if let Ok(ck) = self.checkpoint() {
                        FULL.with(|f| *f.borrow_mut() = Some(FullVerify { ck, warn }));
                    }
                }
                let (k2, k4) = (self.bm_k2(), k4_now);
                return self.bm_start(cs, call, spanning, scanner, k1, ctx, k2, k4, Some(i));
            }
            if let Some(t) = self.bm_replay(cs, i) {
                self.cur_tok = t;
                self.back_input();
            }
            return;
        }
        if let Some(i) = partial {
            why = "Meanings";
            if debug() {
                let (bad, regs) = ST.with(|s| {
                    let s = s.borrow();
                    let e = &s.store[&cs][i];
                    let bad: Vec<String> =
                        e.k3.iter()
                            .filter(|(p, m)| !self.bm_meaning_is(*p, m))
                            .take(4)
                            .map(|(p, m)| {
                                format!(
                                    "\\{} was {m:?} now {:?}",
                                    self.cs_name_string(*p),
                                    self.bm_meaning(*p)
                                )
                            })
                            .collect();
                    let regs: Vec<String> = e
                        .kr
                        .iter()
                        .filter(|(r, v)| self.bm_reg_value(*r) != *v)
                        .take(6)
                        .map(|(r, v)| format!("{r:?} was {v:?} now {:?}", self.bm_reg_value(*r)))
                        .collect();
                    (bad, regs)
                });
                eprintln!(
                    "boxmemo: \\{} key differs: {bad:?}; registers {regs:?}",
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
                        .map(|(p, v)| {
                            format!("{}={:?}", self.bm_loc_name(*p), &v[..v.len().min(3)])
                        })
                        .take(12)
                        .collect()
                })
            });
            if !diff.is_empty() {
                eprintln!(
                    "boxmemo: \\{} miss, key differs at {diff:?}",
                    self.cs_name_string(cs)
                );
            }
            LAST_KEY.with(|l| l.borrow_mut().insert(cs, now));
        }
        if WINDOW.with(|w| w.get()) == Some(false) && !self.bm_name_is(cs, record_anywhere()) {
            return;
        }
        let refused = ST.with(|s| s.borrow().aborts.get(&cs).copied().unwrap_or(0) >= ABORT_BUDGET);
        if refused {
            return;
        }
        let (k2, k4) = (self.bm_k2(), k4_now);
        self.bm_start(cs, call, spanning, scanner, k1, ctx, k2, k4, None);
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
        call: i32,
        spanning: bool,
        scanner: i32,
        k1: Vec<i32>,
        ctx: Context,
        k2: Vec<i32>,
        k4: Vec<i32>,
        verify: Option<usize>,
    ) {
        let start = self.bm_frame();
        let span = if spanning {
            // (`bm_span_offer` checked the level below the body)
            let f = self.input_ptr - 1;
            let fl = self.input_stack[f as usize];
            let index = fl.index_field;
            let (path, at, toks) = if fl.state_field == TOKEN_LIST {
                (String::new(), 0, Some((fl.start_field, fl.loc_field)))
            } else {
                let file = &mut self.input_file[(index - 1) as usize];
                let (Some(path), Some(at)) =
                    (file.opened_path().map(str::to_string), file.read_offset())
                else {
                    return self.bm_miss("SpanFile");
                };
                (path, at, None)
            };
            Some(SpanRec {
                f,
                index,
                file: SpanFile {
                    path,
                    lines: vec![],
                },
                toks,
                at,
                max_buf: self.max_buf_stack,
                graf: None,
                etex_aux: self.cur_list.eTeX_aux_field,
                line0: self.line,
                journal: crate::lineshift::journal_mark().0,
            })
        } else {
            None
        };
        let span_f = span.as_ref().map_or(-1, |s| s.f);
        SPAN_TOKS.with(|t| t.set(span.as_ref().is_some_and(|s| s.toks.is_some())));
        let rec = Rec {
            cs,
            call,
            span,
            obj_refs: vec![],
            new_objs: vec![],
            last_link: self.pdf_last_link,
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
            group_read: false,
            group0: (self.cur_level, self.cur_group),
            boxes: HashSet::new(),
            box_stack: vec![],
            gboxes: vec![],
            pending_box: None,
            alter: None,
            kb: vec![],
            kc: vec![],
            copies: vec![],
            copying: None,
            holes: vec![],
            ops: vec![],
            created: vec![],
            packed: false,
            verify,
        };
        // the macro's own meaning is part of K1; mark it seen
        let base = self.input_ptr;
        let size = (k::eqtb_top + 1) as usize;
        FAST.with(|f| {
            let mut f = f.borrow_mut();
            f.base = base;
            f.span_f = span_f;
            f.stamp = f.stamp.wrapping_add(1);
            if f.stamp == 0 || f.seen.len() < size {
                f.seen = vec![0; size];
                f.stamp = 1;
            }
            let st = f.stamp;
            f.seen[call as usize] = st;
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
                s.stale.insert(rec.cs);
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
                // (what the call read up to the refusal is its key: the same
                // reads take it to the same refusal)
                s.clock += 1;
                let now = s.clock;
                let k2hash = k2_hash_of(&rec.k2);
                let bytes = 4 * (rec.k1.len() + rec.k2.len()) + 64;
                let entry = Entry {
                    doomed: true,
                    hash: key_hash(&rec.k1, &rec.ctx, &[], &[]),
                    k2hash,
                    k1: rec.k1,
                    ctx: rec.ctx,
                    k2: s
                        .store
                        .get(&rec.cs)
                        .and_then(|v| {
                            v.iter()
                                .find(|e| e.k2hash == k2hash && *e.k2 == rec.k2[..])
                                .map(|e| e.k2.clone())
                        })
                        .unwrap_or_else(|| rec.k2.into()),
                    k4: rec.k4,
                    k3: rec.k3,
                    absent: rec.absent,
                    kr: rec.kr,
                    kb: rec.kb,
                    kc: rec.kc,
                    holes: rec.holes.iter().map(|h| (h.0, h.2.clone())).collect(),
                    frag: None,
                    aux: 0,
                    trailer: None,
                    voids: rec.voids,
                    line: (rec.line_read && rec.span.is_none()).then_some(self.line),
                    group: rec.group_read.then_some(rec.group0),
                    fast: vec![],
                    new_cs: vec![],
                    last_link: None,
                    obj_refs: rec.obj_refs,
                    new_objs: 0,
                    font_glues: vec![],
                    rs_locs: vec![],
                    ops: vec![],
                    last_badness: None,
                    // (a span's lines read up to the refusal: the same
                    // lines take it there again)
                    span: rec.span.map(|sp| {
                        Box::new(SpanEntry {
                            file: sp.file,
                            end: SpanEnd {
                                buf: vec![],
                                loc: 0,
                                limit: 0,
                                state: 0,
                                first: 0,
                                last: None,
                                max_buf: None,
                                max_buf0: 0,
                                line: 0,
                                journal: vec![],
                            },
                            graf: None,
                            pos: vec![],
                            copies: vec![],
                            toks: None,
                        })
                    }),
                    last_used: now,
                    hits: 0,
                    bytes,
                };
                insert_entry(&mut s, rec.cs, entry);
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

    /// `get_next` popped a used-up token list, during a recording. When it
    /// was the last of the call's levels and `big_switch`'s own
    /// `get_x_token` popped it -- `bm_at_switch`, no `expand` in progress,
    /// no macro's parameters being scanned -- the call is over: the token
    /// `get_next` reads next is the first after it, which the next command
    /// reads as it would after a replay. The recording ends here, before
    /// that token is read (`bm_finish`'s checks as at `big_switch`).
    ///
    /// Without this, a body whose last tokens only expand (LaTeX's `\end`:
    /// `\if@endpe ... \fi`, the `env/.../after` hook, `\if@ignore ...
    /// \fi` after its last `\endgroup`) was used up inside that
    /// `get_x_token`, not at `big_switch`, and the read of the next token
    /// abandoned it (`Level`): every frame on Infinite Descent (ntheorem's
    /// `*@framecommand`, a TikZ picture ending in `\end{tikzpicture}`),
    /// about a third of an edited page's instructions there (lane
    /// EDITED-PAGE). A read past the call anywhere else -- a primitive's
    /// look-ahead, a macro's parameters, an expansion's scan -- still
    /// abandons it (`flashtex_bm_next`).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_popped(&mut self) {
        // (a span reads its file level below `base` and ends at its
        // environment's end, §17)
        let Some(base) = self
            .bm_with_rec(|r| r.span.is_none().then_some(r.base))
            .flatten()
        else {
            return;
        };
        if self.input_ptr < base
            && self.bm_at_switch
            && self.expand_depth_count == 0
            && self.scanner_status == k::normal
        {
            self.bm_finish(None);
        }
    }

    /// The end of `get_next`, during a recording.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_next(&mut self) {
        let (base, span_f) = FAST.with(|f| {
            let f = f.borrow();
            (f.base, f.span_f)
        });
        if self.input_ptr < base {
            // a span reads its file level (§17): its lines are the key's
            if self.input_ptr != span_f
                || (self.cur_input.state_field == TOKEN_LIST) != SPAN_TOKS.with(|t| t.get())
            {
                return self.bm_abort("Level");
            }
        } else if self.cur_input.state_field != TOKEN_LIST {
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
            // the name leads to `p` (part of the key), and `p`'s meaning;
            // not for a name the call entered itself (`Op::NewCs`: an offer
            // checks that it is absent and lands at `p`)
            let name: Vec<i32> = (j..j + l).map(|i| self.buffer[i as usize]).collect();
            self.bm_with_rec(|r| {
                if !r.created.contains(&p)
                    && !r
                        .k3
                        .iter()
                        .any(|(q, m)| *q == p && matches!(m, Meaning::Name(n) if *n == name))
                {
                    r.k3.push((p, Meaning::Name(name)));
                }
            });
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
        self.bm_with_rec(|r| {
            r.box_stack.push(r.boxes.clone());
            r.ops.push(Op::Begin(c));
        });
    }

    /// The box registers the `unsave` about to run puts back (tex.web §282,
    /// e-TeX's `sa_restore`): those the group saved, unless their value is
    /// now a global one ("retaining").
    fn bm_restored_boxes(&self) -> Vec<i32> {
        let mut v = vec![];
        let mut j = self.save_ptr - 1;
        while j >= 0 {
            let w = self.save_stack[j as usize].hh();
            let t = w.b0();
            if t == k::level_boundary {
                break;
            }
            let p = w.rh();
            if (t == k::restore_old_value || t == k::restore_zero)
                && (BOX_BASE..BOX_BASE + 256).contains(&p)
                && self.bm_eq(p).hh().b1() != k::level_one
            {
                v.push(p - BOX_BASE);
            }
            j -= if t == k::restore_old_value { 2 } else { 1 };
        }
        if self.sa_level == self.cur_level {
            let mut c = self.sa_chain;
            while c != 0 {
                let q = self.mem[(c + 1) as usize].hh().lh();
                let h = self.mem[q as usize].hh();
                if h.b0() / 16 == BOX_VAL && h.b1() != k::level_one {
                    v.push(self.bm_sa_number(q));
                }
                c = self.bm_link(c);
            }
        }
        v
    }

    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_unsave(&mut self) {
        let level = self.bm_with_rec(|r| r.start.cur_level).unwrap_or(0);
        if self.cur_level <= level {
            return self.bm_abort("Group");
        }
        // A register the group saved gets back the box it held where the
        // group began, which is the call's own only if it was then: a box
        // the call made in the group and read after it would otherwise be
        // taken for the call's, and the register's box of before the call
        // read unkeyed.
        let restored = self.bm_restored_boxes();
        self.bm_with_rec(|r| {
            let before = r.box_stack.pop().unwrap_or_default();
            for n in restored {
                r.boxes.retain(|&(m, _)| m != n);
                r.boxes.extend(before.iter().filter(|&&(m, _)| m == n));
            }
            r.ops.push(Op::End)
        });
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
                let mut src = self.bm_glue_source(e);
                // `\skip49=\lastskip` (beamer's `\reset@color`): the last
                // glue of the current list, which is a list the call made
                // (`flashtex_bm_internal` refuses the call's outer one), so
                // only the call's own nodes share the specification. Made
                // fresh by the replay, it is shared as the normal path
                // shares it from then on: a node the call's output keeps
                // takes it from the register (`boxfrag`'s `Spec::Eqtb`) or
                // as its own.
                if src.is_none() && self.cur_cmd == k::last_item && self.cur_chr == GLUE_VAL {
                    src = Some(Val::FreshGlue(self.bm_glue_fields(e)));
                }
                if src.is_none() && debug() {
                    eprintln!(
                        "boxmemo: {} gets a shared glue specification ({e}, {} more references) of no single register",
                        self.bm_loc_name(p),
                        self.bm_link(e)
                    );
                }
                return src.ok_or("SharedGlue");
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
        if found.is_empty() {
            for f in 0..=self.font_ptr {
                if self.font_glue[f as usize] == e {
                    found.push(Val::GlueFrom(0, f));
                }
            }
        }
        if found.len() > 1 {
            // (several registers hold it, as `\z@skip` and another after an
            // assignment between them: the one the recording read is the
            // source `scan_glue` took it from, as it is in LaTeX's
            // `\parskip\z@skip\relax`, where the current token is past it)
            let read: Vec<RegId> = self
                .bm_with_rec(|r| r.kr.iter().map(|x| x.0).collect())
                .unwrap_or_default();
            found.retain(|v| match v {
                Val::GlueFrom(p, 0) if *p > 0 => read.contains(&RegId::Eqtb(*p)),
                Val::GlueFrom(t, n) if *t < 0 => read.contains(&RegId::Sa(-*t, *n)),
                _ => false,
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
        // A local assignment at the call's own level outlives the call (to
        // the end of the group it was called in): an effect like a global
        // one, which the replay's `eq_define` at the same level makes again
        // with the same save-stack entry (LaTeX's `\begin` defines
        // `\reserved@a` before its `\begingroup`). A box assigned there is
        // logged by its node, taken by value when the call ends (§16).
        let outer = !global && self.cur_level == level;
        if outer && (BOX_BASE..BOX_BASE + 256).contains(&p) {
            if t != BOX_REF {
                return self.bm_abort("OuterBox");
            }
            self.bm_with_rec(|r| r.boxes.insert((p - BOX_BASE, e)));
            if !self.bm_rec_on {
                return;
            }
            let v = if e == 0 {
                Val::Plain(0)
            } else {
                Val::BoxNode(e)
            };
            self.bm_with_rec(|r| r.ops.push(Op::Def { p, t, v, kind }));
            return;
        }
        if (BOX_BASE..BOX_BASE + 256).contains(&p) {
            // A box made inside the call stays inside it (its local
            // assignments are undone by its groups); a global one may only
            // make the register void (TikZ's `\tikz@figbox`), replayed.
            if global {
                // (a copy made before it is made again from the box after
                // the void: refused, as a BoxDim after a copy is)
                if self
                    .bm_with_rec(|r| r.copies.iter().any(|c| c.0 == p - BOX_BASE))
                    .unwrap_or(false)
                {
                    return self.bm_abort("BoxContent");
                }
                self.bm_with_rec(|r| {
                    if !r.gboxes.contains(&(p - BOX_BASE)) {
                        r.gboxes.push(p - BOX_BASE)
                    }
                });
            }
            self.bm_with_rec(|r| r.boxes.insert((p - BOX_BASE, e)));
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
        if global || outer {
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
        let outer = !global && self.cur_level == level;
        let t = self.mem[p as usize].hh().b0() / 16;
        let n = if t <= DIMEN_VAL {
            self.bm_link(p + 1)
        } else {
            self.bm_sa_number(p)
        };
        if t == BOX_VAL {
            if outer {
                return self.bm_abort("OuterBox");
            }
            if global {
                // (a copy made before it is made again from the box after
                // the void: refused, as a BoxDim after a copy is)
                if self
                    .bm_with_rec(|r| r.copies.iter().any(|c| c.0 == n))
                    .unwrap_or(false)
                {
                    return self.bm_abort("BoxContent");
                }
                self.bm_with_rec(|r| {
                    if !r.gboxes.contains(&n) {
                        r.gboxes.push(n)
                    }
                });
            }
            self.bm_with_rec(|r| r.boxes.insert((n, e)));
            return;
        }
        if global || outer {
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
        let pending = self
            .bm_with_rec(|r| r.pending_box.is_some())
            .unwrap_or(false);
        if pending {
            // the previous fetch was not a dimension, a test or a move: the
            // box's contents were read (`\copy`, `\unhbox`, `\vsplit`, ...)
            return self.bm_abort("BoxContent");
        }
        let p = self.bm_box_value(n);
        let written = self
            .bm_with_rec(|r| r.boxes.contains(&(n, p)))
            .unwrap_or(false);
        if written {
            return;
        }
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

    /// `copy_node_list` made `c`, a `\copy` of box register `n`, during a
    /// recording: of a foreign one a span copies, kept (§17.6).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_copied(&mut self, n: i32, c: i32) {
        self.bm_with_rec(|r| {
            if r.copying.take() == Some(n) {
                r.copies.push((n, c));
            }
        });
    }

    /// `alter_box_dimen` scanned the value `v` for word `c` of a box node,
    /// during a recording: of a foreign register's box (`r.alter`), an
    /// operation (§17.6).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_box_dimen(&mut self, c: i32, v: i32) {
        self.bm_with_rec(|r| {
            if let Some(n) = r.alter.take() {
                r.ops.push(Op::BoxDim { n, c, v });
            }
        });
    }

    /// The box register a recording just fetched is used: `k` 1 for a
    /// dimension (`\wd` & co.), 2 for a test (`\ifvoid`, `\ifhbox`), 3 for a
    /// move (`\box`, which voids the register), 4 a copy, 5 a dimension
    /// assigned.
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_box_use(&mut self, n: i32, k: i32) {
        let Some(Some((m, p))) = self.bm_with_rec(|r| r.pending_box.take()) else {
            return;
        };
        if m != n {
            return self.bm_abort("BoxContent");
        }
        if k == 5 {
            // `\ht n=..` & co. on a box the call did not make: a span keys
            // the box as it is now and replays the assignment
            // (`Op::BoxDim`, §17.6)
            if !self.bm_with_rec(|r| r.span.is_some()).unwrap_or(false) {
                return self.bm_abort("BoxContent");
            }
            // (a copy made before it is made again from the box after it)
            if self
                .bm_with_rec(|r| r.copies.iter().any(|c| c.0 == n))
                .unwrap_or(false)
            {
                return self.bm_abort("BoxContent");
            }
            let key = self.bm_box_key(p);
            self.bm_with_rec(|r| {
                if !r.kb.iter().any(|(x, _)| *x == n) {
                    r.kb.push((n, key));
                }
                r.alter = Some(n);
            });
            return;
        }
        if k == 4 {
            // `\copy` of a box the call did not make: a span keys the box
            // by its contents (amsmath's `\Mathstrut@` copies
            // `\Mathstrutbox@` into every pgfplots label, §17.6)
            if !self.bm_with_rec(|r| r.span.is_some()).unwrap_or(false) {
                return self.bm_abort("BoxContent");
            }
            self.bm_with_rec(|r| r.copying = Some(n));
            let known = self
                .bm_with_rec(|r| r.kc.iter().any(|(x, _, _)| *x == n))
                .unwrap_or(false);
            if known {
                return;
            }
            let key = match self.bm_box_contents(p) {
                Ok(key) => key,
                Err(w) => return self.bm_abort(w),
            };
            self.bm_with_rec(|r| r.kc.push((n, key.0, key.1)));
            return;
        }
        let key = self.bm_box_key(p);
        self.bm_with_rec(|r| {
            if k == 3 {
                r.holes.push((n, p, key));
                // void now, and the call's own from here on
                r.boxes.insert((n, 0));
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

    /// Box `p` (a register's) by value, with its nodes' source positions:
    /// what a `\copy` of it reads (§17.6).
    fn bm_box_contents(&self, p: i32) -> Result<(crate::boxfrag::Frag, Positions), &'static str> {
        let f = self.frag_take(p, self.bm_link(p), &[], &[])?;
        let pos = if crate::displaylist::enabled() {
            let mut at = vec![];
            self.bm_nodes_one(p, &mut at);
            at.iter().map(|&q| self.dl_side_pos(q)).collect()
        } else {
            vec![]
        };
        Ok((f, pos))
    }

    /// Node `p` and what it holds, in `bm_nodes`'s order.
    fn bm_nodes_one(&self, p: i32, out: &mut Vec<i32>) {
        let next = self.bm_link(p);
        // (`bm_nodes` on a list of `p` alone)
        let mut v = vec![];
        self.bm_nodes(p, &[], &mut v);
        let stop = if next == 0 {
            v.len()
        } else {
            v.iter().position(|&q| q == next).unwrap_or(v.len())
        };
        out.extend_from_slice(&v[..stop]);
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
                // \currentgrouplevel, \currentgrouptype: the call's level
                // and group join the key (inside its own groups the call
                // makes both)
                21 | 22 => {
                    self.bm_with_rec(|r| r.group_read = true);
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
        // `\\ignorespaces` as the body's last command reads the tokens after
        // the call: the recording ends before it runs (`bm_finish`'s
        // trailer), and a replay runs it again
        if c == k::ignore_spaces && chr == 0 {
            let (base, span, level) = self
                .bm_with_rec(|r| (r.base, r.span.is_some(), r.start.cur_level))
                .unwrap_or((i32::MAX, false, 0));
            if base != i32::MAX && self.bm_exhausted(base) && (!span || self.cur_level == level) {
                let tok = self.cur_tok;
                return self.bm_finish(Some(tok));
            }
        }
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
                // a link: its object is made again by a replay; an
                // existing form or image: part of the key
                k::pdf_start_link_node,
                k::pdf_end_link_node,
                k::pdf_refxform_node,
                k::pdf_refximage_node,
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

    /// The save-stack entries a call left at its own level: only those of
    /// local assignments (`eq_save`'s `restore_old_value` and
    /// `restore_zero`, e-TeX's `restore_sa`), not an `\\aftergroup` token
    /// or a group boundary. Walked down from the top as `unsave` pops them.
    fn bm_saves_only(&self, lo: i32, hi: i32) -> bool {
        let mut k = hi - 1;
        while k >= lo {
            let w = self.save_stack[k as usize].hh();
            match w.b0() {
                t if t == k::restore_old_value => k -= 2,
                t if t == k::restore_zero || t == k::restore_sa => k -= 1,
                _ => return false,
            }
        }
        k == lo - 1
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

    /// `big_switch`, during a recording: finish it when the body is done --
    /// a span's when its group is closed again too (§17: until then the
    /// call reads on from its file).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_switch(&mut self) {
        let Some((base, span, level)) =
            self.bm_with_rec(|r| (r.base, r.span.is_some(), r.start.cur_level))
        else {
            self.bm_rec_on = false;
            return;
        };
        if !self.bm_exhausted(base) || (span && self.cur_level != level) {
            return;
        }
        self.bm_finish(None);
    }

    /// `input_ln` read a line of the current file level (tex.web §362),
    /// during a recording: a span's file level's line is kept (§17).
    /// (A file level above it is refused at its first token.)
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_line_read(&mut self) {
        let Some(Some((f, at))) = self.bm_with_rec(|r| r.span.as_ref().map(|s| (s.f, s.at))) else {
            return;
        };
        if self.input_ptr != f {
            return;
        }
        let index = self.cur_input.index_field;
        let file = &mut self.input_file[(index - 1) as usize];
        let (Some(line), Some(off)) = (file.read_line().map(<[u8]>::to_vec), file.read_offset())
        else {
            return self.bm_abort("SpanFile");
        };
        self.bm_with_rec(|r| {
            if let Some(s) = r.span.as_mut() {
                s.file.lines.push((line, off - at));
            }
        });
    }

    /// The recording is done: its body is used up, and `trailer` is the
    /// token of a last command that reads past the body (an `\\ignorespaces`
    /// that `main_control` is about to run), left out of the recording and
    /// run again after a replay.
    fn bm_finish(&mut self, trailer: Option<i32>) {
        let end = self.bm_frame();
        let (start, scanner) = self.bm_with_rec(|r| (r.start.clone(), r.scanner)).unwrap();
        // A span (§17): its file level must still be there; begun in
        // vertical mode inside a box, it may leave a paragraph begun (§17.3)
        let (span_f, start_aux, start_etex, span_toks) = self
            .bm_with_rec(|r| {
                (
                    r.span.as_ref().map(|s| (s.f, s.index, s.etex_aux)),
                    r.ctx.aux,
                    r.span.as_ref().map_or(0, |s| s.etex_aux),
                    r.span.as_ref().and_then(|s| s.toks),
                )
            })
            .unwrap();
        let span_f = span_f.map(|(f, i, _)| (f, i));
        let span_lost = span_f.is_some_and(|(f, index)| {
            if self.input_ptr < f {
                return true;
            }
            let l = if self.input_ptr == f {
                self.cur_input
            } else {
                self.input_stack[f as usize]
            };
            (l.state_field == TOKEN_LIST) != span_toks.is_some()
                || l.index_field != index
                || span_toks.is_some_and(|(list, _)| l.start_field != list)
        });
        // (on the main vertical list the page builder ran where the
        // paragraph began: the page is as it left it, and so are the
        // parameters it read)
        let page_then = self
            .bm_with_rec(|r| {
                r.span
                    .as_ref()
                    .and_then(|s| s.graf.as_ref())
                    .and_then(|g| g.page.clone())
            })
            .flatten()
            .filter(|(_, params)| *params == self.bm_page_params())
            .map(|(page, _)| page);
        let graf = span_f.is_some()
            && start.mode.abs() == k::vmode
            && (start.nest_ptr > 0 || page_then.is_some())
            && end.nest_ptr == start.nest_ptr + 1
            && end.mode == k::hmode
            && self.cur_list.eTeX_aux_field == 0
            && self.nest[start.nest_ptr as usize].aux_field.0 == start_aux;
        let why = if span_lost {
            Some("SpanFile")
        } else if end.cur_level != start.cur_level
            || end.save_ptr < start.save_ptr
            || !self.bm_saves_only(start.save_ptr, end.save_ptr)
        {
            Some("Group")
        } else if end.cond != start.cond {
            Some("Cond")
        } else if end.align_state != start.align_state {
            Some("Unbalanced")
        } else if end.strings != start.strings && !self.bm_strings_are_new_names(&start, &end) {
            Some("NewCs")
        } else if end.errors != start.errors {
            Some("Error")
        } else if self.after_token != 0 || self.scanner_status != scanner {
            Some("State")
        } else if (end.nest_ptr != start.nest_ptr || end.mode != start.mode) && !graf {
            Some("OuterList")
        } else if end.tail != start.tail
            && start.mode != -k::hmode
            && !(span_f.is_some()
                && (start.mode.abs() == k::hmode || start.mode.abs() == k::mmode || graf))
        {
            // only in restricted horizontal mode may a call append: no
            // paragraph, no page builder (a span also in a paragraph or in
            // math, and one that begins a paragraph, §17.3)
            Some("OuterList")
        } else if span_f.is_some()
            && start.mode.abs() == k::mmode
            && (self.cur_list.aux_field.0 != start_aux
                || self.cur_list.eTeX_aux_field != start_etex)
        {
            // (in math the list's `aux` is `incompleat_noad` and its
            // `eTeX_aux` the display's box or `delim_ptr`: pointers, which a
            // replay cannot make again; the call must leave them alone)
            Some("OuterList")
        } else if self
            .bm_with_rec(|r| r.pending_box.is_some())
            .unwrap_or(false)
        {
            Some("BoxContent")
        } else if end.page != *page_then.as_ref().filter(|_| graf).unwrap_or(&start.page) {
            Some("Page")
        } else if end.fonts != start.fonts || end.font_glue != start.font_glue {
            Some("Fonts")
        } else if end.marks != start.marks {
            Some("Marks")
        } else if !self
            .bm_with_rec(|r| {
                // only the links' objects, made one after the other: counted
                // by `sys_obj_ptr`, which every object moves. `obj_ptr` is
                // behind it after an object stream's number was taken (a
                // shipout's `pdf_os_prepare_obj`, before the call), and the
                // call's first object makes them equal again, as the replay's
                // `pdf_new_objnum` does; with none, it stays where it was.
                let ((p0, s0), (p1, s1)) = (start.obj_ptr, end.obj_ptr);
                r.new_objs.len() as i32 == s1 - s0
                    && r.new_objs.iter().zip(s0 + 1..).all(|(&o, n)| o == n)
                    && p1 == if r.new_objs.is_empty() { p0 } else { s1 }
                    && (self.pdf_last_link == r.last_link
                        || r.new_objs.contains(&self.pdf_last_link))
            })
            .unwrap_or(false)
        {
            Some("Objects")
        } else if end.interaction != start.interaction || end.arm != start.arm {
            Some("State")
        } else if end.outputs != start.outputs {
            Some("Output")
        } else if end.lines != start.lines
            && (span_f.is_none()
                || end.lines.1 != start.lines.1
                || end.lines.2 != start.lines.2
                || self
                    .bm_with_rec(|r| r.span.as_ref().map(|s| (s.f, s.line0, s.journal)))
                    .flatten()
                    .and_then(|(f, l, m)| self.bm_span_journal(f, l, m))
                    .is_none())
        {
            // (a span's `\inputlineno` reads of its own lines are made
            // again by its replay, §17.4)
            Some("LineShift")
        } else {
            None
        };
        if let Some(w) = why {
            return self.bm_abort(w);
        }
        let gboxes = self.bm_with_rec(|r| r.gboxes.clone()).unwrap_or_default();
        if gboxes.iter().any(|&n| self.bm_box_value(n) != 0) {
            if debug() {
                let left: Vec<i32> = gboxes
                    .iter()
                    .copied()
                    .filter(|&n| self.bm_box_value(n) != 0)
                    .collect();
                eprintln!("boxmemo: global boxes left: {left:?}");
            }
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
        // (a span's foreign copies are holes too, §17.6)
        if let Err(w) = self.bm_outer_boxes(
            &mut rec.ops,
            rec.holes.is_empty() && rec.new_objs.is_empty() && rec.copies.is_empty(),
        ) {
            return self.bm_abort_taken(rec, w);
        }
        // a span's paragraph (§17.3): the paragraph skip appended to the
        // box's list, and the paragraph's own list, with the holes
        let mut span_graf = None;
        let mut pos: Positions = vec![];
        let dl = crate::displaylist::enabled();
        // (a span's `\copy`s of foreign boxes, and the SyncTeX tag of its
        // file level, which every other node it made carries, §17.6)
        let copied: Vec<i32> = rec.copies.iter().map(|c| c.1).collect();
        let span_tag = rec.span.as_ref().map(|s| {
            if self.input_ptr == s.f {
                self.cur_input.synctex_tag_field
            } else {
                self.input_stack[s.f as usize].synctex_tag_field
            }
        });
        if graf {
            let holes: Vec<i32> = rec.holes.iter().map(|h| h.1).collect();
            let skip: Vec<i32> = holes.iter().chain(&copied).copied().collect();
            let note = rec.span.as_mut().and_then(|s| s.graf.take());
            let outer_h: i64 = note.as_ref().map_or(0, |n| {
                n.glues
                    .iter()
                    .map(|g| {
                        (crate::generated::types::memory_word(g.spec_words[1]).int() as i64).max(0)
                    })
                    .sum()
            });
            // the paragraph skips: on the outer list still (a box's), or
            // as the main vertical list's noted them
            let outer = if start.nest_ptr > 0 {
                let first = self.bm_link(start.tail);
                let mut q = first;
                while q != 0 {
                    if q >= self.hi_mem_min || self.mem[q as usize].hh().b0() != k::glue_node {
                        return self.bm_abort_taken(rec, "OuterList");
                    }
                    q = self.bm_link(q);
                }
                if dl {
                    let mut at = vec![];
                    self.bm_nodes(first, &[], &mut at);
                    pos = at.iter().map(|&p| self.dl_side_pos(p)).collect();
                }
                match self.frag_take(first, 0, &[], &rec.new_objs) {
                    Ok(f) => f,
                    Err(w) => return self.bm_abort_taken(rec, w),
                }
            } else {
                let Some(note) = note else {
                    return self.bm_abort_taken(rec, "OuterList");
                };
                if dl {
                    pos = note.glues.iter().map(|g| g.pos).collect();
                }
                match self.bm_graf_glues(&note.glues) {
                    Ok(f) => f,
                    Err(w) => return self.bm_abort_taken(rec, w),
                }
            };
            let head = self.cur_list.head_field;
            let inner = match self.frag_take_span(
                self.bm_link(head),
                0,
                &holes,
                &copied,
                &rec.new_objs,
                span_tag,
            ) {
                Ok(f) => f,
                Err(w) => return self.bm_abort_taken(rec, w),
            };
            if dl {
                let mut at = vec![head];
                self.bm_nodes(self.bm_link(head), &skip, &mut at);
                pos.extend(at.iter().map(|&p| self.dl_side_pos(p)));
            }
            span_graf = Some(Graf {
                outer,
                inner,
                ml: self.cur_list.ml_field,
                pg: self.cur_list.pg_field,
                aux: self.cur_list.aux_field.0,
                outer_h,
            });
        }
        // a draw call's output, with its holes
        let mut frag = None;
        if !graf && (end.tail != start.tail || !rec.holes.is_empty()) {
            let holes: Vec<i32> = rec.holes.iter().map(|h| h.1).collect();
            if end.tail == start.tail {
                return self.bm_abort_taken(rec, "Hole");
            }
            let first = self.bm_link(start.tail);
            let skip: Vec<i32> = holes.iter().chain(&copied).copied().collect();
            match self.frag_take_span(first, 0, &holes, &copied, &rec.new_objs, span_tag) {
                Ok(f) if self.cur_list.tail_field == end.tail => frag = Some(f),
                Ok(_) => return self.bm_abort_taken(rec, "Tail"),
                Err(w) => return self.bm_abort_taken(rec, w),
            }
            if dl && rec.span.is_some() {
                let mut at = vec![];
                self.bm_nodes(first, &skip, &mut at);
                pos = at.iter().map(|&p| self.dl_side_pos(p)).collect();
            }
        }
        let aux = self.cur_list.aux_field.0;
        let last_badness = rec.packed.then_some(self.last_badness);
        let span_read = match rec.span.as_ref() {
            Some(s) => match self.bm_span_toks(s) {
                Ok(t) => t,
                Err(w) => return self.bm_abort_taken(rec, w),
            },
            None => None,
        };
        let span = rec.span.as_ref().map(|s| {
            Box::new(SpanEntry {
                file: s.file.clone(),
                end: self.bm_span_end(s),
                graf: span_graf,
                pos,
                copies: rec.copies.iter().map(|c| c.0).collect(),
                toks: span_read,
            })
        });
        if let Some(i) = rec.verify {
            let (cs, scanner) = (rec.cs, rec.scanner);
            self.bm_verified(rec, i, last_badness, &frag, trailer, &span);
            return self.bm_verify_full(cs, i, scanner);
        }
        self.bm_commit(rec, last_badness, frag, aux, trailer, span);
    }

    /// The boxes the call left in registers at its own level
    /// (`Val::BoxNode`, §16), taken by value. Each such register's last
    /// assignment must be the box it holds now (nothing moved it out or
    /// replaced it since), and the box must be the call's alone: no hole
    /// (`alone`: the call moved no foreign box and made no object), and
    /// nothing in it shared with the state ([`crate::boxfrag::self_contained`]),
    /// so that it can be rebuilt where its assignment is replayed. Every
    /// earlier assignment of the register at that level is replayed with the
    /// same box: the next one at the level destroys it (`eq_destroy`) on both
    /// paths, and a fresh box is never "reassigning".
    fn bm_outer_boxes(&self, ops: &mut [Op], alone: bool) -> Result<(), &'static str> {
        let mut regs: Vec<i32> = ops
            .iter()
            .filter_map(|o| match o {
                Op::Def {
                    p,
                    v: Val::BoxNode(_),
                    ..
                } => Some(*p),
                _ => None,
            })
            .collect();
        if regs.is_empty() {
            return Ok(());
        }
        if !alone {
            return Err("OuterBox");
        }
        regs.sort_unstable();
        regs.dedup();
        for p in regs {
            let last = ops
                .iter()
                .rposition(|o| matches!(o, Op::Def { p: q, .. } if *q == p))
                .expect("a box assignment");
            let Op::Def {
                v: Val::BoxNode(e), ..
            } = &ops[last]
            else {
                return Err("OuterBox");
            };
            let e = *e;
            if self.bm_eq(p).hh().rh() != e {
                return Err("OuterBox");
            }
            let f = self.frag_take(e, self.bm_link(e), &[], &[])?;
            if f.nodes.len() != 1 || !crate::boxfrag::self_contained(&f) {
                return Err("OuterBoxShared");
            }
            let f = std::rc::Rc::new(f);
            for o in ops.iter_mut() {
                if let Op::Def { p: q, v, .. } = o {
                    if *q == p && matches!(v, Val::BoxNode(_)) {
                        *v = Val::Box(f.clone());
                    }
                }
            }
        }
        Ok(())
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
        last_badness: Option<i32>,
        frag: Option<crate::boxfrag::Frag>,
        aux: u64,
        trailer: Option<i32>,
        span: Option<Box<SpanEntry>>,
    ) {
        // (a span's `\inputlineno` reads are of the lines its key holds)
        let line = self.line;
        let line_read = rec.line_read && span.is_none();
        let fast = if span.is_some() {
            let t0 = std::time::Instant::now();
            let f = compress_groups_with(&new_cs_first(&rec.ops), true);
            if debug() {
                eprintln!(
                    "boxmemo: span log of {} operations compressed in {:?}",
                    rec.ops.len(),
                    t0.elapsed()
                );
            }
            f
        } else {
            compress_groups(&rec.ops)
        };
        // A span keeps its whole log only to be verified (§17.5): a
        // pgfplots axis makes some 10^5 to 10^6 operations.
        let keep_ops = span.is_none() || mode() == Mode::Verify;
        let ops_cost = if keep_ops { ops_bytes(&rec.ops) } else { 0 };
        let span_bytes = span.as_ref().map_or(0, |s| {
            s.file.lines.iter().map(|l| 40 + l.0.len()).sum::<usize>()
                + 4 * s.end.buf.len()
                + 16 * s.pos.len()
                + s.graf
                    .as_ref()
                    .map_or(0, |g| g.outer.bytes() + g.inner.bytes() + 64)
        });
        let mut bytes = span_bytes
            + frag
                .as_ref()
                .filter(|_| span.is_some())
                .map_or(0, |f| f.bytes())
            + 4 * (rec.k1.len() + rec.k2.len() + rec.k4.len())
            // (a macro's tokens are shared: `intern_tokens` counts them once)
            + 16 * rec.k3.len()
            + ops_cost
            + if span.is_some() { ops_bytes(&fast) } else { 0 };
        let name = if debug() {
            Some(self.cs_name_string(rec.cs))
        } else {
            None
        };
        // (the size it has where it is not verified)
        if span.is_some() && bytes - ops_cost > MAX_SPAN_BYTES {
            if let Some(n) = &name {
                eprintln!(
                    "boxmemo: span \\{n} too large to keep: {} bytes, {} operations replayed",
                    bytes - ops_cost,
                    fast.len()
                );
            }
            return self.bm_abort_taken(rec, "Large");
        }
        ST.with(|s| {
            let mut s = s.borrow_mut();
            s.clock += 1;
            let now = s.clock;
            s.aborts.remove(&rec.cs);
            let k2hash = k2_hash_of(&rec.k2);
            let k2: std::rc::Rc<[i32]> = s
                .store
                .get(&rec.cs)
                .and_then(|v| {
                    v.iter()
                        .find(|e| e.k2hash == k2hash && *e.k2 == rec.k2[..])
                        .map(|e| e.k2.clone())
                })
                .inspect(|k2| bytes -= 4 * k2.len())
                .unwrap_or_else(|| rec.k2.into());
            let font_glues: Vec<(i32, [i32; 5])> = {
                let mut fs: Vec<i32> = frag.as_ref().map(|f| f.fonts.clone()).unwrap_or_default();
                for o in &rec.ops {
                    if let Op::Def {
                        v: Val::GlueFrom(0, f),
                        ..
                    }
                    | Op::Sa {
                        v: Val::GlueFrom(0, f),
                        ..
                    } = o
                    {
                        if !fs.contains(f) {
                            fs.push(*f);
                        }
                    }
                }
                fs.into_iter()
                    .map(|f| (f, self.bm_glue_fields(self.font_glue[f as usize])))
                    .collect()
            };
            let rs_locs = {
                let mut v: Vec<i32> = rec.k3.iter().map(|(p, _)| *p).collect();
                v.push(rec.call);
                for o in &rec.ops {
                    if let Op::Def { p, v: val, .. } = o {
                        v.push(*p);
                        if let Val::LetCs(s) = val {
                            v.push(*s);
                        }
                    }
                }
                v.sort_unstable();
                v.dedup();
                // (the ones it entered: the replay's `id_lookup` tells the
                // read-set, as the normal path's did, once they exist)
                v.retain(|p| !rec.created.contains(p));
                v
            };
            let entry = Entry {
                doomed: false,
                hash: key_hash(&rec.k1, &rec.ctx, &[], &[]),
                k2hash,
                k1: rec.k1,
                ctx: rec.ctx,
                k2,
                k4: rec.k4,
                k3: rec.k3,
                absent: rec.absent,
                kr: rec.kr,
                kb: rec.kb,
                kc: rec.kc,
                holes: rec.holes.iter().map(|h| (h.0, h.2.clone())).collect(),
                frag,
                aux,
                trailer,
                voids: rec.voids,
                line: line_read.then_some(line),
                group: rec.group_read.then_some(rec.group0),
                fast,
                new_cs: rec
                    .ops
                    .iter()
                    .filter_map(|o| match o {
                        Op::NewCs { p, name } => Some((name.clone(), *p)),
                        _ => None,
                    })
                    .collect(),
                last_link: (self.pdf_last_link != rec.last_link)
                    .then(|| rec.new_objs.iter().position(|&o| o == self.pdf_last_link))
                    .flatten(),
                obj_refs: rec.obj_refs,
                new_objs: rec.new_objs.len(),
                font_glues,
                rs_locs,
                ops: if keep_ops { rec.ops } else { vec![] },
                last_badness,
                span,
                last_used: now,
                hits: 0,
                bytes,
            };
            if let Some(n) = &name {
                // (what the replay makes: groups kept, assignments at the
                // call's level, and those from the state)
                let mut depth = 0;
                let (mut groups, mut top, mut state) = (0, 0, 0);
                for o in &entry.fast {
                    match o {
                        Op::Begin(_) => {
                            groups += usize::from(depth == 0);
                            depth += 1;
                        }
                        Op::End => depth -= 1,
                        Op::Def { v, .. } => {
                            top += usize::from(depth <= 0);
                            state += usize::from(!by_value(v));
                        }
                        _ => {}
                    }
                }
                eprintln!(
                    "boxmemo: recorded \\{n}: {} ops ({} replayed: {groups} groups kept, {top} at the call's level, {state} from the state), {} meanings, {} bytes",
                    entry.ops.len(),
                    entry.fast.len(),
                    entry.k3.len(),
                    bytes
                );
            }
            if name.is_some() && entry.span.is_some() {
                let (mut newcs, mut glob, mut loc, mut grp, mut toks) = (0, 0, 0, 0, 0usize);
                let mut locs = HashSet::new();
                for o in &entry.fast {
                    match o {
                        Op::NewCs { .. } => newcs += 1,
                        Op::Begin(_) => grp += 1,
                        Op::Def { p, kind, v, .. } => {
                            locs.insert(*p);
                            if *kind >= 2 { glob += 1 } else { loc += 1 }
                            if let Val::FreshList(l) | Val::Shared(_, l) = v { toks += l.len() }
                        }
                        Op::Sa { kind, .. } => {
                            if *kind >= 2 { glob += 1 } else { loc += 1 }
                        }
                        _ => {}
                    }
                }
                eprintln!(
                    "boxmemo: span log: {newcs} entered, {glob} global and {loc} local assignments to {} locations, {grp} groups, {toks} tokens",
                    locs.len()
                );
            }
            insert_entry(&mut s, rec.cs, entry);
            s.stats.committed += 1;
        });
        write_stats(false);
    }

    /// The normal path of a call the guard admitted, re-recorded: compare.
    fn bm_verified(
        &mut self,
        rec: Rec,
        i: usize,
        last_badness: Option<i32>,
        frag: &Option<crate::boxfrag::Frag>,
        trailer: Option<i32>,
        span: &Option<Box<SpanEntry>>,
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
                if !same_op(a, b) {
                    diffs.push(format!("op {j}: recorded {a:?} normal path {b:?}"));
                    break;
                }
            }
            let fd = match (&e.frag, frag) {
                (Some(a), Some(b)) => crate::boxfrag::frag_diff(a, b),
                (None, None) => None,
                (a, b) => Some(format!(
                    "output {} normal path {}",
                    a.is_some(),
                    b.is_some()
                )),
            };
            if let Some(d) = fd {
                diffs.push("the output nodes differ".to_string());
                diffs.push(d.chars().take(400).collect());
            }
            if e.trailer != trailer {
                diffs.push("the last \\ignorespaces differs".to_string());
            }
            // a span's file, where it left the file level, its paragraph
            // and its nodes' source positions (§17)
            match (&e.span, span) {
                (None, None) => {}
                (Some(a), Some(b)) => {
                    if a.file != b.file || a.toks != b.toks || a.copies != b.copies {
                        diffs.push("the span's lines differ".to_string());
                    }
                    if !a.end.same(&b.end) {
                        diffs.push(
                            format!(
                                "the span's end: recorded {:?} normal path {:?}",
                                a.end, b.end
                            )
                            .chars()
                            .take(400)
                            .collect(),
                        );
                    }
                    match (&a.graf, &b.graf) {
                        (None, None) => {}
                        (Some(x), Some(y)) => {
                            if (x.ml, x.pg, x.aux) != (y.ml, y.pg, y.aux) {
                                diffs.push(format!(
                                    "the span's paragraph: recorded {:?} normal path {:?}",
                                    (x.ml, x.pg, x.aux),
                                    (y.ml, y.pg, y.aux)
                                ));
                            }
                            for (what, f, g) in
                                [("skip", &x.outer, &y.outer), ("list", &x.inner, &y.inner)]
                            {
                                if let Some(d) = crate::boxfrag::frag_diff(f, g) {
                                    diffs.push(
                                        format!("the span's paragraph {what} differs: {d}")
                                            .chars()
                                            .take(400)
                                            .collect(),
                                    );
                                }
                            }
                        }
                        _ => diffs.push("the span's paragraph: one path began one".to_string()),
                    }
                    if a.pos != b.pos {
                        diffs.push("the span's source positions differ".to_string());
                    }
                }
                _ => diffs.push("one path is a span".to_string()),
            }
            if e.last_badness != last_badness {
                diffs.push(format!(
                    "last_badness: recorded {:?} normal path {last_badness:?}",
                    e.last_badness
                ));
            }
            if diffs.is_empty() && debug() {
                eprintln!("boxmemo: verified \\{name}");
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

    // -- spans (§17) ---------------------------------------------------------

    /// `\begin` was called with `n` arguments: is it a span? `Ok`: the
    /// environment's control sequence (the store's key) and the part of the
    /// key the file level's buffer holds (its line, where it stands in it,
    /// and the rest of the line). `Err(None)`: not an environment offered
    /// (no offer); `Err(Some(why))`: one offered that cannot be a span here.
    fn bm_span_offer(&mut self, n: i32) -> Result<(i32, Vec<i32>), Option<&'static str>> {
        if n != 1 || self.param_ptr < 1 {
            return Err(None);
        }
        let mut arg = vec![];
        if !self.bm_tokens(self.param_stack[(self.param_ptr - 1) as usize], &mut arg)
            || arg.is_empty()
            || arg.len() > 64
        {
            return Err(None);
        }
        // letters and other characters (`cmd*256+chr`, cmd 11 or 12)
        if arg.iter().any(|&t| !(11 * 256..13 * 256).contains(&t)) {
            return Err(None);
        }
        let name: Vec<i32> = arg.iter().map(|&t| t % 256).collect();
        let offered = with_config(|c| {
            c.spans
                .iter()
                .any(|s| s.len() == name.len() && s.iter().zip(&name).all(|(&a, &b)| a as i32 == b))
        });
        if !offered {
            return Err(None);
        }
        let Some(env) = self.bm_find_cs(&name) else {
            return Err(Some("SpanEnv"));
        };
        // the level below the body: a file's line (not the terminal, a
        // `\read` or a pseudo file)
        if self.input_ptr < 1 {
            return Err(Some("SpanLevel"));
        }
        let fl = self.input_stack[(self.input_ptr - 1) as usize];
        if fl.state_field == TOKEN_LIST {
            // (or a token list: a macro's argument, `\scalebox{..}{\begin..}`,
            // or body, read token by token, §17.10)
            if ![k::parameter, k::backed_up, k::inserted, k::macro_].contains(&fl.index_field)
                || fl.loc_field == 0
            {
                return Err(Some("SpanLevel"));
            }
            if self.span_in_preamble() {
                return Err(Some("SpanPreamble"));
            }
            return Ok((
                env,
                vec![-8, self.line, fl.index_field, fl.name_field, self.in_open],
            ));
        }
        if fl.name_field <= 19 || fl.index_field < 1 {
            return Err(Some("SpanLevel"));
        }
        if self.span_in_preamble() {
            return Err(Some("SpanPreamble"));
        }
        let mut key = vec![
            -7,
            self.line,
            fl.loc_field - fl.start_field,
            fl.limit_field - fl.start_field,
            fl.state_field,
            fl.index_field,
            self.in_open,
        ];
        for j in fl.loc_field..=fl.limit_field {
            key.push(self.buffer[j as usize]);
        }
        Ok((env, key))
    }

    /// Of a span's variants whose key holds but for the file (`fits`), the
    /// most recent whose lines the file level's file holds next; else
    /// `Err` with the most recent of them.
    fn bm_span_pick(
        &mut self,
        cs: i32,
        mut fits: Vec<(u64, usize)>,
    ) -> Result<usize, Option<usize>> {
        if fits.is_empty() {
            return Err(None);
        }
        fits.sort_unstable_by(|a, b| b.cmp(a));
        let most = fits[0].1;
        let want: Vec<(usize, String, usize)> = ST.with(|s| {
            let s = s.borrow();
            let v = &s.store[&cs];
            fits.iter()
                .filter_map(|&(_, i)| {
                    let sp = v[i].span.as_ref()?;
                    Some((i, sp.file.path.clone(), sp.file.lines.len()))
                })
                .collect()
        });
        let fl = self.input_stack[(self.input_ptr - 1) as usize];
        if fl.state_field == TOKEN_LIST {
            // (§17.10: the list's next tokens are the recorded ones)
            let high = self.max_buf_stack;
            let found = ST.with(|s| {
                let s = s.borrow();
                let v = &s.store[&cs];
                fits.iter().find_map(|&(_, i)| {
                    let sp = v[i].span.as_ref()?;
                    let t = sp.toks.as_ref()?;
                    let mut q = fl.loc_field;
                    for &x in t {
                        if q == 0 || self.bm_info(q) != x {
                            return None;
                        }
                        q = self.bm_link(q);
                    }
                    ((sp.end.max_buf.is_some() || sp.end.max_buf0 <= high)
                        && sp
                            .graf
                            .as_ref()
                            .is_none_or(|g| self.nest_ptr != 0 || self.bm_page_quiet(g.outer_h)))
                    .then_some(i)
                })
            });
            return found.ok_or(Some(most));
        }
        let n = want.iter().map(|w| w.2).max().unwrap_or(0);
        let (index, high) = (fl.index_field, self.max_buf_stack - fl.start_field);
        let file = &mut self.input_file[(index - 1) as usize];
        let Some(path) = file.opened_path().map(str::to_string) else {
            return Err(Some(most));
        };
        let Some(next) = file.peek_lines(n) else {
            return Err(Some(most));
        };
        let found = ST.with(|s| {
            let s = s.borrow();
            let v = &s.store[&cs];
            want.iter().find_map(|(i, p, len)| {
                let sp = v[*i].span.as_ref()?;
                // (the buffer's high mark: see `SpanEnd::max_buf0`)
                (*p == path
                    && next.len() >= *len
                    && next[..*len] == sp.file.lines[..]
                    && (sp.end.max_buf.is_some() || sp.end.max_buf0 <= high)
                    // (a paragraph begun on the main vertical list: the
                    // page builder's run there must fire nothing, §17.3)
                    && sp
                        .graf
                        .as_ref()
                        .is_none_or(|g| self.nest_ptr != 0 || self.bm_page_quiet(g.outer_h)))
                .then_some(*i)
            })
        });
        found.ok_or(Some(most))
    }

    /// `new_graf` began a paragraph, before (`k` 0) and after (1) the page
    /// builder's run on the main vertical list, during a recording: a
    /// span's paragraph on its own level is noted (§17.3).
    #[cold]
    #[inline(never)]
    pub fn flashtex_bm_new_graf(&mut self, k: i32) {
        let Some(Some((n0, tail0))) = self.bm_with_rec(|r| {
            r.span.as_ref().map(|s| {
                (
                    r.start.nest_ptr,
                    s.graf.as_ref().map_or(r.start.tail, |g| g.tail),
                )
            })
        }) else {
            return;
        };
        if debug() {
            eprintln!(
                "boxmemo: span new_graf {k}: nest {} (call's {n0}), line {}",
                self.nest_ptr, self.line
            );
        }
        // (a box's paragraph skips stay on its list, and are taken at the
        // end; a paragraph inside one of the call's own boxes is the call's)
        if self.nest_ptr != 1 || n0 != 0 {
            return;
        }
        if k == 0 {
            // what the main vertical list got since the call, or since the
            // page builder's last run: only paragraph skips
            let mut glues = vec![];
            let mut q = self.bm_link(tail0);
            while q != 0 {
                if q >= self.hi_mem_min || self.mem[q as usize].hh().b0() != k::glue_node {
                    return self.bm_abort("OuterList");
                }
                let w = |i: i32| self.mem[(q + i) as usize].0;
                let spec = self.mem[(q + 1) as usize].hh().lh();
                if self.mem[(q + 1) as usize].hh().rh() != 0 {
                    return self.bm_abort("OuterList");
                }
                let sw = |i: i32| self.mem[(spec + i) as usize].0;
                glues.push(GlueRec {
                    p: q,
                    words: [w(0), w(1)],
                    spec,
                    spec_words: [sw(0), sw(1), sw(2), sw(3)],
                    pos: self.dl_side_pos(q),
                });
                q = self.bm_link(q);
            }
            self.bm_with_rec(|r| {
                if let Some(s) = r.span.as_mut() {
                    let g = s.graf.get_or_insert(GrafNote {
                        glues: vec![],
                        tail: 0,
                        page: None,
                    });
                    g.glues.extend(glues);
                }
            });
        } else {
            // (the page builder took the contributions and fired nothing,
            // and no insertion is on the page: it read no insertion's
            // registers; each run read the same parameters)
            if self.output_active || self.bm_link(k::page_ins_head) != k::page_ins_head {
                return self.bm_abort("Page");
            }
            let page = self.bm_frame().page;
            let params = self.bm_page_params();
            let tail = self.nest[0].tail_field;
            let same = self
                .bm_with_rec(|r| {
                    let g = r.span.as_mut()?.graf.as_mut()?;
                    let same = g.page.as_ref().is_none_or(|(_, p)| *p == params);
                    g.page = Some((page, params));
                    g.tail = tail;
                    Some(same)
                })
                .flatten();
            if same != Some(true) {
                self.bm_abort("Page");
            }
        }
    }

    /// The paragraph skips a span appended to the main vertical list
    /// (§17.3), as a fragment: each specification as the replay must share
    /// it, decided now (the call's end, where the replay rebuilds them): the
    /// static ones, one an `eqtb` glue location, a sparse register or a
    /// font holds now with the same words, else the fragment's own -- when
    /// every reference to it is one of these nodes still on the page (one
    /// the page builder threw away is made again to be thrown away again).
    fn bm_graf_glues(&self, glues: &[GlueRec]) -> Result<crate::boxfrag::Frag, &'static str> {
        use crate::boxfrag::{Field, Frag, Node, Ref, Spec};
        let mut on_page = HashSet::new();
        let mut q = self.bm_link(k::page_head);
        while q != 0 && q != k::page_head {
            on_page.insert(q);
            q = self.bm_link(q);
        }
        let alive = |g: &GlueRec| {
            on_page.contains(&g.p)
                && self.mem[g.p as usize].hh().b0() == k::glue_node
                && self.mem[(g.p + 1) as usize].hh().lh() == g.spec
        };
        let same_words = |q: i32, w: &[u64; 4]| {
            (0..4).all(|i| {
                let x = self.mem[(q + i) as usize].0;
                // (word 0's link half is the reference count)
                if i == 0 {
                    x >> 32 == w[0] >> 32
                } else {
                    x == w[i as usize]
                }
            })
        };
        let mut f = Frag::default();
        let mut own: HashMap<i32, usize> = HashMap::new();
        for g in glues {
            let q = g.spec;
            let spec = if q <= LO_MEM_STAT_MAX {
                Spec::Static(q)
            } else if let Some(e) = (GLUE_BASE..LOCAL_BASE)
                .find(|&e| self.bm_eq(e).hh().rh() == q && same_words(q, &g.spec_words))
            {
                Spec::Eqtb(e)
            } else if let Some(n) = {
                let mut found = None;
                self.bm_sa_walk(k::glue_val, &mut |n, e| {
                    if found.is_none() && self.bm_link(e + 1) == q {
                        found = Some(n);
                    }
                });
                found.filter(|_| same_words(q, &g.spec_words))
            } {
                Spec::Sparse(k::glue_val, n)
            } else if let Some(i) = own.get(&q) {
                f.specs[*i].1 += 1;
                Spec::Own(*i)
            } else {
                let refs = glues.iter().filter(|x| x.spec == q && alive(x)).count();
                if alive(g) && self.mem[q as usize].hh().rh() as usize + 1 != refs {
                    // (shared with something not named: a register beyond
                    // `eqtb`'s, another list)
                    return Err("SharedGlue");
                }
                f.specs.push((g.spec_words, 1));
                own.insert(q, f.specs.len() - 1);
                Spec::Own(f.specs.len() - 1)
            };
            // (as `boxfrag` keeps a glue node: its first two words, the link
            // and both pointer fields set again; SyncTeX's two are
            // `get_node`'s)
            let words = vec![g.words[0] & !0xffff_ffffu64, 0];
            f.nodes.push(Node::Big {
                size: k::medium_node_size,
                copy: k::medium_node_size - 2,
                words,
                fields: vec![
                    Field {
                        word: 1,
                        left: true,
                        to: Ref::Spec(spec),
                    },
                    Field {
                        word: 1,
                        left: false,
                        to: Ref::Null,
                    },
                ],
            });
        }
        Ok(f)
    }

    /// What the page builder reads of `eqtb` when no insertion and no
    /// output routine is involved (pdftex.web's `build_page` and
    /// `freeze_page_specs`): `\vsize`, `\maxdepth`, `\tracingpages`, and
    /// `\topskip` (its glue on an empty page: width, stretch, shrink and
    /// the orders' word).
    fn bm_page_params(&self) -> [i64; 7] {
        let t = self.bm_eq(GLUE_BASE + k::top_skip_code).hh().rh();
        let w = |i: i32| self.mem[(t + i) as usize].int() as i64;
        [
            self.bm_eq(k::dimen_base + k::vsize_code).int() as i64,
            self.bm_eq(k::dimen_base + k::max_depth_code).int() as i64,
            self.bm_eq(INT_BASE + k::tracing_pages_code).int() as i64,
            w(1),
            w(2),
            w(3),
            (self.mem[t as usize].0 >> 32) as i64,
        ]
    }

    /// Will the page builder, run on the main vertical list's
    /// contributions and then skips of natural width `extra` (each at
    /// least 0), fire nothing and take no insertion (§17.3)? A span begun
    /// there replays its run only then: its recording's paragraph was
    /// typeset with no output routine between. Sufficient, not necessary:
    /// no penalty that forces a break, no insertion, and the page's total
    /// with every contribution's height, depth and natural width added
    /// (each at least 0; shrink only takes away) at most the page's goal,
    /// so no breakpoint is past the goal (pdftex.web's `build_page`, `c =
    /// awful_bad` only then).
    fn bm_page_quiet(&self, extra: i64) -> bool {
        if self.output_active
            || self.bm_link(k::page_ins_head) != k::page_ins_head
            || self.insert_penalties >= k::inf_penalty
            || self.nest_ptr != 0
        {
            return false;
        }
        let sc = |p: i32| (self.mem[p as usize].int() as i64).max(0);
        let (goal, mut h) = if self.page_contents == k::empty {
            let t = self.bm_eq(GLUE_BASE + k::top_skip_code).hh().rh();
            (
                self.bm_eq(k::dimen_base + k::vsize_code).int() as i64,
                sc(t + 1),
            )
        } else {
            (self.page_so_far[0] as i64, 0)
        };
        h += self.page_so_far[1] as i64 + (self.page_so_far[7] as i64).max(0) + extra;
        let mut p = self.bm_link(CONTRIB_HEAD);
        let mut n = 0;
        while p != 0 {
            n += 1;
            if p >= self.hi_mem_min || n > 10_000 {
                return false;
            }
            let t = self.mem[p as usize].hh().b0();
            if t == k::hlist_node || t == k::vlist_node || t == k::rule_node {
                h += sc(p + 3) + sc(p + 2);
            } else if t == k::glue_node {
                h += sc(self.mem[(p + 1) as usize].hh().lh() + 1);
            } else if t == k::kern_node {
                h += sc(p + 1);
            } else if t == k::penalty_node {
                if self.mem[(p + 1) as usize].int() <= -k::inf_penalty {
                    return false;
                }
            } else if t == k::ins_node {
                return false;
            }
            if p == self.cur_list.tail_field {
                break;
            }
            p = self.bm_link(p);
        }
        h <= goal
    }

    /// A token list's F (§17.10): the tokens the span read from it, from
    /// `loc` at the call to `loc` now; refused if one is a macro
    /// parameter (`out_param`: the macro's argument, which the key does not
    /// hold).
    fn bm_span_toks(&self, s: &SpanRec) -> Result<Option<Vec<i32>>, &'static str> {
        let Some((_, loc0)) = s.toks else {
            return Ok(None);
        };
        let l = if self.input_ptr == s.f {
            self.cur_input
        } else {
            self.input_stack[s.f as usize]
        };
        let mut v = vec![];
        let mut q = loc0;
        while q != l.loc_field {
            if q == 0 || v.len() >= 1 << 20 {
                return Err("SpanToks");
            }
            let t = self.bm_info(q);
            if t < k::cs_token_flag && t / 256 == k::out_param {
                return Err("SpanParam");
            }
            v.push(t);
            q = self.bm_link(q);
        }
        Ok(Some(v))
    }

    /// Where a span's recording leaves its file level (§17).
    fn bm_span_end(&self, s: &SpanRec) -> SpanEnd {
        let l = if self.input_ptr == s.f {
            self.cur_input
        } else {
            self.input_stack[s.f as usize]
        };
        if s.toks.is_some() {
            // (a token list: its tokens, `bm_span_toks`; reading it moves
            // nothing in the buffer or the file, and the buffer's high mark
            // is counted from 0)
            return SpanEnd {
                buf: vec![],
                loc: 0,
                limit: 0,
                state: TOKEN_LIST,
                first: 0,
                last: None,
                max_buf: (self.max_buf_stack > s.max_buf).then_some(self.max_buf_stack),
                max_buf0: s.max_buf,
                line: self.line,
                journal: self
                    .bm_span_journal(s.f, s.line0, s.journal)
                    .unwrap_or_default(),
            };
        }
        let st = l.start_field;
        let read = !s.file.lines.is_empty();
        SpanEnd {
            // (the last line read, which `bm_span_pick` compared with the
            // file; a span that read none leaves the line it began on as it
            // was, and the text before the call is not in the key: a replay
            // must not put the recorded run's back, which an error's context
            // prints)
            buf: if read {
                (st..=l.limit_field)
                    .map(|j| self.buffer[j as usize])
                    .collect()
            } else {
                vec![]
            },
            loc: l.loc_field - st,
            limit: l.limit_field - st,
            state: l.state_field,
            first: self.first - st,
            last: read.then_some(self.last - st),
            max_buf: (self.max_buf_stack > s.max_buf).then_some(self.max_buf_stack - st),
            max_buf0: s.max_buf - st,
            line: self.line,
            journal: self
                .bm_span_journal(s.f, s.line0, s.journal)
                .unwrap_or_default(),
        }
    }

    /// The line-shift journal's entries since `mark`, each a read of the
    /// span's file level's line (its SyncTeX tag, the file), as its line's
    /// distance from the span's first line; `None` when one is not, or a
    /// checkpoint took some (§17.4).
    fn bm_span_journal(&self, f: i32, line0: i32, mark: u64) -> Option<Vec<i32>> {
        let l = if self.input_ptr == f {
            self.cur_input
        } else {
            self.input_stack[f as usize]
        };
        crate::lineshift::journal_since(mark)?
            .iter()
            .map(|r| (r.known && r.tag == l.synctex_tag_field).then_some(r.line - line0))
            .collect()
    }

    /// The nodes of list `p` in pre-order, character nodes included,
    /// through boxes' lists, leaders, ligatures' characters and
    /// discretionaries' texts, as `boxfrag` keeps them; a hole and what it
    /// holds left out (its nodes are not the call's).
    fn bm_nodes(&self, mut p: i32, holes: &[i32], out: &mut Vec<i32>) {
        while p != 0 {
            if !holes.contains(&p) {
                out.push(p);
                if p < self.hi_mem_min {
                    let t = self.mem[p as usize].hh().b0();
                    let w1 = self.mem[(p + 1) as usize].hh();
                    if t == k::hlist_node || t == k::vlist_node || t == k::unset_node {
                        self.bm_nodes(self.mem[(p + 5) as usize].hh().rh(), holes, out);
                    } else if t == k::glue_node || t == k::ligature_node {
                        self.bm_nodes(w1.rh(), holes, out);
                    } else if t == k::disc_node {
                        self.bm_nodes(w1.lh(), holes, out);
                        self.bm_nodes(w1.rh(), holes, out);
                    } else if (k::ord_noad..=k::inner_noad).contains(&t)
                        || t == k::under_noad
                        || t == k::over_noad
                        || t == k::vcenter_noad
                    {
                        // (`boxfrag`'s noads: a field's sub-box or sub-list)
                        for w in 1..=3 {
                            let f = self.mem[(p + w) as usize].hh();
                            if f.rh() == k::sub_box || f.rh() == k::sub_mlist {
                                self.bm_nodes(f.lh(), holes, out);
                            }
                        }
                    }
                }
            }
            p = self.bm_link(p);
        }
    }

    /// Give the nodes of the lists a replay rebuilt (in `bm_nodes`'s order)
    /// the source positions the normal path gave them.
    fn bm_put_positions(&mut self, at: &[i32], pos: &[crate::displaylist::SidePos]) {
        if at.len() != pos.len() {
            debug_assert!(
                false,
                "boxmemo: a span's nodes and positions differ in number"
            );
            return;
        }
        for (&p, &v) in at.iter().zip(pos) {
            self.dl_side_put(p, v);
        }
    }

    /// A span's replay leaves its file level as the normal path did: the
    /// `n` lines it read are read again (as `input_ln` reads them) and the
    /// level's buffer, positions and state, `first`, `last` and `line` set.
    /// The file level is the current one (the body is popped).
    fn bm_span_forward(&mut self, n: usize, end: &SpanEnd) {
        let index = self.cur_input.index_field;
        assert!(
            self.cur_input.state_field != TOKEN_LIST && index >= 1,
            "boxmemo: a span's file level is not the current level"
        );
        let ok = self.input_file[(index - 1) as usize].skip_lines(n);
        assert!(ok, "boxmemo: a span's file ended before its lines");
        let st = self.cur_input.start_field;
        for (j, &v) in end.buf.iter().enumerate() {
            self.buffer[st as usize + j] = v;
        }
        self.cur_input.loc_field = st + end.loc;
        self.cur_input.limit_field = st + end.limit;
        self.cur_input.state_field = end.state;
        self.first = st + end.first;
        if let Some(l) = end.last {
            self.last = st + l;
        }
        if let Some(m) = end.max_buf {
            self.max_buf_stack = self.max_buf_stack.max(st + m);
        }
        self.line = end.line;
        // (each line `input_ln` read counts towards the next checkpoint)
        if self.arena.extra.is_some() {
            SPAN_FORWARD.with(|c| c.set(true));
            for _ in 0..n {
                self.maybe_request_timed_checkpoint();
            }
            SPAN_FORWARD.with(|c| c.set(false));
        }
    }

    /// A token list's F (§17.10): a span's replay leaves it as the normal
    /// path did, `n` tokens further on (the body is popped).
    fn bm_span_forward_toks(&mut self, n: usize, end: &SpanEnd) {
        assert!(
            self.cur_input.state_field == TOKEN_LIST,
            "boxmemo: a span's token list is not the current level"
        );
        for _ in 0..n {
            let q = self.cur_input.loc_field;
            assert!(
                q != 0,
                "boxmemo: a span's token list ended before its tokens"
            );
            self.cur_input.loc_field = self.bm_link(q);
        }
        if let Some(m) = end.max_buf {
            self.max_buf_stack = self.max_buf_stack.max(m);
        }
        self.line = end.line;
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
            Val::Shared(id, l) => match SHARED.with(|s| s.borrow().get(id).copied()) {
                Some(e) => {
                    // (`add_token_ref`, as the normal path's `\let`)
                    let r = self.bm_info(e) + 1;
                    self.mem[e as usize].set_hh_lh(r);
                    (None, e)
                }
                None => {
                    let e = self.bm_make_list(l);
                    SHARED.with(|s| s.borrow_mut().insert(*id, e));
                    (None, e)
                }
            },
            Val::GlueFrom(a, b) => {
                let e = if *a > 0 {
                    self.bm_eq(*a).hh().rh()
                } else if *a == 0 {
                    // (the key holds it: `font_glues`)
                    self.font_glue[*b as usize]
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
            Val::Box(f) => {
                // (as the draw calls' fragments are made: SyncTeX's and the
                // display list's fields are `get_node`'s, at the call's
                // place, which the normal path's nodes have too)
                let save = self.scanner_status;
                self.scanner_status = 0;
                let (b, _) = self.frag_make(f, &[], &[]);
                self.scanner_status = save;
                (None, b)
            }
            Val::BoxNode(_) => unreachable!("boxmemo: a box by its node in an entry"),
        }
    }

    /// Replay entry `i` of `cs`; its trailer token, if any, for the caller
    /// to back up (`main_control` then runs it).
    fn bm_replay(&mut self, cs: i32, i: usize) -> Option<i32> {
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
                std::mem::take(&mut e.fast),
                e.last_badness,
                std::mem::take(&mut e.rs_locs),
            ))
        });
        let (ops, last_badness, rs_locs) = taken?;
        SHARED.with(|s| s.borrow_mut().clear());
        // the body, used up: as `get_next` pops it
        self.end_token_list();
        // the L5 read-set (DESIGN.md §5.5), as D9's `intr_report_reads`
        if self.rs_on {
            for &p in &rs_locs {
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
                Op::NewCs { p, name } => self.bm_make_cs(*p, name),
                Op::BoxDim { n, c, v } => {
                    let b = self.bm_box_value(*n);
                    if b != 0 {
                        self.mem[(b + c) as usize].set_int(*v);
                    }
                }
            }
        }
        if let Some(b) = last_badness {
            self.last_badness = b;
        }
        // a draw call: the boxes it moved out of their registers, then its
        // output appended to the current list
        let draw = ST.with(|s| {
            let s = s.borrow();
            let e = s.store.get(&cs)?.get(i)?;
            Some((
                e.holes.clone(),
                e.frag.clone(),
                e.aux,
                e.new_objs,
                e.last_link,
            ))
        });
        // a span (§17): its paragraph, its nodes' positions, where it left
        // its file level
        let span = ST.with(|s| {
            let s = s.borrow();
            let sp = s.store.get(&cs)?.get(i)?.span.as_ref()?;
            Some((
                sp.file.lines.len(),
                sp.end.clone(),
                sp.graf.clone(),
                sp.pos.clone(),
                sp.copies.clone(),
                sp.toks.as_ref().map(Vec::len),
            ))
        });
        let dl = crate::displaylist::enabled();
        // (a span's copies of foreign boxes, made as `\copy` made them, of
        // the boxes as they are now: after its `BoxDim`s, before a box is
        // moved, §17.6)
        let copies: Vec<i32> = span
            .as_ref()
            .map(|sp| {
                sp.4.iter()
                    .map(|&n| {
                        let b = self.bm_box_value(n);
                        self.copy_node_list(b)
                    })
                    .collect()
            })
            .unwrap_or_default();
        // the links' objects, as `pdf_new_objnum` made them on the normal
        // path (nothing else the call did makes or reads an object)
        let mut objs = vec![];
        if let Some((_, _, _, n, last)) = &draw {
            objs = (0..*n).map(|_| self.pdf_new_objnum()).collect();
            if let Some(l) = last {
                self.pdf_last_link = objs[*l];
            }
        }
        if let Some((holes, Some(frag), aux, _, _)) = &draw {
            let mut nodes = vec![];
            for (n, _) in holes {
                nodes.push(self.bm_box_value(*n));
                self.bm_void_box(*n);
            }
            let save = self.scanner_status;
            self.scanner_status = 0;
            let (first, last) = self.frag_make_span(frag, &nodes, &copies, &objs);
            self.scanner_status = save;
            if first != 0 {
                let t = self.cur_list.tail_field;
                self.mem[t as usize].set_hh_rh(first);
                self.cur_list.tail_field = last;
            }
            self.cur_list.aux_field = crate::generated::types::memory_word(*aux);
            if let Some((_, _, _, pos, _, _)) = &span {
                if dl && !pos.is_empty() {
                    let mut at = vec![];
                    let skip: Vec<i32> = nodes.iter().chain(&copies).copied().collect();
                    self.bm_nodes(first, &skip, &mut at);
                    self.bm_put_positions(&at, pos);
                }
            }
        }
        if let (Some((holes, None, _, _, _)), Some((_, _, Some(g), pos, _, _))) = (&draw, &span) {
            // §17.3: the paragraph skip on the box's list, then the
            // paragraph `new_graf` began, its list with the holes
            let mut nodes = vec![];
            for (n, _) in holes {
                nodes.push(self.bm_box_value(*n));
                self.bm_void_box(*n);
            }
            let save = self.scanner_status;
            self.scanner_status = 0;
            // (`new_graf`: `prev_graf:=0` on the outer level, its skip,
            // `push_nest`, the paragraph's level; and on the main vertical
            // list the page builder, which reads only what the key and the
            // recording's checks hold, §17.3)
            self.cur_list.pg_field = 0;
            let (first, last) = self.frag_make(&g.outer, &[], &objs);
            if first != 0 {
                let t = self.cur_list.tail_field;
                self.mem[t as usize].set_hh_rh(first);
                self.cur_list.tail_field = last;
            }
            // (the skips' positions now: the page builder may free them)
            let n_outer = if dl && !pos.is_empty() {
                let mut at = vec![];
                self.bm_nodes(first, &[], &mut at);
                let n = at.len().min(pos.len());
                self.bm_put_positions(&at[..n], &pos[..n]);
                n
            } else {
                0
            };
            self.push_nest();
            self.cur_list.mode_field = k::hmode;
            self.cur_list.aux_field = crate::generated::types::memory_word(g.aux);
            self.cur_list.pg_field = g.pg;
            self.cur_list.ml_field = g.ml;
            if self.nest_ptr == 1 {
                self.build_page();
            }
            let head = self.cur_list.head_field;
            let (ifirst, ilast) = self.frag_make_span(&g.inner, &nodes, &copies, &objs);
            self.scanner_status = save;
            if ifirst != 0 {
                self.mem[head as usize].set_hh_rh(ifirst);
                self.cur_list.tail_field = ilast;
            }
            if dl && !pos.is_empty() {
                let mut at = vec![head];
                let skip: Vec<i32> = nodes.iter().chain(&copies).copied().collect();
                self.bm_nodes(ifirst, &skip, &mut at);
                self.bm_put_positions(&at, &pos[n_outer..]);
            }
        }
        if let Some((n, end, _, _, _, toks)) = &span {
            // (the line-shift journal's entries, as the normal path made
            // them: reads of the file level's lines, whose first is `line`
            // now, the key's)
            let tag = self.cur_input.synctex_tag_field;
            for &d in &end.journal {
                crate::lineshift::journal_push(crate::lineshift::LineRead {
                    tag,
                    known: true,
                    line: self.line + d,
                });
            }
            match toks {
                Some(t) => self.bm_span_forward_toks(*t, end),
                None => self.bm_span_forward(*n, end),
            }
        }
        let n = ops.len() as u64;
        ST.with(|s| {
            let mut s = s.borrow_mut();
            if let Some(e) = s.store.get_mut(&cs).and_then(|v| v.get_mut(i)) {
                e.fast = ops;
                e.rs_locs = rs_locs;
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
        ST.with(|s| {
            s.borrow()
                .store
                .get(&cs)
                .and_then(|v| v.get(i))
                .and_then(|e| e.trailer)
        })
    }

    /// The end of the run: the statistics.
    pub fn flashtex_bm_finish(&mut self) {
        write_stats(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(p: i32, w: i32, kind: i32) -> Op {
        Op::Def {
            p,
            t: 0,
            v: Val::Word(w),
            kind,
        }
    }

    #[test]
    fn low_halves_compare_as_the_word_by_word_loop() {
        use crate::generated::types::memory_word;
        // each length around the blocks of 16, each place changed, in the
        // low half (a difference) and in the high half (none)
        for n in [0usize, 1, 15, 16, 17, 31, 32, 33, 100] {
            let words: Vec<memory_word> = (0..n)
                .map(|i| memory_word(((i as u64 * 7919) << 32) | (i as u64 * 31 + 5)))
                .collect();
            let want: Vec<i32> = words.iter().map(|w| w.int()).collect();
            assert!(low_halves_eq(&words, &want));
            assert!(!low_halves_eq(&words, &want[..n.saturating_sub(1)]) || n == 0);
            for k in 0..n {
                let mut w2 = want.clone();
                w2[k] ^= 1 << (k % 32);
                assert!(!low_halves_eq(&words, &w2), "{n} {k}");
                let mut words2 = words.clone();
                words2[k].0 ^= 1 << 40;
                assert!(low_halves_eq(&words2, &want), "{n} {k}");
                assert_eq!(
                    low_halves_eq(&words, &w2),
                    words.iter().map(|x| x.hh().rh()).eq(w2.iter().copied())
                );
            }
        }
    }

    #[test]
    fn groups_stand_for_their_last_global_assignments() {
        // a local group goes; a group with globals leaves the last global
        // value of each location, where the group was
        let ops = vec![
            def(1, 10, 1),
            Op::Begin(1),
            def(2, 20, 1),
            def(3, 30, 3),
            Op::Begin(1),
            def(3, 31, 3),
            def(4, 40, 3),
            Op::End,
            def(3, 32, 1),
            Op::End,
            Op::Begin(1),
            def(5, 50, 1),
            Op::End,
            def(6, 60, 1),
        ];
        assert_eq!(
            compress_groups(&ops),
            vec![def(1, 10, 1), def(3, 31, 3), def(4, 40, 3), def(6, 60, 1)]
        );
    }

    #[test]
    fn a_value_from_the_state_keeps_its_group() {
        let let_global = Op::Def {
            p: 7,
            t: 0,
            v: Val::LetCs(8),
            kind: 2,
        };
        let ops = vec![
            Op::Begin(1),
            def(8, 1, 1),
            def(10, 5, 1),
            let_global.clone(),
            Op::Begin(1),
            def(9, 2, 1),
            Op::End,
            Op::End,
        ];
        assert_eq!(
            compress_groups(&ops),
            vec![Op::Begin(1), def(8, 1, 1), let_global.clone(), Op::End]
        );
        // a `\let` of what the group has not assigned yet reads the state
        // where the group was: made there
        let early = vec![Op::Begin(1), let_global.clone(), def(8, 1, 1), Op::End];
        assert_eq!(compress_groups(&early), vec![let_global]);
        // a group the call does not close, and an `End` of the caller's
        // group, are replayed as recorded
        let open = vec![Op::Begin(1), def(1, 1, 3), Op::End, Op::End, Op::Begin(2)];
        assert_eq!(
            compress_groups(&open),
            vec![def(1, 1, 3), Op::End, Op::Begin(2)]
        );
    }

    #[test]
    fn a_spans_let_takes_the_value_its_group_assigned() {
        // §17.5: `\def\q{..}` (local) then `\global\let\p\q`: in a span's
        // log the `\let` takes the list; elsewhere the group is kept
        let list = |p: i32, kind: i32, l: Vec<i32>| Op::Def {
            p,
            t: 111,
            v: Val::FreshList(l),
            kind,
        };
        let let_ = |p: i32, q: i32, kind: i32| Op::Def {
            p,
            t: 111,
            v: Val::LetCs(q),
            kind,
        };
        let ops = vec![
            Op::Begin(1),
            list(8, 0, vec![1, 2]),
            let_(7, 8, 2),
            Op::Begin(1),
            list(8, 0, vec![3]),
            let_(6, 8, 2),
            Op::End,
            let_(5, 8, 2),
            Op::End,
        ];
        // (\p and \r share \q's list, as on the normal path; \s's is its own)
        let shared = |p: i32| Op::Def {
            p,
            t: 111,
            v: Val::Shared(1, vec![1, 2]),
            kind: 2,
        };
        assert_eq!(
            compress_groups_with(&ops, true),
            vec![shared(7), list(6, 2, vec![3]), shared(5)]
        );
        assert_eq!(compress_groups_with(&ops, false), ops);
        // a `\let` from one the inner group assigned only locally, which
        // the span does not follow, or of a `\let` whose source changed
        // since: kept
        let ops2 = vec![
            Op::Begin(1),
            Op::Begin(1),
            list(8, 0, vec![3]),
            Op::End,
            let_(5, 8, 2),
            Op::End,
        ];
        assert_eq!(
            compress_groups_with(&ops2, true),
            vec![Op::Begin(1), let_(5, 8, 2), Op::End]
        );
        let ops3 = vec![
            Op::Begin(1),
            let_(8, 9, 0),
            list(9, 0, vec![4]),
            let_(5, 8, 2),
            Op::End,
        ];
        assert_eq!(
            compress_groups_with(&ops3, true),
            vec![
                Op::Begin(1),
                let_(8, 9, 0),
                list(9, 0, vec![4]),
                let_(5, 8, 2),
                Op::End
            ]
        );
    }
}
