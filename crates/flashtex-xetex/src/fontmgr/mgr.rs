//! `XeTeXFontMgr` (XeTeXFontMgr.cpp) with `XeTeXFontMgr_FC`'s platform
//! search (XeTeXFontMgr_FC.cpp), over a [`FontCatalog`] instead of
//! fontconfig. The maps are filled lazily, as XeTeX fills them: a name that
//! is not in them yet makes `searchForHostPlatformFonts` add the faces that
//! seem to match it (and their families), and the second pass of
//! `findFont` looks again. Like XeTeX's, a manager's state therefore
//! depends on the lookups it has already served; one manager belongs to
//! one engine run.

use super::catalog::{CatalogFace, FontCatalog, NameCollection};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// `XeTeXFontMgr::OpSizeRec`, in TeX points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpSizeRec {
    pub design_size: f64,
    pub min_size: f64,
    pub max_size: f64,
    pub sub_family_id: u32,
    pub name_code: u32,
}

/// `XeTeXFontMgr::Font`: a face known to the maps.
#[derive(Debug, Clone)]
pub struct Font {
    /// The catalog face (`fontRef`).
    pub face: usize,
    pub full_name: Option<String>,
    pub ps_name: String,
    pub family_name: String,
    pub style_name: String,
    /// The family of the first family name (`parent`).
    pub parent: Option<usize>,
    pub op_size_info: OpSizeRec,
    pub weight: u16,
    pub width: u16,
    pub slant: i16,
    pub is_reg: bool,
    pub is_bold: bool,
    pub is_italic: bool,
}

/// `XeTeXFontMgr::Family`: its styles by name (a `std::map`, so iterated
/// in byte order) and the ranges of its members' weight, width and slant.
#[derive(Debug, Clone, Default)]
pub struct Family {
    pub styles: BTreeMap<String, usize>,
    pub min_weight: u16,
    pub max_weight: u16,
    pub min_width: u16,
    pub max_width: u16,
    pub min_slant: i16,
    pub max_slant: i16,
}

/// What `findFont` returns besides the font: its side effects on XeTeX's
/// globals.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// The catalog face (`PlatformFontRef`).
    pub face: usize,
    /// The variant string as `findFont` leaves it in place: `/B`, `/I`
    /// and `/S=` removed, only the engine options (`AAT`, `OT`, `GR`)
    /// kept, `ICU` written as `OT`. `None` when there was none.
    pub variant: Option<String>,
    /// `loadedfontdesignsize` (16.16).
    pub loaded_font_design_size: i32,
}

/// XeTeX's font manager over a catalog.
pub struct FontMgr {
    catalog: Arc<FontCatalog>,
    fonts: Vec<Font>,
    families: Vec<Family>,
    /// `m_nameToFont`: full names.
    name_to_font: HashMap<String, usize>,
    /// `m_nameToFamily`.
    name_to_family: HashMap<String, usize>,
    /// `m_platformRefToFont`, by catalog face.
    platform_ref_to_font: HashMap<usize, usize>,
    /// `m_psNameToFont`.
    ps_name_to_font: HashMap<String, usize>,
    /// `sReqEngine`.
    req_engine: u8,
    /// `XeTeXFontMgr_FC::cachedAll`.
    cached_all: bool,
}

/// C's `my_fmax`.
fn my_fmax(x: f64, y: f64) -> f64 {
    if x > y {
        x
    } else {
        y
    }
}

impl FontMgr {
    /// `XeTeXFontMgr::GetFontManager()` then `initialize()`: empty maps.
    pub fn new(catalog: Arc<FontCatalog>) -> FontMgr {
        FontMgr {
            catalog,
            fonts: Vec::new(),
            families: Vec::new(),
            name_to_font: HashMap::new(),
            name_to_family: HashMap::new(),
            platform_ref_to_font: HashMap::new(),
            ps_name_to_font: HashMap::new(),
            req_engine: 0,
            cached_all: false,
        }
    }

    pub fn catalog(&self) -> &Arc<FontCatalog> {
        &self.catalog
    }

    pub fn face(&self, face: usize) -> &CatalogFace {
        &self.catalog.faces()[face]
    }

