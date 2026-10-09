//! Which mode, and so which engine host, typesets the project
//! (docs/design/modes/PROPOSAL.md §4.1), highest first:
//!
//! 1. `--mode classic|unicode` on the command line;
//! 2. `FLASHTEX_MODE`;
//! 3. the manifest's `[project] mode` (`flashtex.toml`);
//! 4. a `% !TEX program = xelatex` (or `TS-program`) line among the main
//!    file's leading comments, the TeXShop/TeXstudio/VS Code convention:
//!    `xelatex` is Unicode, `xetex` Unicode with plain XeTeX's format,
//!    `pdflatex` Classic;
//! 5. Classic.
//!
//! Classic runs `flashtex-host` with the `pdflatex` format; Unicode runs
//! `flashtex-host-unicode` with `xelatex` (owner, Q10: two programs). The
//! opt-in FlashTeX mode (PROPOSAL.md §10) is not built yet: asking for it
//! is an error, never a silent Classic run. A program FlashTeX does not
//! have (LuaLaTeX) or a manifest value it does not know is a warning that
//! says what runs instead.

use flashtex_project_manifest::{Manifest, Mode as ManifestMode, FILE_NAME};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Classic,
    Unicode,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Classic => "classic",
            Mode::Unicode => "unicode",
        }
    }

    /// The host program's file name.
    pub fn host(self) -> &'static str {
        match self {
            Mode::Classic => "flashtex-host",
            Mode::Unicode => "flashtex-host-unicode",
        }
    }

    /// The environment variable naming the host program.
    pub fn host_env(self) -> &'static str {
        match self {
            Mode::Classic => "FLASHTEX_HOST",
            Mode::Unicode => "FLASHTEX_HOST_UNICODE",
        }
    }

    /// The format a compile asks for by default.
    pub fn format(self) -> &'static str {
        match self {
            Mode::Classic => "pdflatex",
            Mode::Unicode => "xelatex",
        }
    }
}

/// The mode, its format, what decided it (for messages) and what to warn
/// about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub mode: Mode,
    pub format: &'static str,
    pub source: String,
    pub warnings: Vec<String>,
}

impl Resolved {
    fn of(mode: Mode, source: impl Into<String>) -> Resolved {
        Resolved {
            mode,
            format: mode.format(),
            source: source.into(),
            warnings: vec![],
        }
    }
}

fn parse(s: &str, what: &str) -> Result<Mode, String> {
    match s {
        "classic" => Ok(Mode::Classic),
        "unicode" => Ok(Mode::Unicode),
        "flashtex" => Err(format!(
            "{what}: the FlashTeX mode is not available in this version (classic or unicode)"
        )),
        other => Err(format!(
            "{what}: unknown mode {other:?} (classic or unicode)"
        )),
    }
}

/// What a `% !TEX program` line says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Program {
    /// A program FlashTeX has: its mode and format.
    Known(Mode, &'static str),
    /// A program it does not have (`lualatex`, anything else), as written.
    Unsupported(String),
}

/// `% !TEX program = NAME` (or `TS-program`; `!TeX`, `! TeX`, either
/// case) among the leading comment lines of `text` (after a byte-order
/// mark): the program it names, if a line names one.
pub fn magic_comment(text: &str) -> Option<Program> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for line in text.lines().take(20) {
        let l = line.trim_start();
        if !l.starts_with('%') {
            if l.is_empty() {
                continue;
            }
            break;
        }
        let body = l.trim_start_matches('%').trim_start().to_ascii_lowercase();
        let Some(rest) = body.strip_prefix('!') else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix("tex") else {
            continue;
        };
        let Some((key, value)) = rest.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key != "program" && key != "ts-program" {
            continue;
        }
        let value = value.trim();
        return Some(match value {
            "xelatex" => Program::Known(Mode::Unicode, "xelatex"),
            "xetex" => Program::Known(Mode::Unicode, "xetex"),
            "pdflatex" | "latex" => Program::Known(Mode::Classic, "pdflatex"),
            other => Program::Unsupported(other.to_string()),
        });
    }
    None
}

