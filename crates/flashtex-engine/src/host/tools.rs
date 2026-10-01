//! `flashtex-host`: the resident engine (DESIGN.md §5.1, L1) as a small
//! program, for the editor and for measuring.
//!
//! ```text
//! flashtex-host serve  [HOST OPTIONS] -- <pdfTeX command line>
//! flashtex-host iserve [HOST OPTIONS] -- <pdfTeX command line>
//! flashtex-host bench  [HOST OPTIONS] -- <pdfTeX command line>
//! flashtex-host open   S0FILE [HOST OPTIONS] -- <pdfTeX command line>
//! flashtex-host selftest [HOST OPTIONS] -- <pdfTeX command line>
//! flashtex-host layout -- <anything>
//! ```
//!
//! The pdfTeX command line is `flashtex-initex`'s (src/cli.rs), e.g.
//! `-fmt=pdflatex -interaction=batchmode doc.tex`.
//!
//! * `serve` reads commands from stdin, one per line, and answers each with
//!   one JSON line: `compile` (from S₀ when it still holds, else in full),
//!   `save PATH` (persist S₀), `quit`.
//! * `bench` compiles once cold, then `--reps N` times after editing
//!   `--edit FILE` (a word in the middle of FILE alternates between two
//!   spellings), and with `--save PATH` then persists S₀; one JSON line
//!   each.
//! * `open` opens a persisted S₀ in this fresh process and compiles from it.
//! * `selftest` is the checkpoint layer's bit-identity test: a run with a
//!   checkpoint at S₀ and after every shipout, then for each checkpoint:
//!   restore it, check the state is bit-identical to the one recorded
//!   there, run to the end, and check every later checkpoint's state, the
//!   final state, every file in the directory and the terminal against the
//!   uninterrupted run; plus `redo_to` and `restore_discard`. Exit status 0
//!   when everything matched.
//!
//! Host options: `--reps N`, `--edit FILE`, `--save PATH`, `--every-shipout`,
//! `--max-targets N` (selftest), `--print-terminal`, `--keep DIR` (bench:
//! copy the directory's files to DIR/<compile>/ after each compile),
//! `--argv0 NAME` (the pdfTeX command line's argv[0]; default `pdftex`).

use super::Session;
use std::collections::BTreeMap;
use std::io::BufRead;

#[derive(Clone)]
struct HostOpts {
    reps: usize,
    edit: Option<String>,
    save: Option<String>,
    every_shipout: bool,
    max_targets: usize,
    print_terminal: bool,
    /// `bench`: copy every file of the directory to DIR/<compile>/ after
    /// each compile.
    keep: Option<String>,
    /// argv[0] of the pdfTeX command line (the C parts print it in
    /// warnings and errors); `pdftex` by default.
    argv0: String,
    /// `iserve`: the undo-log budget in bytes, the timed-checkpoint
    /// interval in seconds, no preview mode, no convergence.
    budget: Option<usize>,
    timed: Option<f64>,
    no_preview: bool,
    no_converge: bool,
}

fn usage() -> ! {
    eprintln!(
        "usage: flashtex-host serve|bench|selftest|open S0FILE [options] -- <pdfTeX command line>"
    );
    std::process::exit(2)
}

