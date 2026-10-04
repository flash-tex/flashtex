//! `XeTeXLayoutInterface.cpp`: a layout engine (a font with its script,
//! language, features and shapers), shaping through HarfBuzz, and the
//! OpenType layout queries of `\XeTeXOT...`. A port of TeX Live 2026's
//! file (MIT, third_party/xetex/COPYING). Graphite (phase S2) and AAT
//! (out of scope, PLAN.md §3.1) are left out.

use super::font_inst::{FontInst, GlyphBBox};
use crate::fontlibs::hb;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_uint, CStr, CString};

/// `XeTeXLayoutEngine_rec`.
pub struct LayoutEngine {
    pub font: FontInst,
    pub script: hb::hb_tag_t,
    pub language: hb::hb_language_t,
    pub features: Vec<hb::hb_feature_t>,
    /// The requested shapers (`ShaperList`); `None` asks for `ot` only.
    pub shaper_list: Option<Vec<CString>>,
    /// The shaper HarfBuzz used last (`shaper`), none before the first
    /// shaping.
    pub shaper: RefCell<Option<String>>,
    pub rgb_value: u32,
    pub extend: f32,
    pub slant: f32,
    pub embolden: f32,
    buffer: *mut hb::hb_buffer_t,
    /// XeTeXLayoutInterface.cpp's `sGlyphBoxes` for this font: glyph
    /// bounding boxes in points, for `\XeTeXuseglyphmetrics`. XeTeX keys
    /// one process-wide map by font number; a map per engine is the same
    /// cache, and stays right when a restored engine loads another font
    /// under the same number.
    pub bbox_cache: RefCell<HashMap<u16, GlyphBBox>>,
    /// The `\XeTeXfonttype` letter XeTeX's font manager had when the
    /// engine was made (`getReqEngine`): 'G' makes the language a BCP 47
    /// tag.
    pub req_engine: u8,
    /// The direction of the last shaped run's script (for
    /// `getDefaultDirection`, which reads the reused buffer).
    pub last_script: Cell<hb::hb_script_t>,
}

impl Drop for LayoutEngine {
    fn drop(&mut self) {
        // SAFETY: the buffer was created by `new` and is destroyed once.
        unsafe { hb::hb_buffer_destroy(self.buffer) };
    }
}

