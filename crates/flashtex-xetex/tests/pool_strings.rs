//! INITEX's string pool is TeX Live's XeTeX's (#2527): the same number of
//! strings and characters, which show after `\dump` (`N strings of total
//! length M`), in a run's statistics (`strings out of`, `string characters
//! out of`) and in `TeX capacity exceeded, sorry [pool size=...]`. Without
//! changes/pool.ch the port had 15 strings and 428 characters fewer. The
//! expected lines are TeX Live 2026's XeTeX 0.999998 (MacTeX 2026, macOS
//! arm64; x86_64-linux gives the same) on the same input, with
//! `SOURCE_DATE_EPOCH=0` (the format's ` (preloaded format=d 1970.1.1)` is
//! one of the strings) and the file named `./s.tex` (kpathsea's name for it,
//! which the `cwd` resolver would shorten). No TeX installation needed.

use std::path::{Path, PathBuf};
use std::process::Command;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flashtex-xetex-pool-strings-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn initex(d: &Path, args: &[&str]) {
    Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .arg("-ini")
        .args(args)
        .current_dir(d)
        .env("FLASHTEX_RESOLVER", "cwd")
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .output()
        .expect("run flashtex-xetex");
}

fn lines_with(log: &Path, pats: &[&str]) -> Vec<String> {
    std::fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .filter(|l| pats.iter().any(|p| l.contains(p)))
        .map(str::to_string)
        .collect()
}

#[test]
fn the_dumped_pool_is_tex_lives() {
    let d = dir("dump");
    initex(&d, &["-jobname=d", "\\dump"]);
    assert_eq!(
        lines_with(&d.join("d.log"), &["strings of total length"]),
        ["66981 strings of total length 30051"]
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_runs_pool_statistics_are_tex_lives() {
    let d = dir("stats");
    std::fs::write(
        d.join("s.tex"),
        "\\catcode`\\{=1 \\catcode`\\}=2 \\tracingstats=1 \\tracingonline=1\n\\showlists\n\\end\n",
    )
    .unwrap();
    initex(&d, &["-jobname=s", "./s.tex"]);
    assert_eq!(
        lines_with(
            &d.join("s.log"),
            &["strings out of", "string characters out of"]
        ),
        [" 6 strings out of 498558", " 23 string characters out of 6219985"]
    );
    let _ = std::fs::remove_dir_all(&d);
}
