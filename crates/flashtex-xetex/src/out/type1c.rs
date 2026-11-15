//! A TFM font's Type 1 program converted to a bare CFF font (`/FontFile3
//! /Subtype /Type1C`) as xdvipdfmx converts it (dvipdfm-x 20260317):
//! t1_load.c `t1_load_font` reads the font dictionary and the Private
//! dictionary into CFF DICTs, type1.c `pdf_font_load_type1` subsets the
//! glyphs the page used (encoding order, `seac` components appended),
//! t1_char.c converts each charstring to Type 2 (`super::t1char`), and
//! type1.c `write_fontfile` writes the CFF (cff.c, cff_dict.c).

use super::t1char::{self, Fatal};
use flashtex_pdf::cff::STANDARD_STRING_NAMES;
use flashtex_pdf::type1::{decrypt, standard_encoding_name};
use std::collections::BTreeSet;

const STDSTR_MAX: usize = 391;

// ---------------------------------------------------------------------------
// cff_dict.c
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum ArgType {
    Number,
    Sid,
    Offset,
}

/// `dict_operator`'s entries that a converted Type 1 font can have: the
/// name, the operator (`12 x` as `0x0c00 | x`) and the operand type.
fn operator(key: &str) -> Option<(u16, ArgType)> {
    use ArgType::*;
    Some(match key {
        "version" => (0, Sid),
        "Notice" => (1, Sid),
        "FullName" => (2, Sid),
        "FamilyName" => (3, Sid),
        "Weight" => (4, Sid),
        "FontBBox" => (5, Number),
        "BlueValues" => (6, Number),
        "OtherBlues" => (7, Number),
        "FamilyBlues" => (8, Number),
        "FamilyOtherBlues" => (9, Number),
        "StdHW" => (10, Number),
        "StdVW" => (11, Number),
        "charset" => (15, Offset),
        "Encoding" => (16, Offset),
        "CharStrings" => (17, Offset),
        "Private" => (18, Offset),
        "defaultWidthX" => (20, Number),
        "nominalWidthX" => (21, Number),
        "Copyright" => (0x0c00, Sid),
        "IsFixedPitch" => (0x0c01, Number),
        "ItalicAngle" => (0x0c02, Number),
        "PaintType" => (0x0c05, Number),
        "FontMatrix" => (0x0c07, Number),
        "StrokeWidth" => (0x0c08, Number),
        "BlueScale" => (0x0c09, Number),
        "BlueShift" => (0x0c0a, Number),
        "BlueFuzz" => (0x0c0b, Number),
        "StemSnapH" => (0x0c0c, Number),
        "StemSnapV" => (0x0c0d, Number),
        "ForceBold" => (0x0c0e, Number),
        "LanguageGroup" => (0x0c11, Number),
        "ExpansionFactor" => (0x0c12, Number),
        _ => return None,
    })
}

