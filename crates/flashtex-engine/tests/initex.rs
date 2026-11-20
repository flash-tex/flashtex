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

/// The pool-filling probe of #2515: `k` control-sequence names of 1000
/// characters, then one of `l` characters, so that `l` sets the room left in
/// the string pool.
fn pool_fill(k: u32, l: usize) -> String {
    format!(
        "\\catcode`\\{{=1 \\catcode`\\}}=2 \\catcode`\\#=6 \\scrollmode \\def\\X{{{x}}}\
         \\count1=10000 \\def\\fill{{\\ifnum\\count1<{end} \\advance\\count1 1 \
         \\expandafter\\let\\csname\\the\\count1 \\X\\endcsname\\relax \\expandafter\\fill\\fi}}\\fill \
         \\expandafter\\let\\csname {y}\\endcsname\\relax ",
        x = "x".repeat(995),
        end = 10000 + k,
        y = "y".repeat(l),
    )
}

/// One run of a pool probe with argv[0] `pdftex`, so that `pdftex_fail`
/// names the program as TeX Live's does: exit status, terminal, stderr.
#[cfg(unix)]
fn initex_as_pdftex(dir: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    use std::os::unix::process::CommandExt;
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .arg0("pdftex")
        .args(args)
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// `maketexstring`'s failure as TeX Live 2026's pdftex prints it on the
/// terminal (`pdftex -ini -etex`, 2026-11-15, exit status 1): utils.c's
/// `check_buf(poolptr + l, poolsize)`.
#[cfg(unix)]
const MAKETEXSTRING_FAIL: &str =
    "\n!pdfTeX error: pdftex: buffer overflow at file ../../../texk/web2c/pdftexdir/ut\n\
     ils.c, line 183\n ==> Fatal error occurred, no output PDF file produced!\n";

/// Sweeps the room left in the pool across the window where a string that
/// web2c's C parts make (`maketexstring`) no longer fits but what TeX made
/// before it did (#2515). TeX Live 2026's pdftex stops there with utils.c's
/// `pdftex_fail` (6212 names: `l` = 1050 to 1100 before a 100-letter
/// `\input` name, 1100 to 1200 before a 200-letter `-jobname`; FlashTeX's
/// INITEX pool has 409 characters more, #2208, hence the wide sweep).
/// FlashTeX wrote past the pool's end and panicked (exit 101). Every run
/// must end as pdftex's can (those lines, `TeX capacity exceeded`, or a
/// normal end), and the window must be hit.
#[cfg(unix)]
fn sweep_maketexstring(tag: &str, run: impl Fn(&Path, usize) -> (Option<i32>, String, String)) {
    let d = scratch(tag);
    let mut failed = 0;
    for l in (1000..=1900).step_by(15) {
        for f in std::fs::read_dir(&d).unwrap() {
            let _ = std::fs::remove_file(f.unwrap().path());
        }
        let (code, term, err) = run(&d, l);
        assert!(
            !err.contains("panicked") && code != Some(101),
            "l={l}: exit {code:?}\nstderr:\n{err}\nterminal:\n{term}"
        );
        if term.contains("buffer overflow") {
            assert_eq!(code, Some(1), "l={l}");
            assert!(term.ends_with(MAKETEXSTRING_FAIL), "l={l}:\n{term}");
            failed += 1;
        } else {
            assert!(
                code == Some(0) || term.contains("! TeX capacity exceeded, sorry [pool size="),
                "l={l}: exit {code:?}\n{term}"
            );
        }
    }
    assert!(failed > 0, "no run hit maketexstring's pool check");
}

/// `makefullnamestring` (`\input`'s full name) with the pool nearly full.
#[cfg(unix)]
#[test]
fn input_full_name_past_the_pool_is_maketexstrings_failure() {
    let name = "a".repeat(100);
    sweep_maketexstring("maketexstring-input", |d, l| {
        std::fs::write(d.join(format!("{name}.tex")), "\\message{[in]}\n").unwrap();
        let t = format!(
            "{}\n\\input {name} \n\\message{{[survived]}}\n\\end\n",
            pool_fill(6212, l)
        );
        std::fs::write(d.join("t.tex"), t).unwrap();
        initex_as_pdftex(d, &["-ini", "-etex", "t.tex"])
    });
}

/// `getjobname` (`-jobname`, made when `\end` opens the log) with the pool
/// nearly full.
#[cfg(unix)]
#[test]
fn jobname_past_the_pool_is_maketexstrings_failure() {
    let job = format!("-jobname={}", "j".repeat(200));
    sweep_maketexstring("maketexstring-jobname", |d, l| {
        let first = format!("{}\\end", pool_fill(6212, l));
        initex_as_pdftex(d, &["-ini", "-etex", &job, &first])
    });
}
