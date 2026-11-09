//! `\openin` and `\input` from a pipe (`|command`, texmfmp.c's
//! `open_in_or_pipe`) in restricted shell mode: as TeX Live's XeTeX and the
//! pdfTeX engine run it (`runpopen`), only a command named in
//! `shell_escape_commands` may run; any other is refused. With full shell
//! escape the same command runs, which shows the test can see it run (the
//! negative control).
//!
//! Restricted mode here comes from the environment (`shell_escape = p`,
//! as texmf.cnf sets it), since the working-directory resolver has no
//! texmf.cnf. The command is `echo`, which only prints a marker.

use std::path::PathBuf;
use std::process::Command;

const MARK: &str = "PIPE-RAN-MARKER";

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flashtex-xetex-pipe-restricted-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("t.tex"),
        format!(
            "\\catcode`\\{{=1 \\catcode`\\}}=2\n\
             \\openin1=\"|echo {MARK}\"\n\
             \\ifeof1 \\message{{[REFUSED]}}\\else\\read1 to\\x \\message{{[GOT \\x]}}\\fi\n\
             \\end\n"
        ),
    )
    .unwrap();
    d
}

/// The run's log and standard error.
fn run(tag: &str, shell: &str) -> (String, String) {
    let d = scratch(tag);
    let o = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&d)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("shell_escape", "p")
        .env("shell_escape_commands", "kpsewhich,bibtex")
        .env_remove("FLASHTEX_CONFINE_READS")
        .args([
            "-ini",
            "-no-pdf",
            "-interaction=nonstopmode",
            shell,
            "t.tex",
        ])
        .output()
        .expect("run flashtex-xetex");
    let log = std::fs::read_to_string(d.join("t.log")).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    (log, String::from_utf8_lossy(&o.stderr).into_owned())
}

#[test]
fn restricted_shell_refuses_a_pipe_command_not_allowed() {
    let (log, err) = run("restricted", "-shell-restricted");
    assert!(log.contains("[REFUSED]"), "{log}");
    assert!(!log.contains(MARK), "{log}");
    assert!(err.contains("runpopen command not allowed: echo"), "{err}");
}

#[test]
fn full_shell_escape_runs_the_same_pipe() {
    let (log, _) = run("full", "-shell-escape");
    assert!(log.contains(&format!("[GOT {MARK} ]")), "{log}");
}
