//! Guarded intrinsics (DESIGN.md §5.6 items 4 and 6, decision D9): a
//! registered macro without parameters, run by `main_control`, is recorded
//! once and afterwards *replayed* -- its assignments made directly through
//! TeX's own `eq_define`/`eq_word_define`/`new_save_level`/`unsave` -- for
//! as long as everything the recorded run read still holds the value it read.
//!
//! The first target is hyperref's `\pdfstringdefPreHook`, which every
//! `\pdfstringdef` runs (twice per page: page label and page anchor) and
//! which siunitx fills with twelve `\DeclareCommandCopy` calls and a
//! `\cs_set_eq:Nc` for each of its ~250 units: most of LaTeX's per-page
//! cost on such documents (docs/evidence/l6-intrinsics-2026-09-29/).
//!
//! # What is a pure leaf function here
//!
//! A *recording* (changes/intrinsics.ch has every hook) is the normal
//! expansion and execution of the macro's body, observed:
//!
//! * **reads**: the meaning of every control sequence `get_next` delivers,
//!   `\csname`/`\ifcsname` look-ups, the internal quantities
//!   `scan_something_internal` fetches (`\catcode`s, integer and dimension
//!   parameters, token parameters), `\escapechar` (printing), all 256
//!   `\catcode`s when a token list is printed (`\meaning`, `\detokenize`,
//!   `\pdfstrcmp`, ...: `print_cs` reads them), and the value every
//!   assignment overwrites. The first access to each `eqtb` entry is kept:
//!   entries below `int_base` in a *watch* list, the rest (integers,
//!   dimensions) as (location, value) pairs.
//! * **writes**: `eq_define`, `geq_define`, `eq_word_define`,
//!   `geq_word_define`, `new_save_level`, `unsave`, in order: the
//!   *operations* replayed later.
//! * **purity**: only an allowlist of commands (`\relax`, `\begingroup`,
//!   `\endgroup`, `{`, `}`, `\ignorespaces`, spaces outside horizontal mode,
//!   and the assignments `\let`, `\futurelet`, `\def`/`\edef`/`\gdef`/
//!   `\xdef`, `\chardef` and friends, `\catcode` and the other codes,
//!   integer parameters and registers, `\advance`/`\multiply`/`\divide`,
//!   token parameters with a braced right-hand side) and of expandable
//!   primitives (`\expandafter`, `\unless`, `\noexpand`, `\csname`, the
//!   conditionals that read only tokens and integers, `\number`,
//!   `\romannumeral`, `\string`, `\meaning`, `\the`, `\unexpanded`,
//!   `\detokenize`, `\expanded`, `\pdfstrcmp`, `\pdfescape...`,
//!   `\numexpr`, version numbers). Anything else -- typesetting, boxes,
//!   glue, dimensions, fonts, files, marks, sparse (e-TeX) registers,
//!   `\afterassignment`, `\aftergroup`, messages, errors, any output, a new
//!   control sequence, reading a token from outside the macro's own input
//!   levels, closing a group or conditional it did not open, leaving braces
//!   unbalanced -- abandons the recording, and the macro stays an ordinary
//!   macro.
//!
//! A recording ends at `big_switch` once every input level at or above the
//! body's is used up: then `main_control` has executed all of it.
//!
//! # The guard (O(1))
//!
//! Every watched entry carries a watch record: the value (`eq_type`,
//! `equiv`, not `eq_level`) the recording read, and whether the entry holds
//! it now. `eq_define`, `geq_define` and `unsave`'s restore report writes to
//! watched entries (`flashtex_intr_touch`), which keeps, per recording, the
//! number of watched entries that do not hold the recorded value. The guard
//! is: that count is zero, the integer pairs still hold (a handful), the
//! mode, `align_state` and `par_token` are as recorded, and the
//! preconditions hold: `\globaldefs=0`, no pending `\afterassignment`, and
//! no tracing that would show the expansion (`\tracingmacros`,
//! `\tracingcommands`, `\tracingassigns`, `\tracingrestores`,
//! `\tracinggroups`, `\tracingifs`, `\tracingnesting`,
//! `\tracingscantokens` all <= 0). The macro's own meaning is a watched
//! entry, so redefining it (or anything it read) fails the guard; then the
//! macro is expanded normally and recorded afresh.
//!
//! Token lists the recording read or created are *pinned* (their reference
//! count raised) while the recording lives, so that an equal pointer is
//! always the same, unchanged list; a replay makes the pinned list the new
//! meaning (as `\let` would) instead of a copy of it. Only the memory-usage
//! statistics can tell (and P-T1 normalises those out, DESIGN.md §1.1).
//!
//! Everything lives in the word space (`intr_state`, `intr_data`,
//! `intr_watch`, `intr_seen`, `intr_cand`), so checkpoints and restores
//! (§5.2) carry it along with the `eqtb` and `mem` it describes.
//!
//! # Verification
//!
//! `FLASHTEX_INTRINSICS=verify` runs both paths for every replayable call
//! and diffs the complete state change (`crate::intrinsics_verify`).
//! `verify-all` does the same for *every* parameterless macro that
//! `big_switch` expands (a stress test of the recording and the guard).
//! `off` disables the intrinsics; the default is on.
//!
//! # Macros with parameters (docs/design/engine-v2/MACRO-REPLAY.md)
//!
//! With `FLASHTEX_INTRINSICS_ARGS=on` (off by default until the gates of
//! MACRO-REPLAY.md §6.4 pass), a registered macro *with* parameters is
//! offered at a second call site: once `macro_call` has scanned its
//! arguments and popped the used-up input levels, just before it feeds the
//! body (`flashtex_intr_call_args`). A recording there also keeps the
//! argument token lists, the key: a call is replayed only with the same
//! tokens, and the guard of the parameterless case besides. A replay frees
//! the argument lists, which the normal path frees when the body's level
//! ends. `verify` covers these replays as well; `verify-all-args` makes
//! every macro with parameters that `big_switch` expands a candidate.

