//! #1219: a space that is the first item of a list, under
//! `\pdfadjustinterwordglue`, reads `font(head)` and `character(head)` from
//! a stale word (changes/ext.ch). Where `pdf_font_kn_bs_base[f]+c` falls
//! past `pdf_mem`, pdfTeX's C reads whatever its heap holds there, so its
//! result is not determined and no lockstep case can hold the port to it:
//! this only checks that the port reads 0 there (the glue stays as it is)
//! and finishes. The determined cases are tools/lockstep/cases/2001-2005.
//! Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn list_head_space_reads_past_pdf_mem() {
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let dir = std::env::temp_dir().join(format!("flashtex-list-head-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // 44 \efcode blocks come before nullfont's kn code block, so nullfont's
    // base is past 11000 and pdf_mem holds about 12000 entries; the head
    // node last held \headthree's token (6010), and base+6010 is past it
    let mut tex = String::from(
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\pdfoutput=1\n\
         \\tracingonline=1 \\showboxdepth=100 \\showboxbreadth=100\n\
         \\font\\f=cmr10 \\f \\fontdimen6\\nullfont=5pt\n",
    );
    for i in 0..44 {
        tex.push_str(&format!(
            "\\font\\F{}=cmr10 at {}pt \\efcode\\F{}`x=1000\n",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ".as_bytes()[i % 26] as char,
            5 + i,
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ".as_bytes()[i % 26] as char,
        ));
    }
    tex.push_str(
        "\\knbscode\\nullfont`x=0 \\pdfadjustinterwordglue=1\n\
         \\def\\m#1{}\\chardef\\zero=0\n\
         \\m{\\headthree\\headthree\\headthree}\\setbox\\zero=\\hbox{ }\n\
         \\showbox\\zero\n\
         \\shipout\\box\\zero\n\\end\n",
    );
    std::fs::write(dir.join("job.tex"), tex).unwrap();
    let status = Command::new(ours)
        .args(["-ini", "-interaction=nonstopmode", "job.tex"])
        .current_dir(&dir)
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    // \showbox is an error in nonstop mode (exit 1), but no panic (101)
    assert!(matches!(status.code(), Some(0 | 1)), "{status:?}");
    let log = std::fs::read_to_string(dir.join("job.log")).unwrap();
    assert!(
        log.contains(".\\glue 3.33333 plus 1.66666 minus 1.11111"),
        "the glue changed:\n{log}"
    );
    assert!(dir.join("job.pdf").exists(), "no PDF");
    let _ = std::fs::remove_dir_all(&dir);
}
