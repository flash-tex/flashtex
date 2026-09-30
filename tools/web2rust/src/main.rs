//! web2rust — translate the WEB Pascal subset (Knuth's `tex.web`) to Rust.
//!
//! Usage:
//!   web2rust <file.web> [--change <file.ch> ...] --out-dir <dir> [--pool <file>] [--stat]
//!            [--const NAME=VALUE ...] [--emit-pascal <file>]
//!
//! See `tools/web2rust/README.md` for the regeneration command that the
//! generated crate is committed with.

// Index loops over statement lists read more plainly than iterator chains
// here, where the index is also a position in the Pascal source.
#![allow(clippy::needless_range_loop)]

mod changes;
mod emit;
mod parse;
mod tangle;
mod tok;

use std::path::PathBuf;
use std::process::ExitCode;

struct Args {
    web: PathBuf,
    /// `--change FILE`: WEB change files, applied in order (see changes.rs).
    changes: Vec<PathBuf>,
    out_dir: Option<PathBuf>,
    pool: Option<PathBuf>,
    emit_pascal: Option<PathBuf>,
    /// `--emit-web FILE`: write the WEB text after the change files, as
    /// `tie -m` would.
    emit_web: Option<PathBuf>,
    stat: bool,
    debug: bool,
    /// `--const NAME=VALUE`: override an outer-block constant. This is the job
    /// web2c splits between `tex.ch` and `texmf.cnf`; `tex.web` itself must stay
    /// unmodified (third_party/knuth/README.md).
    consts: Vec<(String, i64)>,
    /// `--macro NAME=VALUE`: override a WEB numeric macro (`@d name=value`).
    macros: Vec<(String, i64)>,
    /// `--scalar NAME=f32`: narrow a named `real` type, which is what web2c
    /// does to `glue_ratio`; `--scalar NAME=i64` widens a named integer type
    /// (web2c's `longinteger`).
    scalars: Vec<(String, parse::Ty)>,
    /// `--arena-cap NAME=EXPR`: the largest index a growable (`^T`) array
    /// global can reach, as a Rust expression over the outer-block
    /// constants; its region of the engine's word space is reserved for
    /// elements `0..EXPR` (see emit.rs, "the word space").
    arena_caps: Vec<(String, String)>,
}

