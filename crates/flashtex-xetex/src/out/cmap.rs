//! PostScript CMap files as dvipdfmx's `pdf:tounicode` special reads and
//! applies them (dvipdfm-x 20260317: cmap.c, cmap_read.c, pst.c,
//! pst_obj.c, and spc_pdfm.c's `reencode_string`), and the special's state:
//! the CMap in effect and the extra "taint keys" (spc_pdfm.c's
//! `struct tounicode`).
//!
//! With a CMap in effect, every string under a taint key that does not
//! start with the UTF-16BE byte order mark, ASCII or not, is decoded
//! through the CMap and written as `FE FF` plus the CMap's output bytes.
//! Without one (an XDV file), the strings are read as UTF-8
//! (`super::special::text_strings`).

use std::sync::Arc;

const MAP_IS_CID: u8 = 1;
const MAP_IS_NAME: u8 = 2;
const MAP_IS_CODE: u8 = 4;
const MAP_IS_NOTDEF: u8 = 8;
const MAP_TYPE_MASK: u8 = 0x0f;
const MAP_LOOKUP_CONTINUE: u8 = 16;

const CMAP_TYPE_IDENTITY: i32 = 0;
const CMAP_TYPE_CODE_TO_CID: i32 = 1;
const CMAP_TYPE_TO_UNICODE: i32 = 2;
const CMAP_TYPE_CID_TO_CODE: i32 = 3;
const CID_MAX: i32 = 65535;
const TOKEN_LEN_MAX: usize = 127;

fn defined(f: u8) -> bool {
    f & MAP_TYPE_MASK != 0
}

/// cmap_p.h's `mapDef`: one byte's entry of a 256-entry table.
#[derive(Clone, Default, Debug)]
struct Entry {
    flag: u8,
    code: Vec<u8>,
    next: Option<Box<Table>>,
}

#[derive(Clone, Debug)]
struct Table(Vec<Entry>);

impl Table {
    fn new() -> Box<Table> {
        Box::new(Table(vec![Entry::default(); 256]))
    }
}

/// A CMap (cmap.c's `struct CMap`).
#[derive(Clone, Debug)]
pub struct CMap {
    name: Option<Vec<u8>>,
    ty: i32,
    /// (registry, ordering)
    csi: Option<(Vec<u8>, Vec<u8>)>,
    use_cmap: Option<Arc<CMap>>,
    /// (lo, hi), each `dim` bytes long.
    codespace: Vec<(Vec<u8>, Vec<u8>)>,
    map: Option<Box<Table>>,
    min_bytes_in: usize,
    max_bytes_in: usize,
}

/// What a fatal dvipdfmx `ERROR` says: the run fails.
pub type Fatal = String;

impl CMap {
    fn new() -> CMap {
        CMap {
            name: None,
            ty: CMAP_TYPE_CODE_TO_CID,
            csi: None,
            use_cmap: None,
            codespace: Vec::new(),
            map: None,
            min_bytes_in: 65535,
            max_bytes_in: 0,
        }
    }

    fn identity(name: &[u8]) -> CMap {
        let mut c = CMap::new();
        c.name = Some(name.to_vec());
        c.ty = CMAP_TYPE_IDENTITY;
        c.csi = Some((b"Adobe".to_vec(), b"Identity".to_vec()));
        c.add_codespacerange(&[0, 0], &[0xff, 0xff], &mut Vec::new());
        c
    }

    /// `CMap_is_valid`.
    fn is_valid(&self, warn: &mut Vec<String>) -> bool {
        if self.name.is_none()
            || !(CMAP_TYPE_IDENTITY..=CMAP_TYPE_CID_TO_CODE).contains(&self.ty)
            || self.codespace.is_empty()
            || (self.ty != CMAP_TYPE_IDENTITY && self.map.is_none())
        {
            return false;
        }
        if let Some(u) = &self.use_cmap {
            if self.csi != u.csi {
                warn.push("CIDSystemInfo mismatched".into());
                return false;
            }
        }
        true
    }

    fn match_codespace(&self, c: &[u8]) -> bool {
        self.codespace.iter().any(|(lo, hi)| {
            lo.len() == c.len() && (0..c.len()).all(|p| c[p] >= lo[p] && c[p] <= hi[p])
        })
    }

    /// `CMap_add_codespacerange`.
    fn add_codespacerange(&mut self, lo: &[u8], hi: &[u8], warn: &mut Vec<String>) {
        let dim = lo.len();
        for (clo, chi) in &self.codespace {
            let mut overlap = true;
            let mut j = 0;
            while j < clo.len().min(dim) && overlap {
                overlap = (lo[j] >= clo[j] && lo[j] <= chi[j]) || (hi[j] >= clo[j] && hi[j] <= chi[j]);
                j += 1;
            }
            if overlap {
                warn.push("Overlapping codespace found. (ingored)".into());
                return;
            }
        }
        self.min_bytes_in = self.min_bytes_in.min(dim);
        self.max_bytes_in = self.max_bytes_in.max(dim);
        self.codespace.push((lo.to_vec(), hi.to_vec()));
    }

