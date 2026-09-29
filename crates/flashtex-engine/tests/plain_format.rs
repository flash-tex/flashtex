//! End to end through kpathsea: build plain.fmt from the installed TeX Live's
//! plain.tex and hyphen.tex, load it, typeset a box in cmr10, and compare both
//! logs with TeX Live's own `pdftex` (DVI mode, compatibility mode) line by
//! line. Skips where there is no TeX Live (e.g. CI). The lines allowed to
//! differ, each for a reason that is not typesetting:
//!
//! * the banner (web2c adds "(TeX Live 2026)"; the date is the clock's) and the
//!   format's date stamp;
//! * web2c's two status lines " restricted \write18 enabled." and
//!   " %&-line parsing enabled.", which this engine does not print yet;
//! * the pool-string count, which web2c's tex.ch raises with strings of its
//!   own, and the memory words dumped, which SyncTeX's node fields raise in
//!   TeX Live (DESIGN.md section 1.1 normalises memory accounting);
//! * the count of multiletter control sequences: TeX Live's pdfTeX has five
//!   primitives from web2c change files (`\tracingstacklevels`,
//!   `\partokenname`, `\partokencontext`, `\showstream`, `\synctex`) that
//!   pdftex.web does not define.
#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

fn run(bin: &Path, dir: &Path, arg: &str, ours: bool) {
    let mut c = Command::new(bin);
    c.arg(arg)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    c.status().unwrap();
}

fn comparable(log: &str) -> Vec<String> {
    log.lines()
        .skip(1)
        .filter(|l| {
            !l.contains("strings of total length")
                && !l.contains("(preloaded format=")
                && !l.contains("\\write18 enabled.")
                && !l.contains(" %&-line parsing enabled.")
                && !l.contains("memory locations dumped; current usage is")
                && !l.ends_with(" multiletter control sequences")
        })
        .map(str::to_string)
        .collect()
}

#[test]
fn plain_format_matches_tex_live() {
    let Some(texbin) = find_texlive_bin() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = std::env::temp_dir().join(format!("flashtex-plain-{}", std::process::id()));
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        std::fs::write(
            d.join("s.tex"),
            "\\batchmode\\showthe\\hsize \\message{[\\fmtname]}\
             \\setbox0\\hbox{Hello, world}\\showbox0 \\end\n",
        )
        .unwrap();
    }
    run(ours, &a, "\\input plain \\dump", true);
    Command::new(&theirs)
        .args(["-ini", "\\input plain \\dump"])
        .current_dir(&b)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    run(ours, &a, "&plain s", true);
    Command::new(&theirs)
        .arg("&plain s")
        .current_dir(&b)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    for log in ["plain.log", "s.log"] {
        let x = std::fs::read_to_string(a.join(log)).unwrap();
        let y = std::fs::read_to_string(b.join(log)).unwrap();
        assert_eq!(
            comparable(&x),
            comparable(&y),
            "{log} differs from TeX Live's pdftex"
        );
    }
    let s = std::fs::read_to_string(a.join("s.log")).unwrap();
    assert!(s.contains("\\tenrm H"), "{s}");
    let _ = std::fs::remove_dir_all(&base);
}
