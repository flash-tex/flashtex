//! PDF objects as the dvipdfmx `\special` language writes them
//! (`pdf:obj @name <<...>>`, `pdf:bann <<...>>`, `pdf:dest (name) [...]`):
//! the syntax of PDF 32000-1 §7.3 plus dvipdfmx's references to named
//! objects (`@name`, `@thispage`, `@xpos`, ...), read into [`Obj`] and
//! written back as PDF text.
//!
//! Numbers keep the text they were written with, so that a value the
//! special gives is written into the PDF digit for digit.

use std::fmt::Write as _;

pub use super::num::{fmt_num, push_num};

/// A PDF object, or a reference to a named object (`@name`).
#[derive(Clone, Debug, PartialEq)]
pub enum Obj {
    Null,
    Bool(bool),
    /// A number, as written (`12`, `-.5`, `3.14`).
    Num(Vec<u8>),
    /// A string's bytes (a literal string's escapes resolved, a hex
    /// string's digits decoded).
    Str(Vec<u8>),
    /// A name's bytes, without the `/` and with `#xx` escapes resolved.
    Name(Vec<u8>),
    Array(Vec<Obj>),
    Dict(Dict),
    /// An indirect reference `n g R`.
    Ref(u32, u16),
    /// dvipdfmx's reference to a named object, `@name` (without the `@`).
    Named(Vec<u8>),
}

/// A dictionary: its entries in the order written. A later entry for a key
/// replaces an earlier one ([`Dict::set`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dict(pub Vec<(Vec<u8>, Obj)>);

impl Dict {
    pub fn get(&self, k: &[u8]) -> Option<&Obj> {
        self.0.iter().find(|(n, _)| n == k).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, k: &[u8]) -> Option<&mut Obj> {
        self.0.iter_mut().find(|(n, _)| n == k).map(|(_, v)| v)
    }

    /// Set `k` to `v`, replacing an entry of that key in place.
    pub fn set(&mut self, k: &[u8], v: Obj) {
        match self.0.iter_mut().find(|(n, _)| n == k) {
            Some(e) => e.1 = v,
            None => self.0.push((k.to_vec(), v)),
        }
    }

    pub fn remove(&mut self, k: &[u8]) -> Option<Obj> {
        let i = self.0.iter().position(|(n, _)| n == k)?;
        Some(self.0.remove(i).1)
    }

    /// `pdf:put @dict <<...>>`: every entry of `o`, replacing old values.
    pub fn merge(&mut self, o: &Dict) {
        for (k, v) in &o.0 {
            self.set(k, v.clone());
        }
    }
}