    /// `check_range`.
    fn check_range(&mut self, lo: &[u8], hi: &[u8], dstdim: usize, warn: &mut Vec<String>) -> bool {
        let n = lo.len();
        if n < 1 || dstdim < 1 || lo[..n - 1] != hi[..n - 1] || lo[n - 1] > hi[n - 1] {
            warn.push("Invalid CMap mapping entry. (ignored)".into());
            return false;
        }
        if !self.match_codespace(lo) || !self.match_codespace(hi) {
            warn.push("Invalid CMap mapping entry. (ignored)".into());
            return false;
        }
        self.min_bytes_in = self.min_bytes_in.min(n);
        self.max_bytes_in = self.max_bytes_in.max(n);
        true
    }

    /// `locate_tbl`: the table holding `code`'s last byte.
    fn locate<'a>(map: &'a mut Box<Table>, code: &[u8], warn: &mut Vec<String>) -> Option<&'a mut Table> {
        let mut cur: &mut Table = map;
        for &c in &code[..code.len() - 1] {
            let e = &mut cur.0[c as usize];
            if defined(e.flag) {
                warn.push("Ambiguous CMap entry.".into());
                return None;
            }
            e.flag |= MAP_LOOKUP_CONTINUE;
            cur = e.next.get_or_insert_with(Table::new);
        }
        Some(cur)
    }

    fn add_notdefrange(&mut self, lo: &[u8], hi: &[u8], dst: i32, warn: &mut Vec<String>) {
        if !self.check_range(lo, hi, 2, warn) {
            return;
        }
        let map = self.map.get_or_insert_with(Table::new);
        let Some(cur) = Self::locate(map, lo, warn) else {
            return;
        };
        let n = lo.len();
        for c in lo[n - 1]..=hi[n - 1] {
            let e = &mut cur.0[c as usize];
            if defined(e.flag) {
                warn.push("Trying to redefine already defined code mapping. (ignored)".into());
            } else {
                e.flag = MAP_IS_NOTDEF;
                e.code = vec![(dst >> 8) as u8, dst as u8];
            }
        }
    }

    /// `CMap_add_bfrange`.
    fn add_bfrange(&mut self, lo: &[u8], hi: &[u8], base: &[u8], warn: &mut Vec<String>) {
        let dstdim = base.len();
        if !self.check_range(lo, hi, dstdim, warn) {
            return;
        }
        let map = self.map.get_or_insert_with(Table::new);
        let Some(cur) = Self::locate(map, lo, warn) else {
            return;
        };
        let n = lo.len();
        for c in lo[n - 1] as i32..=hi[n - 1] as i32 {
            let e = &mut cur.0[c as usize];
            if !defined(e.flag) || e.code.len() < dstdim {
                e.flag = MAP_IS_CODE;
            }
            let mut code = base.to_vec();
            let mut last = c - lo[n - 1] as i32 + base[dstdim - 1] as i32;
            code[dstdim - 1] = last as u8;
            let mut i = dstdim as i32 - 2;
            while i >= 0 && last > 255 {
                last = code[i as usize] as i32 + 1;
                code[i as usize] = last as u8;
                i -= 1;
            }
            e.code = code;
        }
    }

    /// `CMap_add_cidrange`.
    fn add_cidrange(&mut self, lo: &[u8], hi: &[u8], mut base: i32, warn: &mut Vec<String>) {
        if !self.check_range(lo, hi, 2, warn) {
            return;
        }
        let map = self.map.get_or_insert_with(Table::new);
        let Some(cur) = Self::locate(map, lo, warn) else {
            return;
        };
        let n = lo.len();
        for c in lo[n - 1]..=hi[n - 1] {
            let e = &mut cur.0[c as usize];
            if e.flag != 0 {
                warn.push("Trying to redefine already defined CID mapping. (ignored)".into());
            } else {
                e.flag = MAP_IS_CID;
                e.code = vec![(base >> 8) as u8, base as u8];
            }
            base += 1;
        }
    }

    /// `bytes_consumed`: as the C, the minimum input length unless a
    /// codespace range matches in full (the C's loop always ends at
    /// `codespace.num`).
    fn bytes_consumed(&self, s: &[u8], inbytes: i64) -> i64 {
        for (lo, hi) in &self.codespace {
            let dim = lo.len();
            let mut pos = 0;
            while (pos as i64) < (dim as i64).min(inbytes) {
                let c = s.get(pos).copied().unwrap_or(0);
                if c > hi[pos] || c < lo[pos] {
                    break;
                }
                pos += 1;
            }
            if pos == dim {
                return dim as i64;
            }
        }
        self.min_bytes_in as i64
    }

    fn handle_undefined(&self, d: &mut Decode) -> Result<(), Fatal> {
        if d.outleft < 2 {
            return Err("CMap: Buffer overflow.".into());
        }
        match self.ty {
            CMAP_TYPE_CODE_TO_CID => d.out.extend_from_slice(&[0, 0]),
            CMAP_TYPE_TO_UNICODE => d.out.extend_from_slice(&[0xff, 0xfd]),
            _ => {
                d.warn.push(format!(
                    "Cannot handle undefined mapping for this type of CMap mapping: {}",
                    self.ty
                ));
                d.out.extend_from_slice(&[0, 0]);
            }
        }
        d.outleft -= 2;
        let len = self.bytes_consumed(d.input.get(d.pos..).unwrap_or(&[]), d.left);
        d.pos += len as usize;
        d.left -= len;
        Ok(())
    }

    /// `CMap_decode_char`.
    fn decode_char(&self, d: &mut Decode) -> Result<(), Fatal> {
        if self.ty == CMAP_TYPE_IDENTITY {
            if d.left % 2 != 0 {
                return Err("CMap: Invalid/truncated input string.".into());
            }
            if d.outleft < 2 {
                return Err("CMap: Buffer overflow.".into());
            }
            d.out.extend_from_slice(&d.input[d.pos..d.pos + 2]);
            d.pos += 2;
            d.outleft -= 2;
            d.left -= 2;
            return Ok(());
        }
        let Some(root) = &self.map else {
            if let Some(u) = &self.use_cmap {
                return u.decode_char(d);
            }
            d.warn.push("No mapping available for this character.".into());
            return self.handle_undefined(d);
        };
        let mut t: &Table = root;
        let mut p = d.pos;
        let mut count = 0i64;
        let mut c = 0usize;
        while count < d.left {
            c = d.input[p] as usize;
            p += 1;
            count += 1;
            if t.0[c].flag & MAP_LOOKUP_CONTINUE == 0 {
                break;
            }
            t = t.0[c].next.as_deref().expect("continued entry has a table");
        }
        // As the C: when the input ends inside a code, `t` is the next
        // table and `c` the last byte read.
        let e = &t.0[c];
        if e.flag & MAP_LOOKUP_CONTINUE != 0 {
            return Err("CMap: Premature end of input string.".into());
        }
        if !defined(e.flag) {
            if let Some(u) = &self.use_cmap {
                return u.decode_char(d);
            }
            d.warn.push("No character mapping available.".into());
            return self.handle_undefined(d);
        }
        match e.flag & MAP_TYPE_MASK {
            MAP_IS_NOTDEF | MAP_IS_CID | MAP_IS_CODE => {
                if e.flag & MAP_TYPE_MASK == MAP_IS_NOTDEF {
                    d.warn.push("Character mapped to .notdef found.".into());
                }
                if d.outleft < e.code.len() as i64 {
                    return Err("CMap: Buffer overflow.".into());
                }
                d.out.extend_from_slice(&e.code);
                d.outleft -= e.code.len() as i64;
            }
            MAP_IS_NAME => return Err("CMap: CharName mapping not supported.".into()),
            _ => return Err("CMap: Unknown mapping type.".into()),
        }
        d.left -= count;
        d.pos = p;
        Ok(())
    }
}

