//! `[fonts]` in Unicode mode (docs/design/modes/PROPOSAL.md §4.5, owner
//! ruling Q5): the project manifest's `text`, `sans`, `mono` and `math`
//! roles become the equivalent `fontspec`/`unicode-math` setup, passed to
//! the engine ahead of the main file on its first line, the way
//! `xelatex '\AddToHook…\input{main.tex}'` would take it, so `xelatex` run
//! on that same line is still the oracle.
//!
//! - Text roles: LaTeX's `class/after` hook loads `fontspec` and sets the
//!   roles, so the class's defaults are replaced and the document's own
//!   `\setmainfont`/`\setsansfont`/`\setmonofont` (later) and a local
//!   `\fontspec` group still win.
//! - Math: at `begindocument/before`, and only when the document has not
//!   loaded `unicode-math` itself (whose own choice, Latin Modern Math by
//!   default, then stands), so `amsmath` loaded in the preamble comes first.
//! - Visible: each role applied is a line in the log
//!   (`FlashTeX [fonts]: \setmainfont{TeX Gyre Pagella}`) and an `info`
//!   diagnostic; a name that could not be passed safely is a warning and
//!   is not applied.

use flashtex_project_manifest::Manifest;
use std::path::{Path, PathBuf};

/// The generated setup and what to say about it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Setup {
    /// TeX code for the start of the first line (empty: nothing to do).
    pub code: String,
    /// `(severity, message)`, one per role applied or refused, and the
    /// manifest's own warnings.
    pub notes: Vec<(&'static str, String)>,
}

/// The longest family name passed on (a font's names are far shorter;
/// TeX's first line is a buffer of its own).
pub const MAX_NAME: usize = 256;

/// Why a family name is not passed inside a TeX group, if it is not: a
/// character TeX would read as anything but a letter or other character
/// there, or a name longer than [`MAX_NAME`] bytes.
fn refused(name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return Some("is empty".into());
    }
    if name.len() > MAX_NAME {
        return Some(format!("is {} bytes long (at most {MAX_NAME})", name.len()));
    }
    name.chars()
        .any(|c| c.is_control() || "\\{}%#$&^_~".contains(c))
        .then(|| "has a character TeX cannot take in a font name".into())
}

/// The manifest governing a project at `root`: `flashtex.toml` in it or in
/// a directory above, the search ending at a repository's root (`.git`,
/// `.hg`, `.svn`), at the home directory (never read: a stray one there
/// governs nothing, and the app compiles a copy of the project under its
/// caches, with no repository to end the search), and at a directory this
/// user does not own or that others may write (`/tmp`, a shared folder:
/// anyone could put a manifest there). The file itself must be this
/// user's and not writable by others.
pub fn locate(root: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    // SAFETY: getuid(2) has no preconditions.
    let uid = unsafe { libc::getuid() };
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from);
    let mut dir = root.to_path_buf();
    loop {
        if home.as_deref().is_some_and(|h| h.starts_with(&dir)) {
            return None;
        }
        let m = std::fs::metadata(&dir).ok()?;
        if m.uid() != uid || m.mode() & 0o002 != 0 {
            return None;
        }
        let candidate = dir.join(flashtex_project_manifest::FILE_NAME);
        if let Ok(f) = std::fs::metadata(&candidate) {
            if f.is_file() && f.uid() == uid && f.mode() & 0o002 == 0 {
                return Some(candidate);
            }
        }
        if [".git", ".hg", ".svn"].iter().any(|v| dir.join(v).exists()) || !dir.pop() {
            return None;
        }
    }
}

