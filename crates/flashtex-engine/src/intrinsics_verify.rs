//! Both paths, diffed (DESIGN.md §5.6 item 4: "in CI both paths run and the
//! full state change is diffed").
//!
//! With `FLASHTEX_INTRINSICS=verify` (or `verify-all`), every call that the
//! guard would replay runs twice:
//!
//! 1. `begin` takes a checkpoint C of the whole engine (the word space,
//!    `checkpoint.rs`) and lets the macro expand normally, observed by a
//!    *verification recording* (the same purity checks as a recording; if
//!    the normal path turns out impure, the guard let through a call it
//!    should not have: a difference).
//! 2. When the body is used up (`big_switch`), `normal_path_done` captures
//!    the normal path's state N, restores C, replays the intrinsic, and
//!    compares the replay's state I with N. The run goes on from I.
//!
//! **What is compared** -- every word of the word space either path wrote
//! since C (the arena's open log), that is, every array global and every
//! scalar global, with these semantic rules:
//!
//! * `mem` is compared through what reaches it: every `eqtb` entry and
//!   save-stack entry holding a macro or token list is compared by its
//!   tokens *and reference count* (the two paths allocate different nodes;
//!   TeX never looks at an address), the input stack and parameter stack
//!   by the tokens they have left to read, the condition stack by its
//!   entries.
//! * The save stack above C's `save_ptr` is compared entry by entry
//!   (type, level, location, saved value as above); below it, word by word.
//! * Scratch that no later computation reads before writing is excluded,
//!   by name: `buffer` beyond `first`, `str_pool` beyond `pool_ptr`,
//!   `str_start` beyond `str_ptr`, `save_stack` beyond `save_ptr`, `dig`,
//!   `trick_buf`, `pstack`; the scanner's result registers (`cur_cmd`,
//!   `cur_chr`, `cur_cs`, `cur_tok`, `cur_val`, `cur_val_level`, `radix`,
//!   `cur_order`, `def_ref`, `long_state`); memory-allocation state and
//!   statistics (`avail`, `rover`, `dyn_used`, `var_used`, `lo_mem_max`,
//!   `hi_mem_min`, `mem_end`, the `max_*` high-water marks), pseudo-printing
//!   scratch (`tally`, `first_count`, `trick_count`, `base_ptr`) and the
//!   intrinsics' and checkpoints' own bookkeeping (`intr_*`, `ckpt_*`,
//!   `macro_prof_on`). `EXCLUDED_SCALARS` is the list.
//! * Output: the log file's length (no path writes any).
//!
//! Every difference is counted and described in the statistics
//! (`FLASHTEX_INTRINSICS_STATS`); `FLASHTEX_INTRINSICS_VERIFY_FAIL=1` makes
//! the run exit with status 3 when there was any.

use crate::arena::{CheckpointId, CHUNK_BYTES, CHUNK_WORDS};
use crate::generated::Globals;
use crate::intrinsics::{Why, STATS};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};

const CALL: i32 = 114;
const LONG_OUTER_CALL: i32 = 117;
const V_TEMPLATE: i32 = 2;
const MACRO: i32 = 5;
const RESTORE_OLD_VALUE: i32 = 0;

/// Scalar globals that are scratch, statistics or bookkeeping (see the
/// module documentation).
const EXCLUDED_SCALARS: &[&str] = &[
    "cur_cmd",
    "cur_chr",
    "cur_cs",
    "cur_tok",
    "cur_val",
    "cur_val_level",
    "radix",
    "cur_order",
    "def_ref",
    "long_state",
    "avail",
    "rover",
    "dyn_used",
    "var_used",
    "lo_mem_max",
    "hi_mem_min",
    "mem_end",
    "max_in_stack",
    "max_param_stack",
    "max_buf_stack",
    "max_save_stack",
    "max_nest_stack",
    "tally",
    "first_count",
    "trick_count",
    "base_ptr",
    "input_ptr",
    "param_ptr",
    "cur_input",
    "macro_prof_on",
    // printed only while `scanner_status` is not `normal`, and every
    // routine that sets such a status sets `warning_index` first
    "warning_index",
    // the string `tokens_to_string` made; every caller flushes it at once
    "last_tokens_string",
    // where `pass_text` began skipping: it sets it first, and it is read
    // only while skipping ("Incomplete \if...; all text was ignored after
    // line ...")
    "skip_line",
];