struct Decode<'a> {
    input: &'a [u8],
    pos: usize,
    left: i64,
    out: Vec<u8>,
    outleft: i64,
    warn: &'a mut Vec<String>,
}

/// spc_pdfm.c's `reencode_string` with a CMap: `Ok(Some(new string))`, or
/// `Ok(None)` when input is left over (the C's error: the string stays and
/// the walk stops).
pub fn reencode(cmap: &CMap, s: &[u8], warn: &mut Vec<String>) -> Result<Option<Vec<u8>>, Fatal> {
    let size = s.len() as i64 * 4 + 2;
    let mut d = Decode {
        input: s,
        pos: 0,
        left: s.len() as i64,
        out: vec![0xfe, 0xff],
        outleft: size - 2,
        warn,
    };
    while d.left > 0 && d.outleft > 0 {
        cmap.decode_char(&mut d)?;
    }
    Ok((d.left <= 0).then_some(d.out))
}

// ---------------------------------------------------------------------------
// pst.c's tokenizer, as cmap_read.c uses it
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Tok {
    Null,
    Bool(bool),
    Int(i32),
    Real,
    Str(Vec<u8>),
    Name(Vec<u8>),
    Mark,
    Unknown(Vec<u8>),
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | 0x0c | b'\r' | b'\n' | 0)
}

fn is_delim(c: u8) -> bool {
    matches!(c, b'(' | b')' | b'/' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'%')
}

fn xtoi(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|v| v as u8)
}

struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

