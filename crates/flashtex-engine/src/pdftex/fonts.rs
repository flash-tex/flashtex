//! The state of pdfTeX's font backend, shared by its C files' ports:
//! [`super::mapfile`], [`super::writeenc`], [`super::writefont`],
//! [`super::writet1`], [`super::writet3`] and [`super::tounicode`].
//!
//! In C these are file-level globals and AVL trees. Here they are one
//! struct, taken out of the thread's [`super::CState`] for the length of one
//! call from pdftex.web ([`Globals::with_fonts`]); none of those calls
//! re-enter another. Every AVL tree becomes a `BTreeMap`/`BTreeSet` whose key
//! order is the tree's comparison function (`strcmp` is byte order), so every
//! traversal visits the entries in the C order.

use super::shared::Shared;
use super::with_state;
use crate::generated::Globals;

/// `notdef` (macnames.c): the glyph name of an unused slot. C compares it by
/// pointer; every name equal to `.notdef` is that pointer (the readers never
/// copy one), so comparing the bytes is the same.
pub const NOTDEF: &[u8] = b".notdef";

/// A glyph-name vector (`char **glyph_names`), 256 entries.
pub type GlyphNames = Vec<Vec<u8>>;

/// 256 `notdef`s.
pub fn notdef_names() -> GlyphNames {
    vec![NOTDEF.to_vec(); 256]
}

/// The map is shared inside (`mapfile::State`); the encodings, font
/// descriptors and the glyph-to-Unicode table are [`Shared`] whole: they
/// change when a font is first used or written, not on every page.
#[derive(Default, Clone)]
pub struct Fonts {
    pub map: super::mapfile::State,
    pub enc: Shared<super::writeenc::State>,
    pub wf: Shared<super::writefont::State>,
    pub tu: Shared<super::tounicode::State>,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot.
crate::codec_struct!(Fonts { map, enc, wf, tu });

impl Globals {
    /// Run `f` with the font backend's state.
    pub fn with_fonts<R>(&mut self, f: impl FnOnce(&mut Globals, &mut Fonts) -> R) -> R {
        let mut st = with_state(|s| {
            assert!(!s.fonts_busy, "pdfTeX's font backend was re-entered");
            s.fonts_busy = true;
            std::mem::take(&mut s.fonts)
        });
        let r = f(self, &mut st);
        with_state(|s| {
            s.fonts = st;
            s.fonts_busy = false;
        });
        r
    }

    /// `makecstring(s)`: the bytes of string `s` up to its first NUL.
    pub fn c_string(&self, s: i32) -> Vec<u8> {
        let mut b = self.str_bytes(s);
        if let Some(i) = b.iter().position(|&c| c == 0) {
            b.truncate(i);
        }
        b
    }

    /// `getnullstr()`: the number of the empty string `""` of the string
    /// pool, the initial `\pdffontattr` of every font.
    pub fn null_str(&self) -> i32 {
        let mut s = 256;
        while s < self.str_ptr {
            if self.str_start[s as usize + 1] == self.str_start[s as usize] {
                return s;
            }
            s += 1;
        }
        0
    }
}
