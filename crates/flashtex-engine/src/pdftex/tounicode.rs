//! `tounicode.c`, ported: `\pdfglyphtounicode` and the `/ToUnicode` CMaps
//! of fonts.

use super::cfmt;
use super::fonts::{Fonts, GlyphNames, NOTDEF};
use crate::generated::Globals;
use std::collections::BTreeMap;

const UNI_UNDEF: i64 = -1;
/// A string allocated by `deftounicode`.
const UNI_STRING: i64 = -2;
/// A string allocated by `set_glyph_unicode`.
const UNI_EXTRA_STRING: i64 = -3;
const SMALL_BUF_SIZE: usize = 256;

/// `glyph_unicode_entry` without its name (the key).
#[derive(Clone, Debug)]
struct Gu {
    code: i64,
    unicode_seq: Option<Vec<u8>>,
}
crate::codec_struct!(Gu { code, unicode_seq });

#[derive(Default, Clone)]
pub struct State {
    /// `glyph_unicode_tree`; `None` until the first `\pdfglyphtounicode`.
    tree: Option<BTreeMap<Vec<u8>, Gu>>,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot.
crate::codec_struct!(State { tree });

/// `isXdigit`: a digit or `A`-`F` (upper case only).
fn is_xdigit(c: u8) -> bool {
    c.is_ascii_digit() || (b'A'..=b'F').contains(&c)
}

/// `utf16be_str`.
fn utf16be_str(code: i64) -> String {
    if code <= 0xFFFF {
        format!("{code:04X}")
    } else {
        let v = code - 0x10000;
        let vh = v / 0x400 + 0xD800;
        let vl = v % 0x400 + 0xDC00;
        format!("{vh:04X}{vl:04X}")
    }
}

/// `check_unicode_value`.
fn check_unicode_value(s: &[u8], multiple_value: bool) -> i64 {
    let l = s.len();
    if l == 0 {
        return UNI_UNDEF;
    }
    if multiple_value && !l.is_multiple_of(4) {
        return UNI_UNDEF;
    }
    if !multiple_value && !(4..=6).contains(&l) {
        return UNI_UNDEF;
    }
    let mut code: i64 = 0;
    for i in 0..l {
        if !is_xdigit(s[i]) {
            return UNI_UNDEF;
        }
        if multiple_value {
            if i % 4 == 3 {
                match cfmt::scan_hex4(&s[i - 3..]) {
                    None => return UNI_UNDEF,
                    Some(c) => code = c as i64,
                }
                if !((0x0000..=0xD7FF).contains(&code) || (0xE000..=0xFFFF).contains(&code)) {
                    return UNI_UNDEF;
                }
            }
        } else if i == l - 1 {
            match cfmt::scan_hex(s) {
                None => return UNI_UNDEF,
                Some(c) => code = c as i64,
            }
            if !((0x0000..=0xD7FF).contains(&code) || (0xE000..=0x10FFFF).contains(&code)) {
                return UNI_UNDEF;
            }
        }
    }
    code
}

/// `is_last_byte_valid`.
fn is_last_byte_valid(src_code1: i32, src_code2: i32, code: i64) -> bool {
    let s = utf16be_str(code);
    let l = i64::from_str_radix(&s[s.len() - 2..], 16).unwrap_or(0);
    l < 255 - (src_code2 - src_code1) as i64
}

impl State {
    /// `set_glyph_unicode`: the Unicode value of glyph `s` of TFM `tfmname`.
    fn set_glyph_unicode(&self, s: &[u8], tfmname: &[u8], gp: &mut Gu) {
        // skip dummy entries
        if s == NOTDEF {
            return;
        }
        // strip everything after the first dot
        let s = match s.iter().position(|&c| c == b'.') {
            Some(p) => &s[..p],
            None => s,
        };
        if s.is_empty() {
            return;
        }
        // check for case of multiple components separated by '_'
        if s.contains(&b'_') {
            let mut buf2: Vec<u8> = Vec::new();
            for comp in s.split(|&c| c == b'_') {
                let mut tmp = Gu {
                    code: UNI_UNDEF,
                    unicode_seq: None,
                };
                self.set_glyph_unicode(comp, tfmname, &mut tmp);
                match tmp.code {
                    UNI_UNDEF => {} // not found, do nothing
                    UNI_STRING | UNI_EXTRA_STRING => {
                        buf2.extend_from_slice(tmp.unicode_seq.as_deref().unwrap_or_default());
                    }
                    c => buf2.extend_from_slice(utf16be_str(c).as_bytes()),
                }
            }
            gp.code = UNI_EXTRA_STRING;
            gp.unicode_seq = Some(buf2);
            return;
        }
        let tree = self.tree.as_ref();
        // lookup for glyph name in the tfm's namespace
        let mut key = b"tfm:".to_vec();
        key.extend_from_slice(tfmname);
        key.push(b'/');
        key.extend_from_slice(s);
        key.truncate(SMALL_BUF_SIZE - 1); // snprintf(buf2, SMALL_BUF_SIZE, ...)
        if let Some(e) = tree.and_then(|t| t.get(&key)) {
            *gp = e.clone();
            return;
        }
        // lookup for glyph name in the main database
        let mut key = s.to_vec();
        key.truncate(SMALL_BUF_SIZE - 1);
        if let Some(e) = tree.and_then(|t| t.get(&key)) {
            *gp = e.clone();
            return;
        }
        // check for case of "uniXXXX" (multiple 4-hex-digit values allowed)
        if let Some(p2) = s.strip_prefix(b"uni") {
            let code = check_unicode_value(p2, true);
            if code != UNI_UNDEF {
                if p2.len() == 4 {
                    gp.code = code; // single value
                } else {
                    gp.code = UNI_EXTRA_STRING; // multiple value
                    gp.unicode_seq = Some(p2.to_vec());
                }
            }
            return; // since the last case cannot happen
        }
        // check for case of "uXXXX" (single value up to 6 hex digits)
        if let Some(p2) = s.strip_prefix(b"u") {
            let code = check_unicode_value(p2, false);
            if code != UNI_UNDEF {
                gp.code = code;
            }
        }
    }
}

impl Globals {
    /// `deftounicode` (tounicode.c): `\pdfglyphtounicode`.
    pub fn def_tounicode(&mut self, glyph: i32, unistr: i32) {
        let buf = self.c_string(glyph);
        let p0 = self.c_string(unistr);
        let start = p0.iter().position(|&c| c != b' ').unwrap_or(p0.len());
        let p = &p0[start..]; // ignore leading spaces
        let mut l = p.len();
        while l > 0 && p[l - 1] == b' ' {
            l -= 1; // ignore trailing spaces
        }
        let mut valid_unistr = 1; // a unicode value is the most common case
        for &c in &p[..l] {
            if c == b' ' {
                valid_unistr = 2; // if a space occurs we treat this entry as a string
            } else if !is_xdigit(c) {
                valid_unistr = 0;
                break;
            }
        }
        if l == 0 || valid_unistr == 0 || buf.is_empty() || buf.as_slice() == NOTDEF {
            let mut msg = b"ToUnicode: invalid parameter(s): `".to_vec();
            msg.extend_from_slice(&buf);
            msg.extend_from_slice(b"' => `");
            msg.extend_from_slice(p);
            msg.extend_from_slice(b"'");
            self.pdftex_warn_bytes(&msg);
            return;
        }
        let entry = if valid_unistr == 2 {
            // a string with space(s): copy p, ignoring spaces
            Gu {
                code: UNI_STRING,
                unicode_seq: Some(p.iter().copied().filter(|&c| c != b' ').collect()),
            }
        } else {
            let v = cfmt::scan_hex(p).unwrap_or(0);
            if v > 0x10FFFF {
                self.pdftex_warn(&format!("ToUnicode: value out of range [0,10FFFF]: {v:X}"));
                Gu {
                    code: UNI_UNDEF,
                    unicode_seq: None,
                }
            } else {
                Gu {
                    code: v as i64,
                    unicode_seq: None,
                }
            }
        };
        self.with_fonts(|_, st| {
            // allow overriding existing entries
            st.tu
                .tree
                .get_or_insert_with(BTreeMap::new)
                .insert(buf, entry);
        });
    }

