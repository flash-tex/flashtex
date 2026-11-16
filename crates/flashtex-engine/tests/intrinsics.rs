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

thread_local! {
    /// More of the environment for this test's engines (each test runs on a
    /// thread of its own).
    static ENV: std::cell::RefCell<Vec<(&'static str, &'static str)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

struct Run {
    log: String,
    stats: String,
    /// Standard error.
    err: String,
}

/// Run `\input t` in INITEX with the intrinsics in `mode`.
fn run(dir: &Path, mode: &str, names: &str) -> Run {
    run_in(dir, mode, names, false)
}

/// [`run`], in e-TeX mode with `etex`.
fn run_in(dir: &Path, mode: &str, names: &str, etex: bool) -> Run {
    run_line(dir, mode, names, etex, "\\input t")
}

/// [`run_in`], with `line` as the first line (the command line: input
/// level 0, below every file).
fn run_line(dir: &Path, mode: &str, names: &str, etex: bool, line: &str) -> Run {
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let stats = dir.join(format!("stats-{mode}.txt"));
    let _ = std::fs::remove_file(&stats);
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(if etex {
            vec!["-ini", "-etex", line]
        } else {
            vec!["-ini", line]
        })
        .current_dir(dir)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_INTRINSICS", mode)
        .env("FLASHTEX_INTRINSIC_NAMES", names)
        .env("FLASHTEX_INTRINSICS_STATS", &stats)
        .envs(ENV.with(|e| e.borrow().clone()))
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
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
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

/// In `error_stop_mode` an error answered with an empty line changes no
/// count (`error` returns before `incr(error_count)`), `history` is already
/// `error_message_issued` after an earlier error, and before the log is
/// open (the first line, not `\input`) it writes no log and leaves the
/// terminal at the start of a line. Before, the recording took such a run
/// for pure and the replays dropped two of the three `Invalid code` errors.
#[test]
fn an_error_answered_at_the_terminal_is_not_pure() {
    let d = scratch("err");
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let errors = |mode: &str| {
        let stats = d.join(format!("stats-{mode}.txt"));
        let mut child = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
            .args([
                "-ini",
                "\\catcode`\\{=1 \\catcode`\\}=2 \\undefined\
                 \\def\\hook{\\catcode1=300 }\\hook\\hook\\hook\\end",
            ])
            .current_dir(&d)
            .env("FLASHTEX_POOL", &pool)
            .env("FLASHTEX_RESOLVER", "cwd")
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FORCE_SOURCE_DATE", "1")
            .env("FLASHTEX_INTRINSICS", mode)
            .env("FLASHTEX_INTRINSIC_NAMES", "hook")
            .env("FLASHTEX_INTRINSICS_STATS", &stats)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run flashtex-initex");
        use std::io::Write;
        // an empty answer to each prompt (EOF at a prompt is a fatal error)
        let _ = child.stdin.take().unwrap().write_all(b"\n\n\n\n\n\n");
        let out = child.wait_with_output().unwrap();
        let term = String::from_utf8_lossy(&out.stdout).into_owned();
        let s = std::fs::read_to_string(&stats).unwrap_or_default();
        (term.matches("! Invalid code").count(), s)
    };
    let (off, _) = errors("off");
    assert_eq!(off, 3);
    let (on, s) = errors("on");
    assert_eq!(on, 3, "{s}");
    assert_eq!(stat(&s, "replays"), 0, "{s}");
    assert_eq!(stat(&s, "Error"), 1, "{s}");
    let (verify, s) = errors("verify");
    assert_eq!(verify, 3, "{s}");
    assert_eq!(stat(&s, "verify_differences"), 0, "{s}");
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

/// Both paths make a `\def`'s list from the same free node, so the `eqtb`
/// entry they leave is the same word and only the tokens at that node can
/// tell them apart. A replay that ends each list it makes with another
/// token (`FLASHTEX_INTRINSICS_FAULT=token`) must be a difference in every
/// verified call; before, the verifier compared such a list only by its
/// reference count and passed both calls.
#[test]
fn verify_compares_a_list_at_the_same_node_by_its_tokens() {
    let d = scratch("samenode");
    std::fs::write(
        d.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\scrollmode\\def\\a{A}\\def\\b{B}\
         \\def\\hook{\\def\\y{\\a\\b}}\n\
         \\hook\\hook\\message{[\\meaning\\y]}\\hook\\message{[\\meaning\\y]}\\end\n",
    )
    .unwrap();
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let stats = d.join("stats-fault.txt");
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(["-ini", "\\input t"])
        .current_dir(&d)
        .env("FLASHTEX_POOL", pool)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_INTRINSICS", "verify")
        .env("FLASHTEX_INTRINSIC_NAMES", "hook")
        .env("FLASHTEX_INTRINSICS_FAULT", "token")
        .env("FLASHTEX_INTRINSICS_STATS", &stats)
        .stdin(Stdio::null())
        .output()
        .expect("run flashtex-initex");
    assert!(out.status.code().is_some(), "{out:?}");
    let s = std::fs::read_to_string(&stats).unwrap();
    assert_eq!(stat(&s, "verified"), 2, "{s}");
    assert_eq!(stat(&s, "verify_differences"), 2, "{s}");
    assert!(s.contains("same node, tokens"), "{s}");
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

#[test]
fn a_replay_keeps_the_normal_paths_capacity_errors() {
    // (lane AUDIT-OVERFLOW, #2308) a replay puts nothing in the buffer or
    // the pool and pushes no input level: a call recorded with room to
    // spare was replayed where its normal path stops with `TeX capacity
    // exceeded` (on main: `[survived]`, exit 0), or, for the groups it
    // makes again, stopped there with the replay's context instead of the
    // call's
    let h = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\n";
    let mut cases = common::capacity_cases("hook", "\\global\\count2=1 ", 3000, 6188);
    // four macro levels, 9993 levels deep (stack_size=10000)
    cases.push((
        "! TeX capacity exceeded, sorry [input stack size=10000]",
        format!(
            "{h}\\def\\ia{{\\ib\\relax}}\\def\\ib{{\\ic\\relax}}\\def\\ic{{\\id\\relax}}\\def\\id{{\\ie\\relax}}\n\
             \\def\\ie{{\\if\\relax\\relax\\global\\count2=2 \\fi\\relax}}\n\
             \\def\\hook{{\\begingroup\\ia\\endgroup}}\n\\hook\\hook\\hook\n\
             \\def\\s{{\\relax\\hook\\relax}}\
             \\def\\r{{\\advance\\count1 1 \\ifnum\\count1<9993 \\expandafter\\r\\else\\expandafter\\s\\fi\\relax}}\n\
             \\r\\message{{[survived]}}\\end\n"
        ),
    ));
    // six groups at level 250 (grouping levels=255)
    cases.push((
        "! TeX capacity exceeded, sorry [grouping levels=255]",
        format!(
            "{h}\\def\\hook{{\\begingroup\\begingroup\\begingroup\\begingroup\\begingroup\\begingroup\
             \\global\\count2=2 \\endgroup\\endgroup\\endgroup\\endgroup\\endgroup\\endgroup}}\n\
             \\hook\\hook\\hook\n\
             \\def\\nest{{\\begingroup\\advance\\count1 1 \\ifnum\\count1<250 \\expandafter\\nest\\fi}}\n\
             \\nest\\relax\\hook\\message{{[survived]}}\\end\n"
        ),
    ));
    for (i, (want, src)) in cases.into_iter().enumerate() {
        let tag = format!("capacity-{i}");
        let d = scratch(&tag);
        std::fs::write(d.join("t.tex"), &src).unwrap();
        let off = run(&d, "off", "hook");
        assert!(off.log.contains(want), "{tag}: {}", off.log);
        let s = check(&tag, &src);
        assert!(stat(&s, "replays") >= 2, "{tag}: {s}");
    }
}

#[test]
fn a_replay_raises_the_stack_statistics_as_its_normal_path() {
    // (#2451) a replay pushes no input level or parameter, so it left the
    // high-water marks of `\tracingstats`' "stack positions" line below the
    // normal path's: at input depth 5000, off printed ` 5009i,...` and on
    // ` 5002i,...`, and verification mode did not see it
    let h = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\#=6 \\scrollmode\\tracingstats=1\n";
    let src = format!(
        "{h}\\def\\ia{{\\ib\\relax}}\\def\\ib{{\\ic\\relax}}\\def\\ic{{\\id\\relax}}\\def\\id{{\\ie\\relax}}\n\
         \\def\\ie{{\\if\\relax\\relax\\global\\count2=2 \\fi\\relax}}\n\
         \\def\\hook{{\\begingroup\\ia\\endgroup}}\n\\hook\\hook\\hook\n\
         \\def\\s{{\\relax\\hook\\relax}}\
         \\def\\r{{\\advance\\count1 1 \\ifnum\\count1<5000 \\expandafter\\r\\else\\expandafter\\s\\fi\\relax}}\n\
         \\r\\end\n"
    );
    let d = scratch("stack-stats");
    std::fs::write(d.join("t.tex"), &src).unwrap();
    let off = run(&d, "off", "hook");
    assert!(off.log.contains("stack positions"), "{}", off.log);
    let s = check("stack-stats", &src);
    assert!(stat(&s, "replays") >= 2, "{s}");

    // The save stack's mark is the replay's own to raise: its `eq_define`
    // saves what this state calls for. The first call's `\count3=7` saves
    // the old value (the mark is raised to `save_ptr` there, two words
    // below where it then stands); the next two find `\count3` at the
    // group's level and save nothing, so the mark stays. A replay that
    // raised it by the recorded peak printed two more words than off
    // (`9s` for `7s`)
    let src = format!("{h}\\def\\hook{{\\count3=7 \\global\\count2=2 }}\n\\begingroup\\hook\\hook\\hook\\endgroup\\end\n");
    let s = check("stack-stats-save", &src);
    assert!(stat(&s, "replays") >= 2, "{s}");

    // A call from input level 0 (the command line) that pushes nothing
    // but its body (no macro, and no `back_input`: a `\let`) leaves the
    // lowered mark at 0, which is still a push at the call's level:
    // replayed one level above every earlier push (in `\s`, 5000 levels
    // deep), it raises the mark by that level (`5001i`, not `5000i`)
    let src = format!(
        "{h}\\def\\hook{{\\global\\let\\x\\relax}}\n\
         \\def\\s{{\\relax\\hook\\relax}}\
         \\def\\r{{\\advance\\count1 1 \\ifnum\\count1<5000 \\expandafter\\r\\else\\expandafter\\s\\fi\\relax}}\n"
    );
    let d = scratch("stack-stats-level0");
    std::fs::write(d.join("t.tex"), &src).unwrap();
    let line = "\\input t \\hook\\hook\\hook\\r\\end";
    let off = run_line(&d, "off", "hook", false, line);
    let on = run_line(&d, "on", "hook", false, line);
    let verify = run_line(&d, "verify", "hook", false, line);
    assert!(off.log.contains("stack positions"), "{}", off.log);
    assert_eq!(off.log, on.log, "level 0: log with intrinsics on differs");
    assert_eq!(off.log, verify.log, "level 0: log in verification mode");
    assert!(stat(&on.stats, "replays") >= 2, "{}", on.stats);
}

#[test]
fn a_replay_keeps_main_memorys_overflow() {
    // (#2309) a replay scans none of the call's tokens: a call whose
    // `\pdfstrcmp` argument is 30,000 tokens long (`\romannumeral`'s `m`s,
    // in a list and in the list `\pdfstrcmp` scans them into), recorded
    // with room to spare, was replayed where about 18,000 words are left
    // and its normal path stops with `main memory size`
    // (the offer's own test, without the rule that would then start the run
    // over because calls were replayed before: `src/capacity.rs`)
    ENV.with(|e| e.borrow_mut().push(("FLASHTEX_MEM_RERUN", "0")));
    let src = common::memory_case(
        "hook",
        "\\def\\hook{\\begingroup\\global\\count2=1 \\ifnum\\pdfstrcmp{\\romannumeral30000000}{}=0 \\fi\\endgroup}",
        "",
        497,
    );
    let d = scratch("capacity-mem");
    std::fs::write(d.join("t.tex"), &src).unwrap();
    let off = run(&d, "off", "hook");
    assert!(off.log.contains(common::MEMORY_FULL), "{}", off.log);
    let s = check("capacity-mem", &src);
    assert!(stat(&s, "replays") >= 2, "{s}");
}

#[test]
fn a_run_that_replayed_and_runs_out_of_main_memory_is_made_again() {
    // (#2309, the ruling's second half) `\keep`'s 60,000 tokens take up
    // the one-word nodes the first calls freed (a `\def`, whose body is
    // stored as it is read, with no list beside it), so the fourth call's
    // normal path lowers `hi_mem_min` by 60,000 words for its own and a
    // replay does not. Those words are free again after the call, but only
    // for one-word nodes: the boxes that follow, six copies of a box of
    // 1,000 kerns each (24,000 words of variable-size memory), ran out of
    // memory later with the intrinsics on. The engine now starts over
    // without replays
    let src = common::memory_drift_case(
        "hook",
        "\\def\\kloop{\\ifnum\\count3<1000 \\advance\\count3 1 \\kern0.1pt \\expandafter\\kloop\\fi}\n\
         \\setbox0\\hbox{\\count3=0 \\kloop}\n\
         \\def\\hook{\\begingroup\\global\\count2=1 \\ifnum\\pdfstrcmp{\\romannumeral30000000}{}=0 \\fi\\endgroup}",
        &format!("\\def\\keep{{{}}}", "k".repeat(60_000)),
        "\\global\\setbox\\count1=\\hbox{\\copy0\\copy0\\copy0\\copy0\\copy0\\copy0}",
    );
    let d = scratch("memory-drift");
    std::fs::write(d.join("t.tex"), &src).unwrap();
    let off = run(&d, "off", "hook");
    assert!(off.log.contains(common::MEMORY_FULL), "{}", off.log);
    assert!(!off.err.contains(common::MEMORY_AGAIN), "{}", off.err);
    let on = run(&d, "on", "hook");
    assert_eq!(off.log, on.log, "log with intrinsics on differs");
    assert!(on.err.contains(common::MEMORY_AGAIN), "{}", on.err);
    // the second piece fired (the replaying run overflowed), not the third
    assert!(on.err.contains(common::MEMORY_RAN_OUT), "{}", on.err);
    assert!(!on.err.contains(common::MEMORY_NEARLY), "{}", on.err);
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
