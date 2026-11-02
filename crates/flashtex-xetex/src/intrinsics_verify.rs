//! The semantic-state check of a guarded XeTeX call (`crate::intrinsics`).
//! The real path always runs and the run goes on from it; the replay runs
//! only in a state that is thrown away.
//!
//! At the call (`macro_call`, before the body is pushed): take checkpoint
//! C, replay the previous call's recording through TeX's own routines
//! (`eq_define`, `geq_define`, `eq_word_define`, `geq_word_define`,
//! `new_save_level`, `unsave`), capture that state I, restore C. When the
//! real path's body is used up (`big_switch`), its state N is compared with
//! I. **What is compared** (the pdfTeX verifier's rules,
//! `crates/flashtex-engine/src/intrinsics_verify.rs`):
//!
//! * every word of the word space either path wrote since C (the arena's
//!   open log), region by region, bit for bit, except the allocation and
//!   scratch regions `EXCLUDED_REGIONS`; `buffer` beyond `first`,
//!   `str_pool` beyond `pool_ptr`, `str_start` beyond `str_ptr` and
//!   `save_stack` beyond `save_ptr` are dead and skipped;
//! * every `eqtb` entry in those chunks, a macro or token-list value by its
//!   tokens (the two paths' lists are different nodes, or the same address
//!   holding other tokens), any other value bit for bit;
//! * every scalar global by name, except the scanner's result registers,
//!   allocation state, high-water marks, pseudo-printing scratch, the
//!   input-stack pointers (compared below) and the intrinsics' and
//!   checkpoints' own bookkeeping (`EXCLUDED_SCALARS`);
//! * the save stack above C's `save_ptr`, entry by entry, saved macro
//!   values by their tokens; the input levels still to be read (used-up
//!   token lists left out) and the macro parameters they use; the
//!   condition stack's entries; the log file's length.
//!
//! **Not compared:** `mem` itself beyond what reaches it from `eqtb`, the
//! save stack, the input and condition stacks: token-list reference counts
//! (a replayed `\let` shares its source's list, as `\let` does; a list not
//! from a control sequence is copied), glue specifications, boxes, marks,
//! nodes of the current list (only its pointers, which are scalars). State
//! outside the word space (`Host`: fonts, native handles, the display
//! list's source map, open files other than the log's length) is not
//! compared; the replay neither reads nor writes it. A page checkpoint
//! taken inside the body (a shipout there) opens a new log, so the real
//! path's writes before it are not among the chunks compared.

use crate::generated::consts::{
    eqtb_size, int_base, level_boundary, macro_, restore_old_value, token_list, v_template,
};
use crate::generated::types::memory_word;
use crate::intrinsics::{Op, Val, NULL};
use crate::Globals;
use flashtex_engine::arena::{CheckpointId, CHUNK_BYTES, CHUNK_WORDS};
use std::collections::{BTreeSet, HashMap};

/// Scratch and bookkeeping (the pdfTeX verifier's list). `warning_index`
/// is printed only while `scanner_status` is not normal, and whatever sets
/// such a status sets it first; `skip_line` is set before it is read. The
/// READ-TRACK watch (`dorm_*`, `pop_watch`) counts input pops, which the
/// replay does not make.
const EXCLUDED_SCALARS: &str = "cur_cmd cur_chr cur_cs cur_tok cur_val cur_val_level radix \
    cur_order def_ref long_state avail rover dyn_used var_used lo_mem_max hi_mem_min mem_end \
    max_in_stack max_param_stack max_buf_stack max_save_stack max_nest_stack tally first_count \
    trick_count base_ptr input_ptr param_ptr cur_input warning_index skip_line \
    dorm_fired dorm_level pop_watch";

const EXCLUDED_REGIONS: &str = "mem dig trick_buf pstack input_stack param_stack";

/// A check in progress: C and the replay's state I.
pub(crate) struct Pending {
    ck: CheckpointId,
    save_ptr0: i32,
    log0: u64,
    i: Snap,
}

impl Pending {
    pub(crate) fn ck(&self) -> CheckpointId {
        self.ck
    }
}

/// The classes of `Snap::stacks`.
const STACKS: [&str; 3] = ["save_stack", "input", "conditions"];

struct Snap {
    /// The contents of the chunks the path wrote (I only).
    chunks: HashMap<u32, Vec<u64>>,
    touched: Vec<u32>,
    scalars: Vec<u8>,
    stacks: [Vec<String>; 3],
    log_len: u64,
}

