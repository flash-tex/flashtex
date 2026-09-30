//! `\write18` as TeX Live's pdfTeX does it: restricted by default to
//! texmf.cnf's `shell_escape_commands`, with web2c's log lines, and every
//! command that runs recorded as an external effect.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\
\\message{[\\the\\pdfshellescape]}\n\
\\immediate\\write18{kpsewhich plain.tex}\n\
\\immediate\\write18{ls}\n\
\\immediate\\write18{kpsewhich 'plain.tex'}\n\
\\message{[\\ifeof18 off\\else on\\fi]}\n\
\\end\n";

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-w18-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("w.tex"), DOC).unwrap();
    d
}

fn ours(dir: &Path, envs: &[(&str, &str)]) -> String {
    let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    c.args(["-ini", "w.tex"])
        .current_dir(dir)
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .env("FLASHTEX_EXTERNAL_EFFECTS", dir.join("effects.txt"))
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (k, v) in envs {
        c.env(k, v);
    }
    c.status().unwrap();
    std::fs::read_to_string(dir.join("w.log")).unwrap()
}

/// texmf.cnf's `shell_escape = p` makes the default restricted. Here the
/// working-directory resolver has no texmf.cnf, so the two values come from
/// the environment, where kpathsea looks first.
#[test]
fn restricted_by_default_and_effects_recorded() {
    let d = scratch("env");
    let log = ours(
        &d,
        &[
            ("FLASHTEX_RESOLVER", "cwd"),
            ("shell_escape", "p"),
            ("shell_escape_commands", "kpsewhich,bibtex"),
        ],
    );
    assert!(log.contains("\n restricted \\write18 enabled.\n"), "{log}");
    assert!(log.contains("[2]"), "{log}");
    assert!(
        log.contains("runsystem(kpsewhich plain.tex)...executed safely (allowed)."),
        "{log}"
    );
    assert!(
        log.contains("runsystem(ls)...disabled (restricted)."),
        "{log}"
    );
    assert!(
        log.contains("runsystem(kpsewhich 'plain.tex')...quotation error in system command."),
        "{log}"
    );
    assert!(log.contains("[on]"), "{log}");
    let effects = std::fs::read_to_string(d.join("effects.txt")).unwrap();
    assert_eq!(effects, "write18 kpsewhich 'plain.tex'\n");
    let _ = std::fs::remove_dir_all(&d);
}

/// `-no-shell-escape` still turns it off.
#[test]
fn no_shell_escape() {
    let d = scratch("off");
    let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    c.args(["-no-shell-escape", "-ini", "w.tex"])
        .current_dir(&d)
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("FLASHTEX_EXTERNAL_EFFECTS", d.join("effects.txt"))
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    c.status().unwrap();
    let log = std::fs::read_to_string(d.join("w.log")).unwrap();
    assert!(!log.contains("write18 enabled"), "{log}");
    assert!(log.contains("[0]"), "{log}");
    assert!(log.contains("runsystem(ls)...disabled."), "{log}");
    assert!(log.contains("[off]"), "{log}");
    assert!(!d.join("effects.txt").exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// Against TeX Live's own pdftex in its default mode, through kpathsea and
/// TeX Live's texmf.cnf: the logs are identical after the banner.
#[cfg(feature = "kpathsea")]
#[test]
fn matches_tex_live_default() {
    let Some(texbin) = flashtex_engine::resolver::find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let (a, b) = (scratch("ours"), scratch("tl"));
    let x = ours(&a, &[]);
    Command::new(texbin.join("pdftex"))
        .args(["-ini", "w.tex"])
        .current_dir(&b)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    let y = std::fs::read_to_string(b.join("w.log")).unwrap();
    let body = |s: &str| s.lines().skip(1).map(str::to_string).collect::<Vec<_>>();
    assert_eq!(body(&x), body(&y), "w.log differs from TeX Live's pdftex");
    assert!(x.contains("runsystem(kpsewhich plain.tex)...executed safely (allowed)."));
    assert!(x.contains("runsystem(ls)...disabled (restricted)."));
    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}
