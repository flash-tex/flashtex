//! tex.ch [6.84] and [51.1333], texmfmp.c's `calledit`, against TeX Live's
//! pdfTeX: `E` at an error prompt runs the editor command of `TEXEDIT` once
//! the files are closed (`%s` the file name, `%d` the line), and the program
//! ends with status 1. Interactive runs, which the lockstep harness cannot
//! hold, so the two engines run here on the same input from stdin and their
//! terminal output, stderr, log, editor output and exit codes are compared.
//! The editor commands write to a file, never to the terminal: in a pipe,
//! pdfTeX's terminal output is fully buffered by stdio and comes out after
//! the editor's, where this engine's has already been written (on a
//! terminal both are line-buffered, and the order is the same).
//! Skips where there is no TeX Live (e.g. CI).
// Unix: the editor commands are `/bin/sh` scripts.
#![cfg(all(feature = "kpathsea", unix))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

// The same numbers on Linux and macOS; SIG_DFL is 0 on both.
const SIGINT: std::ffi::c_int = 2;
const SIGQUIT: std::ffi::c_int = 3;

extern "C" {
    fn signal(sig: std::ffi::c_int, handler: usize) -> usize;
}

/// `signal(sig, SIG_DFL)`.
fn default_signal(sig: std::ffi::c_int) {
    // SAFETY: SIG_DFL (0) is a valid disposition for SIGINT and SIGQUIT.
    unsafe { signal(sig, 0) };
}

const T_TEX: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\\undefined\n\\end\n";

/// What a run shows: exit code, terminal and stderr (both after their first
/// line, the banner, with the program's path as `pdftex`), the log after its
/// banner, and what the editor wrote.
#[derive(Debug, PartialEq)]
struct Seen {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    log: String,
    edit: Option<String>,
}

struct Case<'a> {
    files: &'a [(&'a str, &'a str)],
    args: &'a [&'a str],
    stdin: &'a str,
    /// `None`: `TEXEDIT` is unset, and a `vi` on PATH writes its arguments.
    texedit: Option<&'a str>,
    job: &'a str,
}

