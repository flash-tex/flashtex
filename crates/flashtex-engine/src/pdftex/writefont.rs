//! `writefont.c`, ported: font dictionaries, font descriptors, `/Widths`
//! arrays, and the embedding of font files.

use super::fonts::{Fonts, GlyphNames, NOTDEF};
use super::mapfile::{
    check_std_t1font, FmEntry, FD_FLAGS_DEFAULT_EMBED, FD_FLAGS_DEFAULT_NON_EMBED,
    FD_FLAGS_NOT_SET_IN_MAPLINE,
};
use crate::generated::Globals;
use std::collections::{BTreeMap, BTreeSet};

pub const ASCENT_CODE: usize = 0;
pub const CAPHEIGHT_CODE: usize = 1;
pub const DESCENT_CODE: usize = 2;
pub const ITALIC_ANGLE_CODE: usize = 3;
pub const STEMV_CODE: usize = 4;
pub const XHEIGHT_CODE: usize = 5;
pub const FONTBBOX1_CODE: usize = 6;
pub const FONTBBOX2_CODE: usize = 7;
pub const FONTBBOX3_CODE: usize = 8;
pub const FONTBBOX4_CODE: usize = 9;
pub const FONTNAME_CODE: usize = 10;
const GEN_KEY_NUM: usize = XHEIGHT_CODE + 1;
const INT_KEYS_NUM: usize = FONTBBOX4_CODE + 1;
const FONT_KEYS_NUM: usize = FONTNAME_CODE + 1;

/// `font_key` (ptexlib.h): (PDF name, Type 1 name).
pub const FONT_KEYS: [(&[u8], &[u8]); FONT_KEYS_NUM] = [
    (b"Ascent", b"Ascender"),
    (b"CapHeight", b"CapHeight"),
    (b"Descent", b"Descender"),
    (b"ItalicAngle", b"ItalicAngle"),
    (b"StemV", b"StdVW"),
    (b"XHeight", b"XHeight"),
    (b"FontBBox", b"FontBBox"),
    (b"", b""),
    (b"", b""),
    (b"", b""),
    (b"FontName", b"FontName"),
];

/// `intparm`.
#[derive(Clone, Copy, Default)]
pub struct IntParm {
    pub val: i32,
    pub set: bool,
}
crate::codec_struct!(IntParm { val, set });

/// `fd_entry` (ptexlib.h): a `/FontDescriptor`.
#[derive(Default, Clone)]
pub struct FdEntry {
    pub fd_objnum: i32,
    pub fontname: Option<Vec<u8>>,
    pub subset_tag: Option<[u8; 6]>,
    pub ff_found: bool,
    pub ff_objnum: i32,
    pub fn_objnum: i32,
    pub all_glyphs: bool,
    pub write_ttf_glyph_names: bool,
    pub font_dim: [IntParm; FONT_KEYS_NUM],
    pub fe: Option<usize>,
    pub builtin_glyph_names: Option<GlyphNames>,
    /// The map entry (arena index).
    pub fm: usize,
    pub tx_tree: Option<BTreeSet<i32>>,
    pub gl_tree: Option<BTreeSet<Vec<u8>>>,
}
crate::codec_struct!(FdEntry {
    fd_objnum,
    fontname,
    subset_tag,
    ff_found,
    ff_objnum,
    fn_objnum,
    all_glyphs,
    write_ttf_glyph_names,
    font_dim,
    fe,
    builtin_glyph_names,
    fm,
    tx_tree,
    gl_tree
});

/// `cw_entry` (its `width` array is only written, so it is not kept).
#[derive(Clone)]
struct CwEntry {
    cw_objnum: i32,
}
crate::codec_struct!(CwEntry { cw_objnum });

/// `fo_entry` (ptexlib.h): a `/Font` dictionary.
#[derive(Clone)]
struct FoEntry {
    fo_objnum: i32,
    tex_font: i32,
    fm: usize,
    fd: Option<usize>,
    fe: Option<usize>,
    cw: Option<CwEntry>,
    first_char: i32,
    last_char: i32,
    tounicode_objnum: i32,
}
crate::codec_struct!(FoEntry {
    fo_objnum,
    tex_font,
    fm,
    fd,
    fe,
    cw,
    first_char,
    last_char,
    tounicode_objnum
});

