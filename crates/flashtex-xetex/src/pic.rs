//! Pictures, ported from XeTeX's `XeTeX_pic.c` and `pdfimage.cpp` (MIT) and
//! the dimension scanners of `image/pngimage.c`, `image/jpegimage.c` and
//! `image/bmpimage.c` (GPL-2.0-or-later, from dvipdfmx), TeX Live 2026:
//! `\XeTeXpicfile` and `\XeTeXpdffile` need only a picture's size, never
//! its pixels (docs/design/xetex/PLAN.md, S2).
//!
//! The size of a PNG, JPEG or BMP file is read here from its header, as the
//! C does (libpng's `png_read_info` stops at the first `IDAT`, so only the
//! chunks before it count). A PDF file's page boxes are read through the
//! pdfTeX engine's xpdf (`flashtex_engine::pdftex::xpdf`) with pplib's
//! rules, which `pdfimage.cpp` uses: a box is the page's or an ancestor's
//! (`Parent`) array of four direct numbers, `Rotate` is the page's own
//! direct integer, and the page number is clamped to the document.
//!
//! The C's `realrect` and `realpoint` hold `float`s; `round_f32` stores
//! values into them as C does (here they are `f64` fields holding values
//! rounded to `f32`, which gives the same results: one `f64` operation on
//! two `f32` values then rounded to `f32` equals the `f32` operation).

use crate::generated::types::{memory_word, real_point, real_rect};
use crate::generated::Globals;
use crate::state::Object;
use flashtex_engine::pdftex::xpdf;
use flashtex_engine::resolver::Format;
use std::sync::Arc;

/// A value stored into a C `float` field.
pub fn round_f32(v: f64) -> f64 {
    v as f32 as f64
}

/// `pdfbox_crop` ... `pdfbox_art` (XeTeX_ext.h).
const PDFBOX_MEDIA: i32 = 2;
const PDFBOX_BLEED: i32 = 3;
const PDFBOX_TRIM: i32 = 4;
const PDFBOX_ART: i32 = 5;

/// A picture's pixel size and resolution (`png_info`, `JPEG_info`,
/// `bmp_info`: the fields `find_pic_file` reads).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelSize {
    pub width: f64,
    pub height: f64,
    pub xdpi: f64,
    pub ydpi: f64,
}

impl PixelSize {
    /// `bounds->wd`, `bounds->ht` in TeX points.
    fn bounds(&self) -> (f64, f64) {
        (
            (self.width * 72.27) / self.xdpi,
            (self.height * 72.27) / self.ydpi,
        )
    }
}

// ---- PNG (pngimage.c over libpng's png_read_info) ------------------------

/// `check_for_png`: `png_sig_cmp` on the first four bytes.
pub fn is_png(data: &[u8]) -> bool {
    data.len() >= 4 && data[..4] == [0x89, b'P', b'N', b'G']
}

