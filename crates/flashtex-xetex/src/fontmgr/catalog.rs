//! The fonts XeTeX's font manager can search: fontconfig's `allFonts`
//! (`XeTeXFontMgr_FC::initialize`) made platform-free. The file list comes
//! from `crates/font-discovery` (the installed and project fonts, PLAN.md
//! §3.1); every name is read here from the files, with FreeType's rules
//! (`sfnt.rs`), as `XeTeXFontMgr_FC::readNames` reads them.

use super::sfnt::{self, Face, FileBytes, NameRecord, SizeParams};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

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
        })
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
        }
    }

    /// `getOpSizeRecAndStyleFlags`'s inputs (`createFont` failing gives
    /// `None`, and the face keeps the defaults).
    pub fn style(&self) -> Option<StyleData> {
        *self
            .style
            .get_or_init(|| read_style(&self.path, self.index))
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