/// `flashtex-host serve|iserve|bench|open|selftest|layout ...`: `argv` is
/// the whole command line; returns the exit status.
pub fn main(argv: Vec<String>) -> i32 {
    let Some(cmd) = argv.get(1).cloned() else {
        usage()
    };
    let mut i = 2;
    let mut s0file = None;
    if cmd == "open" {
        s0file = Some(argv.get(2).cloned().unwrap_or_else(|| usage()));
        i = 3;
    }
    let mut ho = HostOpts {
        reps: 3,
        edit: None,
        save: None,
        every_shipout: false,
        max_targets: 8,
        print_terminal: false,
        keep: None,
        argv0: "pdftex".into(),
        budget: None,
        timed: None,
        no_preview: false,
        no_converge: false,
    };
    while i < argv.len() && argv[i] != "--" {
        let v = argv.get(i + 1).cloned();
        match argv[i].as_str() {
            "--reps" => {
                ho.reps = v.and_then(|v| v.parse().ok()).unwrap_or_else(|| usage());
                i += 1;
            }
            "--edit" => {
                ho.edit = Some(v.unwrap_or_else(|| usage()));
                i += 1;
            }
            "--save" => {
                ho.save = Some(v.unwrap_or_else(|| usage()));
                i += 1;
            }
            "--max-targets" => {
                ho.max_targets = v.and_then(|v| v.parse().ok()).unwrap_or_else(|| usage());
                i += 1;
            }
            "--every-shipout" => ho.every_shipout = true,
            "--print-terminal" => ho.print_terminal = true,
            "--argv0" => {
                ho.argv0 = v.unwrap_or_else(|| usage());
                i += 1;
            }
            "--keep" => {
                ho.keep = Some(v.unwrap_or_else(|| usage()));
                i += 1;
            }
            "--budget" => {
                ho.budget = Some(v.and_then(|v| v.parse().ok()).unwrap_or_else(|| usage()));
                i += 1;
            }
            "--timed" => {
                ho.timed = Some(v.and_then(|v| v.parse().ok()).unwrap_or_else(|| usage()));
                i += 1;
            }
            "--no-preview" => ho.no_preview = true,
            "--no-converge" => ho.no_converge = true,
            _ => usage(),
        }
        i += 1;
    }
    if i >= argv.len() {
        usage();
    }
    // The pdfTeX command line, with this program's name as argv[0].
    let mut pdftex_argv = vec![ho.argv0.clone()];
    pdftex_argv.extend_from_slice(&argv[i + 1..]);
    let o = crate::cli::parse(&pdftex_argv);
    let code = match cmd.as_str() {
        "serve" => serve(o, &ho),
        "iserve" => iserve(o, &ho),
        "bench" => bench(o, &ho),
        "open" => open(o, &ho, s0file.as_deref().unwrap()),
        "selftest" => selftest(o, &ho),
        "layout" => layout(),
        _ => usage(),
    };
    code
}

fn fail(e: String) -> i32 {
    println!("{{\"error\":{e:?}}}");
    1
}

fn serve(o: crate::system::RunOptions, ho: &HostOpts) -> i32 {
    let mut s = Session::new(o, None);
    s.every_shipout = ho.every_shipout;
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line == "quit" {
            break;
        } else if line == "compile" {
            match s.compile() {
                Ok(r) => println!("{}", r.json()),
                Err(e) => return fail(e),
            }
        } else if let Some(p) = line.strip_prefix("save ") {
            let t = std::time::Instant::now();
            match s.save_s0(p) {
                Ok((len, disk)) => println!(
                    "{{\"saved\":{p:?},\"bytes\":{len},\"on_disk\":{disk},\"seconds\":{:.6}}}",
                    t.elapsed().as_secs_f64()
                ),
                Err(e) => println!("{{\"error\":{e:?}}}"),
            }
        } else {
            println!("{{\"error\":\"unknown command {line}\"}}");
        }
    }
    0
}

/// `iserve`: the L2-L4 session (`crate::incr`) on a line
/// protocol: `compile` (JSON report), `compile N` (stop once page N is
/// shipped), `finish` (continue a stopped compile), `pages` (every page's
/// frame hash and checkpoint), `stats`, `quit`.
fn iserve(o: crate::system::RunOptions, ho: &HostOpts) -> i32 {
    // On a thread with the socket host's deep stack (TeX's recursion; the
    // main thread has the system's 8 MB): an overflow there is the likeliest
    // way a session ended with its stderr discarded (review 2026-09-30).
    let ho = ho.clone();
    match std::thread::Builder::new()
        .name("engine".into())
        .stack_size(512 << 20)
        .spawn(move || iserve_on_this_thread(o, &ho))
    {
        Ok(h) => h.join().unwrap_or(101),
        Err(e) => {
            eprintln!("flashtex-host: cannot start the engine thread: {e}");
            1
        }
    }
}

