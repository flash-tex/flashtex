//! `writejpg.c`, ported: JPEG images, copied unchanged into a
//! `/DCTDecode` stream. Only the markers are read: the resolution (JFIF or
//! Exif) and the frame header (size, bits, colour space).

use super::cfile::{CFile, Whence};
use super::images::{ImageData, ImageEntry, IMAGE_COLOR_B, IMAGE_COLOR_C};
use crate::generated::Globals;

/// Gray color space, use /DeviceGray
const JPG_GRAY: i32 = 1;
/// RGB color space, use /DeviceRGB
const JPG_RGB: i32 = 3;
/// CMYK color space, use /DeviceCMYK
const JPG_CMYK: i32 = 4;

/// `JPG_IMAGE_INFO`.
#[derive(Clone)]
pub struct JpgImage {
    pub color_space: i32,
    pub bits_per_component: i32,
    /// `length`: of the file.
    pub length: u64,
    pub file: CFile,
}

/// `read2bytes`: a big-endian pair (EOF reads as -1 in each byte, as in C).
fn read2bytes(f: &mut CFile) -> i32 {
    let c = f.getc();
    (c << 8) + f.getc()
}

/// `get_unsigned_pair`: a big-endian pair of `unsigned char`s.
fn get_unsigned_pair(f: &mut CFile) -> u16 {
    let a = f.getc() as u8 as u16;
    let b = f.getc() as u8 as u16;
    (a << 8) | b
}

/// `read_exif_bytes`: an `n`-byte integer at `*p`, big-endian if `b`. The
/// reads stay inside `buf` (C would read beyond a truncated Exif block;
/// here those bytes read as 0).
fn read_exif_bytes(buf: &[u8], p: &mut usize, n: usize, b: bool) -> u32 {
    let at = |i: usize| buf.get(i).copied().unwrap_or(0) as u32;
    let mut rval = 0u32;
    if b {
        for k in 0..n {
            rval = (rval << 8).wrapping_add(at(p.wrapping_add(k)));
        }
    } else {
        for k in (0..n).rev() {
            rval = (rval << 8).wrapping_add(at(p.wrapping_add(k)));
        }
    }
    *p = p.wrapping_add(n);
    rval
}

