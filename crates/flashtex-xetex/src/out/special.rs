//! dvipdfmx's `\special` language, as the `xetex` and `dvipdfmx` drivers of
//! real packages emit it (`xetex.def`, `l3backend-xetex.def`, `hxetex.def`,
//! `pgfsys-xetex.def`, xcolor), read from its documentation
//! (`dvipdfmx-special.pdf`, the dvipdfmx manual) and from what TeX Live
//! 2026's `xdvipdfmx` measurably does with it (PLAN.md §3.2). dvipdfmx is
//! not ported.
//!
//! Each special acts on the display list being built (colours, PDF
//! operators, transformations, images, forms, links, destinations) or on
//! the document (named objects, the outline, the document information).
//! A special that is not understood is counted by its first word
//! ([`Doc::unknown`]) and reported, never silently dropped.

use super::doc::{Col, Named, Outline, Tracking};
use super::page::{fx, Builder, Metrics};
use super::pdfobj::{Dict, Obj, Parser};
use flashtex_display_list::page::{paint, Item, Path};
use flashtex_engine::displaylist::fixed::{Fx, Mat};

/// The specials of a page's bytes, in order (for the scan dvipdfmx makes
/// before a page: its size).
pub fn scan_specials(b: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let rd = |i: usize, n: usize| -> Option<u32> {
        Some(
            b.get(i..i + n)?
                .iter()
                .fold(0u32, |a, &c| (a << 8) | c as u32),
        )
    };
    while i < b.len() {
        let op = b[i];
        i += 1;
        let skip = match op {
            0..=127 | 138 | 140..=142 | 147 | 152 | 161 | 166 | 171..=234 => 0,
            128..=131 | 133..=136 | 235..=238 => (op as usize & 3) + 1,
            132 | 137 => 8,
            139 => 44,
            143..=146 => op as usize - 142,
            148..=151 => op as usize - 147,
            153..=156 => op as usize - 152,
            157..=160 => op as usize - 156,
            162..=165 => op as usize - 161,
            167..=170 => op as usize - 166,
            239..=242 => {
                let n = op as usize - 238;
                let Some(k) = rd(i, n) else { break };
                let st = i + n;
                if let Some(s) = b.get(st..st + k as usize) {
                    out.push(s.to_vec());
                }
                n + k as usize
            }
            243..=246 => {
                let n = op as usize - 242;
                let Some(a) = rd(i + n + 12, 1) else { break };
                let Some(l) = rd(i + n + 13, 1) else { break };
                n + 14 + a as usize + l as usize
            }
            247 => {
                let Some(k) = rd(i + 13, 1) else { break };
                14 + k as usize
            }
            252 => {
                let Some(fl) = rd(i + 8, 2) else { break };
                let Some(l) = rd(i + 10, 1) else { break };
                let mut n = 11 + l as usize + 4;
                for f in [0x0200, 0x1000, 0x2000, 0x4000] {
                    if fl & f != 0 {
                        n += 4;
                    }
                }
                n
            }
            253 => {
                let Some(k) = rd(i + 4, 2) else { break };
                6 + 10 * k as usize
            }
            254 => {
                let Some(l) = rd(i, 2) else { break };
                let j = i + 2 + 2 * l as usize;
                let Some(k) = rd(j + 4, 2) else { break };
                2 + 2 * l as usize + 6 + 10 * k as usize
            }
            _ => break,
        };
        i += skip;
    }
    out
}

/// A paper-size special: `Some(Some((w, h)))` (bp), `Some(None)` for
/// `pdf:pagesize default`, `None` if `s` is not one.
pub fn paper_of(s: &[u8]) -> Option<Option<(Fx, Fx)>> {
    let t = trim(s);
    if let Some(r) = t.strip_prefix(b"papersize=") {
        let txt = String::from_utf8_lossy(r).into_owned();
        let (w, h) = txt.split_once(',')?;
        let mut pw = Parser::new(w.trim().as_bytes());
        let mut ph = Parser::new(h.trim().as_bytes());
        return Some(Some((fx(pw.dimension()?), fx(ph.dimension()?))));
    }
    let r = t.strip_prefix(b"pdf:")?;
    let mut p = Parser::new(r);
    if !(p.keyword(b"pagesize") || p.keyword(b"papersize")) {
        return None;
    }
    if p.keyword(b"default") {
        return Some(None);
    }
    let (mut w, mut h) = (None, None);
    while !p.at_end() {
        match p.word() {
            Some(b"width") => w = p.dimension(),
            Some(b"height") => h = p.dimension(),
            Some(b"letter") => {
                w = Some(612.0);
                h = Some(792.0);
            }
            Some(b"a4") => {
                w = Some(595.27559);
                h = Some(841.88976);
            }
            Some(_) => {
                p.dimension();
            }
            None => break,
        }
    }
    Some(Some((fx(w?), fx(h?))))
}

