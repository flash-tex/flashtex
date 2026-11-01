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
    /// The image `name` as a copy holds it: not open yet (a persisted S₀'s
    /// table, `images::State`'s codec).
    pub(crate) fn unopened(name: &[u8]) -> PngImage {
        let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        PngImage {
            raw: std::cell::Cell::new(std::ptr::null_mut()),
            name: CString::new(&name[..end]).expect("no NUL"),
        }
    }

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

crate::codec_struct!(State {
    transparent_page_group,
    last_png_needs_page_group,
    transparent_page_group_was_written
});

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

/// How many of a chunk's `len` bytes are colour, `j mod P < C`.
fn colour_len<const P: usize, const C: usize>(len: usize) -> usize {
    len / P * C + (len % P).min(C)
}

/// The colour bytes of `chunk` (`j mod P < C`) into `dst`, which holds
/// exactly [`colour_len`] of them, the others pushed onto `smask`, each in
/// order.
fn split_alpha<const P: usize, const C: usize>(chunk: &[u8], dst: &mut [i32], smask: &mut Vec<u8>) {
    let groups = chunk.chunks_exact(P);
    let rem = groups.remainder();
    let tail = rem.len().min(C);
    smask.reserve(chunk.len() - dst.len());
    let mut w = 0;
    for g in groups {
        for (d, &s) in dst[w..w + C].iter_mut().zip(&g[..C]) {
            *d = s as i32;
        }
        w += C;
        smask.extend_from_slice(&g[C..]);
    }
    for (d, &s) in dst[w..].iter_mut().zip(&rem[..tail]) {
        *d = s as i32;
    }
    smask.extend_from_slice(&rem[tail..]);
}

impl Pixel {
    /// Does byte `j` of a chunk go to the alpha mask? (the rule
    /// `split_alpha` applies; the test below compares the two)
    #[cfg(test)]
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

/// What `write_png` reads of the run to decide how an image is written:
/// the PDF version and the image parameters `check_pdfversion` fixed (with
/// `\pdfimagehicolor` off below PDF 1.5, as `write_png` sets it).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct PngParams {
    major: i32,
    minor: i32,
    hicolor: bool,
    apply_gamma: i32,
    gamma: i32,
    image_gamma: i32,
}

impl PngParams {
    fn of(g: &Globals) -> PngParams {
        PngParams {
            major: g.fixed_pdf_major_version,
            minor: g.fixed_pdf_minor_version,
            hicolor: g.fixed_image_hicolor
                && !(g.fixed_pdf_major_version == 1 && g.fixed_pdf_minor_version < 5),
            apply_gamma: g.fixed_image_apply_gamma,
            gamma: g.fixed_gamma,
            image_gamma: g.fixed_image_gamma,
        }
    }
}

/// `write_png`'s transformations of the decoded image (transparency to
/// alpha, alpha stripped below PDF 1.4, 16 bits stripped without hicolor,
/// gamma; then the interlace handling and the info update), in its order;
/// `Ok((png_copy, int_file_gamma))`, or `Err` when libpng fails.
fn png_prepare(png: &PngImage, p: &PngParams) -> Result<(bool, i64), ()> {
    let tr = |what: c_int, a: f64, b: f64| {
        // SAFETY: a live handle; errors come back as -1.
        if unsafe { ftpng_transform(png.h(), what, a, b) } != 0 {
            Err(())
        } else {
            Ok(())
        }
    };
    let mut png_copy = true;
    let mut gamma = 0.0f64;
    let mut int_file_gamma: i64 = 0;
    // simple transparency support
    if png.valid(PNG_INFO_TRNS) {
        tr(TRNS_TO_ALPHA, 0.0, 0.0)?;
        png_copy = false;
    }
    // alpha channel support
    if p.major == 1 && p.minor < 4 && png.color_type() & PNG_COLOR_MASK_ALPHA != 0 {
        tr(STRIP_ALPHA, 0.0, 0.0)?;
        png_copy = false;
    }
    // 16 bit depth support
    if png.bit_depth() == 16 && !p.hicolor {
        tr(STRIP_16, 0.0, 0.0)?;
        png_copy = false;
    }
    // gamma support
    if png.valid(PNG_INFO_GAMA) {
        (gamma, int_file_gamma) = png.gamma();
    }
    if p.apply_gamma != 0 {
        let screen = p.gamma as f64 / 1000.0;
        if png.valid(PNG_INFO_GAMA) {
            tr(SET_GAMMA, screen, gamma)?;
        } else {
            let file = 1000.0 / p.image_gamma as f64;
            tr(SET_GAMMA, screen, file)?;
        }
        png_copy = false;
    }
    // reset structure
    tr(INTERLACE_HANDLING, 0.0, 0.0)?;
    tr(UPDATE_INFO, 0.0, 0.0)?;
    Ok((png_copy, int_file_gamma))
}