impl LayoutEngine {
    /// `createLayoutEngine`. `language` is the `language=` option, if any.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        font: FontInst,
        script: hb::hb_tag_t,
        language: Option<&[u8]>,
        features: Vec<hb::hb_feature_t>,
        shaper_list: Option<Vec<CString>>,
        rgb_value: u32,
        extend: f32,
        slant: f32,
        embolden: f32,
        req_engine: u8,
    ) -> LayoutEngine {
        let lang_c = language.map(|l| CString::new(l.to_vec()).unwrap_or_default());
        let lang_ptr = lang_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        // SAFETY: HarfBuzz reads the NUL-terminated string (or NULL).
        let language = unsafe {
            if req_engine == b'G' {
                hb::hb_language_from_string(lang_ptr, -1)
            } else {
                hb::hb_ot_tag_to_language(hb::hb_tag_from_string(lang_ptr, -1))
            }
        };
        LayoutEngine {
            font,
            script,
            language,
            features,
            shaper_list,
            shaper: RefCell::new(None),
            rgb_value,
            extend,
            slant,
            embolden,
            // SAFETY: a new buffer, destroyed in Drop.
            buffer: unsafe { hb::hb_buffer_create() },
            bbox_cache: RefCell::new(HashMap::new()),
            req_engine,
            last_script: Cell::new(hb::HB_SCRIPT_INVALID),
        }
    }

    /// `layoutChars`: shape `chars[offset..offset+count]` (with the whole
    /// of `chars` as context); the number of glyphs.
    pub fn layout_chars(&self, chars: &[u16], offset: usize, count: usize, rtl: bool) -> usize {
        let font = &self.font;
        let hb_font = font.hb_font;
        let mut direction = hb::HB_DIRECTION_LTR;
        if font.vertical.get() {
            direction = hb::HB_DIRECTION_TTB;
        } else if rtl {
            direction = hb::HB_DIRECTION_RTL;
        }
        // SAFETY: the buffer, font and face are live; the text and the
        // feature and shaper arrays outlive the calls.
        unsafe {
            let hb_face = hb::hb_font_get_face(hb_font);
            let script = hb::hb_ot_tag_to_script(self.script);
            let buf = self.buffer;
            hb::hb_buffer_reset(buf);
            hb::hb_buffer_add_utf16(
                buf,
                chars.as_ptr(),
                chars.len() as i32,
                offset as c_uint,
                count as i32,
            );
            hb::hb_buffer_set_direction(buf, direction);
            hb::hb_buffer_set_script(buf, script);
            hb::hb_buffer_set_language(buf, self.language);
            hb::hb_buffer_guess_segment_properties(buf);
            let mut props = std::mem::zeroed::<hb::hb_segment_properties_t>();
            hb::hb_buffer_get_segment_properties(buf, &mut props);

            // HarfBuzz prefers graphite2 for hybrid fonts; XeTeX prefers
            // OpenType, and since "ot" never fails, asks for it alone.
            let ot = c"ot";
            let mut list: Vec<*const c_char> = match &self.shaper_list {
                Some(v) => v.iter().map(|s| s.as_ptr()).collect(),
                None => vec![ot.as_ptr()],
            };
            list.push(std::ptr::null());

            let feats = self.features.as_ptr();
            let nfeats = self.features.len() as c_uint;
            let mut plan =
                hb::hb_shape_plan_create_cached(hb_face, &props, feats, nfeats, list.as_ptr());
            let mut res = hb::hb_shape_plan_execute(plan, hb_font, buf, feats, nfeats) != 0;
            if !res {
                // All selected shapers failed: retry with the default
                // (not cached: the cached plan would fail again).
                hb::hb_shape_plan_destroy(plan);
                plan = hb::hb_shape_plan_create(hb_face, &props, feats, nfeats, std::ptr::null());
                res = hb::hb_shape_plan_execute(plan, hb_font, buf, feats, nfeats) != 0;
                if !res {
                    eprintln!("\nERROR: all shapers failed");
                    std::process::exit(3);
                }
            }
            let shaper = CStr::from_ptr(hb::hb_shape_plan_get_shaper(plan))
                .to_string_lossy()
                .into_owned();
            *self.shaper.borrow_mut() = Some(shaper);
            hb::hb_buffer_set_content_type(buf, hb::HB_BUFFER_CONTENT_TYPE_GLYPHS);
            hb::hb_shape_plan_destroy(plan);
            self.last_script.set(hb::hb_buffer_get_script(buf));
            hb::hb_buffer_get_length(buf) as usize
        }
    }

    fn infos(&self) -> &[hb::hb_glyph_info_t] {
        // SAFETY: the buffer's arrays stay valid until it changes.
        unsafe {
            let mut n: c_uint = 0;
            let p = hb::hb_buffer_get_glyph_infos(self.buffer, &mut n);
            if p.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts(p, n as usize)
            }
        }
    }

    fn positions(&self) -> &[hb::hb_glyph_position_t] {
        // SAFETY: as `infos`.
        unsafe {
            let mut n: c_uint = 0;
            let p = hb::hb_buffer_get_glyph_positions(self.buffer, &mut n);
            if p.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts(p, n as usize)
            }
        }
    }

    /// `getGlyphs`.
    pub fn glyphs(&self) -> Vec<u32> {
        self.infos().iter().map(|g| g.codepoint).collect()
    }

    /// `getGlyphAdvances`.
    pub fn glyph_advances(&self) -> Vec<f32> {
        let v = self.font.vertical.get();
        self.positions()
            .iter()
            .map(|p| {
                self.font
                    .units_to_points(if v { p.y_advance } else { p.x_advance } as f32)
            })
            .collect()
    }

    /// `getGlyphPositions`: one more position than glyphs (the end).
    pub fn glyph_positions(&self) -> Vec<(f32, f32)> {
        let pos = self.positions();
        let f = &self.font;
        let mut out = Vec::with_capacity(pos.len() + 1);
        let (mut x, mut y) = (0f32, 0f32);
        if f.vertical.get() {
            for p in pos {
                out.push((
                    -f.units_to_points(x + p.y_offset as f32), // negative is forwards
                    f.units_to_points(y - p.x_offset as f32),
                ));
                x += p.y_advance as f32;
                y += p.x_advance as f32;
            }
            out.push((-f.units_to_points(x), f.units_to_points(y)));
        } else {
            for p in pos {
                out.push((
                    f.units_to_points(x + p.x_offset as f32),
                    -f.units_to_points(y + p.y_offset as f32), // negative is upwards
                ));
                x += p.x_advance as f32;
                y += p.y_advance as f32;
            }
            out.push((f.units_to_points(x), -f.units_to_points(y)));
        }
        if self.extend != 1.0 || self.slant != 0.0 {
            for p in out.iter_mut() {
                p.0 = p.0 * self.extend - p.1 * self.slant;
            }
        }
        out
    }

    /// `getDefaultDirection`: whether the script of the buffer (of the
    /// last shaping) is written right to left.
    pub fn default_rtl(&self) -> bool {
        // SAFETY: a pure function of the script.
        unsafe {
            hb::hb_script_get_horizontal_direction(hb::hb_buffer_get_script(self.buffer))
                == hb::HB_DIRECTION_RTL
        }
    }

    /// `getGlyphBounds` (with the `extend` factor).
    pub fn glyph_bounds(&self, gid: u32) -> GlyphBBox {
        let mut b = self.font.glyph_bounds(gid);
        if self.extend != 0.0 {
            b.x_min *= self.extend;
            b.x_max *= self.extend;
        }
        b
    }

    /// `getGlyphWidthFromEngine`.
    pub fn glyph_width(&self, gid: u32) -> f32 {
        self.extend * self.font.glyph_width(gid)
    }

    /// `getGlyphSidebearings` (with the `extend` factor).
    pub fn glyph_sidebearings(&self, gid: u32) -> (f32, f32) {
        let (mut l, mut r) = self.font.glyph_sidebearings(gid);
        if self.extend != 0.0 {
            l *= self.extend;
            r *= self.extend;
        }
        (l, r)
    }

    /// `getGlyphItalCorr`.
    pub fn glyph_ital_corr(&self, gid: u32) -> f32 {
        self.extend * self.font.glyph_ital_corr(gid)
    }

    /// `usingOpenType`.
    pub fn using_open_type(&self) -> bool {
        self.shaper.borrow().as_deref().is_none_or(|s| s == "ot")
    }

    /// `usingGraphite`.
    pub fn using_graphite(&self) -> bool {
        self.shaper.borrow().as_deref() == Some("graphite2")
    }

    /// `isOpenTypeMathFont`.
    pub fn is_open_type_math_font(&self) -> bool {
        // SAFETY: a live face.
        unsafe { hb::hb_ot_math_has_data(self.font.hb_face()) != 0 }
    }
}

