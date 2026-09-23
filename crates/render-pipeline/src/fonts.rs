//! Font set: bounded resolution of the faces the pipeline may use, with
//! content-addressed identity for the display list.
//!
//! Default document face is Latin Modern (LaTeX's default look: Computer
//! Modern outlines, GUST Font License, OpenType CFF) with the
//! size-to-optical-design mapping of `t1lmr.fd`; Times (Adobe Core 14
//! metrics through font-engine) is used only when the document selects it
//! (`\usepackage{times}` / `mathptmx`). Resolution is bounded: an explicit
//! directory list is probed for explicit file names (font-engine's
//! `FontSearch`); nothing is scanned or substituted silently — a missing
//! Latin Modern file is a diagnostic and a `.notdef`-free failure, not a
//! silent Times.
//!
//! **Named families** ([`Family::Named`]) are the third kind: any font
//! installed on the machine or shipped with the project, selected by
//! fontspec's `\setmainfont{Helvetica}` and friends (`crate::fontspec`) or
//! the manifest's `[fonts]` table (`RenderOptions::fonts`). They are found
//! through `flashtex_font_discovery`'s index (built lazily, on the first
//! named lookup, so a document that names no font never scans a
//! directory), loaded from their real file and face index, and shaped by
//! font-engine from the OpenType tables alone (`hmtx`, `GPOS`/`kern`,
//! `GSUB`): **no TFM**. A missing weight or style takes the nearest face
//! and says so; a missing family takes Latin Modern and says so; nothing
//! is substituted silently.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use flashtex_font_discovery::{FontFile, FontIndex};
use flashtex_font_engine::core14::{Core14, Core14Face};
use flashtex_font_engine::math::MathTable;
use flashtex_font_engine::resolve::FontSearch;
use flashtex_font_engine::truetype::{Outlines, TrueTypeFace};
use flashtex_font_engine::{sha256, Face};

use flashtex_font_resources::required_tfm::{Manifest as RequiredManifest, MetricAsset, RequiredMetrics};
use flashtex_project_files::ProjectRoot;

use crate::cff::{self, Cff};
use crate::nfss::{FamilyKind, FontKey, Series, Shape};
use crate::tfm::Tfm;

/// The 12 pt metric set pdfLaTeX+`lmodern` lays the reference documents
/// out with, pinned to the official Latin Modern 2.004 release
/// (font-resources `fixtures/lm-required-metrics-provenance.json`):
/// `ec-lmr12` for text and `rm-lmr12/8/6` for the math roman family, plus
/// the GUST font licence next to them. They are read through
/// font-resources' rooted, digest-bound `RequiredMetrics::load` from the
/// TeX Live `texmf-dist` tree; a missing or mismatched file is a blocking
/// diagnostic, never a silent switch to OpenType metrics.
pub const REQUIRED_TFMS: [(&str, &str); 4] = [
    ("ec-lmr12.tfm", "299021120f0a29ef61278a2363903bd8defbb8faaade458eb79067342aecb56f"),
    ("rm-lmr12.tfm", "9d4e3d8e39a41b93d91f79c1c47d2297efb7b1af220b94860693c08361f227aa"),
    ("rm-lmr8.tfm", "80bcbfd844d2310ac1d3bead45aee25e91b1a4a0a60ff1771959b9a1e90ec1a2"),
    ("rm-lmr6.tfm", "eb0bfdf8db3ae1409639fac9c88f84923872500d882d9ff8dc37aff445c723fe"),
];
pub const REQUIRED_TFM_DIR: &str = "fonts/tfm/public/lm";
pub const REQUIRED_LICENSE: (&str, &str) = (
    "doc/fonts/lm/GUST-FONT-LICENSE.TXT",
    "49ea6cb9257bbee0a3979c48a774cd221550ac1c20c95549efe45fc99cc18050",
);

/// The two directory layouts the required set is accepted in, both read
/// through the rooted, digest-bound loader:
///
/// * **texmf** — a TeX Live style tree rooted at `<root>`:
///   `<root>/fonts/tfm/public/lm/{ec-lmr12,rm-lmr12,rm-lmr8,rm-lmr6}.tfm`
///   and `<root>/doc/fonts/lm/GUST-FONT-LICENSE.TXT` (MacTeX:
///   `/usr/local/texlive/2026/texmf-dist`; an app bundle can ship
///   `Contents/Resources/texmf/...` and point `FLASHTEX_FONT_DIRS` /
///   `FLASHTEX_TFM_DIRS` at `<root>/fonts/{opentype,tfm}/public/lm`, or
///   rely on the `../Resources/texmf` default below).
/// * **flat** — every TFM directory itself as the root, the four TFMs and
///   `GUST-FONT-LICENSE.TXT` directly inside it (a bundle's
///   `Contents/Resources/Fonts` next to the OTFs, or `FLASHTEX_TFM_DIRS`).
fn required_manifest(flat: bool) -> RequiredManifest {
    let (prefix, license) = if flat {
        (String::new(), "GUST-FONT-LICENSE.TXT".to_string())
    } else {
        (format!("{REQUIRED_TFM_DIR}/"), REQUIRED_LICENSE.0.to_string())
    };
    RequiredManifest {
        schema_version: 1,
        metrics: REQUIRED_TFMS
            .iter()
            .map(|(file, sha)| MetricAsset {
                path: format!("{prefix}{file}"),
                sha256: (*sha).to_string(),
                license_path: license.clone(),
                license_sha256: REQUIRED_LICENSE.1.to_string(),
            })
            .collect(),
    }
}

/// Why a TFM is not attached to a face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TfmStatus {
    /// Loaded (digest-bound for the required set, parsed for the others).
    Loaded,
    /// A required 12 pt asset could not be loaded: blocking.
    RequiredUnavailable(String),
    /// A non-required TFM was not found or did not parse: the face uses
    /// its OpenType metrics and the typesetter warns.
    Missing(String),
}
use crate::ids::GlyphId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    LatinModern,
    Times,
    /// LaTeX's default `cmr` under `\usepackage[T1]{fontenc}` (no
    /// `lmodern`): the EC metrics `t1cmr.fd` loads (`ecrm1095`, `ecbx1200`,
    /// ...) laid out with the Latin Modern outlines, which draw the same
    /// Computer Modern designs. Math is unchanged (Latin Modern Math).
    ComputerModern,
    /// LaTeX's default `cmr` without `fontenc` (OT1, no `lmodern`): the
    /// metrics `ot1cmr.fd`/`ot1cmss.fd` load (`cmr10` at 10.95pt, `cmbx10`,
    /// `cmti10`, `cmss10`, ...) laid out with the Latin Modern outlines.
    /// Latin Modern's `ec-lm*` widths match Knuth's to 1e-5 em, but its
    /// kern programs do not (`ec-lmss12` kerns `T`–`w`, `cmss12` does not;
    /// `ec-lmr10` kerns `.`–`”`), which is what moved `Two columns` 1.17 bp
    /// and `doing.''` 1.65 bp against pdflatex. Typewriter text keeps the
    /// `ec-lmtt` metrics: `cmtt` is fixed-pitch without kerns, so nothing
    /// differs. Math is unchanged (Latin Modern Math).
    ComputerModernOt1,
    /// An installed font family named by the document or the manifest
    /// (`\setmainfont{Helvetica}`, `[fonts] text = "..."`), interned on the
    /// [`FontSet`] ([`FontSet::intern_named`]) and resolved through the
    /// discovery index. Math under a named family stays Latin Modern Math
    /// until `\setmathfont` lands (`docs/proposals/font-system-math.md`).
    Named(NamedId),
}

/// The interned identity of a [`NamedSpec`] on one [`FontSet`]: stable for
/// the set's lifetime, equal for equal specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NamedId(pub u16);

/// fontspec's `Scale=` option: a factor, or match the main font's
/// lowercase (x-height) or uppercase (cap height) size (fontspec §4.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scale {
    Factor(f64),
    MatchLowercase,
    MatchUppercase,
}

impl Default for Scale {
    fn default() -> Self {
        Scale::Factor(1.0)
    }
}

/// A named family and the fontspec options this pipeline honours for it
/// (`\setmainfont[opts]{Family}`, `\newfontfamily\cmd[opts]{Family}`,
/// `\fontspec[opts]{Family}`; `crate::fontspec` parses them).
///
/// `bold_font`/`italic_font`/`bold_italic_font` are fontspec's explicit
/// face names for the shapes (`BoldFont=Helvetica Neue Medium`); when
/// absent the family's own nearest face is used. `tex_ligatures` is
/// `Ligatures=TeX` (on by default for `\setmainfont`, as fontspec does):
/// `--`/`---`/quotes are the TeX ligatures the adapter already forms.
/// `oldstyle_numbers` is `Numbers=OldStyle`: the face's GSUB `onum`
/// substitutions, applied after shaping (`crate::shape::ShapeFlags`), as
/// `\scshape` applies its `smcp`.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedSpec {
    pub family: String,
    pub scale: Scale,
    pub upright_font: Option<String>,
    pub bold_font: Option<String>,
    pub italic_font: Option<String>,
    pub bold_italic_font: Option<String>,
    pub oldstyle_numbers: bool,
    pub tex_ligatures: bool,
}

impl NamedSpec {
    pub fn new(family: &str) -> NamedSpec {
        NamedSpec {
            family: family.trim().to_string(),
            scale: Scale::default(),
            upright_font: None,
            bold_font: None,
            italic_font: None,
            bold_italic_font: None,
            oldstyle_numbers: false,
            tex_ligatures: true,
        }
    }
}

/// A named face resolved for one series/shape: the face, and what was
/// substituted on the way (reported once each by the typesetter).
#[derive(Clone)]
pub struct NamedResolution {
    pub face: Rc<LoadedFace>,
    /// The nearest weight or style was taken (`Georgia has no 600 weight;
    /// Georgia Bold (700) used`).
    pub substituted: Option<String>,
    /// Small caps were requested; the face is the upright one because GSUB
    /// `smcp` is not applied by the shaper.
    pub caps_note: Option<String>,
}

/// Which typographic role a face plays; selects the design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// Text face (roman/bold/italic per the style).
    Text { bold: bool, italic: bool },
    /// Upright-medium slanted text (`\slshape`, running heads): Latin Modern
    /// `lmromanslant*` with `ec-lmro*` metrics.
    Slanted,
    /// Math letters, symbols and operators: Latin Modern Math (`MATH`
    /// table) for both families, because `\usepackage{times}` leaves math
    /// in Computer Modern.
    Math,
    /// Any NFSS text shape (`crate::nfss`): the loaded (terminal) font of a
    /// family slot, series and shape. `Text` and `Slanted` are the roman
    /// shapes spelled the older way and resolve identically.
    Font(FontKey),
}

