//! The port against TeX Live 2026's makeindex on small cases whose oracle
//! output is committed (tests/golden/, written by
//! docs/evidence/cold-speed-2026-10-04/makeindex/harness/golden.py): every
//! output file, standard output, standard error and the exit status must be
//! identical. The full comparison (fixtures, books, thousands of random
//! cases) runs against the oracle itself; see that directory's README.

use std::io::Write;
use std::path::{Path, PathBuf};

struct TestHost {
    dir: PathBuf,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl flashtex_makeindex::Host for TestHost {
    fn find_ist(&mut self, name: &[u8]) -> Option<Vec<u8>> {
        // kpathsea's answer for a style file in the working directory
        let name = String::from_utf8_lossy(name).into_owned();
        [name.clone(), format!("{name}.ist")]
            .into_iter()
            .find(|n| self.dir.join(n).is_file())
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

fn check(case: &Path) -> Result<(), String> {
    let name = case.file_name().unwrap().to_string_lossy().into_owned();
    let dir = std::env::temp_dir().join(format!(
        "flashtex-makeindex-golden-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut inputs = vec![];
    for e in std::fs::read_dir(case).unwrap().flatten() {
        let f = e.file_name().to_string_lossy().into_owned();
        if e.path().is_file() && f != "args" {
            std::fs::copy(e.path(), dir.join(&f)).unwrap();
            inputs.push(f);
        }
    }
    let args: Vec<Vec<u8>> = std::fs::read_to_string(case.join("args"))
        .unwrap()
        .lines()
        .map(|l| l.as_bytes().to_vec())
        .collect();
    let mut host = TestHost {
        dir: dir.clone(),
        stdout: vec![],
        stderr: vec![],
    };
    let status = flashtex_makeindex::run(&args, &mut host);
    let exp = case.join("expected");
    let mut problems = vec![];
    let want_status: i32 = std::fs::read_to_string(exp.join("status"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    if status != want_status {
        problems.push(format!("status {status}, oracle {want_status}"));
    }
    for (what, got) in [("stdout", &host.stdout), ("stderr", &host.stderr)] {
        if &std::fs::read(exp.join(what)).unwrap() != got {
            problems.push(format!(
                "{what} differs: {:?}",
                String::from_utf8_lossy(got)
            ));
        }
    }
    let mut want: Vec<String> = std::fs::read_dir(&exp)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|f| !["stdout", "stderr", "status"].contains(&f.as_str()))
        .collect();
    want.sort();
    let mut made: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|f| !inputs.contains(f))
        .collect();
    made.sort();
    if made != want {
        problems.push(format!("files {made:?}, oracle {want:?}"));
    }
    for f in &want {
        let got = std::fs::read(dir.join(f)).unwrap_or_default();
        if got != std::fs::read(exp.join(f)).unwrap() {
            problems.push(format!("{f} differs:\n{}", String::from_utf8_lossy(&got)));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!("{name}: {}", problems.join("\n")))
    }
}

#[test]
fn golden_cases_match_texlive() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut cases: Vec<PathBuf> = std::fs::read_dir(&root)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert!(cases.len() >= 8, "golden cases missing");
    let failures: Vec<String> = cases.iter().filter_map(|c| check(c).err()).collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