fn word(bytes: &[u8], k: usize) -> u64 {
    u64::from_le_bytes(bytes[8 * k..8 * k + 8].try_into().expect("8 bytes"))
}

impl Globals {
    /// Checkpoint, replay `ops`, capture, restore: the run goes on from C.
    pub(crate) fn intr_verify_begin(&mut self, ops: &[Op]) -> Pending {
        let save_ptr0 = self.save_ptr;
        let log0 = self.intr_log_len();
        let ck = self.checkpoint();
        self.host.intrinsics.replaying = true;
        self.intr_replay(ops);
        self.host.intrinsics.replaying = false;
        let i = self.intr_capture(save_ptr0, true);
        if let Err(e) = self.restore_in_call(ck) {
            // the replay's state cannot be the run's
            panic!("intrinsics verify: cannot restore the call's checkpoint: {e}");
        }
        Pending {
            ck,
            save_ptr0,
            log0,
            i,
        }
    }

    /// The real path is done: compare, count, drop C.
    pub(crate) fn intr_verify_end(&mut self, p: Pending) {
        let diffs = self.intr_compare(&p);
        self.drop_checkpoint(p.ck);
        let r = &mut self.host.intrinsics;
        r.checked += 1;
        if diffs.is_empty() {
            r.identical += 1;
            return;
        }
        r.differing += 1;
        let classes: BTreeSet<&'static str> = diffs.iter().map(|d| d.0).collect();
        for c in classes {
            *r.classes.entry(c).or_default() += 1;
        }
        for (_, d) in diffs.iter().take(3) {
            if r.details.len() < 8 {
                r.details.push(d.clone());
            }
        }
    }

