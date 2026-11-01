//! Summary statistics for a set of samples.
//!
//! Every reported number is an order statistic. The mean is recorded but
//! never compared against: one build lane waking up mid-run moves a mean and
//! barely moves a median, and this machine always has other lanes on it.

use flashtex_compiler::json::{self, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub n: usize,
    pub min: f64,
    pub p25: f64,
    pub median: f64,
    pub p75: f64,
    pub p95: f64,
    pub max: f64,
    pub mean: f64,
    /// Median absolute deviation — spread that a single slow sample cannot
    /// inflate, unlike a standard deviation.
    pub mad: f64,
    /// `(p75 - p25) / median`. The run's own noise estimate for this metric;
    /// the regression gate refuses to call anything smaller than this a
    /// regression.
    pub rel_iqr: f64,
}

/// Nearest-rank percentile over an already-sorted slice.
fn pct(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let i = (((sorted.len() - 1) as f64) * q).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

pub fn summarize(samples: &[f64]) -> Stats {
    let mut s: Vec<f64> = samples.iter().copied().filter(|x| x.is_finite()).collect();
    if s.is_empty() {
        return Stats { n: 0, min: f64::NAN, p25: f64::NAN, median: f64::NAN, p75: f64::NAN, p95: f64::NAN, max: f64::NAN, mean: f64::NAN, mad: f64::NAN, rel_iqr: f64::NAN };
    }
    s.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let median = pct(&s, 0.5);
    let p25 = pct(&s, 0.25);
    let p75 = pct(&s, 0.75);
    let mut dev: Vec<f64> = s.iter().map(|x| (x - median).abs()).collect();
    dev.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    Stats {
        n: s.len(),
        min: s[0],
        p25,
        median,
        p75,
        p95: pct(&s, 0.95),
        max: s[s.len() - 1],
        mean: s.iter().sum::<f64>() / s.len() as f64,
        mad: pct(&dev, 0.5),
        rel_iqr: if median > 0.0 { (p75 - p25) / median } else { 0.0 },
    }
}

impl Stats {
    pub fn to_json(&self) -> Value {
        let mut v = Value::obj();
        v.set("n", json::num(self.n as f64));
        for (k, x) in [
            ("min", self.min),
            ("p25", self.p25),
            ("median", self.median),
            ("p75", self.p75),
            ("p95", self.p95),
            ("max", self.max),
            ("mean", self.mean),
            ("mad", self.mad),
            ("rel_iqr", self.rel_iqr),
        ] {
            v.set(k, json::num(round6(x)));
        }
        v
    }

    pub fn from_json(v: &Value) -> Option<Stats> {
        let num = |k: &str| match v.get(k) {
            Some(Value::Num(n)) => Some(*n),
            _ => None,
        };
        Some(Stats {
            n: num("n")? as usize,
            min: num("min")?,
            p25: num("p25")?,
            median: num("median")?,
            p75: num("p75")?,
            p95: num("p95")?,
            max: num("max")?,
            mean: num("mean")?,
            mad: num("mad")?,
            rel_iqr: num("rel_iqr")?,
        })
    }
}

/// Six decimals: enough for a 1 µs metric, few enough that the JSON is
/// stable under re-serialisation.
pub fn round6(x: f64) -> f64 {
    if x.is_finite() {
        (x * 1e6).round() / 1e6
    } else {
        0.0
    }
}
