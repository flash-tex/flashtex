//! Public-API error paths and edge cases the golden cases and `limits.rs`
//! do not cover: fatal argument errors, missing files, refusing hosts,
//! the two sentinel statuses, and the input-size boundary. Messages are
//! the `mkind.rs` literals; sentinel triggers were found empirically.

use std::io::Write;
use std::path::{Path, PathBuf};

struct TestHost {
    dir: PathBuf,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    stdin: Vec<u8>,
    out_ok: bool,
    in_ok: bool,
}

impl TestHost {
    fn new(dir: &Path) -> Self {
        TestHost {
            dir: dir.to_path_buf(),
            stdout: vec![],
            stderr: vec![],
            stdin: vec![],
            out_ok: true,
            in_ok: true,
        }
    }
}

impl flashtex_makeindex::Host for TestHost {
    fn find_ist(&mut self, name: &[u8]) -> Option<Vec<u8>> {
        let name = String::from_utf8_lossy(name).into_owned();
        [name.clone(), format!("{name}.ist")]
            .into_iter()
            .find(|n| self.dir.join(n).is_file())
            .map(|n| format!("./{n}").into_bytes())
    }
    fn in_name_ok(&mut self, _name: &[u8]) -> bool {
        self.in_ok
    }
    fn out_name_ok(&mut self, _name: &[u8]) -> bool {
        self.out_ok
    }
    fn stdout(&mut self) -> &mut dyn Write {
        &mut self.stdout
    }
    fn stderr(&mut self) -> &mut dyn Write {
        &mut self.stderr
    }
    fn read_stdin(&mut self) -> Vec<u8> {
        self.stdin.clone()
    }
    fn cwd(&self) -> Option<&Path> {
        Some(&self.dir)
    }
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flashtex-makeindex-apierr-{}-{tag}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run(h: &mut TestHost, args: &[&str]) -> i32 {
    let args: Vec<Vec<u8>> = args.iter().map(|a| a.as_bytes().to_vec()).collect();
    flashtex_makeindex::run(&args, h)
}

fn err(h: &TestHost) -> String {
    String::from_utf8_lossy(&h.stderr).into_owned()
}

/// Every `fatal` ends with the usage line on standard error.
fn assert_usage(stderr: &str) {
    assert!(
        stderr.contains("Usage: makeindex [-ilqrcgLT] [-s sty] [-o ind] [-t log] [-p num]"),
        "{stderr}"
    );
}

#[test]
fn unknown_option_is_fatal() {
    let d = dir("unknown");
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["-z"]), 1);
    let e = err(&h);
    assert!(e.contains("Unknown option -z.\n"), "{e}");
    assert_usage(&e);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn missing_option_arguments_are_fatal() {
    for (opt, want) in [
        ("-s", "Expected -s <stylefile>\n"),
        ("-o", "Expected -o <ind>\n"),
        ("-t", "Expected -t <logfile>\n"),
        ("-p", "Expected -p <num>\n"),
    ] {
        let d = dir("optarg");
        let mut h = TestHost::new(&d);
        assert_eq!(run(&mut h, &[opt]), 1, "{opt}");
        let e = err(&h);
        assert!(e.contains(want), "{opt}: {e}");
        assert_usage(&e);
        let _ = std::fs::remove_dir_all(&d);
    }
}