use crate::generated::Globals;
use std::cell::RefCell;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Command codes (merged pdftex.web with this engine's change files:
// partoken.ch adds `partoken_name`, so max_command = 103). Checked by
// docs/evidence/l6-intrinsics-2026-09-29/scripts/webconsts.py.
const RELAX: i32 = 0;
const LEFT_BRACE: i32 = 1;
const RIGHT_BRACE: i32 = 2;
const SPACER: i32 = 10;
const IGNORE_SPACES: i32 = 39;
const BEGIN_GROUP: i32 = 61;
const END_GROUP: i32 = 62;
const CHAR_GIVEN: i32 = 68;
const MATH_GIVEN: i32 = 69;
const LAST_ITEM: i32 = 70;
const MAX_NON_PREFIXED_COMMAND: i32 = 70;
const TOKS_REGISTER: i32 = 71;
const ASSIGN_TOKS: i32 = 72;
const ASSIGN_INT: i32 = 73;
const ASSIGN_DIMEN: i32 = 74;
const DEF_CODE: i32 = 85;
const REGISTER: i32 = 89;
const ADVANCE: i32 = 90;
const MULTIPLY: i32 = 91;
const DIVIDE: i32 = 92;
const PREFIX: i32 = 93;
const LET: i32 = 94;
const SHORTHAND_DEF: i32 = 95;
const DEF: i32 = 97;
const EXPAND_AFTER: i32 = 105;
const NO_EXPAND: i32 = 106;
const IF_TEST: i32 = 108;
const FI_OR_ELSE: i32 = 109;
const CS_NAME: i32 = 110;
const CONVERT: i32 = 111;
const THE: i32 = 112;
const TOP_BOT_MARK: i32 = 113;
const CALL: i32 = 114;
const LONG_OUTER_CALL: i32 = 117;
const END_TEMPLATE: i32 = 118;
const GLUE_REF: i32 = 120;
const SHAPE_REF: i32 = 121;
const BOX_REF: i32 = 122;

const VMODE: i32 = 1;
const MMODE: i32 = 209;
const SIMPLE_GROUP: i32 = 1;
const SEMI_SIMPLE_GROUP: i32 = 14;
const MEM_BOT: i32 = 0;
const LO_MEM_STAT_MAX: i32 = 19;
const PROTECTED_TOKEN: i32 = 3585;
const END_MATCH_TOKEN: i32 = 3584;
const V_TEMPLATE: i32 = 2;

// Integer parameter codes.
const TRACING_MACROS_CODE: i32 = 30;
const TRACING_COMMANDS_CODE: i32 = 36;
const TRACING_RESTORES_CODE: i32 = 37;
const GLOBAL_DEFS_CODE: i32 = 43;
const ESCAPE_CHAR_CODE: i32 = 45;
// e-TeX's integer parameters follow pdfTeX's, which follow web2c's (and
// TeX Live's encTeX's), so their codes come from the generated layout
const ETEX_INT_BASE: i32 = crate::generated::consts::layout_etex_int_base;
const TRACING_ASSIGNS_CODE: i32 = ETEX_INT_BASE;
const TRACING_GROUPS_CODE: i32 = ETEX_INT_BASE + 1;
const TRACING_IFS_CODE: i32 = ETEX_INT_BASE + 2;
const TRACING_SCAN_TOKENS_CODE: i32 = ETEX_INT_BASE + 3;
const TRACING_NESTING_CODE: i32 = ETEX_INT_BASE + 4;
const TRACING_CODES: [i32; 8] = [
    TRACING_MACROS_CODE,
    TRACING_COMMANDS_CODE,
    TRACING_RESTORES_CODE,
    TRACING_ASSIGNS_CODE,
    TRACING_GROUPS_CODE,
    TRACING_IFS_CODE,
    TRACING_SCAN_TOKENS_CODE,
    TRACING_NESTING_CODE,
];

// ---------------------------------------------------------------------------
// The layout of intr_state and intr_data
// ---------------------------------------------------------------------------

const S_SERIAL: usize = 0;
const S_REC_SLOT: usize = 1; // slot being recorded, plus one
const S_REC_BASE: usize = 2; // input level of the body, 0 until fed
const S_REC_LEVEL: usize = 3;
const S_REC_COND: usize = 4;
const S_REC_IF_LIMIT: usize = 5;
const S_REC_CUR_IF: usize = 6;
const S_REC_IF_LINE: usize = 7;
const S_REC_ALIGN: usize = 8;
const S_REC_STR_PTR: usize = 9;
const S_REC_HASH_USED: usize = 10;
const S_REC_ERRORS: usize = 11;
const S_REC_HISTORY: usize = 12;
const S_REC_FILE_OFF: usize = 13;
const S_REC_TERM_OFF: usize = 14;
const S_REC_MODE: usize = 15;
const S_REC_NEST_PTR: usize = 16;
const S_REC_TAIL: usize = 17;
const S_REC_POOL_PTR: usize = 18;
const S_REC_VERIFY: usize = 19; // 1: a verification run of a valid slot
const S_REC_CATCODES: usize = 20; // 1: the catcodes are already read
const S_REC_LOG_LO: usize = 21;
const S_REC_LOG_HI: usize = 22;
const S_REC_SCANNER: usize = 23;
// The begin-document snapshot's arming state at the recording's start: a
// recording commits only if it is unchanged (MACRO-REPLAY.md §3.1).
const S_REC_ARM_CS: usize = 24;
const S_REC_ARM_LEVEL: usize = 25;
const S_REC_REQUEST: usize = 26;
/// For the convergence test (`crate::incr`): the slot being recorded,
/// and the scratch a recording sets at its start.
pub(crate) const REC_SLOT: usize = S_REC_SLOT;
pub(crate) const REC_SCRATCH: (usize, usize) = (S_REC_BASE, S_REC_REQUEST);
const S_WATCH_FREE: usize = 30; // free watch records, index + 1
const S_WATCH_TOP: usize = 31; // records ever allocated
const S_NSLOTS: usize = 32; // slots ever used
const S_CALLS: usize = 33; // calls of candidates (for choosing a variant to replace)
                           // The argument site (MACRO-REPLAY.md §3.3).
const S_ATOP: usize = 34; // argument-site slots ever used
const S_AFREE: usize = 35; // free argument-site slots, id + 1
const S_HTOP: usize = 36; // words of the slot heap ever handed out since a compaction
const S_HLIVE: usize = 37; // words of the slot heap in use
const S_HUSED: usize = 38; // entries of the argument index in use or deleted

// Layout constants, set by changes/intrinsics.ch at `Set init`.
const L_HASH_BASE: usize = 100;
const L_FONT_ID_BASE: usize = 102;
const L_UNDEFINED_CONTROL_SEQUENCE: usize = 103;
const L_GLUE_BASE: usize = 104;
const L_LOCAL_BASE: usize = 105;
const L_BOX_BASE: usize = 107;
const L_CAT_CODE_BASE: usize = 109;
const L_INT_BASE: usize = 110;
const L_EQTB_SIZE: usize = 111;
const L_HASH_PRIME: usize = 120;
const L_EQTB_TOP: usize = 121;

const SLOT0: usize = 256;
const SLOT_INTS: usize = 32;
const MAX_SLOTS: usize = 64;
/// Recordings kept per macro (contexts it is called in).
const MAX_VARIANTS: usize = 4;
// Slot fields.
const F_CS: usize = 0;
const F_STATE: usize = 1;
const F_MISMATCH: usize = 2;
const F_MODE: usize = 3;
const F_ALIGN: usize = 4;
const F_PAR_TOKEN: usize = 5;
const F_NRW: usize = 6;
const F_NRH: usize = 7;
const F_NPIN: usize = 8;
const F_NOPS: usize = 9;
const F_ABORTS: usize = 10;
const F_RECORDS: usize = 11;
const F_HITS: usize = 12;
const F_NEXT: usize = 13; // next variant, plus one
const F_HEAD: usize = 14; // the chain's first slot, plus one
const F_DIS: usize = 15; // (head) 1: neither replayed nor recorded
const F_NOREC: usize = 16; // (head) 1: no more recordings
const F_LAST: usize = 17; // call number of the last hit
const F_SITE: usize = 18; // 1: recorded at the argument site (MACRO-REPLAY)
const F_NARGW: usize = 19; // words of the argument key (`R_ARGS`)
                           // The argument site's slots (MACRO-REPLAY.md §3.3).
const F_BASE: usize = 20; // its region's first word in `intr_data`
const F_HLEN: usize = 21; // words of its block in the slot heap (0: the scratch region)
const F_INHASH: usize = 22; // 1: the argument index names it
const F_KEYHASH: usize = 23; // the hash of its macro and arguments
const F_NVAR: usize = 24; // (head) valid recordings at the argument site
const F_NDEAD: usize = 25; // (head) argument lists that could not be recorded

const SEEN_DEP: i32 = 1;
const SEEN_WRITTEN: i32 = 2;
const SEEN_WEAK: i32 = 3;
/// A watch record wanting only "not `\outer`, not a parameter character".
const WANT_PLAIN: i32 = -1;
const MAC_PARAM: i32 = 6;
const OUTER_CALL: i32 = 116;
const MATCHING: i32 = 3;

const ST_FREE: i32 = 0;
const ST_RECORDING: i32 = 1;
const ST_VALID: i32 = 2;
/// An argument-site slot that keeps only a key whose recording was
/// abandoned: the next call with it is expanded without recording again
/// (until a retry, for the reasons that allow one).
const ST_DEAD: i32 = 3;

/// How often a slot may be recorded afresh after its guard failed.
const RECORD_BUDGET: i32 = 64;
/// Abandoned recordings before a macro is left alone.
const ABORT_BUDGET: i32 = 3;
/// The argument site (MACRO-REPLAY.md §3.3): valid recordings per macro
/// (the least recently replayed is evicted), keys kept as not recordable
/// per macro, recordings per argument list (contexts), and recordings per
/// macro in all.
const MAX_VARIANTS_ARGS: i32 = 256;
const MAX_DEAD_ARGS: i32 = 256;
const KEY_VARIANTS: usize = 4;
const RECORD_BUDGET_ARGS: i32 = 4096;

// intr_data: watch records first, then one region per slot.
const WATCH_INTS: usize = 6; // slot, want type, want equiv, ok, next+1, loc
const WATCH_RECORDS: usize = 400_000;
const REGION0: usize = WATCH_INTS * WATCH_RECORDS;
const RW_CAP: usize = 2048; // (loc, value) pairs
const RH_CAP: usize = 30_000; // watch record indices
const PIN_CAP: usize = 16_000;
const OPS_CAP: usize = 10_000; // 4 ints each
/// The argument key: the count, then each argument's length and tokens.
const ARG_CAP: usize = 3_000;
const REGION_INTS: usize = 2 * RW_CAP + RH_CAP + PIN_CAP + 4 * OPS_CAP + ARG_CAP;
const R_RW: usize = 0;
const R_RH: usize = 2 * RW_CAP;
const R_PIN: usize = R_RH + RH_CAP;
const R_OPS: usize = R_PIN + PIN_CAP;
const R_ARGS: usize = R_OPS + 4 * OPS_CAP;
const _: () = assert!(REGION0 + MAX_SLOTS * REGION_INTS <= A_SLOTS0);
// The argument site's slots: ids from `MAX_SLOTS` on, their records in
// `intr_data` from `A_SLOTS0`, the argument index (open addressing: the
// slot plus one, 0 empty, -1 deleted; and the key's hash), the region a
// recording in progress fills, and the slot heap, where a committed
// recording is kept at its exact size (MACRO-REPLAY.md §3.3).
const ARG_SLOTS: usize = 4096;
const A_SLOTS0: usize = 8_388_608;
const HASH0: usize = A_SLOTS0 + ARG_SLOTS * SLOT_INTS;
const HASH_SIZE: usize = 8192;
const SCRATCH0: usize = HASH0 + 2 * HASH_SIZE;
const HEAP0: usize = SCRATCH0 + REGION_INTS;
// (changes/intrinsics.ch: intr_data_size = 16777215)
const HEAP_END: usize = 16_777_216;
const _: () = assert!(HEAP0 + REGION_INTS < HEAP_END && 2 * ARG_SLOTS <= HASH_SIZE);
// A slot's parts.
const P_RW: usize = 0;
const P_RH: usize = 1;
const P_PIN: usize = 2;
const P_OPS: usize = 3;
const P_ARGS: usize = 4;
const R_PARTS: [usize; 5] = [R_RW, R_RH, R_PIN, R_OPS, R_ARGS];

/// Where `slot`'s part `p` starts in `intr_data`, given its fields `sf`.
fn part_of(slot: usize, p: usize, sf: &dyn Fn(usize) -> i32) -> usize {
    if slot < MAX_SLOTS {
        return REGION0 + slot * REGION_INTS + R_PARTS[p];
    }
    let base = sf(F_BASE).max(0) as usize;
    if sf(F_HLEN) == 0 {
        return base + R_PARTS[p];
    }
    let n = |f: usize| sf(f).max(0) as usize;
    let sizes = [2 * n(F_NRW), n(F_NRH), n(F_NPIN), 4 * n(F_NOPS)];
    base + sizes[..p].iter().sum::<usize>()
}

// Operations.
const K_DEF: i32 = 1;
const K_FRESH: i32 = 5;
const K_LETCS: i32 = 6;
const K_WORD: i32 = 2;
const K_BEGIN: i32 = 3;
const K_END: i32 = 4;
const K_GLOBAL: i32 = 256;

/// A live word of `intr_data`, for the convergence test's structural
/// comparison (`crate::iso`): the words below may hold `mem` pointers,
/// which differ between two runs that allocated nodes elsewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiveWord {
    /// A plain value (an `eqtb` location, an integer, a count, an index).
    Value(usize),
    /// The `equiv` of the `eq_type` held at `ty`, as in `eqtb` (a watch
    /// record's wanted meaning, a `K_DEF` operation's value).
    Equiv { ty: usize, at: usize },
    /// A pinned token list (reference-count node).
    Tok(usize),
}

/// Every word of `intr_data` that the intrinsics can read again, given the
/// state's `intr_state` (`st`) and `intr_data` (`data`). Nothing reads any
/// other word before writing it: a watch record on the free list is read
/// only for its link (`watch_alloc`), and a slot's region only below its
/// counts (`guard`, `replay`, `slot_clear`, `intr_report_reads`, and
/// `flashtex_intr_touch` through the records the slots hold). `Err` if the
/// bookkeeping is out of range (then nothing may be taken as dead).
pub(crate) fn live_words(
    st: &dyn Fn(usize) -> i32,
    data: &dyn Fn(usize) -> i32,
    out: &mut dyn FnMut(LiveWord),
) -> Result<(), String> {
    let top = st(S_WATCH_TOP);
    if !(0..=WATCH_RECORDS as i32).contains(&top) {
        return Err(format!("intrinsics: {top} watch records"));
    }
    let record = |r: i32| -> Result<usize, String> {
        if (0..top).contains(&r) {
            Ok(r as usize * WATCH_INTS)
        } else {
            Err(format!("intrinsics: watch record {r} of {top}"))
        }
    };
    // the free records: their links
    let mut f = st(S_WATCH_FREE);
    let mut n = 0;
    while f > 0 {
        let b = record(f - 1)?;
        out(LiveWord::Value(b + 4));
        f = data(b + 4);
        n += 1;
        if n > top {
            return Err("intrinsics: the free watch records form a cycle".into());
        }
    }
    let slots = st(S_NSLOTS);
    if !(0..=MAX_SLOTS as i32).contains(&slots) {
        return Err(format!("intrinsics: {slots} slots"));
    }
    let atop = st(S_ATOP);
    if !(0..=ARG_SLOTS as i32).contains(&atop) {
        return Err(format!("intrinsics: {atop} argument-site slots"));
    }
    if atop > 0 {
        // the argument-site slots' records, and the argument index
        for i in 0..atop as usize * SLOT_INTS {
            out(LiveWord::Value(A_SLOTS0 + i));
        }
        for i in 0..2 * HASH_SIZE {
            out(LiveWord::Value(HASH0 + i));
        }
    }
    let field = |slot: usize, f: usize| -> i32 {
        if slot < MAX_SLOTS {
            st(SLOT0 + slot * SLOT_INTS + f)
        } else {
            data(A_SLOTS0 + (slot - MAX_SLOTS) * SLOT_INTS + f)
        }
    };
    let ids = (0..slots as usize).chain(MAX_SLOTS..MAX_SLOTS + atop as usize);
    for slot in ids {
        let count = |f: usize, cap: usize| -> Result<usize, String> {
            let n = field(slot, f);
            if (0..=cap as i32).contains(&n) {
                Ok(n as usize)
            } else {
                Err(format!("intrinsics: slot {slot} field {f} = {n}"))
            }
        };
        let base = |p: usize| part_of(slot, p, &|f| field(slot, f));
        let end = base(P_ARGS) + count(F_NARGW, ARG_CAP)?;
        if slot >= MAX_SLOTS && field(slot, F_HLEN) > 0 && end > HEAP_END {
            return Err(format!("intrinsics: slot {slot} beyond the heap"));
        }
        for i in 0..2 * count(F_NRW, RW_CAP)? {
            out(LiveWord::Value(base(P_RW) + i));
        }
        for i in 0..count(F_NRH, RH_CAP)? {
            out(LiveWord::Value(base(P_RH) + i));
            let b = record(data(base(P_RH) + i))?;
            for k in [0, 1, 3, 4, 5] {
                out(LiveWord::Value(b + k));
            }
            out(LiveWord::Equiv {
                ty: b + 1,
                at: b + 2,
            });
        }
        for i in 0..count(F_NPIN, PIN_CAP)? {
            out(LiveWord::Tok(base(P_PIN) + i));
        }
        for i in 0..count(F_NOPS, OPS_CAP)? {
            let o = base(P_OPS) + 4 * i;
            for k in 0..3 {
                out(LiveWord::Value(o + k));
            }
            out(match data(o) & 0xff {
                K_DEF => LiveWord::Equiv {
                    ty: o + 2,
                    at: o + 3,
                },
                K_FRESH => LiveWord::Tok(o + 3),
                _ => LiveWord::Value(o + 3),
            });
        }
        for i in 0..count(F_NARGW, ARG_CAP)? {
            out(LiveWord::Value(base(P_ARGS) + i));
        }
    }
    Ok(())
}

/// Why a recording was abandoned or a call was not replayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Why {
    // abandoned recordings
    Dimension,
    TokenCopy,
    Sparse,
    Checkpoint,
    Command,
    Expandable,
    Internal,
    Level,
    Cond,
    Group,
    Mark,
    Region,
    PointerType,
    UnpinnedList,
    Capacity,
    NewCs,
    Output,
    Error,
    Unbalanced,
    State,
    Parameters,
    /// the begin-document snapshot's arming changed (MACRO-REPLAY §3.1)
    Arm,
    // calls not replayed
    Tracing,
    GlobalDefs,
    AfterAssignment,
    Deps,
    WordDeps,
    Mode,
    Align,
    ParToken,
    Disabled,
    NoSlot,
    /// no valid recording (yet, or the macro is left alone)
    NotRecorded,
    /// the call expands the snapshot's arming control sequence
    ArmCs,
    /// no recording has these argument tokens
    ArgsDiffer,
    /// the arguments are too long to keep as a key
    ArgsCapacity,
    /// a recording with these arguments was abandoned before
    Unrecordable,
}

