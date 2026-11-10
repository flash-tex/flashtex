//! writet1.c's `t1_fontname_offset` is a static that only a subset font's
//! `/FontName` line sets, and `t1_subset_ascii_part` writes the subset tag
//! at it once it reaches `/Encoding`. A font whose `/FontName` comes after
//! `/Encoding` (legal: a font dictionary's keys come in any order) therefore
//! gets its tag written where the previous subset font had its name, over
//! its own bytes there; pdfTeX asserts only when no subset font came
//! before. Measured with pdfTeX 1.40.29 (TeX Live 2026): two cmr10
//! subsets, the second with `/FontName` moved after `/Encoding`, and the
//! second font's `/FontBBox {-40 -250 ...` reads `/FontBBox {RITJWY50 ...`
//! in pdfTeX's PDF. Found by lane AUDIT-PDFC. Skips where there is no TeX
//! Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const JOB: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0 \
    \\pdfobjcompresslevel=0\n\
    \\pdfmapline{=one CMR10 <one.pfb}\\pdfmapline{=two CMR10 <two.pfb}\n\
    \\font\\a=one \\font\\b=two \\shipout\\hbox{\\a Hello AV fi \\b Hello BW}\n\\end\n";

fn run(bin: &Path, dir: &Path, fonts: [&[u8]; 2], tfm: &Path, ours: bool) -> Vec<u8> {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).unwrap();
    for (name, font) in ["one", "two"].iter().zip(fonts) {
        std::fs::write(dir.join(format!("{name}.pfb")), font).unwrap();
        std::fs::copy(tfm, dir.join(format!("{name}.tfm"))).unwrap();
    }
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

fn kpsewhich(texbin: &Path, name: &str) -> std::path::PathBuf {
    let o = Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .unwrap();
    String::from_utf8(o.stdout).unwrap().trim().into()
}

/// `cmr10.pfb` with `/FontName` moved from before `/Encoding` to the end
/// of the clear part (the first segment's length fixed up).
fn name_after_encoding(pfb: &[u8]) -> Vec<u8> {
    assert_eq!(
        &pfb[..2],
        &[0x80, 1],
        "cmr10.pfb starts with a clear segment"
    );
    let len = u32::from_le_bytes(pfb[2..6].try_into().unwrap()) as usize;
    let clear = String::from_utf8_lossy(&pfb[6..6 + len]).into_owned();
    let name = "/FontName /CMR10 def\n";
    let end = "currentdict end\n";
    assert!(
        clear.contains(name) && clear.contains(end),
        "cmr10.pfb's clear part"
    );
    let moved = clear
        .replacen(name, "", 1)
        .replacen(end, &format!("{name}{end}"), 1);
    let mut out = vec![0x80, 1];
    out.extend_from_slice(&(moved.len() as u32).to_le_bytes());
    out.extend_from_slice(moved.as_bytes());
    out.extend_from_slice(&pfb[6 + len..]);
    out
}

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

#[test]
fn a_late_font_name_takes_the_previous_fonts_tag_offset() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let cmr10 = std::fs::read(kpsewhich(&texbin, "cmr10.pfb")).unwrap();
    let tfm = kpsewhich(&texbin, "cmr10.tfm");
    let late = name_after_encoding(&cmr10);
    let base = common::fresh_dir("flashtex-t1-fontname-offset");
    let want = run(&theirs, &base.join("theirs"), [&cmr10, &late], &tfm, false);
    let got = run(ours, &base.join("ours"), [&cmr10, &late], &tfm, true);
    assert!(!want.is_empty(), "pdfTeX wrote no PDF");
    // pdfTeX's second font keeps the placeholder in its name and has the
    // tag over its /FontBBox
    assert_eq!(count(&want, b"/FontName /ABCDEF+CMR10 def"), 1);
    assert_eq!(
        count(&want, b"/FontBBox {-40 -250 1009 750 }readonly def"),
        1
    );
    assert!(got == want, "the PDFs differ");
    let _ = std::fs::remove_dir_all(&base);
}
