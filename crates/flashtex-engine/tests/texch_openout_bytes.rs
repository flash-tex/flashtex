//! `\openout` to a name that is not UTF-8 (`fz^^e9.tex` in INITEX without a
//! TCX file: the byte `0xE9`): web2c's `fopen` opens the name's own bytes.
//! macOS refuses such a name (EILSEQ), so pdfTeX stops with `I can't write
//! on file`; Linux creates it. This engine used to create
//! `fz\u{FFFD}.tex` instead (#2298, found by tools/fuzz-parity).
//!
//! Both engines run as `pdftex`; stdout (after the banner), the log (after
//! the banner), the exit status and the names of the files left in the
//! directory must be pdfTeX's. Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

const TEX: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\catcode`\\^=7 \\immediate\\openout3=fz^^e9.tex \
\\immediate\\write3{e}\\immediate\\closeout3 \\message{written}\\end\n";

#[derive(Debug, PartialEq)]
struct Out {
    code: Option<i32>,
    stdout: String,
    log: String,
    files: Vec<Vec<u8>>,
}

fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

fn run(bin: &Path, dir: &Path, ours: bool) -> Out {
    std::fs::write(dir.join("t.tex"), TEX).unwrap();
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    let out = c.output().unwrap();
    let mut files: Vec<Vec<u8>> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().into_encoded_bytes())
        .filter(|n| n.as_slice() != b"pdftex")
        .collect();
    files.sort();
    Out {
        code: out.status.code(),
        stdout: after_banner(&String::from_utf8_lossy(&out.stdout)),
        log: after_banner(&String::from_utf8_lossy(
            &std::fs::read(dir.join("t.log")).unwrap(),
        )),
        files,
    }
}

#[test]
fn openout_opens_a_non_utf8_name_by_its_bytes() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-texch-openout-bytes");
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
    }
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    let x = run(&ours, &a, true);
    let y = run(&texbin.join("pdftex"), &b, false);
    if cfg!(target_os = "macos") {
        assert!(
            y.log.contains("! I can't write on file `fz^^e9.tex'."),
            "pdftex's log:\n{}",
            y.log
        );
    }
    assert!(
        !x.files
            .iter()
            .any(|n| n.windows(3).any(|w| w == "\u{FFFD}".as_bytes())),
        "a file named with U+FFFD was written: {:?}",
        x.files
    );
    assert_eq!(x, y, "\\openout to a non-UTF-8 name differs from pdftex");
    let _ = std::fs::remove_dir_all(&base);
}
