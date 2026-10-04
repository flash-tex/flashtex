//! `XeTeXFontInst.cpp`: a font loaded through FreeType, with the HarfBuzz
//! font XeTeX shapes with. XeTeX's code is MIT-licensed
//! (third_party/xetex/COPYING); this is a port of TeX Live 2026's file,
//! routine by routine, with its arithmetic kept in C's types (`float`
//! metrics in points, `double` where C promotes).

use crate::fontlibs::{ft, hb};
use std::ffi::{c_char, c_uint, c_void, CString};
use std::rc::Rc;

/// The FreeType library (XeTeXFontInst.cpp's `gFreeTypeLibrary`), one per
/// engine (`Host`), shared by the fonts loaded through it.
pub struct FtLibrary(ft::FT_Library);

impl FtLibrary {
    /// `FT_Init_FreeType`. XeTeX exits when this fails; so does the port.
    pub fn new() -> Rc<FtLibrary> {
        let mut lib: ft::FT_Library = std::ptr::null_mut();
        // SAFETY: FT_Init_FreeType writes a new library handle.
        let error = unsafe { ft::FT_Init_FreeType(&mut lib) };
        if error != 0 {
            eprintln!("FreeType initialization failed! ({error})");
            std::process::exit(1);
        }
        Rc::new(FtLibrary(lib))
    }
}

impl Drop for FtLibrary {
    fn drop(&mut self) {
        // SAFETY: every face of this library holds an `Rc` to it, so none is
        // left when it drops.
        unsafe {
            ft::FT_Done_FreeType(self.0);
        }
    }
}

/// `GlyphBBox` (XeTeXLayoutInterface.h): in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlyphBBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

/// `XeTeXFontInst`.
pub struct FontInst {
    pub units_per_em: u16,
    pub point_size: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cap_height: f32,
    pub x_height: f32,
    pub italic_angle: f32,
    pub vertical: std::cell::Cell<bool>,
    /// The font file's path as opened (`m_filename`), and the face index.
    pub filename: Vec<u8>,
    pub index: u32,
    pub ft_face: ft::FT_Face,
    pub hb_font: *mut hb::hb_font_t,
    _lib: Rc<FtLibrary>,
}

impl Drop for FontInst {
    fn drop(&mut self) {
        // SAFETY: the face and font were created by `new` and are dropped
        // once; the HarfBuzz font's callbacks use the face, so it goes first.
        unsafe {
            hb::hb_font_destroy(self.hb_font);
            if !self.ft_face.is_null() {
                ft::FT_Done_Face(self.ft_face);
            }
        }
    }
}

// ---- HarfBuzz font functions (XeTeXFontInst.cpp) ------------------------

unsafe extern "C" fn get_glyph(
    _font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    ch: hb::hb_codepoint_t,
    vs: hb::hb_codepoint_t,
    gid: *mut hb::hb_codepoint_t,
    _user: *mut c_void,
) -> hb::hb_bool_t {
    let face = font_data as ft::FT_Face;
    *gid = 0;
    if vs != 0 {
        *gid = ft::FT_Face_GetCharVariantIndex(face, ch as _, vs as _) as _;
    }
    if *gid == 0 {
        *gid = ft::FT_Get_Char_Index(face, ch as _) as _;
    }
    (*gid != 0) as hb::hb_bool_t
}

/// `_get_glyph_advance`: in font units.
pub(crate) fn glyph_advance(face: ft::FT_Face, gid: u32, vertical: bool) -> ft::FT_Fixed {
    let mut flags = ft::FT_LOAD_NO_SCALE;
    if vertical {
        flags |= ft::FT_LOAD_VERTICAL_LAYOUT;
    }
    let mut advance: ft::FT_Fixed = 0;
    // SAFETY: `face` is a live face.
    let error = unsafe { ft::FT_Get_Advance(face, gid as _, flags, &mut advance) };
    if error != 0 {
        advance = 0;
    }
    // FreeType's vertical metrics grows downward.
    if vertical {
        advance = -advance;
    }
    advance
}

unsafe extern "C" fn get_glyph_h_advance(
    _font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    gid: hb::hb_codepoint_t,
    _user: *mut c_void,
) -> hb::hb_position_t {
    glyph_advance(font_data as ft::FT_Face, gid, false) as hb::hb_position_t
}