fn trim(s: &[u8]) -> &[u8] {
    let st = s
        .iter()
        .position(|c| !c.is_ascii_whitespace())
        .unwrap_or(s.len());
    &s[st..]
}

/// A colour of a special: `rgb r g b`, `cmyk c m y k`, `gray g`,
/// `hsb h s b`, or a PDF array `[g]`, `[r g b]`, `[c m y k]`.
pub fn read_color(p: &mut Parser) -> Option<Col> {
    if p.peek() == Some(b'[') {
        let Obj::Array(a) = p.object()? else {
            return None;
        };
        let v: Option<Vec<f64>> = a.iter().map(Obj::as_num).collect();
        let v = v?;
        return matches!(v.len(), 1 | 3 | 4).then_some(Col(v));
    }
    let save = p.i;
    let w = p.word()?.to_vec();
    let nums =
        |p: &mut Parser, n: usize| -> Option<Vec<f64>> { (0..n).map(|_| p.number()).collect() };
    let c = match w.as_slice() {
        b"rgb" => Col(nums(p, 3)?),
        b"cmyk" => Col(nums(p, 4)?),
        b"gray" | b"grey" => Col(nums(p, 1)?),
        b"hsb" => {
            let v = nums(p, 3)?;
            Col(hsb_to_rgb(v[0], v[1], v[2]).to_vec())
        }
        _ => match named_color(&w) {
            Some(c) => c,
            None => {
                p.i = save;
                // a bare number is gray
                let g = p.number()?;
                Col(vec![g])
            }
        },
    };
    Some(c)
}

fn hsb_to_rgb(h: f64, s: f64, b: f64) -> [f64; 3] {
    let h6 = (h * 6.0).rem_euclid(6.0);
    let i = h6.floor();
    let f = h6 - i;
    let (p, q, t) = (b * (1.0 - s), b * (1.0 - s * f), b * (1.0 - s * (1.0 - f)));
    match i as i32 {
        0 => [b, t, p],
        1 => [q, b, p],
        2 => [p, b, t],
        3 => [p, q, b],
        4 => [t, p, b],
        _ => [b, p, q],
    }
}

/// The dvips colour names dvipdfmx knows (`color.pro`'s CMYK values) that
/// drivers still emit; others are reported.
fn named_color(w: &[u8]) -> Option<Col> {
    let cmyk = |c: f64, m: f64, y: f64, k: f64| Some(Col(vec![c, m, y, k]));
    match w {
        b"Black" => cmyk(0.0, 0.0, 0.0, 1.0),
        b"White" => cmyk(0.0, 0.0, 0.0, 0.0),
        b"Red" => cmyk(0.0, 1.0, 1.0, 0.0),
        b"Green" => cmyk(1.0, 0.0, 1.0, 0.0),
        b"Blue" => cmyk(1.0, 1.0, 0.0, 0.0),
        b"Cyan" => cmyk(1.0, 0.0, 0.0, 0.0),
        b"Magenta" => cmyk(0.0, 1.0, 0.0, 0.0),
        b"Yellow" => cmyk(0.0, 0.0, 1.0, 0.0),
        _ => None,
    }
}