#[derive(Default, Clone)]
pub struct State {
    /// Font descriptors; `None` while one is out being written.
    pub fds: Vec<Option<FdEntry>>,
    /// `fd_tree`: descriptors of font files, by (file, slant, extend).
    fd_tree: BTreeMap<(Vec<u8>, i32, i32), usize>,
    fos: Vec<FoEntry>,
    /// `fo_tree`: Type 1 font dictionaries by TFM name.
    fo_tree: BTreeMap<Vec<u8>, usize>,
    /// writet1.c's persistent statics.
    t1: super::writet1::Persist,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot.
crate::codec_struct!(State {
    fds,
    fd_tree,
    fos,
    fo_tree,
    t1
});

fn fm_of(st: &Fonts, id: usize) -> &FmEntry {
    st.map.fms[id].as_ref().expect("live map entry")
}

impl Globals {
    /// `preset_fontmetrics`: fallback metrics from the TFM file; the font
    /// file replaces them where it has them.
    fn preset_fontmetrics(&mut self, fd: &mut FdEntry, f: i32) {
        let size = self.pdf_font_size[f as usize];
        let angle = -((self.get_slant(f) as f64 / 65536.0).atan()) * (180.0 / std::f64::consts::PI);
        fd.font_dim[ITALIC_ANGLE_CODE].val = self.divide_scaled(angle as i32, size, 3);
        let h = self.get_charheight(f, b'h' as i32);
        fd.font_dim[ASCENT_CODE].val = self.divide_scaled(h, size, 3);
        let hh = self.get_charheight(f, b'H' as i32);
        fd.font_dim[CAPHEIGHT_CODE].val = self.divide_scaled(hh, size, 3);
        let y = self.get_chardepth(f, b'y' as i32);
        let i = -self.divide_scaled(y, size, 3);
        fd.font_dim[DESCENT_CODE].val = if i < 0 { i } else { 0 };
        let dot = self.get_charwidth(f, b'.' as i32);
        fd.font_dim[STEMV_CODE].val = self.divide_scaled(dot / 3, size, 3);
        let xh = self.get_x_height(f);
        fd.font_dim[XHEIGHT_CODE].val = self.divide_scaled(xh, size, 3);
        fd.font_dim[FONTBBOX1_CODE].val = 0;
        fd.font_dim[FONTBBOX2_CODE].val = fd.font_dim[DESCENT_CODE].val;
        let quad = self.get_quad(f);
        fd.font_dim[FONTBBOX3_CODE].val = self.divide_scaled(quad, size, 3);
        fd.font_dim[FONTBBOX4_CODE].val = fd.font_dim[CAPHEIGHT_CODE]
            .val
            .max(fd.font_dim[ASCENT_CODE].val);
        for d in fd.font_dim.iter_mut().take(INT_KEYS_NUM) {
            d.set = true;
        }
    }

    /// `fix_fontmetrics`.
    fn fix_fontmetrics(&mut self, fd: &mut FdEntry, ff_name: &[u8]) {
        let p = &mut fd.font_dim;
        if !p[FONTBBOX1_CODE].set
            || !p[FONTBBOX2_CODE].set
            || !p[FONTBBOX3_CODE].set
            || !p[FONTBBOX4_CODE].set
        {
            let mut msg = b"font `".to_vec();
            msg.extend_from_slice(ff_name);
            msg.extend_from_slice(b"' doesn't have a BoundingBox");
            self.pdftex_warn_bytes(&msg);
            return;
        }
        if !p[ASCENT_CODE].set {
            p[ASCENT_CODE] = IntParm {
                val: p[FONTBBOX4_CODE].val,
                set: true,
            };
        }
        if !p[DESCENT_CODE].set {
            p[DESCENT_CODE] = IntParm {
                val: p[FONTBBOX2_CODE].val,
                set: true,
            };
        }
        if !p[CAPHEIGHT_CODE].set {
            p[CAPHEIGHT_CODE] = IntParm {
                val: p[FONTBBOX4_CODE].val,
                set: true,
            };
        }
    }