fn after_first_line(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

fn run(bin: &Path, dir: &Path, case: &Case, ours: bool) -> Seen {
    std::fs::create_dir_all(dir.join("bin")).unwrap();
    for (name, text) in case.files {
        let p = dir.join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let vi = dir.join("bin/vi");
    std::fs::write(&vi, "#!/bin/sh\necho \"vi argc=$# [$1] [$2]\" >edit.out\n").unwrap();
    std::fs::set_permissions(&vi, std::fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        dir.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut c = Command::new(bin);
    c.arg("-ini")
        .args(case.args)
        .current_dir(dir)
        .env("PATH", path)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    match case.texedit {
        Some(e) => c.env("TEXEDIT", e),
        None => c.env_remove("TEXEDIT"),
    };
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    // A shell starts a background job (`cmd &`, `nohup`, a non-interactive
    // script's `&`) with SIGINT and SIGQUIT ignored, the engines inherit
    // that, and system(3) gives the editor what the program started with:
    // the_editor_itself_gets_sigint_as_usual then saw the editor survive its
    // own SIGINT. Both engines start with the default actions, as they do
    // from a terminal, whatever ran this test.
    // SAFETY: signal(2) is async-signal-safe, and nothing else runs between
    // fork and exec.
    unsafe {
        c.pre_exec(|| {
            default_signal(SIGINT);
            default_signal(SIGQUIT);
            Ok(())
        });
    }
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(case.stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).replace(&bin.display().to_string(), "pdftex");
    let log = std::fs::read_to_string(dir.join(format!("{}.log", case.job))).unwrap_or_default();
    Seen {
        code: out.status.code(),
        stdout: after_first_line(&text(&out.stdout)),
        stderr: text(&out.stderr),
        log: after_first_line(&log),
        edit: std::fs::read_to_string(dir.join("edit.out")).ok(),
    }
}

/// Runs `case` with both engines; returns pdfTeX's result after checking
/// that ours is the same.
fn compare(name: &str, case: Case) -> Option<Seen> {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return None;
    };
    let base = common::fresh_dir(&format!("flashtex-call-edit-{name}"));
    let (a, b) = (base.join("ours"), base.join("tex"));
    std::fs::create_dir_all(&a).unwrap();
    // Both run as `pdftex`, so the program name in messages is the same.
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    let (x, y) = (
        run(&ours, &a, &case, true),
        run(&texbin.join("pdftex"), &b, &case, false),
    );
    assert_eq!(
        x, y,
        "{name}: this engine (left) differs from TeX Live's pdftex (right)"
    );
    let _ = std::fs::remove_dir_all(&base);
    Some(y)
}

fn t_case<'a>(stdin: &'a str, texedit: Option<&'a str>) -> Case<'a> {
    Case {
        files: &[("t.tex", T_TEX)],
        args: &["t.tex"],
        stdin,
        texedit,
        job: "t",
    }
}

#[test]
fn e_runs_texedit_with_the_file_and_line() {
    let Some(y) = compare(
        "succeed",
        t_case("E\n", Some("echo EDITOR %s +%d >edit.out")),
    ) else {
        return;
    };
    assert_eq!(y.edit.as_deref(), Some("EDITOR t.tex +2\n"));
    assert!(y.log.ends_with("? E\nNo pages of output.\n"), "{}", y.log);
    assert_eq!(y.code, Some(1));
}

#[test]
fn a_failing_editor_is_reported_on_stderr() {
    let Some(y) = compare("fail", t_case("E\n", Some("echo %s %d >edit.out; exit 3"))) else {
        return;
    };
    assert_eq!(
        y.stderr,
        "! Trouble executing `echo t.tex 2 >edit.out; exit 3'.\n"
    );
    assert_eq!(y.edit.as_deref(), Some("t.tex 2\n"));
}

#[test]
fn without_texedit_the_editor_is_vi() {
    // web2c's default `vi +%d '%s'`; the `vi` here only writes its arguments.
    let Some(y) = compare("default", t_case("E\n", None)) else {
        return;
    };
    assert_eq!(y.edit.as_deref(), Some("vi argc=2 [+2] [t.tex]\n"));
}

#[test]
fn a_repeated_percent_d_or_percent_s_is_fatal() {
    for (name, e) in [
        ("twice-d", "echo %d %d >edit.out"),
        ("twice-s", "echo %s %s >edit.out"),
    ] {
        let Some(y) = compare(name, t_case("E\n", Some(e))) else {
            return;
        };
        assert!(
            y.stderr.contains("appears twice in editor command."),
            "{}",
            y.stderr
        );
        assert_eq!(y.edit, None);
    }
}

#[test]
fn other_percent_sequences_are_kept() {
    // `%%` and `%q` stay as they are, and so does a `%` at the end (here in
    // a shell comment).
    let Some(y) = compare(
        "percent",
        t_case("E\n", Some("echo 'a%%b %q [%s] [%d]' >edit.out #%")),
    ) else {
        return;
    };
    assert_eq!(y.edit.as_deref(), Some("a%%b %q [t.tex] [2]\n"));
}

#[test]
fn e_names_the_file_as_it_was_input() {
    // A file in a subdirectory, after text inserted from the terminal (the
    // inserted line's error is shown with the file under it), and lowercase.
    let Some(y) = compare(
        "sub",
        Case {
            files: &[
                (
                    "u.tex",
                    "\\catcode`\\{=1 \\catcode`\\}=2\n\\input sub/x\n\\end\n",
                ),
                ("sub/x.tex", "a\nb \\undefined\n"),
            ],
            args: &["u"],
            stdin: "I\\undefined\ne\n",
            texedit: Some("echo EDITOR %s +%d >edit.out"),
            job: "u",
        },
    ) else {
        return;
    };
    assert_eq!(y.edit.as_deref(), Some("EDITOR sub/x.tex +2\n"));
}

#[test]
fn e_does_nothing_when_the_error_is_on_the_terminal() {
    let Some(y) = compare(
        "terminal",
        Case {
            files: &[],
            args: &["\\relax"],
            stdin: "\\undefined\nE\nX\n",
            texedit: Some("echo EDITOR %s +%d >edit.out"),
            job: "texput",
        },
    ) else {
        return;
    };
    assert_eq!(y.edit, None);
    assert!(y.log.contains("H for help, X to quit.\n? X\n"), "{}", y.log);
}

#[test]
fn without_a_prompt_e_is_never_read() {
    for mode in [
        "-interaction=nonstopmode",
        "-interaction=scrollmode",
        "-interaction=batchmode",
    ] {
        let Some(y) = compare(
            mode.trim_start_matches("-interaction="),
            Case {
                files: &[("t.tex", T_TEX)],
                args: &[mode, "t.tex"],
                stdin: "E\n",
                texedit: Some("echo EDITOR %s +%d >edit.out"),
                job: "t",
            },
        ) else {
            return;
        };
        assert_eq!(y.edit, None);
    }
}

/// `E` with an editor command that runs `probe.sh`, a shell script whose
/// `$PPID` is the program, as `exec` makes it.
fn signal_case<'a>(files: &'a [(&'a str, &'a str)]) -> Case<'a> {
    Case {
        files,
        args: &["t.tex"],
        stdin: "E\n",
        texedit: Some("exec sh ./probe.sh"),
        job: "t",
    }
}

