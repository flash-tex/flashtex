//! XeTeX intrinsic observations and semantic checks on real LaTeX packages.
//! The report is separate from TeX output; no mode replays into the run.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DOC: &str = r#"\documentclass{article}
\usepackage{siunitx}
\usepackage[unicode]{hyperref}
\makeatletter
\def\測試{\relax}
\g@addto@macro\pdfstringdefPreHook{\測試}
\makeatother
\begin{document}
\pdfstringdefPreHook
\pdfstringdefPreHook
\def\測試{\relax\relax}
\pdfstringdefPreHook
{\globaldefs=1 \pdfstringdefPreHook}
\afterassignment\relax \pdfstringdefPreHook
\tracingmacros=2 \pdfstringdefPreHook \tracingmacros=0
\section{A \SI{1}{\metre}}\label{a}
\end{document}
"#;

/// The hook reads `\IntrSrc` only through `\IntrInner`, so the guard's
/// direct dependencies miss a change to it. (`\relax`: `\begin{document}`
/// ends in `\ignorespaces`, whose expansion is not `big_switch`'s.)
const INDIRECT: &str = r#"\documentclass{article}
\def\IntrSrc{one}
\def\IntrInner{\edef\IntrOut{\IntrSrc}\let\IntrCopy\IntrSrc}
\def\pdfstringdefPreHook{\IntrInner}
\begin{document}\relax
\pdfstringdefPreHook
\pdfstringdefPreHook
\def\IntrSrc{two}
\pdfstringdefPreHook
\IntrOut\IntrCopy
\end{document}
"#;

struct Run {
    log: String,
    xdv: Vec<u8>,
    stats: String,
    status: Option<i32>,
}

fn run(exe: &Path, dir: &Path, format: &Path, mode: Option<&str>) -> Run {
    run_as(exe, dir, format, mode, false)
}

/// `errors`: a document with TeX errors, run on past them.
fn run_as(exe: &Path, dir: &Path, format: &Path, mode: Option<&str>, errors: bool) -> Run {
    // Each mode sees the same first-pass document, with no previous aux data.
    for name in ["main.aux", "main.out", "main.xdv"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    let stats = dir.join(format!("stats-{}.txt", mode.unwrap_or("default")));
    let mut cmd = Command::new(exe);
    cmd.current_dir(dir)
        .args([
            format!("-fmt={}", format.join("xelatex").display()),
            "-no-pdf".into(),
            "-interaction=nonstopmode".into(),
        ])
        .args((!errors).then_some("-halt-on-error"))
        .arg("main.tex")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_INTRINSICS_STATS", &stats)
        .stdin(Stdio::null());
    use std::os::unix::process::CommandExt;
    cmd.arg0("xelatex");
    if let Some(mode) = mode {
        cmd.env("FLASHTEX_INTRINSICS", mode);
    } else {
        cmd.env_remove("FLASHTEX_INTRINSICS");
    }
    let out = cmd.output().expect("run XeTeX");
    let log = std::fs::read_to_string(dir.join("main.log")).expect("main.log");
    assert!(
        out.status.success() || errors,
        "{mode:?}: status {}; log tail: {}\nstderr: {}",
        out.status,
        log.lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" / "),
        String::from_utf8_lossy(&out.stderr)
    );
    Run {
        log,
        xdv: std::fs::read(dir.join("main.xdv")).unwrap_or_default(),
        stats: std::fs::read_to_string(stats).unwrap_or_default(),
        status: out.status.code(),
    }
}

fn count(stats: &str, key: &str) -> u64 {
    stats
        .lines()
        .find_map(|l| l.split_once(": ").filter(|(k, _)| *k == key))
        .and_then(|(_, n)| n.parse().ok())
        .unwrap_or(0)
}

