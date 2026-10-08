//! Native fonts (docs/design/xetex/PLAN.md §3.1, phase S1): the parts of
//! `XeTeX_ext.c` that load OpenType and TrueType fonts and measure native
//! word nodes, over `XeTeXFontInst.cpp` ([`font_inst`]) and
//! `XeTeXLayoutInterface.cpp` ([`layout`]), with HarfBuzz 12.3.2 and
//! FreeType 2.14.1 as TeX Live 2026 builds them (`crate::fontlibs`).
//!
//! XeTeX's C is MIT-licensed (third_party/xetex/COPYING); each routine
//! here names the C routine it ports. A font's layout engine is a handle
//! of `Host::handles` (`changes/ext.ch`), so a checkpoint keeps it.
//!
//! Not in S1: AAT (Core Text) shaping, out of scope by the Commander's
//! ruling (PLAN.md §3.1): a font XeTeX on macOS would shape with AAT is
//! shaped with HarfBuzz; Graphite (S2); OpenType math (S2); ICU's
//! bidirectional analysis, for which text that needs it is reported (see
//! [`needs_bidi`]) and laid out left to right.

pub mod font_inst;
pub mod layout;

use crate::fontlibs::hb;
use crate::generated::consts::otgr_font_flag;
use crate::generated::Globals;
use crate::state::{GlyphInfo, Object};
use font_inst::{d2fix, fix2d, FontInst, FtLibrary, GlyphBBox};
use layout::LayoutEngine;
use std::ffi::CString;
use std::sync::Arc;

const FONT_FLAGS_COLORED: i32 = 0x01;
const FONT_FLAGS_VERTICAL: i32 = 0x02;

const XDV_FLAG_VERTICAL: u16 = 0x0100;
const XDV_FLAG_COLORED: u16 = 0x0200;
const XDV_FLAG_EXTEND: u16 = 0x1000;
const XDV_FLAG_SLANT: u16 = 0x2000;
const XDV_FLAG_EMBOLDEN: u16 = 0x4000;

/// `splitFontName`: the positions of the variant (`/...`) and feature
/// (`:...`) parts of a font name, its end, and a `[file:index]` face
/// index. `var <= feat <= end`, as in C.
pub fn split_font_name(name: &[u8]) -> (usize, usize, usize, u32) {
    let mut var: Option<usize> = None;
    let mut feat: Option<usize> = None;
    let mut index: u32 = 0;
    let mut i = 0;
    if name.first() == Some(&b'[') {
        let mut within = true;
        i = 1;
        while i < name.len() {
            let c = name[i];
            if within && c == b']' {
                within = false;
                if var.is_none() {
                    var = Some(i);
                }
            } else if c == b':' {
                if within && var.is_none() {
                    var = Some(i);
                    i += 1;
                    while i < name.len() && name[i].is_ascii_digit() {
                        index = index.wrapping_mul(10).wrapping_add((name[i] - b'0') as u32);
                        i += 1;
                    }
                    i -= 1;
                } else if !within && feat.is_none() {
                    feat = Some(i);
                }
            }
            i += 1;
        }
    } else {
        while i < name.len() {
            let c = name[i];
            if c == b'/' && var.is_none() && feat.is_none() {
                var = Some(i);
            } else if c == b':' && feat.is_none() {
                feat = Some(i);
            }
            i += 1;
        }
    }
    let end = i;
    let feat = feat.unwrap_or(end);
    let var = var.unwrap_or(feat);
    (var, feat, end, index)
}

/// `read_double`: C's loop, digit by digit, in `double`.
fn read_double(s: &[u8], mut cp: usize) -> (f64, usize) {
    let at = |k: usize| s.get(k).copied().unwrap_or(0);
    let mut neg = false;
    let mut val = 0.0f64;
    while at(cp) == b' ' || at(cp) == b'\t' {
        cp += 1;
    }
    if at(cp) == b'-' {
        neg = true;
        cp += 1;
    } else if at(cp) == b'+' {
        cp += 1;
    }
    while at(cp).is_ascii_digit() {
        val = val * 10.0 + (at(cp) - b'0') as f64;
        cp += 1;
    }
    if at(cp) == b'.' {
        let mut dec = 10.0f64;
        cp += 1;
        while at(cp).is_ascii_digit() {
            val += (at(cp) - b'0') as f64 / dec;
            cp += 1;
            dec *= 10.0;
        }
    }
    (if neg { -val } else { val }, cp)
}

fn tag_from(s: &[u8]) -> hb::hb_tag_t {
    // SAFETY: HarfBuzz reads `len` bytes.
    unsafe { hb::hb_tag_from_string(s.as_ptr() as *const _, s.len() as i32) }
}

/// `read_tag_with_param`: the tag before `:;,=` and an `=n` parameter.
fn read_tag_with_param(s: &[u8], mut param: i32) -> (hb::hb_tag_t, i32) {
    let at = |k: usize| s.get(k).copied().unwrap_or(0);
    let mut cp2 = 0;
    while at(cp2) != 0 && !matches!(at(cp2), b':' | b';' | b',' | b'=') {
        cp2 += 1;
    }
    let tag = tag_from(&s[..cp2.min(s.len())]);
    let mut cp = cp2;
    if at(cp) == b'=' {
        let mut neg = false;
        cp += 1;
        if at(cp) == b'-' {
            neg = true;
            cp += 1;
        }
        while at(cp).is_ascii_digit() {
            param = param.wrapping_mul(10).wrapping_add((at(cp) - b'0') as i32);
            cp += 1;
        }
        if neg {
            param = -param;
        }
    }
    (tag, param)
}

/// `read_rgb_a`: the colour and how many bytes it took.
fn read_rgb_a(s: &[u8]) -> (u32, usize) {
    let hex = |c: u8| match c {
        b'0'..=b'9' => Some((c - b'0') as u32),
        b'A'..=b'F' => Some((c - b'A' + 10) as u32),
        b'a'..=b'f' => Some((c - b'a' + 10) as u32),
        _ => None,
    };
    let at = |k: usize| s.get(k).copied().unwrap_or(0);
    let mut rgb: u32 = 0;
    let mut cp = 0;
    for _ in 0..6 {
        match hex(at(cp)) {
            Some(v) => rgb = (rgb << 4) + v,
            None => return (0x0000_00FF, cp),
        }
        cp += 1;
    }
    rgb <<= 8;
    let mut alpha = 0;
    let mut i = 0;
    while i < 2 {
        match hex(at(cp)) {
            Some(v) => alpha = (alpha << 4) + v,
            None => break,
        }
        cp += 1;
        i += 1;
    }
    if i == 2 {
        rgb += alpha;
    } else {
        rgb += 0xFF;
    }
    (rgb, cp)
}

