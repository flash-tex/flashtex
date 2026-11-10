//! The XeTeX twin of the pdfTeX engine's `texch_ini_only` and
//! `texch_openout_any` tests (#2453):
//!
//! * tex.ch's `Init`...`Tini` (`@d Init==init if ini_version then begin`):
//!   in a production run (a format loaded) `\patterns` is the error
//!   `Patterns can be loaded only by INITEX` (tex.ch \[49.1252\]) and
//!   `\dump` prints `(\dump is performed only by INITEX)` and writes no
//!   format (tex.ch \[51.1335\]);
//! * tex.ch \[53.1374\]'s `kpse_out_name_ok` under TeX Live's `openout_any
//!   = p`: `\openout` to `../x.tex` or a dot file is refused, and kpathsea
//!   says so on stderr after the program's `argv[0]` as given:
//!   `/path/xetex: Not writing to ../x.tex (openout_any = p; no extended
//!   check).` (tex-file.c).
//!
//! Each engine runs as `xetex` by its full path; terminal output (after the
//! banner), stderr with the engine's own path replaced by `XETEX`, the log
//! (after the banner), the exit status and the files left behind must be
//! TeX Live 2026 XeTeX's. The lines XeTeX printed are checked as well, so
//! the test also fails if both engines change. Skips where there is no TeX
//! Live (e.g. CI).

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq)]
struct Out {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    log: String,
    left: Vec<String>,
}

fn after_banner(s: &str) -> String {
    s.split_once('\n')
        .map(|(_, b)| b.to_string())
        .unwrap_or_default()
}

fn fresh(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn engine(bin: &Path, dir: &Path, ours: bool) -> Command {
    let mut c = Command::new(bin);
    c.current_dir(dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env_remove("TEXMFOUTPUT")
        .env_remove("TEXMF_OUTPUT_DIRECTORY")
        .env_remove("openout_any")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if ours {
        c.env("FLASHTEX_FORMATS", dir);
    }
    c
}

fn collect(bin: &Path, dir: &Path, job: &str, out: std::process::Output) -> Out {
    let me = bin.to_str().unwrap();
    let mut left: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("fz") || n == "t.fmt")
        .collect();
    left.sort();
    Out {
        code: out.status.code(),
        stdout: after_banner(&String::from_utf8_lossy(&out.stdout)),
        stderr: String::from_utf8_lossy(&out.stderr).replace(me, "XETEX"),
        log: after_banner(&std::fs::read_to_string(dir.join(format!("{job}.log"))).unwrap()),
        left,
    }
}

/// `-ini` makes `mini.fmt`, then a production run on it tries `\patterns`
/// and `\dump`.
fn production(bin: &Path, dir: &Path, ours: bool) -> Out {
    std::fs::write(
        dir.join("mk.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\dump\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2\n\\patterns{a1b}\n\\message{after}\n\\dump\n",
    )
    .unwrap();
    let mk = engine(bin, dir, ours)
        .args(["-ini", "-jobname=mini", "-no-pdf", "mk.tex"])
        .output()
        .unwrap();
    assert!(
        mk.status.success() && dir.join("mini.fmt").exists(),
        "{}: -ini did not make mini.fmt",
        bin.display()
    );
    let out = engine(bin, dir, ours)
        .args(["-fmt=mini", "-no-pdf", "-interaction=nonstopmode", "t.tex"])
        .output()
        .unwrap();
    collect(bin, dir, "t", out)
}

fn openout(bin: &Path, dir: &Path, name: &str, ours: bool) -> Out {
    std::fs::write(
        dir.join("w.tex"),
        format!(
            "\\catcode`\\{{=1 \\catcode`\\}}=2 \\immediate\\openout3={name} \
             \\immediate\\write3{{x}}\\immediate\\closeout3 \\end\n"
        ),
    )
    .unwrap();
    let out = engine(bin, dir, ours)
        .args(["-ini", "-no-pdf", "-interaction=nonstopmode", "w.tex"])
        .output()
        .unwrap();
    collect(bin, dir, "w", out)
}

fn pair(tag: &str) -> Option<(PathBuf, PathBuf, PathBuf, PathBuf)> {
    let texbin = find_texlive_bin()?;
    let base = fresh(tag);
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d.join("sub")).unwrap();
    }
    let ours = a.join("xetex");
    #[cfg(unix)]
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_flashtex-xetex"), &ours).unwrap();
    #[cfg(not(unix))]
    std::fs::copy(env!("CARGO_BIN_EXE_flashtex-xetex"), &ours).unwrap();
    Some((base, ours, a, texbin.join("xetex")))
}

#[test]
fn patterns_and_dump_are_initex_only() {
    let Some((base, ours, a, theirs)) = pair("ini-only") else {
        eprintln!("no TeX Live: skipped");
        return;
    };
    let b = base.join("tex");
    let x = production(&ours, &a, true);
    let y = production(&theirs, &b, false);
    for line in [
        "! Patterns can be loaded only by INITEX.",
        "(\\dump is performed only by INITEX)",
    ] {
        assert!(
            y.log.contains(line),
            "xetex's log lacks {line:?}:\n{}",
            y.log
        );
    }
    assert!(y.left.is_empty(), "xetex wrote t.fmt in a production run");
    assert_eq!(x, y, "production-run \\patterns/\\dump differ from xetex");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn openout_any_refuses_and_names_the_engine() {
    let Some((base, ours, a, theirs)) = pair("openout-any") else {
        eprintln!("no TeX Live: skipped");
        return;
    };
    let b = base.join("tex");
    for name in ["../fzup.tex", ".fzdot.tex", "sub/../fzsub.tex"] {
        let x = openout(&ours, &a, name, true);
        let y = openout(&theirs, &b, name, false);
        let want = format!("XETEX: Not writing to {name} (openout_any = p; no extended check).");
        assert!(
            y.stderr.contains(&want),
            "xetex's stderr lacks {want:?}:\n{}",
            y.stderr
        );
        assert_eq!(x, y, "{name}: refused \\openout differs from xetex");
        assert!(!base.join("fzup.tex").exists(), "{name}: written outside");
    }
    let _ = std::fs::remove_dir_all(&base);
}
