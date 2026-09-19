//! FlashTeX font discovery: an index of the fonts installed on the machine
//! (and shipped with a project), answering the question fontspec's
//! `\setmainfont{Helvetica}` and typst's `#set text(font: "Helvetica")`
//! both ask -- *which file, which face, for this family at this weight and
//! style?* -- the way typst answers it: a case-insensitive family match,
//! then the nearest weight, then the nearest style.
//!
//! The index reads only font headers (`sfnt`): family and style names from
//! `name`, weight/width/slant from `OS/2`, and whether a `MATH` table is
//! present (the math half of the font system, `docs/proposals/
//! font-system-math.md`, selects its faces from [`FontIndex::math_fonts`]).
//! Loading, shaping and embedding a selected face stay in font-engine.
//!
//! **Platform-free by construction.** Nothing here calls a platform font
//! API (Core Text, fontconfig, DirectWrite); the only OS-specific code is
//! [`default_dirs`], one function with `cfg(target_os)` arms listing where
//! each OS keeps its fonts. The engine crates that depend on this therefore
//! run unchanged on macOS, Linux, Windows and, given a directory list,
//! anywhere else. The directories actually scanned are always explicit
//! ([`FontIndex::scan`]) -- `FLASHTEX_FONT_DIRS`, the project's `fonts/`
//! directory and the defaults are composed by [`scan_dirs`], never applied
//! silently by a lookup.

pub mod json;
pub mod sfnt;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub use sfnt::{FaceInfo, Outlines};

/// One face of one font file, as the index knows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFile {
    pub path: PathBuf,
    /// The face within a `.ttc` collection; 0 for a single-face file.
    pub face_index: u32,
    pub info: FaceInfo,
}

impl FontFile {
    /// The `name` the pipeline gives a loaded face: the file stem, plus the
    /// face index for a collection member (`Helvetica#2`).
    pub fn display_name(&self) -> String {
        let stem = self.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        if self.face_index == 0 {
            stem
        } else {
            format!("{stem}#{}", self.face_index)
        }
    }
}

/// The result of a [`FontIndex::find_match`]: the nearest face and how far
/// it is from what was asked, so a caller can say what it substituted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match<'a> {
    pub file: &'a FontFile,
    /// The requested weight class (100..=900).
    pub wanted_weight: u16,
    pub wanted_italic: bool,
    /// Whether the face has exactly the requested weight.
    pub exact_weight: bool,
    /// Whether the face has the requested slant.
    pub exact_style: bool,
}

impl Match<'_> {
    pub fn exact(&self) -> bool {
        self.exact_weight && self.exact_style
    }
}

/// The scanned index.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FontIndex {
    files: Vec<FontFile>,
    /// The directories scanned, in order, with the modification time the
    /// cache keys on (seconds since the epoch; 0 when unknown).
    dirs: Vec<(PathBuf, u64)>,
    /// Files that were skipped, with the reason (not fonts, unreadable).
    errors: Vec<(PathBuf, String)>,
}

/// How deep a font directory is walked (`/usr/share/fonts/truetype/dejavu`
/// is depth 2 under `/usr/share/fonts`).
const MAX_DEPTH: usize = 6;

/// Most files considered per scan, so a `FLASHTEX_FONT_DIRS` pointed at a
/// home directory stays a bounded walk.
const MAX_FILES: usize = 20_000;

impl FontIndex {
    /// Scans `dirs` (recursively, bounded) for `.otf`, `.ttf` and `.ttc`
    /// files. A directory that does not exist is skipped, not an error.
    /// The result is deterministic: entries are visited in sorted order.
    pub fn scan(dirs: &[PathBuf]) -> FontIndex {
        let mut index = FontIndex::default();
        let mut seen = BTreeSet::new();
        let mut budget = MAX_FILES;
        for dir in dirs {
            let mtime = dir_mtime(dir);
            index.dirs.push((dir.clone(), mtime));
            walk(dir, 0, &mut budget, &mut |path| {
                if !seen.insert(path.to_path_buf()) {
                    return;
                }
                match sfnt::read_faces(path) {
                    Ok(faces) => {
                        for (i, info) in faces.into_iter().enumerate() {
                            index.files.push(FontFile { path: path.to_path_buf(), face_index: i as u32, info });
                        }
                    }
                    Err(e) => index.errors.push((path.to_path_buf(), e)),
                }
            });
        }
        index
    }