const EXCLUDED_REGIONS: &[&str] = &["mem", "dig", "trick_buf", "pstack", "input_stack", "param_stack"];

struct Pending {
    slot: usize,
    ck: CheckpointId,
    save_ptr0: i32,
    log0: u64,
    normal_ops: usize,
}

thread_local! {
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
}

/// A token list, as a path left it: its reference count and tokens.
type ListSnap = (i32, Vec<i32>);

#[derive(Default)]
struct Snap {
    /// Contents of the chunks the normal path wrote (N only).
    chunks: HashMap<u32, Vec<u64>>,
    touched: Vec<u32>,
    scalars: Vec<u8>,
    /// Macro and token-list values of `eqtb` entries, by location.
    lists: HashMap<i32, ListSnap>,
    /// Save-stack entries above C's `save_ptr`, top first.
    save: Vec<String>,
    input: Vec<String>,
    cond: Vec<(i32, i32, i32)>,
    log_len: u64,
}

fn is_list(t: i32, e: i32) -> bool {
    (CALL..=LONG_OUTER_CALL).contains(&t) && e != 0
}

impl Globals {
    fn tokens_from(&self, mut p: i32) -> Vec<i32> {
        let mut v = vec![];
        let mut n = 0;
        while p != 0 && n < 1_000_000 {
            v.push(self.mem[p as usize].hh().lh());
            p = self.mem[p as usize].hh().rh();
            n += 1;
        }
        v
    }

    fn list_snap(&self, e: i32) -> ListSnap {
        (self.mem[e as usize].hh().lh(), self.tokens_from(self.mem[e as usize].hh().rh()))
    }

    fn describe_word(&self, w: u64) -> String {
        let (t, l, e) = (((w >> 32) & 0xffff) as i32, (w >> 48) as i32, (w & 0xffff_ffff) as i32);
        if is_list(t, e) {
            let (rc, toks) = self.list_snap(e);
            format!("type {t} level {l} list(ref {rc}, {} tokens)", toks.len())
        } else {
            format!("type {t} level {l} equiv {e}")
        }
    }

    /// The input levels `get_next` would still read from, and the macro
    /// parameters they use: levels used up at the top are left out, as the
    /// next `get_next` (or `macro_call`) pops them.
    fn input_snap(&self) -> Vec<String> {
        let mut levels: Vec<crate::generated::types::in_state_record> =
            (0..self.input_ptr).map(|k| self.input_stack[k as usize]).collect();
        levels.push(self.cur_input);
        let mut param_ptr = self.param_ptr;
        while let Some(l) = levels.last() {
            if l.state_field == 0 && l.loc_field == 0 && l.index_field != V_TEMPLATE {
                if l.index_field == MACRO {
                    param_ptr = l.limit_field;
                }
                levels.pop();
            } else {
                break;
            }
        }
        let mut out: Vec<String> = levels
            .iter()
            .map(|l| {
                if l.state_field == 0 {
                    format!("list type {} name {} tokens {:?}", l.index_field, l.name_field, self.tokens_from(l.loc_field))
                } else {
                    format!(
                        "file state {} index {} start {} loc {} limit {} name {}",
                        l.state_field, l.index_field, l.start_field, l.loc_field, l.limit_field, l.name_field
                    )
                }
            })
            .collect();
        for k in 0..param_ptr.max(0) {
            out.push(format!("param {k}: {:?}", self.tokens_from(self.param_stack[k as usize])));
        }
        out
    }

