//! `writepng.c`, ported: PNG images, read with TeX Live's libpng 1.6.55
//! (third_party/libpng, unmodified, through `csrc/png_shim.c`).
//!
//! When pdfTeX can (no transformation, not interlaced, grey or RGB, no
//! colour-management chunks, PDF 1.2 or later) it copies the file's IDAT
//! data unchanged into a `/FlateDecode` stream with PNG predictors
//! (`copy_png`, "PNG copy" in the log); DESIGN.md §6.3 requires the same.
//! Otherwise libpng decodes the rows (and applies pdfTeX's transformations:
//! transparency to alpha, stripping alpha or 16 bits, gamma) and they are
//! written, compressed by pdfTeX's zlib, alpha as a separate `/SMask`.

use super::cfile::{CFile, Whence};
use super::images::{ImageData, ImageEntry, IMAGE_COLOR_B, IMAGE_COLOR_C, IMAGE_COLOR_I};
use crate::generated::Globals;
use std::ffi::{c_char, c_int, c_longlong, c_ulong, CStr, CString};

#[repr(C)]
struct FtPng {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PngColor {
    red: u8,
    green: u8,
    blue: u8,
}

super::xpdf::linked_or_unlinked! {
    fn ftpng_open(name: *const c_char, err: *mut c_int) -> *mut FtPng;
    fn ftpng_close(p: *mut FtPng);
    fn ftpng_get(p: *mut FtPng, what: c_int) -> c_ulong;
    fn ftpng_valid(p: *mut FtPng, flag: c_ulong) -> c_ulong;
    fn ftpng_plte(p: *mut FtPng, num: *mut c_int) -> *const PngColor;
    fn ftpng_gamma(p: *mut FtPng, gamma: *mut f64, fixed: *mut c_longlong);
    fn ftpng_transform(p: *mut FtPng, what: c_int, a: f64, b: f64) -> c_int;
    fn ftpng_read_row(p: *mut FtPng, row: *mut u8) -> c_int;
    fn ftpng_read_image(p: *mut FtPng, rows: *mut *mut u8) -> c_int;
    fn ftpng_header_version() -> *const c_char;
    fn ftpng_lib_version() -> *const c_char;
}

// png.h
const PNG_COLOR_MASK_ALPHA: i32 = 4;
const PNG_COLOR_TYPE_GRAY: i32 = 0;
const PNG_COLOR_TYPE_PALETTE: i32 = 3;
const PNG_COLOR_TYPE_RGB: i32 = 2;
const PNG_COLOR_TYPE_RGB_ALPHA: i32 = 6;
const PNG_COLOR_TYPE_GRAY_ALPHA: i32 = 4;
const PNG_INTERLACE_NONE: i32 = 0;
const PNG_INFO_GAMA: u64 = 0x0001;
const PNG_INFO_SBIT: u64 = 0x0002;
const PNG_INFO_CHRM: u64 = 0x0004;
const PNG_INFO_TRNS: u64 = 0x0010;
const PNG_INFO_BKGD: u64 = 0x0020;
const PNG_INFO_HIST: u64 = 0x0040;
const PNG_INFO_PHYS: u64 = 0x0080;
const PNG_INFO_SRGB: u64 = 0x0800;
const PNG_INFO_ICCP: u64 = 0x1000;
const PNG_INFO_SPLT: u64 = 0x2000;
const PNG_FP_1: i64 = 100000;

// csrc/png_shim.c
const WIDTH: c_int = 0;
const HEIGHT: c_int = 1;
const BIT_DEPTH: c_int = 2;
const COLOR_TYPE: c_int = 3;
const INTERLACE_TYPE: c_int = 4;
const ROWBYTES: c_int = 5;
const X_PPM: c_int = 6;
const Y_PPM: c_int = 7;
const TRNS_TO_ALPHA: c_int = 0;
const STRIP_ALPHA: c_int = 1;
const STRIP_16: c_int = 2;
const SET_GAMMA: c_int = 3;
const INTERLACE_HANDLING: c_int = 4;
const UPDATE_INFO: c_int = 5;

/// The libpng version pdfTeX's `--version` reports ("Compiled with libpng
/// %s; using libpng %s").
pub fn libpng_versions() -> (String, String) {
    // SAFETY: static strings.
    unsafe {
        (
            CStr::from_ptr(ftpng_header_version())
                .to_string_lossy()
                .into_owned(),
            CStr::from_ptr(ftpng_lib_version())
                .to_string_lossy()
                .into_owned(),
        )
    }
}

/// `png_image_struct`: libpng's read and info structures, with the file.
///
/// A copy (a checkpoint's, `images::State`) does not share the structures:
/// it opens the file again when it is first used. Between two commands (the
/// only place a checkpoint is taken) a handle is always as `read_png_info`
/// left it, having read the header only, so the reopened one is the same.
pub struct PngImage {
    raw: std::cell::Cell<*mut FtPng>,
    name: CString,
}

impl Drop for PngImage {
    fn drop(&mut self) {
        let p = self.raw.get();
        if !p.is_null() {
            // SAFETY: we own the structures; `ftpng_close` frees them and
            // closes the file.
            unsafe { ftpng_close(p) }
        }
    }
}

impl Clone for PngImage {
    fn clone(&self) -> PngImage {
        PngImage {
            raw: std::cell::Cell::new(std::ptr::null_mut()),
            name: self.name.clone(),
        }
    }
}

impl PngImage {
    /// The handle, opened again if this is a copy.
    fn h(&self) -> *mut FtPng {
        let mut p = self.raw.get();
        if p.is_null() {
            let mut err: c_int = 0;
            // SAFETY: a NUL-terminated name.
            p = unsafe { ftpng_open(self.name.as_ptr(), &mut err) };
            if p.is_null() || err != 0 {
                panic!(
                    "cannot open the image {} again after a restore",
                    self.name.to_string_lossy()
                );
            }
            self.raw.set(p);
        }
        p
    }

