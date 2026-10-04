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
    /// (when, the part from then on), oldest first.
    ring: std::collections::VecDeque<(Instant, Part)>,
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
        s.ring.push_back((Instant::now(), p));
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
    match s.ring.iter().rposition(|(t, _)| *t <= t0) {
        Some(i) => changes.extend(s.ring.range(i..).copied()),
        None if !s.wrapped => {
            changes.push((t0, Part::Idle));
            changes.extend(s.ring.iter().copied());
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
            }
        }
        let v = since(t0);
        let get = |n: &str| v.iter().find(|(k, _)| *k == n).map_or(0.0, |x| x.1);
        assert!(get("test") >= 2.5, "{v:?}");
        assert!(get("jump") >= 1.5, "{v:?}");
        assert_eq!(now(), Part::Idle);
    }
}