/// The engine, a fresh directory holding `doc` as main.tex, and the
/// xelatex format (None: no TeX Live).
fn setup(doc: &str) -> Option<(&'static Path, PathBuf, PathBuf)> {
    static FORMAT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    if flashtex_engine::resolver::discover_texlive().is_none() {
        if std::env::var_os("FLASHTEX_REQUIRE_TEXLIVE").is_some_and(|v| v == "1") {
            panic!("no TeX Live found with FLASHTEX_REQUIRE_TEXLIVE=1");
        }
        eprintln!("skipped: no TeX Live");
        return None;
    }
    let exe = Path::new(env!("CARGO_BIN_EXE_flashtex-xetex"));
    // (the tests run at once: the clock alone may name two alike)
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "flashtex-xetex-intr-{}-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.tex"), doc).unwrap();
    // (the cache directory is the process's environment: one test at a time)
    let _one = FORMAT.lock().unwrap_or_else(|p| p.into_inner());
    std::env::set_var(
        "FLASHTEX_FORMAT_CACHE_DIR",
        std::env::temp_dir().join("flashtex-xetex-intr-format-cache"),
    );
    let children = flashtex_xetex::host::proc::Children::new(exe).unwrap();
    let format = flashtex_xetex::host::format::ensure(exe, "xelatex", &children)
        .expect("build FlashTeX xelatex format");
    std::env::remove_var("FLASHTEX_FORMAT_CACHE_DIR");
    Some((exe, dir, format))
}

fn assert_same_output(a: &Run, b: &Run) {
    assert_eq!(a.status, b.status, "exit status");
    if a.log != b.log {
        let first = a
            .log
            .lines()
            .zip(b.log.lines())
            .enumerate()
            .find(|(_, (x, y))| x != y);
        panic!("log changed at {first:?}");
    }
    assert_eq!(a.xdv, b.xdv, "XDV changed");
}

#[test]
fn packages_and_guards_are_observed_without_replay() {
    let Some((exe, dir, format)) = setup(DOC) else {
        return;
    };
    let default = run(exe, &dir, &format, None);
    let off = run(exe, &dir, &format, Some("off"));
    let on = run(exe, &dir, &format, Some("on"));
    let verify = run(exe, &dir, &format, Some("verify"));
    let report = run(exe, &dir, &format, Some("report"));
    for other in [&off, &on, &verify, &report] {
        assert_same_output(&default, other);
    }
    assert!(default.stats.is_empty() && off.stats.is_empty() && on.stats.is_empty());
    assert!(
        verify.stats.contains("replay_enabled: false"),
        "{}",
        verify.stats
    );
    eprintln!("XeTeX intrinsics verification:\n{}", verify.stats);
    // the packages' hook replays to the real path's state
    let checked = count(&verify.stats, "semantic.checked");
    assert!(checked > 0, "{}", verify.stats);
    assert_eq!(count(&verify.stats, "semantic.identical"), checked);
    assert_eq!(count(&verify.stats, "semantic.differing"), 0);
    for key in [
        "eligible",
        "unicode_dependency_tokens",
        "alternative.siunitx_context_calls",
        "alternative.bookmark_copy_calls",
        "recordings_kept",
        "semantic.unverified.no_recording",
        "rejected.DependenciesChanged",
        "rejected.GlobalDefs",
        "rejected.AfterAssignment",
        "rejected.Tracing",
    ] {
        assert!(count(&verify.stats, key) > 0, "{key}: {}", verify.stats);
    }
    assert_eq!(verify.stats, report.stats);
    std::fs::remove_dir_all(dir).unwrap();
}

/// The second call replays the first's recording to the same state. The
/// third follows a change the guard cannot see: the replay's `\edef` holds
/// the old expansion and the verifier reports `\IntrOut`; `\let` shares
/// the source's value as it is then, so `\IntrCopy` agrees. The output is
/// the real path's either way.
#[test]
fn an_indirect_change_is_a_semantic_difference() {
    let Some((exe, dir, format)) = setup(INDIRECT) else {
        return;
    };
    let default = run(exe, &dir, &format, None);
    let verify = run(exe, &dir, &format, Some("verify"));
    assert_same_output(&default, &verify);
    let s = &verify.stats;
    assert_eq!(count(s, "semantic.unverified.no_recording"), 1, "{s}");
    assert_eq!(count(s, "semantic.checked"), 2, "{s}");
    assert_eq!(count(s, "semantic.identical"), 1, "{s}");
    assert_eq!(count(s, "semantic.differing"), 1, "{s}");
    assert_eq!(count(s, "semantic.differing.eqtb"), 1, "{s}");
    assert!(
        // (token 0, the same in both, is the body's `end_match`)
        s.contains("semantic.detail: \\IntrOut: tokens from 1: replay one real two\n"),
        "{s}"
    );
    assert!(!s.contains("IntrCopy"), "{s}");
    // after two checked calls, as many checkpoints as before them (none)
    assert!(s.contains("\ncheckpoints: 0\n"), "{s}");
    std::fs::remove_dir_all(dir).unwrap();
}

