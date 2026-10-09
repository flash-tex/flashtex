//! PDF objects as the dvipdfmx `\special` language writes them
//! (`pdf:obj @name <<...>>`, `pdf:bann <<...>>`, `pdf:dest (name) [...]`):
//! the syntax of PDF 32000-1 §7.3 plus dvipdfmx's references to named
//! objects (`@name`, `@thispage`, `@xpos`, ...), read into [`Obj`] and
//! written back as PDF text.
//!
//! Numbers keep the text they were written with, so that a value the
//! special gives is written into the PDF digit for digit.

use std::fmt::Write as _;

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
        Obj::Num(fmt_num(v, digits).into_bytes())
    }
}

/// `v` with at most `digits` fraction digits, trailing zeros and a
/// trailing point dropped; `-0` is `0`.
pub fn fmt_num(v: f64, digits: usize) -> String {
    let mut s = format!("{v:.digits$}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
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
            _ => {
                let w = self.word()?;
                match w {
                    b"null" => Some(Obj::Null),
                    b"true" => Some(Obj::Bool(true)),
                    b"false" => Some(Obj::Bool(false)),
                    _ if is_number(w) => {
                        // `n g R`
                        let save = self.i;
                        if is_uint(w) {
                            if let Some(g) = self.word() {
                                if is_uint(g) && self.keyword(b"R") {
                                    let n = std::str::from_utf8(w).ok()?.parse().ok()?;
                                    let g = std::str::from_utf8(g).ok()?.parse().ok()?;
                                    return Some(Obj::Ref(n, g));
                                }
                            }
                            self.i = save;
                        }
                        Some(Obj::Num(w.to_vec()))
                    }
                    _ => None,
                }
            }
        }
    }

    /// `/Name` (the `/` included in the input), `#xx` resolved.
    pub fn name(&mut self) -> Option<Vec<u8>> {
        self.skip_white();
        if self.s.get(self.i) != Some(&b'/') {
            return None;
        }
        self.i += 1;
        let mut out = Vec::new();
        while self.i < self.s.len() && !is_white(self.s[self.i]) && !is_delim(self.s[self.i]) {
            let c = self.s[self.i];
            if c == b'#' {
                if let (Some(h), Some(l)) = (
                    self.s.get(self.i + 1).and_then(|&c| hexval(c)),
                    self.s.get(self.i + 2).and_then(|&c| hexval(c)),
                ) {
                    out.push(h * 16 + l);
                    self.i += 3;
                    continue;
                }
            }
            out.push(c);
            self.i += 1;
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

fn is_uint(w: &[u8]) -> bool {
    !w.is_empty() && w.iter().all(u8::is_ascii_digit)
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

    fn p(s: &str) -> Obj {
        Parser::new(s.as_bytes()).object().unwrap()
    }

    #[test]
    fn objects() {
        assert_eq!(p("null"), Obj::Null);
        assert_eq!(p(" -.5 "), Obj::Num(b"-.5".to_vec()));
        assert_eq!(p("12 0 R"), Obj::Ref(12, 0));
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

    #[test]
    fn written_back() {
        let o = p("<</T(a\\)b\\377)/N/A#20B/R 3 0 R/X[1 -2.50 @x]>>");
        let mut out = Vec::new();
        let mut named = |n: &[u8]| (n == b"x").then(|| "7 0 R".to_string());
        Writer { named: &mut named }.write(&o, &mut out);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "<</T (a\\)b\\377)/N /A#20B/R 3 0 R/X [1 -2.50 7 0 R]>>"
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
}