    /// `write_fontmetrics`.
    fn write_fontmetrics(&mut self, fd: &mut FdEntry, ff_name: &[u8]) {
        self.fix_fontmetrics(fd, ff_name);
        let d = fd.font_dim;
        if d[FONTBBOX1_CODE].set
            && d[FONTBBOX2_CODE].set
            && d[FONTBBOX3_CODE].set
            && d[FONTBBOX4_CODE].set
        {
            let s = format!(
                "/FontBBox [{} {} {} {}]\n",
                d[FONTBBOX1_CODE].val,
                d[FONTBBOX2_CODE].val,
                d[FONTBBOX3_CODE].val,
                d[FONTBBOX4_CODE].val
            );
            self.pdf_printf(s.as_bytes());
        }
        for (i, key) in FONT_KEYS.iter().enumerate().take(GEN_KEY_NUM) {
            if d[i].set {
                let mut s = b"/".to_vec();
                s.extend_from_slice(key.0);
                s.extend_from_slice(format!(" {}\n", d[i].val).as_bytes());
                self.pdf_printf(&s);
            }
        }
    }

    /// `write_fontname`: `/key /TAG+Name`.
    fn write_fontname(&mut self, fd: &FdEntry, key: Option<&[u8]>) {
        self.pdf_puts(b"/");
        if let Some(k) = key {
            let mut s = k.to_vec();
            s.extend_from_slice(b" /");
            self.pdf_printf(&s);
        }
        if let Some(tag) = fd.subset_tag {
            let mut s = tag.to_vec();
            s.push(b'+');
            self.pdf_printf(&s);
        }
        let mut s = fd.fontname.clone().unwrap_or_default();
        s.push(b'\n');
        self.pdf_printf(&s);
    }

    /// `write_fontname_object`.
    fn write_fontname_object(&mut self, fd: &FdEntry) {
        self.pdf_begin_obj(fd.fn_objnum, 1);
        self.write_fontname(fd, None);
        self.pdf_end_obj();
    }

    /// `lookup_fd_entry`.
    pub fn lookup_fd_entry(st: &Fonts, ff_name: &[u8], slant: i32, extend: i32) -> Option<usize> {
        st.wf
            .fd_tree
            .get(&(ff_name.to_vec(), slant, extend))
            .copied()
    }

    /// `register_fd_entry`.
    pub fn register_fd_entry(st: &mut Fonts, fd: usize) {
        let e = st.wf.fds[fd].as_ref().unwrap();
        let fm = fm_of(st, e.fm);
        let key = (fm.ff_name.clone().unwrap_or_default(), fm.slant, fm.extend);
        st.wf.fd_tree.entry(key).or_insert(fd);
    }

    /// `create_fontdescriptor`.
    fn create_fontdescriptor(&mut self, st: &mut Fonts, fo: usize, f: i32) {
        let (fm_id, fe) = (st.wf.fos[fo].fm, st.wf.fos[fo].fe);
        let fm = fm_of(st, fm_id);
        // preset_fontname: just fallback
        let fontname = fm.ps_name.clone().unwrap_or_else(|| fm.tfm_name.clone());
        let mut fd = FdEntry {
            fontname: Some(fontname),
            fe,        // encoding needed by TrueType writing
            fm: fm_id, // map entry needed by TrueType writing
            gl_tree: Some(BTreeSet::new()),
            ..Default::default()
        };
        self.preset_fontmetrics(&mut fd, f);
        st.wf.fds.push(Some(fd));
        st.wf.fos[fo].fd = Some(st.wf.fds.len() - 1);
    }

    /// `mark_reenc_glyphs`: the glyph names of the used characters, from the
    /// `.enc` file, into the descriptor's glyph tree.
    fn mark_reenc_glyphs(&mut self, st: &mut Fonts, fo: usize, f: i32) {
        let o = &st.wf.fos[fo];
        if !fm_of(st, o.fm).is_subsetted() {
            return;
        }
        let (first, last, fe, fd) = (o.first_char, o.last_char, o.fe.unwrap(), o.fd.unwrap());
        for i in first..=last {
            if self.pdf_char_marked(f, i) {
                let g = &st.enc.fes[fe].glyph_names[i as usize];
                if g.as_slice() != NOTDEF {
                    let g = g.clone();
                    let gl = st.wf.fds[fd]
                        .as_mut()
                        .unwrap()
                        .gl_tree
                        .get_or_insert_with(Default::default);
                    gl.insert(g);
                }
            }
        }
    }

    /// `mark_chars`: the used characters of `f` in `first..=last`.
    fn mark_chars(&mut self, tx: &mut Option<BTreeSet<i32>>, first: i32, last: i32, f: i32) {
        let t = tx.get_or_insert_with(BTreeSet::new);
        for i in first..=last {
            if self.pdf_char_marked(f, i) {
                t.insert(i);
            }
        }
    }