#[test]
fn sigint_while_the_editor_runs_does_not_end_the_program() {
    // system(3) ignores SIGINT in the caller while the command runs (Ctrl-C
    // in the editor reaches the whole process group).
    let probe = "echo before >edit.out\nkill -INT $PPID\necho after >>edit.out\n";
    let Some(y) = compare(
        "sigint",
        signal_case(&[("t.tex", T_TEX), ("probe.sh", probe)]),
    ) else {
        return;
    };
    assert_eq!(y.edit.as_deref(), Some("before\nafter\n"));
    assert_eq!(y.code, Some(1));
}

#[test]
fn sigquit_while_the_editor_runs_does_not_end_the_program() {
    let probe = "echo before >edit.out\nkill -QUIT $PPID\necho after >>edit.out\n";
    let Some(y) = compare(
        "sigquit",
        signal_case(&[("t.tex", T_TEX), ("probe.sh", probe)]),
    ) else {
        return;
    };
    assert_eq!(y.edit.as_deref(), Some("before\nafter\n"));
    assert_eq!(y.code, Some(1));
}

#[test]
fn the_editor_itself_gets_sigint_as_usual() {
    // ... and gives the child the default action back: the command dies of
    // its own SIGINT, which is "Trouble executing".
    let probe = "echo before >edit.out\nkill -INT $$\necho after >>edit.out\n";
    let Some(y) = compare(
        "sigint-child",
        signal_case(&[("t.tex", T_TEX), ("probe.sh", probe)]),
    ) else {
        return;
    };
    assert_eq!(y.edit.as_deref(), Some("before\n"));
    assert_eq!(y.stderr, "! Trouble executing `exec sh ./probe.sh'.\n");
}

#[test]
fn the_editor_starts_with_sigpipe_at_its_default() {
    // Rust ignores SIGPIPE in this program, and system(3) passes ignored
    // signals on; pdfTeX's editor gets the default, so a pipeline in it
    // ends quietly instead of writing into a closed pipe.
    let probe = "perl -e 'print $SIG{PIPE}//q(undef)' >edit.out\n\
                 echo >>edit.out\n\
                 yes | head -1 >>edit.out\n";
    let Some(y) = compare(
        "sigpipe",
        signal_case(&[("t.tex", T_TEX), ("probe.sh", probe)]),
    ) else {
        return;
    };
    assert_eq!(y.edit.as_deref(), Some("undef\ny\n"));
    assert_eq!(y.stderr, "");
}