    /// `write_tounicode` (tounicode.c): the `/ToUnicode` CMap of a font
    /// with these glyph names; its object number, or 0.
    pub fn write_tounicode(
        &mut self,
        st: &mut Fonts,
        glyph_names: &GlyphNames,
        tfmname: &[u8],
        encname: Option<&[u8]>,
    ) -> i32 {
        if st.tu.tree.is_none() {
            self.pdftex_warn("no GlyphToUnicode entry has been inserted yet!");
            self.fixed_gen_tounicode = 0;
            return 0;
        }
        // build the name for this CMap as <tfmname>-<encname>; if encname
        // is NULL then this is a builtin encoding
        let mut buf = tfmname.to_vec();
        buf.push(b'-');
        match encname {
            Some(enc) => {
                buf.extend_from_slice(enc);
                match buf.iter().rposition(|&c| c == b'.') {
                    Some(p) if &buf[p..] == b".enc" => buf.truncate(p),
                    _ => {
                        // some silly encoding file name not ending in enc
                        let mut msg = b"Dubious encoding file name: `".to_vec();
                        msg.extend_from_slice(enc);
                        msg.push(b'\'');
                        self.pdftex_warn_bytes(&msg);
                    }
                }
            }
            None => buf.extend_from_slice(b"builtin"),
        }
        let b = String::from_utf8_lossy(&buf).into_owned();
        let objnum = self.pdf_new_objnum();
        self.pdf_begin_dict(objnum, 0);
        self.pdf_begin_stream();
        self.pdf_printf(
            format!(
                "%!PS-Adobe-3.0 Resource-CMap\n\
                 %%DocumentNeededResources: ProcSet (CIDInit)\n\
                 %%IncludeResource: ProcSet (CIDInit)\n\
                 %%BeginResource: CMap (TeX-{b}-0)\n\
                 %%Title: (TeX-{b}-0 TeX {b} 0)\n\
                 %%Version: 1.000\n\
                 %%EndComments\n\
                 /CIDInit /ProcSet findresource begin\n\
                 12 dict begin\n\
                 begincmap\n\
                 /CIDSystemInfo\n\
                 << /Registry (TeX)\n\
                 /Ordering ({b})\n\
                 /Supplement 0\n\
                 >> def\n\
                 /CMapName /TeX-{b}-0 def\n\
                 /CMapType 2 def\n\
                 1 begincodespacerange\n\
                 <00> <FF>\n\
                 endcodespacerange\n"
            )
            .as_bytes(),
        );
        // set gtab
        let mut gtab: Vec<Gu> = vec![
            Gu {
                code: UNI_UNDEF,
                unicode_seq: None
            };
            257
        ];
        for (i, g) in gtab.iter_mut().enumerate().take(256) {
            st.tu.set_glyph_unicode(&glyph_names[i], tfmname, g);
        }
        // set range_size
        let mut range_size = [0i32; 257];
        let mut i = 0usize;
        while i < 256 {
            let c = gtab[i].code;
            if c == UNI_STRING || c == UNI_EXTRA_STRING {
                range_size[i] = 1; // single entry
                i += 1;
            } else if c == UNI_UNDEF {
                range_size[i] = 0; // no entry
                i += 1;
            } else {
                // gtab[i].code >= 0
                let j = i;
                while i < 256
                    && gtab[i + 1].code >= 0
                    && gtab[i].code + 1 == gtab[i + 1].code
                    && is_last_byte_valid(j as i32, i as i32, gtab[i].code)
                {
                    i += 1;
                }
                // at this point i is the last entry of the subrange
                i += 1; // move i to the next entry
                range_size[j] = (i - j) as i32;
            }
        }
        // calculate bfrange_count and bfchar_count
        let (mut bfrange_count, mut bfchar_count) = (0i32, 0i32);
        let mut i = 0usize;
        while i < 256 {
            if range_size[i] == 1 {
                bfchar_count += 1;
                i += 1;
            } else if range_size[i] > 1 {
                bfrange_count += 1;
                i += range_size[i] as usize;
            } else {
                i += 1;
            }
        }
        // write out bfrange
        let mut i = 0usize;
        loop {
            let subrange_count = bfrange_count.min(100);
            bfrange_count -= subrange_count;
            self.pdf_printf(format!("{subrange_count} beginbfrange\n").as_bytes());
            for _ in 0..subrange_count {
                while i < 256 && range_size[i] <= 1 {
                    i += 1;
                }
                let s = format!(
                    "<{:02X}> <{:02X}> <{}>\n",
                    i,
                    i as i32 + range_size[i] - 1,
                    utf16be_str(gtab[i].code)
                );
                self.pdf_printf(s.as_bytes());
                i += range_size[i] as usize;
            }
            self.pdf_printf(b"endbfrange\n");
            if bfrange_count <= 0 {
                break;
            }
        }
        // write out bfchar
        let mut i = 0usize;
        loop {
            let subrange_count = bfchar_count.min(100);
            bfchar_count -= subrange_count;
            self.pdf_printf(format!("{subrange_count} beginbfchar\n").as_bytes());
            for _ in 0..subrange_count {
                while i < 256 {
                    if range_size[i] > 1 {
                        i += range_size[i] as usize;
                    } else if range_size[i] == 0 {
                        i += 1;
                    } else {
                        break; // range_size[i] == 1
                    }
                }
                let c = gtab[i].code;
                let mut s = format!("<{i:02X}> <").into_bytes();
                if c == UNI_STRING || c == UNI_EXTRA_STRING {
                    s.extend_from_slice(gtab[i].unicode_seq.as_deref().unwrap_or_default());
                } else {
                    s.extend_from_slice(utf16be_str(c).as_bytes());
                }
                s.extend_from_slice(b">\n");
                self.pdf_printf(&s);
                i += 1;
            }
            self.pdf_printf(b"endbfchar\n");
            if bfchar_count <= 0 {
                break;
            }
        }
        self.pdf_printf(
            b"endcmap\n\
              CMapName currentdict /CMap defineresource pop\n\
              end\n\
              end\n\
              %%EndResource\n\
              %%EOF\n",
        );
        self.pdf_end_stream();
        objnum
    }