    /// `get_char_range`: the first and last used character of `f`.
    fn get_char_range(&mut self, f: i32) -> (i32, i32) {
        let (bc, ec) = (self.font_bc[f as usize], self.font_ec[f as usize]);
        let mut i = bc;
        while i <= ec {
            if self.pdf_char_marked(f, i) {
                break;
            }
            i += 1;
        }
        let first = i;
        let mut i = ec;
        while i >= bc {
            if self.pdf_char_marked(f, i) {
                break;
            }
            i -= 1;
        }
        let last = i;
        if first > last || !self.pdf_char_marked(f, first) {
            // no character used from this font
            return (1, 0);
        }
        (first, last)
    }

    /// `create_charwidth_array` and `write_charwidth_array`.
    fn charwidth_array(&mut self, st: &mut Fonts, fo: usize, f: i32) {
        let (first, last) = (st.wf.fos[fo].first_char, st.wf.fos[fo].last_char);
        let mut width = [0i32; 256];
        for i in first..=last {
            let w = self.get_charwidth(f, i);
            width[i as usize] = self.divide_scaled(w, self.pdf_font_size[f as usize], 4);
        }
        let cw_objnum = self.pdf_new_objnum();
        self.pdf_begin_obj(cw_objnum, 1);
        self.pdf_puts(b"[");
        for i in first..=last {
            // see adv_char_width() in pdftex.web
            self.pdf_printf(format!("{}", width[i as usize] / 10).as_bytes());
            let j = width[i as usize] % 10;
            if j != 0 {
                self.pdf_printf(format!(".{j}").as_bytes());
            }
            if i != last {
                self.pdf_puts(b" ");
            }
        }
        self.pdf_puts(b"]\n");
        self.pdf_end_obj();
        st.wf.fos[fo].cw = Some(CwEntry { cw_objnum });
    }

    /// `write_fontfile`: embed the font file of `fd`.
    fn write_fontfile(&mut self, st: &mut Fonts, fd: &mut FdEntry) {
        let fm = fm_of(st, fd.fm).clone();
        let r = if fm.is_type1() {
            let mut persist = std::mem::take(&mut st.wf.t1);
            let r = self.writet1(st, fd, &mut persist);
            st.wf.t1 = persist;
            r
        } else {
            // writettf/writeotf: TrueType and OpenType embedding is not
            // ported (DESIGN.md P3 scope: Type 1 text fonts first).
            let ff =
                String::from_utf8_lossy(fm.ff_name.as_deref().unwrap_or_default()).into_owned();
            self.pdftex_fail(&format!(
                "cannot embed `{ff}': TrueType/OpenType font embedding is not implemented yet"
            ));
        };
        fd.ff_found = r.ff_found;
        if !fd.ff_found {
            return;
        }
        fd.ff_objnum = self.pdf_new_objnum();
        self.pdf_begin_dict(fd.ff_objnum, 0); // font file stream
        self.pdf_printf(
            format!(
                "/Length1 {}\n/Length2 {}\n/Length3 {}\n",
                r.length1, r.length2, r.length3
            )
            .as_bytes(),
        );
        self.pdf_begin_stream();
        self.fb_flush();
        self.pdf_end_stream();
    }