    fn get(&self, what: c_int) -> u64 {
        // SAFETY: a live handle; the query cannot fail.
        unsafe { ftpng_get(self.h(), what) as u64 }
    }
    fn width(&self) -> u64 {
        self.get(WIDTH)
    }
    fn height(&self) -> u64 {
        self.get(HEIGHT)
    }
    /// `png_get_bit_depth`.
    pub fn bit_depth(&self) -> i32 {
        self.get(BIT_DEPTH) as i32
    }
    fn color_type(&self) -> i32 {
        self.get(COLOR_TYPE) as i32
    }
    fn interlace_type(&self) -> i32 {
        self.get(INTERLACE_TYPE) as i32
    }
    fn rowbytes(&self) -> usize {
        self.get(ROWBYTES) as usize
    }
    fn valid(&self, flag: u64) -> bool {
        // SAFETY: a live handle.
        unsafe { ftpng_valid(self.h(), flag as c_ulong) != 0 }
    }
    /// `png_get_PLTE`: the palette (empty if none).
    fn palette(&self) -> Vec<PngColor> {
        let mut n: c_int = 0;
        // SAFETY: the shim returns `n` colours owned by libpng, copied at
        // once.
        unsafe {
            let p = ftpng_plte(self.h(), &mut n);
            if p.is_null() || n <= 0 {
                return Vec::new();
            }
            std::slice::from_raw_parts(p, n as usize).to_vec()
        }
    }
    /// `png_get_gAMA` and `png_get_gAMA_fixed`.
    fn gamma(&self) -> (f64, i64) {
        let (mut g, mut f): (f64, c_longlong) = (0.0, 0);
        // SAFETY: out-parameters.
        unsafe { ftpng_gamma(self.h(), &mut g, &mut f) };
        (g, f)
    }
}

#[derive(Default, Clone)]
pub struct State {
    /// `transparent_page_group`.
    transparent_page_group: i32,
    /// `last_png_needs_page_group`.
    last_png_needs_page_group: bool,
    /// `transparent_page_group_was_written`.
    transparent_page_group_was_written: bool,
}

/// How a decoded byte is routed: `write_simple_pixel` and the alpha
/// splitters `write_gray_pixel_8/16`, `write_rgb_pixel_8/16`.
#[derive(Clone, Copy)]
enum Pixel {
    Simple,
    Gray8,
    Gray16,
    Rgb8,
    Rgb16,
}

impl Pixel {
    /// Does byte `j` of a chunk go to the alpha mask?
    fn is_alpha(self, j: usize) -> bool {
        match self {
            Pixel::Simple => false,
            Pixel::Gray16 => j % 4 >= 2,
            Pixel::Gray8 => !j.is_multiple_of(2),
            Pixel::Rgb16 => j % 8 == 6 || j % 8 == 7,
            Pixel::Rgb8 => j % 4 == 3,
        }
    }
}

impl Globals {
    /// "libpng: internal error": libpng longjmp'ed.
    fn png_internal_error(&mut self) -> ! {
        self.pdftex_fail("libpng: internal error")
    }

