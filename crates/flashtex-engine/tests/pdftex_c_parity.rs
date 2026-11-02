//! pdfTeX's C parts (`src/pdftex/`) against TeX Live's pdftex, run side by
//! side on the same INITEX input (lane WEB2RUST-AUDIT, audit of the C
//! ports): the log after its banner line and the PDF, byte for byte.
//!
//! * A map file and an encoding file found in the current directory are
//!   shown as `{my.map}` and `{my.enc}`: texmfmp.c's `open_input` drops the
//!   `./` kpathsea puts in front of them, and writet1.c's `load_enc_file`
//!   and mapfile.c's `fm_read_info` print that name.
//! * `\pdfuniqueresname`'s prefix is the CRC-32 of utils.c's `setjobid`
//!   string, which ends with pdfTeX's banner, web2c's version string and
//!   kpathsea's.
//!
//! Skips where there is no TeX Live (e.g. CI without it).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\
\\pdfoutput=1 \\pdfuniqueresname=1 \\pdfcompresslevel=0\n\
\\pdfmapfile{=my.map}\n\
\\font\\a=cmr10 \\a\n\
\\shipout\\hbox{AB}\n\
\\end\n";

/// Run `exe` (pdftex or this engine) on `doc.tex` in `dir`; the log after its
/// first line and the PDF.
fn run(exe: &Path, dir: &Path) -> (String, Vec<u8>) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("doc.tex"), DOC).unwrap();
    std::fs::write(
        dir.join("my.map"),
        "cmr10 CMR10 \" MyEnc ReEncodeFont \" <my.enc <cmr10.pfb\n",
    )
    .unwrap();
    let mut enc = String::from("/MyEnc [\n");
    for c in 0..256 {
        enc.push_str(match c {
            65 => "/A\n",
            66 => "/B\n",
            _ => "/.notdef\n",
        });
    }
    enc.push_str("] def\n");
    std::fs::write(dir.join("my.enc"), enc).unwrap();
    let st = Command::new(exe)
        .args(["-ini", "-interaction=nonstopmode", "doc.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "1700000000")
        .env("FORCE_SOURCE_DATE", "1")
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert!(st.success(), "{} exited with {st}", exe.display());
    let log = std::fs::read_to_string(dir.join("doc.log")).unwrap();
    let tail = log.split_once('\n').map_or("", |(_, t)| t).to_string();
    (tail, std::fs::read(dir.join("doc.pdf")).unwrap())
}

#[test]
fn cwd_map_and_enc_names_and_unique_resource_names_are_pdftex_s() {
    let Some(bin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-pdftex-c-parity");
    let (olog, opdf) = run(&bin.join("pdftex"), &base.join("o"));
    let (flog, fpdf) = run(
        Path::new(env!("CARGO_BIN_EXE_flashtex-initex")),
        &base.join("f"),
    );
    assert!(
        olog.contains("{my.map}") && olog.contains("{my.enc}"),
        "{olog}"
    );
    assert_eq!(flog, olog, "log after the banner");
    assert!(
        fpdf == opdf,
        "the PDFs differ (pdftex's in {}, ours in {})",
        base.join("o").display(),
        base.join("f").display()
    );
    let _ = std::fs::remove_dir_all(&base);
}