    /// [`FontIndex::scan`] through a JSON cache at `cache_path`: the cache
    /// is used when it was written for the same directory list with the
    /// same modification times, else the directories are scanned and the
    /// cache rewritten (best effort; an unwritable cache is not an error).
    pub fn scan_cached(dirs: &[PathBuf], cache_path: &Path) -> FontIndex {
        let stamps: Vec<(PathBuf, u64)> = dirs.iter().map(|d| (d.clone(), dir_mtime(d))).collect();
        if let Ok(text) = std::fs::read_to_string(cache_path) {
            if let Some(index) = FontIndex::from_cache_json(&text) {
                if index.dirs == stamps {
                    return index;
                }
            }
        }
        let index = FontIndex::scan(dirs);
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(cache_path, index.to_cache_json());
        index
    }

    pub fn files(&self) -> &[FontFile] {
        &self.files
    }

    pub fn dirs(&self) -> Vec<PathBuf> {
        self.dirs.iter().map(|(d, _)| d.clone()).collect()
    }

    pub fn errors(&self) -> &[(PathBuf, String)] {
        &self.errors
    }

    /// Every family name, sorted, deduplicated -- except the hidden system
    /// families whose names start with a dot (`.SF NS`, `.Al Bayan PUA`),
    /// which macOS keeps out of every font menu; they stay findable by
    /// exact name through [`FontIndex::find`].
    pub fn families(&self) -> Vec<String> {
        let set: BTreeSet<&str> = self.files.iter().map(|f| f.info.family.as_str()).filter(|f| !f.starts_with('.')).collect();
        set.into_iter().map(str::to_string).collect()
    }

    /// The faces carrying an OpenType `MATH` table (Latin Modern Math, STIX
    /// Two Math, Libertinus Math, ...), in index order.
    pub fn math_fonts(&self) -> Vec<&FontFile> {
        self.files.iter().filter(|f| f.info.has_math).collect()
    }

    /// The nearest face of `family` at `weight` (100..=900) and slant:
    /// [`FontIndex::find_match`]'s file.
    pub fn find(&self, family: &str, weight: u16, italic: bool) -> Option<&FontFile> {
        self.find_match(family, weight, italic).map(|m| m.file)
    }

