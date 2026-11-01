//! Medians, percentiles and the log-spaced retention policy.

use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Samples {
    pub ns: Vec<f64>,
}

impl Samples {
    pub fn new() -> Samples {
        Samples { ns: Vec::new() }
    }
    pub fn push(&mut self, d: Duration) {
        self.ns.push(d.as_nanos() as f64);
    }
    pub fn push_ns(&mut self, ns: f64) {
        self.ns.push(ns);
    }
    pub fn len(&self) -> usize {
        self.ns.len()
    }
    pub fn is_empty(&self) -> bool {
        self.ns.is_empty()
    }

    fn sorted(&self) -> Vec<f64> {
        let mut v = self.ns.clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v
    }

    pub fn quantile(&self, q: f64) -> f64 {
        if self.ns.is_empty() {
            return f64::NAN;
        }
        let v = self.sorted();
        // Nearest-rank: no interpolation, so a reported figure is always an observation.
        let idx = ((v.len() as f64 - 1.0) * q).round() as usize;
        v[idx]
    }
    pub fn median(&self) -> f64 {
        self.quantile(0.5)
    }
    pub fn max(&self) -> f64 {
        self.sorted().last().copied().unwrap_or(f64::NAN)
    }
}

impl Default for Samples {
    fn default() -> Self {
        Samples::new()
    }
}

/// TeXpresso-style decimation: dense near the newest checkpoint, log-spaced behind it.
///
/// Returns, for a run of `total` checkpoints, the indices still retained once the newest
/// is index `total - 1`. `dense` newest checkpoints are always kept; behind them each
/// power-of-two band of distance keeps `per_octave` of its members.
pub fn log_spaced_retention(total: usize, dense: usize, per_octave: usize) -> Vec<usize> {
    let mut keep = Vec::new();
    if total == 0 {
        return keep;
    }
    let newest = total - 1;
    for k in 0..total {
        let d = newest - k;
        if d < dense {
            keep.push(k);
            continue;
        }
        // Distance band: dense..2*dense, 2*dense..4*dense, ...
        let band = (d / dense.max(1)).max(1);
        let octave = usize::BITS - 1 - band.leading_zeros(); // floor(log2(band))
        let stride = ((dense.max(1) << octave) / per_octave.max(1)).max(1);
        // The test is on `k`, not on `d`, and that is the whole point.
        //
        // Decimation is applied again at every step, and a checkpoint that has been
        // dropped cannot come back, so the policy is only usable if the retained set
        // *shrinks* as the newest index advances. `d` grows by one per step, so a test on
        // `d` throws a checkpoint away the moment its distance hits a value the stride does
        // not divide — every checkpoint fails that within `dense` steps of leaving the dense
        // tail, and the policy degenerates to keeping only the dense tail. A test on `k` is
        // nested instead: the strides are successive doublings, so
        // {k : k % 2s == 0} is a subset of {k : k % s == 0}, and a checkpoint that survives
        // into a wider band was already retained in every narrower one.
        if k % stride == 0 {
            keep.push(k);
        }
    }
    keep.sort_unstable();
    keep.dedup();
    keep
}

pub fn fmt_bytes(b: usize) -> String {
    let b = b as f64;
    if b >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} GiB", b / (1024.0 * 1024.0 * 1024.0))
    } else if b >= 1024.0 * 1024.0 {
        format!("{:.1} MiB", b / (1024.0 * 1024.0))
    } else if b >= 1024.0 {
        format!("{:.1} KiB", b / 1024.0)
    } else {
        format!("{b:.0} B")
    }
}

/// JSON escaping for the machine-readable output. Only what the keys and names need.
pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_keeps_the_dense_tail_and_thins_behind_it() {
        let keep = log_spaced_retention(1000, 16, 4);
        // The newest 16 are all there.
        for d in 0..16 {
            assert!(keep.contains(&(999 - d)), "missing distance {d}");
        }
        // And the whole set is far smaller than 1000 but not trivially small.
        assert!(
            (24..96).contains(&keep.len()),
            "retained {} checkpoints, expected a few dozen",
            keep.len()
        );
        // Strictly increasing, no duplicates.
        assert!(keep.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn retention_of_a_short_run_keeps_everything() {
        assert_eq!(log_spaced_retention(10, 16, 4).len(), 10);
    }

    /// The property the incremental thinning depends on: decimation is re-applied at every
    /// step and a dropped checkpoint cannot return, so the retained set must only ever lose
    /// members (plus the new newest one). Without this the policy collapses to the dense
    /// tail and every memory figure for 1,000 checkpoints is wrong.
    #[test]
    fn retention_is_monotone_as_the_newest_index_advances() {
        let mut prev = log_spaced_retention(1, 16, 4);
        for n in 2..=600usize {
            let now = log_spaced_retention(n, 16, 4);
            for k in &now {
                assert!(
                    *k == n - 1 || prev.contains(k),
                    "checkpoint {k} is retained at n={n} but was not at n={}",
                    n - 1
                );
            }
            prev = now;
        }
    }

    /// And the steady-state size is the dense tail plus a fixed number per octave.
    #[test]
    fn retention_size_is_dense_plus_per_octave() {
        let keep = log_spaced_retention(1000, 16, 4);
        // 16 dense, then 4 per octave over 6 octaves of distance up to 1000.
        assert_eq!(keep.len(), 40, "retained {keep:?}");
    }

    #[test]
    fn quantiles_are_observations() {
        let mut s = Samples::new();
        for v in [5.0, 1.0, 3.0, 2.0, 4.0] {
            s.push_ns(v);
        }
        assert_eq!(s.median(), 3.0);
        assert_eq!(s.quantile(0.0), 1.0);
        assert_eq!(s.max(), 5.0);
    }
}