impl Role {
    /// The NFSS shape of a text role; `None` for math.
    pub fn key(self) -> Option<FontKey> {
        match self {
            Role::Math => None,
            Role::Text { bold, italic } => Some(FontKey::new(
                FamilyKind::Rm,
                if bold { Series::Bx } else { Series::M },
                if italic { Shape::It } else { Shape::N },
            )),
            Role::Slanted => Some(FontKey::new(FamilyKind::Rm, Series::M, Shape::Sl)),
            Role::Font(key) => Some(key),
        }
    }
}

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

    fn split(v: &Option<String>) -> Vec<PathBuf> {
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

/// The `ec-lm*` TFM that `t1lm*.fd` pairs with a Latin Modern text file:
/// `lmroman12-regular` → `ec-lmr12`, `-bold` → `ec-lmbx12`, `-italic` →
/// `ec-lmri12`, `-bolditalic` → `ec-lmbxi10`. `None` for the math face and
/// for names this table does not know.
///
/// The other Latin Modern designs pair the same way (`t1lmr.fd`,
/// `t1lmss.fd`, `t1lmtt.fd`): `lmromanslant` → `ec-lmro`/`ec-lmbxo`,
/// `lmromancaps` → `ec-lmcsc`/`ec-lmcsco`, `lmromandemi` → `ec-lmb`/
/// `ec-lmbo`, `lmromanunsl` → `ec-lmu`, `lmsans` → `ec-lmss`/`ec-lmsso`/
/// `ec-lmssbx`/`ec-lmssbo`, `lmsansdemicond` → `ec-lmssdc`/`ec-lmssdo`,
/// `lmmono` → `ec-lmtt`/`ec-lmtti`, `lmmonoslant` → `ec-lmtto`,
/// `lmmonocaps` → `ec-lmtcsc`/`ec-lmtcso`, `lmmonolt` bold → `ec-lmtk`/
/// `ec-lmtko`.
pub fn latin_modern_tfm(otf_stem: &str) -> Option<String> {
    let digits_at = otf_stem.find(|c: char| c.is_ascii_digit())?;
    let (base, rest) = otf_stem.split_at(digits_at);
    let (digits, style) = rest.split_once('-')?;
    let d: u32 = digits.parse().ok()?;
    let series = match (base, style) {
        ("lmroman", "regular") => "r",
        ("lmroman", "bold") => "bx",
        ("lmroman", "italic") => "ri",
        ("lmroman", "bolditalic") => "bxi",
        ("lmromanslant", "regular") => "ro",
        ("lmromanslant", "bold") => "bxo",
        ("lmromancaps", "regular") => "csc",
        ("lmromancaps", "oblique") => "csco",
        ("lmromandemi", "regular") => "b",
        ("lmromandemi", "oblique") => "bo",
        ("lmromanunsl", "regular") => "u",
        ("lmsans", "regular") => "ss",
        ("lmsans", "oblique") => "sso",
        ("lmsans", "bold") => "ssbx",
        ("lmsans", "boldoblique") => "ssbo",
        ("lmsansdemicond", "regular") => "ssdc",
        ("lmsansdemicond", "oblique") => "ssdo",
        ("lmmono", "regular") => "tt",
        ("lmmono", "italic") => "tti",
        ("lmmonoslant", "regular") => "tto",
        ("lmmonocaps", "regular") => "tcsc",
        ("lmmonocaps", "oblique") => "tcso",
        ("lmmonolt", "bold") => "tk",
        ("lmmonolt", "boldoblique") => "tko",
        _ => return None,
    };
    Some(format!("ec-lm{series}{d}.tfm"))
}

/// The Latin Modern OpenType file drawing an NFSS shape at `size_pt`, with
/// the design sizes of `t1lmr.fd` (`m/n` `<-5.5>`5 ... `<11-15>`12 `<15->`17,
/// `bx/n` up to 12, `m/it` 7-12, `m/sl` 8-17), `t1lmss.fd` (`m/n`, `m/sl`:
/// `<-8.5>`8 `<8.5-9.5>`9 `<9.5-11>`10 `<11-15.5>`12 `<15.5->`17) and
/// `t1lmtt.fd` (`m/n`: 8, 9, 10, `<11->`12); every other shape has one
/// 10 pt design. The note is set when Latin Modern has no design for the
/// shape (the EC fonts' `bx/sc` `ecxc`, for example) and a neighbour is
/// drawn instead.
pub fn latin_modern_outline(key: FontKey, size_pt: f64) -> (String, Option<&'static str>) {
    use FamilyKind::{Rm, Sf, Tt};
    use Series::{Bx, Sbc, B, M};
    use Shape::{Ui, It, Sc, Scit, Scsl, Sl, N};
    const RM: [(f64, u32); 7] = [(5.5, 5), (6.5, 6), (7.5, 7), (8.5, 8), (9.5, 9), (11.0, 10), (15.0, 12)];
    const RM_IT: [(f64, u32); 4] = [(7.5, 7), (8.5, 8), (9.5, 9), (11.0, 10)];
    const RM_SL: [(f64, u32); 4] = [(8.5, 8), (9.5, 9), (11.0, 10), (15.0, 12)];
    const SS: [(f64, u32); 4] = [(8.5, 8), (9.5, 9), (11.0, 10), (15.5, 12)];
    const TT: [(f64, u32); 3] = [(8.5, 8), (9.5, 9), (11.0, 10)];
    const NO_BOLD_CAPS: &str = "Latin Modern has no bold small-caps design; the medium one is drawn";
    let pick = |bounds: &[(f64, u32)], last: u32| bounds.iter().find(|(b, _)| size_pt < *b).map_or(last, |(_, d)| *d);
    let exact = |file: String| (file, None);
    match (key.family, key.series, key.shape) {
        (Rm, M, N) => exact(format!("lmroman{}-regular.otf", pick(&RM, 17))),
        (Rm, Bx, N) => exact(format!("lmroman{}-bold.otf", pick(&RM[..6], 12))),
        (Rm, M, It) => exact(format!("lmroman{}-italic.otf", pick(&RM_IT, 12))),
        (Rm, Bx, It) => exact("lmroman10-bolditalic.otf".into()),
        (Rm, M, Sl) => exact(format!("lmromanslant{}-regular.otf", pick(&RM_SL, 17))),
        (Rm, Bx, Sl) => exact("lmromanslant10-bold.otf".into()),
        (Rm, M, Sc) => exact("lmromancaps10-regular.otf".into()),
        (Rm, M, Scsl) => exact("lmromancaps10-oblique.otf".into()),
        (Rm, M, Scit) => ("lmromancaps10-oblique.otf".into(), Some("Latin Modern has no italic small-caps design; the slanted one is drawn")),
        (Rm, M, Ui) => exact("lmromanunsl10-regular.otf".into()),
        (Rm, B, N) => exact("lmromandemi10-regular.otf".into()),
        (Rm, B, Sl | It) => exact("lmromandemi10-oblique.otf".into()),
        (Rm, Bx | B, Sc) => ("lmromancaps10-regular.otf".into(), Some(NO_BOLD_CAPS)),
        (Rm, Bx | B, Scsl | Scit) => ("lmromancaps10-oblique.otf".into(), Some(NO_BOLD_CAPS)),
        (Sf, M, N) => exact(format!("lmsans{}-regular.otf", pick(&SS, 17))),
        (Sf, M, Sl | It) => exact(format!("lmsans{}-oblique.otf", pick(&SS, 17))),
        (Sf, Bx | B, N) => exact("lmsans10-bold.otf".into()),
        (Sf, Bx | B, Sl | It) => exact("lmsans10-boldoblique.otf".into()),
        (Sf, Sbc, N) => exact("lmsansdemicond10-regular.otf".into()),
        (Sf, Sbc, Sl | It) => exact("lmsansdemicond10-oblique.otf".into()),
        (Tt, M, N) => exact(format!("lmmono{}-regular.otf", pick(&TT, 12))),
        (Tt, M, It) => exact("lmmono10-italic.otf".into()),
        (Tt, M, Sl) => exact("lmmonoslant10-regular.otf".into()),
        (Tt, M, Sc) => exact("lmmonocaps10-regular.otf".into()),
        (Tt, M, Scsl) => exact("lmmonocaps10-oblique.otf".into()),
        (Tt, B | Bx, N) => exact("lmmonolt10-bold.otf".into()),
        (Tt, B | Bx, Sl | It) => exact("lmmonolt10-boldoblique.otf".into()),
        _ => {
            let roman = FontKey::new(Rm, if key.bold() { Bx } else { M }, if key.slanted() { It } else { N });
            (latin_modern_outline(roman, size_pt).0, Some("no Latin Modern design for this font shape; the roman one of the same weight and slant is drawn"))
        }
    }
}

/// How a metrics-fallback note names the family of a TFM file.
fn metric_family_label(tfm: &str) -> &'static str {
    if tfm.starts_with("ec-lm") {
        "Latin Modern"
    } else if tfm_encoding(tfm) == crate::ids::Encoding::OT1 {
        "OT1 cm"
    } else if ["ecss", "ecsi", "ecsx", "ecso"].iter().any(|p| tfm.starts_with(p)) {
        "T1 cmss"
    } else if ["ectt", "ecst", "ecit", "ectc"].iter().any(|p| tfm.starts_with(p)) {
        "T1 cmtt"
    } else {
        "T1 cmr"
    }
}

/// Where TeX Live keeps the EC metrics (`jknappen/ec`), relative to a
/// texmf root.
pub const EC_TFM_DIR: &str = "fonts/tfm/jknappen/ec";

/// Where TeX Live keeps the Euler (`eufm`) metrics `\mathfrak` uses.
pub const AMS_EULER_TFM_DIR: &str = "fonts/tfm/public/amsfonts/euler";

/// The sizes `t1cmr.fd` declares for every EC shape
/// (`<5><6><7><8><9><10><10.95><12><14.4><17.28><20.74><24.88><29.86><35.83>genb*ecrm`)
/// and the file-name suffix `genb*` builds from each.
const EC_SIZES: [(f64, &str); 14] = [
    (5.0, "0500"),
    (6.0, "0600"),
    (7.0, "0700"),
    (8.0, "0800"),
    (9.0, "0900"),
    (10.0, "1000"),
    (10.95, "1095"),
    (12.0, "1200"),
    (14.4, "1440"),
    (17.28, "1728"),
    (20.74, "2074"),
    (24.88, "2488"),
    (29.86, "2986"),
    (35.83, "3583"),
];

/// The EC metric file the T1 Computer Modern `.fd` files load for a text
/// role at `size_pt`, at the declared size nearest `size_pt` (an undeclared
/// size is a LaTeX size substitution to the nearest one):
///
/// * `t1cmr.fd`: `m/n` `ecrm`, `m/sl` `ecsl`, `m/it` `ecti`, `m/sc` `eccc`,
///   `bx/n` `ecbx`, `b/n` `ecrb`, `bx/it` `ecbi`, `bx/sl` `ecbl`, `bx/sc`
///   `ecxc`, `m/ui` `ecui`, `m/scsl` `ecsc`, `bx/scsl` and `b/scsl` `ecoc`;
/// * `t1cmss.fd`: `m/n` `ecss`, `m/sl` and `m/it` `ecsi`, `bx/n` `ecsx`,
///   `bx/it` and `bx/sl` `ecso`;
/// * `t1cmtt.fd`: `m/n` `ectt`, `m/sl` `ecst`, `m/it` `ecit`, `m/sc` `ectc`.
///
/// The sans and typewriter families declare `<5><6><7><8>#50800`: every
/// size up to 8 pt uses the 8 pt file. `None` for math and for shapes the
/// files do not declare (they are substituted before a font is loaded).
pub fn ec_tfm_file(role: Role, size_pt: f64) -> Option<String> {
    use FamilyKind::{Rm, Sf, Tt};
    use Series::{Bx, B, M};
    use Shape::{Ui, It, Sc, Scsl, Sl, N};
    let key = role.key()?;
    let (prefix, small_sizes_share_0800) = match (key.family, key.series, key.shape) {
        (Rm, M, N) => ("ecrm", false),
        (Rm, M, Sl) => ("ecsl", false),
        (Rm, M, It) => ("ecti", false),
        (Rm, M, Sc) => ("eccc", false),
        (Rm, M, Ui) => ("ecui", false),
        (Rm, M, Scsl) => ("ecsc", false),
        (Rm, Bx, N) => ("ecbx", false),
        (Rm, B, N) => ("ecrb", false),
        (Rm, Bx, It) => ("ecbi", false),
        (Rm, Bx, Sl) => ("ecbl", false),
        (Rm, Bx, Sc) => ("ecxc", false),
        (Rm, Bx | B, Scsl) => ("ecoc", false),
        (Sf, M, N) => ("ecss", true),
        (Sf, M, Sl | It) => ("ecsi", true),
        (Sf, Bx, N) => ("ecsx", true),
        (Sf, Bx, Sl | It) => ("ecso", true),
        (Tt, M, N) => ("ectt", true),
        (Tt, M, Sl) => ("ecst", true),
        (Tt, M, It) => ("ecit", true),
        (Tt, M, Sc) => ("ectc", true),
        _ => return None,
    };
    let (size, suffix) = EC_SIZES
        .iter()
        .min_by(|a, b| (a.0 - size_pt).abs().total_cmp(&(b.0 - size_pt).abs()))?;
    let suffix = if small_sizes_share_0800 && *size <= 8.0 { "0800" } else { suffix };
    Some(format!("{prefix}{suffix}.tfm"))
}

/// The TS1 (text companion) metric file set with the text file `tfm`: the
/// font `\UseTextSymbol{TS1}{..}` switches to for a symbol T1 lacks
/// (`\textcopyright`, `\textdegree`, ...), which keeps the family, series,
/// shape and size and changes only the encoding.
///
/// * `ts1cmr.fd`: `tcrm`/`tcsl`/`tcti`/`tcbx`/`tcrb`/`tcbi`/`tcbl`/`tcui`
///   at the EC sizes -- `ec<shape><size>` → `tc<shape><size>`; the small-caps
///   shapes it does not declare (`eccc`, `ecsc`, `ecxc`, `ecoc`, `ectc`)
///   are NFSS-substituted by the upright of the same series (`m/sc` →
///   `m/n`, `bx/sc` → `bx/n`) before the font is loaded;
/// * `ts1cmss.fd`/`ts1cmtt.fd`: `tcss`/`tcsi`/`tcsx`/`tcso`, `tctt`/`tcst`/
///   `tcit` likewise;
/// * `ts1lm*.fd`: `ec-lm<face>` → `ts1-lm<face>`.
///
/// `None` for a file this table cannot pair. The companion is optional:
/// a face without one sets those symbols from its own program's advances,
/// as before.
pub fn ts1_companion_tfm(tfm: &str, size_pt: f64) -> Option<String> {
    let stem = tfm.strip_suffix(".tfm")?;
    if let Some(rest) = stem.strip_prefix("ec-lm") {
        return Some(format!("ts1-lm{rest}.tfm"));
    }
    // Knuth's OT1 files (`cmr10` at 10.95pt): `ts1cmr.fd` declares the
    // companions at the EC sizes (`genb*tcrm`), so the file is chosen by
    // the size, not the design.
    if let Some(cm) = stem.strip_prefix("cm").filter(|_| !stem.starts_with("cm-")) {
        let design = cm.trim_end_matches(|c: char| c.is_ascii_digit());
        let shape = match design {
            "r" | "csc" => "rm",
            "bx" | "b" => "bx",
            "ti" => "ti",
            "sl" => "sl",
            "bxti" => "bi",
            "bxsl" => "bl",
            "u" => "ui",
            "ss" | "ssdc" => "ss",
            "ssi" => "si",
            "ssbx" => "sx",
            "tt" | "tcsc" => "tt",
            "itt" => "it",
            "sltt" => "st",
            _ => return None,
        };
        let (_, suffix) = EC_SIZES.iter().min_by(|a, b| (a.0 - size_pt).abs().total_cmp(&(b.0 - size_pt).abs()))?;
        return Some(format!("tc{shape}{suffix}.tfm"));
    }
    let rest = stem.strip_prefix("ec")?;
    let (shape, size) = rest.split_at(rest.find(|c: char| c.is_ascii_digit())?);
    let shape = match shape {
        "cc" | "sc" => "rm",
        "xc" | "oc" => "bx",
        "tc" => "tt",
        other => other,
    };
    Some(format!("tc{shape}{size}.tfm"))
}

