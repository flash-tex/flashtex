//! The installed TeX Live's LaTeX, end to end: build pdflatex.fmt from its
//! `pdflatex.ini` and `latex.ltx` with this engine and with TeX Live's own
//! `pdftex` (the fmtutil.cnf command line), run `\documentclass{article}` with
//! "Hello" on each, and compare both logs line by line. Skips where there is
//! no TeX Live (e.g. CI).
//!
//! The lines allowed to differ, each for a reason that is not typesetting
//! (crates/flashtex-engine/changes/README.md lists what is not re-specified):
//!
//! * the banner (web2c's "(TeX Live 2026)");
//! * accounting, as DESIGN.md section 1.1 normalises it for P-T1: the format
//!   dump's string and memory counts and its hyphenation-exception count, and
//!   the end-of-run statistics blocks;
//! * the PDF writer's own lines, which P3 ports: the font map file
//!   `{.../pdftex.map}`, embedded font files `<...pfb>`, and the byte count.
mod common;

#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

fn comparable(log: &str) -> Vec<String> {
    let mut out = vec![];
    let mut in_stats = false;
    for l in log.lines().skip(1) {
        if l.starts_with("Here is how much of TeX's memory you used:")
            || l.starts_with("PDF statistics:")
        {
            in_stats = true;
            continue;
        }
        if in_stats {
            if l.starts_with(' ') || l.starts_with('<') {
                continue;
            }
            in_stats = false;
        }
        if l.contains("strings of total length")
            || l.contains("memory locations dumped; current usage is")
            || l.ends_with(" hyphenation exceptions")
        {
            continue;
        }
        // The PDF writer's lines (P3): the map file and embedded fonts, and
        // the byte count of "Output written on hello.pdf (1 page, N bytes).".
        let l = match (l.find('{'), l.find("pdftex.map}")) {
            (Some(i), Some(j)) if i < j => format!("{}{}", &l[..i], &l[j + "pdftex.map}".len()..]),
            _ => l.to_string(),
        };
        let l = match l.find(" page, ").or_else(|| l.find(" pages, ")) {
            Some(i) if l.starts_with("Output written on ") => l[..i].to_string(),
            _ => l,
        };
        out.push(l);
    }
    out
}

#[test]
fn latex_format_and_hello_match_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let base = std::env::temp_dir().join(format!("flashtex-latex-{}", std::process::id()));
    let (a, b) = (base.join("ours"), base.join("texlive"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(
            d.join("hello.tex"),
            "\\documentclass{article}\n\\begin{document}\nHello\n\\end{document}\n",
        )
        .unwrap();
    }
    let run = |dir: &Path, cmd: &mut Command| {
        let st = cmd
            .current_dir(dir)
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FORCE_SOURCE_DATE", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(st.success(), "{cmd:?} failed in {}", dir.display());
    };
    run(
        &a,
        Command::new(ours)
            .args([
                "-ini",
                "-jobname=pdflatex",
                "-progname=pdflatex",
                "-etex",
                "-translate-file=cp227.tcx",
                "pdflatex.ini",
            ])
            .env("FLASHTEX_POOL", &pool),
    );
    run(
        &b,
        Command::new(texbin.join("pdftex")).args([
            "-ini",
            "-jobname=pdflatex",
            "-progname=pdflatex",
            "-etex",
            "-translate-file=cp227.tcx",
            "pdflatex.ini",
        ]),
    );
    run(
        &a,
        Command::new(ours)
            .args(["-progname=pdflatex", "hello"])
            .env("FLASHTEX_POOL", &pool),
    );
    run(
        &b,
        Command::new(texbin.join("pdftex")).args(["-fmt=pdflatex", "-progname=pdflatex", "hello"]),
    );
    for log in ["pdflatex.log", "hello.log"] {
        let x = std::fs::read_to_string(a.join(log)).unwrap();
        let y = std::fs::read_to_string(b.join(log)).unwrap();
        assert_eq!(
            comparable(&x),
            comparable(&y),
            "{log} differs from TeX Live's pdflatex"
        );
    }
    let _ = std::fs::remove_dir_all(&base);
}
