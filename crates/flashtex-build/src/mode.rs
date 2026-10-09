//! Which mode, and so which engine host, typesets the project
//! (docs/design/modes/PROPOSAL.md §4.1), highest first:
//!
//! 1. `--mode classic|unicode` on the command line;
//! 2. `FLASHTEX_MODE`;
//! 3. the manifest's `[project] mode` (`flashtex.toml`);
//! 4. a `% !TEX program = xelatex` (or `TS-program`) line at the top of the
//!    main file, the TeXShop/TeXstudio convention: `xelatex`/`xetex` is
//!    Unicode, `pdflatex`/`pdftex` Classic;
//! 5. Classic.
//!
//! Classic runs `flashtex-host` with the `pdflatex` format; Unicode runs
//! `flashtex-host-unicode` with `xelatex` (owner, Q10: two programs). The
//! opt-in FlashTeX mode (PROPOSAL.md §10) is not built yet: asking for it
//! is an error, never a silent Classic run.

use flashtex_project_manifest::{Manifest, Mode as ManifestMode};
use std::path::Path;

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

    /// The format a compile asks for.
    pub fn format(self) -> &'static str {
        match self {
            Mode::Classic => "pdflatex",
            Mode::Unicode => "xelatex",
        }
    }
}

/// The mode and what decided it (for messages).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub mode: Mode,
    pub source: String,
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

/// `% !TEX program = NAME` (or `% !TEX TS-program = NAME`) in the first
/// lines of `text`: the mode it names, if it names one.
pub fn magic_comment(text: &str) -> Option<Mode> {
    for line in text.lines().take(20) {
        let l = line.trim_start();
        if !l.starts_with('%') {
            if l.is_empty() {
                continue;
            }
            break;
        }
        // TeXShop writes `!TEX`, TeXstudio `!TeX`: either case
        let body = l.trim_start_matches('%').trim_start().to_ascii_lowercase();
        let Some(rest) = body.strip_prefix("!tex") else {
            continue;
        };
        let Some((key, value)) = rest.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if key != "program" && key != "ts-program" {
            continue;
        }
        return match value.trim().to_ascii_lowercase().as_str() {
            "xelatex" | "xetex" => Some(Mode::Unicode),
            "pdflatex" | "pdftex" | "latex" => Some(Mode::Classic),
            _ => None,
        };
    }
    None
}

/// The project's mode (`root`, `main` as the CLI resolved them).
pub fn resolve(
    flag: Option<&str>,
    env: Option<String>,
    root: &Path,
    main: &str,
) -> Result<Resolved, String> {
    if let Some(f) = flag {
        return Ok(Resolved {
            mode: parse(f, "--mode")?,
            source: "--mode".into(),
        });
    }
    if let Some(e) = env.filter(|e| !e.is_empty()) {
        return Ok(Resolved {
            mode: parse(&e, "FLASHTEX_MODE")?,
            source: "FLASHTEX_MODE".into(),
        });
    }
    if let Some(path) = Manifest::locate(root) {
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
            return Ok(Resolved {
                mode,
                source: format!("{}", path.display()),
            });
        }
    }
    if let Some(m) = std::fs::read_to_string(root.join(main))
        .ok()
        .and_then(|t| magic_comment(&t))
    {
        return Ok(Resolved {
            mode: m,
            source: format!("% !TEX program in {main}"),
        });
    }
    Ok(Resolved {
        mode: Mode::Classic,
        source: "default".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_comments() {
        assert_eq!(
            magic_comment("% !TEX program = xelatex\n\\documentclass{x}"),
            Some(Mode::Unicode)
        );
        assert_eq!(
            magic_comment("%!TEX TS-program = XeLaTeX\n"),
            Some(Mode::Unicode)
        );
        assert_eq!(
            magic_comment("% !TeX program=pdflatex\n"),
            Some(Mode::Classic)
        );
        assert_eq!(
            magic_comment("% a comment\n% !TEX program = pdflatex\n"),
            Some(Mode::Classic)
        );
        assert_eq!(
            magic_comment("\\documentclass{x}\n% !TEX program = xelatex\n"),
            None
        );
        assert_eq!(magic_comment("% !TEX program = lualatex\n"), None);
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
        std::fs::remove_file(d.join("flashtex.toml")).unwrap();
        std::fs::write(d.join("main.tex"), "\\documentclass{article}\n").unwrap();
        let def = r(None, None).unwrap();
        assert_eq!((def.mode, def.source.as_str()), (Mode::Classic, "default"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
