//! The positions checker (DESIGN.md §15.10, T1's correctness gate): where
//! typst-pdf's own export puts every glyph, as the reference viewer
//! computes it (display-list-v3 spec §4.2, §11.2), written independently of
//! the host's `src/pdf.rs` and `src/pdfpos.rs`:
//!
//! * its input is the **whole document's default export** (compressed,
//!   tagged: the bytes of `DONE.pdf`), not the host's untagged one-page
//!   exports;
//! * objects are found by a forward scan that steps over each stream by its
//!   `/Length`, not through the cross-reference table;
//! * streams are inflated with `flate2`, not `miniz_oxide`'s direct API;
//! * an operand is read by splitting its text at the point and multiplying
//!   the digits by `"1e-k"` parsed as a double (the spec's rule), a `TJ`
//!   adjustment or width by parsing the text whole.
//!
//! The host's output must equal this bit for bit: every glyph origin, every
//! glyph matrix and the page box.
#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Read;

/// One glyph: origin (X, Y) in stream space and the glyph matrix [a b c d],
/// and the fill colour's components (as the PDF writes them, read as §4.2
/// reads an operand) and fill alpha it is painted with.
#[derive(Clone, Debug, PartialEq)]
pub struct RefGlyph {
    pub origin: [f64; 2],
    pub matrix: [f64; 4],
    pub fill: Vec<f64>,
    pub fill_alpha: f64,
    /// The full paint state: colour spaces ([`space_key`]), the stroke
    /// colour and `CA`, the text render mode `Tr`, and the line state in
    /// stream space (user-space `w`, `d` times the CTM's similarity scale;
    /// `None` when the CTM is not a similarity).
    pub fill_space: String,
    pub stroke: Vec<f64>,
    pub stroke_space: String,
    pub stroke_alpha: f64,
    pub render: u8,
    pub line: Option<(f64, u8, u8, f64, Vec<f64>, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefPage {
    pub media_box: [f64; 4],
    pub glyphs: Vec<RefGlyph>,
    pub paths: Vec<RefPath>,
}

/// A painted or clipping path: paint bits as the display list's
/// (1 fill, 2 even-odd fill, 4 stroke, 8 clip, 16 even-odd clip), the CTM,
/// the line state `(width, cap, join, miter, dash, phase)` when stroked, and
/// the segments as `(op, numbers)`: 0 move, 1 line, 2 curve, 3 close.
#[derive(Clone, Debug, PartialEq)]
pub struct RefPath {
    pub paint: u8,
    pub ctm: [f64; 6],
    pub line: Option<(f64, u8, u8, f64, Vec<f64>, f64)>,
    pub segs: Vec<(u8, Vec<f64>)>,
    /// The fill and stroke colours' components it is painted with.
    pub fill: Vec<f64>,
    pub stroke: Vec<f64>,
    /// Their spaces ([`space_key`]) and the alphas (`ca`, `CA`).
    pub fill_space: String,
    pub stroke_space: String,
    pub fill_alpha: f64,
    pub stroke_alpha: f64,
}

/// A colour space as text, so that the host's and the PDF's compare:
/// `G`, `RGB`, `CMYK`, `ICC:<n>:<SHA-256 of the profile>`,
/// `Sep:<name>:<alternate>:<C0>:<C1>:<N>`, `Pattern`.
pub fn icc_key(n: u8, profile: &[u8]) -> String {
    format!(
        "ICC:{n}:{}",
        flashtex_display_list::sha256::hex(&flashtex_display_list::sha256::sha256(profile))
    )
}

pub fn sep_key(name: &str, alt: &str, c0: &[f64], c1: &[f64], e: f64) -> String {
    format!("Sep:{name}:{alt}:{c0:?}:{c1:?}:{e:?}")
}

fn space_key(o: &Objs, v: &V) -> String {
    match o.val(v) {
        V::Name(n) => match n.as_str() {
            "DeviceGray" | "G" => "G".into(),
            "DeviceRGB" | "RGB" => "RGB".into(),
            "DeviceCMYK" | "CMYK" => "CMYK".into(),
            "Pattern" => "Pattern".into(),
            n => format!("Other:{n}"),
        },
        V::Arr(a) => match &a[0] {
            V::Name(k) if k == "ICCBased" => {
                let V::Ref(r) = &a[1] else { panic!() };
                let (d, data) = &o.objs[r];
                let n: u8 = d.get("N").unwrap().num().parse().unwrap();
                icc_key(n, data.as_ref().unwrap())
            }
            V::Name(k) if k == "Separation" => {
                let V::Name(name) = &a[1] else { panic!() };
                let alt = space_key(o, &a[2]);
                let f = o.val(&a[3]);
                let arr = |k: &str, d: f64| match f.get(k).map(|v| o.val(v)) {
                    Some(V::Arr(x)) => x.iter().map(|v| operand(v.num())).collect(),
                    _ => vec![d],
                };
                let e = operand(o.val(f.get("N").unwrap()).num());
                sep_key(name, &alt, &arr("C0", 0.0), &arr("C1", 1.0), e)
            }
            k => format!("Other:{k:?}"),
        },
        v => format!("Other:{v:?}"),
    }
}

/// A line state: width, cap, join, miter limit, dash array, phase.
pub type Line = (f64, u8, u8, f64, Vec<f64>, f64);

/// The scale of a similarity CTM `[a b -b a]` or `[a b b -a]`.
fn pen_scale(m: [f64; 6]) -> Option<f64> {
    let [a, b, c, d, _, _] = m;
    ((a == d && b == -c) || (a == -d && b == c)).then(|| (a * a + b * b).sqrt())
}

/// A number token: its text.
#[derive(Clone, Debug, PartialEq)]
enum T {
    Num(String),
    Name(String),
    Str(Vec<u8>),
    Open,
    Close,
    DOpen,
    DClose,
    Word(String),
}

/// The viewer's operand: digits × nearest(10^-k), trailing zeros dropped.
fn operand(s: &str) -> f64 {
    let (neg, body) = match s.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    let frac = frac.trim_end_matches('0');
    let digits = format!("{int}{frac}");
    let digits = digits.trim_start_matches('0');
    let k = frac.len();
    let m: u128 = if digits.is_empty() {
        0
    } else {
        match digits.parse() {
            Ok(m) => m,
            Err(_) => return s.parse().unwrap(),
        }
    };
    if k > 12 || m > (1u128 << 53) {
        return s.parse().unwrap();
    }
    let scale: f64 = format!("1e-{k}").parse().unwrap();
    let v = m as f64 * scale;
    if neg {
        -v
    } else {
        v
    }
}

fn nearest(s: &str) -> f64 {
    let s = s.strip_prefix('+').unwrap_or(s);
    let s = if s.starts_with('.') {
        format!("0{s}")
    } else if s.starts_with("-.") {
        format!("-0{}", &s[1..])
    } else if s.ends_with('.') {
        format!("{s}0")
    } else {
        s.to_string()
    };
    s.parse().unwrap_or_else(|_| panic!("number {s:?}"))
}

fn tokens(b: &[u8]) -> Vec<T> {
    let mut v = Vec::new();
    let mut i = 0;
    let ws = |c: u8| b" \n\r\t\x0c\0".contains(&c);
    let delim = |c: u8| b"()<>[]{}/%".contains(&c) || b" \n\r\t\x0c\0".contains(&c);
    while i < b.len() {
        let c = b[i];
        if ws(c) {
            i += 1;
        } else if c == b'%' {
            while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
                i += 1;
            }
        } else if c == b'[' {
            v.push(T::Open);
            i += 1;
        } else if c == b']' {
            v.push(T::Close);
            i += 1;
        } else if b[i..].starts_with(b"<<") {
            v.push(T::DOpen);
            i += 2;
        } else if b[i..].starts_with(b">>") {
            v.push(T::DClose);
            i += 2;
        } else if c == b'<' {
            let j = i + b[i..].iter().position(|&x| x == b'>').unwrap();
            let hex: String = b[i + 1..j]
                .iter()
                .filter(|x| !ws(**x))
                .map(|&x| x as char)
                .collect();
            let hex = if hex.len() % 2 == 1 {
                format!("{hex}0")
            } else {
                hex
            };
            v.push(T::Str(
                (0..hex.len())
                    .step_by(2)
                    .map(|k| u8::from_str_radix(&hex[k..k + 2], 16).unwrap())
                    .collect(),
            ));
            i = j + 1;
        } else if c == b'(' {
            let mut depth = 0;
            let mut out = Vec::new();
            loop {
                let x = b[i];
                i += 1;
                match x {
                    b'(' => {
                        depth += 1;
                        if depth > 1 {
                            out.push(x);
                        }
                    }
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        out.push(x);
                    }
                    b'\\' => {
                        let n = b[i];
                        i += 1;
                        match n {
                            b'n' => out.push(b'\n'),
                            b'r' => out.push(b'\r'),
                            b't' => out.push(b'\t'),
                            b'b' => out.push(8),
                            b'f' => out.push(12),
                            b'0'..=b'7' => {
                                let mut val = (n - b'0') as u32;
                                let mut k = 0;
                                while k < 2 && i < b.len() && (b'0'..=b'7').contains(&b[i]) {
                                    val = val * 8 + (b[i] - b'0') as u32;
                                    i += 1;
                                    k += 1;
                                }
                                out.push(val as u8);
                            }
                            b'\n' => {}
                            b'\r' => {
                                if b.get(i) == Some(&b'\n') {
                                    i += 1;
                                }
                            }
                            o => out.push(o),
                        }
                    }
                    _ => out.push(x),
                }
            }
            v.push(T::Str(out));
        } else if c == b'/' {
            let j = i
                + 1
                + b[i + 1..]
                    .iter()
                    .position(|&x| delim(x))
                    .unwrap_or(b.len() - i - 1);
            // `#xx` is the byte xx (PDF 32000-1 §7.3.5).
            let raw = &b[i + 1..j];
            let mut name = Vec::with_capacity(raw.len());
            let mut k = 0;
            while k < raw.len() {
                if raw[k] == b'#' && k + 3 <= raw.len() {
                    if let Ok(x) = u8::from_str_radix(
                        std::str::from_utf8(&raw[k + 1..k + 3]).unwrap_or("zz"),
                        16,
                    ) {
                        name.push(x);
                        k += 3;
                        continue;
                    }
                }
                name.push(raw[k]);
                k += 1;
            }
            v.push(T::Name(String::from_utf8_lossy(&name).into_owned()));
            i = j;
        } else {
            let j = i + b[i..].iter().position(|&x| delim(x)).unwrap_or(b.len() - i);
            let t = String::from_utf8_lossy(&b[i..j]).into_owned();
            i = j.max(i + 1);
            if t.starts_with(|ch: char| ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.')
            {
                v.push(T::Num(t));
            } else {
                v.push(T::Word(t));
            }
        }
    }
    v
}

/// A value: nested arrays and dictionaries kept as token slices.
#[derive(Clone, Debug)]
enum V {
    Num(String),
    Name(String),
    Ref(u32),
    Arr(Vec<V>),
    Dict(Vec<(String, V)>),
    Other,
}

fn value(t: &[T], i: &mut usize) -> V {
    let x = t[*i].clone();
    *i += 1;
    match x {
        T::Num(n) => {
            if let (Some(T::Num(_)), Some(T::Word(r))) = (t.get(*i), t.get(*i + 1)) {
                if r == "R" {
                    *i += 2;
                    return V::Ref(n.parse().unwrap());
                }
            }
            V::Num(n)
        }
        T::Name(n) => V::Name(n),
        T::Open => {
            let mut a = Vec::new();
            while t[*i] != T::Close {
                a.push(value(t, i));
            }
            *i += 1;
            V::Arr(a)
        }
        T::DOpen => {
            let mut d = Vec::new();
            while t[*i] != T::DClose {
                let T::Name(k) = t[*i].clone() else {
                    panic!("dictionary key expected, got {:?}", t[*i])
                };
                *i += 1;
                d.push((k, value(t, i)));
            }
            *i += 1;
            V::Dict(d)
        }
        _ => V::Other,
    }
}

impl V {
    fn get(&self, k: &str) -> Option<&V> {
        match self {
            V::Dict(d) => d.iter().find(|(x, _)| x == k).map(|(_, v)| v),
            _ => None,
        }
    }
    fn num(&self) -> &str {
        match self {
            V::Num(n) => n,
            v => panic!("number expected, got {v:?}"),
        }
    }
}

/// The objects of a PDF: number → (value, stream bytes).
struct Objs {
    objs: HashMap<u32, (V, Option<Vec<u8>>)>,
}

fn find(h: &[u8], n: &[u8], from: usize) -> Option<usize> {
    h[from..]
        .windows(n.len())
        .position(|w| w == n)
        .map(|p| p + from)
}

impl Objs {
    fn scan(d: &[u8]) -> Objs {
        let mut objs = HashMap::new();
        let mut at = 0;
        while let Some(p) = find(d, b" 0 obj", at) {
            let start = d[..p]
                .iter()
                .rposition(|b| !b.is_ascii_digit())
                .map(|x| x + 1)
                .unwrap_or(0);
            let num: u32 = std::str::from_utf8(&d[start..p]).unwrap().parse().unwrap();
            let body = p + b" 0 obj".len();
            // The object's text up to `stream` or `endobj`, whichever first.
            let endobj = find(d, b"endobj", body).unwrap();
            let stream_kw = find(d, b"stream", body).filter(|&s| s < endobj);
            let head_end = stream_kw.unwrap_or(endobj);
            let toks = tokens(&d[body..head_end]);
            let mut i = 0;
            let v = value(&toks, &mut i);
            let (stream, next) = match stream_kw {
                Some(s) => {
                    let mut s0 = s + b"stream".len();
                    if d[s0] == b'\r' {
                        s0 += 1;
                    }
                    if d[s0] == b'\n' {
                        s0 += 1;
                    }
                    let len: usize = v.get("Length").unwrap().num().parse().unwrap();
                    let raw = d[s0..s0 + len].to_vec();
                    let data = match v.get("Filter") {
                        Some(V::Name(f)) if f == "FlateDecode" => {
                            let mut out = Vec::new();
                            flate2::read::ZlibDecoder::new(&raw[..])
                                .read_to_end(&mut out)
                                .unwrap();
                            out
                        }
                        None => raw,
                        // Images: not read here (a content stream is Flate).
                        _ => Vec::new(),
                    };
                    let e = find(d, b"endobj", s0 + len).unwrap();
                    (Some(data), e + 6)
                }
                None => (None, endobj + 6),
            };
            objs.insert(num, (v, stream));
            at = next;
        }
        Objs { objs }
    }