impl Why {
    fn from_code(r: i32) -> Why {
        match r {
            1 => Why::Dimension,
            2 => Why::TokenCopy,
            3 => Why::Sparse,
            4 => Why::Checkpoint,
            _ => Why::State,
        }
    }
}

/// Faults injected into replays by `FLASHTEX_INTRINSICS_FAULT`, to show
/// that the verifier catches them (never set outside that test).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fault {
    None,
    /// leave out the last recorded operation
    DropLast,
    /// leave out the first `\let` of a macro
    DropFirstLet,
    /// `\let` a macro without taking a reference to it
    NoRef,
    /// make global integer assignments local
    Local,
    /// do not tell L5's read-set what a replay reads
    NoReadset,
}

fn fault() -> Fault {
    static F: std::sync::OnceLock<Fault> = std::sync::OnceLock::new();
    *F.get_or_init(
        || match std::env::var("FLASHTEX_INTRINSICS_FAULT").as_deref() {
            Ok("drop-last") => Fault::DropLast,
            Ok("drop-first-let") => Fault::DropFirstLet,
            Ok("no-ref") => Fault::NoRef,
            Ok("local") => Fault::Local,
            Ok("no-readset") => Fault::NoReadset,
            _ => Fault::None,
        },
    )
}

fn first_let(ops: &[[i32; 4]]) -> usize {
    ops.iter()
        .position(|o| o[0] & 0xff == K_LETCS)
        .unwrap_or(usize::MAX)
}

/// Counters for the report (not part of the engine state).
#[derive(Default, Debug)]
pub struct Stats {
    pub calls: u64,
    pub replays: u64,
    pub replayed_ops: u64,
    /// Wall-clock nanoseconds spent replaying.
    pub replay_ns: u64,
    pub recordings: u64,
    pub committed: u64,
    pub abandoned: std::collections::BTreeMap<String, u64>,
    pub not_replayed: std::collections::BTreeMap<String, u64>,
    pub verified: u64,
    /// verifications given up for a checkpoint taken in the middle
    pub verify_skipped: u64,
    pub verify_differences: u64,
    pub verify_details: Vec<String>,
    pub per_cs: std::collections::BTreeMap<String, (u64, u64)>,
    /// `per_cs`'s replays by location, named at the end of the run.
    #[doc(hidden)]
    pub replays_by_cs: std::collections::HashMap<i32, u64>,
    /// The argument site's share of `calls`, `replays` and `committed`.
    pub args_calls: u64,
    pub args_replays: u64,
    pub args_committed: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Off,
    On,
    Verify,
    VerifyAll,
    /// `verify`, with every macro with parameters a candidate
    VerifyAllArgs,
}

struct Config {
    mode: Mode,
    /// macros with parameters are offered (`FLASHTEX_INTRINSICS_ARGS`)
    args: bool,
    names: Vec<Vec<u8>>,
    stats_out: Option<String>,
}

thread_local! {
    static CONFIG: RefCell<Option<Config>> = const { RefCell::new(None) };
    pub(crate) static STATS: RefCell<Stats> = RefCell::new(Stats::default());
}

/// The macros made intrinsics by default: measured in
/// docs/evidence/l6-intrinsics-2026-09-29/ (the profile's top entries that
/// pass the recording's purity test).
///
/// `XC@col@rlet` (xcolor's `\colorlet`, 47 % of a beamer deck's time) has
/// parameters: it is offered only with `FLASHTEX_INTRINSICS_ARGS=on`
/// (MACRO-REPLAY.md §8).
pub const DEFAULT_NAMES: &[&str] = &["pdfstringdefPreHook", "XC@col@rlet"];

fn config() -> (Mode, Vec<Vec<u8>>) {
    CONFIG.with(|c| {
        let mut c = c.borrow_mut();
        if c.is_none() {
            let mode = match std::env::var("FLASHTEX_INTRINSICS").as_deref() {
                Ok("off") | Ok("0") => Mode::Off,
                Ok("verify") => Mode::Verify,
                Ok("verify-all") => Mode::VerifyAll,
                Ok("verify-all-args") => Mode::VerifyAllArgs,
                _ => Mode::On,
            };
            let args = mode == Mode::VerifyAllArgs
                || matches!(
                    std::env::var("FLASHTEX_INTRINSICS_ARGS").as_deref(),
                    Ok("on") | Ok("1")
                );
            let names = match std::env::var("FLASHTEX_INTRINSIC_NAMES") {
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
                args,
                names,
                stats_out: std::env::var("FLASHTEX_INTRINSICS_STATS").ok(),
            });
        }
        let c = c.as_ref().unwrap();
        (c.mode, c.names.clone())
    })
}

pub fn mode() -> Mode {
    config().0
}

fn verifying() -> bool {
    matches!(mode(), Mode::Verify | Mode::VerifyAll | Mode::VerifyAllArgs)
}

fn args_enabled() -> bool {
    config();
    CONFIG.with(|c| c.borrow().as_ref().is_some_and(|c| c.args))
}

fn bump(map: &mut std::collections::BTreeMap<String, u64>, w: Why) {
    *map.entry(format!("{w:?}")).or_default() += 1;
}

impl Globals {
    // -- small accessors ---------------------------------------------------

    #[inline]
    fn st(&self, i: usize) -> i32 {
        self.intr_state[i]
    }
    #[inline]
    fn set_st(&mut self, i: usize, v: i32) {
        self.intr_state[i] = v;
    }
    #[inline]
    fn sf(&self, slot: usize, f: usize) -> i32 {
        if slot < MAX_SLOTS {
            self.intr_state[SLOT0 + slot * SLOT_INTS + f]
        } else {
            self.intr_data[A_SLOTS0 + (slot - MAX_SLOTS) * SLOT_INTS + f]
        }
    }
    #[inline]
    fn set_sf(&mut self, slot: usize, f: usize, v: i32) {
        if slot < MAX_SLOTS {
            self.intr_state[SLOT0 + slot * SLOT_INTS + f] = v;
        } else {
            self.intr_data[A_SLOTS0 + (slot - MAX_SLOTS) * SLOT_INTS + f] = v;
        }
    }
    /// Where `slot`'s part `p` (`P_RW` .. `P_ARGS`) starts in `intr_data`.
    #[inline]
    fn part(&self, slot: usize, p: usize) -> usize {
        part_of(slot, p, &|f| self.sf(slot, f))
    }
    #[inline]
    fn eq_type_of(&self, p: i32) -> i32 {
        self.eqtb[(p - 1) as usize].hh().b0()
    }
    #[inline]
    fn equiv_of(&self, p: i32) -> i32 {
        self.eqtb[(p - 1) as usize].hh().rh()
    }
    #[inline]
    fn int_par(&self, code: i32) -> i32 {
        let p = self.st(L_INT_BASE) + code;
        self.eqtb[(p - 1) as usize].int()
    }
    #[inline]
    fn link(&self, p: i32) -> i32 {
        self.mem[p as usize].hh().rh()
    }
    #[inline]
    fn info(&self, p: i32) -> i32 {
        self.mem[p as usize].hh().lh()
    }
    fn add_token_ref(&mut self, p: i32) {
        let v = self.mem[p as usize].hh().lh() + 1;
        self.mem[p as usize].set_hh_lh(v);
    }
    fn rec_slot(&self) -> Option<usize> {
        let s = self.st(S_REC_SLOT);
        (s > 0).then(|| (s - 1) as usize)
    }

    // -- set-up --------------------------------------------------------------

    /// `Set init`: is the feature on? (Also sets `intr_all`.)
    pub fn flashtex_intr_enabled(&mut self) -> bool {
        let (m, names) = config();
        self.intr_all = m == Mode::VerifyAll;
        self.intr_all_args = m == Mode::VerifyAllArgs;
        self.intr_args_on = m != Mode::Off && args_enabled();
        m != Mode::Off && (!names.is_empty() || self.intr_all || self.intr_all_args)
    }

    /// After a format is loaded: find the registered names it defines.
    pub fn flashtex_intr_loaded(&mut self) {
        let (_, names) = config();
        for n in names {
            if let Some(p) = self.find_cs_fast(&n) {
                self.make_candidate(p);
            }
        }
    }

    /// A control sequence was just entered into the hash (`id_lookup`).
    pub fn flashtex_intr_new_cs(&mut self, p: i32) {
        let (_, names) = config();
        if names.is_empty() {
            return;
        }
        let t = self.hash[(p - self.st(L_HASH_BASE)) as usize].rh();
        let (a, b) = (
            self.str_start[t as usize] as usize,
            self.str_start[t as usize + 1] as usize,
        );
        for n in &names {
            if b - a == n.len()
                && self.str_pool[a..b]
                    .iter()
                    .zip(n)
                    .all(|(&c, &d)| c == d as i32)
            {
                self.make_candidate(p);
            }
        }
    }

    /// The multi-letter control sequence called `name`, found as
    /// `id_lookup` finds it (§259-261), without entering it.
    fn find_cs_fast(&self, name: &[u8]) -> Option<i32> {
        if name.len() < 2 {
            return None;
        }
        let prime = self.st(L_HASH_PRIME) as i64;
        let hb = self.st(L_HASH_BASE);
        let mut h = name[0] as i64;
        for &c in &name[1..] {
            h = h + h + c as i64;
            while h >= prime {
                h -= prime;
            }
        }
        let mut p = h as i32 + hb;
        loop {
            let t = self.hash[(p - hb) as usize].rh();
            if t > 0 && t < self.str_ptr {
                let (a, b) = (
                    self.str_start[t as usize] as usize,
                    self.str_start[t as usize + 1] as usize,
                );
                if b - a == name.len()
                    && self.str_pool[a..b]
                        .iter()
                        .zip(name)
                        .all(|(&c, &n)| c == n as i32)
                {
                    return Some(p);
                }
            }
            let next = self.hash[(p - hb) as usize].lh();
            if next == 0 {
                return None;
            }
            p = next;
        }
    }

    /// The first slot (the head) of the chain of recordings of `p`,
    /// allocating it if `p` is new.
    fn make_candidate(&mut self, p: i32) -> Option<usize> {
        if self.intr_cand[p as usize] != 0 {
            return Some((self.intr_cand[p as usize] - 1) as usize);
        }
        let slot = self.alloc_slot()?;
        self.set_sf(slot, F_CS, p);
        self.set_sf(slot, F_HEAD, slot as i32 + 1);
        self.intr_cand[p as usize] = slot as i32 + 1;
        Some(slot)
    }

    /// The slots of the chain that starts at `head`.
    fn chain(&self, head: usize) -> Vec<usize> {
        let mut v = vec![head];
        let mut n = self.sf(head, F_NEXT);
        while n > 0 && v.len() <= MAX_VARIANTS {
            v.push((n - 1) as usize);
            n = self.sf((n - 1) as usize, F_NEXT);
        }
        v
    }

    fn alloc_slot(&mut self) -> Option<usize> {
        for s in 0..self.st(S_NSLOTS) as usize {
            if self.sf(s, F_CS) == 0 {
                return Some(s);
            }
        }
        let n = self.st(S_NSLOTS) as usize;
        if n < MAX_SLOTS {
            self.set_st(S_NSLOTS, n as i32 + 1);
            for f in 0..SLOT_INTS {
                self.set_sf(n, f, 0);
            }
            return Some(n);
        }
        // Free the chain of a macro that is left alone (verify-all).
        for s in 0..MAX_SLOTS {
            if self.sf(s, F_HEAD) == s as i32 + 1 && self.sf(s, F_DIS) != 0 {
                let cs = self.sf(s, F_CS);
                self.intr_cand[cs as usize] = 0;
                for c in self.chain(s) {
                    self.slot_clear(c);
                    for f in 0..SLOT_INTS {
                        self.set_sf(c, f, 0);
                    }
                }
                return Some(s);
            }
        }
        None
    }

    // -- watch records -------------------------------------------------------

    fn watch_alloc(&mut self) -> Option<usize> {
        let f = self.st(S_WATCH_FREE);
        if f > 0 {
            let r = (f - 1) as usize;
            let next = self.intr_data[r * WATCH_INTS + 4];
            self.set_st(S_WATCH_FREE, next);
            return Some(r);
        }
        let top = self.st(S_WATCH_TOP) as usize;
        if top >= WATCH_RECORDS {
            return None;
        }
        self.set_st(S_WATCH_TOP, top as i32 + 1);
        Some(top)
    }

