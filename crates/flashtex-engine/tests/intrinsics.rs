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
    run_env(dir, mode, names, args, None)
}

/// The same, with a fault injected (`FLASHTEX_INTRINSICS_FAULT`); then the
/// verifier may report differences.
fn run_env(dir: &Path, mode: &str, names: &str, args: bool, fault: Option<&str>) -> Run {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let stats = dir.join(format!("stats-{mode}.txt"));
    let _ = std::fs::remove_file(&stats);
    // (a test file whose first line is `%etex` runs in e-TeX mode)
    let etex = std::fs::read_to_string(dir.join("t.tex")).is_ok_and(|t| t.starts_with("%etex"));
    let argv: &[&str] = if etex {
        &["-ini", "-etex", "\\input t"]
    } else {
        &["-ini", "\\input t"]
    };
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(argv)
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_INTRINSICS", mode)
        .env("FLASHTEX_INTRINSIC_NAMES", names)
        .env("FLASHTEX_INTRINSICS_STATS", &stats)
        .env("FLASHTEX_INTRINSICS_ARGS", if args { "on" } else { "off" })
        .env("FLASHTEX_INTRINSICS_FAULT", fault.unwrap_or(""))
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    assert!(
        fault.is_some()
            || out.stderr.is_empty()
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
    // \message writes to the log; \hbox and \mark typeset; `em` reads the font
    for (tag, body) in [
        ("msg", "\\message{hi}\\def\\z{1}"),
        ("box", "\\setbox0=\\hbox{}\\def\\z{1}"),
        ("mark", "\\mark{x}\\def\\z{1}"),
        ("outside", "\\let\\z="),
        ("dimen", "\\dimen0=1em"),
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
    // the invariants of MACRO-REPLAY.md §7.2, exactly, at the end of the run
    assert!(
        stat(&verify.stats, "invariant_checks") >= 1,
        "{tag}: {}",
        verify.stats
    );
    assert_eq!(
        stat(&verify.stats, "invariant_failures"),
        0,
        "{tag}: {}",
        verify.stats
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
    // \dm's body scans a dimension in units of the current font only for
    // the argument `d`: that key is kept as not recordable, the others
    // replay
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\def\\d{d}\\def\\dm#1{\\def\\t{#1}\\ifx\\t\\d\\dimen0=1em\\fi\\def\\u{#1}\\relax}\n\
        \\dm a\\dm d\\dm a\\dm d\\dm b\\dm d\\dm b\\message{[\\meaning\\u|\\the\\dimen0]}\\end\n";
    let s = check_args("args-dead", src, "dm");
    assert_eq!(stat(&s, "args_replays"), 2, "{s}");
    assert_eq!(stat(&s, "Unrecordable"), 2, "{s}");
    assert_eq!(stat(&s, "Dimension"), 1, "{s}");
}

const KV: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
    \\def\\a{A}\\def\\b{B}\\def\\kv#1#2{\\edef\\cur{#1:#2}}\\def\\show{\\message{[\\meaning\\cur]}}\n";

/// MACRO-REPLAY.md §6.5: each fault of the argument site is caught by the
/// verifier (the state diff or the leak check).
#[test]
fn faults_at_the_argument_site_are_caught() {
    for (fault, calls) in [
        // the arguments are not freed: the leak check
        ("no-flush", "\\kv\\a\\b\\show\\kv\\a\\b\\show"),
        // {a}{bd} takes {a}{bc}'s recording
        ("args-last", "\\kv{a}{bc}\\show\\kv{a}{bd}\\show"),
        // the redefined \kv is not seen: the recording watched \b
        (
            "cur-cs",
            "\\kv\\a\\b\\show\\def\\kv#1#2{\\edef\\cur{#2:#1}}\\kv\\a\\b\\show",
        ),
    ] {
        let d = scratch(&format!("fault-{fault}"));
        std::fs::write(d.join("t.tex"), format!("{KV}{calls}\\end\n")).unwrap();
        let clean = run_env(&d, "verify", "kv", true, None);
        assert_eq!(
            stat(&clean.stats, "verify_differences"),
            0,
            "{fault}: {}",
            clean.stats
        );
        let faulty = run_env(&d, "verify", "kv", true, Some(fault));
        assert!(
            stat(&faulty.stats, "verify_differences") >= 1,
            "{fault} was not caught: {}",
            faulty.stats
        );
    }
}

/// `const-hash`: every argument list in one bucket of the index costs time
/// only (the keys are compared in full).
#[test]
fn a_constant_hash_changes_nothing() {
    let mut calls = String::new();
    for round in 0..2 {
        for k in 0..10 {
            calls.push_str(&format!("\\kv{{k{k}}}{{r{round}}}\\show"));
        }
    }
    calls.push_str(&calls.clone());
    let d = scratch("fault-const-hash");
    std::fs::write(d.join("t.tex"), format!("{KV}{calls}\\end\n")).unwrap();
    let off = run_env(&d, "off", "kv", true, None);
    let on = run_env(&d, "on", "kv", true, None);
    let hashed = run_env(&d, "on", "kv", true, Some("const-hash"));
    let verify = run_env(&d, "verify", "kv", true, Some("const-hash"));
    assert_eq!(off.log, hashed.log);
    assert_eq!(stat(&on.stats, "args_replays"), 20, "{}", on.stats);
    assert_eq!(stat(&hashed.stats, "args_replays"), 20, "{}", hashed.stats);
    assert_eq!(
        stat(&verify.stats, "verify_differences"),
        0,
        "{}",
        verify.stats
    );
}

// ---------------------------------------------------------------------------
// What beamer's colour code needs (MACRO-REPLAY.md §12): dimensions,
// sparse registers read through their names, modes and `align_state` only
// where the body depends on them, local groups elided.
// ---------------------------------------------------------------------------

#[test]
fn dimension_arithmetic_is_recorded() {
    // the dimension registers and parameters read are value pairs: \pp
    // changed falls back (WordDeps), and the arithmetic replays as K_WORDs
    let src = "%etex\n\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\dimendef\\dd=1 \\dimendef\\pp=2 \\pp=1pt \\countdef\\cc=3\n\
        \\def\\mix#1#2{\\begingroup\\dd=#1\\pp \\advance\\dd by #2.5\\pp \\divide\\dd by 3 \
        \\multiply\\dd by 2 \\ifdim\\dd>2pt \\cc=\\dd \\else\\cc=-1 \\fi\
        \\dd=\\dimexpr\\dd*2+1in-3bp\\relax\\xdef\\r{\\the\\dd/\\the\\cc}\\endgroup\\relax}\n\
        \\def\\show{\\message{[\\r|\\the\\dd]}}\n\
        \\mix12\\show\\mix12\\show\\mix 3{-1}\\show\\mix12\\show\\pp=2pt \\mix12\\show\\mix12\\show\
        \\mix 3{-1}\\show\\end\n";
    let s = check_args("dims", src, "mix");
    assert_eq!(stat(&s, "args_replays"), 3, "{s}");
    assert_eq!(stat(&s, "WordDeps"), 2, "{s}");
    assert_eq!(stat(&s, "Dimension"), 0, "{s}");
}

#[test]
fn units_that_read_the_font_or_the_magnification_are_not_recorded() {
    for (tag, unit) in [
        ("em", "1em"),
        ("ex", "1ex"),
        ("px", "1px"),
        ("true", "1truept"),
    ] {
        let src = format!(
            "\\catcode`\\{{=1 \\catcode`\\}}=2 \\catcode`\\#=6 \\scrollmode\n\
             \\def\\un#1{{\\dimen0={unit}\\def\\q{{#1}}}}\n\
             \\un a\\un a\\un a\\message{{[\\the\\dimen0]}}\\end\n"
        );
        let s = check_args(&format!("unit-{tag}"), &src, "un");
        assert_eq!(stat(&s, "args_replays"), 0, "{tag}: {s}");
        assert_eq!(stat(&s, "Dimension"), 1, "{tag}: {s}");
    }
}

#[test]
fn a_sparse_register_read_by_its_name_is_a_dependency() {
    // e-TeX's \count300, named by \countdef: its value is a pair on the
    // register's element; reading one by number is still refused (`Internal`)
    let src = "%etex\n\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\countdef\\big=300 \\big=5 \\dimendef\\bd=301 \\bd=2pt\n\
        \\def\\rd#1{\\edef\\r{#1\\the\\big/\\the\\bd}\\relax}\n\
        \\def\\rn#1{\\edef\\r{#1\\the\\count300}\\relax}\n\
        \\def\\show{\\message{[\\r]}}\n\
        \\rd a\\show\\rd a\\show\\big=6 \\rd a\\show\\rd a\\show\\bd=1pt \\rd a\\show\\rd a\\show\
        \\rn a\\show\\rn a\\show\\end\n";
    let s = check_args("sparse", src, "rd,rn");
    assert_eq!(stat(&s, "args_replays"), 3, "{s}");
    assert_eq!(stat(&s, "WordDeps"), 2, "{s}");
    assert_eq!(stat(&s, "Internal"), 1, "{s}");
}

#[test]
fn a_body_replays_in_any_mode_unless_a_space_or_brace_depends_on_it() {
    // \kv runs no command that depends on the mode: recorded in vertical
    // mode, it replays in horizontal mode. \sp's body has a space, which
    // horizontal mode typesets: never replayed there.
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\def\\kv#1#2{\\edef\\cur{#1:#2}}\\def\\sp#1{\\def\\q{#1} \\relax}\n\
        \\def\\show{\\message{[\\meaning\\cur|\\meaning\\q]}}\n\
        \\kv ab\\sp a\\show\\kv ab\\sp a\\show\\indent\\kv ab\\sp a\\show\\kv ab\\sp a\\show\\par\
        \\kv ab\\sp a\\show\\end\n";
    let s = check_args("modes", src, "kv,sp");
    // \kv: calls 2-5; \sp: 2 and 5 (vertical mode)
    assert_eq!(stat(&s, "args_replays"), 6, "{s}");
    assert!(stat(&s, "Mode") >= 2, "{s}");
}

#[test]
fn align_state_matters_only_to_a_body_with_a_tab_mark() {
    // inside braces `align_state` is one more: \kv replays there; \am's
    // body scans a `&` (get_next tests `align_state` for it), so it
    // replays only at the depth it was recorded at
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\catcode`\\&=4 \\scrollmode\n\
        \\def\\kv#1#2{\\edef\\cur{#1:#2}}\\def\\am#1{\\def\\q{#1&}}\n\
        \\def\\show{\\message{[\\meaning\\cur|\\meaning\\q]}}\n\
        \\kv ab\\am a\\show{\\kv ab\\am a\\show}\\kv ab\\am a\\show{{\\kv ab\\am a\\show}}\\end\n";
    let s = check_args("align", src, "kv,am");
    // \kv: calls 2-4; \am: call 3
    assert_eq!(stat(&s, "args_replays"), 4, "{s}");
    assert!(stat(&s, "Align") >= 2, "{s}");
}

#[test]
fn a_group_with_only_local_operations_is_elided() {
    // xcolor's pattern: everything inside \begingroup ... \endgroup is
    // local, and the value handed out is a separate operation after it.
    // The first recording of \lg read \out undefined (`\expandafter` reads
    // it as a meaning), which its own \def changes: the second call records
    // again, and the third and fourth replay.
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\def\\lg#1{\\begingroup\\def\\x{#1}\\edef\\y{\\x\\x}\\begingroup\\let\\z\\y\\endgroup\
        \\expandafter\\endgroup\\expandafter\\def\\expandafter\\out\\expandafter{\\y}}\n\
        \\def\\gg#1{\\begingroup\\def\\x{#1}\\global\\let\\gx\\x\\endgroup\\relax}\n\
        \\def\\show{\\message{[\\meaning\\out|\\meaning\\gx|\\meaning\\x]}}\n\
        \\lg a\\show\\lg a\\show\\lg a\\show\\lg a\\show\\gg b\\show\\gg b\\show\\end\n";
    let s = check_args("elide", src, "lg,gg");
    assert_eq!(stat(&s, "args_replays"), 3, "{s}");
    // \lg replays one operation (the \def after its group), twice; \gg its
    // whole group: begin, \def, \global\let, end
    assert_eq!(stat(&s, "replayed_ops"), 2 + 4, "{s}");
}

#[test]
fn a_replay_never_hides_a_grouping_overflow() {
    // \deep opens 6 groups; called 250 levels down, the expansion
    // overflows "grouping levels=255", so the call is refused and the run
    // ends with the expansion's own fatal error
    let mut src = String::from(
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\deep#1{\\begingroup\\begingroup\\begingroup\\begingroup\\begingroup\\begingroup\
         \\def\\q{#1}\\endgroup\\endgroup\\endgroup\\endgroup\\endgroup\\endgroup\\relax}\n\
         \\deep a\\deep a\n",
    );
    for _ in 0..250 {
        src.push_str("\\begingroup");
    }
    src.push_str("\\deep a\\end\n");
    let s = check_args("depth", &src, "deep");
    assert_eq!(stat(&s, "args_replays"), 1, "{s}");
    assert_eq!(stat(&s, "Margin"), 1, "{s}");
}

#[test]
fn a_body_that_ends_in_an_expansion_is_recorded() {
    // \fe's body ends in `\fi`: `big_switch`'s `get_x_token` expands it and
    // reads on past the body; the recording ends just before that read
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\def\\empty{}\\def\\fe#1{\\def\\q{#1}\\ifx\\q\\empty\\def\\s{none}\\else\\def\\r{#1}\\fi}\n\
        \\def\\show{\\message{[\\meaning\\q|\\meaning\\r|\\meaning\\s]}}\n\
        \\fe a\\show\\fe a\\show\\fe a\\relax\\show\\fe a\\fe a\\show\\fe{}\\show\\fe{}\\show\\end\n";
    let s = check_args("ends-in-fi", src, "fe");
    // calls 2-5 and 7 (the tokens a false branch skips are read as
    // meanings: \fe a watches \s, which \fe{} defines, so those calls come
    // last)
    assert_eq!(stat(&s, "args_replays"), 5, "{s}");
    assert_eq!(stat(&s, "Level"), 0, "{s}");
}

#[test]
fn sparse_register_assignments_are_operations() {
    // \st sets a sparse register (local, at level one); \sw reads one and
    // writes another inside a group with a global \xdef (kept whole)
    let src = "%etex\n\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
        \\countdef\\cm=300 \\countdef\\nm=301 \\nm=2\n\
        \\def\\st#1{\\cm=#1\\relax}\n\
        \\def\\sw#1{\\begingroup\\cm=#1 \\advance\\cm by \\nm \\xdef\\r{\\the\\cm}\\endgroup}\n\
        \\def\\show{\\message{[\\the\\cm|\\the\\nm|\\r]}}\n\
        \\st5\\show\\st5\\show\\st7\\show\\sw1\\show\\sw1\\show\\nm=5 \\sw1\\show\\sw1\\show\
        {\\st5\\show}\\show\\end\n";
    let s = check_args("sparse-ops", src, "st,sw");
    // \st: calls 2 and 4 (keys 5, 5, 7, 5); \sw: calls 2 and 4
    assert_eq!(stat(&s, "args_replays"), 4, "{s}");
    assert_eq!(stat(&s, "WordDeps"), 1, "{s}");
    assert_eq!(stat(&s, "Sparse"), 0, "{s}");
}

#[test]
fn colour_stack_pushes_and_aftergroup_are_operations() {
    // xcolor's \set@color: a colour stack push and an \aftergroup'd pop
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\\pdfoutput=1\n\
        \\def\\popc{\\pdfcolorstack0 pop}\
        \\def\\col#1{\\pdfcolorstack0 push{#1}\\aftergroup\\popc}\n\
        {\\col{1 0 0 rg}}{\\col{1 0 0 rg}}{\\col{0 g}}{\\col{1 0 0 rg}}\\col{0 g}\\showlists\\end\n";
    let s = check_args("colorstack", src, "col");
    // calls 2, 4 and 5
    assert_eq!(stat(&s, "args_replays"), 3, "{s}");
}

#[test]
fn a_body_that_ends_in_ignorespaces_is_recorded() {
    // xcolor's \color ends in \ignorespaces, which reads past the body: the
    // recording ends there, and a replay leaves the spaces after the call
    // to be skipped by its caller (horizontal mode would typeset them)
    let src = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\\showboxdepth=9 \\showboxbreadth=99\n\
        \\def\\ig#1{\\def\\q{#1}\\ignorespaces}\\def\\sp{ }\n\
        \\indent\\ig a x\\ig a   x\\ig{a}\\sp x\\ig a\\ig a\\relax x\\ig b\n\n\\ig a y\\showlists\\end\n";
    let s = check_args("ignorespaces", src, "ig");
    // calls 2-5 and 7 (6 has a new argument)
    assert_eq!(stat(&s, "args_replays"), 5, "{s}");
}

/// MACRO-REPLAY.md §12's faults: each is caught by the verifier (the state
/// diff, which now covers sparse registers and the nodes a body appends) or,
/// for what happens after the body, by the output.
#[test]
fn revision_6_faults_are_caught() {
    let pre = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n";
    for (fault, names, src) in [
        (
            "elide-global",
            "gg",
            format!(
                "{pre}\\def\\gg#1{{\\begingroup\\def\\x{{#1}}\\global\\let\\gx\\x\\endgroup\\relax}}\n\
                 \\gg a\\let\\gx\\relax\\gg a\\message{{[\\meaning\\gx]}}\\end\n"
            ),
        ),
        (
            "no-mode-mask",
            "sp",
            format!(
                "{pre}\\def\\sp#1{{\\def\\q{{#1}} \\relax}}\\sp a\\indent\\sp a\\showlists\\end\n"
            ),
        ),
        (
            "sparse-unwatched",
            "rd",
            format!(
                "%etex\n{pre}\\countdef\\big=300 \\big=5 \\def\\rd#1{{\\edef\\r{{#1\\the\\big}}\\relax}}\n\
                 \\rd a\\big=6 \\rd a\\message{{[\\r]}}\\end\n"
            ),
        ),
    ] {
        let d = scratch(&format!("fault6-{fault}"));
        std::fs::write(d.join("t.tex"), &src).unwrap();
        let clean = run_env(&d, "verify", names, true, None);
        assert_eq!(
            stat(&clean.stats, "verify_differences"),
            0,
            "{fault}: {}",
            clean.stats
        );
        let faulty = run_env(&d, "verify", names, true, Some(fault));
        assert!(
            stat(&faulty.stats, "verify_differences") >= 1,
            "{fault} was not caught: {}",
            faulty.stats
        );
    }
    // the spaces after a replayed \ignorespaces body: horizontal mode
    // typesets them under the fault
    let d = scratch("fault6-no-skip-spaces");
    std::fs::write(
        d.join("t.tex"),
        format!(
            "{pre}\\showboxdepth=9 \\showboxbreadth=99 \\def\\ig#1{{\\def\\q{{#1}}\\ignorespaces}}\n\
             \\indent\\ig a x\\ig a   x\\showlists\\end\n"
        ),
    )
    .unwrap();
    let off = run_env(&d, "off", "ig", true, None);
    let on = run_env(&d, "on", "ig", true, None);
    let faulty = run_env(&d, "on", "ig", true, Some("no-skip-spaces"));
    assert_eq!(off.log, on.log);
    assert_eq!(stat(&faulty.stats, "args_replays"), 1, "{}", faulty.stats);
    assert_ne!(off.log, faulty.log, "no-skip-spaces was not caught");
}
