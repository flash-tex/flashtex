//! A minimal PDF reader for typst-pdf's own output (krilla 0.8): the
//! cross-reference table, indirect objects, dictionaries, arrays, numbers
//! kept as the decimals the file writes, and Flate-compressed streams.
//!
//! It reads what this host's per-page export writes (DESIGN.md §15.5: the
//! glyph positions come from typst-pdf's content stream) and nothing more:
//! object streams, cross-reference streams, encryption and any filter but
//! `FlateDecode` are refused, never guessed. Spec §4.2 says how a viewer
//! evaluates a number; [`Num`] keeps the digits so that the caller can.

use std::collections::HashMap;

/// A number as the file writes it: sign, the digits as one integer and the
/// count of fraction digits (trailing zeros dropped), plus its nearest
/// double.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Num {
    pub neg: bool,
    /// The digits, `None` when they do not fit in a u128.
    pub digits: Option<u128>,
    /// Fraction digits after dropping trailing zeros.
    pub k: u32,
    /// The double nearest to the decimal.
    pub nearest: f64,
}

impl Num {
    pub fn parse(t: &[u8]) -> Option<Num> {
        let s = std::str::from_utf8(t).ok()?;
        let (neg, body) = match s.as_bytes().first()? {
            b'-' => (true, &s[1..]),
            b'+' => (false, &s[1..]),
            _ => (false, s),
        };
        let (int, frac) = match body.split_once('.') {
            Some((i, f)) => (i, f),
            None => (body, ""),
        };
        if int.is_empty() && frac.is_empty() {
            return None;
        }
        if !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit()) {
            return None;
        }
        let frac = frac.trim_end_matches('0');
        let mut digits: Option<u128> = Some(0);
        for b in int.bytes().chain(frac.bytes()) {
            digits = digits
                .and_then(|d| d.checked_mul(10))
                .and_then(|d| d.checked_add((b - b'0') as u128));
        }
        // The nearest double, correctly rounded by the standard library.
        let canon = format!(
            "{}{}.{}",
            if neg { "-" } else { "" },
            if int.is_empty() { "0" } else { int },
            if frac.is_empty() { "0" } else { frac }
        );
        Some(Num {
            neg,
            digits,
            k: frac.len() as u32,
            nearest: canon.parse().ok()?,
        })
    }

    /// The value as the reference viewer reads an operand (spec §4.2): a
    /// number with k fraction digits is its digits as an integer m times the
    /// double nearest to 10^-k (`237.283` is `237283 × 0.001`). Past 12
    /// fraction digits or 2^53, the nearest double.
    pub fn viewer(&self) -> f64 {
        const NEG_POW10: [f64; 13] = [
            1.0, 0.1, 0.01, 0.001, 1e-4, 1e-5, 1e-6, 1e-7, 1e-8, 1e-9, 1e-10, 1e-11, 1e-12,
        ];
        match self.digits {
            Some(m) if self.k <= 12 && m <= 1u128 << 53 => {
                let v = m as f64 * NEG_POW10[self.k as usize];
                if self.neg {
                    -v
                } else {
                    v
                }
            }
            _ => self.nearest,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        if self.k != 0 {
            return None;
        }
        let v = i64::try_from(self.digits?).ok()?;
        Some(if self.neg { -v } else { v })
    }
}

/// A PDF object.
#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    Null,
    Bool(bool),
    Num(Num),
    Name(Vec<u8>),
    Str(Vec<u8>),
    Arr(Vec<Obj>),
    Dict(Dict),
    Ref(u32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dict(pub Vec<(Vec<u8>, Obj)>);

impl Dict {
    pub fn get(&self, key: &str) -> Option<&Obj> {
        self.0
            .iter()
            .find(|(k, _)| k.as_slice() == key.as_bytes())
            .map(|(_, v)| v)
    }
}

impl Obj {
    pub fn num(&self) -> Option<&Num> {
        match self {
            Obj::Num(n) => Some(n),
            _ => None,
        }
    }
    pub fn name(&self) -> Option<&[u8]> {
        match self {
            Obj::Name(n) => Some(n),
            _ => None,
        }
    }
}

/// A lexical token of a PDF file or content stream.
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(Num),
    Name(Vec<u8>),
    Str(Vec<u8>),
    ArrOpen,
    ArrClose,
    DictOpen,
    DictClose,
    /// An operator or keyword (`obj`, `R`, `true`, `cm`, `TJ`, ...).
    Kw(Vec<u8>),
}

