//! Embedding one Type 1 font: whole (`t1_include`) or subset to the glyphs
//! a document uses (`t1_subset_ascii_part` to `t1_subset_end`), line by
//! line as writet1.c does, so the same input gives the same bytes.
//!
//! The clear part is copied with its font matrix, italic angle and font
//! name adjusted for the map entry, and a subset's `/Encoding` reduced to
//! the glyphs kept; the encrypted part is decrypted, a subset's `/Subrs`
//! and `/CharStrings` reduced to what those glyphs need
//! ([`super::charstring`]), and encrypted again.

use super::charstring::{Charstring, Charstrings, MarkEnv, Target};
use super::cipher::{self, Cipher};
use super::encoding::{read_names, standard_names};
use super::reader::{c_str, ends_with_before_lf, find, scan_num, Eexec, Reader};
use super::{Fail, Host, Persist, Result};
use crate::pdftex::cfmt;
use crate::pdftex::fonts::{notdef_names, GlyphNames, NOTDEF};
use crate::pdftex::mapfile::FmEntry;
use crate::pdftex::writefont::{
    FdEntry, FONTBBOX1_CODE, FONTNAME_CODE, FONT_KEYS, ITALIC_ANGLE_CODE, STEMV_CODE,
};
use std::collections::{BTreeMap, BTreeSet};

const CHARSTRINGS: &[u8] = b"/CharStrings";
const SUBRS: &[u8] = b"/Subrs";
/// How many lines may come after the `/Subrs` array before `/CharStrings`
/// until the font is taken to be synthetic (`POST_SUBRS_SCAN`).
const POST_SUBRS_SCAN: usize = 5;
/// The charstring operator `return`.
const RETURN: u8 = 11;

/// The begin and end tokens of a charstring (`cs_token_pairs_list`).
const TOKEN_PAIRS: [(&[u8], &[u8]); 4] = [
    (b" RD", b"NP"),
    (b" -|", b"|"),
    (b" RD", b"noaccess put"),
    (b" -|", b"noaccess put"),
];

/// What a map entry asks for.
pub(super) struct Job<'a> {
    pub fm: &'a FmEntry,
    pub fd: &'a mut FdEntry,
    pub persist: &'a mut Persist,
    /// Where the font buffer already ends (`fb_offset()`): offsets count
    /// from the buffer's start.
    pub fb_base: i32,
}

/// The font as embedded, and its `/Length1` and `/Length2` (`/Length3` is
/// always 0: pdfTeX's `fixedcontent` is false).
pub(super) struct Embedded {
    pub bytes: Vec<u8>,
    pub length1: i32,
    pub length2: i32,
}

/// `writet1`'s body: embed `font` as `job` asks.
pub(super) fn embed(font: &[u8], job: Job, host: &mut dyn Host) -> Result<Embedded> {
    let mut t = Type1 {
        r: Reader::new(font),
        out: Writer {
            bytes: Vec::with_capacity(font.len()),
            base: job.fb_base,
            key: None,
            save_offset: 0,
        },
        fm: job.fm,
        fd: job.fd,
        persist: job.persist,
        host,
        len_iv: 4,
        in_charstrings: false,
        scan: true,
        synthetic: false,
        standard_encoding: false,
        fontname_offset: 0,
        length1: 0,
        length2: 0,
        cs: Charstrings::default(),
        dict: Dict::default(),
        subrs: Dict::default(),
        token_pair: None,
    };
    if t.fm.is_subsetted() {
        t.subset_clear_part()?;
        t.start_eexec()?;
        t.read_subrs()?;
        t.subset_charstrings()?;
        t.subset_end()?;
    } else {
        t.include()?;
    }
    Ok(Embedded {
        bytes: t.out.bytes,
        length1: t.length1,
        length2: t.length2,
    })
}

/// The font buffer being written (`fb_array` from `fb_base`), encrypted
/// inside `eexec` (`t1_eexec_encrypt`, key `t1_er`).
struct Writer {
    bytes: Vec<u8>,
    base: i32,
    key: Option<Cipher>,
    save_offset: i32,
}

impl Writer {
    /// `t1_offset()`.
    fn offset(&self) -> i32 {
        self.base + self.bytes.len() as i32
    }

    /// The bytes written since the last call (`get_length1` and
    /// `save_offset`).
    fn take_length(&mut self) -> i32 {
        let n = self.offset() - self.save_offset;
        self.save_offset = self.offset();
        n
    }

    /// `t1_putline`: a line of fewer than two bytes is not written.
    #[inline]
    fn put_line(&mut self, line: &[u8]) {
        if line.len() <= 1 {
            return;
        }
        match &mut self.key {
            Some(key) => self.bytes.extend(line.iter().map(|&b| key.encrypt(b))),
            None => self.bytes.extend_from_slice(line),
        }
    }

    /// `t1_puts(s)`.
    fn put_text(&mut self, s: &[u8]) {
        self.put_line(c_str(s));
    }

    /// `t1_putline` of `line` ended by `eol`.
    fn put_text_line(&mut self, mut line: Vec<u8>) {
        super::reader::end_line(&mut line);
        self.put_line(&line);
    }
}

/// Where the `/Subrs` array or the `/CharStrings` dictionary starts and
/// ends: its first line (with its size), the position of the size on it,
/// and the lines after its last entry.
#[derive(Default)]
struct Dict {
    size: i32,
    size_pos: usize,
    start: Vec<u8>,
    end: Vec<u8>,
}