/// Off-mode and verify-mode runs of `doc` (TeX errors allowed): the same
/// output and status; the verify report.
fn verify_matches_off(doc: &str) -> Option<String> {
    let (exe, dir, format) = setup(doc)?;
    let off = run_as(exe, &dir, &format, Some("off"), true);
    let verify = run_as(exe, &dir, &format, Some("verify"), true);
    assert_eq!(off.status, verify.status, "{}", verify.stats);
    assert_same_output(&off, &verify);
    std::fs::remove_dir_all(dir).unwrap();
    Some(verify.stats)
}

/// A recording made inside a group closes and opens one (`[End, Begin]`);
/// at the top level its replay would `unsave` at level one (TeX's
/// "confusion"). It is not replayed where the group level differs.
#[test]
fn a_group_end_recorded_at_another_level_is_not_replayed() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\def\\pdfstringdefPreHook{\\endgroup\\begingroup}\n\
         \\begin{document}x\\begingroup\\pdfstringdefPreHook\\endgroup\n\
         \\pdfstringdefPreHook\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "semantic.unverified.other_group"), 1, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 0, "{s}");
}

/// `new_graf` sets the enclosing list's `prev_graf` to 0 outside
/// `eq_define`. A call that starts and ends an empty paragraph (here on a
/// `\vbox`'s list that `\lastbox` has emptied after one line) appends
/// nothing and leaves mode, nest level and tail alone, so it was kept, and
/// its replay left `\prevgraf` at 1. The check of each later call differed
/// (`scalar cur_list`); the pdfTeX side is #2222. Not replayable (#2261).
/// (The primitives, with the primitive `\everypar` emptied: LaTeX's
/// paragraph hooks make glue definitions, which are not replayable either.)
#[test]
fn a_call_that_starts_an_empty_paragraph_is_not_replayable() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\n\
         \\expandafter\\let\\expandafter\\IntrN\\csname tex_noindent:D\\endcsname\n\
         \\expandafter\\let\\expandafter\\IntrP\\csname tex_par:D\\endcsname\n\
         \\expandafter\\let\\expandafter\\IntrE\\csname tex_everypar:D\\endcsname\n\
         \\def\\pdfstringdefPreHook{\\begingroup\\IntrN\\IntrP\\endgroup}\n\
         \\def\\IntrOne{\\setbox2\\vbox{\\IntrE{}x\\IntrP\\setbox3\\lastbox\\pdfstringdefPreHook\\relax\
         \\message{[\\the\\prevgraf]}}}\n\
         \\begin{document}\\relax\n\\IntrOne\\IntrOne\\IntrOne\n\
         x\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "calls"), 3, "{s}");
    assert_eq!(count(&s, "not_replayable.State"), 3, "{s}");
    assert_eq!(count(&s, "semantic.differing"), 0, "{s}");
}

/// `\wd` writes the box node in `mem`, which is not compared: a call
/// that sets it is not replayable (else its check would find no change).
#[test]
fn box_dimensions_are_not_replayable() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\setbox0\\hbox{}\n\
         \\def\\pdfstringdefPreHook{\\wd0=\\dimexpr\\wd0+1pt\\relax}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\n\\the\\wd0\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.BoxDimen"), 3, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 0, "{s}");
}

/// `\immediate\write` changes a file, which is not compared: a call that
/// writes is not replayable.
#[test]
fn immediate_writes_are_not_replayable() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\newwrite\\IntrW\\immediate\\openout\\IntrW=\\jobname.w\n\
         \\def\\pdfstringdefPreHook{\\immediate\\write\\IntrW{x}}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\nx\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.FileOutput"), 3, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 0, "{s}");
}