unsafe extern "C" fn get_glyph_v_advance(
    _font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    gid: hb::hb_codepoint_t,
    _user: *mut c_void,
) -> hb::hb_position_t {
    glyph_advance(font_data as ft::FT_Face, gid, true) as hb::hb_position_t
}

unsafe extern "C" fn get_glyph_origin(
    _font: *mut hb::hb_font_t,
    _font_data: *mut c_void,
    _gid: hb::hb_codepoint_t,
    _x: *mut hb::hb_position_t,
    _y: *mut hb::hb_position_t,
    _user: *mut c_void,
) -> hb::hb_bool_t {
    // Horizontal origin is (0, 0); vertical origin is (0, 0) "for now", as
    // in XeTeX (pre-0.9999 compatibility).
    1
}

unsafe extern "C" fn get_glyph_h_kerning(
    _font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    gid1: hb::hb_codepoint_t,
    gid2: hb::hb_codepoint_t,
    _user: *mut c_void,
) -> hb::hb_position_t {
    let face = font_data as ft::FT_Face;
    let mut kerning = ft::FT_Vector { x: 0, y: 0 };
    let error = ft::FT_Get_Kerning(face, gid1, gid2, ft::FT_KERNING_UNSCALED, &mut kerning);
    if error != 0 {
        0
    } else {
        kerning.x as hb::hb_position_t
    }
}

unsafe extern "C" fn get_glyph_v_kerning(
    _font: *mut hb::hb_font_t,
    _font_data: *mut c_void,
    _gid1: hb::hb_codepoint_t,
    _gid2: hb::hb_codepoint_t,
    _user: *mut c_void,
) -> hb::hb_position_t {
    // FreeType does not support vertical kerning.
    0
}

unsafe extern "C" fn get_glyph_extents(
    _font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    gid: hb::hb_codepoint_t,
    extents: *mut hb::hb_glyph_extents_t,
    _user: *mut c_void,
) -> hb::hb_bool_t {
    let face = font_data as ft::FT_Face;
    let error = ft::FT_Load_Glyph(face, gid, ft::FT_LOAD_NO_SCALE);
    if error == 0 {
        let m = &(*(*face).glyph).metrics;
        (*extents).x_bearing = m.horiBearingX as hb::hb_position_t;
        (*extents).y_bearing = m.horiBearingY as hb::hb_position_t;
        (*extents).width = m.width as hb::hb_position_t;
        (*extents).height = -(m.height as hb::hb_position_t);
    }
    (error == 0) as hb::hb_bool_t
}

unsafe extern "C" fn get_glyph_contour_point(
    _font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    gid: hb::hb_codepoint_t,
    point_index: c_uint,
    x: *mut hb::hb_position_t,
    y: *mut hb::hb_position_t,
    _user: *mut c_void,
) -> hb::hb_bool_t {
    let face = font_data as ft::FT_Face;
    let mut ret = false;
    let error = ft::FT_Load_Glyph(face, gid, ft::FT_LOAD_NO_SCALE);
    if error == 0 {
        let slot = &*(*face).glyph;
        if slot.format == ft::FT_GLYPH_FORMAT_OUTLINE
            && point_index < slot.outline.n_points as c_uint
        {
            let p = *slot.outline.points.add(point_index as usize);
            *x = p.x as hb::hb_position_t;
            *y = p.y as hb::hb_position_t;
            ret = true;
        }
    }
    ret as hb::hb_bool_t
}

unsafe extern "C" fn get_glyph_name(
    _font: *mut hb::hb_font_t,
    font_data: *mut c_void,
    gid: hb::hb_codepoint_t,
    name: *mut c_char,
    size: c_uint,
    _user: *mut c_void,
) -> hb::hb_bool_t {
    let face = font_data as ft::FT_Face;
    let mut ret = ft::FT_Get_Glyph_Name(face, gid, name as *mut c_void, size) == 0;
    if ret && size != 0 && *name == 0 {
        ret = false;
    }
    ret as hb::hb_bool_t
}

