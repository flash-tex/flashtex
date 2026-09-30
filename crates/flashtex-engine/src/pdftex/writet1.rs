//! `writet1.c`, ported: Type 1 font embedding and subsetting, and
//! `load_enc_file` (encoding vectors).
//!
//! The subsetter is pdfTeX's line by line: it reads the font a line at a
//! time (normalising blanks and line ends exactly as `append_char_to_buf`
//! does, inside the eexec part too), keeps the lines it needs, rewrites the
//! `/Encoding`, the `/Subrs` array and the `/CharStrings` dictionary, and
//! encrypts the private part again. The same input therefore gives the same
//! subset bytes as pdfTeX.
//!
//! C's `t1_line_array`/`t1_buf_array` are NUL-terminated strings that also
//! carry binary charstring data. Here they are byte vectors: the raw
//! contents up to `t1_line_ptr`, and a "C view" up to the first NUL wherever
//! the C code uses a string function.

use super::cfmt;
use super::fonts::{notdef_names, Fonts, GlyphNames, NOTDEF};
use super::mapfile::FmEntry;
use super::output::set_cur_file_name;
use super::writefont::{
    FdEntry, FONTBBOX1_CODE, FONTNAME_CODE, FONT_KEYS, ITALIC_ANGLE_CODE, STEMV_CODE,
};
use crate::generated::Globals;
use crate::resolver::Format;
use std::collections::{BTreeMap, HashMap};

const T1_C1: u32 = 52845;
const T1_C2: u32 = 22719;
const ENC_BUF_SIZE: usize = 0x1000;
const POST_SUBRS_SCAN: usize = 5;
const CHARSTRINGNAME: &[u8] = b"/CharStrings";
const EEXEC_STR: &[u8] = b"currentfile eexec";

const CS_HSTEM: usize = 1;
const CS_VSTEM: usize = 3;
const CS_VMOVETO: usize = 4;
const CS_RLINETO: usize = 5;
const CS_HLINETO: usize = 6;
const CS_VLINETO: usize = 7;
const CS_RRCURVETO: usize = 8;
const CS_CLOSEPATH: usize = 9;
const CS_CALLSUBR: usize = 10;
const CS_RETURN: usize = 11;
const CS_ESCAPE: usize = 12;
const CS_HSBW: usize = 13;
const CS_ENDCHAR: usize = 14;
const CS_RMOVETO: usize = 21;
const CS_HMOVETO: usize = 22;
const CS_VHCURVETO: usize = 30;
const CS_HVCURVETO: usize = 31;
const CS_1BYTE_MAX: usize = CS_HVCURVETO + 1;
const CS_DOTSECTION: usize = CS_1BYTE_MAX;
const CS_VSTEM3: usize = CS_1BYTE_MAX + 1;
const CS_HSTEM3: usize = CS_1BYTE_MAX + 2;
const CS_SEAC: usize = CS_1BYTE_MAX + 6;
const CS_SBW: usize = CS_1BYTE_MAX + 7;
const CS_DIV: usize = CS_1BYTE_MAX + 12;
const CS_CALLOTHERSUBR: usize = CS_1BYTE_MAX + 16;
const CS_POP: usize = CS_1BYTE_MAX + 17;
const CS_SETCURRENTPOINT: usize = CS_1BYTE_MAX + 33;

/// The deepest `callsubr` nesting `cs_mark` follows (#1237). pdfTeX's
/// `cs_mark` has no limit: it recurses once per nested call on the C stack,
/// so a subr that calls itself, or a cycle of subrs, crashes it (SIGSEGV).
/// Here that is a font error, as DESIGN.md 4.5 asks. The limit is 100 times
/// the Type 1 spec's 10 levels and several times below where the recursion
/// would exhaust the 8 MB main-thread stack (measured: a chain of 5,000
/// nested subrs runs in a debug build, 10,000 overflows; a release build
/// runs 20,000). Up to it the output is pdfTeX's, byte for byte
/// (tests/type1_subr_nesting.rs).
const CS_SUBR_NEST_MAX: u32 = 1000;
const CS_MAX: usize = CS_SETCURRENTPOINT + 1;

/// `cc_entry`: (nargs, bottom, clear, valid).
#[derive(Clone, Copy)]
struct Cc {
    nargs: u8,
    bottom: bool,
    clear: bool,
    valid: bool,
}

/// `cc_init`'s table.
fn cc_tab() -> [Cc; CS_MAX] {
    let mut t = [Cc {
        nargs: 0,
        bottom: false,
        clear: false,
        valid: false,
    }; CS_MAX];
    let mut set = |n: usize, bottom: bool, nargs: u8, clear: bool| {
        t[n] = Cc {
            nargs,
            bottom,
            clear,
            valid: true,
        };
    };
    set(CS_HSTEM, true, 2, true);
    set(CS_VSTEM, true, 2, true);
    set(CS_VMOVETO, true, 1, true);
    set(CS_RLINETO, true, 2, true);
    set(CS_HLINETO, true, 1, true);
    set(CS_VLINETO, true, 1, true);
    set(CS_RRCURVETO, true, 6, true);
    set(CS_CLOSEPATH, false, 0, true);
    set(CS_CALLSUBR, false, 1, false);
    set(CS_RETURN, false, 0, false);
    set(CS_HSBW, true, 2, true);
    set(CS_ENDCHAR, false, 0, true);
    set(CS_RMOVETO, true, 2, true);
    set(CS_HMOVETO, true, 1, true);
    set(CS_VHCURVETO, true, 4, true);
    set(CS_HVCURVETO, true, 4, true);
    set(CS_DOTSECTION, false, 0, true);
    set(CS_VSTEM3, true, 6, true);
    set(CS_HSTEM3, true, 6, true);
    set(CS_SEAC, true, 5, true);
    set(CS_SBW, true, 4, true);
    set(CS_DIV, false, 2, false);
    set(CS_CALLOTHERSUBR, false, 0, false);
    set(CS_POP, false, 0, false);
    set(CS_SETCURRENTPOINT, true, 2, true);
    t
}

/// `standard_glyph_names` (writet1.c): Adobe StandardEncoding.
pub(crate) fn standard_glyph_name(i: usize) -> &'static [u8] {
    const N: &[u8] = NOTDEF;
    const T: [&[u8]; 256] = [
        // 0x00
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        // 0x10
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        // 0x20
        b"space",
        b"exclam",
        b"quotedbl",
        b"numbersign",
        b"dollar",
        b"percent",
        b"ampersand",
        b"quoteright",
        b"parenleft",
        b"parenright",
        b"asterisk",
        b"plus",
        b"comma",
        b"hyphen",
        b"period",
        b"slash",
        // 0x30
        b"zero",
        b"one",
        b"two",
        b"three",
        b"four",
        b"five",
        b"six",
        b"seven",
        b"eight",
        b"nine",
        b"colon",
        b"semicolon",
        b"less",
        b"equal",
        b"greater",
        b"question",
        // 0x40
        b"at",
        b"A",
        b"B",
        b"C",
        b"D",
        b"E",
        b"F",
        b"G",
        b"H",
        b"I",
        b"J",
        b"K",
        b"L",
        b"M",
        b"N",
        b"O",
        // 0x50
        b"P",
        b"Q",
        b"R",
        b"S",
        b"T",
        b"U",
        b"V",
        b"W",
        b"X",
        b"Y",
        b"Z",
        b"bracketleft",
        b"backslash",
        b"bracketright",
        b"asciicircum",
        b"underscore",
        // 0x60
        b"quoteleft",
        b"a",
        b"b",
        b"c",
        b"d",
        b"e",
        b"f",
        b"g",
        b"h",
        b"i",
        b"j",
        b"k",
        b"l",
        b"m",
        b"n",
        b"o",
        // 0x70
        b"p",
        b"q",
        b"r",
        b"s",
        b"t",
        b"u",
        b"v",
        b"w",
        b"x",
        b"y",
        b"z",
        b"braceleft",
        b"bar",
        b"braceright",
        b"asciitilde",
        N,
        // 0x80
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        // 0x90
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        // 0xa0
        N,
        b"exclamdown",
        b"cent",
        b"sterling",
        b"fraction",
        b"yen",
        b"florin",
        b"section",
        b"currency",
        b"quotesingle",
        b"quotedblleft",
        b"guillemotleft",
        b"guilsinglleft",
        b"guilsinglright",
        b"fi",
        b"fl",
        // 0xb0
        N,
        b"endash",
        b"dagger",
        b"daggerdbl",
        b"periodcentered",
        N,
        b"paragraph",
        b"bullet",
        b"quotesinglbase",
        b"quotedblbase",
        b"quotedblright",
        b"guillemotright",
        b"ellipsis",
        b"perthousand",
        N,
        b"questiondown",
        // 0xc0
        N,
        b"grave",
        b"acute",
        b"circumflex",
        b"tilde",
        b"macron",
        b"breve",
        b"dotaccent",
        b"dieresis",
        N,
        b"ring",
        b"cedilla",
        N,
        b"hungarumlaut",
        b"ogonek",
        b"caron",
        // 0xd0
        b"emdash",
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        N,
        // 0xe0
        N,
        b"AE",
        N,
        b"ordfeminine",
        N,
        N,
        N,
        N,
        b"Lslash",
        b"Oslash",
        b"OE",
        b"ordmasculine",
        N,
        N,
        N,
        N,
        // 0xf0
        N,
        b"ae",
        N,
        N,
        N,
        b"dotlessi",
        N,
        N,
        b"lslash",
        b"oslash",
        b"oe",
        b"germandbls",
        N,
        N,
        N,
        N,
    ];
    T.get(i).copied().unwrap_or(N)
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