/// Whether text needs ICU's bidirectional analysis: it holds a character
/// of a right-to-left script, an Arabic number, or an explicit embedding,
/// override or isolate. Text without any is one left-to-right run for
/// `ubidi` (its direction is `UBIDI_LTR`), which is all phase S1 lays out.
pub fn needs_bidi(text: &[u16]) -> bool {
    let mut i = 0;
    while i < text.len() {
        let mut c = text[i] as u32;
        if (0xD800..0xDC00).contains(&c) && i + 1 < text.len() {
            let lo = text[i + 1] as u32;
            if (0xDC00..0xE000).contains(&lo) {
                c = 0x10000 + ((c - 0xD800) << 10) + (lo - 0xDC00);
                i += 1;
            }
        }
        i += 1;
        if matches!(c,
            0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF
            | 0x10800..=0x10FFF | 0x1E800..=0x1EFFF
            | 0x200F | 0x202A..=0x202E | 0x2066..=0x2069)
        {
            return true;
        }
    }
    false
}

impl Globals {
    /// The FreeType library of this engine, made on first use.
    fn ft_library(&mut self) -> Arc<FtLibrary> {
        self.host
            .ft_library
            .get_or_insert_with(FtLibrary::new)
            .clone()
    }

    /// The layout engine of handle `h`.
    pub(crate) fn layout_engine(&self, h: i32) -> Option<Arc<LayoutEngine>> {
        match self.host.handles.get(h)? {
            Object::Other(o) => o.clone().downcast::<LayoutEngine>().ok(),
            _ => None,
        }
    }

    /// The layout engine of font `f`.
    fn font_engine(&self, f: i32) -> Option<Arc<LayoutEngine>> {
        if f < 0 || self.font_area[f as usize] != otgr_font_flag {
            return None;
        }
        self.layout_engine(self.font_layout_engine[f as usize])
    }

    /// `createFontFromFile`: a font at `point_size` (TeX points, as
    /// `Fix2D(pointSize)`).
    fn create_font_from_file(
        &mut self,
        path: &[u8],
        index: u32,
        point_size: i32,
    ) -> Option<FontInst> {
        let lib = self.ft_library();
        // A Type 1 font's AFM file is not attached: the pdfTeX engine's
        // resolver has no `afm` format, and S1's corpus has no Type 1
        // native fonts (their kerns come from the AFM in XeTeX).
        FontInst::new(&lib, path, index, fix2d(point_size) as f32, &mut |_afm| {
            None
        })
    }

    /// `XeTeXFontMgr::getDesignSize`: the `size` feature's design size in
    /// TeX points, else 10.
    fn design_size(font: &FontInst) -> f64 {
        let (mut design, mut sub, mut name, mut min, mut max) = (0, 0, 0, 0, 0);
        // SAFETY: a live face.
        let ok = unsafe {
            hb::hb_ot_layout_get_size_params(
                font.hb_face(),
                &mut design,
                &mut sub,
                &mut name,
                &mut min,
                &mut max,
            ) != 0
        };
        if ok {
            design as f64 * 72.27 / 72.0 / 10.0
        } else {
            10.0
        }
    }

    /// `findnativefont`: the layout engine of the font `name_of_file`
    /// names at size `s` (TeX points, or negative for `scaled`), as a
    /// handle; 0 if there is none.
    pub fn find_native_font(&mut self, s: i32) -> i32 {
        let mut scaled_size = s;
        self.loaded_font_mapping = 0;
        self.loaded_font_flags = 0;
        self.loaded_font_letter_space = 0;

        let n = (self.name_length.max(0) as usize).min(self.name_of_file.len());
        let raw = &self.name_of_file[..n];
        let name: Vec<u8> = raw.iter().copied().take_while(|&b| b != 0).collect();
        let (var, feat, end, index) = split_font_name(&name);
        let name_string = name[..var].to_vec();
        let var_string = (feat > var).then(|| name[var + 1..feat].to_vec());
        let feat_string = (end > feat).then(|| name[feat + 1..end].to_vec());

        let mut rval = 0;
        if name_string.first() == Some(&b'[') {
            // The "[filename]" form: kpathsea's OpenType, TrueType and Type 1
            // paths, no font manager.
            use flashtex_engine::resolver::Format;
            let file = String::from_utf8_lossy(&name_string[1..]).into_owned();
            let path = flashtex_engine::system::find_file(&file, Format::OpenType)
                .or_else(|| flashtex_engine::system::find_file(&file, Format::TrueType))
                .or_else(|| flashtex_engine::system::find_file(&file, Format::Type1));
            if let Some(path) = path {
                let path = path.into_bytes();
                if scaled_size < 0 {
                    if let Some(font) = self.create_font_from_file(&path, index, 655360) {
                        let dsize = d2fix(Self::design_size(&font));
                        scaled_size = if scaled_size == -1000 {
                            dsize
                        } else {
                            self.xn_over_d(dsize, -scaled_size, 1000)
                        };
                    }
                }
                if let Some(font) = self.create_font_from_file(&path, index, scaled_size) {
                    self.loaded_font_design_size = d2fix(Self::design_size(&font));
                    // As XeTeXFontMgr::findFont does.
                    self.host.req_engine = 0;
                    if let Some(v) = &var_string {
                        if v.starts_with(b"/AAT") {
                            self.host.req_engine = b'A';
                        } else if v.starts_with(b"/OT") || v.starts_with(b"/ICU") {
                            self.host.req_engine = b'O';
                        } else if v.starts_with(b"/GR") {
                            self.host.req_engine = b'G';
                        }
                    }
                    rval = self.load_ot_font(font, scaled_size, feat_string.as_deref());
                    if rval != 0 && self.get_tracing_fonts_state() > 0 {
                        self.begin_diagnostic();
                        self.print_nl(b' ' as i32);
                        self.print_bytes(b"-> ");
                        self.print_bytes(&path);
                        self.end_diagnostic(false);
                    }
                }
            }
        } else {
            rval =
                self.find_native_font_by_name(&name_string, var_string, feat_string, scaled_size);
        }
        rval
    }

