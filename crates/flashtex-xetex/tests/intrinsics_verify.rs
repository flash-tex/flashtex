//! XeTeX's guarded intrinsics on real LaTeX packages (`src/intrinsics.rs`):
//! `off`, `on` (replay) and `verify` (every admitted call checked against
//! the real path) give the same log, XDV and PDF; what must not replay does
//! not; and faults put into the replay or the read set on purpose are seen
//! by verify.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// siunitx fills hyperref's hook; `\section` and `\label` make
/// `\pdfstringdef` calls (bookmarks), and so does every page. A unit
/// declared again mid-document (`\DeclareSIUnit\metre`) changes what the
/// hook's `\cs_set_eq:Nc` copies, through no token of the hook's own body;
/// so does a redefinition of `\IntrDeep`, which the hook reaches only
/// through `\IntrMid`. (Direct calls are made in a group, as
/// `\pdfstringdef` makes them: the hook's local changes stay inside.)
const DOC: &str = r#"\documentclass{article}
\usepackage{siunitx}
\usepackage[unicode]{hyperref}
\makeatletter
\def\測試{\relax}
\def\IntrDeep{one}
\def\IntrMid{\xdef\IntrSeen{\IntrDeep}}
\g@addto@macro\pdfstringdefPreHook{\測試\IntrMid}
\makeatother
\begin{document}\relax
\begingroup\pdfstringdefPreHook\endgroup
\begingroup\pdfstringdefPreHook\endgroup
\def\測試{\relax\relax}
\begingroup\pdfstringdefPreHook\endgroup
\section{A \SI{1}{\metre}}\label{a}
\begingroup\pdfstringdefPreHook\endgroup [\IntrSeen]\par\relax
\def\IntrDeep{two}
\begingroup\pdfstringdefPreHook\endgroup [\IntrSeen]\par\relax
\DeclareSIUnit\metre{mmm}
\section{B \SI{2}{\metre}}
\begingroup\pdfstringdefPreHook\endgroup [\IntrSeen]\par\relax
\newpage
\section{C \SI{3}{\metre}}\label{c}
See \ref{a} and \ref{c}.
\newpage
D
\newpage
E
\end{document}
"#;

/// The call's own preconditions: `\globaldefs`, `\afterassignment`, the
/// tracing parameters (`\tracingstats` above LaTeX's 1).
const REJECT: &str = r#"\documentclass{article}
\def\IntrInner{\edef\IntrOut{x}}
\def\pdfstringdefPreHook{\IntrInner}
\begin{document}\relax
\pdfstringdefPreHook
{\globaldefs=1 \pdfstringdefPreHook}
\afterassignment\relax \pdfstringdefPreHook
\tracingmacros=2 \pdfstringdefPreHook \tracingmacros=0
\tracingassigns=1 \pdfstringdefPreHook \tracingassigns=0
\tracingstats=2 \pdfstringdefPreHook \tracingstats=1
\pdfstringdefPreHook
x
\end{document}
"#;

/// The hook reads `\IntrSrc` only through `\IntrInner`: the read set holds
/// it, so the third call is not replayed (stale, it would leave `one`).
const INDIRECT: &str = r#"\documentclass{article}
\def\IntrSrc{one}
\def\IntrInner{\edef\IntrOut{\IntrSrc}\let\IntrCopy\IntrSrc}
\def\pdfstringdefPreHook{\IntrInner}
\begin{document}\relax
\pdfstringdefPreHook
\pdfstringdefPreHook
\def\IntrSrc{two}
\pdfstringdefPreHook
\IntrOut\IntrCopy\par\relax
\pdfstringdefPreHook
\IntrOut\IntrCopy
\end{document}
"#;

/// `INDIRECT`'s hook with a group at its end, whose `unsave` is the last
/// operation (`drop-last`).
const FAULTS: &str = r#"\documentclass{article}
\def\IntrSrc{one}
\def\IntrInner{\edef\IntrOut{\IntrSrc}\let\IntrCopy\IntrSrc}
\def\pdfstringdefPreHook{\IntrInner\begingroup\endgroup}
\begin{document}\relax
\pdfstringdefPreHook
\pdfstringdefPreHook
\def\IntrSrc{two}
\pdfstringdefPreHook
\IntrOut\IntrCopy\par\relax
\pdfstringdefPreHook
\IntrOut\IntrCopy
\end{document}
"#;

struct Run {
    log: String,
    out: Vec<u8>,
    stats: String,
    status: Option<i32>,
    stderr: String,
}

#[derive(Clone, Copy, Default)]
struct Opts<'a> {
    mode: Option<&'a str>,
    /// a document with TeX errors, run on past them
    errors: bool,
    /// FlashTeX's PDF instead of XDV
    pdf: bool,
    fault: Option<&'a str>,
    fail: bool,
}

fn run(exe: &Path, dir: &Path, format: &Path, mode: Option<&str>) -> Run {
    run_with(
        exe,
        dir,
        format,
        Opts {
            mode,
            ..Opts::default()
        },
    )
}

