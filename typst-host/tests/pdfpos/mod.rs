//! Glyph origins as a PDF viewer computes them, from typst-pdf's own export
//! (`pretty: true`: content streams uncompressed), independently of the
//! host: an object table, the page's content stream, each Type0 font's
//! `/W` widths, and the text-showing operators (PDF 32000-1 §9.4.4). The
//! only constructs handled are those typst-pdf (krilla) writes for text;
//! anything else in a text object fails the test rather than being guessed.
#![allow(dead_code)]

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Str(Vec<u8>),
    ArrOpen,
    ArrClose,
    DictOpen,
    DictClose,
    Op(String),
}

fn lex(s: &[u8]) -> Vec<Tok> {
    let mut v = Vec::new();
    let mut i = 0;
    let delim = |c: u8| b"()<>[]{}/%".contains(&c) || c.is_ascii_whitespace();
    while i < s.len() {
        let c = s[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'%' {
            while i < s.len() && s[i] != b'\n' {
                i += 1;
            }
        } else if c == b'[' {
            v.push(Tok::ArrOpen);
            i += 1;
        } else if c == b']' {
            v.push(Tok::ArrClose);
            i += 1;
        } else if c == b'<' && s.get(i + 1) == Some(&b'<') {
            v.push(Tok::DictOpen);
            i += 2;
        } else if c == b'>' && s.get(i + 1) == Some(&b'>') {
            v.push(Tok::DictClose);
            i += 2;
        } else if c == b'<' {
            let j = i + 1 + s[i + 1..].iter().position(|&b| b == b'>').unwrap();
            let hex: Vec<u8> = s[i + 1..j]
                .iter()
                .copied()
                .filter(|b| !b.is_ascii_whitespace())
                .collect();
            let mut out = Vec::new();
            for k in (0..hex.len()).step_by(2) {
                let pair = std::str::from_utf8(&hex[k..(k + 2).min(hex.len())]).unwrap();
                let pair = if pair.len() == 1 {
                    format!("{pair}0")
                } else {
                    pair.to_string()
                };
                out.push(u8::from_str_radix(&pair, 16).unwrap());
            }
            v.push(Tok::Str(out));
            i = j + 1;
        } else if c == b'(' {
            let mut depth = 1;
            let mut out = Vec::new();
            i += 1;
            while depth > 0 {
                let b = s[i];
                match b {
                    b'\\' => {
                        let n = s[i + 1];
                        i += 2;
                        match n {
                            b'n' => out.push(b'\n'),
                            b'r' => out.push(b'\r'),
                            b't' => out.push(b'\t'),
                            b'b' => out.push(8),
                            b'f' => out.push(12),
                            b'0'..=b'7' => {
                                let mut val = (n - b'0') as u32;
                                for _ in 0..2 {
                                    if i < s.len() && (b'0'..=b'7').contains(&s[i]) {
                                        val = val * 8 + (s[i] - b'0') as u32;
                                        i += 1;
                                    }
                                }
                                out.push(val as u8);
                            }
                            b'\n' => {}
                            other => out.push(other),
                        }
                        continue;
                    }
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    out.push(b);
                }
                i += 1;
            }
            v.push(Tok::Str(out));
        } else if c == b'/' {
            let j = i
                + 1
                + s[i + 1..]
                    .iter()
                    .position(|&b| delim(b))
                    .unwrap_or(s.len() - i - 1);
            v.push(Tok::Name(
                String::from_utf8_lossy(&s[i + 1..j]).into_owned(),
            ));
            i = j;
        } else {
            let j = i + s[i..].iter().position(|&b| delim(b)).unwrap_or(s.len() - i);
            let t = std::str::from_utf8(&s[i..j]).unwrap();
            match t.parse::<f64>() {
                Ok(n) => v.push(Tok::Num(n)),
                Err(_) => v.push(Tok::Op(t.to_string())),
            }
            i = j.max(i + 1);
        }
    }
    v
}

/// A parsed pretty-printed PDF: object number → raw bytes between `obj`
/// and `endobj`.
pub struct Pdf {
    objs: HashMap<u32, Vec<u8>>,
}

fn find(h: &[u8], n: &[u8]) -> Option<usize> {
    h.windows(n.len()).position(|w| w == n)
}

impl Pdf {
    pub fn parse(d: &[u8]) -> Pdf {
        let mut objs = HashMap::new();
        let mut i = 0;
        while let Some(p) = find(&d[i..], b" 0 obj\n") {
            let at = i + p;
            let start = d[..at]
                .iter()
                .rposition(|b| !b.is_ascii_digit())
                .map(|x| x + 1)
                .unwrap_or(0);
            let num: u32 = std::str::from_utf8(&d[start..at]).unwrap().parse().unwrap();
            let body = at + 7;
            // Streams may contain "endobj" bytes only if binary; pretty
            // content streams are text, font programs are hex-encoded.
            let end = body + find(&d[body..], b"\nendobj").unwrap();
            objs.insert(num, d[body..end].to_vec());
            i = end + 7;
        }
        Pdf { objs }
    }

    fn dict(&self, n: u32) -> &[u8] {
        let o = &self.objs[&n];
        match find(o, b"\nstream\n") {
            Some(p) => &o[..p],
            None => o,
        }
    }

    fn stream(&self, n: u32) -> &[u8] {
        let o = &self.objs[&n];
        let p = find(o, b"\nstream\n").expect("a stream") + 8;
        let len = self.int_key(n, "Length").expect("/Length") as usize;
        &o[p..p + len]
    }

    /// The tokens following `/key` in object `n`'s dictionary.
    fn key(&self, n: u32, key: &str) -> Vec<Tok> {
        let toks = lex(self.dict(n));
        let at = toks
            .iter()
            .position(|t| *t == Tok::Name(key.into()))
            .unwrap_or_else(|| panic!("/{key} in {n}"));
        toks[at + 1..].to_vec()
    }

    fn int_key(&self, n: u32, key: &str) -> Option<i64> {
        let toks = lex(self.dict(n));
        let at = toks.iter().position(|t| *t == Tok::Name(key.into()))?;
        match toks.get(at + 1) {
            Some(Tok::Num(v)) => Some(*v as i64),
            _ => None,
        }
    }

    fn reference(&self, n: u32, key: &str) -> u32 {
        match &self.key(n, key)[..] {
            [Tok::Num(r), Tok::Num(_), Tok::Op(o), ..] if o == "R" => *r as u32,
            [Tok::ArrOpen, Tok::Num(r), Tok::Num(_), Tok::Op(o), ..] if o == "R" => *r as u32,
            t => panic!(
                "/{key} of {n} is not a reference: {:?}",
                &t[..t.len().min(4)]
            ),
        }
    }

    /// The page objects, in order.
    pub fn pages(&self) -> Vec<u32> {
        let root = *self
            .objs
            .iter()
            .find(|(_, o)| find(o, b"/Type /Pages").is_some() && find(o, b"/Parent").is_none())
            .unwrap()
            .0;
        let toks = self.key(root, "Kids");
        let mut v = Vec::new();
        let mut i = 1;
        while toks[i] != Tok::ArrClose {
            if let Tok::Num(r) = toks[i] {
                v.push(r as u32);
            }
            i += 3;
        }
        v
    }

    pub fn media_box(&self, page: u32) -> [f64; 4] {
        let t = self.key(page, "MediaBox");
        let n: Vec<f64> = t[1..5]
            .iter()
            .map(|x| if let Tok::Num(v) = x { *v } else { panic!() })
            .collect();
        [n[0], n[1], n[2], n[3]]
    }

    /// Resource name → (horizontal widths by CID in thousandths, default).
    fn fonts(&self, page: u32) -> HashMap<String, (HashMap<u32, f64>, f64)> {
        let res = self.reference(page, "Resources");
        let toks = lex(self.dict(res));
        let mut out = HashMap::new();
        let Some(at) = toks.iter().position(|t| *t == Tok::Name("Font".into())) else {
            return out;
        };
        let mut i = at + 2; // skip "<<"
        while toks[i] != Tok::DictClose {
            let Tok::Name(name) = &toks[i] else { panic!() };
            let Tok::Num(r) = toks[i + 1] else { panic!() };
            let desc = self.reference(r as u32, "DescendantFonts");
            let dw = self.int_key(desc, "DW").unwrap_or(1000) as f64;
            let w = self.key(desc, "W");
            let mut widths = HashMap::new();
            let mut j = 1;
            while w[j] != Tok::ArrClose {
                let Tok::Num(first) = w[j] else { panic!() };
                match &w[j + 1] {
                    Tok::ArrOpen => {
                        let mut k = j + 2;
                        let mut c = first as u32;
                        while let Tok::Num(v) = w[k] {
                            widths.insert(c, v);
                            c += 1;
                            k += 1;
                        }
                        j = k + 1;
                    }
                    Tok::Num(last) => {
                        let Tok::Num(v) = w[j + 2] else { panic!() };
                        for c in first as u32..=*last as u32 {
                            widths.insert(c, v);
                        }
                        j += 3;
                    }
                    t => panic!("bad /W entry {t:?}"),
                }
            }
            out.insert(name.clone(), (widths, dw));
            i += 4;
        }
        out
    }

    /// Every glyph's origin on `page`, in painting order, in stream space
    /// (bp, y up), as a viewer computes it.
    pub fn glyph_origins(&self, page: u32) -> Vec<(f64, f64)> {
        let fonts = self.fonts(page);
        let content = self.reference(page, "Contents");
        let toks = lex(self.stream(content));
        type M = [f64; 6];
        let mul = |a: M, b: M| -> M {
            [
                a[0] * b[0] + a[1] * b[2],
                a[0] * b[1] + a[1] * b[3],
                a[2] * b[0] + a[3] * b[2],
                a[2] * b[1] + a[3] * b[3],
                a[4] * b[0] + a[5] * b[2] + b[4],
                a[4] * b[1] + a[5] * b[3] + b[5],
            ]
        };
        let id: M = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut ctm = id;
        let mut stack = vec![];
        let (mut tm, mut tlm) = (id, id);
        let (mut size, mut tc, mut th, mut rise, mut tl) = (0.0, 0.0, 1.0, 0.0, 0.0);
        let mut font: Option<&(HashMap<u32, f64>, f64)> = None;
        let mut out = vec![];
        let mut args: Vec<Tok> = vec![];
        let num = |t: &Tok| {
            if let Tok::Num(v) = t {
                *v
            } else {
                panic!("number expected, got {t:?}")
            }
        };
        let mut i = 0;
        while i < toks.len() {
            let t = toks[i].clone();
            i += 1;
            let Tok::Op(op) = &t else {
                if t == Tok::DictOpen {
                    // An inline property dictionary (BDC): skip it whole.
                    let mut depth = 1;
                    while depth > 0 {
                        match toks[i] {
                            Tok::DictOpen => depth += 1,
                            Tok::DictClose => depth -= 1,
                            _ => {}
                        }
                        i += 1;
                    }
                    args.push(Tok::Name("dict".into()));
                } else {
                    args.push(t);
                }
                continue;
            };
            let a: Vec<f64> = args
                .iter()
                .filter_map(|t| if let Tok::Num(v) = t { Some(*v) } else { None })
                .collect();
            let show = |s: &[u8], tm: &mut M, out: &mut Vec<(f64, f64)>| {
                let (w, dw) = font.expect("a font is set");
                for c in s.chunks(2) {
                    let cid = u16::from_be_bytes([c[0], c[1]]) as u32;
                    let trm = mul([size * th, 0.0, 0.0, size, 0.0, rise], mul(*tm, ctm));
                    out.push((trm[4], trm[5]));
                    let w0 = w.get(&cid).copied().unwrap_or(*dw) / 1000.0;
                    let tx = (w0 * size + tc) * th;
                    *tm = mul([1.0, 0.0, 0.0, 1.0, tx, 0.0], *tm);
                }
            };
            match op.as_str() {
                "q" => stack.push(ctm),
                "Q" => ctm = stack.pop().unwrap(),
                "cm" => ctm = mul([a[0], a[1], a[2], a[3], a[4], a[5]], ctm),
                "BT" => {
                    tm = id;
                    tlm = id;
                }
                "ET" => {}
                "Tf" => {
                    let Tok::Name(n) = &args[0] else { panic!() };
                    font = Some(&fonts[n]);
                    size = a[0];
                }
                "Tm" => {
                    tm = [a[0], a[1], a[2], a[3], a[4], a[5]];
                    tlm = tm;
                }
                "Td" => {
                    tlm = mul([1.0, 0.0, 0.0, 1.0, a[0], a[1]], tlm);
                    tm = tlm;
                }
                "TD" => {
                    tl = -a[1];
                    tlm = mul([1.0, 0.0, 0.0, 1.0, a[0], a[1]], tlm);
                    tm = tlm;
                }
                "T*" => {
                    tlm = mul([1.0, 0.0, 0.0, 1.0, 0.0, -tl], tlm);
                    tm = tlm;
                }
                "TL" => tl = a[0],
                "Tc" => tc = a[0],
                "Tz" => th = a[0] / 100.0,
                "Ts" => rise = a[0],
                "Tw" | "Tr" => {}
                "Tj" => {
                    let Tok::Str(s) = &args[0] else { panic!() };
                    show(s, &mut tm, &mut out);
                }
                "TJ" => {
                    for e in &args {
                        match e {
                            Tok::Str(s) => show(s, &mut tm, &mut out),
                            Tok::Num(n) => {
                                let tx = -n / 1000.0 * size * th;
                                tm = mul([1.0, 0.0, 0.0, 1.0, tx, 0.0], tm);
                            }
                            _ => {}
                        }
                    }
                }
                "'" | "\"" => {
                    panic!("text operator {op} is not written by typst-pdf; extend the checker")
                }
                "Do" => panic!("XObject on a text page: extend the checker"),
                _ => {}
            }
            let _ = num;
            args.clear();
        }
        out
    }
}
