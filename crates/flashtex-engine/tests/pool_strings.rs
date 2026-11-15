//! #2208: INITEX's string pool is TeX Live's pdfTeX's, string for string.
//!
//! TeX Live tangles pdfTeX with encTeX and ML\TeX, whose code adds 18 strings
//! (409 characters) to the pool even though only `-enc` and `-mltex` reach it
//! (changes/pool.ch). The pool's size shows in what a run prints: the string
//! count and pool length after `\dump` (`str_ptr` and `pool_ptr`, "N strings of
//! total length M"), and a production run's capacities counted from what the
//! format holds ("N strings out of M", "N string characters out of M"; the same
//! differences are the capacities in `TeX capacity exceeded, sorry [pool
//! size=...]`). Each of these lines is compared with TeX Live's `pdftex`, for
//! INITEX with nothing loaded and for plain.fmt. Skips where there is no TeX
//! Live (`FLASHTEX_REQUIRE_TEXLIVE=1` makes that a failure).
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::Path;
use std::process::{Command, Stdio};

fn run(bin: &Path, dir: &Path, args: &[&str], ours: bool) {
    let mut c = Command::new(bin);
    c.args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if ours {
        c.env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        );
    }
    c.status().unwrap();
}

/// The log's lines that count the string pool.
fn pool_lines(log: &str) -> Vec<String> {
    log.lines()
        .filter(|l| {
            l.contains(" strings of total length ")
                || l.contains(" strings out of ")
                || l.contains(" string characters out of ")
        })
        .map(str::to_string)
        .collect()
}

#[test]
fn the_pool_after_dump_and_its_capacities_are_tex_live_s() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let theirs = texbin.join("pdftex");
    let base = common::fresh_dir("flashtex-pool-strings");
    let (a, b) = (base.join("ours"), base.join("tex"));
    for d in [&a, &b] {
        std::fs::create_dir_all(d).unwrap();
        // (`\tracingstats` positive: the statistics go to the log at the end)
        std::fs::write(
            d.join("s.tex"),
            "\\batchmode\\tracingstats=1 \\setbox0\\hbox{Hello}\\end\n",
        )
        .unwrap();
    }
    for (bin, dir, mine) in [(ours, &a, true), (theirs.as_path(), &b, false)] {
        // INITEX with nothing loaded: the pool as the program made it
        run(bin, dir, &["-ini", "-jobname=bare", "\\dump"], mine);
        run(bin, dir, &["-ini", "\\input plain \\dump"], mine);
        run(bin, dir, &["&plain s"], mine);
    }
    for (log, want) in [("bare.log", 1), ("plain.log", 1), ("s.log", 2)] {
        let x = std::fs::read_to_string(a.join(log)).unwrap();
        let y = std::fs::read_to_string(b.join(log)).unwrap();
        let (x, y) = (pool_lines(&x), pool_lines(&y));
        assert_eq!(y.len(), want, "{log}: TeX Live's pool lines: {y:?}");
        assert_eq!(
            x, y,
            "{log}: the string pool differs from TeX Live's pdftex"
        );
    }
    let _ = std::fs::remove_dir_all(&base);
}
