//! `epdf.c`, ported: the font descriptors of fonts that an included PDF
//! uses and pdfTeX replaces with its own embedding ([`super::pdftoepdf`]),
//! and the glyphs they need.

use super::fonts::Fonts;
use super::writefont::{FdEntry, IntParm, STEMV_CODE};
use crate::generated::Globals;
use std::collections::BTreeSet;

impl Globals {
    /// `is_subsetable`: whether map entry `fm` asks for a subset.
    /// (C asserts the font is embedded, which `lookup_fontmap`'s entries
    /// always are; one that is not stops the run instead of aborting.)
    pub(crate) fn is_subsetable(&mut self, st: &Fonts, fm: usize) -> bool {
        let e = st.map.fms[fm].as_ref().expect("live map entry");
        if !e.is_included() {
            self.pdftex_fail("PDF inclusion: replaced font is not embedded");
        }
        e.is_subsetted()
    }

    /// `epdf_create_fontdescriptor`: the descriptor of map entry `fm`'s font
    /// file, made now (with a `/StemV` from the included PDF) if TeX and
    /// earlier inclusions have not made it.
    pub(crate) fn epdf_create_fontdescriptor(
        &mut self,
        st: &mut Fonts,
        fm: usize,
        stem_v: i32,
    ) -> usize {
        let (ff, slant, extend, ps_name) = {
            let e = st.map.fms[fm].as_ref().expect("live map entry");
            (
                e.ff_name.clone().unwrap_or_default(),
                e.slant,
                e.extend,
                e.ps_name.clone(),
            )
        };
        if let Some(fd) = Self::lookup_fd_entry(st, &ff, slant, extend) {
            return fd;
        }
        st.map.set_in_use(fm);
        let mut fd = FdEntry {
            fm,
            ..Default::default()
        };
        st.wf.fds.push(None);
        let id = st.wf.fds.len() - 1;
        fd.fd_objnum = self.pdf_new_objnum();
        // assert(fm->ps_name != NULL)
        let Some(ps_name) = ps_name else {
            self.pdftex_fail("PDF inclusion: map entry without a PostScript name");
        };
        fd.fontname = Some(ps_name); // just fallback
                                     // stemV must be copied
        fd.font_dim[STEMV_CODE] = IntParm {
            val: stem_v,
            set: true,
        };
        fd.gl_tree = Some(BTreeSet::new());
        st.wf.fds[id] = Some(fd);
        Self::register_fd_entry(st, id);
        id
    }

    /// `get_fn_objnum`: the object of the descriptor's font name (the
    /// included font dictionary's `/BaseFont`), numbered on first use.
    pub(crate) fn get_fn_objnum(&mut self, st: &mut Fonts, fd: usize) -> i32 {
        if st.wf.fds[fd].as_ref().unwrap().fn_objnum == 0 {
            let n = self.pdf_new_objnum();
            st.wf.fds[fd].as_mut().unwrap().fn_objnum = n;
        }
        st.wf.fds[fd].as_ref().unwrap().fn_objnum
    }

    /// `epdf_mark_glyphs`: the glyphs of a `/CharSet` (`/a/b/c`, with
    /// optional white space before and between the names) into the
    /// descriptor's glyph tree.
    pub(crate) fn epdf_mark_glyphs(st: &mut Fonts, fd: usize, charset: &[u8]) {
        let is_space = |c: u8| matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0c);
        let gl = st.wf.fds[fd]
            .as_mut()
            .unwrap()
            .gl_tree
            .get_or_insert_with(BTreeSet::new);
        // `charset` is NUL-free here (the caller cut it at the first NUL),
        // so C's writing of NULs into it only ends names.
        let mut c = charset;
        while let Some((&b, rest)) = c.split_first() {
            if !is_space(b) {
                break;
            }
            c = rest;
        }
        let q = c.len();
        let mut s = 1usize; // skip the leading character (the first slash)
        while s < q {
            let mut p = s;
            while p < q && c[p] != b'/' && !is_space(c[p]) {
                p += 1;
            }
            let name = &c[s..p];
            if p < q && is_space(c[p]) {
                p += 1;
                while p < q && is_space(c[p]) {
                    p += 1;
                }
            }
            // C writes a NUL at `p` here, then continues at p + 1, so the
            // character at `p` (a slash, or the first character after
            // blanks) is skipped
            gl.insert(name.to_vec());
            s = p + 1;
        }
    }

    /// `embed_whole_font`.
    pub(crate) fn embed_whole_font(st: &mut Fonts, fd: usize) {
        st.wf.fds[fd].as_mut().unwrap().all_glyphs = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marked(cs: &[u8]) -> Vec<Vec<u8>> {
        let mut st = Fonts::default();
        st.wf.fds.push(Some(FdEntry::default()));
        Globals::epdf_mark_glyphs(&mut st, 0, cs);
        st.wf.fds[0]
            .as_ref()
            .unwrap()
            .gl_tree
            .iter()
            .flatten()
            .cloned()
            .collect()
    }

    #[test]
    fn charsets_as_epdf_c_reads_them() {
        let v = |s: &[&str]| s.iter().map(|x| x.as_bytes().to_vec()).collect::<Vec<_>>();
        assert_eq!(marked(b"/a/b/c"), v(&["a", "b", "c"]));
        assert_eq!(marked(b"  /one /two\n/three"), v(&["one", "three", "two"]));
        // blanks then a name without a slash: C skips its first letter
        assert_eq!(marked(b"/a  xb"), v(&["a", "b"]));
        assert_eq!(marked(b""), Vec::<Vec<u8>>::new());
    }
}
