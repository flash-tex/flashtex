//! plan4 measurement harness: run ten real TeX Live `.sty` files through
//! the expansion engine via a `PackageReader` and report, per package,
//! which control sequences the engine does not model.
//!
//! Usage (from the repository root):
//! ```sh
//! cargo run -p flashtex-tex-expansion --example real_sty_harness [--out PATH] [pkg...]
//! ```
//! With no `--out`, the table goes to
//! `docs/evidence/plan4-real-sty-harness-<UTC-date>.md`. With no package
//! arguments all ten packages are measured. Exit status is 0 when the
//! table is written (per-package load errors are data, not failures);
//! nonzero only when the harness itself is blocked (no TeX Live found) or
//! the output cannot be written.
//!
//! Method (see also `tests/real_sty_probe_tests.rs`, which pins the engine
//! assumptions this relies on):
//! - The engine reports a genuinely unknown command with NO diagnostic at
//!   all: it passes the control sequence through to the output untouched
//!   (`Meaning::Undefined => Step::Emit`). So "unmodelled primitives hit"
//!   cannot be read off diagnostics; it must be read off the output.
//! - Run 1 loads `\documentclass{article}\usepackage{<pkg>}` with a reader
//!   that serves ONLY `<pkg>.sty` (anything the file nests inside via
//!   `\RequirePackage` is declined and passes through; every name the
//!   reader is asked for is logged). Distinct control sequences emitted
//!   with the package file's source id are the candidates.
//! - Run 2 repeats the load with a probe tail appended: one
//!   `\ifcsname <name>\endcsname\else <marker>\fi` per candidate (a space
//!   separates `\ifcsname` from the name; the `\csname` form needs no
//!   catcodes, so `@`-names work without `\makeatletter`), plus a plain
//!   `HARNESS-PROBE-DONE` sentinel. A candidate is unmodelled iff its
//!   marker is emitted, i.e. it is still undefined after the whole file
//!   loaded -- this correctly excludes engine-known tokens the run emits
//!   by design (`\par`, `\begingroup`, `\relax`, declined
//!   `\usepackage`s) and anything the package defined itself.
//! - Four controls (`\par`, `\relax`, `\usepackage`, `\@empty`) are probed
//!   in every tail and must come back defined, and the sentinel must be
//!   present, or the package's row is reported `inconclusive` instead of a
//!   count.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use flashtex_tex_expansion::{tokens_to_display_string, Engine, Severity, TokenKind};

/// The ten state-only packages plan4 considers running as real `.sty`.
const PACKAGES: &[&str] = &[
    "parskip", "setspace", "xspace", "relsize", "appendix", "titling", "enumerate", "calc", "ifthen",
    "etoolbox",
];

const BEGIN_MARK: &str = "HARNESS-UNMODELLED-BEGIN";
const END_MARK: &str = "HARNESS-UNMODELLED-END";
const DONE_MARK: &str = "HARNESS-PROBE-DONE";
/// Probed in every tail; engine-known in all runs, so a flag means the
/// probe itself broke and the row is `inconclusive`.
const CONTROLS: &[&str] = &["par", "relax", "usepackage", "@empty"];

struct LoadResult {
    ms: f64,
    tokens: Vec<flashtex_tex_expansion::Token>,
    candidates: BTreeSet<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
    limit_hit: bool,
    /// Every `(name, ext)` the reader was asked for, in order.
    queries: Vec<String>,
}

fn find_kpsewhich() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let cand = dir.join("kpsewhich");
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    let fixed = PathBuf::from("/Library/TeX/texbin/kpsewhich");
    if fixed.is_file() {
        return Some(fixed);
    }
    // /usr/local/texlive/*/bin/*/kpsewhich, expanded by hand (two levels).
    if let Ok(top) = std::fs::read_dir("/usr/local/texlive") {
        let mut versions: Vec<PathBuf> = top.filter_map(|e| e.ok().map(|e| e.path())).collect();
        versions.sort();
        for v in versions {
            if let Ok(bins) = std::fs::read_dir(v.join("bin")) {
                let mut archs: Vec<PathBuf> = bins.filter_map(|e| e.ok().map(|e| e.path())).collect();
                archs.sort();
                for a in archs {
                    let cand = a.join("kpsewhich");
                    if cand.is_file() {
                        return Some(cand);
                    }
                }
            }
        }
    }
    None
}

fn resolve_sty(kpsewhich: &PathBuf, pkg: &str) -> Option<PathBuf> {
    let out = Command::new(kpsewhich).arg(format!("{pkg}.sty")).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() { None } else { Some(PathBuf::from(path)) }
}