/// A CFF DICT: its entries in the order they were added (`cff_dict_add`).
#[derive(Clone, Debug, Default)]
pub struct Dict(Vec<(&'static str, Vec<f64>)>);

impl Dict {
    /// `cff_dict_add`: a new entry of `count` zeros, or nothing when the
    /// key is there.
    fn add(&mut self, key: &'static str, count: usize) {
        if !self.0.iter().any(|(k, _)| *k == key) {
            self.0.push((key, vec![0.0; count]));
        }
    }

    fn set(&mut self, key: &str, idx: usize, v: f64) {
        if let Some((_, vals)) = self.0.iter_mut().find(|(k, _)| *k == key) {
            if let Some(x) = vals.get_mut(idx) {
                *x = v;
            }
        }
    }

    pub fn get(&self, key: &str, idx: usize) -> Option<f64> {
        self.0
            .iter()
            .find(|(k, v)| *k == key && !v.is_empty())
            .and_then(|(_, v)| v.get(idx).copied())
    }

    /// `cff_dict_pack`.
    fn pack(&self, out: &mut Vec<u8>) {
        for (key, vals) in &self.0 {
            if vals.is_empty() {
                continue;
            }
            let (op, ty) = operator(key).expect("a known operator");
            for &v in vals {
                put_number(v, ty == ArgType::Offset, out);
            }
            if op >= 0x0c00 {
                out.extend_from_slice(&[12, (op & 0xff) as u8]);
            } else {
                out.push(op as u8);
            }
        }
    }
}

/// `cff_dict_put_number`.
fn put_number(value: f64, offset: bool, out: &mut Vec<u8>) {
    if offset {
        let v = value as i32;
        out.push(29);
        out.extend_from_slice(&v.to_be_bytes());
        return;
    }
    let nearint = (value + 0.5).floor();
    if value > i32::MAX as f64 || value < i32::MIN as f64 || (value - nearint).abs() > 1.0e-5 {
        pack_real(value, out);
    } else {
        pack_integer(nearint as i32, out);
    }
}

fn pack_integer(v: i32, out: &mut Vec<u8>) {
    if (-107..=107).contains(&v) {
        out.push((v + 139) as u8);
    } else if (108..=1131).contains(&v) {
        let w = 0xf700 + v - 108;
        out.extend_from_slice(&[(w >> 8) as u8, w as u8]);
    } else if (-1131..=-108).contains(&v) {
        let w = 0xfb00 - v - 108;
        out.extend_from_slice(&[(w >> 8) as u8, w as u8]);
    } else if (-32768..=32767).contains(&v) {
        out.extend_from_slice(&[28, (v >> 8) as u8, v as u8]);
    } else {
        out.push(29);
        out.extend_from_slice(&v.to_be_bytes());
    }
}

/// C's `%.13g`.
fn fmt_g13(v: f64) -> String {
    const P: i32 = 13;
    if v == 0.0 {
        return "0".into();
    }
    let e = format!("{:.*e}", (P - 1) as usize, v);
    let (mant, exp) = e.split_once('e').unwrap();
    let x: i32 = exp.parse().unwrap();
    let trim = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if (-4..P).contains(&x) {
        trim(&format!("{:.*}", (P - 1 - x) as usize, v))
    } else {
        let m = trim(mant);
        format!("{m}e{}{:02}", if x < 0 { '-' } else { '+' }, x.abs())
    }
}

/// `pack_real`: CFF's nibble-coded real.
fn pack_real(value: f64, out: &mut Vec<u8>) {
    out.push(30);
    if value == 0.0 {
        out.push(0x0f);
        return;
    }
    let mut nib: Vec<u8> = Vec::new();
    let mut v = value;
    if v < 0.0 {
        nib.push(0x0e);
        v = -v;
    }
    let s = fmt_g13(v);
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        nib.push(match c {
            b'.' => 0x0a,
            b'0'..=b'9' => c - b'0',
            b'e' => {
                i += 1;
                if b.get(i) == Some(&b'-') {
                    0x0c
                } else {
                    0x0b
                }
            }
            _ => 0x0f,
        });
        i += 1;
    }
    nib.push(0x0f);
    if nib.len() % 2 == 1 {
        nib.push(0x0f);
    }
    for p in nib.chunks(2) {
        out.push(p[0] << 4 | p[1]);
    }
}

// ---------------------------------------------------------------------------
// cff.c
// ---------------------------------------------------------------------------

/// `cff_pack_index`.
fn pack_index(items: &[Vec<u8>], out: &mut Vec<u8>) {
    if items.is_empty() {
        out.extend_from_slice(&[0, 0]);
        return;
    }
    let datalen: usize = items.iter().map(Vec::len).sum();
    let offsize = if datalen < 0xff {
        1
    } else if datalen < 0xffff {
        2
    } else if datalen < 0xff_ffff {
        3
    } else {
        4
    };
    out.extend_from_slice(&(items.len() as u16).to_be_bytes());
    out.push(offsize as u8);
    let mut off = 1usize;
    let put = |o: usize, out: &mut Vec<u8>| {
        out.extend_from_slice(&(o as u32).to_be_bytes()[4 - offsize..])
    };
    put(off, out);
    for it in items {
        off += it.len();
        put(off, out);
    }
    for it in items {
        out.extend_from_slice(it);
    }
}

/// The String INDEX being made (`cff->_string`) with `cff_add_string`.
#[derive(Default)]
struct Strings(Vec<Vec<u8>>);

impl Strings {
    fn add(&mut self, s: &[u8], unique: bool) -> u16 {
        if unique {
            if let Some(i) = STANDARD_STRING_NAMES.iter().position(|x| x.as_bytes() == s) {
                return i as u16;
            }
            if let Some(i) = self.0.iter().position(|x| x == s) {
                return (i + STDSTR_MAX) as u16;
            }
        }
        self.0.push(s.to_vec());
        (self.0.len() - 1 + STDSTR_MAX) as u16
    }
}

// ---------------------------------------------------------------------------
// pst.c, as t1_load.c reads a font with it
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Tok {
    Bool(bool),
    Int(i32),
    Real(f64),
    Str(Vec<u8>),
    Name(Vec<u8>),
    Mark,
    Other(Vec<u8>),
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | 0x0c | b'\r' | b'\n' | 0)
}

fn is_delim(c: u8) -> bool {
    matches!(
        c,
        b'(' | b')' | b'/' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'%'
    )
}

struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

