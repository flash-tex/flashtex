//! Inputs the port refuses where the C program would read without end or
//! without bound (a host running it in-process must survive them): a link
//! to `/dev/zero` as the `.aux` or as a `.bib`, a FIFO, a file over
//! `MAX_INPUT` bytes. Each run ends promptly, as for a file that cannot be
//! opened, after a message that says why.

use flashtex_bibtex::{Format, Host};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Finds any directory entry by name (not only regular files), so that the
/// port's own check is what refuses.
struct AnyHost {
    dir: PathBuf,
}

impl Host for AnyHost {
    fn find_file(&mut self, name: &[u8], format: Format) -> Option<Vec<u8>> {
        let name = String::from_utf8_lossy(name).into_owned();
        let suffix = match format {
            Format::Bib => ".bib",
            Format::Bst => ".bst",
        };
        [format!("{name}{suffix}"), name]
            .into_iter()
            .find(|n| self.dir.join(n).symlink_metadata().is_ok())
            .map(|n| format!("./{n}").into_bytes())
    }
    fn var_value(&mut self, _name: &str) -> Option<String> {
        None
    }
    fn cwd(&self) -> Option<&Path> {
        Some(&self.dir)
    }
}

const BST: &str = "ENTRY { title } {} {}\n\
FUNCTION {book} { cite$ write$ newline$ }\n\
READ\n\
ITERATE {call.type$}\n";

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flashtex-bibtex-limits-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("one.bst"), BST).unwrap();
    d
}

fn run(d: &Path) -> flashtex_bibtex::Outcome {
    let t0 = Instant::now();
    let o = flashtex_bibtex::run(
        &[b"doc".to_vec()],
        Box::new(AnyHost {
            dir: d.to_path_buf(),
        }),
    );
    assert!(
        t0.elapsed() < Duration::from_secs(20),
        "took {:?}",
        t0.elapsed()
    );
    o
}

fn write_aux(d: &Path) {
    std::fs::write(
        d.join("doc.aux"),
        "\\citation{a}\n\\bibstyle{one}\n\\bibdata{refs}\n",
    )
    .unwrap();
}

#[cfg(unix)]
#[test]
fn a_bib_linked_to_dev_zero_is_refused() {
    let d = dir("bibzero");
    write_aux(&d);
    std::os::unix::fs::symlink("/dev/zero", d.join("refs.bib")).unwrap();
    let o = run(&d);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("bibtex: refusing to read refs.bib: not a regular file"),
        "{err}"
    );
    let blg = std::fs::read_to_string(d.join("doc.blg")).unwrap();
    assert!(
        blg.contains("I couldn't open database file refs.bib"),
        "{blg}"
    );
    assert_eq!(o.status, 2);
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(unix)]
#[test]
fn an_aux_linked_to_dev_zero_is_refused() {
    let d = dir("auxzero");
    std::os::unix::fs::symlink("/dev/zero", d.join("doc.aux")).unwrap();
    let o = run(&d);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(
        err.contains("bibtex: refusing to read doc.aux: not a regular file"),
        "{err}"
    );
    // bibtex's answer for an .aux it cannot open
    assert_eq!(o.status, 1);
    assert!(String::from_utf8_lossy(&o.stdout).contains("I couldn't open file name `doc.aux'"));
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(unix)]
#[test]
fn a_fifo_is_refused() {
    let d = dir("fifo");
    write_aux(&d);
    let p = d.join("refs.bib");
    let c = std::ffi::CString::new(p.to_str().unwrap()).unwrap();
    extern "C" {
        fn mkfifo(path: *const std::ffi::c_char, mode: u32) -> i32;
    }
    // SAFETY: a NUL-terminated path.
    assert_eq!(unsafe { mkfifo(c.as_ptr(), 0o600) }, 0);
    let o = run(&d);
    assert!(String::from_utf8_lossy(&o.stderr).contains("not a regular file"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_oversized_bib_is_refused() {
    let d = dir("big");
    write_aux(&d);
    let f = std::fs::File::create(d.join("refs.bib")).unwrap();
    f.set_len(flashtex_bibtex::MAX_INPUT + 1).unwrap(); // sparse
    drop(f);
    let o = run(&d);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("more than FlashTeX's limit"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}
