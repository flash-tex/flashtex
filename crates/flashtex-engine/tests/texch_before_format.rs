//! A production run before its format is loaded (tex.ch [1.8], [51.1332]):
//! INITEX alone initializes the tables and starts the string pool, so a
//! production run has no strings until the format supplies them, and what
//! it prints before then -- the banner, a `%&` line naming no format, a
//! TCX file that cannot be found, a format that cannot be found or is not a
//! format -- comes from no pool string (changes/virtex.ch).
//!
//! Each engine makes two formats with its own INITEX, `pdftex` (the
//! default, named after the program) and `g`, then
//! runs each case; the terminal output, the error output, the log, and the
//! exit status are compared with TeX Live's pdfTeX, in full (the dates are
//! fixed by `SOURCE_DATE_EPOCH` and `FORCE_SOURCE_DATE`; `MKTEXFMT=0`, as
//! FlashTeX runs no mktexfmt). Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// What a run left: exit status, terminal, error output, and its log.
#[derive(Debug, PartialEq)]
struct Outcome {
    status: Option<i32>,
    stdout: String,
    stderr: String,
    log: Option<String>,
}

const DOC: &str = "\\message{[\\which]}\\end\n";

/// (name, first line of `t.tex`, arguments)
const CASES: &[(&str, &str, &[&str])] = &[
    // texmfmp.c's parse_first_line: a format that cannot be found is
    // ignored (the default is loaded) ...
    (
        "first-line-no-format",
        "%&nosuch\n",
        &["-parse-first-line", "t.tex"],
    ),
    // ... and one that can is loaded instead of the default.
    (
        "first-line-format",
        "%&g\n",
        &["-parse-first-line", "t.tex"],
    ),
    // ... with a TCX file that cannot be found (a warning, then the run).
    (
        "first-line-no-tcx",
        "%&g -translate-file=nosuch\n",
        &["-parse-first-line", "t.tex"],
    ),
    // tex.ch [29.524]: `&name` names a format that cannot be found.
    ("ampersand-no-format", "", &["&nosuch", "t.tex"]),
    // ... and the default format cannot be found either.
    ("no-default-format", "", &["-fmt=nosuch", "t.tex"]),
    // §1308's bad_fmt: the file is not a format.
    ("not-a-format", "", &["-fmt=bad", "t.tex"]),
];

#[test]
fn a_production_run_prints_what_pdftex_does_before_its_format() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-texch-before-format");
    let mut out: Vec<Vec<Outcome>> = vec![];
    for (ours, sub) in [(true, "ours"), (false, "tex")] {
        let d = base.join(sub);
        std::fs::create_dir_all(&d).unwrap();
        let bin = if ours {
            let b = d.join("pdftex");
            common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &b);
            b
        } else {
            texbin.join("pdftex")
        };
        let pass = |args: &[&str]| {
            let mut c = Command::new(&bin);
            c.args(args)
                .current_dir(&d)
                .env("SOURCE_DATE_EPOCH", "0")
                .env("FORCE_SOURCE_DATE", "1")
                // No mktexfmt for a format that cannot be found: FlashTeX
                // makes no formats on the fly, and the oracle's would
                // write into the user's TeX Live tree.
                .env("MKTEXFMT", "0")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            if ours {
                c.env("FLASHTEX_FORMATS", &d).env(
                    "FLASHTEX_POOL",
                    Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
                );
            }
            c.output().unwrap()
        };
        for f in ["pdftex", "g"] {
            let src = format!("\\catcode`\\{{=1 \\catcode`\\}}=2 \\def\\which{{{f}}}\\dump\n");
            std::fs::write(d.join(format!("{f}.tex")), src).unwrap();
            let made = pass(&["-ini", &format!("{f}.tex")]);
            assert!(
                d.join(format!("{f}.fmt")).is_file(),
                "{sub}: no {f}.fmt: {made:?}"
            );
        }
        std::fs::write(d.join("bad.fmt"), b"not a format file\n").unwrap();
        let mut runs = vec![];
        for (_, first, args) in CASES {
            std::fs::write(d.join("t.tex"), format!("{first}{DOC}")).unwrap();
            let _ = std::fs::remove_file(d.join("t.log"));
            let r = pass(args);
            runs.push(Outcome {
                status: r.status.code(),
                stdout: String::from_utf8_lossy(&r.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&r.stderr).into_owned(),
                log: std::fs::read(d.join("t.log"))
                    .ok()
                    .map(|l| String::from_utf8_lossy(&l).into_owned()),
            });
        }
        out.push(runs);
    }
    let (x, y) = (&out[0], &out[1]);
    for (i, (name, _, _)) in CASES.iter().enumerate() {
        assert_eq!(
            x[i], y[i],
            "{name}: ours (left) differs from pdftex's (right)"
        );
    }
    // The cases are the ones meant: pdfTeX's own outcomes.
    assert!(y[0].stdout.contains("[pdftex]"), "{:?}", y[0]);
    assert!(y[1].stdout.contains("[g]"), "{:?}", y[1]);
    assert!(y[2].stderr.contains("nosuch.tcx"), "{:?}", y[2]);
    assert!(
        y[3].stdout.contains("Sorry, I can't find the format"),
        "{:?}",
        y[3]
    );
    assert!(
        y[4].stdout.contains("I can't find the format file"),
        "{:?}",
        y[4]
    );
    assert!(
        y[5].stdout
            .contains("(Fatal format file error; I'm stymied)"),
        "{:?}",
        y[5]
    );
    let _ = std::fs::remove_dir_all(&base);
}
