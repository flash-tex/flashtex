//! web2rust — translate the WEB Pascal subset (Knuth's `tex.web`) to Rust.
//!
//! Usage:
//!   web2rust <tex.web> --out-dir <dir> [--pool <file>] [--stat] [--emit-pascal <file>]
//!
//! See `tools/web2rust/README.md` for the regeneration command that the
//! generated crate is committed with.

// mod emit;
// mod parse;
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
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out-dir" => a.out_dir = Some(it.next().ok_or("--out-dir needs a value")?.into()),
            "--pool" => a.pool = Some(it.next().ok_or("--pool needs a value")?.into()),
            "--emit-pascal" => {
                a.emit_pascal = Some(it.next().ok_or("--emit-pascal needs a value")?.into())
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
    let opts = tangle::Options { stat: args.stat, debug: args.debug };
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

    let _ = args.out_dir;
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