    /// Watch `eqtb[p]` for `slot`, wanting its current value (or, if
    /// `plain`, only a meaning that is neither `\outer` nor `#`).
    fn watch_add(&mut self, slot: usize, p: i32, plain: bool) -> bool {
        let n = self.sf(slot, F_NRH) as usize;
        if n >= RH_CAP {
            return false;
        }
        let Some(r) = self.watch_alloc() else {
            return false;
        };
        let b = r * WATCH_INTS;
        let head = if plain {
            self.intr_wplain[p as usize]
        } else {
            self.intr_watch[p as usize].max(0)
        };
        self.intr_data[b] = slot as i32;
        self.intr_data[b + 1] = if plain {
            WANT_PLAIN
        } else {
            self.eq_type_of(p)
        };
        self.intr_data[b + 2] = if plain { 0 } else { self.equiv_of(p) };
        self.intr_data[b + 3] = self.holds(p, self.intr_data[b + 1], self.intr_data[b + 2]) as i32;
        if self.intr_data[b + 3] == 0 {
            let m = self.sf(slot, F_MISMATCH) + 1;
            self.set_sf(slot, F_MISMATCH, m);
        }
        self.intr_data[b + 4] = head;
        self.intr_data[b + 5] = p;
        self.set_watch_head(p, plain, r as i32 + 1);
        let o = self.part(slot, P_RH);
        self.intr_data[o + n] = r as i32;
        self.set_sf(slot, F_NRH, n as i32 + 1);
        true
    }

    /// Do the token lists `a` and `b` (reference-count nodes) hold the same
    /// tokens? TeX never looks at an address, only at the tokens.
    pub(crate) fn same_tokens(&self, a: i32, b: i32) -> bool {
        let (mut a, mut b) = (self.link(a), self.link(b));
        while a != 0 && b != 0 {
            if self.info(a) != self.info(b) {
                return false;
            }
            a = self.link(a);
            b = self.link(b);
        }
        a == b
    }

    /// Does `eqtb[p]` hold the meaning (`t`, `e`) -- the same value, or,
    /// for a macro or token list, the same tokens?
    fn holds(&self, p: i32, t: i32, e: i32) -> bool {
        let (t2, e2) = (self.eq_type_of(p), self.equiv_of(p));
        if t == WANT_PLAIN {
            return t2 != MAC_PARAM && t2 != OUTER_CALL && t2 != LONG_OUTER_CALL;
        }
        t2 == t
            && (e2 == e
                || ((CALL..=LONG_OUTER_CALL).contains(&t)
                    && e != 0
                    && e2 != 0
                    && self.same_tokens(e, e2)))
    }

    /// The first watch record (plus one) of `eqtb[p]` that wants a value,
    /// or (`plain`) only a plain meaning: two lists. `intr_watch[p]` is the
    /// first's head, or -1 when only the second is not empty, so that it is
    /// nonzero whenever a write must be reported (changes/intrinsics.ch).
    fn watch_head(&self, p: i32, plain: bool) -> i32 {
        if plain {
            self.intr_wplain[p as usize]
        } else {
            self.intr_watch[p as usize].max(0)
        }
    }

    fn set_watch_head(&mut self, p: i32, plain: bool, v: i32) {
        let (mut value, mut pl) = (self.watch_head(p, false), self.watch_head(p, true));
        if plain {
            pl = v;
        } else {
            value = v;
        }
        self.intr_wplain[p as usize] = pl;
        self.intr_watch[p as usize] = if value > 0 {
            value
        } else if pl > 0 {
            -1
        } else {
            0
        };
    }

    /// `eqtb[p]`, which some recording watches, was just written.
    pub fn flashtex_intr_touch(&mut self, p: i32) {
        // The watchers that want only a plain meaning (neither `\outer`
        // nor `#`: a token a recording only passed on) all hold or all do
        // not: the first tells whether any changes.
        let r = self.intr_wplain[p as usize];
        if r > 0 {
            let now = self.holds(p, WANT_PLAIN, 0) as i32;
            if now != self.intr_data[(r - 1) as usize * WATCH_INTS + 3] {
                let mut r = r;
                while r > 0 {
                    let b = (r - 1) as usize * WATCH_INTS;
                    self.intr_data[b + 3] = now;
                    let slot = self.intr_data[b] as usize;
                    let m = self.sf(slot, F_MISMATCH) + if now == 1 { -1 } else { 1 };
                    self.set_sf(slot, F_MISMATCH, m);
                    r = self.intr_data[b + 4];
                }
            }
        }
        // Many recordings watch an entry for the same value (the same
        // pinned list): `holds` once per wanted value (a macro's is a
        // comparison of tokens).
        let mut memo: [(i32, i32, bool); 8] = [(0, 0, false); 8];
        let mut known = 0;
        let mut r = self.watch_head(p, false);
        while r > 0 {
            let b = (r - 1) as usize * WATCH_INTS;
            let (t, e) = (self.intr_data[b + 1], self.intr_data[b + 2]);
            let now = match memo[..known].iter().find(|m| m.0 == t && m.1 == e) {
                Some(m) => m.2,
                None => {
                    let h = self.holds(p, t, e);
                    if known < memo.len() {
                        memo[known] = (t, e, h);
                        known += 1;
                    }
                    h
                }
            } as i32;
            if now != self.intr_data[b + 3] {
                self.intr_data[b + 3] = now;
                let slot = self.intr_data[b] as usize;
                let m = self.sf(slot, F_MISMATCH) + if now == 1 { -1 } else { 1 };
                self.set_sf(slot, F_MISMATCH, m);
            }
            r = self.intr_data[b + 4];
        }
    }

    /// Drop everything `slot` recorded: its watch records and pins.
    fn slot_clear(&mut self, slot: usize) {
        let (rh, pins) = (self.part(slot, P_RH), self.part(slot, P_PIN));
        for i in 0..self.sf(slot, F_NRH) as usize {
            let r = self.intr_data[rh + i] as usize;
            let b = r * WATCH_INTS;
            let p = self.intr_data[b + 5];
            // unlink r from p's list
            let target = r as i32 + 1;
            let plain = self.intr_data[b + 1] == WANT_PLAIN;
            if self.watch_head(p, plain) == target {
                self.set_watch_head(p, plain, self.intr_data[b + 4]);
            } else {
                let mut q = self.watch_head(p, plain);
                while q > 0 {
                    let qb = (q - 1) as usize * WATCH_INTS;
                    if self.intr_data[qb + 4] == target {
                        self.intr_data[qb + 4] = self.intr_data[b + 4];
                        break;
                    }
                    q = self.intr_data[qb + 4];
                }
            }
            self.intr_data[b + 4] = self.st(S_WATCH_FREE);
            self.set_st(S_WATCH_FREE, target);
        }
        for i in 0..self.sf(slot, F_NPIN) as usize {
            let p = self.intr_data[pins + i];
            self.delete_token_ref(p);
        }
        if slot >= MAX_SLOTS {
            let h = self.sf(slot, F_HLEN);
            if h > 0 {
                let live = self.st(S_HLIVE) - h;
                self.set_st(S_HLIVE, live);
                self.set_sf(slot, F_HLEN, 0);
            }
            self.set_sf(slot, F_BASE, SCRATCH0 as i32);
        }
        for f in [F_MISMATCH, F_NRW, F_NRH, F_NPIN, F_NOPS, F_NARGW] {
            self.set_sf(slot, f, 0);
        }
    }

    fn pin(&mut self, slot: usize, p: i32) -> bool {
        let n = self.sf(slot, F_NPIN) as usize;
        if n >= PIN_CAP {
            return false;
        }
        self.add_token_ref(p);
        let o = self.part(slot, P_PIN);
        self.intr_data[o + n] = p;
        self.set_sf(slot, F_NPIN, n as i32 + 1);
        true
    }

    fn push_op(&mut self, slot: usize, k: i32, a: i32, b: i32, c: i32) -> bool {
        let n = self.sf(slot, F_NOPS) as usize;
        if n >= OPS_CAP {
            return false;
        }
        let o = self.part(slot, P_OPS) + 4 * n;
        self.intr_data[o] = k;
        self.intr_data[o + 1] = a;
        self.intr_data[o + 2] = b;
        self.intr_data[o + 3] = c;
        self.set_sf(slot, F_NOPS, n as i32 + 1);
        true
    }

    // -- recording -----------------------------------------------------------

    fn seen(&self, p: i32) -> i32 {
        let v = self.intr_seen[p as usize];
        if v >> 2 == self.st(S_SERIAL) {
            v & 3
        } else {
            0
        }
    }

    fn set_seen(&mut self, p: i32, st: i32) {
        self.intr_seen[p as usize] = (self.st(S_SERIAL) << 2) | st;
    }

    /// Is `eqtb[p]` an integer or dimension (regions 5 and 6)? Above
    /// `eqtb_size` are control sequences again (tex.ch's `hash_extra`,
    /// changes/web2c.ch).
    fn is_int_slot(&self, p: i32) -> bool {
        p >= self.st(L_INT_BASE) && p <= self.st(L_EQTB_SIZE)
    }

    /// May a recording touch `eqtb[p]` at all? (Glue, the paragraph shape,
    /// box registers and font identifiers hold pointers it does not track,
    /// or are written by routines the watch does not see.)
    fn rec_region_ok(&mut self, p: i32) -> bool {
        let glue = self.st(L_GLUE_BASE);
        let local = self.st(L_LOCAL_BASE);
        let boxb = self.st(L_BOX_BASE);
        let font_id = self.st(L_FONT_ID_BASE);
        let undef = self.st(L_UNDEFINED_CONTROL_SEQUENCE);
        if p <= 0
            || p > self.st(L_EQTB_TOP)
            || (p >= font_id && p <= undef)
            || (p >= glue && p <= local)
            || (p >= boxb && p < boxb + 256)
        {
            self.rec_abort(Why::Region);
            return false;
        }
        true
    }

    /// The recorded run reads `eqtb[p]`. Unless it wrote the entry before
    /// (and the entry does not hold the value it had then), what it holds
    /// is a dependency: watched (below `int_base`) or kept as a pair.
    fn rec_read(&mut self, p: i32) {
        let Some(slot) = self.rec_slot() else { return };
        match self.seen(p) {
            SEEN_DEP => return,
            SEEN_WEAK => {}
            SEEN_WRITTEN => {
                // Derived from the recorded run's own changes unless an
                // `\endgroup` has brought back what the entry held before.
                let pre = self.intr_pre[p as usize];
                let same = if self.is_int_slot(p) {
                    pre.int() == self.eqtb[(p - 1) as usize].int()
                } else {
                    pre.hh().b0() == self.eq_type_of(p) && pre.hh().rh() == self.equiv_of(p)
                };
                if !same {
                    return;
                }
            }
            _ => {
                if !self.rec_region_ok(p) {
                    return;
                }
            }
        }
        self.set_seen(p, SEEN_DEP);
        if self.is_int_slot(p) {
            let n = self.sf(slot, F_NRW) as usize;
            if n >= RW_CAP {
                return self.rec_abort(Why::Capacity);
            }
            let o = self.part(slot, P_RW) + 2 * n;
            self.intr_data[o] = p;
            self.intr_data[o + 1] = self.eqtb[(p - 1) as usize].int();
            self.set_sf(slot, F_NRW, n as i32 + 1);
            return;
        }
        let (t, e) = (self.eq_type_of(p), self.equiv_of(p));
        if t == GLUE_REF || t == SHAPE_REF || t == BOX_REF {
            return self.rec_abort(Why::PointerType);
        }
        if (CALL..=LONG_OUTER_CALL).contains(&t) && e != 0 && !self.pin(slot, e) {
            return self.rec_abort(Why::Capacity);
        }
        if !self.watch_add(slot, p, false) {
            self.rec_abort(Why::Capacity);
        }
    }

    /// The recorded run reads the token `eqtb[p]` names, but looks only at
    /// the token, not its meaning (a macro argument, a parameter text or
    /// unexpanded body, the control sequence being defined): the meaning
    /// matters only if it is `\outer` (an error) or `#`.
    fn rec_read_weak(&mut self, p: i32) {
        let Some(slot) = self.rec_slot() else { return };
        if self.seen(p) != 0 {
            return;
        }
        if self.is_int_slot(p) || !self.rec_region_ok(p) {
            return;
        }
        let t = self.eq_type_of(p);
        if t == MAC_PARAM || t == OUTER_CALL || t == LONG_OUTER_CALL {
            return self.rec_abort(Why::Command);
        }
        self.set_seen(p, SEEN_WEAK);
        if !self.watch_add(slot, p, true) {
            self.rec_abort(Why::Capacity);
        }
    }

