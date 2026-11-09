//! XeTeX guarded intrinsics, verification only: no call is ever replayed
//! into the run, and every output comes from the real path.
//!
//! `eligible` means the registered parameterless macro reached `big_switch`
//! with the entry guards clear and its direct dependency tokens unchanged.
//! It is not a purity verdict: the dependency set is the body's direct
//! control sequences only.
//!
//! In verify mode each guarded call's real path is *recorded*: the
//! definitions (`eq_define`, `eq_word_define`, their global forms) and group
//! boundaries (`new_save_level`, `unsave`) it makes, in order, macro values
//! by their tokens (`changes/intrinsics.ch`). The recording ends at
//! `big_switch` once the body's input levels are used up. A recording is
//! kept for replay only when the call left the group level, conditionals,
//! alignment state, strings, errors, mode, list tail, scanner and output as
//! it found them, and made no glue, shape, box or sparse-register
//! definition, no write to a `\tracing...` parameter, no `\read`,
//! `\openin`, `\closein` or `\XeTeXinputencoding`, and no `\aftergroup` at its own level
//! (`not_replayable.*` otherwise).
//!
//! An eligible call with a kept recording from the previous guarded call is
//! *checked* (`crate::intrinsics_verify`): checkpoint C, replay the previous
//! recording, capture that state, restore C, then let the real path run;
//! when it is done, compare the two states.

use crate::generated::consts::{
    box_ref, cs_token_flag, end_match_token, global_defs_code, glue_ref, hash_base, int_base,
    level_one, null_cs, protected_token, shape_ref, single_base, token_list, tracing_assigns_code,
    tracing_char_sub_def_code, tracing_commands_code, tracing_groups_code, tracing_ifs_code,
    tracing_lost_chars_code, tracing_macros_code, tracing_nesting_code, tracing_online_code,
    tracing_output_code, tracing_pages_code, tracing_paragraphs_code, tracing_restores_code,
    tracing_scan_tokens_code, tracing_stack_levels_code, tracing_stats_code, v_template,
};
use crate::Globals;
use std::collections::BTreeMap;

const NAME: &[u8] = b"pdfstringdefPreHook";

/// Whether `eqtb` location `p` is a `\tracing...` integer parameter.
fn is_tracing(p: i32) -> bool {
    [
        tracing_online_code,
        tracing_macros_code,
        tracing_stats_code,
        tracing_paragraphs_code,
        tracing_pages_code,
        tracing_output_code,
        tracing_lost_chars_code,
        tracing_commands_code,
        tracing_restores_code,
        tracing_char_sub_def_code,
        tracing_stack_levels_code,
        tracing_assigns_code,
        tracing_groups_code,
        tracing_ifs_code,
        tracing_scan_tokens_code,
        tracing_nesting_code,
    ]
    .iter()
    .any(|&c| p == int_base + c)
}
const MAX_BODY_TOKENS: usize = 4096;
const MAX_OPS: usize = 10_000;
// xetex.web's `null` is negative, unlike a C null pointer.
pub(crate) const NULL: i32 = -0x0fff_ffff;

/// One definition or group boundary of a recorded real path: `Def(p, t,
/// e, global)`, `Word(p, w, global)`, `new_save_level(c)`, `unsave`.
#[derive(Clone)]
pub(crate) enum Op {
    Def(i32, i32, Val, bool),
    Word(i32, i32, bool),
    Begin(i32),
    End,
}

/// A defined value: a macro or token list by its tokens, or (`\let`, which
/// shares the list) the control sequence whose value it was; else as is.
#[derive(Clone)]
pub(crate) enum Val {
    Plain(i32),
    Tokens(Vec<i32>),
    Cs(i32),
}

