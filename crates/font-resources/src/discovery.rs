//! Where FlashTeX looks for fonts and TeX font metrics: the override
//! variables, the app bundle's and CLI tarball's texmf trees, then a host
//! TeX installation's. Moved from `render-pipeline/src/fonts.rs` (PLAN3 S1)
//! so the compiler's `\settowidth` measurer and the render pipeline find
//! the same `.tfm` files; the rules are unchanged.

use std::path::{Path, PathBuf};

/// Where the digest-bound 12 pt Latin Modern metrics live, relative to a
/// texmf root.
pub const REQUIRED_TFM_DIR: &str = "fonts/tfm/public/lm";

/// Where TeX Live keeps the EC metrics (`jknappen/ec`), relative to a
/// texmf root.
pub const EC_TFM_DIR: &str = "fonts/tfm/jknappen/ec";

/// Where TeX Live keeps the Euler (`eufm`) metrics `\mathfrak` uses.
pub const AMS_EULER_TFM_DIR: &str = "fonts/tfm/public/amsfonts/euler";

/// Where TeX Live keeps Knuth's Computer Modern metrics (`public/cm`),
/// relative to a texmf root; the bundled tree ships the files
/// [`ot1_tfm_file`] can name under the same path.
pub const CM_TFM_DIR: &str = "fonts/tfm/public/cm";

/// Default search directories, probed in order for explicit file names.
/// `FLASHTEX_FONT_DIRS` (colon separated) is prepended when set.
pub const DEFAULT_FONT_DIRS: [&str; 12] = [
    // MacTeX / BasicTeX (TeX Live 2026, 2025).
    "/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/lm",
    "/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/lm-math",
    "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm",
    "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/lm-math",
    "/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/lm",
    "/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/lm-math",
    "/usr/local/texlive/2025basic/texmf-dist/fonts/opentype/public/lm",
    "/usr/local/texlive/2025basic/texmf-dist/fonts/opentype/public/lm-math",
    // Debian/Ubuntu `fonts-lmodern` and TeX Live packages.
    "/usr/share/texmf/fonts/opentype/public/lm",
    "/usr/share/texmf/fonts/opentype/public/lm-math",
    "/usr/share/texlive/texmf-dist/fonts/opentype/public/lm",
    "/usr/share/texlive/texmf-dist/fonts/opentype/public/lm-math",
];

/// The CLI tarball's data directory relative to the executable:
/// `bin/flashtex` finds `share/flashtex/{Fonts,texmf}` (FHS-style layout,
/// `flashtex-cli-<version>-<platform>.tar.gz`).
pub const SHARE_DIR: &str = "../share/flashtex";

/// What font discovery reads from the process: the three override
/// variables and where the executable lives. [`Discovery::from_process`]
/// samples the real process; tests build one by hand so the bundle-relative
/// rules are checked without a host TeX installation and without touching
/// the environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovery {
    /// `FLASHTEX_FONT_DIRS` (colon separated).
    pub font_dirs: Option<String>,
    /// `FLASHTEX_LM_DIR` (the pdf sibling's variable).
    pub lm_dir: Option<String>,
    /// `FLASHTEX_TFM_DIRS` (colon separated).
    pub tfm_dirs: Option<String>,
    /// The directory holding the executable (`Contents/MacOS` in an app
    /// bundle); `None` when the process cannot tell.
    pub exe_dir: Option<PathBuf>,
}

impl Discovery {
    pub fn from_process() -> Discovery {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        Discovery {
            font_dirs: var("FLASHTEX_FONT_DIRS"),
            lm_dir: var("FLASHTEX_LM_DIR"),
            tfm_dirs: var("FLASHTEX_TFM_DIRS"),
            // Symlinks resolved (`flashtex install-cli` links the binary
            // into /usr/local/bin; `current_exe` reports the link on
            // macOS), so the bundle and tarball layouts are found next to
            // the real file.
            exe_dir: std::env::current_exe()
                .ok()
                .map(|e| std::fs::canonicalize(&e).unwrap_or(e))
                .and_then(|e| e.parent().map(Path::to_path_buf)),
        }
    }

    pub fn split(v: &Option<String>) -> Vec<PathBuf> {
        v.iter().flat_map(|v| v.split(':')).filter(|s| !s.is_empty()).map(PathBuf::from).collect()
    }

    /// The texmf trees an app bundle or a sibling directory can ship,
    /// relative to the executable: `<exe>/../Resources/texmf` (the bundle's
    /// `Contents/Resources/texmf`, sealed with the app), `<exe>/texmf`, then
    /// `<exe>/../share/flashtex/texmf` (the CLI tarball: `bin/flashtex` next
    /// to `share/flashtex/`, see scripts/ci/package-cli.sh).
    /// Under each: `fonts/opentype/public/{lm,lm-math}`,
    /// `fonts/tfm/public/lm` and `doc/fonts/lm/GUST-FONT-LICENSE.TXT`.
    pub fn bundle_texmf_roots(&self) -> Vec<PathBuf> {
        self.exe_dir
            .iter()
            .flat_map(|d| [d.join("../Resources/texmf"), d.join("texmf"), d.join(SHARE_DIR).join("texmf")])
            .collect()
    }

