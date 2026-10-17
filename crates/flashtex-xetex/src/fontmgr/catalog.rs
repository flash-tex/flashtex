//! The fonts XeTeX's font manager can search: fontconfig's `allFonts`
//! (`XeTeXFontMgr_FC::initialize`) made platform-free. The file list comes
//! from `crates/font-discovery` (the installed and project fonts, PLAN.md
//! §3.1); every name is read here from the files, with FreeType's rules
//! (`sfnt.rs`), as `XeTeXFontMgr_FC::readNames` reads them.

use super::sfnt::{self, Face, FileBytes, NameRecord, SizeParams};
use flashtex_engine::system::StatSig;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The catalog kept between runs (`system_cached`).
#[cfg(unix)]
mod kept;

// XeTeXFontMgr_FC.cpp's name ids.
const K_FONT_FAMILY_NAME: u16 = 1;
const K_FONT_STYLE_NAME: u16 = 2;
const K_FONT_FULL_NAME: u16 = 4;
const K_PREFERRED_FAMILY_NAME: u16 = 16;
const K_PREFERRED_SUBFAMILY_NAME: u16 = 17;

/// `XeTeXFontMgr::NameCollection`: what `readNames` collects for a face.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NameCollection {
    pub family_names: Vec<String>,
    pub style_names: Vec<String>,
    pub full_names: Vec<String>,
    pub ps_name: String,
}

/// `XeTeXFontMgr::appendToList`: append a name unless already present.
pub(crate) fn append_to_list(list: &mut Vec<String>, s: &str) {
    if !list.iter().any(|x| x == s) {
        list.push(s.to_string());
    }
}

/// `XeTeXFontMgr::prependToList`: put a name first, removing it from later
/// in the list.
pub(crate) fn prepend_to_list(list: &mut Vec<String>, s: &str) {
    if let Some(k) = list.iter().position(|x| x == s) {
        list.remove(k);
    }
    list.insert(0, s.to_string());
}

/// `XeTeXFontMgr_FC::readNames` for an sfnt face, from FreeType's name
/// records (`records`, table order) and PostScript name (`ps_name`).
///
/// For ids 1, 2, 4, 16 and 17: a Macintosh Roman record in English
/// (platform 1, encoding 0, language 0) is decoded as Mac Roman and put
/// first; a Unicode (0) or Microsoft (3) record of any encoding and
/// language is decoded as UTF-16BE and appended; any other record is
/// ignored. Typographic family and subfamily names (16, 17), when there
/// are any, replace the legacy ones (1, 2). A face without a PostScript
/// name gets an empty collection (and `addToMaps` then skips it).
pub fn read_names(records: &[NameRecord], ps_name: Option<String>) -> NameCollection {
    let mut names = NameCollection::default();
    let Some(ps) = ps_name else { return names };
    names.ps_name = ps;
    let mut family_names: Vec<String> = Vec::new();
    let mut sub_family_names: Vec<String> = Vec::new();
    for r in records {
        if !matches!(
            r.name_id,
            K_FONT_FULL_NAME
                | K_FONT_FAMILY_NAME
                | K_FONT_STYLE_NAME
                | K_PREFERRED_FAMILY_NAME
                | K_PREFERRED_SUBFAMILY_NAME
        ) {
            continue;
        }
        let (utf8name, preferred) = if r.platform == 1 && r.encoding == 0 && r.language == 0 {
            (sfnt::decode_mac_roman(&r.bytes), true)
        } else if r.platform == 0 || r.platform == 3 {
            (sfnt::decode_utf16be(&r.bytes), false)
        } else {
            continue;
        };
        let list = match r.name_id {
            K_FONT_FULL_NAME => &mut names.full_names,
            K_FONT_FAMILY_NAME => &mut names.family_names,
            K_FONT_STYLE_NAME => &mut names.style_names,
            K_PREFERRED_FAMILY_NAME => &mut family_names,
            _ => &mut sub_family_names,
        };
        if preferred {
            prepend_to_list(list, &utf8name);
        } else {
            append_to_list(list, &utf8name);
        }
    }
    if !family_names.is_empty() {
        names.family_names = family_names;
    }
    if !sub_family_names.is_empty() {
        names.style_names = sub_family_names;
    }
    names
}