/// `cs_entry`.
#[derive(Clone, Default)]
struct CsEntry {
    name: Vec<u8>,
    data: Vec<u8>,
    len: u16,
    cslen: u16,
    used: bool,
    valid: bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Encoding {
    Standard,
    Builtin,
}

/// `cs_token_pairs_list`.
const CS_TOKEN_PAIRS: [(&[u8], &[u8]); 4] = [
    (b" RD", b"NP"),
    (b" -|", b"|"),
    (b" RD", b"noaccess put"),
    (b" -|", b"noaccess put"),
];

/// The up-to-NUL part of `s`, as C string functions see it.
fn cview(s: &[u8]) -> &[u8] {
    match s.iter().position(|&c| c == 0) {
        Some(i) => &s[..i],
        None => s,
    }
}

/// `strstr`.
fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    if n.is_empty() {
        return Some(0);
    }
    h.windows(n.len()).position(|w| w == n)
}

/// `str_suffix(begin, end, s)`: the bytes end with `s`, before a final LF.
fn str_suffix(buf: &[u8], s: &[u8]) -> bool {
    let mut end = buf.len();
    if end > 0 && buf[end - 1] == 10 {
        end -= 1;
    }
    end >= s.len() && &buf[end - s.len()..end] == s
}

/// `append_char_to_buf` (ptexmac.h): tab to blank, CR and EOF to LF, no
/// leading or doubled blanks. Returns the converted character.
fn append_char(c: i32, buf: &mut Vec<u8>) -> i32 {
    let mut c = c;
    if c == 9 {
        c = 32;
    }
    if c == 13 || c == -1 {
        c = 10;
    }
    if c != b' ' as i32 || buf.last().is_some_and(|&l| l != 32) {
        buf.push(c as u8);
    }
    c
}

/// `append_eol` (ptexmac.h): end the line with one LF, dropping a blank
/// before it.
fn append_eol(buf: &mut Vec<u8>) {
    if buf.len() > 1 && buf[buf.len() - 1] != 10 {
        buf.push(10);
    }
    if buf.len() > 2 && buf[buf.len() - 2] == 32 {
        let n = buf.len();
        buf[n - 2] = 10;
        buf.pop();
    }
}

/// `eol(s)`: end the C string with LF unless it is shorter than two bytes
/// or ends with LF already.
fn eol(s: &mut Vec<u8>) {
    let n = cview(s).len();
    s.truncate(n);
    if n > 1 && s[n - 1] != 10 {
        s.push(10);
    }
}

/// `remove_eol`: a line for an error message, without its LF.
fn without_eol(s: &[u8]) -> String {
    let mut v = cview(s);
    if v.last() == Some(&10) {
        v = &v[..v.len() - 1];
    }
    String::from_utf8_lossy(v).into_owned()
}

fn is_c_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// One run of `writet1`: the font file being read and everything the C
/// file keeps in globals while it runs.
struct T1<'a> {
    g: &'a mut Globals,
    fd: &'a mut FdEntry,
    fm: FmEntry,
    persist: &'a mut Persist,
    /// The font file, `getc` position and `feof`.
    file: Vec<u8>,
    pos: usize,
    eof: bool,
    /// The font buffer (`fb_array`), from offset `fb_base`.
    fb: Vec<u8>,
    fb_base: i32,
    pfa: bool,
    block_length: i64,
    dr: u16,
    er: u16,
    len_iv: i16,
    in_eexec: i32,
    cs: bool,
    scan: bool,
    eexec_encrypt: bool,
    synthetic: bool,
    last_hexbyte: i32,
    line: Vec<u8>,
    buf: Vec<u8>,
    cslen: u16,
    cs_start: usize,
    encoding: Encoding,
    save_offset: i32,
    fontname_offset: i32,
    length1: i32,
    length2: i32,
    length3: i32,
    cc: [Cc; CS_MAX],
    stack: Vec<i32>,
    cs_tab: Vec<CsEntry>,
    cs_by_name: HashMap<Vec<u8>, usize>,
    cs_size: i32,
    cs_count: i32,
    cs_size_pos: usize,
    cs_dict_start: Vec<u8>,
    cs_dict_end: Vec<u8>,
    cs_notdef: Option<usize>,
    cs_token_pair: Option<usize>,
    subr_tab: Vec<CsEntry>,
    subr_size: i32,
    subr_max: i32,
    subr_size_pos: usize,
    subr_array_start: Vec<u8>,
    subr_array_end: Vec<u8>,
    /// How many `callsubr`s `cs_mark` is inside (`CS_SUBR_NEST_MAX`).
    cs_depth: u32,
}