    fn png_transform(&mut self, p: &PngImage, what: c_int, a: f64, b: f64) {
        // SAFETY: a live handle; errors come back as -1.
        if unsafe { ftpng_transform(p.h(), what, a, b) } != 0 {
            self.png_internal_error();
        }
    }

    /// `read_png_info`.
    pub(crate) fn read_png_info(&mut self, st: &mut State, e: &mut ImageEntry) {
        let name = e.name.clone().unwrap_or_default();
        let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        let cname = CString::new(&name[..end]).expect("no NUL");
        let mut err: c_int = 0;
        // SAFETY: a NUL-terminated name.
        let raw = unsafe { ftpng_open(cname.as_ptr(), &mut err) };
        match err {
            0 => {}
            1 => self.fatal_perror(&name),
            2 => self.pdftex_fail("libpng: png_create_read_struct() failed"),
            3 => self.pdftex_fail("libpng: png_create_info_struct() failed"),
            _ => self.png_internal_error(),
        }
        let png = PngImage {
            raw: std::cell::Cell::new(raw),
            name: cname,
        };
        // resolution support
        e.width = png.width() as i32;
        e.height = png.height() as i32;
        if png.valid(PNG_INFO_PHYS) {
            e.x_res = (0.0254 * png.get(X_PPM) as u32 as f64).round() as i32;
            e.y_res = (0.0254 * png.get(Y_PPM) as u32 as f64).round() as i32;
        }
        let ct = png.color_type();
        e.color_type = match ct {
            PNG_COLOR_TYPE_PALETTE => IMAGE_COLOR_C | IMAGE_COLOR_I,
            PNG_COLOR_TYPE_GRAY | PNG_COLOR_TYPE_GRAY_ALPHA => IMAGE_COLOR_B,
            PNG_COLOR_TYPE_RGB | PNG_COLOR_TYPE_RGB_ALPHA => IMAGE_COLOR_C,
            _ => self.pdftex_fail(&format!("unsupported type of color_type <{ct}>")),
        };
        if (self.fixed_pdf_major_version > 1 || self.fixed_pdf_minor_version >= 4)
            && (ct == PNG_COLOR_TYPE_GRAY_ALPHA || ct == PNG_COLOR_TYPE_RGB_ALPHA)
        {
            // png with alpha channel in device colours; we have to add a
            // Page Group to make Adobe happy, so we have to create a dummy
            // group object
            if st.transparent_page_group == 0 {
                st.transparent_page_group = self.pdf_new_objnum();
            }
            if self.pdf_page_group_val == 0 {
                self.pdf_page_group_val = st.transparent_page_group;
            }
            e.group_ref = self.pdf_page_group_val;
        }
        e.data = ImageData::Png(png);
    }

    /// Put `bytes` at `pdfptr` (room made by the caller), as the C code
    /// stores into `pdfbuf` directly (leaving `pdflastbyte` alone).
    fn pdf_buf_put(&mut self, bytes: &[u8]) {
        self.pdf_buf_store(bytes);
    }

    /// One decoded row through `write_noninterlaced`'s or
    /// `write_interlaced`'s chunk loop: `pdfbufsize` bytes at a time,
    /// `pdfroom` for the whole chunk, each byte to the buffer or the mask.
    fn png_write_row(&mut self, row: &[u8], px: Pixel, smask: &mut Vec<u8>) {
        let mut r = 0usize;
        let mut k = row.len();
        while k > 0 {
            let l = k.min(self.pdf_buf_size as usize);
            self.c_pdf_room(l as i32);
            let chunk = &row[r..r + l];
            r += l;
            if let Pixel::Simple = px {
                self.pdf_buf_store(chunk);
            } else {
                let mut color = Vec::with_capacity(l);
                for (j, &b) in chunk.iter().enumerate() {
                    if px.is_alpha(j) {
                        smask.push(b);
                    } else {
                        color.push(b);
                    }
                }
                self.pdf_buf_store(&color);
            }
            k -= l;
        }
    }

