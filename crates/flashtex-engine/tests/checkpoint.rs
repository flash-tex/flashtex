//! The checkpoint layer and the resident host against the uninterrupted run
//! (DESIGN.md §5.1, §5.2), on the installed TeX Live's LaTeX. Skips where
//! there is no TeX Live (e.g. CI).
//!
//! * `selftest`: with a checkpoint at S₀ and after every shipout, restoring
//!   any of them and running on gives a bit-identical engine state at every
//!   later checkpoint and at the end, and byte-identical files and terminal
//!   (flashtex-host's `selftest`, which also covers `redo_to` and
//!   `restore_discard`).
//! * `from_s0_equals_a_full_run`: after an edit to the body, a compile that
//!   restores S₀ writes the same PDF, log and aux as `flashtex-initex` run
//!   in full on the edited source.
//! * `persisted_s0_reopens`: S₀ saved to disk and opened in a fresh process
//!   gives the same files again.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DOC: &str = r"\documentclass{article}
\usepackage{hyperref}
\begin{document}
\section{One}\label{one}
See section~\ref{two} on page~\pageref{two}.\footnote{A note.}
\input{body}
\section{Two}\label{two}
Back to section~\ref{one}.
\end{document}
";

fn body(word: &str) -> String {
    let mut s = String::new();
    for i in 0..60 {
        s.push_str(&format!(
            "Paragraph {i} of the body, with the word {word} and enough text to fill \
             a few lines of the page so that several pages are shipped out. \
             Math: $a^{i} + b_{{{i}}} = \\sqrt{{c}}$.\n\n"
        ));
    }
    s
}

struct Env {
    dir: PathBuf,
    fmt: PathBuf,
    pool: PathBuf,
}

/// A pdflatex format made by this engine, once per test binary (the tests
/// run in parallel threads; the first one makes it).
fn env() -> Option<Env> {
    static MADE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    find_texlive_bin()?;
    let _once = MADE.lock().unwrap_or_else(|p| p.into_inner());
    let initex = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let base = std::env::temp_dir().join(format!("flashtex-ckpt-{}", std::process::id()));
    let fmt = base.join("fmt");
    std::fs::create_dir_all(&fmt).unwrap();
    let pdftex = fmt.join("pdftex");
    let _ = std::os::unix::fs::symlink(initex, &pdftex);
    if !fmt.join("pdflatex.fmt").exists() {
        let st = Command::new(&pdftex)
            .args([
                "-ini",
                "-jobname=pdflatex",
                "-progname=pdflatex",
                "-translate-file=cp227.tcx",
                "*pdflatex.ini",
            ])
            .current_dir(&fmt)
            .env("FLASHTEX_POOL", &pool)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(st.success() && fmt.join("pdflatex.fmt").exists());
    }
    Some(Env {
        dir: base,
        fmt,
        pool,
    })
}

fn job_dir(e: &Env, name: &str) -> PathBuf {
    let d = e.dir.join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("doc.tex"), DOC).unwrap();
    std::fs::write(d.join("body.tex"), body("alpha")).unwrap();
    d
}

fn run(e: &Env, dir: &Path, bin: &str, args: &[&str]) -> (bool, String) {
    let exe = match bin {
        "host" => PathBuf::from(env!("CARGO_BIN_EXE_flashtex-host")),
        _ => e.fmt.join("pdftex"),
    };
    let out = Command::new(exe)
        .args(args)
        .current_dir(dir)
        .env("FLASHTEX_POOL", &e.pool)
        .env("FLASHTEX_FORMATS", &e.fmt)
        .env("SOURCE_DATE_EPOCH", "1700000000")
        .env("FORCE_SOURCE_DATE", "1")
        // Both engines seed \pdfuniformdeviate from the same pinned clock.
        .env("FLASHTEX_PIN_CLOCK", "1700000000.123456")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

const PDFTEX_ARGS: &[&str] = &["-fmt=pdflatex", "-interaction=nonstopmode", "doc.tex"];

fn host(e: &Env, dir: &Path, cmd: &[&str]) -> (bool, String) {
    let mut a: Vec<&str> = cmd.to_vec();
    a.push("--");
    a.extend_from_slice(PDFTEX_ARGS);
    run(e, dir, "host", &a)
}

fn same_files(a: &Path, b: &Path) {
    for ext in ["pdf", "log", "aux"] {
        let (x, y) = (
            std::fs::read(a.join(format!("doc.{ext}"))).unwrap(),
            std::fs::read(b.join(format!("doc.{ext}"))).unwrap(),
        );
        assert!(
            x == y,
            "doc.{ext} differs ({} vs {} bytes)",
            x.len(),
            y.len()
        );
    }
}

#[test]
fn selftest() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let d = job_dir(&e, "selftest");
    // A first run writes the .aux the measured run reads.
    run(&e, &d, "cli", PDFTEX_ARGS);
    let (ok, out) = host(&e, &d, &["selftest", "--max-targets", "4"]);
    assert!(ok, "selftest failed:\n{out}");
    assert!(out.contains("\"failures\":0"), "{out}");
}

#[test]
fn from_s0_equals_a_full_run() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let a = job_dir(&e, "l1-host");
    let b = job_dir(&e, "l1-cli");
    // Host: cold (no .aux yet), cold again (the .aux appeared), then from
    // S₀ after each of two edits of the body.
    let (ok, out) = host(&e, &a, &["bench", "--reps", "3", "--edit", "body.tex"]);
    assert!(ok, "{out}");
    let modes: Vec<&str> = out
        .lines()
        .filter_map(|l| l.split("\"mode\":\"").nth(1))
        .map(|m| m.split('"').next().unwrap())
        .collect();
    assert_eq!(modes, ["cold", "cold-after-invalid", "s0", "s0"], "{out}");
    // The CLI on the same final source, run twice as a user would (the
    // labels are stable, so the second run reads the .aux the host's last
    // compile read).
    std::fs::copy(a.join("body.tex"), b.join("body.tex")).unwrap();
    run(&e, &b, "cli", PDFTEX_ARGS);
    run(&e, &b, "cli", PDFTEX_ARGS);
    same_files(&a, &b);
}

#[test]
fn persisted_s0_reopens() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let a = job_dir(&e, "reopen");
    let s0 = a.join("doc.s0");
    let s0 = s0.to_str().unwrap();
    let (ok, out) = host(&e, &a, &["bench", "--reps", "1", "--save", s0]);
    assert!(ok, "{out}");
    let before: Vec<Vec<u8>> = ["pdf", "log", "aux"]
        .iter()
        .map(|x| std::fs::read(a.join(format!("doc.{x}"))).unwrap())
        .collect();
    let (ok, out) = host(&e, &a, &["open", s0]);
    assert!(ok, "{out}");
    assert!(out.contains("\"mode\":\"s0\""), "{out}");
    for (i, x) in ["pdf", "log", "aux"].iter().enumerate() {
        assert!(
            std::fs::read(a.join(format!("doc.{x}"))).unwrap() == before[i],
            "doc.{x} differs after reopening S0"
        );
    }
}
