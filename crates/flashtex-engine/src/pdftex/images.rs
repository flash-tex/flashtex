//! `writeimg.c`, ported: the image table behind `\pdfximage`, image type
//! detection, and the dispatch to the readers and writers of each type:
//! [`super::writepng`] (libpng), [`super::writejpg`],
//! [`super::writejbig2`] and [`super::pdftoepdf`] (xpdf).
//!
//! An image is a number (pdftex.web keeps it in `obj_ximage_data`): its
//! index in [`State::images`].

use super::pdftoepdf;
use super::writejbig2::Jbig2Image;
use super::writejpg::JpgImage;
use super::writepng::PngImage;
use crate::generated::Globals;

pub const IMAGE_TYPE_NONE: i32 = 0;
pub const IMAGE_TYPE_PDF: i32 = 1;
pub const IMAGE_TYPE_PNG: i32 = 2;
pub const IMAGE_TYPE_JPG: i32 = 3;
pub const IMAGE_TYPE_JBIG2: i32 = 5;

pub const IMAGE_COLOR_B: i32 = 1;
pub const IMAGE_COLOR_C: i32 = 2;
pub const IMAGE_COLOR_I: i32 = 4;

/// `SMALL_BUF_SIZE` (ptexmac.h): the first size of the image array.
const SMALL_BUF_SIZE: i32 = 256;

/// `pdf_image_struct`.
pub struct PdfImage {
    pub orig_x: i32,
    pub orig_y: i32,
    pub selected_page: i32,
    pub page_box: i32,
    /// `doc`: the document's handle in [`pdftoepdf::State`].
    pub doc: usize,
}

/// `image_struct`, the part that depends on the type.
#[derive(Default)]
pub enum ImageData {
    #[default]
    None,
    Pdf(PdfImage),
    Png(PngImage),
    Jpg(JpgImage),
    Jbig2(Jbig2Image),
}

/// `image_entry`.
#[derive(Default)]
pub struct ImageEntry {
    /// `image_name`: the file found (`NULL` once freed).
    pub name: Option<Vec<u8>>,
    pub image_type: i32,
    pub color_type: i32,
    pub width: i32,
    pub height: i32,
    pub rotate: i32,
    pub x_res: i32,
    pub y_res: i32,
    pub num_pages: i32,
    pub colorspace_ref: i32,
    /// `group_ref`: if it's <= 0, the page has no group.
    pub group_ref: i32,
    pub data: ImageData,
}

#[derive(Default)]
pub struct State {
    /// `image_array` up to `image_ptr`.
    pub images: Vec<ImageEntry>,
    /// `image_limit` (dumped with the format).
    pub image_limit: i32,
    /// `image_array != NULL`.
    allocated: bool,
    pub png: super::writepng::State,
    pub jbig2: super::writejbig2::State,
    pub epdf: pdftoepdf::State,
}