/// One run of `writet1`: everything writet1.c keeps in globals while it
/// embeds a font.
struct Type1<'a, 'h> {
    r: Reader<'a>,
    out: Writer,
    fm: &'a FmEntry,
    fd: &'a mut FdEntry,
    persist: &'a mut Persist,
    host: &'h mut dyn Host,
    /// `/lenIV`.
    len_iv: i16,
    /// `t1_cs`: lines may carry charstrings.
    in_charstrings: bool,
    /// `t1_scan`: look for the keys that feed the font descriptor.
    scan: bool,
    /// The font's charstrings are not after its subrs (`t1_synthetic`).
    synthetic: bool,
    /// The font's `/Encoding` is StandardEncoding (`ENC_STANDARD`).
    standard_encoding: bool,
    /// Where the font name's subset tag goes (`t1_fontname_offset`).
    fontname_offset: i32,
    length1: i32,
    length2: i32,
    cs: Charstrings,
    /// `/CharStrings` (`cs_size`, `cs_size_pos`, `cs_dict_start`,
    /// `cs_dict_end`).
    dict: Dict,
    /// `/Subrs` (`subr_size`, `subr_size_pos`, `subr_array_start`,
    /// `subr_array_end`).
    subrs: Dict,
    /// The charstring begin/end tokens the subrs use (`cs_token_pair`).
    token_pair: Option<(&'static [u8], &'static [u8])>,
}

impl Type1<'_, '_> {
    fn read_line(&mut self) -> Result<()> {
        self.r.read_line(self.in_charstrings)
    }

    fn put_line(&mut self) {
        self.out.put_line(&self.r.line.bytes);
    }

    fn at_charstrings(&self) -> bool {
        self.r.line.contains(CHARSTRINGS)
    }

    fn at_subrs(&self) -> bool {
        self.r.line.starts_with(SUBRS)
    }

    fn at_end_eexec(&self) -> bool {
        self.r.line.ends_with(b"mark currentfile closefile")
    }

    /// The number at `p` on the current line (`t1_scan_num`).
    fn scan_num(&self, p: usize) -> Result<(f32, usize)> {
        scan_num(&self.r.line.bytes, p)
    }

    fn line_error(&self, what: &str) -> Fail {
        Fail(format!("{what}: `{}'", self.r.line.quoted()))
    }

    // --- the encrypted part --------------------------------------------

    /// `t1_start_eexec`.
    fn start_eexec(&mut self) -> Result<()> {
        self.length1 = self.out.take_length();
        self.r.start_eexec()?;
        self.out.key = Some(Cipher::EEXEC);
        // the four bytes that start the encrypted part, as zeros
        self.out.put_line(&[0; 4]);
        Ok(())
    }

    /// `t1_stop_eexec`.
    fn stop_eexec(&mut self) -> Result<()> {
        self.length2 = self.out.take_length();
        self.out.key = None;
        if self.r.stop_eexec()? {
            self.out.put_text(b"00");
        }
        self.in_charstrings = false;
        Ok(())
    }

    // --- keys --------------------------------------------------------------

    /// `t1_scan_param`.
    fn scan_param(&mut self) -> Result<()> {
        if !self.scan || self.r.line.bytes.first() != Some(&b'/') {
            return Ok(());
        }
        if self.r.line.starts_with(b"/lenIV") {
            self.len_iv = self.scan_num(b"/lenIV".len())?.0 as i32 as i16;
            if self.len_iv < 0 {
                return Err(Fail("negative value of lenIV is not supported".into()));
            }
            return Ok(());
        }
        self.scan_keys()
    }

    /// `t1_scan_keys`: the values the font descriptor needs, and the
    /// changes a slanted or extended font makes to the font.
    fn scan_keys(&mut self) -> Result<()> {
        let line = &self.r.line;
        if self.fm.extend != 0 || self.fm.slant != 0 {
            if line.starts_with(b"/FontMatrix") {
                return self.modify_font_matrix();
            }
            if line.starts_with(b"/ItalicAngle") {
                return self.modify_italic_angle();
            }
        }
        if line.starts_with(b"/FontType") {
            let i = self.scan_num(b"FontType".len() + 1)?.0 as i32;
            if i != 1 {
                return Err(Fail(format!("Type{i} fonts unsupported by pdfTeX")));
            }
            return Ok(());
        }
        let text = line.text();
        let Some(k) = FONT_KEYS.iter().position(|key| {
            !key.1.is_empty() && text.get(1..).is_some_and(|l| l.starts_with(key.1))
        }) else {
            return Ok(());
        };
        let mut p = FONT_KEYS[k].1.len() + 1;
        if text.get(p) == Some(&b' ') {
            p += 1;
        }
        if k == FONTNAME_CODE {
            return self.scan_font_name(p);
        }
        if (k == STEMV_CODE || k == FONTBBOX1_CODE) && matches!(text.get(p), Some(b'[' | b'{')) {
            p += 1;
        }
        let values = if k == FONTBBOX1_CODE { 4 } else { 1 };
        for dim in &mut self.fd.font_dim[k..k + values] {
            let (v, end) = scan_num(&self.r.line.bytes, p)?;
            dim.val = v as i32;
            dim.set = true;
            p = end;
        }
        Ok(())
    }