    /// The other branch of `findnativefont`: a font looked up by name
    /// through the font manager (`crate::fontmgr`, XeTeXFontMgr's rules
    /// over the platform-free index). A font XeTeX on macOS would shape
    /// with AAT (`/AAT`, or no `GSUB`/`GPOS` table) is loaded as an
    /// OpenType font, shaped by HarfBuzz (PLAN.md §3.1, out of scope).
    fn find_native_font_by_name(
        &mut self,
        name: &[u8],
        var: Option<Vec<u8>>,
        feat: Option<Vec<u8>>,
        scaled_size: i32,
    ) -> i32 {
        let mut full = name.to_vec();
        if let Some(v) = &var {
            full.push(b'/');
            full.extend_from_slice(v);
        }
        if let Some(f) = &feat {
            full.push(b':');
            full.extend_from_slice(f);
        }
        let mgr = self.host.font_mgr.get_or_insert_with(|| {
            Arc::new(crate::fontmgr::FontMgr::new(std::sync::Arc::new(
                crate::fontmgr::FontCatalog::system_cached(None),
            )))
        });
        let located = Arc::make_mut(mgr).locate(&String::from_utf8_lossy(&full), scaled_size);
        let Some(located) = located else {
            return 0;
        };
        self.host.req_engine = located.req_engine;
        if self.get_tracing_fonts_state() > 0 {
            // XeTeXFontMgr::findFont's diagnostic.
            self.begin_diagnostic();
            self.print_nl(b' ' as i32);
            self.print_bytes(b"-> ");
            self.print_bytes(located.path.as_bytes());
            self.end_diagnostic(false);
        }
        self.loaded_font_design_size = located.loaded_font_design_size;
        // name_of_file becomes the full name, for messages while loading.
        let full_name = located.full_name.clone().unwrap_or_default();
        self.set_name_of_file_bytes(full_name.as_bytes());
        let mut rval = 0;
        let scaled_size = located.scaled_size;
        if let Some(font) =
            self.create_font_from_file(located.path.as_bytes(), located.face_index, scaled_size)
        {
            rval = self.load_ot_font(font, scaled_size, feat.as_deref());
        }
        // The style and feature strings are appended, so that \show of the
        // font gives a full result.
        let nof = located.name_of_file.unwrap_or(full_name);
        self.set_name_of_file_bytes(nof.as_bytes());
        rval
    }

    /// `name_of_file` (and `name_length`) set to `s`, NUL-terminated as
    /// XeTeX's C string.
    fn set_name_of_file_bytes(&mut self, s: &[u8]) {
        let n = s.len().min(self.name_of_file.len() - 1);
        self.name_of_file[..n].copy_from_slice(&s[..n]);
        self.name_of_file[n] = 0;
        self.name_length = n as i32;
    }

    /// texmfmp.c's `printcstring`: bytes, one `print_char` each.
    pub(crate) fn print_bytes(&mut self, s: &[u8]) {
        for &b in s {
            self.print_char(b as i32);
        }
    }

    /// `fontfeaturewarning(cp1, len, 0, 0)`: the feature name is passed to
    /// xetex.web as a handle to its bytes.
    fn feature_warning(&mut self, feature: &[u8]) {
        let h = self
            .host
            .handles
            .alloc(Object::Other(Arc::new(feature.to_vec())));
        self.font_feature_warning(h, feature.len() as i32, 0, 0);
        self.host.handles.free(h);
    }

    /// `printutf8str`: the bytes of handle `s` (a C string in TeX Live).
    pub fn print_utf8_str(&mut self, s: i32, len: i32) {
        let bytes: Vec<u8> = match self.host.handles.get(s) {
            Some(Object::Other(o)) => o
                .downcast_ref::<Vec<u8>>()
                .map(|v| v[..(len.max(0) as usize).min(v.len())].to_vec())
                .unwrap_or_default(),
            _ => vec![],
        };
        self.print_bytes(&bytes);
    }