/// The OT1 metric file `ot1cmr.fd`/`ot1cmss.fd` (TeX Live 2026) load for
/// a text role at `size_pt`, at the declared size nearest `size_pt`. The
/// files are Knuth's design sizes scaled to the requested size (`cmr10 at
/// 10.95pt`), unlike the EC files which exist at every size:
///
/// * `ot1cmr.fd`: `m/n` `<5><6><7><8><9><10><12>gen*cmr <10.95>cmr10
///   <14.4>cmr12 <17.28><20.74><24.88>cmr17`; `m/sl` `<5><6><7>cmsl8
///   <8><9>gen*cmsl <10><10.95>cmsl10 <12>...cmsl12`; `m/it` `<5><6><7>cmti7
///   <8>cmti8 <9>cmti9 <10><10.95>cmti10 <12>...cmti12`; `m/sc` `cmcsc10`;
///   `m/ui` `cmu10`; `b/n` `cmb10`; `bx/n` `<5>...<9>gen*cmbx <10><10.95>cmbx10
///   <12>...cmbx12`; `bx/sl` `cmbxsl10`; `bx/it` `cmbxti10`;
/// * `ot1cmss.fd`: `m/n` `<5>...<8>cmss8 <9>cmss9 <10><10.95>cmss10
///   <12><14.4>cmss12 <17.28>...cmss17`; `m/sl` (and `m/it`, `ssub`) the
///   `cmssi` files at the same sizes; `bx/n` `cmssbx10`; `sbc/n` `cmssdc10`;
///   an undeclared `bx/it`/`bx/sl` is NFSS-substituted by `bx/n`.
///
/// `None` for math, for the typewriter family (see
/// [`Family::ComputerModernOt1`]) and for shapes the files do not declare.
pub fn ot1_tfm_file(role: Role, size_pt: f64) -> Option<String> {
    use FamilyKind::{Rm, Sf};
    use Series::{Bx, Sbc, B, M};
    use Shape::{Ui, It, Sc, Sl, N};
    let key = role.key()?;
    // The declared sizes of both files; an undeclared size is LaTeX's
    // substitution to the nearest one.
    const SIZES: [f64; 12] = [5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 10.95, 12.0, 14.4, 17.28, 20.74, 24.88];
    let size = SIZES.iter().copied().min_by(|a, b| (a - size_pt).abs().total_cmp(&(b - size_pt).abs()))?;
    let gen = |prefix: &str, own: &[u32], else_: &[(f64, u32)]| -> String {
        let d = size.round() as u32;
        if (size - f64::from(d)).abs() < 1e-9 && own.contains(&d) {
            return format!("{prefix}{d}.tfm");
        }
        let (_, d) = else_.iter().find(|(at, _)| (*at - size).abs() < 1e-9).copied().unwrap_or(*else_.last().unwrap());
        format!("{prefix}{d}.tfm")
    };
    // The `<a><b>file` runs of the declarations as (declared size, design).
    let table = |prefix: &str, runs: &[(&[f64], u32)]| -> String {
        let d = runs.iter().find(|(sizes, _)| sizes.iter().any(|at| (*at - size).abs() < 1e-9)).map_or(runs.last().unwrap().1, |(_, d)| *d);
        format!("{prefix}{d}.tfm")
    };
    let file = match (key.family, key.series, key.shape) {
        (Rm, M, N) => gen("cmr", &[5, 6, 7, 8, 9, 10, 12], &[(10.95, 10), (14.4, 12), (17.28, 17), (20.74, 17), (24.88, 17)]),
        (Rm, M, Sl) => table("cmsl", &[(&[5.0, 6.0, 7.0, 8.0], 8), (&[9.0], 9), (&[10.0, 10.95], 10), (&[12.0, 14.4, 17.28, 20.74, 24.88], 12)]),
        (Rm, M, It) => table("cmti", &[(&[5.0, 6.0, 7.0], 7), (&[8.0], 8), (&[9.0], 9), (&[10.0, 10.95], 10), (&[12.0, 14.4, 17.28, 20.74, 24.88], 12)]),
        (Rm, M, Sc) => "cmcsc10.tfm".to_string(),
        (Rm, M, Ui) => "cmu10.tfm".to_string(),
        (Rm, B, N) => "cmb10.tfm".to_string(),
        (Rm, Bx, N) => gen("cmbx", &[5, 6, 7, 8, 9], &[(10.0, 10), (10.95, 10), (12.0, 12), (14.4, 12), (17.28, 12), (20.74, 12), (24.88, 12)]),
        (Rm, Bx, Sl) => "cmbxsl10.tfm".to_string(),
        (Rm, Bx, It) => "cmbxti10.tfm".to_string(),
        (Sf, M, N) => table("cmss", &[(&[5.0, 6.0, 7.0, 8.0], 8), (&[9.0], 9), (&[10.0, 10.95], 10), (&[12.0, 14.4], 12), (&[17.28, 20.74, 24.88], 17)]),
        (Sf, M, Sl | It) => table("cmssi", &[(&[5.0, 6.0, 7.0, 8.0], 8), (&[9.0], 9), (&[10.0, 10.95], 10), (&[12.0, 14.4], 12), (&[17.28, 20.74, 24.88], 17)]),
        (Sf, Bx, N | Sl | It) => "cmssbx10.tfm".to_string(),
        (Sf, Sbc, N) => "cmssdc10.tfm".to_string(),
        _ => return None,
    };
    Some(file)
}

/// Where TeX Live keeps Knuth's Computer Modern metrics (`public/cm`),
/// relative to a texmf root; the bundled tree ships the files
/// [`ot1_tfm_file`] can name under the same path.
pub const CM_TFM_DIR: &str = "fonts/tfm/public/cm";

/// The text encoding of a TFM by its name: Knuth's `cm*` files are OT1,
/// everything else this crate attaches (`ec*`, `ec-lm*`, `ptm*8t`) is T1.
pub fn tfm_encoding(tfm: &str) -> crate::ids::Encoding {
    let stem = tfm.trim_end_matches(".tfm");
    if stem.starts_with("cm") && !stem.starts_with("cm-") {
        crate::ids::Encoding::OT1
    } else {
        crate::ids::Encoding::T1
    }
}

/// `CHARWD` of `tcrm<size>.tfm` (the TS1 `cmr` `m/n` font `ts1cmr.fd`
/// loads, at the same declared sizes as [`EC_SIZES`]) for the TS1 symbols
/// the default itemize labels use, in design-size units: `(\textbullet` and
/// `\textasteriskcentered` (the same width), `\textperiodcentered)`. These
/// are the `cmsy` designs: 0.5em and 0.2777em at 10 pt. Transcribed with
/// `tftopl` from TeX Live 2026 (predating the bundled `tc*` companions,
/// [`ts1_companion_tfm`]; the list labels keep this table).
const TCRM_SYMBOL_WIDTHS: [(f64, f64); 14] = [
    (0.680389, 0.402679),
    (0.610962, 0.351766),
    (0.569305, 0.323334),
    (0.53112, 0.295067),
    (0.513763, 0.285424),
    (0.499878, 0.27771),
    (0.497164, 0.276356),
    (0.489464, 0.271924),
    (0.475939, 0.264197),
    (0.469761, 0.260836),
    (0.462573, 0.256799),
    (0.456601, 0.253447),
    (0.451612, 0.250645),
    (0.447456, 0.248311),
];

/// The width in points at `size_pt` of a TS1 symbol set from `tcrm` (LaTeX's
/// `\textbullet` `•`, `\textasteriskcentered` `∗` and `\textperiodcentered`
/// `·` without `lmodern`), from the declared size nearest `size_pt`. `None`
/// for any other character.
pub fn tcrm_symbol_width(ch: char, size_pt: f64) -> Option<f64> {
    let index = EC_SIZES
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 .0 - size_pt).abs().total_cmp(&(b.1 .0 - size_pt).abs()))?
        .0;
    let (bullet, period) = TCRM_SYMBOL_WIDTHS[index];
    match ch {
        '•' | '∗' => Some(bullet * size_pt),
        '·' | '⋅' => Some(period * size_pt),
        _ => None,
    }
}

/// The box (width, height, depth in points) of a text symbol that OT1 has
/// no slot for and the kernel therefore sets from a *math* font
/// (latex.ltx 10046-10059: `\DeclareTextSymbolDefault{\textbackslash}{OMS}`,
/// `\textbar`, `\textbraceleft`, `\textbraceright` likewise; `\textless`
/// and `\textgreater` `{OML}`). `\UseTextSymbol` keeps the family, series
/// and size and switches the encoding, so the font is what `omscmr.fd`/
/// `omlcmr.fd` declare for the roman family -- `cmsy`/`cmmi` for `m`,
/// `cmbsy`/`cmmib` for `bx` -- and for every other family and series
/// (`cmss`, `cmtt`, `lmss`, `lmtt`, and any `b`) the encoding's default
/// `OMS/cmsy/m/n` / `OML/cmm/m/it` after a "Font shape undefined" warning
/// (`fontmath.ltx` 49-50). Latin Modern's `lmsy`/`lmmi` are metric copies.
/// `bold` is that roman-`bx` case.
///
/// `CHARWD`/`CHARHT`/`CHARDP` in design units per design size, transcribed
/// with `tftopl` from TeX Live 2026 (`cmsy5-10`, `cmbsy5/7/10`, `cmmi5-10`,
/// `cmmib5/7/10`), with the `.fd` size ranges `<-5.5>5 <5.5-6.5>6 <6.5-7.5>7
/// <7.5-8.5>8 <8.5-9.5>9 <9.5->10` (medium) and `<-6>5 <6-8>7 <8->10`
/// (bold), every design scaled linearly to `size_pt` as TeX loads it.
/// `None` for any other character.
///
/// pdflatex 10 pt: `\hbox{\texttt{a\textbackslash b}}` is 15.49992 pt
/// (`cmtt` 5.24995 + `cmsy` 5.0 + 5.24995), `\hbox(7.5+2.5)`; the
/// typewriter font's own `\` (5.25 pt, T1 slot 92) is what `[T1]{fontenc}`
/// sets and what this used to set under OT1 too.
pub fn ot1_math_symbol_box(ch: char, bold: bool, size_pt: f64) -> Option<(f64, f64, f64)> {
    const MEDIUM_BOUNDS: [f64; 5] = [5.5, 6.5, 7.5, 8.5, 9.5];
    const BOLD_BOUNDS: [f64; 2] = [6.0, 8.0];
    // cmsy: `\{` `\}` `\` (slots 102, 103, 110) share one width; `|` (106).
    const CMSY_BRACE: [f64; 6] = [0.73612, 0.6388855, 0.58532, 0.531258, 0.5138855, 0.500002];
    const CMSY_BAR: [f64; 6] = [0.458338, 0.379628, 0.339288, 0.295143, 0.285492, 0.277779];
    const CMBSY_BRACE: [f64; 3] = [0.7916565, 0.65516, 0.574997];
    const CMBSY_BAR: [f64; 3] = [0.4694395, 0.371033, 0.319443];
    // cmmi: `<` and `>` (slots 60, 62) share width, height and depth.
    const CMMI_LESS: [(f64, f64, f64); 6] = [
        (1.083349, 0.600916, 0.100916),
        (0.962956, 0.587987, 0.087987),
        (0.892861, 0.575675, 0.075675),
        (0.826401, 0.563126, 0.063126),
        (0.799377, 0.550973, 0.050973),
        (0.777781, 0.539098, 0.039098),
    ];
    const CMMIB_LESS: [(f64, f64, f64); 3] = [(1.1944275, 0.654114, 0.154114), (1.01032, 0.625319, 0.125319), (0.89444, 0.585556, 0.085556)];
    let medium = MEDIUM_BOUNDS.iter().filter(|b| size_pt >= **b).count();
    let bold_ix = BOLD_BOUNDS.iter().filter(|b| size_pt >= **b).count();
    let (w, h, d) = match ch {
        '\\' | '{' | '}' => (if bold { CMBSY_BRACE[bold_ix] } else { CMSY_BRACE[medium] }, 0.75, 0.25),
        '|' => (if bold { CMBSY_BAR[bold_ix] } else { CMSY_BAR[medium] }, 0.75, 0.25),
        '<' | '>' => {
            if bold {
                CMMIB_LESS[bold_ix]
            } else {
                CMMI_LESS[medium]
            }
        }
        _ => return None,
    };
    Some((w * size_pt, h * size_pt, d * size_pt))
}

/// The `\fontdimen`s XeTeX gives a native (OpenType) font, which is what
/// fontspec's interword glue is under XeLaTeX (xetex.web, `read_font_info`
/// for a native font, and `XeTeX_ext.c`): `\fontdimen2` (space) is the
/// advance of U+0020, stretch is half of it, shrink and extra space a
/// third, the x-height is the `OS/2` `sxHeight` (the `x` glyph's height
/// when the font does not declare it) and the quad is one em. As em
/// fractions, so [`crate::params::TextParams::at`] scales them to the
/// (possibly `Scale=`d) size. A font with no space glyph gets TeX's
/// nominal third of an em.
pub fn opentype_params(face: &LoadedFace) -> crate::params::TextParams {
    let upem = f64::from(face.units_per_em.max(1));
    let space = face
        .face()
        .glyph_id(' ')
        .and_then(|g| face.face().advance(g).ok())
        .map_or(1.0 / 3.0, |a| f64::from(a) / upem);
    crate::params::TextParams {
        space,
        stretch: space / 2.0,
        shrink: space / 3.0,
        x_height: height_em(face, true),
        quad: 1.0,
        extra_space: space / 3.0,
    }
}

/// The x-height (`lowercase`) or cap height of a face in em: the `OS/2`
/// value when the font declares it, else the bounds of `x`/`H` (what
/// fontspec's `Scale=MatchLowercase`/`MatchUppercase` measure through
/// `\fontcharht`), else 0.
pub fn height_em(face: &LoadedFace, lowercase: bool) -> f64 {
    let upem = f64::from(face.units_per_em.max(1));
    let m = face.face().vertical_metrics();
    let declared = if lowercase { m.x_height_declared.then_some(m.x_height) } else { m.cap_height_declared.then_some(m.cap_height) };
    if let Some(v) = declared.filter(|v| *v > 0) {
        return f64::from(v) / upem;
    }
    let ch = if lowercase { 'x' } else { 'H' };
    face.face()
        .glyph_id(ch)
        .map(|g| face.bounds(g, Some(ch)))
        .filter(|b| !b.empty)
        .map_or(0.0, |b| f64::from(b.y_max) / upem)
}

/// Glyph extents in font units: `[x_min, y_min, x_max, y_max]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bounds {
    pub x_min: i32,
    pub y_min: i32,
    pub x_max: i32,
    pub y_max: i32,
    /// No marking contours (space and friends).
    pub empty: bool,
}

pub enum FaceKind {
    /// An OpenType program parsed by font-engine plus, for `CFF ` outlines,
    /// this crate's charstring reader for glyph bounds. `cff` is `None` for
    /// a `glyf` face (a system TrueType font selected by name), whose
    /// bounds come from each glyph's `glyf` header instead.
    Otf { face: TrueTypeFace, cff: Option<Cff> },
    Core14(Core14Face),
}

