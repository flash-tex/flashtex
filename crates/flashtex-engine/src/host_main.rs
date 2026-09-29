//! `flashtex-host`: the resident engine (DESIGN.md §5.1, L1) as a small
//! program, for the editor and for measuring.
//!
//! ```text
//! flashtex-host serve  [HOST OPTIONS] -- <pdfTeX command line>
//! flashtex-host bench  [HOST OPTIONS] -- <pdfTeX command line>
//! flashtex-host open   S0FILE [HOST OPTIONS] -- <pdfTeX command line>
//! flashtex-host selftest [HOST OPTIONS] -- <pdfTeX command line>
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
//! `--max-targets N` (selftest), `--print-terminal`.

use flashtex_engine::host::Session;
use std::collections::BTreeMap;
use std::io::BufRead;

struct HostOpts {
    reps: usize,
    edit: Option<String>,
    save: Option<String>,
    every_shipout: bool,
    max_targets: usize,
    print_terminal: bool,
}

fn usage() -> ! {
    eprintln!(
        "usage: flashtex-host serve|bench|selftest|open S0FILE [options] -- <pdfTeX command line>"
    );
    std::process::exit(2)
}

fn main() {
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
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
            _ => usage(),
        }
        i += 1;
    }
    if i >= argv.len() {
        usage();
    }
    // The pdfTeX command line, with this program's name as argv[0].
    let mut pdftex_argv = vec!["pdftex".to_string()];
    pdftex_argv.extend_from_slice(&argv[i + 1..]);
    let o = flashtex_engine::cli::parse(&pdftex_argv);
    let code = match cmd.as_str() {
        "serve" => serve(o, &ho),
        "bench" => bench(o, &ho),
        "open" => open(o, &ho, s0file.as_deref().unwrap()),
        "selftest" => selftest(o, &ho),
        _ => usage(),
    };
    std::process::exit(code)
}

fn fail(e: String) -> i32 {
    println!("{{\"error\":{e:?}}}");
    1
}

fn serve(o: flashtex_engine::system::RunOptions, ho: &HostOpts) -> i32 {
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

/// Alternate one word in the middle of `path` between two spellings of the
/// same length.
fn edit_file(path: &str, k: usize) {
    let mut d = std::fs::read(path).expect("read the file to edit");
    let mid = d.len() / 2;
    let at = (mid..d.len().saturating_sub(8))
        .find(|&i| d[i] == b' ' && d[i + 1..i + 6].iter().all(|c| c.is_ascii_lowercase()))
        .expect("a word to edit");
    let w = if k % 2 == 0 { b"zzzzz" } else { b"yyyyy" };
    d[at + 1..at + 6].copy_from_slice(w);
    std::fs::write(path, d).expect("write the edited file");
}

fn bench(o: flashtex_engine::system::RunOptions, ho: &HostOpts) -> i32 {
    let mut s = Session::new(o, None);
    s.every_shipout = ho.every_shipout;
    match s.compile() {
        Ok(r) => println!("{}", r.json()),
        Err(e) => return fail(e),
    }
    for k in 0..ho.reps {
        if let Some(f) = &ho.edit {
            edit_file(f, k);
        }
        match s.compile() {
            Ok(r) => println!("{}", r.json()),
            Err(e) => return fail(e),
        }
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

fn open(o: flashtex_engine::system::RunOptions, ho: &HostOpts, path: &str) -> i32 {
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

fn selftest(o: flashtex_engine::system::RunOptions, ho: &HostOpts) -> i32 {
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