/// `\read` moves an input file's position, which is not compared: with a
/// file of equal lines, a replay would define the same macro and its check
/// pass, while the real path read a line further (#2141). Not replayable.
#[test]
fn file_reads_are_not_replayable() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\newwrite\\IntrW\\immediate\\openout\\IntrW=\\jobname.dat\n\
         \\immediate\\write\\IntrW{x}\\immediate\\write\\IntrW{x}\\immediate\\write\\IntrW{x}\n\
         \\immediate\\write\\IntrW{x}\\immediate\\closeout\\IntrW\n\
         \\newread\\IntrR\\openin\\IntrR=\\jobname.dat\n\
         \\def\\pdfstringdefPreHook{\\read\\IntrR to\\IntrL}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\n\\IntrL\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.FileInput"), 3, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 0, "{s}");
}

/// `\openin` and `\closein` open and close an input file, whose position
/// is not compared: not replayable (#2141).
#[test]
fn opening_and_closing_input_files_is_not_replayable() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\newread\\IntrR\n\
         \\def\\pdfstringdefPreHook{\\closein\\IntrR\\openin\\IntrR=\\jobname.tex\\relax}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\nx\\closein\\IntrR\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.FileInput"), 3, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 0, "{s}");
}

/// An `\aftergroup` token at the call's own level stays on the save stack
/// after the call, which a replay would not push (its check found a
/// difference, #2141): not replayable. One inside a group the call opens
/// comes back at that group's end, within the call: replayable, and its
/// checks find nothing.
#[test]
fn aftergroup_at_the_calls_level_is_not_replayable() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\def\\pdfstringdefPreHook{\\aftergroup\\relax}\n\
         \\begin{document}x\\begingroup\\pdfstringdefPreHook\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\\endgroup\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.AfterGroup"), 3, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 0, "{s}");
    assert_eq!(count(&s, "semantic.differing"), 0, "{s}");
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\def\\IntrX{\\gdef\\IntrY{y}}\n\
         \\def\\pdfstringdefPreHook{\\begingroup\\aftergroup\\IntrX\\endgroup}\n\
         \\begin{document}x\\begingroup\\pdfstringdefPreHook\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\\endgroup\\IntrY\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.AfterGroup"), 0, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 2, "{s}");
    assert_eq!(count(&s, "semantic.differing"), 0, "{s}");
}

/// A recorded write to a `\tracing...` parameter is not replayable: a
/// replay's group ends trace (or not) as the parameters are then, and the
/// recording would print where the real path did not (#2141).
#[test]
fn tracing_writes_are_not_replayable() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\def\\pdfstringdefPreHook{\\tracingnesting=0 }\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\nx\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.Tracing"), 3, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 0, "{s}");
}

/// A check compares the replay of the previous call's recording with the
/// real path. When the real path itself is not replayable (here it writes
/// to a file, which is not compared, after a call that did not), the two
/// states agree although a replay would be wrong: the check is a
/// difference (`semantic.differing.not_replayable`), not a pass (#2203).
/// (The body ends in `\relax`, a command, so that the recording ends at
/// the body, before the next token is read.)
#[test]
fn a_check_whose_real_path_is_not_replayable_is_no_pass() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\newwrite\\IntrW\\immediate\\openout\\IntrW=\\jobname.w\n\
         \\newcount\\IntrC\\def\\pdfstringdefPreHook{\\ifodd\\IntrC\\immediate\\write\\IntrW{x}\\fi\\relax}\n\
         \\begin{document}\\relax\n\\IntrC=1 \\pdfstringdefPreHook\n\\IntrC=0 \\pdfstringdefPreHook\n\
         \\IntrC=1 \\pdfstringdefPreHook\nx\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "semantic.checked"), 1, "{s}");
    assert_eq!(count(&s, "semantic.identical"), 0, "{s}");
    assert_eq!(count(&s, "semantic.differing.not_replayable"), 1, "{s}");
}

