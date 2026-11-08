//! `\pdfsetmatrix`'s four numbers read as utils.c's `pdfsetmatrix` reads
//! them, `sscanf(s, " %lf %lf %lf %lf %c")`, against TeX Live's own
//! `pdftex` (lane AUDIT-PDFC, #2229).
//!
//! `%lf` is `strtod`'s grammar: a number ends where the next cannot
//! continue it, so `1-0 0 1` and `.5.5 0 1` are four numbers, and
//! hexadecimal (`0x1p-1`) and `nan(1)` forms are numbers. The engine split
//! the text at blanks and parsed each piece with Rust's float parser, so
//! those matrices stopped the run with "Unrecognized format" where pdfTeX
//! applies them. Each case is an INITEX job whose annotation lies inside
//! the matrix, so its `/Rect` shows the numbers as read; exit status, PDF
//! and log (from the `**` line, the program name normalised) must agree.
//! Skips where there is no TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const SETUP: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0 \
                     \\pdfobjcompresslevel=0 \\pdfpagewidth=200pt \\pdfpageheight=200pt ";

/// (name, the argument of `\pdfsetmatrix`).
const CASES: &[(&str, &str)] = &[
    ("blanks", "1 0 0 1"),
    ("minus-joined", "1-0 0 1"),
    ("dots-joined", ".5.5 0 1"),
    ("plus-joined", "2+3 4 5"),
    ("hex", "0x1p-1 0 0 2"),
    ("nan-paren", "nan(1) 0 0 1"),
    ("inf", "INF 0 0 1"),
    ("exponents", "1e2 0 0 1E-2"),
    ("tab", "1\t0 0 1"),
    // failures in both: a fifth item, a sign alone, fewer numbers
    ("trailing-char", "1 0 0 1x"),
    ("sign-alone", "1 0 0 1+"),
    ("three", "1 0 0"),
];

fn run(bin: &Path, dir: &Path, name: &str, ours: bool) -> Option<i32> {
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=nonstopmode", &format!("{name}.tex")])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    c.status().unwrap().code()
}

/// The log from the `**` line on, lines joined, with the program name
/// after "pdfTeX warning: " and "!pdfTeX error: " replaced.
fn normalised_log(log: &str) -> String {
    let from = log.find("\n**").map_or(0, |i| i + 1);
    let mut s: String = log[from..].lines().collect();
    for key in ["pdfTeX warning: ", "!pdfTeX error: "] {
        let mut out = String::new();
        let mut rest = s.as_str();
        while let Some(i) = rest.find(key) {
            out.push_str(&rest[..i + key.len()]);
            rest = &rest[i + key.len()..];
            let end = rest.find([' ', ':']).unwrap_or(rest.len());
            out.push_str("PROG");
            rest = &rest[end..];
        }
        out.push_str(rest);
        s = out;
    }
    s
}

#[test]
fn setmatrix_numbers_scan_as_in_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-setmatrix-scan");
    let (a, b) = (base.join("ours"), base.join("tex"));
    let mut failures = vec![];
    for (name, m) in CASES {
        for d in [&a, &b] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(
                d.join(format!("{name}.tex")),
                format!(
                    "{SETUP}\\catcode9=12 \\shipout\\hbox{{\\pdfsave\\pdfsetmatrix{{{m}}}\
                     \\pdfannot width 10pt height 5pt depth 0pt {{/Subtype /Text}}\
                     \\pdfrestore}}\\end\n"
                ),
            )
            .unwrap();
        }
        let r = (run(ours, &a, name, true), run(&theirs, &b, name, false));
        if r.0 != r.1 {
            failures.push(format!("{name}: exit {:?} vs TeX Live's {:?}", r.0, r.1));
        }
        let pdf = format!("{name}.pdf");
        if std::fs::read(a.join(&pdf)).ok() != std::fs::read(b.join(&pdf)).ok() {
            failures.push(format!("{pdf}: differs (or one engine wrote none)"));
        }
        let log = format!("{name}.log");
        let lx = normalised_log(&std::fs::read_to_string(a.join(&log)).unwrap_or_default());
        let ly = normalised_log(&std::fs::read_to_string(b.join(&log)).unwrap_or_default());
        if lx != ly || lx.is_empty() {
            failures.push(format!("{log}:\n  ours {lx:?}\n  tex  {ly:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
