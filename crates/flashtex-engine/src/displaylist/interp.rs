//! The page's content stream, read as a PDF viewer reads it, into
//! display-list items.
//!
//! The bytes are exactly those pdfTeX's `pdf_hlist_out`/`pdf_vlist_out` wrote
//! for the page (or form), so every position here is the position in the
//! PDF: the text matrix after `Td`/`Tm` and each glyph's advance (the font's
//! `/Widths` entry, which pdfTeX computes as `divide_scaled(char_width,
//! pdf_font_size, 4)` tenths), the `TJ` adjustments, and the `cm`s of rules,
//! images, forms and literals, all in exact decimal arithmetic
//! ([`super::fixed`]), then rounded once to scaled points.
//!
//! What a node of the box contributed is known from the markers the
//! traversal left ([`Marker`]): each item takes the span of the last marker
//! at or before the byte that drew it.

use super::fixed::{Fx, Mat};
use flashtex_display_list::page::{
    flags, paint, Color, Item, Page, Path, RuleKind, Seg, StreamKind, Stroke, NO_COLUMN,
};
use std::collections::HashMap;

/// Where the traversal was in the stream when it output a node.
#[derive(Clone, Copy, Debug)]
pub struct Marker {
    pub offset: u32,
    pub span: u32,
    pub col: u16,
}

/// What the interpreter needs from the engine.
pub trait Env {
    /// The `/Widths` entry of `code` in font `/F<font>`, in tenths of a
    /// glyph-space unit (pdfTeX writes `w/10` with one decimal), or `None`.
    fn width(&mut self, font: u32, code: u8) -> Option<i64>;
    /// The advance of `code` in font `/F<font>` for a font size of 1, as
    /// the fraction `n/d` of text space: the `/Widths` entry times the
    /// font's `/FontMatrix`. By default a font with pdfTeX's `/FontMatrix`
    /// of 0.001 (Type 1, TrueType, OpenType): `(width, 10_000)`; a PK font
    /// written as Type 3 has its own (writet3.rs).
    fn advance(&mut self, font: u32, code: u8) -> Option<(i64, i64)> {
        self.width(font, code).map(|w| (w, 10_000))
    }
    /// Why the glyphs of font `/F<font>` cannot be drawn from the display
    /// list, if they cannot: the page is then flagged incomplete where the
    /// font is first used.
    fn font_problem(&mut self, _font: u32) -> Option<String> {
        None
    }
    /// The resource-name prefix (`\pdfpkresolution`'s `pdf_resname_prefix`).
    fn resname_prefix(&self) -> &[u8];
}

#[derive(Clone, Debug, PartialEq)]
enum Obj {
    Num(Fx),
    Name(Vec<u8>),
    /// A string's bytes and the stream offset of each.
    Str(Vec<u8>, Vec<u32>),
    Arr(Vec<Obj>),
    Dict,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
struct Col {
    /// 1, 3 or 4 components; 0 for an unsupported space.
    n: u8,
    v: Vec<Fx>,
}

impl Col {
    fn black() -> Col {
        Col {
            n: 1,
            v: vec![Fx::ZERO],
        }
    }
    fn wire(&self) -> Color {
        Color(self.v.iter().map(|x| x.to_f64()).collect())
    }
}

#[derive(Clone, Debug)]
struct GState {
    ctm: Mat,
    fill: Col,
    stroke: Col,
    line_width: Fx,
    cap: u8,
    join: u8,
    miter: Fx,
    dash: Vec<Fx>,
    phase: Fx,
    // text state (part of the graphics state in PDF)
    tc: Fx,
    tw: Fx,
    tz: Fx,
    tl: Fx,
    font: Option<u32>,
    fs: Fx,
    tr: u8,
    ts: Fx,
    // what the client has been told (restored with the graphics state)
    told_fill: Col,
    told_stroke: Col,
    told_tr: u8,
}

impl GState {
    fn new(ctm: Mat) -> GState {
        GState {
            ctm,
            fill: Col::black(),
            stroke: Col::black(),
            line_width: Fx::ONE,
            cap: 0,
            join: 0,
            miter: Fx::from_int(10),
            dash: vec![],
            phase: Fx::ZERO,
            tc: Fx::ZERO,
            tw: Fx::ZERO,
            tz: Fx::from_int(100),
            tl: Fx::ZERO,
            font: None,
            fs: Fx::ZERO,
            tr: 0,
            ts: Fx::ZERO,
            told_fill: Col::black(),
            told_stroke: Col::black(),
            told_tr: 0,
        }
    }
}

/// The result: the page's items and tables, and the resources it used.
pub struct Output {
    pub page: Page,
    pub fonts: Vec<u32>,
    pub images: Vec<u32>,
    pub forms: Vec<u32>,
}

struct Interp<'a, E: Env> {
    env: &'a mut E,
    s: &'a [u8],
    i: usize,
    /// Box height in bp: y down = height - y.
    height: Fx,
    markers: &'a [Marker],
    mi: usize,
    page: Page,
    mats: HashMap<[i128; 6], u32>,
    gs: GState,
    stack: Vec<GState>,
    tm: Mat,
    tlm: Mat,
    in_text: bool,
    path: Vec<Seg>,
    /// Path coordinates in Fx, for rule detection.
    fx_path: Vec<(u8, Fx, Fx)>,
    /// The path is exactly one `re`: (x, y, w, h).
    only_re: Option<(Fx, Fx, Fx, Fx)>,
    re_count: u32,
    clip_pending: Option<u8>,
    told_matrix: u32,
    told_span: u32,
    fonts: Vec<u32>,
    images: Vec<u32>,
    forms: Vec<u32>,
    unsupported: HashMap<String, u32>,
}