    fn fmt_dump_int(&mut self, v: i32) {
        self.fmt_file.buf.set_int(v);
        crate::system::put_word(&mut self.fmt_file);
    }

    fn fmt_undump_int(&mut self) -> i32 {
        crate::system::get_word(&mut self.fmt_file);
        self.fmt_file.buf.int()
    }

    /// `dumpcharptr`: the length with its NUL, then the bytes (one word
    /// each here) with the NUL.
    fn fmt_dump_chars(&mut self, s: &[u8]) {
        self.fmt_dump_int(s.len() as i32 + 1);
        for &b in s {
            self.fmt_dump_int(b as i32);
        }
        self.fmt_dump_int(0);
    }

    fn fmt_undump_chars(&mut self) -> Option<Vec<u8>> {
        let x = self.fmt_undump_int();
        if x <= 0 {
            return None;
        }
        let mut v: Vec<u8> = (0..x).map(|_| self.fmt_undump_int() as u8).collect();
        v.pop(); // the NUL
        Some(v)
    }

    /// `dumptounicode` (tounicode.c): the `\pdfglyphtounicode` entries into
    /// the format.
    pub fn dumptounicode(&mut self) {
        let tree = self.with_fonts(|_, st| st.tu.tree.clone());
        let Some(tree) = tree else {
            self.fmt_dump_int(0);
            return;
        };
        self.fmt_dump_int(tree.len() as i32);
        for (name, gu) in &tree {
            self.fmt_dump_chars(name);
            self.fmt_dump_int(gu.code as i32);
            if gu.code == UNI_STRING {
                let seq = gu.unicode_seq.clone().unwrap_or_default();
                self.fmt_dump_chars(&seq);
            }
        }
    }

    /// `undumptounicode` (tounicode.c).
    pub fn undumptounicode(&mut self) {
        let remaining = self.fmt_undump_int();
        if remaining == 0 {
            return;
        }
        let mut tree = BTreeMap::new();
        for _ in 0..remaining {
            let Some(name) = self.fmt_undump_chars() else {
                self.pdftex_fail("undumpcharptr(gu->name) got NULL");
            };
            let code = self.fmt_undump_int() as i64;
            let mut seq = None;
            if code == UNI_STRING {
                seq = self.fmt_undump_chars();
                if seq.is_none() {
                    self.pdftex_fail("undumpcharptr(gu->unicode_seq) got NULL");
                }
            }
            tree.insert(
                name,
                Gu {
                    code,
                    unicode_seq: seq,
                },
            );
        }
        self.with_fonts(|_, st| st.tu.tree = Some(tree));
    }
}
