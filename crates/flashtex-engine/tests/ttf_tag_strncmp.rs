//! writettf.c's `ttf_name_lookup` finds a table with
//! `strncmp(tab->tag, s, 4) == 0`, and `ttf_copy_font` copies each
//! directory entry by looking its own tag up again. `strncmp` stops at a
//! NUL, so tags that agree up to a NUL (`zz\0a`, `zz\0b`) are one table to
//! pdfTeX: the second entry's copy finds the first table, copies its data
//! again and moves the first entry to it, and the second entry keeps the
//! input file's offset, length and checksum (#2276). Measured with pdfTeX
//! 1.40.29 (TeX Live 2026): Arvo-Bold.ttf with those two tables, embedded
//! whole, has `first table data` twice and `second, longer table data!!`
//! nowhere in its FontFile2. Found by lane AUDIT-PDFC. Skips where there is
//! no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const JOB: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0 \
    \\pdfobjcompresslevel=0\n\
    \\pdfmapline{=Arvo-Bold-tlf-ot1 ArvoX <<nultag.ttf}\n\
    \\font\\f=Arvo-Bold-tlf-ot1 \\shipout\\hbox{\\f AB}\n\\end\n";

const FIRST: &[u8] = b"first table data";
const SECOND: &[u8] = b"second, longer table data!!";

fn run(bin: &Path, dir: &Path, font: &[u8], ours: bool) -> Vec<u8> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("nultag.ttf"), font).unwrap();
    std::fs::write(dir.join("job.tex"), JOB).unwrap();
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=batchmode", "job.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "1700000000")
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
    std::fs::read(dir.join("job.pdf")).unwrap_or_default()
}

fn kpsewhich(texbin: &Path, name: &str) -> PathBuf {
    let o = Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .unwrap();
    String::from_utf8(o.stdout).unwrap().trim().into()
}

/// `font` with two more tables, tagged `zz\0a` and `zz\0b`, after its own.
fn with_nul_tags(font: &[u8]) -> Vec<u8> {
    let n = u16::from_be_bytes([font[4], font[5]]) as usize;
    let old_dir_end = 12 + 16 * n;
    let shift = 32u32;
    let mut out = font[..12].to_vec();
    out[4..6].copy_from_slice(&((n + 2) as u16).to_be_bytes());
    for i in 0..n {
        let e = &font[12 + 16 * i..28 + 16 * i];
        out.extend_from_slice(&e[..8]);
        let off = u32::from_be_bytes(e[8..12].try_into().unwrap());
        out.extend_from_slice(&(off + shift).to_be_bytes());
        out.extend_from_slice(&e[12..16]);
    }
    let mut body = font[old_dir_end..].to_vec();
    body.resize(body.len().next_multiple_of(4), 0);
    let mut extra = Vec::new();
    let mut base = (old_dir_end as u32 + shift) + body.len() as u32;
    for (tag, data) in [(b"zz\0a", FIRST), (b"zz\0b", SECOND)] {
        out.extend_from_slice(tag);
        out.extend_from_slice(&0u32.to_be_bytes()); // pdfTeX ignores it
        out.extend_from_slice(&base.to_be_bytes());
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        extra.extend_from_slice(data);
        extra.resize(extra.len().next_multiple_of(4), 0);
        base = (old_dir_end as u32 + shift) + (body.len() + extra.len()) as u32;
    }
    out.extend(body);
    out.extend(extra);
    out
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

#[test]
fn tags_equal_up_to_a_nul_are_one_table() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let arvo = kpsewhich(&texbin, "Arvo-Bold.ttf");
    if arvo.as_os_str().is_empty() {
        common::no_texlive();
        return;
    }
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let font = with_nul_tags(&std::fs::read(arvo).unwrap());
    let base = common::fresh_dir("flashtex-ttf-tag-strncmp");
    let want = run(&theirs, &base.join("theirs"), &font, false);
    let got = run(ours, &base.join("ours"), &font, true);
    assert!(!want.is_empty(), "pdfTeX wrote no PDF");
    assert_eq!(
        count(&want, FIRST),
        2,
        "pdfTeX copies the first table twice"
    );
    assert_eq!(count(&want, SECOND), 0, "pdfTeX never copies the second");
    assert!(got == want, "the PDFs differ");
    let _ = std::fs::remove_dir_all(&base);
}
