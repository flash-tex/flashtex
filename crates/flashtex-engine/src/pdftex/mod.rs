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
//! | [`writet3`] | `writet3.c`, `pkin.c` | ported: Type 3 fonts from PK files (kpathsea's `kpse_find_pk`, mktexpk) and `.pgc` files |
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
pub mod shared;
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
///
/// A checkpoint clones this after every page (DESIGN.md §5.2), so what is
/// large and changes rarely is [`shared::Shared`] (copied on its first
/// write after a checkpoint) and what grows a little per page is sharded
/// ([`shared::ShardMap`], in `avl`); the rest is small.
#[derive(Default, Clone)]
pub struct CState {
    pub utils: utils::State,
    pub vf: shared::Shared<vfpacket::State>,
    pub avl: avlstuff::State,
    pub fonts: fonts::Fonts,
    /// The font backend is out (see [`Globals::with_fonts`]).
    pub fonts_busy: bool,
    /// Not shared: it holds the zlib stream, which a copy does not carry.
    pub out: output::State,
    /// The image table and the image writers' state.
    pub img: shared::Shared<images::State>,
    /// The display-list writer's side table (`crate::displaylist`), set
    /// only in a snapshot: engine state outside the word space like the
    /// rest, but not what the engine computes, so `same_as` ignores it.
    pub dl: crate::displaylist::Snap,
}

thread_local! {
    static STATE: RefCell<CState> = RefCell::new(CState::default());
}

/// Run `f` with the C state of this thread.
pub fn with_state<R>(f: impl FnOnce(&mut CState) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

thread_local! {
    static WARNINGS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static PREVIEW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static LAST_BYTE_READS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times this run has read `pdf_last_byte` (`pdf_newline`): the
/// convergence test's evidence that an old run's future never read it
/// (`crate::incr`). Part of every checkpoint's host record.
pub fn last_byte_reads() -> u64 {
    LAST_BYTE_READS.with(|c| c.get())
}

pub fn set_last_byte_reads(n: u64) {
    LAST_BYTE_READS.with(|c| c.set(n));
}

/// Preview mode (`crate::incr`): PDF streams are stored (zlib level 0), not
/// compressed. Nothing the engine prints or computes depends on the level
/// zlib is given, only the PDF's bytes; the export is a separate normal run.
pub fn set_preview(on: bool) {
    PREVIEW.with(|p| p.set(on));
}

pub fn preview() -> bool {
    PREVIEW.with(|p| p.get())
}

/// How many `pdftex_warn`s this thread has printed (to tell whether a
/// cached computation printed anything, `mapfile::MapCache`).
pub fn warnings_so_far() -> u64 {
    WARNINGS.with(|w| w.get())
}

/// Forget all C state (a new job in the same thread).
pub fn reset_state() {
    STATE.with(|s| *s.borrow_mut() = CState::default());
    crate::displaylist::engine_reset();
}

crate::codec_struct!(CState {
    utils,
    vf,
    avl,
    fonts,
    fonts_busy,
    out,
    img,
    dl
});

fn enc_of<T: crate::persist::Codec>(x: &T) -> Vec<u8> {
    let mut w = vec![];
    x.enc(&mut w);
    w
}

fn same_enc<T: crate::persist::Codec>(a: &T, b: &T) -> bool {
    enc_of(a) == enc_of(b)
}

fn same_shared<T: Clone + crate::persist::Codec>(
    a: &shared::Shared<T>,
    b: &shared::Shared<T>,
) -> bool {
    shared::Shared::ptr_eq(a, b) || same_enc(&**a, &**b)
}

impl CState {
    /// Whether two C states are the same (the convergence test, DESIGN.md
    /// §5.3). Shared parts that are the same copy are equal without a look;
    /// the rest is compared by its persisted encoding (which lists every
    /// field; maps in key order), the named-object maps by content, the
    /// image table by what each image is. A `false` only costs a missed
    /// convergence, so a part with no exact comparison compares unequal.
    pub fn same_as(&self, o: &CState) -> bool {
        let (f, g) = (&self.fonts, &o.fonts);
        same_enc(&self.utils, &o.utils)
            && same_shared(&self.vf, &o.vf)
            && self.avl.same_as(&o.avl)
            && self.fonts_busy == o.fonts_busy
            && same_enc(&self.out, &o.out)
            && f.map.same_as(&g.map)
            && same_shared(&f.enc, &g.enc)
            && same_shared(&f.wf, &g.wf)
            && same_shared(&f.tu, &g.tu)
            && (shared::Shared::ptr_eq(&self.img, &o.img) || self.img.same_as(&o.img))
    }
}

/// A copy of this thread's C state, for a checkpoint (`crate::checkpoint`).
/// Checkpoints are taken between commands, when the font backend is never
/// out (`with_fonts` runs inside one primitive). `Err` when a part holds
/// state a checkpoint cannot copy.
pub fn snapshot_state() -> Result<CState, String> {
    with_state(|s| {
        assert!(!s.fonts_busy, "checkpoint while the font backend is out");
        let mut c = s.clone();
        c.dl = crate::displaylist::snapshot();
        Ok(c)
    })
}

/// Replace this thread's C state (restoring a checkpoint).
pub fn restore_state(st: CState) {
    crate::displaylist::restore(&st.dl);
    STATE.with(|s| *s.borrow_mut() = st);
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
        WARNINGS.with(|w| w.set(w.get() + 1));
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