    /// The decoded image, row by row (`png_read_row`), or read whole first
    /// if interlaced (`png_read_image`).
    fn png_write_rows(&mut self, png: &PngImage, px: Pixel, smask: &mut Vec<u8>) {
        let (height, rowbytes) = (png.height() as usize, png.rowbytes());
        if png.interlace_type() == PNG_INTERLACE_NONE {
            let mut row = vec![0u8; rowbytes];
            for _ in 0..height {
                // SAFETY: `row` holds rowbytes bytes, what libpng writes.
                if unsafe { ftpng_read_row(png.h(), row.as_mut_ptr()) } != 0 {
                    self.png_internal_error();
                }
                self.png_write_row(&row, px, smask);
            }
        } else {
            if (height as u64).wrapping_mul(rowbytes as u64) >= 10240000 {
                self.pdftex_warn(
                    "large interlaced PNG might cause out of memory (use non-interlaced PNG to fix this)",
                );
            }
            let mut rows: Vec<Vec<u8>> = (0..height).map(|_| vec![0u8; rowbytes]).collect();
            let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
            // SAFETY: `height` row pointers of rowbytes bytes each.
            if unsafe { ftpng_read_image(png.h(), ptrs.as_mut_ptr()) } != 0 {
                self.png_internal_error();
            }
            for row in &rows {
                self.png_write_row(row, px, smask);
            }
        }
    }

    /// `/ColorSpace` of a written image: the `\pdfximage colorspace` object
    /// or the device space.
    fn png_colorspace(&mut self, cs_ref: i32, device: &[u8]) {
        if cs_ref != 0 {
            self.pdf_printf(format!("{cs_ref} 0 R\n").as_bytes());
        } else {
            self.pdf_puts(device);
        }
    }

    /// The palette stream of an indexed image.
    fn png_write_palette_obj(&mut self, palette_objnum: i32, palette: &[PngColor]) {
        self.pdf_begin_dict(palette_objnum, 0);
        self.pdf_begin_stream();
        for c in palette {
            self.c_pdf_room(3);
            self.pdf_buf_put(&[c.red, c.green, c.blue]);
        }
        self.pdf_end_stream();
    }

    /// `write_png_palette`.
    fn write_png_palette(&mut self, png: &PngImage, cs_ref: i32) {
        let palette = png.palette();
        self.pdf_create_obj(0, 0);
        let palette_objnum = self.obj_ptr;
        if cs_ref != 0 {
            self.pdf_printf(format!("{cs_ref} 0 R\n").as_bytes());
        } else {
            self.pdf_printf(
                format!(
                    "[/Indexed /DeviceRGB {} {} 0 R]\n",
                    palette.len() as i32 - 1,
                    palette_objnum
                )
                .as_bytes(),
            );
        }
        self.pdf_begin_stream();
        self.png_write_rows(png, Pixel::Simple, &mut Vec::new());
        self.pdf_end_stream();
        if palette_objnum > 0 {
            self.png_write_palette_obj(palette_objnum, &palette);
        }
    }

    /// `write_png_gray` and `write_png_rgb`.
    fn write_png_plain(&mut self, png: &PngImage, cs_ref: i32, device: &[u8]) {
        self.png_colorspace(cs_ref, device);
        self.pdf_begin_stream();
        self.png_write_rows(png, Pixel::Simple, &mut Vec::new());
        self.pdf_end_stream();
    }