impl Lexer<'_> {
    fn at(&self, i: usize) -> u8 {
        self.s.get(i).copied().unwrap_or(0)
    }

    fn token_end(&self, i: usize) -> bool {
        i >= self.s.len() || is_delim(self.s[i]) || is_space(self.s[i])
    }

    fn skip_white(&mut self) {
        while self.i < self.s.len() && is_space(self.s[self.i]) {
            self.i += 1;
        }
    }

    /// `pst_get_token`.
    fn token(&mut self) -> Result<Option<Tok>, Fatal> {
        self.skip_white();
        while self.i < self.s.len() && self.s[self.i] == b'%' {
            while self.i < self.s.len() && !matches!(self.s[self.i], b'\n' | b'\r') {
                self.i += 1;
            }
            if self.at(self.i) == b'\r' && self.i < self.s.len() {
                self.i += 1;
            }
            if self.at(self.i) == b'\n' && self.i < self.s.len() {
                self.i += 1;
            }
            self.skip_white();
        }
        if self.i >= self.s.len() {
            return Ok(None);
        }
        let c = self.s[self.i];
        let tok = match c {
            b'/' => Some(self.name()),
            b'[' | b'{' => {
                self.i += 1;
                Some(Tok::Mark)
            }
            b'<' => {
                if self.i + 1 >= self.s.len() {
                    return Ok(None);
                }
                let c2 = self.s[self.i + 1];
                if c2 == b'<' {
                    self.i += 2;
                    Some(Tok::Mark)
                } else if c2 == b'~' {
                    return Err("ASCII85 string not supported yet.".into());
                } else if xtoi(c2).is_some() {
                    self.hex()
                } else {
                    None
                }
            }
            b'(' => self.literal(),
            b'>' => {
                if self.at(self.i + 1) != b'>' || self.i + 1 >= self.s.len() {
                    return Err("Unexpected end of ASCII hex string marker.".into());
                }
                self.i += 2;
                Some(Tok::Unknown(b">>".to_vec()))
            }
            b']' | b'}' => {
                self.i += 1;
                Some(Tok::Unknown(vec![c]))
            }
            b't' | b'f' => {
                if self.s[self.i..].starts_with(b"true") && self.token_end(self.i + 4) {
                    self.i += 4;
                    Some(Tok::Bool(true))
                } else if self.s[self.i..].starts_with(b"false") && self.token_end(self.i + 5) {
                    self.i += 5;
                    Some(Tok::Bool(false))
                } else {
                    None
                }
            }
            b'n' => {
                if self.s[self.i..].starts_with(b"null") && self.token_end(self.i + 4) {
                    self.i += 4;
                    Some(Tok::Null)
                } else {
                    None
                }
            }
            b'+' | b'-' | b'0'..=b'9' | b'.' => self.number(),
            _ => None,
        };
        if let Some(t) = tok {
            return Ok(Some(t));
        }
        // `pst_parse_any`
        let st = self.i;
        while !self.token_end(self.i) {
            self.i += 1;
        }
        if self.i == st {
            // the C loops for ever here
            return Ok(None);
        }
        Ok(Some(Tok::Unknown(self.s[st..self.i].to_vec())))
    }

    fn name(&mut self) -> Tok {
        self.i += 1;
        let mut out = Vec::new();
        while !self.token_end(self.i) {
            let mut c = self.s[self.i];
            self.i += 1;
            if c == b'#' {
                if self.i + 2 >= self.s.len() {
                    break;
                }
                let hi = xtoi(self.at(self.i));
                let v = hi.and_then(|h| {
                    self.i += 1;
                    let lo = xtoi(self.at(self.i));
                    lo.map(|l| {
                        self.i += 1;
                        h << 4 | l
                    })
                });
                match v {
                    Some(v) if v > 0 => c = v,
                    _ => continue,
                }
            }
            if out.len() < TOKEN_LEN_MAX {
                out.push(c);
            }
        }
        Tok::Name(out)
    }

    /// `pst_parse_number`: an integer (`strtol`), a radix integer, or a
    /// real (whose value cmap_read.c never uses).
    fn number(&mut self) -> Option<Tok> {
        let s = self.s;
        let st = self.i;
        let mut j = st;
        if matches!(self.at(j), b'+' | b'-') {
            j += 1;
        }
        let ds = j;
        while self.at(j).is_ascii_digit() && j < s.len() {
            j += 1;
        }
        let digits = j > ds;
        let realish = matches!(self.at(j), b'.' | b'e' | b'E') && j < s.len();
        if realish || !digits {
            // strtod
            let mut k = ds;
            let mut any = false;
            while k < s.len() && s[k].is_ascii_digit() {
                k += 1;
                any = true;
            }
            if self.at(k) == b'.' && k < s.len() {
                k += 1;
                while k < s.len() && s[k].is_ascii_digit() {
                    k += 1;
                    any = true;
                }
            }
            if any && matches!(self.at(k), b'e' | b'E') && k < s.len() {
                let mut m = k + 1;
                if matches!(self.at(m), b'+' | b'-') {
                    m += 1;
                }
                if self.at(m).is_ascii_digit() && m < s.len() {
                    while m < s.len() && s[m].is_ascii_digit() {
                        m += 1;
                    }
                    k = m;
                }
            }
            if any && (realish || digits) && self.token_end(k) {
                self.i = k;
                return Some(Tok::Real);
            }
            return None;
        }
        let text = std::str::from_utf8(&s[st..j]).ok()?;
        let v: i64 = text.parse().ok()?;
        let overflow = v > i32::MAX as i64 || v < i32::MIN as i64;
        if overflow {
            // strtol's ERANGE: read as a real
            if self.token_end(j) {
                self.i = j;
                return Some(Tok::Real);
            }
            return None;
        }
        if self.token_end(j) {
            self.i = j;
            return Some(Tok::Int(v as i32));
        }
        if (2..=36).contains(&v) && self.at(j) == b'#' && self.at(j + 1).is_ascii_alphanumeric() {
            let radix = v as u32;
            if radix == 16 && matches!(self.at(j + 2), b'x' | b'X') {
                return None;
            }
            let mut k = j + 1;
            let mut val: i64 = 0;
            while k < s.len() {
                match (s[k] as char).to_digit(radix) {
                    Some(d) => {
                        val = val * radix as i64 + d as i64;
                        k += 1;
                    }
                    None => break,
                }
            }
            if k > j + 1 && val <= i32::MAX as i64 && self.token_end(k) {
                self.i = k;
                return Some(Tok::Int(val as i32));
            }
        }
        None
    }

    /// `pst_string_parse_literal`.
    fn literal(&mut self) -> Option<Tok> {
        let s = self.s;
        if self.i + 2 >= s.len() {
            return None;
        }
        let mut cur = self.i + 1;
        let mut out = Vec::new();
        let mut balance = 1;
        let mut c = 0u8;
        while cur < s.len() && out.len() < 32767 && balance > 0 {
            c = s[cur];
            cur += 1;
            match c {
                b'\\' => {
                    let e = self.at(cur);
                    match e {
                        b'\\' | b')' | b'(' => {
                            out.push(e);
                            cur += 1;
                        }
                        b'n' | b'r' | b't' | b'b' | b'f' => {
                            out.push(match e {
                                b'n' => b'\n',
                                b'r' => b'\r',
                                b't' => b'\t',
                                b'b' => 8,
                                _ => 12,
                            });
                            cur += 1;
                        }
                        b'\r' => {
                            cur += if cur + 1 < s.len() && s[cur + 1] == b'\n' { 2 } else { 1 };
                        }
                        b'\n' => cur += 1,
                        _ => {
                            let st = cur;
                            let mut v = 0u32;
                            while cur < s.len() && cur < st + 3 && (b'0'..=b'7').contains(&s[cur]) {
                                v = v << 3 | (s[cur] - b'0') as u32;
                                cur += 1;
                            }
                            if v <= 255 && cur != st {
                                out.push(v as u8);
                            }
                        }
                    }
                }
                b'(' => {
                    balance += 1;
                    out.push(c);
                }
                b')' => {
                    balance -= 1;
                    if balance > 0 {
                        out.push(c);
                    }
                }
                b'\r' => {
                    if cur < s.len() && s[cur] == b'\n' {
                        cur += 1;
                    }
                    out.push(b'\n');
                }
                _ => out.push(c),
            }
        }
        if c != b')' {
            return None;
        }
        self.i = cur;
        Some(Tok::Str(out))
    }

    /// `pst_string_parse_hex`.
    fn hex(&mut self) -> Option<Tok> {
        let s = self.s;
        if self.i + 2 >= s.len() {
            return None;
        }
        let mut cur = self.i + 1;
        let mut out = Vec::new();
        let white = |cur: &mut usize| {
            while *cur < s.len() && is_space(s[*cur]) {
                *cur += 1;
            }
        };
        while cur < s.len() && out.len() < 32767 {
            white(&mut cur);
            if self.at(cur) == b'>' {
                break;
            }
            let hi = xtoi(self.at(cur)).unwrap_or(0);
            cur += 1;
            white(&mut cur);
            if self.at(cur) == b'>' {
                break;
            }
            let lo = if cur < s.len() {
                let v = xtoi(s[cur]).unwrap_or(0);
                cur += 1;
                v
            } else {
                0
            };
            out.push(hi << 4 | lo);
        }
        if self.at(cur) != b'>' {
            return None;
        }
        self.i = cur + 1;
        Some(Tok::Str(out))
    }
}