/// `png_scan_file`: the size from `IHDR` and the resolution from a `pHYs`
/// chunk in pixels per metre before the first `IDAT` (72 dpi otherwise).
/// None where libpng would fail to read the header.
pub fn png_size(data: &[u8]) -> Option<PixelSize> {
    let be32 = |p: usize| -> Option<u32> {
        Some(u32::from_be_bytes(data.get(p..p + 4)?.try_into().ok()?))
    };
    if data.len() < 8 || data[..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return None;
    }
    let mut pos = 8;
    let mut size: Option<(u32, u32)> = None;
    let mut phys: Option<(u32, u32, u8)> = None;
    loop {
        let len = be32(pos)? as usize;
        let kind = data.get(pos + 4..pos + 8)?;
        let body = pos + 8;
        match kind {
            b"IHDR" => {
                if size.is_some() || len != 13 {
                    return None;
                }
                let (w, h) = (be32(body)?, be32(body + 4)?);
                if w == 0 || h == 0 || w > 0x7fff_ffff || h > 0x7fff_ffff {
                    return None;
                }
                size = Some((w, h));
            }
            // libpng ignores a second pHYs or one of the wrong length (a
            // benign error), and reads none after IDAT.
            b"pHYs" if size.is_some() && phys.is_none() && len == 9 => {
                phys = Some((be32(body)?, be32(body + 4)?, *data.get(body + 8)?));
            }
            b"IDAT" => break,
            b"IEND" => return None,
            // libpng reads IHDR first.
            _ => {
                size?;
            }
        }
        pos = body.checked_add(len)?.checked_add(4)?;
    }
    let (w, h) = size?;
    // png_get_x_pixels_per_meter: the pHYs values only in metres.
    let (xppm, yppm) = match phys {
        Some((x, y, 1)) => (x, y),
        _ => (0, 0),
    };
    let mut xdpi = xppm as f64 * 0.0254;
    let mut ydpi = yppm as f64 * 0.0254;
    if xdpi == 0.0 {
        xdpi = 72.0;
    }
    if ydpi == 0.0 {
        ydpi = 72.0;
    }
    Some(PixelSize {
        width: w as f64,
        height: h as f64,
        xdpi,
        ydpi,
    })
}

// ---- JPEG (jpegimage.c) ---------------------------------------------------

/// `check_for_jpeg`: an SOI marker.
pub fn is_jpeg(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8
}

/// A byte reader with C's `FILE` semantics for what `JPEG_scan_file` does:
/// reads past the end fail, seeks may pass it.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Option<u8> {
        let b = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(b)
    }
    fn pair(&mut self) -> Option<u16> {
        Some(u16::from_be_bytes([self.byte()?, self.byte()?]))
    }
    /// `fread(buf, 1, n, fp)`: the bytes there are (C leaves the rest of
    /// the buffer as it was; the callers' buffers start zeroed here).
    fn read(&mut self, n: usize) -> Vec<u8> {
        let mut v = vec![0; n];
        let have = self.data.len().saturating_sub(self.pos).min(n);
        v[..have].copy_from_slice(&self.data[self.pos..self.pos + have]);
        self.pos += have;
        v
    }
    /// `fread` of exactly `n` bytes, else None.
    fn read_exact(&mut self, n: usize) -> Option<Vec<u8>> {
        if self.data.len().saturating_sub(self.pos) < n {
            self.pos = self.data.len();
            return None;
        }
        Some(self.read(n))
    }
    fn seek_relative(&mut self, n: i64) {
        self.pos = (self.pos as i64 + n).max(0) as usize;
    }
    /// `JPEG_get_marker`: 0xFF, then any 0x00 and 0xFF fill bytes, then the
    /// marker.
    fn marker(&mut self) -> Option<u8> {
        if self.byte()? != 0xFF {
            return None;
        }
        loop {
            let c = self.byte()?;
            if c > 0 && c < 255 {
                return Some(c);
            }
        }
    }
}

/// `read_exif_bytes`: an `n`-byte (2 or 4) integer at `*p` in the TIFF
/// header's byte order; bytes outside `buf` read as 0.
fn exif_bytes(buf: &[u8], p: &mut i64, n: i64, big_endian: bool) -> u32 {
    let at = |i: i64| -> u32 {
        if i < 0 {
            0
        } else {
            buf.get(i as usize).copied().unwrap_or(0) as u32
        }
    };
    let mut v: u32 = 0;
    for k in 0..n {
        let i = if big_endian { *p + k } else { *p + n - 1 - k };
        v = (v << 8).wrapping_add(at(i));
    }
    *p += n;
    v
}

