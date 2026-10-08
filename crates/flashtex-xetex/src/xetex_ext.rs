//! XeTeX's C parts behind the interface of `changes/ext.ch`:
//! `XeTeX_ext.c`, `XeTeX_pic.c`, `XeTeXOTMath.cpp`, `hz.cpp`, `trans.c`,
//! `xetex.h`'s macros, and the texmfmp.c routines xetex.web calls. XeTeX's
//! own code is MIT-licensed (third_party/xetex/COPYING); the ports here keep
//! its behaviour, and its notices are in `LICENSE` of this crate.
//!
//! Phase S0 (docs/design/xetex/PLAN.md) supports TFM fonts only. What serves
//! installed fonts, graphics and TECkit mappings is a stub that answers as
//! TeX Live's XeTeX does when nothing is found: `find_native_font` finds no
//! font, `find_pic_file` no picture, `load_tfm_font_mapping` no mapping. The
//! routines that can only be reached through a native font, an OpenType
//! assembly or a glyph-info array (which then never exist) answer 0. Phases
//! S1-S2 replace them.

use crate::generated::types::{real_point, real_rect, transform};
use crate::generated::Globals;

impl Globals {
    // ---- xetex.h: the bit fields of a math code ---------------------------

    pub fn math_fam_field(&mut self, x: i32) -> i32 {
        ((x as u32 >> 24) & 0xFF) as i32
    }
    pub fn math_class_field(&mut self, x: i32) -> i32 {
        ((x as u32 >> 21) & 0x07) as i32
    }
    pub fn math_char_field(&mut self, x: i32) -> i32 {
        (x as u32 & 0x1FFFFF) as i32
    }
    pub fn set_family_field(&mut self, x: i32) -> i32 {
        ((x as u32 & 0xFF) << 24) as i32
    }
    pub fn set_class_field(&mut self, x: i32) -> i32 {
        ((x as u32 & 0x07) << 21) as i32
    }
    pub fn cast_to_ushort(&mut self, x: i32) -> i32 {
        (x as u32 & 0xFFFF) as i32
    }

    // ---- hz.cpp -----------------------------------------------------------

    pub fn get_cp_code(&mut self, f: i32, c: i32, side: i32) -> i32 {
        self.host
            .protrusion
            .get(&(f, c as u32, side))
            .copied()
            .unwrap_or(0)
    }
    pub fn set_cp_code(&mut self, f: i32, c: i32, side: i32, v: i32) {
        self.host.protrusion.insert((f, c as u32, side), v);
    }

    // ---- XeTeX_ext.c: native fonts (S0: none is ever found) ---------------