impl<M: Metrics> Builder<'_, M> {
    /// Carry out one special.
    pub fn special(&mut self, s: &[u8]) {
        let t = trim(s);
        if let Some(r) = t.strip_prefix(b"pdf:") {
            self.pdf_special(r);
        } else if let Some(r) = t.strip_prefix(b"x:") {
            self.x_special(r);
        } else if t.starts_with(b"color") && t.get(5).is_none_or(|c| c.is_ascii_whitespace()) {
            self.color_special(&t[5..]);
        } else if let Some(r) = t.strip_prefix(b"background") {
            let mut p = Parser::new(r);
            match read_color(&mut p) {
                Some(c) => self.background(c),
                None => self.doc.diag(format!("bad colour in `{}'", lossy(t))),
            }
        } else if t.starts_with(b"papersize=") || t.starts_with(b"dvipdfmx:config") {
            // read before the page (papersize) or configuration flags
        } else if t.starts_with(b"src:") {
            // source specials: no output
        } else if t.is_empty() {
        } else {
            let w = t
                .split(|c| c.is_ascii_whitespace() || *c == b':')
                .next()
                .unwrap_or(t);
            let key = if t.get(w.len()) == Some(&b':') {
                format!("{}:", lossy(w))
            } else {
                lossy(w)
            };
            self.doc.unknown_special(&key);
        }
    }

    fn color_special(&mut self, r: &[u8]) {
        let mut p = Parser::new(r);
        if p.keyword(b"push") {
            let Some(c) = read_color(&mut p) else {
                self.doc
                    .diag(format!("bad colour in `color push{}'", lossy(r)));
                return;
            };
            self.doc.colors.push((c.clone(), c.clone()));
            self.set_colors(Some(c.clone()), Some(c));
        } else if p.keyword(b"pop") {
            self.doc.colors.pop();
            self.reset_color();
        } else {
            let Some(c) = read_color(&mut p) else {
                self.doc.diag(format!("bad colour in `color{}'", lossy(r)));
                return;
            };
            // `color SPEC`: the stack is that colour alone.
            self.doc.colors.clear();
            self.doc.colors.push((c.clone(), c.clone()));
            self.set_colors(Some(c.clone()), Some(c));
        }
    }

    /// `background SPEC`, `pdf:bgcolor SPEC`: the page's background, a
    /// filled rectangle under everything else.
    fn background(&mut self, c: Col) {
        let page = self.streams.first_mut().expect("the page");
        let [x0, y0, x1, y1] = page.dl.pdf_box;
        page.dl.paths.push(Path {
            paint: paint::FILL,
            matrix: 0,
            stroke: None,
            segs: super::page::rect_segs(x0, y0, x1 - x0, y1 - y0),
        });
        let n = page.dl.paths.len() as u32 - 1;
        // After the page's opening SAVE: under everything, in its own state.
        let items = [
            Item::Save,
            Item::FillColor(flashtex_display_list::page::Color(c.0)),
            Item::Path(n),
            Item::Restore,
        ];
        let at = 1.min(page.dl.items.len());
        page.dl.items.splice(at..at, items);
    }

    fn x_special(&mut self, r: &[u8]) {
        let mut p = Parser::new(r);
        let Some(cmd) = p.word().map(<[u8]>::to_vec) else {
            self.doc.unknown_special("x:");
            return;
        };
        match cmd.as_slice() {
            b"gsave" => self.cur().save(),
            b"grestore" => {
                self.cur().restore();
                self.reset_color();
            }
            b"scale" => {
                let (Some(sx), Some(sy)) = (p.number(), p.number()) else {
                    self.doc.diag(format!("bad `x:{}'", lossy(r)));
                    return;
                };
                let m = Mat([fx(sx), Fx::ZERO, Fx::ZERO, fx(sy), Fx::ZERO, Fx::ZERO]);
                self.concat_about_point(m);
            }
            b"rotate" => {
                let Some(a) = p.number() else {
                    self.doc.diag(format!("bad `x:{}'", lossy(r)));
                    return;
                };
                self.concat_about_point(rotation(a));
            }
            b"fontmapline" => {
                let line = trim(p.rest()).to_vec();
                self.doc.fonts.map_line(&line);
            }
            b"backgroundcolor" => {
                if let Some(c) = read_color(&mut p) {
                    self.background(c);
                }
            }
            _ => self.doc.unknown_special(&format!("x:{}", lossy(&cmd))),
        }
    }

    /// Concatenate `m` to the CTM as a transformation about the current
    /// point (dvipdfmx's `pdf:btrans`, `x:scale`, `x:rotate`).
    pub fn concat_about_point(&mut self, m: Mat) {
        let (x, y) = self.user_point();
        let (x, y) = self.rel(x, y);
        let t = Mat::translate(-x, -y).then(&m).then(&Mat::translate(x, y));
        let s = self.cur();
        s.gs.ctm = t.then(&s.gs.ctm);
    }

    fn pdf_special(&mut self, r: &[u8]) {
        let mut p = Parser::new(r);
        let Some(cmd) = p.word().map(<[u8]>::to_vec) else {
            self.doc.unknown_special("pdf:");
            return;
        };
        match cmd.as_slice() {
            b"pagesize" | b"papersize" => {} // read before the page
            b"majorversion" => {
                if let Some(v) = p.number() {
                    self.doc.version.0 = v as u32;
                }
            }
            b"minorversion" => {
                if let Some(v) = p.number() {
                    self.doc.version.1 = v as u32;
                }
            }
            b"code" | b"direct" => self.literal(p.rest()),
            b"literal" => {
                if p.keyword(b"direct") {
                    self.literal(p.rest());
                } else {
                    let reverse = p.keyword(b"reverse");
                    if reverse {
                        self.doc.ignored_special("pdf:literal reverse");
                    }
                    // the origin moved to the current point, and back
                    let (x, y) = self.user_point();
                    let (x, y) = self.rel(x, y);
                    {
                        let s = self.cur();
                        s.gs.ctm = Mat::translate(x, y).then(&s.gs.ctm);
                    }
                    self.literal(p.rest());
                    let s = self.cur();
                    s.gs.ctm = Mat::translate(-x, -y).then(&s.gs.ctm);
                }
            }
            b"content" => {
                let (x, y) = self.user_point();
                let (x, y) = self.rel(x, y);
                {
                    let s = self.cur();
                    s.save();
                    s.gs.ctm = Mat::translate(x, y).then(&s.gs.ctm);
                }
                self.literal(p.rest());
                self.cur().restore();
                self.reset_color();
            }
            b"bcontent" => {
                let (x, y) = self.user_point();
                let (tx, ty) = self.coord_top();
                let s = self.cur();
                s.save();
                s.coords.push((x, y));
                s.gs.ctm = Mat::translate(x - tx, y - ty).then(&s.gs.ctm);
            }
            b"econtent" => {
                let s = self.cur();
                s.coords.pop();
                s.restore();
                self.reset_color();
            }
            b"btrans" | b"begintransform" => {
                let m = read_transform(&mut p);
                self.cur().save();
                self.concat_about_point(m);
            }
            b"etrans" | b"endtransform" => {
                self.cur().restore();
                self.reset_color();
            }
            b"bcolor" | b"begincolor" => {
                let Some(f) = read_color(&mut p) else {
                    self.doc.diag(format!("bad colour in `pdf:{}'", lossy(r)));
                    return;
                };
                let s = read_color(&mut p).unwrap_or_else(|| f.clone());
                self.doc.colors.push((f.clone(), s.clone()));
                self.set_colors(Some(f), Some(s));
            }
            b"scolor" | b"setcolor" => {
                let Some(f) = read_color(&mut p) else {
                    self.doc.diag(format!("bad colour in `pdf:{}'", lossy(r)));
                    return;
                };
                let s = read_color(&mut p).unwrap_or_else(|| f.clone());
                match self.doc.colors.last_mut() {
                    Some(top) => *top = (f.clone(), s.clone()),
                    None => self.doc.colors.push((f.clone(), s.clone())),
                }
                self.set_colors(Some(f), Some(s));
            }
            b"ecolor" | b"endcolor" => {
                self.doc.colors.pop();
                self.reset_color();
            }
            b"bgcolor" => {
                if let Some(c) = read_color(&mut p) {
                    self.background(c);
                }
            }
            b"image" => self.image_special(&mut p, true),
            b"uxobj" | b"usexobj" => self.uxobj(&mut p),
            b"bxobj" | b"beginxobj" => self.bxobj(&mut p),
            b"exobj" | b"endxobj" => {
                let d = match p.object() {
                    Some(Obj::Dict(d)) => Some(self.fixup_dict(d)),
                    _ => None,
                };
                self.end_form(d);
            }
            b"obj" | b"object" => {
                let Some(Obj::Named(name)) = p.object() else {
                    self.doc
                        .diag(format!("pdf:obj without a name: `{}'", lossy(r)));
                    return;
                };
                match p.object() {
                    Some(o) => {
                        let o = self.fixup(o);
                        self.doc.named.insert(name, Named::Obj(o));
                    }
                    None => self
                        .doc
                        .diag(format!("pdf:obj @{} without an object", lossy(&name))),
                }
            }
            b"put" => self.put(&mut p),
            b"close" => {
                if let Some(Obj::Named(n)) = p.object() {
                    self.doc.closed.push(n);
                }
            }
            b"stream" | b"fstream" => self.stream_special(&cmd, &mut p),
            b"ann" | b"annotation" => self.ann(&mut p),
            b"bann" | b"beginann" => {
                let Some(Obj::Dict(d)) = p.object() else {
                    self.doc
                        .diag(format!("pdf:bann without a dictionary: `{}'", lossy(r)));
                    return;
                };
                let d = self.fixup_dict(d);
                if self.doc.tracking.is_some() {
                    self.doc.diag("pdf:bann inside another pdf:bann".into());
                    self.break_annot();
                }
                self.doc.tracking = Some(Tracking {
                    dict: d,
                    depth: self.stack.len(),
                    rect: None,
                });
            }
            b"eann" | b"endann" => {
                self.break_annot();
                self.doc.tracking = None;
            }
            b"dest" => {
                let name = match p.object() {
                    Some(Obj::Str(s)) | Some(Obj::Name(s)) => s,
                    _ => {
                        self.doc
                            .diag(format!("pdf:dest without a name: `{}'", lossy(r)));
                        return;
                    }
                };
                let Some(o) = p.object() else {
                    self.doc
                        .diag(format!("pdf:dest without a destination: `{}'", lossy(r)));
                    return;
                };
                match self.fixup(o) {
                    Obj::Array(a) => self.add_dest(name, &a),
                    o => self.doc.dests.push((name, o)),
                }
            }
            b"outline" | b"out" => {
                // `[-]`: closed, `[]`: open; without either, dvipdfmx's
                // default (`-O 0`, TeX Live's `dvipdfmx.cfg`): closed.
                let mut open = false;
                if p.peek() == Some(b'[') {
                    let r = p.rest();
                    let close = r.iter().position(|&c| c == b']').unwrap_or(r.len());
                    open = !r[..close].contains(&b'-');
                    p.i += (close + 1).min(r.len());
                }
                let level = p.number().unwrap_or(1.0) as i32;
                let Some(Obj::Dict(d)) = p.object() else {
                    self.doc
                        .diag(format!("pdf:outline without a dictionary: `{}'", lossy(r)));
                    return;
                };
                let d = self.fixup_dict(d);
                self.doc.outlines.push(Outline {
                    level,
                    open,
                    dict: d,
                });
            }
            b"docinfo" => {
                if let Some(Obj::Dict(d)) = p.object() {
                    let d = self.fixup_dict(d);
                    self.doc.docinfo.merge(&d);
                }
            }
            b"docview" => {
                if let Some(Obj::Dict(d)) = p.object() {
                    let d = self.fixup_dict(d);
                    self.doc.catalog.merge(&d);
                }
            }
            b"mapline" => {
                let line = trim(p.rest()).to_vec();
                self.doc.fonts.map_line(&line);
            }
            b"tounicode" | b"mapfile" | b"encrypt" | b"nolink" | b"link" => {
                self.doc.ignored_special(&format!("pdf:{}", lossy(&cmd)));
            }
            _ => self.doc.unknown_special(&format!("pdf:{}", lossy(&cmd))),
        }
    }

    /// `@thispage`, `@prevpage`, `@nextpage` as `@page<n>`; `@xpos` and
    /// `@ypos` as the current point (bp, page coordinates).
    pub fn fixup(&self, o: Obj) -> Obj {
        match o {
            Obj::Named(n) => match n.as_slice() {
                b"thispage" => Obj::Named(format!("page{}", self.page_no).into_bytes()),
                b"prevpage" => Obj::Named(
                    format!("page{}", self.page_no.saturating_sub(1).max(1)).into_bytes(),
                ),
                b"nextpage" => Obj::Named(format!("page{}", self.page_no + 1).into_bytes()),
                b"xpos" | b"ypos" => {
                    let (x, y) = self.stream_point_device();
                    let v = if n == b"xpos" { x } else { y };
                    Obj::num(v.to_f64(), 3)
                }
                _ => Obj::Named(n),
            },
            Obj::Array(a) => Obj::Array(a.into_iter().map(|e| self.fixup(e)).collect()),
            Obj::Dict(d) => Obj::Dict(self.fixup_dict(d)),
            o => o,
        }
    }

    pub fn fixup_dict(&self, d: Dict) -> Dict {
        Dict(d.0.into_iter().map(|(k, v)| (k, self.fixup(v))).collect())
    }

    /// The current point as `@xpos`/`@ypos` give it: dvipdfmx's device
    /// coordinates (the page's, without the specials' transformations).
    fn stream_point_device(&self) -> (Fx, Fx) {
        let (x, y) = self.user_point();
        (
            x + Fx(super::page::ORIGIN),
            self.page_height - Fx(super::page::ORIGIN) + y,
        )
    }

    fn put(&mut self, p: &mut Parser) {
        let Some(Obj::Named(name)) = p.object() else {
            self.doc.diag("pdf:put without a name".into());
            return;
        };
        let mut objs = Vec::new();
        while let Some(o) = p.object() {
            objs.push(self.fixup(o));
        }
        let first_dict = || objs.iter().find_map(|o| o.as_dict().cloned());
        match name.as_slice() {
            b"resources" => {
                if let Some(d) = first_dict() {
                    self.cur().sup.resources.merge(&d);
                }
            }
            b"thispage" => {
                if let Some(d) = first_dict() {
                    self.streams[0].sup.page_dict.merge(&d);
                }
            }
            b"catalog" => {
                if let Some(d) = first_dict() {
                    self.doc.catalog.merge(&d);
                }
            }
            b"names" => {
                if let Some(d) = first_dict() {
                    self.doc.names.merge(&d);
                }
            }
            b"docinfo" => {
                if let Some(d) = first_dict() {
                    self.doc.docinfo.merge(&d);
                }
            }
            _ => {
                if self.doc.closed.contains(&name) {
                    self.doc
                        .diag(format!("pdf:put to closed object @{}", lossy(&name)));
                    return;
                }
                match self.doc.named.get_mut(&name) {
                    Some(Named::Obj(Obj::Dict(d))) | Some(Named::Stream(d, _)) => {
                        if let Some(n) = objs.iter().find_map(Obj::as_dict) {
                            d.merge(n);
                        }
                    }
                    Some(Named::Obj(Obj::Array(a))) => a.extend(objs),
                    Some(_) => self.doc.diag(format!(
                        "pdf:put to @{}, which is not a dictionary or array",
                        lossy(&name)
                    )),
                    None => {
                        // dvipdfmx makes the object on a put to an unknown name
                        let o = match objs.len() {
                            1 => objs.pop().expect("one"),
                            _ => Obj::Array(objs),
                        };
                        self.doc.named.insert(name, Named::Obj(o));
                    }
                }
            }
        }
    }

    fn stream_special(&mut self, cmd: &[u8], p: &mut Parser) {
        let Some(Obj::Named(name)) = p.object() else {
            self.doc.diag(format!("pdf:{} without a name", lossy(cmd)));
            return;
        };
        let Some(Obj::Str(s)) = p.object() else {
            self.doc.diag(format!(
                "pdf:{} @{} without a string",
                lossy(cmd),
                lossy(&name)
            ));
            return;
        };
        let dict = match p.object() {
            Some(Obj::Dict(d)) => self.fixup_dict(d),
            _ => Dict::default(),
        };
        let data = if cmd == b"fstream" {
            let f = String::from_utf8_lossy(&s).into_owned();
            let path =
                flashtex_engine::system::find_file(&f, flashtex_engine::resolver::Format::Pict)
                    .or_else(|| std::path::Path::new(&f).is_file().then(|| f.clone()));
            match path.and_then(|p| std::fs::read(p).ok()) {
                Some(d) => d,
                None => {
                    self.doc.diag(format!("pdf:fstream: cannot read {f}"));
                    return;
                }
            }
        } else {
            s
        };
        self.doc.named.insert(name, Named::Stream(dict, data));
    }

    /// `pdf:ann [@name] dims|bbox <<dict>>`: an annotation at the current
    /// point.
    fn ann(&mut self, p: &mut Parser) {
        let _name = if p.peek() == Some(b'@') {
            p.object()
        } else {
            None
        };
        let t = read_dims(p);
        let Some(Obj::Dict(d)) = p.object() else {
            self.doc.diag("pdf:ann without a dictionary".into());
            return;
        };
        let d = self.fixup_dict(d);
        let (x, y) = self.stream_point_device();
        let (x, y) = (x.to_f64(), y.to_f64());
        let r = match t.bbox {
            Some(b) => [x + b[0], y + b[1], x + b[2], y + b[3]],
            None => [
                x,
                y - t.depth.unwrap_or(0.0),
                x + t.width.unwrap_or(0.0),
                y + t.height.unwrap_or(0.0),
            ],
        };
        self.add_link(d, r);
    }

    fn bxobj(&mut self, p: &mut Parser) {
        let Some(Obj::Named(name)) = p.object() else {
            self.doc.diag("pdf:bxobj without a name".into());
            return;
        };
        let t = read_dims(p);
        let bbox = match t.bbox {
            Some(b) => b.map(fx),
            None => [
                Fx::ZERO,
                fx(-t.depth.unwrap_or(0.0)),
                fx(t.width.unwrap_or(0.0)),
                fx(t.height.unwrap_or(0.0)),
            ],
        };
        self.begin_form(name, bbox);
    }

    fn uxobj(&mut self, p: &mut Parser) {
        let Some(Obj::Named(name)) = p.object() else {
            self.doc.diag("pdf:uxobj without a name".into());
            return;
        };
        let t = read_dims(p);
        match self.doc.named.get(&name).cloned() {
            Some(Named::Form(id)) => self.draw_form(id, Mat::IDENTITY),
            Some(Named::Image(id)) => self.place_image(id, &t),
            _ => self
                .doc
                .diag(format!("pdf:uxobj: no XObject @{}", lossy(&name))),
        }
    }

    /// `pdf:image [@name] [dims] (file)`.
    fn image_special(&mut self, p: &mut Parser, draw: bool) {
        let name = if p.peek() == Some(b'@') {
            match p.object() {
                Some(Obj::Named(n)) => Some(n),
                _ => None,
            }
        } else {
            None
        };
        let t = read_dims(p);
        let Some(file) = p.literal_string() else {
            self.doc
                .diag(format!("pdf:image without a file name: `{}'", lossy(p.s)));
            return;
        };
        let f = String::from_utf8_lossy(&file).into_owned();
        let path = if std::path::Path::new(&f).is_absolute() {
            Some(f.clone())
        } else {
            flashtex_engine::system::find_file(&f, flashtex_engine::resolver::Format::Pict)
        };
        let Some(path) = path else {
            self.doc.diag(format!("pdf:image: cannot find {f}"));
            self.cur().unsupported(format!("image {f} not found"), None);
            return;
        };
        let id = match self
            .doc
            .images
            .load(&path, t.page.unwrap_or(0), t.pagebox.unwrap_or(0))
        {
            Ok(id) => id,
            Err(e) => {
                self.doc.diag(e.clone());
                self.cur().unsupported(e, None);
                return;
            }
        };
        if let Some(n) = name {
            self.doc.named.insert(n, Named::Image(id));
        }
        if draw && !t.hide {
            self.place_image(id, &t);
        }
    }

    /// Draw image `id` at the current point as `t` says (dvipdfmx's
    /// placement: the viewport, `bbox` or the image's own box, scaled to
    /// `width`/`height`, its lower left corner at the current point less
    /// `depth`, then `rotate` and `matrix` about that point; `clip 1` clips
    /// to the viewport).
    fn place_image(&mut self, id: u32, t: &Dims) {
        let Some(img) = self.doc.images.get(id).cloned() else {
            return;
        };
        let nat = img.bbox;
        let v = t.bbox.unwrap_or(nat);
        let (vw, vh) = (v[2] - v[0], v[3] - v[1]);
        let depth = t.depth.unwrap_or(0.0);
        let (mut sx, mut sy) = match (t.width, t.height) {
            (Some(w), Some(h)) if vw > 0.0 && vh > 0.0 => (w / vw, (h + depth) / vh),
            (Some(w), None) if vw > 0.0 => (w / vw, w / vw),
            (None, Some(h)) if vh > 0.0 => ((h + depth) / vh, (h + depth) / vh),
            _ => (1.0, 1.0),
        };
        sx *= t.scale.unwrap_or(1.0) * t.xscale.unwrap_or(1.0);
        sy *= t.scale.unwrap_or(1.0) * t.yscale.unwrap_or(1.0);
        // image space -> user space with the reference point at (0, 0)
        let to_ref = Mat([
            fx(sx),
            Fx::ZERO,
            Fx::ZERO,
            fx(sy),
            fx(-v[0] * sx),
            fx(-v[1] * sy - depth),
        ]);
        let mut place = to_ref;
        if let Some(r) = t.rotate {
            place = place.then(&rotation(r));
        }
        if let Some(m) = t.matrix {
            place = place.then(&Mat(m.map(fx)));
        }
        let (x, y) = self.user_point();
        let (x, y) = self.rel(x, y);
        let ctm = self.cur_ref().gs.ctm;
        let to_stream = place.then(&Mat::translate(x, y)).then(&ctm);
        let clip = t.clip.unwrap_or(false) && t.bbox.is_some();
        let s = self.cur();
        if clip {
            s.save();
            let m = s.matrix(to_stream.to_f64());
            s.dl.paths.push(Path {
                paint: paint::CLIP,
                matrix: m,
                stroke: None,
                segs: super::page::rect_segs(v[0], v[1], vw, vh),
            });
            let n = s.dl.paths.len() as u32 - 1;
            s.push(Item::Clip(n));
        }
        let m = match img.kind {
            super::images::Kind::Pdf => to_stream,
            _ => {
                // the unit square onto the natural box
                let unit = Mat([
                    fx(nat[2] - nat[0]),
                    Fx::ZERO,
                    Fx::ZERO,
                    fx(nat[3] - nat[1]),
                    fx(nat[0]),
                    fx(nat[1]),
                ]);
                unit.then(&to_stream)
            }
        };
        if img.kind == super::images::Kind::Bmp {
            s.unsupported(format!("BMP image {}", img.path), None);
        } else {
            let n = s.matrix(m.to_f64());
            s.use_image(id);
            s.push(Item::Image { id, matrix: n });
        }
        if clip {
            s.restore();
        }
        if self.doc.tracking.is_some() {
            // the image's box, placed at the current point (device space)
            let (px, py) = self.stream_point_device();
            let to_dev = place.then(&Mat::translate(px, py));
            let mut r = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
            for (cx, cy) in [(v[0], v[1]), (v[2], v[1]), (v[0], v[3]), (v[2], v[3])] {
                let (qx, qy) = to_dev.apply(fx(cx), fx(cy));
                let (qx, qy) = (qx.to_f64(), qy.to_f64());
                r = [r[0].min(qx), r[1].min(qy), r[2].max(qx), r[3].max(qy)];
            }
            self.expand_annot(r);
        }
    }
}

