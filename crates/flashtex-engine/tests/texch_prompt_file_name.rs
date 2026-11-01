//! tex.ch [29.530]'s `prompt_file_name`, against TeX Live's pdfTeX: after
//! a file that cannot be opened, the context is shown for an input file
//! too (tex.ch gives `\input` no default extension), then
//! `(Press Enter to retry, or Control-D to exit` with
//! `; default file extension is `e'` when there is one, and `)`; an empty
//! reply tries the same name again (area, name and extension restored), not
//! an empty name. The prompts read the terminal, which the lockstep harness
//! does not feed, so the two engines run here with the same bytes on their
//! standard input, and their terminal output and logs (after the banner)
//! and exit codes are compared. Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn run(bin: &Path, dir: &Path, args: &[&str], stdin: &str, ours: bool) -> (Option<i32>, String) {
    let mut c = Command::new(bin);
    c.arg("-ini")
        .args(args)
        .arg("t.tex")
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    let mut child = c.spawn().unwrap();
    // Every engine stops at the end of its terminal input; a write error
    // means it stopped before reading all of it, which the comparison shows.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    let out = child.wait_with_output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

/// Everything after the first line (the banner, whose date the log has).
fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

fn compare(job: &str, tex: &str, args: &[&str], stdin: &str, expect: &[&str]) {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir(&format!("flashtex-prompt-{job}"));
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(d.join("t.tex"), tex).unwrap();
        std::fs::write(d.join("sub").join("exists.tex"), "\\relax\n").unwrap();
    }
    // Both run as `pdftex`, so the invocation name in messages is the same.
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    let (x, ours_term) = run(&ours, &a, args, stdin, true);
    let (y, tex_term) = run(&texbin.join("pdftex"), &b, args, stdin, false);
    let log = |d: &Path| after_banner(&std::fs::read_to_string(d.join("t.log")).unwrap());
    let (ours_log, tex_log) = (log(&a), log(&b));
    for e in expect {
        assert!(
            tex_term.contains(e),
            "pdfTeX's terminal output lacks {e:?}:\n{tex_term}"
        );
    }
    assert_eq!(
        after_banner(&ours_term),
        after_banner(&tex_term),
        "{job}: terminal output differs from TeX Live's pdftex"
    );
    assert_eq!(
        ours_log, tex_log,
        "{job}: t.log differs from TeX Live's pdftex"
    );
    assert_eq!(x, y, "{job}: exit codes differ");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn empty_reply_retries_the_input_file_until_end_of_terminal() {
    compare(
        "input",
        "\\input nonexist\n",
        &[],
        "\n\n",
        &[
            "(Press Enter to retry, or Control-D to exit)\n\
             Please type another input file name: ! I can't find file `nonexist'.\n\
             l.1 \\input nonexist",
            "! Emergency stop.",
        ],
    );
}

#[test]
fn empty_reply_keeps_the_area_and_extension() {
    compare(
        "areaext",
        "\\input sub/nonexist.bar\n",
        &[],
        "\n",
        &["Please type another input file name: ! I can't find file `sub/nonexist.bar'."],
    );
}

#[test]
fn a_reply_names_the_file_to_input() {
    compare(
        "reply",
        "\\input nonexist \\end\n",
        &[],
        "sub/exists\n",
        &["Please type another input file name: (./sub/exists.tex)"],
    );
}

#[test]
fn openout_shows_its_default_extension() {
    compare(
        "openout",
        "\\immediate\\openout1=/nonexistent-dir/x \\end\n",
        &[],
        "\n",
        &[
            "(Press Enter to retry, or Control-D to exit; default file extension is `.tex')\n\
             Please type another output file name: ! I can't write on file `/nonexistent-dir/x.tex'.",
        ],
    );
}

#[test]
fn nonstop_mode_prints_the_help_line_then_stops() {
    compare(
        "nonstop",
        "\\input nonexist\n",
        &["-interaction=nonstopmode"],
        "",
        &["(Press Enter to retry, or Control-D to exit)\n\
             Please type another input file name\n! Emergency stop."],
    );
}