    /// `findnativefont`: no installed font is found in phase S0, so every
    /// font is a TFM font, as with TeX Live's XeTeX when the name is no
    /// installed font.
    pub fn find_native_font(&mut self, _s: i32) -> i32 {
        0
    }
    pub fn release_font_engine(&mut self, _engine: i32, _type_flag: i32) {}
    pub fn ot_get_font_metrics(
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
    /// `makefontdef`: the definition of a native font for the XDV file.
    pub fn make_font_def(&mut self, _f: i32) -> i32 {
        0
    }
    pub fn make_xdv_glyph_array_data(&mut self, _p: i32) -> i32 {
        0
    }
    pub fn xdv_buffer_byte(&mut self, k: i32) -> i32 {
        self.xdv_buffer[k as usize]
    }

    /// The UTF-16 code units of a native word node follow its
    /// `native_node_size` words, four to a word, low half first (C's
    /// `(uint16_t*)&mem[p+native_node_size]` on a little-endian machine).
    pub fn get_native_char(&mut self, p: i32, i: i32) -> i32 {
        let w = self.mem[(p + crate::generated::consts::native_node_size + i / 4) as usize];
        ((w.to_bits() >> (16 * (i % 4))) & 0xFFFF) as i32
    }
    pub fn set_native_char(&mut self, p: i32, i: i32, c: i32) {
        let k = (p + crate::generated::consts::native_node_size + i / 4) as usize;
        let sh = 16 * (i % 4);
        let w = self.mem[k].to_bits();
        let w = (w & !(0xFFFFu64 << sh)) | (((c as u64) & 0xFFFF) << sh);
        self.mem[k] = crate::generated::types::memory_word::from_bits(w);
    }
    /// `get_native_usv`: the character at `i`, a surrogate pair read as one.
    pub fn get_native_usv(&mut self, p: i32, i: i32) -> i32 {
        let c = self.get_native_char(p, i);
        if (0xD800..0xDC00).contains(&c) {
            let lo = self.get_native_char(p, i + 1);
            0x10000 + (c - 0xD800) * 0x400 + lo - 0xDC00
        } else {
            c
        }
    }
    pub fn get_native_glyph(&mut self, _p: i32, _i: i32) -> i32 {
        0
    }
    pub fn set_native_metrics(&mut self, _p: i32, _use_glyph_metrics: bool) {}
    pub fn set_justified_native_glyphs(&mut self, _p: i32) {}
    pub fn set_native_glyph_metrics(&mut self, _p: i32, _use_glyph_metrics: bool) {}
    pub fn get_native_italic_correction(&mut self, _p: i32) -> i32 {
        0
    }
    pub fn get_native_glyph_italic_correction(&mut self, _p: i32) -> i32 {
        0
    }
    pub fn get_native_char_height_depth(&mut self, _f: i32, _c: i32, h: &mut i32, d: &mut i32) {
        (*h, *d) = (0, 0);
    }
    pub fn get_native_char_sidebearings(&mut self, _f: i32, _c: i32, lsb: &mut i32, rsb: &mut i32) {
        (*lsb, *rsb) = (0, 0);
    }
    pub fn getnativecharwd(&mut self, _f: i32, _c: i32) -> i32 {
        0
    }
    pub fn getnativecharht(&mut self, _f: i32, _c: i32) -> i32 {
        0
    }
    pub fn getnativechardp(&mut self, _f: i32, _c: i32) -> i32 {
        0
    }
    pub fn getnativecharic(&mut self, _f: i32, _c: i32) -> i32 {
        0
    }
    pub fn get_glyph_bounds(&mut self, _f: i32, _edge: i32, _gid: i32) -> i32 {
        0
    }
    pub fn map_char_to_glyph(&mut self, _f: i32, _c: i32) -> i32 {
        0
    }
    pub fn map_glyph_to_index(&mut self, _f: i32) -> i32 {
        0
    }
    pub fn get_font_char_range(&mut self, _f: i32, _first: bool) -> i32 {
        0
    }
    pub fn print_glyph_name(&mut self, _f: i32, _gid: i32) {}
    pub fn get_native_word_cp(&mut self, _p: i32, _side: i32) -> i32 {
        0
    }
    #[allow(non_snake_case)]
    pub fn usingOpenType(&mut self, _engine: i32) -> bool {
        false
    }
    #[allow(non_snake_case)]
    pub fn usingGraphite(&mut self, _engine: i32) -> bool {
        false
    }
    #[allow(non_snake_case)]
    pub fn isOpenTypeMathFont(&mut self, _engine: i32) -> bool {
        false
    }
    pub fn aat_font_get(&mut self, _what: i32, _engine: i32) -> i32 {
        0
    }
    pub fn aat_font_get_1(&mut self, _what: i32, _engine: i32, _p: i32) -> i32 {
        0
    }
    pub fn aat_font_get_2(&mut self, _what: i32, _engine: i32, _p1: i32, _p2: i32) -> i32 {
        0
    }
    pub fn aat_font_get_named(&mut self, _what: i32, _engine: i32) -> i32 {
        0
    }
    pub fn aat_font_get_named_1(&mut self, _what: i32, _engine: i32, _p: i32) -> i32 {
        0
    }
    pub fn aat_print_font_name(&mut self, _what: i32, _engine: i32, _p1: i32, _p2: i32) {}
    pub fn ot_font_get(&mut self, _what: i32, _engine: i32) -> i32 {
        0
    }
    pub fn ot_font_get_1(&mut self, _what: i32, _engine: i32, _p: i32) -> i32 {
        0
    }
    pub fn ot_font_get_2(&mut self, _what: i32, _engine: i32, _p1: i32, _p2: i32) -> i32 {
        0
    }
    pub fn ot_font_get_3(&mut self, _what: i32, _engine: i32, _p1: i32, _p2: i32, _p3: i32) -> i32 {
        0
    }
    pub fn gr_font_get_named(&mut self, _what: i32, _engine: i32) -> i32 {
        0
    }
    pub fn gr_font_get_named_1(&mut self, _what: i32, _engine: i32, _p: i32) -> i32 {
        0
    }
    pub fn gr_print_font_name(&mut self, _what: i32, _engine: i32, _p1: i32, _p2: i32) {}
    /// `linebreak_start`/`linebreak_next` (ICU's line breaker): reached only
    /// with a native font.
    pub fn linebreak_start(&mut self, _f: i32, _locale: i32, _s: i32, _len: i32) {}
    pub fn linebreak_next(&mut self) -> i32 {
        -1
    }
    pub fn terminate_font_manager(&mut self) {}
    pub fn print_utf8_str(&mut self, _s: i32, _len: i32) {}

    // ---- TECkit mappings (S0: none) ---------------------------------------

    /// `checkfortfmfontmapping`: a `:mapping=NAME` after a TFM font's name
    /// is cut off `name_of_file` (and would name a TECkit mapping, which
    /// phase S0 does not load).
    pub fn check_for_tfm_font_mapping(&mut self) {
        let n = (self.name_length.max(0) as usize).min(self.name_of_file.len());
        let pat = b":mapping=";
        if let Some(i) = self.name_of_file[..n]
            .windows(pat.len())
            .position(|w| w == pat)
        {
            self.name_of_file[i] = 0;
            self.name_length = i as i32;
        }
    }
    pub fn load_tfm_font_mapping(&mut self) -> i32 {
        0
    }
    pub fn apply_tfm_font_mapping(&mut self, _m: i32, c: i32) -> i32 {
        c
    }
    pub fn apply_mapping_pool(&mut self, _m: i32, _s: i32, _len: i32) -> i32 {
        0
    }
    pub fn apply_mapping_native(&mut self, _m: i32, _len: i32) -> i32 {
        0
    }

    // ---- glyph-info arrays: handles of `Host::handles` (state.rs) --------

    /// xetex.web's `libc_free(native_glyph_info_ptr(p))`.
    pub fn free_glyph_info(&mut self, h: i32) {
        self.host.handles.free(h);
    }
    /// xetex.web's copy of a glyph-info array (`xmalloc_array` and
    /// `memcpy`): a new handle for the same glyphs. The arrays are never
    /// changed in place, so the copy shares them.
    pub fn copy_glyph_info(&mut self, h: i32) -> i32 {
        match self.host.handles.get(h).cloned() {
            Some(o) => self.host.handles.alloc(o),
            None => 0,
        }
    }

    // ---- XeTeXOTMath.cpp (reached only with an OpenType math font) --------

    pub fn get_native_mathsy_param(&mut self, _f: i32, _n: i32) -> i32 {
        0
    }
    pub fn get_native_mathex_param(&mut self, _f: i32, _n: i32) -> i32 {
        0
    }
    pub fn get_ot_math_constant(&mut self, _f: i32, _n: i32) -> i32 {
        0
    }
    pub fn get_ot_math_variant(
        &mut self,
        _f: i32,
        g: i32,
        _v: i32,
        adv: &mut i32,
        _horiz: i32,
    ) -> i32 {
        *adv = -1;
        g
    }
    pub fn get_ot_assembly_ptr(&mut self, _f: i32, _g: i32, _horiz: i32) -> i32 {
        0
    }
    pub fn free_ot_assembly(&mut self, _a: i32) {}
    pub fn get_ot_math_ital_corr(&mut self, _f: i32, _g: i32) -> i32 {
        0
    }
    pub fn get_ot_math_accent_pos(&mut self, _f: i32, _g: i32) -> i32 {
        0x7fffffff
    }
    pub fn get_ot_math_kern(
        &mut self,
        _f: i32,
        _g: i32,
        _sf: i32,
        _sg: i32,
        _cmd: i32,
        _shift: i32,
    ) -> i32 {
        0
    }
    pub fn ot_part_count(&mut self, _a: i32) -> i32 {
        0
    }
    pub fn ot_part_glyph(&mut self, _a: i32, _i: i32) -> i32 {
        0
    }
    pub fn ot_part_is_extender(&mut self, _a: i32, _i: i32) -> bool {
        false
    }
    pub fn ot_part_start_connector(&mut self, _f: i32, _a: i32, _i: i32) -> i32 {
        0
    }
    pub fn ot_part_end_connector(&mut self, _f: i32, _a: i32, _i: i32) -> i32 {
        0
    }
    pub fn ot_part_full_advance(&mut self, _f: i32, _a: i32, _i: i32) -> i32 {
        0
    }
    pub fn ot_min_connector_overlap(&mut self, _f: i32) -> i32 {
        0
    }

    // ---- XeTeX_pic.c and trans.c ------------------------------------------

    /// `find_pic_file`: no picture is read in phase S0; the answer is
    /// XeTeX_pic.c's for a file kpathsea does not find (-1, which xetex.web
    /// reports as "not a recognized image format").
    pub fn find_pic_file(
        &mut self,
        path: &mut i32,
        bounds: &mut real_rect,
        _pdf_box_type: i32,
        _page: i32,
    ) -> i32 {
        *path = 0;
        *bounds = real_rect::default();
        -1
    }
    pub fn pic_path_len(&mut self, _path: i32) -> i32 {
        0
    }
    pub fn pic_path_to_mem(&mut self, _path: i32, _p: i32) {}
    /// `pic_path_byte(p, i)`: byte `i` of the path stored after a picture
    /// node's `pic_node_size` words, eight to a word.
    pub fn pic_path_byte(&mut self, p: i32, i: i32) -> i32 {
        let w = self.mem[(p + crate::generated::consts::pic_node_size + i / 8) as usize];
        ((w.to_bits() >> (8 * (i % 8))) & 0xFF) as i32
    }
    pub fn count_pdf_file_pages(&mut self) -> i32 {
        0
    }
    #[allow(non_snake_case)]
    pub fn D2Fix(&mut self, d: f64) -> i32 {
        (d * 65536.0 + 0.5) as i32
    }
    #[allow(non_snake_case)]
    pub fn Fix2D(&mut self, f: i32) -> f64 {
        f as f64 / 65536.0
    }
    #[allow(non_snake_case)]
    pub fn setPoint(&mut self, p: &mut real_point, x: f64, y: f64) {
        p.x = x;
        p.y = y;
    }
    pub fn make_identity(&mut self, t: &mut transform) {
        *t = transform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            x: 0.0,
            y: 0.0,
        };
    }
    pub fn make_scale(&mut self, t: &mut transform, xs: f64, ys: f64) {
        *t = transform {
            a: xs,
            b: 0.0,
            c: 0.0,
            d: ys,
            x: 0.0,
            y: 0.0,
        };
    }
    pub fn make_translation(&mut self, t: &mut transform, dx: f64, dy: f64) {
        *t = transform {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            x: dx,
            y: dy,
        };
    }
    pub fn make_rotation(&mut self, t: &mut transform, a: f64) {
        *t = transform {
            a: a.cos(),
            b: a.sin(),
            c: -a.sin(),
            d: a.cos(),
            x: 0.0,
            y: 0.0,
        };
    }
    pub fn transform_point(&mut self, p: &mut real_point, t: &mut transform) {
        let r = real_point {
            x: t.a * p.x + t.c * p.y + t.x,
            y: t.b * p.x + t.d * p.y + t.y,
        };
        *p = r;
    }
    pub fn transform_concat(&mut self, t1: &mut transform, t2: &mut transform) {
        let r = transform {
            a: t1.a * t2.a + t1.b * t2.c + 0.0 * t2.x,
            b: t1.a * t2.b + t1.b * t2.d + 0.0 * t2.y,
            c: t1.c * t2.a + t1.d * t2.c + 0.0 * t2.x,
            d: t1.c * t2.b + t1.d * t2.d + 0.0 * t2.y,
            x: t1.x * t2.a + t1.y * t2.c + 1.0 * t2.x,
            y: t1.x * t2.b + t1.y * t2.d + 1.0 * t2.y,
        };
        *t1 = r;
    }

