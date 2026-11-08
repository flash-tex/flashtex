//! Read confinement covers the lookups made outside the run (#2057): the
//! display-list writer's font programs and encodings (`system::lookup_again`,
//! which the incremental system's checks of a run's lookups use too) and the
//! PK lookups (`system::find_pk`, writet3's, and `system::find_pk_quietly`,
//! the writer's). Each must refuse a file outside the allowed roots as the
//! run's own lookups do, and report it in `FLASHTEX_CONFINE_REFUSED`; without
//! confinement each finds the file exactly as before.
//!
//! Confinement is settled once per process (`FLASHTEX_CONFINE_READS`), so
//! each case runs in a child of this test binary (`CONFINE_LOOKUPS_CHILD`
//! names the case), confined and not. Skips where there is no TeX Live.
#![cfg(all(unix, feature = "kpathsea"))]

mod common;

use flashtex_engine::resolver::{find_texlive_bin, Format};
use flashtex_engine::system::{self, Lookup};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const CHILD: &str = "CONFINE_LOOKUPS_CHILD";

/// The scratch layout: `job/` (the project, the working directory, with
/// `out/` its output directory) and `outside/`, which holds the files the
/// project's links and names lead to.
struct Dirs {
    base: PathBuf,
    job: PathBuf,
    outside: PathBuf,
}

fn setup(tag: &str) -> Dirs {
    let base = common::fresh_dir(&format!("flashtex-confine-lookups-{tag}"));
    let (job, outside) = (base.join("job"), base.join("outside"));
    std::fs::create_dir_all(job.join("out")).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    for f in ["evil.pfb", "evil.enc", "evil.map", "evil.600pk"] {
        std::fs::write(outside.join(f), b"SECRET-OUTSIDE-READ\n").unwrap();
    }
    let link = |to: &str, at: &Path| std::os::unix::fs::symlink(outside.join(to), at).unwrap();
    link("evil.enc", &job.join("evil.enc"));
    link("evil.map", &job.join("out/evil.map"));
    link("evil.600pk", &job.join("evil.600pk"));
    std::fs::write(job.join("inside.enc"), b"/inside [ ] def\n").unwrap();
    Dirs { base, job, outside }
}