/// The names fontconfig puts in a face's pattern (`FC_FULLNAME`,
/// `FC_FAMILY`, `FC_STYLE`), which `searchForHostPlatformFonts` and
/// `cacheFamilyMembers` compare a requested name with. Modelled on
/// fontconfig's sfnt query: full names from ids 4 and 18, families from
/// 1, 16 and 21, styles from 2, 17 and 22, every record readNames would
/// decode (only membership matters to the search, not order).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PatternNames {
    pub full_names: Vec<String>,
    pub families: Vec<String>,
    pub styles: Vec<String>,
}

pub fn pattern_names(records: &[NameRecord]) -> PatternNames {
    let mut p = PatternNames::default();
    for r in records {
        let list = match r.name_id {
            4 | 18 => &mut p.full_names,
            1 | 16 | 21 => &mut p.families,
            2 | 17 | 22 => &mut p.styles,
            _ => continue,
        };
        let s = if r.platform == 1 && r.encoding == 0 {
            sfnt::decode_mac_roman(&r.bytes)
        } else if r.platform == 0 || r.platform == 3 {
            sfnt::decode_utf16be(&r.bytes)
        } else {
            continue;
        };
        append_to_list(list, &s);
    }
    p
}

/// What `XeTeXFontMgr::getOpSizeRecAndStyleFlags` reads from a face
/// through FreeType and HarfBuzz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleData {
    /// `(usWeightClass, usWidthClass, fsSelection)` when FreeType has an
    /// `OS/2` table.
    pub os2: Option<(u16, u16, u16)>,
    pub mac_style: u16,
    /// `post.italicAngle`, 16.16.
    pub italic_angle: i32,
    /// `hb_ot_layout_get_size_params`.
    pub size: Option<SizeParams>,
}

/// One face of the catalog: fontconfig's `FcPattern` (file, index and
/// pattern names) plus what readNames and getOpSizeRecAndStyleFlags read.
#[derive(Debug)]
pub struct CatalogFace {
    pub path: PathBuf,
    pub index: u32,
    pub pattern: PatternNames,
    pub names: NameCollection,
    /// Read on first use (when the face is added to a manager's maps).
    style: OnceLock<Option<StyleData>>,
    /// [`CatalogFace::design_size`]'s, with the file's signature it was
    /// read under.
    design: Mutex<Option<(StatSig, f64)>>,
    /// A face of the bundle's font index: its file's basename, fetched
    /// from the bundle before the style is read ([`read_bundle_font_index`]).
    pub fetch: Option<String>,
}

impl CatalogFace {
    /// Reads the face's names; `None` when FreeType could not open it
    /// (`readNames` then returns an empty collection, and the face is
    /// never added; it is left out of the catalog altogether).
    pub fn read(path: &Path, index: u32) -> Option<CatalogFace> {
        let src = FileBytes::open(path).ok()?;
        let face = Face::open(&src, index)?;
        face.mac_style()?;
        let records = face.name_records();
        let ps = face.postscript_name(&records);
        Some(CatalogFace {
            path: path.to_path_buf(),
            index,
            pattern: pattern_names(&records),
            names: read_names(&records, ps),
            style: OnceLock::new(),
            design: Mutex::new(None),
            fetch: None,
        })
    }

    /// A face whose names came from [`NameCache`]: its style data is read
    /// on first use, as for [`CatalogFace::read`].
    fn from_cache(path: PathBuf, index: u32, pattern: PatternNames, names: NameCollection) -> Self {
        CatalogFace {
            path,
            index,
            pattern,
            names,
            style: OnceLock::new(),
            design: Mutex::new(None),
            fetch: None,
        }
    }

    /// A face with given names and style data, no file (for tests and for
    /// callers that index fonts themselves).
    pub fn synthetic(
        path: PathBuf,
        index: u32,
        pattern: PatternNames,
        names: NameCollection,
        style: Option<StyleData>,
    ) -> CatalogFace {
        let cell = OnceLock::new();
        let _ = cell.set(style);
        CatalogFace {
            path,
            index,
            pattern,
            names,
            style: cell,
            design: Mutex::new(None),
            fetch: None,
        }
    }

    /// `getOpSizeRecAndStyleFlags`'s inputs (`createFont` failing gives
    /// `None`, and the face keeps the defaults).
    pub fn style(&self) -> Option<StyleData> {
        *self.style.get_or_init(|| {
            if let Some(name) = &self.fetch {
                fetch_bundle_font(name);
            }
            read_style(&self.path, self.index)
        })
    }
}