    /// `getReqEngine`: the rendering technology the last successful
    /// `findFont` asked for (`b'A'`, `b'O'`, `b'G'`), or 0.
    pub fn req_engine(&self) -> u8 {
        self.req_engine
    }

    /// `setReqEngine`.
    pub fn set_req_engine(&mut self, e: u8) {
        self.req_engine = e;
    }

    /// The font record of a catalog face, once it is in the maps.
    pub fn font_of_face(&self, face: usize) -> Option<&Font> {
        self.platform_ref_to_font
            .get(&face)
            .map(|&f| &self.fonts[f])
    }

    pub fn family(&self, name: &str) -> Option<&Family> {
        self.name_to_family.get(name).map(|&f| &self.families[f])
    }

    /// `XeTeXFontMgr::getFullName`: the first full name, else the
    /// PostScript name. `None` for a face never returned by `findFont`
    /// (XeTeX's `die("internal error 2")`).
    pub fn full_name(&self, face: usize) -> Option<&str> {
        let f = self.font_of_face(face)?;
        Some(f.full_name.as_deref().unwrap_or(&f.ps_name))
    }

    /// `XeTeXFontMgr::findFont(name, variant, ptSize)`. `pt_size` is in TeX
    /// points, or negative for a `scaled` font (then the font's design size
    /// is used for optical sizing). See the module documentation for the
    /// order of the attempts.
    pub fn find_font(
        &mut self,
        name: &str,
        variant: Option<&str>,
        mut pt_size: f64,
    ) -> Option<Found> {
        let mut font: Option<usize> = None;
        let mut dsize = 10.0;
        for pass in 0..2 {
            // try full name as given
            if let Some(&f) = self.name_to_font.get(name) {
                font = Some(f);
                if self.fonts[f].op_size_info.design_size != 0.0 {
                    dsize = self.fonts[f].op_size_info.design_size;
                }
                break;
            }

            // if there's a hyphen, split there and try Family-Style
            if let Some(hyph) = name.find('-') {
                if hyph > 0 && hyph < name.len() - 1 {
                    if let Some(&fam) = self.name_to_family.get(&name[..hyph]) {
                        if let Some(&f) = self.families[fam].styles.get(&name[hyph + 1..]) {
                            font = Some(f);
                            if self.fonts[f].op_size_info.design_size != 0.0 {
                                dsize = self.fonts[f].op_size_info.design_size;
                            }
                            break;
                        }
                    }
                }
            }

            // try as PostScript name
            if let Some(&f) = self.ps_name_to_font.get(name) {
                font = Some(f);
                if self.fonts[f].op_size_info.design_size != 0.0 {
                    dsize = self.fonts[f].op_size_info.design_size;
                }
                break;
            }

            // try for the name as a family name
            if let Some(&fam) = self.name_to_family.get(name) {
                let styles = &self.families[fam].styles;
                // look for a family member with the "regular" bit set in OS/2
                let mut reg_fonts = 0;
                for &f in styles.values() {
                    if self.fonts[f].is_reg {
                        if reg_fonts == 0 {
                            font = Some(f);
                        }
                        reg_fonts += 1;
                    }
                }

                // families with Ornament or similar fonts may flag those as
                // Regular, which confuses the search above... so try some
                // known names
                if font.is_none() || reg_fonts > 1 {
                    // try for style "Regular", "Plain", "Normal", "Roman"
                    for s in ["Regular", "Plain", "Normal", "Roman"] {
                        if let Some(&f) = styles.get(s) {
                            font = Some(f);
                            break;
                        }
                    }
                }

                if font.is_none() {
                    // look through the family for the (weight, width, slant)
                    // nearest to (80, 100, 0)
                    font = self.best_match_from_family(fam, 80, 100, 0);
                }

                if font.is_some() {
                    break;
                }
            }

            if pass == 0 {
                // didn't find it in our caches, so do a platform search
                // (may be relatively expensive); this will update the caches
                // with any fonts that seem to match the name given, so that
                // the second pass might find it
                self.search_for_host_platform_fonts(name);
            }
        }

        let mut font = font?;
        let parent = self.fonts[font].parent;

        // if there are variant requests, try to apply them and delete B, I,
        // and S=... codes from the string, just retain /engine option
        self.req_engine = 0;
        let mut req_bold = false;
        let mut req_ital = false;
        let mut new_variant = None;
        if let Some(variant) = variant {
            let mut var_string = String::new();
            let v = variant.as_bytes();
            let mut cp = 0usize;
            let at = |k: usize| v.get(k).copied().unwrap_or(0);
            let push_engine = |var_string: &mut String, e: &str| {
                if !var_string.is_empty() && !var_string.ends_with('/') {
                    var_string.push('/');
                }
                var_string.push_str(e);
            };
            while cp < v.len() {
                if v[cp..].starts_with(b"AAT") {
                    self.req_engine = b'A';
                    cp += 3;
                    push_engine(&mut var_string, "AAT");
                } else if v[cp..].starts_with(b"ICU") {
                    // for backword compatability
                    self.req_engine = b'O';
                    cp += 3;
                    push_engine(&mut var_string, "OT");
                } else if v[cp..].starts_with(b"OT") {
                    self.req_engine = b'O';
                    cp += 2;
                    push_engine(&mut var_string, "OT");
                } else if v[cp..].starts_with(b"GR") {
                    self.req_engine = b'G';
                    cp += 2;
                    push_engine(&mut var_string, "GR");
                } else if at(cp) == b'S' {
                    cp += 1;
                    if at(cp) == b'=' {
                        cp += 1;
                    }
                    pt_size = 0.0;
                    while at(cp).is_ascii_digit() {
                        pt_size = pt_size * 10.0 + f64::from(at(cp) - b'0');
                        cp += 1;
                    }
                    if at(cp) == b'.' {
                        let mut dec = 1.0;
                        cp += 1;
                        while at(cp).is_ascii_digit() {
                            dec *= 10.0;
                            pt_size += f64::from(at(cp) - b'0') / dec;
                            cp += 1;
                        }
                    }
                } else {
                    // if the code is "B" or "I", we skip putting it in varString
                    loop {
                        match at(cp) {
                            b'B' => req_bold = true,
                            b'I' => req_ital = true,
                            _ => break,
                        }
                        cp += 1;
                    }
                }
                // skip_to_slash:
                while cp < v.len() && v[cp] != b'/' {
                    cp += 1;
                }
                if at(cp) == b'/' {
                    cp += 1;
                }
            }
            new_variant = Some(var_string);

            if let Some(parent) = parent {
                if req_ital {
                    font = self.apply_italic(font, parent);
                }
                if req_bold {
                    font = self.apply_bold(font, parent);
                }
            }
        }

        // if there's optical size info, try to apply it
        if pt_size < 0.0 {
            pt_size = dsize;
        }
        let op = self.fonts[font].op_size_info;
        if op.sub_family_id != 0 && pt_size > 0.0 {
            let mut best_mismatch = my_fmax(op.min_size - pt_size, pt_size - op.max_size);
            if best_mismatch > 0.0 {
                if let Some(parent) = parent {
                    let mut best_match = font;
                    for &s in self.families[parent].styles.values() {
                        let o = self.fonts[s].op_size_info;
                        if o.sub_family_id != op.sub_family_id {
                            continue;
                        }
                        let mismatch = my_fmax(o.min_size - pt_size, pt_size - o.max_size);
                        if mismatch < best_mismatch {
                            best_match = s;
                            best_mismatch = mismatch;
                        }
                        if best_mismatch <= 0.0 {
                            break;
                        }
                    }
                    font = best_match;
                }
            }
        }

        let mut loaded_font_design_size = 655360;
        let ds = self.fonts[font].op_size_info.design_size;
        if ds != 0.0 {
            loaded_font_design_size = (ds * 65536.0 + 0.5) as u32 as i32;
        }

        Some(Found {
            face: self.fonts[font].face,
            variant: new_variant,
            loaded_font_design_size,
        })
    }

