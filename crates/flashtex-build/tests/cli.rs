//! `flashtex-v3` end to end, through a built `flashtex-host` and the user's
//! TeX Live (skipped when either is missing). The PDF `build` writes is the
//! host's `export` run, which the parity gates hold byte-identical to
//! pdflatex's (P-T2); here it is compared with MacTeX's pdflatex as an
//! oracle (never in the product path) after the usual normalisation of the
//! creation dates and the document ID.

use std::path::{Path, PathBuf};
use std::process::Command;

fn host() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FLASHTEX_HOST") {
        return Some(PathBuf::from(p)).filter(|p| p.is_file());
    }
    // The workspace's release host (target/release/), beside the profile under test.
    let target = bin().parent()?.parent()?.to_path_buf();
    // Release only: a debug engine is far too slow for a test run.
    Some(
        target
            .join("release")
            .join(format!("flashtex-host{}", std::env::consts::EXE_SUFFIX)),
    )
    .filter(|p| p.is_file())
}

/// A skipped test says so where it is seen: the test harness captures
/// `eprintln!` of a passing test, but not a direct write to stderr.
///
/// A run that must have both sets `FLASHTEX_REQUIRE_TEXLIVE=1`, as the
/// engine's TeX Live tests read it (crates/flashtex-engine/tests/common):
/// then a missing host or TeX Live fails the test instead, so the leg that
/// sets it (ci.yml's `rust workspace` on a self-hosted Mac, app-parity row
/// D5) proves these tests ran.
fn skip(why: &str) {
    use std::io::Write;
    if std::env::var_os("FLASHTEX_REQUIRE_TEXLIVE").is_some_and(|v| v == "1") {
        panic!(
            "{why}, and FLASHTEX_REQUIRE_TEXLIVE=1 says this run must have both \
             (cargo build --release -p flashtex-engine --bin flashtex-host, or \
             $FLASHTEX_HOST; pdflatex on PATH or in /Library/TeX/texbin)"
        );
    }
    let _ = writeln!(std::io::stderr(), "SKIPPED {}: {why} (build it: cargo build --release -p flashtex-engine --bin flashtex-host)", std::thread::current().name().unwrap_or("?"));
}

fn pdflatex() -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    dirs.push("/Library/TeX/texbin".into());
    dirs.into_iter()
        .map(|d| d.join("pdflatex"))
        .find(|p| p.is_file())
}

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_flashtex-v3"))
}

/// `flashtex-v3`, with the engine's format cache in the temporary
/// directory (shared by these tests, validated per engine build), never
/// the user's `~/Library/Caches/FlashTeX/formats`: the Mac app's test
/// guard watched that directory and these tests, run by the self-hosted
/// Actions runner, wrote it in the middle of its runs.
fn cli() -> Command {
    let mut c = Command::new(bin());
    c.env(
        "FLASHTEX_FORMAT_CACHE_DIR",
        std::env::temp_dir().join("flashtex-v3-e2e-formats"),
    );
    c
}

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("flashtex-v3-e2e-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (n, t) in files {
        std::fs::write(dir.join(n), t).unwrap();
    }
    dir
}

/// The PDF with its dates and ID blanked (they differ between any two runs).
fn normalised(p: &Path) -> Vec<u8> {
    let mut b = std::fs::read(p).unwrap();
    for key in [&b"/CreationDate ("[..], b"/ModDate (", b"/ID ["] {
        let mut i = 0;
        while let Some(at) = b[i..].windows(key.len()).position(|w| w == key) {
            let start = i + at + key.len();
            let close = if key.ends_with(b"[") { b']' } else { b')' };
            let end = start + b[start..].iter().position(|&c| c == close).unwrap_or(0);
            for c in &mut b[start..end] {
                *c = b'0';
            }
            i = end;
        }
    }
    b
}

const DOC: &str = "\\documentclass{article}\n\\begin{document}\n\\section{One}\\label{one}\nSee section~\\ref{one} on page~\\pageref{one}.\n\\end{document}\n";

