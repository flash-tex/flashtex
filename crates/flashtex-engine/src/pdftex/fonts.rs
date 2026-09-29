//! `mapfile.c`, as far as the log sees it, and stubs for `writefont.c`,
//! `writet3.c` and `tounicode.c`.
//!
//! pdfTeX reads its font map (`pdftex.map`, `\pdfmapfile`, `\pdfmapline`)
//! when the first font needs it, and logs each map file it reads as
//! `{/path/to/pdftex.map}`. That much is ported, from `mapfile.c`'s
//! `fm_read_info` and `process_map_item`, so that logs in PDF mode match. The
//! entries themselves are not parsed yet and no font is embedded (P3 ports
//! the rest per file): every font answers as one without a map entry, which
//! leaves DVI output and DVI-mode logs unchanged. What that changes in PDF
//! mode is the `<.../cmr10.pfb>` lines pdfTeX logs for embedded fonts.

use super::with_state;
use crate::generated::Globals;
use crate::resolver::Format;

/// The C globals of `mapfile.c` that the log depends on.
#[derive(Default)]
pub struct State {
    /// `mitem`: the map item still to be read (`pdfinitmapfile` sets the
    /// default map file).
    item: Option<(Vec<u8>, bool)>,
    /// `tfm_tree != NULL`: some map item has been processed.
    tree: bool,
}

impl Globals {
    /// `fm_read_info` (mapfile.c): read the pending map item. A map file is
    /// logged as `{path}`.
    fn fm_read_info(&mut self) {
        let item = with_state(|s| {
            s.fonts.tree = true;
            s.fonts.item.take()
        });
        let Some((line, is_file)) = item else {
            return;
        };
        if !is_file {
            return; // MAPLINE: an entry, not parsed yet (P3)
        }
        let name = String::from_utf8_lossy(&line).into_owned();
        match crate::system::find_file(&name, Format::Map) {
            None => self.pdftex_warn("cannot open font map file"),
            Some(path) => {
                self.print_bytes(b"{");
                self.print_bytes(path.as_bytes());
                self.print_bytes(b"}");
            }
        }
    }

    /// `fmlookup` (mapfile.c): reads the default map file the first time.
    fn fm_lookup(&mut self, _f: i32) -> bool {
        if !with_state(|s| s.fonts.tree) {
            self.fm_read_info();
        }
        false
    }

    /// `process_map_item` (mapfile.c): `\pdfmapfile` and `\pdfmapline`.
    fn process_map_item(&mut self, s: Vec<u8>, is_file: bool) {
        let mut s = &s[..];
        if s.first() == Some(&b' ') {
            s = &s[1..];
        }
        match s.first() {
            Some(b'+' | b'=' | b'-') => s = &s[1..],
            // No prefix: the default map file is not read at all.
            _ => with_state(|st| st.fonts.item = None),
        }
        if s.first() == Some(&b' ') {
            s = &s[1..];
        }
        let s: Vec<u8> = if is_file {
            s.iter().copied().take_while(|&c| c != b' ').collect()
        } else {
            s.to_vec()
        };
        if with_state(|st| st.fonts.item.is_some()) {
            self.fm_read_info(); // read the default map file first
        }
        if !s.is_empty() {
            with_state(|st| st.fonts.item = Some((s, is_file)));
            self.fm_read_info();
        }
    }

    /// `hasfmentry` (mapfile.c): whether font `f` has a map entry.
    pub fn hasfmentry(&mut self, f: i32) -> bool {
        self.fm_lookup(f)
    }
    /// `isscalable` (mapfile.c): whether font `f` is a scalable (Type 1,
    /// TrueType) font.
    pub fn isscalable(&mut self, f: i32) -> bool {
        self.hasfmentry(f)
    }
    /// `hasspacechar` (mapfile.c).
    pub fn hasspacechar(&mut self, f: i32) -> bool {
        self.fm_lookup(f)
    }
    /// `pdfmapfile` (mapfile.c): `\pdfmapfile`.
    pub fn pdfmapfile(&mut self, t: i32) {
        let s = self.tokens_to_string(t);
        let bytes = self.str_bytes(s);
        self.flush_str(self.last_tokens_string);
        self.process_map_item(bytes, true);
    }
    /// `pdfmapline` (mapfile.c): `\pdfmapline`.
    pub fn pdfmapline(&mut self, t: i32) {
        let s = self.tokens_to_string(t);
        let bytes = self.str_bytes(s);
        self.flush_str(self.last_tokens_string);
        self.process_map_item(bytes, false);
    }
    /// `pdfmaplinesp` (mapfile.c): the map line of the fake space font.
    pub fn pdfmaplinesp(&mut self) {
        self.process_map_item(
            b"=pdftexspace PdfTeX-Space <pdftexspace.pfb".to_vec(),
            false,
        );
    }
    /// `pdfinitmapfile("pdftex.map")` (mapfile.c).
    pub fn pdf_init_map_file(&mut self) {
        with_state(|s| s.fonts.item = Some((b"pdftex.map".to_vec(), true)));
    }
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