/// One full engine run of the minimal document; `tail` is appended for the
/// probe run. `timed` selects whether wall-clock time is recorded.
fn run_load(sty_name: &str, sty_text: &str, tail: &str, timed: bool) -> LoadResult {
    let src = format!("\\documentclass{{article}}\n\\usepackage{{{sty_name}}}\n{tail}");
    let queries = Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let mut engine = Engine::new(&src);
    {
        let text = sty_text.to_string();
        let name = sty_name.to_string();
        let queries = queries.clone();
        engine.set_package_reader(Rc::new(move |pkg, ext| {
            queries.borrow_mut().push(format!("{pkg}.{ext}"));
            if format!("{pkg}.{ext}") == format!("{name}.sty") { Some(text.clone()) } else { None }
        }));
    }
    let t0 = Instant::now();
    let tokens = engine.run();
    let ms = if timed { t0.elapsed().as_secs_f64() * 1000.0 } else { 0.0 };
    let sids: BTreeSet<u32> = engine.opened_package_files().iter().map(|f| f.source_id).collect();
    let mut candidates = BTreeSet::new();
    for t in &tokens {
        if let TokenKind::ControlSequence(name) = &t.kind {
            if sids.contains(&t.span.source_id) {
                candidates.insert(name.clone());
            }
        }
    }
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    for d in engine.take_diagnostics() {
        let line = d.message.replace('\n', " / ");
        if d.severity == Severity::Error {
            errors.push(line);
        } else {
            warnings.push(line);
        }
    }
    let limit_hit = errors.iter().any(|m| m.contains("limit exceeded"));
    let queries = queries.borrow().clone();
    LoadResult { ms, tokens, candidates, errors, warnings, limit_hit, queries }
}

/// Classify `candidates` with a second run carrying the probe tail.
/// Returns `None` when the probe is inconclusive (sentinel missing or a
/// control flagged).
fn probe_unmodelled(sty_name: &str, sty_text: &str, candidates: &BTreeSet<String>) -> Option<BTreeSet<String>> {
    let mut probed: BTreeSet<String> = candidates.clone();
    for c in CONTROLS {
        probed.insert(c.to_string());
    }
    let mut tail = String::from("\\relax ");
    for name in &probed {
        tail.push_str(&format!("\\ifcsname {name}\\endcsname\\else {BEGIN_MARK}{name}{END_MARK}\\fi "));
    }
    tail.push_str(DONE_MARK);
    // The probe tail is TeX, not data: it must not disturb the load it
    // measures, so it runs in its own engine run and only reads state.
    let display = tokens_to_display_string(&run_load(sty_name, sty_text, &tail, false).tokens);
    if !display.contains(DONE_MARK) {
        return None;
    }
    let mut flagged = BTreeSet::new();
    for part in display.split(BEGIN_MARK).skip(1) {
        if let Some(end) = part.find(END_MARK) {
            flagged.insert(part[..end].to_string());
        }
    }
    if CONTROLS.iter().any(|c| flagged.contains(*c)) {
        return None;
    }
    for c in CONTROLS {
        flagged.remove(*c);
    }
    Some(flagged)
}

struct Row {
    name: String,
    path: Option<String>,
    completed: bool,
    unmodelled: Option<BTreeSet<String>>,
    ms: Option<f64>,
    declined: Vec<String>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

/// Days since the Unix epoch to a civil (year, month, day) date
/// (Howard Hinnant's algorithm) for the default evidence filename.
fn utc_today() -> String {
    let days = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() / 86400).unwrap_or(0) as i64;
    civil_date(days)
}