impl CatalogFace {
    /// [`design_size_of_file`] of this face's file and index, read again
    /// only when the file changed: the value read is kept with the file's
    /// [`StatSig`] taken before and after the read (a racy one keeps
    /// nothing), and used while the file has that signature. A lookup by
    /// name asks it at every `\font` (`FontMgr::locate_with`), and a
    /// resident engine's catalog outlives its runs.
    pub fn design_size(&self) -> f64 {
        let path = self.path.to_string_lossy();
        let before = StatSig::of(&path);
        let mut kept = self.design.lock().unwrap_or_else(|e| e.into_inner());
        if let (Some((sig, d)), Some(now)) = (kept.as_ref(), before.as_ref()) {
            if sig == now {
                return *d;
            }
        }
        let d = design_size_of_file(&self.path, self.index);
        *kept = match before {
            Some(sig) if Some(sig) == StatSig::of(&path) => Some((sig, d)),
            _ => None,
        };
        d
    }
}

/// The style data of face `index` of the file at `path`.
pub fn read_style(path: &Path, index: u32) -> Option<StyleData> {
    let src = FileBytes::open(path).ok()?;
    let face = Face::open(&src, index)?;
    Some(StyleData {
        os2: face.os2(),
        mac_style: face.mac_style()?,
        italic_angle: face.italic_angle(),
        size: face.size_params(),
    })
}

/// `XeTeXFontMgr::getDesignSize` for face `index` of the file at `path`:
/// the `size` feature's design size in TeX points, else 10.0 (also for a
/// file that is no sfnt, such as a Type 1 font).
pub fn design_size_of_file(path: &Path, index: u32) -> f64 {
    read_style(path, index)
        .and_then(|s| s.size)
        .map_or(10.0, |p| decipoints_to_tex(p.design_size))
}

/// `getOpSize`'s conversion: PostScript decipoints to TeX points.
pub fn decipoints_to_tex(v: u16) -> f64 {
    f64::from(v) * 72.27 / 72.0 / 10.0
}

/// Font directories the system's font service enumerates beyond
/// `font-discovery`'s [`default_dirs`](flashtex_font_discovery::default_dirs).
///
/// On macOS: the fonts Core Text downloads on demand, under
/// `/System/Library/AssetsV2/com_apple_MobileAsset_Font*/<asset>/AssetData/`
/// (PingFang, Hannotate, Osaka, ...). Measured: `xetex` finds them by name
/// once downloaded (`PingFang SC` resolves to `PingFang.ttc` there). They
/// belong in `default_dirs`; they are added here because this lane does not
/// change `crates/font-discovery`. Elsewhere: none.
pub fn os_extra_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        let assets = Path::new("/System/Library/AssetsV2");
        if let Ok(entries) = std::fs::read_dir(assets) {
            let mut dirs: Vec<PathBuf> = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().is_some_and(|n| {
                        n.to_string_lossy()
                            .starts_with("com_apple_MobileAsset_Font")
                    })
                })
                .collect();
            dirs.sort();
            out.extend(dirs);
        }
    }
    out
}

/// fontconfig's `allFonts`: every face XeTeX can find by name, in search
/// order.
#[derive(Debug, Default)]
pub struct FontCatalog {
    faces: Vec<CatalogFace>,
}