/// `_get_font_funcs`: XeTeX makes them once per process; here once per
/// font, which is the same set of functions.
fn font_funcs() -> *mut hb::hb_font_funcs_t {
    // SAFETY: a new funcs object with C function pointers of the right
    // signatures.
    unsafe {
        let f = hb::hb_font_funcs_create();
        let n = std::ptr::null_mut();
        hb::hb_font_funcs_set_glyph_func(f, Some(get_glyph), n, None);
        hb::hb_font_funcs_set_glyph_h_advance_func(f, Some(get_glyph_h_advance), n, None);
        hb::hb_font_funcs_set_glyph_v_advance_func(f, Some(get_glyph_v_advance), n, None);
        hb::hb_font_funcs_set_glyph_h_origin_func(f, Some(get_glyph_origin), n, None);
        hb::hb_font_funcs_set_glyph_v_origin_func(f, Some(get_glyph_origin), n, None);
        hb::hb_font_funcs_set_glyph_h_kerning_func(f, Some(get_glyph_h_kerning), n, None);
        hb::hb_font_funcs_set_glyph_v_kerning_func(f, Some(get_glyph_v_kerning), n, None);
        hb::hb_font_funcs_set_glyph_extents_func(f, Some(get_glyph_extents), n, None);
        hb::hb_font_funcs_set_glyph_contour_point_func(f, Some(get_glyph_contour_point), n, None);
        hb::hb_font_funcs_set_glyph_name_func(f, Some(get_glyph_name), n, None);
        f
    }
}

unsafe extern "C" fn free_table(p: *mut c_void) {
    drop(Box::from_raw(p as *mut Vec<u8>));
}

/// `_get_table`: a copy of the face's table `tag`, writable, as XeTeX
/// gives it to HarfBuzz.
unsafe extern "C" fn get_table(
    _face: *mut hb::hb_face_t,
    tag: hb::hb_tag_t,
    user_data: *mut c_void,
) -> *mut hb::hb_blob_t {
    let face = user_data as ft::FT_Face;
    let mut length: ft::FT_ULong = 0;
    let error = ft::FT_Load_Sfnt_Table(face, tag as _, 0, std::ptr::null_mut(), &mut length);
    if error != 0 {
        return std::ptr::null_mut();
    }
    let mut table = Box::new(vec![0u8; length as usize]);
    let error = ft::FT_Load_Sfnt_Table(face, tag as _, 0, table.as_mut_ptr(), &mut length);
    if error != 0 {
        return std::ptr::null_mut();
    }
    let data = table.as_ptr() as *const c_char;
    let raw = Box::into_raw(table);
    hb::hb_blob_create(
        data,
        length as c_uint,
        hb::HB_MEMORY_MODE_WRITABLE,
        raw as *mut c_void,
        Some(free_table),
    )
}