#[test]
fn page_number_length_boundary() {
    // NUMBER_MAX is 99: 99 bytes are refused, 98 are accepted.
    let d = dir("pageno");
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["-p", &"1".repeat(99)]), 1);
    let e = err(&h);
    assert!(e.contains("Page number too high\n"), "{e}");
    assert_usage(&e);
    let _ = std::fs::remove_dir_all(&d);

    let d = dir("pageno-ok");
    std::fs::write(d.join("doc.idx"), "\\indexentry{a}{1}\n").unwrap();
    let mut h = TestHost::new(&d);
    assert_eq!(
        run(&mut h, &["-p", &"1".repeat(98), "doc.idx"]),
        0,
        "{}",
        err(&h)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn german_sort_with_default_quote_is_rejected() {
    // With an input file the option is invalid; on standard input it is
    // ignored: two different messages for the two paths through process_idx.
    let d = dir("german");
    std::fs::write(d.join("doc.idx"), "\\indexentry{a}{1}\n").unwrap();
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["-g", "doc.idx"]), 1);
    let e = err(&h);
    assert!(
        e.contains("Option -g invalid, quote character must be different from '\"'.\n"),
        "{e}"
    );
    let _ = std::fs::remove_dir_all(&d);

    let d = dir("german-stdin");
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["-g"]), 1);
    let e = err(&h);
    assert!(
        e.contains("Option -g ignored, quote character must be different from '\"'.\n"),
        "{e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn missing_input_file_names_what_was_tried() {
    let d = dir("missing");
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["nope.idx"]), 1);
    let e = err(&h);
    assert!(e.contains("Input index file nope.idx not found.\n"), "{e}");
    assert_usage(&e);
    let _ = std::fs::remove_dir_all(&d);

    let d = dir("missing-stem");
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["nope"]), 1);
    let e = err(&h);
    assert!(
        e.contains("Couldn't find input index file nope nor nope.idx.\n"),
        "{e}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn missing_style_file_is_fatal() {
    let d = dir("nosty");
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["-s", "nope.ist"]), 1);
    let e = err(&h);
    assert!(e.contains("Index style file nope.ist not found.\n"), "{e}");
    assert_usage(&e);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn refused_output_name_is_fatal_before_creating_anything() {
    let d = dir("outref");
    std::fs::write(d.join("doc.idx"), "\\indexentry{a}{1}\n").unwrap();
    let mut h = TestHost::new(&d);
    h.out_ok = false;
    assert_eq!(run(&mut h, &["doc.idx"]), 1);
    let e = err(&h);
    assert!(
        e.contains("Can't create output index file doc.ind.\n"),
        "{e}"
    );
    assert!(!d.join("doc.ind").exists());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn refused_input_name_looks_like_a_missing_file() {
    let d = dir("inref");
    std::fs::write(d.join("doc.idx"), "\\indexentry{a}{1}\n").unwrap();
    let mut h = TestHost::new(&d);
    h.in_ok = false;
    assert_eq!(run(&mut h, &["doc.idx"]), 1);
    assert!(
        err(&h).contains("Input index file doc.idx not found.\n"),
        "{}",
        err(&h)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn missing_log_for_page_init_is_fatal() {
    let d = dir("nolog");
    std::fs::write(d.join("doc.idx"), "\\indexentry{a}{1}\n").unwrap();
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["-p", "odd", "doc.idx"]), 1);
    assert!(
        err(&h).contains("Source log file doc.log not found.\n"),
        "{}",
        err(&h)
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn crlf_log_with_page_init_loops_forever() {
    // find_pageno's backward scan repeats a state on a log ending in CR LF
    // (reading the CR also reads the LF); with LF ends the same run works.
    // The created output files hold nothing, as in the C program.
    let d = dir("loops");
    std::fs::write(d.join("doc.idx"), "\\indexentry{a}{1}\n").unwrap();
    std::fs::write(d.join("doc.log"), b"some log [12]\r\n").unwrap();
    let mut h = TestHost::new(&d);
    assert_eq!(
        run(&mut h, &["-p", "odd", "doc.idx"]),
        flashtex_makeindex::LOOPS_FOREVER
    );
    assert_eq!(std::fs::read(d.join("doc.ind")).unwrap(), b"");
    assert_eq!(std::fs::read(d.join("doc.ilg")).unwrap(), b"");
    let _ = std::fs::remove_dir_all(&d);

    let d = dir("loops-lf");
    std::fs::write(d.join("doc.idx"), "\\indexentry{a}{1}\n").unwrap();
    std::fs::write(d.join("doc.log"), b"some log [12]\n").unwrap();
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &["-p", "odd", "doc.idx"]), 0, "{}", err(&h));
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn duplicate_first_sorted_entry_is_killed_by_sigsegv() {
    // The sort marks one of the two "9" entries a duplicate and the
    // marked one sorts first; the C program then dereferences a NULL
    // prev. Only whole stdio blocks written before reach the files: none.
    let d = dir("sigsegv");
    std::fs::write(
        d.join("doc.idx"),
        "\\indexentry{10}{1}\n\
         \\indexentry{a}{1}\n\
         \\indexentry{9}{1}\n\
         \\indexentry{9|(}{1}\n\
         \\indexentry{ab }{1}\n\
         \\indexentry{9}{1}\n",
    )
    .unwrap();
    let mut h = TestHost::new(&d);
    assert_eq!(
        run(&mut h, &["doc.idx"]),
        flashtex_makeindex::KILLED_BY_SIGSEGV
    );
    assert_eq!(std::fs::read(d.join("doc.ind")).unwrap(), b"");
    assert_eq!(std::fs::read(d.join("doc.ilg")).unwrap(), b"");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn empty_stdin_writes_nothing_and_succeeds() {
    let d = dir("stdin");
    let mut h = TestHost::new(&d);
    assert_eq!(run(&mut h, &[]), 0);
    assert!(h.stdout.is_empty());
    let e = err(&h);
    assert!(e.contains("Nothing written in stdout.\n"), "{e}");
    assert!(e.contains("Transcript written in stderr.\n"), "{e}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn read_capped_enforces_max_input() {
    assert_eq!(flashtex_makeindex::read_capped(&[][..]), Ok(vec![]));
    assert_eq!(
        flashtex_makeindex::read_capped(&b"abc"[..]),
        Ok(b"abc".to_vec())
    );
    let max = flashtex_makeindex::MAX_INPUT as usize;
    let exact = vec![7u8; max];
    assert_eq!(
        flashtex_makeindex::read_capped(&exact[..]).unwrap().len(),
        max
    );
    let over = vec![7u8; max + 1];
    let why = flashtex_makeindex::read_capped(&over[..]).unwrap_err();
    assert!(why.contains("more than FlashTeX's limit"), "{why}");
}

#[test]
fn read_input_reports_missing_files_and_directories() {
    let d = dir("readinput");
    assert!(flashtex_makeindex::read_input(&d.join("absent.idx")).is_none());
    assert_eq!(flashtex_makeindex::read_input(&d), Some(Ok(vec![])));
    std::fs::write(d.join("f.idx"), b"data").unwrap();
    assert_eq!(
        flashtex_makeindex::read_input(&d.join("f.idx")),
        Some(Ok(b"data".to_vec()))
    );
    let _ = std::fs::remove_dir_all(&d);
}