fn run_with(exe: &Path, dir: &Path, format: &Path, o: Opts) -> Run {
    // Each mode sees the same first-pass document, with no previous aux data.
    for name in ["main.aux", "main.out", "main.xdv", "main.pdf"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    let stats = dir.join(format!(
        "stats-{}-{}.txt",
        o.mode.unwrap_or("default"),
        o.fault.unwrap_or("none")
    ));
    let _ = std::fs::remove_file(&stats);
    let mut cmd = Command::new(exe);
    cmd.current_dir(dir)
        .arg(format!("-fmt={}", format.join("xelatex").display()))
        .args((!o.pdf).then_some("-no-pdf"))
        .arg("-interaction=nonstopmode")
        .args((!o.errors).then_some("-halt-on-error"))
        .arg("main.tex")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_INTRINSICS_STATS", &stats)
        .env_remove("FLASHTEX_INTRINSICS_FAULT")
        .env_remove("FLASHTEX_INTRINSICS_VERIFY_FAIL")
        .stdin(Stdio::null());
    use std::os::unix::process::CommandExt;
    cmd.arg0("xelatex");
    match o.mode {
        Some(mode) => cmd.env("FLASHTEX_INTRINSICS", mode),
        None => cmd.env_remove("FLASHTEX_INTRINSICS"),
    };
    if let Some(f) = o.fault {
        cmd.env("FLASHTEX_INTRINSICS_FAULT", f);
    }
    if o.fail {
        cmd.env("FLASHTEX_INTRINSICS_VERIFY_FAIL", "1");
    }
    let out = cmd.output().expect("run XeTeX");
    let log = std::fs::read_to_string(dir.join("main.log")).expect("main.log");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success() || o.errors || o.fail,
        "{:?}: status {}; log tail: {}\nstderr: {stderr}",
        o.mode,
        out.status,
        log.lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" / "),
    );
    Run {
        log,
        out: std::fs::read(dir.join(if o.pdf { "main.pdf" } else { "main.xdv" }))
            .unwrap_or_default(),
        stats: std::fs::read_to_string(stats).unwrap_or_default(),
        status: out.status.code(),
        stderr,
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
    assert!(a.out == b.out, "output (XDV or PDF) changed");
}

/// Every check identical.
fn assert_verified(s: &str) {
    let checked = count(s, "verify.checked");
    assert_eq!(count(s, "verify.identical"), checked, "{s}");
    assert_eq!(count(s, "verify.differing"), 0, "{s}");
}

