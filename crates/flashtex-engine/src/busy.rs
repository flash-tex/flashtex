//! What the engine thread is doing, for the latency accounting of a
//! keystroke that arrives while it is busy (lane LIVE-30MS).
//!
//! The engine thread marks the parts of its work (`enter`, a guard that
//! puts the previous part back when dropped); every change is kept with its
//! time in a short ring. When a newer `COMPILE` waits for the engine thread
//! (DONE's `queue`), [`since`] splits that wait by part: which work held it
//! up, and for how long. Marking costs a lock and a push per change, a few
//! dozen per compile; nothing the engine computes depends on it.

use std::sync::Mutex;
use std::time::Instant;

/// A part of the engine thread's work.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Part {
    /// Waiting for a request (or keeping warm).
    Idle,
    /// A compile's request: parsing, applying the edits, the S₀ key, what changed.
    Request,
    /// Settling or abandoning a paused (preempted) run.
    Paused,
    /// Restoring the restart point.
    Restore,
    /// Typesetting (the default inside a run).
    Typeset,
    /// A convergence test.
    Test,
    /// The convergence jump and the bookkeeping after a run.
    Jump,
    /// After the run: diagnostics, `DONE`.
    Done,
    /// `prepare_next`: the next keystroke's restore, worked out ahead.
    Prepare,
    /// Saving S₀, the external tools' start, a heap trim.
    Other,
}

impl Part {
    pub fn name(self) -> &'static str {
        match self {
            Part::Idle => "idle",
            Part::Request => "request",
            Part::Paused => "paused",
            Part::Restore => "restore",
            Part::Typeset => "typeset",
            Part::Test => "test",
            Part::Jump => "jump",
            Part::Done => "done",
            Part::Prepare => "prepare",
            Part::Other => "other",
        }
    }
}

const RING: usize = 256;

struct State {
    now: Part,
    /// (when, the part from then on, the engine thread's cycles then, and
    /// its instructions), oldest first.
    ring: std::collections::VecDeque<(Instant, Part, u64, u64)>,
    /// The ring dropped its oldest change (else the part before the first
    /// one is `Idle`, the initial part).
    wrapped: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    now: Part::Idle,
    ring: std::collections::VecDeque::new(),
    wrapped: false,
});

fn set(p: Part) -> Part {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let prev = s.now;
    if prev != p {
        s.now = p;
        if s.ring.len() == RING {
            s.ring.pop_front();
            s.wrapped = true;
        }
        let (instr, cycles) = crate::os::thread_counts().unwrap_or((0, 0));
        s.ring.push_back((Instant::now(), p, cycles, instr));
    }
    prev
}

/// Puts the previous part back when dropped.
pub struct Guard(Part);

impl Drop for Guard {
    fn drop(&mut self) {
        set(self.0);
    }
}

/// The engine thread is in part `p` until the guard is dropped.
#[must_use]
pub fn enter(p: Part) -> Guard {
    Guard(set(p))
}

/// The part now (the connection thread, when a request arrives).
pub fn now() -> Part {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).now
}

