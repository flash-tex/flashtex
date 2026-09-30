//! `latency-bench`: the T7 latency gate (docs/design/engine-v2/DESIGN.md
//! §1.2 targets, §8 T7). See README.md.
//!
//! ```text
//! latency-bench run   --host PATH --docs DIR --out DIR [--only plain-10,full-100]
//!                     [--local N] [--reflow N] [--preamble N] [--reopen N]
//! latency-bench check SUMMARY.json [--margin 0.10] [--min-samples 6]
//! ```
//!
//! `run` starts `flashtex-host` (a separate GPL process, never linked) for
//! each document in DIR (`gen.py` makes them), drives scripted edits through
//! its display-list-v3 socket and times, on this side of the socket:
//!
//! * `local`: a one-letter insertion and its revert inside a paragraph at
//!   the start, middle and end of the document, COMPILE to the frame of the
//!   page showing the edit;
//! * `reflow`: a 28-word insertion (two or more lines, so every later page
//!   moves) and its revert, the same;
//! * `preamble`: a `\newcommand` added to and removed from the preamble,
//!   COMPILE to the first visible page (page 1 current);
//! * `reopen`: a new host process opening the document from the S₀ its
//!   predecessor persisted (`--s0-cache`), after an edit on disk, COMPILE
//!   to the first visible page.
//!
//! After every measured compile it compiles again, unmeasured, until the
//! host reports `unchanged` (the client's rerun after an `.aux` change,
//! protocol §6.3). Every sample records the load average and the CI
//! runners' state. It writes `samples.jsonl`, `summary.json` and
//! `summary.md` to `--out`.
//!
//! `check` is the gate: exit 1 when a row's p95 exceeds its §1.2 target by
//! more than the noise margin, or a frame never arrived.
//!
//! MIT. It talks to the GPL host only over the socket.

mod doc;
mod host;
mod machine;
mod measure;
mod report;

use doc::{Doc, Region};
use flashtex_display_list::client::Client;
use flashtex_display_list::json::{obj, s, Json};
use host::Host;
use machine::num;
use measure::{Conn, Timing, Watch};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAIN: &str = "main.tex";

fn arg(a: &[String], k: &str) -> Option<String> {
    a.iter()
        .position(|x| x == k)
        .and_then(|i| a.get(i + 1))
        .cloned()
}

fn usage() -> ! {
    eprintln!(
        "usage: latency-bench run --host PATH --docs DIR --out DIR [--pool FILE] [--only a,b] \
         [--local N] [--reflow N] [--preamble N] [--reopen N] [--label TEXT]\n       \
         latency-bench check SUMMARY.json [--margin 0.10] [--min-samples 6]"
    );
    std::process::exit(2)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let code = match a.get(1).map(String::as_str) {
        Some("run") => run(&a),
        Some("check") => check(&a),
        _ => usage(),
    };
    std::process::exit(code)
}

fn check(a: &[String]) -> i32 {
    let Some(path) = a.get(2) else { usage() };
    let margin: f64 = arg(a, "--margin")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.10);
    let min_n: i64 = arg(a, "--min-samples")
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("latency-bench: {path}: {e}");
            return 2;
        }
    };
    let j = match Json::parse(&text) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("latency-bench: {path}: {e}");
            return 2;
        }
    };
    let rows = j
        .get("rows")
        .and_then(Json::as_array)
        .unwrap_or(&[])
        .to_vec();
    if rows.is_empty() {
        eprintln!("latency-bench: {path}: no rows");
        return 1;
    }
    print!("{}", report::markdown(&rows));
    let fails = report::verdict(&rows, margin, min_n);
    if fails.is_empty() {
        println!(
            "\nT7: all {} rows within their DESIGN.md §1.2 targets (+{:.0}% noise margin)",
            rows.len(),
            margin * 100.0
        );
        0
    } else {
        println!(
            "\nT7: {} of {} rows fail (DESIGN.md §1.2, +{:.0}% noise margin):",
            fails.len(),
            rows.len(),
            margin * 100.0
        );
        for f in &fails {
            println!("  FAIL {f}");
        }
        1
    }
}

