//! `writet3.c` (Type 3 fonts from PK bitmaps or `.pgc` files): not ported
//! yet.
//!
//! pdfTeX writes a font as Type 3 when it has no map entry or its map entry
//! is a bitmap (PK) font. The Type 1 documents of this lane never do that;
//! rather than write a wrong PDF, such a font stops the run with pdfTeX's
//! error layout and names the font.

use super::fonts::Fonts;
use crate::generated::Globals;

impl Globals {
    fn t3_not_ported(&mut self, f: i32) -> ! {
        let name = String::from_utf8_lossy(&self.c_string(self.font_name[f as usize])).into_owned();
        self.pdftex_fail(&format!(
            "font `{name}' has no Type 1 map entry; bitmap (PK/Type 3) fonts are not implemented yet"
        ))
    }

    /// `writet3` (writet3.c).
    pub fn writet3(&mut self, _st: &mut Fonts, _fm: Option<usize>, _objnum: i32, f: i32) {
        self.t3_not_ported(f)
    }

    /// `getpkcharwidth` (writet3.c): the width of a character of bitmap
    /// font `f`.
    pub fn get_pk_char_width(&mut self, f: i32, _w: i32) -> i32 {
        self.t3_not_ported(f)
    }

    /// `kpse_init_prog` and `kpse_set_program_enabled` for PK fonts: only
    /// PK lookup needs them.
    pub fn pk_init(&mut self, _resolution: i32, _pk_mode: i32) {}
}