/// `strcasecmp(a, b) == 0` for ASCII.
fn eq_ignore_case(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

impl Globals {
    /// Run `f` with the image state (taken out of the thread's C state for
    /// the call, like [`Globals::with_fonts`]).
    pub(crate) fn with_images<R>(&mut self, f: impl FnOnce(&mut Globals, &mut State) -> R) -> R {
        let mut st = super::with_state(|s| std::mem::take(&mut s.img));
        let r = f(self, &mut st);
        super::with_state(|s| s.img = st);
        r
    }

    /// A field of image `img` (0 for a number that is no image, where C
    /// would read outside the array).
    fn img_get(&mut self, img: i32, f: impl FnOnce(&ImageEntry) -> i32) -> i32 {
        super::with_state(|s| {
            usize::try_from(img)
                .ok()
                .and_then(|i| s.img.images.get(i))
                .map_or(0, f)
        })
    }

    /// `new_image_entry`, with `alloc_array(image, 1, SMALL_BUF_SIZE)`'s
    /// growth of `image_limit`.
    fn new_image_entry(st: &mut State) -> i32 {
        let used = st.images.len() as i32;
        if !st.allocated {
            st.image_limit = SMALL_BUF_SIZE.max(1);
            st.allocated = true;
        } else if used + 1 > st.image_limit {
            st.image_limit = st.image_limit.wrapping_mul(2);
            if used + 1 > st.image_limit {
                st.image_limit = used + 1;
            }
        }
        st.images.push(ImageEntry::default());
        used
    }

    pub fn image_width(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| e.width)
    }
    pub fn image_height(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| e.height)
    }
    pub fn image_rotate(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| e.rotate)
    }
    pub fn image_x_res(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| e.x_res)
    }
    pub fn image_y_res(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| e.y_res)
    }
    pub fn image_pages(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| e.num_pages)
    }
    pub fn is_pdf_image(&mut self, img: i32) -> bool {
        self.img_get(img, |e| e.image_type) == IMAGE_TYPE_PDF
    }
    pub fn is_png_image(&mut self, img: i32) -> bool {
        self.img_get(img, |e| e.image_type) == IMAGE_TYPE_PNG
    }
    pub fn check_image_b(&mut self, procset: i32) -> bool {
        procset & IMAGE_COLOR_B != 0
    }
    pub fn check_image_c(&mut self, procset: i32) -> bool {
        procset & IMAGE_COLOR_C != 0
    }
    pub fn check_image_i(&mut self, procset: i32) -> bool {
        procset & IMAGE_COLOR_I != 0
    }
    pub fn update_image_procset(&mut self, img: i32) {
        let c = self.img_get(img, |e| e.color_type);
        self.pdf_image_procset |= c;
    }
    pub fn epdf_orig_x(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| match &e.data {
            ImageData::Pdf(p) => p.orig_x,
            _ => 0,
        })
    }
    pub fn epdf_orig_y(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| match &e.data {
            ImageData::Pdf(p) => p.orig_y,
            _ => 0,
        })
    }
    pub fn get_image_group_ref(&mut self, img: i32) -> i32 {
        self.img_get(img, |e| e.group_ref)
    }
    pub fn set_image_group_ref(&mut self, img: i32, value: i32) {
        super::with_state(|s| {
            if let Some(e) = usize::try_from(img)
                .ok()
                .and_then(|i| s.img.images.get_mut(i))
            {
                e.group_ref = value;
            }
        });
    }

    /// `imagecolordepth`.
    pub fn image_colordepth(&mut self, img: i32) -> i32 {
        let t = self.img_get(img, |e| e.image_type);
        match t {
            IMAGE_TYPE_PNG => self.with_images(|_, st| match &st.images[img as usize].data {
                ImageData::Png(p) => p.bit_depth(),
                _ => 0,
            }),
            IMAGE_TYPE_JPG => self.img_get(img, |e| match &e.data {
                ImageData::Jpg(j) => j.bits_per_component,
                _ => 0,
            }),
            IMAGE_TYPE_JBIG2 | IMAGE_TYPE_PDF => 0,
            _ => self.pdftex_fail("unknown type of image"),
        }
    }

    /// `checktypebyheader`: the type from the file's first bytes.
    fn check_type_by_header(&mut self, e: &mut ImageEntry) {
        if e.image_type != IMAGE_TYPE_NONE {
            return; // nothing to do
        }
        let name = e.name.clone().unwrap_or_default();
        // read the header
        let Some(mut file) = super::cfile::CFile::open(&name) else {
            self.fatal_perror(&name);
        };
        let mut header = [0u8; 8];
        for h in header.iter_mut() {
            *h = file.getc() as u8;
            if file.feof() {
                self.pdftex_fail("reading image file failed");
            }
        }
        // tests
        e.image_type = if header.starts_with(b"\xFF\xD8") {
            IMAGE_TYPE_JPG
        } else if header.starts_with(b"\x89PNG\r\n\x1A\n") {
            IMAGE_TYPE_PNG
        } else if header.starts_with(b"\x97\x4A\x42\x32\x0D\x0A\x1A\x0A") {
            IMAGE_TYPE_JBIG2
        } else if header.starts_with(b"%PDF-1.") {
            IMAGE_TYPE_PDF
        } else {
            IMAGE_TYPE_NONE
        };
    }

    /// `checktypebyextension`: the type from the file name's extension
    /// (the text after the last `.` of the whole path).
    fn check_type_by_extension(e: &mut ImageEntry, cur_file_name: &[u8]) {
        if e.image_type != IMAGE_TYPE_NONE {
            return; // nothing to do
        }
        let Some(dot) = cur_file_name.iter().rposition(|&c| c == b'.') else {
            return;
        };
        let suffix = &cur_file_name[dot..];
        e.image_type = if eq_ignore_case(suffix, b".png") {
            IMAGE_TYPE_PNG
        } else if eq_ignore_case(suffix, b".jpg") || eq_ignore_case(suffix, b".jpeg") {
            IMAGE_TYPE_JPG
        } else if eq_ignore_case(suffix, b".jbig2") || eq_ignore_case(suffix, b".jb2") {
            IMAGE_TYPE_JBIG2
        } else if eq_ignore_case(suffix, b".pdf") {
            IMAGE_TYPE_PDF
        } else {
            IMAGE_TYPE_NONE
        };
    }

    /// kpathsea's `FATAL_PERROR(name)` (`xfopen` of a file that cannot be
    /// opened): the program name, the file name and the system's reason on
    /// stderr, then exit.
    pub(crate) fn fatal_perror(&mut self, name: &[u8]) -> ! {
        // perror's text is strerror's: io::Error's without " (os error N)"
        let reason = match std::fs::File::open(super::cfile::os_path(name)) {
            Err(e) => {
                let t = e.to_string();
                t.split(" (os error").next().unwrap_or_default().to_string()
            }
            Ok(_) => "Unknown error".to_string(),
        };
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

    /// `bp2int`: big points as a scaled number, rounded.
    fn bp2int(&self, p: f32) -> i32 {
        let r = (p as f64 * (self.one_hundred_bp as f64 / 100.0)).round();
        r as i32
    }

    /// `readimage` (`\pdfximage`): find and read image `s`, and return its
    /// number.
    #[allow(clippy::too_many_arguments)]
    pub fn read_image(
        &mut self,
        s: i32,
        page_num: i32,
        page_name: i32,
        colorspace: i32,
        pagebox: i32,
        pdf_major_version: i32,
        pdf_minor_version: i32,
        pdf_inclusion_errorlevel: i32,
    ) -> i32 {
        self.with_images(|g, st| {
            let img = Self::new_image_entry(st);
            let mut e = std::mem::take(&mut st.images[img as usize]);
            e.colorspace_ref = colorspace;
            let dest = (page_name != 0).then(|| g.c_string(page_name));
            let found = g.find_input_file(s).map(|p| p.into_bytes());
            super::output::set_cur_file_name(found.as_deref());
            let Some(name) = found else {
                let n = String::from_utf8_lossy(&g.c_string(s)).into_owned();
                g.pdftex_fail(&format!("cannot find image file {n}"));
            };
            e.name = Some(name.clone());
            // type checks
            g.check_type_by_header(&mut e);
            Self::check_type_by_extension(&mut e, &name);
            // read image
            match e.image_type {
                IMAGE_TYPE_PDF => {
                    let info = g.read_pdf_info(
                        &mut st.epdf,
                        &name,
                        dest.as_deref(),
                        page_num,
                        pagebox,
                        pdf_major_version,
                        pdf_minor_version,
                        pdf_inclusion_errorlevel,
                    );
                    e.width = g.bp2int(info.width);
                    e.height = g.bp2int(info.height);
                    e.rotate = info.rotate as i32;
                    e.num_pages = info.num_pages;
                    e.data = ImageData::Pdf(PdfImage {
                        orig_x: g.bp2int(info.orig_x),
                        orig_y: g.bp2int(info.orig_y),
                        selected_page: info.page_num,
                        page_box: pagebox,
                        doc: info.doc,
                    });
                    // page group present, but new object number not set
                    // yet; or no page group
                    e.group_ref = if info.has_page_group { -1 } else { 0 };
                }
                IMAGE_TYPE_PNG => {
                    e.num_pages = 1;
                    g.read_png_info(&mut st.png, &mut e);
                }
                IMAGE_TYPE_JPG => {
                    e.num_pages = 1;
                    g.read_jpg_info(&mut e);
                }
                IMAGE_TYPE_JBIG2 => {
                    if pdf_major_version == 1 && pdf_minor_version < 4 {
                        g.pdftex_fail(&format!(
                            "JBIG2 images only possible with at least PDF 1.4; \
                             you are generating PDF 1.{pdf_minor_version}"
                        ));
                    }
                    e.data = ImageData::Jbig2(Jbig2Image {
                        selected_page: page_num,
                    });
                    g.read_jbig2_info(&mut st.jbig2, &mut e);
                }
                _ => g.pdftex_fail("unknown type of image"),
            }
            st.images[img as usize] = e;
            super::output::set_cur_file_name(None);
            img
        })
    }

    /// `writeimage`: the image's XObject, after the dictionary pdftex.web
    /// has begun.
    pub fn write_image(&mut self, img: i32) {
        self.with_images(|g, st| {
            let Some(mut e) = usize::try_from(img)
                .ok()
                .and_then(|i| st.images.get_mut(i))
                .map(std::mem::take)
            else {
                g.pdftex_fail("unknown type of image");
            };
            let name = e.name.clone().unwrap_or_default();
            super::output::set_cur_file_name(Some(&name));
            let mut s = b" <".to_vec();
            s.extend_from_slice(&name);
            g.tex_printf(&s);
            match e.image_type {
                IMAGE_TYPE_PNG => g.write_png(&mut st.png, &mut e),
                IMAGE_TYPE_JPG => g.write_jpg(&mut e),
                IMAGE_TYPE_JBIG2 => g.write_jbig2(&mut st.jbig2, &mut e),
                IMAGE_TYPE_PDF => {
                    let (doc, page, page_box) = match &e.data {
                        ImageData::Pdf(p) => (p.doc, p.selected_page, p.page_box),
                        _ => g.pdftex_fail("unknown type of image"),
                    };
                    g.write_epdf(&mut st.epdf, doc, page, page_box);
                }
                _ => g.pdftex_fail("unknown type of image"),
            }
            g.tex_printf(b">");
            super::output::set_cur_file_name(None);
            st.images[img as usize] = e;
        })
    }

    /// `deleteimage`: free what reading the image holds (not in INITEX,
    /// where the image may be dumped into the format).
    pub fn delete_image(&mut self, img: i32) {
        if self.ini_version() {
            return; // The image may be \dump{}ed to a format
        }
        self.with_images(|g, st| {
            let Some(e) = usize::try_from(img).ok().and_then(|i| st.images.get_mut(i)) else {
                g.pdftex_fail("unknown type of image");
            };
            match &e.data {
                ImageData::Pdf(p) => {
                    let doc = p.doc;
                    g.epdf_delete(&mut st.epdf, doc);
                }
                // closing the file (and destroying libpng's structures)
                ImageData::Png(_) | ImageData::Jpg(_) => e.data = ImageData::None,
                ImageData::Jbig2(_) => {}
                ImageData::None => g.pdftex_fail("unknown type of image"),
            }
            e.name = None;
        })
    }

    /// `img_free`.
    pub fn img_free(&mut self) {
        super::with_state(|s| s.img.images = Vec::new());
    }

    /// `dumpimagemeta`: the image table into the format, so that
    /// `\pdfrefximage` works in boxes saved by INITEX. The files are read
    /// again when the format is loaded.
    pub fn dumpimagemeta(&mut self) {
        let (limit, entries) = super::with_state(|s| {
            let e: Vec<_> = s
                .img
                .images
                .iter()
                .map(|e| {
                    let extra: Vec<i32> = match &e.data {
                        ImageData::Pdf(p) => vec![p.page_box, p.selected_page],
                        ImageData::Jbig2(j) => vec![j.selected_page],
                        _ => vec![],
                    };
                    (
                        e.name.clone(),
                        [
                            e.image_type,
                            e.color_type,
                            e.width,
                            e.height,
                            e.x_res,
                            e.y_res,
                            e.num_pages,
                            e.colorspace_ref,
                            e.group_ref,
                        ],
                        extra,
                    )
                })
                .collect();
            (s.img.image_limit, e)
        });
        self.fmt_dump_int(limit);
        self.fmt_dump_int(entries.len() as i32);
        for (name, ints, extra) in entries {
            match name {
                Some(n) => self.fmt_dump_chars(&n),
                None => self.fmt_dump_int(0),
            }
            for v in ints.into_iter().chain(extra) {
                self.fmt_dump_int(v);
            }
        }
    }

    /// `undumpimagemeta`: the image table from the format; every image is
    /// found and read again (with the PDF version and inclusion error
    /// level now in force).
    pub fn undumpimagemeta(
        &mut self,
        pdf_major_version: i32,
        pdf_minor_version: i32,
        errorlevel: i32,
    ) {
        let limit = self.fmt_undump_int();
        let cur_image = self.fmt_undump_int();
        self.with_images(|g, st| {
            st.image_limit = limit;
            st.allocated = true;
            st.images = (0..cur_image.max(0))
                .map(|_| ImageEntry::default())
                .collect();
            for img in 0..cur_image.max(0) as usize {
                let mut e = ImageEntry {
                    name: g.fmt_undump_chars(),
                    ..Default::default()
                };
                e.image_type = g.fmt_undump_int();
                e.color_type = g.fmt_undump_int();
                e.width = g.fmt_undump_int();
                e.height = g.fmt_undump_int();
                e.x_res = g.fmt_undump_int();
                e.y_res = g.fmt_undump_int();
                e.num_pages = g.fmt_undump_int();
                e.colorspace_ref = g.fmt_undump_int();
                e.group_ref = g.fmt_undump_int();
                let name = e.name.clone().unwrap_or_default();
                let n = String::from_utf8_lossy(&name).into_owned();
                if crate::system::find_input(&n).is_none() {
                    g.pdftex_fail(&format!("cannot find image file {n}"));
                }
                match e.image_type {
                    IMAGE_TYPE_PDF => {
                        let page_box = g.fmt_undump_int();
                        let selected_page = g.fmt_undump_int();
                        let info = g.read_pdf_info(
                            &mut st.epdf,
                            &name,
                            None,
                            selected_page,
                            page_box,
                            pdf_major_version,
                            pdf_minor_version,
                            errorlevel,
                        );
                        e.width = g.bp2int(info.width);
                        e.height = g.bp2int(info.height);
                        e.num_pages = info.num_pages;
                        e.data = ImageData::Pdf(PdfImage {
                            orig_x: g.bp2int(info.orig_x),
                            orig_y: g.bp2int(info.orig_y),
                            selected_page,
                            page_box,
                            doc: info.doc,
                        });
                    }
                    IMAGE_TYPE_PNG => {
                        e.num_pages = 1;
                        g.read_png_info(&mut st.png, &mut e);
                    }
                    IMAGE_TYPE_JPG => {
                        e.num_pages = 1;
                        g.read_jpg_info(&mut e);
                    }
                    IMAGE_TYPE_JBIG2 => {
                        if pdf_major_version == 1 && pdf_minor_version < 4 {
                            g.pdftex_fail(&format!(
                                "JBIG2 images only possible with at least PDF 1.4; \
                                 you are generating PDF 1.{pdf_minor_version}"
                            ));
                        }
                        let selected_page = g.fmt_undump_int();
                        e.data = ImageData::Jbig2(Jbig2Image { selected_page });
                        g.read_jbig2_info(&mut st.jbig2, &mut e);
                    }
                    _ => g.pdftex_fail("unknown type of image"),
                }
                st.images[img] = e;
            }
        })
    }
}