#[test]
fn build_writes_the_pdf_pdflatex_writes() {
    let (Some(host), Some(pdflatex)) = (host(), pdflatex()) else {
        skip("no flashtex-host or no TeX Live");
        return;
    };
    let dir = project("build", &[("paper.tex", DOC)]);
    let out = cli()
        .args([
            "build",
            &dir.to_string_lossy(),
            "--host",
            &host.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pdf = dir.join("paper.pdf");
    assert!(pdf.is_file(), "the PDF is next to the main file");
    // Only the PDF lands in the project: the .aux and .log stay out of it.
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["paper.pdf", "paper.tex"]);

    // The oracle: pdflatex twice (the cross-reference needs the .aux).
    let oracle = dir.join("oracle");
    std::fs::create_dir_all(&oracle).unwrap();
    for _ in 0..2 {
        let st = Command::new(&pdflatex)
            .args([
                "-interaction=nonstopmode",
                "-output-directory",
                &oracle.to_string_lossy(),
                "paper.tex",
            ])
            .current_dir(&dir)
            .output()
            .unwrap();
        assert!(
            st.status.success(),
            "{}",
            String::from_utf8_lossy(&st.stdout)
        );
    }
    assert_eq!(
        normalised(&pdf),
        normalised(&oracle.join("paper.pdf")),
        "flashtex-v3 build's PDF is pdflatex's"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A bibliography: `build` runs bibtex as latexmk would, then exports. The
/// oracle is pdflatex, bibtex, pdflatex, pdflatex.
#[test]
fn build_runs_bibtex_as_latexmk_would() {
    let (Some(host), Some(pdflatex)) = (host(), pdflatex()) else {
        skip("no flashtex-host or no TeX Live");
        return;
    };
    let bib = "@book{knuth,\n  author = {Donald E. Knuth},\n  title = {The {\\TeX}book},\n  publisher = {Addison-Wesley},\n  year = {1984}\n}\n";
    let doc = "\\documentclass{article}\n\\begin{document}\nAs in~\\cite{knuth}.\n\\bibliographystyle{plain}\n\\bibliography{refs}\n\\end{document}\n";
    let dir = project("bib", &[("main.tex", doc), ("refs.bib", bib)]);
    let out = cli()
        .args([
            "build",
            &dir.join("main.tex").to_string_lossy(),
            "--host",
            &host.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let oracle = dir.join("oracle");
    std::fs::create_dir_all(&oracle).unwrap();
    let tex = |args: &[&str]| {
        let st = Command::new(&pdflatex)
            .args([
                "-interaction=nonstopmode",
                "-output-directory",
                &oracle.to_string_lossy(),
            ])
            .args(args)
            .current_dir(&dir)
            .output()
            .unwrap();
        assert!(
            st.status.success(),
            "{}",
            String::from_utf8_lossy(&st.stdout)
        );
    };
    tex(&["main.tex"]);
    let bibtex = pdflatex.with_file_name("bibtex");
    let st = Command::new(&bibtex)
        .arg("main")
        .current_dir(&oracle)
        .env("BIBINPUTS", &dir)
        .output()
        .unwrap();
    assert!(
        st.status.success(),
        "{}",
        String::from_utf8_lossy(&st.stdout)
    );
    tex(&["main.tex"]);
    tex(&["main.tex"]);
    assert_eq!(
        normalised(&dir.join("main.pdf")),
        normalised(&oracle.join("main.pdf")),
        "the citation resolved as with pdflatex and bibtex"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// A tool that fails (bibtex, on a broken database) is reported by `check`
/// and makes it fail, even when TeX itself had no error; a tool the host
/// does not run (its database is missing) is reported as a warning.
#[test]
fn check_reports_a_failed_tool() {
    let Some(host) = host() else {
        skip("no flashtex-host");
        return;
    };
    if pdflatex().is_none() {
        skip("no TeX Live");
        return;
    }
    let doc = "\\documentclass{article}\n\\begin{document}\nAs in~\\cite{knuth}.\n\\bibliographystyle{plain}\n\\bibliography{refs}\n\\end{document}\n";
    let dir = project(
        "tool",
        &[
            ("main.tex", doc),
            (
                "refs.bib",
                "@book{knuth,\n  title = {The book,\n  year = 1984\n",
            ),
        ],
    );
    let out = cli()
        .args([
            "check",
            &dir.to_string_lossy(),
            "--host",
            &host.to_string_lossy(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(1),
        "a failed bibtex fails check: {stdout}"
    );
    assert!(
        stdout
            .lines()
            .any(|l| l.starts_with("error: bibtex: errors")),
        "{stdout}"
    );
    // The database missing: bibtex is not run, which is a warning.
    std::fs::remove_file(dir.join("refs.bib")).unwrap();
    let out = cli()
        .args([
            "check",
            &dir.to_string_lossy(),
            "--host",
            &host.to_string_lossy(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout
            .lines()
            .any(|l| l.starts_with("warning: bibtex not run: ")),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn check_reports_errors_with_their_place_and_fails() {
    let Some(host) = host() else {
        skip("no flashtex-host");
        return;
    };
    if pdflatex().is_none() {
        skip("no TeX Live");
        return;
    }
    let dir = project("check", &[("main.tex", "\\documentclass{article}\n\\begin{document}\nHello \\undefinedthing{} world.\n\\end{document}\n")]);
    let out = cli()
        .args([
            "check",
            &dir.to_string_lossy(),
            "--host",
            &host.to_string_lossy(),
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "an error fails check: {stdout}");
    assert!(
        stdout.contains("main.tex:3:7: error: Undefined control sequence."),
        "{stdout}"
    );
    let json = cli()
        .args([
            "check",
            &dir.to_string_lossy(),
            "--json",
            "--host",
            &host.to_string_lossy(),
        ])
        .output()
        .unwrap();
    let line = String::from_utf8_lossy(&json.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    assert!(
        line.starts_with('{') && line.contains("\"code\":\"tex/undefined-control-sequence\""),
        "{line}"
    );
    let clean = project(
        "clean",
        &[(
            "main.tex",
            "\\documentclass{article}\n\\begin{document}\nFine.\n\\end{document}\n",
        )],
    );
    let ok = cli()
        .args([
            "check",
            &clean.to_string_lossy(),
            "--host",
            &host.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&clean);
}

/// A fresh directory for a run's temporary files (its `TMPDIR`), so what a
/// run leaves there can be listed.
#[cfg(unix)]
fn fresh_tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ftv3t-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[cfg(unix)]
fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(unix)]
fn entries(d: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(d)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// SIGTERM (and Ctrl-C, SIGINT) while `watch` waits for its host: the host
/// process is killed and the run's work directory removed (Drop does not
/// run on a signal). Needs no engine: the "host" is a script that writes
/// its pid and sleeps.
#[cfg(unix)]
#[test]
fn a_signal_cleans_up_the_work_dir_and_the_host() {
    use std::os::unix::fs::PermissionsExt;
    for sig in ["TERM", "INT"] {
        let tmp = fresh_tmp(&format!("sig{sig}"));
        let host = tmp.join("fake-host.sh");
        let pidfile = tmp.join("host.pid");
        std::fs::write(
            &host,
            format!(
                "#!/bin/sh\necho $$ > '{}'\nexec sleep 600\n",
                pidfile.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&host, std::fs::Permissions::from_mode(0o755)).unwrap();
        let dir = project(
            &format!("sig{sig}"),
            &[("main.tex", "\\documentclass{article}\n")],
        );
        let mut cli = cli()
            .args([
                "watch",
                &dir.to_string_lossy(),
                "--host",
                &host.to_string_lossy(),
            ])
            .env("TMPDIR", &tmp)
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let t0 = std::time::Instant::now();
        while std::fs::read_to_string(&pidfile)
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            assert!(t0.elapsed().as_secs() < 30, "the fake host did not start");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let host_pid: u32 = std::fs::read_to_string(&pidfile)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        let work = format!("flashtex-v3-{}-", cli.id());
        assert!(
            entries(&tmp).iter().any(|e| e.starts_with(&work)),
            "{:?}",
            entries(&tmp)
        );
        Command::new("kill")
            .args([format!("-{sig}"), cli.id().to_string()])
            .status()
            .unwrap();
        let st = cli.wait().unwrap();
        assert!(!st.success(), "ended by the signal: {st}");
        assert!(
            !entries(&tmp).iter().any(|e| e.starts_with(&work)),
            "SIG{sig}: the work dir is gone: {:?}",
            entries(&tmp)
        );
        let t1 = std::time::Instant::now();
        while alive(host_pid) {
            assert!(
                t1.elapsed().as_secs() < 10,
                "SIG{sig}: the host {host_pid} is still running"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// A run killed outright (SIGKILL) leaves its work directory and socket;
/// the next run removes those whose pid is not running, and nothing of a
/// running one.
#[cfg(unix)]
#[test]
fn a_run_sweeps_what_a_killed_run_left() {
    let tmp = fresh_tmp("sweep");
    let mut dead = Command::new("true").spawn().unwrap();
    let dead_pid = dead.id();
    dead.wait().unwrap();
    let me = std::process::id();
    for name in [
        format!("flashtex-v3-{dead_pid}-1-0"),
        format!("flashtex-v3-{me}-1-0"),
    ] {
        std::fs::create_dir_all(tmp.join(&name).join("sub")).unwrap();
    }
    std::fs::write(tmp.join(format!("ftx-v3-{dead_pid}-1.sock")), "").unwrap();
    std::fs::write(tmp.join("unrelated.txt"), "").unwrap();
    let out = cli()
        .args(["check", &tmp.join("no-such-project").to_string_lossy()])
        .env("TMPDIR", &tmp)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(
        entries(&tmp),
        vec![format!("flashtex-v3-{me}-1-0"), "unrelated.txt".to_string()],
        "the dead run's leftovers went, the live one's stayed"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Progress (lane CLI-PROGRESS) is for a terminal: without one (a pipe, as
/// here, as in CI and scripts) `build` prints exactly what it printed before
/// it existed, one line on stderr and nothing on stdout, and no `\r` or
/// escape sequence; `--quiet` prints nothing; `--progress` asks for plain
/// lines (a pass, the export) and keeps the last line.
#[test]
fn without_a_terminal_progress_prints_nothing_unless_asked() {
    let Some(host) = host() else {
        skip("no flashtex-host");
        return;
    };
    if pdflatex().is_none() {
        skip("no TeX Live");
        return;
    }
    let dir = project("progress", &[("paper.tex", DOC)]);
    let run = |extra: &[&str]| {
        let mut c = cli();
        c.args([
            "build",
            &dir.to_string_lossy(),
            "--host",
            &host.to_string_lossy(),
        ])
        .args(extra);
        let out = c.output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.stdout.is_empty(), "stdout stays empty");
        String::from_utf8(out.stderr).unwrap()
    };
    // As before progress: `flashtex-v3: wrote <pdf> (<n> ms)` and a newline.
    let plain = run(&[]);
    let pdf = dir.join("paper.pdf").canonicalize().unwrap();
    let ms = plain
        .strip_prefix(&format!("flashtex-v3: wrote {} (", pdf.display()))
        .and_then(|r| r.strip_suffix(" ms)\n"));
    assert!(
        ms.is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())),
        "{plain:?}"
    );
    assert_eq!(run(&["--no-progress"]).lines().count(), 1);
    assert_eq!(run(&["--quiet"]), "");
    let asked = run(&["--progress"]);
    assert!(
        !asked.contains('\r') && !asked.contains('\x1b'),
        "{asked:?}"
    );
    let lines: Vec<&str> = asked.lines().collect();
    assert!(lines.contains(&"flashtex-v3: pass 1"), "{asked}");
    assert!(lines.contains(&"flashtex-v3: writing the PDF"), "{asked}");
    assert!(
        lines
            .last()
            .is_some_and(|l| l.starts_with("flashtex-v3: wrote ")),
        "{asked}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
