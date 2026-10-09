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
use std::path::Path;

/// The generated setup and what to say about it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Setup {
    /// TeX code for the start of the first line (empty: nothing to do).
    pub code: String,
    /// `(severity, message)`, one per role applied or refused, and the
    /// manifest's own warnings.
    pub notes: Vec<(&'static str, String)>,
}

/// A family name is passed inside a TeX group: no character that TeX
/// would read as anything but a letter or other character there.
fn safe(name: &str) -> bool {
    !name.trim().is_empty()
        && !name
            .chars()
            .any(|c| c.is_control() || "\\{}%#$&^_~".contains(c))
}

/// The setup for a project at `root` compiled with `format` (only the
/// LaTeX format has the hooks).
pub fn setup(root: &Path, format: &str) -> Setup {
    let mut s = Setup::default();
    if format != "xelatex" {
        return s;
    }
    let Some(path) = Manifest::locate(root) else {
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
        if !safe(name) {
            s.notes.push((
                "warning",
                format!("flashtex.toml: fonts.{role} {name:?} has a character TeX cannot take in a font name; not applied"),
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
        if !safe(name) {
            s.notes.push((
                "warning",
                format!("flashtex.toml: fonts.math {name:?} has a character TeX cannot take in a font name; not applied"),
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
    fn unsafe_names_are_refused() {
        let d = project("[fonts]\nsans = \"X}\\\\input{/etc/passwd\"\n");
        let s = setup(&d, "xelatex");
        assert!(s.code.is_empty());
        assert_eq!(s.notes[0].0, "warning");
        let _ = std::fs::remove_dir_all(&d);
    }
}