    /// The recorded run is about to change `eqtb[p]`. What it held is not
    /// a dependency (only `eq_define`'s own bookkeeping looks at it, and a
    /// replay calls `eq_define`), unless the run reads it back later.
    fn rec_write(&mut self, p: i32) {
        let st = self.seen(p);
        if (st != 0 && st != SEEN_WEAK) || !self.rec_region_ok(p) {
            return;
        }
        self.set_seen(p, SEEN_WRITTEN);
        let w = self.eqtb[(p - 1) as usize];
        self.intr_pre[p as usize] = w;
    }

    fn rec_catcodes(&mut self) {
        if self.st(S_REC_CATCODES) == 0 {
            self.set_st(S_REC_CATCODES, 1);
            let b = self.st(L_CAT_CODE_BASE);
            for c in 0..256 {
                if self.intr_rec_on {
                    self.rec_read(b + c);
                }
            }
        }
    }

    /// Start recording `slot`: `cs` is the macro, `scanner` the
    /// `scanner_status` its body runs with (at the argument site neither is
    /// what `cur_cs` and `scanner_status` hold: MACRO-REPLAY.md §3.1).
    fn rec_start(&mut self, slot: usize, verify: bool, cs: i32, scanner: i32) {
        let serial = self.st(S_SERIAL).wrapping_add(1).max(1);
        self.set_st(S_SERIAL, serial);
        self.set_st(S_REC_SLOT, slot as i32 + 1);
        self.set_st(S_REC_BASE, 0);
        self.set_st(S_REC_LEVEL, self.cur_level);
        self.set_st(S_REC_COND, self.cond_ptr);
        self.set_st(S_REC_IF_LIMIT, self.if_limit);
        self.set_st(S_REC_CUR_IF, self.cur_if);
        self.set_st(S_REC_IF_LINE, self.if_line);
        self.set_st(S_REC_ALIGN, self.align_state);
        self.set_st(S_REC_STR_PTR, self.str_ptr);
        self.set_st(S_REC_POOL_PTR, self.pool_ptr);
        self.set_st(S_REC_HASH_USED, self.hash_used);
        self.set_st(S_REC_ERRORS, self.error_count);
        self.set_st(S_REC_HISTORY, self.history);
        self.set_st(S_REC_FILE_OFF, self.file_offset);
        self.set_st(S_REC_TERM_OFF, self.term_offset);
        self.set_st(S_REC_MODE, self.cur_list.mode_field.abs());
        self.set_st(S_REC_NEST_PTR, self.nest_ptr);
        self.set_st(S_REC_TAIL, self.cur_list.tail_field);
        self.set_st(S_REC_SCANNER, scanner);
        self.set_st(S_REC_ARM_CS, self.ckpt_arm_cs);
        self.set_st(S_REC_ARM_LEVEL, self.ckpt_arm_level);
        self.set_st(S_REC_REQUEST, self.ckpt_request);
        self.set_st(S_REC_VERIFY, verify as i32);
        self.set_st(S_REC_CATCODES, 0);
        let len = self.log_len();
        self.set_st(S_REC_LOG_LO, len as u32 as i32);
        self.set_st(S_REC_LOG_HI, (len >> 32) as u32 as i32);
        if !verify {
            self.slot_clear(slot);
            self.set_sf(slot, F_STATE, ST_RECORDING);
            self.set_sf(slot, F_MODE, self.cur_list.mode_field.abs());
            self.set_sf(slot, F_ALIGN, self.align_state);
            self.set_sf(slot, F_PAR_TOKEN, self.par_token);
            if slot < MAX_SLOTS {
                self.set_sf(slot, F_SITE, 0);
            }
        }
        self.intr_rec_on = true;
        STATS.with(|s| s.borrow_mut().recordings += 1);
        if !verify {
            // The macro's own meaning, and \escapechar (printing).
            self.rec_read(cs);
            let ec = self.st(L_INT_BASE) + ESCAPE_CHAR_CODE;
            self.rec_read(ec);
        }
    }