impl T1<'_> {
    fn fail(&mut self, msg: &str) -> ! {
        self.g.pdftex_fail(msg)
    }

    // --- the font file ---------------------------------------------------

    /// `t1_getchar()`: `getc(t1_file)`.
    fn getchar(&mut self) -> i32 {
        match self.file.get(self.pos) {
            Some(&b) => {
                self.pos += 1;
                b as i32
            }
            None => {
                self.eof = true;
                -1
            }
        }
    }

    /// `t1_getbyte`: the next data byte, across PFB segment headers.
    fn getbyte(&mut self) -> i32 {
        let mut c = self.getchar();
        if self.pfa {
            return c;
        }
        if self.block_length == 0 {
            if c != 128 {
                self.fail("invalid marker");
            }
            c = self.getchar();
            if c == 3 {
                while !self.eof {
                    self.getchar();
                }
                return -1;
            }
            self.block_length = (self.getchar() & 0xff) as i64;
            self.block_length |= ((self.getchar() & 0xff) as i64) << 8;
            self.block_length |= ((self.getchar() & 0xff) as i64) << 16;
            self.block_length |= ((self.getchar() & 0xff) as i64) << 24;
            c = self.getchar();
        }
        self.block_length -= 1;
        c
    }

    fn hexval(c: i32) -> i32 {
        match c {
            0x41..=0x46 => c - 0x41 + 10,
            0x61..=0x66 => c - 0x61 + 10,
            0x30..=0x39 => c - 0x30,
            _ => -1,
        }
    }

    /// `edecrypt`.
    fn edecrypt(&mut self, cipher: u8) -> u8 {
        let mut cipher = cipher;
        if self.pfa {
            while cipher == 10 || cipher == 13 {
                cipher = self.getbyte() as u8;
            }
            let hi = Self::hexval(cipher as i32);
            let lo = Self::hexval(self.getbyte());
            cipher = ((hi << 4) + lo) as u8;
            self.last_hexbyte = cipher as i32;
        }
        let plain = cipher ^ (self.dr >> 8) as u8;
        self.dr = ((cipher as u32 + self.dr as u32)
            .wrapping_mul(T1_C1)
            .wrapping_add(T1_C2)) as u16;
        plain
    }

    /// `eencrypt`.
    fn eencrypt(&mut self, plain: u8) -> u8 {
        let cipher = plain ^ (self.er >> 8) as u8;
        self.er = ((cipher as u32 + self.er as u32)
            .wrapping_mul(T1_C1)
            .wrapping_add(T1_C2)) as u16;
        cipher
    }

    fn offset(&self) -> i32 {
        self.fb_base + self.fb.len() as i32
    }

    // --- lines -----------------------------------------------------------

    fn prefix(&self, s: &[u8]) -> bool {
        cview(&self.line).starts_with(s)
    }

    fn suffix(&self, s: &[u8]) -> bool {
        str_suffix(&self.line, s)
    }

    fn charstrings(&self) -> bool {
        find(cview(&self.line), CHARSTRINGNAME).is_some()
    }

    fn subrs(&self) -> bool {
        self.prefix(b"/Subrs")
    }

    fn end_eexec(&self) -> bool {
        self.suffix(b"mark currentfile closefile")
    }

    /// `t1_scan_num(p, &r)`: the number at `line[p..]` (after one optional
    /// blank), and where it ends.
    fn scan_num(&mut self, p: usize) -> (f32, usize) {
        let mut p = p;
        if self.line.get(p) == Some(&b' ') {
            p += 1;
        }
        let rest = cview(self.line.get(p..).unwrap_or_default()).to_vec();
        let Some(f) = cfmt::scan_float(&rest) else {
            let l = without_eol(&self.line);
            self.fail(&format!("a number expected: `{l}'"));
        };
        let mut r = p;
        while let Some(&c) = self.line.get(r) {
            if c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'+' | b'-') {
                r += 1;
            } else {
                break;
            }
        }
        (f, r)
    }

    /// `t1_getline`.
    fn getline(&mut self) {
        let eexec_len = EEXEC_STR.len() as i32;
        loop {
            // restart:
            if self.eof {
                self.fail("unexpected end of file");
            }
            self.line.clear();
            self.cslen = 0;
            let mut eexec_scan: i32 = 0;
            let mut c = self.getbyte();
            if c == -1 {
                return; // exit
            }
            while !self.eof {
                if self.in_eexec == 1 {
                    c = self.edecrypt(c as u8) as i32;
                }
                c = append_char(c, &mut self.line);
                if self.in_eexec == 0 && eexec_scan >= 0 && eexec_scan < eexec_len {
                    if self.line.get(eexec_scan as usize) == Some(&EEXEC_STR[eexec_scan as usize]) {
                        eexec_scan += 1;
                    } else {
                        eexec_scan = -1;
                    }
                }
                if c == 10 || (self.pfa && eexec_scan == eexec_len && c == 32) {
                    break;
                }
                if self.cs
                    && self.cslen == 0
                    && self.line.len() > 4
                    && (self.suffix(b" RD ") || self.suffix(b" -| "))
                {
                    let mut p = self.line.len() - 5;
                    while self.line[p] != b' ' {
                        p -= 1;
                    }
                    let l = self.scan_num(p + 1).0 as i32;
                    self.cslen = l as u16;
                    self.cs_start = self.line.len();
                    for _ in 0..l.max(0) {
                        let b = self.getbyte() as u8;
                        let d = self.edecrypt(b);
                        self.line.push(d);
                    }
                }
                c = self.getbyte();
            }
            append_eol(&mut self.line);
            if self.line.len() < 2 {
                continue; // restart
            }
            if eexec_scan == eexec_len {
                self.in_eexec = 1;
            }
            return;
        }
    }

    /// `t1_putline`.
    fn putline(&mut self) {
        if self.line.len() <= 1 {
            return;
        }
        let line = std::mem::take(&mut self.line);
        if self.eexec_encrypt {
            for &b in &line {
                let c = self.eencrypt(b);
                self.fb.push(c);
            }
        } else {
            self.fb.extend_from_slice(&line);
        }
        self.line = line;
    }

    /// `t1_puts(s)`.
    fn puts(&mut self, s: &[u8]) {
        self.line = cview(s).to_vec();
        self.putline();
    }

    /// `t1_init_params`.
    fn init_params(&mut self, open_name_prefix: &[u8]) {
        self.g.tex_printf(open_name_prefix);
        let name = super::output::cur_file_name().unwrap_or_default();
        self.g.tex_printf(&name);
        self.len_iv = 4;
        self.dr = 55665;
        self.er = 55665;
        self.in_eexec = 0;
        self.cs = false;
        self.scan = true;
        self.synthetic = false;
        self.eexec_encrypt = false;
        self.block_length = 0;
        // t1_check_pfa: peek at the first byte
        self.pfa = self.file.first().copied() != Some(128);
    }

    /// `t1_check_block_len`.
    fn check_block_len(&mut self, decrypt: bool) {
        if self.block_length == 0 {
            return;
        }
        let mut c = self.getbyte();
        if decrypt {
            c = self.edecrypt(c as u8) as i32;
        }
        let l = self.block_length;
        if !(l == 0 && (c == 10 || c == 13)) {
            self.fail(&format!("{} bytes more than expected", l + 1));
        }
    }

    /// `t1_start_eexec`.
    fn start_eexec(&mut self) {
        self.length1 = self.offset() - self.save_offset;
        self.save_offset = self.offset();
        if !self.pfa {
            self.check_block_len(false);
        }
        self.line.clear();
        for _ in 0..4 {
            let b = self.getbyte() as u8;
            self.edecrypt(b);
            self.line.push(0);
        }
        self.eexec_encrypt = true;
        self.putline(); // to put the first four bytes
    }

    /// `t1_stop_eexec`.
    fn stop_eexec(&mut self) {
        self.length2 = self.offset() - self.save_offset;
        self.save_offset = self.offset();
        self.eexec_encrypt = false;
        if !self.pfa {
            self.check_block_len(true);
        } else {
            let b = self.getbyte() as u8;
            let c = self.edecrypt(b);
            if !(c == 10 || c == 13) {
                if self.last_hexbyte == 0 {
                    self.puts(b"00");
                } else {
                    self.fail("unexpected data after eexec");
                }
            }
        }
        self.cs = false;
        self.in_eexec = 2;
    }

    /// `get_length3` (`fixedcontent` is false for pdfTeX).
    fn get_length3(&mut self) {
        self.length3 = 0;
    }

    // --- keys ------------------------------------------------------------

    /// `t1_modify_fm`: apply SlantFont/ExtendFont to `/FontMatrix`.
    fn modify_fm(&mut self) {
        let lv = cview(&self.line).to_vec();
        let p0 = match lv.iter().position(|&c| c == b'[') {
            Some(i) => i,
            None => match lv.iter().position(|&c| c == b'{') {
                Some(i) => i,
                None => {
                    let l = without_eol(&self.line);
                    self.fail(&format!("FontMatrix: an array expected: `{l}'"));
                }
            },
        };
        let c = lv[p0];
        let mut p = p0 + 1;
        let mut out = lv[..p].to_vec();
        let mut a = [0f32; 6];
        for x in a.iter_mut() {
            let (v, q) = self.scan_num(p);
            *x = v;
            p = q;
        }
        let slant = self.fm.slant;
        if slant != 0 {
            // do_slant(a, slant * 1E-3)
            let s = slant as f64 * 1E-3;
            a[0] = (a[0] as f64 + a[1] as f64 * s) as f32;
            a[2] = (a[2] as f64 + a[3] as f64 * s) as f32;
            a[4] = (a[4] as f64 + a[5] as f64 * s) as f32;
        }
        let extend = self.fm.extend;
        if extend != 0 {
            // do_extend = do_xscale(a, extend * 1E-3)
            let e = extend as f64 * 1E-3;
            a[0] = (a[0] as f64 * e) as f32;
            a[2] = (a[2] as f64 * e) as f32;
            a[4] = (a[4] as f64 * e) as f32;
        }
        for x in a {
            out.extend_from_slice(&cfmt::fmt_g(x as f64));
            out.push(b' ');
        }
        let close = if c == b'[' { b']' } else { b'}' };
        while p < lv.len() && lv[p] != close {
            p += 1;
        }
        if p >= lv.len() {
            let l = without_eol(&self.line);
            self.fail(&format!(
                "FontMatrix: cannot find the corresponding character to '{}': `{l}'",
                c as char
            ));
        }
        out.extend_from_slice(&lv[p..]);
        self.line = out;
        eol(&mut self.line);
    }

    /// `t1_modify_italic`: apply SlantFont to `/ItalicAngle`.
    fn modify_italic(&mut self) {
        let slant = self.fm.slant;
        if slant == 0 {
            return;
        }
        let lv = cview(&self.line).to_vec();
        let Some(p) = lv.iter().position(|&c| c == b' ') else {
            return;
        };
        let mut out = lv[..=p].to_vec();
        let (a0, r) = self.scan_num(p + 1);
        let a = (a0 as f64 - (slant as f64 * 1E-3).atan() * (180.0 / std::f64::consts::PI)) as f32;
        out.extend_from_slice(&cfmt::fmt_g(a as f64));
        out.extend_from_slice(&lv[r.min(lv.len())..]);
        self.line = out;
        eol(&mut self.line);
        self.fd.font_dim[ITALIC_ANGLE_CODE].val = (a as f64).round() as i32;
        self.fd.font_dim[ITALIC_ANGLE_CODE].set = true;
    }

    /// `t1_scan_keys`.
    fn scan_keys(&mut self) {
        if self.fm.extend != 0 || self.fm.slant != 0 {
            if self.prefix(b"/FontMatrix") {
                self.modify_fm();
                return;
            }
            if self.prefix(b"/ItalicAngle") {
                self.modify_italic();
                return;
            }
        }
        if self.prefix(b"/FontType") {
            let i = self.scan_num(b"FontType".len() + 1).0 as i32;
            if i != 1 {
                self.fail(&format!("Type{i} fonts unsupported by pdfTeX"));
            }
            return;
        }
        let lv = cview(&self.line).to_vec();
        let Some(k) = FONT_KEYS
            .iter()
            .position(|key| !key.1.is_empty() && lv.get(1..).is_some_and(|l| l.starts_with(key.1)))
        else {
            return;
        };
        let mut p = FONT_KEYS[k].1.len() + 1;
        if lv.get(p) == Some(&b' ') {
            p += 1;
        }
        if k == FONTNAME_CODE {
            if lv.get(p) != Some(&b'/') {
                let l = without_eol(&self.line);
                self.fail(&format!("a name expected: `{l}'"));
            }
            p += 1; // skip the slash
            let r = p;
            let mut name = Vec::new();
            while p < lv.len() && lv[p] != b' ' && lv[p] != 10 {
                name.push(lv[p]);
                p += 1;
            }
            if self.fm.slant != 0 {
                name.extend_from_slice(format!("-Slant_{}", self.fm.slant).as_bytes());
            }
            if self.fm.extend != 0 {
                name.extend_from_slice(format!("-Extend_{}", self.fm.extend).as_bytes());
            }
            self.fd.fontname = Some(name.clone());
            // at this moment we cannot call make_subset_tag() yet, as the
            // encoding is not read; thus we mark the offset of the subset
            // tag and write it later
            if self.fm.is_subsetted() {
                self.fontname_offset = self.offset() + r as i32;
                let mut out = lv[..r].to_vec();
                out.extend_from_slice(b"ABCDEF+");
                out.extend_from_slice(&name);
                out.extend_from_slice(&lv[p..]);
                self.line = out;
                eol(&mut self.line);
            }
            return;
        }
        if (k == STEMV_CODE || k == FONTBBOX1_CODE) && matches!(lv.get(p), Some(b'[' | b'{')) {
            p += 1;
        }
        if k == FONTBBOX1_CODE {
            for i in 0..4 {
                let (v, r) = self.scan_num(p);
                self.fd.font_dim[k + i].val = v as i32;
                self.fd.font_dim[k + i].set = true;
                p = r;
            }
            return;
        }
        let v = self.scan_num(p).0;
        self.fd.font_dim[k].val = v as i32;
        self.fd.font_dim[k].set = true;
    }

    /// `t1_scan_param`.
    fn scan_param(&mut self) {
        if !self.scan || self.line.first() != Some(&b'/') {
            return;
        }
        if self.prefix(b"/lenIV") {
            self.len_iv = self.scan_num(b"/lenIV".len()).0 as i32 as i16;
            if self.len_iv < 0 {
                self.fail("negative value of lenIV is not supported");
            }
            return;
        }
        self.scan_keys();
    }

    // --- the encoding ----------------------------------------------------

    /// `t1_builtin_enc`: the font's own encoding, from its `/Encoding`.
    fn builtin_enc(&mut self) -> GlyphNames {
        let mut glyph_names = notdef_names();
        if self.suffix(b"def") {
            // predefined encoding
            let lv = cview(&self.line).to_vec();
            let rest = &lv[b"/Encoding".len().min(lv.len())..];
            let start = rest
                .iter()
                .position(|&c| !is_c_space(c))
                .unwrap_or(rest.len());
            let tok: Vec<u8> = rest[start..]
                .iter()
                .copied()
                .take_while(|&c| !is_c_space(c))
                .take(255)
                .collect();
            if !tok.is_empty() && tok == b"StandardEncoding" {
                self.encoding = Encoding::Standard;
                for (i, g) in glyph_names.iter_mut().enumerate() {
                    let s = standard_glyph_name(i);
                    if s != NOTDEF {
                        *g = s.to_vec();
                    }
                }
                return glyph_names;
            }
            self.fail(&format!(
                "cannot subset font (unknown predefined encoding `{}')",
                String::from_utf8_lossy(&tok)
            ));
        }
        // We have two possible forms of Encoding vector. The first case is
        //     /Encoding [/a /b /c...] readonly def
        // and the second case can look like
        //     /Encoding 256 array 0 1 255 {1 index exch /.notdef put} for
        //     dup 0 /x put
        //     dup 1 /y put
        //     ...
        //     readonly def
        self.encoding = Encoding::Builtin;
        if self.prefix(b"/Encoding [") || self.prefix(b"/Encoding[") {
            // the first case
            let mut counter = 0usize;
            let mut lv = cview(&self.line).to_vec();
            let mut r = lv.iter().position(|&c| c == b'[').unwrap() + 1;
            if lv.get(r) == Some(&b' ') {
                r += 1;
            }
            loop {
                while lv.get(r) == Some(&b'/') {
                    r += 1;
                    let mut name = Vec::new();
                    while let Some(&c) = lv.get(r) {
                        if c == 32 || c == 10 || c == b']' || c == b'/' {
                            break;
                        }
                        name.push(c);
                        r += 1;
                    }
                    if lv.get(r) == Some(&b' ') {
                        r += 1;
                    }
                    if counter > 255 {
                        self.fail("encoding vector contains more than 256 names");
                    }
                    if name.as_slice() != NOTDEF {
                        glyph_names[counter] = name;
                    }
                    counter += 1;
                }
                let c = lv.get(r).copied().unwrap_or(0);
                if c != 10 && c != b'%' {
                    let rest = &lv[r.min(lv.len())..];
                    if rest.starts_with(b"] def") || rest.starts_with(b"] readonly def") {
                        break;
                    }
                    let l = without_eol(&self.line);
                    self.fail(&format!(
                        "a name or `] def' or `] readonly def' expected: `{l}'"
                    ));
                }
                self.getline();
                lv = cview(&self.line).to_vec();
                r = 0;
            }
        } else {
            // the second case
            let mut lv = cview(&self.line).to_vec();
            let mut p = lv.iter().position(|&c| c == 10).unwrap_or(lv.len());
            loop {
                if lv.get(p).copied().unwrap_or(0) == 10 {
                    self.getline();
                    lv = cview(&self.line).to_vec();
                    p = 0;
                }
                let rest = lv[p.min(lv.len())..].to_vec();
                // check for `dup <index> <glyph> put'
                let (n, i, name) = cfmt::scan_dup_put(&rest);
                if n == 2 && name.first() == Some(&b'/') && (0..256).contains(&i) {
                    if &name[1..] != NOTDEF {
                        glyph_names[i as usize] = name[1..].to_vec();
                    }
                    match find(&rest, b" put") {
                        None => self.fail("invalid pfb, no put found in dup"),
                        Some(k) => p += k + 4,
                    }
                    if lv.get(p) == Some(&b' ') {
                        p += 1;
                    }
                    continue;
                }
                // check for `dup dup <to> exch <from> get put'
                let (n, v) = cfmt::scan_ints(&rest, c"dup dup %i exch %i get put");
                let (b, a) = (v[0], v[1]);
                if n == 2 && (0..256).contains(&a) && (0..256).contains(&b) {
                    copy_glyph_names(&mut glyph_names, a as usize, b as usize);
                    match find(&rest, b" get put") {
                        None => self.fail("invalid pfb, no get put found in dup dup"),
                        Some(k) => p += k + 8,
                    }
                    if lv.get(p) == Some(&b' ') {
                        p += 1;
                    }
                    continue;
                }
                // check for `dup dup <from> <size> getinterval <to> exch putinterval'
                let (n, v) =
                    cfmt::scan_ints(&rest, c"dup dup %i %i getinterval %i exch putinterval");
                let (a, c, b) = (v[0], v[1], v[2]);
                if n == 3 && (0..256).contains(&a) && (0..256).contains(&b) && (0..256).contains(&c)
                {
                    for i in 0..c {
                        let (x, y) = ((a + i) as usize, (b + i) as usize);
                        if x < 256 && y < 256 {
                            copy_glyph_names(&mut glyph_names, x, y);
                        }
                    }
                    match find(&rest, b" putinterval") {
                        None => self.fail("invalid pfb, no putinterval found in dup dup"),
                        Some(k) => p += k + 12,
                    }
                    if lv.get(p) == Some(&b' ') {
                        p += 1;
                    }
                    continue;
                }
                // check for `def' or `readonly def'
                if (p == 0 || lv.get(p.wrapping_sub(1)) == Some(&b' ')) && rest == b"def\n" {
                    return glyph_names;
                }
                // skip an unrecognizable word
                while let Some(&c) = lv.get(p) {
                    if c == b' ' || c == 10 {
                        break;
                    }
                    p += 1;
                }
                if lv.get(p) == Some(&b' ') {
                    p += 1;
                }
                if p >= lv.len() {
                    // C reads the NUL here and stays; the next round tests it
                    // against LF only, so an unterminated line cannot occur:
                    // every line from t1_getline ends with LF.
                    p = lv.len();
                    lv.push(10);
                }
            }
        }
        glyph_names
    }

    // --- whole font --------------------------------------------------------

    /// `t1_include`: the whole font, not subsetted.
    fn include(&mut self) {
        loop {
            self.getline();
            self.scan_param();
            self.putline();
            if self.in_eexec != 0 {
                break;
            }
        }
        self.start_eexec();
        loop {
            self.getline();
            self.scan_param();
            self.putline();
            if self.charstrings() || self.subrs() {
                break;
            }
        }
        self.cs = true;
        loop {
            self.getline();
            self.putline();
            if self.end_eexec() {
                break;
            }
        }
        self.stop_eexec();
        self.get_length3();
    }

    // --- subsetting --------------------------------------------------------

    fn check_subr(&mut self, subr: i32) {
        if subr >= self.subr_size || subr < 0 {
            self.fail(&format!("Subrs array: entry index out of range ({subr})"));
        }
    }

    /// `check_cs_token_pair`.
    fn check_cs_token_pair(&self) -> Option<usize> {
        CS_TOKEN_PAIRS
            .iter()
            .position(|(a, b)| cview(&self.buf).starts_with(a) && str_suffix(&self.buf, b))
    }

    /// `cs_store`: keep the charstring or subr on the current line.
    fn cs_store(&mut self, is_subr: bool) {
        let line = self.line.clone();
        let sp = line.iter().position(|&c| c == b' ').unwrap_or(line.len());
        let head = line[..sp].to_vec();
        let idx;
        if is_subr {
            let subr = self.scan_num(sp + 1).0 as i32;
            self.check_subr(subr);
            idx = subr as usize;
        } else {
            idx = self.cs_tab.len();
            if idx as i32 + 1 > self.cs_size {
                self.fail(&format!(
                    "CharStrings dict: more entries than dict size ({})",
                    self.cs_size
                ));
            }
            let name = cview(head.get(1..).unwrap_or_default()).to_vec();
            self.cs_by_name.entry(name.clone()).or_insert(idx);
            self.cs_tab.push(CsEntry {
                name,
                ..Default::default()
            });
        }
        // copy " RD " + cs data to t1_buf_array
        let cslen = self.cslen as usize;
        let mut buf = line[self.cs_start - 4..self.cs_start + cslen].to_vec();
        // copy the end of cs data to t1_buf_array
        let mut p = self.cs_start + cslen;
        while p < line.len() && line[p] != 10 {
            buf.push(line[p]);
            p += 1;
        }
        buf.push(10);
        self.buf = buf;
        if is_subr && self.cs_token_pair.is_none() {
            self.cs_token_pair = self.check_cs_token_pair();
        }
        let len = self.buf.len() as u16;
        let entry = if is_subr {
            &mut self.subr_tab[idx]
        } else {
            &mut self.cs_tab[idx]
        };
        entry.len = len;
        entry.cslen = self.cslen;
        entry.data = self.buf[..len as usize].to_vec();
        entry.valid = true;
    }

    /// `cs_fail`.
    fn cs_fail(&mut self, cs_name: Option<&[u8]>, subr: i32, msg: String) -> ! {
        match cs_name {
            None => self.fail(&format!("Subr ({subr}): {msg}")),
            Some(n) => self.fail(&format!(
                "CharString (/{}): {msg}",
                String::from_utf8_lossy(n)
            )),
        }
    }

    fn stack_error(&mut self, n: i32) -> ! {
        let depth = self.stack.len();
        self.fail(&format!(
            "CharString: invalid access ({n}) to stack ({depth} entries)"
        ))
    }

    /// `cc_get(N)`.
    fn cc_get(&self, n: i32) -> i32 {
        let i = if n < 0 {
            self.stack.len() as i64 + n as i64
        } else {
            n as i64
        };
        if i < 0 {
            return 0;
        }
        self.stack.get(i as usize).copied().unwrap_or(0)
    }

    /// `cc_pop(N)`.
    fn cc_pop(&mut self, n: i32) {
        if (self.stack.len() as i64) < n as i64 {
            self.stack_error(n);
        }
        let k = self.stack.len() - n.max(0) as usize;
        self.stack.truncate(k);
    }

    /// `append_cs_return`: fix a return-less subr by appending CS_RETURN.
    fn append_cs_return(ptr: &mut CsEntry) {
        let mut cr: u16 = 4330;
        let mut plain = Vec::with_capacity(ptr.cslen as usize + 1);
        for i in 0..ptr.cslen as usize {
            plain.push(cdecrypt(ptr.data[4 + i], &mut cr));
        }
        plain.push(CS_RETURN as u8);
        let mut new_data = ptr.data[..4].to_vec();
        cr = 4330;
        for &b in &plain {
            new_data.push(cencrypt(b, &mut cr));
        }
        new_data.extend_from_slice(&ptr.data[4 + ptr.cslen as usize..ptr.len as usize]);
        ptr.data = new_data;
        ptr.len += 1;
        ptr.cslen += 1;
    }

    /// `cs_mark(cs_name, subr)`: mark a charstring (by name) or a subr (by
    /// number) and everything it calls.
    fn cs_mark(&mut self, cs_name: Option<&[u8]>, subr: i32) {
        let is_subr = cs_name.is_none();
        let idx: usize;
        match cs_name {
            None => {
                self.check_subr(subr);
                idx = subr as usize;
                if !self.subr_tab[idx].valid {
                    return;
                }
            }
            Some(name) => {
                if let (Some(nd), true) = (self.cs_notdef, name == NOTDEF) {
                    idx = nd;
                } else {
                    match self.cs_by_name.get(name) {
                        Some(&i) => idx = i,
                        None => {
                            let mut msg = b"glyph `".to_vec();
                            msg.extend_from_slice(name);
                            msg.extend_from_slice(b"' undefined");
                            self.g.pdftex_warn_bytes(&msg);
                            return;
                        }
                    }
                    if self.cs_tab[idx].name.as_slice() == NOTDEF {
                        self.cs_notdef = Some(idx);
                    }
                }
            }
        }
        {
            let ptr = if is_subr {
                &mut self.subr_tab[idx]
            } else {
                &mut self.cs_tab[idx]
            };
            // only marked CharString entries and invalid entries can be
            // skipped; valid marked subrs must be parsed to keep the stack
            // in sync
            if !ptr.valid || (ptr.used && !is_subr) {
                return;
            }
            ptr.used = true;
        }
        let data = if is_subr {
            self.subr_tab[idx].data.clone()
        } else {
            self.cs_tab[idx].data.clone()
        };
        let mut cr: u16 = 4330;
        let mut cs_len = if is_subr {
            self.subr_tab[idx].cslen as i32
        } else {
            self.cs_tab[idx].cslen as i32
        };
        let mut di = 4usize;
        let mut next = |cr: &mut u16| -> i32 {
            let b = data.get(di).copied().unwrap_or(0);
            di += 1;
            cdecrypt(b, cr) as i32
        };
        for _ in 0..self.len_iv {
            next(&mut cr);
            cs_len -= 1;
        }
        let mut last_cmd = 0usize;
        while cs_len > 0 {
            cs_len -= 1;
            let mut b = next(&mut cr);
            if b >= 32 {
                let a: i32 = if b <= 246 {
                    b - 139
                } else if b <= 250 {
                    cs_len -= 1;
                    ((b - 247) << 8) + 108 + next(&mut cr)
                } else if b <= 254 {
                    cs_len -= 1;
                    -((b - 251) << 8) - 108 - next(&mut cr)
                } else {
                    cs_len -= 4;
                    let mut a = ((next(&mut cr) & 0xff) as u32) << 24;
                    a |= ((next(&mut cr) & 0xff) as u32) << 16;
                    a |= ((next(&mut cr) & 0xff) as u32) << 8;
                    a |= (next(&mut cr) & 0xff) as u32;
                    a as i32
                };
                self.stack.push(a);
            } else {
                if b as usize == CS_ESCAPE {
                    b = next(&mut cr) + CS_1BYTE_MAX as i32;
                    cs_len -= 1;
                }
                if b as usize >= CS_MAX {
                    self.cs_fail(cs_name, subr, format!("command value out of range: {b}"));
                }
                let cc = self.cc[b as usize];
                if !cc.valid {
                    self.cs_fail(cs_name, subr, format!("command not valid: {b}"));
                }
                if cc.bottom {
                    let depth = self.stack.len();
                    if depth < cc.nargs as usize {
                        self.cs_fail(
                            cs_name,
                            subr,
                            format!(
                                "less arguments on stack ({depth}) than required ({})",
                                cc.nargs
                            ),
                        );
                    } else if depth > cc.nargs as usize {
                        self.cs_fail(
                            cs_name,
                            subr,
                            format!(
                                "more arguments on stack ({depth}) than required ({})",
                                cc.nargs
                            ),
                        );
                    }
                }
                last_cmd = b as usize;
                match b as usize {
                    CS_CALLSUBR => {
                        let a1 = self.cc_get(-1);
                        self.cc_pop(1);
                        if self.cs_depth >= CS_SUBR_NEST_MAX {
                            self.cs_fail(
                                cs_name,
                                subr,
                                format!(
                                    "cannot call subr ({a1}): more than \
                                     {CS_SUBR_NEST_MAX} nested subr calls"
                                ),
                            );
                        }
                        self.cs_depth += 1;
                        self.cs_mark(None, a1);
                        self.cs_depth -= 1;
                        if !self.subr_tab[a1 as usize].valid {
                            self.cs_fail(cs_name, subr, format!("cannot call subr ({a1})"));
                        }
                    }
                    CS_DIV => {
                        self.cc_pop(2);
                        self.stack.push(0);
                    }
                    CS_CALLOTHERSUBR => {
                        if self.cc_get(-1) == 3 {
                            self.persist.last_arg_other_subr3 = self.cc_get(-3);
                        }
                        let a1 = self.cc_get(-2) + 2;
                        self.cc_pop(a1);
                    }
                    CS_POP => {
                        // the only case when we care about the value being
                        // pushed onto stack is when POP follows
                        // CALLOTHERSUBR (changing hints by OtherSubrs[3])
                        self.stack.push(self.persist.last_arg_other_subr3);
                    }
                    CS_SEAC => {
                        let a1 = self.cc_get(3);
                        let a2 = self.cc_get(4);
                        self.stack.clear();
                        let n1 = standard_glyph_name(a1 as usize);
                        let n2 = standard_glyph_name(a2 as usize);
                        self.cs_mark(Some(n1), 0);
                        self.cs_mark(Some(n2), 0);
                        // base and accent characters are needed in CharSet
                        if let Some(gl) = self.fd.gl_tree.as_mut() {
                            gl.insert(n1.to_vec());
                            gl.insert(n2.to_vec());
                        }
                    }
                    _ => {
                        if cc.clear {
                            self.stack.clear();
                        }
                    }
                }
            }
        }
        if is_subr && last_cmd != CS_RETURN {
            self.g.pdftex_warn(&format!(
                "last command in subr `{subr}' is not a RETURN; \
                 I will add it now but please consider fixing the font"
            ));
            Self::append_cs_return(&mut self.subr_tab[idx]);
        }
    }

    /// `t1_subset_ascii_part`.
    fn subset_ascii_part(&mut self) {
        self.getline();
        while !self.prefix(b"/Encoding") {
            self.scan_param();
            let lv = cview(&self.line);
            let unique_id_def = self.prefix(b"/UniqueID")
                && lv.len() >= 4
                && lv[lv.len() - 4..].starts_with(b"def");
            if !unique_id_def {
                self.putline();
            }
            self.getline();
        }
        let glyph_names = self.builtin_enc();
        self.fd.builtin_glyph_names = Some(glyph_names.clone());
        if self.fm.is_subsetted() {
            if let Some(tx) = self.fd.tx_tree.clone() {
                // take over collected non-reencoded characters from TeX
                let gl = self.fd.gl_tree.get_or_insert_with(Default::default);
                for p in tx {
                    gl.insert(glyph_names[p as usize].clone());
                }
            }
            let gl = self.fd.gl_tree.clone().unwrap_or_default();
            let fontname = self.fd.fontname.clone().unwrap_or_default();
            let tag = self.g.make_subset_tag(gl.iter(), &fontname);
            self.fd.subset_tag = Some(tag);
            let off = (self.fontname_offset - self.fb_base) as usize;
            if self.fontname_offset != 0 && off + 6 <= self.fb.len() {
                self.fb[off..off + 6].copy_from_slice(&tag);
            }
        }
        // now really all glyphs needed from this font are in fd.gl_tree
        if self.encoding == Encoding::Standard {
            self.puts(b"/Encoding StandardEncoding def\n");
        } else {
            self.puts(b"/Encoding 256 array\n0 1 255 {1 index exch /.notdef put} for\n");
            // create_t1_glyph_tree: glyph name to its first code
            let mut t1_glyphs: BTreeMap<&[u8], usize> = BTreeMap::new();
            for (i, g) in glyph_names.iter().enumerate() {
                if g.as_slice() != NOTDEF {
                    t1_glyphs.entry(g.as_slice()).or_insert(i);
                }
            }
            let mut j = 0;
            let gl = self.fd.gl_tree.clone().unwrap_or_default();
            for glyph in &gl {
                if let Some(&i) = t1_glyphs.get(glyph.as_slice()) {
                    let mut s = format!("dup {i} /").into_bytes();
                    s.extend_from_slice(glyph);
                    s.extend_from_slice(b" put\n");
                    self.puts(&s);
                    j += 1;
                }
            }
            if j == 0 {
                // We didn't mark anything for the Encoding array. We add
                // "dup 0 /.notdef put" for compatibility with Acrobat 5.0.
                self.puts(b"dup 0 /.notdef put\n");
            }
            self.puts(b"readonly def\n");
        }
        loop {
            self.getline();
            self.scan_param();
            if !self.prefix(b"/UniqueID") {
                // ignore UniqueID for subsetted fonts
                self.putline();
            }
            if self.in_eexec != 0 {
                break;
            }
        }
    }

    /// `cs_init`.
    fn cs_init(&mut self) {
        self.cs_tab.clear();
        self.cs_by_name.clear();
        self.cs_dict_start.clear();
        self.cs_dict_end.clear();
        self.cs_count = 0;
        self.cs_size = 0;
        self.cs_size_pos = 0;
        self.cs_token_pair = None;
        self.subr_tab.clear();
        self.subr_array_start.clear();
        self.subr_array_end.clear();
        self.subr_max = 0;
        self.subr_size = 0;
        self.subr_size_pos = 0;
    }

    /// `t1_read_subrs`.
    fn read_subrs(&mut self) {
        self.getline();
        while !(self.charstrings() || self.subrs()) {
            self.scan_param();
            if !self.prefix(b"/UniqueID") {
                // ignore UniqueID for subsetted fonts
                self.putline();
            }
            self.getline();
        }
        loop {
            // found:
            self.cs = true;
            self.scan = false;
            if !self.subrs() {
                return;
            }
            self.subr_size_pos = b"/Subrs".len() + 1;
            // subr_size_pos points to the number indicating dict size after
            // "/Subrs"
            self.subr_size = self.scan_num(self.subr_size_pos).0 as i32;
            if self.subr_size == 0 {
                while !self.charstrings() {
                    self.getline();
                }
                return;
            }
            self.subr_tab = vec![CsEntry::default(); self.subr_size.max(0) as usize];
            self.subr_array_start = cview(&self.line).to_vec();
            self.getline();
            while self.cslen != 0 {
                self.cs_store(true);
                self.getline();
            }
            // mark the first four entries without parsing
            for e in self.subr_tab.iter_mut().take(4) {
                e.used = true;
            }
            // the end of the Subrs array might have more than one line so we
            // need to concatenate them to subr_array_end. Unfortunately some
            // fonts don't have the Subrs array followed by the CharStrings
            // dict immediately (synthetic fonts). If we cannot find
            // CharStrings in next POST_SUBRS_SCAN lines then we will treat
            // the font as synthetic and ignore everything until next Subrs is
            // found
            let mut end = Vec::new();
            let mut i = 0;
            while i < POST_SUBRS_SCAN {
                if self.charstrings() {
                    break;
                }
                end.extend_from_slice(cview(&self.line));
                self.getline();
                i += 1;
            }
            self.subr_array_end = end;
            if i == POST_SUBRS_SCAN {
                // CharStrings not found; suppose synthetic font
                self.cs_init();
                self.cs = false;
                self.synthetic = true;
                while !(self.charstrings() || self.subrs()) {
                    self.getline();
                }
                continue; // goto found
            }
            return;
        }
    }

    /// `t1_flush_cs`: write the kept subrs (`is_subr`) or charstrings.
    fn flush_cs(&mut self, is_subr: bool) {
        let (start_line, line_end, size_pos, count) = if is_subr {
            (
                std::mem::take(&mut self.subr_array_start),
                std::mem::take(&mut self.subr_array_end),
                self.subr_size_pos,
                self.subr_max + 1,
            )
        } else {
            (
                std::mem::take(&mut self.cs_dict_start),
                std::mem::take(&mut self.cs_dict_end),
                self.cs_size_pos,
                self.cs_count,
            )
        };
        let tab = if is_subr {
            let n = count.max(0) as usize;
            let mut t = std::mem::take(&mut self.subr_tab);
            t.truncate(n);
            t
        } else {
            std::mem::take(&mut self.cs_tab)
        };
        let mut line = start_line[..size_pos.min(start_line.len())].to_vec();
        let mut p = size_pos.min(start_line.len());
        while p < start_line.len() && start_line[p].is_ascii_digit() {
            p += 1;
        }
        line.extend_from_slice(format!("{}", count as u32).as_bytes());
        line.extend_from_slice(&start_line[p..]);
        self.line = line;
        eol(&mut self.line);
        self.putline();

        // create return_cs to replace unused subr's
        let mut return_cs = Vec::new();
        let mut cs_len: u16 = 0;
        if is_subr {
            let mut cr: u16 = 4330;
            for _ in 0..self.len_iv {
                return_cs.push(cencrypt(0x00, &mut cr));
                cs_len += 1;
            }
            return_cs.push(cencrypt(CS_RETURN as u8, &mut cr));
            cs_len += 1;
        }
        let pair = self.cs_token_pair.map(|i| CS_TOKEN_PAIRS[i]);
        for (i, e) in tab.iter().enumerate() {
            if e.used {
                let mut l = if is_subr {
                    format!("dup {} {}", i, e.cslen).into_bytes()
                } else {
                    let mut l = b"/".to_vec();
                    l.extend_from_slice(&e.name);
                    l.extend_from_slice(format!(" {}", e.cslen).as_bytes());
                    l
                };
                l.extend_from_slice(&e.data[..e.len as usize]);
                self.line = l;
                self.putline();
            } else if is_subr {
                // replace unused subr's by return_cs
                let (open, close) = pair.unwrap_or((b"", b""));
                let mut l = format!("dup {i} {cs_len}").into_bytes();
                l.extend_from_slice(open);
                l.push(b' ');
                l.extend_from_slice(&return_cs);
                self.line = l;
                self.putline();
                let mut l = b" ".to_vec();
                l.extend_from_slice(close);
                self.line = l;
                eol(&mut self.line);
                self.putline();
            }
        }
        self.line = cview(&line_end).to_vec();
        eol(&mut self.line);
        self.putline();
    }

    /// `t1_mark_glyphs`.
    fn mark_glyphs(&mut self) {
        if self.synthetic || self.fd.all_glyphs {
            // mark everything
            for e in self.cs_tab.iter_mut() {
                if e.valid {
                    e.used = true;
                }
            }
            if !self.subr_tab.is_empty() {
                for e in self.subr_tab.iter_mut() {
                    if e.valid {
                        e.used = true;
                    }
                }
                self.subr_max = self.subr_size - 1;
            }
            return;
        }
        self.cs_mark(Some(NOTDEF), 0);
        let gl: Vec<Vec<u8>> = self.fd.gl_tree.iter().flatten().cloned().collect();
        for glyph in &gl {
            self.cs_mark(Some(glyph), 0);
        }
        if !self.subr_tab.is_empty() {
            self.subr_max = -1;
            for (i, e) in self.subr_tab.iter().enumerate() {
                if e.used && i as i32 > self.subr_max {
                    self.subr_max = i as i32;
                }
            }
        }
    }

    /// `t1_check_unusual_charstring`: `/CharStrings` with its size on the
    /// next line.
    fn check_unusual_charstring(&mut self) {
        let lv = cview(&self.line).to_vec();
        let p = find(&lv, CHARSTRINGNAME).unwrap() + CHARSTRINGNAME.len();
        // if no number follows "/CharStrings", let's read the next line
        if cfmt::scan_one_int(&lv[p..]).is_none() {
            let mut buf = lv.clone();
            // t1_getline always appends EOL to t1_line_array; let's change
            // it to space before appending the next line
            if let Some(last) = buf.last_mut() {
                *last = b' ';
            }
            self.getline();
            buf.extend_from_slice(cview(&self.line));
            self.line = buf;
            eol(&mut self.line);
        }
    }

    /// `t1_subset_charstrings`.
    fn subset_charstrings(&mut self) {
        // at this point the line contains "/CharStrings"; when we hit a
        // case like "dup/CharStrings\n229 dict dup begin" we read the next
        // line and concatenate it before moving on
        self.check_unusual_charstring();
        let lv = cview(&self.line).to_vec();
        self.cs_size_pos = find(&lv, CHARSTRINGNAME).unwrap() + CHARSTRINGNAME.len() + 1;
        // cs_size_pos points to the number indicating dict size after
        // "/CharStrings"
        self.cs_size = self.scan_num(self.cs_size_pos).0 as i32;
        self.cs_tab = Vec::with_capacity(self.cs_size.max(0) as usize);
        self.cs_by_name.clear();
        self.cs_notdef = None;
        self.cs_dict_start = lv;
        self.getline();
        while self.cslen != 0 {
            self.cs_store(false);
            self.getline();
        }
        self.cs_dict_end = cview(&self.line).to_vec();
        self.mark_glyphs();
        if !self.subr_tab.is_empty() {
            if self.cs_token_pair.is_none() {
                self.fail("This Type 1 font uses mismatched subroutine begin/end token pairs.");
            }
            self.flush_cs(true);
        }
        self.cs_count = self.cs_tab.iter().filter(|e| e.used).count() as i32;
        self.flush_cs(false);
    }

    /// `t1_subset_end`.
    fn subset_end(&mut self) {
        if self.synthetic {
            // copy to "dup /FontName get exch definefont pop"
            while find(cview(&self.line), b"definefont").is_none() {
                self.getline();
                self.putline();
            }
            while !self.end_eexec() {
                self.getline(); // ignore the rest
            }
            self.putline(); // write "mark currentfile closefile"
        } else {
            while !self.end_eexec() {
                // copy to "mark currentfile closefile"
                self.getline();
                self.putline();
            }
        }
        self.stop_eexec();
        self.get_length3();
    }
}

