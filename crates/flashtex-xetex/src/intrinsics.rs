//! XeTeX guarded-intrinsic first slice: observe calls without replay.
//!
//! `eligible` means the registered parameterless macro reached `big_switch`
//! with the entry guards clear. It is not a purity verdict. Dynamic reads,
//! writes, inter-character insertion, native handles and state comparison are
//! intentionally not covered, so no call may be replayed in this slice.

use crate::generated::consts::{
    cs_token_flag, end_match_token, global_defs_code, hash_base, int_base, protected_token,
    tracing_assigns_code, tracing_commands_code, tracing_groups_code, tracing_ifs_code,
    tracing_macros_code, tracing_restores_code,
};
use crate::Globals;
use std::collections::BTreeMap;

const NAME: &[u8] = b"pdfstringdefPreHook";
const MAX_BODY_TOKENS: usize = 4096;
// xetex.web's `null` is negative, unlike a C null pointer.
const NULL: i32 = -0x0fff_ffff;

#[derive(Default)]
pub struct Report {
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
}

/// `on` is deliberately inert until the promotion gate has been met.
pub fn enabled() -> bool {
    matches!(
        std::env::var("FLASHTEX_INTRINSICS").as_deref(),
        Ok("verify" | "report")
    )
}

impl Globals {
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

        // `macro_call` itself tests the same delimiter after the protected
        // marker. A parameter scan is never a first-slice candidate.
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
        if report.previous.as_ref().is_some_and(|old| {
            old.iter().any(|&(cs, bits)| {
                deps.iter()
                    .find(|&&(c, _)| c == cs)
                    .is_some_and(|&(_, b)| b != bits)
            })
        }) {
            *report.rejections.entry("DependenciesChanged").or_default() += 1;
        } else {
            report.eligible += 1;
        }
        report.dependency_tokens += deps.len() as u64;
        report.unicode_dependency_tokens += unicode as u64;
        report.previous = Some(deps);
    }

    fn intr_reject(&mut self, reason: &'static str) {
        *self.host.intrinsics.rejections.entry(reason).or_default() += 1;
    }

    fn intr_int(&self, code: i32) -> i32 {
        self.eqtb[(int_base + code - 1) as usize].int()
    }

    fn intr_is_named(&self, cs: i32, name: &[u8]) -> bool {
        if cs < hash_base || cs >= hash_base + self.hash.len() as i32 {
            return false;
        }
        let s = self.hash[(cs - hash_base) as usize].rh();
        if !(65536..self.str_ptr).contains(&s) {
            return false;
        }
        let a = self.str_start[(s - 65536) as usize] as usize;
        let b = self.str_start[(s + 1 - 65536) as usize] as usize;
        b - a == name.len() && (a..b).zip(name).all(|(i, &c)| self.str_pool[i] == c as i32)
    }

    fn intr_cs_has_non_ascii(&self, cs: i32) -> bool {
        if cs < hash_base || cs >= hash_base + self.hash.len() as i32 {
            return false;
        }
        let s = self.hash[(cs - hash_base) as usize].rh();
        if !(65536..self.str_ptr).contains(&s) {
            return false;
        }
        let a = self.str_start[(s - 65536) as usize] as usize;
        let b = self.str_start[(s + 1 - 65536) as usize] as usize;
        (a..b).any(|i| self.str_pool[i] > 127)
    }

    /// A separate report file keeps TeX's log and output byte-for-byte
    /// identical across off, on (inert), and verify runs.
    pub fn write_intrinsics_report(&self) {
        if !self.intr_verify_on {
            return;
        }
        let Some(path) = std::env::var_os("FLASHTEX_INTRINSICS_STATS") else {
            return;
        };
        let r = &self.host.intrinsics;
        let mut s = format!(
            "mode: verify-only\nreplay_enabled: false\ncalls: {}\neligible: {}\ndependency_tokens: {}\nunicode_dependency_tokens: {}\nalternative.siunitx_context_calls: {}\nalternative.bookmark_copy_calls: {}\nsemantic_differences: not_checked\n",
            r.calls,
            r.eligible,
            r.dependency_tokens,
            r.unicode_dependency_tokens,
            r.siunitx_context_calls,
            r.bookmark_copy_calls
        );
        for (&reason, &count) in &r.rejections {
            s.push_str(&format!("rejected.{reason}: {count}\n"));
        }
        if let Err(e) = std::fs::write(&path, s) {
            eprintln!("XeTeX intrinsics report {}: {e}", path.to_string_lossy());
        }
    }
}