/// The manifest governing a project at `root`, as the Unicode host finds
/// it for `[fonts]`: `flashtex.toml` in it or above, the search ending at a
/// repository's root (`.git`, `.hg`, `.svn`), never in or above the home
/// directory (a stray one there governs nothing), and, on Unix, not in a
/// directory this user does not own or others may write (`/tmp`).
pub fn locate_manifest(root: &Path) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from);
    locate_below(root, home.as_deref())
}

fn locate_below(root: &Path, home: Option<&Path>) -> Option<PathBuf> {
    let mut dir = root.to_path_buf();
    loop {
        if home.is_some_and(|h| h.starts_with(&dir)) {
            return None;
        }
        if !owned(&dir) {
            return None;
        }
        let candidate = dir.join(FILE_NAME);
        if candidate.is_file() && owned(&candidate) {
            return Some(candidate);
        }
        if [".git", ".hg", ".svn"].iter().any(|v| dir.join(v).exists()) || !dir.pop() {
            return None;
        }
    }
}

/// Whether `p` is this user's and others may not write it.
#[cfg(unix)]
fn owned(p: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: getuid(2) has no preconditions.
    let uid = unsafe { libc::getuid() };
    std::fs::metadata(p).is_ok_and(|m| m.uid() == uid && m.mode() & 0o002 == 0)
}

#[cfg(not(unix))]
fn owned(p: &Path) -> bool {
    p.exists()
}

