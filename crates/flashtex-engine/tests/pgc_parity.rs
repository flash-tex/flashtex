//! Type 3 fonts from pdfTeX's `.pgc` glyph files (writet3.c's
//! `writet3`/`t3_write_glyph`, src/pdftex/writet3.rs) against TeX Live's
//! pdftex, run side by side on the same INITEX input: exit status, the log
//! after its banner line and the PDF, byte for byte. Both programs are
//! started as `pdftex` from PATH, so the messages that name the program
//! agree.
//!
//! Each case is a font `pgct` (cmr10's TFM under another name, so it has no
//! map entry) with the glyph file `pgct.pgc` in the working directory.
//!
//! * writet3.c rejects a font scale with `t3_font_scale <= 0 ||
//!   t3_font_scale > 1000`, so `\pdffontscale nan` (sscanf's `%g` reads a
//!   NaN) passes and the font matrix is `[nan 0 0 nan 0 0]`.
//!
//! Skips where there is no TeX Live (e.g. CI without it).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// What one run left: exit status, the log after its first line, the PDF if
/// any.
#[derive(Debug, PartialEq)]
struct Outcome {
    code: Option<i32>,
    log: String,
    pdf: Option<Vec<u8>>,
}

const DOC: &[u8] = b"\\catcode`\\{=1 \\catcode`\\}=2\n\
\\pdfoutput=1 \\pdfpagewidth=200pt \\pdfpageheight=200pt\n\
\\pdfcompresslevel=0 \\pdfobjcompresslevel=0\n\
\\font\\f=pgct \\shipout\\hbox{\\f ABC}\n\
\\end\n";

/// Run `pdftex` from `bin` on `doc.tex` in `dir`, with `files` written there
/// first.
fn run(bin: &Path, dir: &Path, files: &[(&str, &[u8])]) -> Outcome {
    std::fs::create_dir_all(dir).unwrap();
    for (name, data) in files {
        std::fs::write(dir.join(name), data).unwrap();
    }
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

/// cmr10's TFM, as kpathsea finds it next to `texlive`'s pdftex.
fn cmr10_tfm(texlive: &Path) -> Vec<u8> {
    let out = Command::new(texlive.join("kpsewhich"))
        .arg("cmr10.tfm")
        .output()
        .unwrap();
    let p = String::from_utf8(out.stdout).unwrap();
    std::fs::read(p.trim()).unwrap()
}

/// pdftex's outcome and ours on the font `pgct` with glyph file `pgc`.
fn both(tag: &str, pgc: &[u8]) -> Option<(Outcome, Outcome)> {
    let Some(texlive) = find_texlive_bin() else {
        common::no_texlive();
        return None;
    };
    let tfm = cmr10_tfm(&texlive);
    let files: &[(&str, &[u8])] = &[("doc.tex", DOC), ("pgct.tfm", &tfm), ("pgct.pgc", pgc)];
    let base = common::fresh_dir(&format!("flashtex-pgc-{tag}"));
    let ours = base.join("bin");
    std::fs::create_dir_all(&ours).unwrap();
    common::link_engine(
        Path::new(env!("CARGO_BIN_EXE_flashtex-initex")),
        &ours.join("pdftex"),
    );
    let o = run(&texlive, &base.join("o"), files);
    let f = run(&ours, &base.join("f"), files);
    let _ = std::fs::remove_dir_all(&base);
    Some((o, f))
}

/// A `.pgc` file: the font scale line, then glyphs A and B.
fn pgc(scale: &str) -> Vec<u8> {
    let mut v = format!("\\pdffontscale {scale}\n").into_bytes();
    for c in [65, 66] {
        v.extend_from_slice(
            format!("\\pdfglyph {c} 500 700 0 0 0 500 700 =\n0 0 m 500 700 l S\n\\endglyph\n")
                .as_bytes(),
        );
    }
    v
}

#[test]
fn pgc_font_scales_are_checked_as_pdftex_checks_them() {
    for (tag, scale) in [
        ("plain", "0.001"),
        ("nan", "nan"),
        ("zero", "0"),
        ("inf", "inf"),
    ] {
        let Some((o, f)) = both(tag, &pgc(scale)) else {
            return;
        };
        if tag == "nan" {
            // pdfTeX takes the NaN: a PDF with `/FontMatrix [nan 0 0 nan 0 0]`
            assert_eq!(o.code, Some(0), "{tag}: {o:?}");
            let pdf = o.pdf.as_deref().unwrap_or_default();
            let want: &[u8] = b"/FontMatrix [nan 0 0 nan 0 0]";
            assert!(
                pdf.windows(want.len()).any(|w| w == want),
                "{tag}: oracle PDF has no NaN font matrix"
            );
        }
        assert_eq!(f, o, "{tag}: ours vs pdftex");
    }
}
