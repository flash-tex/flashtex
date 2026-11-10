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
    run_in(dir, mode, names, false)
}

/// [`run`], in e-TeX mode with `etex`.
fn run_in(dir: &Path, mode: &str, names: &str, etex: bool) -> Run {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let stats = dir.join(format!("stats-{mode}.txt"));
    let _ = std::fs::remove_file(&stats);
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(if etex {
            &["-ini", "-etex", "\\input t"][..]
        } else {
            &["-ini", "\\input t"][..]
        })
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_INTRINSICS", mode)
        .env("FLASHTEX_INTRINSIC_NAMES", names)
        .env("FLASHTEX_INTRINSICS_STATS", &stats)
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
    check_in(tag, src, false)
}

/// [`check`], in e-TeX mode with `etex`.
fn check_in(tag: &str, src: &str, etex: bool) -> String {
    let d = scratch(tag);
    std::fs::write(d.join("t.tex"), src).unwrap();
    let off = run_in(&d, "off", "hook", etex);
    let on = run_in(&d, "on", "hook", etex);
    let verify = run_in(&d, "verify", "hook", etex);
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
    // each check drops its checkpoint (#2141)
    assert_eq!(
        stat(&verify.stats, "checkpoints_at_end"),
        0,
        "{tag}: {}",
        verify.stats
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

/// #2141: a check dropped its checkpoint with `retain_checkpoints`, which
/// keeps the newest, so every check left one behind (its log kept every
/// chunk the run wrote after it, and write logging stayed on). Over many
/// checks, a `\shipout` and a redefined dependency (recorded again), the
/// run ends with no checkpoint, its log as with the intrinsics off.
#[test]
fn checks_leave_no_checkpoint() {
    let d = scratch("orphan");
    let calls = "\\hook\\show".repeat(12);
    std::fs::write(
        d.join("t.tex"),
        format!("{PRELUDE}{calls}\\shipout\\hbox{{}}{calls}\\def\\a{{x}}{calls}\\end\n"),
    )
    .unwrap();
    let off = run(&d, "off", "hook");
    let verify = run(&d, "verify", "hook");
    assert_eq!(off.log, verify.log);
    assert_eq!(
        stat(&verify.stats, "verify_differences"),
        0,
        "{}",
        verify.stats
    );
    assert!(stat(&verify.stats, "verified") >= 30, "{}", verify.stats);
    assert_eq!(
        stat(&verify.stats, "checkpoints_at_end"),
        0,
        "{}",
        verify.stats
    );
}

/// The `\tracingstats` memory lines of a log (`Memory usage before: ...`).
fn memory_lines(log: &str) -> Vec<&str> {
    log.lines()
        .filter(|l| l.starts_with("Memory usage"))
        .collect()
}

/// #2039: a recording kept the lists it read and defined in `mem` (pinned),
/// so at every shipout `dyn_used` (the one-word nodes in use, the second
/// number of each pair) was higher than pdfTeX's by their size. With the
/// lists copied out of `mem`, the memory lines are TeX Live's pdfTeX's at
/// every shipout while the hook is recorded, replayed, redefined around and
/// recorded again.
#[test]
fn memory_usage_is_pdftex_s_while_replaying() {
    let Some(texbin) = flashtex_engine::resolver::find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let d = scratch("mem");
    let ship = "\\shipout\\hbox{}";
    let src = format!(
        "{PRELUDE}\\tracingstats=2 \\hook{ship}\\hook\\show{ship}\\hook\\hook\\show{ship}\
         \\def\\a{{AA}}\\hook\\show{ship}\\hook\\hook\\show{ship}\\def\\w{{gone}}\\let\\z\\relax{ship}\
         \\hook\\hook\\show{ship}\\end\n"
    );
    std::fs::write(d.join("t.tex"), &src).unwrap();
    let on = run(&d, "on", "hook");
    assert!(stat(&on.stats, "replays") >= 4, "{}", on.stats);
    let tex = d.join("tex");
    std::fs::create_dir_all(&tex).unwrap();
    std::fs::write(tex.join("t.tex"), &src).unwrap();
    let out = Command::new(texbin.join("pdftex"))
        .args(["-ini", "\\input t"])
        .current_dir(&tex)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .output()
        .expect("run pdftex");
    assert!(out.status.code().is_some(), "pdftex: {out:?}");
    let reference = std::fs::read_to_string(tex.join("t.log")).unwrap();
    let want = memory_lines(&reference);
    assert_eq!(want.len(), 7, "pdftex's log:\n{reference}");
    assert_eq!(
        memory_lines(&on.log),
        want,
        "the memory lines differ from pdftex's"
    );
}

/// `\globaldefs<0` turns every global assignment local (tex.web §1214): a
/// replay of the hook's `\global` and `\gdef` operations would be wrong, so
/// a negative value, like a positive one, is a precondition failure. Inside
/// the group the hook's global assignments are local, and once the group
/// ends `\n` and `\last` are again what was set before it.
#[test]
fn negative_globaldefs_falls_back() {
    let s = check(
        "pre-neg",
        &format!(
            "{PRELUDE}\\hook\\hook\\show\\n=99 \\def\\last{{X}}\\show\n\
             {{\\globaldefs=-1 \\hook\\show\\hook\\show\\hook\\show}}\\show\n\
             \\hook\\show\\n=98 \\def\\last{{Y}}\\globaldefs=-1 \\hook\\show\\globaldefs=0 \\show\\end\n"
        ),
    );
    assert_eq!(stat(&s, "GlobalDefs"), 4, "{s}");
}

/// Each tracing parameter on its own stops a replay: the normal path would
/// print what it traces (here `\tracingifs` the hook's `\ifx`, and
/// `\tracingassigns` its assignments) and a replay prints nothing. In
/// e-TeX mode: four of them are e-TeX's.
#[test]
fn each_tracing_parameter_falls_back_alone() {
    for p in [
        "tracingmacros",
        "tracingcommands",
        "tracingassigns",
        "tracingrestores",
        "tracinggroups",
        "tracingifs",
    ] {
        let s = check_in(
            &format!("trace-{p}"),
            &format!("{PRELUDE}\\hook\\hook\\{p}=1 \\hook\\show\\{p}=0 \\hook\\show\\end\n"),
            true,
        );
        assert_eq!(stat(&s, "Tracing"), 1, "{p}: {s}");
    }
}

/// A dependency redefined globally (`\gdef`, `geq_define`) fails the guard
/// as a local redefinition does (`eq_define`).
#[test]
fn a_global_redefinition_of_a_dependency_falls_back() {
    let s = check(
        "gredef",
        &format!(
            "{PRELUDE}\\hook\\hook\\show\n\\gdef\\a{{AA}}\\hook\\show\\hook\\show\n\
             {{\\global\\let\\b=\\a}}\\hook\\show\\end\n"
        ),
    );
    assert!(stat(&s, "Deps") >= 2, "{s}");
}

/// `\partokenname` (pdfTeX 1.40.24) changes the token a non-`\long`
/// macro's argument may not hold. Recorded while it is `\x`, the hook
/// passes `\par` to such a macro; once it is `\par` again the normal path
/// stops with "Runaway argument", which a replay would skip.
#[test]
fn a_changed_par_token_falls_back() {
    let s = check(
        "partoken",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\foo#1{\\def\\got{#1}}\\def\\hook{\\foo{\\par}}\n\
         \\partokenname\\x \\hook\\hook\\hook\\message{[\\meaning\\got]}\n\
         \\partokenname\\par \\hook\\message{[\\meaning\\got]}\\end\n",
    );
    assert!(stat(&s, "replays") >= 1, "{s}");
    assert_eq!(stat(&s, "ParToken"), 1, "{s}");
}

/// `\escapechar` is the first integer pair a recording keeps (the guard
/// checks it with the macro's own meaning): a hook that prints a control
/// sequence's name gives another name once `\escapechar` changes.
#[test]
fn a_changed_escapechar_falls_back() {
    let s = check(
        "escape",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n\
         \\def\\hook{\\edef\\w{\\string\\relax}}\n\
         \\hook\\hook\\hook\\message{[\\meaning\\w]}\n\
         \\escapechar=`\\/ \\hook\\message{[\\meaning\\w]}\\hook\\message{[\\meaning\\w]}\\end\n",
    );
    assert!(stat(&s, "replays") >= 2, "{s}");
    assert!(stat(&s, "WordDeps") >= 1, "{s}");
}

/// `align_state` is part of the guard: recorded in a paragraph, a hook
/// whose body holds an alignment tab is called again in an `\halign` cell,
/// where the normal path's `&` ends the cell (tex.web §342) and a replay
/// would only `\let`.
#[test]
fn a_changed_align_state_falls_back() {
    let s = check(
        "align",
        "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\catcode`\\&=4 \\scrollmode\n\
         \\def\\hook{\\let\\z=&}\n\
         \\indent\\hook\\hook\\hook\\message{[\\meaning\\z]}\\par\n\
         \\halign{#\\cr\\relax\\hook\\message{[\\meaning\\z]}\\cr}\\message{[\\meaning\\z]}\\end\n",
    );
    assert!(stat(&s, "replays") >= 1, "{s}");
    assert!(stat(&s, "Align") >= 1, "{s}");
}
