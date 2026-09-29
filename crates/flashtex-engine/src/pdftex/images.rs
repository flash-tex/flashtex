//! Stubs for `writeimg.c` and the image readers (`writepng.c`, `writejpg.c`,
//! `writejbig2.c`, `pdftoepdf.cc`).
//!
//! Image inclusion (`\pdfximage`) is not ported in this lane (P3). Reading an
//! image is therefore a fatal error with pdfTeX's own layout, rather than a
//! silently wrong box; no image can exist, so the queries below are never
//! reached with a real image.

use crate::generated::Globals;

impl Globals {
    /// `readimage`: `\pdfximage`.
    #[allow(clippy::too_many_arguments)]
    pub fn read_image(
        &mut self,
        s: i32,
        _page: i32,
        _page_name: i32,
        _colorspace: i32,
        _page_box: i32,
        _major: i32,
        _minor: i32,
        _errorlevel: i32,
    ) -> i32 {
        let name = String::from_utf8_lossy(&self.str_bytes(s)).into_owned();
        self.pdftex_fail(&format!(
            "cannot read image `{name}': image inclusion is not implemented yet"
        ))
    }
    pub fn check_image_b(&mut self, _procset: i32) -> bool {
        false
    }
    pub fn check_image_c(&mut self, _procset: i32) -> bool {
        false
    }
    pub fn check_image_i(&mut self, _procset: i32) -> bool {
        false
    }
    pub fn is_pdf_image(&mut self, _img: i32) -> bool {
        false
    }
    pub fn is_png_image(&mut self, _img: i32) -> bool {
        false
    }
    pub fn epdf_orig_x(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn epdf_orig_y(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn image_width(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn image_height(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn image_rotate(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn image_pages(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn image_x_res(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn image_y_res(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn image_colordepth(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn get_image_group_ref(&mut self, _img: i32) -> i32 {
        0
    }
    pub fn set_image_group_ref(&mut self, _img: i32, _ref: i32) {}
    pub fn delete_image(&mut self, _img: i32) {}
    pub fn update_image_procset(&mut self, _img: i32) {}
    pub fn write_image(&mut self, _img: i32) {}
    /// `dumpimagemeta`: there are no images to dump.
    pub fn dumpimagemeta(&mut self) {}
    /// `undumpimagemeta`: symmetric with `dumpimagemeta`.
    pub fn undumpimagemeta(&mut self, _major: i32, _minor: i32, _errorlevel: i32) {}
    pub fn flush_jbig2_page0_objects(&mut self) {}
}