    /// `write_png_gray_alpha` and `write_png_rgb_alpha`: the colour into
    /// the image, the alpha channel into its `/SMask`.
    fn write_png_alpha(&mut self, png: &PngImage, cs_ref: i32, rgb: bool) {
        self.png_colorspace(
            cs_ref,
            if rgb {
                b"/DeviceRGB\n"
            } else {
                b"/DeviceGray\n"
            },
        );
        self.pdf_create_obj(0, 0);
        let smask_objnum = self.obj_ptr;
        self.pdf_printf(format!("/SMask {smask_objnum} 0 R\n").as_bytes());
        let smask_size = (png.rowbytes() / if rgb { 4 } else { 2 }) * png.height() as usize;
        let mut smask = Vec::with_capacity(smask_size);
        let sixteen = png.bit_depth() == 16 && self.fixed_image_hicolor;
        let px = match (rgb, sixteen) {
            (false, false) => Pixel::Gray8,
            (false, true) => Pixel::Gray16,
            (true, false) => Pixel::Rgb8,
            (true, true) => Pixel::Rgb16,
        };
        self.pdf_begin_stream();
        self.png_write_rows(png, px, &mut smask);
        self.pdf_end_stream();
        self.pdf_flush();
        // C's mask buffer has exactly smask_size bytes
        smask.resize(smask_size, 0);
        // now write the Smask object
        if smask_objnum > 0 {
            let bitdepth = png.bit_depth();
            self.pdf_begin_dict(smask_objnum, 0);
            self.pdf_puts(b"/Type /XObject\n/Subtype /Image\n");
            self.pdf_printf(
                format!(
                    "/Width {}\n/Height {}\n/BitsPerComponent {}\n",
                    png.width() as i32,
                    png.height() as i32,
                    if bitdepth == 16 { 8 } else { bitdepth }
                )
                .as_bytes(),
            );
            self.pdf_puts(b"/ColorSpace /DeviceGray\n");
            self.pdf_begin_stream();
            // C: `pdfroom(8)` whenever i % 8 == 0, then the byte, with i
            // stepping by 2 (the high bytes) for 16 bits; so eight indices
            // at a time
            let step = if bitdepth == 16 { 2 } else { 1 };
            let mut group = Vec::with_capacity(8);
            let mut i = 0usize;
            while i < smask_size {
                self.c_pdf_room(8);
                let end = (i + 8).min(smask_size);
                group.clear();
                group.extend(smask[i..end].iter().step_by(step));
                self.pdf_buf_put(&group);
                i = end;
            }
            self.pdf_end_stream();
        }
    }

    /// `spng_getint`: a big-endian 32-bit chunk field.
    fn spng_getint(&mut self, fp: &mut CFile) -> i32 {
        let mut buf = [0u8; 4];
        if fp.read(&mut buf) != 4 {
            self.pdftex_fail("writepng: reading chunk type failed");
        }
        ((((((buf[0] as i32) << 8).wrapping_add(buf[1] as i32)) << 8).wrapping_add(buf[2] as i32))
            << 8)
            .wrapping_add(buf[3] as i32)
    }

    fn spng_seek(&mut self, fp: &mut CFile, off: i64, whence: Whence) {
        if !fp.seek(off, whence) {
            self.pdftex_fail("writepng: fseek in PNG file failed");
        }
    }

    /// `copy_png`: the IDAT chunks of the file, unchanged, as the stream
    /// (after a first pass for its `/Length`).
    fn copy_png(&mut self, png: &PngImage, name: &[u8]) {
        const SPNG_CHUNK_IDAT: i32 = 0x49444154;
        const SPNG_CHUNK_IEND: i32 = 0x49454E44;
        let Some(mut fp) = CFile::open(name) else {
            self.fatal_perror(name);
        };
        // 1st pass to find overall stream /Length
        let mut streamlength: i32 = 0;
        self.spng_seek(&mut fp, 8, Whence::Set);
        loop {
            let len = self.spng_getint(&mut fp);
            let typ = self.spng_getint(&mut fp);
            if typ == SPNG_CHUNK_IEND {
                break;
            }
            if typ == SPNG_CHUNK_IDAT {
                streamlength = streamlength.wrapping_add(len);
            }
            self.spng_seek(&mut fp, len as i64 + 4, Whence::Cur);
        }
        self.pdf_printf(
            format!(
                "/Length {streamlength}\n/Filter/FlateDecode\n/DecodeParms<</Colors {}/Columns {}/BitsPerComponent {}/Predictor 10>>\n>>\nstream\n",
                if png.color_type() == 2 { 3 } else { 1 },
                png.width() as i32,
                png.bit_depth()
            )
            .as_bytes(),
        );
        // 2nd pass to copy data
        let mut idat = 0; // flag to check continuous IDAT chunks sequence
        let mut chunk = Vec::new();
        self.spng_seek(&mut fp, 8, Whence::Set);
        loop {
            let mut len = self.spng_getint(&mut fp);
            let typ = self.spng_getint(&mut fp);
            if typ == SPNG_CHUNK_IDAT {
                // do copy
                if idat == 2 {
                    self.pdftex_fail("writepng: IDAT chunk sequence broken");
                }
                idat = 1;
                while len > 0 {
                    let i = len.min(self.pdf_buf_size);
                    self.c_pdf_room(i);
                    // fread into pdfbuf: bytes past the end of the file are
                    // not stored (the buffer keeps what it held)
                    chunk.resize(i as usize, 0);
                    let got = fp.read(&mut chunk);
                    let at = self.pdf_ptr;
                    self.pdf_buf_store(&chunk[..got]);
                    self.pdf_ptr = at + i;
                    len -= i;
                }
                self.spng_seek(&mut fp, 4, Whence::Cur);
            } else if typ == SPNG_CHUNK_IEND {
                // done
                self.pdf_end_stream();
                break;
            } else {
                if idat == 1 {
                    idat = 2;
                }
                self.spng_seek(&mut fp, len as i64 + 4, Whence::Cur);
            }
        }
    }

