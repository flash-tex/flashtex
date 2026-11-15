//! Error paths and edge cases with no golden coverage: option parsing,
//! host refusals and deadlines.

use flashtex_bibtex::{Format, Host};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const BST: &str = "ENTRY { title } {} {}\n\
FUNCTION {book} { cite$ write$ newline$ }\n\
READ\n\
ITERATE {call.type$}\n";

struct Shared(Rc<RefCell<Vec<u8>>>);

impl std::io::Write for Shared {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct TestHost {
    dir: PathBuf,
    vars: HashMap<String, String>,
    refuse_out: bool,
    refuse_read_suffix: Option<String>,
    stream: Option<(Rc<RefCell<Vec<u8>>>, usize)>,
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
        let name = String::from_utf8_lossy(name).into_owned();
        let suffix = if format == Format::Bib {
            ".bib"
        } else {
            ".bst"
        };
        [format!("{name}{suffix}"), name]
            .into_iter()
            .find(|n| self.dir.join(n).is_file())
            .map(|n| format!("./{n}").into_bytes())
    }
    fn var_value(&mut self, name: &str) -> Option<String> {
        self.vars.get(name).cloned()
    }
    fn out_name_ok(&mut self, _name: &[u8]) -> Result<(), Vec<u8>> {
        if self.refuse_out {
            Err(b"paranoid".to_vec())
        } else {
            Ok(())
        }
    }
    fn read_ok(&mut self, name: &[u8], _path: &Path, _searched: bool) -> bool {
        self.refuse_read_suffix
            .as_ref()
            .is_none_or(|s| !name.ends_with(s.as_bytes()))
    }
    fn stdout_stream(&mut self) -> Option<(Box<dyn std::io::Write>, usize)> {
        self.stream
            .clone()
            .map(|(rc, b)| (Box::new(Shared(rc)) as Box<dyn std::io::Write>, b))
    }
    fn cwd(&self) -> Option<&Path> {
        Some(&self.dir)
    }
}

fn workdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-bibtex-edge-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A complete minimal run: one citation, one database entry, a tiny style.
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

// Options

#[test]
fn bad_options_exit_1_with_a_message() {
    // Missing option argument, forbidden argument, an empty name (which
    // prefix-matches every option), and two file arguments.
    for (args, want) in [
        (&["--min-crossrefs"][..], "requires an argument"),
        (&["--terse=x", "doc"][..], "doesn't allow an argument"),
        (&["--="][..], "is ambiguous"),
        (&["doc", "other"][..], "Need exactly one file argument"),
    ] {
        let d = workdir("bad-opt");
        let o = run(args, TestHost::new(d));
        assert_eq!(o.status, 1, "{args:?}");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(err.contains(want), "{args:?}: {err}");
        assert!(err.contains("Try `bibtex --help'"), "{args:?}: {err}");
    }
}

#[test]
fn option_spellings_match_long_form() {
    // getopt_long_only: unique prefixes, one dash, and options after the
    // file argument all parse as the long option; `--` ends option parsing.
    let d = workdir("opt-forms");
    minimal_input(&d);
    let full = run(&["--terse", "doc"], TestHost::new(d.clone()));
    assert_eq!(full.status, 0);
    for args in [
        &["--ters", "doc"][..],
        &["-terse", "doc"][..],
        &["doc", "--terse"][..],
    ] {
        let o = run(args, TestHost::new(d.clone()));
        assert_eq!(o.status, full.status, "{args:?}");
        assert_eq!(o.stdout, full.stdout, "{args:?}");
    }
    let plain = run(&["doc"], TestHost::new(d.clone()));
    let o = run(&["--", "doc"], TestHost::new(d.clone()));
    assert_eq!((o.status, o.stdout), (plain.status, plain.stdout));
    assert!(d.join("doc.bbl").is_file());
    let _ = std::fs::remove_dir_all(&d);
}

// The host's answers

#[test]
fn bad_bound_values_warn_and_keep_defaults() {
    let d = workdir("bad-bound");
    minimal_input(&d);
    let mut host = TestHost::new(d.clone());
    host.vars.insert("max_strings".into(), "-5".into());
    host.vars.insert("max_print_line".into(), "0".into());
    let o = run(&["doc"], host);
    assert_eq!(o.status, 0, "{}", String::from_utf8_lossy(&o.stderr));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("Bad value (-5)"), "{err}");
    assert!(err.contains("for max_strings"), "{err}");
    assert!(err.contains("Bad value (0)"), "{err}");
    assert!(err.contains("for max_print_line"), "{err}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn refused_read_is_reported() {
    let d = workdir("read-no");
    minimal_input(&d);
    let mut host = TestHost::new(d.clone());
    host.refuse_read_suffix = Some(".bib".into());
    let o = run(&["doc"], host);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("refusing to read"), "{err}");
    assert!(
        err.contains("outside the files this host may read"),
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

#[test]
fn refused_output_name_is_reported() {
    let d = workdir("out-no");
    minimal_input(&d);
    let mut host = TestHost::new(d.clone());
    host.refuse_out = true;
    let o = run(&["doc"], host);
    assert!(o.crash.is_none());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("Not writing to"), "{err}");
    assert!(err.contains("openout_any = paranoid"), "{err}");
    assert!(String::from_utf8_lossy(&o.stdout).contains("I couldn't open file name `doc.blg'"));
    assert_eq!(o.status, 1);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn missing_style_file_is_an_error() {
    let d = workdir("no-bst");
    std::fs::write(
        d.join("doc.aux"),
        "\\citation{key1}\n\\bibstyle{nostyle}\n\\bibdata{refs}\n",
    )
    .unwrap();
    std::fs::write(d.join("refs.bib"), "@book{key1, title={Some Title}}\n").unwrap();
    let o = run(&["doc"], TestHost::new(d.clone()));
    assert!(o.crash.is_none());
    assert_eq!(o.status, 2, "{}", String::from_utf8_lossy(&o.stdout));
    assert!(String::from_utf8_lossy(&o.stdout).contains("I couldn't open style file nostyle.bst"));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn custom_stdout_stream_captures_stdout() {
    // Both stdio bufferings: a small block (partial flushes) and block 0
    // (a terminal's line buffering) deliver the same bytes.
    for block in [8usize, 0] {
        let d = workdir("stream");
        let mut host = TestHost::new(d);
        let shared = Rc::new(RefCell::new(Vec::new()));
        host.stream = Some((shared.clone(), block));
        let o = run(&["--version"], host);
        assert_eq!(o.status, 0);
        assert!(o.stdout.is_empty());
        let got = shared.borrow().clone();
        assert!(got.starts_with(b"BibTeX 0.99e (TeX Live 2026)"), "{got:?}");
    }
}

// Deadlines

#[test]
fn deadline_stops_a_past_run_and_spares_a_live_one() {
    use std::time::{Duration, Instant};
    let d = workdir("timeout");
    minimal_input(&d);
    let args = vec![b"doc".to_vec()];
    let o = flashtex_bibtex::run_with_deadline(
        &args,
        Box::new(TestHost::new(d.clone())),
        Some(Instant::now() - Duration::from_secs(1)),
    );
    assert_eq!(o.status, flashtex_bibtex::TIMED_OUT);
    assert!(o.crash.is_none());
    let o = flashtex_bibtex::run_with_deadline(
        &args,
        Box::new(TestHost::new(d.clone())),
        Some(Instant::now() + Duration::from_secs(60)),
    );
    assert_eq!(o.status, 0, "{}", String::from_utf8_lossy(&o.stderr));
    let _ = std::fs::remove_dir_all(&d);
}

// Small public units
