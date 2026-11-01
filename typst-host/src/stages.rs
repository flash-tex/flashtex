//! Stage counters for one compile's way to its first page (DESIGN.md §15.3):
//! where the time between the compile and the edited page on the socket
//! goes. The server thread accumulates them; `DONE` reports them under
//! `stages` (spec §11.9), in milliseconds.

use std::cell::Cell;
use std::time::Instant;

/// A stage of the first page's way to the socket.
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    /// typst-pdf's export of the pages whose positions are derived.
    Export,
    /// Reading that export: parsing, inflating and interpreting it.
    PdfRead,
    /// Resolving source spans to files and lines (`World::source`).
    Span,
}

const N: usize = 3;

thread_local! {
    static ACC: Cell<[f64; N]> = const { Cell::new([0.0; N]) };
}

/// Clear the counters (at the start of a compile).
pub fn reset() {
    ACC.with(|a| a.set([0.0; N]));
}

/// Add the time since `t` to `s`.
pub fn add(s: Stage, t: Instant) {
    let d = t.elapsed().as_secs_f64() * 1e3;
    ACC.with(|a| {
        let mut v = a.get();
        v[s as usize] += d;
        a.set(v);
    });
}

/// The time spent in `s` since the last [`reset`].
pub fn get(s: Stage) -> f64 {
    ACC.with(|a| a.get()[s as usize])
}

/// Time `f` as stage `s`.
pub fn time<T>(s: Stage, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    add(s, t);
    r
}