/// `write_png`'s test for copying the file's IDAT data unchanged.
fn png_copy_ok(png: &PngImage, png_copy: bool, p: &PngParams, int_file_gamma: i64) -> bool {
    let ct = png.color_type();
    png_copy
        && (p.major > 1 || p.minor > 1)
        && png.interlace_type() == PNG_INTERLACE_NONE
        && (ct == PNG_COLOR_TYPE_GRAY || ct == PNG_COLOR_TYPE_RGB)
        && p.apply_gamma == 0
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
}

impl Globals {
    /// "libpng: internal error": libpng longjmp'ed.
    fn png_internal_error(&mut self) -> ! {
        self.pdftex_fail("libpng: internal error")
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
        let ahead_key = ahead::key(self, &cname);
        let png = PngImage {
            raw: std::cell::Cell::new(raw),
            name: cname,
        };
        // resolution support
        e.width = png.width() as i32;
        e.height = png.height() as i32;
        if png.valid(PNG_INFO_PHYS) {
            e.x_res = crate::system::pas_round(0.0254 * png.get(X_PPM) as u32 as f64);
            e.y_res = crate::system::pas_round(0.0254 * png.get(Y_PPM) as u32 as f64);
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
        if let Some(k) = ahead_key {
            ahead::start(k);
        }
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
            match px {
                Pixel::Simple => self.pdf_buf_store(chunk),
                Pixel::Gray8 => self.pdf_buf_store_split::<2, 1>(chunk, smask),
                Pixel::Gray16 => self.pdf_buf_store_split::<4, 2>(chunk, smask),
                Pixel::Rgb8 => self.pdf_buf_store_split::<4, 3>(chunk, smask),
                Pixel::Rgb16 => self.pdf_buf_store_split::<8, 6>(chunk, smask),
            }
            k -= l;
        }
    }

