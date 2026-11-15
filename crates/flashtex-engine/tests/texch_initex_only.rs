//! tex.ch's `Init`...`Tini`: in web2c, INITEX and the production program are
//! one binary, and the parts of pdftex.web that only INITEX may run test
//! `ini_version` at run time (changes/virtex.ch):
//!
//! - tex.ch \[49.1252\]: `\patterns` in a production run is the error
//!   `Patterns can be loaded only by INITEX`, not `Too late for \patterns`;
//! - tex.ch \[51.1335\]: `\dump` in a production run ends the job with
//!   `(\dump is performed only by INITEX)` and writes no format.
//!
//! Each engine makes a format with its own INITEX, then runs `t.tex` with it
//! as `pdftex -fmt=f`; the terminal output, the log (after the banner), the
//! exit status and the files written are compared with TeX Live's pdfTeX.
//! Skips where there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

/// Everything after the first line (the banner, whose date the log has).
fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

#[test]
fn a_production_run_loads_no_patterns_and_dumps_no_format() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-texch-initex-only");
    let fmt = "\\catcode`\\{=1 \\catcode`\\}=2 \\dump\n";
    let doc = "\\catcode`\\{=1 \\catcode`\\}=2 \\patterns{a1b}\n\\showlists\n\\dump\n";
    let mut out = vec![];
    for (ours, sub) in [(true, "ours"), (false, "tex")] {
        let d = base.join(sub);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("f.tex"), fmt).unwrap();
        std::fs::write(d.join("t.tex"), doc).unwrap();
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
        let made = pass(&["-ini", "f.tex"]);
        assert!(d.join("f.fmt").is_file(), "{sub}: no format: {made:?}");
        let run = pass(&["-fmt=f", "-interaction=nonstopmode", "t.tex"]);
        let log = std::fs::read(d.join("t.log")).unwrap();
        out.push((
            run.status.code(),
            after_banner(&String::from_utf8_lossy(&run.stdout)),
            after_banner(&String::from_utf8_lossy(&log)),
            d.join("t.fmt").exists(),
        ));
    }
    let (x, y) = (&out[0], &out[1]);
    for want in [
        "! Patterns can be loaded only by INITEX.",
        "(\\dump is performed only by INITEX)",
    ] {
        assert!(y.2.contains(want), "pdftex's log lacks {want:?}:\n{}", y.2);
    }
    assert!(!y.3, "pdftex wrote t.fmt in a production run");
    assert_eq!(x.2, y.2, "the log differs from pdftex's");
    assert_eq!(x.1, y.1, "the terminal output differs from pdftex's");
    assert_eq!(x.0, y.0, "the exit status differs from pdftex's");
    assert_eq!(x.3, y.3, "whether t.fmt was written differs from pdftex");
    let _ = std::fs::remove_dir_all(&base);
}
