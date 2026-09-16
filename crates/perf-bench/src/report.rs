//! Aggregation, the machine-readable report, the human table, and the
//! regression gate.
//!
//! The JSON follows the house shape (`schema_version` + `meta` + payload)
//! used by `docs/evidence/real-world-corpus-*/report.json`, so the evidence
//! directory stays readable with one set of habits.

use std::collections::BTreeMap;

use flashtex_compiler::json::{self, Value};

use crate::measure::{Phases, Sample};
use crate::stats::{round6, Stats};

pub const SCHEMA_VERSION: f64 = 1.0;

/// One case's aggregate: metric name -> order statistics over the repetitions.
pub struct CaseReport {
    pub id: String,
    pub group: String,
    pub bytes: usize,
    pub documents: usize,
    pub pages: usize,
    pub passes: u32,
    pub metrics: BTreeMap<String, Stats>,
    pub digests: BTreeMap<String, String>,
    /// `sum(phases) / cold.render_ms`. Below ~0.85 means the decomposition
    /// does not replay everything the product path does for this document
    /// (extra label passes, floats), so attribute regressions with care.
    pub coverage: f64,
    pub cache_hit_rate: f64,
    pub load_max: f64,
    pub notes: Vec<String>,
}

/// Turns N repetitions of one case into one aggregate.
pub fn aggregate(group: &str, samples: &[Sample]) -> CaseReport {
    let first = &samples[0];
    let mut metrics: BTreeMap<String, Stats> = BTreeMap::new();
    let mut push = |name: &str, xs: Vec<f64>| {
        if xs.iter().any(|x| x.is_finite()) {
            metrics.insert(name.to_string(), crate::stats::summarize(&xs));
        }
    };

    push("cold.fontset_ms", samples.iter().map(|s| s.fontset_ms).collect());
    push("cold.first_render_ms", samples.iter().map(|s| s.first_render_ms).collect());
    push("cold.render_ms", samples.iter().map(|s| s.cold_render_ms).collect());
    push("cold.render_nocache_ms", samples.iter().map(|s| s.cold_nocache_ms).collect());
    push("export.pdf_ms", samples.iter().map(|s| s.export_pdf_ms).collect());
    push("export.pdf_warm_ms", samples.iter().map(|s| s.export_pdf_warm_ms).collect());
    for n in Phases::NAMES {
        push(&format!("cold.phase.{n}_ms"), samples.iter().map(|s| s.phases.get(n)).collect());
    }
    push("mem.peak_rss_kb", samples.iter().filter_map(|s| s.peak_rss_kb).map(|x| x as f64).collect());
    push("mem.steady_rss_kb", samples.iter().filter_map(|s| s.steady_rss_kb).map(|x| x as f64).collect());

    // Warm: every timed step of every repetition, pooled. p95 over a pooled
    // set is the number a typist actually notices; a p95 over five per-run
    // medians would be an order statistic of an order statistic.
    let mut warm_names: Vec<(String, String)> = Vec::new();
    for s in samples {
        for w in &s.warm {
            let key = (w.name.clone(), w.caps.clone());
            if !warm_names.contains(&key) {
                warm_names.push(key);
            }
        }
    }
    for (name, caps) in &warm_names {
        let pooled: Vec<f64> = samples
            .iter()
            .flat_map(|s| s.warm.iter().filter(|w| w.name == *name && w.caps == *caps))
            .flat_map(|w| w.steps_ms.iter().copied())
            .collect();
        push(&format!("warm.{name}.{caps}_ms"), pooled);
    }

    let mut digests: BTreeMap<String, String> = BTreeMap::new();
    digests.insert("cold.reply".into(), first.digest_reply.clone());
    if !first.digest_pdf.is_empty() {
        digests.insert("export.pdf".into(), first.digest_pdf.clone());
    }
    for w in &first.warm {
        digests.insert(format!("warm.{}.{}", w.name, w.caps), w.digest.clone());
    }

    let mut notes: Vec<String> = Vec::new();
    for s in samples {
        for n in &s.notes {
            if !notes.contains(n) {
                notes.push(n.clone());
            }
        }
        for w in &s.warm {
            if let Some(why) = &w.skipped {
                let n = format!("{}.{}: skipped ({why})", w.name, w.caps);
                if !notes.contains(&n) {
                    notes.push(n);
                }
            }
        }
    }
    // A digest that is not the same in every repetition means the engine is
    // not deterministic, which is a far bigger problem than a slow number.
    for s in &samples[1..] {
        if s.digest_reply != first.digest_reply {
            notes.push(format!("NON-DETERMINISTIC: cold reply digest differs between repetitions ({} vs {})", first.digest_reply, s.digest_reply));
        }
        if s.digest_pdf != first.digest_pdf {
            notes.push(format!("NON-DETERMINISTIC: exported PDF digest differs between repetitions ({} vs {})", first.digest_pdf, s.digest_pdf));
        }
    }

    let cold = metrics.get("cold.render_ms").map_or(f64::NAN, |s| s.median);
    // `expand` overlaps `parse`, and the v1/v2 envelopes are alternatives —
    // only one of them is on any one request path. Counting any of the three
    // would make coverage exceed 1.0 for reasons that have nothing to do with
    // how faithful the decomposition is.
    let phase_sum: f64 = Phases::NAMES
        .iter()
        .filter(|n| !matches!(**n, "expand" | "typeset_notexts" | "json_v1" | "json_v2"))
        .filter_map(|n| metrics.get(&format!("cold.phase.{n}_ms")).map(|s| s.median))
        .sum();
    let hits: u64 = samples.iter().map(|s| s.cache_hits).sum();
    let misses: u64 = samples.iter().map(|s| s.cache_misses).sum();

    CaseReport {
        id: first.case_id.clone(),
        group: group.to_string(),
        bytes: first.bytes,
        documents: first.documents,
        pages: first.pages,
        passes: first.passes,
        coverage: if cold > 0.0 { phase_sum / cold } else { f64::NAN },
        cache_hit_rate: if hits + misses > 0 { hits as f64 / (hits + misses) as f64 } else { f64::NAN },
        // The highest 1-minute load seen around this case. Other build lanes
        // share this machine, so a row measured under load 12 does not
        // compare with the same row under load 2, and the report says which
        // one it was rather than implying an idle box.
        load_max: samples
            .iter()
            .flat_map(|s| [s.load_before, s.load_after])
            .flatten()
            .map(|l| l.0)
            .fold(f64::NAN, f64::max),
        metrics,
        digests,
        notes,
    }
}