fn iserve_on_this_thread(o: crate::system::RunOptions, ho: &HostOpts) -> i32 {
    use crate::incr::{Options, Session};
    let mut opts = Options::default();
    if let Some(b) = ho.budget {
        opts.budget = b;
    }
    if let Some(t) = ho.timed {
        opts.timed_s = t;
    }
    opts.preview = !ho.no_preview;
    opts.converge = !ho.no_converge;
    let mut s = Session::new(o, None, opts);
    let mut reason = "end of input";
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else {
            reason = "input error";
            break;
        };
        let line = line.trim();
        super::crash::serving(line);
        let out = if line == "quit" {
            reason = "quit";
            break;
        } else if line == "compile" {
            s.compile(None).map(|r| r.json())
        } else if line == "compile-defer" {
            // The first pass only, as when external tools are due
            // (`Session::set_defer`): the next `compile` takes up the rest.
            s.set_defer(Some(std::rc::Rc::new(|_| true)));
            let r = s.compile(None).map(|r| r.json());
            s.set_defer(None);
            r
        } else if let Some(a) = line.strip_prefix("compile-interrupt ") {
            // A newer edit arrives during pass P after the run shipped N
            // pages: the compile is preempted there (the soundness
            // driver's interleaved edits; `Session::set_preempt`).
            let v: Vec<usize> = a
                .split_whitespace()
                .filter_map(|x| x.parse().ok())
                .collect();
            match v[..] {
                [pass, n] => {
                    let armed = std::rc::Rc::new(std::cell::Cell::new(true));
                    let a2 = armed.clone();
                    s.set_preempt(Some(std::rc::Rc::new(move |p: usize, pages: usize| {
                        let hit = a2.get() && p == pass && pages >= n;
                        if hit {
                            a2.set(false);
                        }
                        hit
                    })));
                    let r = s.compile(None).map(|r| r.json());
                    s.set_preempt(None);
                    r
                }
                _ => Err(format!("compile-interrupt PASS PAGES, not {a}")),
            }
        } else if let Some(n) = line.strip_prefix("compile ") {
            match n.trim().parse::<usize>() {
                Ok(n) => s.compile(Some(n)).map(|r| r.json()),
                Err(_) => Err(format!("bad page {n}")),
            }
        } else if line == "finish" {
            s.finish().map(|r| r.json())
        } else if let Some(p) = line.strip_prefix("save ") {
            let t = std::time::Instant::now();
            s.save_s0(p.trim()).map(|(len, disk)| {
                format!(
                    "{{\"saved\":{p:?},\"bytes\":{len},\"on_disk\":{disk},\"seconds\":{:.6}}}",
                    t.elapsed().as_secs_f64()
                )
            })
        } else if let Some(rest) = line.strip_prefix("open ") {
            let mut it = rest.split_whitespace();
            let p = it.next().unwrap_or_default().to_string();
            let stop = it.next().and_then(|n| n.parse::<usize>().ok());
            s.open_s0(&p, stop).map(|r| r.json())
        } else if let Some(d) = line.strip_prefix("warm ") {
            s.warm_up(d.trim())
                .map(|t| format!("{{\"warm_s\":{t:.6}}}"))
        } else if line == "pages" {
            let v: Vec<String> = s
                .pages
                .iter()
                .map(|p| {
                    format!(
                        "[\"{:016x}{:016x}\",{},{}]",
                        p.frame[0],
                        p.frame[1],
                        p.frame_len,
                        p.ckpt.map_or("null".to_string(), |c| c.to_string())
                    )
                })
                .collect();
            Ok(format!("{{\"pages\":[{}]}}", v.join(",")))
        } else if line == "mem" {
            // memory accounting (`Session::mem_stats`, lane P4-MEMORY)
            let kv: Vec<String> = s
                .mem_stats()
                .iter()
                .map(|(k, v)| format!("\"{k}\":{v}"))
                .collect();
            Ok(format!("{{\"mem\":{{{}}}}}", kv.join(",")))
        } else if line == "terminal" {
            Ok(format!(
                "{{\"terminal\":{:?}}}",
                String::from_utf8_lossy(&s.terminal())
            ))
        } else {
            Err(format!("unknown command {line}"))
        };
        match out {
            Ok(j) => println!("{j}"),
            Err(e) => println!("{{\"error\":{e:?}}}"),
        }
        use std::io::Write;
        std::io::stdout().flush().ok();
        // as the socket host does after each compile's DONE (and so that
        // the soundness sweeps run through prepared restores);
        // FLASHTEX_NO_PREPARE=1 leaves it out
        if line.starts_with("compile") && std::env::var_os("FLASHTEX_NO_PREPARE").is_none() {
            s.prepare_next(&mut || false);
        }
    }
    super::crash::exit(reason);
    0
}

/// Alternate one word in the middle of `path` between two spellings of the
/// same length.
fn edit_file(path: &str, k: usize) {
    let mut d = std::fs::read(path).expect("read the file to edit");
    let mid = d.len() / 2;
    let at = (mid..d.len().saturating_sub(8))
        .find(|&i| d[i] == b' ' && d[i + 1..i + 6].iter().all(|c| c.is_ascii_lowercase()))
        .expect("a word to edit");
    let w = if k.is_multiple_of(2) {
        b"zzzzz"
    } else {
        b"yyyyy"
    };
    d[at + 1..at + 6].copy_from_slice(w);
    std::fs::write(path, d).expect("write the edited file");
}