/// The real path of a guarded call, being recorded.
struct Rec {
    /// The input level the body is pushed at.
    base: i32,
    /// `cur_level` and `cur_group` at the call: a replay of its group
    /// boundaries is valid only where they are the same.
    at: (i32, i32),
    ops: Vec<Op>,
    bad: Option<&'static str>,
    /// What a replayable call leaves as it found it (`intr_marks`).
    marks: Vec<(&'static str, i64)>,
    verify: Option<crate::intrinsics_verify::Pending>,
}

#[derive(Default)]
pub struct Report {
    /// The run's mode, kept outside the word space: a restore puts it back
    /// into `intr_verify_on` (`intr_after_restore`).
    pub verify: bool,
    calls: u64,
    eligible: u64,
    rejections: BTreeMap<&'static str, u64>,
    previous: Option<Vec<(i32, u64)>>,
    /// Direct control-sequence tokens seen in candidate bodies. This is a
    /// diagnostic read set, not the complete replay dependency set.
    dependency_tokens: u64,
    unicode_dependency_tokens: u64,
    siunitx_context_calls: u64,
    bookmark_copy_calls: u64,
    rec: Option<Rec>,
    /// The latest replayable recording, for the next guarded call.
    /// (`Some`: the group it must replay in.)
    last: Option<(Option<(i32, i32)>, Vec<Op>)>,
    pub(crate) replaying: bool,
    /// Replayable recordings, and their operations.
    kept: u64,
    kept_ops: u64,
    pub(crate) checked: u64,
    pub(crate) identical: u64,
    pub(crate) differing: u64,
    pub(crate) classes: BTreeMap<&'static str, u64>,
    pub(crate) unverified: BTreeMap<&'static str, u64>,
    not_replayable: BTreeMap<&'static str, u64>,
    pub(crate) details: Vec<String>,
}

/// `on` is deliberately inert until the promotion gate has been met.
pub fn enabled() -> bool {
    matches!(
        std::env::var("FLASHTEX_INTRINSICS").as_deref(),
        Ok("verify" | "report")
    )
}

impl Globals {
    /// The run's mode: a fresh report, and the flag the WEB hooks test.
    pub fn set_intrinsics_verify(&mut self, on: bool) {
        self.intr_abandon("unfinished");
        self.host.intrinsics = Report {
            verify: on,
            ..Report::default()
        };
        self.intr_verify_on = on;
        self.intr_at_switch = false;
    }

    /// After a restore (`Globals::restore`): the restored scalars hold the
    /// mode of the run that took the checkpoint (S₀ may come from an earlier
    /// run or a file), so the mode is put back, and a recording in progress
    /// belongs to a state that is gone, as do the previous call's
    /// dependencies and recording (a convergence jump must not replay them).
    pub fn intr_after_restore(&mut self) {
        self.intr_verify_on = self.host.intrinsics.verify;
        self.intr_abandon("restored");
        self.host.intrinsics.previous = None;
        self.host.intrinsics.last = None;
    }

    fn intr_abandon(&mut self, why: &'static str) {
        let Some(rec) = self.host.intrinsics.rec.take() else {
            return;
        };
        self.host.intrinsics.last = None;
        if let Some(p) = rec.verify {
            *self.host.intrinsics.unverified.entry(why).or_default() += 1;
            self.drop_checkpoint(p.ck());
        }
    }