    /// `write_fontdescriptor`.
    fn write_fontdescriptor(&mut self, st: &mut Fonts, fd_id: usize) {
        // Indices for << start with 0, but bits start with 1, so the
        // numbers for << are 1 lower than the bits in table 5.20.
        const STD_FLAGS: [i32; 14] = [
            1 + 2 + (1 << 5),                        // Courier
            1 + 2 + (1 << 5) + (1 << 18),            // Courier-Bold
            1 + 2 + (1 << 5) + (1 << 6),             // Courier-Oblique
            1 + 2 + (1 << 5) + (1 << 6) + (1 << 18), // Courier-BoldOblique
            1 << 5,                                  // Helvetica
            (1 << 5) + (1 << 18),                    // Helvetica-Bold
            (1 << 5) + (1 << 6),                     // Helvetica-Oblique
            (1 << 5) + (1 << 6) + (1 << 18),         // Helvetica-BoldOblique
            4,                                       // Symbol
            2 + (1 << 5),                            // Times-Roman
            2 + (1 << 5) + (1 << 18),                // Times-Bold
            2 + (1 << 5) + (1 << 6),                 // Times-Italic
            2 + (1 << 5) + (1 << 6) + (1 << 18),     // Times-BoldItalic
            4,                                       // ZapfDingbats
        ];
        let mut fd = st.wf.fds[fd_id]
            .take()
            .expect("descriptor not being written");
        let fm = fm_of(st, fd.fm).clone();
        if fm.is_fontfile() {
            self.write_fontfile(st, &mut fd); // sets fd.ff_found if the font file is found
        }
        if fd.fn_objnum != 0 {
            self.write_fontname_object(&fd);
        }
        if fd.fd_objnum == 0 {
            fd.fd_objnum = self.pdf_new_objnum();
        }
        self.pdf_begin_dict(fd.fd_objnum, 1);
        self.pdf_puts(b"/Type /FontDescriptor\n");
        self.write_fontname(&fd, Some(b"FontName"));
        let fd_flags = if fm.fd_flags != FD_FLAGS_NOT_SET_IN_MAPLINE {
            fm.fd_flags
        } else if fd.ff_found {
            FD_FLAGS_DEFAULT_EMBED
        } else {
            let flags = if fm.is_std_t1font() {
                STD_FLAGS[check_std_t1font(fm.ps_name.as_deref().unwrap_or_default()) as usize]
            } else {
                FD_FLAGS_DEFAULT_NON_EMBED
            };
            let mut msg = b"No flags specified for non-embedded font `".to_vec();
            msg.extend_from_slice(fm.ps_name.as_deref().unwrap_or(b"No name given"));
            msg.extend_from_slice(b"' (");
            msg.extend_from_slice(&fm.tfm_name);
            msg.extend_from_slice(format!(") (I'm using {flags}): fix your map entry.").as_bytes());
            self.pdftex_warn_bytes(&msg);
            flags
        };
        self.pdf_printf(format!("/Flags {fd_flags}\n").as_bytes());
        let ff_name = fm.ff_name.clone().unwrap_or_default();
        self.write_fontmetrics(&mut fd, &ff_name);
        if fd.ff_found {
            if self.get_pdf_omit_charset() == 0 && fm.is_subsetted() && fm.is_type1() {
                // Whether CharSet is output can be controlled by the user at
                // runtime via \pdfomitcharset (see writefont.c).
                self.pdf_puts(b"/CharSet (");
                for glyph in fd.gl_tree.iter().flatten() {
                    let mut s = b"/".to_vec();
                    s.extend_from_slice(glyph);
                    self.pdf_printf(&s);
                }
                self.pdf_puts(b")\n");
            }
            if fm.is_type1() {
                self.pdf_printf(format!("/FontFile {} 0 R\n", fd.ff_objnum).as_bytes());
            } else if fm.is_truetype() {
                self.pdf_printf(format!("/FontFile2 {} 0 R\n", fd.ff_objnum).as_bytes());
            } else if fm.is_opentype() {
                self.pdf_printf(format!("/FontFile3 {} 0 R\n", fd.ff_objnum).as_bytes());
            }
        }
        self.pdf_end_dict();
        st.wf.fds[fd_id] = Some(fd);
    }

    /// `write_fontdescriptors`: every registered descriptor, in `fd_tree`
    /// order.
    fn write_fontdescriptors(&mut self, st: &mut Fonts) {
        let order: Vec<usize> = st.wf.fd_tree.values().copied().collect();
        for fd in order {
            self.write_fontdescriptor(st, fd);
        }
    }

