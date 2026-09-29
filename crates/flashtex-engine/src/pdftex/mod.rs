//! pdfTeX's C parts, behind the interface `changes/ext.ch` declares.
//!
//! In TeX Live, pdftex.web calls routines written in C (`pdftexdir/*.c`,
//! declared to web2c by `pdftex.defines`). `changes/ext.ch` declares them as
//! Pascal `external` routines, so the translated engine calls them with their
//! real types, and their bodies are here, one module per C file:
//!
//! | module | C original | state in this lane |
//! |---|---|---|
//! | [`utils`] | `utils.c`, `texmfmp.c` | ported: arithmetic, dates, escapes, file queries, MD5, colour stacks, `\pdfsave`/`\pdfsetmatrix`; `\pdfmatch` is not |
//! | [`vfpacket`] | `vfpacket.c` | ported |
//! | [`avlstuff`] | `avlstuff.c` | ported (a map per object type) |
//! | [`output`] | `pdftex.h`'s `writepdf`, `writezip.c` | no-op writer: the PDF bytes are counted, not written |
//! | [`fonts`] | `mapfile.c`, `writefont.c`, `writet3.c`, `tounicode.c` | stubs: no font map, nothing embedded |
//! | [`images`] | `writeimg.c` and the image readers | stubs: image inclusion is an error |
//!
//! DESIGN.md section 4.1 ports the C parts per file under the lockstep
//! harness (phase P3); until then the engine runs in DVI mode exactly and in
//! PDF mode with a writer that produces no file. Logs and box dumps (P-T1)
//! are unaffected by the writer, except where a stub says otherwise.
//!
//! The C files keep their state in C globals; here it is in [`CState`], one
//! per thread (one engine runs per thread).

pub mod avlstuff;
pub mod fonts;
pub mod images;
pub mod md5;
pub mod output;
pub mod utils;
pub mod vfpacket;

use crate::generated::Globals;
use std::cell::RefCell;

/// The C globals of pdfTeX's C parts.
#[derive(Default)]
pub struct CState {
    pub utils: utils::State,
    pub vf: vfpacket::State,
    pub avl: avlstuff::State,
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
        self.print_ln();
        self.print_ln();
        self.print_bytes(b"pdfTeX warning: ");
        let name = crate::system::invocation_name();
        self.print_bytes(name.as_bytes());
        self.print_bytes(b": ");
        self.print_bytes(msg.as_bytes());
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
