//! tex.ch hunks that show in how a run ends or what it writes, against TeX
//! Live's pdfTeX (the lockstep harness fixes the job name, the options and
//! the input lines, so it cannot show them):
//!
//! - tex.ch \[51.1333\] and \[32.642\]: `Transcript written on` and, for a
//!   DVI file, `Output written on` print the name with `print_file_name`,
//!   quoted when it has a space;
//! - tex.ch \[32.617\]: `-output-comment` (texmfmp.c truncates it to 255
//!   characters) or texmf.cnf's `output_comment` is the DVI file's comment;
//! - tex.ch \[49.1265\]: in `\batchmode` kpathsea's mktex scripts are silent;
//! - texmfmp.c's `input_line`: a line that does not fit in `buf_size` stops
//!   the run with two lines on stderr and exit status 1.
//!
//! Both engines run as `pdftex` on the same input; terminal output, stderr,
//! the log (after the banner), the DVI file and the exit status are
//! compared. Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    log: Option<String>,
    dvi: Option<Vec<u8>>,
}

fn run(bin: &Path, dir: &Path, args: &[&str], job: &str, ours: bool) -> Out {
    let mut c = Command::new(bin);
    c.arg("-ini")
        .args(args)
        .arg("t.tex")
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
        );
    }
    let out = c.output().unwrap();
    Out {
        code: out.status.code(),
        stdout: after_banner(&String::from_utf8_lossy(&out.stdout)),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        log: std::fs::read_to_string(dir.join(format!("{job}.log")))
            .ok()
            .map(|l| after_banner(&l)),
        dvi: std::fs::read(dir.join(format!("{job}.dvi"))).ok(),
    }
}

/// Everything after the first line (the banner, whose date the log has).
fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

/// Runs both engines on `tex` and returns (ours, pdfTeX's), or None
/// without TeX Live.
fn both(name: &str, tex: &[u8], args: &[&str], job: &str) -> Option<(Out, Out, PathBuf)> {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return None;
    };
    let base = common::fresh_dir(&format!("flashtex-texch-run-{name}"));
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(d.join("t.tex"), tex).unwrap();
    }
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    let x = run(&ours, &a, args, job, true);
    let y = run(&texbin.join("pdftex"), &b, args, job, false);
    Some((x, y, base))
}

fn assert_same(name: &str, x: &Out, y: &Out) {
    assert_eq!(
        x.stdout, y.stdout,
        "{name}: terminal output differs from pdftex"
    );
    assert_eq!(x.stderr, y.stderr, "{name}: stderr differs from pdftex");
    assert_eq!(x.log, y.log, "{name}: log differs from pdftex");
    assert_eq!(x.dvi, y.dvi, "{name}: DVI file differs from pdftex");
    assert_eq!(x.code, y.code, "{name}: exit status differs from pdftex");
}

const SHIP: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\shipout\\hbox{}\\end\n";

#[test]
fn transcript_and_dvi_names_with_a_space_are_quoted() {
    for (mode, expect) in [
        ("\\pdfoutput=1 ", "Transcript written on \"a b.log\"."),
        ("\\pdfoutput=0 ", "Output written on \"a b.dvi\""),
    ] {
        let tex = format!("{mode}{SHIP}");
        let Some((x, y, base)) = both("quoted", tex.as_bytes(), &["-jobname=a b"], "a b") else {
            return;
        };
        assert!(
            y.stdout.contains(expect),
            "pdftex lacks {expect:?}:\n{}",
            y.stdout
        );
        let dvi = mode.contains('0');
        let (mut x, mut y) = (x, y);
        if !dvi {
            // The PDF file's bytes are another test's (tools/parity).
            (x.dvi, y.dvi) = (None, None);
        }
        assert_same(mode, &x, &y);
        let _ = std::fs::remove_dir_all(&base);
    }
}

#[test]
fn output_comment_is_the_dvi_comment() {
    let long = format!("-output-comment={}", "c".repeat(300));
    for args in [
        vec!["-output-comment=HELLO"],
        vec!["-output-comment="],
        vec![long.as_str()],
        vec!["-cnf-line=output_comment=FROM CNF"],
        vec![],
    ] {
        let tex = format!("\\pdfoutput=0 {SHIP}");
        let Some((x, y, base)) = both("comment", tex.as_bytes(), &args, "t") else {
            return;
        };
        assert!(y.dvi.is_some(), "{args:?}: pdftex wrote no DVI file");
        assert_same(&format!("{args:?}"), &x, &y);
        let _ = std::fs::remove_dir_all(&base);
    }
}

#[test]
fn batchmode_silences_mktextfm() {
    let tex = "\\batchmode\\font\\x=qqzzxnofont \\end\n";
    let Some((x, y, base)) = both("batchmode", tex.as_bytes(), &[], "t") else {
        return;
    };
    assert_eq!(y.stderr, "", "pdftex's mktextfm was not silent");
    assert_same("batchmode", &x, &y);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn a_line_longer_than_the_buffer_stops_the_run() {
    // `first` is past the terminal line `t.tex`; the lengths straddle the
    // last one that fits, and pdftex must stop for the longest.
    let mut stopped = false;
    for len in 199_990..=200_001 {
        let mut tex = b"%".to_vec();
        tex.extend(std::iter::repeat_n(b'x', len - 1));
        tex.extend_from_slice(b"\n\\end\n");
        let Some((x, y, base)) = both("bufsize", &tex, &[], "t") else {
            return;
        };
        stopped |= y.stderr.contains("Unable to read an entire line");
        assert_same(&format!("line of {len}"), &x, &y);
        let _ = std::fs::remove_dir_all(&base);
    }
    assert!(stopped, "no line was too long for pdftex");
}