fn bench(o: crate::system::RunOptions, ho: &HostOpts) -> i32 {
    let mut s = Session::new(o, None);
    s.every_shipout = ho.every_shipout;
    let keep = |k: usize| {
        if let Some(d) = &ho.keep {
            let d = format!("{d}/{k}");
            std::fs::create_dir_all(&d).expect("make the --keep directory");
            for (n, b) in dir_snapshot() {
                std::fs::write(format!("{d}/{n}"), b).expect("keep a file");
            }
        }
    };
    match s.compile() {
        Ok(r) => println!("{}", r.json()),
        Err(e) => return fail(e),
    }
    keep(0);
    for k in 0..ho.reps {
        if let Some(f) = &ho.edit {
            edit_file(f, k);
        }
        match s.compile() {
            Ok(r) => println!("{}", r.json()),
            Err(e) => return fail(e),
        }
        keep(k + 1);
    }
    if let Some(p) = &ho.save {
        let t = std::time::Instant::now();
        match s.save_s0(p) {
            Ok((len, disk)) => println!(
                "{{\"saved\":{p:?},\"bytes\":{len},\"on_disk\":{disk},\"seconds\":{:.6}}}",
                t.elapsed().as_secs_f64()
            ),
            Err(e) => return fail(e),
        }
    }
    if ho.print_terminal {
        print!("{}", String::from_utf8_lossy(&s.terminal()));
    }
    0
}

fn open(o: crate::system::RunOptions, ho: &HostOpts, path: &str) -> i32 {
    let t = std::time::Instant::now();
    let (mut s, rep) = match Session::open_s0(o, path) {
        Ok(x) => x,
        Err(e) => return fail(e),
    };
    println!("{}", rep.json());
    s.every_shipout = ho.every_shipout;
    match s.compile() {
        Ok(r) => println!("{}", r.json()),
        Err(e) => return fail(e),
    }
    println!(
        "{{\"open_and_compile_s\":{:.6}}}",
        t.elapsed().as_secs_f64()
    );
    if ho.print_terminal {
        print!("{}", String::from_utf8_lossy(&s.terminal()));
    }
    0
}

/// The word space's layout, as JSON: its size, the scalar region, and the
/// largest arrays.
fn layout() -> i32 {
    let g = crate::Globals::new();
    let a = &g.arena;
    let mut regions = a.regions.clone();
    regions.sort_by_key(|r| std::cmp::Reverse(r.bytes));
    let top: Vec<String> = regions
        .iter()
        .take(12)
        .map(|r| format!("{:?}:{}", r.name, r.bytes))
        .collect();
    println!(
        "{{\"bytes\":{},\"chunks\":{},\"scalar_bytes\":{},\"arrays\":{},\"largest\":{{{}}}}}",
        a.len_bytes(),
        a.chunks(),
        a.scalar_bytes(),
        a.regions.len() - 1,
        top.join(",")
    );
    0
}

/// Every regular file in the working directory, by name.
fn dir_snapshot() -> BTreeMap<String, Vec<u8>> {
    let mut m = BTreeMap::new();
    for e in std::fs::read_dir(".").unwrap().flatten() {
        if e.file_type().map(|t| t.is_file()).unwrap_or(false) {
            let n = e.file_name().to_string_lossy().into_owned();
            if let Ok(d) = std::fs::read(e.path()) {
                m.insert(n, d);
            }
        }
    }
    m
}

fn diff_dirs(a: &BTreeMap<String, Vec<u8>>, b: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut out = vec![];
    for (k, v) in a {
        match b.get(k) {
            None => out.push(format!("{k} missing")),
            Some(w) if w != v => {
                out.push(format!("{k} differs ({} vs {} bytes)", v.len(), w.len()))
            }
            _ => {}
        }
    }
    for k in b.keys() {
        if !a.contains_key(k) {
            out.push(format!("{k} extra"));
        }
    }
    out
}