pub fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' | 0)
}

fn is_delim(c: u8) -> bool {
    matches!(
        c,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// A tokenizer over `b`, from `i`.
pub struct Lexer<'a> {
    pub b: &'a [u8],
    pub i: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(b: &'a [u8]) -> Lexer<'a> {
        Lexer { b, i: 0 }
    }

    fn skip_ws(&mut self) {
        while self.i < self.b.len() {
            let c = self.b[self.i];
            if is_ws(c) {
                self.i += 1;
            } else if c == b'%' {
                while self.i < self.b.len() && !matches!(self.b[self.i], b'\n' | b'\r') {
                    self.i += 1;
                }
            } else {
                break;
            }
        }
    }

    pub fn token(&mut self) -> Result<Option<Tok>, String> {
        self.skip_ws();
        let b = self.b;
        let Some(&c) = b.get(self.i) else {
            return Ok(None);
        };
        let start = self.i;
        let tok = match c {
            b'[' => {
                self.i += 1;
                Tok::ArrOpen
            }
            b']' => {
                self.i += 1;
                Tok::ArrClose
            }
            b'<' if b.get(self.i + 1) == Some(&b'<') => {
                self.i += 2;
                Tok::DictOpen
            }
            b'>' if b.get(self.i + 1) == Some(&b'>') => {
                self.i += 2;
                Tok::DictClose
            }
            b'<' => {
                let end = b[self.i..]
                    .iter()
                    .position(|&x| x == b'>')
                    .ok_or("unterminated hex string")?
                    + self.i;
                let hex: Vec<u8> = b[self.i + 1..end]
                    .iter()
                    .copied()
                    .filter(|x| !is_ws(*x))
                    .collect();
                let mut out = Vec::with_capacity(hex.len() / 2 + 1);
                for pair in hex.chunks(2) {
                    let h = |x: u8| -> Result<u8, String> {
                        (x as char)
                            .to_digit(16)
                            .map(|d| d as u8)
                            .ok_or_else(|| format!("bad hex digit {x:#x}"))
                    };
                    let hi = h(pair[0])?;
                    let lo = if pair.len() == 2 { h(pair[1])? } else { 0 };
                    out.push(hi << 4 | lo);
                }
                self.i = end + 1;
                Tok::Str(out)
            }
            b'(' => {
                let mut depth = 1;
                let mut out = Vec::new();
                self.i += 1;
                loop {
                    let Some(&x) = b.get(self.i) else {
                        return Err("unterminated string".into());
                    };
                    self.i += 1;
                    match x {
                        b'\\' => {
                            let Some(&n) = b.get(self.i) else {
                                return Err("unterminated string".into());
                            };
                            self.i += 1;
                            match n {
                                b'n' => out.push(b'\n'),
                                b'r' => out.push(b'\r'),
                                b't' => out.push(b'\t'),
                                b'b' => out.push(8),
                                b'f' => out.push(12),
                                b'0'..=b'7' => {
                                    let mut v = (n - b'0') as u32;
                                    for _ in 0..2 {
                                        match b.get(self.i) {
                                            Some(&d @ b'0'..=b'7') => {
                                                v = v * 8 + (d - b'0') as u32;
                                                self.i += 1;
                                            }
                                            _ => break,
                                        }
                                    }
                                    out.push(v as u8);
                                }
                                b'\r' => {
                                    if b.get(self.i) == Some(&b'\n') {
                                        self.i += 1;
                                    }
                                }
                                b'\n' => {}
                                other => out.push(other),
                            }
                        }
                        b'(' => {
                            depth += 1;
                            out.push(x);
                        }
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                            out.push(x);
                        }
                        _ => out.push(x),
                    }
                }
                Tok::Str(out)
            }
            b'/' => {
                self.i += 1;
                let s = self.i;
                while self.i < b.len() && !is_ws(b[self.i]) && !is_delim(b[self.i]) {
                    self.i += 1;
                }
                // `#xx` escapes.
                let raw = &b[s..self.i];
                let mut out = Vec::with_capacity(raw.len());
                let mut j = 0;
                while j < raw.len() {
                    if raw[j] == b'#' && j + 2 < raw.len() {
                        if let (Some(h), Some(l)) = (
                            raw.get(j + 1).and_then(|c| (*c as char).to_digit(16)),
                            raw.get(j + 2).and_then(|c| (*c as char).to_digit(16)),
                        ) {
                            out.push((h * 16 + l) as u8);
                            j += 3;
                            continue;
                        }
                    }
                    out.push(raw[j]);
                    j += 1;
                }
                Tok::Name(out)
            }
            b'{' | b'}' | b')' | b'>' => {
                return Err(format!("unexpected {:?} at byte {start}", c as char));
            }
            _ => {
                while self.i < b.len() && !is_ws(b[self.i]) && !is_delim(b[self.i]) {
                    self.i += 1;
                }
                let t = &b[start..self.i];
                match t[0] {
                    b'0'..=b'9' | b'-' | b'+' | b'.' => {
                        Tok::Num(Num::parse(t).ok_or_else(|| {
                            format!("bad number {:?}", String::from_utf8_lossy(t))
                        })?)
                    }
                    _ => Tok::Kw(t.to_vec()),
                }
            }
        };
        Ok(Some(tok))
    }

    /// One object (an indirect reference `n g R` included).
    pub fn object(&mut self) -> Result<Obj, String> {
        let t = self.token()?.ok_or("unexpected end of data")?;
        self.object_from(t)
    }

    pub fn object_from(&mut self, t: Tok) -> Result<Obj, String> {
        Ok(match t {
            Tok::Num(n) => {
                // `n g R`?
                let save = self.i;
                if let Some(gen) = n.as_i64().filter(|v| *v >= 0) {
                    if let Ok(Some(Tok::Num(g))) = self.token() {
                        if g.as_i64().is_some() {
                            if let Ok(Some(Tok::Kw(k))) = self.token() {
                                if k == b"R" {
                                    return Ok(Obj::Ref(gen as u32));
                                }
                            }
                        }
                    }
                }
                self.i = save;
                Obj::Num(n)
            }
            Tok::Name(n) => Obj::Name(n),
            Tok::Str(s) => Obj::Str(s),
            Tok::ArrOpen => {
                let mut v = Vec::new();
                loop {
                    match self.token()?.ok_or("unterminated array")? {
                        Tok::ArrClose => break,
                        t => v.push(self.object_from(t)?),
                    }
                }
                Obj::Arr(v)
            }
            Tok::DictOpen => {
                let mut d = Vec::new();
                loop {
                    match self.token()?.ok_or("unterminated dictionary")? {
                        Tok::DictClose => break,
                        Tok::Name(k) => {
                            let v = self.object()?;
                            d.push((k, v));
                        }
                        t => return Err(format!("dictionary key expected, got {t:?}")),
                    }
                }
                Obj::Dict(Dict(d))
            }
            Tok::Kw(k) => match k.as_slice() {
                b"null" => Obj::Null,
                b"true" => Obj::Bool(true),
                b"false" => Obj::Bool(false),
                _ => {
                    return Err(format!(
                        "unexpected keyword {:?}",
                        String::from_utf8_lossy(&k)
                    ))
                }
            },
            t => return Err(format!("unexpected token {t:?}")),
        })
    }
}

/// A parsed file: object offsets from the cross-reference table.
pub struct Pdf<'a> {
    data: &'a [u8],
    offsets: HashMap<u32, usize>,
    pub trailer: Dict,
}

