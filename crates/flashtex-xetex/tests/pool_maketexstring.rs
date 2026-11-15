//! texmfmp.c's `maketexstring` with the string pool nearly full (#2515).
//! Its `checkpoolpointer(poolptr, len)` is not a TeX error: TeX Live 2026's
//! `xetex -ini -etex -no-pdf -jobname=<200 letters> <fill>\end` (2026-11-15)
//! writes `string pool overflow [6250000 bytes]` on stderr and leaves with
//! status 1 before the log is opened, for every room left from 1800 to 1975
//! characters with 6218 names of 1000 characters (25-character steps; 1700
//! to 1775 end normally, 2000 and up stop with `TeX capacity exceeded`
//! when the room is made). FlashTeX wrote past the pool's end there and
//! panicked (exit 101).

use std::path::PathBuf;
use std::process::{Command, Stdio};

fn run_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "flashtex-xetex-pool-maketexstring-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `k` control-sequence names of 1000 characters, then one of `l`
/// characters, so that `l` sets the room left in the pool; then `\end`.
fn pool_fill_end(k: u32, l: usize) -> String {
    format!(
        "\\catcode`\\{{=1 \\catcode`\\}}=2 \\catcode`\\#=6 \\scrollmode \\def\\X{{{x}}}\
         \\count1=10000 \\def\\fill{{\\ifnum\\count1<{end} \\advance\\count1 1 \
         \\expandafter\\let\\csname\\the\\count1 \\X\\endcsname\\relax \\expandafter\\fill\\fi}}\\fill \
         \\expandafter\\let\\csname {y}\\endcsname\\relax \\end",
        x = "x".repeat(995),
        end = 10000 + k,
        y = "y".repeat(l),
    )
}

/// `getjobname` makes `-jobname`'s string when `\end` opens the log. The
/// sweep is wide because FlashTeX's INITEX pool and TeX Live's need not
/// have the same room left (#2208); the window must be hit, and every other
/// run must end as xetex's can.
#[test]
fn jobname_past_the_pool_is_string_pool_overflow() {
    let dir = run_dir();
    let job = format!("-jobname={}", "j".repeat(200));
    let mut exits = 0;
    for l in (1700..=2700).step_by(25) {
        for f in std::fs::read_dir(&dir).unwrap() {
            let _ = std::fs::remove_file(f.unwrap().path());
        }
        let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
            .current_dir(&dir)
            .env("SOURCE_DATE_EPOCH", "0")
            .env("FORCE_SOURCE_DATE", "1")
            .env("FLASHTEX_RESOLVER", "cwd")
            .args(["-ini", "-etex", "-no-pdf", &job, &pool_fill_end(6218, l)])
            .stdin(Stdio::null())
            .output()
            .expect("run flashtex-xetex");
        let code = out.status.code();
        let err = String::from_utf8_lossy(&out.stderr);
        let term = String::from_utf8_lossy(&out.stdout);
        assert!(
            !err.contains("panicked") && code != Some(101),
            "l={l}: exit {code:?}\nstderr:\n{err}"
        );
        if err.contains("string pool overflow") {
            assert_eq!(err, "\nstring pool overflow [6250000 bytes]\n", "l={l}");
            assert_eq!(code, Some(1), "l={l}");
            let logs = std::fs::read_dir(&dir)
                .unwrap()
                .filter(|f| {
                    f.as_ref()
                        .unwrap()
                        .path()
                        .extension()
                        .is_some_and(|e| e == "log")
                })
                .count();
            assert_eq!(logs, 0, "l={l}: no log is opened");
            exits += 1;
        } else {
            assert!(
                code == Some(0) || term.contains("! TeX capacity exceeded, sorry [pool size="),
                "l={l}: exit {code:?}\n{term}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(exits > 0, "no run hit maketexstring's pool check");
}