/// A loaded face plus the identity fields the display list publishes.
pub struct LoadedFace {
    /// Content-addressed id used on the wire: the SHA-256 hex of the RAW
    /// font file bytes (what `fonts[].sha256` of rendering-v2 and
    /// font-resources' digest checks mean). font-engine's own
    /// `FontId::content_sha256` hashes bytes ‖ face_index and is kept in
    /// `engine_id` for diagnostics only.
    pub font_id: Rc<str>,
    /// font-engine's identity (SHA-256 of bytes ‖ big-endian face index).
    pub engine_id: String,
    /// Stable human-readable name (file stem or Core 14 name), for
    /// diagnostics and tests only.
    pub name: String,
    pub kind: FaceKind,
    pub sha256: [u8; 32],
    pub byte_length: u64,
    pub units_per_em: u32,
    pub glyph_count: u32,
    pub postscript_name: String,
    /// `opentype-cff` (Latin Modern), `static-truetype` (glyf) or
    /// `core14-afm` (metrics only, no program).
    pub format: &'static str,
    pub path: Option<PathBuf>,
    /// The face within a `.ttc` collection (0 for a single-face file and
    /// for every face the explicit-file-name path loads). Published on the
    /// wire as `fonts[].face_index`; for a member other than the first the
    /// `font_id` is font-engine's content hash (bytes ‖ index) so two faces
    /// of one collection never share an id.
    pub face_index: u32,
    /// The TeX metrics pdfTeX lays this face out with (`ec-lm*.tfm`), when
    /// found; shaping then takes widths/kerns/ligatures/heights from here.
    pub tfm: Option<Rc<Tfm>>,
    /// The text encoding `tfm` is laid out in ([`tfm_encoding`]): the slots
    /// characters are shaped through. T1 when no TFM is attached.
    pub encoding: crate::ids::Encoding,
    /// The TS1 companion of `tfm` ([`ts1_companion_tfm`]), when found: the
    /// metrics of the symbols T1 has no slot for (`©`, `°`, `€`, ...),
    /// which pdfTeX sets from the companion font at the same size. `None`
    /// leaves those to the program's own advances.
    pub ts1_tfm: Option<Rc<Tfm>>,
    /// Why no TFM is attached (reported once by the typesetter).
    pub tfm_missing: Option<String>,
    pub tfm_status: TfmStatus,
    /// Shaping-cache identity: `font_id` for the default metrics, extended
    /// with the TFM for a face laid out with EC metrics (the same program
    /// then shapes differently).
    pub shape_key: Rc<str>,
    /// Set when EC metrics were requested but unavailable and the Latin
    /// Modern (`ec-lm*`) TFM was attached instead (reported once).
    pub metrics_fallback: Option<String>,
    bounds_cache: RefCell<BTreeMap<u16, Bounds>>,
    /// `GSUB` single-substitution maps by feature tag (`smcp`, `onum`),
    /// parsed on first request; `None` when the face has no such feature.
    feature_maps: RefCell<BTreeMap<[u8; 4], Option<Rc<BTreeMap<u16, u16>>>>>,
}

impl LoadedFace {
    pub fn face(&self) -> &dyn Face {
        match &self.kind {
            FaceKind::Otf { face, .. } => face,
            FaceKind::Core14(f) => f,
        }
    }

    pub fn otf(&self) -> Option<&TrueTypeFace> {
        match &self.kind {
            FaceKind::Otf { face, .. } => Some(face),
            FaceKind::Core14(_) => None,
        }
    }

    pub fn program(&self) -> Option<&[u8]> {
        self.otf().map(TrueTypeFace::program)
    }

    pub fn math(&self) -> Option<&MathTable> {
        self.otf().and_then(TrueTypeFace::math)
    }

    /// Font-unit value in points at `size_pt`.
    pub fn pt(&self, units: i64, size_pt: f64) -> f64 {
        units as f64 * size_pt / f64::from(self.units_per_em)
    }

    /// paragraph-layout's opaque 32-byte identity: the content hash itself.
    pub fn layout_id(&self) -> flashtex_paragraph_layout::FontId {
        flashtex_paragraph_layout::FontId(self.sha256)
    }

    /// The face's `GSUB` single substitutions for feature `tag` (glyph ->
    /// glyph): `smcp` for small capitals, `onum` for old-style figures.
    /// `None` when the face has no `GSUB` or no such feature (or is a Core
    /// 14 metric set), so a caller can say the feature is not applied.
    pub fn feature_map(&self, tag: &[u8; 4]) -> Option<Rc<BTreeMap<u16, u16>>> {
        if let Some(m) = self.feature_maps.borrow().get(tag) {
            return m.clone();
        }
        let map = self
            .otf()
            .and_then(|f| f.table(b"GSUB"))
            .and_then(|g| crate::mathfont::single_substitutions(g, tag).ok())
            .filter(|m| !m.is_empty())
            .map(Rc::new);
        self.feature_maps.borrow_mut().insert(*tag, map.clone());
        map
    }

    /// Glyph extents in font units. CFF faces use the real charstring
    /// bounds; Core 14 faces (no outlines available) use class-based
    /// approximations from the AFM header, stated in README.
    pub fn bounds(&self, gid: GlyphId, ch: Option<char>) -> Bounds {
        if let Some(b) = self.bounds_cache.borrow().get(&gid.0) {
            return *b;
        }
        let b = match &self.kind {
            FaceKind::Otf { face, cff } => {
                let bb = match cff {
                    Some(cff) => face
                        .cff_table()
                        .and_then(|t| cff.glyph_bbox(t, gid.0).ok())
                        .and_then(|(bb, _)| cff::round_bbox(bb)),
                    // `glyf`: every non-empty glyph, simple or composite,
                    // opens with numberOfContours, xMin, yMin, xMax, yMax
                    // (OpenType 1.9 §5.3.2); an empty glyph has no data.
                    None => face.glyph_data(gid).ok().filter(|g| g.len() >= 10).map(|g| {
                        let at = |i: usize| i32::from(i16::from_be_bytes([g[i], g[i + 1]]));
                        [at(2), at(4), at(6), at(8)]
                    }),
                };
                match bb {
                    Some([x0, y0, x1, y1]) => Bounds {
                        x_min: x0,
                        y_min: y0,
                        x_max: x1,
                        y_max: y1,
                        empty: false,
                    },
                    None => Bounds {
                        empty: true,
                        ..Bounds::default()
                    },
                }
            }
            FaceKind::Core14(f) => {
                let h = f.which().header();
                let adv = i32::from(f.advance(gid).unwrap_or(0));
                let (y_min, y_max) = match ch {
                    Some(c) if c.is_ascii_digit() || c.is_uppercase() => (0, i32::from(h.cap_height.max(662))),
                    Some(c) if "bdfhklt".contains(c) => (0, i32::from(h.ascender)),
                    Some(c) if "gjpqy".contains(c) => (i32::from(h.descender), i32::from(h.x_height)),
                    Some(c) if "()[]{}/|".contains(c) => (i32::from(h.descender), i32::from(h.ascender)),
                    Some(c) if ",;".contains(c) => (-140, i32::from(h.x_height) / 3),
                    Some(c) if c.is_lowercase() => (0, i32::from(h.x_height)),
                    Some(c) if "+=<>-".contains(c) => (100, 500),
                    Some(c) if c == '.' => (0, 100),
                    Some(' ') => (0, 0),
                    _ => (i32::from(h.descender), i32::from(h.ascender)),
                };
                Bounds {
                    x_min: 0,
                    y_min,
                    x_max: adv,
                    y_max,
                    empty: ch == Some(' '),
                }
            }
        };
        self.bounds_cache.borrow_mut().insert(gid.0, b);
        b
    }
}

pub struct FontSet {
    search: FontSearch,
    faces: RefCell<Vec<Rc<LoadedFace>>>,
    by_name: RefCell<BTreeMap<String, usize>>,
    /// File names that failed to load, with the reason (reported once).
    failures: RefCell<BTreeMap<String, String>>,
    tfm_dirs: Vec<PathBuf>,
    /// The executable's directory when the search list was derived from
    /// it; diagnostics show directories under it relative to
    /// [`EXE_DIR_LABEL`] so output never depends on the install location.
    exe_dir: Option<PathBuf>,
    /// The required 12 pt set, loaded once on first use.
    required: RefCell<Option<Result<Rc<RequiredMetrics>, String>>>,
    /// Whether the required set was found in the flat layout.
    required_flat: RefCell<bool>,
    /// Non-required TFMs parsed so far, by file name.
    tfms: RefCell<BTreeMap<String, Result<Rc<Tfm>, String>>>,
    /// Shaped words, keyed by (face, text); shaping is size-independent and
    /// a keystroke changes one word, so this outlives requests. Bounded.
    shaper: crate::shape::Shaper,
    /// The directories the discovery index scans for named families
    /// (`flashtex_font_discovery::scan_dirs`), and the index itself, built
    /// on the first named lookup and kept for the set's lifetime. A
    /// document that names no font never touches it.
    index_dirs: RefCell<Vec<PathBuf>>,
    /// `with_index_dirs` was called: the list is the caller's and
    /// `set_project_root` leaves it alone (hermetic tests, explicit CLIs).
    index_dirs_explicit: bool,
    index: RefCell<Option<Rc<FontIndex>>>,
    /// Interned named-family specs, by [`NamedId`].
    named: RefCell<Vec<NamedSpec>>,
    /// Named faces resolved so far, by (id, bold, italic).
    named_faces: RefCell<BTreeMap<(u16, bool, bool), Result<NamedResolution, String>>>,
    /// The font program each Core 14 face is drawn with on output
    /// ([`FontSet::core14_program`]), looked up once per face.
    core14_programs: RefCell<Vec<(Core14, Option<Rc<Core14Program>>)>>,
}

/// The TeX Gyre OpenType face that draws a Core 14 metric face on output:
/// Termes for Times, Heros for Helvetica, Cursor for Courier (GUST Font
/// License; the URW Nimbus designs pdfTeX's psnfss maps embed, extended by
/// GUST). Symbol has none.
pub fn core14_program_file(which: Core14) -> Option<&'static str> {
    Some(match which {
        Core14::TimesRoman => "texgyretermes-regular.otf",
        Core14::TimesBold => "texgyretermes-bold.otf",
        Core14::TimesItalic => "texgyretermes-italic.otf",
        Core14::TimesBoldItalic => "texgyretermes-bolditalic.otf",
        Core14::Helvetica => "texgyreheros-regular.otf",
        Core14::Courier => "texgyrecursor-regular.otf",
        Core14::Symbol => return None,
    })
}

/// Host TeX trees' TeX Gyre directories, probed after the font search
/// list (which holds the bundled copies) for [`core14_program_file`].
pub const TEX_GYRE_DIRS: [&str; 6] = [
    "/usr/local/texlive/2026/texmf-dist/fonts/opentype/public/tex-gyre",
    "/usr/local/texlive/2026basic/texmf-dist/fonts/opentype/public/tex-gyre",
    "/usr/local/texlive/2025/texmf-dist/fonts/opentype/public/tex-gyre",
    "/usr/local/texlive/2025basic/texmf-dist/fonts/opentype/public/tex-gyre",
    "/usr/share/texmf/fonts/opentype/public/tex-gyre",
    "/usr/share/texlive/texmf-dist/fonts/opentype/public/tex-gyre",
];

/// A Core 14 face's output program: the OpenType face and, per Core 14
/// glyph id, the program's glyph for the same character.
///
/// Layout keeps the Core 14 AFM metrics, which are the widths and kerns
/// pdfTeX sets Times/Helvetica/Courier with (psnfss `ptmr8t`/`ptmr7t`,
/// `phvr8t`, `pcrr8t` were generated from the same Adobe AFMs; every
/// ASCII width agrees to 0.01/1000 em). Only the glyph ids and the font a
/// run names change, so the display list carries a program the exact PDF
/// route can embed, as pdfTeX embeds URW's Nimbus faces for these names.
pub struct Core14Program {
    pub face: Rc<LoadedFace>,
    /// Indexed by Core 14 glyph id (0 is `.notdef` and maps to 0).
    gids: Vec<u16>,
}

impl Core14Program {
    /// The program's glyph for Core 14 glyph `gid`.
    pub fn gid(&self, gid: u16) -> Option<u16> {
        self.gids.get(usize::from(gid)).copied().filter(|g| *g != 0)
    }
}

/// The program character for an AFM character TeX Gyre encodes under
/// another code point: the modifier macron U+02C9 as U+00AF, the increment
/// U+2206 as U+0394, Adobe's private-use `commaaccent` U+F6C3 as U+0326.
fn core14_program_alternate(ch: char) -> Option<char> {
    match ch {
        '\u{02C9}' => Some('\u{00AF}'),
        '\u{2206}' => Some('\u{0394}'),
        '\u{F6C3}' => Some('\u{0326}'),
        _ => None,
    }
}

/// Redraws every glyph run of a Core 14 face in `used` with its program
/// ([`FontSet::core14_program`]): the run names the program's `font_id`
/// and each glyph its program glyph id; origins and advances (the AFM
/// metrics layout placed them with) are untouched, and `used` lists the
/// program instead of the metric face. A face without a program keeps its
/// runs as they are. Decided per face, never per glyph, so a windowed
/// render names the same fonts as the whole document.
pub fn embed_core14_programs(fonts: &FontSet, used: &mut BTreeMap<Rc<str>, Rc<LoadedFace>>, pages: &mut [crate::display::Page]) {
    let programs: Vec<(Rc<str>, Rc<Core14Program>)> =
        used.values().filter_map(|f| fonts.core14_program(f).map(|p| (f.font_id.clone(), p))).collect();
    if programs.is_empty() {
        return;
    }
    for (id, p) in &programs {
        used.remove(id);
        used.entry(p.face.font_id.clone()).or_insert_with(|| p.face.clone());
    }
    for page in pages.iter_mut() {
        let Some(items) = page.items_mut() else { continue };
        for item in items.iter_mut() {
            let crate::display::Item::GlyphRun(run) = item else { continue };
            let Some((_, p)) = programs.iter().find(|(id, _)| *id == run.font_id) else { continue };
            // Every Core 14 glyph has a program glyph (checked when the
            // program was loaded), so the whole run moves.
            for g in &mut run.glyphs {
                if let Some(to) = p.gid(g.gid) {
                    g.gid = to;
                }
            }
            run.font_id = p.face.font_id.clone();
        }
    }
}

pub struct Resolved {
    pub face: Rc<LoadedFace>,
    /// Set when the requested Latin Modern file was unavailable. The face
    /// returned is then Times, and the caller must publish this reason as
    /// an error diagnostic: the output is not the requested document.
    pub substituted: Option<String>,
    /// Set when the face's outlines are a stand-in for the requested design
    /// (no Latin Modern design exists, or its file is not installed) while
    /// the metrics are the requested ones; the caller reports it once.
    pub note: Option<String>,
    /// Set when a named family ([`Family::Named`]) was not found in the
    /// index (or its file failed to load): the face is Latin Modern's for
    /// the same shape, and the caller reports this once as a warning
    /// naming the family and the directories searched.
    pub family_missing: Option<String>,
}

impl Resolved {
    fn plain(face: Rc<LoadedFace>) -> Resolved {
        Resolved { face, substituted: None, note: None, family_missing: None }
    }
}

impl FontSet {
    /// Bounded search list: `FLASHTEX_FONT_DIRS` entries first, then any
    /// explicit extra directories, then [`default_font_dirs`].
    pub fn with_default_dirs(extra: &[PathBuf]) -> FontSet {
        let d = Discovery::from_process();
        let mut dirs = Discovery::split(&d.font_dirs);
        dirs.extend(extra.iter().cloned());
        dirs.extend(d.font_dirs());
        let tfm_dirs = d.tfm_dirs_for(&dirs);
        FontSet::with_dirs(dirs, tfm_dirs).with_exe_dir(d.exe_dir)
    }