/// The setup for a project at `root` compiled with `format` (only the
/// LaTeX format has the hooks).
pub fn setup(root: &Path, format: &str) -> Setup {
    let mut s = Setup::default();
    if format != "xelatex" {
        return s;
    }
    let Some(path) = locate(root) else {
        return s;
    };
    let loaded = match Manifest::load(&path) {
        Ok(l) => l,
        Err(e) => {
            s.notes.push((
                "warning",
                format!("{}: {e}; [fonts] not applied", path.display()),
            ));
            return s;
        }
    };
    // the manifest's other keys are the client's to report
    for w in loaded
        .warnings
        .iter()
        .filter(|w| w.key == "fonts" || w.key.starts_with("fonts."))
    {
        s.notes.push(("warning", format!("flashtex.toml: {w}")));
    }
    let f = &loaded.manifest.fonts;
    let mut text = String::new();
    for (role, cmd, name) in [
        ("text", "setmainfont", &f.text),
        ("sans", "setsansfont", &f.sans),
        ("mono", "setmonofont", &f.mono),
    ] {
        let Some(name) = name else { continue };
        if let Some(why) = refused(name) {
            let shown: String = name.chars().take(64).collect();
            s.notes.push((
                "warning",
                format!("flashtex.toml: fonts.{role} {shown:?} {why}; not applied"),
            ));
            continue;
        }
        let name = name.trim();
        text.push_str(&format!(
            "\\{cmd}{{{name}}}\\typeout{{FlashTeX [fonts]: \\string\\{cmd}{{{name}}}}}"
        ));
        s.notes
            .push(("info", format!("FlashTeX [fonts]: \\{cmd}{{{name}}}")));
    }
    if !text.is_empty() {
        s.code.push_str(&format!(
            "\\AddToHook{{class/after}}[flashtex-fonts]{{\\RequirePackage{{fontspec}}{text}}}"
        ));
    }
    if let Some(name) = &f.math {
        if let Some(why) = refused(name) {
            let shown: String = name.chars().take(64).collect();
            s.notes.push((
                "warning",
                format!("flashtex.toml: fonts.math {shown:?} {why}; not applied"),
            ));
        } else {
            let name = name.trim();
            s.code.push_str(&format!(
                "\\AddToHook{{begindocument/before}}[flashtex-fonts]{{\\IfPackageLoadedTF{{unicode-math}}{{}}{{\\RequirePackage{{unicode-math}}\\setmathfont{{{name}}}\\typeout{{FlashTeX [fonts]: \\string\\setmathfont{{{name}}}}}}}}}"
            ));
            s.notes.push((
                "info",
                format!("FlashTeX [fonts]: \\setmathfont{{{name}}} (unless the document loads unicode-math)"),
            ));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(toml: &str) -> std::path::PathBuf {
        let d =
            std::env::temp_dir().join(format!("ftx-fonts-{}-{}", std::process::id(), toml.len()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(".git"), "").unwrap();
        std::fs::write(d.join("flashtex.toml"), toml).unwrap();
        d
    }

    #[test]
    fn roles_become_hooks() {
        let d = project("[fonts]\ntext = \"TeX Gyre Pagella\"\nmath = \"Latin Modern Math\"\n");
        let s = setup(&d, "xelatex");
        assert_eq!(
            s.code,
            "\\AddToHook{class/after}[flashtex-fonts]{\\RequirePackage{fontspec}\\setmainfont{TeX Gyre Pagella}\\typeout{FlashTeX [fonts]: \\string\\setmainfont{TeX Gyre Pagella}}}\
             \\AddToHook{begindocument/before}[flashtex-fonts]{\\IfPackageLoadedTF{unicode-math}{}{\\RequirePackage{unicode-math}\\setmathfont{Latin Modern Math}\\typeout{FlashTeX [fonts]: \\string\\setmathfont{Latin Modern Math}}}}"
        );
        assert_eq!(s.notes.len(), 2);
        assert!(
            setup(&d, "xetex").code.is_empty(),
            "plain XeTeX has no hooks"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn long_names_are_refused() {
        let long = "A".repeat(MAX_NAME + 1);
        let d = project(&format!("[fonts]\ntext = \"{long}\"\n"));
        let s = setup(&d, "xelatex");
        assert!(s.code.is_empty());
        assert!(s.notes[0].1.contains("bytes long"), "{:?}", s.notes);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn no_manifest_from_a_shared_directory() {
        // a directory others may write (as /tmp): its manifest is not read
        use std::os::unix::fs::PermissionsExt;
        let d = project("[fonts]\ntext = \"X\"\n");
        std::fs::remove_file(d.join(".git")).unwrap();
        let inner = d.join("doc");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(
            locate(&inner).is_some(),
            "found above, in an owned directory"
        );
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(locate(&inner).is_none(), "not in a world-writable one");
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o755)).unwrap();
        // nor a manifest others may write, in an owned directory
        let toml = d.join("flashtex.toml");
        std::fs::set_permissions(&toml, std::fs::Permissions::from_mode(0o666)).unwrap();
        assert!(locate(&inner).is_none(), "not a world-writable file");
        std::fs::set_permissions(&toml, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(locate(&inner).is_some());
        std::fs::write(inner.join(".git"), "").unwrap();
        assert!(locate(&inner).is_none(), "nor above a repository's root");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn unsafe_names_are_refused() {
        let d = project("[fonts]\nsans = \"X}\\\\input{/etc/passwd\"\n");
        let s = setup(&d, "xelatex");
        assert!(s.code.is_empty());
        assert_eq!(s.notes[0].0, "warning");
        let _ = std::fs::remove_dir_all(&d);
    }
}