    /// findFont's `if (reqItal)` block.
    fn apply_italic(&self, font: usize, parent: usize) -> usize {
        let fam = &self.families[parent];
        let f = &self.fonts[font];
        let mut best_match = Some(font);
        if f.slant < fam.max_slant {
            // try for a face with more slant
            best_match = self.best_match_from_family(
                parent,
                i32::from(f.weight),
                i32::from(f.width),
                i32::from(fam.max_slant),
            );
        }

        if best_match == Some(font) && f.slant > fam.min_slant {
            // maybe the slant is negated, or maybe this was something like
            // "Times-Italic/I"
            best_match = self.best_match_from_family(
                parent,
                i32::from(f.weight),
                i32::from(f.width),
                i32::from(fam.min_slant),
            );
        }

        if fam.min_weight == fam.max_weight
            && best_match.map(|b| self.fonts[b].is_bold) != Some(f.is_bold)
        {
            // try again using the bold flag, as we can't trust weight values
            let mut new_best = None;
            for &s in fam.styles.values() {
                let sf = &self.fonts[s];
                if sf.is_bold == f.is_bold && new_best.is_none() && sf.is_italic != f.is_italic {
                    new_best = Some(s);
                    break;
                }
            }
            if new_best.is_some() {
                best_match = new_best;
            }
        }

        if best_match == Some(font) {
            // maybe slant values weren't present; try the style bits as a
            // fallback
            best_match = None;
            for &s in fam.styles.values() {
                let sf = &self.fonts[s];
                if sf.is_italic == !f.is_italic {
                    if fam.min_weight != fam.max_weight {
                        // weight info was available, so try to match that
                        if best_match.is_none_or(|b| {
                            self.weight_and_width_diff(sf, f)
                                < self.weight_and_width_diff(&self.fonts[b], f)
                        }) {
                            best_match = Some(s);
                        }
                    } else {
                        // no weight info, so try matching style bits
                        if best_match.is_none() && sf.is_bold == f.is_bold {
                            best_match = Some(s);
                            break; // found a match, no need to look further as we can't distinguish!
                        }
                    }
                }
            }
        }
        best_match.unwrap_or(font)
    }

