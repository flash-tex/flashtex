//! pdfTeX's C parts, behind the interface `changes/ext.ch` declares.
//!
//! In TeX Live, pdftex.web calls routines written in C (`pdftexdir/*.c`,
//! declared to web2c by `pdftex.defines`). `changes/ext.ch` declares them as
//! Pascal `external` routines, so the translated engine calls them with their
//! real types, and their bodies are here, one module per C file:
//!
//! | module | C original | state |
//! |---|---|---|
//! | [`utils`] | `utils.c`, `texmfmp.c` | ported: arithmetic, dates, escapes, file queries, MD5, colour stacks, `\pdfsave`/`\pdfsetmatrix`; `\pdfmatch` is not |
//! | [`vfpacket`] | `vfpacket.c` | ported |
//! | [`avlstuff`] | `avlstuff.c` | ported (a map per object type) |
//! | [`output`] | `pdftex.h`'s `writepdf`, `writezip.c`, `utils.c`'s output routines | ported; streams compressed by TeX Live's zlib ([`zlib`]) |
//! | [`mapfile`] | `mapfile.c` (and `subfont.c`'s test) | ported; TrueType subfont entries stop the run |
//! | [`writefont`] | `writefont.c` | ported; TrueType/OpenType embedding stops the run |
//! | [`writet1`] | `writet1.c` | ported: Type 1 embedding and subsetting |
//! | [`writeenc`] | `writeenc.c` | ported |
//! | [`tounicode`] | `tounicode.c` | ported |
//! | [`writet3`] | `writet3.c`, `pkin.c` | not ported: a Type 3 (PK) font stops the run |
//! | [`images`] | `writeimg.c` | ported: the image table, type detection, dispatch, (un)dumping |
//! | [`writepng`] | `writepng.c` | ported, over TeX Live's libpng (linked, `csrc/png_shim.c`); IDAT copied unchanged where pdfTeX copies it |
//! | [`writejpg`] | `writejpg.c` | ported |
//! | [`writejbig2`] | `writejbig2.c` | ported |
//! | [`pdftoepdf`] | `pdftoepdf.cc` | ported, over TeX Live's xpdf (linked, [`xpdf`], `csrc/xpdf_shim.cc`) |
//! | [`epdf`] | `epdf.c` | ported |
//!
//! The font backend's globals are one struct, [`fonts::Fonts`]; see there.
//! [`cfmt`] calls the C library's own `sprintf`/`sscanf` where pdfTeX's C
//! code formats or parses floating-point numbers.
//!
//! The C files keep their state in C globals; here it is in [`CState`], one
//! per thread (one engine runs per thread).

pub mod avlstuff;
pub mod cfile;
pub mod cfmt;
pub mod epdf;
pub mod fonts;
pub mod images;
pub mod mapfile;
pub mod md5;
pub mod output;
pub mod pdftoepdf;
pub mod tounicode;
pub mod utils;
pub mod vfpacket;
pub mod writeenc;
pub mod writefont;
pub mod writejbig2;
pub mod writejpg;
pub mod writepng;
pub mod writet1;
pub mod writet3;
pub mod xpdf;
pub mod zlib;

use crate::generated::Globals;
use std::cell::RefCell;

/// The C globals of pdfTeX's C parts.
#[derive(Default)]
pub struct CState {
    pub utils: utils::State,
    pub vf: vfpacket::State,
    pub avl: avlstuff::State,
    pub fonts: fonts::Fonts,
    /// The font backend is out (see [`Globals::with_fonts`]).
    pub fonts_busy: bool,
    pub out: output::State,
    /// The image table and the image writers' state.
    pub img: images::State,
}

thread_local! {
    static STATE: RefCell<CState> = RefCell::new(CState::default());
}

/// Run `f` with the C state of this thread.
pub fn with_state<R>(f: impl FnOnce(&mut CState) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// Forget all C state (a new job in the same thread).
pub fn reset_state() {
    STATE.with(|s| *s.borrow_mut() = CState::default());
}

impl Globals {
    /// The bytes of string `s` of the string pool.
    pub fn str_bytes(&self, s: i32) -> Vec<u8> {
        let (a, b) = (
            self.str_start[s as usize] as usize,
            self.str_start[s as usize + 1] as usize,
        );
        self.str_pool[a..b].iter().map(|&c| c as u8).collect()
    }

    /// The bytes of the string being built, from `start` to `pool_ptr`.
    pub fn pool_bytes_from(&self, start: i32) -> Vec<u8> {
        self.str_pool[start as usize..self.pool_ptr as usize]
            .iter()
            .map(|&c| c as u8)
            .collect()
    }

    /// Append bytes at `pool_ptr`, as the C parts do (`strpool[poolptr++]`).
    /// Like them, an overflowing append stops at `pool_size` and leaves the
    /// error to the next `str_room`.
    pub fn pool_append(&mut self, bytes: &[u8]) {
        let size = crate::generated::consts::pool_size;
        for &b in bytes {
            if self.pool_ptr >= size {
                self.pool_ptr = size;
                return;
            }
            self.str_pool[self.pool_ptr as usize] = b as i32;
            self.pool_ptr += 1;
        }
    }

    /// `maketexstring`: a new pool string with these bytes.
    pub fn make_tex_string(&mut self, bytes: &[u8]) -> i32 {
        self.pool_append(bytes);
        self.make_string()
    }

    /// `pdftex_warn` (utils.c): the same layout as pdftex.web's
    /// `pdf_warning`.
    pub fn pdftex_warn(&mut self, msg: &str) {
        self.pdftex_warn_bytes(msg.as_bytes())
    }

    /// `pdftex_warn` of a message that need not be UTF-8 (glyph and file
    /// names are bytes).
    pub fn pdftex_warn_bytes(&mut self, msg: &[u8]) {
        self.print_ln();
        self.print_ln();
        self.print_bytes(b"pdfTeX warning: ");
        let name = crate::system::invocation_name();
        self.print_bytes(name.as_bytes());
        if let Some(f) = output::cur_file_name() {
            self.print_bytes(b" (file ");
            self.print_bytes(&f);
            self.print_bytes(b")");
        }
        self.print_bytes(b": ");
        self.print_bytes(output::printf_cut(msg));
        self.print_ln();
    }

    /// Print raw bytes, as `tex_printf` does through a temporary string
    /// (`print` of a string prints its characters with `print_char`).
    pub fn print_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.print_char(b as i32);
        }
    }
}