    /// `/FontName /Name`: the descriptor's font name, with the map entry's
    /// slant and extension. A subset's line gets a placeholder subset tag,
    /// written over once the glyphs are known.
    fn scan_font_name(&mut self, mut p: usize) -> Result<()> {
        let text = self.r.line.text();
        if text.get(p) != Some(&b'/') {
            return Err(self.line_error("a name expected"));
        }
        p += 1;
        let start = p;
        p += text[p..]
            .iter()
            .take_while(|&&c| c != b' ' && c != b'\n')
            .count();
        let mut name = text[start..p].to_vec();
        if self.fm.slant != 0 {
            name.extend_from_slice(format!("-Slant_{}", self.fm.slant).as_bytes());
        }
        if self.fm.extend != 0 {
            name.extend_from_slice(format!("-Extend_{}", self.fm.extend).as_bytes());
        }
        if self.fm.is_subsetted() {
            self.fontname_offset = self.out.offset() + start as i32;
            let line = [&text[..start], b"ABCDEF+", &name, &text[p..]].concat();
            self.r.line.set_text(line);
        }
        self.fd.fontname = Some(name);
        Ok(())
    }

    /// `t1_modify_fm`: apply SlantFont and ExtendFont to `/FontMatrix`.
    fn modify_font_matrix(&mut self) -> Result<()> {
        let text = self.r.line.text();
        let open = text.iter().position(|&c| c == b'[');
        let Some(open) = open.or_else(|| text.iter().position(|&c| c == b'{')) else {
            return Err(self.line_error("FontMatrix: an array expected"));
        };
        let bracket = text[open];
        let mut out = text[..=open].to_vec();
        let mut p = open + 1;
        let mut a = [0f32; 6];
        for x in &mut a {
            (*x, p) = self.scan_num(p)?;
        }
        if self.fm.slant != 0 {
            // do_slant(a, slant * 1E-3)
            let s = f64::from(self.fm.slant) * 1E-3;
            for i in [0, 2, 4] {
                a[i] = (f64::from(a[i]) + f64::from(a[i + 1]) * s) as f32;
            }
        }
        if self.fm.extend != 0 {
            // do_extend = do_xscale(a, extend * 1E-3)
            let e = f64::from(self.fm.extend) * 1E-3;
            for i in [0, 2, 4] {
                a[i] = (f64::from(a[i]) * e) as f32;
            }
        }
        for x in a {
            out.extend_from_slice(&cfmt::fmt_g(f64::from(x)));
            out.push(b' ');
        }
        let text = self.r.line.text();
        let close = if bracket == b'[' { b']' } else { b'}' };
        let Some(end) = text
            .get(p..)
            .and_then(|t| t.iter().position(|&c| c == close))
        else {
            return Err(self.line_error(&format!(
                "FontMatrix: cannot find the corresponding character to '{}'",
                bracket as char
            )));
        };
        out.extend_from_slice(&text[p + end..]);
        self.r.line.set_text(out);
        Ok(())
    }

    /// `t1_modify_italic`: apply SlantFont to `/ItalicAngle`.
    fn modify_italic_angle(&mut self) -> Result<()> {
        let slant = self.fm.slant;
        if slant == 0 {
            return Ok(());
        }
        let text = self.r.line.text();
        let Some(blank) = text.iter().position(|&c| c == b' ') else {
            return Ok(());
        };
        let mut out = text[..=blank].to_vec();
        let (a0, end) = self.scan_num(blank + 1)?;
        let a = (f64::from(a0) - (f64::from(slant) * 1E-3).atan() * (180.0 / std::f64::consts::PI))
            as f32;
        out.extend_from_slice(&cfmt::fmt_g(f64::from(a)));
        let text = self.r.line.text();
        out.extend_from_slice(&text[end.min(text.len())..]);
        self.r.line.set_text(out);
        let dim = &mut self.fd.font_dim[ITALIC_ANGLE_CODE];
        dim.val = crate::system::pas_round(f64::from(a));
        dim.set = true;
        Ok(())
    }

    // --- the encoding --------------------------------------------------------

    /// `t1_builtin_enc`: the font's own encoding, from its `/Encoding`.
    fn builtin_encoding(&mut self) -> Result<GlyphNames> {
        if self.r.line.ends_with(b"def") {
            // a predefined encoding: `sscanf("%255s")` after `/Encoding`
            let text = self.r.line.text();
            let rest = &text[b"/Encoding".len().min(text.len())..];
            let word: Vec<u8> = rest
                .iter()
                .copied()
                .skip_while(|&c| is_c_space(c))
                .take_while(|&c| !is_c_space(c))
                .take(255)
                .collect();
            if word == b"StandardEncoding" {
                self.standard_encoding = true;
                return Ok(standard_names());
            }
            return Err(Fail(format!(
                "cannot subset font (unknown predefined encoding `{}')",
                String::from_utf8_lossy(&word)
            )));
        }
        // Two forms: `/Encoding [/a /b ...] readonly def`, or
        //     /Encoding 256 array 0 1 255 {1 index exch /.notdef put} for
        //     dup 0 /x put
        //     ...
        //     readonly def
        if self.r.line.starts_with(b"/Encoding [") || self.r.line.starts_with(b"/Encoding[") {
            self.encoding_array()
        } else {
            self.encoding_puts()
        }
    }