    fn log_len(&mut self) -> u64 {
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

    fn rec_abort(&mut self, why: Why) {
        if !self.intr_rec_on {
            return;
        }
        self.intr_rec_on = false;
        let Some(slot) = self.rec_slot() else { return };
        self.set_st(S_REC_SLOT, 0);
        STATS.with(|s| bump(&mut s.borrow_mut().abandoned, why));
        if std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some() {
            let cs = self.sf(slot, F_CS);
            eprintln!(
                "intrinsics: recording of \\{} abandoned: {why:?} (cmd {} chr {} cs {})",
                self.cs_name_string(cs),
                self.cur_cmd,
                self.cur_chr,
                if self.cur_cs > 0 {
                    self.cs_name_string(self.cur_cs)
                } else {
                    String::new()
                }
            );
        }
        if self.st(S_REC_VERIFY) == 1 {
            if why == Why::Checkpoint {
                // (not a difference: the run goes on along the normal path)
                crate::intrinsics_verify::verify_skipped(self);
                return;
            }
            // A valid slot whose run this time was not pure: the guard let
            // through a call it should not have. A difference.
            crate::intrinsics_verify::verify_aborted(self, slot, why);
            return;
        }
        if slot >= MAX_SLOTS {
            return self.arg_dead(slot, why);
        }
        self.slot_clear(slot);
        self.set_sf(slot, F_STATE, ST_FREE);
        let head = (self.sf(slot, F_HEAD) - 1) as usize;
        let a = self.sf(head, F_ABORTS) + 1;
        self.set_sf(head, F_ABORTS, a);
        // The first run may enter new control sequences (\csname); a
        // checkpoint may interrupt a recording. Anything else is structural.
        let retry = matches!(why, Why::Checkpoint | Why::NewCs | Why::Arm);
        if !retry || a >= ABORT_BUDGET {
            self.set_sf(head, F_NOREC, 1);
        }
    }

    /// `flashtex_intr_abort(r)`: the recorded run did something impure.
    pub fn flashtex_intr_abort(&mut self, r: i32) {
        self.rec_abort(Why::from_code(r));
    }

    pub fn flashtex_intr_group(&mut self, c: i32) {
        if c != SIMPLE_GROUP && c != SEMI_SIMPLE_GROUP {
            return self.rec_abort(Why::Group);
        }
        if let Some(slot) = self.rec_slot() {
            if !self.rec_verify_or_push(slot, K_BEGIN, c, 0, 0) {
                self.rec_abort(Why::Capacity);
            }
        }
    }

    pub fn flashtex_intr_unsave(&mut self) {
        if self.cur_level <= self.st(S_REC_LEVEL) {
            return self.rec_abort(Why::Group);
        }
        if let Some(slot) = self.rec_slot() {
            if !self.rec_verify_or_push(slot, K_END, 0, 0, 0) {
                self.rec_abort(Why::Capacity);
            }
        }
    }

    /// Record an operation, or, in a verification run, only count it.
    fn rec_verify_or_push(&mut self, slot: usize, k: i32, a: i32, b: i32, c: i32) -> bool {
        if self.st(S_REC_VERIFY) == 1 {
            crate::intrinsics_verify::note_op(k, a, b, c);
            return true;
        }
        self.push_op(slot, k, a, b, c)
    }

    /// `eq_define` (k=0), `eq_word_define` (1), `geq_define` (2),
    /// `geq_word_define` (3), before they change `eqtb[p]`.
    ///
    /// A macro or token list value becomes one of two operations: a list
    /// just made by `\def`, `\edef` or a braced token assignment (reference
    /// count null) is kept as a template (`K_FRESH`: a replay defines a
    /// fresh copy, as the run did); a list shared with the control sequence
    /// just read (`\let`, `\futurelet`) becomes `K_LETCS`, which a replay
    /// takes from that control sequence as it is then. Other values are
    /// replayed as they are (`K_DEF`, `K_WORD`).
    pub fn flashtex_intr_def(&mut self, p: i32, t: i32, e: i32, k: i32) {
        let Some(slot) = self.rec_slot() else { return };
        let verify = self.st(S_REC_VERIFY) == 1;
        if !verify {
            self.rec_write(p);
            if !self.intr_rec_on {
                return;
            }
        }
        let global = if k >= 2 { K_GLOBAL } else { 0 };
        if k == 1 || k == 3 {
            if !self.rec_verify_or_push(slot, K_WORD | global, p, e, 0) {
                self.rec_abort(Why::Capacity);
            }
            return;
        }
        if t == GLUE_REF || t == SHAPE_REF || t == BOX_REF {
            return self.rec_abort(Why::PointerType);
        }
        if (t == REGISTER || t == TOKS_REGISTER) && !(MEM_BOT..=LO_MEM_STAT_MAX).contains(&e) {
            return self.rec_abort(Why::Sparse);
        }
        let (op, a2) = if (CALL..=LONG_OUTER_CALL).contains(&t) && e != 0 {
            if self.info(e) == 0 {
                if !verify && !self.pin(slot, e) {
                    return self.rec_abort(Why::Capacity);
                }
                (K_FRESH, e)
            } else {
                let src = self.cur_cs;
                if src == 0 || self.eq_type_of(src) != t || self.equiv_of(src) != e {
                    return self.rec_abort(Why::UnpinnedList);
                }
                (K_LETCS, src)
            }
        } else {
            (K_DEF, e)
        };
        if !self.rec_verify_or_push(slot, op | global, p, t, a2) {
            self.rec_abort(Why::Capacity);
        }
    }

    /// The end of `get_next`, during a recording.
    pub fn flashtex_intr_next(&mut self) {
        let base = self.st(S_REC_BASE);
        if base == 0 || self.input_ptr < base {
            return self.rec_abort(Why::Level);
        }
        if self.cur_cs != 0 {
            if self.cur_cmd == TOP_BOT_MARK {
                return self.rec_abort(Why::Mark);
            }
            if self.st(S_REC_VERIFY) == 0 {
                if self.intr_weak || self.scanner_status == MATCHING {
                    self.rec_read_weak(self.cur_cs);
                } else {
                    self.rec_read(self.cur_cs);
                }
            }
        }
    }

    /// `macro_call` fed a body to the scanner, during a recording.
    pub fn flashtex_intr_fed(&mut self) {
        if self.st(S_REC_BASE) == 0 {
            self.set_st(S_REC_BASE, self.input_ptr);
        }
    }

    pub fn flashtex_intr_read(&mut self, p: i32) {
        if self.st(S_REC_VERIFY) == 0 {
            if p == self.st(L_UNDEFINED_CONTROL_SEQUENCE) {
                return self.rec_abort(Why::Region);
            }
            self.rec_read(p);
        }
    }

    /// `expand`, at `reswitch`, during a recording.
    pub fn flashtex_intr_expand(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        let ok = match c {
            c if (CALL..END_TEMPLATE).contains(&c) => true,
            EXPAND_AFTER | FI_OR_ELSE | CS_NAME => true,
            NO_EXPAND => chr == 0,
            // \if \ifcat \ifnum \ifodd \ifx \iftrue \iffalse \ifcase
            // \ifdefined \ifcsname \ifpdfabsnum (and \unless of them)
            IF_TEST => matches!(chr % 32, 0 | 1 | 2 | 4 | 12 | 14 | 15 | 16 | 17 | 18 | 22),
            // \number \romannumeral \string \meaning \eTeXrevision
            // \expanded \pdftexrevision \pdftexbanner \pdfescapestring
            // \pdfescapename \pdfstrcmp \pdfescapehex \pdfunescapehex
            CONVERT => matches!(chr, 0 | 1 | 2 | 3 | 5 | 6 | 7 | 8 | 14 | 15 | 18 | 20 | 21),
            // \the \unexpanded \detokenize
            THE => matches!(chr, 0 | 1 | 5),
            _ => false,
        };
        if !ok {
            return self.rec_abort(Why::Expandable);
        }
        // printing a token list reads the catcodes (print_cs)
        if ((c == CONVERT && matches!(chr, 3 | 14 | 15 | 18 | 20)) || (c == THE && chr == 5))
            && self.st(S_REC_VERIFY) == 0
        {
            self.rec_catcodes();
        }
    }

    /// `scan_something_internal`, during a recording.
    pub fn flashtex_intr_internal(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        match c {
            DEF_CODE | CHAR_GIVEN | MATH_GIVEN => {}
            ASSIGN_TOKS | ASSIGN_INT | ASSIGN_DIMEN => {
                if self.st(S_REC_VERIFY) == 0 {
                    self.rec_read(chr)
                }
            }
            // \eTeXversion, \pdftexversion, \numexpr
            LAST_ITEM if matches!(chr, 20 | 6 | 39) => {}
            _ => self.rec_abort(Why::Internal),
        }
    }

    /// `main_control` at `reswitch`, and `prefixed_command` once the
    /// prefixes are read, during a recording.
    pub fn flashtex_intr_command(&mut self) {
        let (c, chr) = (self.cur_cmd, self.cur_chr);
        let m = self.cur_list.mode_field.abs();
        let inside = self.cur_level > self.st(S_REC_LEVEL);
        let ok = if c > MAX_NON_PREFIXED_COMMAND {
            match c {
                TOKS_REGISTER | ASSIGN_TOKS | ASSIGN_INT | DEF_CODE | REGISTER | ADVANCE
                | MULTIPLY | DIVIDE | PREFIX | LET | DEF => true,
                // \chardef \mathchardef \countdef \dimendef \skipdef
                // \muskipdef \toksdef
                SHORTHAND_DEF => (0..=6).contains(&chr),
                _ => false,
            }
        } else {
            match c {
                RELAX | BEGIN_GROUP => true,
                LEFT_BRACE => m != MMODE,
                RIGHT_BRACE => self.cur_group == SIMPLE_GROUP && inside,
                END_GROUP => self.cur_group == SEMI_SIMPLE_GROUP && inside,
                SPACER => m == VMODE || m == MMODE,
                IGNORE_SPACES => chr == 0,
                _ => false,
            }
        };
        if !ok {
            self.rec_abort(Why::Command);
        }
    }

    /// Popping the condition stack, during a recording.
    pub fn flashtex_intr_pop_cond(&mut self) {
        if self.cond_ptr == self.st(S_REC_COND) {
            self.rec_abort(Why::Cond);
        }
    }

    /// Is the recorded body used up (every input level from the body's on
    /// is a token list with nothing left)?
    fn rec_exhausted(&self) -> bool {
        let base = self.st(S_REC_BASE);
        if base == 0 {
            return false;
        }
        if self.input_ptr < base {
            return true;
        }
        (base..=self.input_ptr).all(|k| {
            let l = if k == self.input_ptr {
                self.cur_input
            } else {
                self.input_stack[k as usize]
            };
            l.state_field == 0 && l.loc_field == 0 && l.index_field != V_TEMPLATE
        })
    }

    /// `big_switch`, during a recording: finish it when the body is done.
    pub fn flashtex_intr_switch(&mut self) {
        if !self.rec_exhausted() {
            return;
        }
        let Some(slot) = self.rec_slot() else {
            self.intr_rec_on = false;
            return;
        };
        let why = if self.cur_level != self.st(S_REC_LEVEL) {
            Some(Why::Group)
        } else if self.cond_ptr != self.st(S_REC_COND)
            || self.if_limit != self.st(S_REC_IF_LIMIT)
            || self.cur_if != self.st(S_REC_CUR_IF)
            || self.if_line != self.st(S_REC_IF_LINE)
        {
            Some(Why::Cond)
        } else if self.align_state != self.st(S_REC_ALIGN) {
            Some(Why::Unbalanced)
        } else if self.str_ptr != self.st(S_REC_STR_PTR)
            || self.hash_used != self.st(S_REC_HASH_USED)
            || self.pool_ptr != self.st(S_REC_POOL_PTR)
        {
            Some(Why::NewCs)
        } else if self.error_count != self.st(S_REC_ERRORS)
            || self.history != self.st(S_REC_HISTORY)
        {
            Some(Why::Error)
        } else if self.ckpt_arm_cs != self.st(S_REC_ARM_CS)
            || self.ckpt_arm_level != self.st(S_REC_ARM_LEVEL)
            || self.ckpt_request != self.st(S_REC_REQUEST)
        {
            Some(Why::Arm)
        } else if self.after_token != 0
            || self.cur_list.mode_field.abs() != self.st(S_REC_MODE)
            || self.nest_ptr != self.st(S_REC_NEST_PTR)
            || self.cur_list.tail_field != self.st(S_REC_TAIL)
            || self.scanner_status != self.st(S_REC_SCANNER)
        {
            Some(Why::State)
        } else {
            let len = self.log_len();
            let was = (self.st(S_REC_LOG_LO) as u32 as u64)
                | ((self.st(S_REC_LOG_HI) as u32 as u64) << 32);
            if len != was
                || self.file_offset != self.st(S_REC_FILE_OFF)
                || self.term_offset != self.st(S_REC_TERM_OFF)
            {
                Some(Why::Output)
            } else {
                None
            }
        };
        if let Some(w) = why {
            return self.rec_abort(w);
        }
        if self.st(S_REC_VERIFY) == 0 && slot >= MAX_SLOTS && !self.arg_commit(slot) {
            return self.rec_abort(Why::Capacity);
        }
        self.intr_rec_on = false;
        self.set_st(S_REC_SLOT, 0);
        if self.st(S_REC_VERIFY) == 1 {
            crate::intrinsics_verify::normal_path_done(self, slot);
            return;
        }
        self.set_sf(slot, F_STATE, ST_VALID);
        let args = self.sf(slot, F_SITE) != 0;
        STATS.with(|s| {
            let mut s = s.borrow_mut();
            s.committed += 1;
            s.args_committed += args as u64;
        });
        if std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some() {
            let ops = self.intr_slot_ops(slot);
            let mut kinds = [0usize; 8];
            for o in &ops {
                kinds[(o[0] & 7) as usize] += 1;
            }
            let globals = ops.iter().filter(|o| o[0] & K_GLOBAL != 0).count();
            eprintln!(
                "intrinsics: op kinds def/word/begin/end/fresh/letcs = {}/{}/{}/{}/{}/{}, global {globals}, last {:?}",
                kinds[1], kinds[2], kinds[3], kinds[4], kinds[5], kinds[6], ops.last()
            );
            eprintln!(
                "intrinsics: recorded \\{}: {} ops, {} watched, {} integers, {} pins, {} now differ",
                self.cs_name_string(self.sf(slot, F_CS)),
                self.sf(slot, F_NOPS),
                self.sf(slot, F_NRH),
                self.sf(slot, F_NRW),
                self.sf(slot, F_NPIN),
                self.sf(slot, F_MISMATCH)
            );
        }
    }

    // -- calls -----------------------------------------------------------------

    fn no_replay(&mut self, why: Why) -> bool {
        STATS.with(|s| bump(&mut s.borrow_mut().not_replayed, why));
        false
    }

    /// The guard: may `slot` be replayed now?
    fn guard(&self, slot: usize) -> Result<(), Why> {
        if self.sf(slot, F_MISMATCH) != 0 {
            return Err(Why::Deps);
        }
        if self.cur_list.mode_field.abs() != self.sf(slot, F_MODE) {
            return Err(Why::Mode);
        }
        if self.align_state != self.sf(slot, F_ALIGN) {
            return Err(Why::Align);
        }
        if self.par_token != self.sf(slot, F_PAR_TOKEN) {
            return Err(Why::ParToken);
        }
        let base = self.part(slot, P_RW);
        for i in 0..self.sf(slot, F_NRW) as usize {
            let p = self.intr_data[base + 2 * i];
            if self.eqtb[(p - 1) as usize].int() != self.intr_data[base + 2 * i + 1] {
                return Err(Why::WordDeps);
            }
        }
        Ok(())
    }

    /// `macro_call`, entered from `big_switch`'s `get_x_token` for a
    /// candidate: replay it (and return true), or let it expand.
    pub fn flashtex_intr_call(&mut self) -> bool {
        if self.intr_rec_on {
            return false;
        }
        let cs = self.cur_cs;
        // Only macros without parameters: one with a parameter text is the
        // argument site's (`flashtex_intr_call_args`, MACRO-REPLAY.md §3.1).
        let mut r = self.link(self.cur_chr);
        if self.info(r) == PROTECTED_TOKEN {
            r = self.link(r);
        }
        if self.info(r) != END_MATCH_TOKEN {
            return false;
        }
        let slot = if self.intr_cand[cs as usize] != 0 {
            (self.intr_cand[cs as usize] - 1) as usize
        } else if self.intr_all {
            match self.make_candidate(cs) {
                Some(s) => s,
                None => return self.no_replay(Why::NoSlot),
            }
        } else {
            return false;
        };
        let head = slot;
        STATS.with(|s| s.borrow_mut().calls += 1);
        let ncall = self.st(S_CALLS).wrapping_add(1);
        self.set_st(S_CALLS, ncall);
        if self.sf(head, F_DIS) != 0 {
            return self.no_replay(Why::Disabled);
        }
        if let Err(w) = self.preconditions(cs) {
            return self.no_replay(w);
        }
        // The guard, for each recorded variant.
        let chain = self.chain(head);
        let mut miss = Why::NotRecorded;
        for &s in &chain {
            if self.sf(s, F_STATE) != ST_VALID || self.sf(s, F_SITE) != 0 {
                continue;
            }
            match self.guard(s) {
                Ok(()) => {
                    self.set_sf(s, F_LAST, ncall);
                    if verifying() {
                        crate::intrinsics_verify::begin(self, s, None);
                        return false;
                    }
                    self.replay(s);
                    return true;
                }
                Err(w) => {
                    if std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some() {
                        self.debug_mismatches(s, w);
                    }
                    miss = w;
                }
            }
        }
        self.no_replay(miss);
        // Record this context as a variant.
        let Some(target) = self.record_target(head, &chain, cs) else {
            return false;
        };
        let scanner = self.scanner_status;
        self.rec_start(target, false, cs, scanner);
        false
    }

    /// The preconditions of every replay (DESIGN.md §5.6), and the
    /// begin-document snapshot's arming control sequence, which a replay
    /// would not arm (MACRO-REPLAY.md §3.1).
    fn preconditions(&self, cs: i32) -> Result<(), Why> {
        if self.ckpt_arm_cs != 0 && cs == self.ckpt_arm_cs {
            return Err(Why::ArmCs);
        }
        if self.int_par(GLOBAL_DEFS_CODE) != 0 {
            return Err(Why::GlobalDefs);
        }
        if self.after_token != 0 {
            return Err(Why::AfterAssignment);
        }
        if TRACING_CODES.iter().any(|&c| self.int_par(c) > 0) {
            return Err(Why::Tracing);
        }
        Ok(())
    }

    /// The slot a new recording of the macro `cs` (chain `chain` from
    /// `head`) goes to, within the budgets; `None` if it is not recorded.
    fn record_target(&mut self, head: usize, chain: &[usize], cs: i32) -> Option<usize> {
        if self.sf(head, F_NOREC) != 0 || self.sf(head, F_RECORDS) >= RECORD_BUDGET {
            return None;
        }
        let target = if let Some(&s) = chain.iter().find(|&&s| self.sf(s, F_STATE) != ST_VALID) {
            s
        } else if chain.len() < MAX_VARIANTS {
            let s = self.alloc_slot()?;
            self.set_sf(s, F_CS, cs);
            self.set_sf(s, F_HEAD, head as i32 + 1);
            let last = *chain.last().unwrap();
            self.set_sf(last, F_NEXT, s as i32 + 1);
            s
        } else {
            // replace the variant least recently replayed
            let s = *chain.iter().min_by_key(|&&s| self.sf(s, F_LAST)).unwrap();
            self.slot_clear(s);
            self.set_sf(s, F_STATE, ST_FREE);
            s
        };
        let n = self.sf(head, F_RECORDS) + 1;
        self.set_sf(head, F_RECORDS, n);
        Some(target)
    }

    // -- the argument site (MACRO-REPLAY.md) ---------------------------------

    /// The tokens of `pstack[m]` (a list without a reference count).
    fn arg_tokens(&self, m: usize) -> impl Iterator<Item = i32> + '_ {
        let mut p = self.pstack[m];
        std::iter::from_fn(move || {
            (p != 0).then(|| {
                let t = self.info(p);
                p = self.link(p);
                t
            })
        })
    }

    /// Does `slot`'s key equal the `n` arguments in `pstack`? Token by
    /// token (MACRO-REPLAY.md §3.2): a control sequence by its location.
    fn args_equal(&self, slot: usize, n: usize) -> bool {
        let base = self.part(slot, P_ARGS);
        let len = self.sf(slot, F_NARGW) as usize;
        if len == 0 || self.intr_data[base] != n as i32 {
            return false;
        }
        let mut k = base + 1;
        let end = base + len;
        for m in 0..n {
            if k >= end {
                return false;
            }
            let want = self.intr_data[k] as usize;
            k += 1;
            let mut got = 0;
            for t in self.arg_tokens(m) {
                if got >= want || self.intr_data[k + got] != t {
                    return false;
                }
                got += 1;
            }
            if got != want {
                return false;
            }
            k += want;
        }
        k == end
    }