fn civil_date(days: i64) -> String {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

fn md_escape_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out_path: Option<String> = None;
    let mut only: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--out" && i + 1 < args.len() {
            out_path = Some(args[i + 1].clone());
            i += 2;
        } else {
            only.push(args[i].clone());
            i += 1;
        }
    }
    let pkgs: Vec<&str> =
        PACKAGES.iter().copied().filter(|p| only.is_empty() || only.iter().any(|o| o == p)).collect();

    let kpsewhich = find_kpsewhich().unwrap_or_else(|| {
        eprintln!("BLOCKED: no TeX Live reachable: no `kpsewhich` on $PATH, at /Library/TeX/texbin/kpsewhich, or under /usr/local/texlive/*/bin/*.");
        std::process::exit(2);
    });
    eprintln!("kpsewhich: {}", kpsewhich.display());

    // Warm up so the first package does not pay one-time engine setup.
    Engine::new("warmup").run();

    let mut rows: Vec<Row> = Vec::new();
    for pkg in pkgs {
        let path = resolve_sty(&kpsewhich, pkg);
        let Some(path) = path else {
            rows.push(Row {
                name: pkg.to_string(),
                path: None,
                completed: false,
                unmodelled: None,
                ms: None,
                declined: Vec::new(),
                errors: Vec::new(),
                warnings: Vec::new(),
            });
            continue;
        };
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("BLOCKED: cannot read {}: {e}", path.display());
            std::process::exit(2);
        });
        let load = run_load(pkg, &text, "", true);
        let unmodelled = probe_unmodelled(pkg, &text, &load.candidates);
        let declined: Vec<String> = {
            let mut seen = BTreeSet::new();
            for q in &load.queries {
                if q != &format!("{pkg}.sty") && q != "article.cls" {
                    seen.insert(q.clone());
                }
            }
            seen.into_iter().collect()
        };
        let completed = load.errors.is_empty() && !load.limit_hit && unmodelled.is_some();
        rows.push(Row {
            name: pkg.to_string(),
            path: Some(path.display().to_string()),
            completed,
            unmodelled,
            ms: Some(load.ms),
            declined,
            errors: load.errors,
            warnings: load.warnings,
        });
    }

    // Least new primitive support first; packages without a file last.
    rows.sort_by(|a, b| match (&a.unmodelled, &b.unmodelled) {
        (Some(x), Some(y)) => x.len().cmp(&y.len()).then(a.name.cmp(&b.name)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.name.cmp(&b.name),
    });

    let date = utc_today();
    let mut md = format!(
        "# plan4 real-.sty harness — {date} (UTC)\n\n\
        Minimal document `\\documentclass{{article}}\\usepackage{{<pkg>}}` with a package reader \
        serving ONLY that package's real TeX Live `.sty` file; nested `\\RequirePackage`s are \
        declined (pass through) and listed below. Signal for \"unmodelled\": the engine emits an \
        unknown command with no diagnostic, so run 1 collects control sequences the package file's \
        execution emits and run 2 classifies each with an `\\ifcsname` probe tail (controls \
        `\\par`, `\\relax`, `\\usepackage`, `\\@empty` must stay unflagged and a `HARNESS-PROBE-DONE` \
        sentinel must be present, else the row is `inconclusive`). Times are one debug-build run \
        each (`std::time::Instant` around `Engine::run`). Rows with errors may mix root gaps with \
        cascade fallout once argument parsing derails (see per-package diagnostics). Method pinned by \
        `crates/tex-expansion/tests/real_sty_probe_tests.rs`.\n\n\
        | Package | Completed | Unmodelled (n) | Unmodelled names | Time (ms) | Nested deps requested (declined) |\n\
        | --- | --- | --- | --- | --- | --- |\n"
    );
    for r in &rows {
        let n = r.unmodelled.as_ref().map(|u| u.len().to_string()).unwrap_or_else(|| "n/a".to_string());
        let names = r
            .unmodelled
            .as_ref()
            .map(|u| {
                if u.is_empty() {
                    "—".to_string()
                } else {
                    u.iter().map(|s| format!("`\\{s}`")).collect::<Vec<_>>().join(", ")
                }
            })
            .unwrap_or_else(|| if r.path.is_none() { "no .sty found".to_string() } else { "inconclusive".to_string() });
        let ms = r.ms.map(|m| format!("{m:.2}")).unwrap_or_else(|| "n/a".to_string());
        let declined = if r.declined.is_empty() { "—".to_string() } else { r.declined.join(", ") };
        md.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} |\n",
            r.name,
            if r.completed { "Yes" } else { "No" },
            n,
            md_escape_cell(&names),
            ms,
            md_escape_cell(&declined)
        ));
    }
    md.push_str("\n## Diagnostics per package (run 1)\n");
    for r in &rows {
        if r.path.is_none() {
            md.push_str(&format!("\n### {} — no .sty file found\n", r.name));
            continue;
        }
        md.push_str(&format!("\n### {} — {} error(s), {} warning(s)\n", r.name, r.errors.len(), r.warnings.len()));
        for e in &r.errors {
            md.push_str(&format!("- [Error] {}\n", md_escape_cell(e)));
        }
        for w in &r.warnings {
            md.push_str(&format!("- [Warning] {}\n", md_escape_cell(w)));
        }
        if r.errors.is_empty() && r.warnings.is_empty() {
            md.push_str("- none\n");
        }
    }
    md.push_str(&format!(
        "\n## Environment\n\n- kpsewhich: `{}`\n- measured: {} UTC\n",
        kpsewhich.display(),
        chrono_stamp()
    ));
    for r in &rows {
        if let Some(p) = &r.path {
            md.push_str(&format!("- {}: `{p}`\n", r.name));
        }
    }

    let out = out_path.unwrap_or_else(|| {
        format!("{}/../../docs/evidence/plan4-real-sty-harness-{date}.md", env!("CARGO_MANIFEST_DIR"))
    });
    std::fs::write(&out, &md).unwrap_or_else(|e| {
        eprintln!("BLOCKED: cannot write {out}: {e}");
        std::process::exit(2);
    });
    print!("{md}");
    eprintln!("wrote {out}");
}

fn chrono_stamp() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let day = (secs / 86400) as i64;
    let tod = (secs % 86400) as i64;
    let date = civil_date(day);
    format!("{date}T{:02}:{:02}:{:02}Z", tod / 3600, tod % 3600 / 60, tod % 60)
}
