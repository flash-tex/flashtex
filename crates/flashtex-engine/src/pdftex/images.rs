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
use crate::persist::Codec;

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
#[derive(Clone)]
pub struct PdfImage {
    pub orig_x: i32,
    pub orig_y: i32,
    pub selected_page: i32,
    pub page_box: i32,
    /// `doc`: the document's handle in [`pdftoepdf::State`].
    pub doc: usize,
    /// Not pdfTeX's: the page box as `read_pdf_info` found it, in bp
    /// (`orig_x`, `orig_y`, width, height), before `bp2int` made the scaled
    /// points above. The display list's IMAGE resource sends it
    /// (docs/protocol/display-list-v3.md §5.2: the box in bp).
    pub box_bp: [f32; 4],
}

/// `image_struct`, the part that depends on the type.
#[derive(Default, Clone)]
pub enum ImageData {
    #[default]
    None,
    Pdf(PdfImage),
    Png(PngImage),
    Jpg(JpgImage),
    Jbig2(Jbig2Image),
}

/// `image_entry`.
#[derive(Default, Clone)]
pub struct ImageEntry {
    /// `image_name`: the file found (`NULL` once freed).
    pub name: Option<Vec<u8>>,
    /// The file found, kept after `delete_image` frees `name` (not
    /// pdfTeX's): the display list describes the image by it on any page
    /// that draws it, also one typeset after the XObject was written or
    /// after a restore to such a state (`crate::displaylist`).
    pub file: Option<Vec<u8>>,
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
    /// Not pdfTeX's: the content hash of the file `read_image` read, which
    /// a persisted S₀ compares with the file when it is reopened
    /// ([`Globals::verify_persisted_images`]); `None` for an image read
    /// with the format.
    pub hash: Option<[u64; 2]>,
    /// Not pdfTeX's: `write_image` has written the image (an S₀ that holds
    /// such an image is not reopened, see [`State`]'s codec).
    pub written: bool,
}

/// Cloned at every checkpoint (`crate::checkpoint`): the entries are
/// plain data except the open handles, which a copy opens again when it
/// is used ([`super::writepng::PngImage`], [`pdftoepdf::PdfDocument`]).
#[derive(Default, Clone)]
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
    /// Not pdfTeX's: a table decoded from a persisted S₀ whose files are not
    /// read again yet ([`Globals::verify_persisted_images`]).
    pending: Option<Box<Pending>>,
}

/// What a table decoded from a persisted S₀ keeps for the check against
/// its files: the position each JPEG reader had reached in its file (entry,
/// `ftell`, `feof`), which a reader opened again must reach too.
#[derive(Default, Clone)]
struct Pending {
    jpg: Vec<(usize, i64, bool)>,
}