impl FontInst {
    /// `XeTeXFontInst(pathname, index, pointSize, status)` and
    /// `initialize`: `None` where XeTeX sets `status` to 1. `afm` finds the
    /// AFM file of a Type 1 font (kpathsea's `afm` format).
    pub fn new(
        lib: &Rc<FtLibrary>,
        pathname: &[u8],
        index: u32,
        point_size: f32,
        afm: &mut dyn FnMut(&str) -> Option<String>,
    ) -> Option<FontInst> {
        let cpath = CString::new(pathname.to_vec()).ok()?;
        let mut face: ft::FT_Face = std::ptr::null_mut();
        // SAFETY: a live library and a NUL-terminated path.
        let error = unsafe { ft::FT_New_Face(lib.0, cpath.as_ptr(), index as _, &mut face) };
        if error != 0 {
            return None;
        }
        // From here the face is owned by `inst`.
        let mut inst = FontInst {
            units_per_em: 0,
            point_size,
            ascent: 0.0,
            descent: 0.0,
            cap_height: 0.0,
            x_height: 0.0,
            italic_angle: 0.0,
            vertical: std::cell::Cell::new(false),
            filename: pathname.to_vec(),
            index,
            ft_face: face,
            hb_font: std::ptr::null_mut(),
            _lib: lib.clone(),
        };
        // SAFETY: `face` is live.
        unsafe {
            if !ft::FT_IS_SCALABLE(face) {
                return None;
            }
            // For non-sfnt-packaged fonts (presumably Type 1), see if there
            // is an AFM file we can attach.
            if index == 0 && !ft::FT_IS_SFNT(face) {
                let base = pathname.rsplit(|&b| b == b'/').next().unwrap_or(pathname);
                let mut afm_name = base.to_vec();
                if let Some(dot) = afm_name.iter().rposition(|&b| b == b'.') {
                    let ext = &afm_name[dot..];
                    if ext.len() == 4
                        && ext[1].eq_ignore_ascii_case(&b'p')
                        && ext[2].eq_ignore_ascii_case(&b'f')
                    {
                        afm_name.truncate(dot);
                        afm_name.extend_from_slice(b".afm");
                    }
                }
                if let Some(full) = afm(&String::from_utf8_lossy(&afm_name)) {
                    if let Ok(c) = CString::new(full) {
                        ft::FT_Attach_File(face, c.as_ptr());
                    }
                }
            }
            inst.units_per_em = (*face).units_per_EM;
            inst.ascent = inst.units_to_points((*face).ascender as f32);
            inst.descent = inst.units_to_points((*face).descender as f32);

            let post = ft::FT_Get_Sfnt_Table(face, ft::ft_sfnt_post) as *const ft::TT_Postscript;
            if !post.is_null() {
                inst.italic_angle = fix2d((*post).italicAngle as i32) as f32;
            }
            let os2 = ft::FT_Get_Sfnt_Table(face, ft::ft_sfnt_os2) as *const ft::TT_OS2;
            if !os2.is_null() {
                inst.cap_height = inst.units_to_points((*os2).sCapHeight as f32);
                inst.x_height = inst.units_to_points((*os2).sxHeight as f32);
            }

            // Set up the HarfBuzz font.
            let hb_face = hb::hb_face_create_for_tables(Some(get_table), face as *mut c_void, None);
            hb::hb_face_set_index(hb_face, index);
            hb::hb_face_set_upem(hb_face, inst.units_per_em as c_uint);
            inst.hb_font = hb::hb_font_create(hb_face);
            hb::hb_face_destroy(hb_face);
            let funcs = font_funcs();
            hb::hb_font_set_funcs(inst.hb_font, funcs, face as *mut c_void, None);
            hb::hb_font_funcs_destroy(funcs);
            hb::hb_font_set_scale(
                inst.hb_font,
                inst.units_per_em as i32,
                inst.units_per_em as i32,
            );
            // We don't want device tables adjustments.
            hb::hb_font_set_ppem(inst.hb_font, 0, 0);
        }
        Some(inst)
    }

    /// `unitsToPoints`, in C's `float`.
    pub fn units_to_points(&self, units: f32) -> f32 {
        (units * self.point_size) / (self.units_per_em as f32)
    }

    /// `getFontTable(OTTag)`: a copy of the table, if the font has it.
    pub fn font_table(&self, tag: u32) -> Option<Vec<u8>> {
        let mut length: ft::FT_ULong = 0;
        // SAFETY: a live face.
        unsafe {
            if ft::FT_Load_Sfnt_Table(self.ft_face, tag as _, 0, std::ptr::null_mut(), &mut length)
                != 0
            {
                return None;
            }
            let mut v = vec![0u8; length as usize];
            if ft::FT_Load_Sfnt_Table(self.ft_face, tag as _, 0, v.as_mut_ptr(), &mut length) != 0 {
                return None;
            }
            Some(v)
        }
    }

    pub fn hb_face(&self) -> *mut hb::hb_face_t {
        // SAFETY: a live font.
        unsafe { hb::hb_font_get_face(self.hb_font) }
    }

    /// `getGlyphBounds`.
    pub fn glyph_bounds(&self, gid: u32) -> GlyphBBox {
        let mut bbox = GlyphBBox::default();
        // SAFETY: a live face; the glyph is copied and freed.
        unsafe {
            if ft::FT_Load_Glyph(self.ft_face, gid, ft::FT_LOAD_NO_SCALE) != 0 {
                return bbox;
            }
            let mut glyph: ft::FT_Glyph = std::ptr::null_mut();
            if ft::FT_Get_Glyph((*self.ft_face).glyph, &mut glyph) == 0 {
                let mut b = ft::FT_BBox {
                    xMin: 0,
                    yMin: 0,
                    xMax: 0,
                    yMax: 0,
                };
                ft::FT_Glyph_Get_CBox(glyph, ft::FT_GLYPH_BBOX_UNSCALED, &mut b);
                bbox.x_min = self.units_to_points(b.xMin as f32);
                bbox.y_min = self.units_to_points(b.yMin as f32);
                bbox.x_max = self.units_to_points(b.xMax as f32);
                bbox.y_max = self.units_to_points(b.yMax as f32);
                ft::FT_Done_Glyph(glyph);
            }
        }
        bbox
    }