/// `copy_glyph_names`.
fn copy_glyph_names(glyph_names: &mut GlyphNames, a: usize, b: usize) {
    if glyph_names[b].as_slice() != NOTDEF {
        glyph_names[b] = NOTDEF.to_vec();
    }
    if glyph_names[a].as_slice() != NOTDEF {
        glyph_names[b] = glyph_names[a].clone();
    }
}

/// `cdecrypt`.
fn cdecrypt(cipher: u8, cr: &mut u16) -> u8 {
    let plain = cipher ^ (*cr >> 8) as u8;
    *cr = ((cipher as u32 + *cr as u32)
        .wrapping_mul(T1_C1)
        .wrapping_add(T1_C2)) as u16;
    plain
}

/// `cencrypt`.
fn cencrypt(plain: u8, cr: &mut u16) -> u8 {
    let cipher = plain ^ (*cr >> 8) as u8;
    *cr = ((cipher as u32 + *cr as u32)
        .wrapping_mul(T1_C1)
        .wrapping_add(T1_C2)) as u16;
    cipher
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
        set_cur_file_name(Some(enc_name));
        let name = String::from_utf8_lossy(enc_name).into_owned();
        let Some((path, data)) = crate::system::find_file(&name, Format::Enc)
            .and_then(|p| std::fs::read(&p).ok().map(|d| (p, d)))
        else {
            self.pdftex_fail("cannot open encoding file for reading");
        };
        let mut glyph_names = notdef_names();
        self.tex_printf(b"{");
        set_cur_file_name(Some(path.as_bytes()));
        self.tex_printf(path.as_bytes());
        let mut pos = 0usize;
        let mut eof = false;
        // enc_getline
        let mut getline = |g: &mut Globals| -> Vec<u8> {
            loop {
                if eof {
                    g.pdftex_fail("unexpected end of file");
                }
                let mut line = Vec::new();
                loop {
                    let c = match data.get(pos) {
                        Some(&b) => {
                            pos += 1;
                            b as i32
                        }
                        None => {
                            eof = true;
                            -1
                        }
                    };
                    let c = append_char(c, &mut line);
                    if line.len() + 1 > ENC_BUF_SIZE {
                        g.pdftex_fail("buffer overflow at file writet1.c, line 223");
                    }
                    if c == 10 {
                        break;
                    }
                }
                append_eol(&mut line);
                if line.len() < 2 || line[0] == b'%' {
                    continue;
                }
                return line;
            }
        };
        let mut line = getline(self);
        let bracket = line.iter().position(|&c| c == b'[');
        if line[0] != b'/' || bracket.is_none() {
            let l = without_eol(&line);
            self.pdftex_fail(&format!(
                "invalid encoding vector (a name or `[' missing): `{l}'"
            ));
        }
        let mut names_count = 0usize;
        let mut r = bracket.unwrap() + 1; // skip '['
        if line.get(r) == Some(&b' ') {
            r += 1;
        }
        loop {
            while line.get(r) == Some(&b'/') {
                r += 1;
                let mut buf = Vec::new();
                while let Some(&c) = line.get(r) {
                    if c == b' ' || c == 10 || c == b']' || c == b'/' {
                        break;
                    }
                    buf.push(c);
                    r += 1;
                }
                if line.get(r) == Some(&b' ') {
                    r += 1;
                }
                if names_count > 255 {
                    self.pdftex_fail("encoding vector contains more than 256 names");
                }
                if buf.as_slice() != NOTDEF {
                    glyph_names[names_count] = buf;
                }
                names_count += 1;
            }
            let c = line.get(r).copied().unwrap_or(0);
            if c != 10 && c != b'%' {
                if line[r.min(line.len())..].starts_with(b"] def") {
                    break;
                }
                let l = without_eol(&line);
                self.pdftex_fail(&format!(
                    "invalid encoding vector: a name or `] def' expected: `{l}'"
                ));
            }
            line = getline(self);
            r = 0;
        }
        self.tex_printf(b"}");
        set_cur_file_name(None);
        glyph_names
    }

    /// `writet1` (writet1.c): embed the Type 1 font of `fd` into the font
    /// buffer, subsetted to its glyph tree unless its map entry says `<<`.
    pub fn writet1(&mut self, st: &mut Fonts, fd: &mut FdEntry, persist: &mut Persist) -> T1Result {
        let fm = st.map.fms[fd.fm].clone().expect("live map entry");
        let ff_name = fm.ff_name.clone().unwrap_or_default();
        // t1_open_fontfile
        let path = self.check_ff_exist(st, &ff_name, fm.is_truetype());
        let file = match path.as_ref().and_then(|p| std::fs::read(p).ok()) {
            Some(d) => {
                set_cur_file_name(path.as_ref().map(|p| p.as_bytes()));
                d
            }
            None => {
                set_cur_file_name(Some(&ff_name));
                self.pdftex_fail("cannot open Type 1 font file for reading");
            }
        };
        let fb_base = self.fb_offset();
        let subsetted = fm.is_subsetted();
        let mut t = T1 {
            g: self,
            fd,
            fm,
            persist,
            file,
            pos: 0,
            eof: false,
            fb: Vec::new(),
            fb_base,
            pfa: false,
            block_length: 0,
            dr: 0,
            er: 0,
            len_iv: 4,
            in_eexec: 0,
            cs: false,
            scan: true,
            eexec_encrypt: false,
            synthetic: false,
            last_hexbyte: 0,
            line: Vec::new(),
            buf: Vec::new(),
            cslen: 0,
            cs_start: 0,
            encoding: Encoding::Standard,
            save_offset: 0,
            fontname_offset: 0,
            length1: 0,
            length2: 0,
            length3: 0,
            cc: cc_tab(),
            stack: Vec::new(),
            cs_tab: Vec::new(),
            cs_by_name: HashMap::new(),
            cs_size: 0,
            cs_count: 0,
            cs_size_pos: 0,
            cs_dict_start: Vec::new(),
            cs_dict_end: Vec::new(),
            cs_notdef: None,
            cs_token_pair: None,
            subr_tab: Vec::new(),
            subr_max: 0,
            subr_size: 0,
            subr_size_pos: 0,
            subr_array_start: Vec::new(),
            subr_array_end: Vec::new(),
            cs_depth: 0,
        };
        if !subsetted {
            // include entire font
            t.init_params(b"<<");
            t.include();
            t.g.tex_printf(b">>");
        } else {
            // partial downloading
            t.init_params(b"<");
            t.subset_ascii_part();
            t.start_eexec();
            t.cs_init();
            t.read_subrs();
            t.subset_charstrings();
            t.subset_end();
            t.g.tex_printf(b">");
        }
        set_cur_file_name(None);
        let r = T1Result {
            ff_found: true,
            length1: t.length1,
            length2: t.length2,
            length3: t.length3,
        };
        let fb = std::mem::take(&mut t.fb);
        super::with_state(|s| s.out.fb.extend_from_slice(&fb));
        r
    }
}
