//! web2rust — translate the WEB Pascal subset (Knuth's `tex.web`) to Rust.
//!
//! Usage:
//!   web2rust <tex.web> --out-dir <dir> [--pool <file>] [--stat]
//!            [--const NAME=VALUE ...] [--emit-pascal <file>]
//!
//! See `tools/web2rust/README.md` for the regeneration command that the
//! generated crate is committed with.

mod emit;
mod parse;
mod tangle;
mod tok;

use std::path::PathBuf;
use std::process::ExitCode;

struct Args {
    web: PathBuf,
    out_dir: Option<PathBuf>,
    pool: Option<PathBuf>,
    emit_pascal: Option<PathBuf>,
    stat: bool,
    debug: bool,
    /// `--const NAME=VALUE`: override an outer-block constant. This is the job
    /// web2c splits between `tex.ch` and `texmf.cnf`; `tex.web` itself must stay
    /// unmodified (third_party/knuth/README.md).
    consts: Vec<(String, i64)>,
    /// `--macro NAME=VALUE`: override a WEB numeric macro (`@d name=value`).
    macros: Vec<(String, i64)>,
    /// `--scalar NAME=f32`: narrow a named `real` type, which is what web2c
    /// does to `glue_ratio`.
    scalars: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let mut web = None;
    let mut a = Args {
        web: PathBuf::new(),
        out_dir: None,
        pool: None,
        emit_pascal: None,
        stat: false,
        debug: false,
        consts: vec![],
        macros: vec![],
        scalars: vec![],
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out-dir" => a.out_dir = Some(it.next().ok_or("--out-dir needs a value")?.into()),
            "--pool" => a.pool = Some(it.next().ok_or("--pool needs a value")?.into()),
            "--emit-pascal" => {
                a.emit_pascal = Some(it.next().ok_or("--emit-pascal needs a value")?.into())
            }
            "--const" => {
                let v = it.next().ok_or("--const needs NAME=VALUE")?;
                let (n, val) = v.split_once('=').ok_or("--const needs NAME=VALUE")?;
                let val: i64 = val.parse().map_err(|_| format!("bad --const value in {v}"))?;
                a.consts.push((n.to_string(), val));
            }
            "--macro" => {
                let v = it.next().ok_or("--macro needs NAME=VALUE")?;
                let (n, val) = v.split_once('=').ok_or("--macro needs NAME=VALUE")?;
                let val: i64 = val.parse().map_err(|_| format!("bad --macro value in {v}"))?;
                a.macros.push((n.to_string(), val));
            }
            "--scalar" => {
                let v = it.next().ok_or("--scalar needs NAME=f32")?;
                let (n, k) = v.split_once('=').ok_or("--scalar needs NAME=f32")?;
                if k != "f32" {
                    return Err(format!("--scalar: only `f32` is supported, got {k}"));
                }
                a.scalars.push(n.to_string());
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
    let src = match std::fs::read(&args.web) {
        Ok(b) => b.iter().map(|&c| c as char).collect::<String>(),
        Err(e) => {
            eprintln!("web2rust: cannot read {}: {e}", args.web.display());
            return ExitCode::FAILURE;
        }
    };
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

    let mut t = t;
    if !args.consts.is_empty() {
        if let Err(e) = tangle::override_consts(&mut t, &args.consts) {
            eprintln!("web2rust: {e}");
            return ExitCode::FAILURE;
        }
    }
    let program = match parse::parse(&t) {
        Ok(mut p) => {
            for n in &args.scalars {
                if p.type_map.insert(n.clone(), parse::Ty::Real32).is_none() {
                    eprintln!("web2rust: --scalar: `{n}` is not a declared type");
                    return ExitCode::FAILURE;
                }
                for (tn, ty, _) in p.types.iter_mut() {
                    if tn == n {
                        *ty = parse::Ty::Real32;
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
    if let Err(e) = emit::emit(&program, &t, &out_dir) {
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
