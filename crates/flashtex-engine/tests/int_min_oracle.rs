//! Oracle pins for the arithmetic surface around INT_MIN (-2147483648).
//!
//! Every expected tail below was measured from the installed TeX Live 2026
//! pdftex on arm64 (2026-11-02): in a fresh dir under $HOME/tmp, write t.tex,
//! run `SOURCE_DATE_EPOCH=0 FORCE_SOURCE_DATE=1 perl -e 'alarm 20; exec @ARGV'
//! /Library/TeX/texbin/pdftex -ini t.tex </dev/null`, read t.log. The three
//! e-TeX tests add pdftex's `-etex` flag (plain `-ini` leaves `\numexpr` and
//! `\dimexpr` undefined) and run FlashTeX with `-etex` too. Only the stable
//! tail from `(t.tex` is compared. This file does not repeat initex.rs's pins
//! (`\dimen0=\count0 sp` at -2^31, `\showthe` of a -2^31 `\dimen`,
//! `\pdfsetrandomseed`, `\pdfsnapy 0pt`).

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let d = common::fresh_dir(&format!("flashtex-engine-{tag}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn initex_with(dir: &Path, extra: &[&str], first_line: &str) -> (bool, String) {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let mut args: Vec<&str> = vec!["-ini"];
    args.extend(extra.iter());
    args.push(first_line);
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
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
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

fn initex(dir: &Path, first_line: &str) -> (bool, String) {
    initex_with(dir, &[], first_line)
}

fn initex_etex(dir: &Path, first_line: &str) -> (bool, String) {
    initex_with(dir, &["-etex"], first_line)
}

/// `\divide` of INT_MIN (rebuilt before each): by 2 and by -2 print
/// 1073741824, by 3 prints 715827882 -- pdftex's divide negates through a
/// wrapping abs, so the sign comes out positive.
#[test]
fn divide_int_min_by_two_minus_two_and_three() {
    let d = scratch("intmin-div3");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\divide\\count0 by 2\n\
         \\showthe\\count0\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\divide\\count0 by -2\n\
         \\showthe\\count0\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\divide\\count0 by 3\n\
         \\showthe\\count0\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         > 1073741824.\n\
         l.5 \\showthe\\count0\n\
         {}\n\
         \n\
         > 1073741824.\n\
         l.9 \\showthe\\count0\n\
         {}\n\
         \n\
         > 715827882.\n\
         l.13 \\showthe\\count0\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(19),
        sp(19),
        sp(20),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}

/// `\divide` of INT_MIN by -1 and by 1: both print -2147483648 with no
/// error (the negation wraps back to INT_MIN instead of overflowing).
#[test]
fn divide_int_min_by_minus_one_and_one() {
    let d = scratch("intmin-divn1");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\divide\\count0 by -1\n\
         \\showthe\\count0\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\divide\\count0 by 1\n\
         \\showthe\\count0\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         > -2147483648.\n\
         l.5 \\showthe\\count0\n\
         {}\n\
         \n\
         > -2147483648.\n\
         l.9 \\showthe\\count0\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(19),
        sp(19),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}

/// `\multiply` of INT_MIN by 1 and by -1: both report Arithmetic overflow
/// and keep -2147483648 (even times 1 overflows pdftex's check).
#[test]
fn multiply_int_min_by_one_and_minus_one() {
    let d = scratch("intmin-mul");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\multiply\\count0 by 1\n\
         \\showthe\\count0\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\multiply\\count0 by -1\n\
         \\showthe\\count0\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         ! Arithmetic overflow.\n\
         l.4 \\multiply\\count0 by 1\n\
         {}\n\
         I can't carry out that multiplication or division,\n\
         since the result is out of range.\n\
         \n\
         > -2147483648.\n\
         l.5 \\showthe\\count0\n\
         {}\n\
         \n\
         ! Arithmetic overflow.\n\
         l.8 \\multiply\\count0 by -1\n\
         {}\n\
         I can't carry out that multiplication or division,\n\
         since the result is out of range.\n\
         \n\
         > -2147483648.\n\
         l.9 \\showthe\\count0\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(25),
        sp(19),
        sp(26),
        sp(19),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}

/// `\number` of INT_MIN prints -2147483648; `\romannumeral` of it is empty
/// (as of any non-positive number).
#[test]
fn number_and_romannumeral_of_int_min() {
    let d = scratch("intmin-num");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\message{[\\number\\count0]}\n\
         \\message{[\\romannumeral\\count0]}\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let expected = "(t.tex\n[-2147483648] [] )\nNo pages of output.\n";
    assert!(log.ends_with(expected), "{log}");
}

/// e-TeX (-etex): `\numexpr` of INT_MIN overflows even for /2 and *-1 --
/// any INT_MIN operand fails with "I can't evaluate this expression" and
/// evaluates to 0.
#[test]
fn numexpr_of_int_min_overflows() {
    let d = scratch("intmin-numexpr");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         \\count1=\\numexpr\\count0/2\\relax\n\
         \\showthe\\count1\n\
         \\count2=\\numexpr\\count0*-1\\relax\n\
         \\showthe\\count2\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex_etex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         ! Arithmetic overflow.\n\
         <recently read> \\relax \n\
         {}\n\
         l.4 \\count1=\\numexpr\\count0/2\\relax\n\
         {}\n\
         I can't evaluate this expression,\n\
         since the result is out of range.\n\
         \n\
         > 0.\n\
         l.5 \\showthe\\count1\n\
         {}\n\
         \n\
         ! Arithmetic overflow.\n\
         <recently read> \\relax \n\
         {}\n\
         l.6 \\count2=\\numexpr\\count0*-1\\relax\n\
         {}\n\
         I can't evaluate this expression,\n\
         since the result is out of range.\n\
         \n\
         > 0.\n\
         l.7 \\showthe\\count2\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(23),
        sp(35),
        sp(19),
        sp(23),
        sp(36),
        sp(19),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}

/// e-TeX (-etex): `\dimexpr` of a -2^31 sp dimen (built silently with the
/// same `\advance` trick as initex.rs): /2 reports Dimension too large and
/// gives 8192.0pt, unary minus reports Dimension too large and gives
/// -16383.99998pt.
#[test]
fn dimexpr_of_int_min_dimen() {
    let d = scratch("intmin-dimexpr");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\dimen0=-16383.99999pt\n\
         \\advance\\dimen0 by \\dimen0\n\
         \\advance\\dimen0 by -2sp\n\
         \\dimen1=\\dimexpr\\dimen0/2\\relax\n\
         \\showthe\\dimen1\n\
         \\dimen2=\\dimexpr-\\dimen0\\relax\n\
         \\showthe\\dimen2\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex_etex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         ! Dimension too large.\n\
         <to be read again> \n\
         {}/\n\
         l.5 \\dimen1=\\dimexpr\\dimen0/\n\
         {}2\\relax\n\
         I can't work with sizes bigger than about 19 feet.\n\
         Continue and I'll use the largest value I can.\n\
         \n\
         > 8192.0pt.\n\
         l.6 \\showthe\\dimen1\n\
         {}\n\
         \n\
         ! Dimension too large.\n\
         <to be read again> \n\
         {}\\relax \n\
         l.7 \\dimen2=\\dimexpr-\\dimen0\\relax\n\
         {}\n\
         I can't work with sizes bigger than about 19 feet.\n\
         Continue and I'll use the largest value I can.\n\
         \n\
         > -16383.99998pt.\n\
         l.8 \\showthe\\dimen2\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(19),
        sp(28),
        sp(19),
        sp(19),
        sp(34),
        sp(19),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}

/// `\skip0` with a -2^31 sp natural width (four -8192pt `\advance`s, which
/// unlike assignment skip the range check): `\showthe` prints -32768.0pt.
#[test]
fn skip_int_min_natural_width() {
    let d = scratch("intmin-skip");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\skip0=-8192pt\n\
         \\advance\\skip0 by -8192pt\n\
         \\advance\\skip0 by -8192pt\n\
         \\advance\\skip0 by -8192pt\n\
         \\showthe\\skip0\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         > -32768.0pt.\n\
         l.6 \\showthe\\skip0\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(18),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}

/// `\divide` by 0: Arithmetic overflow ("I can't carry out that
/// multiplication or division"), and the register keeps its old value.
#[test]
fn divide_count_by_zero() {
    let d = scratch("intmin-div0");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=7\n\
         \\divide\\count0 by 0\n\
         \\showthe\\count0\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         ! Arithmetic overflow.\n\
         l.3 \\divide\\count0 by 0\n\
         {}\n\
         I can't carry out that multiplication or division,\n\
         since the result is out of range.\n\
         \n\
         > 7.\n\
         l.4 \\showthe\\count0\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(23),
        sp(19),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}

/// e-TeX (-etex): `\numexpr 1/0` is Arithmetic overflow ("I can't evaluate
/// this expression") and evaluates to 0.
#[test]
fn numexpr_divide_by_zero() {
    let d = scratch("intmin-numdiv0");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count3=\\numexpr1/0\\relax\n\
         \\showthe\\count3\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex_etex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         ! Arithmetic overflow.\n\
         <recently read> \\relax \n\
         {}\n\
         l.2 \\count3=\\numexpr1/0\\relax\n\
         {}\n\
         I can't evaluate this expression,\n\
         since the result is out of range.\n\
         \n\
         > 0.\n\
         l.3 \\showthe\\count3\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(23),
        sp(29),
        sp(19),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}

/// `\penalty\count0` with INT_MIN in a paragraph: `\showlists` shows
/// `\penalty -2147483648`. Only the showlists lines are pinned (pdftex also
/// ships page [-2147483648], whose dvi byte count is not compared).
#[test]
fn penalty_int_min_in_showlists() {
    let d = scratch("intmin-penalty");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -1\n\
         A\\penalty\\count0 B\\showlists\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    assert!(
        log.contains("### horizontal mode entered at line 4"),
        "{log}"
    );
    assert!(
        log.contains("\\penalty -2147483648\nspacefactor 999\n"),
        "{log}"
    );
}

/// `\advance` does no range check: 2147483647 + 1 wraps to -2147483648 and
/// -2147483647 + -2 wraps to 2147483647, both silently.
#[test]
fn advance_wraps_around_int_min() {
    let d = scratch("intmin-advwrap");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\n\
         \\count0=2147483647\n\
         \\advance\\count0 by 1\n\
         \\showthe\\count0\n\
         \\count0=-2147483647\n\
         \\advance\\count0 by -2\n\
         \\showthe\\count0\n\
         \\end\n",
    )
    .unwrap();
    let _ = initex(&d, "t.tex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let sp = |n: usize| " ".repeat(n);
    let expected = format!(
        "(t.tex\n\
         > -2147483648.\n\
         l.4 \\showthe\\count0\n\
         {}\n\
         \n\
         > 2147483647.\n\
         l.7 \\showthe\\count0\n\
         {}\n\
         \n\
         {})\n\
         No pages of output.\n",
        sp(19),
        sp(19),
        sp(1)
    );
    assert!(log.ends_with(&expected), "{log}");
}