    /// Font directories, in order: `FLASHTEX_FONT_DIRS`, `FLASHTEX_LM_DIR`,
    /// the bundled texmf trees' OpenType directories, a flat `Fonts`
    /// directory next to the executable, in the bundle's `Resources` or in
    /// the tarball's `share/flashtex`, then [`DEFAULT_FONT_DIRS`] (host
    /// TeX). Nothing is scanned outside this list; explicit overrides
    /// always come first.
    pub fn font_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Discovery::split(&self.font_dirs);
        dirs.extend(self.lm_dir.iter().map(PathBuf::from));
        for root in self.bundle_texmf_roots() {
            dirs.push(root.join("fonts/opentype/public/lm"));
            dirs.push(root.join("fonts/opentype/public/lm-math"));
        }
        if let Some(dir) = &self.exe_dir {
            dirs.push(dir.join("Fonts"));
            dirs.push(dir.join("../Resources/Fonts"));
            dirs.push(dir.join(SHARE_DIR).join("Fonts"));
        }
        dirs.extend(DEFAULT_FONT_DIRS.iter().map(PathBuf::from));
        dirs
    }

    /// TFM directories, in order: `FLASHTEX_TFM_DIRS`, the bundled texmf
    /// trees' `fonts/tfm/public/lm`, then for every font directory its
    /// `/opentype/` → `/tfm/` sibling (the TeX Live layout) and the
    /// directory itself (the flat layout). Duplicates are dropped, first
    /// occurrence wins, so an explicit override keeps precedence over the
    /// same path discovered later.
    pub fn tfm_dirs_for(&self, font_dirs: &[PathBuf]) -> Vec<PathBuf> {
        let mut dirs = Discovery::split(&self.tfm_dirs);
        let mut push = |p: PathBuf| {
            if !dirs.contains(&p) {
                dirs.push(p);
            }
        };
        for root in self.bundle_texmf_roots() {
            push(root.join(REQUIRED_TFM_DIR));
        }
        for d in font_dirs {
            push(PathBuf::from(d.to_string_lossy().replace("/opentype/", "/tfm/")));
            push(d.clone());
        }
        // The EC metrics of T1 `cmr` documents (`Family::ComputerModern`):
        // the bundled trees' and each TeX Live tree's `fonts/tfm/jknappen/ec`,
        // after every Latin Modern candidate so their order is unchanged.
        for root in self.bundle_texmf_roots() {
            push(root.join(EC_TFM_DIR));
        }
        for d in font_dirs {
            let d = d.to_string_lossy();
            if let Some(at) = d.find("/fonts/opentype/public/lm") {
                push(PathBuf::from(format!("{}/{EC_TFM_DIR}", &d[..at])));
            }
        }
        // The OT1 metrics of `cmr` documents without `fontenc`
        // (`Family::ComputerModernOt1`): `fonts/tfm/public/cm`, likewise.
        for root in self.bundle_texmf_roots() {
            push(root.join(CM_TFM_DIR));
        }
        for d in font_dirs {
            let d = d.to_string_lossy();
            if let Some(at) = d.find("/fonts/opentype/public/lm") {
                push(PathBuf::from(format!("{}/{CM_TFM_DIR}", &d[..at])));
            }
        }
        // ...and next to every explicit metrics directory of a TeX tree
        // (`FLASHTEX_TFM_DIRS` naming `<texmf>/fonts/tfm/jknappen/ec` or
        // `.../public/lm` finds `<texmf>/fonts/tfm/public/cm` without being
        // told), after the explicit ones so their order is unchanged.
        let explicit: Vec<PathBuf> = Discovery::split(&self.tfm_dirs);
        for d in &explicit {
            let d = d.to_string_lossy();
            if let Some(at) = d.find("/fonts/tfm/") {
                push(PathBuf::from(format!("{}/{CM_TFM_DIR}", &d[..at])));
            }
        }
        // `\mathfrak`'s `eufm` metrics (amsfonts `euler`), after the EC ones.
        for root in self.bundle_texmf_roots() {
            push(root.join(AMS_EULER_TFM_DIR));
        }
        for d in font_dirs {
            let d = d.to_string_lossy();
            if let Some(at) = d.find("/fonts/opentype/public/lm") {
                push(PathBuf::from(format!("{}/{AMS_EULER_TFM_DIR}", &d[..at])));
            }
        }
        // ...and next to every explicit metrics directory of a TeX tree, as
        // for `public/cm` above: the Mac app and CI name the bundled tree's
        // `public/lm`/`jknappen/ec` in `FLASHTEX_TFM_DIRS` and ship `eufm`
        // beside them. Without this only a host TeX Live's copy was found,
        // so `\mathfrak` differed between a Mac with MacTeX and one without.
        for d in &explicit {
            let d = d.to_string_lossy();
            if let Some(at) = d.find("/fonts/tfm/") {
                push(PathBuf::from(format!("{}/{AMS_EULER_TFM_DIR}", &d[..at])));
            }
        }
        dirs
    }
}

/// How diagnostics spell the executable's directory.
pub const EXE_DIR_LABEL: &str = "<executable-dir>";

/// A comma-separated directory list for diagnostics. Directories under
/// `exe_dir` (the bundle and sibling trees [`Discovery`] derives from the
/// executable) are written relative to [`EXE_DIR_LABEL`], so identical
/// documents produce identical output wherever the binary is installed.
/// Explicit and host directories are configuration and stay as given.
pub fn describe_dirs(dirs: &[PathBuf], exe_dir: Option<&Path>) -> String {
    dirs.iter()
        .map(|d| match exe_dir.and_then(|e| d.strip_prefix(e).ok()) {
            Some(rel) if rel.as_os_str().is_empty() => EXE_DIR_LABEL.to_string(),
            Some(rel) => format!("{EXE_DIR_LABEL}/{}", rel.display()),
            None => d.display().to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// [`Discovery::font_dirs`] for the running process.
pub fn default_font_dirs() -> Vec<PathBuf> {
    Discovery::from_process().font_dirs()
}

/// [`Discovery::tfm_dirs_for`] over [`default_font_dirs`] for the running
/// process.
pub fn default_tfm_dirs() -> Vec<PathBuf> {
    let d = Discovery::from_process();
    d.tfm_dirs_for(&d.font_dirs())
}
