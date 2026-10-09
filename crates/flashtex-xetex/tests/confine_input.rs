//! Read confinement (`FLASHTEX_CONFINE_READS=1`, a Live Share host:
//! docs/design/live-collab/PROPOSAL.md §6.2) of the `-output-directory`
//! shortcut `\input` and `\openin` take (texmfmp.c looks there first): a
//! name with `..` or a link in the output directory must not reach a file
//! outside the project, as the pdfTeX engine's `find_input` refuses it.
//! Without confinement the same document reads the file, which shows the
//! test can see it.

use std::path::{Path, PathBuf};
use std::process::Command;

const SECRET: &str = "SECRET-OUTSIDE-INPUT";

fn setup(tag: &str) -> (PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!(
        "flashtex-xetex-confine-input-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let job = base.join("job");
    let out = job.join("out");
    let outside = base.join("outside");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("sec.tex"), format!("\\message{{{SECRET}}}\n")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.join("sec.tex"), out.join("link.tex")).unwrap();
    std::fs::write(
        job.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2\n\\input ../../outside/sec.tex\n\\input link.tex\n\\end\n",
    )
    .unwrap();
    (base, job)
}

/// The log of the run, and how many times the secret was read.
fn run(job: &Path, confine: bool) -> (String, usize) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"));
    cmd.current_dir(job)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args([
            "-ini",
            "-no-pdf",
            "-output-directory=out",
            "-interaction=nonstopmode",
            "t.tex",
        ]);
    if confine {
        cmd.env("FLASHTEX_CONFINE_READS", "1");
    } else {
        cmd.env_remove("FLASHTEX_CONFINE_READS");
    }
    cmd.output().expect("run flashtex-xetex");
    let log = std::fs::read_to_string(job.join("out/t.log")).unwrap();
    let n = log.matches(SECRET).count();
    (log, n)
}

#[test]
fn confined_reads_refuse_the_output_directory_shortcut_out_of_the_project() {
    let (base, job) = setup("on");
    let (log, n) = run(&job, true);
    assert_eq!(n, 0, "{log}");
    assert!(log.contains("I can't find file"), "{log}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn without_confinement_the_shortcut_reads_the_file() {
    let (base, job) = setup("off");
    let (log, n) = run(&job, false);
    // `../../outside/sec.tex` and the link
    assert_eq!(n, if cfg!(unix) { 2 } else { 1 }, "{log}");
    let _ = std::fs::remove_dir_all(&base);
}
