//! A `TEXINPUTS` with no default path (`./sub:./`, no trailing `:`), against
//! TeX Live's own `pdftex` (#2310, lane AUDIT-KPSE).
//!
//! pdfTeX finds its format through `TEXFORMATS`. fmtutil built it without
//! the document's `TEXINPUTS`, so the job compiles. The format cache used to
//! repeat its lookups through the overridden path, find the format stale,
//! and rebuild it with the same `TEXINPUTS`. INITEX then could not find
//! `pdfetex.ini`, and the job ended with "I can't find the format file".
//!
//! The job's exit status and its log from the `**` line on must agree with
//! pdfTeX's, on a cold cache and on a warm one. The format must be built
//! once: the run without the override is a cache hit too. Skips where there
//! is no TeX Live.
#![cfg(all(feature = "distribution", feature = "kpathsea"))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// Exit status and stderr of a job in `dir`.
fn run(bin: &Path, dir: &Path, texinputs: Option<&str>, cache: &Path) -> (Option<i32>, String) {
    let mut c = Command::new(bin);
    c.args(["-interaction=nonstopmode", "main.tex"])
        .current_dir(dir)
        .env(
            "FLASHTEX_POOL",
            concat!(env!("CARGO_MANIFEST_DIR"), "/pdftex.pool"),
        )
        .env("FLASHTEX_FORMAT_CACHE_DIR", cache)
        .env("FLASHTEX_DEBUG_FORMATS", "1")
        .env_remove("FLASHTEX_FORMATS")
        .env_remove("FLASHTEX_FORMAT_CACHE")
        .env_remove("TEXINPUTS")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(t) = texinputs {
        c.env("TEXINPUTS", t);
    }
    let out = c.output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The log from the `**` line on.
fn log_tail(dir: &Path) -> String {
    let log = std::fs::read_to_string(dir.join("main.log")).unwrap_or_default();
    let from = log.find("\n**").map_or(log.len(), |i| i + 1);
    log[from..].to_string()
}

#[test]
fn texinputs_without_default_path_compiles_as_in_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let d = common::fresh_dir("flashtex-fmt-texinputs");
    let _ = std::fs::remove_dir_all(&d);
    for s in ["bin", "cache", "tex/sub", "ours/sub"] {
        std::fs::create_dir_all(d.join(s)).unwrap();
    }
    // argv[0] `pdftex`: the pdftex format, built by the cache.
    let bin = d.join("bin").join(flashtex_engine::os::exe_name("pdftex"));
    flashtex_engine::os::link_executable(env!("CARGO_BIN_EXE_flashtex-initex").as_ref(), &bin)
        .unwrap();
    for s in ["tex", "ours"] {
        std::fs::write(d.join(s).join("sub/y.tex"), "\\message{Y}").unwrap();
        std::fs::write(d.join(s).join("main.tex"), "\\input y \\bye\n").unwrap();
    }
    let cache = d.join("cache");
    let theirs = run(
        &texbin.join("pdftex"),
        &d.join("tex"),
        Some("./sub:./"),
        &cache,
    );
    let want = log_tail(&d.join("tex"));
    assert_eq!(theirs.0, Some(0), "TeX Live's pdftex: {}", theirs.1);
    assert!(want.contains("(./sub/y.tex Y)"), "TeX Live's log: {want}");
    // Cold cache, then warm, with the override; then warm without it.
    for (pass, texinputs, how) in [
        (1, Some("./sub:./"), "built"),
        (2, Some("./sub:./"), "cache hit"),
        (3, None, "cache hit"),
    ] {
        let _ = std::fs::remove_file(d.join("ours/main.log"));
        let ours = run(&bin, &d.join("ours"), texinputs, &cache);
        // (without the override `y` is not found: only the cache is checked)
        if texinputs.is_some() {
            assert_eq!(
                ours.0, theirs.0,
                "pass {pass}: exit status; stderr:\n{}",
                ours.1
            );
            assert_eq!(log_tail(&d.join("ours")), want, "pass {pass}: log");
        }
        assert!(
            ours.1.contains(&format!("[formats] pdftex: {how}")),
            "pass {pass}: want {how}; stderr:\n{}",
            ours.1
        );
    }
    let _ = std::fs::remove_dir_all(&d);
}