/// Read `stream` (a page's or form's content, `height` its box height in
/// bp, `ctm` the matrix in force at its start).
pub fn interpret<E: Env>(
    env: &mut E,
    kind: StreamKind,
    index: u32,
    stream: &[u8],
    height: Fx,
    ctm: Mat,
    markers: &[Marker],
) -> Output {
    let mut it = Interp {
        env,
        s: stream,
        i: 0,
        height,
        markers,
        mi: 0,
        page: Page::new(kind, index),
        mats: HashMap::new(),
        gs: GState::new(ctm),
        stack: Vec::new(),
        tm: Mat::IDENTITY,
        tlm: Mat::IDENTITY,
        in_text: false,
        path: Vec::new(),
        fx_path: Vec::new(),
        only_re: None,
        re_count: 0,
        clip_pending: None,
        told_matrix: 0,
        told_span: 0,
        fonts: Vec::new(),
        images: Vec::new(),
        forms: Vec::new(),
        unsupported: HashMap::new(),
    };
    it.run();
    Output {
        page: it.page,
        fonts: it.fonts,
        images: it.images,
        forms: it.forms,
    }
}

fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\n' | b'\r' | b'\t' | b'\x0c' | 0)
}

fn is_delim(c: u8) -> bool {
    matches!(
        c,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

enum Tok {
    Obj(Obj),
    ArrOpen,
    ArrClose,
    DictOpen,
    DictClose,
    Op(Vec<u8>),
}

impl<'a, E: Env> Interp<'a, E> {
    fn run(&mut self) {
        let mut ops: Vec<Obj> = Vec::new();
        // Nested arrays under construction.
        let mut arrs: Vec<Vec<Obj>> = Vec::new();
        let mut dict_depth = 0u32;
        loop {
            let start = self.i;
            let Some(t) = self.token() else { break };
            let obj = match t {
                Tok::Obj(o) => o,
                Tok::ArrOpen => {
                    arrs.push(Vec::new());
                    continue;
                }
                Tok::ArrClose => match arrs.pop() {
                    Some(a) => Obj::Arr(a),
                    None => continue,
                },
                Tok::DictOpen => {
                    dict_depth += 1;
                    continue;
                }
                Tok::DictClose => {
                    dict_depth = dict_depth.saturating_sub(1);
                    if dict_depth == 0 {
                        // A whole dictionary is one operand (BDC's).
                        if let Some(a) = arrs.last_mut() {
                            a.push(Obj::Dict);
                        } else {
                            ops.push(Obj::Dict);
                        }
                    }
                    continue;
                }
                Tok::Op(op) => {
                    if dict_depth > 0 {
                        continue; // `true`, `null` inside a dictionary
                    }
                    if !arrs.is_empty() {
                        arrs.last_mut().unwrap().push(Obj::Other);
                        continue;
                    }
                    self.op(&op, &ops, start as u32);
                    ops.clear();
                    continue;
                }
            };
            if dict_depth > 0 {
                continue;
            }
            if let Some(a) = arrs.last_mut() {
                a.push(obj);
            } else {
                ops.push(obj);
            }
        }
    }

    fn token(&mut self) -> Option<Tok> {
        let s = self.s;
        loop {
            while self.i < s.len() && is_ws(s[self.i]) {
                self.i += 1;
            }
            if self.i >= s.len() {
                return None;
            }
            if s[self.i] == b'%' {
                while self.i < s.len() && s[self.i] != b'\n' && s[self.i] != b'\r' {
                    self.i += 1;
                }
                continue;
            }
            break;
        }
        let c = s[self.i];
        match c {
            b'[' => {
                self.i += 1;
                Some(Tok::ArrOpen)
            }
            b']' => {
                self.i += 1;
                Some(Tok::ArrClose)
            }
            b'<' if s.get(self.i + 1) == Some(&b'<') => {
                self.i += 2;
                Some(Tok::DictOpen)
            }
            b'>' if s.get(self.i + 1) == Some(&b'>') => {
                self.i += 2;
                Some(Tok::DictClose)
            }
            b'<' => {
                self.i += 1;
                let mut bytes = Vec::new();
                let mut offs = Vec::new();
                let mut hi: Option<u8> = None;
                let mut at = self.i as u32;
                while self.i < s.len() && s[self.i] != b'>' {
                    let h = s[self.i];
                    let v = match h {
                        b'0'..=b'9' => Some(h - b'0'),
                        b'a'..=b'f' => Some(h - b'a' + 10),
                        b'A'..=b'F' => Some(h - b'A' + 10),
                        _ => None,
                    };
                    if let Some(v) = v {
                        match hi {
                            None => {
                                hi = Some(v);
                                at = self.i as u32;
                            }
                            Some(x) => {
                                bytes.push(x * 16 + v);
                                offs.push(at);
                                hi = None;
                            }
                        }
                    }
                    self.i += 1;
                }
                if let Some(x) = hi {
                    bytes.push(x * 16);
                    offs.push(at);
                }
                self.i += 1;
                Some(Tok::Obj(Obj::Str(bytes, offs)))
            }
            b'(' => Some(Tok::Obj(self.literal_string())),
            b'/' => {
                self.i += 1;
                let st = self.i;
                while self.i < s.len() && !is_ws(s[self.i]) && !is_delim(s[self.i]) {
                    self.i += 1;
                }
                Some(Tok::Obj(Obj::Name(unescape_name(&s[st..self.i]))))
            }
            b')' | b'>' | b'{' | b'}' => {
                self.i += 1;
                Some(Tok::Obj(Obj::Other))
            }
            _ => {
                let st = self.i;
                while self.i < s.len() && !is_ws(s[self.i]) && !is_delim(s[self.i]) {
                    self.i += 1;
                }
                let w = &s[st..self.i];
                if let Some(v) = Fx::parse(w) {
                    Some(Tok::Obj(Obj::Num(v)))
                } else if w == b"BI" {
                    self.skip_inline_image();
                    Some(Tok::Op(b"BI".to_vec()))
                } else {
                    Some(Tok::Op(w.to_vec()))
                }
            }
        }
    }

    fn literal_string(&mut self) -> Obj {
        let s = self.s;
        self.i += 1; // (
        let mut depth = 1;
        let mut bytes = Vec::new();
        let mut offs = Vec::new();
        while self.i < s.len() {
            let at = self.i as u32;
            let c = s[self.i];
            self.i += 1;
            match c {
                b'(' => {
                    depth += 1;
                    bytes.push(c);
                    offs.push(at);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    bytes.push(c);
                    offs.push(at);
                }
                b'\\' => {
                    let Some(&e) = s.get(self.i) else { break };
                    self.i += 1;
                    let v = match e {
                        b'n' => Some(b'\n'),
                        b'r' => Some(b'\r'),
                        b't' => Some(b'\t'),
                        b'b' => Some(8),
                        b'f' => Some(12),
                        b'\r' => {
                            if s.get(self.i) == Some(&b'\n') {
                                self.i += 1;
                            }
                            None
                        }
                        b'\n' => None,
                        b'0'..=b'7' => {
                            let mut v = (e - b'0') as u32;
                            for _ in 0..2 {
                                match s.get(self.i) {
                                    Some(&d @ b'0'..=b'7') => {
                                        v = v * 8 + (d - b'0') as u32;
                                        self.i += 1;
                                    }
                                    _ => break,
                                }
                            }
                            Some((v & 0xff) as u8)
                        }
                        other => Some(other),
                    };
                    if let Some(v) = v {
                        bytes.push(v);
                        offs.push(at);
                    }
                }
                b'\r' => {
                    // An end of line in a string is a newline.
                    if s.get(self.i) == Some(&b'\n') {
                        self.i += 1;
                    }
                    bytes.push(b'\n');
                    offs.push(at);
                }
                _ => {
                    bytes.push(c);
                    offs.push(at);
                }
            }
        }
        Obj::Str(bytes, offs)
    }

    fn skip_inline_image(&mut self) {
        // To `ID`, then to an `EI` between white space.
        let s = self.s;
        while self.i + 1 < s.len() {
            if s[self.i] == b'I'
                && s[self.i + 1] == b'D'
                && (self.i == 0 || is_ws(s[self.i - 1]))
                && s.get(self.i + 2).is_none_or(|&c| is_ws(c))
            {
                self.i += 3;
                break;
            }
            self.i += 1;
        }
        while self.i + 1 < s.len() {
            if is_ws(s[self.i - 1])
                && s[self.i] == b'E'
                && s[self.i + 1] == b'I'
                && s.get(self.i + 2).is_none_or(|&c| is_ws(c))
            {
                self.i += 2;
                return;
            }
            self.i += 1;
        }
        self.i = s.len();
    }

    // ---------------------------------------------------------------------
    // output

    fn unsupported(&mut self, what: &str) {
        let n = match self.unsupported.get(what) {
            Some(&n) => n,
            None => {
                let n = self.page.unsupported.len() as u32;
                self.page.unsupported.push(what.to_string());
                self.unsupported.insert(what.to_string(), n);
                n
            }
        };
        self.page.flags |= flags::INCOMPLETE;
        self.page.items.push(Item::Unsupported(n));
    }

    fn matrix(&mut self, m: &Mat) -> u32 {
        if *m == Mat::IDENTITY {
            return 0;
        }
        let key = m.0.map(|x| x.0);
        if let Some(&n) = self.mats.get(&key) {
            return n;
        }
        self.page.matrices.push(m.to_f64());
        let n = self.page.matrices.len() as u32;
        self.mats.insert(key, n);
        n
    }

    /// The marker for stream offset `at`: (span, column).
    fn source(&mut self, at: u32) -> (u32, u16) {
        let m = self.markers;
        while self.mi + 1 < m.len() && m[self.mi + 1].offset <= at {
            self.mi += 1;
        }
        match m.get(self.mi) {
            Some(mk) if mk.offset <= at => (mk.span, mk.col),
            _ => (0, NO_COLUMN),
        }
    }

    fn set_span(&mut self, at: u32) -> u16 {
        let (span, col) = self.source(at);
        if span != self.told_span {
            self.page.items.push(Item::Span(span));
            self.told_span = span;
        }
        col
    }

    fn tell_fill(&mut self) {
        if self.gs.fill != self.gs.told_fill {
            if self.gs.fill.n == 0 {
                self.unsupported("fill colour space");
            } else {
                self.page.items.push(Item::FillColor(self.gs.fill.wire()));
            }
            self.gs.told_fill = self.gs.fill.clone();
        }
    }

    fn tell_stroke(&mut self) {
        if self.gs.stroke != self.gs.told_stroke {
            if self.gs.stroke.n == 0 {
                self.unsupported("stroke colour space");
            } else {
                self.page
                    .items
                    .push(Item::StrokeColor(self.gs.stroke.wire()));
            }
            self.gs.told_stroke = self.gs.stroke.clone();
        }
    }

    fn y_down(&self, y: Fx) -> i32 {
        clamp32((self.height - y).to_sp())
    }

    // ---------------------------------------------------------------------
    // operators

    fn nums<const N: usize>(ops: &[Obj]) -> Option<[Fx; N]> {
        if ops.len() < N {
            return None;
        }
        let mut out = [Fx::ZERO; N];
        for (k, o) in ops[ops.len() - N..].iter().enumerate() {
            match o {
                Obj::Num(v) => out[k] = *v,
                _ => return None,
            }
        }
        Some(out)
    }

    fn op(&mut self, op: &[u8], ops: &[Obj], at: u32) {
        match op {
            b"q" => {
                self.stack.push(self.gs.clone());
                self.page.items.push(Item::Save);
            }
            b"Q" => {
                if let Some(g) = self.stack.pop() {
                    self.gs = g;
                    self.page.items.push(Item::Restore);
                }
            }
            b"cm" => {
                if let Some([a, b, c, d, e, f]) = Self::nums::<6>(ops) {
                    self.gs.ctm = Mat([a, b, c, d, e, f]).then(&self.gs.ctm);
                }
            }
            b"w" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.line_width = v;
                }
            }
            b"J" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.cap = (v.0 / super::fixed::ONE).clamp(0, 2) as u8;
                }
            }
            b"j" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.join = (v.0 / super::fixed::ONE).clamp(0, 2) as u8;
                }
            }
            b"M" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.miter = v;
                }
            }
            b"d" => {
                if let (Some(Obj::Arr(a)), Some(Obj::Num(p))) = (ops.first(), ops.get(1)) {
                    self.gs.dash = a
                        .iter()
                        .filter_map(|o| if let Obj::Num(v) = o { Some(*v) } else { None })
                        .collect();
                    self.gs.phase = *p;
                }
            }
            b"ri" | b"i" => {}
            b"gs" => self.unsupported("gs"),
            // path construction
            b"m" | b"l" => {
                if let Some([x, y]) = Self::nums::<2>(ops) {
                    self.only_re = None;
                    let k = if op == b"m" { 0 } else { 1 };
                    self.fx_path.push((k, x, y));
                    self.path.push(if k == 0 {
                        Seg::Move(x.to_f64(), y.to_f64())
                    } else {
                        Seg::Line(x.to_f64(), y.to_f64())
                    });
                    self.re_count = u32::MAX;
                }
            }
            b"c" | b"v" | b"y" => {
                let (x0, y0) = self.current_point();
                let pts: Option<[Fx; 6]> = match op {
                    b"c" => Self::nums::<6>(ops),
                    b"v" => Self::nums::<4>(ops).map(|[a, b, c, d]| [x0, y0, a, b, c, d]),
                    _ => Self::nums::<4>(ops).map(|[a, b, c, d]| [a, b, c, d, c, d]),
                };
                if let Some([a, b, c, d, e, f]) = pts {
                    self.fx_path.push((2, e, f));
                    self.path.push(Seg::Curve(
                        a.to_f64(),
                        b.to_f64(),
                        c.to_f64(),
                        d.to_f64(),
                        e.to_f64(),
                        f.to_f64(),
                    ));
                    self.only_re = None;
                    self.re_count = u32::MAX;
                }
            }
            b"h" => {
                self.path.push(Seg::Close);
                self.fx_path.push((3, Fx::ZERO, Fx::ZERO));
                self.only_re = None;
                self.re_count = u32::MAX;
            }
            b"re" => {
                if let Some([x, y, w, h]) = Self::nums::<4>(ops) {
                    self.re_count = if self.path.is_empty() { 1 } else { u32::MAX };
                    self.only_re = if self.re_count == 1 {
                        Some((x, y, w, h))
                    } else {
                        None
                    };
                    self.path.push(Seg::Move(x.to_f64(), y.to_f64()));
                    self.path.push(Seg::Line((x + w).to_f64(), y.to_f64()));
                    self.path
                        .push(Seg::Line((x + w).to_f64(), (y + h).to_f64()));
                    self.path.push(Seg::Line(x.to_f64(), (y + h).to_f64()));
                    self.path.push(Seg::Close);
                    self.fx_path.push((0, x, y));
                    self.fx_path.push((3, Fx::ZERO, Fx::ZERO));
                }
            }
            b"W" => self.clip_pending = Some(paint::CLIP),
            b"W*" => self.clip_pending = Some(paint::CLIP_EVEN_ODD),
            b"S" => self.paint(paint::STROKE, false, at),
            b"s" => self.paint(paint::STROKE, true, at),
            b"f" | b"F" => self.paint(paint::FILL, false, at),
            b"f*" => self.paint(paint::FILL_EVEN_ODD, false, at),
            b"B" => self.paint(paint::FILL | paint::STROKE, false, at),
            b"B*" => self.paint(paint::FILL_EVEN_ODD | paint::STROKE, false, at),
            b"b" => self.paint(paint::FILL | paint::STROKE, true, at),
            b"b*" => self.paint(paint::FILL_EVEN_ODD | paint::STROKE, true, at),
            b"n" => self.paint(0, false, at),
            // colour
            b"g" | b"G" | b"rg" | b"RG" | b"k" | b"K" => {
                let n = match op {
                    b"g" | b"G" => 1,
                    b"rg" | b"RG" => 3,
                    _ => 4,
                };
                let v: Option<Vec<Fx>> = match n {
                    1 => Self::nums::<1>(ops).map(|a| a.to_vec()),
                    3 => Self::nums::<3>(ops).map(|a| a.to_vec()),
                    _ => Self::nums::<4>(ops).map(|a| a.to_vec()),
                };
                if let Some(v) = v {
                    let c = Col { n, v };
                    if op[0].is_ascii_lowercase() {
                        self.gs.fill = c;
                    } else {
                        self.gs.stroke = c;
                    }
                }
            }
            b"cs" | b"CS" => {
                let n = match ops.last() {
                    Some(Obj::Name(nm)) if nm == b"DeviceGray" => 1,
                    Some(Obj::Name(nm)) if nm == b"DeviceRGB" => 3,
                    Some(Obj::Name(nm)) if nm == b"DeviceCMYK" => 4,
                    _ => 0,
                };
                // The space's initial colour: black.
                let v = match n {
                    1 => vec![Fx::ZERO],
                    3 => vec![Fx::ZERO; 3],
                    4 => vec![Fx::ZERO, Fx::ZERO, Fx::ZERO, Fx::ONE],
                    _ => vec![],
                };
                let c = Col { n, v };
                if op == b"cs" {
                    self.gs.fill = c;
                } else {
                    self.gs.stroke = c;
                }
            }
            b"sc" | b"scn" | b"SC" | b"SCN" => {
                let fill = op[0] == b's';
                let n = if fill {
                    self.gs.fill.n
                } else {
                    self.gs.stroke.n
                };
                let v: Vec<Fx> = ops
                    .iter()
                    .filter_map(|o| if let Obj::Num(v) = o { Some(*v) } else { None })
                    .collect();
                let c = if n != 0 && v.len() == n as usize && ops.len() == v.len() {
                    Col { n, v }
                } else {
                    Col { n: 0, v: vec![] }
                };
                if fill {
                    self.gs.fill = c;
                } else {
                    self.gs.stroke = c;
                }
            }
            // text
            b"BT" => {
                self.in_text = true;
                self.tm = Mat::IDENTITY;
                self.tlm = Mat::IDENTITY;
            }
            b"ET" => self.in_text = false,
            b"Tc" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.tc = v;
                }
            }
            b"Tw" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.tw = v;
                }
            }
            b"Tz" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.tz = v;
                }
            }
            b"TL" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.tl = v;
                }
            }
            b"Ts" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.ts = v;
                }
            }
            b"Tr" => {
                if let Some([v]) = Self::nums::<1>(ops) {
                    self.gs.tr = (v.0 / super::fixed::ONE).clamp(0, 7) as u8;
                }
            }
            b"Tf" => {
                if let (Some(Obj::Name(nm)), Some([size])) = (ops.first(), Self::nums::<1>(ops)) {
                    self.gs.font = self.font_number(nm);
                    self.gs.fs = size;
                }
            }
            b"Td" | b"TD" => {
                if let Some([tx, ty]) = Self::nums::<2>(ops) {
                    if op == b"TD" {
                        self.gs.tl = -ty;
                    }
                    self.tlm = Mat::translate(tx, ty).then(&self.tlm);
                    self.tm = self.tlm;
                }
            }
            b"Tm" => {
                if let Some([a, b, c, d, e, f]) = Self::nums::<6>(ops) {
                    self.tlm = Mat([a, b, c, d, e, f]);
                    self.tm = self.tlm;
                }
            }
            b"T*" => self.next_line(),
            b"Tj" => {
                if let Some(Obj::Str(b, o)) = ops.last() {
                    self.show(b, o);
                }
            }
            b"'" => {
                self.next_line();
                if let Some(Obj::Str(b, o)) = ops.last() {
                    self.show(b, o);
                }
            }
            b"\"" => {
                if let (Some([aw, ac]), Some(Obj::Str(b, o))) = (
                    Self::nums::<2>(&ops[..ops.len().saturating_sub(1)]),
                    ops.last(),
                ) {
                    self.gs.tw = aw;
                    self.gs.tc = ac;
                    self.next_line();
                    self.show(b, o);
                }
            }
            b"TJ" => {
                if let Some(Obj::Arr(a)) = ops.last() {
                    for o in a {
                        match o {
                            Obj::Str(b, offs) => self.show(b, offs),
                            Obj::Num(n) => {
                                let tx = self.gs.fs.times(*n).mul_div(-1, 1000);
                                self.advance(tx);
                            }
                            _ => {}
                        }
                    }
                }
            }
            // XObjects
            b"Do" => {
                self.set_span(at);
                if let Some(Obj::Name(nm)) = ops.last() {
                    let prefix = self.env.resname_prefix().to_vec();
                    let num = |p: &[u8]| -> Option<u32> {
                        let rest = nm.strip_prefix(p)?;
                        let digits = rest.strip_suffix(prefix.as_slice()).unwrap_or(rest);
                        std::str::from_utf8(digits).ok()?.parse().ok()
                    };
                    let m = self.gs.ctm;
                    if let Some(n) = num(b"Im") {
                        let mi = self.matrix(&m);
                        if !self.images.contains(&n) {
                            self.images.push(n);
                        }
                        self.page.items.push(Item::Image { id: n, matrix: mi });
                    } else if let Some(n) = num(b"Fm") {
                        let mi = self.matrix(&m);
                        if !self.forms.contains(&n) {
                            self.forms.push(n);
                        }
                        self.page.items.push(Item::Form { id: n, matrix: mi });
                    } else {
                        let what = format!("Do /{}", String::from_utf8_lossy(nm));
                        self.unsupported(&what);
                    }
                }
            }
            b"BI" => {
                self.set_span(at);
                self.unsupported("inline image");
            }
            b"sh" => {
                self.set_span(at);
                self.unsupported("sh");
            }
            b"d0" | b"d1" => {}
            b"BMC" | b"BDC" | b"EMC" | b"MP" | b"DP" | b"BX" | b"EX" => {}
            _ => {
                let what = format!("operator {}", String::from_utf8_lossy(op));
                self.unsupported(&what);
            }
        }
    }

    fn font_number(&self, name: &[u8]) -> Option<u32> {
        let rest = name.strip_prefix(b"F")?;
        let prefix = self.env.resname_prefix();
        let digits = if !prefix.is_empty() {
            rest.strip_suffix(prefix).unwrap_or(rest)
        } else {
            rest
        };
        std::str::from_utf8(digits).ok()?.parse().ok()
    }

    fn current_point(&self) -> (Fx, Fx) {
        self.fx_path
            .iter()
            .rev()
            .find(|(k, _, _)| *k != 3)
            .map(|(_, x, y)| (*x, *y))
            .unwrap_or_default()
    }

    fn next_line(&mut self) {
        self.tlm = Mat::translate(Fx::ZERO, -self.gs.tl).then(&self.tlm);
        self.tm = self.tlm;
    }

    /// Move the text matrix by `tx` text-space units (already scaled by
    /// the font size): `Tm = [1 0 0 1 tx·Th 0] × Tm`.
    fn advance(&mut self, tx: Fx) {
        let tx = if self.gs.tz == Fx::from_int(100) {
            tx
        } else {
            tx.times(self.gs.tz).mul_div(1, 100)
        };
        let [a, b, _, _, e, f] = self.tm.0;
        if a == Fx::ONE && b.is_zero() {
            self.tm.0[4] = e + tx;
        } else {
            self.tm.0[4] = e + tx.times(a);
            self.tm.0[5] = f + tx.times(b);
        }
    }

    fn show(&mut self, bytes: &[u8], offs: &[u32]) {
        let font = self.gs.font;
        if font.is_none() {
            if !bytes.is_empty() {
                self.unsupported("text without a pdfTeX font");
            }
            return;
        }
        let k = font.unwrap();
        if k > u16::MAX as u32 {
            self.unsupported("font number");
            return;
        }
        if !self.fonts.contains(&k) {
            self.fonts.push(k);
            if let Some(why) = self.env.font_problem(k) {
                self.set_span(offs.first().copied().unwrap_or(0));
                self.unsupported(&why);
            }
        }
        let fs = self.gs.fs;
        // The linear part of the text rendering matrix, for these glyphs.
        let th = self.gs.tz.mul_div(1, 100);
        let text = Mat([fs.times(th), Fx::ZERO, Fx::ZERO, fs, Fx::ZERO, Fx::ZERO]);
        for (idx, &code) in bytes.iter().enumerate() {
            let at = offs.get(idx).copied().unwrap_or(0);
            let col = self.set_span(at);
            let trm = self.tm.then(&self.gs.ctm);
            let mut lin = text.then(&Mat([
                trm.0[0],
                trm.0[1],
                trm.0[2],
                trm.0[3],
                Fx::ZERO,
                Fx::ZERO,
            ]));
            lin.0[4] = Fx::ZERO;
            lin.0[5] = Fx::ZERO;
            let mi = self.matrix(&lin);
            if mi != self.told_matrix {
                self.page.items.push(Item::Matrix(mi));
                self.told_matrix = mi;
            }
            if self.gs.tr != self.gs.told_tr {
                if self.gs.tr > 3 {
                    self.unsupported("text clipping (Tr 4-7)");
                }
                self.page.items.push(Item::TextRender(self.gs.tr));
                self.gs.told_tr = self.gs.tr;
            }
            match self.gs.tr & 3 {
                0 => self.tell_fill(),
                1 => self.tell_stroke(),
                2 => {
                    self.tell_fill();
                    self.tell_stroke();
                }
                _ => {}
            }
            let (x, y) = if self.gs.ts.is_zero() {
                trm.apply(Fx::ZERO, Fx::ZERO)
            } else {
                trm.apply(Fx::ZERO, self.gs.ts)
            };
            self.page.items.push(Item::Glyph {
                font: k as u16,
                code: code as u16,
                x: clamp32(x.to_sp()),
                y: self.y_down(y),
                col,
            });
            // The advance: (w0·Tfs + Tc + Tw) · Th, w0 the width in text
            // space (/1000 for all but Type 3 fonts).
            let w = match self.env.advance(k, code) {
                Some((n, d)) => fs.mul_div(n, d),
                None => {
                    self.unsupported("glyph width");
                    Fx::ZERO
                }
            };
            let mut tx = w + self.gs.tc;
            if code == 32 {
                tx = tx + self.gs.tw;
            }
            self.advance(tx);
        }
    }

    fn paint(&mut self, bits: u8, close: bool, at: u32) {
        if close {
            self.path.push(Seg::Close);
            self.fx_path.push((3, Fx::ZERO, Fx::ZERO));
            self.only_re = None;
        }
        let clip = self.clip_pending.take();
        let path = std::mem::take(&mut self.path);
        let fx_path = std::mem::take(&mut self.fx_path);
        let only_re = self.only_re.take();
        self.re_count = 0;
        if path.is_empty() {
            return;
        }
        self.set_span(at);
        let ctm = self.gs.ctm;
        let mut done = false;
        if bits != 0 && clip.is_none() && ctm.is_translation() {
            if let Some(r) = self.as_rule(bits, &fx_path, only_re, &ctm) {
                match r.0 {
                    RuleKind::Fill => self.tell_fill(),
                    _ => self.tell_stroke(),
                }
                let (kind, x, y, w, h) = r;
                self.page.items.push(Item::Rule { kind, x, y, w, h });
                done = true;
            }
        }
        let mi = if (bits != 0 && !done) || clip.is_some() {
            self.matrix(&ctm)
        } else {
            0
        };
        if bits != 0 && !done {
            if bits & (paint::FILL | paint::FILL_EVEN_ODD) != 0 {
                self.tell_fill();
            }
            let stroke = if bits & paint::STROKE != 0 {
                self.tell_stroke();
                Some(Stroke {
                    width: self.gs.line_width.to_f64(),
                    cap: self.gs.cap,
                    join: self.gs.join,
                    miter: self.gs.miter.to_f64(),
                    dash: self.gs.dash.iter().map(|d| d.to_f64()).collect(),
                    phase: self.gs.phase.to_f64(),
                })
            } else {
                None
            };
            let n = self.page.paths.len() as u32;
            self.page.paths.push(Path {
                paint: bits,
                matrix: mi,
                stroke,
                segs: path.clone(),
            });
            self.page.items.push(Item::Path(n));
        }
        if let Some(c) = clip {
            let n = self.page.paths.len() as u32;
            self.page.paths.push(Path {
                paint: c,
                matrix: mi,
                stroke: None,
                segs: path,
            });
            self.page.items.push(Item::Clip(n));
        }
    }

    /// pdfTeX's rules (`pdf_set_rule`) and anything drawn like them: one
    /// `re` filled, or one horizontal or vertical line stroked with butt
    /// caps and no dash, under a CTM without rotation or scaling.
    fn as_rule(
        &self,
        bits: u8,
        fx_path: &[(u8, Fx, Fx)],
        only_re: Option<(Fx, Fx, Fx, Fx)>,
        ctm: &Mat,
    ) -> Option<(RuleKind, i32, i32, i32, i32)> {
        let (e, f) = (ctm.0[4], ctm.0[5]);
        let edges = |x0: Fx, y0: Fx, x1: Fx, y1: Fx| {
            let (l, r) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
            let (b, t) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
            let left = clamp32((l + e).to_sp());
            let right = clamp32((r + e).to_sp());
            let top = self.y_down(t + f);
            let bottom = self.y_down(b + f);
            (left, top, right - left, bottom - top)
        };
        if bits == paint::FILL || bits == paint::FILL_EVEN_ODD {
            let (x, y, w, h) = only_re?;
            let (l, t, w, h) = edges(x, y, x + w, y + h);
            return Some((RuleKind::Fill, l, t, w, h));
        }
        if bits == paint::STROKE && self.gs.cap == 0 && self.gs.dash.is_empty() {
            if let [(0, x0, y0), (1, x1, y1)] = fx_path {
                let hw = self.gs.line_width.mul_div(1, 2);
                if y0 == y1 && x0 != x1 {
                    let (l, t, w, h) = edges(*x0, *y0 - hw, *x1, *y0 + hw);
                    return Some((RuleKind::StrokeH, l, t, w, h));
                }
                if x0 == x1 && y0 != y1 {
                    let (l, t, w, h) = edges(*x0 - hw, *y0, *x0 + hw, *y1);
                    return Some((RuleKind::StrokeV, l, t, w, h));
                }
            }
        }
        None
    }
}

