//! Font map and encoding files opened as pdfTeX opens them (mapfile.c's
//! `fm_open`, writet1.c's `enc_open`: lib/openclose.c's `open_input`),
//! against TeX Live's own `pdftex` (lane REVIEW-PDF).
//!
//! `open_input` looks in `-output-directory` first, records the file for
//! `-recorder`, and leaves `nameoffile` without the `./` kpathsea puts in
//! front of a file found in the current directory (unless the name asked
//! for had it), so pdfTeX logs `{my.map}` and `(file my.map)`. The engine
//! used kpathsea's answer as it was: `{./my.map}`, no output directory, no
//! recorder line. Each case is a plain-TeX job; exit status, the PDF and
//! the log from the `**` line on (lines joined, the program name
//! normalised) must agree, and with `-recorder` the `.fls` lines that name
//! the map and encoding files. Skips where there is no TeX Live.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const SETUP: &str = "\\pdfoutput=1 \\pdfcompresslevel=0 \\pdfobjcompresslevel=0 \
                     \\pdfsuppressptexinfo=-1 ";

/// (name, body after `SETUP`).
const CASES: &[(&str, &str)] = &[
    ("local-map", "\\pdfmapfile{my.map}"),
    ("local-map-plus", "\\pdfmapfile{+my.map}"),
    ("dot-map", "\\pdfmapfile{./my.map}"),
    (
        "local-enc",
        "\\pdfmapline{=cmr10 CMR10 <my8r.enc <cmr10.pfb}",
    ),
    (
        "dot-enc",
        "\\pdfmapline{=cmr10 CMR10 <./my8r.enc <cmr10.pfb}",
    ),
    ("no-map", "\\pdfmapfile{nosuch.map}"),
    (
        "no-enc",
        "\\pdfmapline{=cmr10 CMR10 <nosuch.enc <cmr10.pfb}",
    ),
];

/// A map file whose second line repeats the first and whose third is bad,
/// so its warnings name the file, and a copy of TeX Live's 8r.enc.
fn support_files(texbin: &Path, dir: &Path) {
    std::fs::write(
        dir.join("my.map"),
        "cmr10 CMR10 <my8r.enc <cmr10.pfb\ncmr10 CMR10 <cmr10.pfb\nbad \"abc <cmr10.pfb\n",
    )
    .unwrap();
    let out = Command::new(texbin.join("kpsewhich"))
        .arg("8r.enc")
        .output()
        .unwrap();
    let enc = String::from_utf8(out.stdout).unwrap();
    std::fs::copy(enc.trim(), dir.join("my8r.enc")).unwrap();
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

/// Compare one job's exit status, PDF and log in `a` (ours) and `b`.
fn compare(
    name: &str,
    out: &str,
    (a, b): (&Path, &Path),
    r: (Option<i32>, Option<i32>),
) -> Vec<String> {
    let mut failures = vec![];
    if r.0 != r.1 {
        failures.push(format!("{name}: exit {:?} vs TeX Live's {:?}", r.0, r.1));
    }
    let pdf = format!("{out}/{name}.pdf");
    if std::fs::read(a.join(&pdf)).ok() != std::fs::read(b.join(&pdf)).ok() {
        failures.push(format!("{pdf}: differs (or one engine wrote none)"));
    }
    let log = format!("{out}/{name}.log");
    let lx = normalised_log(&std::fs::read_to_string(a.join(&log)).unwrap_or_default());
    let ly = normalised_log(&std::fs::read_to_string(b.join(&log)).unwrap_or_default());
    if lx != ly || lx.is_empty() {
        failures.push(format!("{log}:\n  ours {lx:?}\n  tex  {ly:?}"));
    }
    failures
}

#[test]
fn map_and_enc_files_open_as_in_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-mapencopen");
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d.join("od")).unwrap();
        support_files(&texbin, d);
        support_files(&texbin, &d.join("od"));
        for (name, body) in CASES {
            std::fs::write(
                d.join(format!("{name}.tex")),
                format!("{SETUP}{body}Ab\\bye\n"),
            )
            .unwrap();
        }
        // the map and encoding files only in the output directory
        std::fs::write(
            d.join("od.tex"),
            format!(
                "{SETUP}\\pdfmapfile{{od.map}}\\pdfmapline{{=cmbx10 CMBX10 <od8r.enc <cmbx10.pfb}}\
                 Ab {{\\bf Cd}}\\bye\n"
            ),
        )
        .unwrap();
        std::fs::rename(d.join("od/my.map"), d.join("od/od.map")).unwrap();
        std::fs::rename(d.join("od/my8r.enc"), d.join("od/od8r.enc")).unwrap();
    }
    run(ours, &a, &["-ini", "\\input plain \\dump"], true);
    run(&theirs, &b, &["-ini", "\\input plain \\dump"], false);
    let mut failures = vec![];
    for (name, _) in CASES {
        let arg = format!("&plain {name}");
        let r = (
            run(ours, &a, &["-fmt=plain", &arg], true),
            run(&theirs, &b, &[&arg], false),
        );
        failures.extend(compare(name, ".", (&a, &b), r));
    }
    let args = ["-recorder", "-output-directory=od", "-fmt=plain", "od"];
    let r = (run(ours, &a, &args, true), run(&theirs, &b, &args, false));
    failures.extend(compare("od", "od", (&a, &b), r));
    // The map and encoding files found in the output directory. (The .fls
    // names its own file after the program and process id; od.map's
    // my8r.enc, found in the current directory, is left out: the engine's
    // recorder writes every such file with kpathsea's `./`.)
    let fls = |d: &Path| -> Vec<String> {
        let mut lines = vec![];
        for e in std::fs::read_dir(d.join("od")).unwrap().flatten() {
            if e.path().extension().is_some_and(|x| x == "fls") {
                let t = std::fs::read_to_string(e.path()).unwrap();
                lines.extend(
                    t.lines()
                        .filter(|l| l.starts_with("INPUT od/"))
                        .filter(|l| l.ends_with(".map") || l.ends_with(".enc"))
                        .map(str::to_string),
                );
            }
        }
        lines
    };
    let (fx, fy) = (fls(&a), fls(&b));
    if fx != fy || fy.is_empty() {
        failures.push(format!(
            "od.fls map/enc lines:\n  ours {fx:?}\n  tex  {fy:?}"
        ));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    if std::env::var_os("FLASHTEX_KEEP_TEST_DIR").is_none() {
        let _ = std::fs::remove_dir_all(&base);
    }
}