/// `read_APP1_Exif`: the resolution of the Exif IFD0 (`XResolution`,
/// `YResolution`, `ResolutionUnit`), set if no JFIF density came first.
fn read_exif(xdpi: &mut f64, ydpi: &mut f64, buf: &[u8]) {
    let length = buf.len() as i64;
    let at = |i: i64| -> u8 {
        if i < 0 {
            0
        } else {
            buf.get(i as usize).copied().unwrap_or(0)
        }
    };
    let mut p: i64 = 0;
    while p < length && at(p) == 0 {
        p += 1;
    }
    let tiff = p;
    let big_endian = match (at(p), at(p + 1)) {
        (b'M', b'M') => true,
        (b'I', b'I') => false,
        _ => return,
    };
    p += 2;
    if exif_bytes(buf, &mut p, 2, big_endian) != 42 {
        return;
    }
    let ifd = exif_bytes(buf, &mut p, 4, big_endian) as i32;
    p = tiff + ifd as i64;
    let mut fields = exif_bytes(buf, &mut p, 2, big_endian) as i32;
    let (mut value, mut num, mut den): (i32, i32, i32) = (0, 0, 0);
    let (mut xres, mut yres, mut unit) = (72.0f64, 72.0f64, 1.0f64);
    while fields > 0 {
        fields -= 1;
        let tag = exif_bytes(buf, &mut p, 2, big_endian) as i32;
        let typ = exif_bytes(buf, &mut p, 2, big_endian) as i32;
        exif_bytes(buf, &mut p, 4, big_endian);
        match typ {
            1 | 7 => {
                value = at(p) as i32;
                p += 4;
            }
            3 => {
                value = exif_bytes(buf, &mut p, 2, big_endian) as i32;
                p += 2;
            }
            4 | 9 => value = exif_bytes(buf, &mut p, 4, big_endian) as i32,
            5 | 10 => {
                value = exif_bytes(buf, &mut p, 4, big_endian) as i32;
                let mut rp = tiff + value as i64;
                num = exif_bytes(buf, &mut rp, 4, big_endian) as i32;
                den = exif_bytes(buf, &mut rp, 4, big_endian) as i32;
            }
            _ => p += 4,
        }
        match tag {
            // `xres = num / den`: C's integer division.
            282 if den != 0 => xres = num.wrapping_div(den) as f64,
            283 if den != 0 => yres = num.wrapping_div(den) as f64,
            296 => match value {
                2 => unit = 1.0,
                3 => unit = 2.54,
                _ => {}
            },
            _ => {}
        }
    }
    if *xdpi < 0.1 && *ydpi < 0.1 {
        *xdpi = xres * unit;
        *ydpi = yres * unit;
    }
}