struct Opts {
    host: PathBuf,
    /// The engine's string pool (`FLASHTEX_POOL`), when not beside the host.
    pool: Option<PathBuf>,
    out: PathBuf,
    local: usize,
    reflow: usize,
    preamble: usize,
    reopen: usize,
}

fn run(a: &[String]) -> i32 {
    let (Some(host), Some(docs), Some(out)) = (arg(a, "--host"), arg(a, "--docs"), arg(a, "--out"))
    else {
        usage()
    };
    let n = |k: &str, d: usize| arg(a, k).and_then(|v| v.parse().ok()).unwrap_or(d);
    let o = Opts {
        host: std::fs::canonicalize(&host).unwrap_or_else(|_| PathBuf::from(&host)),
        pool: arg(a, "--pool")
            .map(PathBuf::from)
            .or_else(|| {
                // A development build: the pool is in the source tree.
                let p = PathBuf::from("crates/flashtex-engine/pdftex.pool");
                (std::env::var_os("FLASHTEX_POOL").is_none() && p.is_file()).then_some(p)
            })
            .map(|p| std::fs::canonicalize(&p).unwrap_or(p)),
        out: PathBuf::from(&out),
        local: n("--local", 6),
        reflow: n("--reflow", 3),
        preamble: n("--preamble", 3),
        reopen: n("--reopen", 6),
    };
    let only: Vec<String> = arg(a, "--only")
        .map(|v| v.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    if let Err(e) = std::fs::create_dir_all(&o.out) {
        eprintln!("latency-bench: {out}: {e}");
        return 2;
    }
    let mut names: Vec<(String, u32)> = std::fs::read_dir(&docs)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().join(MAIN).is_file())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let size = name.rsplit('-').next()?.parse().ok()?;
                    Some((name, size))
                })
                .collect()
        })
        .unwrap_or_default();
    names.retain(|(n, _)| only.is_empty() || only.contains(n));
    // plain before full, small before large.
    names.sort_by_key(|(n, size)| (*size, n.starts_with("full")));
    if names.is_empty() {
        eprintln!("latency-bench: no documents in {docs} (run gen.py {docs})");
        return 2;
    }
    let started = machine::snapshot();
    eprintln!("latency-bench: machine {}", started);
    let samples_path = o.out.join("samples.jsonl");
    let mut samples_f = match std::fs::File::create(&samples_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("latency-bench: {}: {e}", samples_path.display());
            return 2;
        }
    };
    let mut rows = Vec::new();
    let mut hello = Json::Null;
    let mut startup = String::new();
    let mut failed = Vec::new();
    for (name, _) in &names {
        let t = Instant::now();
        eprintln!("latency-bench: {name} ...");
        match bench_doc(&o, Path::new(&docs), name, &mut hello, &mut startup) {
            Ok((pages, samples)) => {
                for j in &samples {
                    let _ = writeln!(samples_f, "{j}");
                }
                for m in &report::METRICS {
                    let mine: Vec<&Json> = samples
                        .iter()
                        .filter(|j| j.str_field("metric") == Some(m.name))
                        .collect();
                    if !mine.is_empty() {
                        let r = report::row(name, pages, m, &mine);
                        eprintln!("  {r}");
                        rows.push(r);
                    }
                }
                eprintln!(
                    "latency-bench: {name}: {pages} pages, {} samples, {:.0} s",
                    samples.len(),
                    t.elapsed().as_secs_f64()
                );
            }
            Err(e) => {
                eprintln!("latency-bench: {name}: FAILED: {e}");
                failed.push(format!("{name}: {e}"));
            }
        }
    }
    let summary = obj([
        ("tool", s("latency-bench 0.1.0 (T7, DESIGN.md §1.2)")),
        ("label", s(arg(a, "--label").unwrap_or_default())),
        ("machine", machine::describe()),
        ("start", started),
        ("end", machine::snapshot()),
        ("host", hello),
        ("host_startup", s(startup)),
        (
            "targets",
            Json::Arr(
                report::METRICS
                    .iter()
                    .map(|m| {
                        obj([
                            ("metric", s(m.name)),
                            ("target_ms", num(m.target_ms)),
                            ("what", s(m.what)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "trials",
            obj([
                ("local", Json::Int(o.local as i64)),
                ("reflow", Json::Int(o.reflow as i64)),
                ("preamble", Json::Int(o.preamble as i64)),
                ("reopen", Json::Int(o.reopen as i64)),
            ]),
        ),
        (
            "failed",
            Json::Arr(failed.iter().map(|f| s(f.as_str())).collect()),
        ),
        ("rows", Json::Arr(rows.clone())),
    ]);
    let _ = std::fs::write(o.out.join("summary.json"), format!("{summary}\n"));
    let md = report::markdown(&rows);
    let _ = std::fs::write(o.out.join("summary.md"), &md);
    print!("{md}");
    if failed.is_empty() {
        0
    } else {
        1
    }
}

/// Fresh working copy of a document, and its S₀ cache directory.
fn prepare(o: &Opts, docs: &Path, name: &str) -> Result<(PathBuf, PathBuf), String> {
    let work = o.out.join("work").join(name);
    let s0 = o.out.join("work").join(format!("{name}.s0"));
    let _ = std::fs::remove_dir_all(&work);
    let _ = std::fs::remove_dir_all(&s0);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&s0).map_err(|e| e.to_string())?;
    std::fs::copy(docs.join(name).join(MAIN), work.join(MAIN)).map_err(|e| e.to_string())?;
    let work = std::fs::canonicalize(&work).map_err(|e| e.to_string())?;
    let s0 = std::fs::canonicalize(&s0).map_err(|e| e.to_string())?;
    Ok((work, s0))
}

/// A short socket path (sun_path holds 104 bytes on macOS).
fn socket_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("lb-{}-{name}.sock", std::process::id()))
}

fn connect(h: &Host, main_abs: &str, output_dir: &str) -> Result<Conn, String> {
    let mut c = Client::connect(&h.socket).map_err(|e| format!("connect: {e}"))?;
    // A hung host fails the run instead of the job's timeout.
    let _ = c
        .reader()
        .get_ref()
        .set_read_timeout(Some(Duration::from_secs(1800)));
    Ok(Conn::new(c, main_abs, output_dir))
}

/// Sample record: the timing, the host's report, the machine's state.
/// What a sample measured.
struct Tag<'a> {
    metric: &'a str,
    region: &'a str,
    op: &'a str,
    trial: usize,
    viewport: Option<u32>,
}

fn sample(name: &str, tag: Tag, t: &Timing, extra: Vec<(String, Json)>) -> Json {
    let Tag {
        metric,
        region,
        op,
        trial,
        viewport,
    } = tag;
    let l = machine::loadavg();
    let (reg, busy) = machine::runners();
    let d = &t.done;
    let pick = |k: &str| d.get(k).cloned().unwrap_or(Json::Null);
    let mut kv: Vec<(String, Json)> = vec![
        ("doc".into(), s(name)),
        ("metric".into(), s(metric)),
        ("region".into(), s(region)),
        ("op".into(), s(op)),
        ("trial".into(), Json::Int(trial as i64)),
        (
            "viewport".into(),
            viewport.map(|v| Json::Int(v as i64)).unwrap_or(Json::Null),
        ),
        (
            "watched_ms".into(),
            t.watched_ms.map(num3).unwrap_or(Json::Null),
        ),
        (
            "watched_page".into(),
            t.watched_page
                .map(|v| Json::Int(v as i64))
                .unwrap_or(Json::Null),
        ),
        ("watched_by_frame".into(), Json::Bool(t.watched_by_frame)),
        (
            "first_frame_ms".into(),
            t.first_frame_ms.map(num3).unwrap_or(Json::Null),
        ),
        ("done_ms".into(), num3(t.done_ms)),
        ("frames".into(), Json::Int(t.frames as i64)),
        ("frames_after".into(), Json::Int(t.frames_after as i64)),
        ("host_first_page_ms".into(), pick("first_page_ms")),
        ("host_viewport_ms".into(), pick("viewport_ms")),
        ("host_run_ms".into(), pick("run_ms")),
        ("mode".into(), pick("mode")),
        ("status".into(), pick("status")),
        ("restart_page".into(), pick("restart_page")),
        ("converged_at".into(), pick("converged_at")),
        ("typeset_pages".into(), pick("typeset_pages")),
        ("pages".into(), pick("pages")),
        ("cold_reason".into(), pick("cold_reason")),
        ("load1".into(), num(l[0])),
        ("load5".into(), num(l[1])),
        ("runners".into(), Json::Int(reg as i64)),
        ("runners_busy".into(), Json::Int(busy as i64)),
    ];
    kv.extend(extra);
    Json::Obj(kv)
}

fn num3(x: f64) -> Json {
    Json::Num((x * 1000.0).round() / 1000.0)
}

fn bench_doc(
    o: &Opts,
    docs: &Path,
    name: &str,
    hello: &mut Json,
    startup: &mut String,
) -> Result<(u32, Vec<Json>), String> {
    let (work, s0) = prepare(o, docs, name)?;
    let root = work.to_string_lossy().into_owned();
    let main_abs = work.join(MAIN).to_string_lossy().into_owned();
    let outdir = format!("{}.out", work.to_string_lossy());
    let _ = std::fs::remove_dir_all(&outdir);
    std::fs::create_dir_all(&outdir).map_err(|e| e.to_string())?;
    let log = o.out.join(format!("host-{name}.log"));
    let _ = std::fs::remove_file(&log);
    let mut d = Doc::new(std::fs::read_to_string(work.join(MAIN)).map_err(|e| e.to_string())?)?;
    let mut out = Vec::new();

    // Session 1: the editing session.
    let h = Host::start(&o.host, o.pool.as_deref(), &socket_path(name), &s0, &log)?;
    let mut c = connect(&h, &main_abs, &outdir)?;
    *hello = c.c.hello.clone();
    *startup = h.startup.clone();
    let cold = c.compile(&root, MAIN, vec![], Some(0), Some(Watch::Page(0)))?;
    if cold.watched_ms.is_none() || cold.done.str_field("status") != Some("ok") {
        return Err(format!(
            "the first compile did not produce page 1: DONE {} DIAGNOSTICS {}",
            cold.done,
            Json::Arr(cold.diagnostics.clone())
        ));
    }
    let settle = c.settle(&root, MAIN, 6)?;
    out.push(sample(
        name,
        Tag {
            metric: "cold",
            region: "",
            op: "open",
            trial: 0,
            viewport: Some(0),
        },
        &cold,
        vec![
            ("settle_runs".into(), Json::Int(settle.0 as i64)),
            ("settle_ms".into(), num3(settle.1)),
            ("host_ready_ms".into(), num3(h.ready_ms)),
        ],
    ));
    let pages = c.pages;
    eprintln!(
        "  cold: {pages} pages, first page {:.1} ms, done {:.0} ms, settled in {} runs",
        cold.watched_ms.unwrap_or(f64::NAN),
        cold.done_ms,
        settle.0
    );

    // Body edits: a letter (local) and a sentence (reflow), each with its
    // revert, at the start, middle and end.
    for (metric, text, trials) in [("local", "x", o.local), ("reflow", doc::SENTENCE, o.reflow)] {
        for region in Region::ALL {
            for trial in 0..trials {
                let site = d.site(region, trial);
                let vp = c.page_of(site.line, site.col);
                let watch = Watch::Edit {
                    line: site.line,
                    col: site.col,
                };
                for op in ["insert", "revert"] {
                    let e = if op == "insert" {
                        doc::insert(MAIN, site.offset, text)
                    } else {
                        doc::delete(MAIN, site.offset, text.len())
                    };
                    d.apply(&e);
                    let t = c.compile(&root, MAIN, vec![e], vp, Some(watch))?;
                    let st = c.settle(&root, MAIN, 6)?;
                    out.push(sample(
                        name,
                        Tag {
                            metric,
                            region: region.name(),
                            op,
                            trial,
                            viewport: vp,
                        },
                        &t,
                        vec![
                            ("line".into(), Json::Int(site.line as i64)),
                            ("col".into(), Json::Int(site.col as i64)),
                            ("settle_runs".into(), Json::Int(st.0 as i64)),
                            ("settle_ms".into(), num3(st.1)),
                        ],
                    ));
                }
            }
        }
    }

    // Preamble edits: a definition added and removed (a full run from the
    // format each time), page 1 in view.
    for trial in 0..o.preamble {
        let def = format!(
            "\\newcommand{{\\latencybench{}}}{{{trial}}}",
            "x".repeat(trial + 1)
        );
        let at = d.preamble_end;
        for op in ["insert", "revert"] {
            let e = if op == "insert" {
                doc::insert(MAIN, at, &def)
            } else {
                doc::delete(MAIN, at, def.len())
            };
            d.apply(&e);
            let t = c.compile(&root, MAIN, vec![e], Some(0), Some(Watch::Page(0)))?;
            let st = c.settle(&root, MAIN, 6)?;
            out.push(sample(
                name,
                Tag {
                    metric: "preamble",
                    region: "preamble",
                    op,
                    trial,
                    viewport: Some(0),
                },
                &t,
                vec![
                    ("settle_runs".into(), Json::Int(st.0 as i64)),
                    ("settle_ms".into(), num3(st.1)),
                ],
            ));
        }
    }
    // The last full run (the preamble revert) persisted S₀.
    let saved: Vec<String> = h
        .drain()
        .into_iter()
        .filter(|l| l.contains("saved_s0"))
        .collect();
    let _ = c.c.bye();
    drop(c);
    drop(h);
    if saved.is_empty() && o.reopen > 0 {
        return Err("the host never reported saving S0 (saved_s0); cannot measure reopen".into());
    }

    // Reopen: a new host each time, after an edit on disk ("a recently
    // edited document"), from the persisted S₀.
    for trial in 0..o.reopen {
        let site = d.site(Region::Middle, trial);
        let e = if trial % 2 == 0 {
            doc::insert(MAIN, site.offset, "x")
        } else {
            doc::delete(MAIN, d.site(Region::Middle, trial - 1).offset, 1)
        };
        d.apply(&e);
        std::fs::write(work.join(MAIN), &d.text).map_err(|e| e.to_string())?;
        let h = Host::start(&o.host, o.pool.as_deref(), &socket_path(name), &s0, &log)?;
        let tc = Instant::now();
        let mut c = connect(&h, &main_abs, &outdir)?;
        let connect_ms = tc.elapsed().as_secs_f64() * 1e3;
        let t = c.compile(&root, MAIN, vec![], Some(0), Some(Watch::Page(0)))?;
        let spawn_to_page = t.watched_ms.map(|w| h.ready_ms + connect_ms + w);
        if t.done.str_field("mode") != Some("open") {
            eprintln!("  reopen {trial}: mode {} (not from S0)", t.done);
        }
        out.push(sample(
            name,
            Tag {
                metric: "reopen",
                region: "",
                op: "open",
                trial,
                viewport: Some(0),
            },
            &t,
            vec![
                ("host_ready_ms".into(), num3(h.ready_ms)),
                ("connect_ms".into(), num3(connect_ms)),
                (
                    "spawn_to_first_page_ms".into(),
                    spawn_to_page.map(num3).unwrap_or(Json::Null),
                ),
            ],
        ));
        let _ = c.c.bye();
    }
    // Leave the file as generated.
    if o.reopen % 2 == 1 {
        let e = doc::delete(MAIN, d.site(Region::Middle, o.reopen - 1).offset, 1);
        d.apply(&e);
        let _ = std::fs::write(work.join(MAIN), &d.text);
    }
    Ok((pages, out))
}
