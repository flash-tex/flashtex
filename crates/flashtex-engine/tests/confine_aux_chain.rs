//! Read confinement (`FLASHTEX_CONFINE_READS=1`: a Live Share host, or an
//! untrusted project) of the `.aux` chain the host reads after every
//! compile to decide whether bibtex is due (`host::external::snapshot`):
//! an `\@input` name in the `.aux` that is absolute, has a `..` component,
//! or is a link out of the output directory must not be read or hashed.
//! The project's own files (`\@input{ch/one.aux}`) still are. Without
//! confinement the same `.aux` reads all of them, which shows the test can
//! see a read (the negative control).
//!
//! Confinement is read from the environment once per process, so each case
//! runs this test binary again as a child with its environment set.
//!
//! Everything lives in one fresh temporary directory; the "outside" files
//! are scratch `.aux` files in it, never real system files.

#![cfg(not(feature = "tex82"))]

use flashtex_engine::host::external::snapshot;
use flashtex_engine::resolver::Format;
use flashtex_engine::system::{Lookup, ReadLog};
use std::path::{Path, PathBuf};
use std::process::Command;

const CHILD: &str = "FLASHTEX_TEST_AUX_CHAIN_CHILD";

/// The project: `base/proj` (the job's directory) with output directory
/// `base/proj/out`, and `base/outside`, a directory beside it.
fn layout(base: &Path) {
    let out = base.join("proj/out");
    let outside = base.join("outside");
    std::fs::create_dir_all(out.join("ch")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    for n in ["abs.aux", "dots.aux", "linked.aux"] {
        std::fs::write(outside.join(n), "\\bibdata{outside}\n").unwrap();
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.join("linked.aux"), out.join("link.aux")).unwrap();
    std::fs::write(out.join("ch/one.aux"), "\\citation{b}\n").unwrap();
    std::fs::write(
        out.join("main.aux"),
        format!(
            "\\relax\n\\citation{{a}}\n\\@input{{ch/one.aux}}\n\\@input{{{}}}\n\
             \\@input{{../../outside/dots.aux}}\n\\@input{{link.aux}}\n\
             \\bibstyle{{plain}}\n\\bibdata{{refs}}\n",
            outside.join("abs.aux").display()
        ),
    )
    .unwrap();
}

/// The child: the snapshot of a compile that wrote `main.aux` and looked
/// for `main.bbl`; prints the files it read, one per line.
#[test]
fn child_snapshot() {
    let Some(base) = std::env::var_os(CHILD) else {
        return; // runs only as the child of the tests below
    };
    let base = PathBuf::from(base);
    let proj = base.join("proj");
    let out = proj.join("out");
    std::env::set_current_dir(&proj).unwrap();
    let mut j = ReadLog::default();
    j.outputs
        .push(out.join("main.aux").to_string_lossy().into_owned());
    j.lookups_mut().push(Lookup {
        name: "main.bbl".into(),
        format: Format::Tex,
        must_exist: Some(false),
        found: None,
    });
    let snap = snapshot(&proj, &out, "main", Some(&j));
    for f in snap.files_read() {
        println!("READ\t{f}");
    }
}

/// The files the child read, with confinement on or off.
fn files_read(tag: &str, confined: bool) -> Vec<String> {
    let base = std::env::temp_dir().join(format!(
        "flashtex-confine-aux-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    // (links resolved: macOS's temporary directory is under a link)
    let base = std::fs::canonicalize(&base).unwrap();
    layout(&base);
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args(["--exact", "child_snapshot", "--nocapture", "--test-threads=1"])
        .env(CHILD, &base)
        .env_remove("FLASHTEX_CONFINE_ROOTS");
    if confined {
        cmd.env("FLASHTEX_CONFINE_READS", "1");
    } else {
        cmd.env_remove("FLASHTEX_CONFINE_READS");
    }
    let o = cmd.output().expect("run the child");
    let stdout = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(
        o.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let _ = std::fs::remove_dir_all(&base);
    let b = base.to_string_lossy().into_owned();
    stdout
        .lines()
        .filter_map(|l| l.strip_prefix("READ\t"))
        .map(|l| l.replace(&b, "BASE"))
        .collect()
}

#[test]
fn a_confined_host_reads_no_aux_input_outside_the_project() {
    let read = files_read("on", true);
    // the project's own .aux files are still read: bibtex is still due
    assert_eq!(read, vec!["main.aux", "ch/one.aux"]);
}

#[test]
fn unconfined_the_aux_chain_reads_every_input() {
    let read = files_read("off", false);
    let mut want = vec![
        "main.aux".to_string(),
        "ch/one.aux".into(),
        "BASE/outside/abs.aux".into(),
        "../../outside/dots.aux".into(),
    ];
    if cfg!(unix) {
        want.push("link.aux".into());
    }
    assert_eq!(read, want);
}
