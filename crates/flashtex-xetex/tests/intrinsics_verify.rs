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
         {\\catcode`\\b=12 \\pdfstringdefPreHook \\IntrS\\par}\n\
         {\\escapechar=`\\! \\pdfstringdefPreHook \\IntrS\\par}\n\
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