    fn save_snap(&self, save_ptr0: i32) -> Vec<String> {
        let mut out = vec![];
        let mut k = self.save_ptr - 1;
        while k >= save_ptr0 {
            let w = self.save_stack[k as usize].hh();
            let (ty, lv, ix) = (w.b0(), w.b1(), w.rh());
            if ty == RESTORE_OLD_VALUE && k - 1 >= save_ptr0 {
                let v = self.save_stack[(k - 1) as usize].0;
                let (t, e) = (((v >> 32) & 0xffff) as i32, (v & 0xffff_ffff) as i32);
                let val = if is_list(t, e) {
                    format!("type {t} level {} list {:?}", v >> 48, self.list_snap(e))
                } else {
                    format!("word {v:#x}")
                };
                out.push(format!("restore_old_value level {lv} p {ix}: {val}"));
                k -= 2;
            } else {
                out.push(format!("type {ty} level {lv} index {ix}"));
                k -= 1;
            }
        }
        out
    }

    fn cond_snap(&self) -> Vec<(i32, i32, i32)> {
        let mut v = vec![];
        let mut p = self.cond_ptr;
        while p != 0 && v.len() < 100_000 {
            let w = self.mem[p as usize].hh();
            v.push((w.b0(), w.b1(), self.mem[(p + 1) as usize].int()));
            p = w.rh();
        }
        v
    }

    fn eqtb_range_of_chunk(&self, c: u32) -> Option<(i32, i32)> {
        let off = c as usize * CHUNK_BYTES;
        let (r, rel) = self.arena.region_at(off);
        if r.name != "eqtb" {
            // a chunk may start in the region before eqtb and reach into it
            let (r2, rel2) = self.arena.region_at(off + CHUNK_BYTES - 1);
            if r2.name != "eqtb" {
                return None;
            }
            let last = (rel2 / 8) as i32 + 1;
            return Some((1, last));
        }
        let first = (rel / 8) as i32 + 1;
        Some((first, first + CHUNK_WORDS as i32 - 1))
    }

    fn capture(&mut self, p: &Pending, keep_chunks: bool, extra: &BTreeSet<i32>) -> Snap {
        self.spill_scalars();
        let touched = self.arena.open_log_chunks();
        let mut s = Snap {
            touched: touched.clone(),
            scalars: self.arena.read(0, crate::generated::globals::SCALAR_BYTES).to_vec(),
            ..Default::default()
        };
        let size = self.eqtb.len() as i32;
        let mut locs: BTreeSet<i32> = extra.clone();
        for &c in &touched {
            if keep_chunks {
                s.chunks.insert(c, self.arena.chunk(c as usize).to_vec());
            }
            if let Some((a, b)) = self.eqtb_range_of_chunk(c) {
                for q in a.max(1)..=b.min(size) {
                    locs.insert(q);
                }
            }
        }
        for q in locs {
            if q < 1 || q > size {
                continue;
            }
            let w = self.eqtb[(q - 1) as usize].hh();
            if is_list(w.b0(), w.rh()) {
                s.lists.insert(q, self.list_snap(w.rh()));
            }
        }
        s.save = self.save_snap(p.save_ptr0);
        s.input = self.input_snap();
        s.cond = self.cond_snap();
        s.log_len = self.verify_log_len();
        s
    }

