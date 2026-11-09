//! First-slice XeTeX intrinsic observations on real LaTeX packages.
//! The report is separate from TeX output; no mode can replay.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const DOC: &str = r#"\documentclass{article}
\usepackage{siunitx}
\usepackage[unicode]{hyperref}
\makeatletter
\def\測試{A}
\g@addto@macro\pdfstringdefPreHook{\測試}
\makeatother
\begin{document}
\pdfstringdefPreHook
\pdfstringdefPreHook
\def\測試{B}
\pdfstringdefPreHook
{\globaldefs=1 \pdfstringdefPreHook}
\afterassignment\relax \pdfstringdefPreHook
\tracingmacros=2 \pdfstringdefPreHook \tracingmacros=0
\section{A \SI{1}{\metre}}\label{a}
\end{document}
"#;

struct Run {
    log: String,
    xdv: Vec<u8>,
    stats: String,
}

fn run(exe: &Path, dir: &Path, format: &Path, mode: Option<&str>) -> Run {
    // Each mode sees the same first-pass document, with no previous aux data.
    for name in ["main.aux", "main.out", "main.xdv"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    let stats = dir.join(format!("stats-{}.txt", mode.unwrap_or("default")));
    let mut cmd = Command::new(exe);
    cmd.current_dir(dir)
        .args([
            format!("-fmt={}", format.join("xelatex").display()),
            "-no-pdf".into(),
            "-interaction=nonstopmode".into(),
            "-halt-on-error".into(),
            "main.tex".into(),
        ])
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env("FLASHTEX_INTRINSICS_STATS", &stats)
        .stdin(Stdio::null());
    use std::os::unix::process::CommandExt;
    cmd.arg0("xelatex");
    if let Some(mode) = mode {
        cmd.env("FLASHTEX_INTRINSICS", mode);
    } else {
        cmd.env_remove("FLASHTEX_INTRINSICS");
    }
    let out = cmd.output().expect("run XeTeX");
    let log = std::fs::read_to_string(dir.join("main.log")).expect("main.log");
    assert!(
        out.status.success(),
        "{mode:?}: status {}; log tail: {}\nstderr: {}",
        out.status,
        log.lines()
            .rev()
            .take(20)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" / "),
        String::from_utf8_lossy(&out.stderr)
    );
    Run {
        log,
        xdv: std::fs::read(dir.join("main.xdv")).expect("main.xdv"),
        stats: std::fs::read_to_string(stats).unwrap_or_default(),
    }
}

fn count(stats: &str, key: &str) -> u64 {
    stats
        .lines()
        .find_map(|l| l.split_once(": ").filter(|(k, _)| *k == key))
        .and_then(|(_, n)| n.parse().ok())
        .unwrap_or(0)
}

#[test]
fn packages_and_guards_are_observed_without_replay() {
    if flashtex_engine::resolver::discover_texlive().is_none() {
        if std::env::var_os("FLASHTEX_REQUIRE_TEXLIVE").is_some_and(|v| v == "1") {
            panic!("no TeX Live found with FLASHTEX_REQUIRE_TEXLIVE=1");
        }
        eprintln!("skipped: no TeX Live");
        return;
    }
    let exe = Path::new(env!("CARGO_BIN_EXE_flashtex-xetex"));
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "flashtex-xetex-intr-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.tex"), DOC).unwrap();
    std::env::set_var(
        "FLASHTEX_FORMAT_CACHE_DIR",
        std::env::temp_dir().join("flashtex-xetex-intr-format-cache"),
    );
    let children = flashtex_xetex::host::proc::Children::new(exe).unwrap();
    let format = flashtex_xetex::host::format::ensure(exe, "xelatex", &children)
        .expect("build FlashTeX xelatex format");
    std::env::remove_var("FLASHTEX_FORMAT_CACHE_DIR");

    let default = run(exe, &dir, &format, None);
    let off = run(exe, &dir, &format, Some("off"));
    let on = run(exe, &dir, &format, Some("on"));
    let verify = run(exe, &dir, &format, Some("verify"));
    let report = run(exe, &dir, &format, Some("report"));
    for other in [&off, &on, &verify, &report] {
        if default.log != other.log {
            let first = default
                .log
                .lines()
                .zip(other.log.lines())
                .enumerate()
                .find(|(_, (a, b))| a != b);
            panic!("log changed at {first:?}");
        }
        assert_eq!(default.xdv, other.xdv, "XDV changed");
    }
    assert!(default.stats.is_empty() && off.stats.is_empty() && on.stats.is_empty());
    assert!(
        verify.stats.contains("replay_enabled: false"),
        "{}",
        verify.stats
    );
    assert!(verify.stats.contains("semantic_differences: not_checked"));
    for key in [
        "eligible",
        "unicode_dependency_tokens",
        "alternative.siunitx_context_calls",
        "alternative.bookmark_copy_calls",
        "rejected.DependenciesChanged",
        "rejected.GlobalDefs",
        "rejected.AfterAssignment",
        "rejected.Tracing",
    ] {
        assert!(count(&verify.stats, key) > 0, "{key}: {}", verify.stats);
    }
    assert_eq!(verify.stats, report.stats);
    eprintln!("XeTeX intrinsics verification:\n{}", verify.stats);
    std::fs::remove_dir_all(dir).unwrap();
}