fn parse_args() -> Result<Args, String> {
    // `@file` expands to the whitespace-separated arguments in `file` (`#`
    // starts a comment). The engine's configurations live in such files so
    // that the regeneration command, the drift test and the trip build all
    // read the same flags.
    let mut expanded: Vec<String> = vec![];
    for arg in std::env::args().skip(1) {
        match arg.strip_prefix('@') {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .map_err(|e| format!("cannot read argument file {path}: {e}"))?;
                for line in text.lines() {
                    let line = line.split('#').next().unwrap_or("");
                    expanded.extend(line.split_whitespace().map(str::to_string));
                }
            }
            None => expanded.push(arg),
        }
    }
    let mut it = expanded.into_iter();
    let mut web = None;
    let mut a = Args {
        web: PathBuf::new(),
        changes: vec![],
        out_dir: None,
        pool: None,
        emit_pascal: None,
        emit_web: None,
        stat: false,
        debug: false,
        consts: vec![],
        macros: vec![],
        scalars: vec![],
        arena_caps: vec![],
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--change" => a
                .changes
                .push(it.next().ok_or("--change needs a file")?.into()),
            "--out-dir" => a.out_dir = Some(it.next().ok_or("--out-dir needs a value")?.into()),
            "--pool" => a.pool = Some(it.next().ok_or("--pool needs a value")?.into()),
            "--emit-web" => a.emit_web = Some(it.next().ok_or("--emit-web needs a value")?.into()),
            "--emit-pascal" => {
                a.emit_pascal = Some(it.next().ok_or("--emit-pascal needs a value")?.into())
            }
            "--const" => {
                let v = it.next().ok_or("--const needs NAME=VALUE")?;
                let (n, val) = v.split_once('=').ok_or("--const needs NAME=VALUE")?;
                let val: i64 = val
                    .parse()
                    .map_err(|_| format!("bad --const value in {v}"))?;
                a.consts.push((n.to_string(), val));
            }
            "--macro" => {
                let v = it.next().ok_or("--macro needs NAME=VALUE")?;
                let (n, val) = v.split_once('=').ok_or("--macro needs NAME=VALUE")?;
                let val: i64 = val
                    .parse()
                    .map_err(|_| format!("bad --macro value in {v}"))?;
                a.macros.push((n.to_string(), val));
            }
            "--scalar" => {
                let v = it.next().ok_or("--scalar needs NAME=f32|i64")?;
                let (n, k) = v.split_once('=').ok_or("--scalar needs NAME=f32|i64")?;
                let t = match k {
                    "f32" => parse::Ty::Real32,
                    "i64" => parse::Ty::Int64,
                    _ => return Err(format!("--scalar: only `f32` and `i64`, got {k}")),
                };
                a.scalars.push((n.to_string(), t));
            }
            "--arena-cap" => {
                let v = it.next().ok_or("--arena-cap needs NAME=EXPR")?;
                let (n, e) = v.split_once('=').ok_or("--arena-cap needs NAME=EXPR")?;
                a.arena_caps.push((n.to_string(), e.to_string()));
            }
            "--stat" => a.stat = true,
            "--debug" => a.debug = true,
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            s => web = Some(PathBuf::from(s)),
        }
    }
    a.web = web.ok_or("no WEB input file given")?;
    Ok(a)
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("web2rust: {e}");
            return ExitCode::FAILURE;
        }
    };
    // WEB files are 8-bit text; each byte is one `char` (Latin-1).
    let read = |p: &PathBuf| -> Result<String, String> {
        std::fs::read(p)
            .map(|b| b.iter().map(|&c| c as char).collect())
            .map_err(|e| format!("cannot read {}: {e}", p.display()))
    };
    let mut src = match read(&args.web) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("web2rust: {e}");
            return ExitCode::FAILURE;
        }
    };
    for ch in &args.changes {
        let applied = read(ch).and_then(|c| changes::apply(&src, &c, &ch.display().to_string()));
        match applied {
            Ok(s) => src = s,
            Err(e) => {
                eprintln!("web2rust: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    if let Some(p) = &args.emit_web {
        let bytes: Vec<u8> = src.chars().map(|c| c as u32 as u8).collect();
        if let Err(e) = std::fs::write(p, bytes) {
            eprintln!("web2rust: cannot write {}: {e}", p.display());
            return ExitCode::FAILURE;
        }
    }
    let opts = tangle::Options {
        stat: args.stat,
        debug: args.debug,
        macros: args.macros.clone(),
    };
    let t = tangle::tangle(&src, opts);
    eprintln!(
        "web2rust: {} sections, {} tokens, {} pool strings, checksum {}",
        t.n_sections,
        t.tokens.len(),
        t.pool.len(),
        t.checksum
    );

    if let Some(p) = &args.pool {
        if let Err(e) = std::fs::write(p, tangle::write_pool(&t)) {
            eprintln!("web2rust: cannot write {}: {e}", p.display());
            return ExitCode::FAILURE;
        }
    }
    if let Some(p) = &args.emit_pascal {
        let mut out = String::new();
        for tk in &t.tokens {
            out.push_str(&dump_tok(tk));
            out.push('\n');
        }
        if let Err(e) = std::fs::write(p, out) {
            eprintln!("web2rust: cannot write {}: {e}", p.display());
            return ExitCode::FAILURE;
        }
    }

    // Without --out-dir only the tangle stage runs (--pool, --emit-pascal,
    // --emit-web): pdftex.web on its own tangles, but it is not a complete
    // Pascal program until changes/web2c.ch supplies what web2c's tex.ch does.
    if args.out_dir.is_none() {
        return ExitCode::SUCCESS;
    }
    let mut t = t;
    if !args.consts.is_empty() {
        if let Err(e) = tangle::override_consts(&mut t, &args.consts) {
            eprintln!("web2rust: {e}");
            return ExitCode::FAILURE;
        }
    }
    let program = match parse::parse(&t) {
        Ok(mut p) => {
            for (n, t) in &args.scalars {
                if p.type_map.insert(n.clone(), t.clone()).is_none() {
                    eprintln!("web2rust: --scalar: `{n}` is not a declared type");
                    return ExitCode::FAILURE;
                }
                for (tn, ty, _) in p.types.iter_mut() {
                    if tn == n {
                        *ty = t.clone();
                    }
                }
            }
            p
        }
        Err(e) => {
            eprintln!("web2rust: parse error: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "web2rust: parsed {} constants, {} types, {} globals, {} routines, {} main statements",
        program.consts.len(),
        program.types.len(),
        program.globals.len(),
        program.routines.len(),
        program.main.len()
    );
    let Some(out_dir) = args.out_dir else {
        return ExitCode::SUCCESS;
    };
    let sources: Vec<String> = std::iter::once(&args.web)
        .chain(args.changes.iter())
        .map(|p| p.display().to_string())
        .collect();
    if let Err(e) = emit::emit(&program, &t, &out_dir, &sources, &args.arena_caps) {
        eprintln!("web2rust: emit error: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// Normalised one-token-per-line dump, for comparison with a token dump of the
/// `tex.p` produced by Knuth's TANGLE.
fn dump_tok(t: &tok::Tok) -> String {
    use tok::Tok::*;
    match t {
        Id(n) => format!("i {}", n.replace('_', "").to_ascii_lowercase()),
        Int(v) => format!("n {v}"),
        Real(s) => format!("r {s}"),
        Str(s) => format!("s {s}"),
        Op(o) => format!("o {o}"),
        Sec(n) => format!("# {n}"),
        SecEnd => "#end".to_string(),
    }
}