    /// Keep the `n` arguments in `pstack` as `slot`'s key; false if they do
    /// not fit.
    fn args_store(&mut self, slot: usize, n: usize) -> bool {
        let base = self.part(slot, P_ARGS);
        let mut v = vec![n as i32];
        for m in 0..n {
            let at = v.len();
            v.push(0);
            v.extend(self.arg_tokens(m));
            v[at] = (v.len() - at - 1) as i32;
            if v.len() > ARG_CAP {
                return false;
            }
        }
        self.intr_data[base..base + v.len()].copy_from_slice(&v);
        self.set_sf(slot, F_NARGW, v.len() as i32);
        true
    }

    /// `macro_call`, entered from `big_switch`'s `get_x_token`, has scanned
    /// the `n` arguments of the candidate `cs` (body `rc`) into `pstack`
    /// and popped the used-up input levels; `ss` and `sw` are the
    /// `scanner_status` and `warning_index` its `exit` restores. Replay the
    /// call (and return true: `macro_call` leaves through its `exit`), or
    /// let it expand, possibly recording it.
    pub fn flashtex_intr_call_args(&mut self, cs: i32, rc: i32, n: i32, ss: i32, sw: i32) -> bool {
        let _ = (rc, sw);
        if self.intr_rec_on || !(0..=9).contains(&n) {
            return false;
        }
        let n = n as usize;
        let slot = if self.intr_cand[cs as usize] != 0 {
            (self.intr_cand[cs as usize] - 1) as usize
        } else if self.intr_all_args {
            match self.make_candidate(cs) {
                Some(s) => s,
                None => return self.no_replay(Why::NoSlot),
            }
        } else {
            return false;
        };
        let head = slot;
        STATS.with(|s| {
            let mut s = s.borrow_mut();
            s.calls += 1;
            s.args_calls += 1;
        });
        let ncall = self.st(S_CALLS).wrapping_add(1);
        self.set_st(S_CALLS, ncall);
        if self.sf(head, F_DIS) != 0 {
            return self.no_replay(Why::Disabled);
        }
        if let Err(w) = self.preconditions(cs) {
            return self.no_replay(w);
        }
        // The recordings with these arguments (the index, then the key in
        // full), then the guard for each.
        let h = self.key_hash(cs, n);
        let mut valid = vec![];
        let mut dead = None;
        for s in self.hash_find(h) {
            if self.sf(s, F_CS) != cs || !self.args_equal(s, n) {
                continue;
            }
            match self.sf(s, F_STATE) {
                ST_VALID => valid.push(s),
                ST_DEAD => dead = Some(s),
                _ => {}
            }
        }
        let mut miss = if dead.is_some() {
            Why::Unrecordable
        } else {
            Why::NotRecorded
        };
        for &s in &valid {
            match self.guard(s) {
                Ok(()) => {
                    self.set_sf(s, F_LAST, ncall);
                    if verifying() {
                        let a = crate::intrinsics_verify::ArgCall { cs, n, ss, sw };
                        crate::intrinsics_verify::begin(self, s, Some(a));
                        return false;
                    }
                    self.replay_args(s, n);
                    return true;
                }
                Err(w) => {
                    if std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some() {
                        self.debug_mismatches(s, w);
                    }
                    miss = w;
                }
            }
        }
        self.no_replay(miss);
        // Record this call.
        if self.sf(head, F_NOREC) != 0 || self.sf(head, F_RECORDS) >= RECORD_BUDGET_ARGS {
            return false;
        }
        let target = if let Some(d) = dead {
            if self.sf(d, F_NOREC) != 0 {
                return false;
            }
            let k = self.sf(head, F_NDEAD) - 1;
            self.set_sf(head, F_NDEAD, k);
            d
        } else {
            if valid.len() >= KEY_VARIANTS {
                let lru = *valid.iter().min_by_key(|&&s| self.sf(s, F_LAST)).unwrap();
                self.free_arg_slot(lru);
            }
            if self.sf(head, F_NVAR) >= MAX_VARIANTS_ARGS {
                self.evict_lru(head, ST_VALID);
            }
            match self.alloc_arg_slot() {
                Some(s) => s,
                None => return false,
            }
        };
        self.slot_clear(target);
        self.set_sf(target, F_CS, cs);
        self.set_sf(target, F_HEAD, head as i32 + 1);
        self.set_sf(target, F_SITE, 1);
        self.set_sf(target, F_KEYHASH, h);
        self.set_sf(target, F_LAST, ncall);
        let r = self.sf(head, F_RECORDS) + 1;
        self.set_sf(head, F_RECORDS, r);
        self.rec_start(target, false, cs, ss);
        if !self.args_store(target, n) {
            self.rec_abort(Why::ArgsCapacity);
        }
        false
    }

    // -- the argument site's slots, index and heap (MACRO-REPLAY.md §3.3) -----

    /// FNV-1a over the macro, the argument count and the arguments' tokens
    /// (a separator before each): it only selects candidates, whose keys are
    /// then compared in full.
    fn key_hash(&self, cs: i32, n: usize) -> i32 {
        let mut h: u32 = 0x811c_9dc5;
        let mut mix = |v: i32| {
            for b in v.to_le_bytes() {
                h ^= b as u32;
                h = h.wrapping_mul(16_777_619);
            }
        };
        mix(cs);
        mix(n as i32);
        for m in 0..n {
            mix(-1);
            for t in self.arg_tokens(m) {
                mix(t);
            }
        }
        (h & 0x7fff_ffff) as i32
    }

    /// The slots the index names under hash `h`.
    fn hash_find(&self, h: i32) -> Vec<usize> {
        let mut out = vec![];
        let mut i = h as usize % HASH_SIZE;
        for _ in 0..HASH_SIZE {
            let e = self.intr_data[HASH0 + 2 * i];
            if e == 0 {
                break;
            }
            if e > 0 && self.intr_data[HASH0 + 2 * i + 1] == h {
                out.push((e - 1) as usize);
            }
            i = (i + 1) % HASH_SIZE;
        }
        out
    }

    fn hash_insert(&mut self, slot: usize) {
        if self.st(S_HUSED) as usize >= HASH_SIZE * 3 / 4 {
            self.hash_rebuild();
        }
        let h = self.sf(slot, F_KEYHASH);
        let mut i = h as usize % HASH_SIZE;
        while self.intr_data[HASH0 + 2 * i] > 0 {
            i = (i + 1) % HASH_SIZE;
        }
        if self.intr_data[HASH0 + 2 * i] == 0 {
            let u = self.st(S_HUSED) + 1;
            self.set_st(S_HUSED, u);
        }
        self.intr_data[HASH0 + 2 * i] = slot as i32 + 1;
        self.intr_data[HASH0 + 2 * i + 1] = h;
        self.set_sf(slot, F_INHASH, 1);
    }

    fn hash_remove(&mut self, slot: usize) {
        let mut i = self.sf(slot, F_KEYHASH) as usize % HASH_SIZE;
        for _ in 0..HASH_SIZE {
            let e = self.intr_data[HASH0 + 2 * i];
            if e == 0 {
                break;
            }
            if e == slot as i32 + 1 {
                self.intr_data[HASH0 + 2 * i] = -1;
                break;
            }
            i = (i + 1) % HASH_SIZE;
        }
        self.set_sf(slot, F_INHASH, 0);
    }

    /// The index afresh, without its deleted entries (in slot order).
    fn hash_rebuild(&mut self) {
        for i in 0..2 * HASH_SIZE {
            self.intr_data[HASH0 + i] = 0;
        }
        self.set_st(S_HUSED, 0);
        for s in MAX_SLOTS..MAX_SLOTS + self.st(S_ATOP) as usize {
            if self.sf(s, F_INHASH) != 0 {
                self.hash_insert(s);
            }
        }
    }

    fn alloc_arg_slot(&mut self) -> Option<usize> {
        let f = self.st(S_AFREE);
        let s = if f > 0 {
            let s = (f - 1) as usize;
            self.set_st(S_AFREE, self.sf(s, F_NEXT));
            s
        } else {
            let top = self.st(S_ATOP) as usize;
            if top >= ARG_SLOTS {
                return None;
            }
            self.set_st(S_ATOP, top as i32 + 1);
            MAX_SLOTS + top
        };
        for f in 0..SLOT_INTS {
            self.set_sf(s, f, 0);
        }
        self.set_sf(s, F_BASE, SCRATCH0 as i32);
        Some(s)
    }

    /// Drop an argument-site slot: its recording or key, its index entry.
    fn free_arg_slot(&mut self, s: usize) {
        let head = (self.sf(s, F_HEAD) - 1) as usize;
        let k = match self.sf(s, F_STATE) {
            ST_VALID => Some(F_NVAR),
            ST_DEAD => Some(F_NDEAD),
            _ => None,
        };
        if let Some(k) = k {
            let v = self.sf(head, k) - 1;
            self.set_sf(head, k, v);
        }
        self.slot_clear(s);
        if self.sf(s, F_INHASH) != 0 {
            self.hash_remove(s);
        }
        for f in 0..SLOT_INTS {
            self.set_sf(s, f, 0);
        }
        self.set_sf(s, F_NEXT, self.st(S_AFREE));
        self.set_st(S_AFREE, s as i32 + 1);
    }

    /// Free the least recently used slot of `head`'s macro in `state`.
    fn evict_lru(&mut self, head: usize, state: i32) {
        let lru = (MAX_SLOTS..MAX_SLOTS + self.st(S_ATOP) as usize)
            .filter(|&s| self.sf(s, F_HEAD) == head as i32 + 1 && self.sf(s, F_STATE) == state)
            .min_by_key(|&s| self.sf(s, F_LAST));
        if let Some(s) = lru {
            self.free_arg_slot(s);
        }
    }

    /// `len` words of the slot heap, compacting it if need be.
    fn heap_alloc(&mut self, len: usize) -> Option<usize> {
        let cap = HEAP_END - HEAP0;
        if self.st(S_HTOP) as usize + len > cap {
            if self.st(S_HLIVE) as usize + len > cap {
                return None;
            }
            self.heap_compact();
        }
        let b = HEAP0 + self.st(S_HTOP) as usize;
        self.set_st(S_HTOP, self.st(S_HTOP) + len as i32);
        self.set_st(S_HLIVE, self.st(S_HLIVE) + len as i32);
        Some(b)
    }

    /// Move the heap's blocks down over the holes, in address order.
    fn heap_compact(&mut self) {
        let mut blocks: Vec<(usize, usize)> = (MAX_SLOTS..MAX_SLOTS + self.st(S_ATOP) as usize)
            .filter(|&s| self.sf(s, F_HLEN) > 0)
            .map(|s| (self.sf(s, F_BASE) as usize, s))
            .collect();
        blocks.sort_unstable();
        let mut top = HEAP0;
        for (b, s) in blocks {
            let len = self.sf(s, F_HLEN) as usize;
            if b != top {
                self.intr_data[top..b + len].copy_within(b - top..b - top + len, 0);
                self.set_sf(s, F_BASE, top as i32);
            }
            top += len;
        }
        self.set_st(S_HTOP, (top - HEAP0) as i32);
    }

    /// A recording at the argument site commits: its parts move from the
    /// scratch region to a block of the heap of their exact size, and the
    /// index names it. False if the heap is full.
    fn arg_commit(&mut self, slot: usize) -> bool {
        let parts: Vec<(usize, usize)> = [
            (P_RW, 2 * self.sf(slot, F_NRW)),
            (P_RH, self.sf(slot, F_NRH)),
            (P_PIN, self.sf(slot, F_NPIN)),
            (P_OPS, 4 * self.sf(slot, F_NOPS)),
            (P_ARGS, self.sf(slot, F_NARGW)),
        ]
        .iter()
        .map(|&(p, n)| (self.part(slot, p), n as usize))
        .collect();
        let len: usize = parts.iter().map(|p| p.1).sum();
        let Some(b) = self.heap_alloc(len) else {
            return false;
        };
        let mut at = b;
        for (from, n) in parts {
            for i in 0..n {
                let v = self.intr_data[from + i];
                self.intr_data[at + i] = v;
            }
            at += n;
        }
        self.set_sf(slot, F_BASE, b as i32);
        self.set_sf(slot, F_HLEN, len as i32);
        if self.sf(slot, F_INHASH) == 0 {
            self.hash_insert(slot);
        }
        let head = (self.sf(slot, F_HEAD) - 1) as usize;
        let v = self.sf(head, F_NVAR) + 1;
        self.set_sf(head, F_NVAR, v);
        true
    }