impl Obj {
    pub fn as_num(&self) -> Option<f64> {
        match self {
            Obj::Num(t) => std::str::from_utf8(t).ok()?.parse::<f64>().ok(),
            _ => None,
        }
    }
    pub fn as_name(&self) -> Option<&[u8]> {
        match self {
            Obj::Name(n) => Some(n),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&[u8]> {
        match self {
            Obj::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_dict(&self) -> Option<&Dict> {
        match self {
            Obj::Dict(d) => Some(d),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&[Obj]> {
        match self {
            Obj::Array(a) => Some(a),
            _ => None,
        }
    }

    /// A number written with at most `digits` fraction digits, trailing
    /// zeros dropped (`-0` written `0`).
    pub fn num(v: f64, digits: usize) -> Obj {
        let mut out = Vec::new();
        push_num(&mut out, v, digits);
        Obj::Num(out)
    }
}

pub fn is_white(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | 0)
}

pub fn is_delim(c: u8) -> bool {
    matches!(
        c,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// A reader over a special's bytes.
pub struct Parser<'a> {
    pub s: &'a [u8],
    pub i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(s: &'a [u8]) -> Self {
        Parser { s, i: 0 }
    }

    pub fn rest(&self) -> &'a [u8] {
        &self.s[self.i.min(self.s.len())..]
    }

    pub fn at_end(&mut self) -> bool {
        self.skip_white();
        self.i >= self.s.len()
    }

    /// Skip white space and comments.
    pub fn skip_white(&mut self) {
        while self.i < self.s.len() {
            let c = self.s[self.i];
            if is_white(c) {
                self.i += 1;
            } else if c == b'%' {
                while self.i < self.s.len() && !matches!(self.s[self.i], b'\n' | b'\r') {
                    self.i += 1;
                }
            } else {
                break;
            }
        }
    }

    pub fn peek(&mut self) -> Option<u8> {
        self.skip_white();
        self.s.get(self.i).copied()
    }

    /// A bare word (letters, digits and any regular character).
    pub fn word(&mut self) -> Option<&'a [u8]> {
        self.skip_white();
        let st = self.i;
        while self.i < self.s.len() && !is_white(self.s[self.i]) && !is_delim(self.s[self.i]) {
            self.i += 1;
        }
        (self.i > st).then(|| &self.s[st..self.i])
    }

    /// The next word if it is `w` (consumed), else nothing is consumed.
    pub fn keyword(&mut self, w: &[u8]) -> bool {
        let save = self.i;
        match self.word() {
            Some(x) if x == w => true,
            _ => {
                self.i = save;
                false
            }
        }
    }

    /// A number (as text), or `None` (nothing consumed).
    pub fn number(&mut self) -> Option<f64> {
        let save = self.i;
        match self.word() {
            Some(w) if is_number(w) => std::str::from_utf8(w).ok()?.parse::<f64>().ok(),
            _ => {
                self.i = save;
                None
            }
        }
    }

    /// A dimension of dvipdfmx's specials: a number and an optional unit
    /// (`pt`, `bp`, `in`, `cm`, `mm`, `pc`, `dd`, `cc`, `sp`, each
    /// possibly `true`), in bp. A bare number is in bp.
    pub fn dimension(&mut self) -> Option<f64> {
        self.skip_white();
        let save = self.i;
        let w = self.word()?;
        // the number, and a unit right after it (`614.295pt`) or after
        // white space (`72 bp`)
        let n = w
            .iter()
            .position(|c| !matches!(c, b'0'..=b'9' | b'.' | b'+' | b'-'))
            .unwrap_or(w.len());
        if n == 0 || !is_number(&w[..n]) {
            self.i = save;
            return None;
        }
        let v: f64 = std::str::from_utf8(&w[..n]).ok()?.parse().ok()?;
        let after_number = self.i;
        let (unit, unit_glued) = if n < w.len() {
            (w[n..].to_vec(), true)
        } else {
            match self.word() {
                Some(u) => (u.to_vec(), false),
                None => return Some(v),
            }
        };
        let u = unit.strip_prefix(b"true").unwrap_or(&unit);
        let k = match u {
            b"pt" => 72.0 / 72.27,
            b"bp" => 1.0,
            b"in" => 72.0,
            b"cm" => 72.0 / 2.54,
            b"mm" => 72.0 / 25.4,
            b"pc" => 12.0 * 72.0 / 72.27,
            b"dd" => 1238.0 / 1157.0 * 72.0 / 72.27,
            b"cc" => 12.0 * 1238.0 / 1157.0 * 72.0 / 72.27,
            b"sp" => 72.0 / 72.27 / 65536.0,
            _ => {
                if unit_glued {
                    self.i = save;
                    return None;
                }
                self.i = after_number;
                return Some(v);
            }
        };
        Some(v * k)
    }

    /// One object, or `None` (with the position unchanged) if the next
    /// token does not begin one.
    pub fn object(&mut self) -> Option<Obj> {
        let save = self.i;
        let r = self.object_inner(0);
        if r.is_none() {
            self.i = save;
        }
        r
    }

    fn object_inner(&mut self, depth: u32) -> Option<Obj> {
        if depth > 64 {
            return None;
        }
        let c = self.peek()?;
        match c {
            b'(' => self.literal_string().map(Obj::Str),
            b'<' if self.s.get(self.i + 1) == Some(&b'<') => {
                self.i += 2;
                let mut d = Dict::default();
                loop {
                    match self.peek()? {
                        b'>' if self.s.get(self.i + 1) == Some(&b'>') => {
                            self.i += 2;
                            break;
                        }
                        b'/' => {
                            let k = self.name()?;
                            let v = self.object_inner(depth + 1)?;
                            d.set(&k, v);
                        }
                        _ => return None,
                    }
                }
                Some(Obj::Dict(d))
            }
            b'<' => self.hex_string().map(Obj::Str),
            b'[' => {
                self.i += 1;
                let mut a = Vec::new();
                loop {
                    if self.peek()? == b']' {
                        self.i += 1;
                        break;
                    }
                    a.push(self.object_inner(depth + 1)?);
                }
                Some(Obj::Array(a))
            }
            b'/' => self.name().map(Obj::Name),
            b'@' => {
                self.i += 1;
                let st = self.i;
                while self.i < self.s.len()
                    && !is_white(self.s[self.i])
                    && !is_delim(self.s[self.i])
                {
                    self.i += 1;
                }
                (self.i > st).then(|| Obj::Named(self.s[st..self.i].to_vec()))
            }
            // pdfparse.c's parse_pdf_number: a sign (not the input's last
            // byte), digits and at most one point up to the next white
            // space or delimiter; `-`, `.` and `-.` are 0. There are no
            // indirect references `n g R` in a special (the C parses them
            // only in a PDF file): `3 0 R` is 3, 0 and a token that is not
            // an object.
            b'+' | b'-' | b'.' | b'0'..=b'9' => {
                let st = self.i;
                let mut e = st;
                if matches!(c, b'+' | b'-') {
                    if st + 1 >= self.s.len() {
                        return None;
                    }
                    e += 1;
                }
                let mut dots = 0;
                while e < self.s.len() && !is_white(self.s[e]) && !is_pdf_delim(self.s[e]) {
                    match self.s[e] {
                        b'0'..=b'9' => {}
                        b'.' if dots == 0 => dots += 1,
                        _ => return None,
                    }
                    e += 1;
                }
                self.i = e;
                Some(Obj::Num(self.s[st..e].to_vec()))
            }
            _ => {
                let w = self.word()?;
                match w {
                    b"null" => Some(Obj::Null),
                    b"true" => Some(Obj::Bool(true)),
                    b"false" => Some(Obj::Bool(false)),
                    _ => None,
                }
            }
        }
    }

    /// `/Name` (the `/` included in the input), `#xx` resolved, as
    /// pdfparse.c's parse_pdf_name reads it: a `#` without two hex digits
    /// after it is dropped with the two bytes after it (whatever they are),
    /// one too near the end ends the name, `#00` is dropped, and an empty
    /// name is no name.
    pub fn name(&mut self) -> Option<Vec<u8>> {
        self.skip_white();
        if self.s.get(self.i) != Some(&b'/') {
            return None;
        }
        let save = self.i;
        self.i += 1;
        let mut out = Vec::new();
        while self.i < self.s.len() && !is_white(self.s[self.i]) && !is_pdf_delim(self.s[self.i]) {
            let c = self.s[self.i];
            if c == b'#' {
                if self.i + 2 >= self.s.len() {
                    self.i = self.s.len();
                    continue;
                }
                let (h, l) = (hexval(self.s[self.i + 1]), hexval(self.s[self.i + 2]));
                self.i += 3;
                if let (Some(h), Some(l)) = (h, l) {
                    if h * 16 + l != 0 {
                        out.push(h * 16 + l);
                    }
                }
                continue;
            }
            out.push(c);
            self.i += 1;
        }
        if out.is_empty() {
            self.i = save;
            return None;
        }
        Some(out)
    }

    /// `(...)`: balanced parentheses, escapes resolved (PDF 32000-1 §7.3.4.2).
    pub fn literal_string(&mut self) -> Option<Vec<u8>> {
        self.skip_white();
        if self.s.get(self.i) != Some(&b'(') {
            return None;
        }
        self.i += 1;
        let mut out = Vec::new();
        let mut depth = 1;
        while self.i < self.s.len() {
            let c = self.s[self.i];
            self.i += 1;
            match c {
                b'(' => {
                    depth += 1;
                    out.push(c);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(out);
                    }
                    out.push(c);
                }
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else {
                        break;
                    };
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'0'..=b'7' => {
                            let mut v = (e - b'0') as u32;
                            for _ in 0..2 {
                                match self.s.get(self.i) {
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
                            if self.s.get(self.i) == Some(&b'\n') {
                                self.i += 1;
                            }
                        }
                        b'\n' => {}
                        _ => out.push(e),
                    }
                }
                _ => out.push(c),
            }
        }
        None
    }

    /// `<hex>`.
    pub fn hex_string(&mut self) -> Option<Vec<u8>> {
        self.skip_white();
        if self.s.get(self.i) != Some(&b'<') {
            return None;
        }
        self.i += 1;
        let mut digits = Vec::new();
        while self.i < self.s.len() {
            let c = self.s[self.i];
            self.i += 1;
            if c == b'>' {
                if digits.len() % 2 == 1 {
                    digits.push(0);
                }
                return Some(digits.chunks(2).map(|p| p[0] * 16 + p[1]).collect());
            }
            if let Some(v) = hexval(c) {
                digits.push(v);
            } else if !is_white(c) {
                return None;
            }
        }
        None
    }
}

fn hexval(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// pdfparse.c's delimiters (no `{` and `}`, which PDF has but the C
/// does not treat as delimiters).
fn is_pdf_delim(c: u8) -> bool {
    matches!(c, b'(' | b')' | b'/' | b'<' | b'>' | b'[' | b']' | b'%')
}

/// A number of a special's object as xdvipdfmx writes it: the value
/// pdfparse.c's parse_pdf_number computes from the text (digits after the
/// point added as `d / 10^k`), written by pdfdev.c's `p_dtoa` with 8
/// fraction digits (`.5`, `-.5`, `1.5`, `13` for `12.999999999`, `0` for
/// `-0`). `t` is a number [`Parser`] read.
pub fn special_number(t: &[u8]) -> Vec<u8> {
    let (sign, digits) = match t.first() {
        Some(b'-') => (-1.0, &t[1..]),
        Some(b'+') => (1.0, &t[1..]),
        _ => (1.0, t),
    };
    let mut v = 0.0f64;
    let mut frac: Option<i32> = None;
    for &c in digits {
        match (c, frac) {
            (b'.', _) => frac = Some(0),
            (b'0'..=b'9', None) => v = v * 10.0 + (c - b'0') as f64,
            (b'0'..=b'9', Some(n)) => {
                v += (c - b'0') as f64 / 10f64.powf((n + 1) as f64);
                frac = Some(n + 1);
            }
            _ => {}
        }
    }
    p_dtoa(sign * v, 8)
}

/// pdfdev.c's `p_dtoa(value, prec, buf)`.
fn p_dtoa(value: f64, prec: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut v = value;
    if v < 0.0 {
        v = -v;
        out.push(b'-');
    }
    let mut i = v.trunc();
    let f = v - i;
    let p = 10i32.pow(prec);
    let mut g = (f * p as f64 + 0.5) as i32;
    if g == p {
        g = 0;
        i += 1.0;
    }
    if i != 0.0 {
        let _ = write!(Bytes(&mut out), "{i:.0}");
    } else if g == 0 {
        out.clear();
        out.push(b'0');
    }
    if g != 0 {
        let mut d = format!("{g:0w$}", w = prec as usize).into_bytes();
        while d.last() == Some(&b'0') {
            d.pop();
        }
        out.push(b'.');
        out.extend_from_slice(&d);
    }
    out
}

/// `[+-]digits[.digits]`, also `.5` and `5.`.
pub fn is_number(w: &[u8]) -> bool {
    let w = match w.first() {
        Some(b'+' | b'-') => &w[1..],
        _ => w,
    };
    let mut digits = 0;
    let mut dots = 0;
    for &c in w {
        match c {
            b'0'..=b'9' => digits += 1,
            b'.' => dots += 1,
            _ => return false,
        }
    }
    digits > 0 && dots <= 1
}

/// Writes objects as PDF text. `named` resolves `@name` references (to
/// `n 0 R`, or to the text of a value such as `@xpos`); an unresolved name
/// is written `null`.
pub struct Writer<'a> {
    pub named: &'a mut dyn FnMut(&[u8]) -> Option<String>,
}

impl Writer<'_> {
    pub fn write(&mut self, o: &Obj, out: &mut Vec<u8>) {
        match o {
            Obj::Null => out.extend_from_slice(b"null"),
            Obj::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
            Obj::Num(t) => out.extend_from_slice(t),
            Obj::Str(s) => write_string(s, out),
            Obj::Name(n) => write_name(n, out),
            Obj::Array(a) => {
                out.push(b'[');
                for (i, e) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(b' ');
                    }
                    self.write(e, out);
                }
                out.push(b']');
            }
            Obj::Dict(d) => {
                out.extend_from_slice(b"<<");
                for (k, v) in &d.0 {
                    write_name(k, out);
                    out.push(b' ');
                    self.write(v, out);
                }
                out.extend_from_slice(b">>");
            }
            Obj::Ref(n, g) => {
                let _ = write!(Bytes(out), "{n} {g} R");
            }
            Obj::Named(n) => match (self.named)(n) {
                Some(t) => out.extend_from_slice(t.as_bytes()),
                None => out.extend_from_slice(b"null"),
            },
        }
    }
}

