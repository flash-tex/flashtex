//! PFB segment lengths that do not fit the segments (review of #1501, from
//! its fuzz cases): pdfTeX keeps the bytes left in a segment in a C `int`
//! (`t1_block_length`) and reads the next header only when it is exactly 0.
//! A length of 0 therefore runs negative and never ends, `ff ff ff ff` is
//! -1, and -2^31 wraps to 2^31 - 1. Each ends in pdfTeX's error
//! "N bytes more than expected" with its own N, which the engine must
//! print alike (P-T1 compares the logs). The fonts are TeX Live's
//! `cmr10.pfb` with one segment header changed. Skips where there is no
//! TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const JOB: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0\n\
    \\pdfmapline{+seg seg <seg.pfb}\\font\\x=seg \\shipout\\hbox{\\x abc}\n\\end\n";

/// The fatal error's message after "(file ...): ", joined across the log's
/// line breaks; None if the run had none.
fn error(log: &str) -> Option<String> {
    let start = log.find("!pdfTeX error: ")?;
    let end = log[start..].find(" ==> Fatal error")? + start;
    let msg = log[start..end].replace('\n', "");
    Some(msg[msg.find("): ")? + 3..].to_string())
}

fn run(bin: &Path, dir: &Path, pfb: &[u8], tfm: &Path, ours: bool) -> Option<String> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("seg.pfb"), pfb).unwrap();
    std::fs::copy(tfm, dir.join("seg.tfm")).unwrap();
    std::fs::write(dir.join("job.tex"), JOB).unwrap();
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=batchmode", "job.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    c.status().unwrap();
    error(&String::from_utf8_lossy(
        &std::fs::read(dir.join("job.log")).unwrap_or_default(),
    ))
}

fn kpsewhich(texbin: &Path, name: &str) -> std::path::PathBuf {
    let o = Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .unwrap();
    String::from_utf8(o.stdout).unwrap().trim().into()
}

#[test]
fn segment_lengths_count_as_a_c_int() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let cmr10 = std::fs::read(kpsewhich(&texbin, "cmr10.pfb")).unwrap();
    let tfm = kpsewhich(&texbin, "cmr10.tfm");
    let base = common::fresh_dir("flashtex-pfb-segments");
    // the offsets of the segment headers: clear, binary, and the clear
    // trailer (zeros and `cleartomark`, which pdfTeX does not read)
    let mut heads = vec![];
    let mut pos = 0;
    while cmr10.get(pos) == Some(&0x80) && matches!(cmr10.get(pos + 1), Some(1 | 2)) {
        heads.push(pos);
        pos += 6 + u32::from_le_bytes(cmr10[pos + 2..pos + 6].try_into().unwrap()) as usize;
    }
    assert!(heads.len() >= 2, "cmr10.pfb: a clear and a binary segment");
    for (seg, &at) in heads[..2].iter().enumerate() {
        for len in [0u32, 1, 0xffff_ffff, 0x7fff_ffff, 0x8000_0000] {
            let mut pfb = cmr10.clone();
            pfb[at + 2..at + 6].copy_from_slice(&len.to_le_bytes());
            let want = run(&theirs, &base.join("theirs"), &pfb, &tfm, false);
            let got = run(ours, &base.join("ours"), &pfb, &tfm, true);
            assert!(
                want.is_some(),
                "segment {seg}, length {len:#x}: pdfTeX gave no error"
            );
            assert_eq!(got, want, "segment {}, length {len:#x}", seg + 1);
        }
    }
    let _ = std::fs::remove_dir_all(&base);
}
