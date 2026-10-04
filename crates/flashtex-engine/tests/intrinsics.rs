//! Guarded intrinsics (DESIGN.md §5.6 item 4, src/intrinsics.rs): a
//! registered parameterless macro run by `main_control` is recorded and
//! replayed while its guard holds. These tests run INITEX on small files
//! with `FLASHTEX_INTRINSIC_NAMES` naming the macro, and check that
//!
//! * the log and output are byte-identical with the intrinsics off, on,
//!   and in the both-paths verification mode;
//! * the verifier finds no difference in any replayed call;
//! * the guard falls back exactly when it must: a dependency redefined
//!   mid-document, `\globaldefs=1`, a pending `\afterassignment`, tracing;
//! * a macro that is not pure is never replayed.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(tag: &str) -> PathBuf {
    let d = common::fresh_dir(&format!("flashtex-intr-{tag}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

struct Run {
    log: String,
    stats: String,
}

/// Run `\input t` in INITEX with the intrinsics in `mode`.
fn run(dir: &Path, mode: &str, names: &str) -> Run {
    run_with(dir, mode, names, false)
}

/// The same, with macros with parameters offered (`args`,
/// `FLASHTEX_INTRINSICS_ARGS=on`; MACRO-REPLAY.md).
fn run_with(dir: &Path, mode: &str, names: &str, args: bool) -> Run {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let stats = dir.join(format!("stats-{mode}.txt"));
    let _ = std::fs::remove_file(&stats);
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(["-ini", "\\input t"])
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_INTRINSICS", mode)
        .env("FLASHTEX_INTRINSIC_NAMES", names)
        .env("FLASHTEX_INTRINSICS_STATS", &stats)
        .env("FLASHTEX_INTRINSICS_ARGS", if args { "on" } else { "off" })
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    assert!(
        out.stderr.is_empty()
            || !String::from_utf8_lossy(&out.stderr).contains("intrinsics verify"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
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
    let off = run(&d, "off", "hook");
    let on = run(&d, "on", "hook");
    let verify = run(&d, "verify", "hook");
    assert_eq!(off.log, on.log, "{tag}: log with intrinsics on differs");
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
    assert_eq!(
        stat(&verify.stats, "verified"),
        stat(&on.stats, "replays"),
        "{tag}"
    );
    on.stats
}

const PRELUDE: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
    \\def\\a{A}\\def\\b{B}\\countdef\\n=1 \\toksdef\\t=2\n\
    \\def\\hook{\\begingroup\\let\\x\\a\\def\\y{\\b\\a}\\endgroup\
    \\global\\n=\\the\\catcode`\\@\\relax\\gdef\\last{\\a}\\let\\z\\b\\edef\\w{\\a\\b\\the\\n}\
    \\ifx\\a\\b\\def\\v{same}\\else\\def\\v{different}\\fi\
    \\expandafter\\def\\csname made\\endcsname{\\z}\\t{\\w}\\catcode`\\@=11 }\n\
    \\def\\show{\\message{[\\meaning\\z|\\meaning\\w|\\meaning\\last|\\meaning\\v|\\meaning\\made|\\the\\t|\\the\\n|\\the\\catcode`\\@]}}\n";

#[test]
fn replays_and_matches() {
    let s = check(
        "basic",
        &format!("{PRELUDE}\\hook\\show\\hook\\hook\\show\\hook\\show\\end\n"),
    );
    assert!(stat(&s, "replays") >= 2, "{s}");
}

#[test]
fn redefined_dependency_falls_back() {
    let s = check(
        "redef",
        &format!(
            "{PRELUDE}\\hook\\hook\\show\n\\def\\a{{AA}}\\hook\\show\\hook\\show\n\
             \\let\\b=\\a \\hook\\show\\def\\b{{B}}\\hook\\show\\hook\\show\
             \\def\\hook{{\\def\\w{{new}}}}\\hook\\show\\hook\\show\\end\n"
        ),
    );
    // after each redefinition the guard fails (Deps), the macro is recorded
    // afresh, and the next call replays again
    assert!(stat(&s, "Deps") >= 3, "{s}");
    assert!(stat(&s, "replays") >= 3, "{s}");
}

#[test]
fn globaldefs_and_afterassignment_fall_back() {
    let s = check(
        "pre",
        &format!(
            "{PRELUDE}\\hook\\hook\\show\n\
             {{\\globaldefs=1 \\hook\\show}}\\show\n\
             \\def\\after{{\\message{{[after]}}}}\\afterassignment\\after\\hook\\show\n\
             \\hook\\show\\end\n"
        ),
    );
    assert_eq!(stat(&s, "GlobalDefs"), 1, "{s}");
    assert_eq!(stat(&s, "AfterAssignment"), 1, "{s}");
}

#[test]
fn tracing_falls_back() {
    let s = check(
        "trace",
        &format!(
            "{PRELUDE}\\hook\\hook\\tracingonline=1 \\tracingmacros=2 \\tracingcommands=3 \
             \\tracingassigns=1 \\tracingrestores=1 \\tracinggroups=1 \\tracingifs=1\n\
             \\hook\\show\\tracingmacros=0 \\tracingcommands=0 \\tracingassigns=0 \
             \\tracingrestores=0 \\tracinggroups=0 \\tracingifs=0 \\hook\\show\\end\n"
        ),
    );
    assert_eq!(stat(&s, "Tracing"), 1, "{s}");
}

#[test]
fn impure_macro_is_not_replayed() {
    // \message writes to the log; \hbox typesets; \aftergroup leaves the group
    for (tag, body) in [
        ("msg", "\\message{hi}\\def\\z{1}"),
        ("box", "\\setbox0=\\hbox{}\\def\\z{1}"),
        ("after", "\\begingroup\\aftergroup\\relax\\endgroup"),
        ("outside", "\\let\\z="),
        ("dimen", "\\dimen0=1pt"),
    ] {
        let src = format!(
            "\\catcode`\\{{=1 \\catcode`\\}}=2 \\scrollmode\\def\\hook{{{body}}}\n\
             \\hook\\relax\\hook\\relax\\hook\\relax\\message{{[\\meaning\\z]}}\\end\n"
        );
        let s = check(tag, &src);
        assert_eq!(stat(&s, "replays"), 0, "{tag}: {s}");
    }
}

#[test]
fn register_arithmetic_is_a_dependency() {
    // \advance reads the register: while the count changes from call to
    // call the hook is never replayed; with it reset to the same value it
    // is, and each replay gives what the normal path gives.
    let body = "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\\countdef\\m=3 \\countdef\\k=4\n\
        \\def\\hook{\\global\\advance\\m by 1 \\multiply\\k by 2 \\edef\\w{\\the\\m.\\the\\k}}\n";
    let s = check(
        "adv",
        &format!("{body}\\k=3 \\hook\\hook\\hook\\hook\\message{{[\\w]}}\\end\n"),
    );
    assert_eq!(stat(&s, "replays"), 0, "{s}");
    let s = check(
        "adv-reset",
        &format!(
            "{body}\\m=0 \\k=3 \\hook\\m=0 \\k=3 \\hook\\message{{[\\w]}}\\m=0 \\k=3 \\hook\\message{{[\\w]}}\
             \\m=5 \\hook\\message{{[\\w]}}\\end\n"
        ),
    );
    assert_eq!(stat(&s, "replays"), 2, "{s}");
}

#[test]
fn verify_all_on_the_test_file() {
    // every parameterless macro big_switch expands is a candidate
    let d = scratch("all");
    std::fs::write(
        d.join("t.tex"),
        format!(
            "{PRELUDE}\\hook\\show\\hook\\show\\hook\\show\\def\\a{{x}}\\hook\\show\\hook\\show\\hook\\show\
             \\relax\\hook\\relax\\hook\\show\\end\n"
        ),
    )
    .unwrap();
    let off = run(&d, "off", "");
    let all = run(&d, "verify-all", "");
    assert_eq!(off.log, all.log);
    assert_eq!(stat(&all.stats, "verify_differences"), 0, "{}", all.stats);
    assert!(stat(&all.stats, "verified") >= 2, "{}", all.stats);
}

// ---------------------------------------------------------------------------
// Macros with parameters (docs/design/engine-v2/MACRO-REPLAY.md): the call
// site after the argument scan, keyed on the argument tokens.
// ---------------------------------------------------------------------------

/// Run `src` off, on and verify with macros with parameters offered and
/// `names` registered; the logs must agree, the verifier find nothing (the
/// leak check included) and verify every call the "on" run replays.
/// Returns the "on" statistics.
fn check_args(tag: &str, src: &str, names: &str) -> String {
    let d = scratch(tag);
    std::fs::write(d.join("t.tex"), src).unwrap();
    let off = run_with(&d, "off", names, true);
    let on = run_with(&d, "on", names, true);
    let verify = run_with(&d, "verify", names, true);
    assert_eq!(off.log, on.log, "{tag}: log with replays differs");
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
    assert_eq!(
        stat(&verify.stats, "verified"),
        stat(&on.stats, "replays"),
        "{tag}: {}\n{}",
        verify.stats,
        on.stats
    );
    on.stats
}

const ARGS_PRELUDE: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
    \\def\\a{A}\\def\\b{B}\\def\\c{C}\n\
    \\def\\cl#1#2{\\begingroup\\edef\\x{#1:#2}\\global\\let\\cur\\x\\endgroup\\let\\last=#1\\relax}\n\
    \\def\\show{\\message{[\\meaning\\cur|\\meaning\\last]}}\n";

#[test]
fn a_macro_with_arguments_replays_per_argument_list() {
    let s = check_args(
        "args-basic",
        &format!(
            "{ARGS_PRELUDE}\\cl\\a\\b\\show\\cl\\a\\b\\show\\cl\\a\\c\\show\\cl\\a\\b\\show\
             \\cl\\a\\c\\show\\cl{{x}}{{y}}\\show\\cl{{x}}{{y}}\\show\\end\n"
        ),
        "cl",
    );
    // calls 2, 4, 5 and 7 replay; 1, 3 and 6 have new argument lists
    assert_eq!(stat(&s, "args_replays"), 4, "{s}");
    assert_eq!(stat(&s, "NotRecorded"), 3, "{s}");
}

#[test]
fn a_changed_meaning_of_an_argument_falls_back() {
    // the body expands its first argument: the meaning of \z is read
    let s = check_args(
        "args-meaning",
        &format!(
            "{ARGS_PRELUDE}\\let\\z\\a \\cl\\z\\b\\show\\cl\\z\\b\\show\
             \\let\\z\\c \\cl\\z\\b\\show\\cl\\z\\b\\show\\def\\b{{BB}}\\cl\\z\\b\\show\\end\n"
        ),
        "cl",
    );
    assert!(stat(&s, "Deps") >= 2, "{s}");
    assert_eq!(stat(&s, "args_replays"), 2, "{s}");
}

#[test]
fn delimited_and_conditional_arguments() {
    // (the body ends in a command: one that ends in an expansion, here
    // `\fi` after a true branch, makes `get_x_token` read the token after
    // the call before `big_switch` sees the body done, which abandons the
    // recording, as for a macro without parameters; and the tokens a
    // false branch skips are read as meanings, so each branch defines a
    // macro of its own)
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\def\\a{A}\\def\\b{B}\n\
        \\def\\dl#1.#2\\stop{\\ifx#1\\a\\def\\r{yes #2}\\else\\def\\s{no #2}\\fi\\relax}\n\
        \\def\\show{\\message{[\\meaning\\r|\\meaning\\s]}}\n\
        \\dl\\a.x y\\stop\\show\\dl\\a.x y\\stop\\show\\dl\\b.x y\\stop\\show\
        \\dl\\b.x y\\stop\\show\\dl\\a.{x}y\\stop\\show\\end\n";
    let s = check_args("args-delim", src, "dl");
    assert_eq!(stat(&s, "args_replays"), 2, "{s}");
}

#[test]
fn a_body_that_reads_past_its_arguments_is_not_replayed() {
    // \la's body ends in a \let whose source is the token after the call
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\def\\a{A}\\def\\la#1{\\def\\q{#1}\\let\\next=}\n\
        \\la x\\a\\la x\\a\\la x\\a\\message{[\\meaning\\q|\\meaning\\next]}\\end\n";
    let s = check_args("args-lookahead", src, "la");
    assert_eq!(stat(&s, "args_replays"), 0, "{s}");
}

#[test]
fn macros_with_arguments_are_off_by_default() {
    let d = scratch("args-default");
    std::fs::write(
        d.join("t.tex"),
        format!("{ARGS_PRELUDE}\\cl\\a\\b\\show\\cl\\a\\b\\show\\end\n"),
    )
    .unwrap();
    let on = run(&d, "on", "cl");
    assert_eq!(stat(&on.stats, "args_calls"), 0, "{}", on.stats);
    assert_eq!(stat(&on.stats, "replays"), 0, "{}", on.stats);
}

#[test]
fn verify_all_args_on_the_test_file() {
    // every macro with parameters big_switch expands is a candidate
    let d = scratch("args-all");
    std::fs::write(
        d.join("t.tex"),
        format!(
            "{ARGS_PRELUDE}\\def\\two#1#2{{\\def\\p{{#2#1}}}}\\two xy\\two xy\\two yx\
             \\cl\\a\\b\\show\\cl\\a\\b\\show\\def\\a{{x}}\\cl\\a\\b\\show\\cl\\a\\b\\show\\two xy\\end\n"
        ),
    )
    .unwrap();
    let off = run_with(&d, "off", "", true);
    let all = run_with(&d, "verify-all-args", "", true);
    assert_eq!(off.log, all.log);
    assert_eq!(stat(&all.stats, "verify_differences"), 0, "{}", all.stats);
    assert!(stat(&all.stats, "verified") >= 3, "{}", all.stats);
}

#[test]
fn many_argument_lists_each_replay() {
    // more argument lists than the parameterless site's four variants: the
    // index keeps them all (MACRO-REPLAY.md §3.3)
    let mut src = String::from(
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\kv#1#2{\\edef\\cur{#1:#2}}\\def\\show{\\message{[\\meaning\\cur]}}\n",
    );
    for round in 0..3 {
        for k in 0..12 {
            src.push_str(&format!("\\kv{{k{k}}}{{r{round}}}\\show"));
        }
        src.push('\n');
    }
    for k in 0..12 {
        src.push_str(&format!("\\kv{{k{k}}}{{r0}}\\show"));
    }
    src.push_str("\\end\n");
    let s = check_args("args-many", &src, "kv");
    assert_eq!(stat(&s, "args_committed"), 36, "{s}");
    assert_eq!(stat(&s, "args_replays"), 12, "{s}");
}

#[test]
fn an_argument_list_that_cannot_be_recorded_is_not_recorded_again() {
    // \dm's body scans a dimension only for the argument `d`: that key is
    // kept as not recordable, the others replay
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\def\\d{d}\\def\\dm#1{\\def\\t{#1}\\ifx\\t\\d\\dimen0=1pt\\fi\\def\\u{#1}\\relax}\n\
        \\dm a\\dm d\\dm a\\dm d\\dm b\\dm d\\dm b\\message{[\\meaning\\u|\\the\\dimen0]}\\end\n";
    let s = check_args("args-dead", src, "dm");
    assert_eq!(stat(&s, "args_replays"), 2, "{s}");
    assert_eq!(stat(&s, "Unrecordable"), 2, "{s}");
    assert_eq!(stat(&s, "Dimension"), 1, "{s}");
}