struct Bytes<'a>(&'a mut Vec<u8>);
impl std::fmt::Write for Bytes<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

/// A literal string, with `(`, `)` and `\` escaped and bytes outside
/// printable ASCII as octal escapes.
pub fn write_string(s: &[u8], out: &mut Vec<u8>) {
    out.push(b'(');
    for &c in s {
        match c {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(c);
            }
            0x20..=0x7e => out.push(c),
            _ => {
                let _ = write!(Bytes(out), "\\{c:03o}");
            }
        }
    }
    out.push(b')');
}

/// `/Name`, with delimiters, `#` and bytes outside `!`..`~` as `#xx`.
pub fn write_name(n: &[u8], out: &mut Vec<u8>) {
    out.push(b'/');
    for &c in n {
        if c <= b' ' || c > b'~' || c == b'#' || is_delim(c) {
            let _ = write!(Bytes(out), "#{c:02X}");
        } else {
            out.push(c);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::out::num::fmt_num_std as fmt_num_reference;

    fn p(s: &str) -> Obj {
        Parser::new(s.as_bytes()).object().unwrap()
    }

    #[test]
    fn objects() {
        assert_eq!(p("null"), Obj::Null);
        assert_eq!(p(" -.5 "), Obj::Num(b"-.5".to_vec()));
        // no `n g R` in a special: the 12 alone
        assert_eq!(p("12 0 R"), Obj::Num(b"12".to_vec()));
        assert_eq!(p("(a\\(b\\)\\101\\\nc)"), Obj::Str(b"a(b)Ac".to_vec()));
        assert_eq!(p("<48 69>"), Obj::Str(b"Hi".to_vec()));
        assert_eq!(p("<4>"), Obj::Str(vec![0x40]));
        assert_eq!(p("/A#20B"), Obj::Name(b"A B".to_vec()));
        assert_eq!(p("@thispage"), Obj::Named(b"thispage".to_vec()));
        let d = p("<</Type/Annot/Border[0 0 1]/A<</S/GoTo/D(section.1)>>>>");
        let d = d.as_dict().unwrap();
        assert_eq!(d.get(b"Type"), Some(&Obj::Name(b"Annot".to_vec())));
        let a = d.get(b"A").unwrap().as_dict().unwrap();
        assert_eq!(a.get(b"D").unwrap().as_str(), Some(&b"section.1"[..]));
        let arr = p("[@thispage /XYZ @xpos @ypos null]");
        assert_eq!(arr.as_array().unwrap().len(), 5);
    }

    /// pdfparse.c's object reader on what TeX Live 2026's xdvipdfmx
    /// measurably does: `pdf:obj @c <</A#41 /x#2 /B#zz /C#20D /E 1>>` is
    /// `<</AA /x /B /C#20D /E 1>>`; `[true false null /n 3 0 R]` is no
    /// object; the numbers `0.5 -.5 1. 1.50 +3 0.123456789 -0 007
    /// .0000000001 12.999999999` are written `.5 -.5 1 1.5 3 .12345679 0 7
    /// 0 13`.
    #[test]
    fn as_xdvipdfmx_reads_them() {
        let d = p("<</A#41 /x#2 /B#zz /C#20D /E 1>>");
        let d = d.as_dict().unwrap();
        assert_eq!(d.0.len(), 3, "{d:?}");
        assert_eq!(d.get(b"AA"), Some(&Obj::Name(b"x".to_vec())));
        assert_eq!(d.get(b"B"), Some(&Obj::Name(b"C D".to_vec())));
        assert_eq!(d.get(b"E"), Some(&Obj::Num(b"1".to_vec())));
        assert_eq!(Parser::new(b"[true false null /n 3 0 R]").object(), None);
        assert_eq!(Parser::new(b"<</A 3 0 R>>").object(), None);
        assert_eq!(Parser::new(b"/").object(), None);
        assert_eq!(Parser::new(b"1.2.3").object(), None);
        assert_eq!(p("- "), Obj::Num(b"-".to_vec()));
        for (t, w) in [
            ("0.5", ".5"),
            ("-.5", "-.5"),
            ("1.", "1"),
            ("1.50", "1.5"),
            ("+3", "3"),
            ("0.123456789", ".12345679"),
            ("-0", "0"),
            ("007", "7"),
            ("1000000", "1000000"),
            (".0000000001", "0"),
            ("-.0000000001", "0"),
            ("12.999999999", "13"),
            ("-", "0"),
            (".", "0"),
            ("595.27559", "595.27559"),
        ] {
            assert_eq!(
                String::from_utf8(special_number(t.as_bytes())).unwrap(),
                w,
                "{t}"
            );
        }
    }

    #[test]
    fn written_back() {
        let o = p("<</T(a\\)b\\377)/N/A#20B/X[1 -2.50 @x]>>");
        let mut out = Vec::new();
        let mut named = |n: &[u8]| (n == b"x").then(|| "7 0 R".to_string());
        Writer { named: &mut named }.write(&o, &mut out);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "<</T (a\\)b\\377)/N /A#20B/X [1 -2.50 7 0 R]>>"
        );
    }

    #[test]
    fn dimensions() {
        let mut q = Parser::new(b"614.295pt 72 1in 2.54cm 10 truept");
        assert!((q.dimension().unwrap() - 612.0).abs() < 1e-3);
        assert_eq!(q.dimension(), Some(72.0));
        assert_eq!(q.dimension(), Some(72.0));
        assert!((q.dimension().unwrap() - 72.0).abs() < 1e-9);
        assert!((q.dimension().unwrap() - 9.9626).abs() < 1e-4);
        assert_eq!(fmt_num(1.0, 3), "1");
        assert_eq!(fmt_num(-0.0001, 3), "0");
        assert_eq!(fmt_num(88.75949, 3), "88.759");
    }

    /// A deterministic generator (xorshift64*), so that a failure repeats.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
        }
    }

    fn same(v: f64, d: usize) {
        let want = fmt_num_reference(v, d);
        let got = fmt_num(v, d);
        assert_eq!(got, want, "fmt_num({v:e} = {:#018x}, {d})", v.to_bits());
    }

    /// The fast path writes what `format!` writes: random bit patterns
    /// over 2^-80..2^100 of both signs, for every digit count the writer
    /// uses and the others to 9.
    #[test]
    fn fmt_num_is_the_reference_on_random_values() {
        let mut r = Rng(0x9E37_79B9_7F4A_7C15);
        for _ in 0..1_500_000 {
            let x = r.next();
            let exp = 1023 - 80 + (x % 181);
            let bits = (x & (1 << 63)) | (exp << 52) | (r.next() & ((1 << 52) - 1));
            let v = f64::from_bits(bits);
            same(v, (r.next() % 10) as usize);
        }
    }

    /// Next to every rounding boundary: `k / 10^d` (a value the writer
    /// is given exactly) and `(k + 1/2) / 10^d` (where the rounding
    /// turns), each with its neighbours a few ulps away, and integers.
    #[test]
    fn fmt_num_is_the_reference_at_rounding_boundaries() {
        let mut r = Rng(0xD1B5_4A32_D192_ED03);
        for _ in 0..60_000 {
            let d = [0usize, 1, 2, 3, 4, 5, 6, 9][(r.next() % 8) as usize];
            let p = 10f64.powi(d as i32);
            let k = (r.next() % 2_000_000_000) as f64 - 1e9;
            for base in [k / p, (k + 0.5) / p, k, k + 0.5] {
                let b = base.to_bits() as i64;
                for delta in -3i64..=3 {
                    let v = f64::from_bits((b + delta) as u64);
                    same(v, d);
                    same(-v, d);
                }
            }
        }
        for d in 0..=9 {
            for v in [
                0.0, -0.0, 0.5, -0.5, 1.5, 2.5, 1e-300, -1e-300, 5e-324, 1e20, 1e30, -1e30,
            ] {
                same(v, d);
            }
        }
    }

    /// Values as the PDF writer meets them: TeX dimensions (scaled points,
    /// /65536, times the bp factor 72/72.27), glyph matrix entries, and
    /// RGB components (n/255), at the digit counts they are written with.
    #[test]
    fn fmt_num_is_the_reference_on_writer_values() {
        let k = 65536.0 * 72.27 / 72.0;
        for sp in (-20_000_000i64..20_000_000).step_by(97) {
            let v = sp as f64 / k;
            same(v, 3);
            same(v, 5);
            same(792.0 - v, 5);
        }
        for i in 0..=255 {
            same(i as f64 / 255.0, 3);
            same(i as f64 / 255.0, 6);
        }
        let mut r = Rng(0x2545_F491_4F6C_DD1D);
        for _ in 0..300_000 {
            let size = (r.next() % 4_000_000) as f64 / 65536.0;
            let angle = (r.next() % 6283) as f64 / 1000.0;
            same(size * angle.cos(), 6);
            same(size * angle.sin(), 6);
            same(angle.cos(), 6);
        }
    }
}