/// `JPEG_scan_file`: the size from the first SOF marker and the resolution
/// from a JFIF or Exif segment before it (72 dpi otherwise). None where the
/// C returns an error.
pub fn jpeg_size(data: &[u8]) -> Option<PixelSize> {
    let mut r = Reader { data, pos: 0 };
    let (mut xdpi, mut ydpi) = (0.0f64, 0.0f64);
    let mut size: Option<(u16, u16)> = None;
    while size.is_none() {
        let Some(marker) = r.marker() else { break };
        // SOI and RST0..RST7 have no length.
        if marker == 0xD8 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        let mut length = r.pair()?.wrapping_sub(2);
        match marker {
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF => {
                r.byte()?;
                let height = r.pair()?;
                let width = r.pair()?;
                r.byte()?;
                size = Some((width, height));
            }
            0xE0 => {
                if length > 5 {
                    let sig = r.read_exact(5)?;
                    length = length.wrapping_sub(5);
                    if sig == b"JFIF\0" {
                        // read_APP0_JFIF
                        let _version = r.pair()?;
                        let units = r.byte()?;
                        let xd = r.pair()?;
                        let yd = r.pair()?;
                        let xt = r.byte()?;
                        let yt = r.byte()?;
                        let thumb = 3u16.wrapping_mul(xt as u16).wrapping_mul(yt as u16);
                        if thumb > 0 {
                            r.read(thumb as usize);
                        }
                        match units {
                            1 => (xdpi, ydpi) = (xd as f64, yd as f64),
                            2 => (xdpi, ydpi) = (xd as f64 * 2.54, yd as f64 * 2.54),
                            _ => (xdpi, ydpi) = (72.0, 72.0),
                        }
                        length = length.wrapping_sub(9u16.wrapping_add(thumb));
                    } else if sig == b"JFXX\0" {
                        // read_APP0_JFXX: the extension code, then the
                        // thumbnail skipped.
                        r.byte()?;
                        r.seek_relative(length as i64 - 1);
                        length = 0;
                    }
                }
                r.seek_relative(length as i64);
            }
            0xE1 => {
                if length > 5 {
                    let sig = r.read_exact(5)?;
                    length = length.wrapping_sub(5);
                    if sig == b"Exif\0" {
                        let buf = r.read(length as usize);
                        read_exif(&mut xdpi, &mut ydpi, &buf);
                        length = 0;
                    }
                }
                r.seek_relative(length as i64);
            }
            0xE2 => {
                if length >= 14 {
                    let sig = r.read_exact(12)?;
                    length = length.wrapping_sub(12);
                    if sig == b"ICC_PROFILE\0" {
                        // read_APP2_ICC: two bytes, then the chunk.
                        r.byte()?;
                        r.byte()?;
                        r.read(length.wrapping_sub(2) as usize);
                        length = 0;
                    }
                }
                r.seek_relative(length as i64);
            }
            0xEE => {
                if length > 5 {
                    let sig = r.read_exact(5)?;
                    length = length.wrapping_sub(5);
                    if sig == b"Adobe" {
                        // read_APP14_Adobe: seven bytes.
                        r.pair()?;
                        r.pair()?;
                        r.pair()?;
                        r.byte()?;
                        length = length.wrapping_sub(7);
                    }
                }
                r.seek_relative(length as i64);
            }
            _ => r.seek_relative(length as i64),
        }
    }
    let (width, height) = size?;
    if xdpi < 0.1 && ydpi < 0.1 {
        xdpi = 72.0;
        ydpi = 72.0;
    }
    Some(PixelSize {
        width: width as f64,
        height: height as f64,
        xdpi,
        ydpi,
    })
}

// ---- BMP (bmpimage.c) -----------------------------------------------------

/// `check_for_bmp`.
pub fn is_bmp(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == b'B' && data[1] == b'M'
}

/// What `bmp_scan_file` finds: a size, an error (-1), or a header type it
/// does not know, on which XeTeX exits.
#[derive(Debug, PartialEq)]
pub enum Bmp {
    Size(PixelSize),
    Error,
    UnknownHeader,
}

/// `bmp_scan_file`.
pub fn bmp_size(data: &[u8]) -> Bmp {
    // ULONG_LE: C's int arithmetic, so the top byte's bit 7 makes it
    // negative.
    let ulong = |p: usize| -> i64 {
        let b = |i: usize| data.get(p + i).copied().unwrap_or(0) as i32;
        b(0).wrapping_add(b(1) << 8)
            .wrapping_add(b(2) << 16)
            .wrapping_add(b(3).wrapping_shl(24)) as i64
    };
    let ushort = |p: usize| -> i64 {
        let b = |i: usize| data.get(p + i).copied().unwrap_or(0) as i64;
        b(0) + (b(1) << 8)
    };
    if data.len() < 18 || !is_bmp(data) {
        return Bmp::Error;
    }
    if ulong(6) != 0 {
        return Bmp::Error;
    }
    let offset = ulong(10);
    let hsize = ulong(14);
    if hsize < 4 || (data.len() as i64) < 18 + hsize - 4 {
        return Bmp::Error;
    }
    let p = 18;
    let (width, height, xdpi, ydpi, bit_count, psize);
    if hsize == 12 {
        width = ushort(p);
        height = ushort(p + 2);
        (xdpi, ydpi) = (72.0, 72.0);
        if ushort(p + 4) != 1 {
            return Bmp::Error;
        }
        bit_count = ushort(p + 6);
        psize = 3;
    } else if matches!(hsize, 40 | 64 | 108 | 124) {
        width = ulong(p) as i32 as i64;
        let h = ulong(p + 4) as i32;
        if ushort(p + 8) != 1 {
            return Bmp::Error;
        }
        bit_count = ushort(p + 10);
        // biXPelsPerMeter is an unsigned long.
        xdpi = (ulong(p + 20) as u64) as f64 * 0.0254;
        ydpi = (ulong(p + 24) as u64) as f64 * 0.0254;
        height = h.wrapping_abs() as i64;
        psize = 4;
    } else {
        return Bmp::UnknownHeader;
    }
    let num_palette = if bit_count < 24 {
        if bit_count != 1 && bit_count != 4 && bit_count != 8 {
            return Bmp::Error;
        }
        (offset - hsize - 14) / psize
    } else if bit_count == 24 {
        1
    } else {
        return Bmp::Error;
    };
    if width == 0 || height == 0 || num_palette < 1 {
        return Bmp::Error;
    }
    Bmp::Size(PixelSize {
        width: width as f64,
        height: height as f64,
        xdpi,
        ydpi,
    })
}