    fn verify_log_len(&mut self) -> u64 {
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

    /// Compare the normal path's state `n` with the live (replayed) state.
    fn compare(&mut self, p: &Pending, n: &Snap) -> Vec<String> {
        let mut d = vec![];
        let i = self.capture(p, false, &BTreeSet::new());
        // scalars, by name
        let layout = crate::statediff::scalar_layout(self);
        for sl in &layout {
            if EXCLUDED_SCALARS.contains(&sl.name) || sl.name.starts_with("intr_") || sl.name.starts_with("ckpt_") {
                continue;
            }
            let (a, b) = (&n.scalars[sl.off..sl.off + sl.size], &i.scalars[sl.off..sl.off + sl.size]);
            if a != b {
                d.push(format!("scalar {}: {:?} -> {:?}", sl.name, a, b));
            }
        }
        // every word either path wrote
        let view = match self.arena.view_at(p.ck) {
            Ok(v) => v,
            Err(e) => return vec![format!("cannot view the checkpoint: {e}")],
        };
        let mut all: BTreeSet<u32> = n.touched.iter().copied().collect();
        all.extend(i.touched.iter().copied());
        if std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some() {
            eprintln!("verify: chunks written: normal {} replay {} union {}", n.touched.len(), i.touched.len(), all.len());
        }
        let (first, pool_ptr, str_ptr, save_ptr) = (self.first, self.pool_ptr, self.str_ptr, self.save_ptr);
        let mut eqtb_diffs: Vec<i32> = vec![];
        for c in all {
            let nw: Vec<u64> = match n.chunks.get(&c) {
                Some(v) => v.clone(),
                None => {
                    let b = view.chunk(c as usize);
                    (0..CHUNK_WORDS).map(|k| u64::from_le_bytes(b[8 * k..8 * k + 8].try_into().unwrap())).collect()
                }
            };
            let iw = self.arena.chunk(c as usize);
            for k in 0..CHUNK_WORDS {
                if nw[k] == iw[k] {
                    continue;
                }
                // an 8-byte word holds one element of 8 bytes or more, or
                // two of 4 bytes: compare each element on its own
                let (r0, _) = self.arena.region_at(c as usize * CHUNK_BYTES + 8 * k);
                let parts: &[(usize, u64)] = if r0.elem == 4 { &[(0, 0xffff_ffff), (4, 0xffff_ffff_0000_0000)] } else { &[(0, u64::MAX)] };
                for &(boff, mask) in parts {
                if nw[k] & mask == iw[k] & mask {
                    continue;
                }
                let off = c as usize * CHUNK_BYTES + 8 * k + boff;
                let (r, rel) = self.arena.region_at(off);
                let idx = rel / r.elem.max(1);
                let name = r.name;
                if name == "(scalars)" || EXCLUDED_REGIONS.contains(&name) || name.starts_with("intr_") {
                    continue;
                }
                let skip = match name {
                    "buffer" => idx as i32 >= first,
                    "str_pool" => idx as i32 >= pool_ptr,
                    "str_start" => idx as i32 > str_ptr,
                    "save_stack" => idx as i32 >= p.save_ptr0 || idx as i32 >= save_ptr,
                    _ => false,
                };
                if skip {
                    continue;
                }
                if name == "eqtb" {
                    eqtb_diffs.push(idx as i32 + 1);
                    continue;
                }
                d.push(format!("{name}[{idx}]: {:#x} -> {:#x}", nw[k] & mask, iw[k] & mask));
                }
            }
        }
        drop(view);
        if std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some() {
            eprintln!("verify: eqtb words differing {:?}; eqtb[329] now {:#x}", &eqtb_diffs[..eqtb_diffs.len().min(10)], self.eqtb[328].0);
        }
        for q in eqtb_diffs {
            let (nv, iv) = (self.eqtb_word_n(n, q, p), self.eqtb[(q - 1) as usize].0);
            let (nt, ne, nl) = (((nv >> 32) & 0xffff) as i32, (nv & 0xffff_ffff) as i32, nv >> 48);
            let (it, ie, il) = (((iv >> 32) & 0xffff) as i32, (iv & 0xffff_ffff) as i32, iv >> 48);
            if nt == it && nl == il && is_list(nt, ne) && is_list(it, ie) {
                if let (Some(a), Some(b)) = (n.lists.get(&q), Some(self.list_snap(ie))) {
                    if *a == b {
                        continue;
                    }
                    d.push(format!(
                        "eqtb[{q}] \\{}: list ref {} {} tokens -> ref {} {} tokens",
                        self.cs_name_string(q),
                        a.0,
                        a.1.len(),
                        b.0,
                        b.1.len()
                    ));
                    continue;
                }
            }
            d.push(format!(
                "eqtb[{q}] \\{}: {} -> {}",
                self.cs_name_string(q),
                self.describe_n(nv),
                self.describe_word(iv)
            ));
        }
        // lists shared by both paths: the same node must have the same count
        let dbg = std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some();
        let mut shown = 0;
        for (q, a) in &n.lists {
            if dbg && shown < 5 {
                let w = self.eqtb[(*q - 1) as usize].hh();
                if is_list(w.b0(), w.rh()) {
                    let b = self.list_snap(w.rh());
                    eprintln!("verify: list {q} \\{}: normal ref {} ({} tokens) replay ref {} ({} tokens) raw equal {}", self.cs_name_string(*q), a.0, a.1.len(), b.0, b.1.len(), self.eqtb_word_n(n, *q, p) == self.eqtb[(*q - 1) as usize].0);
                    shown += 1;
                }
            }
            let w = self.eqtb[(*q - 1) as usize].hh();
            if is_list(w.b0(), w.rh()) {
                let b = self.list_snap(w.rh());
                if a.1 == b.1 && a.0 != b.0 && self.eqtb_word_n(n, *q, p) == self.eqtb[(*q - 1) as usize].0 {
                    d.push(format!("eqtb[{q}] \\{}: shared list, reference count {} -> {}", self.cs_name_string(*q), a.0, b.0));
                }
            }
        }
        if n.save != i.save {
            d.push(format!("save stack above {}: {:?} -> {:?}", p.save_ptr0, n.save, i.save));
        }
        if n.input != i.input {
            d.push(format!("input: {:?} -> {:?}", n.input, i.input));
        }
        if n.cond != i.cond {
            d.push(format!("conditions: {:?} -> {:?}", n.cond, i.cond));
        }
        if n.log_len != i.log_len || n.log_len != p.log0 {
            d.push(format!("log length: before {} normal {} replayed {}", p.log0, n.log_len, i.log_len));
        }
        d
    }

    /// The normal path's word for `eqtb[q]`.
    fn eqtb_word_n(&self, n: &Snap, q: i32, p: &Pending) -> u64 {
        let off = self.eqtb_offset(q);
        let c = (off / CHUNK_BYTES) as u32;
        let k = (off % CHUNK_BYTES) / 8;
        if let Some(v) = n.chunks.get(&c) {
            return v[k];
        }
        match self.arena.view_at(p.ck) {
            Ok(v) => {
                let b = v.chunk(c as usize);
                u64::from_le_bytes(b[8 * k..8 * k + 8].try_into().unwrap())
            }
            Err(_) => 0,
        }
    }

    fn eqtb_offset(&self, q: i32) -> usize {
        let base = &self.eqtb[0] as *const _ as usize;
        let space = self.arena.bytes().as_ptr() as usize;
        base - space + 8 * (q - 1) as usize
    }

    fn describe_n(&self, w: u64) -> String {
        let (t, l, e) = (((w >> 32) & 0xffff) as i32, (w >> 48) as i32, (w & 0xffff_ffff) as i32);
        format!("type {t} level {l} equiv {e}")
    }
}

/// Leave the used-up token lists at the top of the input stack, as the
/// next `get_next` would (§357: `end_token_list; goto restart`), so that
/// the references they hold are released in both paths before comparing.
fn pop_used_up(g: &mut Globals) {
    while g.cur_input.state_field == 0 && g.cur_input.loc_field == 0 && g.cur_input.index_field != V_TEMPLATE && g.input_ptr > 0 {
        g.end_token_list();
    }
}

/// The guard passed in a verifying run: checkpoint, then let the macro expand.
pub(crate) fn begin(g: &mut Globals, slot: usize) {
    let ck = match g.checkpoint() {
        Ok(id) => id,
        Err(e) => {
            STATS.with(|s| {
                let mut s = s.borrow_mut();
                s.verify_differences += 1;
                s.verify_details.push(format!("cannot take a checkpoint: {e}"));
            });
            return;
        }
    };
    let log0 = g.verify_log_len();
    PENDING.with(|p| {
        *p.borrow_mut() = Some(Pending {
            slot,
            ck,
            save_ptr0: g.save_ptr,
            log0,
            normal_ops: 0,
        })
    });
    g.intr_rec_start_verify(slot);
}

/// An operation of the normal path (counted, and compared by count).
pub(crate) fn note_op(_k: i32, _a: i32, _b: i32, _c: i32) {
    PENDING.with(|p| {
        if let Some(p) = p.borrow_mut().as_mut() {
            p.normal_ops += 1;
        }
    });
}

/// The normal path is done: capture it, restore, replay, compare.
pub(crate) fn normal_path_done(g: &mut Globals, slot: usize) {
    let Some(p) = PENDING.with(|p| p.borrow_mut().take()) else { return };
    debug_assert_eq!(p.slot, slot);
    let targets: BTreeSet<i32> = g
        .intr_slot_ops(slot)
        .iter()
        .filter(|o| matches!(o[0] & 0xff, 1 | 5 | 6))
        .map(|o| o[1])
        .collect();
    pop_used_up(g);
    let n = g.capture(&p, true, &targets);
    let mut diffs = vec![];
    if let Err(e) = g.restore_discard(p.ck) {
        diffs.push(format!("cannot restore the checkpoint: {e}"));
    } else {
        g.replay(slot);
        pop_used_up(g);
        diffs = g.compare(&p, &n);
    }
    if std::env::var_os("FLASHTEX_INTRINSICS_DEBUG").is_some() {
        eprintln!("verify: {} differences {:?}; lists captured {}", diffs.len(), diffs.first(), n.lists.len());
        for o in g.intr_slot_ops(slot).iter().filter(|o| o[0] & 0xff == 6).take(3) {
            let q = o[1];
            let w = g.eqtb[(q - 1) as usize].hh();
            eprintln!("verify: letcs target {q} \\{} from \\{}: normal {:?} replay type {} level {} ref {}", g.cs_name_string(q), g.cs_name_string(o[3]), n.lists.get(&q).map(|a| (a.0, a.1.len())), w.b0(), w.b1(), g.mem[w.rh() as usize].hh().lh());
        }
    }
    let recorded_ops = g.intr_slot_ops(slot).len();
    if p.normal_ops != recorded_ops {
        diffs.push(format!("operations: normal path {} replay {}", p.normal_ops, recorded_ops));
    }
    let ck = p.ck;
    g.retain_checkpoints(&|id| id != ck);
    let name = g.cs_name_string(g.intr_slot_cs(slot));
    if !diffs.is_empty() {
        eprintln!("intrinsics verify: \\{name}: {} differences: {:?}", diffs.len(), &diffs[..diffs.len().min(3)]);
    }
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        s.verified += 1;
        s.per_cs.entry(name.clone()).or_default().1 += 1;
        if !diffs.is_empty() {
            s.verify_differences += 1;
            if s.verify_details.len() < 20 {
                s.verify_details.push(format!("\\{name}: {} differences: {:?}", diffs.len(), &diffs[..diffs.len().min(12)]));
            }
        }
    });
}

/// The normal path of a call the guard let through was not pure.
pub(crate) fn verify_aborted(g: &mut Globals, slot: usize, why: Why) {
    let Some(p) = PENDING.with(|p| p.borrow_mut().take()) else { return };
    let ck = p.ck;
    g.retain_checkpoints(&|id| id != ck);
    let name = g.cs_name_string(g.intr_slot_cs(slot));
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        s.verified += 1;
        s.verify_differences += 1;
        s.verify_details.push(format!("\\{name}: the guard passed but the normal path was not pure: {why:?}"));
    });
    g.intr_disable(slot);
}

/// Did any verification find a difference?
pub fn differences() -> u64 {
    STATS.with(|s| s.borrow().verify_differences)
}
