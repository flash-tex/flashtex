//! A font metric file that is not found: web2c's `open_input` asks kpathsea
//! for it with `must_exist`, so kpathsea runs `mktextfm NAME`, loads what
//! that made, and writes `mktextfm NAME` to `missfont.log` when it made
//! nothing. The port asked without `must_exist`, so it never ran the script:
//! no `missfont.log`, and a font TeX Live makes on the spot (an EC font at a
//! size that has no TFM file) was "not loadable" (flash-tex/flashtex#2514).
//!
//! Expected results are TeX Live 2026's XeTeX 0.999998 (x86_64-linux,
//! 2026-11-15) for the same input with an empty `TEXMFVAR`: `missfont.log`
//! is the one line `mktextfm nosuchfontzz`, and `\font\b=ecrm1300` loads
//! (`\fontname\b` is `ecrm1300`) after mktextfm wrote
//! `fonts/tfm/jknappen/ec/ecrm1300.tfm` under `TEXMFVAR`.
//!
//! The tests need TeX Live (kpathsea's scripts and, for the second,
//! METAFONT and the EC sources), an oracle only; they skip without it unless
//! `FLASHTEX_REQUIRE_TEXLIVE=1`.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Does kpathsea find `name` (a file of the TeX Live tree), or, for a
/// script kpathsea runs, is it on the PATH?
fn has(name: &str) -> bool {
    let (program, arg) = if name.starts_with("mktex") {
        (name, "--version")
    } else {
        ("kpsewhich", name)
    };
    Command::new(program)
        .arg(arg)
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Is there a TeX Live with what `what` names? (Skips the test otherwise.)
fn texlive_with(what: &[&str]) -> bool {
    let missing: Vec<_> = what.iter().filter(|n| !has(n)).collect();
    if missing.is_empty() {
        return true;
    }
    if std::env::var_os("FLASHTEX_REQUIRE_TEXLIVE").is_some_and(|v| v == "1") {
        panic!("TeX Live lacks {missing:?} with FLASHTEX_REQUIRE_TEXLIVE=1");
    }
    println!("skipped: TeX Live with {missing:?} not found (TeX Live is an oracle only)");
    false
}

fn run_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flashtex-xetex-mktextfm-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("var")).unwrap();
    d
}

/// `flashtex-xetex -ini` on `input` in `dir`, with `dir/var` as TEXMFVAR
/// (where mktextfm writes), like `xetex -ini -etex -no-pdf`.
fn run(dir: &Path, input: &str) -> String {
    std::fs::write(dir.join("f.tex"), input).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("TEXMFVAR", dir.join("var"))
        .env_remove("TEXMFOUTPUT")
        .args([
            "-ini",
            "-etex",
            "-no-pdf",
            "-interaction=nonstopmode",
            "f.tex",
        ])
        .output()
        .expect("run flashtex-xetex");
    assert!(
        out.status.code().is_some(),
        "the engine was killed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(dir.join("f.log")).unwrap()
}

#[test]
fn a_font_that_cannot_be_made_is_noted_in_missfont_log() {
    if !texlive_with(&["cmr10.tfm", "mktextfm"]) {
        return;
    }
    let dir = run_dir("miss");
    let log = run(&dir, "\\font\\a=nosuchfontzz \\end\n");
    // (the log breaks the message at max_print_line)
    assert!(
        log.replace('\n', "").contains(
            "! Font \\a=nosuchfontzz not loadable: Metric (TFM) file or installed font not found."
        ),
        "log:\n{log}"
    );
    let miss = std::fs::read_to_string(dir.join("missfont.log"))
        .unwrap_or_else(|e| panic!("no missfont.log ({e}); log:\n{log}"));
    assert_eq!(miss, "mktextfm nosuchfontzz\n");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_font_mktextfm_can_make_is_loaded() {
    if !texlive_with(&["cmr10.tfm", "mktextfm", "ecrm.mf", "exbase.mf"]) {
        return;
    }
    let dir = run_dir("make");
    let log = run(
        &dir,
        "\\catcode`\\{=1 \\catcode`\\}=2 \\font\\b=ecrm1300 \\message{[\\fontname\\b]}\\end\n",
    );
    assert!(log.contains("[ecrm1300]"), "log:\n{log}");
    assert!(!log.contains("not loadable"), "log:\n{log}");
    assert!(
        dir.join("var/fonts/tfm/jknappen/ec/ecrm1300.tfm").is_file(),
        "mktextfm made no TFM file under TEXMFVAR; log:\n{log}"
    );
    assert!(!dir.join("missfont.log").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