    // ---- texmfmp.c: time, files, MD5 --------------------------------------

    /// `initstarttime` (texmfmp.c): `SOURCE_DATE_EPOCH`, else the clock.
    pub fn init_start_time(&mut self) {
        let _ = self.start_time();
    }

    /// `get_seconds_and_micros` (texmfmp.c).
    pub fn seconds_and_micros(&mut self, s: &mut i32, m: &mut i32) {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        *s = d.as_secs() as i32;
        *m = d.subsec_micros() as i32;
    }

    /// `get_date_and_time` (texmfmp.c): the time in UTC of
    /// `SOURCE_DATE_EPOCH` (or of now) if `FORCE_SOURCE_DATE=1`, else the
    /// local time now.
    pub fn date_and_time(&mut self, t: &mut i32, d: &mut i32, m: &mut i32, y: &mut i32) {
        let forced = std::env::var("FORCE_SOURCE_DATE").map(|v| v == "1") == Ok(true);
        let tm = if forced {
            flashtex_engine::os::broken_down(self.start_time(), true)
        } else {
            flashtex_engine::os::broken_down(now_secs(), false)
        };
        *t = tm.tm_hour * 60 + tm.tm_min;
        *d = tm.tm_mday;
        *m = tm.tm_mon + 1;
        *y = tm.tm_year + 1900;
    }