/// `pst_getSV` as `check_next_token` compares it.
fn sv(t: &Tok) -> Result<Vec<u8>, Fatal> {
    Ok(match t {
        Tok::Name(v) | Tok::Str(v) | Tok::Unknown(v) => v.clone(),
        Tok::Int(i) => i.to_string().into_bytes(),
        Tok::Real => b"0".to_vec(),
        Tok::Bool(b) => if *b { b"true".to_vec() } else { b"false".to_vec() },
        Tok::Null => b"null".to_vec(),
        Tok::Mark => return Err("typecheck: object not of type".into()),
    })
}

// ---------------------------------------------------------------------------
// cmap_read.c
// ---------------------------------------------------------------------------

/// `CMap_parse_check_sig`.
fn check_sig(data: &[u8]) -> bool {
    if data.len() < 64 || !data.starts_with(b"%!PS") {
        return false;
    }
    let sig = &data[4..64];
    let sig = &sig[..sig.iter().position(|&c| c == 0).unwrap_or(sig.len())];
    sig.windows(13).any(|w| w == b"Resource-CMap")
}

struct Reader<'a, 'c> {
    lx: Lexer<'a>,
    cmap: CMap,
    cache: &'c mut Cache,
    warn: Vec<String>,
}

type Status = Result<bool, Fatal>;