/// The project's mode (`root`, `main` as the CLI resolved them).
pub fn resolve(
    flag: Option<&str>,
    env: Option<String>,
    root: &Path,
    main: &str,
) -> Result<Resolved, String> {
    if let Some(f) = flag {
        return Ok(Resolved::of(parse(f, "--mode")?, "--mode"));
    }
    if let Some(e) = env.filter(|e| !e.is_empty()) {
        return Ok(Resolved::of(parse(&e, "FLASHTEX_MODE")?, "FLASHTEX_MODE"));
    }
    let magic = std::fs::read_to_string(root.join(main))
        .ok()
        .and_then(|t| magic_comment(&t));
    if let Some(path) = locate_manifest(root) {
        let loaded = Manifest::load(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(m) = loaded.manifest.project.mode {
            let mode = match m {
                ManifestMode::Classic => Mode::Classic,
                ManifestMode::Unicode => Mode::Unicode,
                ManifestMode::FlashTeX => {
                    return Err(format!(
                        "{}: mode = \"flashtex\" needs a FlashTeX that implements it (flashtex_version {}); this one does not",
                        path.display(),
                        loaded.manifest.project.flashtex_version.as_deref().unwrap_or("unset")
                    ))
                }
            };
            return Ok(Resolved::of(mode, path.display().to_string()));
        }
        // A value this version does not know: the manifest meant to choose,
        // so Classic, said, and the `% !TEX` line does not overrule it.
        if let Some(w) = loaded.warnings.iter().find(|w| w.key == "project.mode") {
            let mut r = Resolved::of(Mode::Classic, path.display().to_string());
            r.warnings.push(format!("{}: {w}", path.display()));
            if let Some(Program::Known(m, _)) = &magic {
                if *m != Mode::Classic {
                    r.warnings.push(format!(
                        "the `% !TEX program` line in {main} is not used: flashtex.toml sets the mode"
                    ));
                }
            }
            return Ok(r);
        }
    }
    match magic {
        Some(Program::Known(mode, format)) => {
            let mut r = Resolved::of(mode, format!("% !TEX program in {main}"));
            r.format = format;
            Ok(r)
        }
        Some(Program::Unsupported(p)) => {
            let mut r = Resolved::of(Mode::Classic, "default");
            r.warnings.push(if p.starts_with("lua") {
                format!(
                    "{main}: `% !TEX program = {p}`: LuaTeX isn't supported; using Classic (pdfTeX). For OpenType fonts use Unicode mode (`% !TEX program = xelatex`)"
                )
            } else {
                format!(
                    "{main}: `% !TEX program = {p}` is not a program FlashTeX has; using Classic (pdfTeX)"
                )
            });
            Ok(r)
        }
        None => Ok(Resolved::of(Mode::Classic, "default")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(t: &str) -> Option<(Mode, &'static str)> {
        match magic_comment(t)? {
            Program::Known(m, f) => Some((m, f)),
            Program::Unsupported(_) => None,
        }
    }

    #[test]
    fn magic_comments() {
        let u = Some((Mode::Unicode, "xelatex"));
        assert_eq!(known("% !TEX program = xelatex\n\\documentclass{x}"), u);
        assert_eq!(known("%!TEX TS-program = XeLaTeX\n"), u);
        assert_eq!(
            known("\u{feff}% !TEX program = xelatex\n"),
            u,
            "after a BOM"
        );
        assert_eq!(
            known("% ! TeX program = xelatex\n"),
            u,
            "VS Code's spelling"
        );
        assert_eq!(
            known("% !TeX program=pdflatex\n"),
            Some((Mode::Classic, "pdflatex"))
        );
        assert_eq!(
            known("% !TEX program = xetex\n"),
            Some((Mode::Unicode, "xetex"))
        );
        assert_eq!(
            known("% a comment\n% !TEX program = pdflatex\n"),
            Some((Mode::Classic, "pdflatex"))
        );
        assert_eq!(
            magic_comment("\\documentclass{x}\n% !TEX program = xelatex\n"),
            None
        );
        assert_eq!(
            magic_comment("% !TEX program = lualatex\n"),
            Some(Program::Unsupported("lualatex".into()))
        );
        assert_eq!(magic_comment("% !TEX root = main.tex\n"), None);
    }

    #[test]
    fn the_order_of_the_sources() {
        let d = std::env::temp_dir().join(format!("ftx-mode-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(".git"), "").unwrap(); // stop the manifest search here
        std::fs::write(d.join("main.tex"), "% !TEX program = xelatex\n").unwrap();
        let r = |flag: Option<&str>, env: Option<&str>| {
            resolve(flag, env.map(String::from), &d, "main.tex")
        };
        assert_eq!(r(None, None).unwrap().mode, Mode::Unicode);
        std::fs::write(d.join("flashtex.toml"), "[project]\nmode = \"classic\"\n").unwrap();
        assert_eq!(r(None, None).unwrap().mode, Mode::Classic);
        assert_eq!(r(None, Some("unicode")).unwrap().mode, Mode::Unicode);
        assert_eq!(
            r(Some("classic"), Some("unicode")).unwrap().mode,
            Mode::Classic
        );
        assert!(r(Some("flashtex"), None).is_err());
        std::fs::write(d.join("flashtex.toml"), "[project]\nmode = \"flashtex\"\n").unwrap();
        assert!(r(None, None).is_err());
        // an unknown manifest value: Classic, said, the line not consulted
        std::fs::write(d.join("flashtex.toml"), "[project]\nmode = \"lualatex\"\n").unwrap();
        let x = r(None, None).unwrap();
        assert_eq!(x.mode, Mode::Classic);
        assert_eq!(x.warnings.len(), 2, "{:?}", x.warnings);
        std::fs::remove_file(d.join("flashtex.toml")).unwrap();
        std::fs::write(d.join("main.tex"), "% !TEX program = lualatex\n").unwrap();
        let x = r(None, None).unwrap();
        assert_eq!(x.mode, Mode::Classic);
        assert!(
            x.warnings[0].contains("LuaTeX isn't supported"),
            "{:?}",
            x.warnings
        );
        std::fs::write(d.join("main.tex"), "% !TEX program = xetex\n").unwrap();
        assert_eq!(r(None, None).unwrap().format, "xetex");
        std::fs::write(d.join("main.tex"), "\\documentclass{article}\n").unwrap();
        let def = r(None, None).unwrap();
        assert_eq!((def.mode, def.source.as_str()), (Mode::Classic, "default"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn no_manifest_from_home_or_a_shared_directory() {
        use std::os::unix::fs::PermissionsExt;
        let d = std::env::temp_dir().join(format!("ftx-mode-shared-{}", std::process::id()));
        let inner = d.join("doc");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(d.join("flashtex.toml"), "[project]\nmode = \"unicode\"\n").unwrap();
        assert!(locate_manifest(&inner).is_some());
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(
            locate_manifest(&inner).is_none(),
            "a world-writable directory's"
        );
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(
            locate_below(&inner, Some(&d)).is_none(),
            "the home directory's"
        );
        assert!(
            locate_below(&inner, Some(&inner)).is_none(),
            "above the home directory"
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