    /// Called at the start of `macro_call`, before scanner or input changes.
    pub fn flashtex_intr_observe(&mut self) {
        let cs = self.cur_cs;
        if self.intr_is_named(cs, b"siunitx_unit_pdfstring_context:") {
            self.host.intrinsics.siunitx_context_calls += 1;
        }
        if self.intr_is_named(cs, b"DeclareCommandCopy") {
            self.host.intrinsics.bookmark_copy_calls += 1;
        }
        if !self.intr_is_named(cs, NAME) {
            return;
        }
        self.host.intrinsics.calls += 1;
        if self.host.intrinsics.rec.is_some() {
            self.intr_reject("Nested");
            return;
        }

        // `macro_call` itself tests the same delimiter after the protected
        // marker. A parameter scan is never a candidate.
        let mut p = self.mem[self.cur_chr as usize].hh().rh();
        if self.mem[p as usize].hh().lh() == protected_token {
            p = self.mem[p as usize].hh().rh();
        }
        if self.mem[p as usize].hh().lh() != end_match_token {
            self.intr_reject("Parameters");
            return;
        }
        if !self.intr_at_switch {
            self.intr_reject("NotAtSwitch");
            return;
        }
        if self.intr_int(global_defs_code) != 0 {
            self.intr_reject("GlobalDefs");
            return;
        }
        if self.after_token != 0 {
            self.intr_reject("AfterAssignment");
            return;
        }
        if [
            tracing_macros_code,
            tracing_commands_code,
            tracing_assigns_code,
            tracing_restores_code,
            tracing_groups_code,
            tracing_ifs_code,
            tracing_nesting_code,
        ]
        .iter()
        .any(|&c| self.intr_int(c) != 0)
        {
            self.intr_reject("Tracing");
            return;
        }

        let mut deps = Vec::new();
        let mut n = 0;
        p = self.mem[p as usize].hh().rh();
        while p != NULL && n < MAX_BODY_TOKENS {
            let tok = self.mem[p as usize].hh().lh();
            if tok > cs_token_flag {
                let dep = tok - cs_token_flag;
                if dep != cs && dep > 0 && (dep as usize) <= self.eqtb.len() {
                    deps.push((dep, self.eqtb[(dep - 1) as usize].to_bits()));
                }
            }
            p = self.mem[p as usize].hh().rh();
            n += 1;
        }
        if p != NULL {
            self.intr_reject("BodyTooLong");
            return;
        }
        deps.sort_unstable_by_key(|&(cs, _)| cs);
        deps.dedup_by_key(|dep| dep.0);

        let unicode = deps
            .iter()
            .filter(|&&(cs, _)| self.intr_cs_has_non_ascii(cs))
            .count();
        let report = &mut self.host.intrinsics;
        let changed = report.previous.as_ref().is_some_and(|old| {
            old.iter().any(|&(cs, bits)| {
                deps.iter()
                    .find(|&&(c, _)| c == cs)
                    .is_some_and(|&(_, b)| b != bits)
            })
        });
        if changed {
            *report.rejections.entry("DependenciesChanged").or_default() += 1;
        } else {
            report.eligible += 1;
        }
        report.dependency_tokens += deps.len() as u64;
        report.unicode_dependency_tokens += unicode as u64;
        report.previous = Some(deps);

        // Record this call's real path; check it against the previous
        // recording's replay when the guard would have replayed.
        let base = self.intr_body_level();
        let at = (self.cur_level, self.cur_group);
        let why = match (changed, self.host.intrinsics.last.take()) {
            (false, Some((was, ops))) if was.is_none_or(|w| w == at) => {
                Ok(Some(self.intr_verify_begin(&ops)))
            }
            (false, Some(_)) => Err("other_group"),
            (false, None) => Err("no_recording"),
            (true, _) => Ok(None),
        };
        let verify = why.unwrap_or_else(|w| {
            *self.host.intrinsics.unverified.entry(w).or_default() += 1;
            None
        });
        let marks = self.intr_marks();
        self.host.intrinsics.rec = Some(Rec {
            base,
            at,
            ops: vec![],
            bad: None,
            marks,
            verify,
        });
    }

    /// `changes/intrinsics.ch`: a definition or group boundary (kinds in
    /// the change file), while a real path is recorded.
    pub fn flashtex_intr_op(&mut self, k: i32, p: i32, t: i32, e: i32) {
        let r = &self.host.intrinsics;
        if r.replaying || !r.rec.as_ref().is_some_and(|rec| rec.bad.is_none()) {
            return;
        }
        let global = k & 8 != 0;
        let call_level = r.rec.as_ref().expect("checked").at.0;
        let op = match k {
            // the input files' positions and encodings are not compared
            16 => Err("FileInput"),
            // an `\aftergroup` token left at the call's level (or below,
            // after closing an outer group) is on the save stack after the
            // call, which a replay does not push; one inside a group the
            // call opens comes back at its end and its effects are recorded
            17 if p > level_one && p <= call_level => Err("AfterGroup"),
            17 => return,
            _ => match k & 7 {
                1 if [glue_ref, shape_ref, box_ref].contains(&t) => Err("GlueShapeOrBox"),
                1 => Ok(Op::Def(p, t, self.intr_val(t, e), global)),
                // a replay's group ends would trace (restores, nesting)
                // where the real path's did, or not, as the tracing now is
                2 if is_tracing(p) => Err("Tracing"),
                2 => Ok(Op::Word(p, t, global)),
                3 => Ok(Op::Begin(p)),
                4 => Ok(Op::End),
                5 => Err("SparseRegister"),
                6 => Err("BoxDimen"),
                _ => Err("FileOutput"),
            },
        };
        let rec = self.host.intrinsics.rec.as_mut().expect("checked");
        match op {
            Ok(op) if rec.ops.len() < MAX_OPS => rec.ops.push(op),
            Ok(_) => rec.bad = Some("TooManyOps"),
            Err(why) => rec.bad = Some(why),
        }
    }