impl Reader<'_, '_> {
    fn tok(&mut self) -> Result<Option<Tok>, Fatal> {
        self.lx.token()
    }

    /// `check_next_token`: `Ok(true)` when the next token is `key`.
    fn next_is(&mut self, key: &[u8]) -> Status {
        if self.lx.i >= self.lx.s.len() {
            return Ok(false);
        }
        match self.tok()? {
            None => Ok(false),
            Some(t) => Ok(sv(&t)? == key),
        }
    }

    fn coderange(&mut self) -> Result<Option<(Vec<u8>, Vec<u8>)>, Fatal> {
        let Some(t1) = self.tok()? else {
            return Ok(None);
        };
        let Some(t2) = self.tok()? else {
            return Ok(None);
        };
        match (t1, t2) {
            (Tok::Str(a), Tok::Str(b)) if a.len() == b.len() && a.len() <= TOKEN_LEN_MAX => {
                Ok(Some((a, b)))
            }
            _ => Ok(None),
        }
    }

    fn codespacerange(&mut self, mut count: i32) -> Status {
        while count > 0 {
            count -= 1;
            let Some((lo, hi)) = self.coderange()? else {
                return Ok(false);
            };
            if lo.is_empty() {
                continue;
            }
            self.cmap.add_codespacerange(&lo, &hi, &mut self.warn);
        }
        self.next_is(b"endcodespacerange")
    }

    fn range(&mut self, mut count: i32, kind: &[u8]) -> Status {
        while count > 0 {
            count -= 1;
            let Some((lo, hi)) = self.coderange()? else {
                return Ok(false);
            };
            let Some(t) = self.tok()? else {
                return Ok(false);
            };
            match (kind, t) {
                (b"bfrange", Tok::Str(dst)) if !dst.is_empty() => {
                    self.cmap.add_bfrange(&lo, &hi, &dst, &mut self.warn)
                }
                (b"bfrange", Tok::Str(_)) => {
                    self.warn.push("Invalid CMap mapping entry. (ignored)".into())
                }
                (b"bfrange", Tok::Mark) => {
                    if lo.is_empty() {
                        return Err("Invalid code range.".into());
                    }
                    let n = lo.len();
                    let mut code = lo.clone();
                    let k = hi[n - 1] as i32 - lo[n - 1] as i32 + 1;
                    for _ in 0..k.max(0) {
                        match self.tok()? {
                            None => return Ok(false),
                            Some(Tok::Str(dst)) => {
                                if dst.is_empty() {
                                    self.warn.push("Invalid CMap mapping entry. (ignored)".into());
                                } else {
                                    self.cmap.add_bfrange(&code, &code, &dst, &mut self.warn);
                                }
                            }
                            Some(Tok::Name(_)) => {
                                return Err("CMap_parse: Mapping to charName not supported.".into())
                            }
                            Some(_) => return Err("CMap_parse: Invalid CMap mapping record.".into()),
                        }
                        code[n - 1] = code[n - 1].wrapping_add(1);
                    }
                    if !self.next_is(b"]")? {
                        return Ok(false);
                    }
                }
                (b"cidrange" | b"notdefrange", Tok::Int(v)) => {
                    if (0..=CID_MAX).contains(&v) {
                        if kind == b"cidrange" {
                            self.cmap.add_cidrange(&lo, &hi, v, &mut self.warn);
                        } else {
                            self.cmap.add_notdefrange(&lo, &hi, v, &mut self.warn);
                        }
                    }
                }
                _ => self
                    .warn
                    .push("CMap_parse: Invalid CMap mapping record. (ignored)".into()),
            }
        }
        let mut end = b"end".to_vec();
        end.extend_from_slice(kind);
        self.next_is(&end)
    }

    fn chars(&mut self, mut count: i32, kind: &[u8]) -> Status {
        while count > 0 {
            count -= 1;
            let Some(t1) = self.tok()? else {
                return Ok(false);
            };
            let Some(t2) = self.tok()? else {
                return Ok(false);
            };
            match (kind, t1, t2) {
                (b"bfchar", Tok::Str(src), Tok::Str(dst)) => {
                    if src.is_empty() || dst.is_empty() {
                        self.warn.push("Invalid CMap mapping entry. (ignored)".into());
                    } else {
                        self.cmap.add_bfrange(&src, &src, &dst, &mut self.warn);
                    }
                }
                (b"bfchar", _, Tok::Name(_)) => {
                    return Err("CMap_parse: Mapping to charName not supported.".into())
                }
                (b"cidchar" | b"notdefchar", Tok::Str(src), Tok::Int(v)) => {
                    if (0..=CID_MAX).contains(&v) {
                        if src.is_empty() {
                            self.warn.push("Invalid CMap mapping entry. (ignored)".into());
                        } else if kind == b"cidchar" {
                            self.cmap.add_cidrange(&src, &src, v, &mut self.warn);
                        } else {
                            self.cmap.add_notdefrange(&src, &src, v, &mut self.warn);
                        }
                    }
                }
                _ => self
                    .warn
                    .push("CMap_parse: Invalid CMap mapping record. (ignored)".into()),
            }
        }
        let mut end = b"end".to_vec();
        end.extend_from_slice(kind);
        self.next_is(&end)
    }