    /// findFont's `if (reqBold)` block.
    fn apply_bold(&self, font: usize, parent: usize) -> usize {
        let fam = &self.families[parent];
        let f = &self.fonts[font];
        // try for more boldness, with the same width and slant
        let mut best_match = Some(font);
        if f.weight < fam.max_weight {
            // try to increase weight by 1/2 x (max - min), rounding up
            best_match = self.best_match_from_family(
                parent,
                i32::from(f.weight)
                    + (i32::from(fam.max_weight) - i32::from(fam.min_weight)) / 2
                    + 1,
                i32::from(f.width),
                i32::from(f.slant),
            );
            if fam.min_slant == fam.max_slant {
                // double-check the italic flag, as we can't trust slant values
                let mut new_best: Option<usize> = None;
                if let Some(b) = best_match {
                    let bf = &self.fonts[b];
                    for &s in fam.styles.values() {
                        let sf = &self.fonts[s];
                        if sf.is_italic == f.is_italic
                            && new_best.is_none_or(|n| {
                                self.weight_and_width_diff(sf, bf)
                                    < self.weight_and_width_diff(&self.fonts[n], bf)
                            })
                        {
                            new_best = Some(s);
                        }
                    }
                }
                if new_best.is_some() {
                    best_match = new_best;
                }
            }
        }
        if best_match == Some(font) && !f.is_bold {
            for &s in fam.styles.values() {
                let sf = &self.fonts[s];
                if sf.is_italic == f.is_italic && sf.is_bold {
                    best_match = Some(s);
                    break;
                }
            }
        }
        // XeTeX assigns bestMatch even when it is NULL (an empty family,
        // which cannot hold `font`); keep the font instead of crashing.
        best_match.unwrap_or(font)
    }

    /// `XeTeXFontMgr::weightAndWidthDiff`.
    pub fn weight_and_width_diff(&self, a: &Font, b: &Font) -> i32 {
        if a.weight == 0 && a.width == 0 {
            // assume there was no OS/2 info
            return if a.is_bold == b.is_bold { 0 } else { 10000 };
        }

        let mut wid_diff = (i32::from(a.width) - i32::from(b.width)).abs();
        if wid_diff < 10 {
            wid_diff *= 50;
        }

        (i32::from(a.weight) - i32::from(b.weight)).abs() + wid_diff
    }