impl FontCatalog {
    /// The faces of the given files, in the given order; a face FreeType
    /// could not open is left out.
    ///
    /// A file that is a symbolic link is listed under the path it resolves
    /// to, once: the face of a link and of its target is one face, and
    /// Core Text reports the target (measured: macOS's
    /// `/Library/Fonts/Arial Unicode.ttf` links to the copy in
    /// `/System/Library/Fonts/Supplemental`, which is the path `xetex`
    /// writes).
    pub fn from_files<I: IntoIterator<Item = (PathBuf, u32)>>(files: I) -> FontCatalog {
        let mut seen = std::collections::HashSet::new();
        let mut faces = Vec::new();
        for (path, index) in files {
            let is_link =
                std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink());
            // One face per real file: duplicates are found through the
            // canonical path (a linked directory too), but only a linked
            // file is renamed.
            let real = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let path = if is_link { real.clone() } else { path };
            if !seen.insert((real, index)) {
                continue;
            }
            faces.extend(CatalogFace::read(&path, index));
        }
        FontCatalog { faces }
    }

    /// [`FontCatalog::from_files`] with the names of each face taken from
    /// `cache` when its file has the size and modification time the cache
    /// recorded, else read (and recorded). The result is the same faces with
    /// the same names: the cache only saves reading them.
    pub fn from_files_cached<I: IntoIterator<Item = (PathBuf, u32)>>(
        files: I,
        cache: &mut NameCache,
    ) -> FontCatalog {
        let mut seen = std::collections::HashSet::new();
        let mut faces = Vec::new();
        for (path, index) in files {
            let is_link =
                std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink());
            let real = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            let path = if is_link { real.clone() } else { path };
            if !seen.insert((real, index)) {
                continue;
            }
            let stamp = file_stamp(&path);
            if let Some((pattern, names)) = stamp.and_then(|st| cache.get(&path, index, st)) {
                faces.push(CatalogFace::from_cache(path, index, pattern, names));
                continue;
            }
            if let Some(face) = CatalogFace::read(&path, index) {
                if let Some(st) = stamp {
                    cache.put(&path, index, st, &face.pattern, &face.names);
                }
                faces.push(face);
            }
        }
        FontCatalog { faces }
    }

    /// [`FontCatalog::system`] through the name cache at
    /// [`NameCache::default_path`] (none if there is no cache directory):
    /// a cold lookup by name otherwise reads every installed face's names
    /// (about 1,400 faces, 0.3-1 s on macOS, measured 2026-10-04).
    ///
    /// The catalog itself is also kept between runs (`catalog/kept.rs`,
    /// beside the name cache): while no directory the walk lists and no
    /// font file it visits has changed, a run takes the whole catalog from
    /// there and opens no font file.
    pub fn system_cached(project_root: Option<&Path>) -> FontCatalog {
        let Some(cache_path) = NameCache::default_path() else {
            return FontCatalog::system(project_root);
        };
        let mut dirs = flashtex_font_discovery::scan_dirs(project_root);
        for d in os_extra_dirs() {
            if !dirs.contains(&d) {
                dirs.push(d);
            }
        }
        #[cfg(unix)]
        return kept::system(&cache_path, &dirs);
        #[cfg(not(unix))]
        FontCatalog::scan_named(&cache_path, &dirs, &mut |_| {}).0
    }

    /// The catalog of `dirs`, its names through the name cache at
    /// `cache_path`, and the scan it was made from; `on_dir` sees every
    /// directory the scan lists, before it is listed.
    fn scan_named(
        cache_path: &Path,
        dirs: &[PathBuf],
        on_dir: &mut dyn FnMut(&Path),
    ) -> (FontCatalog, flashtex_font_discovery::FontIndex) {
        let index = flashtex_font_discovery::FontIndex::scan_traced(dirs, on_dir);
        let mut cache = NameCache::load(cache_path);
        let catalog = FontCatalog::from_files_cached(
            index.files().iter().map(|f| (f.path.clone(), f.face_index)),
            &mut cache,
        );
        cache.save(cache_path);
        (catalog, index)
    }

    /// The catalog of the fonts in `dirs`, in `font-discovery`'s scan order
    /// (directories in the order given, entries sorted, `.otf`, `.ttf` and
    /// `.ttc` only).
    pub fn scan(dirs: &[PathBuf]) -> FontCatalog {
        let index = flashtex_font_discovery::FontIndex::scan(dirs);
        FontCatalog::from_files(index.files().iter().map(|f| (f.path.clone(), f.face_index)))
    }

    /// The installed fonts and a project's own `fonts/` directory:
    /// `font-discovery`'s [`scan_dirs`](flashtex_font_discovery::scan_dirs)
    /// (`FLASHTEX_FONT_DIRS`, the project, then the system's directories),
    /// then [`os_extra_dirs`].
    pub fn system(project_root: Option<&Path>) -> FontCatalog {
        let mut dirs = flashtex_font_discovery::scan_dirs(project_root);
        for d in os_extra_dirs() {
            if !dirs.contains(&d) {
                dirs.push(d);
            }
        }
        FontCatalog::scan(&dirs)
    }

    /// This catalog with `faces` after its own (the bundle's fonts after the
    /// installed ones).
    pub fn with_faces(mut self, faces: Vec<CatalogFace>) -> FontCatalog {
        self.faces.extend(faces);
        self
    }

    /// The catalog a lookup by name searches: the installed fonts
    /// ([`FontCatalog::system_cached`]), then, with no TeX Live, the
    /// bundle's ([`bundle_faces`], #1783). Every engine, spare and resident
    /// one, makes it here. The bundle is fixed for a process's life, so the
    /// font directories alone still decide when a kept catalog is stale.
    pub fn for_lookup() -> FontCatalog {
        FontCatalog::system_cached(None).with_faces(bundle_faces())
    }

    /// A catalog of faces built by the caller.
    pub fn from_faces(faces: Vec<CatalogFace>) -> FontCatalog {
        FontCatalog { faces }
    }

    pub fn faces(&self) -> &[CatalogFace] {
        &self.faces
    }

    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }
}

