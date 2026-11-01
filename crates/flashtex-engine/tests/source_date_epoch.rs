//! `SOURCE_DATE_EPOCH` and `FORCE_SOURCE_DATE` as texmfmp.c reads them,
//! against TeX Live's own `pdftex`.
//!
//! texmfmp.c's `init_start_time` reads `SOURCE_DATE_EPOCH` with the C
//! library's `strtoull` and stops the run (kpathsea's `FATAL1`, exit 1,
//! before any log) on a value it does not take whole; `get_date_and_time`
//! warns on a `FORCE_SOURCE_DATE` other than "", "0" or "1". The engine used
//! to read any bad epoch as 0 and say nothing (lane REVIEW-PDF). Each value
//! runs a plain-TeX job that prints `\pdfcreationdate` and the date
//! primitives; exit status, stderr (program name normalised), the log from
//! the `**` line on and the PDF must all agree. Skips without TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// `\year` and the rest are the local clock unless `FORCE_SOURCE_DATE=1`,
/// so they are printed only for a year before 1980, which only a forced
/// epoch of these cases gives (a minute boundary between the two runs
/// cannot make them differ).
const DOC: &str = "\\pdfoutput=1 \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 \
                   \\message{[\\pdfcreationdate]}\
                   \\ifnum\\year<1980 \\message{[\\the\\year/\\the\\month/\\the\\day/\\the\\time]}\\fi\
                   A\\bye\n";

/// (SOURCE_DATE_EPOCH, FORCE_SOURCE_DATE); `None` leaves a variable unset.
/// The empty epoch is left out: macOS's `strtoull` rejects it (EINVAL) and
/// glibc's reads 0, and the engine follows whichever C library it links.
const CASES: &[(Option<&str>, Option<&str>)] = &[
    (Some("abc"), Some("1")),
    (Some("12abc"), Some("1")),
    (Some("5 "), Some("1")),
    (Some("1e3"), Some("1")),
    (Some("0x10"), Some("1")),
    (Some("99999999999999999999"), Some("1")),
    (Some(" 5"), Some("1")),
    (Some("+7"), Some("1")),
    (Some("-1"), Some("1")),
    (Some("18446744073709551615"), Some("1")),
    (Some("1700000000"), None),
    (Some("abc"), None),
    (Some("0"), Some("2")),
    (Some("0"), Some("yes")),
    (Some("0"), Some("0")),
    (Some("0"), Some("")),
];

struct Out {
    status: Option<i32>,
    stderr: String,
    log: Option<String>,
    pdf: Option<Vec<u8>>,
}

fn run(
    bin: &Path,
    dir: &Path,
    args: &[&str],
    ours: bool,
    env: (Option<&str>, Option<&str>),
) -> Out {
    for f in ["d.log", "d.pdf"] {
        let _ = std::fs::remove_file(dir.join(f));
    }
    let mut c = Command::new(bin);
    c.args(args)
        .current_dir(dir)
        .env_remove("SOURCE_DATE_EPOCH")
        .env_remove("FORCE_SOURCE_DATE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(v) = env.0 {
        c.env("SOURCE_DATE_EPOCH", v);
    }
    if let Some(v) = env.1 {
        c.env("FORCE_SOURCE_DATE", v);
    }
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    let o = c.output().unwrap();
    // kpathsea prints argv[0] before ": fatal: "
    let stderr = String::from_utf8_lossy(&o.stderr)
        .lines()
        .map(|l| match l.find(": fatal: ") {
            Some(i) => format!("PROG{}", &l[i..]),
            None => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let log = std::fs::read_to_string(dir.join("d.log")).ok().map(|l| {
        let from = l.find("\n**").map_or(0, |i| i + 1);
        l[from..].to_string()
    });
    Out {
        status: o.status.code(),
        stderr,
        log,
        pdf: std::fs::read(dir.join("d.pdf")).ok(),
    }
}

#[test]
fn source_date_epoch_matches_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-sde");
    let (a, b) = (base.join("ours"), base.join("tex"));
    let fmt_env = (Some("0"), Some("1"));
    for (d, bin, is_ours) in [(&a, ours, true), (&b, theirs.as_path(), false)] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(d.join("d.tex"), DOC).unwrap();
        run(bin, d, &["-ini", "\\input plain \\dump"], is_ours, fmt_env);
    }
    let mut failures = vec![];
    for &env in CASES {
        let x = run(ours, &a, &["-fmt=plain", "d"], true, env);
        let y = run(&theirs, &b, &["-fmt=plain", "d"], false, env);
        let what = format!(
            "SOURCE_DATE_EPOCH={:?} FORCE_SOURCE_DATE={:?}",
            env.0, env.1
        );
        if x.status != y.status {
            failures.push(format!(
                "{what}: exit {:?} vs TeX Live's {:?}",
                x.status, y.status
            ));
        }
        if x.stderr != y.stderr {
            failures.push(format!(
                "{what}: stderr\n  ours {:?}\n  tex  {:?}",
                x.stderr, y.stderr
            ));
        }
        if x.log != y.log {
            failures.push(format!(
                "{what}: log\n  ours {:?}\n  tex  {:?}",
                x.log, y.log
            ));
        }
        if x.pdf != y.pdf {
            failures.push(format!("{what}: PDFs differ"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