/// A rotation by `deg` degrees, counterclockwise (PDF space, y up).
pub fn rotation(deg: f64) -> Mat {
    let r = deg.to_radians();
    let (s, c) = r.sin_cos();
    let snap = |v: f64| if v.abs() < 1e-12 { 0.0 } else { v };
    Mat([
        fx(snap(c)),
        fx(snap(s)),
        fx(snap(-s)),
        fx(snap(c)),
        Fx::ZERO,
        Fx::ZERO,
    ])
}

/// dvipdfmx's dimension and transformation keywords.
#[derive(Clone, Debug, Default)]
pub struct Dims {
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub depth: Option<f64>,
    pub scale: Option<f64>,
    pub xscale: Option<f64>,
    pub yscale: Option<f64>,
    pub rotate: Option<f64>,
    pub bbox: Option<[f64; 4]>,
    pub matrix: Option<[f64; 6]>,
    pub clip: Option<bool>,
    pub page: Option<i32>,
    pub pagebox: Option<i32>,
    pub hide: bool,
}

pub fn read_dims(p: &mut Parser) -> Dims {
    let mut t = Dims::default();
    loop {
        let save = p.i;
        let Some(w) = p.word() else { break };
        match w {
            b"width" => t.width = p.dimension(),
            b"height" => t.height = p.dimension(),
            b"depth" => t.depth = p.dimension(),
            b"scale" => t.scale = p.number(),
            b"xscale" => t.xscale = p.number(),
            b"yscale" => t.yscale = p.number(),
            b"rotate" => t.rotate = p.number(),
            b"bbox" => {
                let v: Option<Vec<f64>> = (0..4).map(|_| p.number()).collect();
                t.bbox = v.map(|v| [v[0], v[1], v[2], v[3]]);
            }
            b"matrix" => {
                let v: Option<Vec<f64>> = (0..6).map(|_| p.number()).collect();
                t.matrix = v.map(|v| [v[0], v[1], v[2], v[3], v[4], v[5]]);
            }
            b"clip" => t.clip = p.number().map(|v| v != 0.0),
            b"page" => t.page = p.number().map(|v| v as i32),
            b"pagebox" => t.pagebox = p.word().and_then(super::images::pagebox_code),
            b"hide" => t.hide = true,
            _ => {
                p.i = save;
                break;
            }
        }
    }
    t
}

