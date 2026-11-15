//! File-system branches the earlier suites never reach: `$TEXMFOUTPUT`
//! output fallback, a directory where an input file is expected, and the
//! `\@input` search beside the top-level `.aux`.

use flashtex_bibtex::{Format, Host};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const BST: &str = "ENTRY { title } {} {}\n\
FUNCTION {book} { cite$ write$ newline$ }\n\
READ\n\
ITERATE {call.type$}\n";

#[derive(Default)]
struct TestHost {
    dir: PathBuf,
    vars: HashMap<String, String>,
}

impl TestHost {
    fn new(dir: PathBuf) -> TestHost {
        TestHost {
            dir,
            ..TestHost::default()
        }
    }
}

impl Host for TestHost {
    fn find_file(&mut self, name: &[u8], format: Format) -> Option<Vec<u8>> {
        // Any directory entry, so the port's own open decides (as in
        // limits.rs): a directory must reach fopen_read, not be skipped.
        let name = String::from_utf8_lossy(name).into_owned();
        let suffix = if format == Format::Bib {
            ".bib"
        } else {
            ".bst"
        };
        [format!("{name}{suffix}"), name]
            .into_iter()
            .find(|n| self.dir.join(n).symlink_metadata().is_ok())
            .map(|n| format!("./{n}").into_bytes())
    }
    fn var_value(&mut self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
    fn cwd(&self) -> Option<&Path> {
        Some(&self.dir)
    }

    /// kpathsea's default `openout_any = p` (tex-file.c:1338-1346): an
    /// absolute name is refused unless it lies under `$TEXMFOUTPUT`.
    fn out_name_ok(&mut self, name: &[u8]) -> Result<(), Vec<u8>> {
        match self.vars.get("TEXMFOUTPUT") {
            Some(o) if name.first() == Some(&b'/') && !name.starts_with(o.as_bytes()) => {
                Err(b"p".to_vec())
            }
            _ => Ok(()),
        }
    }
}

fn workdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-bibtex-io-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn minimal_input(d: &Path) {
    std::fs::write(
        d.join("doc.aux"),
        "\\citation{key1}\n\\bibstyle{one}\n\\bibdata{refs}\n",
    )
    .unwrap();
    std::fs::write(d.join("refs.bib"), "@book{key1, title={Some Title}}\n").unwrap();
    std::fs::write(d.join("one.bst"), BST).unwrap();
}

fn run(args: &[&str], host: TestHost) -> flashtex_bibtex::Outcome {
    let args: Vec<Vec<u8>> = args.iter().map(|a| a.as_bytes().to_vec()).collect();
    flashtex_bibtex::run(&args, Box::new(host))
}

/// `open_output` falls back to `$TEXMFOUTPUT/name` when a relative name
/// cannot be created, so with `doc.blg` blocked by a directory the log lands
/// in `$TEXMFOUTPUT`. As in TeX Live, a successful fallback copies the full
/// path into `name_of_file` (openclose.c), and bibtex.ch §106 builds the
/// `.bbl` name from the first `aux_name_length` bytes of that buffer: it
/// becomes the fallback path's 3-byte prefix + `.bbl`, which kpathsea
/// refuses. Faithful, not a port bug.
#[test]
fn texmfoutput_receives_outputs_cwd_cannot_hold() {
    let d = workdir("texmfoutput");
    minimal_input(&d);
    std::fs::create_dir(d.join("doc.blg")).unwrap();
    let out = d.join("texmfout");
    std::fs::create_dir(&out).unwrap();
    let mut host = TestHost::new(d.clone());
    host.vars
        .insert("TEXMFOUTPUT".into(), out.to_str().unwrap().to_owned());
    let o = run(&["doc"], host);
    let bbl = format!("{}.bbl", &out.to_str().unwrap()[..3]); // "doc".len() == 3
    assert_eq!(o.status, 1);
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert!(
        stdout.contains(&format!("I couldn't open file name `{bbl}'")),
        "{stdout}"
    );
    let stderr = String::from_utf8_lossy(&o.stderr);
    assert!(
        stderr.contains(&format!("Not writing to {bbl} (openout_any = p")),
        "{stderr}"
    );
    // The log lands in $TEXMFOUTPUT. TeX Live's bibtex leaves it empty on this
    // abort path too, so only its location is asserted.
    assert!(out.join("doc.blg").is_file());
    assert!(!d.join("doc.bbl").exists());
    let _ = std::fs::remove_dir_all(&d);
}

/// Without the fallback (unset or empty `$TEXMFOUTPUT`), the blocked
/// output is the run's error, as for an unwritable name.
#[test]
fn blocked_output_without_texmfoutput_is_an_error() {
    for (tag, var) in [("unset", None), ("empty", Some(""))] {
        let d = workdir(&format!("no-texmfoutput-{tag}"));
        minimal_input(&d);
        std::fs::create_dir(d.join("doc.blg")).unwrap();
        let mut host = TestHost::new(d.clone());
        if let Some(v) = var {
            host.vars.insert("TEXMFOUTPUT".into(), v.into());
        }
        let o = run(&["doc"], host);
        assert_eq!(o.status, 1, "{tag}");
        assert!(
            String::from_utf8_lossy(&o.stdout).contains("I couldn't open file name `doc.blg'"),
            "{tag}: {}",
            String::from_utf8_lossy(&o.stdout)
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}

/// `fopen` of a directory succeeds and reads EOF at once: a directory as
/// the `.bib` behaves exactly like an empty database file (no refusal,
/// the citation simply has no entry).
#[test]
fn a_directory_reads_as_an_empty_file() {
    let d = workdir("bibdir");
    minimal_input(&d);
    std::fs::remove_file(d.join("refs.bib")).unwrap();
    std::fs::create_dir(d.join("refs.bib")).unwrap();
    let o = run(&["doc"], TestHost::new(d.clone()));
    assert!(
        !String::from_utf8_lossy(&o.stderr).contains("refusing to read"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );

    let e = workdir("emptybib");
    minimal_input(&e);
    std::fs::write(e.join("refs.bib"), "").unwrap();
    let want = run(&["doc"], TestHost::new(e.clone()));

    assert_eq!(o.status, want.status);
    assert_eq!(o.stdout, want.stdout);
    assert_eq!(o.stderr, want.stderr);
    let blg = std::fs::read(d.join("doc.blg")).unwrap();
    let want_blg = std::fs::read(e.join("doc.blg")).unwrap();
    assert_eq!(blg, want_blg);
    let blg = String::from_utf8_lossy(&blg);
    assert!(
        blg.contains("I didn't find a database entry for \"key1\""),
        "{blg}"
    );
    let _ = std::fs::remove_dir_all(&d);
    let _ = std::fs::remove_dir_all(&e);
}

/// A nested `\@input` file is found beside the top-level `.aux` when it is
/// not beside the working directory (`open_input_with_dirname`): the
/// citation it carries reaches the `.bbl`.
#[test]
fn nested_aux_is_found_beside_the_top_aux() {
    let d = workdir("dirname");
    let sub = d.join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(
        sub.join("doc.aux"),
        "\\@input{part.aux}\n\\bibstyle{one}\n\\bibdata{refs}\n",
    )
    .unwrap();
    std::fs::write(sub.join("part.aux"), "\\citation{key1}\n").unwrap();
    std::fs::write(d.join("refs.bib"), "@book{key1, title={Some Title}}\n").unwrap();
    std::fs::write(d.join("one.bst"), BST).unwrap();
    assert!(!d.join("part.aux").exists());
    let o = run(&["sub/doc"], TestHost::new(d.clone()));
    assert_eq!(o.status, 0, "{}", String::from_utf8_lossy(&o.stderr));
    let bbl = std::fs::read_to_string(sub.join("doc.bbl")).unwrap();
    assert!(bbl.contains("key1"), "{bbl}");
    let blg = std::fs::read_to_string(sub.join("doc.blg")).unwrap();
    assert!(blg.contains("A level-1 auxiliary file"), "{blg}");
    let _ = std::fs::remove_dir_all(&d);
}
