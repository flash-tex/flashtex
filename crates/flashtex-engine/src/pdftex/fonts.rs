//! Stubs for `mapfile.c`, `writefont.c`, `writet3.c` and `tounicode.c`.
//!
//! These read the font map (`pdftex.map`) and embed fonts in the PDF. None
//! of that is ported in this lane (P3 ports them per file). The stubs answer
//! as pdfTeX does when a font has no map entry, which leaves DVI output and
//! every DVI-mode log line unchanged.
//!
//! Known P-T1 effect in PDF mode: pdfTeX loads the map file when the first
//! font is used and logs its name (`{.../pdftex.map}`); these stubs do not.

use crate::generated::Globals;

impl Globals {
    /// `hasfmentry` (mapfile.c): whether font `f` has a map entry.
    pub fn hasfmentry(&mut self, _f: i32) -> bool {
        false
    }
    /// `isscalable` (mapfile.c): whether font `f` is a scalable (Type 1,
    /// TrueType) font.
    pub fn isscalable(&mut self, _f: i32) -> bool {
        false
    }
    /// `hasspacechar` (mapfile.c).
    pub fn hasspacechar(&mut self, _f: i32) -> bool {
        false
    }
    /// `pdfmapfile` (mapfile.c): `\pdfmapfile`.
    pub fn pdfmapfile(&mut self, _t: i32) {}
    /// `pdfmapline` (mapfile.c): `\pdfmapline`.
    pub fn pdfmapline(&mut self, _t: i32) {}
    /// `pdfmaplinesp` (mapfile.c): the map line of the fake space font.
    pub fn pdfmaplinesp(&mut self) {}
    /// `pdfinitmapfile("pdftex.map")` (mapfile.c).
    pub fn pdf_init_map_file(&mut self) {}
    /// `kpse_init_prog` and `kpse_set_program_enabled` for PK fonts.
    pub fn pk_init(&mut self, _resolution: i32, _pk_mode: i32) {}
    /// `dopdffont` (writefont.c): register font `f` as PDF object `n`.
    pub fn do_pdf_font(&mut self, _n: i32, _f: i32) {}
    /// `writefontstuff` (writefont.c): embed all fonts.
    pub fn write_fontstuff(&mut self) {}
    /// `getpkcharwidth` (writet3.c).
    pub fn get_pk_char_width(&mut self, _f: i32, _w: i32) -> i32 {
        0
    }
    /// `deftounicode` (tounicode.c): `\pdfglyphtounicode`.
    pub fn def_tounicode(&mut self, _glyph: i32, _unistr: i32) {}
    /// `dumptounicode` (tounicode.c): nothing is dumped, symmetrically with
    /// `undumptounicode`.
    pub fn dumptounicode(&mut self) {}
    /// `undumptounicode` (tounicode.c).
    pub fn undumptounicode(&mut self) {}
}
