//! tex.ch \[53.1374\]'s `kpse_out_name_ok` under TeX Live's `openout_any =
//! p`: `\openout` to `../x.tex` or a dot file is refused, and kpathsea says
//! so on stderr after the program's `argv[0]` as given:
//! `/path/pdftex: Not writing to ../x.tex (openout_any = p; no extended
//! check).` (tex-file.c). This engine's kpathsea is set up from
//! `kpsewhich`'s path, which the message used to name (#2274, found by
//! tools/fuzz-parity).
//!
//! Both engines run as `pdftex` by their full path; stdout (after the
//! banner), stderr with each engine's own path replaced by `PDFTEX`, the
//! log (after the banner) and the exit status are compared. Skips where
//! there is no TeX Live (e.g. CI).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq)]
struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    log: String,
}

fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

fn run(bin: &Path, dir: &Path, tex: &str, ours: bool) -> Out {
    std::fs::write(dir.join("t.tex"), tex).unwrap();
    let mut c = Command::new(bin);
    c.args(["-ini", "-interaction=nonstopmode", "t.tex"])
        .current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("TEXMFOUTPUT")
        .env_remove("TEXMF_OUTPUT_DIRECTORY")
        .env_remove("openout_any")
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
    let me = bin.to_str().unwrap();
    Out {
        code: out.status.code(),
        stdout: after_banner(&String::from_utf8_lossy(&out.stdout)),
        stderr: String::from_utf8_lossy(&out.stderr).replace(me, "PDFTEX"),
        log: after_banner(&std::fs::read_to_string(dir.join("t.log")).unwrap()),
    }
}

#[test]
fn openout_refusal_names_the_engine() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let base = common::fresh_dir("flashtex-texch-openout-any");
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
    }
    let ours = a.join("pdftex");
    common::link_engine(Path::new(env!("CARGO_BIN_EXE_flashtex-initex")), &ours);
    for name in ["../fzup.tex", ".fzdot.tex"] {
        let tex = format!(
            "\\catcode`\\{{=1 \\catcode`\\}}=2 \\immediate\\openout3={name} \
             \\immediate\\write3{{x}}\\immediate\\closeout3 \\end\n"
        );
        let x = run(&ours, &a, &tex, true);
        let y = run(&texbin.join("pdftex"), &b, &tex, false);
        let want = format!("PDFTEX: Not writing to {name} (openout_any = p; no extended check).");
        assert!(
            y.stderr.contains(&want),
            "pdftex's stderr lacks {want:?}:\n{}",
            y.stderr
        );
        assert_eq!(x, y, "{name}: refused \\openout differs from pdftex");
    }
    let _ = std::fs::remove_dir_all(&base);
}
