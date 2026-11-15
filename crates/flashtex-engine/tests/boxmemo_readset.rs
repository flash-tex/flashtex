//! BOX-MEMO and `\ifcsname` of a short name: the replay must not use a stale
//! answer.
//!
//! A memoized call that tests `\ifcsname Q\endcsname` (a ONE-character name)
//! or `\ifcsname\endcsname` (the EMPTY name) reads the meaning it tests
//! without going through `id_lookup`, so only BOX-MEMO's own hook in the
//! `\ifcsname` code (`flashtex_bm_read(cur_cs)`, `changes/boxmemo.ch`) puts
//! that read into the box's key. If the name is defined or undefined between
//! two otherwise identical calls, the second must NOT replay the first's
//! answer.
//!
//! Each test runs one document with BOX-MEMO off, on, and in verification
//! mode (helpers copied from `boxmemo.rs`: integration test files cannot
//! import each other): the log and the PDF bytes must be equal, and the ON
//! run must really have replayed (`hits == 2`: the second call replays the
//! first variant, the fourth the second). A stale replay would show in the
//! log, because every call reports `\n`, which the taken branch advances by
//! 100. Disabling that hook makes the four short-name and empty-name flip
//! tests fail (BOX-MEMO's verifier reports the stale replay); the two-or-more
//! character control does not depend on it.
//!
//! (The incremental read-set has its own test, `l5_ifcsname_of_a_short_name_is_a_read`
//! in `tests/incremental.rs`; this file is about the memoized-box replay.)

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let d = common::fresh_dir(&format!("flashtex-bmrs-{tag}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

struct Run {
    log: String,
    stats: String,
    /// The PDF, when the test made one.
    pdf: Option<Vec<u8>>,
}

/// Run `\input t` in INITEX with BOX-MEMO in `mode`, offering `names`.
fn run(dir: &Path, mode: &str, names: &str) -> Run {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let stats = dir.join(format!("stats-{mode}.txt"));
    let _ = std::fs::remove_file(&stats);
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(["-ini", "-etex", "\\input t"])
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_BOXMEMO", mode)
        .env("FLASHTEX_BOXMEMO_NAMES", names)
        .env("FLASHTEX_BOXMEMO_STATS", &stats)
        .env("FLASHTEX_BOXMEMO_VERIFY_FAIL", "1")
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("boxmemo verify"), "{err}");
    let pdf = std::fs::read(dir.join("t.pdf")).ok();
    let _ = std::fs::remove_file(dir.join("t.pdf"));
    Run {
        log: std::fs::read_to_string(dir.join("t.log")).unwrap(),
        stats: std::fs::read_to_string(&stats).unwrap_or_default(),
        pdf,
    }
}

fn stat(stats: &str, key: &str) -> u64 {
    stats
        .lines()
        .find_map(|l| {
            let l = l.trim().trim_end_matches(',');
            let (k, v) = l.split_once(':')?;
            (k.trim().trim_matches('"') == key).then(|| v.trim().parse().ok())?
        })
        .unwrap_or(0)
}

/// Run `src` off, on and verify; the logs must agree and the verifier find
/// nothing. Returns the "on" statistics.
fn check(tag: &str, src: &str) -> String {
    let d = scratch(tag);
    std::fs::write(d.join("t.tex"), src).unwrap();
    let off = run(&d, "off", "measure");
    let on = run(&d, "on", "measure");
    let verify = run(&d, "verify", "measure");
    assert_eq!(off.log, on.log, "{tag}: log with BOX-MEMO on differs");
    assert!(off.pdf == on.pdf, "{tag}: PDF with BOX-MEMO on differs");
    assert!(
        off.pdf == verify.pdf,
        "{tag}: PDF in verification mode differs"
    );
    assert_eq!(
        off.log, verify.log,
        "{tag}: log in verification mode differs"
    );
    assert_eq!(
        stat(&verify.stats, "verify_differences"),
        0,
        "{tag}: {}",
        verify.stats
    );
    // every admitted call is also compared on the whole engine state
    assert_eq!(
        stat(&verify.stats, "full_verified"),
        stat(&verify.stats, "verified"),
        "{tag}: {}",
        verify.stats
    );
    assert_eq!(
        stat(&verify.stats, "full_differences"),
        0,
        "{tag}: {}",
        verify.stats
    );
    assert_eq!(
        stat(&verify.stats, "verified"),
        stat(&on.stats, "hits"),
        "{tag}: {}\n{}",
        verify.stats,
        on.stats
    );
    on.stats
}

/// The catcodes, `\n`, and the per-call report; each test defines its own
/// `\measure` whose taken branch advances `\n` by 100.
const PRE: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
    \\countdef\\n=1\n\
    \\def\\show{\\message{[\\the\\n]}}\n";

/// A memoizable call like `boxmemo.rs`'s `\measure` (a box it throws away,
/// global state kept), testing `test` and counting the taken branch in `\n`.
fn measure(test: &str) -> String {
    format!(
        "\\def\\measure#1{{\\begingroup\\setbox0\\vbox{{\\hbox{{\\vrule width #1 height 4pt depth 1pt}}}}%\n\
         \\global\\dimen1=\\wd0 {test}\\global\\advance\\n by 100 \\fi\\endgroup}}\n"
    )
}

#[test]
fn one_char_name_flipped_to_defined() {
    // `\ifcsname Q\endcsname`: call twice with `\Q` undefined (the second
    // replays), `\def\Q{}`, call twice more (a miss, then a replay of the new
    // variant). A stale replay of the third call would log 0, not 100.
    let s = check(
        "rs-q-def",
        &[
            PRE,
            &measure("\\ifcsname Q\\endcsname"),
            "\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\
             \\def\\Q{}\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert_eq!(stat(&s, "hits"), 2, "{s}");
}

#[test]
fn empty_name_flipped_to_defined() {
    // the same with the EMPTY name `\ifcsname\endcsname`.
    let s = check(
        "rs-empty-def",
        &[
            PRE,
            &measure("\\ifcsname\\endcsname"),
            "\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\
             \\expandafter\\def\\csname\\endcsname{}\\n=0 \\measure{5pt}\\show\
             \\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert_eq!(stat(&s, "hits"), 2, "{s}");
}

#[test]
fn one_char_name_flipped_to_undefined() {
    // the flip in the OTHER direction: defined first, then `\let\Q\undefined`.
    let s = check(
        "rs-q-undef",
        &[
            PRE,
            &measure("\\ifcsname Q\\endcsname"),
            "\\def\\Q{}\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\
             \\let\\Q\\undefined\\n=0 \\measure{5pt}\\show\
             \\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert_eq!(stat(&s, "hits"), 2, "{s}");
}

#[test]
fn empty_name_flipped_to_undefined() {
    // the EMPTY name flipped the other way.
    let s = check(
        "rs-empty-undef",
        &[
            PRE,
            &measure("\\ifcsname\\endcsname"),
            "\\expandafter\\def\\csname\\endcsname{}\
             \\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\
             \\expandafter\\let\\csname\\endcsname\\undefined\
             \\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert_eq!(stat(&s, "hits"), 2, "{s}");
}

#[test]
fn two_char_name_flipped_to_defined() {
    // CONTROL: a two-character name, recorded through `id_lookup` before the
    // PR as well; it must pass and replay.
    let s = check(
        "rs-qq-def",
        &[
            PRE,
            &measure("\\ifcsname QQ\\endcsname"),
            "\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\
             \\def\\QQ{}\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert_eq!(stat(&s, "hits"), 2, "{s}");
}