/// An indirect object: its value and, for a stream, the raw stream bytes.
pub struct Indirect<'a> {
    pub obj: Obj,
    pub stream: Option<&'a [u8]>,
}

impl<'a> Pdf<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Pdf<'a>, String> {
        let tail = &data[data.len().saturating_sub(64)..];
        let at = rfind(tail, b"startxref").ok_or("no startxref")? + data.len() - tail.len();
        let mut lx = Lexer::new(data);
        lx.i = at + b"startxref".len();
        let xref = match lx.token()? {
            Some(Tok::Num(n)) => n.as_i64().ok_or("bad startxref")? as usize,
            _ => return Err("bad startxref".into()),
        };
        lx.i = xref;
        match lx.token()? {
            Some(Tok::Kw(k)) if k == b"xref" => {}
            _ => return Err("not a cross-reference table (xref streams are not read)".into()),
        }
        let mut offsets = HashMap::new();
        loop {
            match lx.token()? {
                Some(Tok::Num(first)) => {
                    let first = first.as_i64().ok_or("bad xref section")? as u32;
                    let count = match lx.token()? {
                        Some(Tok::Num(c)) => c.as_i64().ok_or("bad xref count")? as u32,
                        _ => return Err("bad xref section".into()),
                    };
                    for j in 0..count {
                        let off = match lx.token()? {
                            Some(Tok::Num(o)) => o.as_i64().ok_or("bad xref offset")?,
                            _ => return Err("bad xref entry".into()),
                        };
                        let _gen = lx.token()?;
                        match lx.token()? {
                            Some(Tok::Kw(k)) if k == b"n" => {
                                offsets.insert(first + j, off as usize);
                            }
                            Some(Tok::Kw(k)) if k == b"f" => {}
                            t => return Err(format!("bad xref entry type {t:?}")),
                        }
                    }
                }
                Some(Tok::Kw(k)) if k == b"trailer" => break,
                _ => return Err("bad cross-reference table".into()),
            }
        }
        let trailer = match lx.object()? {
            Obj::Dict(d) => d,
            _ => return Err("trailer is not a dictionary".into()),
        };
        if trailer.get("Encrypt").is_some() {
            return Err("encrypted PDF".into());
        }
        Ok(Pdf {
            data,
            offsets,
            trailer,
        })
    }

    /// Indirect object `n`.
    pub fn get(&self, n: u32) -> Result<Indirect<'a>, String> {
        let &off = self
            .offsets
            .get(&n)
            .ok_or_else(|| format!("no object {n}"))?;
        let mut lx = Lexer::new(self.data);
        lx.i = off;
        for want in [None, None, Some(&b"obj"[..])] {
            match (lx.token()?, want) {
                (Some(Tok::Num(_)), None) => {}
                (Some(Tok::Kw(k)), Some(w)) if k == w => {}
                t => return Err(format!("object {n}: bad header {t:?}")),
            }
        }
        let obj = lx.object()?;
        let stream = match lx.token()? {
            Some(Tok::Kw(k)) if k == b"stream" => {
                // The data starts after the EOL that follows `stream`.
                let mut s = lx.i;
                if self.data.get(s) == Some(&b'\r') {
                    s += 1;
                }
                if self.data.get(s) == Some(&b'\n') {
                    s += 1;
                }
                let Obj::Dict(d) = &obj else {
                    return Err(format!("object {n}: a stream without a dictionary"));
                };
                let len = match d.get("Length") {
                    Some(Obj::Num(l)) => l.as_i64().ok_or("bad /Length")? as usize,
                    Some(Obj::Ref(r)) => match self.get(*r)?.obj {
                        Obj::Num(l) => l.as_i64().ok_or("bad /Length")? as usize,
                        _ => return Err("bad /Length".into()),
                    },
                    _ => return Err(format!("object {n}: stream without /Length")),
                };
                Some(
                    self.data
                        .get(s..s + len)
                        .ok_or_else(|| format!("object {n}: stream past the end"))?,
                )
            }
            _ => None,
        };
        Ok(Indirect { obj, stream })
    }

    /// Follow a reference (one level; objects of typst-pdf never chain).
    pub fn resolve(&self, o: &Obj) -> Result<Obj, String> {
        match o {
            Obj::Ref(r) => Ok(self.get(*r)?.obj),
            o => Ok(o.clone()),
        }
    }

    /// A dictionary, directly or by reference.
    pub fn dict(&self, o: &Obj) -> Result<Dict, String> {
        match self.resolve(o)? {
            Obj::Dict(d) => Ok(d),
            o => Err(format!("dictionary expected, got {o:?}")),
        }
    }

    /// The decoded data of stream object `n`.
    pub fn stream(&self, n: u32) -> Result<(Dict, Vec<u8>), String> {
        let ind = self.get(n)?;
        let Obj::Dict(d) = ind.obj else {
            return Err(format!("object {n} is not a stream"));
        };
        let raw = ind
            .stream
            .ok_or_else(|| format!("object {n} is not a stream"))?;
        let data = match d.get("Filter").map(|f| self.resolve(f)).transpose()? {
            None => raw.to_vec(),
            Some(Obj::Name(f)) if f == b"FlateDecode" => inflate(raw)?,
            Some(Obj::Arr(a)) if a.is_empty() => raw.to_vec(),
            Some(Obj::Arr(a)) if a.len() == 1 && a[0] == Obj::Name(b"FlateDecode".to_vec()) => {
                inflate(raw)?
            }
            Some(f) => return Err(format!("object {n}: filter {f:?} is not read")),
        };
        if d.get("DecodeParms").is_some() {
            return Err(format!("object {n}: /DecodeParms is not read"));
        }
        Ok((d, data))
    }

    /// The page dictionaries, in order (the page tree from the catalog).
    pub fn pages(&self) -> Result<Vec<(u32, Dict)>, String> {
        let root = self.dict(self.trailer.get("Root").ok_or("no /Root")?)?;
        let mut out = Vec::new();
        let top = root.get("Pages").ok_or("no /Pages")?;
        self.walk_pages(top, &mut out, 0)?;
        Ok(out)
    }

    fn walk_pages(&self, node: &Obj, out: &mut Vec<(u32, Dict)>, depth: u32) -> Result<(), String> {
        if depth > 64 {
            return Err("page tree too deep".into());
        }
        let Obj::Ref(n) = node else {
            return Err("page tree node is not a reference".into());
        };
        let d = self.dict(node)?;
        match d.get("Type").and_then(Obj::name) {
            Some(b"Pages") => {
                let Some(Obj::Arr(kids)) = d.get("Kids").map(|k| self.resolve(k)).transpose()?
                else {
                    return Err("/Pages without /Kids".into());
                };
                for k in &kids {
                    self.walk_pages(k, out, depth + 1)?;
                }
            }
            Some(b"Page") => out.push((*n, d)),
            t => return Err(format!("page tree node of type {t:?}")),
        }
        Ok(())
    }

    /// A page attribute, inherited from the page tree when absent.
    pub fn inherited(&self, page: &Dict, key: &str) -> Result<Option<Obj>, String> {
        let mut d = page.clone();
        for _ in 0..64 {
            if let Some(v) = d.get(key) {
                return Ok(Some(v.clone()));
            }
            match d.get("Parent") {
                Some(p) => d = self.dict(p)?,
                None => return Ok(None),
            }
        }
        Err("page tree too deep".into())
    }
}