    /// `/Encoding [/a /b ...] def`, over as many lines as it takes.
    fn encoding_array(&mut self) -> Result<GlyphNames> {
        let mut names = notdef_names();
        let mut count = 0;
        let text = self.r.line.text();
        let mut at = text.iter().position(|&c| c == b'[').map_or(0, |i| i + 1);
        if text.get(at) == Some(&b' ') {
            at += 1;
        }
        loop {
            let text = self.r.line.text();
            at = read_names(text, at, &mut names, &mut count)?;
            let c = text.get(at).copied().unwrap_or(0);
            if c != b'\n' && c != b'%' {
                let rest = &text[at.min(text.len())..];
                if rest.starts_with(b"] def") || rest.starts_with(b"] readonly def") {
                    return Ok(names);
                }
                return Err(self.line_error("a name or `] def' or `] readonly def' expected"));
            }
            self.read_line()?;
            at = 0;
        }
    }

    /// `/Encoding 256 array ...` then `dup <code> /<name> put` (and the
    /// `get put` and `getinterval`/`putinterval` copies some fonts use), up
    /// to `def`.
    fn encoding_puts(&mut self) -> Result<GlyphNames> {
        let byte = |v: i32| (0..256).contains(&v);
        let get_put = |rest: &[u8]| {
            let (n, [to, from, _]) = cfmt::scan_ints(rest, c"dup dup %i exch %i get put");
            (n == 2 && byte(from) && byte(to)).then_some((to as usize, from as usize))
        };
        let put_interval = |rest: &[u8]| {
            let (n, [from, count, to]) =
                cfmt::scan_ints(rest, c"dup dup %i %i getinterval %i exch putinterval");
            let ok = n == 3 && byte(from) && byte(to) && byte(count);
            ok.then_some((from as usize, count as usize, to as usize))
        };
        let after = |rest: &[u8], token: &[u8], msg: &str| {
            find(rest, token)
                .map(|k| k + token.len())
                .ok_or_else(|| Fail(msg.into()))
        };
        let mut names = notdef_names();
        let mut text = self.r.line.text().to_vec();
        let mut p = text.iter().position(|&c| c == b'\n').unwrap_or(text.len());
        loop {
            if text.get(p) == Some(&b'\n') {
                self.read_line()?;
                text = self.r.line.text().to_vec();
                p = 0;
            }
            let rest = &text[p.min(text.len())..];
            let (n, code, name) = cfmt::scan_dup_put(rest);
            if n == 2 && name.first() == Some(&b'/') && byte(code) {
                // `dup <code> /<name> put`
                if &name[1..] != NOTDEF {
                    names[code as usize] = name[1..].to_vec();
                }
                p += after(rest, b" put", "invalid pfb, no put found in dup")?;
            } else if let Some((to, from)) = get_put(rest) {
                // `dup dup <to> exch <from> get put`
                copy_glyph_name(&mut names, from, to);
                p += after(
                    rest,
                    b" get put",
                    "invalid pfb, no get put found in dup dup",
                )?;
            } else if let Some((from, count, to)) = put_interval(rest) {
                // `dup dup <from> <count> getinterval <to> exch putinterval`
                for i in 0..count {
                    if from + i < 256 && to + i < 256 {
                        copy_glyph_name(&mut names, from + i, to + i);
                    }
                }
                let msg = "invalid pfb, no putinterval found in dup dup";
                p += after(rest, b" putinterval", msg)?;
            } else if (p == 0 || text.get(p.wrapping_sub(1)) == Some(&b' ')) && rest == b"def\n" {
                // `def' or `readonly def'
                return Ok(names);
            } else {
                // skip an unrecognisable word
                p += rest
                    .iter()
                    .take_while(|&&c| c != b' ' && c != b'\n')
                    .count();
            }
            if text.get(p) == Some(&b' ') {
                p += 1;
            }
            if p >= text.len() {
                // C reads the NUL here and stays; the next round tests it
                // against LF only, so an unterminated line cannot occur:
                // every line from t1_getline ends with LF.
                p = text.len();
                text.push(b'\n');
            }
        }
    }

    // --- whole font ----------------------------------------------------------

    /// `t1_include`: the whole font, not subset.
    fn include(&mut self) -> Result<()> {
        loop {
            self.read_line()?;
            self.scan_param()?;
            self.put_line();
            if self.r.eexec != Eexec::Before {
                break;
            }
        }
        self.start_eexec()?;
        loop {
            self.read_line()?;
            self.scan_param()?;
            self.put_line();
            if self.at_charstrings() || self.at_subrs() {
                break;
            }
        }
        self.in_charstrings = true;
        loop {
            self.read_line()?;
            self.put_line();
            if self.at_end_eexec() {
                break;
            }
        }
        self.stop_eexec()
    }

    // --- subsetting ------------------------------------------------------------