/// `read_APP1_Exif`: the resolution tags of an Exif block, in dots per
/// inch: (x, y), or `None` where C gives up (`goto err`).
fn read_app1_exif(fp: &mut CFile, length: u16) -> Option<(i32, i32)> {
    let mut buffer = vec![0u8; length as usize];
    let got = fp.read(&mut buffer);
    // fread into a malloc'ed buffer: the unread rest is indeterminate in
    // C; zeros here
    buffer[got..].iter_mut().for_each(|b| *b = 0);
    let len = length as usize;
    let mut p = 0usize;
    while p < len && buffer[p] == 0 {
        p += 1;
    }
    let tiff_header = p;
    let at = |i: usize| buffer.get(i).copied().unwrap_or(0);
    let bigendian = if at(p) == b'M' && at(p + 1) == b'M' {
        true
    } else if at(p) == b'I' && at(p + 1) == b'I' {
        false
    } else {
        return None;
    };
    p = p.wrapping_add(2);
    let i = read_exif_bytes(&buffer, &mut p, 2, bigendian);
    if i != 42 {
        return None;
    }
    let i = read_exif_bytes(&buffer, &mut p, 4, bigendian);
    p = tiff_header.wrapping_add(i as usize);
    let mut num_fields = read_exif_bytes(&buffer, &mut p, 2, bigendian) as i32;
    let (mut value, mut num, mut den) = (0i32, 0i32, 0i32);
    let (mut xres, mut yres, mut res_unit) = (72.0f64, 72.0f64, 1.0f64);
    while num_fields > 0 {
        num_fields -= 1;
        let tag = read_exif_bytes(&buffer, &mut p, 2, bigendian) as i32;
        let typ = read_exif_bytes(&buffer, &mut p, 2, bigendian) as i32;
        read_exif_bytes(&buffer, &mut p, 4, bigendian);
        match typ {
            1 | 7 => {
                // byte, undefined
                value = at(p) as i32;
                p = p.wrapping_add(4);
            }
            3 => {
                // short
                value = read_exif_bytes(&buffer, &mut p, 2, bigendian) as i32;
                p = p.wrapping_add(2);
            }
            4 | 9 => {
                // long, slong
                value = read_exif_bytes(&buffer, &mut p, 4, bigendian) as i32;
            }
            5 | 10 => {
                // rational, srational
                value = read_exif_bytes(&buffer, &mut p, 4, bigendian) as i32;
                let mut rp = tiff_header.wrapping_add(value as u32 as usize);
                num = read_exif_bytes(&buffer, &mut rp, 4, bigendian) as i32;
                den = read_exif_bytes(&buffer, &mut rp, 4, bigendian) as i32;
            }
            _ => {
                // ascii and the rest
                p = p.wrapping_add(4);
            }
        }
        match tag {
            282 => {
                // x res (an integer division, as in C)
                if den != 0 {
                    xres = num.wrapping_div(den) as f64;
                }
            }
            283 => {
                // y res
                if den != 0 {
                    yres = num.wrapping_div(den) as f64;
                }
            }
            296 => {
                // res unit
                match value {
                    2 => res_unit = 1.0,
                    3 => res_unit = 2.54,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    Some(((xres * res_unit) as i32, (yres * res_unit) as i32))
}

impl Globals {
    /// `xfseek` (kpathsea): a seek that stops the run if it fails.
    pub(crate) fn xfseek(&mut self, f: &mut CFile, off: i64, whence: Whence, name: &[u8]) {
        if !f.seek(off, whence) {
            self.fatal_perror_msg(name, "Invalid argument");
        }
    }

    /// kpathsea's `FATAL_PERROR` with a given reason.
    pub(crate) fn fatal_perror_msg(&mut self, name: &[u8], reason: &str) -> ! {
        eprintln!(
            "{}: {}: {}",
            crate::system::invocation_name(),
            String::from_utf8_lossy(name),
            reason
        );
        use crate::system::PasFile;
        self.log_file.flush();
        self.term_out.flush();
        std::process::exit(1)
    }

    /// `read_jpg_info`: size, depth, colour space and resolution.
    pub(crate) fn read_jpg_info(&mut self, e: &mut ImageEntry) {
        let name = e.name.clone().unwrap_or_default();
        e.x_res = 0;
        e.y_res = 0;
        let Some(mut f) = CFile::open(&name) else {
            self.fatal_perror(&name);
        };
        let file_len = f.len();
        if read2bytes(&mut f) != 0xFFD8 {
            self.pdftex_fail("reading JPEG image failed (no JPEG header found)");
        }
        // currently JFIF and Exif files allow extracting img_xres and
        // img_yres
        let appmk = read2bytes(&mut f);
        if appmk == 0xFFE0 {
            // check for JFIF
            let _ = read2bytes(&mut f);
            let jpg_id = b"JFIF\0";
            let mut i = 0;
            while i < 5 {
                if f.getc() != jpg_id[i] as i32 {
                    break;
                }
                i += 1;
            }
            if i == 5 {
                // it's JFIF
                read2bytes(&mut f);
                let units = f.getc();
                e.x_res = read2bytes(&mut f);
                e.y_res = read2bytes(&mut f);
                match units {
                    1 => {} // pixels per inch
                    2 => {
                        // pixels per cm
                        e.x_res = (e.x_res as f64 * 2.54) as i32;
                        e.y_res = (e.y_res as f64 * 2.54) as i32;
                    }
                    _ => {
                        e.x_res = 0;
                        e.y_res = 0;
                    }
                }
            }
            // if either xres or yres is 0 but the other isn't, set it to
            // the value of the other
            if e.x_res == 0 && e.y_res != 0 {
                e.x_res = e.y_res;
            }
            if e.y_res == 0 && e.x_res != 0 {
                e.y_res = e.x_res;
            }
        } else if appmk == 0xFFE1 {
            // check for Exif
            let (mut xxres, mut yyres) = (0, 0);
            let mut length = get_unsigned_pair(&mut f).wrapping_sub(2);
            if length > 5 {
                let mut app_sig = [0u8; 5];
                if f.read(&mut app_sig) != 5 {
                    // C returns here, with the file and the rest unset
                    e.data = ImageData::Jpg(JpgImage {
                        color_space: 0,
                        bits_per_component: 0,
                        length: file_len,
                        file: f,
                    });
                    return;
                }
                length -= 5;
                if &app_sig == b"Exif\0" {
                    if let Some((x, y)) = read_app1_exif(&mut f, length) {
                        xxres = x;
                        yyres = y;
                    }
                }
            }
            e.x_res = xxres;
            e.y_res = yyres;
        }
        self.xfseek(&mut f, 0, Whence::Set, &name);
        loop {
            if f.feof() {
                self.pdftex_fail("reading JPEG image failed (premature file end)");
            }
            if f.getc() != 0xFF {
                self.pdftex_fail("reading JPEG image failed (no marker found)");
            }
            match f.getc() {
                // M_SOF5..M_SOF7, M_SOF9..M_SOF11, M_SOF13..M_SOF15
                0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf => {
                    self.pdftex_fail("unsupported type of compression");
                }
                m @ 0xc0..=0xc3 => {
                    if m == 0xc2
                        && self.fixed_pdf_major_version == 1
                        && self.fixed_pdf_minor_version <= 2
                    {
                        self.pdftex_fail("cannot use progressive DCT with PDF-1.2");
                    }
                    let _ = read2bytes(&mut f); // read segment length
                    let bits_per_component = f.getc() as u8 as i32;
                    e.height = read2bytes(&mut f);
                    e.width = read2bytes(&mut f);
                    let color_space = f.getc();
                    self.xfseek(&mut f, 0, Whence::Set, &name);
                    e.color_type = match color_space {
                        JPG_GRAY => IMAGE_COLOR_B,
                        JPG_RGB | JPG_CMYK => IMAGE_COLOR_C,
                        _ => self.pdftex_fail(&format!("Unsupported color space {color_space}")),
                    };
                    e.data = ImageData::Jpg(JpgImage {
                        color_space,
                        bits_per_component,
                        length: file_len,
                        file: f,
                    });
                    return;
                }
                // ignore markers without parameters: M_SOI, M_EOI, M_TEM,
                // M_RST0..M_RST7
                0xd8 | 0xd9 | 0x01 | 0xd0..=0xd7 => {}
                _ => {
                    // skip variable length markers: C passes the
                    // unsigned `read2bytes(f) - 2` as a long, so a length
                    // below 2 (or EOF) seeks far ahead, never back
                    let skip = (read2bytes(&mut f) as u32).wrapping_sub(2);
                    self.xfseek(&mut f, skip as i64, Whence::Cur, &name);
                }
            }
        }
    }

    /// `write_jpg`: the XObject dictionary and the file as its stream.
    pub(crate) fn write_jpg(&mut self, e: &mut ImageEntry) {
        let (w, h, cs_ref) = (e.width, e.height, e.colorspace_ref);
        let ImageData::Jpg(j) = &mut e.data else {
            self.pdftex_fail("unknown type of image");
        };
        self.pdf_puts(b"/Type /XObject\n/Subtype /Image\n");
        self.pdf_printf(
            format!(
                "/Width {w}\n/Height {h}\n/BitsPerComponent {}\n/Length {}\n",
                j.bits_per_component, j.length as i32
            )
            .as_bytes(),
        );
        self.pdf_puts(b"/ColorSpace ");
        if cs_ref != 0 {
            self.pdf_printf(format!("{cs_ref} 0 R\n").as_bytes());
        } else {
            match j.color_space {
                JPG_GRAY => self.pdf_puts(b"/DeviceGray\n"),
                JPG_RGB => self.pdf_puts(b"/DeviceRGB\n"),
                JPG_CMYK => self.pdf_puts(b"/DeviceCMYK\n/Decode [1 0 1 0 1 0 1 0]\n"),
                cs => self.pdftex_fail(&format!("Unsupported color space {cs}")),
            }
        }
        self.pdf_puts(b"/Filter /DCTDecode\n>>\nstream\n");
        // `pdfout(xgetc(f))` for each of the file's bytes (from the start:
        // read_jpg_info left the file there); a byte past the end reads as
        // EOF, i.e. 0xFF
        let mut bytes = vec![0u8; j.length as usize];
        let got = j.file.read(&mut bytes);
        bytes[got..].iter_mut().for_each(|b| *b = 0xFF);
        self.c_pdf_out_bytes(&bytes);
        self.pdf_end_stream();
    }
}
