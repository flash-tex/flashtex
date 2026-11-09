//! An included PDF's font named `<name>-Slant_<n>` is replaced by the map
//! entry whose slant is `(int) strtol(n)`, as mapfile.c's `lookup_fontmap`
//! reads it, against TeX Live's own `pdftex` (lane AUDIT-PDFC, #2241).
//!
//! `strtol` gives a 64-bit `long` (`LONG_MAX`/`LONG_MIN` past that) and the
//! cast keeps its low 32 bits, so `-Slant_4294967463` is slant 167 and
//! `-Slant_99999999999999999999` is -1. The engine clamped to `int`'s range,
//! found no entry and copied the font where pdfTeX embeds the map's
//! `cmr10.pfb`. The PDF to include is made by TeX Live's pdftex and its
//! `/BaseFont` renamed; exit status, PDF and log (from the `**` line, the
//! program name normalised) must agree. Skips where there is no TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const SETUP: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1 \\pdfcompresslevel=0 \
                     \\pdfobjcompresslevel=0 \\pdfpagewidth=100pt \\pdfpageheight=50pt ";

/// (name, the suffix given to the included font's name).
const CASES: &[(&str, &str)] = &[
    ("slant-167", "-Slant_167"),
    ("slant-int-wrap", "-Slant_4294967463"),
    ("slant-long-overflow", "-Slant_99999999999999999999"),
    ("extend-int-wrap", "-Slant_167-Extend_4294968196"),
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

/// `pdf` with `from` replaced by `to` and its classic xref table and
/// `startxref` rebuilt (pdfTeX's uncompressed output: every object starts
/// a line with `N 0 obj`).
fn renamed(pdf: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let at = pdf
        .windows(from.len())
        .position(|w| w == from)
        .expect("the font name is in the PDF");
    let mut data = pdf[..at].to_vec();
    data.extend_from_slice(to);
    data.extend_from_slice(&pdf[at + from.len()..]);
    let find = |h: &[u8], n: &[u8], from: usize| {
        h[from..]
            .windows(n.len())
            .position(|w| w == n)
            .map(|i| i + from)
    };
    let x = data
        .windows(6)
        .rposition(|w| w == b"\nxref\n")
        .expect("a classic xref table");
    let body = data[..x + 1].to_vec();
    let mut offs: Vec<Option<usize>> = vec![];
    let mut line = 0;
    while line < body.len() {
        let end = find(&body, b"\n", line).unwrap_or(body.len());
        let l = &body[line..end];
        if let Some(sp) = l.iter().position(|&c| c == b' ') {
            if l[sp..].starts_with(b" 0 obj") && l[..sp].iter().all(u8::is_ascii_digit) {
                let n: usize = std::str::from_utf8(&l[..sp]).unwrap().parse().unwrap();
                if offs.len() <= n {
                    offs.resize(n + 1, None);
                }
                offs[n] = Some(line);
            }
        }
        line = end + 1;
    }
    let trailer = find(&data, b"trailer", x).unwrap();
    let startxref = find(&data, b"startxref", trailer).unwrap();
    let mut out = body.clone();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offs.len()).as_bytes());
    for o in &offs[1..] {
        out.extend_from_slice(
            match o {
                Some(o) => format!("{o:010} 00000 n \n"),
                None => "0000000000 65535 f \n".to_string(),
            }
            .as_bytes(),
        );
    }
    out.extend_from_slice(&data[trailer..startxref]);
    out.extend_from_slice(format!("startxref\n{}\n%%EOF\n", body.len()).as_bytes());
    out
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
fn slant_and_extend_names_read_as_strtol_then_int() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-fontmap-slant-name");
    let (a, b) = (base.join("ours"), base.join("tex"));
    // the PDF to include: cmr10 embedded by TeX Live's pdftex
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(
        b.join("src.tex"),
        format!("{SETUP}\\font\\a=cmr10 \\shipout\\hbox{{\\a Hello}}\\end\n"),
    )
    .unwrap();
    run(&theirs, &b, "src", false);
    let src = std::fs::read(b.join("src.pdf")).expect("pdftex made src.pdf");
    // "/BaseFont /ABCDEF+CMR10"
    let from = src
        .windows(23)
        .find(|w| w.starts_with(b"/BaseFont /") && w.ends_with(b"+CMR10"))
        .expect("a subset cmr10 in src.pdf")
        .to_vec();
    let mut failures = vec![];
    for (name, suffix) in CASES {
        let mut to = from.clone();
        to.extend_from_slice(suffix.as_bytes());
        let pdf = renamed(&src, &from, &to);
        for d in [&a, &b] {
            std::fs::create_dir_all(d).unwrap();
            std::fs::write(d.join(format!("{name}.pdf")), &pdf).unwrap();
            std::fs::write(
                d.join(format!("{name}-job.tex")),
                format!(
                    "{SETUP}\\pdfmapline{{+cmrsa CMR10 \"0.167 SlantFont\" <cmr10.pfb}}\
                     \\pdfmapline{{+cmrsb CMR10 \"-0.001 SlantFont\" <cmr10.pfb}}\
                     \\pdfmapline{{+cmrsc CMR10 \"0.167 SlantFont 0.9 ExtendFont\" <cmr10.pfb}}\
                     \\pdfximage{{{name}.pdf}}\\shipout\\hbox{{\\pdfrefximage\\pdflastximage}}\\end\n"
                ),
            )
            .unwrap();
        }
        let job = format!("{name}-job");
        let r = (run(ours, &a, &job, true), run(&theirs, &b, &job, false));
        if r.0 != r.1 {
            failures.push(format!("{name}: exit {:?} vs TeX Live's {:?}", r.0, r.1));
        }
        let pdf = format!("{job}.pdf");
        if std::fs::read(a.join(&pdf)).ok() != std::fs::read(b.join(&pdf)).ok() {
            failures.push(format!("{pdf}: differs (or one engine wrote none)"));
        }
        let log = format!("{job}.log");
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
