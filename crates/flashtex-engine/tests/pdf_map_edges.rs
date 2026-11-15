//! Font map and encoding file edge cases (src/pdftex/mapfile.rs, subfont.rs,
//! writet1/encoding.rs) against TeX Live's own `pdftex`: SlantFont and
//! ExtendFont values that saturate C's `int`, `/Flags` numbers too big for
//! `atoi`, and every `check_buf` overflow of a map line, a map file line
//! and an `.enc` line, whose fatal message names the C file and line as
//! TeX Live's build has them (lane REVIEW-PDF). Each case is a plain-TeX job;
//! exit status, the PDF (bytes, or none from either) and the log from the
//! `**` line on (lines joined, the program name normalised) must agree.
//! Skips where there is no TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const SETUP: &str = "\\pdfoutput=1 \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 \
                     \\pdfsuppressptexinfo=-1 ";

/// (name, document). `files` is the directory of [`support_files`], named by
/// its absolute path (shared by both engines, so their logs name the same
/// file).
fn cases(files: &Path) -> Vec<(String, String)> {
    let f = files.display();
    let x = "x".repeat(1100);
    let mut v: Vec<(String, String)> = [
        // (integer)(d - 0.5) of -1e33 is INT_MIN, which `abs(x) > 1000`
        // rejects (the entry is dropped with a warning)
        (
            "slant-min",
            "\\pdfmapline{=cmr10 CMR10 \"-1e30 SlantFont\" <cmr10.pfb}".to_string(),
        ),
        (
            "extend-min",
            "\\pdfmapline{=cmr10 CMR10 \"-1e30 ExtendFont\" <cmr10.pfb}".to_string(),
        ),
        (
            "extend-max",
            "\\pdfmapline{=cmr10 CMR10 \"1e30 ExtendFont\" <cmr10.pfb}".to_string(),
        ),
        // atoi: strtol saturates, then the cast to int
        (
            "flags-huge",
            "\\pdfmapline{=cmr10 CMR10 123456789012345678901234 <cmr10.pfb}".to_string(),
        ),
        (
            "flags-wrap",
            "\\pdfmapline{=cmr10 CMR10 4294967297 <cmr10.pfb}".to_string(),
        ),
        (
            "flags-2p31",
            "\\pdfmapline{=cmr10 CMR10 2147483648 <cmr10.pfb}".to_string(),
        ),
        // read_field's check_buf at mapfile.c's lines 455, 460 and 531
        ("over-tfm", format!("\\pdfmapline{{={x}}}")),
        ("over-ps", format!("\\pdfmapline{{=cmr10 {x}}}")),
        (
            "over-fontfile",
            format!("\\pdfmapline{{=cmr10 CMR10 <<{x}}}"),
        ),
        (
            "over-enc",
            format!("\\pdfmapline{{=cmr10 CMR10 <{x}.enc <cmr10.pfb}}"),
        ),
        // a map file's line: append_char_to_buf at line 440
        ("mapfile-1023", format!("\\pdfmapfile{{{f}/l1023.map}}")),
        ("mapfile-1024", format!("\\pdfmapfile{{{f}/l1024.map}}")),
    ]
    .into_iter()
    .map(|(n, b)| (n.to_string(), b))
    .collect();
    // an .enc line of N bytes: append_char_to_buf (writet1.c line 223) from
    // 4096, append_eol (line 225) from 4094
    for n in 4092..=4097 {
        v.push((
            format!("enc-{n}"),
            format!("\\pdfmapline{{=cmr10 CMR10 <{f}/e{n}.enc <cmr10.pfb}}"),
        ));
    }
    v.into_iter()
        .map(|(n, b)| (n, format!("{SETUP}{b}Ab\\bye\n")))
        .collect()
}

/// The support files: `.enc` files with one long line, map files whose
/// one line is 1023 and 1024 bytes long (blanks are not doubled, since a
/// map file's blanks collapse).
fn support_files(dir: &Path) {
    for n in 4092..=4097usize {
        let name = format!("/{}", "a".repeat(n - 1));
        let rest = " /a".repeat(255);
        std::fs::write(
            dir.join(format!("e{n}.enc")),
            format!("/E [\n{name}\n{rest} ] def\n"),
        )
        .unwrap();
    }
    for n in [1023usize, 1024] {
        let base = "cmr10 CMR10 <cmr10.pfb";
        let line = format!("{base} {}", "y".repeat(n - base.len() - 1));
        std::fs::write(dir.join(format!("l{n}.map")), format!("{line}\n")).unwrap();
    }
}

fn run(bin: &Path, dir: &Path, args: &[&str], ours: bool) -> Option<i32> {
    let mut c = Command::new(bin);
    c.args(args)
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
fn map_edges_match_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-mapedges");
    let (a, b) = (base.join("ours"), base.join("tex"));
    let files = base.join("files");
    std::fs::create_dir_all(&files).unwrap();
    support_files(&files);
    let cases = cases(&files);
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        for (name, body) in &cases {
            std::fs::write(d.join(format!("{name}.tex")), body).unwrap();
        }
    }
    run(ours, &a, &["-ini", "\\input plain \\dump"], true);
    run(&theirs, &b, &["-ini", "\\input plain \\dump"], false);
    let mut failures = vec![];
    for (name, _) in &cases {
        let arg = format!("&plain {name}");
        let rx = run(ours, &a, &["-fmt=plain", &arg], true);
        let ry = run(&theirs, &b, &[&arg], false);
        if rx != ry {
            failures.push(format!("{name}: exit {rx:?} vs TeX Live's {ry:?}"));
        }
        let pdf = format!("{name}.pdf");
        let (x, y) = (
            std::fs::read(a.join(&pdf)).ok(),
            std::fs::read(b.join(&pdf)).ok(),
        );
        if x != y {
            failures.push(format!(
                "{pdf}: ours {:?} bytes, TeX Live's {:?}",
                x.map(|v| v.len()),
                y.map(|v| v.len())
            ));
        }
        let log = format!("{name}.log");
        let lx = normalised_log(&std::fs::read_to_string(a.join(&log)).unwrap_or_default());
        let ly = normalised_log(&std::fs::read_to_string(b.join(&log)).unwrap_or_default());
        if lx != ly || lx.is_empty() {
            let at = lx
                .chars()
                .zip(ly.chars())
                .position(|(p, q)| p != q)
                .unwrap_or(lx.len().min(ly.len()));
            let ctx = |s: &str| {
                s.chars()
                    .skip(at.saturating_sub(40))
                    .take(160)
                    .collect::<String>()
            };
            failures.push(format!(
                "{log}: differs at {at}:\n  ours {:?}\n  tex  {:?}",
                ctx(&lx),
                ctx(&ly)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