    /// typst-style matching, in three steps:
    ///
    /// 1. **Family.** Faces whose typographic family (`name` id 16, else 1)
    ///    equals `family` ignoring case and runs of whitespace. When none
    ///    does, the legacy family, the full name and the PostScript name
    ///    are tried the same way, so `\setmainfont{Helvetica Neue Light}`
    ///    and `BoldFont=Georgia-Bold` also resolve.
    /// 2. **Style.** Among those, faces whose slant matches `italic` win;
    ///    otherwise the other slant is used and `exact_style` is false.
    /// 3. **Weight.** The face whose `usWeightClass` is nearest `weight`,
    ///    the lighter one on a tie for weights up to 500 and the heavier one
    ///    above (CSS Fonts 4 §5.2 / typst `font.rs`), then the width nearest
    ///    normal (5), then the path and face index for determinism.
    pub fn find_match(&self, family: &str, weight: u16, italic: bool) -> Option<Match<'_>> {
        let wanted = normalize(family);
        if wanted.is_empty() {
            return None;
        }
        let mut candidates: Vec<&FontFile> = self.files.iter().filter(|f| normalize(&f.info.family) == wanted).collect();
        if candidates.is_empty() {
            candidates = self
                .files
                .iter()
                .filter(|f| {
                    normalize(&f.info.legacy_family) == wanted
                        || normalize(&f.info.full_name) == wanted
                        || normalize(&f.info.postscript_name) == wanted
                })
                .collect();
        }
        if candidates.is_empty() {
            return None;
        }
        let style_ok = candidates.iter().any(|f| f.info.italic == italic);
        let pool: Vec<&FontFile> = if style_ok { candidates.into_iter().filter(|f| f.info.italic == italic).collect() } else { candidates };
        let key = |f: &FontFile| {
            let w = f.info.weight;
            let dist = w.abs_diff(weight);
            // Direction on a tie: lighter first at or below 500, heavier above.
            let tie = if weight <= 500 { w > weight } else { w < weight };
            (dist, tie, f.info.width.abs_diff(5), f.path.clone(), f.face_index)
        };
        let best = pool.into_iter().min_by_key(|f| key(f))?;
        Some(Match {
            file: best,
            wanted_weight: weight,
            wanted_italic: italic,
            exact_weight: best.info.weight == weight,
            exact_style: best.info.italic == italic,
        })
    }

    /// The cache file's JSON.
    pub fn to_cache_json(&self) -> String {
        use json::Value;
        use std::collections::BTreeMap;
        let s = |v: &str| Value::Str(v.to_string());
        let n = |v: u64| Value::Num(v as f64);
        let mut root = BTreeMap::new();
        root.insert("version".into(), n(CACHE_VERSION));
        root.insert(
            "dirs".into(),
            Value::Arr(
                self.dirs
                    .iter()
                    .map(|(d, m)| {
                        let mut o = BTreeMap::new();
                        o.insert("path".into(), s(&d.to_string_lossy()));
                        o.insert("mtime".into(), n(*m));
                        Value::Obj(o)
                    })
                    .collect(),
            ),
        );
        root.insert(
            "faces".into(),
            Value::Arr(
                self.files
                    .iter()
                    .map(|f| {
                        let i = &f.info;
                        let mut o = BTreeMap::new();
                        o.insert("path".into(), s(&f.path.to_string_lossy()));
                        o.insert("face_index".into(), n(u64::from(f.face_index)));
                        o.insert("family".into(), s(&i.family));
                        o.insert("legacy_family".into(), s(&i.legacy_family));
                        o.insert("subfamily".into(), s(&i.subfamily));
                        o.insert("full_name".into(), s(&i.full_name));
                        o.insert("postscript_name".into(), s(&i.postscript_name));
                        o.insert("weight".into(), n(u64::from(i.weight)));
                        o.insert("width".into(), n(u64::from(i.width)));
                        o.insert("italic".into(), Value::Bool(i.italic));
                        o.insert("bold".into(), Value::Bool(i.bold));
                        o.insert("math".into(), Value::Bool(i.has_math));
                        o.insert("outlines".into(), s(match i.outlines {
                            Outlines::Glyf => "glyf",
                            Outlines::Cff => "cff",
                        }));
                        o.insert("units_per_em".into(), n(u64::from(i.units_per_em)));
                        o.insert("revision".into(), n(u64::from(i.revision)));
                        Value::Obj(o)
                    })
                    .collect(),
            ),
        );
        let mut out = String::new();
        json::write(&Value::Obj(root), &mut out);
        out.push('\n');
        out
    }

    /// The index a cache file describes; `None` for any other JSON or an
    /// older cache version (which is then simply rescanned).
    pub fn from_cache_json(text: &str) -> Option<FontIndex> {
        let root = json::parse(text).ok()?;
        if root.get("version")?.as_u64()? != CACHE_VERSION {
            return None;
        }
        let dirs = root
            .get("dirs")?
            .as_arr()?
            .iter()
            .map(|d| Some((PathBuf::from(d.get("path")?.as_str()?), d.get("mtime")?.as_u64()?)))
            .collect::<Option<Vec<_>>>()?;
        let files = root
            .get("faces")?
            .as_arr()?
            .iter()
            .map(|f| {
                let str_ = |k: &str| f.get(k).and_then(|v| v.as_str()).map(str::to_string);
                let num = |k: &str| f.get(k).and_then(|v| v.as_u64());
                let flag = |k: &str| f.get(k).and_then(|v| v.as_bool());
                Some(FontFile {
                    path: PathBuf::from(str_("path")?),
                    face_index: num("face_index")? as u32,
                    info: FaceInfo {
                        family: str_("family")?,
                        legacy_family: str_("legacy_family")?,
                        subfamily: str_("subfamily")?,
                        full_name: str_("full_name")?,
                        postscript_name: str_("postscript_name")?,
                        weight: num("weight")? as u16,
                        width: num("width")? as u16,
                        italic: flag("italic")?,
                        bold: flag("bold")?,
                        has_math: flag("math")?,
                        outlines: match str_("outlines")?.as_str() {
                            "glyf" => Outlines::Glyf,
                            "cff" => Outlines::Cff,
                            _ => return None,
                        },
                        units_per_em: num("units_per_em")? as u16,
                        revision: num("revision")? as u32,
                    },
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(FontIndex { files, dirs, errors: Vec::new() })
    }
}

const CACHE_VERSION: u64 = 1;

/// Family-name comparison key: lower-case, runs of whitespace collapsed to
/// one space, so `Latin  Modern Roman` and `latin modern roman` agree.
pub fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut space = false;
    for c in name.trim().chars() {
        if c.is_whitespace() {
            space = true;
            continue;
        }
        if space && !out.is_empty() {
            out.push(' ');
        }
        space = false;
        out.extend(c.to_lowercase());
    }
    out
}

fn dir_mtime(dir: &Path) -> u64 {
    std::fs::metadata(dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

fn walk(dir: &Path, depth: usize, budget: &mut usize, visit: &mut dyn FnMut(&Path)) {
    if depth > MAX_DEPTH || *budget == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if *budget == 0 {
            return;
        }
        let hidden = path.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if hidden {
            continue;
        }
        // `metadata` follows symlinks, so a linked font directory (Homebrew
        // casks, a project's `fonts -> ../shared`) is walked.
        let Ok(meta) = std::fs::metadata(&path) else { continue };
        if meta.is_dir() {
            walk(&path, depth + 1, budget, visit);
        } else if meta.is_file() && is_font_file(&path) {
            *budget -= 1;
            visit(&path);
        }
    }
}

fn is_font_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "otf" | "ttf" | "ttc"))
}