fn clamp32(v: i64) -> i32 {
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn unescape_name(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'#' && i + 2 < b.len() {
            if let Ok(v) =
                u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or("zz"), 16)
            {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEnv;
    impl Env for TestEnv {
        fn width(&mut self, _font: u32, code: u8) -> Option<i64> {
            Some(if code == b'o' { 5000 } else { 2777 })
        }
        fn resname_prefix(&self) -> &[u8] {
            b""
        }
    }

    fn fx(s: &str) -> Fx {
        Fx::parse(s.as_bytes()).unwrap()
    }

    #[test]
    fn text_positions_are_exact() {
        let s =
            b"BT\n/F41 9.9626 Tf 86.944 710.037 Td [(oo)-357(o)]TJ -14.944 -11.955 Td [(o)]TJ\nET";
        let out = interpret(
            &mut TestEnv,
            StreamKind::Page,
            0,
            s,
            fx("792"),
            Mat::IDENTITY,
            &[],
        );
        let g: Vec<(i32, i32)> = out
            .page
            .items
            .iter()
            .filter_map(|i| {
                if let Item::Glyph { x, y, .. } = i {
                    Some((*x, *y))
                } else {
                    None
                }
            })
            .collect();
        // x of the 2nd o: 86.944 + 0.5*9.9626 = 91.9253 bp.
        assert_eq!(
            g[0],
            (
                fx("86.944").to_sp() as i32,
                (fx("792") - fx("710.037")).to_sp() as i32
            )
        );
        assert_eq!(g[1].0, fx("91.9253").to_sp() as i32);
        // 3rd: 86.944 + 2*4.9813 + 0.357*9.9626 = 100.4632482 bp.
        assert_eq!(g[2].0, fx("100.4632482").to_sp() as i32);
        // After Td: 86.944 - 14.944 = 72.
        assert_eq!(
            g[3],
            (
                fx("72").to_sp() as i32,
                (fx("792") - fx("698.082")).to_sp() as i32
            )
        );
        assert_eq!(out.fonts, vec![41]);
        assert_eq!(out.page.matrices[0][0], 9.9626);
    }

    #[test]
    fn rules_and_paths() {
        let s = b"q\n1 0 0 1 72 700 cm\n0 0 100 0.4 re f\nQ\nq\n1 0 0 1 72 690 cm\n[]0 d 0 J 0.398 w 0 0 m 50 0 l S\nQ\n0 0 1 rg 0 0 m 10 10 l 5 5 5 5 5 0 c f\n";
        let out = interpret(
            &mut TestEnv,
            StreamKind::Page,
            0,
            s,
            fx("792"),
            Mat::IDENTITY,
            &[],
        );
        let rules: Vec<&Item> = out
            .page
            .items
            .iter()
            .filter(|i| matches!(i, Item::Rule { .. }))
            .collect();
        assert_eq!(rules.len(), 2);
        if let Item::Rule { kind, x, y, w, h } = rules[0] {
            assert_eq!(*kind, RuleKind::Fill);
            assert_eq!(*x, fx("72").to_sp() as i32);
            assert_eq!(*y, (fx("792") - fx("700.4")).to_sp() as i32);
            assert_eq!(*w, fx("172").to_sp() as i32 - *x);
            assert_eq!(*h, (fx("92")).to_sp() as i32 - *y);
        }
        if let Item::Rule { kind, .. } = rules[1] {
            assert_eq!(*kind, RuleKind::StrokeH);
        }
        assert_eq!(out.page.paths.len(), 1);
        assert!(out
            .page
            .items
            .contains(&Item::FillColor(Color(vec![0.0, 0.0, 1.0]))));
    }

    #[test]
    fn markers_give_spans() {
        let s = b"BT /F1 10 Tf 0 0 Td [(oo)]TJ ET";
        let at = s.iter().position(|&c| c == b'(').unwrap() as u32;
        let m = [
            Marker {
                offset: at + 1,
                span: 5,
                col: 1,
            },
            Marker {
                offset: at + 2,
                span: 5,
                col: 2,
            },
        ];
        let out = interpret(
            &mut TestEnv,
            StreamKind::Page,
            0,
            s,
            fx("792"),
            Mat::IDENTITY,
            &m,
        );
        let cols: Vec<u16> = out
            .page
            .items
            .iter()
            .filter_map(|i| {
                if let Item::Glyph { col, .. } = i {
                    Some(*col)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(cols, vec![1, 2]);
        assert!(out.page.items.contains(&Item::Span(5)));
    }
}