/// Run case `name` of this binary in a child, in `job`, confined or not;
/// the lines of its refusals file (None: not written).
fn in_child(name: &str, d: &Dirs, confine: bool) -> Option<Vec<String>> {
    let refused = d.base.join(format!("refused-{confine}"));
    let _ = std::fs::remove_file(&refused);
    let mut c = Command::new(std::env::current_exe().unwrap());
    c.args([name, "--exact", "--nocapture", "--test-threads=1"])
        .current_dir(&d.job)
        .env(CHILD, name)
        .env("CONFINE_LOOKUPS_OUTSIDE", &d.outside)
        .env("CONFINE_LOOKUPS_CONFINED", if confine { "1" } else { "0" })
        .env("TEXMFVAR", d.base.join("texmf-var"))
        .env("FLASHTEX_CONFINE_REFUSED", &refused)
        .stdin(Stdio::null());
    if confine {
        c.env("FLASHTEX_CONFINE_READS", "1")
            .env("FLASHTEX_CONFINE_ROOTS", &d.job);
    } else {
        c.env_remove("FLASHTEX_CONFINE_READS");
    }
    let out = c.output().unwrap();
    assert!(
        out.status.success(),
        "case {name} (confined: {confine}) failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::read_to_string(&refused)
        .ok()
        .map(|s| s.lines().map(str::to_string).collect())
}

/// In the parent: run `name` confined and not; the confined run's
/// refusals, which must be `want`. Unconfined, nothing is refused. In the
/// child: run `body` (confined or not) and return None.
fn case(name: &str, want: impl Fn(&Dirs) -> Vec<String>, body: impl FnOnce(bool, &Path)) {
    if std::env::var(CHILD).as_deref() == Ok(name) {
        let confined = std::env::var("CONFINE_LOOKUPS_CONFINED").as_deref() == Ok("1");
        let outside = PathBuf::from(std::env::var_os("CONFINE_LOOKUPS_OUTSIDE").unwrap());
        body(confined, &outside);
        return;
    }
    if std::env::var_os(CHILD).is_some() {
        return; // another case's child
    }
    if find_texlive_bin().is_none() {
        common::no_texlive();
        return;
    }
    let d = setup(name);
    assert_eq!(
        in_child(name, &d, false),
        None,
        "unconfined, nothing is refused"
    );
    let refused = in_child(name, &d, true);
    assert_eq!(refused, Some(want(&d)), "the refusals of the confined run");
    let _ = std::fs::remove_dir_all(&d.base);
}

fn real(p: &Path) -> String {
    std::fs::canonicalize(p).unwrap().display().to_string()
}

fn lookup(name: &str, format: Format) -> Option<String> {
    system::lookup_again(&Lookup {
        name: name.to_string(),
        format,
        must_exist: None,
        found: None,
    })
}

/// A map line's font file named by an absolute path outside the roots
/// (`\pdfmapline{+evil Evil <"/outside/evil.pfb"}`): the writer's lookup
/// of the font program finds nothing.
#[test]
fn lookup_again_refuses_a_font_file_named_outside() {
    case(
        "lookup_again_refuses_a_font_file_named_outside",
        |d| vec![d.outside.join("evil.pfb").display().to_string()],
        |confined, outside| {
            let name = outside.join("evil.pfb").display().to_string();
            let found = lookup(&name, Format::Type1);
            if confined {
                assert_eq!(found, None);
            } else {
                assert_eq!(found.as_deref(), Some(name.as_str()));
            }
        },
    );
}

/// An encoding in the project that is a link out of the roots: the
/// writer's lookup finds nothing; a plain project encoding is still found.
#[test]
fn lookup_again_refuses_a_linked_encoding() {
    case(
        "lookup_again_refuses_a_linked_encoding",
        |d| vec![format!("evil.enc\t{}", real(&d.outside.join("evil.enc")))],
        |confined, _| {
            let found = lookup("evil.enc", Format::Enc);
            if confined {
                assert_eq!(found, None);
            } else {
                let found = found.expect("found unconfined");
                assert!(found.ends_with("evil.enc"), "{found}");
            }
            let inside = lookup("inside.enc", Format::Enc).expect("a project encoding");
            assert!(inside.ends_with("inside.enc"), "{inside}");
        },
    );
}

/// A file in the output directory that is a link out of the roots (the
/// shortcut `open_input` takes before the resolver): refused, and the
/// lookup goes on to the resolver, which has no such file.
#[test]
fn lookup_again_refuses_a_linked_file_in_the_output_directory() {
    case(
        "lookup_again_refuses_a_linked_file_in_the_output_directory",
        |d| vec![format!("evil.map\t{}", real(&d.outside.join("evil.map")))],
        |confined, _| {
            let argv: Vec<String> = ["flashtex-initex", "-ini", "-output-directory=out", "x.tex"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            system::configure(flashtex_engine::cli::parse(&argv));
            let found = lookup("evil.map", Format::Map);
            if confined {
                assert_eq!(found, None);
            } else {
                assert_eq!(found.as_deref(), Some("out/evil.map"));
            }
        },
    );
}

/// A PK file in the project that is a link out of the roots (`PKFONTS`
/// starts with `.`): the writer's quiet lookup finds nothing.
#[test]
fn find_pk_quietly_refuses_a_linked_pk_file() {
    case(
        "find_pk_quietly_refuses_a_linked_pk_file",
        |d| vec![format!("evil\t{}", real(&d.outside.join("evil.600pk")))],
        |confined, _| {
            system::pk_init(600, None);
            let found = system::find_pk_quietly("evil", 600);
            if confined {
                assert!(found.is_none(), "{found:?}");
            } else {
                let g = found.expect("found unconfined");
                assert!(g.path.ends_with("evil.600pk"), "{g:?}");
            }
        },
    );
}

/// The same for writet3's lookup, the one that puts the bitmaps in the PDF.
#[test]
fn find_pk_refuses_a_linked_pk_file() {
    case(
        "find_pk_refuses_a_linked_pk_file",
        |d| vec![format!("evil\t{}", real(&d.outside.join("evil.600pk")))],
        |confined, _| {
            system::pk_init(600, None);
            let found = system::find_pk("evil", 600);
            if confined {
                assert!(found.is_none(), "{found:?}");
            } else {
                let g = found.expect("found unconfined");
                assert!(g.path.ends_with("evil.600pk"), "{g:?}");
            }
        },
    );
}
