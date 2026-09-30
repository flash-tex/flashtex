//! Percentiles, the summary against DESIGN.md §1.2, and the gate verdict.

use crate::machine::num;
use flashtex_display_list::json::{obj, s, Json};

/// A metric the gate checks, with its §1.2 target.
pub struct Metric {
    pub name: &'static str,
    pub target_ms: f64,
    pub what: &'static str,
}

pub const METRICS: [Metric; 4] = [
    Metric {
        name: "local",
        target_ms: 16.0,
        what: "edit inside a paragraph: COMPILE to the edited page's frame",
    },
    Metric {
        name: "reflow",
        target_ms: 16.0,
        what: "edit that reflows later pages: COMPILE to the edited page's frame",
    },
    Metric {
        name: "preamble",
        target_ms: 400.0,
        what: "preamble edit: COMPILE to the first visible page",
    },
    Metric {
        name: "reopen",
        target_ms: 100.0,
        what: "reopen from the persisted S0: COMPILE to the first visible page",
    },
];

/// Nearest-rank percentile (`p` in 0..=100) of sorted values.
pub fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted[rank.min(sorted.len()) - 1]
}

pub fn f64_of(j: &Json) -> Option<f64> {
    match j {
        Json::Int(i) => Some(*i as f64),
        Json::Num(f) => Some(*f),
        _ => None,
    }
}

/// One summary row: a document, a metric, its samples.
pub fn row(doc: &str, pages: u32, m: &Metric, samples: &[&Json]) -> Json {
    let mut v: Vec<f64> = samples
        .iter()
        .filter_map(|j| j.get("watched_ms").and_then(f64_of))
        .collect();
    v.sort_by(f64::total_cmp);
    let misses = samples.len() - v.len();
    let mut host: Vec<f64> = samples
        .iter()
        .filter_map(|j| j.get("host_first_page_ms").and_then(f64_of))
        .collect();
    host.sort_by(f64::total_cmp);
    let mut done: Vec<f64> = samples
        .iter()
        .filter_map(|j| j.get("done_ms").and_then(f64_of))
        .collect();
    done.sort_by(f64::total_cmp);
    let mut load: Vec<f64> = samples
        .iter()
        .filter_map(|j| j.get("load1").and_then(f64_of))
        .collect();
    load.sort_by(f64::total_cmp);
    let busy_max = samples
        .iter()
        .filter_map(|j| j.int_field("runners_busy"))
        .max()
        .unwrap_or(0);
    let p95 = pct(&v, 95.0);
    let status = if v.is_empty() || misses > 0 {
        "missing frames"
    } else if p95 <= m.target_ms {
        "met"
    } else {
        "not met"
    };
    obj([
        ("doc", s(doc)),
        ("pages", Json::Int(pages as i64)),
        ("metric", s(m.name)),
        ("target_ms", num(m.target_ms)),
        ("n", Json::Int(samples.len() as i64)),
        ("misses", Json::Int(misses as i64)),
        ("p50_ms", num(pct(&v, 50.0))),
        ("p95_ms", num(p95)),
        ("max_ms", num(v.last().copied().unwrap_or(f64::NAN))),
        ("host_first_page_p95_ms", num(pct(&host, 95.0))),
        ("done_p50_ms", num(pct(&done, 50.0))),
        ("load1_min", num(load.first().copied().unwrap_or(f64::NAN))),
        ("load1_max", num(load.last().copied().unwrap_or(f64::NAN))),
        ("runners_busy_max", Json::Int(busy_max)),
        ("status", s(status)),
    ])
}

fn cell(j: &Json, k: &str) -> String {
    match j.get(k) {
        Some(Json::Null) | None => "–".into(),
        Some(v) => match f64_of(v) {
            Some(x) => format!("{x:.1}"),
            None => v.as_str().unwrap_or("").to_string(),
        },
    }
}

/// The summary's rows as a Markdown table.
pub fn markdown(rows: &[Json]) -> String {
    let mut out = String::from(
        "| doc | pages | metric | target | n | p50 ms | p95 ms | max ms | host p95 | load1 | busy runners | status |\n\
         |---|---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for r in rows {
        out.push_str(&format!(
            "| {} | {} | {} | ≤ {} | {}{} | {} | {} | {} | {} | {}–{} | {} | {} |\n",
            cell(r, "doc"),
            r.int_field("pages").unwrap_or(0),
            cell(r, "metric"),
            cell(r, "target_ms"),
            r.int_field("n").unwrap_or(0),
            match r.int_field("misses").unwrap_or(0) {
                0 => String::new(),
                m => format!(" ({m} missed)"),
            },
            cell(r, "p50_ms"),
            cell(r, "p95_ms"),
            cell(r, "max_ms"),
            cell(r, "host_first_page_p95_ms"),
            cell(r, "load1_min"),
            cell(r, "load1_max"),
            r.int_field("runners_busy_max").unwrap_or(0),
            cell(r, "status"),
        ));
    }
    out
}

/// The gate: a row fails when its p95 exceeds the target by more than the
/// noise margin, when a frame never arrived, or with too few samples.
/// Returns the failures, one line each.
pub fn verdict(rows: &[Json], margin: f64, min_n: i64) -> Vec<String> {
    let mut fails = Vec::new();
    for r in rows {
        let name = format!(
            "{} {}",
            r.str_field("doc").unwrap_or("?"),
            r.str_field("metric").unwrap_or("?")
        );
        let target = r.get("target_ms").and_then(f64_of).unwrap_or(f64::NAN);
        let p95 = r.get("p95_ms").and_then(f64_of);
        let n = r.int_field("n").unwrap_or(0);
        let misses = r.int_field("misses").unwrap_or(0);
        if n < min_n {
            fails.push(format!("{name}: {n} samples, fewer than {min_n}"));
        } else if misses > 0 {
            fails.push(format!(
                "{name}: {misses} of {n} compiles never showed the page"
            ));
        } else if let Some(p) = p95 {
            let limit = target * (1.0 + margin);
            if p > limit {
                fails.push(format!(
                    "{name}: p95 {p:.1} ms > {limit:.1} ms (target {target} ms + {:.0}% noise margin)",
                    margin * 100.0
                ));
            }
        } else {
            fails.push(format!("{name}: no p95"));
        }
    }
    fails
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank() {
        let v: Vec<f64> = (1..=20).map(f64::from).collect();
        assert_eq!(pct(&v, 50.0), 10.0);
        assert_eq!(pct(&v, 95.0), 19.0);
        assert_eq!(pct(&v, 100.0), 20.0);
        assert_eq!(pct(&[3.0], 95.0), 3.0);
        assert!(pct(&[], 95.0).is_nan());
    }

    fn fake(p95: f64, n: i64, misses: i64) -> Json {
        obj([
            ("doc", s("plain-10")),
            ("metric", s("local")),
            ("target_ms", num(16.0)),
            ("n", Json::Int(n)),
            ("misses", Json::Int(misses)),
            ("p95_ms", num(p95)),
        ])
    }

    #[test]
    fn verdict_margin() {
        // Within the target, within the margin, beyond it.
        assert!(verdict(&[fake(15.0, 36, 0)], 0.10, 6).is_empty());
        assert!(verdict(&[fake(17.5, 36, 0)], 0.10, 6).is_empty());
        assert_eq!(verdict(&[fake(17.7, 36, 0)], 0.10, 6).len(), 1);
        // A page that never came, and too few samples, always fail.
        assert_eq!(verdict(&[fake(1.0, 36, 1)], 0.10, 6).len(), 1);
        assert_eq!(verdict(&[fake(1.0, 3, 0)], 0.10, 6).len(), 1);
    }
}
