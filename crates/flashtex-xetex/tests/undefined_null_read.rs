//! #2391: under `\XeTeXinterwordspaceshaping`, two native words of a font
//! with only an explicit `\hskip` between them, before any space has been
//! typed in that font, make xetex.web read `width(font_glue[f])` with
//! `font_glue[f]=null`. TeX Live 2026's xetex crashes there (signal 11 on
//! macOS arm64); the port panicked on the checked index (exit 101).
//!
//! DESIGN.md section 13 (Commander ruling on #2391): FlashTeX never crashes.
//! The run stops as a fatal error (changes/undefined.ch): the log written so
//! far and the page already shipped are kept, the log says
//! "FlashTeX: internal error avoided (TeX Live xetex crashes here)", and the
//! exit status is non-zero. The font is Latin Modern Roman, the copy apps/mac
//! bundles; the fuzzer's case used Norasi, but any OpenType font reaches it.

use std::path::{Path, PathBuf};
use std::process::Command;

const INPUT: &str = r#"\catcode`\{=1 \catcode`\}=2
\shipout\hbox{}
\message{before-the-null-read}
\XeTeXinterwordspaceshaping=1
\font\x="[lmroman10-regular.otf]"
\setbox12=\vbox{\hsize=150pt \x abc\hskip0pt def\par}\showbox12
\end
"#;

fn run_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("flashtex-xetex-nullread-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn null_font_glue_is_a_fatal_error_not_a_crash() {
    let dir = run_dir();
    let font =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/lmroman10-regular.otf");
    std::fs::copy(&font, dir.join("lmroman10-regular.otf")).unwrap();
    std::fs::write(dir.join("nr.tex"), INPUT).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_flashtex-xetex"))
        .current_dir(&dir)
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_RESOLVER", "cwd")
        .args([
            "-ini",
            "-etex",
            "-no-pdf",
            "-interaction=nonstopmode",
            "nr.tex",
        ])
        .output()
        .expect("run flashtex-xetex");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // A fatal error, not a panic (101) and not a signal (None).
    let code = out.status.code();
    assert!(
        matches!(code, Some(c) if c != 0 && c != 101),
        "status {code:?}\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let log = std::fs::read_to_string(dir.join("nr.log")).unwrap();
    assert!(log.contains("before-the-null-read"), "log:\n{log}");
    assert!(log.contains("! Emergency stop."), "log:\n{log}");
    assert!(
        log.contains("FlashTeX: internal error avoided (TeX Live xetex crashes here)"),
        "log:\n{log}"
    );
    // The page shipped before the null read is kept.
    assert!(
        log.contains("Output written on nr.xdv (1 page"),
        "log:\n{log}"
    );
    assert!(dir.join("nr.xdv").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