/// The time from `t0` to now by part, in ms, largest first (parts that
/// changed before `t0` count from `t0`). Empty when the ring no longer
/// reaches back to `t0`.
pub fn since(t0: Instant) -> Vec<(&'static str, f64)> {
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let end = Instant::now();
    // the part at t0: the last change at or before it (none: the initial
    // part, unless the ring no longer reaches back that far)
    let mut changes: Vec<(Instant, Part)> = vec![];
    match s.ring.iter().rposition(|(t, _, _, _)| *t <= t0) {
        Some(i) => changes.extend(s.ring.range(i..).map(|&(t, p, _, _)| (t, p))),
        None if !s.wrapped => {
            changes.push((t0, Part::Idle));
            changes.extend(s.ring.iter().map(|&(t, p, _, _)| (t, p)));
        }
        None => return vec![],
    }
    let mut acc: Vec<(&'static str, f64)> = vec![];
    for k in 0..changes.len() {
        let (t, p) = changes[k];
        let from = t.max(t0);
        let to = changes.get(k + 1).map_or(end, |n| n.0);
        let ms = to.saturating_duration_since(from).as_secs_f64() * 1e3;
        match acc.iter_mut().find(|(n, _)| *n == p.name()) {
            Some(a) => a.1 += ms,
            None => acc.push((p.name(), ms)),
        }
    }
    acc.sort_by(|a, b| b.1.total_cmp(&a.1));
    acc
}

/// The engine thread's work from `t0` to now by part: (part, instructions,
/// cycles), largest cycle count first. The counterpart of [`since`] in
/// counts, which load does not change (a keystroke that waited behind the
/// previous compile's background work: which work, and how much of it;
/// lane P4-TYPING-200WPM). The interval the ring has at `t0` is counted from
/// `t0` by its share of wall time, as [`cycles_at`] does. The engine thread
/// calls it; empty where the counters are not available or the ring no
/// longer reaches `t0`.
pub fn counts_since(t0: Instant) -> Vec<(&'static str, u64, u64)> {
    let Some((ni, nc)) = crate::os::thread_counts() else {
        return vec![];
    };
    let end = Instant::now();
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    // the change at or before t0; none: the ring starts after t0 (then the
    // time before its first change was idle), unless it wrapped
    let (i, partial) = match s.ring.iter().rposition(|(t, _, _, _)| *t <= t0) {
        Some(i) => (i, true),
        None if !s.wrapped && !s.ring.is_empty() => (0, false),
        None => return vec![],
    };
    let mut acc: Vec<(&'static str, u64, u64)> = vec![];
    for k in i..s.ring.len() {
        let (ta, p, ca, ia) = s.ring[k];
        let (tb, cb, ib) = s
            .ring
            .get(k + 1)
            .map_or((end, nc, ni), |&(t, _, c, n)| (t, c, n));
        let (mut dc, mut di) = (cb.saturating_sub(ca), ib.saturating_sub(ia));
        if partial && k == i {
            // only the part of the first interval after t0
            let span = tb.saturating_duration_since(ta).as_secs_f64();
            let after = tb.saturating_duration_since(t0).as_secs_f64();
            let f = if span > 0.0 {
                (after / span).min(1.0)
            } else {
                1.0
            };
            dc = (dc as f64 * f) as u64;
            di = (di as f64 * f) as u64;
        }
        match acc.iter_mut().find(|(n, _, _)| *n == p.name()) {
            Some(a) => {
                a.1 += di;
                a.2 += dc;
            }
            None => acc.push((p.name(), di, dc)),
        }
    }
    acc.sort_by_key(|a| std::cmp::Reverse(a.2));
    acc
}

/// The engine thread's cycle count (`os::thread_counts`) at `t0`,
/// interpolated between the changes of part around it (by wall time; the
/// engine thread calls this, and its count now closes the last interval).
/// A request's arrival mark: the engine cycles from it to a page's
/// (`DONE.stages`'s `arrival_mark_kc`, `first_page_mark_kc`) are the work the
/// engine thread did meanwhile, which load does not change (lane
/// LIVE-30MS: keystroke latency in engine cycles). `None` where the
/// counters are not available or the ring no longer reaches `t0`.
pub fn cycles_at(t0: Instant) -> Option<u64> {
    let now = (Instant::now(), crate::os::thread_counts()?.1);
    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let i = s.ring.iter().rposition(|(t, _, _, _)| *t <= t0)?;
    let (ta, _, ca, _) = s.ring[i];
    let (tb, cb) = s.ring.get(i + 1).map_or(now, |&(t, _, c, _)| (t, c));
    let span = tb.saturating_duration_since(ta).as_secs_f64();
    let part = t0.saturating_duration_since(ta).as_secs_f64();
    let f = if span > 0.0 {
        (part / span).min(1.0)
    } else {
        1.0
    };
    Some(ca + ((cb.saturating_sub(ca)) as f64 * f) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_wait_by_part() {
        let t0 = Instant::now();
        {
            let _a = enter(Part::Test);
            std::thread::sleep(std::time::Duration::from_millis(3));
            {
                let _b = enter(Part::Jump);
                std::thread::sleep(std::time::Duration::from_millis(2));
                // work the counters see (instructions, not sleep)
                let mut x = 0u64;
                for i in 0..2_000_000u64 {
                    x = std::hint::black_box(x.wrapping_mul(31).wrapping_add(i));
                }
                std::hint::black_box(x);
            }
        }
        let v = since(t0);
        let get = |n: &str| v.iter().find(|(k, _)| *k == n).map_or(0.0, |x| x.1);
        assert!(get("test") >= 2.5, "{v:?}");
        assert!(get("jump") >= 1.5, "{v:?}");
        // the same wait in counts, where the counters are available
        let c = counts_since(t0);
        if crate::os::thread_counts().is_some() {
            let instr = |n: &str| c.iter().find(|(k, _, _)| *k == n).map_or(0, |x| x.1);
            assert!(instr("jump") >= 2_000_000, "{c:?}");
            assert!(
                instr("jump") > instr("test"),
                "{c:?}: the sleep costs no instructions"
            );
        } else {
            assert!(c.is_empty());
        }
        assert_eq!(now(), Part::Idle);
    }
}