    /// Append ASCII bytes to the pool (`strpool[poolptr++] = (uint16_t)c`).
    fn pool_append_ascii(&mut self, s: &[u8]) {
        for &c in s {
            self.str_pool[self.pool_ptr as usize] = c as i32;
            self.pool_ptr += 1;
        }
    }

    /// `getcreationdate` (texmfmp.c).
    pub fn getcreationdate(&mut self) {
        let s = self.start_time_str();
        if self.pool_ptr as usize + s.len() >= crate::generated::consts::pool_size as usize {
            self.pool_ptr = crate::generated::consts::pool_size;
            return;
        }
        self.pool_append_ascii(&s);
    }

    /// `find_input_file` (texmfmp.c, XeTeX's): string `s` as UTF-8, looked
    /// for in `-output-directory`, then as a TeX input.
    fn find_input_file(&mut self, s: i32) -> Option<String> {
        let name = crate::system::tex_string_utf8(self, s);
        let name = String::from_utf8_lossy(&name).into_owned();
        flashtex_engine::system::find_input(&name)
    }

    /// `getfilemoddate` (texmfmp.c).
    pub fn getfilemoddate(&mut self, s: i32) {
        let Some(path) = self.find_input_file(s) else {
            return;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            return;
        };
        let t = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let s = flashtex_engine::pdftex::utils::make_pdf_time(t, force_source_date());
        if self.pool_ptr as usize + s.len() >= crate::generated::consts::pool_size as usize {
            self.pool_ptr = crate::generated::consts::pool_size;
        } else {
            self.pool_append_ascii(&s);
        }
    }