    /// `loadOTfont`: parse the feature string, make the layout engine.
    fn load_ot_font(&mut self, mut font: FontInst, scaled_size: i32, feat: Option<&[u8]>) -> i32 {
        let req_engine = self.host.req_engine;
        let mut script: hb::hb_tag_t = 0;
        let mut language: Option<Vec<u8>> = None;
        let mut features: Vec<hb::hb_feature_t> = vec![];
        let mut shapers: Option<Vec<CString>> = None;
        let mut rgb_value: u32 = 0x0000_00FF;
        let mut extend: f32 = 1.0;
        let mut slant: f32 = 0.0;
        let mut embolden: f32 = 0.0;
        let mut letterspace: f32 = 0.0;

        if req_engine == b'O' || req_engine == b'G' {
            let s = if req_engine == b'O' {
                "ot"
            } else {
                "graphite2"
            };
            shapers
                .get_or_insert_with(Vec::new)
                .push(CString::new(s).unwrap());
        }

        if let Some(f) = feat {
            let at = |k: usize| f.get(k).copied().unwrap_or(0);
            let mut cp1 = 0;
            while at(cp1) != 0 {
                if matches!(at(cp1), b':' | b';' | b',') {
                    cp1 += 1;
                }
                while at(cp1) == b' ' || at(cp1) == b'\t' {
                    cp1 += 1;
                }
                if at(cp1) == 0 {
                    break;
                }
                let mut cp2 = cp1;
                while at(cp2) != 0 && !matches!(at(cp2), b':' | b';' | b',') {
                    cp2 += 1;
                }
                let opt = &f[cp1..cp2];
                let mut ok = false;
                if let Some(rest) = opt.strip_prefix(b"script") {
                    if rest.first() == Some(&b'=') {
                        script = tag_from(&rest[1..]);
                        ok = true;
                    }
                } else if let Some(rest) = opt.strip_prefix(b"language") {
                    if rest.first() == Some(&b'=') {
                        language = Some(rest[1..].to_vec());
                        ok = true;
                    }
                } else if let Some(rest) = opt.strip_prefix(b"shaper") {
                    if rest.first() == Some(&b'=') {
                        let mut v = rest[1..].to_vec();
                        if let Some(z) = v.iter().position(|&b| b == 0) {
                            v.truncate(z);
                        }
                        shapers
                            .get_or_insert_with(Vec::new)
                            .push(CString::new(v).unwrap_or_default());
                        ok = true;
                    }
                } else {
                    match self.read_common_features(
                        opt,
                        &mut extend,
                        &mut slant,
                        &mut embolden,
                        &mut letterspace,
                        &mut rgb_value,
                    ) {
                        1 => ok = true,
                        -1 => {}
                        _ => {
                            if opt.first() == Some(&b'+') {
                                let (tag, mut param) = read_tag_with_param(&opt[1..], 0);
                                // Feature indices started from 0 before 0.9999.
                                if param >= 0 {
                                    param += 1;
                                }
                                features.push(hb::hb_feature_t {
                                    tag,
                                    value: param as u32,
                                    start: 0,
                                    end: u32::MAX,
                                });
                                ok = true;
                            } else if opt.first() == Some(&b'-') {
                                features.push(hb::hb_feature_t {
                                    tag: tag_from(&opt[1..]),
                                    value: 0,
                                    start: 0,
                                    end: u32::MAX,
                                });
                                ok = true;
                            } else if opt.starts_with(b"vertical") {
                                // `vertical`, allowing trailing blanks.
                                let mut cp3 = cp2 as isize;
                                if matches!(at(cp3 as usize), b';' | b':' | b',') {
                                    cp3 -= 1;
                                }
                                while cp3 >= 0 && matches!(at(cp3 as usize), 0 | b' ' | b'\t') {
                                    cp3 -= 1;
                                }
                                if cp3 >= 0 && at(cp3 as usize) != 0 {
                                    cp3 += 1;
                                }
                                if cp3 == (cp1 + 8) as isize {
                                    self.loaded_font_flags |= FONT_FLAGS_VERTICAL;
                                    ok = true;
                                }
                            }
                        }
                    }
                }
                if !ok {
                    self.feature_warning(opt);
                }
                cp1 = cp2;
            }
        }

        if embolden != 0.0 {
            embolden = (embolden as f64 * fix2d(scaled_size) / 100.0) as f32;
        }
        if letterspace != 0.0 {
            self.loaded_font_letter_space =
                ((letterspace as f64 / 100.0) * scaled_size as f64) as i32;
        }
        if self.loaded_font_flags & FONT_FLAGS_COLORED == 0 {
            rgb_value = 0x0000_00FF;
        }
        if self.loaded_font_flags & FONT_FLAGS_VERTICAL != 0 {
            font.vertical = true;
        }
        let engine = LayoutEngine::new(
            font,
            script,
            language.as_deref(),
            features,
            shapers,
            rgb_value,
            extend,
            slant,
            embolden,
            req_engine,
        );
        self.native_font_type_flag = otgr_font_flag;
        self.host.handles.alloc(Object::Other(Arc::new(engine)))
    }

    /// `readCommonFeatures`: 1 for a recognised option, -1 for a bad one,
    /// 0 for another kind.
    fn read_common_features(
        &mut self,
        feat: &[u8],
        extend: &mut f32,
        slant: &mut f32,
        embolden: &mut f32,
        letterspace: &mut f32,
        rgb_value: &mut u32,
    ) -> i32 {
        let value = |key: &[u8]| -> Option<Option<usize>> {
            feat.starts_with(key)
                .then(|| (feat.get(key.len()) == Some(&b'=')).then_some(key.len() + 1))
        };
        if let Some(v) = value(b"mapping") {
            let Some(start) = v else { return -1 };
            let m = self.load_mapping_file(&feat[start..], false);
            self.loaded_font_mapping = m;
            return 1;
        }
        for (key, slot) in [
            (&b"extend"[..], 0),
            (&b"slant"[..], 1),
            (&b"embolden"[..], 2),
            (&b"letterspace"[..], 3),
        ] {
            if let Some(v) = value(key) {
                let Some(start) = v else { return -1 };
                let (d, _) = read_double(feat, start);
                match slot {
                    0 => *extend = d as f32,
                    1 => *slant = d as f32,
                    2 => *embolden = d as f32,
                    _ => *letterspace = d as f32,
                }
                return 1;
            }
        }
        if let Some(v) = value(b"color") {
            let Some(start) = v else { return -1 };
            let (rgb, used) = read_rgb_a(&feat[start..]);
            *rgb_value = rgb;
            if used == 6 || used == 8 {
                self.loaded_font_flags |= FONT_FLAGS_COLORED;
            } else {
                return -1;
            }
            return 1;
        }
        0
    }

