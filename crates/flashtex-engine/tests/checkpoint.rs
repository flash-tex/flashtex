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
    // one directory of this process's own (common::fresh_dir), made once
    static BASE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    let base = BASE
        .get_or_init(|| common::fresh_dir("flashtex-ckpt"))
        .clone();
    let fmt = base.join("fmt");
    std::fs::create_dir_all(&fmt).unwrap();
    let pdftex = fmt.join("pdftex");
    common::link_engine(initex, &pdftex);
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
    // Unix: S0's pages are mapped into the word space, not copied
    if cfg!(unix) {
        assert!(
            out.contains("\"mapped_pages\":") && !out.contains("\"mapped_pages\":0"),
            "{out}"
        );
    }
    for (i, x) in ["pdf", "log", "aux"].iter().enumerate() {
        assert!(
            std::fs::read(a.join(format!("doc.{x}"))).unwrap() == before[i],
            "doc.{x} differs after reopening S0"
        );
    }
}

/// #2058: S₀ stored under one configuration is not opened under another,
/// with the same first line: another switch (F1), another job name (F2),
/// an environment variable kpathsea reads for a setting (`max_print_line`,
/// which a run takes at its start and S₀ holds) or for a search path
/// (`TEXINPUTS`, which a lookup in the preamble depends on), and a
/// database (`ls-R`) of a tree a lookup in the preamble found nothing in,
/// with the file added (F4). Without `Key::check_config` the open restored
/// S₀ for the switch, the job name and `max_print_line`, as the control
/// with nothing changed does (VERIFIED, PC, 2026-11-08); the `TEXINPUTS`
/// and `ls-R` cases were refused there by the lookups made again ("finds
/// another file now"), and stay as guards of the key's own check.
#[test]
fn a_persisted_s0_is_not_opened_under_another_configuration() {
    let Some(e) = env() else {
        common::no_texlive();
        return;
    };
    let a = job_dir(&e, "reopen-config");
    std::fs::write(
        a.join("doc.tex"),
        "\\documentclass{article}\n\
         \\IfFileExists{zzalt.sty}{\\def\\alt{found}}{\\def\\alt{missing}}\n\
         \\IfFileExists{zzlocal.sty}{\\def\\local{found}}{\\def\\local{missing}}\n\
         \\begin{document}\nThe alternative is \\alt; the local one is \\local.\n\
         \\input{body}\n\\end{document}\n",
    )
    .unwrap();
    let alt = a.join("alt");
    std::fs::create_dir_all(&alt).unwrap();
    std::fs::write(alt.join("zzalt.sty"), "\\endinput\n").unwrap();
    let tree = e.dir.join("reopen-config-texmf");
    let _ = std::fs::remove_dir_all(&tree);
    std::fs::create_dir_all(tree.join("tex/latex")).unwrap();
    let ls_r = |files: &str| {
        format!(
            "% ls-R -- filename database for kpathsea; do not change this line.\n\
             ./:\ntex\n\n./tex:\nlatex\n\n./tex/latex:\n{files}"
        )
    };
    std::fs::write(tree.join("ls-R"), ls_r("")).unwrap();
    let tree_s = tree.to_str().unwrap().to_string();
    let s0 = a.join("doc.s0");
    let s0 = s0.to_str().unwrap().to_string();
    let host_with = |pdftex: &[&str], cmd: &[&str], extra: &[(&str, &str)]| {
        let mut args: Vec<&str> = cmd.to_vec();
        args.push("--");
        args.extend_from_slice(pdftex);
        let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-host"));
        c.args(&args)
            .current_dir(&a)
            .env("FLASHTEX_POOL", &e.pool)
            .env("FLASHTEX_FORMATS", &e.fmt)
            .env("SOURCE_DATE_EPOCH", "1700000000")
            .env("FORCE_SOURCE_DATE", "1")
            .env("FLASHTEX_PIN_CLOCK", "1700000000.123456")
            .env("TEXMFLOCAL", &tree_s)
            .stdin(Stdio::null());
        for (k, v) in extra {
            c.env(k, v);
        }
        let out = c.output().unwrap();
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    // a run first (the next reads its .aux, and the working directory's
    // listing settles), then S0 saved
    let (ok, out) = host_with(PDFTEX_ARGS, &["bench", "--reps", "1"], &[]);
    assert!(ok, "{out}");
    let (ok, out) = host_with(PDFTEX_ARGS, &["bench", "--reps", "1", "--save", &s0], &[]);
    assert!(ok, "{out}");
    let (ok, out) = host_with(PDFTEX_ARGS, &["open", &s0], &[]);
    assert!(ok && out.contains("\"mode\":\"s0\""), "the control: {out}");
    let batch: &[&str] = &["-fmt=pdflatex", "-interaction=batchmode", "doc.tex"];
    let jobname: &[&str] = &[
        "-fmt=pdflatex",
        "-interaction=nonstopmode",
        "-jobname=other",
        "doc.tex",
    ];
    // (what, the command line, the environment added, what the refusal says)
    type Case<'a> = (&'a str, &'a [&'a str], Vec<(&'a str, &'a str)>, &'a str);
    let cases: [Case; 4] = [
        (
            "another switch",
            batch,
            vec![],
            "configuration changed (run)",
        ),
        (
            "another job name",
            jobname,
            vec![],
            "configuration changed (run)",
        ),
        (
            "max_print_line",
            PDFTEX_ARGS,
            vec![("max_print_line", "100")],
            "variable max_print_line changed",
        ),
        (
            "TEXINPUTS",
            PDFTEX_ARGS,
            vec![("TEXINPUTS", "./alt:")],
            "variable TEXINPUTS changed",
        ),
    ];
    // (every case, then one assertion: which ones failed, and how)
    let said = |out: &str| {
        out.lines()
            .find(|l| l.contains("\"mode\"") || l.contains("\"error\""))
            .unwrap_or("")
            .to_string()
    };
    let mut bad = vec![];
    for (what, pdftex, extra, why) in cases {
        let (ok, out) = host_with(pdftex, &["open", &s0], &extra);
        if ok || !out.contains(why) {
            bad.push(format!("{what}: {}", said(&out)));
        }
    }
    // a package installed into the tree, its ls-R updated
    std::fs::write(tree.join("tex/latex/zzlocal.sty"), "\\endinput\n").unwrap();
    std::fs::write(tree.join("ls-R"), ls_r("zzlocal.sty\n")).unwrap();
    let (ok, out) = host_with(PDFTEX_ARGS, &["open", &s0], &[]);
    if ok || !out.contains("ls-R, of kpathsea's configuration, changed") {
        bad.push(format!("ls-R: {}", said(&out)));
    }
    assert!(
        bad.is_empty(),
        "the stored S0 was opened, or refused for another reason:\n{}",
        bad.join("\n")
    );
}