    /// Everything [`Discovery`] finds, and nothing else: the set the
    /// packaged helper runs with.
    pub fn from_discovery(d: &Discovery) -> FontSet {
        let dirs = d.font_dirs();
        let tfm_dirs = d.tfm_dirs_for(&dirs);
        FontSet::with_dirs(dirs, tfm_dirs).with_exe_dir(d.exe_dir.clone())
    }

    /// Records the executable directory the search list was derived from,
    /// so diagnostics name those directories relative to it.
    pub fn with_exe_dir(mut self, exe_dir: Option<PathBuf>) -> FontSet {
        self.exe_dir = exe_dir;
        self
    }

    /// `dirs` for a diagnostic: see [`describe_dirs`].
    fn describe(&self, dirs: &[PathBuf]) -> String {
        describe_dirs(dirs, self.exe_dir.as_deref())
    }

    /// Whether the Latin Modern text and math faces the tests and the
    /// default document need are reachable through this set's directories.
    pub fn latin_modern_available(&self) -> bool {
        let has = |file: &str| self.dirs().iter().any(|d| d.join(file).is_file());
        has("lmroman12-regular.otf") && has("lmroman10-regular.otf") && has("latinmodern-math.otf")
    }

    /// Explicit font directories; TFMs come from `FLASHTEX_TFM_DIRS` and
    /// the directories' TeX Live / flat siblings (no bundle probing).
    pub fn new(dirs: Vec<PathBuf>) -> FontSet {
        let d = Discovery { tfm_dirs: std::env::var("FLASHTEX_TFM_DIRS").ok(), ..Discovery::default() };
        let tfm_dirs = d.tfm_dirs_for(&dirs);
        FontSet::with_dirs(dirs, tfm_dirs)
    }

    /// Explicit font and TFM directories, both searched in the given order.
    pub fn with_dirs(dirs: Vec<PathBuf>, tfm_dirs: Vec<PathBuf>) -> FontSet {
        let mut search = FontSearch::new();
        for d in dirs {
            search = search.with_dir(d);
        }
        FontSet {
            search,
            faces: RefCell::new(Vec::new()),
            by_name: RefCell::new(BTreeMap::new()),
            failures: RefCell::new(BTreeMap::new()),
            tfm_dirs,
            exe_dir: None,
            required: RefCell::new(None),
            required_flat: RefCell::new(false),
            tfms: RefCell::new(BTreeMap::new()),
            shaper: crate::shape::Shaper::new(),
            // Named families: the override directories, then the OS
            // defaults (`scan_dirs`); a project's `fonts/` is added per
            // render by `set_project_root`. Scanned lazily.
            index_dirs: RefCell::new(flashtex_font_discovery::scan_dirs(None)),
            index_dirs_explicit: false,
            index: RefCell::new(None),
            named: RefCell::new(Vec::new()),
            named_faces: RefCell::new(BTreeMap::new()),
            core14_programs: RefCell::new(Vec::new()),
        }
    }

    /// The shaping cache shared by every request on this font set.
    pub fn shaper(&self) -> &crate::shape::Shaper {
        &self.shaper
    }

    /// Replaces the directories named families are discovered in (tests
    /// and hermetic callers; the default is `scan_dirs(None)`). Drops any
    /// index and named faces already built.
    pub fn with_index_dirs(mut self, dirs: Vec<PathBuf>) -> FontSet {
        *self.index_dirs.borrow_mut() = dirs;
        self.index_dirs_explicit = true;
        *self.index.borrow_mut() = None;
        self.named_faces.borrow_mut().clear();
        self
    }

    /// Adds the project's own `fonts/` directory (when it exists) ahead of
    /// the OS defaults, the way `flashtex_font_discovery::scan_dirs` composes
    /// it. Called by `render` with `RenderOptions::project_root`; a change
    /// of project drops the index so the new project's fonts are seen. A
    /// set built with an explicit directory list keeps it.
    pub fn set_project_root(&self, root: Option<&Path>) {
        if self.index_dirs_explicit {
            return;
        }
        let dirs = flashtex_font_discovery::scan_dirs(root);
        if *self.index_dirs.borrow() != dirs {
            *self.index_dirs.borrow_mut() = dirs;
            *self.index.borrow_mut() = None;
            self.named_faces.borrow_mut().clear();
        }
    }

    /// The discovery index, scanned on first use.
    pub fn index(&self) -> Rc<FontIndex> {
        if let Some(i) = &*self.index.borrow() {
            return i.clone();
        }
        let index = Rc::new(FontIndex::scan(&self.index_dirs.borrow()));
        *self.index.borrow_mut() = Some(index.clone());
        index
    }

    /// Interns a named-family spec: the same spec gets the same id.
    pub fn intern_named(&self, spec: &NamedSpec) -> NamedId {
        let mut named = self.named.borrow_mut();
        if let Some(i) = named.iter().position(|s| s == spec) {
            return NamedId(i as u16);
        }
        named.push(spec.clone());
        NamedId((named.len() - 1) as u16)
    }

    /// The spec behind an id (`None` for an id this set never issued).
    pub fn named_spec(&self, id: NamedId) -> Option<NamedSpec> {
        self.named.borrow().get(usize::from(id.0)).cloned()
    }

    /// The `texmf-dist` roots implied by the TFM directories
    /// (`<root>/fonts/tfm/public/lm`).
    fn texmf_roots(&self) -> Vec<PathBuf> {
        self.tfm_dirs
            .iter()
            .filter_map(|d| {
                let s = d.to_string_lossy();
                s.strip_suffix(&format!("/{REQUIRED_TFM_DIR}")).map(PathBuf::from)
            })
            .collect()
    }

    /// The required 12 pt metrics (see [`REQUIRED_TFMS`]), loaded through
    /// font-resources from the first `texmf-dist` root that satisfies the
    /// whole manifest. `Err` names what failed.
    pub fn required_metrics(&self) -> Result<Rc<RequiredMetrics>, String> {
        if let Some(r) = &*self.required.borrow() {
            return r.clone();
        }
        // texmf roots first (TeX Live / bundled tree), then every TFM
        // directory as a flat root.
        let candidates: Vec<(PathBuf, bool)> = self
            .texmf_roots()
            .into_iter()
            .map(|r| (r, false))
            .chain(self.tfm_dirs.iter().filter(|d| d.is_dir()).map(|d| (d.clone(), true)))
            .collect();
        let mut errors = Vec::new();
        let mut result = Err(String::new());
        for (root, flat) in &candidates {
            match ProjectRoot::open(root) {
                Ok(pr) => match RequiredMetrics::load(&pr, &required_manifest(*flat)) {
                    Ok(m) => {
                        result = Ok(Rc::new(m));
                        *self.required_flat.borrow_mut() = *flat;
                        break;
                    }
                    Err(e) => errors.push(format!(
                        "{}{}: {e:?}",
                        self.describe(std::slice::from_ref(root)),
                        if *flat { " (flat)" } else { "" }
                    )),
                },
                Err(e) => errors.push(format!("{}: {e:?}", self.describe(std::slice::from_ref(root)))),
            }
        }
        if result.is_err() {
            result = Err(if candidates.is_empty() {
                format!(
                    "no TFM directory exists among ({})",
                    self.describe(&self.tfm_dirs)
                )
            } else {
                errors.join("; ")
            });
        }
        *self.required.borrow_mut() = Some(result.clone());
        result
    }

    /// A TFM by file name: the digest-bound required set when it holds
    /// the file, else the search directories through the shared parser.
    pub fn tfm(&self, file: &str) -> Result<Rc<Tfm>, TfmStatus> {
        if REQUIRED_TFMS.iter().any(|(f, _)| *f == file) {
            let set = self.required_metrics().map_err(TfmStatus::RequiredUnavailable)?;
            let key = if *self.required_flat.borrow() { file.to_string() } else { format!("{REQUIRED_TFM_DIR}/{file}") };
            let (_, t) = set.get(&key).map_err(|e| TfmStatus::RequiredUnavailable(format!("{e:?}")))?;
            return Ok(Rc::new(Tfm::from_shared(t.clone())));
        }
        if let Some(r) = self.tfms.borrow().get(file) {
            return r.clone().map_err(TfmStatus::Missing);
        }
        let r = match self.tfm_dirs.iter().map(|d| d.join(file)).find(|p| p.is_file()) {
            Some(p) => Tfm::load(&p).map(Rc::new),
            None => Err(format!(
                "{file} not found in {}",
                self.describe(&self.tfm_dirs)
            )),
        };
        self.tfms.borrow_mut().insert(file.to_string(), r.clone());
        r.map_err(TfmStatus::Missing)
    }

    pub fn dirs(&self) -> &[PathBuf] {
        self.search.dirs()
    }

    /// Where `.tfm` files are looked for (see [`default_tfm_dirs`]).
    pub fn tfm_dirs(&self) -> &[PathBuf] {
        &self.tfm_dirs
    }

    /// Every face loaded so far, in load order.
    pub fn loaded(&self) -> Vec<Rc<LoadedFace>> {
        self.faces.borrow().clone()
    }