impl Lexer<'_> {
    fn end(&self, i: usize) -> bool {
        i >= self.s.len() || is_delim(self.s[i]) || is_space(self.s[i])
    }

    fn at(&self, i: usize) -> u8 {
        self.s.get(i).copied().unwrap_or(0)
    }

    /// `pst_get_token` (comments and white space skipped).
    fn token(&mut self) -> Option<Tok> {
        loop {
            while self.i < self.s.len() && is_space(self.s[self.i]) {
                self.i += 1;
            }
            if self.at(self.i) == b'%' && self.i < self.s.len() {
                while self.i < self.s.len() && !matches!(self.s[self.i], b'\n' | b'\r') {
                    self.i += 1;
                }
                continue;
            }
            break;
        }
        if self.i >= self.s.len() {
            return None;
        }
        let c = self.s[self.i];
        let t = match c {
            b'/' => {
                self.i += 1;
                let st = self.i;
                while !self.end(self.i) {
                    self.i += 1;
                }
                Some(Tok::Name(self.s[st..self.i].to_vec()))
            }
            b'[' | b'{' => {
                self.i += 1;
                Some(Tok::Mark)
            }
            b'<' => {
                if self.at(self.i + 1) == b'<' {
                    self.i += 2;
                    Some(Tok::Mark)
                } else {
                    self.hex()
                }
            }
            b'(' => self.literal(),
            b'>' | b']' | b'}' => {
                let n = if c == b'>' && self.at(self.i + 1) == b'>' {
                    2
                } else {
                    1
                };
                let t = Tok::Other(self.s[self.i..self.i + n].to_vec());
                self.i += n;
                Some(t)
            }
            b't' | b'f' | b'n' => {
                let r = &self.s[self.i..];
                if r.starts_with(b"true") && self.end(self.i + 4) {
                    self.i += 4;
                    Some(Tok::Bool(true))
                } else if r.starts_with(b"false") && self.end(self.i + 5) {
                    self.i += 5;
                    Some(Tok::Bool(false))
                } else {
                    None
                }
            }
            b'+' | b'-' | b'0'..=b'9' | b'.' => self.number(),
            _ => None,
        };
        if t.is_some() {
            return t;
        }
        let st = self.i;
        while !self.end(self.i) {
            self.i += 1;
        }
        if self.i == st {
            self.i += 1;
        }
        Some(Tok::Other(self.s[st..self.i].to_vec()))
    }

    fn number(&mut self) -> Option<Tok> {
        let st = self.i;
        let mut j = st;
        while !self.end(j) {
            j += 1;
        }
        let w = std::str::from_utf8(&self.s[st..j]).ok()?;
        let tok = if let Ok(v) = w.parse::<i32>() {
            Tok::Int(v)
        } else if let Some((r, d)) = w.split_once('#') {
            let radix: u32 = r.parse().ok()?;
            if !(2..=36).contains(&radix) {
                return None;
            }
            Tok::Int(i32::from_str_radix(d, radix).ok()?)
        } else {
            Tok::Real(w.parse::<f64>().ok()?)
        };
        self.i = j;
        Some(tok)
    }

    fn literal(&mut self) -> Option<Tok> {
        let s = self.s;
        let mut i = self.i + 1;
        let mut out = Vec::new();
        let mut depth = 1;
        while i < s.len() {
            let c = s[i];
            i += 1;
            match c {
                b'\\' => {
                    let e = s.get(i).copied().unwrap_or(0);
                    i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'0'..=b'7' => {
                            let mut v = (e - b'0') as u32;
                            for _ in 0..2 {
                                match s.get(i) {
                                    Some(&d @ b'0'..=b'7') => {
                                        v = v * 8 + (d - b'0') as u32;
                                        i += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push(v as u8);
                        }
                        b'\n' => {}
                        b'\r' => {
                            if s.get(i) == Some(&b'\n') {
                                i += 1;
                            }
                        }
                        _ => out.push(e),
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(c);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        self.i = i;
                        return Some(Tok::Str(out));
                    }
                    out.push(c);
                }
                b'\r' => {
                    if s.get(i) == Some(&b'\n') {
                        i += 1;
                    }
                    out.push(b'\n');
                }
                _ => out.push(c),
            }
        }
        None
    }

    fn hex(&mut self) -> Option<Tok> {
        let mut i = self.i + 1;
        let mut d = Vec::new();
        while i < self.s.len() {
            let c = self.s[i];
            i += 1;
            if c == b'>' {
                if d.len() % 2 == 1 {
                    d.push(0);
                }
                self.i = i;
                return Some(Tok::Str(d.chunks(2).map(|p| p[0] << 4 | p[1]).collect()));
            }
            if let Some(v) = (c as char).to_digit(16) {
                d.push(v as u8);
            } else if !is_space(c) {
                d.push(0);
            }
        }
        None
    }

    fn get_next_key(&mut self) -> Option<Vec<u8>> {
        while self.i < self.s.len() {
            match self.token()? {
                Tok::Name(n) => return Some(n),
                _ => continue,
            }
        }
        None
    }

    fn seek_operator(&mut self, op: &[u8]) -> bool {
        while self.i < self.s.len() {
            match self.token() {
                Some(Tok::Other(o)) if o == op => return true,
                Some(_) => {}
                None => return false,
            }
        }
        false
    }

    /// `parse_nvalue`: up to `max` numbers, a number or an array of them;
    /// `None` where the C's count is not what the caller needs.
    fn nvalue(&mut self, max: usize) -> Option<Vec<f64>> {
        let num = |t: &Tok| match t {
            Tok::Int(i) => Some(*i as f64),
            Tok::Real(r) => Some(*r),
            _ => None,
        };
        let t = self.token()?;
        if let Some(v) = num(&t) {
            return (max > 0).then(|| vec![v]).or(Some(Vec::new()));
        }
        if t != Tok::Mark {
            return Some(Vec::new());
        }
        let mut out = Vec::new();
        loop {
            let t = self.token()?;
            match num(&t) {
                Some(v) if out.len() < max => out.push(v),
                _ => {
                    return match t {
                        Tok::Other(o) if o == b"]" || o == b"}" => Some(out),
                        _ => None,
                    }
                }
            }
        }
    }

    fn bvalue(&mut self) -> Option<bool> {
        match self.token()? {
            Tok::Bool(b) => Some(b),
            _ => None,
        }
    }

    fn svalue(&mut self) -> Option<Vec<u8>> {
        match self.token()? {
            Tok::Name(s) | Tok::Str(s) => Some(s),
            _ => None,
        }
    }

    /// The binary bytes after `RD`/`-|` (or `readstring`): `len` of
    /// them, decrypted with `lenIV` as `t1_decrypt` (`None` past the end).
    fn binary(&mut self, len: usize, len_iv: i32, last: usize) -> Option<Vec<u8>> {
        match self.token()? {
            Tok::Other(o) if o == b"RD" || o == b"-|" => {}
            _ => {
                if !self.seek_operator(b"readstring") {
                    return None;
                }
            }
        }
        self.i += 1;
        if self.i + len + last >= self.s.len() {
            return None;
        }
        let raw = &self.s[self.i..self.i + len];
        self.i += len;
        Some(if len_iv >= 0 {
            decrypt(raw, 4330, len_iv as usize)
        } else {
            raw.to_vec()
        })
    }

    /// `parse_encoding` (with `try_put_or_putinterval`): `None` is its
    /// failure (-1, ExpertEncoding).
    fn encoding(&mut self) -> Option<Vec<Option<Vec<u8>>>> {
        let named = |t: &dyn Fn(i32) -> Option<&'static str>| {
            (0..256)
                .map(|c| {
                    t(c).filter(|n| *n != ".notdef")
                        .map(|n| n.as_bytes().to_vec())
                })
                .collect::<Vec<_>>()
        };
        let op = |t: &Option<Tok>, o: &[u8]| matches!(t, Some(Tok::Other(x)) if x == o);
        let t = self.token();
        if op(&t, b"StandardEncoding") {
            return Some(named(&standard_encoding_name));
        }
        if op(&t, b"ISOLatin1Encoding") {
            return Some(named(&|c| ISO_LATIN1[c as usize]));
        }
        if op(&t, b"ExpertEncoding") {
            return None;
        }
        let mut enc: Vec<Option<Vec<u8>>> = vec![None; 256];
        self.seek_operator(b"array");
        let code_of = |t: Option<Tok>, max: i32| match t {
            Some(Tok::Int(n)) if (0..=max).contains(&n) => Some(n as usize),
            _ => None,
        };
        while self.i < self.s.len() {
            let Some(t) = self.token() else {
                break;
            };
            match &t {
                Tok::Other(o) if o == b"def" || o == b"readonly" => break,
                Tok::Other(o) if o == b"dup" => {}
                _ => continue,
            }
            let t = self.token();
            if op(&t, b"dup") {
                // dup num exch num get put / dup num num getinterval num
                // exch putinterval (cmctt10.pfb)
                let _ = (|| -> Option<()> {
                    let n1 = code_of(self.token(), 255)?;
                    let t = self.token()?;
                    if t == Tok::Other(b"exch".to_vec()) {
                        let n2 = code_of(self.token(), 255)?;
                        if !op(&self.token(), b"get") || !op(&self.token(), b"put") {
                            return None;
                        }
                        enc[n1] = enc[n2].clone();
                    } else if let Tok::Int(n2) = t {
                        if n2 < 0 || n2 as usize + n1 > 255 {
                            return None;
                        }
                        let n2 = n2 as usize;
                        if !op(&self.token(), b"getinterval") {
                            return None;
                        }
                        let n3 = match self.token()? {
                            Tok::Int(n) if n >= 0 && n as usize + n2 <= 255 => n as usize,
                            _ => return None,
                        };
                        if !op(&self.token(), b"exch") || !op(&self.token(), b"putinterval") {
                            return None;
                        }
                        for i in 0..n2 {
                            if enc[n1 + i].is_some() {
                                enc[n3 + i] = enc[n1 + i].clone();
                            }
                        }
                    } else {
                        return None;
                    }
                    Some(())
                })();
                continue;
            }
            let Some(code) = code_of(t, 255) else {
                continue;
            };
            let Some(Tok::Name(name)) = self.token() else {
                continue;
            };
            enc[code] = Some(name);
            if !op(&self.token(), b"put") {
                enc[code] = None;
            }
        }
        Some(enc)
    }

    /// `parse_subrs`: `None` is its failure (-1).
    fn subrs(
        &mut self,
        len_iv: i32,
        out: &mut Option<Vec<Vec<u8>>>,
        warn: &mut Vec<String>,
    ) -> Option<()> {
        let count = match self.token()? {
            Tok::Int(n) if n >= 0 => n as usize,
            _ => {
                warn.push("Parsing Subrs failed.".into());
                return None;
            }
        };
        if count == 0 {
            return Some(());
        }
        if self.token()? != Tok::Other(b"array".to_vec()) {
            return None;
        }
        let mut subrs = vec![Vec::new(); count];
        let mut i = 0;
        while i < count {
            match self.token()? {
                Tok::Other(o) if o == b"ND" || o == b"|-" || o == b"def" => break,
                Tok::Other(o) if o == b"dup" => {}
                _ => continue,
            }
            let idx = match self.token()? {
                Tok::Int(n) if n >= 0 && (n as usize) < count => n as usize,
                _ => return None,
            };
            let len = match self.token()? {
                Tok::Int(n) if (0..=65536).contains(&n) => n as usize,
                _ => return None,
            };
            subrs[idx] = self.binary(len, len_iv, 0)?;
            i += 1;
        }
        if out.is_none() {
            *out = Some(subrs);
        } else {
            warn.push("Already found /Subrs; ignores the other /Subrs dicts.".into());
        }
        Some(())
    }

    /// `parse_charstrings`: the glyphs, `.notdef` first, then the others
    /// in the program's order. `None` is its failure (-1).
    fn charstrings(
        &mut self,
        len_iv: i32,
        out: &mut Vec<(Vec<u8>, Vec<u8>)>,
        warn: &mut Vec<String>,
    ) -> Option<()> {
        let count = match self.token()? {
            Tok::Int(n) if (0..=64999).contains(&n) => n as usize,
            _ => {
                warn.push("Ignores non dict \"/CharStrings ...\"".into());
                return Some(());
            }
        };
        out.clear();
        let mut notdef: Option<(Vec<u8>, Vec<u8>)> = None;
        self.seek_operator(b"begin");
        for i in 0..count {
            let name = match self.token()? {
                Tok::Name(n) => n,
                Tok::Other(o) if o == b"end" => break,
                _ => return None,
            };
            if name != b".notdef" && notdef.is_none() && i == count - 1 {
                warn.push("No .notdef glyph???".into());
                return None;
            }
            let len = match self.token()? {
                Tok::Int(n) if (0..=65536).contains(&n) => n as usize,
                _ => return None,
            };
            let cs = self.binary(len, len_iv, 1)?;
            match self.token()? {
                Tok::Other(o) if o == b"ND" || o == b"|-" => {}
                _ => return None,
            }
            if name == b".notdef" {
                notdef = Some((name, cs));
            } else {
                out.push((name, cs));
            }
        }
        if let Some(n) = notdef {
            out.insert(0, n);
        }
        Some(())
    }
}

