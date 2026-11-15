//! utils.c's `setjobid` writes the date part of the job id with
//! `"%.4d/%.2d/%.2d %.2d:%.2d"` of `\year`, `\month`, `\day`, `\time / 60`
//! and `\time % 60`, and `\pdfuniqueresname`'s prefix is the job id's CRC-32.
//! C's precision puts a sign in front of the padded digits (`-0003`), where
//! Rust's `{:04}` counts it in the width (`-003`), so a negative date gave
//! other resource names (#2270). Run side by side with TeX Live's pdftex on
//! the same INITEX input: exit status, the log after its banner line and the
//! PDF, byte for byte. Found by lane AUDIT-PDFC. Skips where there is no TeX
//! Live (e.g. CI without it).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// What one run left: exit status, the log after its first line, the PDF.
#[derive(Debug, PartialEq)]
struct Outcome {
    code: Option<i32>,
    log: String,
    pdf: Option<Vec<u8>>,
}

fn run(bin: &Path, dir: &Path, doc: &[u8]) -> Outcome {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("doc.tex"), doc).unwrap();
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs = vec![bin.to_path_buf()];
    dirs.extend(std::env::split_paths(&path));
    let out = Command::new("sh")
        .args([
            "-c",
            "exec pdftex -ini -interaction=nonstopmode doc.tex 2>&1",
        ])
        .current_dir(dir)
        .env("PATH", std::env::join_paths(dirs).unwrap())
        .env("SOURCE_DATE_EPOCH", "1700000000")
        .env("FORCE_SOURCE_DATE", "1")
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let log = std::fs::read_to_string(dir.join("doc.log")).unwrap_or_default();
    Outcome {
        code: out.status.code(),
        log: log.split_once('\n').map_or("", |(_, t)| t).to_string(),
        pdf: std::fs::read(dir.join("doc.pdf")).ok(),
    }
}

#[test]
fn unique_resource_names_follow_setjobids_printf() {
    let Some(texlive) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-jobid");
    let ours = base.join("bin");
    std::fs::create_dir_all(&ours).unwrap();
    common::link_engine(
        Path::new(env!("CARGO_BIN_EXE_flashtex-initex")),
        &ours.join("pdftex"),
    );
    for (tag, date) in [
        ("negative", "\\year=-3 \\time=-5 \\month=-1 \\day=-7"),
        ("hour", "\\time=-61"),
        ("large", "\\year=12345 \\time=100000 \\month=123 \\day=0"),
        ("plain", ""),
    ] {
        let doc = format!(
            "\\catcode`\\{{=1 \\catcode`\\}}=2 \\pdfoutput=1 \\pdfcompresslevel=0 \
             \\pdfobjcompresslevel=0 \\pdfuniqueresname=1\n{date}\n\
             \\font\\a=cmr10 \\shipout\\hbox{{\\a A}}\\end\n"
        );
        let o = run(&texlive, &base.join(format!("o-{tag}")), doc.as_bytes());
        let f = run(&ours, &base.join(format!("f-{tag}")), doc.as_bytes());
        assert_eq!(o.code, Some(0), "{tag}: {o:?}");
        assert_eq!(f, o, "{tag}: ours vs pdftex");
    }
    let _ = std::fs::remove_dir_all(&base);
}