// ---- PDF (pdfimage.cpp over pplib) ----------------------------------------

/// `ppdict_get_rect`'s value: an array (direct, or through one reference)
/// of four direct numbers (`pparray_to_rect`).
fn rect_of(doc: &xpdf::Doc, o: xpdf::Obj) -> Option<[f64; 4]> {
    let a = if o.is_ref() { o.fetch(doc) } else { o };
    if !a.is_array() || a.array_len() != 4 {
        return None;
    }
    let mut r = [0.0; 4];
    for (i, v) in r.iter_mut().enumerate() {
        let e = a.array_get_nf(i as i32);
        if !e.is_num() {
            return None;
        }
        *v = e.get_num();
    }
    Some(r)
}

/// `ppdict_get_box`: box `name` of the page dictionary or of its nearest
/// ancestor (`Parent`) that has one.
fn inherited_box(doc: &xpdf::Doc, page: &xpdf::Obj, name: &[u8]) -> Option<[f64; 4]> {
    if let Some(r) = rect_of(doc, page.dict_lookup_nf(name)) {
        return Some(r);
    }
    let mut dict = page.dict_lookup(b"Parent");
    // A page tree deeper than this is a cycle.
    for _ in 0..1024 {
        if !dict.is_dict() {
            return None;
        }
        if let Some(r) = rect_of(doc, dict.dict_lookup_nf(name)) {
            return Some(r);
        }
        dict = dict.dict_lookup(b"Parent");
    }
    None
}

/// Opens a PDF file, as `ppdoc_load` does (None if it cannot be read).
fn open_pdf(path: &str) -> Option<xpdf::Doc> {
    if !xpdf::LINKED {
        return None;
    }
    xpdf::init();
    let doc = xpdf::Doc::open(path.as_bytes());
    doc.ok().then_some(doc)
}

/// `pdf_get_rect`: box `pdf_box` of page `page_num` (clamped to the
/// document; negative counts from the end), in TeX points: x, y, width,
/// height. None for the C's -1.
pub fn pdf_rect(path: &str, page_num: i32, pdf_box: i32) -> Option<[f64; 4]> {
    let doc = open_pdf(path)?;
    let pages = doc.num_pages();
    let mut n = page_num;
    if n > pages {
        n = pages;
    }
    if n < 0 {
        n += pages + 1;
    }
    if n < 1 {
        n = 1;
    }
    let (num, gen) = doc.page_ref(n)?;
    let page = doc.fetch(num, gen);
    if !page.is_dict() {
        return None;
    }
    let first: &[u8] = match pdf_box {
        PDFBOX_MEDIA => b"MediaBox",
        PDFBOX_BLEED => b"BleedBox",
        PDFBOX_TRIM => b"TrimBox",
        PDFBOX_ART => b"ArtBox",
        _ => b"CropBox",
    };
    let names: [&[u8]; 6] = [
        first,
        b"CropBox",
        b"MediaBox",
        b"BleedBox",
        b"TrimBox",
        b"ArtBox",
    ];
    let [lx, ly, rx, ry] = names
        .iter()
        .find_map(|name| inherited_box(&doc, &page, name))?;
    // ppdict_get_int: the page's own Rotate, a direct integer.
    let rot = page.dict_lookup_nf(b"Rotate");
    let mut angle = if rot.is_int() { rot.get_int() } else { 0 } % 360;
    if angle < 0 {
        angle += 360;
    }
    let k = 72.27 / 72.0;
    let (wd, ht) = if angle == 90 || angle == 270 {
        (k * (ry - ly).abs(), k * (rx - lx).abs())
    } else {
        (k * (rx - lx).abs(), k * (ry - ly).abs())
    };
    // `my_fmin`.
    let fmin = |x: f64, y: f64| if x < y { x } else { y };
    Some([k * fmin(lx, rx), k * fmin(ly, ry), wd, ht])
}

