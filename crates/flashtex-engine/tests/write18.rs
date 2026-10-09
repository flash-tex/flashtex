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
    let d = common::fresh_dir(&format!("flashtex-w18-{tag}"));
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

/// makeindex through restricted `\write18` runs in-process (the makeindex
/// port, crates/makeindex), so it needs no TeX Live program: here no
/// `makeindex` is on PATH at all, and the `.ind` and `.ilg` are made all
/// the same, with the run recorded as an external effect as before.
/// `FLASHTEX_MAKEINDEX=external` goes back to the shell, which then finds
/// no program.
#[cfg(all(feature = "makeindex", unix))]
#[test]
fn makeindex_runs_in_process() {
    const IDX_DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\
\\immediate\\write18{makeindex x.idx}\n\
\\end\n";
    let run = |tag: &str, extra: &[(&str, &str)]| {
        let d = common::fresh_dir(&format!("flashtex-w18-mki-{tag}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("w.tex"), IDX_DOC).unwrap();
        std::fs::write(
            d.join("x.idx"),
            "\\indexentry{beta}{2}\n\\indexentry{alpha!one}{1}\n\\indexentry{alpha}{3}\n",
        )
        .unwrap();
        let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
        c.args(["-ini", "w.tex"])
            .current_dir(&d)
            .env(
                "FLASHTEX_POOL",
                Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
            )
            .env("FLASHTEX_EXTERNAL_EFFECTS", d.join("effects.txt"))
            .env("FLASHTEX_RESOLVER", "cwd")
            .env("shell_escape", "p")
            .env("shell_escape_commands", "makeindex")
            .env("PATH", "/nonexistent")
            .env_remove("FLASHTEX_MAKEINDEX")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        for (k, v) in extra {
            c.env(k, v);
        }
        let o = c.output().unwrap();
        (d, String::from_utf8_lossy(&o.stderr).into_owned())
    };

    let (d, stderr) = run("in", &[]);
    let log = std::fs::read_to_string(d.join("w.log")).unwrap();
    assert!(
        log.contains("runsystem(makeindex x.idx)...executed safely (allowed)."),
        "{log}"
    );
    assert_eq!(
        std::fs::read_to_string(d.join("effects.txt")).unwrap(),
        "write18 makeindex 'x.idx'\n"
    );
    // TeX Live 2026's makeindex on the same .idx
    assert_eq!(
        std::fs::read_to_string(d.join("x.ind")).unwrap(),
        "\\begin{theindex}\n\n  \\item alpha, 3\n    \\subitem one, 1\n\n  \
\\indexspace\n\n  \\item beta, 2\n\n\\end{theindex}\n"
    );
    let ilg = std::fs::read_to_string(d.join("x.ilg")).unwrap();
    assert!(
        ilg.starts_with(
            "This is makeindex, version 2.18 [TeX Live 2026] (kpathsea + Thai support).\n"
        ),
        "{ilg}"
    );
    assert!(stderr.contains("Output written in x.ind."), "{stderr}");
    assert!(!stderr.contains("system returned"), "{stderr}");
    let _ = std::fs::remove_dir_all(&d);

    let (d, stderr) = run("ext", &[("FLASHTEX_MAKEINDEX", "external")]);
    assert!(!d.join("x.ind").exists());
    assert!(
        stderr.contains("system returned with code 32512"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// bibtex through restricted `\write18` runs in-process too (the BibTeX
/// port, crates/bibtex): no `bibtex` on PATH, and the `.bbl` and `.blg` are
/// made, the run recorded as an external effect. `FLASHTEX_BIBTEX=external`
/// goes back to the shell, which then finds no program.
#[cfg(all(feature = "bibtex", unix))]
#[test]
fn bibtex_runs_in_process() {
    const BIB_DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\
\\immediate\\write18{bibtex x}\n\
\\end\n";
    // A style of the test's own: one line per entry.
    const BST: &str = "ENTRY { title } {} {}\n\
FUNCTION {book} { \"\\bibitem{\" cite$ * \"}\" * write$ newline$ title write$ newline$ }\n\
READ\n\
ITERATE {call.type$}\n";
    let run = |tag: &str, extra: &[(&str, &str)]| {
        let d = common::fresh_dir(&format!("flashtex-w18-bib-{tag}"));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("w.tex"), BIB_DOC).unwrap();
        std::fs::write(
            d.join("x.aux"),
            "\\citation{b}\n\\citation{a}\n\\bibstyle{one}\n\\bibdata{refs}\n",
        )
        .unwrap();
        std::fs::write(d.join("one.bst"), BST).unwrap();
        std::fs::write(
            d.join("refs.bib"),
            "@book{a, title = {Alpha}}\n@book{b, title = \"Beta\"}\n",
        )
        .unwrap();
        let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
        c.args(["-ini", "w.tex"])
            .current_dir(&d)
            .env(
                "FLASHTEX_POOL",
                Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
            )
            .env("FLASHTEX_EXTERNAL_EFFECTS", d.join("effects.txt"))
            .env("FLASHTEX_RESOLVER", "cwd")
            .env("shell_escape", "p")
            .env("shell_escape_commands", "bibtex")
            .env("PATH", "/nonexistent")
            .env_remove("FLASHTEX_BIBTEX")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in extra {
            c.env(k, v);
        }
        let o = c.output().unwrap();
        (
            d,
            String::from_utf8_lossy(&o.stdout).into_owned(),
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };

    let (d, stdout, stderr) = run("in", &[]);
    let log = std::fs::read_to_string(d.join("w.log")).unwrap();
    assert!(
        log.contains("runsystem(bibtex x)...executed safely (allowed)."),
        "{log}"
    );
    assert_eq!(
        std::fs::read_to_string(d.join("effects.txt")).unwrap(),
        "write18 bibtex 'x'\n"
    );
    // In citation order, as the style writes them.
    assert_eq!(
        std::fs::read_to_string(d.join("x.bbl")).unwrap(),
        "\\bibitem{b}\nBeta\n\\bibitem{a}\nAlpha\n"
    );
    let blg = std::fs::read_to_string(d.join("x.blg")).unwrap();
    // No texmf.cnf here: bibtex.ch's built-in capacities.
    assert!(
        blg.starts_with(
            "This is BibTeX, Version 0.99e (TeX Live 2026)\n\
Capacity: max_strings=4000, hash_size=5000, hash_prime=4253\n\
The top-level auxiliary file: x.aux\n\
The style file: one.bst\n\
Database file #1: refs.bib\n"
        ),
        "{blg}"
    );
    assert!(
        stdout.contains("The top-level auxiliary file: x.aux"),
        "{stdout}"
    );
    assert!(!stderr.contains("system returned"), "{stderr}");
    let _ = std::fs::remove_dir_all(&d);

    let (d, _, stderr) = run("ext", &[("FLASHTEX_BIBTEX", "external")]);
    assert!(!d.join("x.bbl").exists());
    assert!(
        stderr.contains("system returned with code 32512"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// `\write18{makeindex z.idx}` with `z.idx` a link to `/dev/zero`: the C
/// program would read without end; the in-process port refuses the file
/// (not a regular file) and fails as for a missing input, so the engine
/// (and a host running it) carries on.
#[cfg(all(feature = "makeindex", unix))]
#[test]
fn makeindex_refuses_dev_zero() {
    const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\
\\immediate\\write18{makeindex z.idx}\n\
\\end\n";
    let d = common::fresh_dir("flashtex-w18-mki-zero");
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("w.tex"), DOC).unwrap();
    std::os::unix::fs::symlink("/dev/zero", d.join("z.idx")).unwrap();
    let t0 = std::time::Instant::now();
    let o = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(["-ini", "w.tex"])
        .current_dir(&d)
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("shell_escape", "p")
        .env("shell_escape_commands", "makeindex")
        .env("PATH", "/nonexistent")
        .env_remove("FLASHTEX_MAKEINDEX")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(t0.elapsed() < std::time::Duration::from_secs(60));
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        stderr.contains("makeindex: refusing to read z.idx: not a regular file"),
        "{stderr}"
    );
    assert!(stderr.contains("system returned with code 256"), "{stderr}");
    let _ = std::fs::remove_dir_all(&d);
}

/// `\write18{bibtex ...}` with inputs linked to `/dev/zero`: the C program
/// would read without end; the in-process port finds no database there
/// (kpathsea's lookup takes regular files only) and refuses the `.aux`
/// (not a regular file), so the engine carries on.
#[cfg(all(feature = "bibtex", unix))]
#[test]
fn bibtex_refuses_dev_zero() {
    const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2\n\
\\immediate\\write18{bibtex x}\n\
\\immediate\\write18{bibtex y}\n\
\\end\n";
    let d = common::fresh_dir("flashtex-w18-bib-zero");
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("w.tex"), DOC).unwrap();
    std::fs::write(
        d.join("x.aux"),
        "\\citation{a}\n\\bibstyle{plain}\n\\bibdata{refs}\n",
    )
    .unwrap();
    std::os::unix::fs::symlink("/dev/zero", d.join("refs.bib")).unwrap();
    std::os::unix::fs::symlink("/dev/zero", d.join("y.aux")).unwrap();
    let t0 = std::time::Instant::now();
    let o = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
        .args(["-ini", "w.tex"])
        .current_dir(&d)
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("shell_escape", "p")
        .env("shell_escape_commands", "bibtex")
        .env("PATH", "/nonexistent")
        .env_remove("FLASHTEX_BIBTEX")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert!(t0.elapsed() < std::time::Duration::from_secs(60));
    let stderr = String::from_utf8_lossy(&o.stderr);
    let blg = std::fs::read_to_string(d.join("x.blg")).unwrap();
    assert!(
        blg.contains("I couldn't open database file refs.bib"),
        "{blg}"
    );
    assert!(
        stderr.contains("bibtex: refusing to read y.aux: not a regular file"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&d);
}
