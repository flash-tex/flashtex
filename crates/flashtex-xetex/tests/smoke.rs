//! A smoke test of the built program, without TeX Live: INITEX, a box of
//! rules shipped out, Unicode input. The lines checked are TeX Live 2026's
//! `xetex -ini -no-pdf` output for the same input (verified 2026-10-04); the
//! P-T1 and XDV comparison with xetex itself is tools/xetex-lockstep
//! (scripts/xetex-lockstep.sh), which needs TeX Live.

use std::path::PathBuf;
use std::process::Command;

const INPUT: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\showboxdepth=10 \\showboxbreadth=10
\\setbox0=\\hbox{\\vrule width 2pt height 3pt depth 1pt \\kern1pt}
\\showbox0 \\shipout\\box0
\\message{[\\Uchar\"1F34C][^^^^00e9][\\number`^^^^^^01f34c]}
\\end
";

fn run_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-smoke-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn initex_ships_a_box_and_reads_unicode() {
    let dir = run_dir();
    std::fs::write(dir.join("smoke.tex"), INPUT).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        // Files are found in the working directory, no texmf.cnf needed.
        .env("FLASHTEX_RESOLVER", "cwd")
        .args([
            "-ini",
            "-no-pdf",
            "-shell-restricted",
            "-parse-first-line",
            "-interaction=nonstopmode",
            "smoke.tex",
        ])
        .output()
        .expect("run flashtex-xetex");
    // \showbox in nonstop mode is an error: exit status 1, as xetex's.
    assert_eq!(out.status.code(), Some(1));
    let log = std::fs::read_to_string(dir.join("smoke.log")).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(
        lines[0],
        "This is XeTeX, Version 3.141592653-2.6-0.999998 (TeX Live 2026) (INITEX)  1 JAN 1970 00:00"
    );
    for want in [
        " restricted \\write18 enabled.",
        " %&-line parsing enabled.",
        "> \\box0=",
        "\\hbox(3.0+1.0)x3.0",
        ".\\rule(3.0+1.0)x2.0",
        ".\\kern 1.0",
        "! OK.",
        "[0] [\u{1F34C}][^^^^00e9][94^^^^^01f34c] )",
        "Output written on smoke.xdv (1 page, 164 bytes).",
    ] {
        assert!(lines.contains(&want), "missing {want:?} in:\n{log}");
    }
    // An XDV file: `pre`, XeTeX's id byte 7, and the comment of the date
    // SOURCE_DATE_EPOCH pins.
    let xdv = std::fs::read(dir.join("smoke.xdv")).unwrap();
    assert_eq!(xdv.len(), 164);
    assert_eq!(&xdv[..2], &[0xf7, 0x07]);
    assert_eq!(&xdv[15..44], b" XeTeX output 1970.01.01:0000");
    assert!(xdv.ends_with(&[0xdf, 0xdf, 0xdf, 0xdf]));
    let _ = std::fs::remove_dir_all(&dir);
}