#[test]
fn the_packages_hook_replays_to_the_real_paths_output() {
    let Some((exe, dir, format)) = setup(DOC) else {
        return;
    };
    for pdf in [false, true] {
        let o = |mode| Opts {
            mode,
            pdf,
            ..Opts::default()
        };
        let default = run_with(exe, &dir, &format, o(None));
        let off = run_with(exe, &dir, &format, o(Some("off")));
        let on = run_with(exe, &dir, &format, o(Some("on")));
        let verify = run_with(exe, &dir, &format, o(Some("verify")));
        for other in [&off, &on, &verify] {
            assert_same_output(&default, other);
        }
        assert!(default.stats.is_empty() && off.stats.is_empty());
        eprintln!("on (pdf {pdf}):\n{}\nverify:\n{}", on.stats, verify.stats);
        assert!(count(&on.stats, "replayed") > 0, "{}", on.stats);
        assert_eq!(count(&verify.stats, "replayed"), 0, "{}", verify.stats);
        assert!(
            count(&verify.stats, "verify.checked") > 0,
            "{}",
            verify.stats
        );
        assert_verified(&verify.stats);
        for s in [&on.stats, &verify.stats] {
            // `\def\測試`, `\def\IntrDeep`, `\DeclareSIUnit`
            for key in ["recordings.committed", "missed.Meaning"] {
                assert!(count(s, key) > 0, "{key}: {s}");
            }
        }
        assert_eq!(count(&on.stats, "offers"), count(&verify.stats, "offers"));
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_calls_preconditions_are_checked() {
    let Some((exe, dir, format)) = setup(REJECT) else {
        return;
    };
    let off = run(exe, &dir, &format, Some("off"));
    let on = run(exe, &dir, &format, Some("on"));
    assert_same_output(&off, &on);
    let s = &on.stats;
    assert_eq!(count(s, "rejected.GlobalDefs"), 1, "{s}");
    assert_eq!(count(s, "rejected.AfterAssignment"), 1, "{s}");
    assert_eq!(count(s, "rejected.Tracing"), 3, "{s}");
    assert_eq!(count(s, "replayed"), 1, "{s}");
    std::fs::remove_dir_all(dir).unwrap();
}

/// A change to a macro the hook reads only through another is in the read
/// set: the call after it is recorded again, not replayed stale, and the
/// output is the real path's in every mode.
#[test]
fn a_change_the_body_does_not_name_is_not_replayed_stale() {
    let Some((exe, dir, format)) = setup(INDIRECT) else {
        return;
    };
    let off = run(exe, &dir, &format, Some("off"));
    let on = run(exe, &dir, &format, Some("on"));
    let verify = run(exe, &dir, &format, Some("verify"));
    assert_same_output(&off, &on);
    assert_same_output(&off, &verify);
    let s = &on.stats;
    // recorded at 1 and 3, replayed at 2 and 4 (after `\par\relax`: in a
    // paragraph the main loop's look-ahead expands the hook, and LaTeX's
    // `\par` ends in expansions; neither is `big_switch`'s own)
    assert_eq!(count(s, "offers"), 4, "{s}");
    assert_eq!(count(s, "recordings.committed"), 2, "{s}");
    assert_eq!(count(s, "replayed"), 2, "{s}");
    assert_eq!(count(s, "missed.Meaning"), 1, "{s}");
    let s = &verify.stats;
    assert_eq!(count(s, "verify.checked"), 2, "{s}");
    assert_verified(s);
    // after the checks, as many checkpoints as before them (none)
    assert!(s.contains("\ncheckpoints: 0\n"), "{s}");
    std::fs::remove_dir_all(dir).unwrap();
}

/// The hook of a `\let` test: `body`, called around a change of `\IntrSrc`.
fn let_doc(body: &str) -> String {
    format!(
        "\\documentclass{{article}}\\def\\IntrSrc{{one}}\n\
         \\def\\pdfstringdefPreHook{{{body}}}\n\
         \\begin{{document}}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\def\\IntrSrc{{two}}\n\\pdfstringdefPreHook\n\\IntrC\\par\\relax\n\
         \\pdfstringdefPreHook\n\\IntrC\\par\\relax\\end{{document}}\n"
    )
}

/// A `\let` copies its source's meaning as it is at the replay
/// (`Val::Cs`): a source the call reads only to copy it is not a
/// dependency (only its class: not a space, a parameter character,
/// `\outer` or a sparse register), so a change to it still replays, to the
/// real path's state.
#[test]
fn a_let_source_read_only_to_be_copied_is_not_a_dependency() {
    let Some((on, verify)) = modes_agree(&let_doc("\\let\\IntrC\\IntrSrc\\let\\IntrE\\relax"))
    else {
        return;
    };
    assert_eq!(count(&on, "replayed"), 3, "{on}");
    assert_eq!(count(&on, "missed.Meaning"), 0, "{on}");
    assert_eq!(count(&verify, "verify.checked"), 3, "{verify}");
}

/// ... but once the call reads its copy, the source's meaning is.
#[test]
fn a_let_copy_the_call_reads_makes_its_source_a_dependency() {
    let Some((on, verify)) = modes_agree(&let_doc("\\let\\IntrC\\IntrSrc\\edef\\IntrOut{\\IntrC}"))
    else {
        return;
    };
    assert_eq!(count(&on, "replayed"), 2, "{on}");
    assert_eq!(count(&on, "missed.Meaning"), 1, "{on}");
    assert_eq!(count(&verify, "verify.checked"), 2, "{verify}");
}

/// Each fault makes verify report a difference, and `VERIFY_FAIL` turns
/// it into exit status 3; the output stays the real path's.
#[test]
fn faults_in_the_replay_or_the_read_set_are_seen_by_verify() {
    let Some((exe, dir, format)) = setup(FAULTS) else {
        return;
    };
    let off = run(exe, &dir, &format, Some("off"));
    for (fault, class) in [
        // the group's `unsave` left out: the level differs
        ("drop-last", "verify.differing"),
        // `\IntrSrc` left out: the third call replays `one`
        ("skip-read", "verify.differing.eqtb"),
        // `\let` without its reference
        ("no-ref", "verify.differing.refs"),
        // a node the call found changed in place (`\limits`' kind of
        // write, which the allowlist keeps out of every recording): only
        // the `mem` comparison sees it
        ("in-place", "verify.differing.mem"),
    ] {
        let o = Opts {
            mode: Some("verify"),
            fault: Some(fault),
            ..Opts::default()
        };
        let v = run_with(exe, &dir, &format, o);
        assert_same_output(&off, &v);
        assert!(
            count(&v.stats, "verify.differing") > 0,
            "{fault}: {}",
            v.stats
        );
        assert!(count(&v.stats, class) > 0, "{fault}: {}", v.stats);
        let fail = run_with(exe, &dir, &format, Opts { fail: true, ..o });
        assert_eq!(fail.status, Some(3), "{fault}: {}", fail.stderr);
        assert!(
            fail.stderr.contains("checked calls differ"),
            "{}",
            fail.stderr
        );
    }
    // without a fault, VERIFY_FAIL leaves the status alone
    let ok = run_with(
        exe,
        &dir,
        &format,
        Opts {
            mode: Some("verify"),
            fail: true,
            ..Opts::default()
        },
    );
    assert_eq!(ok.status, Some(0), "{}", ok.stderr);
    std::fs::remove_dir_all(dir).unwrap();
}

/// Off, on and verify runs of `doc` (TeX errors allowed): the same output
/// and status; the reports of on and verify.
fn modes_agree(doc: &str) -> Option<(String, String)> {
    let (exe, dir, format) = setup(doc)?;
    let o = |mode| Opts {
        mode,
        errors: true,
        ..Opts::default()
    };
    let off = run_with(exe, &dir, &format, o(Some("off")));
    let on = run_with(exe, &dir, &format, o(Some("on")));
    let verify = run_with(exe, &dir, &format, o(Some("verify")));
    assert_same_output(&off, &on);
    assert_same_output(&off, &verify);
    assert_verified(&verify.stats);
    std::fs::remove_dir_all(dir).unwrap();
    Some((on.stats, verify.stats))
}

/// A hook of the given body, called three times in the body.
fn hook_doc(pre: &str, body: &str, after: &str) -> String {
    format!(
        "\\documentclass{{article}}{pre}\n\\def\\pdfstringdefPreHook{{{body}}}\n\
         \\begin{{document}}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\n{after}\\end{{document}}\n"
    )
}

/// Calls that may not replay: none is kept, none replayed, the reason
/// counted, the output the same.
fn never_replayed(pre: &str, body: &str, after: &str, why: &str) {
    let Some((on, verify)) = modes_agree(&hook_doc(pre, body, after)) else {
        return;
    };
    for s in [&on, &verify] {
        assert_eq!(count(s, "recordings.committed"), 0, "{s}");
        assert_eq!(count(s, "replayed"), 0, "{s}");
        assert_eq!(count(s, "verify.checked"), 0, "{s}");
        assert!(count(s, &format!("abandoned.{why}")) > 0, "{why}: {s}");
    }
}

/// `\read` and `\openin` (#2141 item 2): file input is not a pure function
/// of the state the guard compares; neither command is on the allowlist.
#[test]
fn file_input_is_never_replayed() {
    never_replayed(
        "\\newread\\IntrR\\immediate\\openin\\IntrR=\\jobname.tex",
        "\\read\\IntrR to\\IntrLine",
        "",
        "Command",
    );
    never_replayed(
        "\\newread\\IntrR",
        "\\openin\\IntrR=\\jobname.tex",
        "",
        "Command",
    );
}

/// `\aftergroup` at the call's own level (#2141 item 3) puts a token after
/// a group the call does not end.
#[test]
fn aftergroup_is_never_replayed() {
    never_replayed("", "\\aftergroup\\relax", "", "Command");
    never_replayed(
        "",
        "\\begingroup\\aftergroup\\relax\\endgroup",
        "",
        "Command",
    );
}

/// A write to a tracing parameter (#2141 item 4) could make a replay print.
#[test]
fn tracing_writes_are_never_replayed() {
    never_replayed(
        "",
        "\\tracingrestores=1 \\tracingrestores=0 ",
        "",
        "Tracing",
    );
    never_replayed("", "\\tracingnesting=0 ", "", "Tracing");
}

/// A command that edits the current list in place (`\limits` sets the
/// tail noad's subtype, `^` and `_` its script fields) is not on the
/// allowlist: such calls never replay, so the output cannot change, even
/// though verify does not compare `mem` words outside the lists it walks.
#[test]
fn in_place_list_edits_are_never_replayed() {
    for (body, what) in [
        ("\\limits", "limit_switch"),
        ("\\nolimits", "limit_switch"),
        ("^{2}", "sup_mark"),
        ("_{1}", "sub_mark"),
        ("\\def\\IntrX{a}\\limits", "a def, then limit_switch"),
    ] {
        let doc = format!(
            "\\documentclass{{article}}\\def\\pdfstringdefPreHook{{{body}}}\n\
             \\begin{{document}}\\relax\n$\\sum\\pdfstringdefPreHook x$\n\
             $\\sum\\pdfstringdefPreHook x$\n$\\sum\\pdfstringdefPreHook x$\n\
             \\[\\sum\\pdfstringdefPreHook x\\]\\end{{document}}\n"
        );
        let Some((on, verify)) = modes_agree(&doc) else {
            return;
        };
        for s in [&on, &verify] {
            assert!(count(s, "offers") > 0, "{what}: {s}");
            assert_eq!(count(s, "recordings.committed"), 0, "{what}: {s}");
            assert_eq!(count(s, "replayed"), 0, "{what}: {s}");
            assert_eq!(count(s, "verify.checked"), 0, "{what}: {s}");
            assert!(count(s, "abandoned.Command") > 0, "{what}: {s}");
        }
    }
}

/// A recording made inside a group closes one it did not open: abandoned.
#[test]
fn a_group_the_call_did_not_open_is_never_closed_by_a_replay() {
    let Some((on, _)) = modes_agree(
        "\\documentclass{article}\\def\\pdfstringdefPreHook{\\endgroup\\begingroup}\n\
         \\begin{document}x\\begingroup\\pdfstringdefPreHook\\endgroup\n\
         \\begingroup\\pdfstringdefPreHook\\endgroup\\end{document}\n",
    ) else {
        return;
    };
    // (`\endgroup` of a group the call did not open is refused as a
    // command, before its `unsave`)
    assert!(
        count(&on, "abandoned.Group") + count(&on, "abandoned.Command") > 0,
        "{on}"
    );
    assert_eq!(count(&on, "replayed"), 0, "{on}");
}

/// `\wd` writes a box node and `\immediate\write` a file: neither is kept.
#[test]
fn box_dimensions_and_file_output_are_never_replayed() {
    never_replayed(
        "\\setbox0\\hbox{}",
        "\\wd0=\\dimexpr\\wd0+1pt\\relax",
        "\\the\\wd0",
        "Command",
    );
    never_replayed(
        "\\newwrite\\IntrW\\immediate\\openout\\IntrW=\\jobname.w",
        "\\immediate\\write\\IntrW{x}",
        "x",
        "Command",
    );
}

/// What a recording reads through printing: `\string` of a one-character
/// control sequence reads its category code; a changed `\escapechar` or
/// category code is seen.
#[test]
fn printing_reads_category_codes_and_the_escape_character() {
    let Some((on, verify)) = modes_agree(
        "\\documentclass{article}\\def\\IntrM{\\b}\n\
         \\def\\pdfstringdefPreHook{\\edef\\IntrS{\\meaning\\IntrM}}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\\pdfstringdefPreHook \\IntrS\\par\n\
         \\catcode`\\b=12 \\pdfstringdefPreHook \\IntrS\\par\\catcode`\\b=11\n\
         \\escapechar=`\\! \\pdfstringdefPreHook \\IntrS\\par\\escapechar=92\n\
         \\pdfstringdefPreHook \\IntrS\\par\\end{document}\n",
    ) else {
        return;
    };
    assert!(count(&on, "replayed") > 0, "{on}");
    assert!(
        count(&on, "missed.Meaning") + count(&on, "missed.Word") > 0,
        "{on}"
    );
    assert!(count(&verify, "verify.checked") > 0, "{verify}");
}

/// An `\ifcsname` that finds nothing is not recorded (no key for absence).
#[test]
fn an_absent_name_is_not_recorded() {
    never_replayed("", "\\ifcsname IntrNone\\endcsname\\fi", "", "Absent");
}

/// `ctx` (where `@H@` calls the hook) with the hook's body `body`: off, on
/// and verify agree, and no call is kept, replayed or checked, for `why`.
fn never_replayed_in(pre: &str, body: &str, ctx: &str, why: &str) {
    let doc = format!(
        "\\documentclass{{article}}{pre}\n\\def\\pdfstringdefPreHook{{{body}}}\n\
         \\begin{{document}}\\relax\n{}\n\\end{{document}}\n",
        ctx.replace("@H@", "\\pdfstringdefPreHook ")
    );
    let Some((on, verify)) = modes_agree(&doc) else {
        return;
    };
    for s in [&on, &verify] {
        assert!(count(s, "offers") > 0, "{body}: {s}");
        assert_eq!(count(s, "recordings.committed"), 0, "{body}: {s}");
        assert_eq!(count(s, "replayed"), 0, "{body}: {s}");
        assert_eq!(count(s, "verify.checked"), 0, "{body}: {s}");
        assert!(
            count(s, &format!("abandoned.{why}")) > 0,
            "{body}: {why}: {s}"
        );
    }
}

/// The holes found in the verify-only recorder (slice 2), each a write it
/// did not see or a node it changed in place: slice 3's allowlist refuses
/// every one. #2452 (`\prevdepth`, `\spacefactor`, `\prevgraf`, a script,
/// `\nolimits`, `\over` on an empty list, `\deadcycles`, `\pagegoal`,
/// `\batchmode`, `\fontdimen`, `\hyphenchar`, `\hyphenation`,
/// `\uniformdeviate`), #2259 (`\font`), #2369 (`\XeTeXlinebreaklocale`,
/// `\XeTeXdefaultencoding`), #2176's question 2 (`\XeTeXinputencoding`),
/// #2261 (an empty paragraph resets `\prevgraf`), and reading `\prevgraf`
/// (#2176's dead top of `nest`).
#[test]
fn the_verify_recorders_holes_are_never_replayed() {
    let vbox = "\\setbox2\\vbox{\\hbox{}@H@\\hbox{}@H@\\hbox{}@H@}";
    let hbox = "\\setbox2\\hbox{x\\relax@H@x\\relax@H@x\\relax@H@}";
    let para = "x\\relax@H@y\\relax@H@z\\relax@H@\\par";
    let vmode = "\\relax@H@\\relax@H@\\relax@H@";
    let empty_par = "\\setbox2\\vbox{x\\par\\setbox3\\lastbox@H@\\relax}";
    let empty_par = empty_par.repeat(3);
    for (body, ctx, why) in [
        ("\\prevdepth=7pt\\relax", vbox, "Command"),
        ("\\spacefactor=2000\\relax", hbox, "Command"),
        ("\\prevgraf=5\\relax", para, "Command"),
        ("\\ifnum\\prevgraf<0 \\fi\\relax", para, "Internal"),
        ("^2\\relax", "\\setbox2\\hbox{$x@H@x@H@x@H@$}", "Command"),
        (
            "\\nolimits\\relax",
            "\\setbox2\\hbox{$\\sum@H@\\sum@H@\\sum@H@$}",
            "Command",
        ),
        (
            "\\over\\relax",
            "$\\relax@H@x$ $\\relax@H@x$ $\\relax@H@x$\\par",
            "Command",
        ),
        ("\\deadcycles=0\\relax", vmode, "Command"),
        ("\\pagegoal=500pt\\relax", vmode, "Command"),
        ("\\batchmode\\relax", vmode, "Command"),
        (
            "\\fontdimen2\\font=\\fontdimen2\\font\\relax",
            vmode,
            "Command",
        ),
        ("\\hyphenchar\\font=45\\relax", vmode, "Command"),
        ("\\hyphenation{ab-cd}\\relax", vmode, "Command"),
        (
            "\\edef\\IntrU{\\uniformdeviate 10}\\relax",
            vmode,
            "Expandable",
        ),
        ("\\font\\IntrF=cmr10\\relax", vmode, "Command"),
        ("\\XeTeXlinebreaklocale \"zh\"\\relax", vmode, "Command"),
        ("\\XeTeXdefaultencoding \"utf8\"\\relax", vmode, "Command"),
        ("\\XeTeXinputencoding \"utf8\"\\relax", vmode, "Command"),
        (
            "\\begingroup\\noindent\\par\\endgroup",
            &empty_par,
            "Command",
        ),
    ] {
        never_replayed_in("", body, ctx, why);
    }
}

/// A meaning read only for its token (`\def`'s parameter text) or to be
/// copied (`\let`'s source) is kept by its class, and the class tells an
/// alignment's `&` apart: at `align_state` 0 `get_next` ends the entry
/// there (it inserts the v-part), whoever reads the token. Recorded with
/// `\IntrB` a `\relax`, the call must not replay in a cell once `\IntrB`
/// is `&`: a replay would make the definition, the real path ends the
/// cell (and goes on with errors, the same in every mode).
#[test]
fn an_ampersand_read_only_for_its_token_is_seen_by_the_guard() {
    for body in [
        "\\let\\IntrA=\\IntrB\\relax",
        "\\def\\IntrD\\IntrB{}\\relax",
    ] {
        let Some((on, verify)) = modes_agree(&format!(
            "\\documentclass{{article}}\\let\\IntrB=\\relax\n\
             \\def\\pdfstringdefPreHook{{{body}}}\n\
             \\begin{{document}}\\relax\n\
             \\setbox0\\hbox{{\\relax\\pdfstringdefPreHook\\relax\\pdfstringdefPreHook}}\n\
             \\let\\IntrB=&\n\
             \\halign{{#\\relax\\cr\\relax\\pdfstringdefPreHook x\\cr}}\n\
             \\end{{document}}\n"
        )) else {
            return;
        };
        assert_eq!(count(&on, "replayed"), 1, "{body}: {on}");
        assert_eq!(count(&on, "missed.Meaning"), 1, "{body}: {on}");
        assert_eq!(count(&verify, "verify.checked"), 1, "{body}: {verify}");
    }
}

/// A document with `\IntrA` of 4 * 2^18 = 1,048,576 tokens, whose hook
/// copies it; with `headroom`, a list of three times as many tokens made
/// and dropped first, so that the free single-word nodes hold a copy.
fn long_doc(headroom: bool) -> String {
    let copy = "\\unexpanded\\expandafter{\\IntrA}";
    let double = format!("\\edef\\IntrA{{{copy}{copy}}}\n");
    let room = if headroom {
        format!("\\edef\\IntrBig{{{copy}{copy}{copy}}}\\let\\IntrBig\\relax\n")
    } else {
        String::new()
    };
    format!(
        "\\documentclass{{article}}\\def\\IntrA{{aaaa}}\n{}{room}\
         \\def\\pdfstringdefPreHook{{\\edef\\IntrX{{{copy}}}}}\n\
         \\begin{{document}}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\edef\\IntrA{{{copy}b}}\n\\pdfstringdefPreHook\n\
         \\pdfstringdefPreHook\nx\\end{{document}}\n",
        double.repeat(18)
    )
}

/// A value of more than a million tokens is recorded, replayed and
/// compared to its end (#2258: both were cut at the millionth token, so
/// the check of a correct replay differed, and two lists alike in their
/// first million compared equal).
#[test]
fn a_value_longer_than_a_million_tokens_is_compared_to_its_end() {
    let Some((on, verify)) = modes_agree(&long_doc(true)) else {
        return;
    };
    assert_eq!(count(&on, "replayed"), 2, "{on}");
    assert_eq!(count(&on, "missed.Meaning"), 1, "{on}");
    assert_eq!(count(&verify, "verify.checked"), 2, "{verify}");
}

/// A name of the letters of `k` (base 26), for generated control sequences.
fn letters(mut k: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'a' + (k % 26) as u8) as char);
        k /= 26;
        if k == 0 {
            return s;
        }
    }
}