/// A file's size and modification time (nanoseconds since the epoch), the
/// key a cached face's names are valid for.
fn file_stamp(path: &Path) -> Option<(u64, u128)> {
    let m = std::fs::metadata(path).ok()?;
    let t = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    Some((m.len(), t.as_nanos()))
}

/// The names read from each face, kept between runs in a JSON file: by
/// (path, face index), with the file's size and modification time. A face
/// whose file changed is read again. Only entries used by the last scan are
/// kept, so the file does not grow with fonts that were removed.
#[derive(Default)]
pub struct NameCache {
    old: std::collections::HashMap<(PathBuf, u32), Entry>,
    new: Vec<((PathBuf, u32), Entry)>,
    changed: bool,
}

#[derive(Clone)]
struct Entry {
    stamp: (u64, u128),
    pattern: PatternNames,
    names: NameCollection,
}

/// The format of the cache file; another version is ignored.
const NAME_CACHE_VERSION: &str = "flashtex-xetex font names 1";

impl NameCache {
    /// `$FLASHTEX_CACHE_DIR`, else the user's cache directory
    /// (`~/Library/Caches` on macOS, `$XDG_CACHE_HOME` or `~/.cache`
    /// elsewhere, `%LOCALAPPDATA%` on Windows) under `flashtex/`.
    pub fn default_path() -> Option<PathBuf> {
        let var = |k: &str| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let dir = if let Some(d) = var("FLASHTEX_CACHE_DIR") {
            d
        } else if cfg!(target_os = "macos") {
            var("HOME")?.join("Library/Caches/flashtex")
        } else if cfg!(windows) {
            var("LOCALAPPDATA")?.join("flashtex")
        } else if let Some(x) = var("XDG_CACHE_HOME") {
            x.join("flashtex")
        } else {
            var("HOME")?.join(".cache/flashtex")
        };
        Some(dir.join("xetex-font-names.json"))
    }

    /// The cache at `path`; empty if it is missing, unreadable or of
    /// another version.
    pub fn load(path: &Path) -> NameCache {
        use flashtex_font_discovery::json::{parse, Value};
        let mut c = NameCache::default();
        let Ok(text) = std::fs::read_to_string(path) else {
            return c;
        };
        let Ok(v) = parse(&text) else {
            return c;
        };
        if v.get("version").and_then(Value::as_str) != Some(NAME_CACHE_VERSION) {
            return c;
        }
        let strs = |v: Option<&Value>| -> Option<Vec<String>> {
            v?.as_arr()?
                .iter()
                .map(|s| s.as_str().map(str::to_string))
                .collect()
        };
        for e in v.get("faces").and_then(Value::as_arr).unwrap_or(&[]) {
            let entry = (|| {
                let path = PathBuf::from(e.get("path")?.as_str()?);
                let index = e.get("index")?.as_u64()? as u32;
                let size = e.get("size")?.as_u64()?;
                let mtime: u128 = e.get("mtime")?.as_str()?.parse().ok()?;
                let pattern = PatternNames {
                    full_names: strs(e.get("pfull"))?,
                    families: strs(e.get("pfam"))?,
                    styles: strs(e.get("pstyle"))?,
                };
                let names = NameCollection {
                    family_names: strs(e.get("fam"))?,
                    style_names: strs(e.get("style"))?,
                    full_names: strs(e.get("full"))?,
                    ps_name: e.get("ps")?.as_str()?.to_string(),
                };
                Some((
                    (path, index),
                    Entry {
                        stamp: (size, mtime),
                        pattern,
                        names,
                    },
                ))
            })();
            if let Some((k, v)) = entry {
                c.old.insert(k, v);
            }
        }
        c
    }

