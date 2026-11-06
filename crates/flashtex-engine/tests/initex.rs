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

/// A register at -2^31 (TeX does not check `\advance` for overflow), where
/// web2c's C negates it in `abs` and `negate`, which C leaves undefined and
/// clang compiles as if the negation were exact: scan_dimen's
/// `abs(cur_val)>=@'10000000000` holds (web2rust `--exact-abs`),
/// `print_scaled` prints `-32768.0pt`, and `init_randoms` starts from 2^31
/// halved (changes/intmin.ch). The expected log is TeX Live 2026's pdftex's
/// (arm64 and x86-64 on macOS give the same, 2026-11-01: `pdftex -ini t.tex`)
/// after `(./t.tex`, which the cwd resolver prints as `(t.tex`.
#[test]
fn int_min_registers_as_texlive_prints_them() {
    let d = scratch("intmin");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=-2147483647 \\advance\\count0 by -1\n\
         \\dimen0=\\count0 sp \\showthe\\dimen0\n\
         \\dimen1=-16383.99999pt \\advance\\dimen1 by \\dimen1 \\advance\\dimen1 by -2sp\n\
         \\showthe\\dimen1\n\
         \\pdfsetrandomseed\\count0\n\
         \\message{\\pdfuniformdeviate 1000, \\pdfuniformdeviate 1000000, \\pdfnormaldeviate}\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         ! Dimension too large.\n\
         l.3 \\dimen0=\\count0 sp \n\
         {}\\showthe\\dimen0\n\
         I can't work with sizes bigger than about 19 feet.\n\
         Continue and I'll use the largest value I can.\n\
         \n\
         > -16383.99998pt.\n\
         l.3 \\dimen0=\\count0 sp \\showthe\\dimen0\n\
         {}\n\
         \n\
         > -32768.0pt.\n\
         l.5 \\showthe\\dimen1\n\
         {}\n\
         \n\
         741, 224026, -30694 )\n\
         No pages of output.\n",
        sp(23),
        sp(38),
        sp(19)
    );
    assert!(log.ends_with(&expected), "{log}");
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