/// A page of a PDF file as the output includes it (`pdf:image`): the page
/// number (1-based, clamped as [`pdf_rect`] clamps it), the selected box
/// in bp as the file has it (`[llx, lly, urx, ury]`, normalised), and the
/// page's own `/Rotate` (0, 90, 180 or 270).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PdfPage {
    pub page: i32,
    pub bbox: [f64; 4],
    pub rotate: i32,
}

/// [`pdf_rect`]'s page and box, in bp, for the output.
pub fn pdf_page(path: &str, page_num: i32, pdf_box: i32) -> Option<PdfPage> {
    let doc = open_pdf(path)?;
    let pages = doc.num_pages();
    let mut n = page_num;
    if n > pages {
        n = pages;
    }
    if n < 0 {
        n += pages + 1;
    }
    if n < 1 {
        n = 1;
    }
    let (num, gen) = doc.page_ref(n)?;
    let page = doc.fetch(num, gen);
    if !page.is_dict() {
        return None;
    }
    let first: &[u8] = match pdf_box {
        PDFBOX_MEDIA => b"MediaBox",
        PDFBOX_BLEED => b"BleedBox",
        PDFBOX_TRIM => b"TrimBox",
        PDFBOX_ART => b"ArtBox",
        _ => b"CropBox",
    };
    let names: [&[u8]; 6] = [
        first,
        b"CropBox",
        b"MediaBox",
        b"BleedBox",
        b"TrimBox",
        b"ArtBox",
    ];
    let [lx, ly, rx, ry] = names
        .iter()
        .find_map(|name| inherited_box(&doc, &page, name))?;
    let rot = page.dict_lookup_nf(b"Rotate");
    let mut angle = if rot.is_int() { rot.get_int() } else { 0 } % 360;
    if angle < 0 {
        angle += 360;
    }
    Some(PdfPage {
        page: n,
        bbox: [lx.min(rx), ly.min(ry), lx.max(rx), ly.max(ry)],
        rotate: angle,
    })
}

/// `pdf_count_pages`.
pub fn pdf_pages(path: &str) -> i32 {
    open_pdf(path).map_or(0, |d| d.num_pages())
}

/// The bounds of the picture file at `path` (`find_pic_file` after the
/// lookup): a PDF page's box when `pdf_box_type` is not 0, else a JPEG,
/// BMP or PNG image's size at its origin. None for the C's -1.
pub fn pic_bounds(path: &str, pdf_box_type: i32, page: i32) -> Option<[f64; 4]> {
    if pdf_box_type != 0 {
        return pdf_rect(path, page, pdf_box_type);
    }
    let data = std::fs::read(path).ok()?;
    let size = if is_jpeg(&data) {
        jpeg_size(&data)
    } else if is_bmp(&data) {
        match bmp_size(&data) {
            Bmp::Size(s) => Some(s),
            Bmp::Error => None,
            Bmp::UnknownHeader => {
                eprintln!("Unknown BMP header type.");
                std::process::exit(1);
            }
        }
    } else if is_png(&data) {
        png_size(&data)
    } else {
        None
    }?;
    let (wd, ht) = size.bounds();
    Some([0.0, 0.0, wd, ht])
}