    /// `pdf_buf_store` of the bytes of `chunk` that are not alpha, the
    /// others pushed onto `smask`, both in order: the bytes `j` of the chunk
    /// with `j mod P < C` are colour, the rest alpha, which is
    /// `Pixel::is_alpha` for each pixel layout (Gray8 2/1, Gray16 4/2, Rgb8
    /// 4/3, Rgb16 8/6). A group at the chunk's end cut short keeps the same
    /// rule. The caller has made room for the whole chunk.
    fn pdf_buf_store_split<const P: usize, const C: usize>(
        &mut self,
        chunk: &[u8],
        smask: &mut Vec<u8>,
    ) {
        let n = colour_len::<P, C>(chunk.len());
        let p = self.pdf_ptr as usize;
        let buf = if self.pdf_buf_is_os {
            &mut self.pdf_os_buf
        } else {
            &mut self.pdf_op_buf
        };
        split_alpha::<P, C>(chunk, &mut buf[p..p + n], smask);
        self.pdf_ptr += n as i32;
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
            self.png_interlace_warning(png);
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

    /// `write_interlaced`'s warning about a large interlaced image.
    fn png_interlace_warning(&mut self, png: &PngImage) {
        if png.interlace_type() != PNG_INTERLACE_NONE
            && png.height().wrapping_mul(png.rowbytes() as u64) >= 10240000
        {
            self.pdftex_warn(
                "large interlaced PNG might cause out of memory (use non-interlaced PNG to fix this)",
            );
        }
    }

    /// A stream of rows: the bytes `ahead` compressed if the stream takes
    /// them (`pdf_stream_compressed`; the warning `png_write_rows` would
    /// give is given), else `png_write_rows`. True for the first.
    fn png_rows_or(
        &mut self,
        png: &PngImage,
        px: Pixel,
        smask: &mut Vec<u8>,
        pre: Option<&[u8]>,
    ) -> bool {
        match pre {
            Some(b) if self.pdf_stream_compressed(b) => {
                self.png_interlace_warning(png);
                true
            }
            _ => {
                self.png_write_rows(png, px, smask);
                false
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
    fn write_png_palette(&mut self, png: &PngImage, cs_ref: i32, pre: Option<&ahead::Streams>) {
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
        self.png_rows_or(png, Pixel::Simple, &mut Vec::new(), ahead::plain(pre));
        self.pdf_end_stream();
        if palette_objnum > 0 {
            self.png_write_palette_obj(palette_objnum, &palette);
        }
    }

    /// `write_png_gray` and `write_png_rgb`.
    fn write_png_plain(
        &mut self,
        png: &PngImage,
        cs_ref: i32,
        device: &[u8],
        pre: Option<&ahead::Streams>,
    ) {
        self.png_colorspace(cs_ref, device);
        self.pdf_begin_stream();
        self.png_rows_or(png, Pixel::Simple, &mut Vec::new(), ahead::plain(pre));
        self.pdf_end_stream();
    }

    /// `write_png_gray_alpha` and `write_png_rgb_alpha`: the colour into
    /// the image, the alpha channel into its `/SMask`.
    fn write_png_alpha(
        &mut self,
        png: &PngImage,
        cs_ref: i32,
        rgb: bool,
        pre: Option<&ahead::Streams>,
    ) {
        let (pre_colour, pre_mask) = ahead::alpha(pre);
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
        let colour_ahead = self.png_rows_or(png, px, &mut smask, pre_colour);
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
            if colour_ahead {
                // the mask compressed ahead with the colour; else (it never
                // is, both streams being in the same state) the rows that
                // fill `smask` are read now
                if pre_mask.is_some_and(|b| self.pdf_stream_compressed(b)) {
                    self.pdf_end_stream();
                    return;
                }
                match ahead::rows(png, px) {
                    Some((_, m)) => smask = m,
                    None => self.png_internal_error(),
                }
                smask.resize(smask_size, 0);
            }
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
        st.last_png_needs_page_group = false;
        let palette = png.palette();
        if self.fixed_pdf_major_version == 1 && self.fixed_pdf_minor_version < 5 {
            self.fixed_image_hicolor = false;
        }
        self.pdf_puts(b"/Type /XObject\n/Subtype /Image\n");
        // transparency, alpha, 16 bits, gamma; reset structure
        let params = PngParams::of(self);
        let Ok((png_copy, int_file_gamma)) = png_prepare(png, &params) else {
            self.png_internal_error();
        };

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
        if png_copy_ok(png, png_copy, &params, int_file_gamma) {
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
            let t0 = std::time::Instant::now();
            let pre = if ahead::buffer_ok(self) {
                ahead::key(self, &png.name).and_then(|k| ahead::take(&k))
            } else {
                None
            };
            if std::env::var_os("FLASHTEX_PNG_AHEAD_DEBUG").is_some() {
                // (stderr only: nothing the run writes changes)
                eprintln!(
                    "png-ahead: {} {}, waited {:.1} ms",
                    png.name.to_string_lossy(),
                    if pre.is_some() { "ahead" } else { "not ahead" },
                    t0.elapsed().as_secs_f64() * 1e3
                );
            }
            let pre = pre.as_deref();
            match ct {
                PNG_COLOR_TYPE_PALETTE => self.write_png_palette(png, cs_ref, pre),
                PNG_COLOR_TYPE_GRAY => self.write_png_plain(png, cs_ref, b"/DeviceGray\n", pre),
                PNG_COLOR_TYPE_GRAY_ALPHA => {
                    if alpha_ok {
                        self.write_png_alpha(png, cs_ref, false, pre);
                        st.last_png_needs_page_group = true;
                    } else {
                        self.write_png_plain(png, cs_ref, b"/DeviceGray\n", pre);
                    }
                }
                PNG_COLOR_TYPE_RGB => self.write_png_plain(png, cs_ref, b"/DeviceRGB\n", pre),
                PNG_COLOR_TYPE_RGB_ALPHA => {
                    if alpha_ok {
                        self.write_png_alpha(png, cs_ref, true, pre);
                        st.last_png_needs_page_group = true;
                    } else {
                        self.write_png_plain(png, cs_ref, b"/DeviceRGB\n", pre);
                    }
                }
                _ => self.pdftex_fail(&format!("unsupported type of color_type <{ct}>")),
            }
        }
        self.pdf_flush();
        self.write_additional_png_objects(st);
    }
}

/// PNG streams compressed ahead (lane THROUGHPUT, DESIGN.md §5.6 item 7:
/// "compress ... in parallel at export. zlib is deterministic, so the PDF
/// stays byte-identical").
///
/// A PNG that `write_png` decodes (not copied) is decoded and its streams
/// compressed twice over in pdfTeX's order: libpng's rows, then zlib's
/// level-9 deflate of every stream byte, on the engine's thread, when the
/// page that uses it is shipped out. Here `\pdfximage` (`read_png_info`)
/// starts a worker thread that does that work on a handle of its own: the
/// same transformations (`png_prepare`, with the parameters
/// `check_pdfversion` has already fixed), the same rows, the colour and mask
/// bytes `write_png_plain`, `write_png_palette` and `write_png_alpha` put in
/// their streams, each compressed by TeX Live's zlib at `\pdfcompresslevel`
/// in one stream from `deflateInit` to `Z_FINISH`. A stream's compressed
/// bytes depend only on its bytes and the level, not on how they reach
/// zlib (`write_zip` gives them a PDF buffer at a time), so they are the
/// bytes `write_zip` would write. When `write_png` reaches the stream, it
/// takes the result if everything the worker assumed still holds -- the file
/// (its length and modification time), the parameters, the level, no
/// preview or draft mode, a PDF buffer whose size keeps a pixel's bytes in
/// one chunk -- and `write_zip` writes those bytes where it would have
/// compressed (`super::output`). Anything else, or a worker that failed,
/// and the stream is made as before. The dictionaries are written as before
/// in every case.
mod ahead {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Condvar, Mutex, OnceLock};

    /// What a result was computed from.
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    pub(super) struct Key {
        name: CString,
        len: u64,
        mtime: Option<std::time::SystemTime>,
        params: PngParams,
        level: i32,
    }

    /// The compressed streams of an image: the image's, and its mask's.
    pub(super) enum Streams {
        Plain(Vec<u8>),
        Alpha(Vec<u8>, Vec<u8>),
    }

    #[derive(Default)]
    struct Job {
        done: Mutex<Option<Option<Arc<Streams>>>>,
        cv: Condvar,
    }

    /// Results by key; the bytes they hold, to bound them.
    #[derive(Default)]
    struct Table {
        jobs: HashMap<Key, Arc<Job>>,
        bytes: usize,
    }

    /// At most this many compressed bytes are kept (a resident host compiles
    /// the same document again; a command-line run uses each once).
    const KEEP: usize = 256 << 20;

    fn table() -> &'static Mutex<Table> {
        static T: OnceLock<Mutex<Table>> = OnceLock::new();
        T.get_or_init(Default::default)
    }

    /// Workers running at once: the other cores.
    fn permits() -> &'static (Mutex<usize>, Condvar) {
        static P: OnceLock<(Mutex<usize>, Condvar)> = OnceLock::new();
        P.get_or_init(|| {
            let n = std::thread::available_parallelism().map_or(1, |n| n.get());
            (Mutex::new(n.saturating_sub(1).max(1)), Condvar::new())
        })
    }

    fn enabled() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var("FLASHTEX_PNG_AHEAD").as_deref() != Ok("0"))
    }

    /// The key of image `name` as the run stands now, if a result could be
    /// used for it.
    pub(super) fn key(g: &mut Globals, name: &CString) -> Option<Key> {
        if !enabled()
            || !super::super::xpdf::LINKED
            || super::super::preview()
            || g.fixed_pdf_draftmode != 0
        {
            return None;
        }
        let level = g.get_pdf_compress_level();
        if level <= 0 {
            return None;
        }
        let m = std::fs::metadata(name.to_str().ok()?).ok()?;
        Some(Key {
            name: name.clone(),
            len: m.len(),
            mtime: m.modified().ok(),
            params: PngParams::of(g),
            level,
        })
    }

    /// Whether the PDF buffer keeps a pixel in one chunk now (its size a
    /// multiple of 8; an image is written outside object streams), as the
    /// worker assumed. (At `\\pdfximage` the buffer may be the object
    /// stream's; only the writing matters.)
    pub(super) fn buffer_ok(g: &Globals) -> bool {
        !g.pdf_os_mode && g.pdf_buf_size > 0 && g.pdf_buf_size % 8 == 0
    }

    /// Start the work for `key` unless it is done or under way.
    pub(super) fn start(key: Key) {
        let job = {
            let mut t = table().lock().unwrap_or_else(|e| e.into_inner());
            if t.jobs.contains_key(&key) {
                return;
            }
            if t.bytes > KEEP {
                // the finished ones go; one under way is still waited for
                t.jobs.retain(|_, j| match j.done.lock() {
                    Ok(d) => d.is_none(),
                    Err(_) => false,
                });
                t.bytes = 0;
            }
            let job = Arc::new(Job::default());
            t.jobs.insert(key.clone(), job.clone());
            job
        };
        let k = key.clone();
        let spawned = std::thread::Builder::new()
            .name("png-ahead".into())
            .spawn(move || {
                let (m, cv) = permits();
                {
                    let mut n = m.lock().unwrap_or_else(|e| e.into_inner());
                    while *n == 0 {
                        n = cv.wait(n).unwrap_or_else(|e| e.into_inner());
                    }
                    *n -= 1;
                }
                let r = std::panic::catch_unwind(|| compute(&k))
                    .ok()
                    .flatten()
                    .map(Arc::new);
                {
                    *m.lock().unwrap_or_else(|e| e.into_inner()) += 1;
                    cv.notify_one();
                }
                if let Some(s) = &r {
                    let n = match &**s {
                        Streams::Plain(a) => a.len(),
                        Streams::Alpha(a, b) => a.len() + b.len(),
                    };
                    table().lock().unwrap_or_else(|e| e.into_inner()).bytes += n;
                }
                *job.done.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
                job.cv.notify_all();
            });
        if spawned.is_err() {
            table()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .jobs
                .remove(&key);
        }
    }

    /// The result for `key`, waiting for its worker; `None` if there is none.
    pub(super) fn take(key: &Key) -> Option<Arc<Streams>> {
        let job = table()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .jobs
            .get(key)?
            .clone();
        let mut d = job.done.lock().unwrap_or_else(|e| e.into_inner());
        while d.is_none() {
            d = job.cv.wait(d).unwrap_or_else(|e| e.into_inner());
        }
        d.clone().flatten()
    }

    /// The image stream, of a result for a plain or palette image.
    pub(super) fn plain(s: Option<&Streams>) -> Option<&[u8]> {
        match s {
            Some(Streams::Plain(a)) => Some(a),
            _ => None,
        }
    }

    /// The image and mask streams, of a result for an image with alpha.
    pub(super) fn alpha(s: Option<&Streams>) -> (Option<&[u8]>, Option<&[u8]>) {
        match s {
            Some(Streams::Alpha(a, b)) => (Some(a), Some(b)),
            _ => (None, None),
        }
    }

    /// TeX Live's zlib on all of `data` at `level`: `deflateInit`, one
    /// `deflate(Z_FINISH)` loop, `deflateEnd`.
    fn compress(level: i32, data: &[u8]) -> Option<Vec<u8>> {
        let n = u32::try_from(data.len()).ok()?;
        let mut zs = super::super::zlib::ZStream::new();
        if zs.deflate_init(level) != super::super::zlib::Z_OK {
            return None;
        }
        let mut out = Vec::with_capacity(data.len() / 2 + 64);
        let mut buf = vec![0u8; 1 << 16];
        zs.next_in = data.as_ptr();
        zs.avail_in = n;
        let ok = loop {
            zs.next_out = buf.as_mut_ptr();
            zs.avail_out = buf.len() as u32;
            let r = zs.deflate(super::super::zlib::Z_FINISH);
            out.extend_from_slice(&buf[..buf.len() - zs.avail_out as usize]);
            if r == super::super::zlib::Z_STREAM_END {
                break true;
            }
            if r != super::super::zlib::Z_OK {
                break false;
            }
        };
        zs.next_in = std::ptr::null();
        zs.deflate_end();
        ok.then_some(out)
    }

    /// The streams `write_png` would compress for the image of `key`, or
    /// `None` when it copies the file's data instead (or libpng fails).
    fn compute(key: &Key) -> Option<Streams> {
        let mut err: c_int = 0;
        // SAFETY: a NUL-terminated name.
        let raw = unsafe { ftpng_open(key.name.as_ptr(), &mut err) };
        if raw.is_null() || err != 0 {
            return None;
        }
        let png = PngImage {
            raw: std::cell::Cell::new(raw),
            name: key.name.clone(),
        };
        let p = &key.params;
        let (png_copy, int_file_gamma) = png_prepare(&png, p).ok()?;
        if png_copy_ok(&png, png_copy, p, int_file_gamma) {
            return None;
        }
        let alpha_ok = p.minor >= 4 || p.major > 1;
        let ct = png.color_type();
        let rgb = match ct {
            PNG_COLOR_TYPE_GRAY_ALPHA if alpha_ok => false,
            PNG_COLOR_TYPE_RGB_ALPHA if alpha_ok => true,
            PNG_COLOR_TYPE_PALETTE
            | PNG_COLOR_TYPE_GRAY
            | PNG_COLOR_TYPE_GRAY_ALPHA
            | PNG_COLOR_TYPE_RGB
            | PNG_COLOR_TYPE_RGB_ALPHA => {
                let (colour, _) = rows(&png, Pixel::Simple)?;
                return Some(Streams::Plain(compress(key.level, &colour)?));
            }
            _ => return None,
        };
        // write_png_alpha
        let smask_size = (png.rowbytes() / if rgb { 4 } else { 2 }) * png.height() as usize;
        let sixteen = png.bit_depth() == 16 && p.hicolor;
        let px = match (rgb, sixteen) {
            (false, false) => Pixel::Gray8,
            (false, true) => Pixel::Gray16,
            (true, false) => Pixel::Rgb8,
            (true, true) => Pixel::Rgb16,
        };
        let (colour, mut smask) = rows(&png, px)?;
        smask.resize(smask_size, 0);
        let mask: Vec<u8> = if png.bit_depth() == 16 {
            smask.iter().step_by(2).copied().collect()
        } else {
            smask
        };
        Some(Streams::Alpha(
            compress(key.level, &colour)?,
            compress(key.level, &mask)?,
        ))
    }

    /// `png_write_rows`' bytes: the colour bytes and the mask bytes, in
    /// order. A row is split by its own byte positions, which are its
    /// chunks' (a chunk is `pdf_buf_size` bytes, a multiple of 8, and so of
    /// every pixel size).
    pub(super) fn rows(png: &PngImage, px: Pixel) -> Option<(Vec<u8>, Vec<u8>)> {
        let (height, rowbytes) = (png.height() as usize, png.rowbytes());
        let mut colour = Vec::new();
        let mut smask = Vec::new();
        let mut put = |row: &[u8]| {
            let at = colour.len();
            let n = match px {
                Pixel::Simple => row.len(),
                Pixel::Gray8 => colour_len::<2, 1>(row.len()),
                Pixel::Gray16 => colour_len::<4, 2>(row.len()),
                Pixel::Rgb8 => colour_len::<4, 3>(row.len()),
                Pixel::Rgb16 => colour_len::<8, 6>(row.len()),
            };
            if let Pixel::Simple = px {
                colour.extend_from_slice(row);
                return;
            }
            let mut dst = vec![0i32; n];
            match px {
                Pixel::Gray8 => split_alpha::<2, 1>(row, &mut dst, &mut smask),
                Pixel::Gray16 => split_alpha::<4, 2>(row, &mut dst, &mut smask),
                Pixel::Rgb8 => split_alpha::<4, 3>(row, &mut dst, &mut smask),
                _ => split_alpha::<8, 6>(row, &mut dst, &mut smask),
            }
            colour.extend(dst.iter().map(|&s| s as u8));
            debug_assert_eq!(colour.len(), at + n);
        };
        if png.interlace_type() == PNG_INTERLACE_NONE {
            let mut row = vec![0u8; rowbytes];
            for _ in 0..height {
                // SAFETY: `row` holds rowbytes bytes, what libpng writes.
                if unsafe { ftpng_read_row(png.h(), row.as_mut_ptr()) } != 0 {
                    return None;
                }
                put(&row);
            }
        } else {
            let mut rows: Vec<Vec<u8>> = (0..height).map(|_| vec![0u8; rowbytes]).collect();
            let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
            // SAFETY: `height` row pointers of rowbytes bytes each.
            if unsafe { ftpng_read_image(png.h(), ptrs.as_mut_ptr()) } != 0 {
                return None;
            }
            for row in &rows {
                put(row);
            }
        }
        Some((colour, smask))
    }

    #[cfg(test)]
    mod tests {
        use super::super::super::zlib::{ZStream, Z_FINISH, Z_NO_FLUSH, Z_OK, Z_STREAM_END};

        /// `write_zip`'s way: the bytes a PDF buffer at a time with
        /// `Z_NO_FLUSH`, then `Z_FINISH`, into a 32 KB output buffer.
        fn chunked(level: i32, data: &[u8], chunk: usize) -> Vec<u8> {
            let mut zs = ZStream::new();
            assert_eq!(zs.deflate_init(level), Z_OK);
            let mut out = Vec::new();
            let mut buf = vec![0u8; 32768];
            zs.next_out = buf.as_mut_ptr();
            zs.avail_out = buf.len() as u32;
            let parts: Vec<&[u8]> = data.chunks(chunk).collect();
            for (i, part) in parts.iter().enumerate() {
                let finish = i + 1 == parts.len();
                zs.next_in = part.as_ptr();
                zs.avail_in = part.len() as u32;
                loop {
                    if zs.avail_out == 0 {
                        out.extend_from_slice(&buf);
                        zs.next_out = buf.as_mut_ptr();
                        zs.avail_out = buf.len() as u32;
                    }
                    let r = zs.deflate(if finish { Z_FINISH } else { Z_NO_FLUSH });
                    if finish && r == Z_STREAM_END {
                        break;
                    }
                    assert_eq!(r, Z_OK);
                    if !finish && zs.avail_in == 0 {
                        break;
                    }
                }
            }
            out.extend_from_slice(&buf[..buf.len() - zs.avail_out as usize]);
            zs.next_in = std::ptr::null();
            zs.deflate_end();
            out
        }

        /// What `ahead` relies on: one stream compressed at once gives the
        /// bytes `write_zip` gives it a buffer at a time, at every level.
        #[test]
        fn compressed_at_once_is_compressed_a_buffer_at_a_time() {
            let mut x: u32 = 12345;
            let mut data = Vec::new();
            // runs, repeats and noise, a little over 1 MB
            while data.len() < 1_100_000 {
                x = x.wrapping_mul(1664525).wrapping_add(1013904223);
                let n = (x >> 24) as usize + 1;
                match x % 3 {
                    0 => data.extend(std::iter::repeat_n((x >> 8) as u8, n)),
                    1 => {
                        let at = data.len().saturating_sub(n * 3);
                        let rep: Vec<u8> = data[at..].iter().take(n).copied().collect();
                        data.extend(rep);
                    }
                    _ => data.extend((0..n).map(|i| (x >> (i % 24)) as u8)),
                }
            }
            for level in [1, 6, 9] {
                let once = super::compress(level, &data).expect("compressed");
                for chunk in [1000, 16384, 16385] {
                    assert_eq!(
                        once,
                        chunked(level, &data, chunk),
                        "level {level}, chunk {chunk}"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `split_alpha` sends each byte where the original per-byte loop did
    /// (`Pixel::is_alpha`), for every layout and every chunk length up to
    /// three of the largest pixel groups, a cut-short group included.
    fn check<const P: usize, const C: usize>(px: Pixel) {
        for len in 0..=3 * 8 + 7 {
            let chunk: Vec<u8> = (0..len).map(|j| (j * 7 + 1) as u8).collect();
            let (mut colour, mut alpha) = (Vec::new(), Vec::new());
            for (j, &b) in chunk.iter().enumerate() {
                if px.is_alpha(j) {
                    alpha.push(b);
                } else {
                    colour.push(b as i32);
                }
            }
            let mut dst = vec![-1; colour_len::<P, C>(len)];
            let mut smask = vec![9u8];
            split_alpha::<P, C>(&chunk, &mut dst, &mut smask);
            assert_eq!(dst, colour, "colour, len {len}");
            assert_eq!(smask[0], 9);
            assert_eq!(&smask[1..], &alpha[..], "alpha, len {len}");
        }
    }

    #[test]
    fn split_alpha_is_the_per_byte_loop() {
        check::<2, 1>(Pixel::Gray8);
        check::<4, 2>(Pixel::Gray16);
        check::<4, 3>(Pixel::Rgb8);
        check::<8, 6>(Pixel::Rgb16);
    }
}
