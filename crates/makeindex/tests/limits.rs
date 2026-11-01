//! Inputs the port refuses where the C program would read without end or
//! without bound (a host running it in-process must survive them): a link
//! to `/dev/zero`, a FIFO, a file over `MAX_INPUT` bytes. Each run ends
//! promptly with makeindex's own failure for a file it cannot open, after a
//! message that says why.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

struct TestHost {
    dir: PathBuf,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl flashtex_makeindex::Host for TestHost {
    fn find_ist(&mut self, name: &[u8]) -> Option<Vec<u8>> {
        let name = String::from_utf8_lossy(name).into_owned();
        [name.clone(), format!("{name}.ist")]
            .into_iter()
            .find(|n| self.dir.join(n).exists())
            .map(|n| format!("./{n}").into_bytes())
    }
    fn stdout(&mut self) -> &mut dyn Write {
        &mut self.stdout
    }
    fn stderr(&mut self) -> &mut dyn Write {
        &mut self.stderr
    }
    fn read_stdin(&mut self) -> Vec<u8> {
        vec![]
    }
    fn cwd(&self) -> Option<&Path> {
        Some(&self.dir)
    }
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flashtex-makeindex-limits-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(d: &Path, args: &[&str]) -> (i32, String) {
    let mut h = TestHost {
        dir: d.to_path_buf(),
        stdout: vec![],
        stderr: vec![],
    };
    let args: Vec<Vec<u8>> = args.iter().map(|a| a.as_bytes().to_vec()).collect();
    let t0 = Instant::now();
    let status = flashtex_makeindex::run(&args, &mut h);
    assert!(
        t0.elapsed() < Duration::from_secs(20),
        "took {:?}",
        t0.elapsed()
    );
    (status, String::from_utf8_lossy(&h.stderr).into_owned())
}

#[cfg(unix)]
#[test]
fn a_link_to_dev_zero_is_refused() {
    let d = dir("zero");
    std::os::unix::fs::symlink("/dev/zero", d.join("z.idx")).unwrap();
    let (status, err) = run(&d, &["z.idx"]);
    assert_eq!(status, 1, "{err}");
    assert!(
        err.contains("makeindex: refusing to read z.idx: not a regular file"),
        "{err}"
    );
    // as a style file too
    std::fs::write(d.join("x.idx"), "\\indexentry{a}{1}\n").unwrap();
    std::os::unix::fs::symlink("/dev/zero", d.join("zz.ist")).unwrap();
    let (status, err) = run(&d, &["-s", "zz.ist", "x.idx"]);
    assert_eq!(status, 1, "{err}");
    assert!(err.contains("not a regular file"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[cfg(unix)]
#[test]
fn a_fifo_is_refused() {
    let d = dir("fifo");
    let p = d.join("f.idx");
    let c = std::ffi::CString::new(p.to_str().unwrap()).unwrap();
    // SAFETY: a NUL-terminated path.
    assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
    let (status, err) = run(&d, &["f.idx"]);
    assert_eq!(status, 1, "{err}");
    assert!(err.contains("not a regular file"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn an_oversized_input_is_refused() {
    let d = dir("big");
    // sparse: no bytes are written
    let f = std::fs::File::create(d.join("big.idx")).unwrap();
    f.set_len(flashtex_makeindex::MAX_INPUT + 1).unwrap();
    drop(f);
    let (status, err) = run(&d, &["big.idx"]);
    assert_eq!(status, 1, "{err}");
    assert!(err.contains("more than FlashTeX's limit"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}
