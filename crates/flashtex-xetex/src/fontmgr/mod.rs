//! XeTeX's font lookup, platform-free: which file and face a `\font`
//! name selects, as `findnativefont` (XeTeX_ext.c) decides it **before**
//! loading the font, with `XeTeXFontMgr`'s matching rules
//! (XeTeXFontMgr.cpp) over FlashTeX's index of the installed fonts
//! (`crates/font-discovery`) instead of Core Text or fontconfig
//! (docs/design/xetex/PLAN.md §3.1). Ported from TeX Live 2026's XeTeX
//! (MIT; each function names the C function it ports), with the sfnt data
//! FreeType and HarfBuzz hand XeTeX read from the font bytes (`sfnt`).
//! Loading, metrics and shaping are not here, and nothing here uses
//! FreeType or HarfBuzz.
//!
//! # API
//!
//! * [`split_font_name`] -- `splitFontName`: name, `/variant`,
//!   `:features` and the `[file:index]` form.
//! * [`FontCatalog`] -- the faces a name can select: fontconfig's
//!   `allFonts`, built from `font-discovery`'s directory list and scan
//!   ([`FontCatalog::system`]) or from explicit files. Immutable and
//!   shareable between engines.
//! * [`FontMgr`] -- `XeTeXFontMgr` over a catalog: the name maps, filled
//!   lazily as XeTeX fills them, [`FontMgr::find_font`] (`findFont`),
//!   [`FontMgr::full_name`] (`getFullName`) and the requested engine
//!   (`getReqEngine`/`setReqEngine`). It is per-engine state: XeTeX keeps
//!   one manager per run, and what it has cached changes later answers.
//! * [`FontMgr::locate`] / [`FontMgr::locate_with`] -- `findnativefont`
//!   up to the point where it loads the font: a [`Located`] (path, face
//!   index, variant and feature strings, requested engine, full name, the
//!   `name_of_file` XeTeX leaves behind, design size and the size the font
//!   is to be loaded at), or `None` where XeTeX finds no font.
//! * [`design_size_of_file`] -- `getDesignSize` for a face (the GPOS
//!   `size` feature, else 10pt).
//!
//! # What is and is not the same as `xetex`
//!
//! The matching rules are XeTeX's own, and they are the same on every
//! platform. What differs between XeTeX's builds is the *platform search*
//! that fills the maps and the names read from each face. This module
//! follows the fontconfig build (`XeTeXFontMgr_FC`): `readNames` reads
//! every Mac Roman English, Unicode and Microsoft `name` record of ids 1,
//! 2, 4, 16 and 17, and `searchForHostPlatformFonts` adds the faces whose
//! full name, family, or family plus style is the requested name, with
//! their families, else every face. TeX Live's macOS build asks Core Text
//! instead (`XeTeXFontMgr_Mac`: display-name matching, `NSFontManager`
//! families, the default and localized names only); where the two select
//! another face, the difference is measured by
//! `tools/xetex-lockstep/fontmatch.py`, not assumed away. Not ported:
//! `XeTeXFontMgr_FC::getOpSizeRecAndStyleFlags`'s fallback to fontconfig's
//! weight, width and slant for faces without `OS/2` (macOS does not do it
//! either), and the `\XeTeXtracingfonts` diagnostic (the engine prints
//! [`Located::path`]).
//!
//! Measured against TeX Live 2026's `xetex` on macOS 26 (mac-m1max-a,
//! 2026-10-04, `fontnames.txt`): the same face for every lookup but two,
//! both Core Text behaviour the platform-free model does not have:
//!
//! * **Variable fonts' named instances.** Core Text lists each `fvar`
//!   named instance as a face of its own (`STIX Two Text Bold`, PostScript
//!   name `STIXTwoText_Bold`, in `STIXTwoText.ttf`); the catalog has one
//!   face per file and face index, so such a name is not found. Core Text
//!   also names a variable font's faces after their instances, so the
//!   `name_of_file` differs (`STIX Two Text Regular` against the `name`
//!   table's `STIX Two Text`).
//! * **Two installed families of one name.** XeTeX's macOS search adds a
//!   family's members as `NSFontManager` lists them, one face per style;
//!   when two installed families share a family name, the faces it leaves
//!   out are never found, not even by PostScript name (`SFMono-Regular`
//!   when `SF-Mono-*.otf` and `SFMonoLigaturized-*.ttf`, both family `SF
//!   Mono`, are installed). Here, as in XeTeX's fontconfig build, they are.
//!
//! The catalog also reads what Core Text sees and `font-discovery`'s
//! `default_dirs` leaves out: the fonts macOS downloads on demand
//! ([`catalog::os_extra_dirs`]: PingFang, Hannotate, Osaka ...) and a
//! font file that is a symbolic link, listed under its target
//! ([`FontCatalog::from_files`]).
//!
//! The engine's `find_native_font` (`xetex_ext.rs`) does not call this
//! yet; the native-fonts work wires it in.