    /// `getfilesize` (texmfmp.c).
    pub fn getfilesize(&mut self, s: i32) {
        let Some(path) = self.find_input_file(s) else {
            return;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            return;
        };
        let b = meta.len().to_string().into_bytes();
        if self.pool_ptr as usize + b.len() >= crate::generated::consts::pool_size as usize {
            self.pool_ptr = crate::generated::consts::pool_size;
        } else {
            self.pool_append_ascii(&b);
        }
    }

    /// `getfiledump` (texmfmp.c): `len` bytes from `offset`, in hex.
    pub fn getfiledump(&mut self, s: i32, offset: i32, len: i32) {
        if len == 0 {
            return;
        }
        let size = crate::generated::consts::pool_size;
        if self.pool_ptr + 2 * len + 1 >= size {
            self.pool_ptr = size;
            return;
        }
        let Some(path) = self.find_input_file(s) else {
            return;
        };
        let Ok(data) = std::fs::read(&path) else {
            return;
        };
        if offset < 0 {
            return;
        }
        let from = (offset as usize).min(data.len());
        let to = (from + len as usize).min(data.len());
        let hex: Vec<u8> = data[from..to]
            .iter()
            .flat_map(|b| format!("{b:02X}").into_bytes())
            .collect();
        self.pool_append_ascii(&hex);
    }

    /// `getmd5sum` (texmfmp.c): of a file, or of a string as UTF-8.
    pub fn getmd5sum(&mut self, s: i32, is_file: bool) {
        let digest = if is_file {
            let Some(path) = self.find_input_file(s) else {
                return;
            };
            let Ok(data) = std::fs::read(&path) else {
                return;
            };
            flashtex_engine::pdftex::md5::digest(&data)
        } else {
            let bytes = crate::system::tex_string_utf8(self, s);
            flashtex_engine::pdftex::md5::digest(&bytes)
        };
        if self.pool_ptr + 32 >= crate::generated::consts::pool_size {
            return;
        }
        let hex: Vec<u8> = digest
            .iter()
            .flat_map(|b| format!("{b:02X}").into_bytes())
            .collect();
        self.pool_append_ascii(&hex);
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Globals {
    /// texmfmp.c's `start_time`: `SOURCE_DATE_EPOCH`, else the clock, read
    /// once per run (`Host::start`).
    fn start_time(&mut self) -> i64 {
        if let Some((t, _)) = self.host.start.as_ref() {
            return *t;
        }
        let (t, sde) = match std::env::var("SOURCE_DATE_EPOCH") {
            Ok(v) => (v.trim().parse::<i64>().unwrap_or(0), true),
            Err(_) => (now_secs(), false),
        };
        let s = flashtex_engine::pdftex::utils::make_pdf_time(t, sde);
        self.host.start = Some((t, s));
        t
    }

    /// texmfmp.c's `start_time_str`: the start time as a PDF date.
    fn start_time_str(&mut self) -> Vec<u8> {
        self.start_time();
        self.host
            .start
            .as_ref()
            .map(|s| s.1.clone())
            .unwrap_or_default()
    }
}

/// Whether `FORCE_SOURCE_DATE` and `SOURCE_DATE_EPOCH` are both set.
fn force_source_date() -> bool {
    std::env::var("FORCE_SOURCE_DATE").map(|v| v == "1") == Ok(true)
        && std::env::var_os("SOURCE_DATE_EPOCH").is_some()
}