    /// `load_mapping_file`: the TECkit mapping `name.tec`, found as kpathsea's
    /// `misc fonts`, as a handle (0 if none): a byte mapping for a TFM font
    /// (`byte_mapping`), else a UTF-16 one. Reported as XeTeX does: not
    /// found (1), not usable (2), or, with `\XeTeXtracingfonts` above 1,
    /// loaded (0).
    ///
    /// C's `kpse_find_file(buffer, kpse_miscfonts_format, 1)` is the
    /// resolver's lookup of the name, which searches what kpathsea's
    /// `ls-R` databases and the path's plain directories (`.`) hold;
    /// `must_exist`'s further disk search of `ls-R` trees is not made.
    pub(crate) fn load_mapping_file(&mut self, name: &[u8], byte_mapping: bool) -> i32 {
        use flashtex_engine::resolver::Format;
        // strncpy(buffer, s, e - s): the name ends at a NUL.
        let mut buffer = name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())].to_vec();
        buffer.extend_from_slice(b".tec");
        let file = String::from_utf8_lossy(&buffer).into_owned();
        let (mapping, warning) = match flashtex_engine::system::find_file(&file, Format::MiscFonts)
        {
            Some(path) => {
                let mapping = std::fs::read(&path)
                    .ok()
                    .and_then(|tec| crate::xetex_ext::Mapping::new(tec, byte_mapping));
                let warning = if mapping.is_none() {
                    Some(2)
                } else if self.get_tracing_fonts_state() > 1 {
                    Some(0)
                } else {
                    None
                };
                (mapping, warning)
            }
            None => (None, Some(1)),
        };
        if let Some(w) = warning {
            let h = self
                .host
                .handles
                .alloc(Object::Other(Arc::new(buffer.clone())));
            self.font_mapping_warning(h, buffer.len() as i32, w);
            self.host.handles.free(h);
        }
        match mapping {
            Some(m) => self.host.handles.alloc(Object::Other(Arc::new(m))),
            None => 0,
        }
    }

    /// `releasefontengine`.
    pub fn release_font_engine(&mut self, engine: i32, type_flag: i32) {
        if type_flag == otgr_font_flag {
            self.host.handles.free(engine);
        }
    }

    /// `otgetfontmetrics`.
    pub fn ot_get_font_metrics(
        &mut self,
        engine: i32,
        ascent: &mut i32,
        descent: &mut i32,
        xheight: &mut i32,
        capheight: &mut i32,
        slant: &mut i32,
    ) {
        let Some(e) = self.layout_engine(engine) else {
            return;
        };
        let f = &e.font;
        *ascent = d2fix(f.ascent as f64);
        *descent = d2fix(f.descent as f64);
        // getSlant: D2Fix(tan(-italAngle * M_PI / 180.0)).
        let font_slant = d2fix((-(f.italic_angle) as f64 * std::f64::consts::PI / 180.0).tan());
        *slant = d2fix(fix2d(font_slant) * e.extend as f64 + e.slant as f64);
        *capheight = d2fix(f.cap_height as f64);
        *xheight = d2fix(f.x_height as f64);
        // Fallbacks for a font without an OS/2 table.
        if *xheight == 0 {
            let gid = f.map_char_to_glyph(b'x' as u32);
            if gid != 0 {
                *xheight = d2fix(f.glyph_height_depth(gid).0 as f64);
            } else {
                *xheight = *ascent / 2;
            }
        }
        if *capheight == 0 {
            let gid = f.map_char_to_glyph(b'X' as u32);
            if gid != 0 {
                *capheight = d2fix(f.glyph_height_depth(gid).0 as f64);
            } else {
                *capheight = *ascent;
            }
        }
    }

    /// AAT fonts are never made (PLAN.md §3.1), so this is never reached.
    pub fn aat_get_font_metrics(
        &mut self,
        _engine: i32,
        a: &mut i32,
        d: &mut i32,
        xh: &mut i32,
        ch: &mut i32,
        sl: &mut i32,
    ) {
        (*a, *d, *xh, *ch, *sl) = (0, 0, 0, 0, 0);
    }

    // ---- native word nodes ---------------------------------------------

    fn node_text(&mut self, p: i32) -> Vec<u16> {
        let len = self.mem[(p + 4) as usize].qqqq_b2();
        (0..len)
            .map(|i| self.get_native_char(p, i) as u16)
            .collect()
    }

    fn native_font_of(&self, p: i32) -> i32 {
        self.mem[(p + 4) as usize].qqqq_b1()
    }

    fn glyph_info_of(&self, p: i32) -> Option<Arc<GlyphInfo>> {
        self.host
            .handles
            .glyph_info(self.mem[(p + 5) as usize].int())
            .cloned()
    }

    /// Store a node's new glyph-info array, freeing the old handle first
    /// (XeTeX overwrites the pointer; the handle is reused, so the table
    /// does not grow where C leaks).
    fn set_glyph_info(&mut self, p: i32, gi: Option<GlyphInfo>) {
        let old = self.mem[(p + 5) as usize].int();
        self.host.handles.free(old);
        let count = gi.as_ref().map_or(0, |g| g.ids.len());
        let h = match gi {
            Some(g) if count > 0 => self.host.handles.alloc(Object::GlyphInfo(Arc::new(g))),
            _ => 0,
        };
        self.mem[(p + 5) as usize].set_int(h);
        self.mem[(p + 4) as usize].set_qqqq_b3(count as i32);
    }

    /// `measure_native_node` (`set_native_metrics`).
    pub fn set_native_metrics(&mut self, p: i32, use_glyph_metrics: bool) {
        let f = self.native_font_of(p);
        let Some(engine) = self.font_engine(f) else {
            eprintln!("\n! Internal error: bad native font flag in `measure_native_node'");
            std::process::exit(3);
        };
        let text = self.node_text(p);
        if needs_bidi(&text) && !self.host.bidi_warned {
            self.host.bidi_warned = true;
            eprintln!(
                "flashtex-xetex: right-to-left or bidirectional text needs ICU's bidi \
                 analysis, which phase S1 does not have; it is laid out left to right"
            );
        }
        let _ = engine.default_rtl();
        let n = engine.layout_chars(&text, 0, text.len(), false);
        let glyphs = engine.glyphs();
        let advances = engine.glyph_advances();
        let positions = engine.glyph_positions();
        let mut width = 0f64;
        let mut info = GlyphInfo::default();
        let mut glyph_advances: Vec<i32> = vec![];
        if n > 0 {
            for i in 0..n {
                info.ids.push(glyphs[i] as u16);
                glyph_advances.push(d2fix(advances[i] as f64));
                info.locations
                    .push((d2fix(positions[i].0 as f64), d2fix(positions[i].1 as f64)));
            }
            width = positions[n].0 as f64;
        }
        self.mem[(p + 1) as usize].set_int(d2fix(width));

        let ls = self.font_letter_space[f as usize];
        if ls != 0 {
            let mut delta = 0i32;
            // (indexed as XeTeX's C loop is; two arrays move together)
            #[allow(clippy::needless_range_loop)]
            for i in 0..info.ids.len() {
                if glyph_advances[i] == 0 && delta != 0 {
                    delta -= ls;
                }
                info.locations[i].0 += delta;
                delta += ls;
            }
            if delta != 0 {
                delta -= ls;
                let w = self.mem[(p + 1) as usize].int();
                self.mem[(p + 1) as usize].set_int(w + delta);
            }
        }
        let count = info.ids.len();
        self.set_glyph_info(p, Some(info));

        if !use_glyph_metrics || count == 0 {
            // For efficiency, height and depth are the font's ascent and
            // descent unless glyph metrics are asked for.
            self.mem[(p + 3) as usize].set_int(self.height_base[f as usize]);
            self.mem[(p + 2) as usize].set_int(self.depth_base[f as usize]);
        } else {
            let gi = self.glyph_info_of(p).unwrap();
            let mut y_min = 65536.0f32;
            let mut y_max = -65536.0f32;
            for i in 0..gi.ids.len() {
                let y = fix2d(-gi.locations[i].1) as f32; // negative is upwards
                let bbox = cached_bbox(&engine, gi.ids[i]);
                let ht = bbox.y_max;
                let dp = -bbox.y_min;
                if y + ht > y_max {
                    y_max = y + ht;
                }
                if y - dp < y_min {
                    y_min = y - dp;
                }
            }
            self.mem[(p + 3) as usize].set_int(d2fix(y_max as f64));
            self.mem[(p + 2) as usize].set_int(-d2fix(y_min as f64));
        }
    }

    /// `store_justified_native_glyphs` (`set_justified_native_glyphs`).
    pub fn set_justified_native_glyphs(&mut self, p: i32) {
        let f = self.native_font_of(p);
        let saved_width = self.mem[(p + 1) as usize].int();
        self.set_native_metrics(p, false);
        let width = self.mem[(p + 1) as usize].int();
        if width == saved_width {
            return;
        }
        let just_amount = fix2d(saved_width - width);
        let Some(gi) = self.glyph_info_of(p) else {
            self.mem[(p + 1) as usize].set_int(saved_width);
            return;
        };
        let mut gi = (*gi).clone();
        let glyph_count = gi.ids.len();
        let space_glyph = self.map_char_to_glyph(f, b' ' as i32);
        let space_count = gi.ids.iter().filter(|&&g| g as i32 == space_glyph).count();
        if space_count > 0 {
            let mut adjustment = 0f64;
            let mut space_index = 0;
            for i in 0..glyph_count {
                gi.locations[i].0 = d2fix(fix2d(gi.locations[i].0) + adjustment);
                if gi.ids[i] as i32 == space_glyph {
                    space_index += 1;
                    adjustment = just_amount * space_index as f64 / space_count as f64;
                }
            }
        } else {
            for (i, loc) in gi.locations.iter_mut().enumerate().skip(1) {
                loc.0 = d2fix(fix2d(loc.0) + just_amount * i as f64 / (glyph_count - 1) as f64);
            }
        }
        // The array is shared with checkpoints: the changed copy replaces it.
        self.set_glyph_info(p, Some(gi));
        self.mem[(p + 1) as usize].set_int(saved_width);
    }

    /// `get_native_glyph`.
    pub fn get_native_glyph(&mut self, p: i32, i: i32) -> i32 {
        match self.glyph_info_of(p) {
            Some(g) if i >= 0 && (i as usize) < g.ids.len() => g.ids[i as usize] as i32,
            _ => 0,
        }
    }

    /// `measure_native_glyph` (`set_native_glyph_metrics`).
    pub fn set_native_glyph_metrics(&mut self, p: i32, use_glyph_metrics: bool) {
        let gid = self.mem[(p + 4) as usize].qqqq_b2() as u32;
        let f = self.native_font_of(p);
        let Some(engine) = self.font_engine(f) else {
            eprintln!("\n! Internal error: bad native font flag in `measure_native_glyph'");
            std::process::exit(3);
        };
        self.mem[(p + 1) as usize].set_int(d2fix(engine.font.glyph_width(gid) as f64));
        if use_glyph_metrics {
            let (ht, dp) = engine.font.glyph_height_depth(gid);
            self.mem[(p + 3) as usize].set_int(d2fix(ht as f64));
            self.mem[(p + 2) as usize].set_int(d2fix(dp as f64));
        } else {
            self.mem[(p + 3) as usize].set_int(self.height_base[f as usize]);
            self.mem[(p + 2) as usize].set_int(self.depth_base[f as usize]);
        }
    }

    /// `get_native_italic_correction`.
    pub fn get_native_italic_correction(&mut self, p: i32) -> i32 {
        let f = self.native_font_of(p);
        let Some(gi) = self.glyph_info_of(p) else {
            return 0;
        };
        match (gi.ids.last(), self.font_engine(f)) {
            (Some(&g), Some(e)) => {
                d2fix(e.glyph_ital_corr(g as u32) as f64) + self.font_letter_space[f as usize]
            }
            _ => 0,
        }
    }

    /// `get_native_glyph_italic_correction`.
    pub fn get_native_glyph_italic_correction(&mut self, p: i32) -> i32 {
        let gid = self.mem[(p + 4) as usize].qqqq_b2() as u32;
        let f = self.native_font_of(p);
        match self.font_engine(f) {
            Some(e) => d2fix(e.glyph_ital_corr(gid) as f64),
            None => 0,
        }
    }

    /// `getnativecharheightdepth`, with the snapping to the baseline,
    /// x-height and cap-height within 4% of the em.
    pub fn get_native_char_height_depth(&mut self, f: i32, c: i32, h: &mut i32, d: &mut i32) {
        let Some(e) = self.font_engine(f) else {
            eprintln!("\n! Internal error: bad native font flag in `get_native_char_height_depth`");
            std::process::exit(3);
        };
        let gid = e.font.map_char_to_glyph(c as u32);
        let (ht, dp) = e.font.glyph_height_depth(gid);
        *h = d2fix(ht as f64);
        *d = d2fix(dp as f64);
        let pb = self.param_base[f as usize];
        let quad = self.font_info[(6 + pb) as usize].int();
        let x_height = self.font_info[(5 + pb) as usize].int();
        let cap_height = self.font_info[(8 + pb) as usize].int();
        let fuzz = quad / 25;
        let snap = |v: &mut i32, to: i32| {
            let diff = v.wrapping_sub(to);
            if diff <= fuzz && diff >= -fuzz {
                *v = to;
            }
        };
        snap(d, 0);
        snap(h, 0);
        snap(h, x_height);
        snap(h, cap_height);
    }

    pub fn getnativecharht(&mut self, f: i32, c: i32) -> i32 {
        let (mut h, mut d) = (0, 0);
        self.get_native_char_height_depth(f, c, &mut h, &mut d);
        h
    }

    pub fn getnativechardp(&mut self, f: i32, c: i32) -> i32 {
        let (mut h, mut d) = (0, 0);
        self.get_native_char_height_depth(f, c, &mut h, &mut d);
        d
    }

    /// `getnativecharsidebearings`.
    pub fn get_native_char_sidebearings(&mut self, f: i32, c: i32, lsb: &mut i32, rsb: &mut i32) {
        let Some(e) = self.font_engine(f) else {
            eprintln!(
                "\n! Internal error: bad native font flag in `get_native_char_side_bearings'"
            );
            std::process::exit(3);
        };
        let gid = e.font.map_char_to_glyph(c as u32);
        let (l, r) = e.glyph_sidebearings(gid);
        *lsb = d2fix(l as f64);
        *rsb = d2fix(r as f64);
    }

    /// `getglyphbounds`: edge 1, 2, 3, 4 is left, top, right, bottom.
    pub fn get_glyph_bounds(&mut self, f: i32, edge: i32, gid: i32) -> i32 {
        let Some(e) = self.font_engine(f) else {
            eprintln!("\n! Internal error: bad native font flag in `get_glyph_bounds'");
            std::process::exit(3);
        };
        let (a, b) = if edge & 1 != 0 {
            e.glyph_sidebearings(gid as u32)
        } else {
            e.font.glyph_height_depth(gid as u32)
        };
        d2fix(if edge <= 2 { a } else { b } as f64)
    }

    /// `getnativecharic`.
    pub fn getnativecharic(&mut self, f: i32, c: i32) -> i32 {
        let (mut lsb, mut rsb) = (0, 0);
        self.get_native_char_sidebearings(f, c, &mut lsb, &mut rsb);
        if rsb < 0 {
            self.font_letter_space[f as usize] - rsb
        } else {
            self.font_letter_space[f as usize]
        }
    }

    /// `getnativecharwd`.
    pub fn getnativecharwd(&mut self, f: i32, c: i32) -> i32 {
        let Some(e) = self.font_engine(f) else {
            eprintln!("\n! Internal error: bad native font flag in `get_native_char_wd'");
            std::process::exit(3);
        };
        let gid = e.font.map_char_to_glyph(c as u32);
        d2fix(e.glyph_width(gid) as f64)
    }

    /// `mapchartoglyph`.
    pub fn map_char_to_glyph(&mut self, f: i32, c: i32) -> i32 {
        if !(0..=0x10FFFF).contains(&c) || (0xD800..=0xDFFF).contains(&c) {
            return 0;
        }
        match self.font_engine(f) {
            Some(e) => e.font.map_char_to_glyph(c as u32) as i32,
            None => {
                eprintln!("\n! Internal error: bad native font flag in `map_char_to_glyph'");
                std::process::exit(3);
            }
        }
    }

    /// `mapglyphtoindex`: the glyph named by `name_of_file`.
    pub fn map_glyph_to_index(&mut self, f: i32) -> i32 {
        let n = (self.name_length.max(0) as usize).min(self.name_of_file.len());
        let name: Vec<u8> = self.name_of_file[..n]
            .iter()
            .copied()
            .take_while(|&b| b != 0)
            .collect();
        match self.font_engine(f) {
            Some(e) => e.font.map_glyph_to_index(&name) as i32,
            None => {
                eprintln!("\n! Internal error: bad native font flag in `map_glyph_to_index'");
                std::process::exit(3);
            }
        }
    }

    /// `getfontcharrange`.
    pub fn get_font_char_range(&mut self, f: i32, first: bool) -> i32 {
        match self.font_engine(f) {
            Some(e) if first => e.font.first_char_code(),
            Some(e) => e.font.last_char_code(),
            None => {
                eprintln!("\n! Internal error: bad native font flag in `get_font_char_range'");
                std::process::exit(3);
            }
        }
    }

    /// `printglyphname`.
    pub fn print_glyph_name(&mut self, f: i32, gid: i32) {
        let Some(e) = self.font_engine(f) else {
            eprintln!("\n! Internal error: bad native font flag in `print_glyph_name'");
            std::process::exit(3);
        };
        let name = e.font.glyph_name(gid as u16 as u32);
        for b in name {
            self.print_char(b as i32);
        }
    }

    /// `get_native_word_cp`: the protrusion code of the node's first or
    /// last glyph.
    pub fn get_native_word_cp(&mut self, p: i32, side: i32) -> i32 {
        let f = self.native_font_of(p);
        let Some(gi) = self.glyph_info_of(p) else {
            return 0;
        };
        let g = if side == 0 {
            gi.ids.first()
        } else {
            gi.ids.last()
        };
        match g {
            Some(&g) => self.get_cp_code(f, g as i32, side),
            None => 0,
        }
    }

    #[allow(non_snake_case)]
    pub fn usingOpenType(&mut self, engine: i32) -> bool {
        self.layout_engine(engine)
            .is_some_and(|e| e.using_open_type())
    }

    #[allow(non_snake_case)]
    pub fn usingGraphite(&mut self, engine: i32) -> bool {
        self.layout_engine(engine)
            .is_some_and(|e| e.using_graphite())
    }

    #[allow(non_snake_case)]
    pub fn isOpenTypeMathFont(&mut self, engine: i32) -> bool {
        self.layout_engine(engine)
            .is_some_and(|e| e.is_open_type_math_font())
    }

    /// `otfontget`.
    pub fn ot_font_get(&mut self, what: i32, engine: i32) -> i32 {
        let Some(e) = self.layout_engine(engine) else {
            return 0;
        };
        match what {
            1 => e.font.num_glyphs() as i32,
            8 => 0, // Graphite features (S2)
            16 => layout::count_scripts(&e.font) as i32,
            _ => 0,
        }
    }

    /// `otfontget1`.
    pub fn ot_font_get_1(&mut self, what: i32, engine: i32, p: i32) -> i32 {
        let Some(e) = self.layout_engine(engine) else {
            return 0;
        };
        match what {
            17 => layout::count_languages(&e.font, p as u32) as i32,
            19 => layout::ind_script(&e.font, p as u32) as i32,
            11 => 1,
            _ => 0, // Graphite features (S2)
        }
    }

    /// `otfontget2`.
    pub fn ot_font_get_2(&mut self, what: i32, engine: i32, p1: i32, p2: i32) -> i32 {
        let Some(e) = self.layout_engine(engine) else {
            return 0;
        };
        match what {
            20 => layout::ind_language(&e.font, p1 as u32, p2 as u32) as i32,
            18 => layout::count_features(&e.font, p1 as u32, p2 as u32) as i32,
            _ => 0, // Graphite features (S2)
        }
    }

    /// `otfontget3`.
    pub fn ot_font_get_3(&mut self, what: i32, engine: i32, p1: i32, p2: i32, p3: i32) -> i32 {
        let Some(e) = self.layout_engine(engine) else {
            return 0;
        };
        match what {
            21 => layout::ind_feature(&e.font, p1 as u32, p2 as u32, p3 as u32) as i32,
            _ => 0,
        }
    }

    // ---- the XDV file ---------------------------------------------------

    /// `makeXDVGlyphArrayData`: the node's width, glyph count, positions
    /// and ids, big-endian, into `xdv_buffer`; the length.
    pub fn make_xdv_glyph_array_data(&mut self, p: i32) -> i32 {
        let width = self.mem[(p + 1) as usize].int();
        let gi = self.glyph_info_of(p).unwrap_or_default();
        let count = self.mem[(p + 4) as usize].qqqq_b3() as usize;
        let mut out = Vec::with_capacity(6 + count * 10);
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&(count as u16).to_be_bytes());
        for i in 0..count {
            let (x, y) = gi.locations.get(i).copied().unwrap_or((0, 0));
            out.extend_from_slice(&x.to_be_bytes());
            out.extend_from_slice(&y.to_be_bytes());
        }
        for i in 0..count {
            out.extend_from_slice(&gi.ids.get(i).copied().unwrap_or(0).to_be_bytes());
        }
        self.write_xdv_buffer(&out)
    }

    fn write_xdv_buffer(&mut self, bytes: &[u8]) -> i32 {
        for (k, &b) in bytes.iter().enumerate() {
            self.xdv_buffer[k] = b as i32;
        }
        bytes.len() as i32
    }

    /// `makefontdef`: a native font's definition for the XDV file
    /// (`define_native_font` after the font number) into `xdv_buffer`;
    /// the length.
    pub fn make_font_def(&mut self, f: i32) -> i32 {
        let Some(e) = self.font_engine(f) else {
            eprintln!("\n! Internal error: bad native font flag in `make_font_def'");
            std::process::exit(3);
        };
        let mut flags: u16 = 0;
        let font_flags = self.font_flags[f as usize];
        if font_flags & FONT_FLAGS_VERTICAL != 0 {
            flags |= XDV_FLAG_VERTICAL;
        }
        let size = d2fix(e.font.point_size as f64);
        let filename = &e.font.filename;
        let filename_len = filename.len() as u8;
        if font_flags & FONT_FLAGS_COLORED != 0 {
            flags |= XDV_FLAG_COLORED;
        }
        if e.extend != 1.0 {
            flags |= XDV_FLAG_EXTEND;
        }
        if e.slant != 0.0 {
            flags |= XDV_FLAG_SLANT;
        }
        if e.embolden != 0.0 {
            flags |= XDV_FLAG_EMBOLDEN;
        }
        let mut out = vec![];
        out.extend_from_slice(&size.to_be_bytes());
        out.extend_from_slice(&flags.to_be_bytes());
        out.push(filename_len);
        out.extend_from_slice(&filename[..filename_len as usize]);
        out.extend_from_slice(&e.font.index.to_be_bytes());
        if font_flags & FONT_FLAGS_COLORED != 0 {
            out.extend_from_slice(&e.rgb_value.to_be_bytes());
        }
        if flags & XDV_FLAG_EXTEND != 0 {
            out.extend_from_slice(&d2fix(e.extend as f64).to_be_bytes());
        }
        if flags & XDV_FLAG_SLANT != 0 {
            out.extend_from_slice(&d2fix(e.slant as f64).to_be_bytes());
        }
        if flags & XDV_FLAG_EMBOLDEN != 0 {
            out.extend_from_slice(&d2fix(e.embolden as f64).to_be_bytes());
        }
        self.write_xdv_buffer(&out)
    }

    /// `terminatefontmanager`.
    pub fn terminate_font_manager(&mut self) {}

    /// `get_ot_math_constant` would be the MATH table's constant (phase
    /// S2); an OpenType math font's extra fontdimens are 0 until then.
    pub fn get_ot_math_constant(&mut self, _f: i32, _n: i32) -> i32 {
        0
    }
}