/// Where each operating system keeps its fonts: the one place in the engine
/// crates with `cfg(target_os)` arms. The user's own directory comes first
/// so a font installed for one user shadows the system copy, as the OS
/// itself resolves it. `$HOME`/`%WINDIR%`/`%LOCALAPPDATA%` are read from
/// the environment; an unset variable drops its entries.
pub fn default_dirs() -> Vec<PathBuf> {
    let var = |k: &str| std::env::var_os(k).map(PathBuf::from).filter(|p| !p.as_os_str().is_empty());
    let mut dirs = Vec::new();
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = var("HOME") {
            dirs.push(home.join("Library/Fonts"));
        }
        dirs.push(PathBuf::from("/Library/Fonts"));
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/System/Library/Fonts/Supplemental"));
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = var("HOME") {
            dirs.push(home.join(".fonts"));
            dirs.push(home.join(".local/share/fonts"));
        }
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(windir) = var("WINDIR") {
            dirs.push(windir.join("Fonts"));
        }
        if let Some(local) = var("LOCALAPPDATA") {
            dirs.push(local.join("Microsoft/Windows/Fonts"));
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = var;
    }
    dirs
}

/// `FLASHTEX_FONT_DIRS` (colon separated; semicolons on Windows), in order.
pub fn env_dirs() -> Vec<PathBuf> {
    let Some(v) = std::env::var_os("FLASHTEX_FONT_DIRS") else { return Vec::new() };
    let v = v.to_string_lossy();
    let sep = if cfg!(windows) { ';' } else { ':' };
    v.split(sep).filter(|s| !s.is_empty()).map(PathBuf::from).collect()
}

