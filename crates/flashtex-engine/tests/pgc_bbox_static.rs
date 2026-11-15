//! writet3.c keeps a Type 3 font's bounding box in the file statics
//! `t3_b0`..`t3_b3`, which only a written glyph sets and `writet3` never
//! resets. A Type 3 font that gets no glyph written (none of its used
//! characters is in its `.pgc` file) therefore takes the previous Type 3
//! font's `/FontBBox` (#2269). Run side by side with TeX Live's pdftex on
//! the same INITEX input: exit status, the log after its banner line and the
//! PDF, byte for byte. Both programs are started as `pdftex` from PATH, so
//! the messages that name the program agree. Found by lane AUDIT-PDFC.
//! Skips where there is no TeX Live (e.g. CI without it).
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

/// `pgca` has glyph 65, `pgcb` only 66; both are asked for 65.
const PGCA: &[u8] = b"\\pdffontscale 0.001\n\
\\pdfglyph 65 500 700 0 10 -20 510 720 =\n0 0 m 500 700 l S\n\\endglyph\n";
const PGCB: &[u8] = b"\\pdffontscale 0.002\n\
\\pdfglyph 66 500 700 0 0 0 400 600 =\n0 0 m 400 600 l S\n\\endglyph\n";

fn doc(body: &str) -> Vec<u8> {
    format!(
        "\\catcode`\\{{=1 \\catcode`\\}}=2 \\pdfoutput=1 \
         \\pdfcompresslevel=0 \\pdfobjcompresslevel=0\n\
         \\font\\a=pgca \\font\\b=pgcb \\shipout\\hbox{{{body}}}\n\\end\n"
    )
    .into_bytes()
}

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

fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}

#[test]
fn a_type3_font_without_glyphs_keeps_the_last_fonts_bbox() {
    let Some(texlive) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let tfm = cmr10_tfm(&texlive);
    let base = common::fresh_dir("flashtex-pgc-bbox");
    let ours = base.join("bin");
    std::fs::create_dir_all(&ours).unwrap();
    common::link_engine(
        Path::new(env!("CARGO_BIN_EXE_flashtex-initex")),
        &ours.join("pdftex"),
    );
    // `pgca` is written first, then `pgcb` (no glyph): pdfTeX gives `pgcb`
    // `pgca`'s box. The other way round both boxes are as written.
    for (tag, body, stale) in [("late", "\\b A\\a A", 2), ("early", "\\a A\\b A", 1)] {
        let d = doc(body);
        let files: &[(&str, &[u8])] = &[
            ("doc.tex", &d),
            ("pgca.tfm", &tfm),
            ("pgcb.tfm", &tfm),
            ("pgca.pgc", PGCA),
            ("pgcb.pgc", PGCB),
        ];
        let o = run(&texlive, &base.join(format!("o-{tag}")), files);
        let f = run(&ours, &base.join(format!("f-{tag}")), files);
        assert_eq!(o.code, Some(0), "{tag}: {o:?}");
        let pdf = o.pdf.as_deref().unwrap_or_default();
        assert_eq!(
            count(pdf, b"/FontBBox [ 10 -20 510 720 ]"),
            stale,
            "{tag}: the oracle's boxes"
        );
        assert_eq!(f, o, "{tag}: ours vs pdftex");
    }
    let _ = std::fs::remove_dir_all(&base);
}
