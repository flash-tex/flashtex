//! The format cache against the installed TeX Live: `pdflatex.fmt` built on
//! first use from the user's TeX Live must behave exactly as one built by
//! hand with fmtutil's command line. A document is run with each (the cached
//! one found through the cache, the other through `FLASHTEX_FORMATS`), with
//! `\tracingall` and box dumps, and the logs and PDFs must be byte-identical.
//! Skips where there is no TeX Live (e.g. CI).
mod common;

#![cfg(feature = "distribution")]

use flashtex_engine::resolver::discover_texlive;
use std::path::Path;
use std::process::{Command, Stdio};

const DOC: &str = "\\documentclass{article}\n\\usepackage{amsmath}\n\
\\showboxdepth=2147483647 \\showboxbreadth=2147483647 \\tracingonline=1\n\
\\begin{document}\\tracingall\nHello, $a^2+b^2=c^2$.\n\\section{One}\nText.\\end{document}\n";

fn run(bin: &Path, dir: &Path, env: &[(&str, &Path)]) -> (String, Vec<u8>) {
    let mut c = Command::new(bin);
    c.args(["-fmt=pdflatex", "-interaction=nonstopmode", "main.tex"])
        .current_dir(dir)
        .env(
            "FLASHTEX_POOL",
            concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool"),
        )
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("FLASHTEX_FORMATS")
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    for (k, v) in env {
        c.env(k, v);
    }
    assert!(c.status().unwrap().success());
    (
        std::fs::read_to_string(dir.join("main.log")).unwrap(),
        std::fs::read(dir.join("main.pdf")).unwrap(),
    )
}

#[test]
fn cached_format_matches_a_hand_built_one() {
    if discover_texlive().is_none() {
        common::no_texlive();
        return;
    }
    let d = std::env::temp_dir().join(format!("flashtex-fmtcache-tl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    for s in ["bin", "hand", "cache", "a", "b", "c"] {
        std::fs::create_dir_all(d.join(s)).unwrap();
    }
    // argv[0] `pdftex`, as the harnesses run it.
    let bin = d.join("bin/pdftex");
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_flashtex-initex"), &bin).unwrap();
    // By hand: fmtutil.cnf's pdflatex line, fmtutil's command line.
    let st = Command::new(&bin)
        .args([
            "-ini",
            "-jobname=pdflatex",
            "-progname=pdflatex",
            "-translate-file=cp227.tcx",
            "*pdflatex.ini",
        ])
        .current_dir(d.join("hand"))
        .env(
            "FLASHTEX_POOL",
            concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool"),
        )
        .env("FLASHTEX_FORMAT_CACHE", "off")
        // As for the runs, so that the format dates agree (fmtutil passes
        // the environment through, and so does the cache).
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(st.success());
    for s in ["a", "b", "c"] {
        std::fs::write(d.join(s).join("main.tex"), DOC).unwrap();
    }
    let (log_a, pdf_a) = run(&bin, &d.join("a"), &[("FLASHTEX_FORMATS", &d.join("hand"))]);
    let (log_b, pdf_b) = run(
        &bin,
        &d.join("b"),
        &[("FLASHTEX_FORMAT_CACHE_DIR", &d.join("cache"))],
    );
    // The second run of the cache is a hit and must still agree.
    let (log_c, pdf_c) = run(
        &bin,
        &d.join("c"),
        &[("FLASHTEX_FORMAT_CACHE_DIR", &d.join("cache"))],
    );
    assert!(log_a.contains("Completed box being shipped out"));
    assert_eq!(
        log_a.replace("/a/", "/b/"),
        log_b,
        "log with the cached format differs"
    );
    assert_eq!(log_b.replace("/b/", "/c/"), log_c);
    assert_eq!(pdf_a, pdf_b, "PDF with the cached format differs");
    assert_eq!(pdf_b, pdf_c);
    let _ = std::fs::remove_dir_all(&d);
}
