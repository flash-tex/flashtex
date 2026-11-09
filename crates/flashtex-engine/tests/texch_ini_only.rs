//! tex.ch's `Init`...`Tini` (`@d Init==init if ini_version then begin`):
//! code that pdftex.web compiles only into INITEX runs, in web2c's single
//! binary, only when `-ini` was given. In a production run (a format
//! loaded) `\patterns` is the error `Patterns can be loaded only by INITEX`
//! (tex.ch \[49.1252\]) and `\dump` prints `(\dump is performed only by
//! INITEX)` and writes no format (tex.ch \[51.1335\]). Found by
//! tools/fuzz-parity (#2253).
//!
//! Each engine makes a small format with `-ini` and then runs on it without
//! `-ini`; terminal output (after the banner), the log (after the banner),
//! the exit status and the format file left behind must be pdfTeX's. The
//! lines pdfTeX printed (TeX Live 2026, pdfTeX 1.40.29, macOS arm64) are
//! checked as well, so the test also fails if both engines change. Skips
//! where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const MK: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\dump\n";
const RUN: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\
\\patterns{a1b}\n\
\\message{after}\n\
\\dump\n";

#[derive(Debug, PartialEq)]
struct Out {
    code: Option<i32>,
    terminal: String,
    log: String,
    dumped: bool,
}

fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

fn engine(bin: &Path, dir: &Path, ours: bool) -> Command {
    let mut c = Command::new(bin);
    c.current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .env("FLASHTEX_FORMATS", dir);
    }
    c
}

fn run(bin: &Path, dir: &Path, ours: bool) -> Out {
    std::fs::write(dir.join("mk.tex"), MK).unwrap();
    std::fs::write(dir.join("t.tex"), RUN).unwrap();
    let mk = engine(bin, dir, ours)
        .args(["-ini", "-jobname=mini", "mk.tex"])
        .output()
        .unwrap();
    assert!(
        mk.status.success() && dir.join("mini.fmt").exists(),
        "{}: -ini did not make mini.fmt",
        bin.display()
    );
    let out = engine(bin, dir, ours)
        .args(["-fmt=mini", "-interaction=nonstopmode", "t.tex"])
        .output()
        .unwrap();
    Out {
        code: out.status.code(),
        terminal: after_banner(&String::from_utf8_lossy(&out.stdout)),
        log: after_banner(&std::fs::read_to_string(dir.join("t.log")).unwrap()),
        dumped: dir.join("t.fmt").exists(),
    }
}

#[test]
fn patterns_and_dump_are_initex_only() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-texch-ini-only");
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
    }
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    let x = run(&ours, &a, true);
    let y = run(&texbin.join("pdftex"), &b, false);
    for line in [
        "! Patterns can be loaded only by INITEX.",
        "(\\dump is performed only by INITEX)",
    ] {
        assert!(
            y.log.contains(line),
            "pdftex's log lacks {line:?}:\n{}",
            y.log
        );
    }
    assert!(!y.dumped, "pdftex wrote t.fmt in a production run");
    assert_eq!(x, y, "production-run \\patterns/\\dump differ from pdftex");
    let _ = std::fs::remove_dir_all(&base);
}