    /// `mapCharToGlyph`.
    pub fn map_char_to_glyph(&self, ch: u32) -> u32 {
        // SAFETY: a live face.
        unsafe { ft::FT_Get_Char_Index(self.ft_face, ch as _) as u32 }
    }

    /// `getNumGlyphs`: `num_glyphs` as C's `uint16_t`.
    pub fn num_glyphs(&self) -> u16 {
        // SAFETY: a live face.
        unsafe { (*self.ft_face).num_glyphs as u16 }
    }

    /// `getGlyphWidth`.
    pub fn glyph_width(&self, gid: u32) -> f32 {
        self.units_to_points(glyph_advance(self.ft_face, gid, false) as f32)
    }

    /// `getGlyphHeightDepth`.
    pub fn glyph_height_depth(&self, gid: u32) -> (f32, f32) {
        let b = self.glyph_bounds(gid);
        (b.y_max, -b.y_min)
    }

    /// `getGlyphSidebearings`.
    pub fn glyph_sidebearings(&self, gid: u32) -> (f32, f32) {
        let width = self.glyph_width(gid);
        let b = self.glyph_bounds(gid);
        (b.x_min, width - b.x_max)
    }

    /// `getGlyphItalCorr`.
    pub fn glyph_ital_corr(&self, gid: u32) -> f32 {
        let width = self.glyph_width(gid);
        let b = self.glyph_bounds(gid);
        if b.x_max > width {
            b.x_max - width
        } else {
            0.0
        }
    }

    /// `mapGlyphToIndex`.
    pub fn map_glyph_to_index(&self, name: &[u8]) -> u32 {
        let Ok(c) = CString::new(name.to_vec()) else {
            return 0;
        };
        // SAFETY: a live face and a NUL-terminated name.
        unsafe { ft::FT_Get_Name_Index(self.ft_face, c.as_ptr()) as u32 }
    }

    /// `getGlyphName`: the glyph's name, empty without glyph names.
    pub fn glyph_name(&self, gid: u32) -> Vec<u8> {
        // SAFETY: a live face; the buffer is 256 bytes, as XeTeX's.
        unsafe {
            if !ft::FT_HAS_GLYPH_NAMES(self.ft_face) {
                return vec![];
            }
            let mut buf = [0u8; 256];
            ft::FT_Get_Glyph_Name(self.ft_face, gid, buf.as_mut_ptr() as *mut c_void, 256);
            let n = buf.iter().position(|&b| b == 0).unwrap_or(256);
            buf[..n].to_vec()
        }
    }

    /// `getFirstCharCode`.
    pub fn first_char_code(&self) -> i32 {
        let mut gindex: ft::FT_UInt = 0;
        // SAFETY: a live face.
        unsafe { ft::FT_Get_First_Char(self.ft_face, &mut gindex) as i32 }
    }

    /// `getLastCharCode`.
    pub fn last_char_code(&self) -> i32 {
        let mut gindex: ft::FT_UInt = 0;
        // SAFETY: a live face.
        unsafe {
            let mut ch = ft::FT_Get_First_Char(self.ft_face, &mut gindex) as i32;
            let mut prev = ch;
            while gindex != 0 {
                prev = ch;
                ch = ft::FT_Get_Next_Char(self.ft_face, ch as _, &mut gindex) as i32;
            }
            prev
        }
    }
}

/// XeTeX_ext.c's `D2Fix`: C's `(int)(d * 65536.0 + 0.5)`, truncating.
pub fn d2fix(d: f64) -> i32 {
    (d * 65536.0 + 0.5) as i32
}

/// XeTeX_ext.c's `Fix2D`.
pub fn fix2d(f: i32) -> f64 {
    f as f64 / 65536.0
}