pub mod catalog;
pub mod mgr;
pub mod sfnt;

pub use catalog::{
    decipoints_to_tex, design_size_of_file, CatalogFace, FontCatalog, NameCollection, PatternNames,
    StyleData,
};
pub use mgr::{Family, Font, FontMgr, Found, OpSizeRec};

use flashtex_engine::resolver::Format;
use std::path::Path;

/// The parts of a font name, as `splitFontName` delimits them and
/// `findnativefont` copies them out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitFontName {
    /// `nameString`: the name up to the variant. For the file form it keeps
    /// its opening `[` (the file name is `name[1..]`).
    pub name: String,
    /// `varString`: the text between the variant's `/` and the features
    /// (none when there is no variant). For the file form the variant
    /// starts at the `]` or the `:index`, so there is always one (empty for
    /// a bare `[x.otf]`): it keeps a leading `/` (`[x.otf]/OT` gives `/OT`)
    /// or includes the rest of the index part (`[x.ttc:1]` gives `1]`).
    pub variant: Option<String>,
    /// `featString`: the text after the features' `:` (none without one).
    pub features: Option<String>,
    /// The face index of `[file:index]`, 0 otherwise.
    pub index: u32,
}

impl SplitFontName {
    /// Whether this is the `[file]` form.
    pub fn is_file(&self) -> bool {
        self.name.starts_with('[')
    }
}

/// `splitFontName` (XeTeX_ext.c) and the copies `findnativefont` makes of
/// its parts.
///
/// * `[file]`, `[file:index]`, then `/variant` and `:features` after the
///   `]`; inside the brackets a `:` (the first one, and on Windows not the
///   drive letter's) starts the face index, whose digits are read;
/// * otherwise the first `/` before any `:` starts the variant and the
///   first `:` starts the features.
pub fn split_font_name(name: &str) -> SplitFontName {
    let b = name.as_bytes();
    let mut var: Option<usize> = None;
    let mut feat: Option<usize> = None;
    let mut index: u32 = 0;
    let end = b.len();
    if b.first() == Some(&b'[') {
        let mut within_file_name = true;
        let start = 1usize;
        let mut k = 1usize;
        while k < end {
            let c = b[k];
            if within_file_name && c == b']' {
                within_file_name = false;
                if var.is_none() {
                    var = Some(k);
                }
            } else if c == b':' {
                let drive = cfg!(windows) && k - start == 1 && b[start].is_ascii_alphabetic();
                if within_file_name && var.is_none() && !drive {
                    var = Some(k);
                    k += 1;
                    while k < end && b[k].is_ascii_digit() {
                        index = index.wrapping_mul(10).wrapping_add(u32::from(b[k] - b'0'));
                        k += 1;
                    }
                    k -= 1;
                } else if !within_file_name && feat.is_none() {
                    feat = Some(k);
                }
            }
            k += 1;
        }
    } else {
        for (k, &c) in b.iter().enumerate() {
            if c == b'/' && var.is_none() && feat.is_none() {
                var = Some(k);
            } else if c == b':' && feat.is_none() {
                feat = Some(k);
            }
        }
    }
    let feat = feat.unwrap_or(end);
    let var = var.unwrap_or(feat);
    // The delimiters are ASCII, so every slice below is on a UTF-8 boundary.
    SplitFontName {
        name: name[..var].to_string(),
        variant: (feat > var).then(|| name[var + 1..feat].to_string()),
        features: (end > feat).then(|| name[feat + 1..end].to_string()),
        index,
    }
}

/// What `findnativefont` has decided when it is about to load a font.
#[derive(Debug, Clone, PartialEq)]
pub struct Located {
    /// The font file: the path kpathsea found for `[file]`, the catalog
    /// face's file for a name.
    pub path: String,
    pub face_index: u32,
    /// The `[file]` form (kpathsea; the maps are not searched).
    pub by_file: bool,
    /// For a name, the variant as `findFont` rewrote it (engine options
    /// only); for a file, `varString` as split.
    pub variant: Option<String>,
    /// `featString`.
    pub features: Option<String>,
    /// `getReqEngine()` after the lookup: `b'A'` (AAT), `b'O'` (OpenType),
    /// `b'G'` (Graphite) or 0.
    pub req_engine: u8,
    /// `getFullName(fontRef)` (names only).
    pub full_name: Option<String>,
    /// What `findnativefont` leaves in `name_of_file + 1` (names only): the
    /// full name, then `/variant` and `:features` when non-empty. For a
    /// file `name_of_file` is left as it was.
    pub name_of_file: Option<String>,
    /// `getDesignSize(font)` of the selected face, in TeX points.
    pub design_size: f64,
    /// `loadedfontdesignsize` (16.16) as `findnativefont` leaves it.
    pub loaded_font_design_size: i32,
    /// The size to load the font at (`scaled_size` after a negative
    /// `scaled` value is resolved against the design size).
    pub scaled_size: i32,
}