// ---------------------------------------------------------------------------
// t1_load.c
// ---------------------------------------------------------------------------

/// What `t1_load_font` makes of a Type 1 program: its CFF DICTs, Subrs
/// and charstrings (decrypted).
pub struct Loaded {
    pub top: Dict,
    pub private: Dict,
    /// The FontInfo strings, by key.
    pub strings: Vec<(&'static str, Vec<u8>)>,
    /// `/FontName` (at most 127 bytes, as `parse_part1` keeps it).
    pub font_name: Vec<u8>,
    /// The built-in encoding (`parse_encoding`), when the font has one:
    /// a glyph name or none for each code.
    pub encoding: Option<Vec<Option<Vec<u8>>>>,
    /// The Subrs (an undefined one empty); `None` without `/Subrs`.
    pub subrs: Option<Vec<Vec<u8>>>,
    /// The glyphs by name, `.notdef` first (CFF glyph order).
    pub glyphs: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Loaded {
    /// `cff_glyph_lookup`: the glyph's index, 0 (`.notdef`) for a name the
    /// font lacks.
    pub fn lookup(&self, name: &[u8]) -> usize {
        if name == b".notdef" {
            return 0;
        }
        self.glyphs
            .iter()
            .skip(1)
            .position(|(n, _)| n == name)
            .map_or(0, |i| i + 1)
    }

    /// `t1char_get_metrics` for glyph `gid`.
    pub fn metrics(&self, gid: usize, warn: &mut Vec<String>) -> Result<t1char::Glyph, Fatal> {
        let subrs = self.subrs.as_deref().unwrap_or(&[]);
        let subr = |i: usize| subrs.get(i).cloned();
        let cs = self.glyphs.get(gid).map_or(&[][..], |g| &g.1);
        t1char::metrics(cs, &subr, subrs.len(), warn)
    }

    /// `get_font_attr`'s `/Flags`: Symbolic, Serif unless the FontName has
    /// "Sans", SmallCap when it has "Caps", Italic for a non-zero
    /// ItalicAngle, ForceBold (FixedPitch never: `parse_part1` does not
    /// read `isFixedPitch` into the Private DICT).
    pub fn flags(&self) -> u32 {
        let has = |s: &[u8]| self.font_name.windows(s.len()).any(|w| w == s);
        let mut f = 4;
        if self.top.get("ItalicAngle", 0).is_some_and(|a| a != 0.0) {
            f |= 64;
        }
        if self.private.get("ForceBold", 0).is_some_and(|b| b != 0.0) {
            f |= 1 << 18;
        }
        if !has(b"Sans") {
            f |= 2;
        }
        if has(b"Caps") {
            f |= 1 << 17;
        }
        f
    }

    /// `get_font_attr`'s CapHeight, Ascent and Descent: the tops of "H"
    /// and "b" and the bottom of "p" (".notdef" for any the font lacks),
    /// with the FontBBox's (or 680, 690, -190 without one) when the
    /// font has no glyphs.
    pub fn vertical_metrics(&self, warn: &mut Vec<String>) -> Result<(f64, f64, f64), Fatal> {
        let (mut cap, mut asc, mut desc) = match self.top.get("FontBBox", 3) {
            Some(top) => (top, top, self.top.get("FontBBox", 1).unwrap_or(0.0)),
            None => (680.0, 690.0, -190.0),
        };
        if !self.glyphs.is_empty() {
            cap = self.metrics(self.lookup(b"H"), warn)?.bbox[3];
            desc = self.metrics(self.lookup(b"p"), warn)?.bbox[1];
            asc = self.metrics(self.lookup(b"b"), warn)?.bbox[3];
        }
        Ok((cap, asc, desc))
    }
}

fn pfb_segment(pfb: &[u8], at: &mut usize, ty: u8) -> Vec<u8> {
    let mut out = Vec::new();
    while pfb.get(*at) == Some(&0x80) && pfb.get(*at + 1) == Some(&ty) {
        let Some(len) = pfb
            .get(*at + 2..*at + 6)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        else {
            break;
        };
        out.extend_from_slice(pfb.get(*at + 6..*at + 6 + len).unwrap_or(&[]));
        *at += 6 + len;
    }
    out
}

const FAILED1: &str = "Reading PFB (ASCII part) file failed.";
const FAILED2: &str = "Reading PFB (BINARY part) file failed.";

/// `t1_load_font` (its dictionaries; `parse_part1`, `parse_part2`).
pub fn load(pfb: &[u8], warn: &mut Vec<String>) -> Result<Loaded, Fatal> {
    let mut at = 0;
    let clear = pfb_segment(pfb, &mut at, 1);
    let bin = pfb_segment(pfb, &mut at, 2);
    if clear.is_empty() {
        return Err(FAILED1.into());
    }
    let mut l = Loaded {
        top: Dict::default(),
        private: Dict::default(),
        strings: Vec::new(),
        font_name: Vec::new(),
        encoding: None,
        subrs: None,
        glyphs: Vec::new(),
    };
    let mut lx = Lexer { s: &clear, i: 0 };
    if !lx.seek_operator(b"begin") {
        return Err(FAILED1.into());
    }
    const SKEYS: [&str; 6] = [
        "version",
        "Notice",
        "FullName",
        "FamilyName",
        "Weight",
        "Copyright",
    ];
    while lx.i < lx.s.len() {
        let Some(key) = lx.get_next_key() else {
            break;
        };
        let key = String::from_utf8_lossy(&key).into_owned();
        match key.as_str() {
            "FontType" => {
                let v = lx.nvalue(1).filter(|v| v.len() == 1).ok_or(FAILED1)?;
                if v[0] != 1.0 {
                    return Err(FAILED1.into());
                }
            }
            "FontName" => {
                let mut n = lx.svalue().ok_or(FAILED1)?;
                if n.len() > 127 {
                    warn.push(format!(
                        "FontName too long: {} ({} bytes)",
                        String::from_utf8_lossy(&n),
                        n.len()
                    ));
                    n.truncate(127);
                }
                l.font_name = n;
            }
            "Encoding" => {
                l.encoding = Some(lx.encoding().ok_or(FAILED1)?);
            }
            "ItalicAngle" | "StrokeWidth" | "PaintType" => {
                let v = lx.nvalue(1).filter(|v| v.len() == 1).ok_or(FAILED1)?;
                if v[0] != 0.0 {
                    let k = operator_key(&key);
                    l.top.add(k, 1);
                    l.top.set(k, 0, v[0]);
                }
            }
            "FontBBox" => {
                let v = lx.nvalue(4).filter(|v| v.len() == 4).ok_or(FAILED1)?;
                l.top.add("FontBBox", 4);
                for (i, x) in v.iter().enumerate() {
                    l.top.set("FontBBox", i, *x);
                }
            }
            "FontMatrix" => {
                let v = lx.nvalue(6).filter(|v| v.len() == 6).ok_or(FAILED1)?;
                if v != [0.001, 0.0, 0.0, 0.001, 0.0, 0.0] {
                    l.top.add("FontMatrix", 6);
                    for (i, x) in v.iter().enumerate() {
                        l.top.set("FontMatrix", i, *x);
                    }
                }
            }
            k if SKEYS.contains(&k) => {
                let s = lx.svalue().ok_or(FAILED1)?;
                let k = operator_key(k);
                l.top.add(k, 1);
                if let Some(e) = l.strings.iter_mut().find(|(x, _)| *x == k) {
                    e.1 = s;
                } else {
                    l.strings.push((k, s));
                }
            }
            _ => {}
        }
    }
    if bin.is_empty() {
        return Err(FAILED2.into());
    }
    let plain = decrypt(&bin, 55665, 4);
    let mut lx = Lexer { s: &plain, i: 0 };
    let mut len_iv = 4;
    while lx.i < lx.s.len() {
        let Some(key) = lx.get_next_key() else {
            break;
        };
        let key = String::from_utf8_lossy(&key).into_owned();
        match key.as_str() {
            "Subrs" => lx.subrs(len_iv, &mut l.subrs, warn).ok_or(FAILED2)?,
            "CharStrings" => lx.charstrings(len_iv, &mut l.glyphs, warn).ok_or(FAILED2)?,
            "lenIV" => {
                len_iv = lx.nvalue(1).filter(|v| v.len() == 1).ok_or(FAILED2)?[0] as i32;
            }
            "BlueValues" | "OtherBlues" | "FamilyBlues" | "FamilyOtherBlues" | "StemSnapH"
            | "StemSnapV" => {
                let v = lx.nvalue(127).ok_or(FAILED2)?;
                let k = operator_key(&key);
                l.private.add(k, v.len());
                for i in 0..v.len() {
                    l.private
                        .set(k, i, if i == 0 { v[0] } else { v[i] - v[i - 1] });
                }
            }
            "StdHW" | "StdVW" | "BlueScale" | "BlueShift" | "BlueFuzz" | "LanguageGroup"
            | "ExpansionFactor" => {
                let v = lx.nvalue(1).filter(|v| v.len() == 1).ok_or(FAILED2)?;
                let k = operator_key(&key);
                l.private.add(k, 1);
                l.private.set(k, 0, v[0]);
            }
            "ForceBold" if lx.bvalue().ok_or(FAILED2)? => {
                l.private.add("ForceBold", 1);
                l.private.set("ForceBold", 0, 1.0);
            }
            _ => {}
        }
    }
    Ok(l)
}

/// The `&'static` name of a dict key.
fn operator_key(k: &str) -> &'static str {
    const KEYS: [&str; 22] = [
        "version",
        "Notice",
        "FullName",
        "FamilyName",
        "Weight",
        "Copyright",
        "ItalicAngle",
        "StrokeWidth",
        "PaintType",
        "BlueValues",
        "OtherBlues",
        "FamilyBlues",
        "FamilyOtherBlues",
        "StemSnapH",
        "StemSnapV",
        "StdHW",
        "StdVW",
        "BlueScale",
        "BlueShift",
        "BlueFuzz",
        "LanguageGroup",
        "ExpansionFactor",
    ];
    KEYS.iter().find(|x| **x == k).copied().unwrap_or("version")
}

// ---------------------------------------------------------------------------
// type1.c
// ---------------------------------------------------------------------------

/// What [`convert`] makes.
pub struct Converted {
    /// The CFF font (`/FontFile3 /Subtype /Type1C`).
    pub program: Vec<u8>,
    /// The descriptor's `/CharSet`: `/name` for each glyph, in the CFF's
    /// order, `.notdef` left out.
    pub charset: Vec<u8>,
    /// Codes the C drops (mapped to `.notdef`, or a glyph the font lacks).
    pub dropped: BTreeSet<u16>,
    /// `add_metrics`: each code kept, its glyph's width scaled by the
    /// FontMatrix to 1000 units per em (not rounded).
    pub widths: std::collections::BTreeMap<u16, f64>,
}

/// `pdf_font_load_type1` and `write_fontfile`: the program `l` (from
/// [`load`]) subset to the codes `used` of `enc` (code to glyph name),
/// named `fullname` (`TAG+FontName`).
pub fn convert(
    mut l: Loaded,
    fullname: &str,
    enc: &[Vec<u8>],
    used: &BTreeSet<u16>,
    warn: &mut Vec<String>,
) -> Result<Converted, Fatal> {
    // get_font_attr: `defaultWidthX` is the width of "space" (of ".notdef"
    // when the font has no "space", `cff_glyph_lookup` giving glyph 0)
    let default_width = if l.glyphs.is_empty() {
        500.0
    } else {
        l.metrics(l.lookup(b"space"), warn)?.sbw[2]
    };
    if default_width != 0.0 {
        l.private.add("defaultWidthX", 1);
        l.private.set("defaultWidthX", 0, default_width);
    }
    if l.glyphs.first().is_none_or(|g| g.0 != b".notdef") {
        return Err("Type 1 font with no \".notdef\" glyph???".into());
    }
    let has = |n: &str| l.lookup(n.as_bytes()) > 0;
    let mut strings = Strings::default();
    // GIDMap: the glyph names, `.notdef` first
    let mut glyphs: Vec<String> = vec![".notdef".into()];
    let mut charset_sids: Vec<u16> = Vec::new();
    let mut ranges: Vec<(u8, u8)> = Vec::new();
    let mut supps: Vec<(u8, u16)> = Vec::new();
    let mut pdfcharset = Vec::new();
    let mut dropped = BTreeSet::new();
    let mut prev: i32 = -2;
    let mut kept: Vec<(u16, &[u8])> = Vec::new();
    for &code in used.iter().filter(|&&c| c <= 0xff) {
        let name = enc
            .get(code as usize)
            .map(Vec::as_slice)
            .unwrap_or(b".notdef");
        if name == b".notdef" {
            warn.push(format!(
                "Character mapped to .notdef used in font: {fullname}"
            ));
            dropped.insert(code);
            continue;
        }
        let gname = String::from_utf8_lossy(name).into_owned();
        if l.lookup(name) == 0 {
            warn.push(format!("Glyph \"{gname}\" missing in font \"{fullname}\"."));
            dropped.insert(code);
            continue;
        }
        let duplicate = kept.iter().find(|(_, n)| *n == name).map(|(c, _)| *c);
        let sid = strings.add(name, true);
        kept.push((code, name));
        if let Some(d) = duplicate {
            supps.push((d as u8, sid));
        } else {
            glyphs.push(gname.clone());
            charset_sids.push(sid);
            if code as i32 != prev + 1 {
                ranges.push((code as u8, 0));
            } else if let Some(r) = ranges.last_mut() {
                r.1 += 1;
            }
            prev = code as i32;
            pdfcharset.push(b'/');
            pdfcharset.extend_from_slice(name);
        }
    }
    // the charstrings, `seac` components appended as they come
    let nominal = 0.0;
    let subrs = l.subrs.as_deref().unwrap_or(&[]);
    let subr = |i: usize| subrs.get(i).cloned();
    let mut cstrings: Vec<Vec<u8>> = Vec::new();
    let mut wx: Vec<f64> = Vec::new();
    let mut gid = 0;
    while gid < glyphs.len() {
        let cs = &l.glyphs[l.lookup(glyphs[gid].as_bytes())].1;
        let (t2, gm) = t1char::convert(cs, &subr, subrs.len(), default_width, nominal, warn)?;
        cstrings.push(t2);
        wx.push(gm.sbw[2]);
        if let Some(s) = gm.seac {
            for code in [s.achar, s.bchar] {
                let name = standard_encoding_name(code as i32);
                // `cff_glyph_lookup` gives glyph 0 (.notdef) for a name the
                // font lacks
                let found = match name {
                    Some(n) if has(n) => n,
                    _ => ".notdef",
                };
                if !glyphs.iter().any(|g| g == found) {
                    glyphs.push(found.to_string());
                    let sid = STANDARD_STRING_NAMES
                        .iter()
                        .position(|x| *x == found)
                        .map_or(u16::MAX, |i| i as u16);
                    charset_sids.push(sid);
                    pdfcharset.push(b'/');
                    pdfcharset.extend_from_slice(found.as_bytes());
                }
            }
        }
        gid += 1;
    }
    // cff_dict_update: the FontInfo strings after the glyph names
    let mut sid_of = Vec::new();
    for (k, _) in &l.top.0 {
        if let Some((_, s)) = l.strings.iter().find(|(x, _)| x == k) {
            sid_of.push((*k, strings.add(s, true)));
        }
    }
    for (k, sid) in sid_of {
        l.top.set(k, 0, sid as f64);
    }
    // write_fontfile
    l.top.add("CharStrings", 1);
    l.top.add("charset", 1);
    l.top.add("Encoding", 1);
    let mut private = Vec::new();
    l.private.pack(&mut private);
    l.top.add("Private", 2);
    let mut out = vec![1, 0, 4, 4];
    pack_index(&[fullname.as_bytes().to_vec()], &mut out);
    let mut probe = Vec::new();
    l.top.pack(&mut probe);
    let topdict_at = out.len();
    let topdict_size = 2 + 1 + 2 * if probe.len() < 0xff { 1 } else { 2 } + probe.len();
    out.resize(out.len() + topdict_size, 0);
    let strs: Vec<Vec<u8>> = strings.0.clone();
    pack_index(&strs, &mut out);
    pack_index(&[], &mut out);
    // Encoding: format 1 (with supplements: | 0x80)
    l.top.set("Encoding", 0, out.len() as f64);
    out.push(if supps.is_empty() { 1 } else { 0x81 });
    out.push(ranges.len() as u8);
    for (first, n_left) in &ranges {
        out.extend_from_slice(&[*first, *n_left]);
    }
    if !supps.is_empty() {
        out.push(supps.len() as u8);
        for (code, sid) in &supps {
            out.push(*code);
            out.extend_from_slice(&sid.to_be_bytes());
        }
    }
    // charset: format 0
    l.top.set("charset", 0, out.len() as f64);
    out.push(0);
    for sid in &charset_sids {
        out.extend_from_slice(&sid.to_be_bytes());
    }
    l.top.set("CharStrings", 0, out.len() as f64);
    pack_index(&cstrings, &mut out);
    if !private.is_empty() {
        l.top.set("Private", 1, out.len() as f64);
        l.top.set("Private", 0, private.len() as f64);
        out.extend_from_slice(&private);
    }
    let mut top = Vec::new();
    l.top.pack(&mut top);
    let mut idx = Vec::new();
    pack_index(&[top], &mut idx);
    out[topdict_at..topdict_at + idx.len()].copy_from_slice(&idx);
    let scaling = l.top.get("FontMatrix", 0).map_or(1.0, |m| 1000.0 * m);
    let widths = kept
        .iter()
        .map(|(code, name)| {
            let g = glyphs
                .iter()
                .position(|g| g.as_bytes() == *name)
                .unwrap_or(0);
            (*code, scaling * wx.get(g).copied().unwrap_or(0.0))
        })
        .collect();
    Ok(Converted {
        program: out,
        charset: pdfcharset,
        dropped,
        widths,
    })
}

/// t1_load.c's `ISOLatin1Encoding`.
const ISO_LATIN1: [Option<&str>; 256] = [
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some("space"),
    Some("exclam"),
    Some("quotedbl"),
    Some("numbersign"),
    Some("dollar"),
    Some("percent"),
    Some("ampersand"),
    Some("quotesingle"),
    Some("parenleft"),
    Some("parenright"),
    Some("asterisk"),
    Some("plus"),
    Some("comma"),
    Some("hyphen"),
    Some("period"),
    Some("slash"),
    Some("zero"),
    Some("one"),
    Some("two"),
    Some("three"),
    Some("four"),
    Some("five"),
    Some("six"),
    Some("seven"),
    Some("eight"),
    Some("nine"),
    Some("colon"),
    Some("semicolon"),
    Some("less"),
    Some("equal"),
    Some("greater"),
    Some("question"),
    Some("at"),
    Some("A"),
    Some("B"),
    Some("C"),
    Some("D"),
    Some("E"),
    Some("F"),
    Some("G"),
    Some("H"),
    Some("I"),
    Some("J"),
    Some("K"),
    Some("L"),
    Some("M"),
    Some("N"),
    Some("O"),
    Some("P"),
    Some("Q"),
    Some("R"),
    Some("S"),
    Some("T"),
    Some("U"),
    Some("V"),
    Some("W"),
    Some("X"),
    Some("Y"),
    Some("Z"),
    Some("bracketleft"),
    Some("backslash"),
    Some("bracketright"),
    Some("asciicircum"),
    Some("underscore"),
    Some("grave"),
    Some("a"),
    Some("b"),
    Some("c"),
    Some("d"),
    Some("e"),
    Some("f"),
    Some("g"),
    Some("h"),
    Some("i"),
    Some("j"),
    Some("k"),
    Some("l"),
    Some("m"),
    Some("n"),
    Some("o"),
    Some("p"),
    Some("q"),
    Some("r"),
    Some("s"),
    Some("t"),
    Some("u"),
    Some("v"),
    Some("w"),
    Some("x"),
    Some("y"),
    Some("z"),
    Some("braceleft"),
    Some("bar"),
    Some("braceright"),
    Some("asciitilde"),
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    None,
    Some("dotlessi"),
    Some("quoteleft"),
    Some("quoteright"),
    Some("circumflex"),
    Some("tilde"),
    Some("macron"),
    Some("breve"),
    Some("dotaccent"),
    Some("dieresis"),
    None,
    Some("ring"),
    Some("cedilla"),
    None,
    Some("hungarumlaut"),
    Some("ogonek"),
    Some("caron"),
    Some("space"),
    Some("exclamdown"),
    Some("cent"),
    Some("sterling"),
    Some("currency"),
    Some("yen"),
    Some("brokenbar"),
    Some("section"),
    Some("dieresis"),
    Some("copyright"),
    Some("ordfeminine"),
    Some("guillemotleft"),
    Some("logicalnot"),
    Some("hyphen"),
    Some("registered"),
    Some("macron"),
    Some("degree"),
    Some("plusminus"),
    Some("twosuperior"),
    Some("threesuperior"),
    Some("acute"),
    Some("mu"),
    Some("paragraph"),
    Some("periodcentered"),
    Some("cedilla"),
    Some("onesuperior"),
    Some("ordmasculine"),
    Some("guillemotright"),
    Some("onequarter"),
    Some("onehalf"),
    Some("threequarters"),
    Some("questiondown"),
    Some("Agrave"),
    Some("Aacute"),
    Some("Acircumflex"),
    Some("Atilde"),
    Some("Adieresis"),
    Some("Aring"),
    Some("AE"),
    Some("Ccedilla"),
    Some("Egrave"),
    Some("Eacute"),
    Some("Ecircumflex"),
    Some("Edieresis"),
    Some("Igrave"),
    Some("Iacute"),
    Some("Icircumflex"),
    Some("Idieresis"),
    Some("Eth"),
    Some("Ntilde"),
    Some("Ograve"),
    Some("Oacute"),
    Some("Ocircumflex"),
    Some("Otilde"),
    Some("Odieresis"),
    Some("multiply"),
    Some("Oslash"),
    Some("Ugrave"),
    Some("Uacute"),
    Some("Ucircumflex"),
    Some("Udieresis"),
    Some("Yacute"),
    Some("Thorn"),
    Some("germandbls"),
    Some("agrave"),
    Some("aacute"),
    Some("acircumflex"),
    Some("atilde"),
    Some("adieresis"),
    Some("aring"),
    Some("ae"),
    Some("ccedilla"),
    Some("egrave"),
    Some("eacute"),
    Some("ecircumflex"),
    Some("edieresis"),
    Some("igrave"),
    Some("iacute"),
    Some("icircumflex"),
    Some("idieresis"),
    Some("eth"),
    Some("ntilde"),
    Some("ograve"),
    Some("oacute"),
    Some("ocircumflex"),
    Some("otilde"),
    Some("odieresis"),
    Some("divide"),
    Some("oslash"),
    Some("ugrave"),
    Some("uacute"),
    Some("ucircumflex"),
    Some("udieresis"),
    Some("yacute"),
    Some("thorn"),
    Some("ydieresis"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reals_as_cff_packs_them() {
        let r = |v: f64| {
            let mut o = Vec::new();
            pack_real(v, &mut o);
            o
        };
        assert_eq!(r(0.001), [30, 0x0a, 0x00, 0x1f]);
        assert_eq!(r(-0.5), [30, 0xe0, 0xa5, 0xff]);
        assert_eq!(r(0.039625), [30, 0x0a, 0x03, 0x96, 0x25, 0xff]);
        assert_eq!(fmt_g13(1e-5), "1e-05");
        assert_eq!(fmt_g13(0.0001), "0.0001");
        assert_eq!(fmt_g13(123456.5), "123456.5");
    }

    #[test]
    fn integers_and_offsets() {
        let mut o = Vec::new();
        put_number(500.0, false, &mut o);
        put_number(-250.0, false, &mut o);
        put_number(0.0, false, &mut o);
        put_number(42.0, true, &mut o);
        assert_eq!(o, [0xf8, 0x88, 0xfb, 0x8e, 139, 29, 0, 0, 0, 42]);
    }
}