    fn get(
        &mut self,
        path: &Path,
        index: u32,
        stamp: (u64, u128),
    ) -> Option<(PatternNames, NameCollection)> {
        let key = (path.to_path_buf(), index);
        let e = self.old.get(&key).filter(|e| e.stamp == stamp)?.clone();
        let out = (e.pattern.clone(), e.names.clone());
        self.new.push((key, e));
        Some(out)
    }

    fn put(
        &mut self,
        path: &Path,
        index: u32,
        stamp: (u64, u128),
        p: &PatternNames,
        n: &NameCollection,
    ) {
        self.changed = true;
        self.new.push((
            (path.to_path_buf(), index),
            Entry {
                stamp,
                pattern: p.clone(),
                names: n.clone(),
            },
        ));
    }

    /// Write the entries the last scan used, if any face was read or one
    /// went away (best effort, through a temporary file and a rename).
    pub fn save(&self, path: &Path) {
        use flashtex_font_discovery::json::{write, Value};
        use std::collections::BTreeMap;
        if !self.changed && self.new.len() == self.old.len() {
            return;
        }
        let arr = |v: &[String]| Value::Arr(v.iter().cloned().map(Value::Str).collect());
        let faces = self
            .new
            .iter()
            .map(|((p, i), e)| {
                let mut m = BTreeMap::new();
                m.insert("path".into(), Value::Str(p.to_string_lossy().into_owned()));
                m.insert("index".into(), Value::Num(f64::from(*i)));
                m.insert("size".into(), Value::Num(e.stamp.0 as f64));
                m.insert("mtime".into(), Value::Str(e.stamp.1.to_string()));
                m.insert("pfull".into(), arr(&e.pattern.full_names));
                m.insert("pfam".into(), arr(&e.pattern.families));
                m.insert("pstyle".into(), arr(&e.pattern.styles));
                m.insert("fam".into(), arr(&e.names.family_names));
                m.insert("style".into(), arr(&e.names.style_names));
                m.insert("full".into(), arr(&e.names.full_names));
                m.insert("ps".into(), Value::Str(e.names.ps_name.clone()));
                Value::Obj(m)
            })
            .collect();
        let mut top = BTreeMap::new();
        top.insert("version".into(), Value::Str(NAME_CACHE_VERSION.into()));
        top.insert("faces".into(), Value::Arr(faces));
        let mut out = String::new();
        write(&Value::Obj(top), &mut out);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension(format!("json.{}", std::process::id()));
        if std::fs::write(&tmp, out).is_ok() && std::fs::rename(&tmp, path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

// ---- the bundle's fonts by name (no-TeX-Live mode) ---------------------------
//
// With no TeX Live, the bundle's OpenType and TrueType fonts can be named
// (`\setmainfont{TeX Gyre Pagella}`), as xelatex finds TeX Live's fonts on
// Linux and Windows through fontconfig. This deliberately differs from
// xelatex on macOS, whose Core Text search does not see TeX Live's trees
// (owner direction, #1783). With a TeX Live the bundle is not used and
// lookups are as before. The bundle is fetched file by file, so its fonts'
// names cannot be read lazily: `texbundle.py pack` has this program write an
// index of every face it packs ([`write_bundle_font_index`]), stored in the
// bundle as FlashTeX's own generated file (TeX Live's files stay as they
// are), and a face's file is fetched only when a lookup selects its family.

/// The index's basename, which the bundle's resolver finds.
pub const BUNDLE_FONT_INDEX: &str = "flashtex-xetex-fonts.json";
/// Where the index lives in the bundle's tree.
/// (`fonts/misc` of `TEXMFVAR`: kpathsea's `misc fonts` path finds it.)
pub const BUNDLE_FONT_INDEX_PATH: &str = "texmf-var/fonts/misc/flashtex/flashtex-xetex-fonts.json";
const BUNDLE_INDEX_VERSION: &str = "flashtex-xetex bundle fonts 1";

/// The index of the faces of `files` (absolute paths in the tree at
/// `root`; other than `.otf`, `.ttf` and `.ttc` files are skipped), with the
/// names [`CatalogFace::read`] reads, in the order given; paths are written
/// relative to `root`. Deterministic: the same files give the same bytes.
pub fn write_bundle_font_index(root: &Path, files: &[PathBuf]) -> String {
    use flashtex_font_discovery::json::{write, Value};
    use std::collections::BTreeMap;
    let arr = |v: &[String]| Value::Arr(v.iter().cloned().map(Value::Str).collect());
    let mut faces = vec![];
    for f in files {
        let ext = f
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !matches!(ext.as_str(), "otf" | "ttf" | "ttc") {
            continue;
        }
        let Ok(rel) = f.strip_prefix(root) else {
            continue;
        };
        let n = FileBytes::open(f)
            .ok()
            .and_then(|src| sfnt::face_count(&src))
            .unwrap_or(1);
        for index in 0..n {
            let Some(face) = CatalogFace::read(f, index) else {
                continue;
            };
            let mut m = BTreeMap::new();
            m.insert(
                "path".into(),
                Value::Str(rel.to_string_lossy().replace('\\', "/")),
            );
            m.insert("index".into(), Value::Num(f64::from(index)));
            m.insert("pfull".into(), arr(&face.pattern.full_names));
            m.insert("pfam".into(), arr(&face.pattern.families));
            m.insert("pstyle".into(), arr(&face.pattern.styles));
            m.insert("fam".into(), arr(&face.names.family_names));
            m.insert("style".into(), arr(&face.names.style_names));
            m.insert("full".into(), arr(&face.names.full_names));
            m.insert("ps".into(), Value::Str(face.names.ps_name.clone()));
            faces.push(Value::Obj(m));
        }
    }
    let mut top = BTreeMap::new();
    top.insert("version".into(), Value::Str(BUNDLE_INDEX_VERSION.into()));
    top.insert("faces".into(), Value::Arr(faces));
    let mut out = String::new();
    write(&Value::Obj(top), &mut out);
    out
}

/// The faces of a bundle font index whose tree is at `tree`: each is
/// fetched from the bundle when its style is first read
/// ([`CatalogFace::style`]), which happens only for the family a lookup
/// selects. Empty for a text of another version.
pub fn read_bundle_font_index(tree: &Path, text: &str) -> Vec<CatalogFace> {
    use flashtex_font_discovery::json::{parse, Value};
    let Ok(v) = parse(text) else { return vec![] };
    if v.get("version").and_then(Value::as_str) != Some(BUNDLE_INDEX_VERSION) {
        return vec![];
    }
    let strs = |v: Option<&Value>| -> Option<Vec<String>> {
        v?.as_arr()?
            .iter()
            .map(|s| s.as_str().map(str::to_string))
            .collect()
    };
    let mut out = vec![];
    for e in v.get("faces").and_then(Value::as_arr).unwrap_or(&[]) {
        let face = (|| {
            let rel = e.get("path")?.as_str()?;
            // a relative path inside the tree only
            if rel.starts_with('/') || rel.split('/').any(|c| c == ".." || c.is_empty()) {
                return None;
            }
            let mut f = CatalogFace::from_cache(
                tree.join(rel),
                e.get("index")?.as_u64()? as u32,
                PatternNames {
                    full_names: strs(e.get("pfull"))?,
                    families: strs(e.get("pfam"))?,
                    styles: strs(e.get("pstyle"))?,
                },
                NameCollection {
                    family_names: strs(e.get("fam"))?,
                    style_names: strs(e.get("style"))?,
                    full_names: strs(e.get("full"))?,
                    ps_name: e.get("ps")?.as_str()?.to_string(),
                },
            );
            f.fetch = Some(rel.rsplit('/').next()?.to_string());
            Some(f)
        })();
        out.extend(face);
    }
    out
}

/// The bundle's faces, when the engine's files come from a bundle that has
/// the index; none otherwise (with a TeX Live, or a bundle before
/// texbundle-tl2026-4).
pub fn bundle_faces() -> Vec<CatalogFace> {
    use flashtex_engine::resolver::Format;
    let Some(found) = flashtex_engine::system::find_file(BUNDLE_FONT_INDEX, Format::MiscFonts)
    else {
        return vec![];
    };
    let found = found.replace('\\', "/");
    let Some(tree) = found.strip_suffix(BUNDLE_FONT_INDEX_PATH) else {
        return vec![];
    };
    let Ok(text) = std::fs::read_to_string(&found) else {
        return vec![];
    };
    read_bundle_font_index(Path::new(tree), &text)
}

/// Fetches a bundle face's file (by its basename, through the engine's
/// resolver, which fetches from the bundle); its path, if it is there.
pub fn fetch_bundle_font(name: &str) -> Option<PathBuf> {
    use flashtex_engine::resolver::Format;
    flashtex_engine::system::find_file(name, Format::OpenType)
        .or_else(|| flashtex_engine::system::find_file(name, Format::TrueType))
        .map(PathBuf::from)
}

#[cfg(test)]
mod bundle_index_tests {
    use super::*;

    /// The index holds every face of the font files given, with the names
    /// the catalog reads from the files; read back under another tree it
    /// gives the same names at the same relative paths, other files and
    /// paths that leave the tree are skipped.
    #[test]
    fn the_index_round_trips() {
        let dir = std::env::temp_dir().join(format!("ftx-bfi-{}", std::process::id()));
        let fonts = dir.join("texmf-dist/fonts/opentype/x");
        std::fs::create_dir_all(&fonts).unwrap();
        let src = [
            "/System/Library/Fonts/Supplemental/Georgia.ttf",
            "/System/Library/Fonts/Menlo.ttc",
        ];
        let mut files = vec![];
        for s in src {
            let s = Path::new(s);
            if !s.is_file() {
                eprintln!("skipped: no {}", s.display());
                return;
            }
            let d = fonts.join(s.file_name().unwrap());
            std::fs::copy(s, &d).unwrap();
            files.push(d);
        }
        std::fs::write(fonts.join("README"), "x").unwrap();
        files.push(fonts.join("README"));
        let text = write_bundle_font_index(&dir, &files);
        assert_eq!(text, write_bundle_font_index(&dir, &files), "deterministic");
        let tree = Path::new("/elsewhere/tree");
        let faces = read_bundle_font_index(tree, &text);
        let direct = FontCatalog::from_files(files[..2].iter().flat_map(|f| {
            let n = FileBytes::open(f)
                .ok()
                .and_then(|s| sfnt::face_count(&s))
                .unwrap();
            (0..n).map(move |i| (f.clone(), i))
        }));
        assert_eq!(faces.len(), direct.len());
        assert!(faces.len() >= 3, "Georgia and Menlo's faces");
        for (a, b) in faces.iter().zip(direct.faces()) {
            assert_eq!(a.names, b.names);
            assert_eq!(a.pattern, b.pattern);
            assert_eq!(a.index, b.index);
            assert_eq!(a.path, tree.join(b.path.strip_prefix(&dir).unwrap()));
            assert!(a.fetch.is_some());
        }
        let bad = text.replace("texmf-dist/fonts", "../fonts");
        assert!(read_bundle_font_index(tree, &bad).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    /// A catalog built through the cache has the faces and names of one
    /// built from the files, both when the cache is empty and when it is
    /// read back; a changed file is read again.
    #[test]
    fn name_cache_round_trip() {
        let fonts: Vec<PathBuf> = [
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ]
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect();
        if fonts.is_empty() {
            eprintln!("skipped: no system fonts to read");
            return;
        }
        let files: Vec<(PathBuf, u32)> = fonts.iter().map(|p| (p.clone(), 0)).collect();
        let plain = FontCatalog::from_files(files.clone());
        let dir = std::env::temp_dir().join(format!("xetex-name-cache-{}", std::process::id()));
        let cache_path = dir.join("names.json");
        let mut c = NameCache::load(&cache_path);
        let first = FontCatalog::from_files_cached(files.clone(), &mut c);
        c.save(&cache_path);
        let mut c2 = NameCache::load(&cache_path);
        assert_eq!(c2.old.len(), plain.len());
        let second = FontCatalog::from_files_cached(files.clone(), &mut c2);
        for cat in [&first, &second] {
            assert_eq!(cat.len(), plain.len());
            for (a, b) in cat.faces().iter().zip(plain.faces()) {
                assert_eq!((&a.path, a.index), (&b.path, b.index));
                assert_eq!(a.names, b.names);
                assert_eq!(a.pattern, b.pattern);
            }
        }
        // An entry whose stamp does not match is not used.
        let key = (fonts[0].clone(), 0);
        let mut c3 = NameCache::load(&cache_path);
        let st = c3.old[&key].stamp;
        assert!(c3.get(&fonts[0], 0, (st.0 + 1, st.1)).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