// ---------------------------------------------------------------------------
// Targets
// ---------------------------------------------------------------------------

pub struct Target {
    pub id: &'static str,
    pub description: &'static str,
    pub case: &'static str,
    pub metric: TargetMetric,
    pub budget: f64,
    pub unit: &'static str,
}

pub enum TargetMetric {
    /// A single named metric's median.
    Named(&'static str),
    /// The slowest warm scenario's median: a keystroke budget has to hold for
    /// every kind of keystroke, not just the cheapest one.
    WorstWarm,
}

/// The FT-070 acceptance targets, verbatim from the assignment.
pub const TARGETS: &[Target] = &[
    Target { id: "warm-500kb", description: "warm keystroke < 2 ms at 500 KB", case: "synthetic-500kb", metric: TargetMetric::WorstWarm, budget: 2.0, unit: "ms" },
    Target { id: "warm-2mb", description: "warm keystroke < 10 ms at 2 MB", case: "synthetic-2mb", metric: TargetMetric::WorstWarm, budget: 10.0, unit: "ms" },
    Target { id: "cold-hw1", description: "cold HW1 full compile < 10 ms", case: "hw1", metric: TargetMetric::Named("cold.render_ms"), budget: 10.0, unit: "ms" },
    Target { id: "export-hw1", description: "PDF export for HW1 < 10 ms", case: "hw1", metric: TargetMetric::Named("export.pdf_ms"), budget: 10.0, unit: "ms" },
    Target { id: "mem-2mb", description: "steady-state memory < 100 MB at 2 MB", case: "synthetic-2mb", metric: TargetMetric::Named("mem.steady_rss_kb"), budget: 102_400.0, unit: "KiB" },
];

pub struct TargetResult {
    pub id: &'static str,
    pub description: &'static str,
    pub budget: f64,
    pub unit: &'static str,
    pub measured: Option<f64>,
    pub from_metric: String,
}

impl TargetResult {
    pub fn met(&self) -> Option<bool> {
        self.measured.map(|m| m <= self.budget)
    }
    /// How far away we are, as a multiple of the budget. 1.0 is exactly on it.
    pub fn ratio(&self) -> Option<f64> {
        self.measured.map(|m| m / self.budget)
    }
}

pub fn evaluate_targets(cases: &[CaseReport]) -> Vec<TargetResult> {
    TARGETS
        .iter()
        .map(|t| {
            let case = cases.iter().find(|c| c.id == t.case);
            let (measured, from) = match (&t.metric, case) {
                (TargetMetric::Named(m), Some(c)) => (c.metrics.get(*m).map(|s| s.median), (*m).to_string()),
                (TargetMetric::WorstWarm, Some(c)) => {
                    let worst = c
                        .metrics
                        .iter()
                        .filter(|(k, _)| k.starts_with("warm."))
                        .max_by(|a, b| a.1.median.partial_cmp(&b.1.median).expect("finite"));
                    (worst.map(|(_, s)| s.median), worst.map_or("-".into(), |(k, _)| format!("{k} (slowest)")))
                }
                _ => (None, "case not measured".to_string()),
            };
            TargetResult { id: t.id, description: t.description, budget: t.budget, unit: t.unit, measured, from_metric: from }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// The report document
// ---------------------------------------------------------------------------

pub struct Report {
    pub meta: Vec<(String, Value)>,
    pub cases: Vec<CaseReport>,
    /// Cases that produced no timing, with the reason. Recorded in the report
    /// so a shrinking corpus is visible instead of silent.
    pub unmeasured: Vec<(String, String)>,
}

impl Report {
    pub fn to_json(&self) -> Value {
        let mut root = Value::obj();
        root.set("schema_version", json::num(SCHEMA_VERSION));
        let mut meta = Value::obj();
        for (k, v) in &self.meta {
            meta.set(k, v.clone());
        }
        root.set("meta", meta);
        let cases: Vec<Value> = self
            .cases
            .iter()
            .map(|c| {
                let mut o = Value::obj();
                o.set("id", json::str_(c.id.as_str()));
                o.set("group", json::str_(c.group.as_str()));
                o.set("bytes", json::num(c.bytes as f64));
                o.set("documents", json::num(c.documents as f64));
                o.set("pages", json::num(c.pages as f64));
                o.set("passes", json::num(c.passes as f64));
                o.set("phase_coverage", json::num(round6(c.coverage)));
                o.set("cache_hit_rate", json::num(round6(c.cache_hit_rate)));
                o.set("load1_max", json::num(round6(c.load_max)));
                let mut m = Value::obj();
                for (k, s) in &c.metrics {
                    m.set(k, s.to_json());
                }
                o.set("metrics", m);
                let mut d = Value::obj();
                for (k, v) in &c.digests {
                    d.set(k, json::str_(v.as_str()));
                }
                o.set("digests", d);
                o.set("notes", Value::Arr(c.notes.iter().map(|n| json::str_(n.as_str())).collect()));
                o
            })
            .collect();
        root.set("cases", Value::Arr(cases));
        let unmeasured: Vec<Value> = self
            .unmeasured
            .iter()
            .map(|(id, why)| {
                let mut o = Value::obj();
                o.set("id", json::str_(id.as_str()));
                o.set("reason", json::str_(why.as_str()));
                o
            })
            .collect();
        root.set("unmeasured", Value::Arr(unmeasured));
        let targets: Vec<Value> = evaluate_targets(&self.cases)
            .iter()
            .map(|t| {
                let mut o = Value::obj();
                o.set("id", json::str_(t.id));
                o.set("description", json::str_(t.description));
                o.set("budget", json::num(t.budget));
                o.set("unit", json::str_(t.unit));
                o.set("from_metric", json::str_(t.from_metric.as_str()));
                o.set("measured", t.measured.map_or(Value::Null, |m| json::num(round6(m))));
                o.set("met", t.met().map_or(Value::Null, Value::Bool));
                o.set("ratio_to_budget", t.ratio().map_or(Value::Null, |r| json::num(round6(r))));
                o
            })
            .collect();
        root.set("targets", Value::Arr(targets));
        root
    }

    pub fn from_json(v: &Value) -> Option<Report> {
        let meta = match v.get("meta") {
            Some(Value::Obj(m)) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            _ => Vec::new(),
        };
        let cases = v
            .get("cases")?
            .as_arr()?
            .iter()
            .map(|c| {
                let num = |k: &str| match c.get(k) {
                    Some(Value::Num(n)) => *n,
                    _ => 0.0,
                };
                let metrics = match c.get("metrics") {
                    Some(Value::Obj(m)) => m.iter().filter_map(|(k, v)| Stats::from_json(v).map(|s| (k.clone(), s))).collect(),
                    _ => BTreeMap::new(),
                };
                let digests = match c.get("digests") {
                    Some(Value::Obj(m)) => m.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect(),
                    _ => BTreeMap::new(),
                };
                CaseReport {
                    id: c.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
                    group: c.get("group").and_then(Value::as_str).unwrap_or("").to_string(),
                    bytes: num("bytes") as usize,
                    documents: num("documents") as usize,
                    pages: num("pages") as usize,
                    passes: num("passes") as u32,
                    coverage: num("phase_coverage"),
                    cache_hit_rate: num("cache_hit_rate"),
                    load_max: num("load1_max"),
                    metrics,
                    digests,
                    notes: c
                        .get("notes")
                        .and_then(Value::as_arr)
                        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                        .unwrap_or_default(),
                }
            })
            .collect();
        let unmeasured = v
            .get("unmeasured")
            .and_then(Value::as_arr)
            .map(|a| {
                a.iter()
                    .filter_map(|o| Some((o.get("id")?.as_str()?.to_string(), o.get("reason").and_then(Value::as_str).unwrap_or("").to_string())))
                    .collect()
            })
            .unwrap_or_default();
        Some(Report { meta, cases, unmeasured })
    }
}

// ---------------------------------------------------------------------------
// Human table
// ---------------------------------------------------------------------------

fn f(x: f64, w: usize, p: usize) -> String {
    if x.is_finite() {
        format!("{x:>w$.p$}")
    } else {
        format!("{:>w$}", "-")
    }
}

pub fn human_table(report: &Report) -> String {
    let mut out = String::new();
    out.push_str("== run metadata ==\n");
    for (k, v) in &report.meta {
        out.push_str(&format!("  {k:<18} {}\n", flatten(v)));
    }

    out.push_str("\n== headline (median over repetitions; warm = slowest scenario) ==\n");
    out.push_str(&format!(
        "{:<20} {:>9} {:>5} {:>3} {:>10} {:>10} {:>10} {:>9} {:>10} {:>9} {:>8} {:>7} {:>6} {:>5}\n",
        "case", "bytes", "pg", "n", "first ms", "cold ms", "nocache", "export ms", "warm p50", "warm p95", "peakRSS", "steady", "±IQR", "load"
    ));
    for c in &report.cases {
        let worst = worst_warm(c);
        let g = |k: &str| c.metrics.get(k).map_or(f64::NAN, |s| s.median);
        out.push_str(&format!(
            "{:<20} {:>9} {:>5} {:>3} {} {} {} {} {} {} {} {} {} {}\n",
            c.id,
            c.bytes,
            c.pages,
            c.metrics.get("cold.render_ms").map_or(0, |s| s.n),
            f(g("cold.first_render_ms"), 10, 2),
            f(g("cold.render_ms"), 10, 2),
            f(g("cold.render_nocache_ms"), 10, 2),
            f(g("export.pdf_ms"), 9, 2),
            f(worst.map_or(f64::NAN, |s| s.median), 10, 3),
            f(worst.map_or(f64::NAN, |s| s.p95), 9, 3),
            f(g("mem.peak_rss_kb") / 1024.0, 7, 0),
            f(g("mem.steady_rss_kb") / 1024.0, 6, 0),
            f(c.metrics.get("cold.render_ms").map_or(f64::NAN, |s| s.rel_iqr) * 100.0, 5, 1),
            f(c.load_max, 5, 1),
        ));
    }
    out.push_str("  n = repetitions behind the cold columns (capped by size; every warm row carries its own n below).
  first ms = first render in a fresh process (cold caches AND cold font faces); cold ms = empty RenderCache, faces loaded.\n");
    out.push_str("  nocache = the same render with no RenderCache attached: the gap is what populating the reuse cache costs on the first compile.
  export ms includes the one-time font-program read and subset (export.pdf_warm_ms in the JSON is a second export).\n");
    out.push_str("  peakRSS/steady in MiB. ±IQR is (p75-p25)/median of cold.render_ms, in percent: the run's own noise estimate for that case.
  load is the highest 1-minute load average seen around that case. Rows measured at very different loads are not comparable with each other.\n");

    out.push_str("\n== cold phase split, median ms (decomposition at the public seams) ==\n");
    let w = |n: &str| n.len().max(9);
    out.push_str(&format!("{:<20}", "case"));
    for n in Phases::NAMES {
        out.push_str(&format!(" {n:>width$}", width = w(n)));
    }
    out.push_str(&format!(" {:>7}\n", "cover"));
    for c in &report.cases {
        out.push_str(&format!("{:<20}", c.id));
        for n in Phases::NAMES {
            out.push_str(&format!(" {}", f(c.metrics.get(&format!("cold.phase.{n}_ms")).map_or(f64::NAN, |s| s.median), w(n), 2)));
        }
        out.push_str(&format!(" {}\n", f(c.coverage, 7, 2)));
    }
    out.push_str("  parse = parser::parse_project (lex + expand + parse). expand is that same expansion measured alone, so it overlaps parse and is NOT summed.\n");
    out.push_str("  typeset_notexts is a control: the same typesetting with Context::new (no document sources), which is what examples/stages.rs times. Not summed.
  json_v1 and json_v2 are alternatives; only one is on any request path, so neither is summed either.\n  cover = sum(parse..v1) / cold.render_ms. Below 0.85: the product path does work the decomposition does not replay (float scan, TikZ scan, extra label passes).\n");

    out.push_str("\n== warm keystroke, ms per edit (pooled over every step of every repetition) ==\n");
    out.push_str(&format!("{:<20} {:<22} {:>5} {:>6} {:>9} {:>9} {:>9} {:>9}  {}\n", "case", "scenario", "caps", "n", "p50", "p75", "p95", "max", "digest"));
    for c in &report.cases {
        for (k, s) in c.metrics.iter().filter(|(k, _)| k.starts_with("warm.")) {
            let rest = k.trim_start_matches("warm.").trim_end_matches("_ms");
            let (name, caps) = rest.rsplit_once('.').unwrap_or((rest, ""));
            out.push_str(&format!(
                "{:<20} {:<22} {:>5} {:>6} {} {} {} {}  {}\n",
                c.id,
                name,
                caps,
                s.n,
                f(s.median, 9, 3),
                f(s.p75, 9, 3),
                f(s.p95, 9, 3),
                f(s.max, 9, 3),
                c.digests.get(&format!("warm.{name}.{caps}")).map_or("-", String::as_str)
            ));
        }
    }

    out.push_str("\n== targets ==\n");
    out.push_str(&format!("{:<12} {:<44} {:>11} {:>11} {:>8}  {:<6} {}\n", "id", "target", "budget", "measured", "x budget", "met", "from"));
    for t in evaluate_targets(&report.cases) {
        out.push_str(&format!(
            "{:<12} {:<44} {:>11} {:>11} {:>8}  {:<6} {}\n",
            t.id,
            t.description,
            format!("{:.1} {}", t.budget, t.unit),
            t.measured.map_or("-".into(), |m| format!("{m:.3}")),
            t.ratio().map_or("-".into(), |r| format!("{r:.1}x")),
            match t.met() {
                Some(true) => "YES",
                Some(false) => "no",
                None => "-",
            },
            t.from_metric
        ));
    }

    if !report.unmeasured.is_empty() {
        out.push_str("\n== not measured (no valid measurement was possible; no timing is reported) ==\n");
        for (id, why) in &report.unmeasured {
            out.push_str(&format!("  {id}: {why}\n"));
        }
    }

    let notes: Vec<String> = report.cases.iter().flat_map(|c| c.notes.iter().map(move |n| format!("{}: {n}", c.id))).collect();
    if !notes.is_empty() {
        out.push_str("\n== notes ==\n");
        for n in notes {
            out.push_str(&format!("  {n}\n"));
        }
    }
    out
}

fn worst_warm(c: &CaseReport) -> Option<&Stats> {
    c.metrics
        .iter()
        .filter(|(k, _)| k.starts_with("warm."))
        .max_by(|a, b| a.1.median.partial_cmp(&b.1.median).expect("finite"))
        .map(|(_, s)| s)
}

fn flatten(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Num(n) => format!("{n}"),
        Value::Bool(b) => format!("{b}"),
        Value::Null => "-".into(),
        other => json::write(other),
    }
}

// ---------------------------------------------------------------------------
// Regression gate
// ---------------------------------------------------------------------------

pub struct Regression {
    pub case: String,
    pub metric: String,
    pub baseline: f64,
    pub now: f64,
    pub pct: f64,
    pub noise_band_pct: f64,
}

pub struct GateOutcome {
    pub regressions: Vec<Regression>,
    pub improvements: Vec<Regression>,
    pub digest_mismatches: Vec<String>,
    pub warnings: Vec<String>,
    pub compared: usize,
    /// True when the baseline was recorded on a different host, build profile
    /// or font configuration.
    pub host_differs: bool,
}

/// Metrics below this are dominated by timer resolution and scheduling, not by
/// the engine; a 5 % move on 30 µs is not a signal.
const MIN_ABS_MS: f64 = 0.05;
const MIN_ABS_KB: f64 = 2048.0;

/// Compares a run against a committed baseline.
///
/// A metric counts as regressed only when it exceeds **both** bars: the
/// percentage tolerance, and the baseline's own measured spread for that
/// metric. A harness that fires on anything above 5 % on a box with other
/// build lanes on it would be turned off within a day, which is worse than no
/// gate at all.
pub fn gate(baseline: &Report, now: &Report, tolerance_pct: f64) -> GateOutcome {
    let mut o = GateOutcome { regressions: Vec::new(), improvements: Vec::new(), digest_mismatches: Vec::new(), warnings: Vec::new(), compared: 0, host_differs: false };

    let meta = |r: &Report, k: &str| r.meta.iter().find(|(n, _)| n == k).map(|(_, v)| flatten(v)).unwrap_or_default();
    for k in ["host_fingerprint", "build_profile", "build_opt_level", "caps", "font_config"] {
        let (a, b) = (meta(baseline, k), meta(now, k));
        if a != b && !a.is_empty() {
            o.host_differs = true;
            o.warnings.push(format!("{k} differs from the baseline ({a} -> {b}); the comparison is between two different configurations"));
        }
    }
    let num = |r: &Report, k: &str| meta(r, k).parse::<f64>().unwrap_or(f64::NAN);
    let (ca, cb) = (num(baseline, "calibration_ns"), num(now, "calibration_ns"));
    if ca.is_finite() && cb.is_finite() && ca > 0.0 {
        let drift = (cb - ca) / ca * 100.0;
        if drift.abs() > 10.0 {
            o.warnings.push(format!("calibration loop is {drift:+.1}% away from the baseline's: the host is in a different state, treat every delta with suspicion"));
        }
    }
    // Measured on the reference machine: the same case moved by ~70 % between
    // a run at load 2 and a run at load 6, while its own within-run spread
    // stayed under 3 %. Memory-bandwidth contention from the other build lanes
    // does not show up in the single-threaded calibration loop, so the load
    // average has to be compared on its own. A 5 % tolerance means nothing
    // across a gap this size, and the gate says so rather than pretending.
    let (la, lb) = (num(baseline, "load1_max"), num(now, "load1_max"));
    if la.is_finite() && lb.is_finite() && (lb > la * 2.0 + 1.0 || la > lb * 2.0 + 1.0) {
        o.host_differs = true;
        o.warnings.push(format!(
            "peak 1-minute load was {la:.1} for the baseline and {lb:.1} for this run; on a shared machine that difference is larger than the tolerance, so timing deltas are not evidence. Re-record on a quiet machine, or pin with --pin."
        ));
    }

    for b in &baseline.cases {
        let Some(n) = now.cases.iter().find(|c| c.id == b.id) else {
            o.warnings.push(format!("{}: in the baseline but not in this run", b.id));
            continue;
        };
        for (k, want) in &b.digests {
            match n.digests.get(k) {
                Some(got) if got == want => {}
                Some(got) => o.digest_mismatches.push(format!("{}/{k}: output changed ({want} -> {got})", b.id)),
                None => o.warnings.push(format!("{}/{k}: digest missing from this run", b.id)),
            }
        }
        for (k, bs) in &b.metrics {
            let Some(ns) = n.metrics.get(k) else {
                o.warnings.push(format!("{}/{k}: metric missing from this run", b.id));
                continue;
            };
            if !bs.median.is_finite() || !ns.median.is_finite() || bs.median <= 0.0 {
                continue;
            }
            o.compared += 1;
            let pct = (ns.median - bs.median) / bs.median * 100.0;
            let floor = if k.ends_with("_kb") { MIN_ABS_KB } else { MIN_ABS_MS };
            // The baseline's own p25..p75 spread, as a percentage, is the
            // smallest move this metric can distinguish from noise.
            let noise = (bs.rel_iqr * 100.0).max(0.0);
            let r = Regression { case: b.id.clone(), metric: k.clone(), baseline: bs.median, now: ns.median, pct, noise_band_pct: noise };
            let moved_enough = (ns.median - bs.median).abs() >= floor;
            if pct > tolerance_pct && pct > noise && moved_enough {
                o.regressions.push(r);
            } else if pct < -tolerance_pct && -pct > noise && moved_enough {
                o.improvements.push(r);
            }
        }
    }
    o.regressions.sort_by(|a, b| b.pct.partial_cmp(&a.pct).expect("finite"));
    o.improvements.sort_by(|a, b| a.pct.partial_cmp(&b.pct).expect("finite"));
    o
}

/// Whether the run should fail.
///
/// A changed output digest always fails: the fonts come from the repository,
/// so byte identity does not depend on the machine, and a faster run that
/// changed a byte is a failed run. Timing regressions fail too, unless
/// `require_same_host` is set and the baseline was recorded somewhere else —
/// on a different machine a wall-clock delta is not evidence about the code.
pub fn gate_failed(o: &GateOutcome, require_same_host: bool) -> bool {
    if !o.digest_mismatches.is_empty() {
        return true;
    }
    !o.regressions.is_empty() && !(require_same_host && o.host_differs)
}

pub fn gate_table(o: &GateOutcome, tolerance_pct: f64) -> String {
    let mut out = String::new();
    let row = |out: &mut String, r: &Regression| {
        out.push_str(&format!(
            "  {:<20} {:<34} {:>10.3} -> {:>10.3}  {:>+7.1}%  (noise band {:.1}%)\n",
            r.case, r.metric, r.baseline, r.now, r.pct, r.noise_band_pct
        ));
    };
    out.push_str(&format!("== regression gate (tolerance {tolerance_pct:.1}%, {} metrics compared) ==\n", o.compared));
    if !o.digest_mismatches.is_empty() {
        out.push_str("  OUTPUT CHANGED — byte-identical output is a hard gate, so this fails regardless of timings:\n");
        for m in &o.digest_mismatches {
            out.push_str(&format!("    {m}\n"));
        }
    }
    if o.regressions.is_empty() {
        out.push_str("  no regression above tolerance and above each metric's own noise band\n");
    } else {
        out.push_str("  REGRESSIONS:\n");
        for r in &o.regressions {
            row(&mut out, r);
        }
    }
    if !o.improvements.is_empty() {
        out.push_str("  improvements:\n");
        for r in &o.improvements {
            row(&mut out, r);
        }
    }
    for w in &o.warnings {
        out.push_str(&format!("  warning: {w}\n"));
    }
    out
}

/// A short line for a CI log or a PR comment.
pub fn gate_summary(o: &GateOutcome, require_same_host: bool) -> String {
    if !o.digest_mismatches.is_empty() {
        return format!("FAIL: output changed in {} place(s)", o.digest_mismatches.len());
    }
    if o.regressions.is_empty() {
        return format!("OK: {} metrics compared, no regression", o.compared);
    }
    if require_same_host && o.host_differs {
        return format!(
            "{} timing regression(s) reported but not enforced: the baseline was recorded on a different host/profile",
            o.regressions.len()
        );
    }
    format!("FAIL: {} timing regression(s)", o.regressions.len())
}