    /// `write_fontdictionary`.
    fn write_fontdictionary(&mut self, st: &mut Fonts, fo: usize) {
        let (fm_id, fd, fe, tex_font) = {
            let o = &st.wf.fos[fo];
            (o.fm, o.fd, o.fe, o.tex_font)
        };
        let fm = fm_of(st, fm_id).clone();
        // write ToUnicode entry if needed. (The second test is writefont.c's
        // own: `(fd != NULL && tfm_name != NULL && strcmp(...)) == 0`.)
        let dummy_space = !(fd.is_some() && fm.tfm_name.as_slice() != b"dummy-space");
        if (self.fixed_gen_tounicode > 0
            && fd.is_some()
            && !self.pdf_font_nobuiltin_tounicode[tex_font as usize])
            || dummy_space
        {
            if let Some(fe) = fe {
                let (names, name) = (
                    st.enc.fes[fe].glyph_names.clone(),
                    st.enc.fes[fe].name.clone(),
                );
                st.wf.fos[fo].tounicode_objnum =
                    self.write_tounicode(st, &names, &fm.tfm_name, Some(&name));
            } else if fm.is_type1() {
                let names =
                    fd.and_then(|d| st.wf.fds[d].as_ref().unwrap().builtin_glyph_names.clone());
                let Some(names) = names else {
                    self.pdftex_fail("builtin glyph names is empty");
                };
                st.wf.fos[fo].tounicode_objnum =
                    self.write_tounicode(st, &names, &fm.tfm_name, None);
            }
        }
        let o = &st.wf.fos[fo];
        let (fo_objnum, first, last, tu) =
            (o.fo_objnum, o.first_char, o.last_char, o.tounicode_objnum);
        let cw_objnum = o.cw.as_ref().map(|c| c.cw_objnum).unwrap_or(0);
        self.pdf_begin_dict(fo_objnum, 1);
        self.pdf_puts(b"/Type /Font\n");
        self.pdf_puts(b"/Subtype /");
        if fm.is_type1() || fm.is_opentype() {
            self.pdf_printf(b"Type1\n");
        } else if fm.is_truetype() {
            self.pdf_printf(b"TrueType\n");
        }
        let fd_e = st.wf.fds[fd.expect("font descriptor")].as_ref().unwrap();
        let fd_objnum = fd_e.fd_objnum;
        // write_fontname needs the descriptor; copy what it reads
        let name_view = FdEntry {
            fontname: fd_e.fontname.clone(),
            subset_tag: fd_e.subset_tag,
            ..Default::default()
        };
        self.write_fontname(&name_view, Some(b"BaseFont"));
        self.pdf_printf(format!("/FontDescriptor {fd_objnum} 0 R\n").as_bytes());
        self.pdf_printf(
            format!("/FirstChar {first}\n/LastChar {last}\n/Widths {cw_objnum} 0 R\n").as_bytes(),
        );
        if let Some(fe) = fe {
            let fe_objnum = st.enc.fes[fe].fe_objnum;
            if (fm.is_type1() || fm.is_opentype()) && fe_objnum != 0 {
                self.pdf_printf(format!("/Encoding {fe_objnum} 0 R\n").as_bytes());
            }
        }
        if tu != 0 {
            self.pdf_printf(format!("/ToUnicode {tu} 0 R\n").as_bytes());
        }
        let attr = self.pdf_font_attr[tex_font as usize];
        if attr != self.null_str() {
            self.pdf_print(attr);
            self.pdf_puts(b"\n");
        }
        self.pdf_end_dict();
    }

    /// `write_fontdictionaries`: every Type 1 font dictionary, in
    /// `fo_tree` order.
    fn write_fontdictionaries(&mut self, st: &mut Fonts) {
        let order: Vec<usize> = st.wf.fo_tree.values().copied().collect();
        for fo in order {
            self.write_fontdictionary(st, fo);
        }
    }

    /// `writefontstuff` (writefont.c): the final flush of all font objects,
    /// from pdftex.web's `@<Output fonts definition@>`.
    pub fn write_fontstuff(&mut self) {
        self.with_fonts(|g, st| {
            g.write_fontdescriptors(st);
            g.write_fontencodings(st); // see writeenc.c
            g.write_fontdictionaries(st);
        });
    }