/// 300 macros, each calling the next before its own `\relax` (an input
/// level each).
fn chain() -> String {
    (0..300)
        .map(|k| {
            format!(
                "\\def\\IntrC{}{{\\IntrC{}\\relax}}",
                letters(k),
                letters(k + 1)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + &format!("\\def\\IntrC{}{{\\relax}}", letters(300))
}

/// A hook of body `body`, called at the top level, then as `deeper`, then
/// at the top level again.
fn three_calls(pre: &str, body: &str, deeper: &str) -> String {
    format!(
        "\\documentclass{{article}}{pre}\n\
         \\def\\pdfstringdefPreHook{{{body}}}\n\
         \\def\\IntrWrap{{\\relax\\pdfstringdefPreHook\\relax}}\n\
         \\begin{{document}}\\relax\n\\pdfstringdefPreHook\\relax\n{deeper}\n\
         \\pdfstringdefPreHook\\relax\nx\\end{{document}}\n"
    )
}

/// The final statistics print the high-water marks of the input and
/// parameter stacks and of the buffer, and the extent of `mem`; a replay
/// pushes and allocates (almost) nothing. So a variant is admitted only
/// where the real path would move none of them: here a copy of a million
/// tokens with no free nodes to hold it (`mem` grows; before, the replay's
/// log said one word less), and a chain of 300 macros called one input
/// level deeper than it was recorded. Calls where the marks are as high
/// already still replay. The save stack's mark needs no guard: 600 local
/// definitions under more saves replay, and verify (which compares every
/// mark) finds them identical.
#[test]
fn a_replay_is_refused_where_the_real_path_would_raise_a_high_water_mark() {
    let Some((on, _)) = modes_agree(&long_doc(false)) else {
        return;
    };
    assert!(count(&on, "missed.HighWater") > 0, "{on}");
    let Some((on, verify)) = modes_agree(&three_calls(&chain(), "\\IntrCa", "\\IntrWrap")) else {
        return;
    };
    assert_eq!(count(&on, "missed.HighWater"), 1, "{on}");
    assert_eq!(count(&on, "replayed"), 1, "{on}");
    assert_eq!(count(&verify, "verify.checked"), 1, "{verify}");
    let saves: String = (0..600)
        .map(|k| format!("\\def\\IntrQ{}{{}}", letters(k)))
        .collect();
    // (every call in a group: a variant recorded at level one is not
    // admitted deeper, `Ctx::level_one`)
    let Some((on, verify)) = modes_agree(&format!(
        "\\documentclass{{article}}\n\
         \\def\\pdfstringdefPreHook{{\\begingroup{saves}\\endgroup}}\n\
         \\begin{{document}}\\relax\n\
         \\begingroup\\relax\\pdfstringdefPreHook\\relax\\endgroup\n\
         \\begingroup\\def\\IntrZa{{}}\\def\\IntrZb{{}}\\def\\IntrZc{{}}\\relax\
         \\pdfstringdefPreHook\\relax\\endgroup\n\
         \\begingroup\\relax\\pdfstringdefPreHook\\relax\\endgroup\n\
         x\\end{{document}}\n"
    )) else {
        return;
    };
    assert_eq!(count(&on, "missed.HighWater"), 0, "{on}");
    assert_eq!(count(&on, "replayed"), 2, "{on}");
    assert_eq!(count(&verify, "verify.checked"), 2, "{verify}");
}

/// The one-word room a recording keeps counts only cells its run
/// allocated: a free of a cell from before the run (here `\def\IntrFoo`
/// freeing the long old value) depends on a reference count or a level the
/// read set does not hold. At the second call `\IntrFoo` is short and the
/// free list nearly empty, so the real path's copy of the argument grows
/// `mem`; the replay must be refused. (Review F-XINTR-REV on #2251: before,
/// on said 1966749 words of memory where off said 2227664, and verify
/// passed.)
#[test]
fn the_one_word_room_counts_only_the_runs_own_cells() {
    // \IntrA: 4 * 2^16 = 262144 letters
    let copy = "\\unexpanded\\expandafter{\\IntrA}";
    let double = format!("\\edef\\IntrA{{{copy}{copy}}}\n");
    let Some((on, _)) = modes_agree(&format!(
        "\\documentclass{{article}}\n\\def\\IntrPa#1{{}}\n\
         \\begin{{document}}\\relax\n\\def\\IntrA{{aaaa}}\n{}\
         \\edef\\IntrFoo{{{copy}b}}\n\
         \\edef\\pdfstringdefPreHook{{\\noexpand\\def\\noexpand\\IntrFoo{{x}}\
         \\noexpand\\IntrPa{{{copy}}}\\noexpand\\relax}}\n\
         \\pdfstringdefPreHook\\relax\n\
         \\edef\\IntrD{{{copy}{copy}{copy}{copy}}}\n\
         \\def\\IntrT{{abcdefghij}}\\let\\IntrT\\relax\n\
         \\pdfstringdefPreHook\\relax\n\
         x\\end{{document}}\n",
        double.repeat(16)
    )) else {
        return;
    };
    assert_eq!(count(&on, "replayed"), 0, "{on}");
    assert_eq!(count(&on, "missed.HighWater"), 1, "{on}");
}

/// Whether the run frees a list of its own depends on the level it
/// starts at: the hook `\gdef`s `\IntrX` to a long list (its own cells),
/// then `\def`s it again, which frees the list at level one
/// (`eq_destroy`) and saves it deeper (`eq_save`). Recorded in the preamble
/// (level one, a peak of about one copy), called inside a group the real
/// path holds two copies at once; the context's `level_one` keeps the
/// variant from that call. (Re-review F-XINTR-REV on #2251: before, on
/// said 1048023 words of memory where off said 1080792.)
#[test]
fn a_variant_recorded_at_level_one_is_not_replayed_in_a_group() {
    // \IntrA: 4 * 2^14 = 65536 letters; \IntrH: half of it
    let copy = "\\unexpanded\\expandafter{\\IntrA}";
    let half = "\\unexpanded\\expandafter{\\IntrH}";
    let double = format!("\\edef\\IntrA{{{copy}{copy}}}\n");
    let Some((on, _)) = modes_agree(&format!(
        "\\documentclass{{article}}\n\\def\\IntrPa#1{{}}\n\
         \\def\\IntrA{{aaaa}}\n{}\\let\\IntrH\\IntrA\n{}\
         \\edef\\pdfstringdefPreHook{{\\noexpand\\gdef\\noexpand\\IntrX{{{copy}}}\
         \\noexpand\\def\\noexpand\\IntrX{{y}}\\noexpand\\IntrPa{{{copy}}}\\noexpand\\relax}}\n\
         \\relax\\pdfstringdefPreHook\\relax\n\
         \\begin{{document}}\\relax\n\
         \\edef\\IntrD{{{copy}{copy}{copy}{copy}{copy}}}\n\
         \\edef\\IntrT{{{copy}{half}}}\\let\\IntrT\\relax\n\
         \\begingroup\\relax\\pdfstringdefPreHook\\relax\\endgroup\n\
         x\\end{{document}}\n",
        double.repeat(13),
        double
    )) else {
        return;
    };
    assert_eq!(count(&on, "replayed"), 0, "{on}");
    assert_eq!(count(&on, "missed.Context"), 1, "{on}");
}

/// The usual case: `\pdfstringdef` calls the hook inside its group, at
/// whatever depth the document is. A variant recorded inside one group
/// replays inside two and three (only level one or not is part of the
/// context), with its local and global definitions, and every mode agrees.
#[test]
fn a_variant_recorded_in_a_group_replays_at_another_depth() {
    let Some((on, verify)) = modes_agree(
        "\\documentclass{article}\\def\\IntrSrc{one}\n\
         \\def\\pdfstringdefPreHook{\\def\\IntrL{\\IntrSrc}\\xdef\\IntrG{\\IntrSrc}\
         \\begingroup\\let\\IntrC\\IntrSrc\\endgroup\\relax}\n\
         \\begin{document}\\relax\n\
         \\begingroup\\relax\\pdfstringdefPreHook\\relax\\endgroup\n\
         \\begingroup\\begingroup\\relax\\pdfstringdefPreHook\\relax\\endgroup\\endgroup\n\
         {{{\\relax\\pdfstringdefPreHook\\relax}}}\n\
         [\\IntrG]\\end{document}\n",
    ) else {
        return;
    };
    assert_eq!(count(&on, "recordings.committed"), 1, "{on}");
    assert_eq!(count(&on, "replayed"), 2, "{on}");
    assert_eq!(count(&verify, "verify.checked"), 2, "{verify}");
}

/// Verify compares the high-water marks and the extent of `mem`: with the
/// guard's room check switched off (`no-room`), the chain's call one input
/// level deeper is admitted, and its check differs in `max_in_stack`.
#[test]
fn verify_sees_a_replay_that_leaves_a_high_water_mark_lower() {
    let Some((exe, dir, format)) = setup(&three_calls(&chain(), "\\IntrCa", "\\IntrWrap")) else {
        return;
    };
    let off = run(exe, &dir, &format, Some("off"));
    let v = run_with(
        exe,
        &dir,
        &format,
        Opts {
            mode: Some("verify"),
            fault: Some("no-room"),
            ..Opts::default()
        },
    );
    assert_same_output(&off, &v);
    assert_eq!(count(&v.stats, "verify.differing.scalar"), 1, "{}", v.stats);
    assert!(v.stats.contains("scalar max_in_stack"), "{}", v.stats);
    std::fs::remove_dir_all(dir).unwrap();
}

/// A body whose last tokens only expand (`\fi`) is used up inside
/// `big_switch`'s own `get_x_token`, which then reads the token after the
/// call: the recording reads past the body (`Level`) and is not kept, so
/// no check compares a call after that token (#2210).
#[test]
fn a_body_ending_in_an_expansion_is_not_kept() {
    never_replayed_in(
        "",
        "\\ifx aa\\fi",
        "@H@\\relax\n@H@\\relax\n@H@\\relax",
        "Level",
    );
}

/// A call that reads a token from below its body's input level reads past
/// itself (here `scan_int` looks for the end of `1` and finds the caller's
/// `\relax`, which it backs up): a replay would not, so the call is not
/// kept (#2176's rule, `not_replayable.ReadPast` there, here the
/// recording's `Level` abort at `get_next`'s exit). The same body with its
/// own `\relax` reads nothing past it, and replays and checks identical.
#[test]
fn a_call_that_reads_past_its_body_is_not_replayable() {
    never_replayed_in(
        "",
        "\\count255=1",
        "@H@\\relax\n@H@\\relax\n@H@\\relax\nx",
        "Level",
    );
    let Some((on, verify)) = modes_agree(&hook_doc("", "\\count255=1\\relax", "x")) else {
        return;
    };
    assert_eq!(count(&on, "replayed"), 2, "{on}");
    assert_eq!(count(&verify, "verify.checked"), 2, "{verify}");
}

/// A check whose real path is not pure (here `\immediate\write`, which a
/// recording refuses) is a difference, not a pass (#2203). The guard
/// admits such a call only when it is wrong: `skip-read` leaves `\IntrSel`
/// out of the read set, so the third call is admitted after `\IntrSel`
/// changed, and its check finds the real path impure.
#[test]
fn a_check_whose_real_path_is_not_pure_is_a_difference() {
    let Some((exe, dir, format)) = setup(
        "\\documentclass{article}\\newwrite\\IntrW\\immediate\\openout\\IntrW=\\jobname.w\n\
         \\def\\IntrSel{\\relax}\\def\\pdfstringdefPreHook{\\IntrSel}\n\
         \\begin{document}\\relax\n\\pdfstringdefPreHook\n\\pdfstringdefPreHook\n\
         \\def\\IntrSel{\\immediate\\write\\IntrW{x}\\relax}\n\\pdfstringdefPreHook\n\
         x\\end{document}\n",
    ) else {
        return;
    };
    let off = run(exe, &dir, &format, Some("off"));
    let o = |fault| Opts {
        mode: Some("verify"),
        fault,
        ..Opts::default()
    };
    let v = run_with(exe, &dir, &format, o(None));
    assert_same_output(&off, &v);
    assert_verified(&v.stats);
    assert_eq!(count(&v.stats, "verify.checked"), 1, "{}", v.stats);
    let f = run_with(exe, &dir, &format, o(Some("skip-read")));
    assert_same_output(&off, &f);
    assert_eq!(count(&f.stats, "verify.differing"), 1, "{}", f.stats);
    assert_eq!(
        count(&f.stats, "verify.differing.impure.Command"),
        1,
        "{}",
        f.stats
    );
    std::fs::remove_dir_all(dir).unwrap();
}

/// A `\let` of the end of a template (here `\IntrE`, which a `\futurelet`
/// in a template's v-part made) adds a reference to its list, as `\let`
/// does for every meaning from `call` on: a replay of the value would not,
/// so such a call is not kept.
#[test]
fn a_let_of_the_end_of_a_template_is_never_replayed() {
    never_replayed_in(
        "",
        "\\let\\IntrA=\\IntrE\\relax",
        "\\halign{#\\global\\futurelet\\IntrE\\relax\\cr x\\cr}\n\\relax@H@\\relax@H@\\relax@H@x",
        "Template",
    );
}