    /// `t1_subset_ascii_part`: the clear part, with the font's encoding
    /// reduced to the glyphs kept and without `/UniqueID`.
    fn subset_clear_part(&mut self) -> Result<()> {
        self.read_line()?;
        while !self.r.line.starts_with(b"/Encoding") {
            self.scan_param()?;
            let text = self.r.line.text();
            let unique_id_def = self.r.line.starts_with(b"/UniqueID")
                && text.len() >= 4
                && text[text.len() - 4..].starts_with(b"def");
            if !unique_id_def {
                self.put_line();
            }
            self.read_line()?;
        }
        let names = self.builtin_encoding()?;
        if self.fm.is_subsetted() {
            if let Some(chars) = &self.fd.tx_tree {
                // take over collected non-reencoded characters from TeX
                let glyphs = self.fd.gl_tree.get_or_insert_with(Default::default);
                for &c in chars {
                    if let Some(n) = usize::try_from(c).ok().and_then(|c| names.get(c)) {
                        glyphs.insert(n.clone());
                    }
                }
            }
            let empty = BTreeSet::new();
            let glyphs = self.fd.gl_tree.as_ref().unwrap_or(&empty);
            let fontname = self.fd.fontname.as_deref().unwrap_or_default();
            let tag = self.host.subset_tag(glyphs, fontname);
            self.fd.subset_tag = Some(tag);
            let at = usize::try_from(self.fontname_offset - self.out.base).ok();
            if let (true, Some(at)) = (self.fontname_offset != 0, at) {
                if let Some(slot) = self.out.bytes.get_mut(at..at + 6) {
                    slot.copy_from_slice(&tag);
                }
            }
        }
        // now really all glyphs needed from this font are in fd.gl_tree
        if self.standard_encoding {
            self.out.put_text(b"/Encoding StandardEncoding def\n");
        } else {
            self.out
                .put_text(b"/Encoding 256 array\n0 1 255 {1 index exch /.notdef put} for\n");
            // create_t1_glyph_tree: each glyph name's first code
            let mut first_code: BTreeMap<&[u8], usize> = BTreeMap::new();
            for (i, g) in names.iter().enumerate() {
                if g != NOTDEF {
                    first_code.entry(g).or_insert(i);
                }
            }
            let mut any = false;
            for glyph in self.fd.gl_tree.iter().flatten() {
                if let Some(&i) = first_code.get(glyph.as_slice()) {
                    let line = [format!("dup {i} /").as_bytes(), glyph, b" put\n"].concat();
                    self.out.put_text(&line);
                    any = true;
                }
            }
            if !any {
                // We didn't mark anything for the Encoding array. We add
                // "dup 0 /.notdef put" for compatibility with Acrobat 5.0.
                self.out.put_text(b"dup 0 /.notdef put\n");
            }
            self.out.put_text(b"readonly def\n");
        }
        self.fd.builtin_glyph_names = Some(names);
        loop {
            self.read_line()?;
            self.scan_param()?;
            if !self.r.line.starts_with(b"/UniqueID") {
                // ignore UniqueID for subsetted fonts
                self.put_line();
            }
            if self.r.eexec != Eexec::Before {
                return Ok(());
            }
        }
    }

    /// `cs_store`: keep the subr (`is_subr`) or charstring on the current
    /// line.
    fn store_charstring(&mut self, is_subr: bool) -> Result<()> {
        let line = &self.r.line;
        let bytes = &line.bytes;
        let blank = bytes.iter().position(|&c| c == b' ').unwrap_or(bytes.len());
        // what follows the length: ` RD `, the charstring, the rest of the
        // line up to its LF, and LF. (A length C cannot have read, which is
        // undefined behaviour there, keeps what is on the line.)
        let cs_end = (line.cs_start + usize::from(line.cs_len)).min(bytes.len());
        let tail = bytes[cs_end..].iter().take_while(|&&c| c != b'\n').count();
        let mut data = bytes[line.cs_start.saturating_sub(4)..cs_end + tail].to_vec();
        data.push(b'\n');
        let mut cs = Charstring {
            name: Vec::new(),
            data,
            cs_len: line.cs_len,
            used: false,
            valid: true,
        };
        if is_subr {
            let n = self.scan_num(blank + 1)?.0 as i32;
            let Some(slot) = usize::try_from(n)
                .ok()
                .and_then(|i| self.cs.subrs.get_mut(i))
            else {
                return Err(Fail(format!("Subrs array: entry index out of range ({n})")));
            };
            if self.token_pair.is_none() {
                self.token_pair = token_pair(&cs.data);
            }
            *slot = cs;
        } else {
            if self.cs.glyphs.len() as i32 + 1 > self.dict.size {
                return Err(Fail(format!(
                    "CharStrings dict: more entries than dict size ({})",
                    self.dict.size
                )));
            }
            cs.name = c_str(bytes.get(1..blank).unwrap_or_default()).to_vec();
            self.cs.add_glyph(cs);
        }
        Ok(())
    }

    /// `t1_read_subrs`: the clear part of the encrypted part, then the
    /// `/Subrs` array (none in some fonts).
    fn read_subrs(&mut self) -> Result<()> {
        self.read_line()?;
        while !(self.at_charstrings() || self.at_subrs()) {
            self.scan_param()?;
            if !self.r.line.starts_with(b"/UniqueID") {
                // ignore UniqueID for subsetted fonts
                self.put_line();
            }
            self.read_line()?;
        }
        loop {
            self.in_charstrings = true;
            self.scan = false;
            if !self.at_subrs() {
                return Ok(());
            }
            // the array's size follows "/Subrs"
            self.subrs.size_pos = SUBRS.len() + 1;
            self.subrs.size = self.scan_num(self.subrs.size_pos)?.0 as i32;
            if self.subrs.size == 0 {
                while !self.at_charstrings() {
                    self.read_line()?;
                }
                return Ok(());
            }
            self.cs.subrs = vec![Charstring::default(); self.subrs.size.max(0) as usize];
            self.subrs.start = self.r.line.text().to_vec();
            self.read_line()?;
            while self.r.line.has_charstring() {
                self.store_charstring(true)?;
                self.read_line()?;
            }
            // the first four subrs are kept without being parsed
            for subr in self.cs.subrs.iter_mut().take(4) {
                subr.used = true;
            }
            // The end of the Subrs array might have more than one line, so
            // they are concatenated. Some (synthetic) fonts do not have the
            // CharStrings dict right after the Subrs array; if CharStrings
            // is not in the next POST_SUBRS_SCAN lines, the font is treated
            // as synthetic and everything up to the next Subrs is ignored.
            let mut end = Vec::new();
            let mut scanned = 0;
            while scanned < POST_SUBRS_SCAN && !self.at_charstrings() {
                end.extend_from_slice(self.r.line.text());
                self.read_line()?;
                scanned += 1;
            }
            self.subrs.end = end;
            if scanned < POST_SUBRS_SCAN {
                return Ok(());
            }
            // `cs_init`, then look again
            self.cs = Charstrings::default();
            self.dict = Dict::default();
            self.subrs = Dict::default();
            self.token_pair = None;
            self.in_charstrings = false;
            self.synthetic = true;
            while !(self.at_charstrings() || self.at_subrs()) {
                self.read_line()?;
            }
        }
    }

