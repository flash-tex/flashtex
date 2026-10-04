// Derived from pdfTeX's writet1.c (TeX Live 2026, pdfTeX 1.40.29),
// Copyright 1996-2023 Han The Thanh <thanh@pdftex.org>; GPL-2.0-or-later
// (crates/flashtex-engine/LICENSE). Modified for FlashTeX: ported to Rust
// line by line, then rewritten into this module on 2026-10-04
// (docs/design/engine-v2/REWRITE.md); git history dates each later change.

//! Type 1 font embedding and subsetting, and encoding files: pdfTeX's
//! `writet1.c`, rewritten.
//!
//! Rewritten into the modules below on 2026-10-04 (lane IDIOMATIC-REWRITE,
//! the pilot of docs/design/engine-v2/REWRITE.md), with the same output for
//! the same input. The C function each part stands for is named in its
//! comment.
//!
//! | module | what | writet1.c |
//! |---|---|---|
//! | [`cipher`] | the Type 1 encryption | `edecrypt`, `eencrypt`, `cdecrypt`, `cencrypt` |
//! | [`reader`] | a font file, a line at a time | `t1_getbyte`, `t1_getline`, `t1_scan_num` |
//! | [`encoding`] | StandardEncoding, `.enc` files | `standard_glyph_names`, `load_enc_file` |
//! | [`charstring`] | which subrs and glyphs a glyph needs | `cs_store`'s table, `cs_mark` |
//! | [`subset`] | embedding a font, whole or subset | `t1_include`, `t1_subset_*`, `t1_flush_cs` |
//!
//! The parts here print nothing and end no run: a font error is a
//! [`Fail`], which [`Globals::writet1`] reports as `pdftex_fail` does,
//! and warnings and the subset tag go through [`Host`], so each part can
//! be tested on its own. Where pdfTeX's behaviour is undefined (a
//! charstring that announces more bytes than it has, a blank missing
//! before a charstring's length), the port panicked; these are font
//! errors or keep what the line has.
#![forbid(unsafe_code)]

mod charstring;
mod cipher;
mod encoding;
mod reader;
mod subset;

pub(crate) use encoding::standard_glyph_name;

use super::fonts::{Fonts, GlyphNames};
use super::output::set_cur_file_name;
use super::writefont::FdEntry;
use crate::generated::Globals;
use crate::resolver::Format;
use std::collections::BTreeSet;

/// A font error: the message of pdfTeX's `pdftex_fail`, which ends the run.
#[derive(Debug)]
pub(super) struct Fail(pub String);

type Result<T, E = Fail> = std::result::Result<T, E>;

/// What embedding a font needs from the engine.
trait Host {
    /// `pdftex_warn`.
    fn warn(&mut self, msg: &[u8]);
    /// `make_subset_tag`: the six letters that name a subset.
    fn subset_tag(&mut self, glyphs: &BTreeSet<Vec<u8>>, fontname: &[u8]) -> [u8; 6];
}

impl Host for Globals {
    fn warn(&mut self, msg: &[u8]) {
        self.pdftex_warn_bytes(msg);
    }

    fn subset_tag(&mut self, glyphs: &BTreeSet<Vec<u8>>, fontname: &[u8]) -> [u8; 6] {
        self.make_subset_tag(glyphs.iter(), fontname)
    }
}

/// The persistent statics of writet1.c.
#[derive(Clone)]
pub struct Persist {
    /// cs_mark's `static integer lastargOtherSubr3 = 3`.
    last_arg_other_subr3: i32,
}
crate::codec_struct!(Persist {
    last_arg_other_subr3
});

impl Default for Persist {
    fn default() -> Self {
        Persist {
            last_arg_other_subr3: 3,
        }
    }
}

/// The results of a `writet1` run that `write_fontfile` needs.
pub struct T1Result {
    pub ff_found: bool,
    pub length1: i32,
    pub length2: i32,
    pub length3: i32,
}

impl Globals {
    /// `load_enc_file` (writet1.c): read encoding file `enc_name`; fails the
    /// run if it cannot be read.
    pub fn load_enc_file(&mut self, enc_name: &[u8]) -> GlyphNames {
        self.set_cur_file_name_str(Some(enc_name));
        let name = String::from_utf8_lossy(enc_name).into_owned();
        let Some((path, data)) = crate::system::find_file(&name, Format::Enc)
            .and_then(|p| std::fs::read(&p).ok().map(|d| (p, d)))
        else {
            self.pdftex_fail("cannot open encoding file for reading");
        };
        self.tex_printf(b"{");
        set_cur_file_name(Some(path.as_bytes()));
        self.tex_printf(path.as_bytes());
        let names = encoding::parse_enc_file(&data).unwrap_or_else(|e| self.pdftex_fail(&e.0));
        self.tex_printf(b"}");
        set_cur_file_name(None);
        names
    }

    /// `writet1` (writet1.c): embed the Type 1 font of `fd` into the font
    /// buffer, subset to its glyph tree unless its map entry says `<<`.
    pub fn writet1(&mut self, st: &mut Fonts, fd: &mut FdEntry, persist: &mut Persist) -> T1Result {
        let fm = st.map.fms[fd.fm].clone().expect("live map entry");
        let ff_name = fm.ff_name.clone().unwrap_or_default();
        // t1_open_fontfile
        let path = self.check_ff_exist(st, &ff_name, fm.is_truetype());
        let Some((path, font)) = path.and_then(|p| std::fs::read(&p).ok().map(|d| (p, d))) else {
            self.set_cur_file_name_str(Some(&ff_name));
            self.pdftex_fail("cannot open Type 1 font file for reading");
        };
        set_cur_file_name(Some(path.as_bytes()));
        // t1_init_params
        let (open, close): (&[u8], &[u8]) = if fm.is_subsetted() {
            (b"<", b">")
        } else {
            (b"<<", b">>")
        };
        self.tex_printf(open);
        self.tex_printf(path.as_bytes());
        let job = subset::Job {
            fm: &fm,
            fd,
            persist,
            fb_base: self.fb_offset(),
        };
        let font = subset::embed(&font, job, self).unwrap_or_else(|e| self.pdftex_fail(&e.0));
        self.tex_printf(close);
        set_cur_file_name(None);
        super::with_state(|s| s.out.fb.extend_from_slice(&font.bytes));
        T1Result {
            ff_found: true,
            length1: font.length1,
            length2: font.length2,
            length3: 0,
        }
    }
}