/// The directory list a document's fonts are found in, in precedence
/// order: `FLASHTEX_FONT_DIRS`, the project's own `fonts/` directory when
/// it exists (fonts shipped with the document, like typst's project-local
/// fonts), then [`default_dirs`]. Duplicates are dropped, first occurrence
/// wins.
pub fn scan_dirs(project_root: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = env_dirs();
    if let Some(root) = project_root {
        let fonts = root.join("fonts");
        if fonts.is_dir() {
            dirs.push(fonts);
        }
    }
    dirs.extend(default_dirs());
    let mut seen = BTreeSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temp directory holding copies of the bundled Latin Modern faces
    /// (`apps/mac/Fonts`), so the tests are hermetic: no system font, no
    /// `FLASHTEX_FONT_DIRS`.
    struct Staged(PathBuf);

    impl Staged {
        fn new(tag: &str, files: &[&str]) -> Staged {
            let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts");
            let dir = std::env::temp_dir().join(format!("flashtex-font-discovery-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("nested")).unwrap();
            for f in files {
                let dst = if f.starts_with("lmsans") { dir.join("nested").join(f) } else { dir.join(f) };
                std::fs::copy(src.join(f), dst).unwrap_or_else(|e| panic!("{f}: {e}"));
            }
            std::fs::write(dir.join("notes.txt"), "not a font").unwrap();
            std::fs::write(dir.join("broken.otf"), "OTTO but truncated").unwrap();
            Staged(dir)
        }
    }

    impl Drop for Staged {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const LM: [&str; 8] = [
        "lmroman10-regular.otf",
        "lmroman10-bold.otf",
        "lmroman10-italic.otf",
        "lmroman10-bolditalic.otf",
        "lmroman12-regular.otf",
        "lmsans10-regular.otf",
        "lmsans10-bold.otf",
        "latinmodern-math.otf",
    ];

    #[test]
    fn scans_name_and_os2_tables_recursively() {
        let s = Staged::new("scan", &LM);
        let index = FontIndex::scan(&[s.0.clone()]);
        assert_eq!(index.files().len(), 8, "{:?}", index.errors());
        assert_eq!(index.errors().len(), 1, "{:?}", index.errors());
        assert!(index.errors()[0].0.ends_with("broken.otf"));
        let reg = index.files().iter().find(|f| f.path.ends_with("lmroman10-regular.otf")).unwrap();
        assert_eq!(reg.info.family, "Latin Modern Roman");
        assert_eq!(reg.info.postscript_name, "LMRoman10-Regular");
        assert_eq!((reg.info.weight, reg.info.italic, reg.info.bold), (400, false, false));
        assert_eq!(reg.info.outlines, Outlines::Cff);
        assert_eq!(reg.info.units_per_em, 1000);
        let bi = index.files().iter().find(|f| f.path.ends_with("lmroman10-bolditalic.otf")).unwrap();
        assert_eq!((bi.info.weight, bi.info.italic, bi.info.bold), (700, true, true));
        // Families are deduplicated and sorted; the math face is found.
        assert_eq!(index.families(), vec!["Latin Modern Math", "Latin Modern Roman", "Latin Modern Sans"]);
        let math: Vec<_> = index.math_fonts().iter().map(|f| f.info.postscript_name.clone()).collect();
        assert_eq!(math, vec!["LatinModernMath-Regular"]);
    }

    #[test]
    fn matching_is_case_insensitive_then_style_then_nearest_weight() {
        let s = Staged::new("match", &LM);
        let index = FontIndex::scan(&[s.0.clone()]);
        let ps = |m: Option<Match<'_>>| m.map(|m| (m.file.info.postscript_name.clone(), m.exact_weight, m.exact_style));
        assert_eq!(ps(index.find_match("latin modern roman", 400, false)), Some(("LMRoman10-Regular".into(), true, true)));
        assert_eq!(ps(index.find_match("LATIN  Modern Roman", 700, true)), Some(("LMRoman10-BoldItalic".into(), true, true)));
        // Semibold: nearest is bold (700) for 600.
        assert_eq!(ps(index.find_match("Latin Modern Roman", 600, false)), Some(("LMRoman10-Bold".into(), false, true)));
        // No italic sans: the upright is used and the mismatch reported.
        assert_eq!(ps(index.find_match("Latin Modern Sans", 400, true)), Some(("LMSans10-Regular".into(), true, false)));
        assert_eq!(ps(index.find_match("Latin Modern Sans", 700, false)), Some(("LMSans10-Bold".into(), true, true)));
        // Full and PostScript names resolve too (fontspec's `BoldFont=`).
        assert_eq!(ps(index.find_match("LMRoman10-Bold", 400, false)), Some(("LMRoman10-Bold".into(), false, true)));
        assert!(index.find("Helvetica Neue Ultra Light Extended", 400, false).is_none());
        assert!(index.find("", 400, false).is_none());
        // Two regular faces at 400 (10 and 12 pt designs): the tie is
        // broken by path, deterministically.
        assert_eq!(index.find("Latin Modern Roman", 400, false).unwrap().path, index.find("Latin Modern Roman", 400, false).unwrap().path);
    }

    #[test]
    fn cache_round_trips_and_keys_on_directory_mtimes() {
        let s = Staged::new("cache", &LM[..3]);
        let cache = s.0.join("cache").join("index.json");
        let a = FontIndex::scan_cached(&[s.0.clone()], &cache);
        assert!(cache.is_file());
        let text = std::fs::read_to_string(&cache).unwrap();
        let parsed = FontIndex::from_cache_json(&text).unwrap();
        assert_eq!(parsed.files(), a.files());
        assert_eq!(parsed.dirs(), a.dirs());
        // Same stamps: the second call is served from the cache (the
        // scan's errors are not cached, which tells the two apart).
        let b = FontIndex::scan_cached(&[s.0.clone()], &cache);
        assert_eq!(b.files(), a.files());
        assert!(b.errors().is_empty() && !a.errors().is_empty());
        // Another directory list: rescanned and rewritten.
        let c = FontIndex::scan_cached(&[s.0.join("nested")], &cache);
        assert!(c.files().is_empty());
        assert_ne!(std::fs::read_to_string(&cache).unwrap(), text);
        // Garbage or an old version: rescanned.
        assert!(FontIndex::from_cache_json("{\"version\": 0}").is_none());
        assert!(FontIndex::from_cache_json("nope").is_none());
    }

    #[test]
    fn scan_dirs_puts_the_override_and_the_project_first() {
        let s = Staged::new("dirs", &[]);
        std::fs::create_dir_all(s.0.join("fonts")).unwrap();
        let dirs = scan_dirs(Some(&s.0));
        assert_eq!(dirs.first(), Some(&s.0.join("fonts")).filter(|_| env_dirs().is_empty()).or(env_dirs().first()));
        assert!(dirs.contains(&s.0.join("fonts")));
        // Without a `fonts/` directory the project contributes nothing.
        assert!(!scan_dirs(Some(&s.0.join("nested"))).iter().any(|d| d.starts_with(&s.0)));
        let d = default_dirs();
        if cfg!(target_os = "macos") {
            assert!(d.contains(&PathBuf::from("/System/Library/Fonts/Supplemental")));
        }
        if cfg!(target_os = "linux") {
            assert!(d.contains(&PathBuf::from("/usr/share/fonts")));
        }
        assert_eq!(normalize("  Latin\tModern   Roman "), "latin modern roman");
    }

    #[test]
    fn a_missing_directory_is_skipped_not_an_error() {
        let index = FontIndex::scan(&[PathBuf::from("/nonexistent/flashtex-font-discovery")]);
        assert!(index.files().is_empty() && index.errors().is_empty());
        assert_eq!(index.dirs().len(), 1);
    }
}