    /// `t1_flush_cs`: write the subrs (`is_subr`) or charstrings that are
    /// kept, a subr that is not as a bare `return`.
    fn flush_charstrings(&mut self, is_subr: bool) {
        let (dict, count) = if is_subr {
            let subr_max = self.subr_max();
            (std::mem::take(&mut self.subrs), subr_max + 1)
        } else {
            let used = self.cs.glyphs.iter().filter(|e| e.used).count() as i32;
            (std::mem::take(&mut self.dict), used)
        };
        let table = if is_subr {
            let mut t = std::mem::take(&mut self.cs.subrs);
            t.truncate(count.max(0) as usize);
            t
        } else {
            std::mem::take(&mut self.cs.glyphs)
        };
        // the first line, with the number of entries written
        let start = &dict.start;
        let at = dict.size_pos.min(start.len());
        let digits = start[at..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        let count_text = (count as u32).to_string();
        let first = [&start[..at], count_text.as_bytes(), &start[at + digits..]];
        self.out.put_text_line(first.concat());
        // a bare `return` stands in for an unused subr
        let mut bare_return = Vec::new();
        if is_subr {
            let plain = std::iter::repeat_n(0, self.len_iv.max(0) as usize).chain([RETURN]);
            bare_return = cipher::encrypt(Cipher::CHARSTRING, plain);
        }
        let (open, close) = self.token_pair.unwrap_or((b"", b""));
        for (i, e) in table.iter().enumerate() {
            if e.used {
                let head = if is_subr {
                    format!("dup {i} {}", e.cs_len).into_bytes()
                } else {
                    [
                        b"/".as_slice(),
                        &e.name,
                        format!(" {}", e.cs_len).as_bytes(),
                    ]
                    .concat()
                };
                self.out.put_line(&[head.as_slice(), &e.data].concat());
            } else if is_subr {
                let head = format!("dup {i} {}", bare_return.len());
                let line = [head.as_bytes(), open, b" ", &bare_return].concat();
                self.out.put_line(&line);
                self.out.put_text_line([b" ", close].concat());
            }
        }
        self.out.put_text_line(c_str(&dict.end).to_vec());
    }

    /// The last subr kept (`subr_max`; -1 for none).
    fn subr_max(&self) -> i32 {
        if self.synthetic || self.fd.all_glyphs {
            return self.subrs.size - 1;
        }
        self.cs
            .subrs
            .iter()
            .rposition(|e| e.used)
            .map_or(-1, |i| i as i32)
    }

    /// `t1_mark_glyphs`: everything, for a synthetic font or one embedded
    /// with all its glyphs; otherwise `.notdef`, the glyphs the document
    /// uses, and what they need.
    fn mark_glyphs(&mut self) -> Result<()> {
        if self.synthetic || self.fd.all_glyphs {
            for e in self.cs.glyphs.iter_mut().chain(self.cs.subrs.iter_mut()) {
                e.used |= e.valid;
            }
            return Ok(());
        }
        let mut env = MarkEnv {
            len_iv: self.len_iv,
            last_arg_other_subr3: &mut self.persist.last_arg_other_subr3,
            glyph_set: &mut self.fd.gl_tree,
            host: &mut *self.host,
        };
        self.cs.mark(&mut env, Target::Glyph(NOTDEF))?;
        // the glyph set as it is now: a `seac` adds to it as the walks go
        let glyphs: Vec<Vec<u8>> = env.glyph_set.iter().flatten().cloned().collect();
        for glyph in &glyphs {
            self.cs.mark(&mut env, Target::Glyph(glyph))?;
        }
        Ok(())
    }

    /// `t1_subset_charstrings`.
    fn subset_charstrings(&mut self) -> Result<()> {
        // A line like "dup/CharStrings\n229 dict dup begin" has the size on
        // the next line: read it and join the two.
        let text = self.r.line.text();
        let key_end = find(text, CHARSTRINGS).map_or(text.len(), |k| k + CHARSTRINGS.len());
        if cfmt::scan_one_int(&text[key_end..]).is_none() {
            let mut joined = text.to_vec();
            // t1_getline always ends a line with LF; it becomes a blank
            if let Some(last) = joined.last_mut() {
                *last = b' ';
            }
            self.read_line()?;
            joined.extend_from_slice(self.r.line.text());
            self.r.line.set_text(joined);
        }
        let text = self.r.line.text();
        // the dictionary's size follows "/CharStrings "
        self.dict.size_pos =
            find(text, CHARSTRINGS).map_or(text.len(), |k| k + CHARSTRINGS.len()) + 1;
        self.dict.size = self.scan_num(self.dict.size_pos)?.0 as i32;
        self.dict.start = self.r.line.text().to_vec();
        self.cs.clear_glyphs();
        self.read_line()?;
        while self.r.line.has_charstring() {
            self.store_charstring(false)?;
            self.read_line()?;
        }
        self.dict.end = self.r.line.text().to_vec();
        self.mark_glyphs()?;
        if !self.cs.subrs.is_empty() {
            if self.token_pair.is_none() {
                return Err(Fail(
                    "This Type 1 font uses mismatched subroutine begin/end token pairs.".into(),
                ));
            }
            self.flush_charstrings(true);
        }
        self.flush_charstrings(false);
        Ok(())
    }

    /// `t1_subset_end`: the rest of the encrypted part (of a synthetic font
    /// only up to its `definefont`), then the clear end.
    fn subset_end(&mut self) -> Result<()> {
        if self.synthetic {
            // copy to "dup /FontName get exch definefont pop"
            while !self.r.line.contains(b"definefont") {
                self.read_line()?;
                self.put_line();
            }
            while !self.at_end_eexec() {
                self.read_line()?; // ignore the rest
            }
            self.put_line(); // write "mark currentfile closefile"
        } else {
            while !self.at_end_eexec() {
                // copy to "mark currentfile closefile"
                self.read_line()?;
                self.put_line();
            }
        }
        self.stop_eexec()
    }
}

/// `isspace` in the C locale.
fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// `copy_glyph_names(glyph_names, from, to)`.
fn copy_glyph_name(names: &mut GlyphNames, from: usize, to: usize) {
    if names[to] != NOTDEF {
        names[to] = NOTDEF.to_vec();
    }
    if names[from] != NOTDEF {
        names[to] = names[from].clone();
    }
}

/// `check_cs_token_pair`: the begin/end tokens a stored charstring uses.
fn token_pair(stored: &[u8]) -> Option<(&'static [u8], &'static [u8])> {
    TOKEN_PAIRS
        .into_iter()
        .find(|(open, close)| c_str(stored).starts_with(open) && ends_with_before_lf(stored, close))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdftex::mapfile::{F_SUBSETTED, F_TYPE1};

    /// A host that records warnings and names every subset `XYZXYZ`.
    #[derive(Default)]
    struct TestHost {
        warnings: Vec<String>,
    }

    impl Host for TestHost {
        fn warn(&mut self, msg: &[u8]) {
            self.warnings
                .push(String::from_utf8_lossy(msg).into_owned());
        }

        fn subset_tag(&mut self, _: &BTreeSet<Vec<u8>>, _: &[u8]) -> [u8; 6] {
            *b"XYZXYZ"
        }
    }

    /// An encrypted charstring (lenIV 4, the random bytes zeros).
    fn charstring(code: &[u8]) -> Vec<u8> {
        cipher::encrypt(Cipher::CHARSTRING, [0; 4].iter().chain(code).copied())
    }

    const CLEAR: &[u8] = b"%!PS-AdobeFont-1.0: Test 001\n\
        /FontType 1 def\n\
        /FontName /Test def\n\
        /UniqueID 5000000 def\n\
        /ItalicAngle 0 def\n\
        /FontMatrix [0.001 0 0 0.001 0 0] readonly def\n\
        /FontBBox {0 -10 500 700} readonly def\n\
        /Encoding 256 array\n\
        0 1 255 {1 index exch /.notdef put} for\n\
        dup 65 /A put\n\
        dup 66 /B put\n\
        readonly def\n\
        currentfile eexec\n";

    /// The private part's plaintext: five subrs that only return, and
    /// `.notdef`, `A` (which calls subr 4) and `B`.
    fn private() -> Vec<u8> {
        let ret = charstring(&[11]);
        // 0 500 hsbw [4 callsubr] endchar
        let plain = charstring(&[139, 248, 136, 13, 14]);
        let calls = charstring(&[139, 248, 136, 13, 143, 10, 14]);
        let mut p = b"dup /Private 8 dict dup begin\n\
            /RD{string currentfile exch readstring pop}executeonly def\n\
            /ND{noaccess def}executeonly def\n\
            /NP{noaccess put}executeonly def\n\
            /lenIV 4 def\n\
            /Subrs 5 array\n"
            .to_vec();
        for i in 0..5 {
            p.extend_from_slice(format!("dup {i} {} RD ", ret.len()).as_bytes());
            p.extend_from_slice(&ret);
            p.extend_from_slice(b" NP\n");
        }
        p.extend_from_slice(b"ND\n2 index /CharStrings 3 dict dup begin\n");
        for (name, cs) in [(".notdef", &plain), ("A", &calls), ("B", &plain)] {
            p.extend_from_slice(format!("/{name} {} RD ", cs.len()).as_bytes());
            p.extend_from_slice(cs);
            p.extend_from_slice(b" ND\n");
        }
        p.extend_from_slice(
            b"end\nend\nreadonly put\nnoaccess put\n\
              dup /FontName get exch definefont pop\nmark currentfile closefile\n\n",
        );
        p
    }

    /// The font as a PFA: the private part encrypted after four zero
    /// bytes, in hexadecimal, then the zeros and `cleartomark`.
    fn pfa() -> Vec<u8> {
        let sealed = cipher::encrypt(Cipher::EEXEC, [0; 4].into_iter().chain(private()));
        let mut font = CLEAR.to_vec();
        for chunk in sealed.chunks(32) {
            for b in chunk {
                font.extend_from_slice(format!("{b:02x}").as_bytes());
            }
            font.push(b'\n');
        }
        for _ in 0..8 {
            font.extend_from_slice(&[b'0'; 64]);
            font.push(b'\n');
        }
        font.extend_from_slice(b"cleartomark\n");
        font
    }

    struct Run {
        out: Embedded,
        fd: FdEntry,
        host: TestHost,
    }

    fn embed_with(font: &[u8], typ: u16, slant: i32, glyphs: &[&str]) -> Result<Run> {
        let mut fm = FmEntry::new();
        fm.typ = typ | F_TYPE1;
        fm.slant = slant;
        let mut fd = FdEntry {
            gl_tree: Some(glyphs.iter().map(|g| g.as_bytes().to_vec()).collect()),
            ..Default::default()
        };
        let mut persist = Persist::default();
        let mut host = TestHost::default();
        let job = Job {
            fm: &fm,
            fd: &mut fd,
            persist: &mut persist,
            fb_base: 0,
        };
        let out = embed(font, job, &mut host)?;
        Ok(Run { out, fd, host })
    }

    fn run(typ: u16, slant: i32, glyphs: &[&str]) -> Run {
        embed_with(&pfa(), typ, slant, glyphs).unwrap()
    }

    /// The encrypted part of an embedded font, decrypted, without its four
    /// leading bytes.
    fn decrypted(out: &Embedded) -> Vec<u8> {
        let start = out.length1 as usize;
        let sealed = &out.bytes[start..start + out.length2 as usize];
        let mut key = Cipher::EEXEC;
        sealed.iter().map(|&c| key.decrypt(c)).skip(4).collect()
    }

    fn has(hay: &[u8], needle: &str) -> bool {
        find(hay, needle.as_bytes()).is_some()
    }

    #[test]
    fn a_whole_font_is_copied() {
        let Run { out, fd, .. } = run(0, 0, &[]);
        assert_eq!(&out.bytes[..out.length1 as usize], CLEAR);
        // the private part through its `closefile` line, encrypted again
        // from the same four bytes
        let private = private();
        let end = find(&private, b"closefile\n").unwrap() + b"closefile\n".len();
        assert_eq!(decrypted(&out), &private[..end]);
        assert_eq!(fd.fontname.as_deref(), Some(b"Test".as_slice()));
        let bbox: Vec<i32> = fd.font_dim[FONTBBOX1_CODE..FONTBBOX1_CODE + 4]
            .iter()
            .map(|d| d.val)
            .collect();
        assert_eq!(bbox, [0, -10, 500, 700]);
    }

    #[test]
    fn a_subset_keeps_what_its_glyphs_need() {
        let Run { out, fd, host } = run(F_SUBSETTED, 0, &["A"]);
        let clear = &out.bytes[..out.length1 as usize];
        assert!(has(clear, "/FontName /XYZXYZ+Test def\n"));
        assert!(!has(clear, "/UniqueID"));
        assert!(has(
            clear,
            "/Encoding 256 array\n0 1 255 {1 index exch /.notdef put} for\n"
        ));
        assert!(has(clear, "dup 65 /A put\nreadonly def\n"));
        assert!(!has(clear, "/B put"));
        assert_eq!(fd.subset_tag, Some(*b"XYZXYZ"));
        assert_eq!(fd.builtin_glyph_names.as_ref().unwrap()[66], b"B");
        let private = decrypted(&out);
        // A calls subr 4, so all five subrs stay
        assert!(has(&private, "/Subrs 5 array\n"));
        assert!(has(&private, "/CharStrings 2 dict dup begin\n"));
        assert!(has(&private, "/.notdef 9 RD "));
        assert!(has(&private, "/A 11 RD "));
        assert!(!has(&private, "/B "));
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn subrs_no_glyph_needs_are_dropped() {
        let Run { out, .. } = run(F_SUBSETTED, 0, &["B"]);
        let private = decrypted(&out);
        // the first four subrs are always kept; subr 4 is not needed
        assert!(has(&private, "/Subrs 4 array\n"));
        assert!(!has(&private, "dup 4 "));
        assert!(has(&private, "/B 9 RD "));
    }

    #[test]
    fn slant_changes_the_matrix_angle_and_name() {
        let Run { out, fd, .. } = run(0, 167, &[]);
        let clear = &out.bytes[..out.length1 as usize];
        assert!(has(
            clear,
            "/FontMatrix [0.001 0 0.000167 0.001 0 0 ] readonly def\n"
        ));
        assert!(has(clear, "/ItalicAngle -9.4809 def\n"));
        assert_eq!(fd.fontname.as_deref(), Some(b"Test-Slant_167".as_slice()));
        assert_eq!(fd.font_dim[ITALIC_ANGLE_CODE].val, -9);
    }

    #[test]
    fn an_undefined_glyph_is_a_warning() {
        let Run { host, .. } = run(F_SUBSETTED, 0, &["A", "nosuch"]);
        assert_eq!(host.warnings, ["glyph `nosuch' undefined"]);
    }

    #[test]
    fn a_truncated_font_is_an_error() {
        let mut font = pfa();
        font.truncate(CLEAR.len() + 40);
        let e = embed_with(&font, 0, 0, &[]).err().unwrap();
        assert_eq!(e.0, "unexpected end of file");
    }
}