fn selftest(o: crate::system::RunOptions, ho: &HostOpts) -> i32 {
    let mut s = Session::new(o, None);
    s.every_shipout = true;
    s.hash_states = true;
    let r = match s.cold() {
        Ok(r) => r,
        Err(e) => return fail(e),
    };
    let g = s.g.as_mut().unwrap();
    let hashes: Vec<(u64, [u64; 2])> = g.layer().state_hashes.clone();
    let end_hash = g.state_hash();
    let dir = dir_snapshot();
    let term = s.terminal();
    let mut failures = vec![];
    let mut checks = 0usize;
    println!(
        "{{\"uninterrupted\":{},\"checkpoints\":{},\"s0\":{}}}",
        r.json(),
        hashes.len(),
        s.s0.is_some()
    );
    if hashes.is_empty() {
        return fail("no checkpoint was taken".into());
    }
    // The targets: all checkpoints, or an even sample, latest first (a
    // branching restore detaches the later ones).
    let n = hashes.len();
    let mut targets: Vec<usize> = if n <= ho.max_targets {
        (0..n).collect()
    } else {
        (0..ho.max_targets)
            .map(|k| k * (n - 1) / (ho.max_targets - 1).max(1))
            .collect()
    };
    targets.dedup();
    targets.reverse();

    // redo_to: restore a middle checkpoint and jump straight back.
    {
        let g = s.g.as_mut().unwrap();
        let (id, _) = hashes[n / 2];
        let res = g.restore(id).and_then(|_| g.redo_to(id));
        checks += 1;
        match res {
            Err(e) => failures.push(format!("redo_to({id}): {e}")),
            Ok(()) => {
                if g.state_hash() != end_hash {
                    failures.push(format!("redo_to({id}): state differs from the end"));
                }
                let d = diff_dirs(&dir, &dir_snapshot());
                if !d.is_empty() {
                    failures.push(format!("redo_to({id}): files: {}", d.join(", ")));
                }
                if s.terminal() != term {
                    failures.push(format!("redo_to({id}): terminal differs"));
                }
            }
        }
    }

    for &t in &targets {
        let g = s.g.as_mut().unwrap();
        let (id, h) = hashes[t];
        let t0 = std::time::Instant::now();
        if let Err(e) = g.restore(id) {
            failures.push(format!("restore({id}): {e}"));
            continue;
        }
        let restore_s = t0.elapsed().as_secs_f64();
        checks += 1;
        if g.state_hash() != h {
            failures.push(format!("restore({id}): state is not the one recorded"));
        }
        g.layer().state_hashes.clear();
        let t1 = std::time::Instant::now();
        let st = g.resume_to_end();
        let run_s = t1.elapsed().as_secs_f64();
        if let Err(e) = st {
            failures.push(format!("resume from {id}: {e}"));
            continue;
        }
        let again: Vec<[u64; 2]> = g.layer().state_hashes.iter().map(|x| x.1).collect();
        let want: Vec<[u64; 2]> = hashes[t + 1..].iter().map(|x| x.1).collect();
        checks += 1;
        if again != want {
            failures.push(format!(
                "resume from {id}: {} of {} later checkpoints differ",
                again.iter().zip(&want).filter(|(a, b)| a != b).count()
                    + again.len().abs_diff(want.len()),
                want.len()
            ));
        }
        checks += 1;
        if g.state_hash() != end_hash {
            failures.push(format!("resume from {id}: final state differs"));
        }
        checks += 1;
        let d = diff_dirs(&dir, &dir_snapshot());
        if !d.is_empty() {
            failures.push(format!("resume from {id}: files: {}", d.join(", ")));
        }
        checks += 1;
        if s.terminal() != term {
            failures.push(format!("resume from {id}: terminal differs"));
        }
        println!(
            "{{\"target\":{id},\"index\":{t},\"restore_s\":{restore_s:.6},\"rerun_s\":{run_s:.6},\"later_checkpoints\":{}}}",
            want.len()
        );
    }

    // The plain restart.
    {
        let g = s.g.as_mut().unwrap();
        let (id, h) = hashes[0];
        match g.restore_discard(id) {
            Err(e) => failures.push(format!("restore_discard({id}): {e}")),
            Ok(()) => {
                checks += 1;
                if g.state_hash() != h {
                    failures.push(format!(
                        "restore_discard({id}): state is not the one recorded"
                    ));
                }
                match g.resume_to_end() {
                    Err(e) => failures.push(format!("resume after restore_discard: {e}")),
                    Ok(_) => {
                        checks += 2;
                        if g.state_hash() != end_hash {
                            failures.push("restore_discard: final state differs".into());
                        }
                        let d = diff_dirs(&dir, &dir_snapshot());
                        if !d.is_empty() {
                            failures.push(format!("restore_discard: files: {}", d.join(", ")));
                        }
                    }
                }
            }
        }
    }

    println!(
        "{{\"checks\":{checks},\"failures\":{},\"detail\":{:?}}}",
        failures.len(),
        failures
    );
    if failures.is_empty() {
        0
    } else {
        1
    }
}