/// `pdf:btrans`'s transformation: `matrix a b c d e f`, `scale`,
/// `xscale`, `yscale`, `rotate` (in that order of application: scale, then
/// rotate, then matrix).
pub fn read_transform(p: &mut Parser) -> Mat {
    let t = read_dims(p);
    let (sx, sy) = (
        t.scale.unwrap_or(1.0) * t.xscale.unwrap_or(1.0),
        t.scale.unwrap_or(1.0) * t.yscale.unwrap_or(1.0),
    );
    let mut m = Mat([fx(sx), Fx::ZERO, Fx::ZERO, fx(sy), Fx::ZERO, Fx::ZERO]);
    if let Some(r) = t.rotate {
        m = m.then(&rotation(r));
    }
    if let Some(x) = t.matrix {
        m = m.then(&Mat(x.map(fx)));
    }
    m
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_specials() {
        let (w, h) = paper_of(b"pdf:pagesize width 614.295pt height 794.96999pt")
            .unwrap()
            .unwrap();
        assert!((w.to_f64() - 612.0).abs() < 1e-3 && (h.to_f64() - 792.0).abs() < 1e-3);
        assert_eq!(paper_of(b"pdf:pagesize default"), Some(None));
        let (w, _) = paper_of(b"papersize=597.50787pt,845.04684pt")
            .unwrap()
            .unwrap();
        assert!((w.to_f64() - 595.2756).abs() < 1e-3);
        assert_eq!(paper_of(b"pdf:dest (x) [@thispage /Fit]"), None);
    }

    #[test]
    fn colors() {
        let c = |s: &str| read_color(&mut Parser::new(s.as_bytes()));
        assert_eq!(c("rgb 1 0 0.5"), Some(Col(vec![1.0, 0.0, 0.5])));
        assert_eq!(c("cmyk 0.1 0.8 0.2 0.05").unwrap().0.len(), 4);
        assert_eq!(c("gray 0.5"), Some(Col(vec![0.5])));
        assert_eq!(c("[1]"), Some(Col(vec![1.0])));
        assert_eq!(c("[0 0.5 1]"), Some(Col(vec![0.0, 0.5, 1.0])));
        assert_eq!(c("Red"), Some(Col(vec![0.0, 1.0, 1.0, 0.0])));
        assert_eq!(c("hsb 0 1 1"), Some(Col(vec![1.0, 0.0, 0.0])));
    }

    #[test]
    fn transforms() {
        let m = read_transform(&mut Parser::new(b"rotate 30"));
        let [a, b, c, d, _, _] = m.to_f64();
        assert!((a - 0.866025).abs() < 1e-6 && (b - 0.5).abs() < 1e-9);
        assert!((c + 0.5).abs() < 1e-9 && (d - 0.866025).abs() < 1e-6);
        let m = read_transform(&mut Parser::new(
            b"matrix 1.0 0.0 0.0 1.0 88.75949 26.16994",
        ));
        assert_eq!(m.to_f64()[4], 88.75949);
    }
}
