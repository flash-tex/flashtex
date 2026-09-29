//! Incremental compiles (DESIGN.md §5.2-§5.4, `flashtex-host iserve`)
//! against from-scratch runs, on the installed TeX Live's LaTeX. Skips where
//! there is no TeX Live (e.g. CI).
//!
//! Every compile of a session is compared, byte for byte (PDF, log, aux),
//! with `flashtex-initex` run once on a copy of the directory as the compile
//! found it, in preview mode like the host. The documents are the cases the
//! restart and convergence tests got wrong once:
//!
//! * `edits_equal_scratch_runs`: a file `\input` twice (a restart point in
//!   its second read must not skip an edit the first read saw), labels that
//!   move when a sentence is inserted (the next compile reads a changed
//!   `.aux`: it restarts at the `.aux` point), edits and reverts.
//! * `a_file_written_then_read_is_a_barrier`: a file written from a macro
//!   the edit changes and `\input` pages later; the run must not converge
//!   before that read (DESIGN.md §5.3's barriers).
#![cfg(feature = "kpathsea")]

use flashtex_engine::resolver::find_texlive_bin;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Env {
    dir: PathBuf,
    fmt: PathBuf,
    pool: PathBuf,
}

/// A pdflatex format made by this engine, once per test binary.
fn env() -> Option<Env> {
    static MADE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    find_texlive_bin()?;
    let _once = MADE.lock().unwrap_or_else(|p| p.into_inner());
    let initex = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let pool = Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool");
    let base = std::env::temp_dir().join(format!("flashtex-incr-{}", std::process::id()));
    let fmt = base.join("fmt");
    std::fs::create_dir_all(&fmt).unwrap();
    let pdftex = fmt.join("pdftex");
    let _ = std::os::unix::fs::symlink(initex, &pdftex);
    if !fmt.join("pdflatex.fmt").exists() {
        let st = Command::new(&pdftex)
            .args([
                "-ini",
                "-jobname=pdflatex",
                "-progname=pdflatex",
                "-translate-file=cp227.tcx",
                "*pdflatex.ini",
            ])
            .current_dir(&fmt)
            .env("FLASHTEX_POOL", &pool)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .unwrap();
        assert!(st.success() && fmt.join("pdflatex.fmt").exists());
    }
    Some(Env {
        dir: base,
        fmt,
        pool,
    })
}

const ARGS: &[&str] = &["-fmt=pdflatex", "-interaction=nonstopmode", "doc.tex"];

fn engine_env(c: &mut Command, e: &Env) {
    c.env("FLASHTEX_POOL", &e.pool)
        .env("FLASHTEX_FORMATS", &e.fmt)
        .env("SOURCE_DATE_EPOCH", "1700000000")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_PIN_CLOCK", "1700000000.123456")
        .env_remove("FLASHTEX_NO_AUX_POINT")
        .env_remove("FLASHTEX_NO_RELABEL");
}

struct Host {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Host {
    fn start(e: &Env, dir: &Path) -> Host {
        let mut c = Command::new(env!("CARGO_BIN_EXE_flashtex-host"));
        c.arg("iserve").arg("--").args(ARGS).current_dir(dir);
        engine_env(&mut c, e);
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        Host {
            child,
            stdin,
            stdout,
        }
    }

    fn cmd(&mut self, c: &str) -> String {
        writeln!(self.stdin, "{c}").unwrap();
        self.stdin.flush().unwrap();
        let mut line = String::new();
        self.stdout.read_line(&mut line).unwrap();
        assert!(!line.contains("\"error\""), "{c}: {line}");
        line
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.child.wait();
    }
}

fn copy_dir(from: &Path, to: &Path) {
    let _ = std::fs::remove_dir_all(to);
    std::fs::create_dir_all(to).unwrap();
    for f in std::fs::read_dir(from).unwrap() {
        let f = f.unwrap();
        if f.file_type().unwrap().is_file() {
            std::fs::copy(f.path(), to.join(f.file_name())).unwrap();
        }
    }
}

/// The directory's files but the PDF and the log, with their contents: what
/// the next from-scratch run may read.
fn dir_state(d: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = std::fs::read_dir(d)
        .unwrap()
        .filter_map(|f| f.ok())
        .filter(|f| f.file_type().is_ok_and(|t| t.is_file()))
        .map(|f| f.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.ends_with(".pdf") && !n.ends_with(".log"))
        .map(|n| {
            let b = std::fs::read(d.join(&n)).unwrap();
            (n, b)
        })
        .collect();
    v.sort();
    v
}

/// Compile in the host after writing `files`, and compare with scratch runs
/// on the directory as the compile found it: repeated, like the host's
/// passes (DESIGN.md §5.5), while a run changes a file other than the PDF
/// and the log, up to five runs, stopping when the files repeat a state an
/// earlier run started from. Returns the host's report.
fn compile_and_check(
    e: &Env,
    h: &mut Host,
    dir: &Path,
    files: &[(&str, &str)],
    what: &str,
) -> String {
    for (name, text) in files {
        std::fs::write(dir.join(name), text).unwrap();
    }
    let reference = dir.with_extension("ref");
    copy_dir(dir, &reference);
    let report = h.cmd("compile");
    let mut seen = vec![];
    for _ in 0..5 {
        seen.push(dir_state(&reference));
        let mut c = Command::new(e.fmt.join("pdftex"));
        c.args(ARGS).current_dir(&reference);
        engine_env(&mut c, e);
        c.env("FLASHTEX_PREVIEW", "1");
        c.stdin(Stdio::null()).stdout(Stdio::null());
        c.status().unwrap();
        if seen.contains(&dir_state(&reference)) {
            break;
        }
    }
    for ext in ["pdf", "log", "aux"] {
        // (a run that fails produces no PDF: then neither may have one)
        let (x, y) = (
            std::fs::read(dir.join(format!("doc.{ext}"))).ok(),
            std::fs::read(reference.join(format!("doc.{ext}"))).ok(),
        );
        assert!(
            x == y,
            "{what}: doc.{ext} differs from a scratch run ({:?} vs {:?} bytes)\n{report}",
            x.as_ref().map(|v| v.len()),
            y.as_ref().map(|v| v.len())
        );
    }
    report
}

#[test]
fn a_failed_run_then_a_revert() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("failed");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // an object written in the preamble opens the PDF before the .aux
    // point (beamer does that); a run that fails removes the PDF
    let body: String = (0..30).map(|i| para(i, "alpha")).collect();
    let doc = format!(
        "\\documentclass{{article}}\n\\usepackage{{hyperref}}\n\
         \\immediate\\pdfobj{{null}}\n\\begin{{document}}\n\
         \\section{{One}}\\label{{one}}\n{body}See page~\\pageref{{one}}.\n\\end{{document}}\n"
    );
    let broken = doc.replace("\\end{document}", "\\end{documen}");
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc)], "settle");
    }
    compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("doc.tex", &broken)],
        "a run that fails",
    );
    compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc)], "the revert");
    compile_and_check(&e, &mut h, &dir, &[], "settle again");
}