    /// `do_cidsysteminfo`.
    fn cidsysteminfo(&mut self) -> Status {
        let mut simple = false;
        while let Some(t) = self.tok()? {
            match t {
                Tok::Mark => {
                    simple = true;
                    break;
                }
                Tok::Unknown(v) if v == b"begin" => break,
                _ => {}
            }
        }
        let (mut reg, mut ord, mut sup) = (None, None, -1);
        let mut ok = true;
        while ok {
            let Some(t1) = self.tok()? else {
                break;
            };
            match t1 {
                Tok::Unknown(v) if v == b">>" && simple => break,
                Tok::Unknown(v) if v == b"end" && !simple => break,
                Tok::Name(n) if n == b"Registry" || n == b"Ordering" || n == b"Supplement" => {
                    let Some(t2) = self.tok()? else {
                        continue;
                    };
                    let good = match (&t2, n.as_slice()) {
                        (Tok::Str(_), b"Registry" | b"Ordering") | (Tok::Int(_), b"Supplement") => true,
                        _ => false,
                    };
                    if !good || (!simple && !self.next_is(b"def")?) {
                        ok = false;
                    } else {
                        match t2 {
                            Tok::Str(s) if n == b"Registry" => reg = Some(s),
                            Tok::Str(s) => ord = Some(s),
                            Tok::Int(i) => sup = i,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if ok && !self.next_is(b"def")? {
            ok = false;
        }
        if ok && sup >= 0 {
            if let (Some(r), Some(o)) = (reg, ord) {
                self.cmap.csi = Some((r, o));
            }
        }
        Ok(ok)
    }

    /// `CMap_parse`: `Ok(true)` when the CMap is valid; an `Err` for a
    /// parse failure (fatal in `CMap_cache_find`) or a fatal error.
    fn parse(&mut self) -> Result<bool, Fatal> {
        let mut tmpint = -1;
        loop {
            let Some(t1) = self.tok()? else {
                break;
            };
            let ok = match &t1 {
                Tok::Name(n) if n == b"CMapName" => match self.tok()? {
                    Some(Tok::Name(v) | Tok::Str(v)) if self.next_is(b"def")? => {
                        let v = v[..v.iter().position(|&c| c == 0).unwrap_or(v.len())].to_vec();
                        self.cmap.name = Some(v);
                        true
                    }
                    _ => false,
                },
                Tok::Name(n) if n == b"CMapType" || n == b"WMode" => match self.tok()? {
                    Some(Tok::Int(v)) if self.next_is(b"def")? => {
                        if n == b"CMapType" {
                            self.cmap.ty = v;
                        }
                        true
                    }
                    _ => false,
                },
                Tok::Name(n) if n == b"CIDSystemInfo" => self.cidsysteminfo()?,
                Tok::Name(n) if n == b"Version" || n == b"UIDOffset" || n == b"XUID" => true,
                Tok::Name(n) => match self.tok()? {
                    Some(Tok::Unknown(op)) if op == b"usecmap" => {
                        match self.cache.find(n, &mut self.warn)? {
                            None => false,
                            Some(id) => {
                                let u = self.cache.cmaps[id].clone();
                                self.set_usecmap(u)?;
                                true
                            }
                        }
                    }
                    _ => true,
                },
                Tok::Unknown(op) => match op.as_slice() {
                    b"begincodespacerange" => self.codespacerange(tmpint)?,
                    b"beginnotdefrange" => self.range(tmpint, b"notdefrange")?,
                    b"beginnotdefchar" => self.chars(tmpint, b"notdefchar")?,
                    b"beginbfrange" => self.range(tmpint, b"bfrange")?,
                    b"beginbfchar" => self.chars(tmpint, b"bfchar")?,
                    b"begincidrange" => self.range(tmpint, b"cidrange")?,
                    b"begincidchar" => self.chars(tmpint, b"cidchar")?,
                    _ => true,
                },
                Tok::Int(v) => {
                    tmpint = *v;
                    true
                }
                _ => true,
            };
            if !ok {
                return Err("CMap: Parsing CMap file failed.".into());
            }
        }
        Ok(self.cmap.is_valid(&mut self.warn))
    }

    /// `CMap_set_usecmap`.
    fn set_usecmap(&mut self, u: Arc<CMap>) -> Result<(), Fatal> {
        if !u.is_valid(&mut self.warn) {
            return Err("CMap: Invalid CMap.".into());
        }
        if self.cmap.name.is_some() && self.cmap.name == u.name {
            return Err("CMap: CMap refering itself not allowed".into());
        }
        if let Some(c) = &self.cmap.csi {
            if Some(c) != u.csi.as_ref() {
                return Err("CMap: CMap required by another have different CSI.".into());
            }
        }
        for (lo, hi) in &u.codespace {
            self.cmap.add_codespacerange(lo, hi, &mut self.warn);
        }
        self.cmap.use_cmap = Some(u);
        Ok(())
    }
}

/// cmap.c's CMap cache: Identity-H and Identity-V, then every CMap loaded,
/// found again by its `/CMapName`.
#[derive(Clone, Debug)]
pub struct Cache {
    cmaps: Vec<Arc<CMap>>,
}

impl Default for Cache {
    fn default() -> Self {
        Cache {
            cmaps: vec![
                Arc::new(CMap::identity(b"Identity-H")),
                Arc::new(CMap::identity(b"Identity-V")),
            ],
        }
    }
}

impl Cache {
    pub fn get(&self, id: usize) -> &Arc<CMap> {
        &self.cmaps[id]
    }

    /// `CMap_cache_find`: the CMap's id, `None` when there is no such CMap
    /// file (or it does not look like one), an `Err` when it fails to parse.
    pub fn find(&mut self, name: &[u8], warn: &mut Vec<String>) -> Result<Option<usize>, Fatal> {
        if let Some(id) = self.cmaps.iter().position(|c| c.name.as_deref() == Some(name)) {
            return Ok(Some(id));
        }
        let Ok(n) = std::str::from_utf8(name) else {
            return Ok(None);
        };
        let Some(path) = flashtex_engine::system::find_file(n, flashtex_engine::resolver::Format::Cmap)
        else {
            return Ok(None);
        };
        let Ok(data) = std::fs::read(&path) else {
            return Ok(None);
        };
        self.load(&data, warn)
    }

    /// The part of [`Cache::find`] after the file is found.
    pub fn load(&mut self, data: &[u8], warn: &mut Vec<String>) -> Result<Option<usize>, Fatal> {
        if !check_sig(data) {
            return Ok(None);
        }
        let mut r = Reader {
            lx: Lexer { s: data, i: 0 },
            cmap: CMap::new(),
            cache: self,
            warn: Vec::new(),
        };
        let res = r.parse();
        warn.append(&mut r.warn);
        res?;
        let cmap = r.cmap;
        self.cmaps.push(Arc::new(cmap));
        Ok(Some(self.cmaps.len() - 1))
    }
}

/// spc_pdfm.c's `struct tounicode`: the CMap of the last `pdf:tounicode`
/// (`None` before one, or after one naming a CMap that does not load),
/// whether strings are read "tainted" (a CMap named for a Shift-JIS-like
/// encoding: `RKSJ`, `B5`, `GBK`, `KSC`), and the taint keys the specials
/// added to the default ones.
#[derive(Clone, Debug, Default)]
pub struct ToUnicode {
    pub cache: Cache,
    pub cmap: Option<usize>,
    pub unescape_backslash: bool,
    pub extra_keys: Vec<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const UTF8: &[u8] = b"%!PS-Adobe-3.0 Resource-CMap\n%%Title: (T)\n%%padding padding padding\n\
/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
/CIDSystemInfo 3 dict dup begin /Registry (DPX) def /Ordering (UTF8_UCS2) def /Supplement 0 def end def\n\
/CMapName /T def /CMapType 2 def\n\
2 begincodespacerange <00> <7F> <C280> <DFBF> endcodespacerange\n\
2 beginbfrange <00> <7F> <0000> <C380> <C3BF> <00C0> endbfrange\n\
1 beginbfchar <C2A0> <00A0> endbfchar\n\
endcmap CMapName currentdict /CMap defineresource pop end end\n";

    fn decode(c: &CMap, s: &[u8]) -> Option<Vec<u8>> {
        reencode(c, s, &mut Vec::new()).unwrap()
    }

    #[test]
    fn utf8_cmap() {
        let mut cache = Cache::default();
        let id = cache.load(UTF8, &mut Vec::new()).unwrap().unwrap();
        assert_eq!(cache.find(b"T", &mut Vec::new()).unwrap(), Some(id));
        let c = cache.get(id).clone();
        assert_eq!(decode(&c, b"a\xc3\xa9"), Some(b"\xfe\xff\x00a\x00\xe9".to_vec()));
        assert_eq!(decode(&c, b""), Some(b"\xfe\xff".to_vec()));
        // as xdvipdfmx (measured: `x𝔸` through UTF8-UCS2 is
        // <feff0078fffdfffdfffdfffd>): each byte outside the codespace is
        // one U+FFFD
        assert_eq!(
            decode(&c, "x𝔸".as_bytes()),
            Some(b"\xfe\xff\x00x\xff\xfd\xff\xfd\xff\xfd\xff\xfd".to_vec())
        );
        assert_eq!(decode(&c, b"\xc2\xa0"), Some(b"\xfe\xff\x00\xa0".to_vec()));
    }

    #[test]
    fn identity() {
        let c = Cache::default().get(0).clone();
        assert_eq!(decode(&c, b"abcd"), Some(b"\xfe\xffabcd".to_vec()));
        assert!(reencode(&c, b"abc", &mut Vec::new()).is_err());
    }

    #[test]
    fn signature() {
        assert!(Cache::default().load(b"%!PS-Adobe-3.0 Resource-Font", &mut Vec::new()).unwrap().is_none());
    }
}