    /// How a defined value is recorded. A list another reference already
    /// holds that is the value of `cur_cs` comes from `\let` (or
    /// `\futurelet`): its replay shares that control sequence's value then,
    /// as `\let` does, so that e-TeX's "reassigning" case (the same
    /// pointer) is met alike. Other lists are recorded by their tokens.
    fn intr_val(&self, t: i32, e: i32) -> Val {
        if !self.intr_is_list(t, e) {
            return Val::Plain(e);
        }
        let c = self.cur_cs;
        if self.mem[e as usize].hh().lh() != NULL && c > 0 && (c as usize) <= self.eqtb.len() {
            let w = self.eqtb[(c - 1) as usize];
            if w.hh_rh() == e && w.hh_b0() == t {
                return Val::Cs(c);
            }
        }
        Val::Tokens(self.intr_tokens(self.mem[e as usize].hh().rh()))
    }

    /// `big_switch`: end the recording once the body is used up.
    pub fn flashtex_intr_switch(&mut self) {
        let Some(base) = self.host.intrinsics.rec.as_ref().map(|r| r.base) else {
            return;
        };
        if !self.intr_used_up(base) {
            return;
        }
        self.intr_finish();
    }

    /// `get_next` popped a used-up token list inside `big_switch`'s own
    /// `get_x_token` (`changes/intrinsics.ch`). When that was the body's
    /// level, with no expansion in progress and no macro's parameters
    /// being scanned, the body's last tokens only expanded (`...\fi`, as
    /// LaTeX's `\end` does) and the token `get_next` reads next is the
    /// first after the call: the recording ends here, before it is read,
    /// where the replay's state was taken. At the next `big_switch` it
    /// included that token and the command it is, and the check compared
    /// the two states at different places (#2210). Reads past the call
    /// anywhere else (a primitive's look-ahead, a macro's parameters, an
    /// expansion's scan) are as before.
    pub fn flashtex_intr_popped(&mut self) {
        let Some(base) = self.host.intrinsics.rec.as_ref().map(|r| r.base) else {
            return;
        };
        if self.input_ptr < base
            && self.expand_depth_count == 0
            && self.scanner_status == crate::generated::consts::normal
        {
            self.intr_finish();
        }
    }

    /// The recording is over: classify it, finish its check, keep it.
    fn intr_finish(&mut self) {
        let rec = self.host.intrinsics.rec.take().expect("checked");
        let now = self.intr_marks();
        let why = rec.bad.or_else(|| {
            if self.after_token != 0 {
                return Some("State");
            }
            rec.marks
                .iter()
                .zip(&now)
                .find(|(a, b)| a != b)
                .map(|(a, _)| a.0)
        });
        // (the verdict knows whether this real path is replayable: one that
        // is not did what a replay would not, outside the compared state)
        if let Some(p) = rec.verify {
            self.intr_verify_end(p, why);
        }
        let r = &mut self.host.intrinsics;
        match why {
            Some(w) => *r.not_replayable.entry(w).or_default() += 1,
            None => {
                r.kept += 1;
                r.kept_ops += rec.ops.len() as u64;
                // a recording that ends a group it did not begin replays
                // only at the group it was made in
                let mut depth = 0;
                let outer = rec.ops.iter().any(|o| {
                    depth += matches!(o, Op::Begin(_)) as i32 - matches!(o, Op::End) as i32;
                    depth < 0
                });
                r.last = Some((outer.then_some(rec.at), rec.ops));
            }
        }
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

    /// The state a replayable call leaves as it found it, by reason.
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
            ("Error", self.error_count),
            ("Error", self.history),
            ("State", c.mode_field.abs()),
            ("State", self.nest_ptr),
            ("State", c.tail_field),
            // `new_graf` sets the list's `prev_graf` outside `eq_define`: an
            // empty paragraph the call starts and ends resets it (#2261)
            ("State", c.pg_field),
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
        (crate::generated::consts::call..=crate::generated::consts::long_outer_call).contains(&t)
            && e >= 0
            && (e as usize) < self.mem.len()
    }