    /// `XeTeXFontMgr::styleDiff`.
    pub fn style_diff(a: &Font, wt: i32, wd: i32, slant: i32) -> i32 {
        let mut wid_diff = (i32::from(a.width) - wd).abs();
        if wid_diff < 10 {
            wid_diff *= 200;
        }

        ((i32::from(a.slant)).abs() - slant.abs()).abs() * 2
            + (i32::from(a.weight) - wt).abs()
            + wid_diff
    }

    /// `XeTeXFontMgr::bestMatchFromFamily`: the first style (in name order)
    /// with the smallest `styleDiff`.
    pub fn best_match_from_family(
        &self,
        fam: usize,
        wt: i32,
        wd: i32,
        slant: i32,
    ) -> Option<usize> {
        let mut best_match: Option<usize> = None;
        for &s in self.families[fam].styles.values() {
            if best_match.is_none_or(|b| {
                Self::style_diff(&self.fonts[s], wt, wd, slant)
                    < Self::style_diff(&self.fonts[b], wt, wd, slant)
            }) {
                best_match = Some(s);
            }
        }
        best_match
    }

    /// `XeTeXFontMgr::getOpSizeRecAndStyleFlags` (the base class's: the
    /// macOS manager's too. `XeTeXFontMgr_FC`'s override, which falls back
    /// to fontconfig's weight, width and slant for a face without `OS/2`,
    /// is not ported: there is no fontconfig, and the oracle on macOS does
    /// not do it either).
    fn op_size_rec_and_style_flags(&self, the_font: &mut Font) {
        let Some(st) = self.catalog.faces()[the_font.face].style() else {
            return;
        };
        if let Some(p) = st.size {
            the_font.op_size_info.design_size = super::catalog::decipoints_to_tex(p.design_size);
            if !(p.subfamily_id == 0
                && p.subfamily_name_id == 0
                && p.range_start == 0
                && p.range_end == 0)
            {
                // (otherwise the feature is valid, but no 'size' range)
                the_font.op_size_info.sub_family_id = u32::from(p.subfamily_id);
                the_font.op_size_info.name_code = u32::from(p.subfamily_name_id);
                the_font.op_size_info.min_size = super::catalog::decipoints_to_tex(p.range_start);
                the_font.op_size_info.max_size = super::catalog::decipoints_to_tex(p.range_end);
            }
        }

        if let Some((weight, width, sel)) = st.os2 {
            the_font.weight = weight;
            the_font.width = width;
            the_font.is_reg = sel & (1 << 6) != 0;
            the_font.is_bold = sel & (1 << 5) != 0;
            the_font.is_italic = sel & (1 << 0) != 0;
        }

        let ms = st.mac_style;
        if ms & (1 << 0) != 0 {
            the_font.is_bold = true;
        }
        if ms & (1 << 1) != 0 {
            the_font.is_italic = true;
        }

        // (int)(1000 * tan(Fix2D(-italicAngle) * M_PI / 180.0)), stored in
        // an int16_t
        let angle = -(i64::from(st.italic_angle) as f64) / 65536.0;
        the_font.slant = ((1000.0 * (angle * std::f64::consts::PI / 180.0).tan()) as i32) as i16;
    }

