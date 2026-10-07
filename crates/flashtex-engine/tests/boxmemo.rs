//! BOX-MEMO (docs/design/engine-v2/BOX-MEMO.md, src/boxmemo.rs): a
//! registered macro call that typesets only what it throws away is recorded
//! and replayed while its key holds. These tests run INITEX on small files
//! with `FLASHTEX_BOXMEMO_NAMES` naming the macro, and check that
//!
//! * the log is byte-identical with BOX-MEMO off, on, and in the
//!   both-paths verification mode, and the verifier finds no difference;
//! * calls with an equal key are replayed;
//! * the key falls back exactly when it must: a parameter, a register, a
//!   macro it reads, a name it looked up and did not find, a font
//!   parameter, an argument;
//! * a call outside the model (output, a box it did not make, the outer
//!   list) is never replayed.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let d = common::fresh_dir(&format!("flashtex-bm-{tag}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

struct Run {
    log: String,
    stats: String,
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
    Run {
        log: std::fs::read_to_string(dir.join("t.log")).unwrap(),
        stats: std::fs::read_to_string(&stats).unwrap_or_default(),
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
    assert_eq!(stat(&verify.stats, "full_differences"), 0, "{tag}: {}", verify.stats);
    assert_eq!(
        stat(&verify.stats, "verified"),
        stat(&on.stats, "hits"),
        "{tag}: {}\n{}",
        verify.stats,
        on.stats
    );
    on.stats
}

/// `\measure{W}` measures what a frame adds around a rule of width W, in a
/// box it throws away (framed.sty's `\fb@sizeofframe` in miniature), and
/// keeps two global dimensions; on the way it makes local and global
/// assignments of every kind the log replays.
const PRELUDE: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
    \\countdef\\n=1 \\dimendef\\w=2 \\dimendef\\h=3 \\skipdef\\s=4 \\toksdef\\t=5\n\
    \\def\\pad{3pt}\\def\\frame#1{\\hbox{\\vrule width 1pt\\kern\\pad\\vbox{\\hrule\\kern\\pad#1\\kern\\pad\\hrule}\\kern\\pad\\vrule width 1pt}}\n\
    \\def\\measure#1{\\begingroup\\setbox0\\vbox{\\frame{\\hbox{\\vrule width #1 height 4pt depth 1pt}}}%\n\
    \\global\\w=\\wd0 \\global\\h=\\ht0 \\global\\advance\\n by 1 \\global\\s=1pt plus 2pt\n\
    \\def\\inner{x}\\xdef\\last{\\the\\w/\\the\\h}\\global\\let\\copy\\last\\global\\t{\\inner}\n\
    \\ifcsname absent\\endcsname\\global\\advance\\n by 100 \\fi\\endgroup}\n\
    \\def\\show{\\message{[\\the\\w|\\the\\h|\\the\\n|\\the\\s|\\meaning\\last|\\meaning\\copy|\\the\\t|\\the\\badness]}}\n";

#[test]
fn replays_and_matches() {
    let s = check(
        "basic",
        &[
            PRELUDE,
            "\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\
             \\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 2, "{s}");
}

#[test]
fn the_key_falls_back() {
    // a different argument, a register it reads changed (\n), a macro it
    // reads redefined (\pad), a parameter changed (\baselineskip... here
    // \boxmaxdepth), a name it looked up defined: each is a miss, and the
    // logs still agree
    let s = check(
        "fallback",
        &[
            PRELUDE,
            "\\n=0 \\measure{5pt}\\show\\n=0 \\measure{6pt}\\show\
             \\n=7 \\measure{5pt}\\show\\n=0 \\def\\pad{4pt}\\measure{5pt}\\show\
             \\n=0 \\boxmaxdepth=0pt \\measure{5pt}\\show\
             \\n=0 \\def\\absent{}\\measure{5pt}\\show\
             \\n=0 \\measure{5pt}\\show\\n=0 \\measure{5pt}\\show\\end\n",
        ]
        .concat(),
    );
    assert!(stat(&s, "hits") >= 1, "{s}");
}

#[test]
fn outside_the_model_never_replays() {
    // output, a box register it did not make (and that is not void), and
    // the outer list: recorded calls are abandoned, so nothing replays
    let s = check(
        "impure",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\setbox9\\hbox{\\vrule width 2pt}\n\
         \\def\\measure#1{\\begingroup\\message{#1}\\endgroup}\n\
         \\measure{a}\\measure{a}\\measure{a}\n\
         \\def\\measure#1{\\begingroup\\global\\dimen1=\\wd9 \\endgroup}\n\
         \\measure{a}\\measure{a}\\measure{a}\n\
         \\def\\measure#1{\\kern#1}\n\
         \\measure{1pt}\\measure{1pt}\\measure{1pt}\\showlists\\end\n",
    );
    assert_eq!(stat(&s, "hits"), 0, "{s}");
}

#[test]
fn void_box_reads_are_part_of_the_key() {
    let s = check(
        "void",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\measure#1{\\begingroup\\setbox0\\copy9 \\global\\dimen1=\\wd0 \\endgroup}\n\
         \\measure{a}\\measure{a}\\measure{a}\\message{[\\the\\dimen1]}\n\
         \\setbox9\\hbox{\\vrule width 2pt}\\measure{a}\\message{[\\the\\dimen1]}\\end\n",
    );
    assert!(stat(&s, "hits") >= 1, "{s}");
}