    /// The tokens of the list starting at `p` (after its reference count),
    /// all of them: a list in `mem` has fewer nodes than `mem` has words, so
    /// the bound only stops a cycle. A shorter bound recorded a long value
    /// cut short and compared two long lists by their first tokens only, so
    /// a check passed that a replay would get wrong (the pdfTeX side: #2194).
    pub(crate) fn intr_tokens(&self, mut p: i32) -> Vec<i32> {
        let mut v = vec![];
        while p >= 0 && (p as usize) < self.mem.len() && v.len() < self.mem.len() {
            let w = self.mem[p as usize].hh();
            v.push(w.lh());
            p = w.rh();
        }
        v
    }

    fn intr_reject(&mut self, reason: &'static str) {
        *self.host.intrinsics.rejections.entry(reason).or_default() += 1;
    }

    fn intr_int(&self, code: i32) -> i32 {
        self.eqtb[(int_base + code - 1) as usize].int()
    }

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

    fn intr_cs_has_non_ascii(&self, cs: i32) -> bool {
        self.intr_str(cs)
            .is_some_and(|mut r| r.any(|i| self.str_pool[i] > 127))
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

    /// A separate report file keeps TeX's log and output byte-for-byte
    /// identical across off, on (inert), and verify runs.
    pub fn write_intrinsics_report(&mut self) {
        if !self.intr_verify_on {
            return;
        }
        self.intr_abandon("unfinished");
        let Some(path) = std::env::var_os("FLASHTEX_INTRINSICS_STATS") else {
            return;
        };
        let r = &self.host.intrinsics;
        // (the verifier's checkpoints are gone: a driver run has none)
        let checkpoints = self.arena.checkpoint_ids().len();
        let mut s = format!(
            "mode: verify-only\nreplay_enabled: false\ncheckpoints: {checkpoints}\ncalls: {}\neligible: {}\ndependency_tokens: {}\nunicode_dependency_tokens: {}\nalternative.siunitx_context_calls: {}\nalternative.bookmark_copy_calls: {}\nrecordings_kept: {}\nrecorded_ops: {}\nsemantic.checked: {}\nsemantic.identical: {}\nsemantic.differing: {}\n",
            r.calls,
            r.eligible,
            r.dependency_tokens,
            r.unicode_dependency_tokens,
            r.siunitx_context_calls,
            r.bookmark_copy_calls,
            r.kept,
            r.kept_ops,
            r.checked,
            r.identical,
            r.differing,
        );
        let groups = [
            ("rejected", &r.rejections),
            ("semantic.differing", &r.classes),
            ("semantic.unverified", &r.unverified),
            ("not_replayable", &r.not_replayable),
        ];
        for (prefix, map) in groups {
            for (&reason, &count) in map {
                s.push_str(&format!("{prefix}.{reason}: {count}\n"));
            }
        }
        for d in &r.details {
            s.push_str(&format!("semantic.detail: {d}\n"));
        }
        if let Err(e) = std::fs::write(&path, s) {
            eprintln!("XeTeX intrinsics report {}: {e}", path.to_string_lossy());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Globals;

    /// A checkpoint taken while verification was off (an earlier run's
    /// S₀, or one read from a file) does not turn it off in a verifying
    /// run that restores it.
    #[test]
    fn a_restore_keeps_the_runs_verify_mode() {
        let mut g = Globals::new();
        let s0 = g.checkpoint();
        assert!(!g.intr_verify_on);
        g.set_intrinsics_verify(true);
        g.restore(s0).unwrap();
        assert!(g.intr_verify_on);
        let s0 = g.checkpoint();
        g.set_intrinsics_verify(false);
        g.restore(s0).unwrap();
        assert!(!g.intr_verify_on);
    }
}
