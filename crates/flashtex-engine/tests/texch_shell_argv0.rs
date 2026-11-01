//! The shell that `\write18`, `\input|cmd` and `\openout` to `|cmd` start,
//! against TeX Live's pdfTeX: web2c's `runsystem` and `runpopen` call the C
//! library's `system()` and `popen()`, which run `sh -c CMD` with `argv[0]`
//! `sh`, so a command the shell cannot find is reported as `sh: ...` (not
//! `/bin/sh: ...`) on the standard error both engines inherit. A command's
//! own output goes to the engine's standard output and error. The two engines
//! run with `-shell-escape` and the same document, and their standard output,
//! standard error and logs (after the banner) and exit codes are compared
//! byte for byte; what pdfTeX printed is the expected data. Skips where there
//! is no TeX Live (e.g. CI).
#![cfg(all(unix, feature = "kpathsea"))]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

struct Run {
    code: Option<i32>,
    out: String,
    err: String,
    log: String,
}

fn run(bin: &Path, dir: &Path, ours: bool) -> Run {
    let mut c = Command::new(bin);
    c.args(["-ini", "-shell-escape", "t.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .env("FLASHTEX_EXTERNAL_EFFECTS", dir.join("effects.txt"));
    }
    let o = c.output().unwrap();
    Run {
        code: o.status.code(),
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
        err: String::from_utf8_lossy(&o.stderr).into_owned(),
        log: std::fs::read_to_string(dir.join("t.log")).unwrap(),
    }
}

/// Everything after the first line (the banner, whose date the log has).
fn after_banner(s: &str) -> &str {
    s.split_once('\n').map(|(_, b)| b).unwrap_or_default()
}

/// Runs `body` in both engines; `sh_names` are the commands pdfTeX's shell
/// must have reported with a line starting `sh: `, and `effects` is this
/// engine's external-effects journal, which the shell's name does not change.
/// Returns pdfTeX's run, or `None` where there is no TeX Live.
fn compare(job: &str, body: &str, sh_names: &[&str], effects: &str) -> Option<Run> {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return None;
    };
    let tex = format!("\\catcode`\\{{=1 \\catcode`\\}}=2\n{body}\\end\n");
    let base = common::fresh_dir(&format!("flashtex-shargv0-{job}"));
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(d.join("t.tex"), &tex).unwrap();
    }
    // Both run as `pdftex`, so the invocation name in messages is the same.
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    let x = run(&ours, &a, true);
    let y = run(&texbin.join("pdftex"), &b, false);
    for name in sh_names {
        assert!(
            y.err
                .lines()
                .any(|l| l.starts_with("sh: ") && l.contains(name)),
            "{job}: pdfTeX's standard error has no `sh: ` line for {name}:\n{}",
            y.err
        );
    }
    assert_eq!(
        after_banner(&x.out),
        after_banner(&y.out),
        "{job}: standard output differs from TeX Live's pdftex"
    );
    assert_eq!(
        x.err, y.err,
        "{job}: standard error differs from TeX Live's pdftex"
    );
    assert_eq!(
        after_banner(&x.log),
        after_banner(&y.log),
        "{job}: t.log differs from TeX Live's pdftex"
    );
    assert_eq!(x.code, y.code, "{job}: exit codes differ");
    assert_eq!(
        std::fs::read_to_string(a.join("effects.txt")).unwrap(),
        effects,
        "{job}: external effects"
    );
    let _ = std::fs::remove_dir_all(&base);
    Some(y)
}

#[test]
fn write18_of_a_missing_command() {
    let Some(y) = compare(
        "w18fail",
        "\\immediate\\write18{nonexistent-command-xyz}\n",
        &["nonexistent-command-xyz"],
        "write18 nonexistent-command-xyz\n",
    ) else {
        return;
    };
    assert!(
        y.log
            .contains("runsystem(nonexistent-command-xyz)...executed."),
        "{}",
        y.log
    );
}

#[test]
fn write18_output_goes_to_the_terminal() {
    let Some(y) = compare(
        "w18ok",
        "\\immediate\\write18{echo shell-stdout; echo shell-stderr >&2}\n",
        &[],
        "write18 echo shell-stdout; echo shell-stderr >&2\n",
    ) else {
        return;
    };
    assert!(y.out.contains("shell-stdout"), "{}", y.out);
    assert!(y.err.contains("shell-stderr"), "{}", y.err);
}

#[test]
fn input_from_a_missing_command() {
    compare(
        "inpipe",
        "\\input|nonexistent-input-xyz \n",
        &["nonexistent-input-xyz"],
        "pipe-in nonexistent-input-xyz\n",
    );
}

#[test]
fn openout_to_a_missing_command() {
    // Nothing is written, so no write races the shell's exit.
    compare(
        "outpipe",
        "\\immediate\\openout3=|nonexistent-openout-xyz\n\\immediate\\closeout3\n",
        &["nonexistent-openout-xyz"],
        "pipe-out nonexistent-openout-xyz\n",
    );
}
