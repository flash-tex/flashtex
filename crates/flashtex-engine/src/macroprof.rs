//! A macro-level profiler (DESIGN.md §5.6 item 1: "profile first"), off by
//! default.
//!
//! `FLASHTEX_MACRO_PROFILE=FILE` switches it on (`macro_prof_on`,
//! changes/intrinsics.ch): `macro_call` reports every macro body it feeds to
//! the scanner (`flashtex_prof_enter`) and `end_token_list` every macro level
//! it leaves (`flashtex_prof_leave`). A shadow stack of the macro levels on
//! the input stack attributes the time between two events to the innermost
//! macro level ("self" time: the macro's own tokens and every primitive run
//! while its body is the innermost macro, including typesetting it
//! triggers), and the time from entering a macro to leaving it to that
//! macro ("inclusive", counted once when a macro is active recursively).
//! TeX leaves a macro whose body is used up *before* it feeds the next one
//! (§390), so a macro's last call is not part of its inclusive time.
//!
//! At the end of the run the table is written to FILE, tab-separated,
//! sorted by self time: `self_ns incl_ns calls name`, plus a line for the
//! time outside any macro (`<none>`) and the number of shipouts.
//!
//! The clock is the ARM generic timer (24 MHz on Apple silicon), read in a
//! few nanoseconds; elsewhere `Instant`. Profiling only reads the engine's
//! state; its cost is its own (about 2x on macro-heavy documents).

use crate::generated::Globals;
use std::cell::RefCell;

#[derive(Default)]
struct Prof {
    /// (input level, control sequence, entry tick)
    stack: Vec<(i32, i32, u64)>,
    last: u64,
    self_t: Vec<u64>,
    incl: Vec<u64>,
    calls: Vec<u64>,
    active: Vec<u32>,
    none_t: u64,
    out: String,
}

thread_local! {
    static P: RefCell<Option<Prof>> = const { RefCell::new(None) };
}

#[inline(always)]
fn tick() -> u64 {
    #[cfg(target_arch = "aarch64")]
    {
        let t: u64;
        // SAFETY: CNTVCT_EL0 is readable at EL0 on macOS and Linux.
        unsafe { std::arch::asm!("mrs {}, cntvct_el0", out(reg) t) };
        t
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        use std::sync::OnceLock;
        static T0: OnceLock<std::time::Instant> = OnceLock::new();
        T0.get_or_init(std::time::Instant::now).elapsed().as_nanos() as u64
    }
}

fn ns_per_tick() -> f64 {
    #[cfg(target_arch = "aarch64")]
    {
        let f: u64;
        // SAFETY: CNTFRQ_EL0 is readable at EL0.
        unsafe { std::arch::asm!("mrs {}, cntfrq_el0", out(reg) f) };
        1e9 / f.max(1) as f64
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        1.0
    }
}

/// Switch the profiler on if `FLASHTEX_MACRO_PROFILE` names an output file.
pub fn start_from_env(g: &mut Globals) {
    if let Some(f) = std::env::var_os("FLASHTEX_MACRO_PROFILE") {
        let n = g.eqtb.len() + 1;
        P.with(|p| {
            *p.borrow_mut() = Some(Prof {
                self_t: vec![0; n],
                incl: vec![0; n],
                calls: vec![0; n],
                active: vec![0; n],
                last: tick(),
                out: f.to_string_lossy().into_owned(),
                ..Default::default()
            })
        });
        g.macro_prof_on = true;
    }
}

impl Prof {
    #[inline]
    fn charge(&mut self, now: u64) {
        let d = now.wrapping_sub(self.last);
        self.last = now;
        match self.stack.last() {
            Some(&(_, cs, _)) => self.self_t[cs as usize] += d,
            None => self.none_t += d,
        }
    }
    fn pop_to(&mut self, level: i32, now: u64) {
        while let Some(&(l, cs, t0)) = self.stack.last() {
            if l < level {
                break;
            }
            self.stack.pop();
            let a = &mut self.active[cs as usize];
            *a = a.saturating_sub(1);
            if *a == 0 {
                self.incl[cs as usize] += now.wrapping_sub(t0);
            }
        }
    }
}

impl Globals {
    /// `macro_call` fed the body of `cs` to the scanner (input level
    /// `input_ptr`).
    pub fn flashtex_prof_enter(&mut self, cs: i32) {
        let level = self.input_ptr;
        P.with(|p| {
            if let Some(p) = p.borrow_mut().as_mut() {
                let now = tick();
                p.charge(now);
                p.pop_to(level, now);
                let c = (cs.max(0) as usize).min(p.calls.len() - 1) as i32;
                p.calls[c as usize] += 1;
                p.active[c as usize] += 1;
                p.stack.push((level, c, now));
            }
        });
    }

    /// `end_token_list` is leaving the macro level `input_ptr`.
    pub fn flashtex_prof_leave(&mut self) {
        let level = self.input_ptr;
        P.with(|p| {
            if let Some(p) = p.borrow_mut().as_mut() {
                let now = tick();
                p.charge(now);
                p.pop_to(level, now);
            }
        });
    }

    /// The name of control sequence `p` (without the escape character).
    pub fn cs_name_string(&self, p: i32) -> String {
        const HASH_BASE: i32 = 514;
        const SINGLE_BASE: i32 = 257;
        const NULL_CS: i32 = 513;
        let s = if p < HASH_BASE {
            if p < SINGLE_BASE {
                return format!("<active {}>", p - 1);
            } else if p == NULL_CS {
                return "csname\\endcsname".into();
            } else {
                return char::from_u32((p - SINGLE_BASE) as u32).map_or("?".into(), |c| c.to_string());
            }
        } else {
            let i = (p - HASH_BASE) as usize;
            if i >= self.hash.len() {
                return format!("<cs {p}>");
            }
            self.hash[i].rh()
        };
        self.str_string(s)
    }

    /// String number `s` of the pool as text.
    pub fn str_string(&self, s: i32) -> String {
        if s < 0 || s >= self.str_ptr {
            return format!("<str {s}>");
        }
        let (a, b) = (self.str_start[s as usize] as usize, self.str_start[s as usize + 1] as usize);
        self.str_pool[a..b].iter().map(|&c| c as u8 as char).collect()
    }

    /// Write the profile (end of the run).
    pub fn flashtex_prof_finish(&mut self) {
        let Some(mut p) = P.with(|p| p.borrow_mut().take()) else { return };
        let now = tick();
        p.charge(now);
        p.pop_to(i32::MIN, now);
        let k = ns_per_tick();
        let mut rows: Vec<(u64, u64, u64, i32)> = (0..p.calls.len())
            .filter(|&i| p.calls[i] > 0)
            .map(|i| (p.self_t[i], p.incl[i], p.calls[i], i as i32))
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0));
        let total: u64 = rows.iter().map(|r| r.0).sum::<u64>() + p.none_t;
        let mut s = String::new();
        s.push_str(&format!(
            "# total_ns={:.0} none_ns={:.0} shipouts={}\n# self_ns\tincl_ns\tcalls\tname\n",
            total as f64 * k,
            p.none_t as f64 * k,
            self.dead_cycles.max(0) + self.total_pages
        ));
        for (st, inc, c, cs) in rows {
            s.push_str(&format!(
                "{:.0}\t{:.0}\t{}\t{}\n",
                st as f64 * k,
                inc as f64 * k,
                c,
                self.cs_name_string(cs)
            ));
        }
        if let Err(e) = std::fs::write(&p.out, s) {
            eprintln!("flashtex: cannot write the macro profile {}: {e}", p.out);
        }
    }
}
