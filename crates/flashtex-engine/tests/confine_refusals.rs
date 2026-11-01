//! Read confinement's refusals are reported (`FLASHTEX_CONFINE_REFUSED`):
//! the app confines an untrusted project's compiles (a download, a
//! received project, a Live Share session) and shows each refused read in
//! the Problems list with a way to trust the project. Every refused name
//! is appended to the file once, as the document asked for it, with where
//! it led when it was refused for resolving outside the allowed roots.
//! Without confinement nothing is refused and nothing is written.

use std::path::{Path, PathBuf};
use std::process::Command;

const SECRET: &str = "SECRET-OUTSIDE-READ";

fn setup(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
    let base = std::env::temp_dir().join(format!(
        "flashtex-confine-refused-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let job = base.join("job");
    let outside = base.join("outside");
    std::fs::create_dir_all(job.join("out")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("sec.tex"), format!("\\message{{{SECRET}}}\n")).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.join("sec.tex"), job.join("link.tex")).unwrap();
    let abs = outside.join("sec.tex");
    std::fs::write(
        job.join("t.tex"),
        format!(
            "\\catcode`\\{{=1 \\catcode`\\}}=2\n\
             \\openin1={abs}\n\\ifeof1 \\else\\read1 to\\x \\message{{\\x}}\\fi\n\
             \\openin1=../outside/sec.tex\n\\ifeof1 \\else\\read1 to\\x \\message{{\\x}}\\fi\n\
             \\openin1={abs}\n\\ifeof1 \\else\\read1 to\\x \\message{{\\x}}\\fi\n\
             \\openin1=link.tex\n\\ifeof1 \\else\\read1 to\\x \\message{{\\x}}\\fi\n\
             \\openin1=inside.tex\n\\ifeof1 \\else\\message{{[inside]}}\\fi\n\
             \\end\n",
            abs = abs.display()
        ),
    )
    .unwrap();
    std::fs::write(job.join("inside.tex"), "inside\n").unwrap();
    (base, job, outside)
}

/// The log of the run and the refusals file's lines (None: not written).
fn run(job: &Path, refused: &Path, confine: bool) -> (String, Option<Vec<String>>) {
    let _ = std::fs::remove_file(refused);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    cmd.current_dir(job)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FLASHTEX_CONFINE_REFUSED", refused)
        .args([
            "-ini",
            "-output-directory=out",
            "-interaction=nonstopmode",
            "t.tex",
        ]);
    if confine {
        cmd.env("FLASHTEX_CONFINE_READS", "1")
            .env("FLASHTEX_CONFINE_ROOTS", job);
    } else {
        cmd.env_remove("FLASHTEX_CONFINE_READS");
    }
    cmd.output().expect("run flashtex-initex");
    let log = std::fs::read_to_string(job.join("out/t.log")).unwrap_or_default();
    let lines = std::fs::read_to_string(refused)
        .ok()
        .map(|s| s.lines().map(str::to_string).collect());
    (log, lines)
}

#[test]
fn confined_refusals_are_listed_once_each() {
    let (base, job, outside) = setup("on");
    let refused = base.join("refused");
    let (log, lines) = run(&job, &refused, true);
    assert_eq!(log.matches(SECRET).count(), 0, "{log}");
    assert!(
        log.contains("[inside]"),
        "a file in the project is still read: {log}"
    );
    let lines = lines.expect("the refusals file is written");
    let abs = outside.join("sec.tex").display().to_string();
    let real = std::fs::canonicalize(outside.join("sec.tex"))
        .unwrap()
        .display()
        .to_string();
    let mut want = vec![abs.clone(), "../outside/sec.tex".to_string()];
    if cfg!(unix) {
        want.push(format!("link.tex\t{real}"));
    }
    assert_eq!(lines, want, "{log}");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn without_confinement_nothing_is_refused_or_written() {
    let (base, job, _) = setup("off");
    let refused = base.join("refused");
    let (log, lines) = run(&job, &refused, false);
    assert!(log.matches(SECRET).count() >= 3, "{log}");
    assert_eq!(lines, None);
    let _ = std::fs::remove_dir_all(&base);
}
