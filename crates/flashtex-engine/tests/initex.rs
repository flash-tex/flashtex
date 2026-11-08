//! End-to-end checks of the INITEX binary, with no TeX installation needed.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let d = common::fresh_dir(&format!("flashtex-engine-{tag}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn initex(dir: &Path, first_line: &str) -> (bool, String) {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        // web2c: without -ini a run loads a format (pdftex.fmt).
        .args(["-ini", first_line])
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        // web2c's reproducible date: 1 JAN 1970 00:00 in the log.
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn relax_end() {
    let d = scratch("relax");
    let (ok, term) = initex(&d, "\\relax\\end");
    assert!(ok, "exit status; terminal:\n{term}");
    assert!(
        term.starts_with(
            "This is pdfTeX, Version 3.141592653-2.6-1.40.29 (TeX Live 2026) (INITEX)"
        ),
        "{term}"
    );
    assert!(term.contains("No pages of output."), "{term}");
    let log = std::fs::read_to_string(d.join("texput.log")).unwrap();
    assert_eq!(
        log,
        "This is pdfTeX, Version 3.141592653-2.6-1.40.29 (TeX Live 2026) (INITEX)  1 JAN 1970 00:00\n\
         **\\relax\\end\n\
         No pages of output.\n"
    );
}

/// The committed configuration is TeX Live's: error_line=79 and
/// half_error_line=50 mean a 50-character context line is not truncated, and
/// `\showbox` reports a 32-bit glue ratio exactly.
#[test]
fn production_capacities_and_glue_display() {
    let d = scratch("show");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\showboxdepth=10 \\showboxbreadth=10\n\
         \\setbox0=\\hbox to 100pt{\\kern 3pt\\hskip 5pt plus 2fil\\kern 7pt}\\showbox0\n\
         \\skip0=3pt plus 1fill minus 2pt \\showthe\\skip0\n\
         \\end\n",
    )
    .unwrap();
    // \show... counts as an error, so the exit status is 1, as in web2c.
    let _ = initex(&d, "\\input t");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    assert!(
        log.contains("\\hbox(0.0+0.0)x100.0, glue set 42.5fil"),
        "{log}"
    );
    assert!(
        log.contains("l.4 \\skip0=3pt plus 1fill minus 2pt \\showthe\\skip0"),
        "{log}"
    );
    assert!(log.contains("> 3.0pt plus 1.0fill minus 2.0pt."), "{log}");
}

/// `\pdfsnapy` with a snap glue of width 0: pdftex.web's `gap_amount`
/// computes `(cur_pos-pdf_snapy_refpos) div snap_unit` with `snap_unit=0`.
/// TeX Live's arm64 pdftex (whose `sdiv` gives 0) ships the page; its x86-64
/// build dies of SIGFPE; Rust's `/` panicked. With web2rust's
/// `--total-division` the engine does what the arm64 build does. The
/// expected page stream is TeX Live 2026's pdftex's (arm64, 2026-11-01:
/// `pdftex -ini t.tex`, exit status 0); the file sizes differ only by the
/// banner in `/PTEX.Fullbanner`, so they are not compared.
#[test]
fn pdfsnapy_zero_width_does_what_texlive_arm64_does() {
    let d = scratch("snapy0");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0\n\
         \\shipout\\vbox{\\hrule height 3pt width 10pt \\pdfsnapy 0pt \\hrule height 5pt width 10pt}\n\
         \\end\n",
    )
    .unwrap();
    let (ok, term) = initex(&d, "t.tex");
    assert!(ok, "exit status; terminal:\n{term}");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    assert!(
        log.contains("t.tex [0] )\nOutput written on t.pdf (1 page, "),
        "{log}"
    );
    let pdf = std::fs::read(d.join("t.pdf")).unwrap();
    let stream: &[u8] = b"stream\nq\n1 0 0 1 72 76.981 cm\n0 0 9.963 2.989 re f\nQ\n\
q\n1 0 0 1 72 146.989 cm\n0 0 9.963 4.981 re f\nQ\n\nendstream";
    assert!(
        pdf.windows(stream.len()).any(|w| w == stream),
        "page stream differs from pdftex's:\n{}",
        String::from_utf8_lossy(&pdf)
    );
}
