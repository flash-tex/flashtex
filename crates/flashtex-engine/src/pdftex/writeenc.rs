//! `writeenc.c`, ported: encoding vectors (`.enc` files) and their
//! `/Encoding` dictionaries.

use super::fonts::{Fonts, GlyphNames, NOTDEF};
use crate::generated::Globals;
use std::collections::{BTreeMap, BTreeSet};

/// `fe_entry` (ptexlib.h).
#[derive(Clone)]
pub struct FeEntry {
    /// `fe_objnum`: the `/Encoding` object, or 0 if none is written.
    pub fe_objnum: i32,
    /// `name`: the encoding file name.
    pub name: Vec<u8>,
    /// `glyph_names`.
    pub glyph_names: GlyphNames,
    /// `tx_tree`: the encoding positions TeX used; `None` until one is.
    pub tx_tree: Option<BTreeSet<i32>>,
}
crate::codec_struct!(FeEntry {
    fe_objnum,
    name,
    glyph_names,
    tx_tree
});

#[derive(Default, Clone)]
pub struct State {
    pub fes: Vec<FeEntry>,
    /// `fe_tree`: encodings by file name.
    fe_tree: BTreeMap<Vec<u8>, usize>,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot.
crate::codec_struct!(State { fes, fe_tree });

impl Globals {
    /// `get_fe_entry`: the encoding read from file `s`, read now if it has
    /// not been. `load_enc_file` fails the run if it cannot be read.
    pub fn get_fe_entry(&mut self, st: &mut Fonts, s: &[u8]) -> Option<usize> {
        if let Some(&fe) = st.enc.fe_tree.get(s) {
            return Some(fe);
        }
        let gl = self.load_enc_file(s);
        let fe = st.enc.fes.len();
        st.enc.fes.push(FeEntry {
            fe_objnum: 0,
            name: s.to_vec(),
            glyph_names: gl,
            tx_tree: None,
        });
        st.enc.fe_tree.insert(s.to_vec(), fe);
        Some(fe)
    }

    /// `epdf_write_enc`: an `/Encoding` of every named slot (PDF inclusion).
    pub fn epdf_write_enc(&mut self, glyph_names: &GlyphNames, fe_objnum: i32) {
        self.pdf_begin_dict(fe_objnum, 1);
        self.pdf_puts(b"/Type /Encoding\n");
        self.pdf_puts(b"/Differences [");
        let mut i_old = -2i32;
        for i in 0..256i32 {
            let g = &glyph_names[i as usize];
            if g.as_slice() != NOTDEF {
                self.write_difference(i, i_old, g);
                i_old = i;
            }
        }
        self.pdf_puts(b"]\n");
        self.pdf_end_dict();
    }

    /// One `/Differences` entry: `/name`, or `code/name` after a gap.
    fn write_difference(&mut self, i: i32, i_old: i32, g: &[u8]) {
        let mut s = Vec::new();
        if i == i_old + 1 {
            // no gap
            s.push(b'/');
        } else if i_old == -2 {
            s.extend_from_slice(format!("{i}/").as_bytes());
        } else {
            s.extend_from_slice(format!(" {i}/").as_bytes());
        }
        s.extend_from_slice(g);
        self.pdf_printf(&s);
    }

    /// `write_enc`: an `/Encoding` of the positions TeX used.
    fn write_enc(&mut self, glyph_names: &GlyphNames, tx_tree: &BTreeSet<i32>, fe_objnum: i32) {
        self.pdf_begin_dict(fe_objnum, 1);
        self.pdf_puts(b"/Type /Encoding\n");
        self.pdf_puts(b"/Differences [");
        let mut i_old = -2i32;
        for &p in tx_tree {
            self.write_difference(p, i_old, &glyph_names[p as usize]);
            i_old = p;
        }
        self.pdf_puts(b"]\n");
        self.pdf_end_dict();
    }

    /// `write_fontencodings`: every encoding that got an object number.
    pub fn write_fontencodings(&mut self, st: &mut Fonts) {
        let order: Vec<usize> = st.enc.fe_tree.values().copied().collect();
        for fe in order {
            let e = &st.enc.fes[fe];
            if e.fe_objnum != 0 {
                let (names, objnum) = (e.glyph_names.clone(), e.fe_objnum);
                let tx = e.tx_tree.clone().unwrap_or_default();
                self.write_enc(&names, &tx, objnum);
            }
        }
    }
}