    pub fn failures(&self) -> Vec<(String, String)> {
        self.failures.borrow().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub fn by_name(&self, name: &str) -> Option<Rc<LoadedFace>> {
        let idx = *self.by_name.borrow().get(name)?;
        self.faces.borrow().get(idx).cloned()
    }

    pub fn by_font_id(&self, font_id: &str) -> Option<Rc<LoadedFace>> {
        self.faces.borrow().iter().find(|f| &*f.font_id == font_id).cloned()
    }

    /// Latin Modern file for a role at `size_pt` ([`latin_modern_outline`]:
    /// the design-size boundaries of `t1lmr.fd`/`t1lmss.fd`/`t1lmtt.fd`).
    pub fn latin_modern_file(role: Role, size_pt: f64) -> String {
        match role.key() {
            None => "latinmodern-math.otf".to_string(),
            Some(key) => latin_modern_outline(key, size_pt).0,
        }
    }

    /// The Core 14 face of a role under `\usepackage{times}`: Times for the
    /// roman family (slanted shapes as italic, small caps as roman),
    /// Helvetica for `\sffamily` and Courier for `\ttfamily` (the `helvet`/
    /// `courier` families `times.sty` selects; Core 14 metrics have no
    /// bold or oblique Helvetica/Courier here).
    fn core14_for(role: Role) -> Core14 {
        let Some(key) = role.key() else { return Core14::Symbol };
        match (key.family, key.bold(), key.slanted()) {
            (FamilyKind::Sf, ..) => Core14::Helvetica,
            (FamilyKind::Tt, ..) => Core14::Courier,
            (FamilyKind::Rm, false, false) => Core14::TimesRoman,
            (FamilyKind::Rm, true, false) => Core14::TimesBold,
            (FamilyKind::Rm, false, true) => Core14::TimesItalic,
            (FamilyKind::Rm, true, true) => Core14::TimesBoldItalic,
        }
    }

    /// Resolves (and loads once) the face for `family`/`role` at `size_pt`.
    ///
    /// A text shape whose Latin Modern file is not installed (a sans,
    /// typewriter or small-caps design missing from a bundle) keeps its own
    /// metrics and draws the roman design of the same weight and slant, with
    /// [`Resolved::note`] saying so; only a missing roman design falls back
    /// to Times and is an error.
    pub fn resolve(&self, family: Family, role: Role, size_pt: f64) -> Resolved {
        let key = role.key();
        if family == Family::Times && key.is_some() {
            return Resolved::plain(self.core14(Self::core14_for(role)));
        }
        if let (Family::Named(id), Some(key)) = (family, key) {
            return self.resolve_named(id, key, size_pt);
        }
        let file = Self::latin_modern_file(role, size_pt);
        let note = key.and_then(|k| latin_modern_outline(k, size_pt).1).map(|n| format!("{file}: {n}"));
        let ec = match family {
            Family::ComputerModern => ec_tfm_file(role, size_pt),
            Family::ComputerModernOt1 => ot1_tfm_file(role, size_pt),
            _ => None,
        };
        let loaded = match &ec {
            Some(ec) => self.otf_with_tfm(&file, Some(ec), size_pt),
            None => self.otf(&file),
        };
        let reason = match loaded {
            Ok(f) => return Resolved { note, ..Resolved::plain(f) },
            Err(reason) => reason,
        };
        if let Some(key) = key {
            let roman = FontKey::new(FamilyKind::Rm, if key.bold() { Series::Bx } else { Series::M }, if key.slanted() { Shape::It } else { Shape::N });
            let roman_file = latin_modern_outline(roman, size_pt).0;
            if roman_file != file {
                let metrics = ec.clone().or_else(|| latin_modern_tfm(file.trim_end_matches(".otf")));
                if let Ok(face) = self.otf_with_tfm(&roman_file, metrics.as_deref(), size_pt) {
                    return Resolved {
                        note: Some(format!(
                            "{file}: {reason}; outlines drawn from {roman_file} with the {} metrics",
                            metrics.as_deref().unwrap_or("OpenType")
                        )),
                        ..Resolved::plain(face)
                    };
                }
            }
        }
        Resolved {
            substituted: Some(format!("{file}: {reason}")),
            ..Resolved::plain(self.core14(Self::core14_for(role)))
        }
    }

    /// A named family for an NFSS shape: the family's face at weight 700
    /// (`bx`/`b`) or 400 and the shape's slant, through the index, or the
    /// explicit `BoldFont=`/`ItalicFont=`/`BoldItalicFont=` name when the
    /// spec gives one (fontspec §4.1: those name a font, matched here by
    /// family, full or PostScript name). Small caps are not applied (GSUB
    /// `smcp` is not in the shaper): the same-weight upright or slanted
    /// face is used and the note says so. A family the index does not
    /// have falls back to Latin Modern's face for the same shape with
    /// [`Resolved::family_missing`] set.
    fn resolve_named(&self, id: NamedId, key: FontKey, size_pt: f64) -> Resolved {
        let latin_modern = |this: &FontSet| this.resolve(Family::LatinModern, Role::Font(key), size_pt);
        let Some(spec) = self.named_spec(id) else {
            return Resolved { family_missing: Some(format!("named family #{} was never interned on this font set", id.0)), ..latin_modern(self) };
        };
        let (bold, italic) = (key.bold(), key.slanted());
        let cache_key = (id.0, bold, italic);
        let cached = self.named_faces.borrow().get(&cache_key).cloned();
        let result = match cached {
            Some(r) => r,
            None => {
                let r = self.load_named(&spec, bold, italic);
                self.named_faces.borrow_mut().insert(cache_key, r.clone());
                r
            }
        };
        match result {
            Ok(r) => {
                // Small caps are the face's own `smcp` substitutions
                // (`crate::shape`, `ShapeFlags::SMALL_CAPS`); a face without
                // the feature sets the full-size letters and says so.
                let caps = (matches!(key.shape, Shape::Sc | Shape::Scit | Shape::Scsl) && r.face.feature_map(b"smcp").is_none()).then(|| {
                    format!(
                        "{}: \\scshape asks for small caps but {} has no GSUB `smcp` feature; the {} face is used as is",
                        spec.family,
                        r.face.name,
                        if italic { "italic" } else { "upright" }
                    )
                });
                // Both notes go through `note`; the typesetter keys its
                // once-only report on the text, so each is reported once.
                let note = match (r.substituted, caps) {
                    (Some(s), Some(c)) => Some(format!("{s}; {c}")),
                    (s, c) => s.or(c),
                };
                Resolved { note, ..Resolved::plain(r.face) }
            }
            Err(reason) => Resolved { family_missing: Some(reason), ..latin_modern(self) },
        }
    }

    /// Finds and loads the face of `spec` for a weight and slant.
    fn load_named(&self, spec: &NamedSpec, bold: bool, italic: bool) -> Result<NamedResolution, String> {
        let index = self.index();
        let weight = if bold { 700 } else { 400 };
        let explicit = match (bold, italic) {
            (true, true) => spec.bold_italic_font.as_deref(),
            (true, false) => spec.bold_font.as_deref(),
            (false, true) => spec.italic_font.as_deref(),
            (false, false) => spec.upright_font.as_deref(),
        };
        let (m, explicit_name) = match explicit.and_then(|name| index.find_match(name, weight, italic).map(|m| (m, Some(name)))) {
            Some(found) => found,
            None => {
                let m = index.find_match(&spec.family, weight, italic).ok_or_else(|| {
                    format!(
                        "font family \"{}\" not found among the {} faces indexed in {}",
                        spec.family,
                        index.files().len(),
                        index.dirs().iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
                    )
                })?;
                (m, None)
            }
        };
        let face = self.load_file(m.file)?;
        let substituted = if m.exact() {
            None
        } else {
            let asked = match (bold, italic) {
                (true, true) => "bold italic (700)",
                (true, false) => "bold (700)",
                (false, true) => "italic (400)",
                (false, false) => "regular (400)",
            };
            Some(format!(
                "{}: no {asked} face{}; {} (weight {}{}) used",
                spec.family,
                explicit_name.map(|n| format!(" named \"{n}\"")).unwrap_or_default(),
                m.file.info.full_name,
                m.file.info.weight,
                if m.file.info.italic { ", italic" } else { "" }
            ))
        };
        Ok(NamedResolution { face, substituted, caps_note: None })
    }

    /// Loads one indexed file/face (once; later calls return the same
    /// `Rc`). Both outline formats are accepted: `CFF ` faces get this
    /// crate's charstring bounds, `glyf` faces their glyph headers.
    pub fn load_file(&self, file: &FontFile) -> Result<Rc<LoadedFace>, String> {
        self.load_path(file.display_name(), &file.path, file.face_index)
    }

    /// [`FontSet::load_file`] by path: face `face_index` of `path`, loaded
    /// once under `name`.
    fn load_path(&self, name: String, path: &Path, face_index: u32) -> Result<Rc<LoadedFace>, String> {
        struct File {
            path: PathBuf,
            face_index: u32,
        }
        let file = File { path: path.to_path_buf(), face_index };
        if let Some(existing) = self.by_name(&name) {
            return Ok(existing);
        }
        let key = format!("{}#{}", file.path.display(), file.face_index);
        if let Some(reason) = self.failures.borrow().get(&key) {
            return Err(reason.clone());
        }
        let fail = |reason: String| -> String {
            self.failures.borrow_mut().insert(key.clone(), reason.clone());
            reason
        };
        let face = flashtex_font_engine::load_from_path_index(&file.path, file.face_index).map_err(|e| fail(format!("{}: {e}", file.path.display())))?;
        let (format, cff) = match face.outlines() {
            Outlines::Cff => {
                let table = face.cff_table().ok_or_else(|| fail("OTTO face without CFF table".into()))?;
                let cff = Cff::parse(table).map_err(|e| fail(format!("CFF: {e}")))?;
                if cff.num_glyphs() != usize::from(face.num_glyphs()) {
                    return Err(fail(format!("CFF has {} charstrings but maxp says {}", cff.num_glyphs(), face.num_glyphs())));
                }
                ("opentype-cff", Some(cff))
            }
            Outlines::Glyf => ("static-truetype", None),
        };
        // The wire id is the raw file's digest for face 0 (what every
        // explicit-file face publishes); another member of a collection
        // takes font-engine's bytes ‖ index hash so the two never collide.
        let sha = if file.face_index == 0 { sha256::digest(face.program()) } else { face.id().content_sha256 };
        let engine_id = sha256::hex(&face.id().content_sha256);
        let loaded = LoadedFace {
            font_id: Rc::from(sha256::hex(&sha)),
            engine_id,
            name: name.clone(),
            sha256: sha,
            byte_length: face.program().len() as u64,
            units_per_em: u32::from(face.units_per_em()),
            glyph_count: u32::from(face.num_glyphs()),
            postscript_name: face.postscript_name().to_string(),
            format,
            path: Some(file.path.clone()),
            face_index: file.face_index,
            kind: FaceKind::Otf { face, cff },
            tfm: None,
            encoding: crate::ids::Encoding::T1,
            ts1_tfm: None,
            tfm_missing: None,
            tfm_status: TfmStatus::Missing("named family: OpenType metrics by design".into()),
            shape_key: Rc::from(sha256::hex(&sha)),
            metrics_fallback: None,
            bounds_cache: RefCell::new(BTreeMap::new()),
            feature_maps: RefCell::new(BTreeMap::new()),
        };
        Ok(self.insert(name, loaded))
    }

    fn core14(&self, which: Core14) -> Rc<LoadedFace> {
        // Look the face up before constructing it: `Core14Face::new` hashes
        // the metrics (SHA-256) and `resolve` runs once per word.
        if let Some(existing) = self.by_name(which.header().font_name) {
            return existing;
        }
        let f = Core14Face::new(which);
        let name = f.postscript_name().to_string();
        if let Some(existing) = self.by_name(&name) {
            return existing;
        }
        let sha = f.id().content_sha256;
        let loaded = LoadedFace {
            font_id: Rc::from(sha256::hex(&sha)),
            engine_id: sha256::hex(&sha),
            name: name.clone(),
            sha256: sha,
            byte_length: 0,
            units_per_em: u32::from(f.units_per_em()),
            glyph_count: u32::from(f.num_glyphs()),
            postscript_name: f.postscript_name().to_string(),
            format: "core14-afm",
            path: None,
            face_index: 0,
            kind: FaceKind::Core14(f),
            tfm: None,
            encoding: crate::ids::Encoding::T1,
            ts1_tfm: None,
            tfm_missing: None,
            tfm_status: TfmStatus::Missing("Core 14 face: AFM metrics".into()),
            shape_key: Rc::from(sha256::hex(&sha)),
            metrics_fallback: None,
            bounds_cache: RefCell::new(BTreeMap::new()),
            feature_maps: RefCell::new(BTreeMap::new()),
        };
        self.insert(name, loaded)
    }

    /// The program `face` (a Core 14 metric face) is drawn with on output,
    /// when one is found and covers every glyph of the face; `None` for any
    /// other face, for Symbol, and when the file is missing (the exact PDF
    /// route then still refuses the Core 14 face, as before). Looked up in
    /// the font search list (the bundled `Fonts` directory), then in each
    /// Latin Modern TeX directory's `tex-gyre` sibling, then in
    /// [`TEX_GYRE_DIRS`].
    pub fn core14_program(&self, face: &LoadedFace) -> Option<Rc<Core14Program>> {
        let FaceKind::Core14(core14) = &face.kind else { return None };
        let which = core14.which();
        if let Some((_, p)) = self.core14_programs.borrow().iter().find(|(w, _)| *w == which) {
            return p.clone();
        }
        let program = self.load_core14_program(core14).map(Rc::new);
        self.core14_programs.borrow_mut().push((which, program.clone()));
        program
    }

    fn load_core14_program(&self, core14: &Core14Face) -> Option<Core14Program> {
        let file = core14_program_file(core14.which())?;
        let mut dirs: Vec<PathBuf> = self.search.dirs().to_vec();
        for d in self.search.dirs() {
            let s = d.to_string_lossy();
            if let Some(at) = s.find("/fonts/opentype/public/lm") {
                dirs.push(PathBuf::from(format!("{}/fonts/opentype/public/tex-gyre", &s[..at])));
            }
        }
        dirs.extend(TEX_GYRE_DIRS.iter().map(PathBuf::from));
        let path = dirs.iter().map(|d| d.join(file)).find(|p| p.is_file())?;
        let face = self.load_path(file.trim_end_matches(".otf").to_string(), &path, 0).ok()?;
        let otf = face.otf()?;
        let mut gids = vec![0u16; usize::from(core14.num_glyphs())];
        for (gid, slot) in gids.iter_mut().enumerate().skip(1) {
            let ch = core14.char_for(flashtex_font_engine::GlyphId(gid as u16))?;
            let g = otf.glyph_id(ch).or_else(|| core14_program_alternate(ch).and_then(|a| otf.glyph_id(a)))?;
            *slot = g.0;
        }
        Some(Core14Program { face, gids })
    }

    /// Loads an explicit file name from the bounded search list.
    pub fn otf(&self, file: &str) -> Result<Rc<LoadedFace>, String> {
        self.otf_with_tfm(file, None, 10.0)
    }

    /// [`FontSet::otf`] laid out with the EC or OT1 metric file `ec_tfm`
    /// instead of the `ec-lm*` TFM paired with the file. The face is a
    /// separate entry named `<stem>+<tfm stem>` (same program and wire
    /// `font_id`, its own `shape_key`). When `ec_tfm` is not found the
    /// `ec-lm*` TFM is attached and [`LoadedFace::metrics_fallback`] says
    /// so. `size_pt` selects the TS1 companion of an OT1 file (`cmr10` is
    /// loaded at several sizes, each with its own `tcrm<size>`; the face is
    /// then named `<stem>+<tfm>+<companion>`).
    pub fn otf_with_tfm(&self, file: &str, ec_tfm: Option<&str>, size_pt: f64) -> Result<Rc<LoadedFace>, String> {
        let stem = file.trim_end_matches(".otf").trim_end_matches(".ttf").to_string();
        let name = match ec_tfm {
            Some(t) => {
                let mut name = format!("{stem}+{}", t.trim_end_matches(".tfm"));
                if tfm_encoding(t) == crate::ids::Encoding::OT1 {
                    if let Some(c) = ts1_companion_tfm(t, size_pt) {
                        name.push('+');
                        name.push_str(c.trim_end_matches(".tfm"));
                    }
                }
                name
            }
            None => stem.clone(),
        };
        if let Some(existing) = self.by_name(&name) {
            return Ok(existing);
        }
        if let Some(reason) = self.failures.borrow().get(file) {
            return Err(reason.clone());
        }
        let fail = |reason: String| -> String {
            self.failures.borrow_mut().insert(file.to_string(), reason.clone());
            reason
        };
        let Some(path) = self.search.find(file) else {
            let n = self.search.dirs().len();
            return Err(fail(format!(
                "not found in {n} search director{}: {}; add a directory holding it with --font-dir or FLASHTEX_FONT_DIRS",
                if n == 1 { "y" } else { "ies" },
                self.describe(self.search.dirs())
            )));
        };
        let face = match self.search.load(file, 0) {
            Ok(f) => f,
            Err(e) => return Err(fail(e.to_string())),
        };
        let (format, cff) = match face.outlines() {
            Outlines::Cff => {
                let table = face.cff_table().ok_or_else(|| fail("OTTO face without CFF table".into()))?;
                let cff = Cff::parse(table).map_err(|e| fail(format!("CFF: {e}")))?;
                if cff.num_glyphs() != usize::from(face.num_glyphs()) {
                    return Err(fail(format!(
                        "CFF has {} charstrings but maxp says {}",
                        cff.num_glyphs(),
                        face.num_glyphs()
                    )));
                }
                ("opentype-cff", cff)
            }
            Outlines::Glyf => {
                return Err(fail("glyf outlines are not used by this pipeline (Latin Modern is CFF)".into()));
            }
        };
        // The published digest is over the raw file bytes; the engine's id
        // (bytes ‖ face index) is a different value and is not the resource
        // digest rendering-core / font-resources verify.
        let sha = sha256::digest(face.program());
        let engine_id = sha256::hex(&face.id().content_sha256);
        let mut metrics_fallback = None;
        let tfm_choice = match ec_tfm {
            Some(ec) => match self.tfm(ec) {
                Ok(_) => Some(ec.to_string()),
                Err(_) => {
                    let lm = latin_modern_tfm(&stem);
                    metrics_fallback = Some(format!(
                        "{ec} ({} metrics) not found; {} used, so line breaks can differ from pdfLaTeX",
                        metric_family_label(ec),
                        lm.as_deref().unwrap_or("OpenType advances")
                    ));
                    lm
                }
            },
            None => latin_modern_tfm(&stem),
        };
        let (tfm, tfm_missing, tfm_status) = match &tfm_choice {
            Some(tfm_file) => match self.tfm(tfm_file) {
                Ok(t) => (Some(t), None, TfmStatus::Loaded),
                Err(TfmStatus::RequiredUnavailable(e)) => {
                    let msg = format!("required metric asset {tfm_file}: {e}");
                    (None, Some(msg.clone()), TfmStatus::RequiredUnavailable(msg))
                }
                Err(TfmStatus::Missing(e)) => (None, Some(e.clone()), TfmStatus::Missing(e)),
                Err(TfmStatus::Loaded) => unreachable!(),
            },
            None => (None, None, TfmStatus::Missing("no TFM pairs with this file".into())),
        };
        // The companion is best effort: pdfTeX would stop on a missing
        // `tcrm1095.tfm`, but a bundle without the `tc*` files still sets
        // the symbol, from the program's advance, as it always has.
        let ts1_tfm = tfm
            .as_ref()
            .and_then(|_| tfm_choice.as_deref())
            .and_then(|t| ts1_companion_tfm(t, size_pt))
            .and_then(|f| self.tfm(&f).ok());
        let encoding = tfm.as_ref().and_then(|_| tfm_choice.as_deref()).map_or(crate::ids::Encoding::T1, tfm_encoding);
        let loaded = LoadedFace {
            font_id: Rc::from(sha256::hex(&sha)),
            engine_id,
            name: name.clone(),
            sha256: sha,
            byte_length: face.program().len() as u64,
            units_per_em: u32::from(face.units_per_em()),
            glyph_count: u32::from(face.num_glyphs()),
            postscript_name: face.postscript_name().to_string(),
            format,
            path: Some(path),
            face_index: 0,
            kind: FaceKind::Otf { face, cff: Some(cff) },
            tfm,
            encoding,
            ts1_tfm,
            tfm_missing,
            tfm_status,
            shape_key: match ec_tfm {
                None => Rc::from(sha256::hex(&sha)),
                Some(t) => Rc::from(format!("{}+{t}", sha256::hex(&sha))),
            },
            metrics_fallback,
            bounds_cache: RefCell::new(BTreeMap::new()),
            feature_maps: RefCell::new(BTreeMap::new()),
        };
        Ok(self.insert(name, loaded))
    }

    fn insert(&self, name: String, loaded: LoadedFace) -> Rc<LoadedFace> {
        let rc = Rc::new(loaded);
        let mut faces = self.faces.borrow_mut();
        self.by_name.borrow_mut().insert(name, faces.len());
        faces.push(rc.clone());
        rc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn lm_available() -> bool {
        FontSet::with_default_dirs(&[]).latin_modern_available()
    }

    #[test]
    fn optical_sizes_follow_t1lmr_fd() {
        assert_eq!(FontSet::latin_modern_file(Role::Text { bold: false, italic: false }, 12.0), "lmroman12-regular.otf");
        assert_eq!(FontSet::latin_modern_file(Role::Text { bold: false, italic: false }, 10.0), "lmroman10-regular.otf");
        assert_eq!(FontSet::latin_modern_file(Role::Text { bold: false, italic: false }, 17.28), "lmroman17-regular.otf");
        assert_eq!(FontSet::latin_modern_file(Role::Text { bold: true, italic: false }, 14.4), "lmroman12-bold.otf");
        assert_eq!(FontSet::latin_modern_file(Role::Text { bold: false, italic: false }, 8.0), "lmroman8-regular.otf");
    }

    #[test]
    fn latin_modern_glyph_bounds_match_the_design() {
        if !lm_available() {
            eprintln!("skipping: Latin Modern not installed");
            return;
        }
        let set = FontSet::with_default_dirs(&[]);
        let r = set.resolve(Family::LatinModern, Role::Text { bold: false, italic: false }, 10.0);
        assert!(r.substituted.is_none());
        let f = r.face;
        assert_eq!(f.format, "opentype-cff");
        assert_eq!(f.units_per_em, 1000);
        let x = f.face().glyph_id('x').unwrap();
        let b = f.bounds(x, Some('x'));
        // Computer Modern x-height is 430.55 units (lmr10 TFM fontdimen 5).
        assert!((b.y_max - 431).abs() <= 2, "x y_max {}", b.y_max);
        assert_eq!(b.y_min, 0);
        let h = f.face().glyph_id('H').unwrap();
        let hb = f.bounds(h, Some('H'));
        // Cap height 683.33 units in lmr10.
        assert!((hb.y_max - 683).abs() <= 2, "H y_max {}", hb.y_max);
        let p = f.face().glyph_id('p').unwrap();
        let pb = f.bounds(p, Some('p'));
        // Descender depth 194.44 units.
        assert!((pb.y_min + 194).abs() <= 2, "p y_min {}", pb.y_min);
        let sp = f.face().glyph_id(' ').unwrap();
        assert!(f.bounds(sp, Some(' ')).empty);
        // Ids are content hashes, distinct per file.
        let b12 = set.resolve(Family::LatinModern, Role::Text { bold: false, italic: false }, 12.0).face;
        assert_ne!(f.font_id, b12.font_id);
        assert_eq!(f.font_id.len(), 64);
    }

    #[test]
    fn missing_font_diagnostics_do_not_depend_on_the_executable_location() {
        let message = |exe: &str| {
            let d = Discovery { exe_dir: Some(PathBuf::from(exe)), ..Discovery::default() };
            let set = FontSet::from_discovery(&d);
            let Err(otf) = set.otf("flashtex-no-such-font.otf") else { panic!("font unexpectedly found") };
            let tfm = match set.tfm("flashtex-no-such-metrics.tfm") {
                Err(TfmStatus::Missing(m)) => m,
                other => panic!("unexpected {:?}", other.map(|_| ())),
            };
            (otf, tfm)
        };
        let a = message("/opt/flashtex-a/bin");
        let b = message("/Users/someone/Applications/FlashTeX.app/Contents/MacOS");
        assert_eq!(a, b);
        for m in [&a.0, &a.1] {
            assert!(!m.contains("flashtex-a") && !m.contains("someone"), "{m}");
            assert!(m.contains("<executable-dir>/../Resources/texmf/fonts"), "{m}");
        }
        assert!(a.0.contains("--font-dir"), "{}", a.0);
        assert_eq!(
            describe_dirs(&[PathBuf::from("/x/bin"), PathBuf::from("/x/bin/Fonts"), PathBuf::from("/usr/share/fonts")], Some(Path::new("/x/bin"))),
            "<executable-dir>, <executable-dir>/Fonts, /usr/share/fonts"
        );
    }

    /// `ot1cmr.fd`/`ot1cmss.fd` (TeX Live 2026) as transcribed on
    /// [`ot1_tfm_file`]: an 11pt article's body is `cmr10` at 10.95pt
    /// (pdflatex's `\showbox` names it `\OT1/cmr/m/n/10.95`), its `\large`
    /// `cmr12`, its `\Large` `cmr17`; beamer's `\large` frame title
    /// `cmss12`.
    #[test]
    fn ot1_cmr_sizes_select_knuths_files_of_ot1cmr_fd() {
        let rm = Role::Text { bold: false, italic: false };
        let bf = Role::Text { bold: true, italic: false };
        let it = Role::Text { bold: false, italic: true };
        assert_eq!(ot1_tfm_file(rm, 10.95).as_deref(), Some("cmr10.tfm"));
        assert_eq!(ot1_tfm_file(rm, 10.0).as_deref(), Some("cmr10.tfm"));
        assert_eq!(ot1_tfm_file(rm, 12.0).as_deref(), Some("cmr12.tfm"));
        assert_eq!(ot1_tfm_file(rm, 14.4).as_deref(), Some("cmr12.tfm"));
        assert_eq!(ot1_tfm_file(rm, 17.28).as_deref(), Some("cmr17.tfm"));
        assert_eq!(ot1_tfm_file(rm, 24.88).as_deref(), Some("cmr17.tfm"));
        assert_eq!(ot1_tfm_file(rm, 8.0).as_deref(), Some("cmr8.tfm"));
        assert_eq!(ot1_tfm_file(rm, 5.0).as_deref(), Some("cmr5.tfm"));
        assert_eq!(ot1_tfm_file(bf, 10.95).as_deref(), Some("cmbx10.tfm"));
        assert_eq!(ot1_tfm_file(bf, 9.0).as_deref(), Some("cmbx9.tfm"));
        assert_eq!(ot1_tfm_file(bf, 14.4).as_deref(), Some("cmbx12.tfm"));
        assert_eq!(ot1_tfm_file(it, 10.95).as_deref(), Some("cmti10.tfm"));
        assert_eq!(ot1_tfm_file(it, 7.0).as_deref(), Some("cmti7.tfm"));
        assert_eq!(ot1_tfm_file(it, 8.0).as_deref(), Some("cmti8.tfm"));
        assert_eq!(ot1_tfm_file(it, 12.0).as_deref(), Some("cmti12.tfm"));
        let sf = |bold: bool, shape: Shape| Role::Font(FontKey::new(FamilyKind::Sf, if bold { Series::Bx } else { Series::M }, shape));
        assert_eq!(ot1_tfm_file(sf(false, Shape::N), 10.95).as_deref(), Some("cmss10.tfm"));
        assert_eq!(ot1_tfm_file(sf(false, Shape::N), 12.0).as_deref(), Some("cmss12.tfm"));
        assert_eq!(ot1_tfm_file(sf(false, Shape::N), 7.0).as_deref(), Some("cmss8.tfm"));
        assert_eq!(ot1_tfm_file(sf(false, Shape::N), 20.74).as_deref(), Some("cmss17.tfm"));
        assert_eq!(ot1_tfm_file(sf(false, Shape::It), 10.95).as_deref(), Some("cmssi10.tfm"));
        assert_eq!(ot1_tfm_file(sf(true, Shape::N), 12.0).as_deref(), Some("cmssbx10.tfm"));
        assert_eq!(ot1_tfm_file(sf(true, Shape::It), 12.0).as_deref(), Some("cmssbx10.tfm"));
        let sc = Role::Font(FontKey::new(FamilyKind::Rm, Series::M, Shape::Sc));
        assert_eq!(ot1_tfm_file(sc, 10.95).as_deref(), Some("cmcsc10.tfm"));
        let tt = Role::Font(FontKey::new(FamilyKind::Tt, Series::M, Shape::N));
        assert_eq!(ot1_tfm_file(tt, 10.95), None);
        assert_eq!(ot1_tfm_file(Role::Math, 10.95), None);
        assert_eq!(tfm_encoding("cmr10.tfm"), crate::ids::Encoding::OT1);
        assert_eq!(tfm_encoding("cmssbx10.tfm"), crate::ids::Encoding::OT1);
        assert_eq!(tfm_encoding("ecrm1095.tfm"), crate::ids::Encoding::T1);
        assert_eq!(tfm_encoding("ec-lmr10.tfm"), crate::ids::Encoding::T1);
        // Companions of Knuth's files go by the size the file is used at.
        assert_eq!(ts1_companion_tfm("cmr10.tfm", 10.95).as_deref(), Some("tcrm1095.tfm"));
        assert_eq!(ts1_companion_tfm("cmr10.tfm", 10.0).as_deref(), Some("tcrm1000.tfm"));
        assert_eq!(ts1_companion_tfm("cmbx12.tfm", 14.4).as_deref(), Some("tcbx1440.tfm"));
        assert_eq!(ts1_companion_tfm("cmss12.tfm", 12.0).as_deref(), Some("tcss1200.tfm"));
        assert_eq!(ts1_companion_tfm("cmcsc10.tfm", 10.95).as_deref(), Some("tcrm1095.tfm"));
    }

    /// pdflatex (TeX Live 2026) `\showbox` in a 10pt article without
    /// `fontenc`: `\hbox{doing.''}` is 31.66676pt and `\hbox{Two}` in
    /// `\sffamily\large` 21.54836pt (`T`, `w`, `\kern-0.32639`, `o`) --
    /// `cmr10`/`cmss12` have no `.`–`”` or `T`–`w` kern where Latin
    /// Modern's `ec-lmr10`/`ec-lmss12` do, which put `doing.''` 1.65 bp and
    /// `Two columns` 1.17 bp short on fixtures/real-world/plain-article
    /// page 1 and beamer-blocks-columns page 3. `\hbox{Caf\'e}` is
    /// 19.72226pt (`C a f` + the `\accent` construction at `e`'s width,
    /// no kern), `\hbox{office---fine}` 47.77786pt (the `ffi` and `---`
    /// ligatures), `\hbox{\copyright}` 11.1084pt (`tcrm1000`).
    #[test]
    fn ot1_documents_lay_out_with_knuths_metrics() {
        let set = FontSet::with_default_dirs(&[]);
        if !set.latin_modern_available() || !set.tfm_dirs().iter().any(|d| d.join("cmr10.tfm").is_file()) {
            eprintln!("skipping: Latin Modern or the cm metrics not installed");
            return;
        }
        let shaper = crate::shape::Shaper::new();
        let rm = Role::Text { bold: false, italic: false };
        let cm = set.resolve(Family::ComputerModernOt1, rm, 10.0).face;
        let lm = set.resolve(Family::LatinModern, rm, 10.0).face;
        assert_eq!(cm.name, "lmroman10-regular+cmr10+tcrm1000");
        assert_eq!(cm.encoding, crate::ids::Encoding::OT1);
        assert_eq!(cm.font_id, lm.font_id);
        assert_ne!(cm.shape_key, lm.shape_key);
        let w = |face: &Rc<LoadedFace>, text: &str| shaper.shape(face, text).width_pt(10.0);
        assert!((w(&cm, "doing.”") - 31.66676).abs() < 1e-3, "{}", w(&cm, "doing.”"));
        assert!(w(&lm, "doing.”") < w(&cm, "doing.”") - 1.0, "{}", w(&lm, "doing.”"));
        let sf = Role::Font(FontKey::new(FamilyKind::Sf, Series::M, Shape::N));
        let cmss = set.resolve(Family::ComputerModernOt1, sf, 12.0).face;
        let lmss = set.resolve(Family::LatinModern, sf, 12.0).face;
        assert_eq!(cmss.name, "lmsans12-regular+cmss12+tcss1200");
        let w12 = |face: &Rc<LoadedFace>, text: &str| shaper.shape(face, text).width_pt(12.0);
        assert!((w12(&cmss, "Two") - 21.54836).abs() < 1e-3, "{}", w12(&cmss, "Two"));
        assert!(w12(&lmss, "Two") < w12(&cmss, "Two") - 0.9, "{}", w12(&lmss, "Two"));
        // The accented letter: the base's width, no kern, shaped by the TFM.
        let s = shaper.shape(&cm, "Café");
        assert!(s.tfm_metrics);
        assert!((s.width_pt(10.0) - 19.72226).abs() < 1e-3, "{}", s.width_pt(10.0));
        assert_eq!(s.clusters.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(), ["C", "a", "f", "é"]);
        // The f-ligatures and the dashes are OT1 slots of the TFM.
        let s = shaper.shape(&cm, "office—fine");
        assert!(s.tfm_metrics && s.clusters.iter().any(|c| c.text == "ffi"));
        assert!((s.width_pt(10.0) - 47.77786).abs() < 1e-3, "{}", s.width_pt(10.0));
        // `©` still comes from the companion: `tcrm1000` slot 169.
        assert!((w(&cm, "©") - 11.1084).abs() < 1e-3, "{}", w(&cm, "©"));
    }

    #[test]
    fn t1_cmr_sizes_select_the_ec_metric_files_of_t1cmr_fd() {
        let rm = Role::Text { bold: false, italic: false };
        assert_eq!(ec_tfm_file(rm, 10.95).as_deref(), Some("ecrm1095.tfm"));
        assert_eq!(ec_tfm_file(rm, 10.0).as_deref(), Some("ecrm1000.tfm"));
        assert_eq!(ec_tfm_file(rm, 9.0).as_deref(), Some("ecrm0900.tfm"));
        assert_eq!(ec_tfm_file(rm, 14.4).as_deref(), Some("ecrm1440.tfm"));
        assert_eq!(ec_tfm_file(Role::Text { bold: true, italic: false }, 12.0).as_deref(), Some("ecbx1200.tfm"));
        assert_eq!(ec_tfm_file(Role::Text { bold: true, italic: false }, 17.28).as_deref(), Some("ecbx1728.tfm"));
        assert_eq!(ec_tfm_file(Role::Text { bold: false, italic: true }, 10.95).as_deref(), Some("ecti1095.tfm"));
        assert_eq!(ec_tfm_file(Role::Text { bold: true, italic: true }, 10.95).as_deref(), Some("ecbi1095.tfm"));
        assert_eq!(ec_tfm_file(Role::Slanted, 10.95).as_deref(), Some("ecsl1095.tfm"));
        // An undeclared size substitutes the nearest declared one.
        assert_eq!(ec_tfm_file(rm, 10.5).as_deref(), Some("ecrm1095.tfm"));
        assert_eq!(ec_tfm_file(rm, 50.0).as_deref(), Some("ecrm3583.tfm"));
        assert_eq!(ec_tfm_file(Role::Math, 10.95), None);
    }

    #[test]
    fn computer_modern_shares_the_program_but_not_the_metrics_of_latin_modern() {
        let set = FontSet::with_default_dirs(&[]);
        if !set.latin_modern_available() {
            eprintln!("skipping: Latin Modern not installed");
            return;
        }
        let rm = Role::Text { bold: false, italic: false };
        let lm = set.resolve(Family::LatinModern, rm, 10.95).face;
        let cm = set.resolve(Family::ComputerModern, rm, 10.95).face;
        // Same OpenType program on the wire, separate shaping identity.
        assert_eq!(lm.font_id, cm.font_id);
        assert_ne!(lm.shape_key, cm.shape_key);
        assert_eq!(lm.name, "lmroman10-regular");
        let has_ec = set.tfm_dirs().iter().any(|d| d.join("ecrm1095.tfm").is_file());
        if has_ec {
            assert_eq!(cm.name, "lmroman10-regular+ecrm1095");
            assert!(cm.metrics_fallback.is_none());
            let (l, c) = (lm.tfm.as_ref().unwrap(), cm.tfm.as_ref().unwrap());
            assert_eq!(l.design_size_pt, 10.0);
            assert!((c.design_size_pt - 10.95).abs() < 1e-3);
            // Shaping goes through the face's own TFM, not a cached LM run.
            let shaper = crate::shape::Shaper::new();
            let (a, b) = (shaper.shape(&lm, "counterexample"), shaper.shape(&cm, "counterexample"));
            assert!(b.width_pt(10.95) < a.width_pt(10.95), "{} vs {}", b.width_pt(10.95), a.width_pt(10.95));
        } else {
            // No EC metrics: Latin Modern's TFM stands in, and that is said.
            assert!(cm.metrics_fallback.as_deref().is_some_and(|m| m.contains("ecrm1095.tfm")));
            assert!(cm.tfm.is_some());
        }
    }

    #[test]
    fn ts1_companions_follow_the_ts1_fd_files() {
        assert_eq!(ts1_companion_tfm("ecrm1095.tfm", 10.0).as_deref(), Some("tcrm1095.tfm"));
        assert_eq!(ts1_companion_tfm("ecbx1200.tfm", 10.0).as_deref(), Some("tcbx1200.tfm"));
        assert_eq!(ts1_companion_tfm("ecti1000.tfm", 10.0).as_deref(), Some("tcti1000.tfm"));
        assert_eq!(ts1_companion_tfm("ecss0800.tfm", 10.0).as_deref(), Some("tcss0800.tfm"));
        assert_eq!(ts1_companion_tfm("ectt1095.tfm", 10.0).as_deref(), Some("tctt1095.tfm"));
        // `TS1/cmr/m/sc` is undefined: pdflatex substitutes `m/n` (probe
        // log: "Font shape `TS1/cmr/m/sc' undefined ... using `TS1/cmr/m/n'").
        assert_eq!(ts1_companion_tfm("eccc1095.tfm", 10.0).as_deref(), Some("tcrm1095.tfm"));
        assert_eq!(ts1_companion_tfm("ecxc1095.tfm", 10.0).as_deref(), Some("tcbx1095.tfm"));
        assert_eq!(ts1_companion_tfm("ectc1000.tfm", 10.0).as_deref(), Some("tctt1000.tfm"));
        assert_eq!(ts1_companion_tfm("ec-lmr10.tfm", 10.0).as_deref(), Some("ts1-lmr10.tfm"));
        assert_eq!(ts1_companion_tfm("ec-lmbxi10.tfm", 10.0).as_deref(), Some("ts1-lmbxi10.tfm"));
        assert_eq!(ts1_companion_tfm("rm-lmr10.tfm", 10.0), None);
    }

    /// pdflatex (TeX Live 2026) `\showbox` of `\hbox{a\copyright b}` in an
    /// 11pt `[T1]{fontenc}` article: `\T1/cmr/m/n/10.95 a`, `\TS1/cmr/m/n/10.95
    /// ©` (`tcrm1095` slot 169, CHARWD 1.11084), `\T1/cmr/m/n/10.95 b`, the
    /// box `hbox(8.21059+2.7369)x23.58533`; `\hbox{\textbf{a\copyright b}}`
    /// is 26.92859 (`tcbx1095`), `\hbox{\textregistered}` 12.093,
    /// `\hbox{\texttrademark}` 7.25731, `\hbox{\textdegree}` 3.63054. Latin
    /// Modern Roman's own `©` advance is 0.683 em: without the companion the
    /// first box was 18.98pt, and `\copyright~2026;` in
    /// fixtures/real-world/unicode-accents sat 4.60bp left of the reference.
    #[test]
    fn ts1_symbols_take_the_companion_fonts_metrics() {
        let set = FontSet::with_default_dirs(&[]);
        if !set.latin_modern_available() || !set.tfm_dirs().iter().any(|d| d.join("tcrm1095.tfm").is_file()) {
            eprintln!("skipping: Latin Modern or the tc* companions not installed");
            return;
        }
        let shaper = crate::shape::Shaper::new();
        let width = |bold: bool, text: &str| -> f64 {
            let face = set.resolve(Family::ComputerModern, Role::Text { bold, italic: false }, 10.95).face;
            assert!(face.ts1_tfm.is_some(), "{}: no companion", face.name);
            shaper.shape(&face, text).width_pt(10.95)
        };
        assert!((width(false, "a©b") - 23.58533).abs() < 1e-3, "{}", width(false, "a©b"));
        assert!((width(true, "a©b") - 26.92859).abs() < 1e-3, "{}", width(true, "a©b"));
        assert!((width(false, "®") - 12.093).abs() < 1e-3, "{}", width(false, "®"));
        assert!((width(false, "™") - 7.25731).abs() < 1e-3, "{}", width(false, "™"));
        assert!((width(false, "°") - 3.63054).abs() < 1e-3, "{}", width(false, "°"));
        // The companion's box, not the outline's: `hbox(8.21059+2.7369)`.
        let face = set.resolve(Family::ComputerModern, Role::Text { bold: false, italic: false }, 10.95).face;
        let s = shaper.shape(&face, "a©b");
        assert!(s.tfm_metrics);
        assert!((s.height_pt(10.95) - 8.21059).abs() < 1e-3 && (s.depth_pt(10.95) - 2.7369).abs() < 1e-3, "{} {}", s.height_pt(10.95), s.depth_pt(10.95));
        // Three clusters: the symbol stands outside `a`/`b`'s ligkern run.
        assert_eq!(s.clusters.iter().map(|c| c.text.as_str()).collect::<Vec<_>>(), ["a", "©", "b"]);
    }

    #[test]
    fn missing_ec_metrics_fall_back_to_latin_modern_and_say_so() {
        let fonts = FontSet::with_default_dirs(&[]);
        if !fonts.latin_modern_available() {
            eprintln!("skipping: Latin Modern not installed");
            return;
        }
        // Only the Latin Modern TFM directories: no jknappen/ec.
        let tfm_dirs: Vec<PathBuf> = fonts.tfm_dirs().iter().filter(|d| !d.ends_with(EC_TFM_DIR)).cloned().collect();
        let set = FontSet::with_dirs(fonts.dirs().to_vec(), tfm_dirs);
        let cm = set.resolve(Family::ComputerModern, Role::Text { bold: false, italic: false }, 10.95).face;
        let note = cm.metrics_fallback.as_deref().expect("fallback reported");
        assert!(note.contains("ecrm1095.tfm") && note.contains("ec-lmr10.tfm"), "{note}");
        assert_eq!(cm.tfm_status, TfmStatus::Loaded);
    }

    #[test]
    fn ec_metrics_are_discovered_next_to_each_tex_live_tree_after_latin_modern() {
        let d = Discovery::default();
        let fonts = vec![PathBuf::from("/tl/texmf-dist/fonts/opentype/public/lm"), PathBuf::from("/flat/Fonts")];
        let dirs = d.tfm_dirs_for(&fonts);
        let lm = dirs.iter().position(|p| p == Path::new("/tl/texmf-dist/fonts/tfm/public/lm")).unwrap();
        let ec = dirs.iter().position(|p| p == Path::new("/tl/texmf-dist/fonts/tfm/jknappen/ec")).unwrap();
        assert!(lm < ec);
        let euler = dirs.iter().position(|p| p == Path::new("/tl/texmf-dist/fonts/tfm/public/amsfonts/euler")).unwrap();
        assert!(ec < euler);
        assert_eq!(dirs.last().unwrap(), Path::new("/tl/texmf-dist/fonts/tfm/public/amsfonts/euler"));
        assert!(!dirs.iter().any(|p| p.starts_with("/flat") && p.ends_with(EC_TFM_DIR)));
    }

    #[test]
    fn nfss_shapes_select_the_fd_metrics_and_latin_modern_designs() {
        use crate::nfss::{FamilyKind::*, FontKey, Series::*, Shape::*};
        let role = |f, s, sh| Role::Font(FontKey::new(f, s, sh));
        // The roman roles spelled the old way resolve identically.
        assert_eq!(Role::Text { bold: true, italic: false }.key(), role(Rm, Bx, N).key());
        assert_eq!(Role::Slanted.key(), role(Rm, M, Sl).key());
        // t1cmss.fd / t1cmtt.fd: `<5><6><7><8>ecss0800`, genb sizes above.
        assert_eq!(ec_tfm_file(role(Sf, M, N), 10.95).as_deref(), Some("ecss1095.tfm"));
        assert_eq!(ec_tfm_file(role(Sf, M, N), 6.0).as_deref(), Some("ecss0800.tfm"));
        assert_eq!(ec_tfm_file(role(Sf, M, It), 10.0).as_deref(), Some("ecsi1000.tfm"));
        assert_eq!(ec_tfm_file(role(Sf, Bx, Sl), 12.0).as_deref(), Some("ecso1200.tfm"));
        assert_eq!(ec_tfm_file(role(Tt, M, N), 8.0).as_deref(), Some("ectt0800.tfm"));
        assert_eq!(ec_tfm_file(role(Tt, M, Sc), 10.95).as_deref(), Some("ectc1095.tfm"));
        // t1cmr.fd shapes.
        assert_eq!(ec_tfm_file(role(Rm, M, Sc), 10.95).as_deref(), Some("eccc1095.tfm"));
        assert_eq!(ec_tfm_file(role(Rm, Bx, Sc), 10.0).as_deref(), Some("ecxc1000.tfm"));
        assert_eq!(ec_tfm_file(role(Rm, M, Scsl), 10.0).as_deref(), Some("ecsc1000.tfm"));
        assert_eq!(ec_tfm_file(role(Rm, Bx, Sl), 10.0).as_deref(), Some("ecbl1000.tfm"));
        // t1lmss.fd / t1lmtt.fd design sizes and the ec-lm* pairing.
        assert_eq!(FontSet::latin_modern_file(role(Sf, M, N), 10.95), "lmsans10-regular.otf");
        assert_eq!(FontSet::latin_modern_file(role(Sf, M, N), 14.4), "lmsans12-regular.otf");
        assert_eq!(FontSet::latin_modern_file(role(Sf, M, Sl), 17.28), "lmsans17-oblique.otf");
        assert_eq!(FontSet::latin_modern_file(role(Tt, M, N), 8.0), "lmmono8-regular.otf");
        assert_eq!(FontSet::latin_modern_file(role(Tt, B, N), 10.0), "lmmonolt10-bold.otf");
        assert_eq!(FontSet::latin_modern_file(role(Rm, M, Sc), 12.0), "lmromancaps10-regular.otf");
        for (stem, tfm) in [
            ("lmsans10-regular", "ec-lmss10.tfm"),
            ("lmsans12-oblique", "ec-lmsso12.tfm"),
            ("lmsans10-bold", "ec-lmssbx10.tfm"),
            ("lmsans10-boldoblique", "ec-lmssbo10.tfm"),
            ("lmromancaps10-regular", "ec-lmcsc10.tfm"),
            ("lmromancaps10-oblique", "ec-lmcsco10.tfm"),
            ("lmromanslant10-bold", "ec-lmbxo10.tfm"),
            ("lmromanslant12-regular", "ec-lmro12.tfm"),
            ("lmmono9-regular", "ec-lmtt9.tfm"),
            ("lmmono10-italic", "ec-lmtti10.tfm"),
            ("lmmonolt10-bold", "ec-lmtk10.tfm"),
            ("lmroman10-bolditalic", "ec-lmbxi10.tfm"),
        ] {
            assert_eq!(latin_modern_tfm(stem).as_deref(), Some(tfm), "{stem}");
        }
        // No Latin Modern bold small caps: the medium design, with a note.
        let (file, note) = latin_modern_outline(FontKey::new(Rm, Bx, Sc), 10.0);
        assert_eq!(file, "lmromancaps10-regular.otf");
        assert!(note.is_some());
    }

    #[test]
    fn missing_latin_modern_is_reported_not_silent() {
        let set = FontSet::new(vec![PathBuf::from("/nonexistent/flashtex-fonts")]);
        let r = set.resolve(Family::LatinModern, Role::Text { bold: false, italic: false }, 10.0);
        assert!(r.substituted.is_some());
        assert_eq!(r.face.format, "core14-afm");
        assert_eq!(set.failures().len(), 1);
    }
}