    /// `write_additional_png_objects`: the transparency group the page
    /// dictionary refers to, once, after the first PNG with alpha.
    fn write_additional_png_objects(&mut self, st: &mut State) {
        if st.last_png_needs_page_group
            && !st.transparent_page_group_was_written
            && st.transparent_page_group > 0
        {
            // create new group object
            st.transparent_page_group_was_written = true;
            self.pdf_begin_obj(st.transparent_page_group, 2);
            if self.get_pdf_compress_level() == 0 {
                self.pdf_puts(b"%PTEX Group needed for transparent pngs\n");
            }
            self.pdf_puts(b"<</Type/Group /S/Transparency /CS/DeviceRGB /I true>>\n");
            self.pdf_end_obj();
        }
    }

    /// `write_png`.
    pub(crate) fn write_png(&mut self, st: &mut State, e: &mut ImageEntry) {
        let cs_ref = e.colorspace_ref;
        let name = e.name.clone().unwrap_or_default();
        let ImageData::Png(png) = &e.data else {
            self.pdftex_fail("unknown type of image");
        };
        let mut png_copy = true;
        let mut gamma = 0.0f64;
        let mut int_file_gamma: i64 = 0;
        st.last_png_needs_page_group = false;
        let palette = png.palette();
        if self.fixed_pdf_major_version == 1 && self.fixed_pdf_minor_version < 5 {
            self.fixed_image_hicolor = false;
        }
        self.pdf_puts(b"/Type /XObject\n/Subtype /Image\n");
        // simple transparency support
        if png.valid(PNG_INFO_TRNS) {
            self.png_transform(png, TRNS_TO_ALPHA, 0.0, 0.0);
            png_copy = false;
        }
        // alpha channel support
        if self.fixed_pdf_major_version == 1
            && self.fixed_pdf_minor_version < 4
            && png.color_type() & PNG_COLOR_MASK_ALPHA != 0
        {
            self.png_transform(png, STRIP_ALPHA, 0.0, 0.0);
            png_copy = false;
        }
        // 16 bit depth support
        if self.fixed_pdf_major_version == 1 && self.fixed_pdf_minor_version < 5 {
            self.fixed_image_hicolor = false;
        }
        if png.bit_depth() == 16 && !self.fixed_image_hicolor {
            self.png_transform(png, STRIP_16, 0.0, 0.0);
            png_copy = false;
        }
        // gamma support
        if png.valid(PNG_INFO_GAMA) {
            (gamma, int_file_gamma) = png.gamma();
        }
        if self.fixed_image_apply_gamma != 0 {
            let screen = self.fixed_gamma as f64 / 1000.0;
            if png.valid(PNG_INFO_GAMA) {
                self.png_transform(png, SET_GAMMA, screen, gamma);
            } else {
                let file = 1000.0 / self.fixed_image_gamma as f64;
                self.png_transform(png, SET_GAMMA, screen, file);
            }
            png_copy = false;
        }
        // reset structure
        self.png_transform(png, INTERLACE_HANDLING, 0.0, 0.0);
        self.png_transform(png, UPDATE_INFO, 0.0, 0.0);

        self.pdf_printf(
            format!(
                "/Width {}\n/Height {}\n/BitsPerComponent {}\n",
                png.width() as i32,
                png.height() as i32,
                png.bit_depth()
            )
            .as_bytes(),
        );
        self.pdf_puts(b"/ColorSpace ");
        let ct = png.color_type();
        let apply_gamma = self.fixed_image_apply_gamma != 0;
        if png_copy
            && (self.fixed_pdf_major_version > 1 || self.fixed_pdf_minor_version > 1)
            && png.interlace_type() == PNG_INTERLACE_NONE
            && (ct == PNG_COLOR_TYPE_GRAY || ct == PNG_COLOR_TYPE_RGB)
            && !apply_gamma
            && (!png.valid(PNG_INFO_GAMA) || int_file_gamma == PNG_FP_1)
            && !png.valid(
                PNG_INFO_CHRM
                    | PNG_INFO_ICCP
                    | PNG_INFO_SBIT
                    | PNG_INFO_SRGB
                    | PNG_INFO_BKGD
                    | PNG_INFO_HIST
                    | PNG_INFO_TRNS
                    | PNG_INFO_SPLT,
            )
        {
            // Copy PNG
            let mut palette_objnum = 0;
            if cs_ref != 0 {
                self.pdf_printf(format!("{cs_ref} 0 R\n").as_bytes());
            } else {
                match ct {
                    PNG_COLOR_TYPE_PALETTE => {
                        self.pdf_create_obj(0, 0);
                        palette_objnum = self.obj_ptr;
                        self.pdf_printf(
                            format!(
                                "[/Indexed /DeviceRGB {} {} 0 R]\n",
                                palette.len() as i32 - 1,
                                palette_objnum
                            )
                            .as_bytes(),
                        );
                    }
                    PNG_COLOR_TYPE_GRAY => self.pdf_puts(b"/DeviceGray\n"),
                    _ => self.pdf_puts(b"/DeviceRGB\n"), // RGB
                }
            }
            self.tex_printf(b" (PNG copy)");
            self.copy_png(png, &name);
            if palette_objnum > 0 {
                self.png_write_palette_obj(palette_objnum, &palette);
            }
        } else {
            if std::env::var("TEXMF_DEBUG_PNG_COPY").as_deref() == Ok("1") {
                self.tex_printf(b" *** PNG copy skipped because:");
                if !png_copy {
                    self.tex_printf(b" !png_copy");
                }
                if self.fixed_pdf_major_version == 1 && self.fixed_pdf_minor_version <= 1 {
                    self.tex_printf(
                        format!(
                            " minorversion={} (and majorversion=1)",
                            self.fixed_pdf_minor_version
                        )
                        .as_bytes(),
                    );
                }
                if png.interlace_type() != PNG_INTERLACE_NONE {
                    self.tex_printf(b" interlaced");
                }
                if !(ct == PNG_COLOR_TYPE_GRAY || ct == PNG_COLOR_TYPE_RGB) {
                    self.tex_printf(b" colortype");
                }
                if apply_gamma {
                    self.tex_printf(b" apply gamma");
                }
                if !(!png.valid(PNG_INFO_GAMA) || int_file_gamma == PNG_FP_1) {
                    self.tex_printf(b" gamma");
                }
                for (flag, what) in [
                    (PNG_INFO_CHRM, &b" cHRM"[..]),
                    (PNG_INFO_ICCP, b" iCCP"),
                    (PNG_INFO_SBIT, b" sBIT"),
                    (PNG_INFO_SRGB, b" sRGB"),
                    (PNG_INFO_BKGD, b" bKGD"),
                    (PNG_INFO_HIST, b" hIST"),
                    (PNG_INFO_TRNS, b" tRNS"),
                    (PNG_INFO_SPLT, b" sPLT"),
                ] {
                    if png.valid(flag) {
                        self.tex_printf(what);
                    }
                }
            }
            let alpha_ok = self.fixed_pdf_minor_version >= 4 || self.fixed_pdf_major_version > 1;
            match ct {
                PNG_COLOR_TYPE_PALETTE => self.write_png_palette(png, cs_ref),
                PNG_COLOR_TYPE_GRAY => self.write_png_plain(png, cs_ref, b"/DeviceGray\n"),
                PNG_COLOR_TYPE_GRAY_ALPHA => {
                    if alpha_ok {
                        self.write_png_alpha(png, cs_ref, false);
                        st.last_png_needs_page_group = true;
                    } else {
                        self.write_png_plain(png, cs_ref, b"/DeviceGray\n");
                    }
                }
                PNG_COLOR_TYPE_RGB => self.write_png_plain(png, cs_ref, b"/DeviceRGB\n"),
                PNG_COLOR_TYPE_RGB_ALPHA => {
                    if alpha_ok {
                        self.write_png_alpha(png, cs_ref, true);
                        st.last_png_needs_page_group = true;
                    } else {
                        self.write_png_plain(png, cs_ref, b"/DeviceRGB\n");
                    }
                }
                _ => self.pdftex_fail(&format!("unsupported type of color_type <{ct}>")),
            }
        }
        self.pdf_flush();
        self.write_additional_png_objects(st);
    }
}