// ---- OpenType layout queries (XeTeXLayoutInterface.cpp) -----------------

fn tags(f: impl Fn(*mut c_uint, *mut hb::hb_tag_t) -> c_uint) -> Vec<hb::hb_tag_t> {
    let n = f(std::ptr::null_mut(), std::ptr::null_mut());
    let mut v = vec![0; n as usize];
    let mut count = n;
    f(&mut count, v.as_mut_ptr());
    v.truncate(count as usize);
    v
}

/// `getLargerScriptListTable`. XeTeX reads GSUB's list twice (its second
/// call names GSUB where GPOS is meant); so does this port.
fn larger_script_list(font: &FontInst) -> Vec<hb::hb_tag_t> {
    let face = font.hb_face();
    // SAFETY: a live face and arrays of the counts HarfBuzz reported.
    let sub = tags(|c, t| unsafe {
        hb::hb_ot_layout_table_get_script_tags(face, hb::HB_OT_TAG_GSUB, 0, c, t)
    });
    let count_pos = unsafe {
        hb::hb_ot_layout_table_get_script_tags(
            face,
            hb::HB_OT_TAG_GPOS,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    let mut pos = vec![0; count_pos as usize];
    let mut n = count_pos;
    unsafe {
        hb::hb_ot_layout_table_get_script_tags(
            face,
            hb::HB_OT_TAG_GSUB,
            0,
            &mut n,
            pos.as_mut_ptr(),
        )
    };
    // The second call wrote GSUB's first `n` tags and set `n` to how many
    // it wrote; C compares and returns that `n`.
    pos.truncate(n as usize);
    if sub.len() as c_uint > n {
        sub
    } else {
        pos
    }
}

/// `countScripts`.
pub fn count_scripts(font: &FontInst) -> u32 {
    larger_script_list(font).len() as u32
}

/// `getIndScript`.
pub fn ind_script(font: &FontInst, index: u32) -> u32 {
    larger_script_list(font)
        .get(index as usize)
        .copied()
        .unwrap_or(0)
}

/// `countLanguages`.
pub fn count_languages(font: &FontInst, script: u32) -> u32 {
    let face = font.hb_face();
    let list = larger_script_list(font);
    for (i, &s) in list.iter().enumerate() {
        if s == script {
            // SAFETY: counting calls on a live face.
            unsafe {
                let n = std::ptr::null_mut();
                return hb::hb_ot_layout_script_get_language_tags(
                    face,
                    hb::HB_OT_TAG_GSUB,
                    i as _,
                    0,
                    n,
                    n,
                ) + hb::hb_ot_layout_script_get_language_tags(
                    face,
                    hb::HB_OT_TAG_GPOS,
                    i as _,
                    0,
                    n,
                    n,
                );
            }
        }
    }
    0
}

/// `getIndLanguage`.
pub fn ind_language(font: &FontInst, script: u32, index: u32) -> u32 {
    let face = font.hb_face();
    let list = larger_script_list(font);
    for (i, &s) in list.iter().enumerate() {
        if s == script {
            for table in [hb::HB_OT_TAG_GSUB, hb::HB_OT_TAG_GPOS] {
                // SAFETY: a live face and arrays of the reported counts.
                let langs = tags(|c, t| unsafe {
                    hb::hb_ot_layout_script_get_language_tags(face, table, i as _, 0, c, t)
                });
                if (index as usize) < langs.len() {
                    return langs[index as usize];
                }
            }
        }
    }
    0
}

fn for_each_language_system(
    font: &FontInst,
    script: u32,
    language: u32,
    mut f: impl FnMut(hb::hb_tag_t, c_uint, c_uint) -> bool,
) {
    let face = font.hb_face();
    for table in [hb::HB_OT_TAG_GSUB, hb::HB_OT_TAG_GPOS] {
        let mut script_index: c_uint = 0;
        let mut lang_index: c_uint = 0;
        // SAFETY: lookups on a live face.
        unsafe {
            if hb::hb_ot_layout_table_find_script(face, table, script, &mut script_index) != 0
                && (hb::hb_ot_layout_script_find_language(
                    face,
                    table,
                    script_index,
                    language,
                    &mut lang_index,
                ) != 0
                    || language == 0)
                && f(table, script_index, lang_index)
            {
                return;
            }
        }
    }
}

/// `countFeatures`.
pub fn count_features(font: &FontInst, script: u32, language: u32) -> u32 {
    let face = font.hb_face();
    let mut rval = 0;
    for_each_language_system(font, script, language, |table, si, li| {
        // SAFETY: a counting call on a live face.
        rval += unsafe {
            hb::hb_ot_layout_language_get_feature_tags(
                face,
                table,
                si,
                li,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        false
    });
    rval
}

/// `getIndFeature`.
pub fn ind_feature(font: &FontInst, script: u32, language: u32, mut index: u32) -> u32 {
    let face = font.hb_face();
    let mut rval = 0;
    for_each_language_system(font, script, language, |table, si, li| {
        // SAFETY: a live face and an array of the reported count.
        let feats = tags(|c, t| unsafe {
            hb::hb_ot_layout_language_get_feature_tags(face, table, si, li, 0, c, t)
        });
        if (index as usize) < feats.len() {
            rval = feats[index as usize];
            return true;
        }
        index -= feats.len() as u32;
        false
    });
    rval
}