/// C's `D2Fix`.
pub fn d2fix(d: f64) -> i32 {
    (d * 65536.0 + 0.5) as i32
}

/// C's `Fix2D`.
pub fn fix2d(f: i32) -> f64 {
    f64::from(f) / 65536.0
}

/// tex.web's `xn_over_d` (`zxnoverd`): `x * n / d` truncated toward zero,
/// for `n, d` below 2^16.
pub fn xn_over_d(x: i32, n: i32, d: i32) -> i32 {
    let positive = x >= 0;
    let r = (i64::from(x).abs() * i64::from(n)) / i64::from(d);
    (if positive { r } else { -r }) as i32
}

/// `findnativefont`'s size: `scaled_size` as given, or for a negative
/// (`scaled`) one the design size scaled by it.
fn resolve_scaled(scaled_size: i32, design_size: f64) -> i32 {
    if scaled_size >= 0 {
        return scaled_size;
    }
    let dsize = d2fix(design_size);
    if scaled_size == -1000 {
        dsize
    } else {
        xn_over_d(dsize, -scaled_size, 1000)
    }
}

/// kpathsea's lookup through the engine's resolver
/// (`kpse_find_file(name, format, 0)`).
pub fn kpse_find_file(name: &str, format: Format) -> Option<String> {
    flashtex_engine::system::find_file(name, format)
}

impl FontMgr {
    /// [`FontMgr::locate_with`] through the engine's kpathsea.
    pub fn locate(&mut self, name: &str, scaled_size: i32) -> Option<Located> {
        self.locate_with(name, scaled_size, &mut kpse_find_file)
    }

    /// `findnativefont(name, scaled_size)` up to loading: `scaled_size` is
    /// in scaled points, or negative for `scaled` (-1000 is the design
    /// size). `find_file` is kpathsea (`kpse_find_file(name, format, 0)`).
    ///
    /// * `[file]`: the file is looked up as an OpenType, then a TrueType,
    ///   then a Type 1 font; the face index is the one given. The
    ///   requested engine is set from the variant (`/AAT`, `/OT` or `/ICU`,
    ///   `/GR`). `None` when no file is found, or when it is an sfnt
    ///   without that face (FreeType would not open it).
    /// * a name: `findFont(name, variant, Fix2D(scaled_size))` over the
    ///   maps; `None` when no face matches.
    ///
    /// Whether the selected face then loads is the loader's question.
    pub fn locate_with(
        &mut self,
        name: &str,
        scaled_size: i32,
        find_file: &mut dyn FnMut(&str, Format) -> Option<String>,
    ) -> Option<Located> {
        let split = split_font_name(name);
        if split.is_file() {
            let file = &split.name[1..];
            let path = find_file(file, Format::OpenType)
                .or_else(|| find_file(file, Format::TrueType))
                .or_else(|| find_file(file, Format::Type1))?;
            let src = sfnt::FileBytes::open(Path::new(&path)).ok()?;
            if let Some(n) = sfnt::face_count(&src) {
                if split.index >= n {
                    return None;
                }
            }
            let design_size = design_size_of_file(Path::new(&path), split.index);
            // This is duplicated in XeTeXFontMgr::findFont!
            self.set_req_engine(0);
            if let Some(v) = &split.variant {
                if v.starts_with("/AAT") {
                    self.set_req_engine(b'A');
                } else if v.starts_with("/OT") || v.starts_with("/ICU") {
                    self.set_req_engine(b'O');
                } else if v.starts_with("/GR") {
                    self.set_req_engine(b'G');
                }
            }
            return Some(Located {
                path,
                face_index: split.index,
                by_file: true,
                variant: split.variant,
                features: split.features,
                req_engine: self.req_engine(),
                full_name: None,
                name_of_file: None,
                design_size,
                loaded_font_design_size: d2fix(design_size),
                scaled_size: resolve_scaled(scaled_size, design_size),
            });
        }

        let found = self.find_font(&split.name, split.variant.as_deref(), fix2d(scaled_size))?;
        let face = self.face(found.face);
        let (path, face_index) = (face.path.to_string_lossy().into_owned(), face.index);
        let full_name = self.full_name(found.face).map(str::to_string);
        let design_size = design_size_of_file(Path::new(&path), face_index);
        let mut name_of_file = full_name.clone().unwrap_or_default();
        if let Some(v) = found.variant.as_deref().filter(|v| !v.is_empty()) {
            name_of_file.push('/');
            name_of_file.push_str(v);
        }
        if let Some(f) = split.features.as_deref().filter(|f| !f.is_empty()) {
            name_of_file.push(':');
            name_of_file.push_str(f);
        }
        Some(Located {
            path,
            face_index,
            by_file: false,
            variant: found.variant,
            features: split.features,
            req_engine: self.req_engine(),
            full_name,
            name_of_file: Some(name_of_file),
            design_size,
            loaded_font_design_size: found.loaded_font_design_size,
            scaled_size: resolve_scaled(scaled_size, design_size),
        })
    }
}

#[cfg(test)]
mod tests;