fn rfind(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).rposition(|w| w == n)
}

fn inflate(raw: &[u8]) -> Result<Vec<u8>, String> {
    miniz_oxide::inflate::decompress_to_vec_zlib(raw).map_err(|e| format!("FlateDecode: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_keep_their_digits() {
        let n = Num::parse(b"237.283").unwrap();
        assert_eq!((n.digits, n.k, n.neg), (Some(237283), 3, false));
        // The viewer's value is one ulp above the nearest double here.
        assert_eq!(n.viewer(), 237283.0 * 0.001);
        assert_eq!(n.nearest, 237.283);
        assert_ne!(n.viewer(), n.nearest);
        let n = Num::parse(b"-0.50").unwrap();
        assert_eq!((n.digits, n.k, n.neg), (Some(5), 1, true));
        assert_eq!(n.viewer(), -0.5);
        assert_eq!(Num::parse(b".5").unwrap().viewer(), 0.5);
        assert_eq!(Num::parse(b"5.").unwrap().as_i64(), Some(5));
        assert_eq!(Num::parse(b"-12").unwrap().as_i64(), Some(-12));
        assert!(Num::parse(b"-").is_none());
        assert!(Num::parse(b"1e5").is_none());
    }

    #[test]
    fn lexer_reads_strings_names_and_references() {
        let mut lx = Lexer::new(b"<</A 1 0 R/B[(a\\)b\\101) <4142 4>]/C#20d -.5>>");
        let Obj::Dict(d) = lx.object().unwrap() else {
            panic!()
        };
        assert_eq!(d.get("A"), Some(&Obj::Ref(1)));
        assert_eq!(
            d.get("B"),
            Some(&Obj::Arr(vec![
                Obj::Str(b"a)bA".to_vec()),
                Obj::Str(vec![0x41, 0x42, 0x40])
            ]))
        );
        assert_eq!(d.get("C d").and_then(Obj::num).unwrap().viewer(), -0.5);
    }
}