impl Globals {
    /// `find_pic_file`: `name_of_file` looked up as a picture
    /// (`kpse_pict_format`) and its bounds in TeX points; 0 and the path's
    /// handle on success, else -1 and no path.
    pub fn find_pic_file(
        &mut self,
        path: &mut i32,
        bounds: &mut real_rect,
        pdf_box_type: i32,
        page: i32,
    ) -> i32 {
        *path = 0;
        *bounds = real_rect::default();
        let name = self.raw_file_name();
        let Some(found) = flashtex_engine::system::find_file(&name, Format::Pict) else {
            return -1;
        };
        let Some([x, y, wd, ht]) = pic_bounds(&found, pdf_box_type, page) else {
            return -1;
        };
        if !self.host.no_pdf.0 {
            self.host.pictures.insert(found.clone());
        }
        *bounds = real_rect {
            x: round_f32(x),
            y: round_f32(y),
            wd: round_f32(wd),
            ht: round_f32(ht),
        };
        *path = self
            .host
            .handles
            .alloc(Object::PicPath(Arc::from(found.as_bytes())));
        0
    }

    /// `strlen(pic_path)`.
    pub fn pic_path_len(&mut self, path: i32) -> i32 {
        match self.host.handles.get(path) {
            Some(Object::PicPath(p)) => p.len() as i32,
            _ => 0,
        }
    }

    /// `memcpy(&mem[p], pic_path, strlen(pic_path)); free(pic_path)`: the
    /// path's bytes into the words from `p` on, eight to a word in
    /// little-endian order (`pic_path_byte` reads them back; the rest of
    /// the last word is 0), and the handle freed.
    pub fn pic_path_to_mem(&mut self, path: i32, p: i32) {
        let bytes = match self.host.handles.get(path) {
            Some(Object::PicPath(b)) => b.clone(),
            _ => return,
        };
        for (k, chunk) in bytes.chunks(8).enumerate() {
            let mut w = [0u8; 8];
            w[..chunk.len()].copy_from_slice(chunk);
            self.mem[p as usize + k] = memory_word::from_bits(u64::from_le_bytes(w));
        }
        self.host.handles.free(path);
    }

    /// `countpdffilepages`: the pages of the PDF file `name_of_file` names
    /// (0 if it is not found or not read).
    pub fn count_pdf_file_pages(&mut self) -> i32 {
        let name = self.raw_file_name();
        match flashtex_engine::system::find_file(&name, Format::Pict) {
            Some(found) => pdf_pages(&found),
            None => 0,
        }
    }