/// `strcasecmp(a, b) == 0` for ASCII.
fn eq_ignore_case(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

impl Globals {
    /// Store `bytes` at `pdfptr` of the PDF buffer, as the C code stores
    /// into `pdfbuf` directly (leaving `pdflastbyte` alone); the caller has
    /// made room (`pdfroom`).
    pub(crate) fn pdf_buf_store(&mut self, bytes: &[u8]) {
        let p = self.pdf_ptr as usize;
        let buf = if self.pdf_buf_is_os {
            &mut self.pdf_os_buf
        } else {
            &mut self.pdf_op_buf
        };
        for (d, &s) in buf[p..p + bytes.len()].iter_mut().zip(bytes) {
            *d = s as i32;
        }
        self.pdf_ptr += bytes.len() as i32;
    }

    /// `pdfout` of each byte of `bytes`: the same buffer flushes (or
    /// object-stream buffer growth) as one `pdfroom(1)` per byte, at the
    /// bytes where the buffer is full, with the copying done a buffer at a
    /// time.
    pub(crate) fn c_pdf_out_bytes(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while !rest.is_empty() {
            if self.pdf_ptr >= self.pdf_buf_size {
                self.c_pdf_room(1);
            }
            let n = ((self.pdf_buf_size - self.pdf_ptr) as usize).min(rest.len());
            self.pdf_buf_store(&rest[..n]);
            rest = &rest[n..];
        }
    }

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
        // (not pdfTeX's: the file as read, for a persisted S₀)
        e.hash = Some(crate::persist::hash128(file.bytes()));
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
        if probing() {
            probe_fail(format!("{}: cannot open it", String::from_utf8_lossy(name)));
        }
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

    /// `bp2int`: big points as a scaled number, rounded. writeimg.c's `round`
    /// is web2c's `zround` (ptexlib.h -> pdftexd.h -> texmfmp.h -> cpascal.h),
    /// as in every pdfTeX C file, hence `pas_round`, not `f64::round`.
    fn bp2int(&self, p: f32) -> i32 {
        crate::system::pas_round(p as f64 * (self.one_hundred_bp as f64 / 100.0))
    }

    /// Not pdfTeX's: finish an image table that a persisted S₀ brought
    /// (`host::read_s0`, once the C state is in place). Every image is read
    /// again, as `undumpimagemeta` reads the format's: with `read_image`'s
    /// readers, in the table's order, so that the included documents'
    /// handles and their reuse come out as the run left them. Then the
    /// files must have the content they had when they were read, and
    /// everything read again must equal what the table holds. `Err` says
    /// why not (an image written already, a file changed, anything that
    /// differs or cannot be read): the caller then runs in full. A table
    /// not decoded from a file is left alone.
    pub fn verify_persisted_images(&mut self) -> Result<(), String> {
        self.with_images(|g, st| {
            let Some(pending) = st.pending.take() else {
                return Ok(());
            };
            let r = g.verify_images(st, &pending);
            if r.is_err() {
                *st = State::default();
            }
            r
        })
    }

    fn verify_images(&mut self, st: &mut State, pending: &Pending) -> Result<(), String> {
        fn enc_of<T: crate::persist::Codec>(x: &T) -> Vec<u8> {
            let mut w = vec![];
            x.enc(&mut w);
            w
        }
        st.check_files()?;
        // The readers' state, built again from nothing; the PNG reader's
        // from the table's (`read_png_info` makes a page-group object only
        // while there is none: it must make none here).
        let mut epdf = pdftoepdf::State::default();
        let mut jbig2 = super::writejbig2::State::default();
        let mut png = st.png.clone();
        let (major, minor) = (self.fixed_pdf_major_version, self.fixed_pdf_minor_version);
        for (i, e) in st.images.iter_mut().enumerate() {
            let name = e.name.clone().unwrap_or_default();
            let shown = String::from_utf8_lossy(&name).into_owned();
            let differs =
                |what: &str| format!("image {i} ({shown}): {what} differs when read again");
            let failed = |m: String| format!("image {i} ({shown}) cannot be read again: {m}");
            let mut f = ImageEntry {
                name: Some(name.clone()),
                file: Some(name.clone()),
                hash: e.hash,
                image_type: e.image_type,
                colorspace_ref: e.colorspace_ref,
                ..ImageEntry::default()
            };
            match e.image_type {
                IMAGE_TYPE_PDF => {
                    let ImageData::Pdf(p) = &e.data else {
                        return Err(differs("its type"));
                    };
                    let p = p.clone();
                    // By page number, as `undumpimagemeta` does (a named
                    // destination has selected `selected_page`); no version
                    // warning (`errorlevel` < 0): the read gave it.
                    let info = probe(|| {
                        self.read_pdf_info(
                            &mut epdf,
                            &name,
                            None,
                            p.selected_page,
                            p.page_box,
                            major,
                            minor,
                            -1,
                        )
                    })
                    .map_err(failed)?;
                    f.width = self.bp2int(info.width);
                    f.height = self.bp2int(info.height);
                    f.rotate = info.rotate as i32;
                    f.num_pages = info.num_pages;
                    f.group_ref = if info.has_page_group { -1 } else { 0 };
                    // (a page group's object number is made when the image
                    // is first put on a page, pdftex.web)
                    if f.group_ref == -1 && e.group_ref > 0 {
                        f.group_ref = e.group_ref;
                    }
                    let q = PdfImage {
                        orig_x: self.bp2int(info.orig_x),
                        orig_y: self.bp2int(info.orig_y),
                        selected_page: info.page_num,
                        page_box: p.page_box,
                        doc: info.doc,
                        box_bp: [info.orig_x, info.orig_y, info.width, info.height],
                    };
                    let key = |p: &PdfImage| {
                        (
                            [p.orig_x, p.orig_y, p.selected_page, p.page_box],
                            p.doc,
                            p.box_bp.map(f32::to_bits),
                        )
                    };
                    if key(&q) != key(&p) {
                        return Err(differs("the page"));
                    }
                    f.data = ImageData::Pdf(q);
                }
                IMAGE_TYPE_PNG => {
                    if !matches!(e.data, ImageData::Png(_)) {
                        return Err(differs("its type"));
                    }
                    f.num_pages = 1;
                    let (objs, group) = (self.obj_ptr, self.pdf_page_group_val);
                    let r = probe(|| self.read_png_info(&mut png, &mut f));
                    let made_object = self.obj_ptr != objs;
                    self.pdf_page_group_val = group;
                    r.map_err(failed)?;
                    if made_object {
                        return Err(differs("the page group"));
                    }
                    // The page group in force when it was read: the file
                    // says only whether it needs one.
                    if (f.group_ref != 0) != (e.group_ref != 0) {
                        return Err(differs("the page group"));
                    }
                    f.group_ref = e.group_ref;
                }
                IMAGE_TYPE_JPG => {
                    let ImageData::Jpg(j) = &e.data else {
                        return Err(differs("its type"));
                    };
                    let had = (j.color_space, j.bits_per_component, j.length);
                    f.num_pages = 1;
                    probe(|| self.read_jpg_info(&mut f)).map_err(failed)?;
                    let ImageData::Jpg(k) = &f.data else {
                        return Err(differs("its type"));
                    };
                    let at = pending.jpg.iter().find(|x| x.0 == i).map(|x| (x.1, x.2));
                    if (k.color_space, k.bits_per_component, k.length) != had
                        || at != Some((k.file.tell(), k.file.feof()))
                    {
                        return Err(differs("the JPEG header"));
                    }
                    if Some(crate::persist::hash128(k.file.bytes())) != e.hash {
                        return Err(format!("image {i} ({shown}) changed"));
                    }
                }
                IMAGE_TYPE_JBIG2 => {
                    let ImageData::Jbig2(j) = &e.data else {
                        return Err(differs("its type"));
                    };
                    f.data = ImageData::Jbig2(j.clone());
                    probe(|| self.read_jbig2_info(&mut jbig2, &mut f)).map_err(failed)?;
                }
                _ => return Err(differs("its type")),
            }
            let plain = |e: &ImageEntry| {
                (
                    (e.name.clone(), e.file.clone(), e.hash, e.written),
                    [
                        e.image_type,
                        e.color_type,
                        e.width,
                        e.height,
                        e.rotate,
                        e.x_res,
                        e.y_res,
                        e.num_pages,
                        e.colorspace_ref,
                        e.group_ref,
                    ],
                )
            };
            if plain(&f) != plain(&*e) {
                return Err(differs("what the table holds"));
            }
            // the readers opened again stand for the run's
            *e = f;
        }
        if enc_of(&png) != enc_of(&st.png) {
            return Err("the PNG reader's state differs when the images are read again".into());
        }
        if enc_of(&jbig2) != enc_of(&st.jbig2) {
            return Err("the JBIG2 reader's state differs when the images are read again".into());
        }
        if enc_of(&epdf) != enc_of(&st.epdf) {
            return Err("the included PDF documents differ when the images are read again".into());
        }
        // Once more, after the readers: a file changed between the first
        // check and its reader's open would otherwise pass where the reader
        // returns nothing the table compares (a PNG's bit depth, which TeX
        // sees as `\pdflastximagecolordepth`; #1685 review).
        st.check_files()?;
        st.epdf = epdf;
        Ok(())
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
        if !super::xpdf::LINKED {
            let name = String::from_utf8_lossy(&self.str_bytes(s)).into_owned();
            self.pdftex_fail(&format!(
                "cannot read image `{name}': libpng and xpdf are not linked into this build"
            ));
        }
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
            e.file = Some(name.clone());
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
                        box_bp: [info.orig_x, info.orig_y, info.width, info.height],
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
            e.written = true;
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
                e.file = e.name.clone();
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
                            box_bp: [info.orig_x, info.orig_y, info.width, info.height],
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

thread_local! {
    /// [`probe`] is running: a reader's failure returns to it.
    static PROBING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Not pdfTeX's: a reader's failure while [`probing`] (what `pdftex_fail`
/// or a fatal file error would have printed), unwound to [`probe`].
struct ProbeFail(String);

/// A reader is reading an image again for a persisted S₀
/// ([`Globals::verify_persisted_images`]): `pdftex_fail` and the fatal file
/// errors return to [`probe`], printing nothing, instead of ending the run.
pub(crate) fn probing() -> bool {
    PROBING.with(|p| p.get())
}

/// Return from a reader to [`probe`] with `msg`.
pub(crate) fn probe_fail(msg: String) -> ! {
    std::panic::resume_unwind(Box::new(ProbeFail(msg)))
}

/// Run a reader for [`Globals::verify_persisted_images`]: its failure is an
/// `Err`, as is a panic (an S₀ that cannot be checked is not used).
fn probe<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    let was = PROBING.with(|p| p.replace(true));
    // (xpdf's own messages too: a reopen prints nothing a run did not)
    super::xpdf::set_quiet(true);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    super::xpdf::set_quiet(false);
    PROBING.with(|p| p.set(was));
    r.map_err(|e| match e.downcast::<ProbeFail>() {
        Ok(p) => p.0,
        Err(_) => "the reader panicked".to_string(),
    })
}

impl State {
    /// [`Globals::verify_persisted_images`]'s first test, before a reader
    /// opens anything: no image is written or freed (an S₀ is reopened only
    /// while its images are read and no more), each has its file's content
    /// hash, and each file has that content now.
    fn check_files(&self) -> Result<(), String> {
        for (i, e) in self.images.iter().enumerate() {
            let shown = String::from_utf8_lossy(e.file.as_deref().unwrap_or_default()).into_owned();
            let Some(name) = &e.name else {
                return Err(format!("image {i} ({shown}) was written before S0"));
            };
            if e.written {
                return Err(format!("image {i} ({shown}) was written before S0"));
            }
            let Some(hash) = e.hash else {
                return Err(format!(
                    "image {i} ({shown}) has no content hash (it came with the format)"
                ));
            };
            let now = std::fs::read(super::cfile::os_path(name))
                .map_err(|err| format!("image {i} ({shown}): {err}"))?;
            if crate::persist::hash128(&now) != hash {
                return Err(format!("image {i} ({shown}) changed"));
            }
        }
        Ok(())
    }

    /// The same image table (`CState::same_as`), for two copies that are
    /// not the same one: an image read, written or deleted after the
    /// restart point makes a convergence test fail (a missed convergence,
    /// never a wrong one), since the open handles have no value to compare.
    pub fn same_as(&self, _o: &State) -> bool {
        false
    }
}

/// A persisted checkpoint (`host::Session::save_s0`) carries the image
/// table as `dumpimagemeta` dumps it into a format, with what a copy of the
/// table carries besides (`crate::checkpoint`): every entry's fields (a
/// freed one's too), its type's keys, the readers' state, the included PDF
/// documents by handle, and each file's content hash. The open readers are
/// not bytes: a decoded table is `pending` until
/// [`Globals::verify_persisted_images`] has read every image again and
/// compared.
impl crate::persist::Codec for State {
    fn enc(&self, w: &mut Vec<u8>) {
        self.images.len().enc(w);
        for e in &self.images {
            e.name.enc(w);
            e.file.enc(w);
            e.hash.enc(w);
            e.written.enc(w);
            [
                e.image_type,
                e.color_type,
                e.width,
                e.height,
                e.rotate,
                e.x_res,
                e.y_res,
                e.num_pages,
                e.colorspace_ref,
                e.group_ref,
            ]
            .enc(w);
            match &e.data {
                ImageData::None => 0u8.enc(w),
                ImageData::Pdf(p) => {
                    1u8.enc(w);
                    [p.orig_x, p.orig_y, p.selected_page, p.page_box].enc(w);
                    p.doc.enc(w);
                    p.box_bp.map(f32::to_bits).enc(w);
                }
                ImageData::Png(_) => 2u8.enc(w),
                ImageData::Jpg(j) => {
                    3u8.enc(w);
                    j.color_space.enc(w);
                    j.bits_per_component.enc(w);
                    j.length.enc(w);
                    j.file.tell().enc(w);
                    j.file.feof().enc(w);
                }
                ImageData::Jbig2(j) => {
                    4u8.enc(w);
                    j.selected_page.enc(w);
                }
            }
        }
        self.image_limit.enc(w);
        self.allocated.enc(w);
        self.png.enc(w);
        self.jbig2.enc(w);
        self.epdf.enc(w);
    }
    fn dec(r: &mut crate::persist::Reader) -> Result<Self, String> {
        let n = usize::dec(r)?;
        if n > r.buf.len() {
            return Err("persisted state: bad length".into());
        }
        let mut pending = Pending::default();
        let mut images = Vec::with_capacity(n);
        for i in 0..n {
            let mut e = ImageEntry {
                name: Codec::dec(r)?,
                file: Codec::dec(r)?,
                hash: Codec::dec(r)?,
                written: Codec::dec(r)?,
                ..ImageEntry::default()
            };
            let [t, c, wd, ht, rot, xr, yr, np, cs, gr] = <[i32; 10]>::dec(r)?;
            (e.image_type, e.color_type, e.width, e.height, e.rotate) = (t, c, wd, ht, rot);
            (e.x_res, e.y_res, e.num_pages, e.colorspace_ref, e.group_ref) = (xr, yr, np, cs, gr);
            e.data = match u8::dec(r)? {
                0 => ImageData::None,
                1 => {
                    let [orig_x, orig_y, selected_page, page_box] = <[i32; 4]>::dec(r)?;
                    ImageData::Pdf(PdfImage {
                        orig_x,
                        orig_y,
                        selected_page,
                        page_box,
                        doc: usize::dec(r)?,
                        box_bp: <[u32; 4]>::dec(r)?.map(f32::from_bits),
                    })
                }
                2 => ImageData::Png(PngImage::unopened(e.name.as_deref().unwrap_or_default())),
                3 => {
                    let j = JpgImage {
                        color_space: i32::dec(r)?,
                        bits_per_component: i32::dec(r)?,
                        length: u64::dec(r)?,
                        // (the file's bytes come with the check)
                        file: super::cfile::CFile::from_bytes(Vec::new()),
                    };
                    pending.jpg.push((i, i64::dec(r)?, bool::dec(r)?));
                    ImageData::Jpg(j)
                }
                4 => ImageData::Jbig2(Jbig2Image {
                    selected_page: i32::dec(r)?,
                }),
                k => return Err(format!("persisted state: image data of kind {k}")),
            };
            images.push(e);
        }
        Ok(State {
            images,
            image_limit: i32::dec(r)?,
            allocated: bool::dec(r)?,
            png: Codec::dec(r)?,
            jbig2: Codec::dec(r)?,
            epdf: Codec::dec(r)?,
            pending: Some(Box::new(pending)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::cfile::{CFile, Whence};
    use super::*;
    use crate::persist::{hash128, Reader};

    fn enc_of(st: &State) -> Vec<u8> {
        let mut w = vec![];
        st.enc(&mut w);
        w
    }

    fn table(name: &[u8], hash: [u64; 2]) -> State {
        let mut jpg = CFile::from_bytes(b"\xFF\xD8xyz".to_vec());
        jpg.seek(3, Whence::Set);
        let e = |t: i32, data: ImageData| ImageEntry {
            name: Some(name.to_vec()),
            file: Some(name.to_vec()),
            hash: Some(hash),
            image_type: t,
            width: 100,
            height: -7,
            data,
            ..ImageEntry::default()
        };
        State {
            images: vec![
                e(
                    IMAGE_TYPE_PDF,
                    ImageData::Pdf(PdfImage {
                        orig_x: 1,
                        orig_y: 2,
                        selected_page: 3,
                        page_box: 2,
                        doc: 0,
                        box_bp: [0.5, -1.25, 612.0, 791.999],
                    }),
                ),
                e(
                    IMAGE_TYPE_JPG,
                    ImageData::Jpg(JpgImage {
                        color_space: 3,
                        bits_per_component: 8,
                        length: 5,
                        file: jpg,
                    }),
                ),
                e(
                    IMAGE_TYPE_JBIG2,
                    ImageData::Jbig2(Jbig2Image { selected_page: 2 }),
                ),
                ImageEntry {
                    name: None,
                    ..e(IMAGE_TYPE_PNG, ImageData::None)
                },
            ],
            image_limit: 256,
            allocated: true,
            ..State::default()
        }
    }

    /// Every field the codec carries comes back, and the JPEG reader's
    /// position waits for the check.
    #[test]
    fn the_table_round_trips() {
        let mut st = table(b"./a.pdf", [1, 2]);
        let w = enc_of(&st);
        let back = State::dec(&mut Reader::new(&w)).unwrap();
        // (the decoded JPEG reader has no file yet: its position is pending)
        if let ImageData::Jpg(j) = &mut st.images[1].data {
            j.file.seek(0, Whence::Set);
        }
        assert_eq!(enc_of(&back), enc_of(&st));
        let p = back.pending.as_ref().expect("a decoded table is pending");
        assert_eq!(p.jpg, vec![(1, 3, false)]);
    }

    /// The check before any reader runs: a written (or freed) image, an
    /// image without a hash and a changed file each refuse the table, with
    /// a reason naming the file.
    #[test]
    fn a_changed_or_written_image_is_refused() {
        let d = std::env::temp_dir().join(format!("flashtex-s0img-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("pic.png");
        std::fs::write(&f, b"first").unwrap();
        let name = f.to_str().unwrap().as_bytes().to_vec();
        let mut st = table(&name, hash128(b"first"));
        let freed = st.images.pop().unwrap();
        st.check_files().unwrap();
        // the file changes
        std::fs::write(&f, b"second").unwrap();
        let err = st.check_files().unwrap_err();
        assert!(err.contains("pic.png") && err.contains("changed"), "{err}");
        std::fs::write(&f, b"first").unwrap();
        st.check_files().unwrap();
        // a written image
        st.images[0].written = true;
        assert!(st.check_files().unwrap_err().contains("written"));
        st.images[0].written = false;
        // a freed one
        st.images.push(freed);
        assert!(st.check_files().unwrap_err().contains("written"));
        st.images.pop();
        // one that came with the format
        st.images[1].hash = None;
        assert!(st.check_files().unwrap_err().contains("no content hash"));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// A failure inside a probe returns to it; outside one, `probing` is
    /// off.
    #[test]
    fn a_probe_returns_a_failure() {
        assert!(!probing());
        let r: Result<(), String> = probe(|| probe_fail("no such page".into()));
        assert_eq!(r, Err("no such page".to_string()));
        assert!(!probing());
        assert_eq!(probe(|| 7), Ok(7));
    }
}
