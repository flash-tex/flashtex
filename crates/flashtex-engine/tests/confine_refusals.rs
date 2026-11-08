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

/// `FLASHTEX_CONFINE_SEARCH=roots` (an untrusted project's host, whose roots
/// name the TeX trees; #2026 review, finding 2): a file found by a bare
/// name in a directory of the user's `TEXINPUTS` outside the roots is
/// refused. Without it such a search-path hit stays readable, as for Live
/// Share, whose roots are the project alone.
#[cfg(unix)]
#[test]
fn search_hits_outside_the_roots_are_refused_when_the_roots_name_the_trees() {
    let (base, job, outside) = setup("search");
    std::fs::write(
        job.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2\n\
         \\openin1=sec.tex\n\\ifeof1 \\message{[missing]}\\else\\read1 to\\x \\message{\\x}\\fi\n\
         \\end\n",
    )
    .unwrap();
    let refused = base.join("refused");
    // (with its links resolved: a hit through a link, as macOS's /var is,
    // is refused either way)
    let texinputs = format!(".:{}:", std::fs::canonicalize(&outside).unwrap().display());
    let run = |strict: bool| {
        let _ = std::fs::remove_file(&refused);
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_flashtex-initex"));
        cmd.current_dir(&job)
            .env("SOURCE_DATE_EPOCH", "0")
            .env("TEXINPUTS", &texinputs)
            .env("FLASHTEX_CONFINE_READS", "1")
            .env("FLASHTEX_CONFINE_ROOTS", &job)
            .env("FLASHTEX_CONFINE_REFUSED", &refused)
            .args([
                "-ini",
                "-output-directory=out",
                "-interaction=nonstopmode",
                "t.tex",
            ]);
        if strict {
            cmd.env("FLASHTEX_CONFINE_SEARCH", "roots");
        } else {
            cmd.env_remove("FLASHTEX_CONFINE_SEARCH");
        }
        cmd.output().expect("run flashtex-initex");
        let log = std::fs::read_to_string(job.join("out/t.log")).unwrap_or_default();
        let lines: Vec<String> = std::fs::read_to_string(&refused)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();
        (log, lines)
    };
    let (log, lines) = run(false);
    assert_eq!(
        log.matches(SECRET).count(),
        1,
        "a TEXINPUTS hit is read: {log}"
    );
    assert!(lines.is_empty(), "{lines:?}");
    let (log, lines) = run(true);
    assert_eq!(log.matches(SECRET).count(), 0, "{log}");
    assert!(log.contains("[missing]"), "{log}");
    let real = std::fs::canonicalize(outside.join("sec.tex")).unwrap();
    assert_eq!(lines, vec![format!("sec.tex\t{}", real.display())]);
    let _ = std::fs::remove_dir_all(&base);
}

/// A root whose name has a `:` (#2026 review: a folder `~/Downloads/:paper`)
/// is one root when the app escapes it (`%3A`): the project's own file is
/// read, a link to a file beside the folder is refused. The negative
/// control passes the same root unescaped, as the app did before: the list
/// splits at the `:` and the folder above becomes a root.
#[cfg(unix)]
#[test]
fn a_root_with_a_colon_in_its_name_is_one_root() {
    let base = std::env::temp_dir().join(format!("flashtex-confine-colon-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (dl, job) = (base.join("dl"), base.join("job"));
    let project = dl.join(":paper");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(job.join("out")).unwrap();
    std::fs::write(project.join("in.tex"), "\\message{[inside]}\n").unwrap();
    std::fs::write(dl.join("secret.tex"), format!("\\message{{{SECRET}}}\n")).unwrap();
    std::os::unix::fs::symlink(project.join("in.tex"), job.join("in.tex")).unwrap();
    std::os::unix::fs::symlink(dl.join("secret.tex"), job.join("s.tex")).unwrap();
    std::fs::write(
        job.join("t.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2\n\
         \\openin1=in.tex \\ifeof1 \\else\\read1 to\\x \\x\\fi\n\
         \\openin1=s.tex \\ifeof1 \\else\\read1 to\\x \\x\\fi\n\\end\n",
    )
    .unwrap();
    let run = |roots: &str| {
        Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
            .current_dir(&job)
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FLASHTEX_CONFINE_READS", "1")
            .env("FLASHTEX_CONFINE_ROOTS", roots)
            .env_remove("FLASHTEX_CONFINE_REFUSED")
            .args([
                "-ini",
                "-output-directory=out",
                "-interaction=nonstopmode",
                "t.tex",
            ])
            .output()
            .expect("run flashtex-initex");
        std::fs::read_to_string(job.join("out/t.log")).unwrap_or_default()
    };
    let raw = project.display().to_string();
    let escaped = raw.replace('%', "%25").replace(':', "%3A");
    let log = run(&escaped);
    assert!(
        log.contains("[inside]"),
        "the project's file is read: {log}"
    );
    assert_eq!(log.matches(SECRET).count(), 0, "{log}");
    let log = run(&raw);
    assert!(
        log.contains(SECRET),
        "unescaped, the folder above is a root: {log}"
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// A document that asks for many names confinement refuses (#2026 review):
/// the refusals file holds at most `MAX_REFUSED` lines; the negative
/// control, a few names, lists each.
#[test]
fn the_refusals_list_is_bounded() {
    let base = std::env::temp_dir().join(format!("flashtex-confine-many-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("out")).unwrap();
    let refused = base.join("refused");
    let run = |n: usize| {
        let _ = std::fs::remove_file(&refused);
        std::fs::write(
            base.join("t.tex"),
            format!(
                "\\catcode`\\{{=1 \\catcode`\\}}=2 \\count1=0\n\
                 \\def\\loop{{\\advance\\count1 by1 \\openin1=/nonexistent/f\\the\\count1.tex \
                 \\closein1 \\ifnum\\count1<{n} \\expandafter\\loop\\fi}}\\loop\n\\end\n"
            ),
        )
        .unwrap();
        Command::new(env!("CARGO_BIN_EXE_flashtex-initex"))
            .current_dir(&base)
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FLASHTEX_CONFINE_READS", "1")
            .env("FLASHTEX_CONFINE_ROOTS", &base)
            .env("FLASHTEX_CONFINE_REFUSED", &refused)
            .args([
                "-ini",
                "-output-directory=out",
                "-interaction=nonstopmode",
                "t.tex",
            ])
            .output()
            .expect("run flashtex-initex");
        std::fs::read_to_string(&refused)
            .unwrap_or_default()
            .lines()
            .count()
    };
    assert_eq!(run(12), 12, "a few refusals are each listed");
    assert_eq!(run(1000), flashtex_engine::system::MAX_REFUSED);
    let _ = std::fs::remove_dir_all(&base);
}