    /// `create_fontdictionary`.
    fn create_fontdictionary(&mut self, st: &mut Fonts, fm_id: usize, font_objnum: i32, f: i32) {
        let (first_char, last_char) = self.get_char_range(f);
        let fm = fm_of(st, fm_id).clone();
        st.wf.fos.push(FoEntry {
            fo_objnum: font_objnum,
            tex_font: f,
            fm: fm_id,
            fd: None,
            fe: None,
            cw: None,
            first_char,
            last_char,
            tounicode_objnum: 0,
        });
        let fo = st.wf.fos.len() - 1;
        if let Some(enc) = &fm.encname {
            // at least the map entry tells so
            let fe = self.get_fe_entry(st, enc);
            st.wf.fos[fo].fe = fe;
            if let Some(fe) = fe {
                if fm.is_type1() || fm.is_opentype() {
                    if st.enc.fes[fe].fe_objnum == 0 {
                        st.enc.fes[fe].fe_objnum = self.pdf_new_objnum(); // then it will be written out
                    }
                    // mark encoding pairs used by TeX to optimize encoding vector
                    let mut tx = st.enc.fes[fe].tx_tree.take();
                    self.mark_chars(&mut tx, first_char, last_char, f);
                    st.enc.fes[fe].tx_tree = tx;
                }
            }
        }
        if !fm.is_builtin() {
            if fm.is_type1() {
                let ff = fm.ff_name.clone().unwrap_or_default();
                match Self::lookup_fd_entry(st, &ff, fm.slant, fm.extend) {
                    Some(fd) => st.wf.fos[fo].fd = Some(fd),
                    None => {
                        self.create_fontdescriptor(st, fo, f);
                        let fd = st.wf.fos[fo].fd.unwrap();
                        Self::register_fd_entry(st, fd);
                    }
                }
            } else {
                self.create_fontdescriptor(st, fo, f);
            }
            self.charwidth_array(st, fo, f);
            let fd = st.wf.fos[fo].fd.unwrap();
            if st.wf.fos[fo].fe.is_some() {
                self.mark_reenc_glyphs(st, fo, f);
                if !fm.is_type1() {
                    // mark reencoded characters as chars on TeX level
                    let mut tx = st.wf.fds[fd].as_mut().unwrap().tx_tree.take();
                    self.mark_chars(&mut tx, first_char, last_char, f);
                    let e = st.wf.fds[fd].as_mut().unwrap();
                    e.tx_tree = tx;
                    if fm.is_truetype() {
                        e.write_ttf_glyph_names = true;
                    }
                }
            } else {
                // mark non-reencoded characters as chars on TeX level
                let mut tx = st.wf.fds[fd].as_mut().unwrap().tx_tree.take();
                self.mark_chars(&mut tx, first_char, last_char, f);
                st.wf.fds[fd].as_mut().unwrap().tx_tree = tx;
            }
            if !fm.is_type1() {
                self.write_fontdescriptor(st, fd);
            }
        } else {
            // builtin fonts still need the /Widths array and /FontDescriptor
            // (to avoid error 'font FOO contains bad /BBox')
            self.charwidth_array(st, fo, f);
            self.create_fontdescriptor(st, fo, f);
            let fd = st.wf.fos[fo].fd.unwrap();
            self.write_fontdescriptor(st, fd);
            if !fm.is_std_t1font() {
                let mut msg = b"font `".to_vec();
                msg.extend_from_slice(fm.ps_name.as_deref().unwrap_or(b"(null)"));
                msg.extend_from_slice(
                    b"' is not a standard font; I suppose it is available to your PDF viewer then",
                );
                self.pdftex_warn_bytes(&msg);
            }
        }
        if fm.is_type1() {
            st.wf.fo_tree.entry(fm.tfm_name.clone()).or_insert(fo);
        } else {
            self.write_fontdictionary(st, fo);
        }
    }

    /// `font_is_used`: whether any character of `f` is used.
    fn font_is_used(&mut self, f: i32) -> bool {
        let (bc, ec) = (self.font_bc[f as usize], self.font_ec[f as usize]);
        let mut i = bc;
        while i <= ec {
            if self.pdf_char_marked(f, i) {
                break;
            }
            i += 1;
        }
        let s = i;
        let mut i = ec;
        while i >= bc {
            if self.pdf_char_marked(f, i) {
                break;
            }
            i -= 1;
        }
        s <= i
    }

    /// `dopdffont` (writefont.c): the font dictionary of font `f`, PDF
    /// object `font_objnum`.
    pub fn do_pdf_font(&mut self, font_objnum: i32, f: i32) {
        if !self.font_is_used(f) {
            return; // avoid failed assertion in create_fontdictionary
        }
        self.with_fonts(|g, st| {
            let fm = if g.fm_has_entry(st, f) {
                Some((g.pdf_font_map[f as usize] - 1) as usize)
            } else {
                None
            };
            match fm {
                Some(id) if !fm_of(st, id).is_pk() => {
                    g.create_fontdictionary(st, id, font_objnum, f)
                }
                _ => g.writet3(st, fm, font_objnum, f),
            }
        });
    }
}