/// A body whose last tokens only expand (`\fi`) is used up inside
/// `big_switch`'s `get_x_token`, which reads the token after the call: the
/// recording ends where the body is popped, before that `\relax` (or
/// `\count1=1`) is read, where the replay's state was taken. Compared after
/// the command, the file position (and the count) differed on a correct
/// replay (#2210).
#[test]
fn a_body_ending_in_an_expansion_is_compared_at_its_end() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\def\\pdfstringdefPreHook{\\ifx aa\\fi}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\\relax\n\\pdfstringdefPreHook\\relax\n\
         \\pdfstringdefPreHook\\count1=1 \\pdfstringdefPreHook\\count1=2 \\pdfstringdefPreHook\\relax\n\
         x\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "semantic.checked"), 4, "{s}");
    assert_eq!(count(&s, "semantic.identical"), 4, "{s}");
    assert_eq!(count(&s, "semantic.differing"), 0, "{s}");
}

/// `\XeTeXinputencoding` sets how the current input file's next lines are
/// decoded (`set_input_file_encoding`, Host-side state no check compares),
/// which a replay would not do: not replayable, as `\read` is (#2176's
/// question 2). Once the name's string exists, nothing compared moves.
#[test]
fn input_encoding_changes_are_not_replayable() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\def\\pdfstringdefPreHook{\\XeTeXinputencoding \"utf8\"\\relax}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\nx\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.FileInput"), 3, "{s}");
    assert_eq!(count(&s, "semantic.checked"), 0, "{s}");
}

/// A reason that only means the recorder could not keep the real path
/// (`TooManyOps`: the loop makes 10,006 assignments) is no difference when
/// the states agree: everything such a path changes is compared, and here
/// it ends where the previous call's one assignment does. The check is
/// unverified, not a difference (REVIEW-C2 on #2205); a sweep in verify
/// mode stays green on it.
#[test]
fn a_check_whose_real_path_was_too_long_to_record_is_no_difference() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\countdef\\IntrC=200 \\countdef\\IntrK=201\n\
         \\def\\IntrLoop{\\advance\\IntrK by 1 \\ifnum\\IntrK<10005 \\expandafter\\IntrLoop\\fi}\n\
         \\def\\pdfstringdefPreHook{\\ifodd\\IntrC\\IntrLoop\\fi\\IntrK=0 \\relax}\n\
         \\begin{document}\\relax\n\\IntrC=0 \\pdfstringdefPreHook\n\\IntrC=1 \\pdfstringdefPreHook\n\
         x\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "not_replayable.TooManyOps"), 1, "{s}");
    assert_eq!(count(&s, "semantic.unverified.TooManyOps"), 1, "{s}");
    assert_eq!(count(&s, "semantic.differing"), 0, "{s}");
    assert_eq!(count(&s, "semantic.identical"), 0, "{s}");
}

/// A value longer than a million tokens (`\IntrT` doubled 18 times from
/// four, 1,048,576 `\relax`; the main memory holds about four such lists,
/// so `\IntrE` is let go) was recorded cut short at its millionth token,
/// and the check compared two lists by their first million only. So the
/// second call, whose real path appends an `x` at the end, passed against
/// the first call's replay, which a replay would get wrong twice over (the
/// pdfTeX side's over-long key values, #2194). Every token is compared.
#[test]
fn a_value_longer_than_a_million_tokens_is_compared_to_its_end() {
    let Some(s) = verify_matches_off(
        "\\documentclass{article}\\newtoks\\IntrT \\IntrT{\\relax\\relax\\relax\\relax}\n\
         \\def\\IntrD{\\edef\\IntrE{\\the\\IntrT\\the\\IntrT}\\IntrT\\expandafter{\\IntrE}}\n\
         \\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\
         \\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\IntrD\\let\\IntrE\\relax\n\
         \\def\\IntrY{\\edef\\IntrX{\\the\\IntrT}}\\def\\pdfstringdefPreHook{\\IntrY}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\n\
         \\IntrT\\expandafter{\\the\\IntrT x}\\pdfstringdefPreHook\n\
         x\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&s, "semantic.checked"), 1, "{s}");
    assert_eq!(count(&s, "semantic.identical"), 0, "{s}");
    assert_eq!(count(&s, "semantic.differing.eqtb"), 1, "{s}");
}