    /// A recording at the argument site was abandoned: keep its key, so
    /// that the next call with it is not recorded again (unless `why`
    /// allows a retry and the budget is not used up).
    fn arg_dead(&mut self, slot: usize, why: Why) {
        let nargw = self.sf(slot, F_NARGW) as usize;
        let src = self.part(slot, P_ARGS);
        let key: Vec<i32> = (0..nargw).map(|i| self.intr_data[src + i]).collect();
        self.slot_clear(slot);
        self.set_sf(slot, F_STATE, ST_FREE);
        if nargw == 0 {
            return self.free_arg_slot(slot);
        }
        let Some(b) = self.heap_alloc(nargw) else {
            return self.free_arg_slot(slot);
        };
        for (i, &v) in key.iter().enumerate() {
            self.intr_data[b + i] = v;
        }
        self.set_sf(slot, F_BASE, b as i32);
        self.set_sf(slot, F_HLEN, nargw as i32);
        self.set_sf(slot, F_NARGW, nargw as i32);
        self.set_sf(slot, F_STATE, ST_DEAD);
        let a = self.sf(slot, F_ABORTS) + 1;
        self.set_sf(slot, F_ABORTS, a);
        let retry = matches!(why, Why::Checkpoint | Why::NewCs | Why::Arm);
        if !retry || a >= ABORT_BUDGET {
            self.set_sf(slot, F_NOREC, 1);
        }
        if self.sf(slot, F_INHASH) == 0 {
            self.hash_insert(slot);
        }
        let head = (self.sf(slot, F_HEAD) - 1) as usize;
        let d = self.sf(head, F_NDEAD) + 1;
        self.set_sf(head, F_NDEAD, d);
        if d > MAX_DEAD_ARGS {
            self.evict_lru(head, ST_DEAD);
        }
    }

    /// Replay `slot`, recorded at the argument site, for the `n` arguments
    /// in `pstack`, and free them, as the end of the body's level would
    /// (`end_token_list`).
    pub(crate) fn replay_args(&mut self, slot: usize, n: usize) {
        self.replay(slot);
        for m in 0..n {
            let p = self.pstack[m];
            self.flush_list(p);
        }
        STATS.with(|s| s.borrow_mut().args_replays += 1);
    }

    /// `big_switch` is about to take a checkpoint: a recording whose body is
    /// done is finished first (as the next statement would), and one still
    /// in progress is abandoned (MACRO-REPLAY.md §7.1): a checkpoint never
    /// holds one.
    pub(crate) fn intr_before_checkpoint(&mut self) {
        if self.intr_rec_on {
            self.flashtex_intr_switch();
        }
        if self.intr_rec_on {
            self.rec_abort(Why::Checkpoint);
        }
    }

    fn debug_mismatches(&self, slot: usize, w: Why) {
        let rh = self.part(slot, P_RH);
        let mut shown = 0;
        eprintln!(
            "intrinsics: \\{} not replayed: {w:?}",
            self.cs_name_string(self.sf(slot, F_CS))
        );
        for i in 0..self.sf(slot, F_NRH) as usize {
            let r = self.intr_data[rh + i] as usize * WATCH_INTS;
            if self.intr_data[r + 3] == 0 && shown < 12 {
                let p = self.intr_data[r + 5];
                eprintln!(
                    "  {} ({p}): want type {} equiv {}, have type {} equiv {}",
                    self.cs_name_string(p),
                    self.intr_data[r + 1],
                    self.intr_data[r + 2],
                    self.eq_type_of(p),
                    self.equiv_of(p)
                );
                shown += 1;
            }
        }
    }

    /// Make the recorded changes of `slot`, through TeX's own routines.
    pub(crate) fn replay(&mut self, slot: usize) {
        let t0 = std::time::Instant::now();
        if self.rs_on && fault() != Fault::NoReadset {
            self.intr_report_reads(slot);
        }
        let base = self.part(slot, P_OPS);
        let n = self.sf(slot, F_NOPS) as usize;
        let fault = fault();
        for i in 0..n {
            let o = base + 4 * i;
            let (k, a, b, c) = (
                self.intr_data[o],
                self.intr_data[o + 1],
                self.intr_data[o + 2],
                self.intr_data[o + 3],
            );
            let global = k & K_GLOBAL != 0;
            // Test only: break the replay on purpose, to show that the
            // verifier sees it (docs/evidence/l6-intrinsics-2026-09-29/).
            match fault {
                Fault::DropLast if i + 1 == n => continue,
                Fault::DropFirstLet if i == first_let(&self.intr_slot_ops(slot)) => continue,
                Fault::Local if global && k & 0xff == K_WORD => {
                    self.eq_word_define(a, b);
                    continue;
                }
                _ => {}
            }
            match k & 0xff {
                K_DEF | K_FRESH | K_LETCS => {
                    let (t, e) = match k & 0xff {
                        K_FRESH => (b, self.copy_token_list(c)),
                        K_LETCS => {
                            let (t, e) = (self.eq_type_of(c), self.equiv_of(c));
                            if (CALL..=LONG_OUTER_CALL).contains(&t)
                                && e != 0
                                && fault != Fault::NoRef
                            {
                                self.add_token_ref(e);
                            }
                            (t, e)
                        }
                        _ => (b, c),
                    };
                    if global {
                        self.geq_define(a, t, e)
                    } else {
                        self.eq_define(a, t, e)
                    }
                }
                K_WORD => {
                    if global {
                        self.geq_word_define(a, b)
                    } else {
                        self.eq_word_define(a, b)
                    }
                }
                K_BEGIN => self.new_save_level(a),
                K_END => self.unsave(),
                _ => {}
            }
        }
        let h = self.sf(slot, F_HITS) + 1;
        self.set_sf(slot, F_HITS, h);
        let cs = self.sf(slot, F_CS);
        STATS.with(|s| {
            let mut s = s.borrow_mut();
            s.replays += 1;
            s.replayed_ops += n as u64;
            s.replay_ns += t0.elapsed().as_nanos() as u64;
            if s.replays % 100 == 0 && std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some() {
                eprintln!("intrinsics: {} replays", s.replays);
            }
            *s.replays_by_cs.entry(cs).or_default() += 1;
        });
    }

    /// Tell the L5 read-set (`src/readset.rs`, DESIGN.md §5.5) about every
    /// control sequence the replayed run would have looked at, as its
    /// `get_next` and `id_lookup` would have: every entry the recording
    /// watches (what it read, including tokens it only scanned), every
    /// entry it assigned (a location first written is looked at by
    /// `get_r_token`) and every `\\let` source. All of them at once, at
    /// the start of the replay, which is no later than the normal path's
    /// first read of each (a restart point before the call covers them).
    /// Names looked up and not found, and names made, never occur: a
    /// recording with either is abandoned.
    fn intr_report_reads(&mut self, slot: usize) {
        // control sequences: below undefined_control_sequence, and above
        // eqtb_size (tex.ch's hash_extra, changes/web2c.ch)
        let (limit, size) = (self.st(L_UNDEFINED_CONTROL_SEQUENCE), self.st(L_EQTB_SIZE));
        let rh = self.part(slot, P_RH);
        let mut locs: Vec<i32> = Vec::new();
        for i in 0..self.sf(slot, F_NRH) as usize {
            let r = self.intr_data[rh + i] as usize * WATCH_INTS;
            locs.push(self.intr_data[r + 5]);
        }
        for o in self.intr_slot_ops(slot) {
            match o[0] & 0xff {
                K_DEF | K_FRESH | K_WORD => locs.push(o[1]),
                K_LETCS => {
                    locs.push(o[1]);
                    locs.push(o[3]);
                }
                _ => {}
            }
        }
        locs.push(self.sf(slot, F_CS));
        for p in locs {
            if p > 0 && (p < limit || p > size) && !self.rs_seen[p as usize] {
                self.flashtex_cs_read(p);
            }
        }
    }

    /// A new token list with the tokens of `l` (reference count null), as
    /// `scan_toks` would have made it.
    fn copy_token_list(&mut self, l: i32) -> i32 {
        let head = self.get_avail();
        self.mem[head as usize].set_hh_lh(0);
        let (mut q, mut r) = (head, self.link(l));
        while r != 0 {
            let n = self.get_avail();
            let t = self.info(r);
            self.mem[n as usize].set_hh_lh(t);
            self.mem[q as usize].set_hh_rh(n);
            q = n;
            r = self.link(r);
        }
        head
    }

    /// The recorded operations of `slot` (for the verifier).
    pub(crate) fn intr_slot_ops(&self, slot: usize) -> Vec<[i32; 4]> {
        let base = self.part(slot, P_OPS);
        (0..self.sf(slot, F_NOPS) as usize)
            .map(|i| {
                let o = base + 4 * i;
                [
                    self.intr_data[o],
                    self.intr_data[o + 1],
                    self.intr_data[o + 2],
                    self.intr_data[o + 3],
                ]
            })
            .collect()
    }

    pub(crate) fn intr_slot_cs(&self, slot: usize) -> i32 {
        self.sf(slot, F_CS)
    }

    /// Start the verification run of a valid `slot` (the normal path,
    /// observed): the macro `cs`, its body run with `scanner`.
    pub(crate) fn intr_rec_start_verify(&mut self, slot: usize, cs: i32, scanner: i32) {
        self.rec_start(slot, true, cs, scanner);
    }

    pub(crate) fn intr_disable(&mut self, slot: usize) {
        if slot >= MAX_SLOTS {
            return self.free_arg_slot(slot);
        }
        self.slot_clear(slot);
        self.set_sf(slot, F_STATE, ST_FREE);
        let head = (self.sf(slot, F_HEAD) - 1) as usize;
        self.set_sf(head, F_NOREC, 1);
    }

    /// End of the run: write the report (`FLASHTEX_INTRINSICS_STATS`).
    pub fn flashtex_intr_finish(&mut self) {
        let out = CONFIG.with(|c| c.borrow().as_ref().and_then(|c| c.stats_out.clone()));
        let Some(out) = out else { return };
        let by_cs: Vec<(i32, u64)> = STATS.with(|s| s.borrow_mut().replays_by_cs.drain().collect());
        for (cs, n) in by_cs {
            let name = self.cs_name_string(cs);
            STATS.with(|s| s.borrow_mut().per_cs.entry(name).or_default().0 += n);
        }
        let s = STATS.with(|s| format!("{:#?}\n", s.borrow()));
        let _ = std::fs::write(out, s);
    }
}

#[cfg(test)]
mod heap_tests {
    use super::*;

    /// The argument site's heap and index (MACRO-REPLAY.md §3.3): blocks
    /// keep their contents through a compaction, which closes the holes in
    /// address order, and the index finds exactly the slots it names, also
    /// after a rebuild.
    #[test]
    fn the_slot_heap_compacts_and_the_index_finds_its_slots() {
        let mut g = Globals::new();
        let mut ids = vec![];
        for k in 0..6usize {
            let s = g.alloc_arg_slot().unwrap();
            let len = 3 + k;
            let b = g.heap_alloc(len).unwrap();
            for i in 0..len {
                g.intr_data[b + i] = (100 * k + i) as i32;
            }
            g.set_sf(s, F_HEAD, 1);
            g.set_sf(s, F_BASE, b as i32);
            g.set_sf(s, F_HLEN, len as i32);
            g.set_sf(s, F_NARGW, len as i32);
            g.set_sf(s, F_KEYHASH, (k % 2) as i32);
            g.set_sf(s, F_STATE, ST_DEAD);
            g.hash_insert(s);
            ids.push(s);
        }
        g.free_arg_slot(ids[1]);
        g.free_arg_slot(ids[4]);
        let live = [0usize, 2, 3, 5];
        let words: usize = live.iter().map(|k| 3 + k).sum();
        assert_eq!(g.st(S_HLIVE) as usize, words);
        g.heap_compact();
        assert_eq!(g.st(S_HTOP) as usize, words);
        let mut at = HEAP0;
        for &k in &live {
            let s = ids[k];
            assert_eq!(g.sf(s, F_BASE) as usize, at, "slot {k}");
            let a = g.part(s, P_ARGS);
            for i in 0..3 + k {
                assert_eq!(
                    g.intr_data[a + i],
                    (100 * k + i) as i32,
                    "slot {k} word {i}"
                );
            }
            at += 3 + k;
        }
        for _ in 0..2 {
            let mut even = g.hash_find(0);
            even.sort_unstable();
            assert_eq!(even, vec![ids[0], ids[2]]);
            assert_eq!(g.hash_find(1), vec![ids[3], ids[5]]);
            g.hash_rebuild();
        }
        // a freed slot is used again, and a block after a compaction
        let s = g.alloc_arg_slot().unwrap();
        assert!(s == ids[1] || s == ids[4]);
        let b = g.heap_alloc(4).unwrap();
        assert_eq!(b, HEAP0 + words);
    }
}
