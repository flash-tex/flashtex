//! The port against TeX Live 2026's bibtex, with no TeX Live needed: each
//! directory of `tests/golden/` holds a case's inputs (the `.aux`, the
//! `.bib` files and the `.bst` it uses), its `args`, and in `expected/`
//! what TeX Live's bibtex did (exit status, standard output and error, and
//! every file it wrote), written by
//! `docs/evidence/rusttools-2026-10-09/bibtex/harness/golden.py`.
//!
//! The host finds files in the case's directory, where kpathsea's `.` would
//! find them, and answers texmf.cnf's values for program `bibtex`.

use flashtex_bibtex::{Format, Host};
use std::path::{Path, PathBuf};

struct CaseHost {
    dir: PathBuf,
}

impl Host for CaseHost {
    fn find_file(&mut self, name: &[u8], format: Format) -> Option<Vec<u8>> {
        let name = String::from_utf8_lossy(name).into_owned();
        let suffix = match format {
            Format::Bib => ".bib",
            Format::Bst => ".bst",
        };
        let mut tries = vec![];
        if !name.ends_with(suffix) {
            tries.push(format!("{name}{suffix}"));
        }
        tries.push(name);
        tries
            .into_iter()
            .find(|n| self.dir.join(n).is_file())
            .map(|n| format!("./{n}").into_bytes())
    }
    fn var_value(&mut self, name: &str) -> Option<String> {
        // TeX Live 2026's texmf.cnf, as `kpsewhich -progname=bibtex`
        // answers it.
        match name {
            "max_strings" => Some("200000".into()),
            "ent_str_size" => Some("500".into()),
            "glob_str_size" => Some("200000".into()),
            "max_print_line" => Some("79".into()),
            _ => None,
        }
    }
    fn cwd(&self) -> Option<&Path> {
        Some(&self.dir)
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let n = e.file_name();
        if n == "expected" || n == "args" {
            continue;
        }
        std::fs::copy(e.path(), to.join(n)).unwrap();
    }
}

#[test]
fn golden() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut cases: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert!(cases.len() >= 10, "golden cases missing");
    let work = std::env::temp_dir().join(format!("bibtex-golden-{}", std::process::id()));
    let mut failures = vec![];
    for case in &cases {
        let name = case.file_name().unwrap().to_string_lossy().into_owned();
        let dir = work.join(&name);
        let _ = std::fs::remove_dir_all(&dir);
        copy_dir(case, &dir);
        let args: Vec<Vec<u8>> = std::fs::read_to_string(case.join("args"))
            .unwrap()
            .lines()
            .map(|l| l.as_bytes().to_vec())
            .collect();
        let o = flashtex_bibtex::run(&args, Box::new(CaseHost { dir: dir.clone() }));
        let exp = case.join("expected");
        let want_status: i32 = std::fs::read_to_string(exp.join("status"))
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        if o.status != want_status {
            failures.push(format!("{name}: status {} want {want_status}", o.status));
        }
        for (what, got) in [("stdout", &o.stdout), ("stderr", &o.stderr)] {
            if &std::fs::read(exp.join(what)).unwrap() != got {
                failures.push(format!(
                    "{name}: {what} differs:\n{}",
                    String::from_utf8_lossy(got)
                ));
            }
        }
        for e in std::fs::read_dir(&exp).unwrap() {
            let p = e.unwrap().path();
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            if ["status", "stdout", "stderr"].contains(&n.as_str()) {
                continue;
            }
            let got = std::fs::read(dir.join(&n)).unwrap_or_default();
            if got != std::fs::read(&p).unwrap() {
                failures.push(format!("{name}: {n} differs"));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&work);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