    /// `XeTeXFontMgr::addToMaps`.
    pub fn add_to_maps(&mut self, platform_font: usize, names: &NameCollection) {
        if self.platform_ref_to_font.contains_key(&platform_font) {
            return; // this font has already been cached
        }

        if names.ps_name.is_empty() {
            return; // can't use a font that lacks a PostScript name
        }

        if self.ps_name_to_font.contains_key(&names.ps_name) {
            return; // duplicates an earlier PS name, so skip
        }

        let mut this_font = Font {
            face: platform_font,
            full_name: None,
            ps_name: names.ps_name.clone(),
            family_name: String::new(),
            style_name: String::new(),
            parent: None,
            op_size_info: OpSizeRec {
                design_size: 10.0,
                min_size: 0.0,
                max_size: 0.0,
                sub_family_id: 0,
                name_code: 0,
            },
            weight: 0,
            width: 0,
            slant: 0,
            is_reg: false,
            is_bold: false,
            is_italic: false,
        };
        self.op_size_rec_and_style_flags(&mut this_font);

        this_font.full_name = names.full_names.first().cloned();
        this_font.family_name = names
            .family_names
            .first()
            .cloned()
            .unwrap_or_else(|| names.ps_name.clone());
        this_font.style_name = names.style_names.first().cloned().unwrap_or_default();

        let id = self.fonts.len();
        let (weight, width, slant) = (this_font.weight, this_font.width, this_font.slant);
        self.fonts.push(this_font);
        self.ps_name_to_font.insert(names.ps_name.clone(), id);
        self.platform_ref_to_font.insert(platform_font, id);

        for fname in &names.family_names {
            let family = match self.name_to_family.get(fname) {
                None => {
                    let fam = self.families.len();
                    self.families.push(Family {
                        styles: BTreeMap::new(),
                        min_weight: weight,
                        max_weight: weight,
                        min_width: width,
                        max_width: width,
                        min_slant: slant,
                        max_slant: slant,
                    });
                    self.name_to_family.insert(fname.clone(), fam);
                    fam
                }
                Some(&fam) => {
                    let f = &mut self.families[fam];
                    f.min_weight = f.min_weight.min(weight);
                    f.max_weight = f.max_weight.max(weight);
                    f.min_width = f.min_width.min(width);
                    f.max_width = f.max_width.max(width);
                    f.min_slant = f.min_slant.min(slant);
                    f.max_slant = f.max_slant.max(slant);
                    fam
                }
            };

            if self.fonts[id].parent.is_none() {
                self.fonts[id].parent = Some(family);
            }

            // ensure all style names in the family point to thisFont
            for style in &names.style_names {
                self.families[family]
                    .styles
                    .entry(style.clone())
                    .or_insert(id);
            }
        }

        for full in &names.full_names {
            self.name_to_font.entry(full.clone()).or_insert(id);
        }
    }

    /// Reads a catalog face's names and adds it (`readNames` +
    /// `addToMaps`).
    fn add_face(&mut self, f: usize) {
        let catalog = Arc::clone(&self.catalog);
        self.add_to_maps(f, &catalog.faces()[f].names);
    }

    /// `XeTeXFontMgr_FC::cacheFamilyMembers`: add every face one of whose
    /// pattern families is one of `family_names`.
    fn cache_family_members(&mut self, family_names: &[String]) {
        if family_names.is_empty() {
            return;
        }
        let catalog = Arc::clone(&self.catalog);
        for (f, face) in catalog.faces().iter().enumerate() {
            if self.platform_ref_to_font.contains_key(&f) {
                continue;
            }
            if face
                .pattern
                .families
                .iter()
                .any(|s| family_names.iter().any(|j| j == s))
            {
                self.add_face(f);
            }
        }
    }

    /// `XeTeXFontMgr_FC::searchForHostPlatformFonts`: add every face whose
    /// full name is `name`, or one of whose families is `name` (or the part
    /// before a hyphen), or whose "family style" is `name`, together with
    /// the members of its families. If none does, add every face, once.
    pub fn search_for_host_platform_fonts(&mut self, name: &str) {
        if self.cached_all {
            // we've already loaded everything on an earlier search
            return;
        }

        let mut fam_name = "";
        let mut hyph = 0;
        if let Some(h) = name.find('-') {
            if h > 0 && h < name.len() - 1 {
                fam_name = &name[..h];
                hyph = h;
            }
        }

        let catalog = Arc::clone(&self.catalog);
        let mut found = false;
        loop {
            for (f, face) in catalog.faces().iter().enumerate() {
                if self.platform_ref_to_font.contains_key(&f) {
                    continue;
                }

                if self.cached_all {
                    // failed to find it via FC; add everything to our maps
                    // (potentially slow) as a last resort
                    self.add_face(f);
                    continue;
                }

                let p = &face.pattern;
                let matched = p.full_names.iter().any(|s| s == name)
                    || p.families.iter().any(|s| {
                        s == name
                            || (hyph != 0 && fam_name == s)
                            || p.styles.iter().any(|t| {
                                name.len() == s.len() + 1 + t.len() && name == format!("{s} {t}")
                            })
                    });
                if matched {
                    self.add_face(f);
                    let families = face.names.family_names.clone();
                    self.cache_family_members(&families);
                    found = true;
                }
            }

            if found || self.cached_all {
                break;
            }
            self.cached_all = true;
        }
    }
}