    fn intr_replay(&mut self, ops: &[Op]) {
        for op in ops {
            match op {
                Op::Def(p, t, e, global) => {
                    let (t, e) = match e {
                        Val::Plain(v) => (*t, *v),
                        Val::Tokens(ts) => (*t, self.intr_new_list(ts)),
                        Val::Cs(c) => {
                            // `\let`: the source's value now, one more reference
                            let w = self.eqtb[(c - 1) as usize];
                            let (t, e) = (w.hh_b0(), w.hh_rh());
                            if self.intr_is_list(t, e) {
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

    /// A fresh token list (reference count `null`) holding `ts`.
    fn intr_new_list(&mut self, ts: &[i32]) -> i32 {
        let h = self.get_avail();
        self.mem[h as usize].set_hh_lh(NULL);
        let mut q = h;
        for &t in ts {
            let r = self.get_avail();
            self.mem[r as usize].set_hh_lh(t);
            self.mem[q as usize].set_hh_rh(r);
            q = r;
        }
        h
    }

    pub(crate) fn intr_log_len(&mut self) -> u64 {
        if !self.log_opened {
            return 0;
        }
        match self.log_file.snapshot(false) {
            Ok(s) => match s.stream {
                crate::system::Stream::Out { len, .. } => len,
                _ => 0,
            },
            Err(_) => u64::MAX,
        }
    }

    fn intr_capture(&mut self, save_ptr0: i32, keep: bool) -> Snap {
        self.spill_scalars();
        let touched = self.arena.open_log_chunks();
        let chunks = if keep {
            touched
                .iter()
                .map(|&c| (c, self.arena.chunk(c as usize).to_vec()))
                .collect()
        } else {
            HashMap::new()
        };
        Snap {
            chunks,
            touched,
            scalars: self
                .arena
                .read(0, crate::generated::globals::SCALAR_BYTES)
                .to_vec(),
            stacks: [
                self.intr_save_snap(save_ptr0),
                self.intr_input_snap(),
                self.intr_cond_snap(),
            ],
            log_len: self.intr_log_len(),
        }
    }

    /// The save stack above `save_ptr0`, top first.
    fn intr_save_snap(&self, save_ptr0: i32) -> Vec<String> {
        let mut out = vec![];
        let mut k = self.save_ptr - 1;
        while k >= save_ptr0 {
            let w = self.save_stack[k as usize].hh();
            let (ty, lv, ix) = (w.b0(), w.b1(), w.rh());
            if ty == restore_old_value && k > save_ptr0 {
                let v = self.save_stack[(k - 1) as usize].hh();
                let val = if self.intr_is_list(v.b0(), v.rh()) {
                    let toks = self.intr_tokens(self.mem[v.rh() as usize].hh().rh());
                    format!("type {} level {} tokens {toks:?}", v.b0(), v.b1())
                } else {
                    format!("word {:#x}", self.save_stack[(k - 1) as usize].to_bits())
                };
                out.push(format!("restore_old_value level {lv} p {ix}: {val}"));
                k -= 2;
            } else if ty == level_boundary && self.eTeX_mode == 1 && k > save_ptr0 {
                let line = self.save_stack[(k - 1) as usize].int();
                out.push(format!(
                    "level_boundary group {lv} boundary {ix} line {line}"
                ));
                k -= 2;
            } else {
                out.push(format!("type {ty} level {lv} index {ix}"));
                k -= 1;
            }
        }
        out
    }

    /// The input levels `get_next` would still read from, and the macro
    /// parameters they use (used-up token lists at the top left out, as
    /// the next `get_next` pops them).
    fn intr_input_snap(&self) -> Vec<String> {
        let mut levels: Vec<_> = (0..self.input_ptr)
            .map(|k| self.input_stack[k as usize])
            .collect();
        levels.push(self.cur_input);
        let mut param_ptr = self.param_ptr;
        while let Some(l) = levels.last() {
            if l.state_field != token_list || l.loc_field != NULL || l.index_field == v_template {
                break;
            }
            if l.index_field == macro_ {
                param_ptr = l.limit_field;
            }
            levels.pop();
        }
        let mut out: Vec<String> = levels
            .iter()
            .map(|l| {
                if l.state_field == token_list {
                    format!(
                        "list type {} name {} tokens {:?}",
                        l.index_field,
                        l.name_field,
                        self.intr_tokens(l.loc_field)
                    )
                } else {
                    let f = (l.state_field, l.index_field, l.start_field, l.loc_field);
                    format!("file {f:?} limit {} name {}", l.limit_field, l.name_field)
                }
            })
            .collect();
        for k in 0..param_ptr.max(0) {
            out.push(format!(
                "param {k}: {:?}",
                self.intr_tokens(self.param_stack[k as usize])
            ));
        }
        out
    }

    /// The condition stack's entries: type, subtype (`if_limit`), line.
    fn intr_cond_snap(&self) -> Vec<String> {
        let mut v = vec![];
        let mut p = self.cond_ptr;
        while p >= 0 && (p as usize) < self.mem.len() && v.len() < 100_000 {
            let w = self.mem[p as usize].hh();
            let line = self.mem[(p + 1) as usize].int();
            v.push(format!("{:?}", (w.b0(), w.b1(), line)));
            p = w.rh();
        }
        v
    }

    /// Tokens, roughly as TeX shows them, for the report.
    fn intr_show_tokens(&self, ts: &[i32]) -> String {
        let flag = crate::generated::consts::cs_token_flag;
        ts.iter()
            .take(24)
            .map(|&t| {
                if t >= flag {
                    format!("\\{} ", self.intr_cs_name(t - flag))
                } else {
                    char::from_u32((t & 0x1f_ffff) as u32).map_or("?".into(), |c| c.to_string())
                }
            })
            .collect()
    }

    /// Byte offset of an array global's element in the word space.
    fn intr_off<T>(&self, x: &T) -> usize {
        x as *const T as usize - self.arena.bytes().as_ptr() as usize
    }

    /// The replay's state I against the live state N; each difference with
    /// its class.
    fn intr_compare(&mut self, p: &Pending) -> Vec<(&'static str, String)> {
        let mut d: Vec<(&'static str, String)> = vec![];
        let n = self.intr_capture(p.save_ptr0, false);
        let i = &p.i;
        for sl in crate::statediff::scalar_layout(self) {
            if EXCLUDED_SCALARS.split_whitespace().any(|s| s == sl.name)
                || sl.name.starts_with("intr_")
                || sl.name.starts_with("ckpt_")
            {
                continue;
            }
            let r = sl.off..sl.off + sl.size;
            if i.scalars[r.clone()] != n.scalars[r] {
                d.push(("scalar", format!("scalar {}", sl.name)));
            }
        }
        let view = match self.arena.view_at(p.ck) {
            Ok(v) => v,
            Err(e) => return vec![("checkpoint", format!("cannot view C: {e}"))],
        };
        // I's word at byte offset `off`: the replay's chunk, else C's
        let iword = |off: usize| -> u64 {
            let (c, k) = (off / CHUNK_BYTES, (off % CHUNK_BYTES) / 8);
            match i.chunks.get(&(c as u32)) {
                Some(v) => v[k],
                None => word(view.chunk(c), k),
            }
        };
        let mem0 = self.intr_off(&self.mem[0]);
        let eqtb0 = self.intr_off(&self.eqtb[0]);
        let itokens = |e: i32| -> Vec<i32> {
            let mut v = vec![];
            let mut q = memory_word::from_bits(iword(mem0 + 8 * e as usize)).hh_rh();
            while q >= 0 && (q as usize) < self.mem.len() && v.len() < 1_000_000 {
                let w = memory_word::from_bits(iword(mem0 + 8 * q as usize));
                v.push(w.hh_lh());
                q = w.hh_rh();
            }
            v
        };
        let mut all: BTreeSet<u32> = n.touched.iter().copied().collect();
        all.extend(i.touched.iter().copied());
        let (first, pool_ptr, str_ptr, save_ptr) =
            (self.first, self.pool_ptr, self.str_ptr, self.save_ptr);
        // every `eqtb` entry in a written chunk: a list both paths allocated
        // may sit at the same address with other tokens
        let mut eqtb_locs = vec![];
        for c in all {
            let live = self.arena.chunk(c as usize);
            for (k, &nw) in live.iter().enumerate().take(CHUNK_WORDS) {
                let off0 = c as usize * CHUNK_BYTES + 8 * k;
                let (r0, rel0) = self.arena.region_at(off0);
                if r0.name == "eqtb" {
                    eqtb_locs.push((rel0 / 8) as i32 + 1);
                    continue;
                }
                let iw = iword(off0);
                if iw == nw {
                    continue;
                }
                // a word holds one element of 8 bytes or more, or two of 4
                let parts: &[(usize, u64)] = if r0.elem == 4 {
                    &[(0, 0xffff_ffff), (4, 0xffff_ffff_0000_0000)]
                } else {
                    &[(0, u64::MAX)]
                };
                for &(boff, mask) in parts {
                    if iw & mask == nw & mask {
                        continue;
                    }
                    let (r, rel) = self.arena.region_at(off0 + boff);
                    let idx = (rel / r.elem.max(1)) as i32;
                    let name = r.name;
                    let skip = name == "(scalars)"
                        || EXCLUDED_REGIONS.split(' ').any(|s| s == name)
                        || match name {
                            "buffer" => idx >= first,
                            "str_pool" => idx >= pool_ptr,
                            "str_start" => idx > str_ptr - 65536,
                            "save_stack" => idx >= p.save_ptr0 || idx >= save_ptr,
                            _ => false,
                        };
                    if skip {
                        continue;
                    }
                    d.push((
                        name,
                        format!(
                            "{name}[{idx}]: replay {:#x} real {:#x}",
                            iw & mask,
                            nw & mask
                        ),
                    ));
                }
            }
        }
        for q in eqtb_locs {
            let (iv, nv) = (
                iword(eqtb0 + 8 * (q - 1) as usize),
                self.eqtb[(q - 1) as usize].to_bits(),
            );
            let (a, b) = (memory_word::from_bits(iv), memory_word::from_bits(nv));
            // (the hash's extension, past `eqtb_size`, holds control sequences)
            if (q < int_base || q > eqtb_size)
                && a.hh_b0() == b.hh_b0()
                && a.hh_b1() == b.hh_b1()
                && self.intr_is_list(a.hh_b0(), a.hh_rh())
                && self.intr_is_list(b.hh_b0(), b.hh_rh())
            {
                let real = self.intr_tokens(self.mem[b.hh_rh() as usize].hh().rh());
                let replay = itokens(a.hh_rh());
                if replay == real {
                    continue;
                }
                let k = replay.iter().zip(&real).take_while(|(x, y)| x == y).count();
                let show = |v: &[i32]| self.intr_show_tokens(&v[k.min(v.len())..]);
                d.push((
                    "eqtb",
                    format!(
                        "\\{}: tokens from {k}: replay {:.80} real {:.80}",
                        self.intr_cs_name(q),
                        show(&replay),
                        show(&real)
                    ),
                ));
                continue;
            }
            if iv == nv {
                continue;
            }
            d.push((
                "eqtb",
                format!("\\{}: replay {iv:#x} real {nv:#x}", self.intr_cs_name(q)),
            ));
        }
        drop(view);
        for ((&class, a), b) in STACKS.iter().zip(&i.stacks).zip(&n.stacks) {
            if a != b {
                let (a, b) = (a.join(" / "), b.join(" / "));
                d.push((class, format!("{class}: replay {a:.160} real {b:.160}")));
            }
        }
        if i.log_len != n.log_len || n.log_len != p.log0 {
            let l = (p.log0, i.log_len, n.log_len);
            d.push(("log", format!("log length (C, replay, real): {l:?}")));
        }
        d
    }
}