    /// `setPoint`: into a C `realpoint`, of `float`s.
    #[allow(non_snake_case)]
    pub fn setPoint(&mut self, p: &mut real_point, x: f64, y: f64) {
        p.x = round_f32(x);
        p.y = round_f32(y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        for (kind, body) in chunks {
            v.extend_from_slice(&(body.len() as u32).to_be_bytes());
            v.extend_from_slice(*kind);
            v.extend_from_slice(body);
            v.extend_from_slice(&[0; 4]);
        }
        v
    }

    fn ihdr(w: u32, h: u32) -> Vec<u8> {
        let mut b = w.to_be_bytes().to_vec();
        b.extend_from_slice(&h.to_be_bytes());
        b.extend_from_slice(&[8, 2, 0, 0, 0]);
        b
    }

    fn phys(x: u32, y: u32, unit: u8) -> Vec<u8> {
        let mut b = x.to_be_bytes().to_vec();
        b.extend_from_slice(&y.to_be_bytes());
        b.push(unit);
        b
    }

    #[test]
    fn png_resolution() {
        let plain = png(&[(b"IHDR", ihdr(10, 20)), (b"IDAT", vec![])]);
        assert_eq!(png_size(&plain).unwrap().xdpi, 72.0);
        let metres = png(&[
            (b"IHDR", ihdr(10, 20)),
            (b"pHYs", phys(11811, 5906, 1)),
            (b"IDAT", vec![]),
        ]);
        let s = png_size(&metres).unwrap();
        assert_eq!((s.width, s.height), (10.0, 20.0));
        assert_eq!(s.xdpi, 11811.0 * 0.0254);
        let unknown_unit = png(&[
            (b"IHDR", ihdr(10, 20)),
            (b"pHYs", phys(11811, 5906, 0)),
            (b"IDAT", vec![]),
        ]);
        assert_eq!(png_size(&unknown_unit).unwrap().ydpi, 72.0);
        let after_idat = png(&[
            (b"IHDR", ihdr(10, 20)),
            (b"IDAT", vec![]),
            (b"pHYs", phys(11811, 5906, 1)),
        ]);
        assert_eq!(png_size(&after_idat).unwrap().xdpi, 72.0);
    }

    #[test]
    fn jpeg_jfif_and_exif() {
        // SOI, APP0 JFIF at 2 dots per cm... (units 2), SOF0 3x5.
        let mut j = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16];
        j.extend_from_slice(b"JFIF\0");
        j.extend_from_slice(&[1, 2, 2, 0, 100, 0, 50, 0, 0]);
        j.extend_from_slice(&[0xFF, 0xC0, 0, 11, 8, 0, 5, 0, 3, 3, 0, 0, 0]);
        let s = jpeg_size(&j).unwrap();
        assert_eq!(
            (s.width, s.height, s.xdpi, s.ydpi),
            (3.0, 5.0, 254.0, 127.0)
        );
        // Exif (big-endian) with XResolution 300/1 and unit inches.
        let mut tiff = b"MM\0\x2a\0\0\0\x08".to_vec();
        tiff.extend_from_slice(&[0, 2]);
        tiff.extend_from_slice(&[0x01, 0x1A, 0, 5, 0, 0, 0, 1, 0, 0, 0, 38]);
        tiff.extend_from_slice(&[0x01, 0x28, 0, 3, 0, 0, 0, 1, 0, 2, 0, 0]);
        tiff.extend_from_slice(&[0, 0, 0, 0]);
        tiff.extend_from_slice(&[0, 0, 1, 44, 0, 0, 0, 1]);
        let mut e = vec![0xFF, 0xD8, 0xFF, 0xE1];
        e.extend_from_slice(&((tiff.len() + 2 + 6) as u16).to_be_bytes());
        e.extend_from_slice(b"Exif\0\0");
        e.extend_from_slice(&tiff);
        e.extend_from_slice(&[0xFF, 0xC0, 0, 11, 8, 0, 5, 0, 3, 3, 0, 0, 0]);
        let s = jpeg_size(&e).unwrap();
        assert_eq!((s.xdpi, s.ydpi), (300.0, 72.0));
        assert!(jpeg_size(&[0xFF, 0xD8, 0xFF, 0xD9]).is_none());
    }

    #[test]
    fn bmp_headers() {
        let mut b = b"BM".to_vec();
        b.extend_from_slice(&[0; 4]);
        b.extend_from_slice(&[0; 4]);
        b.extend_from_slice(&54u32.to_le_bytes());
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&7i32.to_le_bytes());
        b.extend_from_slice(&(-9i32).to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&[0; 8]);
        b.extend_from_slice(&2835u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&[0; 8]);
        match bmp_size(&b) {
            Bmp::Size(s) => {
                assert_eq!((s.width, s.height), (7.0, 9.0));
                assert_eq!((s.xdpi, s.ydpi), (2835.0 * 0.0254, 0.0));
            }
            other => panic!("{other:?}"),
        }
        // A header size it does not know, read in full: XeTeX exits.
        b[14] = 41;
        assert_eq!(bmp_size(&b), Bmp::Error);
        b.push(0);
        assert_eq!(bmp_size(&b), Bmp::UnknownHeader);
    }
}