    fn val(&self, v: &V) -> V {
        match v {
            V::Ref(r) => self.objs[r].0.clone(),
            v => v.clone(),
        }
    }
}

pub fn reference(pdf: &[u8]) -> Vec<RefPage> {
    let o = Objs::scan(pdf);
    let catalog = o
        .objs
        .values()
        .find(|(v, _)| matches!(v.get("Type"), Some(V::Name(t)) if t == "Catalog"))
        .unwrap()
        .0
        .clone();
    let mut pages = Vec::new();
    collect_pages(&o, catalog.get("Pages").unwrap(), &mut pages);
    pages.iter().map(|p| page(&o, p)).collect()
}

fn collect_pages(o: &Objs, node: &V, out: &mut Vec<V>) {
    let d = o.val(node);
    match d.get("Type") {
        Some(V::Name(t)) if t == "Pages" => {
            let V::Arr(kids) = o.val(d.get("Kids").unwrap()) else {
                panic!()
            };
            for k in &kids {
                collect_pages(o, k, out);
            }
        }
        _ => out.push(d),
    }
}

#[derive(Clone)]
struct St {
    ctm: [f64; 6],
    font: Option<u32>,
    fs: f64,
    tc: f64,
    tw: f64,
    tz: f64,
    tl: f64,
    rise: f64,
    line: (f64, u8, u8, f64, Vec<f64>, f64),
    fill: Vec<f64>,
    stroke: Vec<f64>,
    ca: f64,
    fill_space: String,
    stroke_space: String,
    stroke_ca: f64,
    tr: u8,
}

fn mul(m: [f64; 6], n: [f64; 6]) -> [f64; 6] {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

/// Code width model of a font.
enum Fw {
    Cid(HashMap<u32, f64>, f64),
    T3(u32, Vec<f64>),
}

fn font_widths(o: &Objs, r: u32) -> Fw {
    let d = &o.objs[&r].0;
    match d.get("Subtype") {
        Some(V::Name(s)) if s == "Type0" => {
            let V::Arr(desc) = o.val(d.get("DescendantFonts").unwrap()) else {
                panic!()
            };
            let cid = o.val(&desc[0]);
            let dw = cid.get("DW").map(|v| nearest(v.num())).unwrap_or(1000.0);
            let mut w = HashMap::new();
            if let Some(wv) = cid.get("W") {
                let V::Arr(a) = o.val(wv) else { panic!() };
                let mut i = 0;
                while i < a.len() {
                    let first: u32 = a[i].num().parse().unwrap();
                    match &a[i + 1] {
                        V::Arr(ws) => {
                            for (k, x) in ws.iter().enumerate() {
                                w.insert(first + k as u32, nearest(x.num()));
                            }
                            i += 2;
                        }
                        V::Num(last) => {
                            let last: u32 = last.parse().unwrap();
                            let x = nearest(a[i + 2].num());
                            for c in first..=last {
                                w.insert(c, x);
                            }
                            i += 3;
                        }
                        v => panic!("/W {v:?}"),
                    }
                }
            }
            Fw::Cid(w, dw)
        }
        Some(V::Name(s)) if s == "Type3" => {
            let V::Arr(fm) = o.val(d.get("FontMatrix").unwrap()) else {
                panic!()
            };
            let a = fm[0].num().to_string();
            let first: u32 = d.get("FirstChar").unwrap().num().parse().unwrap();
            let V::Arr(ws) = o.val(d.get("Widths").unwrap()) else {
                panic!()
            };
            // The double nearest to W · a · 1000, from the exact decimal
            // product (big-decimal by string arithmetic on the digits).
            let ws = ws
                .iter()
                .map(|w| decimal_product_1000(w.num(), &a))
                .collect();
            Fw::T3(first, ws)
        }
        s => panic!("font subtype {s:?}"),
    }
}

fn decimal_product_1000(x: &str, y: &str) -> f64 {
    let parts = |s: &str| -> (bool, u128, i32) {
        let (neg, b) = match s.strip_prefix('-') {
            Some(b) => (true, b),
            None => (false, s),
        };
        let (i, f) = b.split_once('.').unwrap_or((b, ""));
        let m: u128 = format!("{i}{f}")
            .trim_start_matches('0')
            .parse()
            .unwrap_or(0);
        (neg, m, f.len() as i32)
    };
    let (n1, m1, k1) = parts(x);
    let (n2, m2, k2) = parts(y);
    let s = format!(
        "{}{}e{}",
        if n1 != n2 { "-" } else { "" },
        m1 * m2,
        3 - k1 - k2
    );
    s.parse().unwrap()
}

fn page(o: &Objs, p: &V) -> RefPage {
    let V::Arr(mb) = o.val(p.get("MediaBox").unwrap()) else {
        panic!()
    };
    let mb: Vec<f64> = mb.iter().map(|v| operand(v.num())).collect();
    let res = o.val(p.get("Resources").unwrap());
    let content = match p.get("Contents").unwrap() {
        V::Ref(r) => o.objs[r].1.clone().unwrap(),
        v => panic!("contents {v:?}"),
    };
    let mut glyphs = Vec::new();
    let mut paths = Vec::new();
    let st = St {
        ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        font: None,
        fs: 0.0,
        tc: 0.0,
        tw: 0.0,
        tz: 100.0,
        tl: 0.0,
        rise: 0.0,
        line: (1.0, 0, 0, 10.0, vec![], 0.0),
        fill: vec![0.0],
        stroke: vec![0.0],
        ca: 1.0,
        fill_space: "G".into(),
        stroke_space: "G".into(),
        stroke_ca: 1.0,
        tr: 0,
    };
    run(o, &content, &res, st, &mut glyphs, &mut paths);
    RefPage {
        media_box: [mb[0], mb[1], mb[2], mb[3]],
        glyphs,
        paths,
    }
}

fn run(
    o: &Objs,
    content: &[u8],
    res: &V,
    mut st: St,
    out: &mut Vec<RefGlyph>,
    paths: &mut Vec<RefPath>,
) {
    let mut segs: Vec<(u8, Vec<f64>)> = vec![];
    let mut cur = (0.0, 0.0);
    let mut clip = 0u8;
    let fonts = res.get("Font").map(|f| o.val(f));
    let spaces = res.get("ColorSpace").map(|f| o.val(f));
    let states = res.get("ExtGState").map(|f| o.val(f));
    // A colour space's initial colour: zeros of its component count; a
    // Separation's full tint.
    let initial = |name: &str| -> Vec<f64> {
        match name {
            "DeviceGray" => vec![0.0],
            "DeviceRGB" => vec![0.0; 3],
            "DeviceCMYK" => vec![0.0, 0.0, 0.0, 1.0],
            "Pattern" => vec![],
            _ => match spaces.as_ref().and_then(|d| d.get(name)).map(|v| o.val(v)) {
                Some(V::Arr(a)) => match a.first() {
                    Some(V::Name(k)) if k == "ICCBased" => {
                        let n: usize = o.val(&a[1]).get("N").unwrap().num().parse().unwrap();
                        vec![0.0; n]
                    }
                    Some(V::Name(k)) if k == "Separation" => vec![1.0],
                    k => panic!("colour space {k:?}"),
                },
                v => panic!("colour space /{name}: {v:?}"),
            },
        }
    };
    let mut widths: HashMap<u32, Fw> = HashMap::new();
    let toks = tokens(content);
    let mut stack: Vec<St> = vec![];
    let id = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    let (mut tm, mut tlm) = (id, id);
    let mut args: Vec<T> = vec![];
    let mut i = 0;
    while i < toks.len() {
        let t = toks[i].clone();
        i += 1;
        let op = match t {
            T::Word(w) => w,
            T::DOpen => {
                let mut depth = 1;
                while depth > 0 {
                    match toks[i] {
                        T::DOpen => depth += 1,
                        T::DClose => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }
                args.push(T::Word("dict".into()));
                continue;
            }
            t => {
                args.push(t);
                continue;
            }
        };
        let nums: Vec<String> = args
            .iter()
            .filter_map(|t| {
                if let T::Num(n) = t {
                    Some(n.clone())
                } else {
                    None
                }
            })
            .collect();
        let a = |k: usize| operand(&nums[k]);
        let mut show = |s: &[u8], st: &St, tm: &mut [f64; 6], widths: &mut HashMap<u32, Fw>| {
            let f = st.font.expect("a font");
            let fw = widths.entry(f).or_insert_with(|| font_widths(o, f));
            let codes: Vec<u32> = match fw {
                Fw::Cid(..) => s
                    .chunks(2)
                    .map(|c| ((c[0] as u32) << 8) | c[1] as u32)
                    .collect(),
                Fw::T3(..) => s.iter().map(|&c| c as u32).collect(),
            };
            for c in codes {
                let trm = mul(*tm, st.ctm);
                let th = st.tz / 100.0;
                out.push(RefGlyph {
                    origin: [st.rise * trm[2] + trm[4], st.rise * trm[3] + trm[5]],
                    matrix: [
                        st.fs * th * trm[0],
                        st.fs * th * trm[1],
                        st.fs * trm[2],
                        st.fs * trm[3],
                    ],
                    fill: st.fill.clone(),
                    fill_alpha: st.ca,
                    fill_space: st.fill_space.clone(),
                    stroke: st.stroke.clone(),
                    stroke_space: st.stroke_space.clone(),
                    stroke_alpha: st.stroke_ca,
                    render: st.tr,
                    line: pen_scale(st.ctm).map(|k| {
                        let l = &st.line;
                        (
                            l.0 * k,
                            l.1,
                            l.2,
                            l.3,
                            l.4.iter().map(|d| d * k).collect(),
                            l.5 * k,
                        )
                    }),
                });
                let (w, space) = match fw {
                    Fw::Cid(w, dw) => (*w.get(&c).unwrap_or(dw), false),
                    Fw::T3(first, w) => (w[(c - *first) as usize], c == 32),
                };
                let mut tx = w / 1000.0 * st.fs + st.tc;
                if space {
                    tx += st.tw;
                }
                let tx = tx * st.tz / 100.0;
                *tm = [
                    tm[0],
                    tm[1],
                    tm[2],
                    tm[3],
                    tx * tm[0] + tm[4],
                    tx * tm[1] + tm[5],
                ];
            }
        };
        match op.as_str() {
            "q" => stack.push(st.clone()),
            "m" | "l" => {
                cur = (a(0), a(1));
                segs.push((if op == "m" { 0 } else { 1 }, vec![cur.0, cur.1]));
            }
            "c" => {
                let v: Vec<f64> = (0..6).map(a).collect();
                cur = (v[4], v[5]);
                segs.push((2, v));
            }
            "v" => {
                let v = vec![cur.0, cur.1, a(0), a(1), a(2), a(3)];
                cur = (v[4], v[5]);
                segs.push((2, v));
            }
            "y" => {
                let v = vec![a(0), a(1), a(2), a(3), a(2), a(3)];
                cur = (v[4], v[5]);
                segs.push((2, v));
            }
            "h" => {
                segs.push((3, vec![]));
            }
            "re" => {
                let (x, y, w, h) = (a(0), a(1), a(2), a(3));
                segs.push((0, vec![x, y]));
                segs.push((1, vec![x + w, y]));
                segs.push((1, vec![x + w, y + h]));
                segs.push((1, vec![x, y + h]));
                segs.push((3, vec![]));
                cur = (x, y);
            }
            "W" => clip = 8,
            "W*" => clip = 16,
            "f" | "F" | "f*" | "S" | "s" | "B" | "B*" | "b" | "b*" | "n" => {
                if matches!(op.as_str(), "s" | "b" | "b*") {
                    segs.push((3, vec![]));
                }
                let bits = match op.as_str() {
                    "f" | "F" => 1,
                    "f*" => 2,
                    "S" | "s" => 4,
                    "B" | "b" => 5,
                    "B*" | "b*" => 6,
                    _ => 0,
                } | clip;
                if bits != 0 {
                    paths.push(RefPath {
                        paint: bits,
                        ctm: st.ctm,
                        line: (bits & 4 != 0).then(|| st.line.clone()),
                        segs: std::mem::take(&mut segs),
                        fill: st.fill.clone(),
                        stroke: st.stroke.clone(),
                        fill_space: st.fill_space.clone(),
                        stroke_space: st.stroke_space.clone(),
                        fill_alpha: st.ca,
                        stroke_alpha: st.stroke_ca,
                    });
                }
                segs.clear();
                clip = 0;
            }
            "w" => st.line.0 = a(0),
            "J" => st.line.1 = nums[0].parse().unwrap(),
            "j" => st.line.2 = nums[0].parse().unwrap(),
            "M" => st.line.3 = a(0),
            "d" => {
                let v: Vec<f64> = (0..nums.len()).map(a).collect();
                st.line.5 = *v.last().unwrap();
                st.line.4 = v[..v.len() - 1].to_vec();
            }
            "Q" => st = stack.pop().unwrap(),
            "cm" => st.ctm = mul([a(0), a(1), a(2), a(3), a(4), a(5)], st.ctm),
            "BT" => {
                tm = id;
                tlm = id;
            }
            "Tf" => {
                let T::Name(n) = &args[0] else { panic!() };
                let Some(V::Ref(r)) = fonts.as_ref().and_then(|f| f.get(n)) else {
                    panic!("font {n}")
                };
                st.font = Some(*r);
                st.fs = a(0);
            }
            "Tc" => st.tc = a(0),
            "Tr" => st.tr = nums[0].parse().unwrap(),
            "cs" | "CS" => {
                let T::Name(n) = &args[0] else { panic!() };
                let c = initial(n);
                let key = match n.as_str() {
                    "DeviceGray" | "DeviceRGB" | "DeviceCMYK" | "Pattern" => {
                        space_key(o, &V::Name(n.clone()))
                    }
                    _ => space_key(o, spaces.as_ref().unwrap().get(n).unwrap()),
                };
                if op == "cs" {
                    st.fill = c;
                    st.fill_space = key;
                } else {
                    st.stroke = c;
                    st.stroke_space = key;
                }
            }
            "sc" | "scn" | "SC" | "SCN" => {
                let c: Vec<f64> = (0..nums.len()).map(a).collect();
                if op.starts_with('s') {
                    st.fill = c;
                } else {
                    st.stroke = c;
                }
            }
            "g" | "rg" | "k" | "G" | "RG" | "K" => {
                let c: Vec<f64> = (0..nums.len()).map(a).collect();
                let key = match op.to_lowercase().as_str() {
                    "g" => "G",
                    "rg" => "RGB",
                    _ => "CMYK",
                };
                if op.chars().next().unwrap().is_lowercase() {
                    st.fill = c;
                    st.fill_space = key.into();
                } else {
                    st.stroke = c;
                    st.stroke_space = key.into();
                }
            }
            "gs" => {
                let T::Name(n) = &args[0] else { panic!() };
                let d = o.val(
                    states
                        .as_ref()
                        .and_then(|s| s.get(n))
                        .expect("an ExtGState"),
                );
                if let Some(v) = d.get("ca") {
                    st.ca = operand(v.num());
                }
                if let Some(v) = d.get("CA") {
                    st.stroke_ca = operand(v.num());
                }
                if let Some(v) = d.get("LW") {
                    st.line.0 = operand(v.num());
                }
            }
            "Tw" => st.tw = a(0),
            "Tz" => st.tz = a(0),
            "TL" => st.tl = a(0),
            "Ts" => st.rise = a(0),
            "Tm" => {
                tm = [a(0), a(1), a(2), a(3), a(4), a(5)];
                tlm = tm;
            }
            "Td" | "TD" => {
                let (x, y) = (a(0), a(1));
                if op == "TD" {
                    st.tl = -y;
                }
                tlm = [
                    tlm[0],
                    tlm[1],
                    tlm[2],
                    tlm[3],
                    x * tlm[0] + y * tlm[2] + tlm[4],
                    x * tlm[1] + y * tlm[3] + tlm[5],
                ];
                tm = tlm;
            }
            "T*" | "'" | "\"" => {
                if op == "\"" {
                    st.tw = a(0);
                    st.tc = a(1);
                }
                let y = -st.tl;
                tlm = [
                    tlm[0],
                    tlm[1],
                    tlm[2],
                    tlm[3],
                    y * tlm[2] + tlm[4],
                    y * tlm[3] + tlm[5],
                ];
                tm = tlm;
                if op != "T*" {
                    let Some(T::Str(s)) = args.last() else {
                        panic!()
                    };
                    show(s, &st, &mut tm, &mut widths);
                }
            }
            "Tj" => {
                let Some(T::Str(s)) = args.last() else {
                    panic!()
                };
                show(s, &st, &mut tm, &mut widths);
            }
            "TJ" => {
                for e in &args {
                    match e {
                        T::Str(s) => show(s, &st, &mut tm, &mut widths),
                        T::Num(n) => {
                            let tx = -nearest(n) / 1000.0 * st.fs;
                            let tx = tx * st.tz / 100.0;
                            tm = [
                                tm[0],
                                tm[1],
                                tm[2],
                                tm[3],
                                tx * tm[0] + tm[4],
                                tx * tm[1] + tm[5],
                            ];
                        }
                        _ => {}
                    }
                }
            }
            // XObjects are images (raster, SVG, PDF): their glyphs are the
            // image's own, not the page's text (spec §11.5).
            "Do" => {}
            _ => {}
        }
        args.clear();
    }
}

/// The display list's paths and clips, in item order, as [`RefPath`]s.
pub fn host_paths(p: &flashtex_display_list::page::Page) -> Vec<RefPath> {
    use flashtex_display_list::page::{Item, Seg};
    p.items
        .iter()
        .filter_map(|it| match it {
            Item::Path(n) | Item::Clip(n) => Some(&p.paths[*n as usize]),
            _ => None,
        })
        .map(|path| RefPath {
            paint: path.paint,
            ctm: p.matrix(path.matrix),
            line: path
                .stroke
                .as_ref()
                .map(|s| (s.width, s.cap, s.join, s.miter, s.dash.clone(), s.phase)),
            segs: path
                .segs
                .iter()
                .map(|s| match s {
                    Seg::Move(x, y) => (0, vec![*x, *y]),
                    Seg::Line(x, y) => (1, vec![*x, *y]),
                    Seg::Curve(a, b, c, d, e, f) => (2, vec![*a, *b, *c, *d, *e, *f]),
                    Seg::Close => (3, vec![]),
                })
                .collect(),
            fill: vec![],
            stroke: vec![],
            fill_space: String::new(),
            stroke_space: String::new(),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
        })
        .zip(host_paints(p).1)
        .map(|(mut r, h)| {
            r.fill = h.fill;
            r.stroke = h.stroke;
            r.fill_space = h.fill_space;
            r.stroke_space = h.stroke_space;
            r.fill_alpha = h.fill_alpha;
            r.stroke_alpha = h.stroke_alpha;
            r
        })
        .collect()
}

/// The fill colour's components and fill alpha in effect at each GLYPH,
/// and the fill and stroke colours at each PATH or CLIP, in item order
/// (SAVE and RESTORE scope them; FILL_COLOR and FILL_COLOR_CS alike).
#[allow(clippy::type_complexity)]
pub fn host_colors(
    p: &flashtex_display_list::page::Page,
) -> (Vec<(Vec<f64>, f64)>, Vec<(Vec<f64>, Vec<f64>)>) {
    use flashtex_display_list::page::Item;
    let mut st = (vec![0.0], vec![0.0], 1.0);
    let mut stack = vec![];
    let (mut glyphs, mut paths) = (vec![], vec![]);
    for it in &p.items {
        match it {
            Item::Save => stack.push(st.clone()),
            Item::Restore => st = stack.pop().unwrap(),
            Item::FillColor(c) | Item::FillColorCs { color: c, .. } => st.0 = c.0.clone(),
            Item::StrokeColor(c) | Item::StrokeColorCs { color: c, .. } => st.1 = c.0.clone(),
            Item::FillAlpha(a) => st.2 = *a,
            Item::Glyph { .. } => glyphs.push((st.0.clone(), st.2)),
            Item::Path(_) | Item::Clip(_) => paths.push((st.0.clone(), st.1.clone())),
            _ => {}
        }
    }
    (glyphs, paths)
}

/// The paint state of each host GLYPH, in item order, in [`RefGlyph`]'s
/// terms (origin and matrix left zero).
pub fn host_glyph_paints(p: &flashtex_display_list::page::Page) -> Vec<RefGlyph> {
    host_paints(p).0
}

/// A host colour space as [`space_key`] writes the PDF's.
fn host_space_key(p: &flashtex_display_list::page::Page, cs: u32) -> String {
    use flashtex_display_list::page::{alternate, ColorSpace};
    match cs {
        alternate::DEVICE_GRAY => return "G".into(),
        alternate::DEVICE_RGB => return "RGB".into(),
        alternate::DEVICE_CMYK => return "CMYK".into(),
        _ => {}
    }
    match p.colorspaces.get((cs as usize).wrapping_sub(1)) {
        Some(ColorSpace::Icc { n, profile }) => icc_key(*n, profile),
        Some(ColorSpace::Separation {
            name,
            alternate,
            c0,
            c1,
            e,
        }) => sep_key(name, &host_space_key(p, *alternate), c0, c1, *e),
        None => format!("Other:cs{cs}"),
    }
}

/// Every GLYPH's and PATH/CLIP's full paint state as the display list
/// sets it (SAVE and RESTORE scope it).
fn host_paints(p: &flashtex_display_list::page::Page) -> (Vec<RefGlyph>, Vec<RefPath>) {
    use flashtex_display_list::page::Item;
    let dev = |c: &[f64]| match c.len() {
        1 => "G".to_string(),
        3 => "RGB".into(),
        4 => "CMYK".into(),
        n => format!("Other:{n}"),
    };
    #[derive(Clone)]
    struct H {
        fill: (Vec<f64>, String),
        stroke: (Vec<f64>, String),
        ca: f64,
        sca: f64,
        tr: u8,
        line: Option<(f64, u8, u8, f64, Vec<f64>, f64)>,
    }
    let mut st = H {
        fill: (vec![0.0], "G".into()),
        stroke: (vec![0.0], "G".into()),
        ca: 1.0,
        sca: 1.0,
        tr: 0,
        line: None,
    };
    let mut stack = vec![];
    let (mut glyphs, mut paths) = (vec![], vec![]);
    for it in &p.items {
        match it {
            Item::Save => stack.push(st.clone()),
            Item::Restore => st = stack.pop().unwrap(),
            Item::FillColor(c) => st.fill = (c.0.clone(), dev(&c.0)),
            Item::FillColorCs { cs, color } => st.fill = (color.0.clone(), host_space_key(p, *cs)),
            Item::StrokeColor(c) => st.stroke = (c.0.clone(), dev(&c.0)),
            Item::StrokeColorCs { cs, color } => {
                st.stroke = (color.0.clone(), host_space_key(p, *cs))
            }
            Item::FillAlpha(a) => st.ca = *a,
            Item::StrokeAlpha(a) => st.sca = *a,
            Item::TextRender(r) => st.tr = *r,
            Item::LineState(l) => {
                st.line = Some((l.width, l.cap, l.join, l.miter, l.dash.clone(), l.phase))
            }
            Item::Glyph { .. } => glyphs.push(RefGlyph {
                origin: [0.0; 2],
                matrix: [0.0; 4],
                fill: st.fill.0.clone(),
                fill_alpha: st.ca,
                fill_space: st.fill.1.clone(),
                stroke: st.stroke.0.clone(),
                stroke_space: st.stroke.1.clone(),
                stroke_alpha: st.sca,
                render: st.tr,
                line: st.line.clone(),
            }),
            Item::Path(_) | Item::Clip(_) => paths.push(RefPath {
                paint: 0,
                ctm: [0.0; 6],
                line: None,
                segs: vec![],
                fill: st.fill.0.clone(),
                stroke: st.stroke.0.clone(),
                fill_space: st.fill.1.clone(),
                stroke_space: st.stroke.1.clone(),
                fill_alpha: st.ca,
                stroke_alpha: st.sca,
            }),
            _ => {}
        }
    }
    (glyphs, paths)
}

/// What differs between a host glyph's paint and the PDF's (spec §11.3,
/// §11.4): the fill colour, its space and `ca` when the glyph is filled;
/// the stroke colour, its space, `CA` and the line state in stream space
/// when it is stroked; the render mode. `None` when equal bit for bit.
pub fn glyph_paint_mismatch(h: &RefGlyph, r: &RefGlyph) -> Option<String> {
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    if h.render != r.render {
        return Some(format!("Tr {} vs the PDF's {}", h.render, r.render));
    }
    let fills = matches!(r.render, 0 | 2 | 4 | 6);
    let strokes = matches!(r.render, 1 | 2 | 5 | 6);
    if fills
        && (bits(&h.fill) != bits(&r.fill)
            || h.fill_space != r.fill_space
            || h.fill_alpha.to_bits() != r.fill_alpha.to_bits())
    {
        return Some(format!(
            "fill {:?} {} ca {} vs the PDF's {:?} {} ca {}",
            h.fill, h.fill_space, h.fill_alpha, r.fill, r.fill_space, r.fill_alpha
        ));
    }
    if strokes {
        let line_bits = |l: &Option<Line>| {
            l.as_ref().map(|l| {
                (
                    l.0.to_bits(),
                    l.1,
                    l.2,
                    l.3.to_bits(),
                    bits(&l.4),
                    l.5.to_bits(),
                )
            })
        };
        if bits(&h.stroke) != bits(&r.stroke)
            || h.stroke_space != r.stroke_space
            || h.stroke_alpha.to_bits() != r.stroke_alpha.to_bits()
            || r.line.is_none()
            || line_bits(&h.line) != line_bits(&r.line)
        {
            return Some(format!(
                "stroke {:?} {} CA {} line {:?} vs the PDF's {:?} {} CA {} line {:?}",
                h.stroke,
                h.stroke_space,
                h.stroke_alpha,
                h.line,
                r.stroke,
                r.stroke_space,
                r.stroke_alpha,
                r.line
            ));
        }
    }
    None
}

/// The PDF as a client without `color-spaces` gets it drawn: ICCBased
/// colours in the Device space of their component count, no alpha.
pub fn device_only(pages: &mut [RefPage]) {
    let dev = |k: &mut String| {
        if let Some(rest) = k.strip_prefix("ICC:") {
            *k = match rest.split(':').next() {
                Some("1") => "G",
                Some("3") => "RGB",
                _ => "CMYK",
            }
            .into();
        }
    };
    for p in pages {
        for g in &mut p.glyphs {
            dev(&mut g.fill_space);
            dev(&mut g.stroke_space);
        }
        for path in &mut p.paths {
            dev(&mut path.fill_space);
            dev(&mut path.stroke_space);
            path.fill_alpha = 1.0;
            path.stroke_alpha = 1.0;
        }
    }
}

/// How many of the host's paths are not, in order, one of the PDF's paths
/// bit for bit (CTM, segments, line state), painted with part of what the
/// PDF paints (the host leaves out a gradient fill or stroke it cannot
/// draw, and flags the page).
pub fn unmatched_paths(host: &[RefPath], pdf: &[RefPath]) -> usize {
    let bits = |v: &[f64]| v.iter().map(|x| x.to_bits()).collect::<Vec<_>>();
    let same = |h: &RefPath, r: &RefPath| {
        let fills = 1 | 2;
        let paint_ok =
            h.paint & !(fills) & !r.paint == 0 && (h.paint & fills == 0 || r.paint & fills != 0);
        let colours_ok = (h.paint & fills == 0
            || (bits(&h.fill) == bits(&r.fill)
                && h.fill_space == r.fill_space
                && h.fill_alpha.to_bits() == r.fill_alpha.to_bits()))
            && (h.paint & 4 == 0
                || (bits(&h.stroke) == bits(&r.stroke)
                    && h.stroke_space == r.stroke_space
                    && h.stroke_alpha.to_bits() == r.stroke_alpha.to_bits()));
        paint_ok
            && colours_ok
            && bits(&h.ctm) == bits(&r.ctm)
            && h.segs.len() == r.segs.len()
            && h.segs
                .iter()
                .zip(&r.segs)
                .all(|(a, b)| a.0 == b.0 && bits(&a.1) == bits(&b.1))
            && (h.paint & 4 == 0
                || match (&h.line, &r.line) {
                    (Some(a), Some(b)) => {
                        a.0.to_bits() == b.0.to_bits()
                            && a.1 == b.1
                            && a.2 == b.2
                            && a.3.to_bits() == b.3.to_bits()
                            && bits(&a.4) == bits(&b.4)
                            && a.5.to_bits() == b.5.to_bits()
                    }
                    _ => false,
                })
    };
    let mut k = 0;
    let mut missing = 0;
    for h in host {
        match (k..pdf.len()).find(|&j| same(h, &pdf[j])) {
            Some(j) => k = j + 1,
            None => missing += 1,
        }
    }
    missing
}

/// What the PDF shows of a frame, in typst-pdf's painting order: text runs
/// (does the host draw it -- a solid process-colour fill -- and where the
/// frame puts each glyph, stream space) and images (an SVG image's text is
/// shown inline, so glyphs that are no run's may follow one).
pub enum Ev {
    Run(bool, Vec<[f64; 2]>),
    Image,
}

pub fn events(
    frame: &typst::layout::Frame,
    ts: typst::layout::Transform,
    h: f64,
    out: &mut Vec<Ev>,
) {
    use typst::layout::{Abs, FrameItem, Point, Transform};
    use typst::visualize::{Color, Paint};
    for (pos, item) in frame.items() {
        let ts = ts.pre_concat(Transform::translate(pos.x, pos.y));
        match item {
            FrameItem::Group(g) => events(&g.frame, ts.pre_concat(g.transform), h, out),
            FrameItem::Text(t) => {
                let d = matches!(t.fill, Paint::Solid(Color::Process(_)));
                let (mut x, mut y) = (Abs::zero(), Abs::zero());
                let mut o = Vec::new();
                for g in &t.glyphs {
                    let p = Point::new(x + g.x_offset.at(t.size), y - g.y_offset.at(t.size))
                        .transform(ts);
                    o.push([p.x.to_pt(), h - p.y.to_pt()]);
                    x += g.x_advance.at(t.size);
                    y -= g.y_advance.at(t.size);
                }
                out.push(Ev::Run(d, o));
            }
            FrameItem::Image(..) => out.push(Ev::Image),
            _ => {}
        }
    }
}

/// The PDF glyphs the host must draw, in order, or why they cannot be
/// matched to the frame's runs.
pub fn expected<'a>(evs: &[Ev], pdf: &'a [RefGlyph]) -> Result<Vec<&'a RefGlyph>, String> {
    let mut k = 0;
    let mut gap = false;
    let mut out = Vec::new();
    for ev in evs {
        match ev {
            Ev::Image => gap = true,
            Ev::Run(drawn, fo) => {
                let n = fo.len();
                let near = |a: [f64; 2], b: [f64; 2]| {
                    (a[0] - b[0]).abs() < 0.01 && (a[1] - b[1]).abs() < 0.01
                };
                let fit = |at: usize| {
                    at + n <= pdf.len() && (0..n).all(|j| near(pdf[at + j].origin, fo[j]))
                };
                let at = if fit(k) {
                    Some(k)
                } else if gap {
                    (k + 1..=pdf.len().saturating_sub(n)).find(|&a| fit(a))
                } else {
                    None
                };
                gap = false;
                let Some(at) = at else {
                    return Err(format!("a run of {n} glyphs is not at PDF glyph {k}"));
                };
                if *drawn {
                    out.extend(&pdf[at..at + n]);
                }
                k = at + n;
            }
        }
    }
    if k != pdf.len() && !gap {
        return Err(format!("{} PDF glyphs after the last run", pdf.len() - k));
    }
    Ok(out)
}