fn para(i: usize, word: &str) -> String {
    format!(
        "Paragraph {i} with the word {word}, and enough text to fill a few lines \
         of the page so that the document ships several pages; math $a^{i}+b$.\n\n"
    )
}

#[test]
fn edits_equal_scratch_runs() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("edits");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = r"\documentclass{article}
\usepackage{hyperref}
\begin{document}
\section{One}\label{one}
See page~\pageref{two}.
\input{body}
\section{Two}\label{two}
Back to page~\pageref{one}.
\input{body}
\end{document}
";
    let body: String = (0..40).map(|i| para(i, "alpha")).collect();
    let mut h = Host::start(&e, &dir);
    // settle: the .aux appears, then the labels are stable
    for k in 0..4 {
        let r = compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", doc), ("body.tex", &body)],
            "settle",
        );
        if r.contains("\"mode\":\"unchanged\"") || k == 3 {
            break;
        }
    }
    // an edit late in body.tex: the second read has page checkpoints
    // before it, which the first read's pages already passed
    let e1 = body.replacen(
        "Paragraph 33 with the word alpha",
        "Paragraph 33 with the word alphb",
        1,
    );
    assert_ne!(e1, body);
    let r = compile_and_check(
        &e,
        &mut h,
        &dir,
        &[("body.tex", &e1)],
        "edit in a file read twice",
    );
    assert!(r.contains("\"mode\":\"incremental\""), "{r}");
    // a sentence that moves the second section to another page
    let extra = "More words to move the label. ".repeat(40);
    let e2 = e1.replacen("Paragraph 3 ", &format!("{extra}Paragraph 3 "), 1);
    compile_and_check(&e, &mut h, &dir, &[("body.tex", &e2)], "reflow");
    // the next compile reads the .aux the reflow rewrote
    let r = compile_and_check(&e, &mut h, &dir, &[], "after the .aux changed");
    assert!(
        !r.contains("\"mode\":\"cold\""),
        "a changed .aux re-ran from the format: {r}"
    );
    compile_and_check(&e, &mut h, &dir, &[("body.tex", &body)], "revert");
    compile_and_check(&e, &mut h, &dir, &[], "settle again");
}

#[test]
fn a_file_written_then_read_is_a_barrier() {
    let Some(e) = env() else {
        eprintln!("no TeX Live found; skipping");
        return;
    };
    let dir = e.dir.join("barrier");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let doc = |word: &str| -> String {
        let mut s = String::from("\\documentclass{article}\n\\newwrite\\tmp\n\\begin{document}\n");
        for i in 0..12 {
            s.push_str(&para(i, "lorem"));
        }
        s.push_str(&format!(
            "\\def\\x{{{word}}}\\immediate\\openout\\tmp=\\jobname.tmp \
             \\immediate\\write\\tmp{{Written \\x.}}\\immediate\\closeout\\tmp \\def\\x{{}}\n\n"
        ));
        for i in 12..60 {
            s.push_str(&para(i, "ipsum"));
        }
        s.push_str("\\input{\\jobname.tmp}\n\n");
        for i in 60..70 {
            s.push_str(&para(i, "dolor"));
        }
        s.push_str("\\end{document}\n");
        s
    };
    let mut h = Host::start(&e, &dir);
    for _ in 0..3 {
        compile_and_check(
            &e,
            &mut h,
            &dir,
            &[("doc.tex", &doc("alphaword"))],
            "settle",
        );
    }
    for w in ["alphawore", "alphaword", "betaword"] {
        let r = compile_and_check(&e, &mut h, &dir, &[("doc.tex", &doc(w))], w);
        assert!(r.contains("\"mode\":\"incremental\""), "{r}");
    }
}