/// `getCachedGlyphBBox`/`cacheGlyphBBox` around `getGlyphBounds`.
fn cached_bbox(engine: &LayoutEngine, gid: u16) -> GlyphBBox {
    if let Some(b) = engine.bbox_cache.lock().unwrap().get(&gid) {
        return *b;
    }
    let b = engine.glyph_bounds(gid as u32);
    engine.bbox_cache.lock().unwrap().insert(gid, b);
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_font_name_cases() {
        fn parts(s: &str) -> (&str, &str, &str, u32) {
            let b = s.as_bytes();
            let (v, f, e, i) = split_font_name(b);
            (&s[..v], &s[v..f], &s[f..e], i)
        }
        assert_eq!(
            parts("Latin Modern Roman"),
            ("Latin Modern Roman", "", "", 0)
        );
        assert_eq!(parts("Helvetica/B:+smcp"), ("Helvetica", "/B", ":+smcp", 0));
        assert_eq!(parts("Helvetica:a/b"), ("Helvetica", "", ":a/b", 0));
        assert_eq!(
            parts("[lmroman10-regular.otf]:mapping=tex-text"),
            ("[lmroman10-regular.otf", "]", ":mapping=tex-text", 0)
        );
        assert_eq!(parts("[foo.ttc:2]/OT"), ("[foo.ttc", ":2]/OT", "", 2));
    }

    #[test]
    fn rgb_and_doubles() {
        assert_eq!(read_rgb_a(b"FF0000"), (0xFF0000FF, 6));
        assert_eq!(read_rgb_a(b"00FF0080"), (0x00FF0080, 8));
        assert_eq!(read_rgb_a(b"12"), (0xFF, 2));
        assert_eq!(read_double(b" -1.25x", 0), (-1.25, 6));
    }

    #[test]
    fn bidi_detection() {
        let u = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
        assert!(!needs_bidi(&u("office — naïve 123")));
        assert!(needs_bidi(&u("abc \u{05D0}")));
        assert!(needs_bidi(&u("\u{202E}x")));
    }
}
