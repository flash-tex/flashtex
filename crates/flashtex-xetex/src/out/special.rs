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

use super::doc::{Col, Named, Outline, Phantom, Tracking};
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
    // dvipdfmx writes colour components with 3 decimals (measured: xcolor's
    // `rgb 0.7843 0.3921 0` is `.784 .392 0 rg`)
    read_color_exact(p).map(|c| {
        Col(c
            .0
            .into_iter()
            .map(|v| (v * 1000.0).round() / 1000.0)
            .collect())
    })
}

fn read_color_exact(p: &mut Parser) -> Option<Col> {
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
        // measured: `color push Black` is `0 g`
        b"Black" => Some(Col(vec![0.0])),
        b"White" => Some(Col(vec![1.0])),
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
        } else if let Some(r) = t.strip_prefix(b"dvipdfmx:") {
            self.dvipdfmx_special(r);
        } else if t.starts_with(b"papersize=") {
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
        let (ux, uy) = self.user_point();
        let (x, y) = self.rel(ux, uy);
        let t = Mat::translate(-x, -y).then(&m).then(&Mat::translate(x, y));
        // dvipdfmx's `pdf_dev_concat`: its own CTM follows, about the
        // point as it has it (the same on a page)
        let (dx, dy) = self.dev_rel(ux, uy);
        let td = Mat::translate(-dx, -dy)
            .then(&m)
            .then(&Mat::translate(dx, dy));
        let s = self.cur();
        s.gs.ctm = t.then(&s.gs.ctm);
        s.gs.dev_ctm = td.then(&s.gs.dev_ctm);
    }

    fn pdf_special(&mut self, r: &[u8]) {
        let mut p = Parser::new(r);
        let Some(cmd) = p.word().map(<[u8]>::to_vec) else {
            self.doc.unknown_special("pdf:");
            return;
        };
        match cmd.as_slice() {
            b"pagesize" | b"papersize" => {} // read before the page
            // PDF 1.4 to 1.7, what the writer writes (dvipdfmx's range); a
            // version outside it is reported and clamped
            b"majorversion" => {
                if let Some(v) = p.number() {
                    if v != 1.0 {
                        self.doc
                            .diag(format!("pdf:majorversion {v}: PDF 1.x is written"));
                    }
                }
            }
            b"minorversion" => {
                if let Some(v) = p.number() {
                    let m = (v as i64).clamp(4, 7) as u32;
                    if m as f64 != v {
                        self.doc
                            .diag(format!("pdf:minorversion {v}: PDF 1.{m} is written"));
                    }
                    self.doc.version.1 = m;
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
                let (dx, dy) = self.dev_rel(x, y);
                let s = self.cur();
                s.save();
                s.coords.push((x, y));
                s.gs.ctm = Mat::translate(x - tx, y - ty).then(&s.gs.ctm);
                s.gs.dev_ctm = Mat::translate(dx, dy).then(&s.gs.dev_ctm);
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
                let Some((f, s)) = self.read_fill_stroke(&mut p) else {
                    self.doc.diag(format!("bad colour in `pdf:{}'", lossy(r)));
                    return;
                };
                self.doc.colors.push((f.clone(), s.clone()));
                self.set_colors(Some(f), Some(s));
            }
            b"scolor" | b"setcolor" => {
                let Some((f, s)) = self.read_fill_stroke(&mut p) else {
                    self.doc.diag(format!("bad colour in `pdf:{}'", lossy(r)));
                    return;
                };
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
            b"ann" | b"annot" | b"annotate" | b"annotation" => self.ann(&mut p),
            b"xann" | b"xannot" | b"extendann" => self.xann(&mut p),
            b"link" => self.doc.nolink = false,
            b"nolink" => self.doc.nolink = true,
            b"bann" | b"bannot" | b"beginann" => {
                // dvipdfmx's `spc_handler_pdfm_bann`: a link already open
                // stays open and this one is ignored
                if self.doc.tracking.is_some() {
                    self.doc
                        .diag("pdf:bann: Can't begin an annotation when one is pending.".into());
                    return;
                }
                let name = match p.peek() {
                    Some(b'@') => match p.object() {
                        Some(Obj::Named(n)) => Some(n),
                        _ => None,
                    },
                    _ => None,
                };
                let Some(Obj::Dict(d)) = p.object() else {
                    self.doc
                        .diag(format!("pdf:bann without a dictionary: `{}'", lossy(r)));
                    return;
                };
                let d = self.fixup_dict(d);
                if let Some(n) = &name {
                    self.doc
                        .named
                        .insert(n.clone(), Named::Obj(Obj::Dict(d.clone())));
                }
                self.doc.tracking = Some(Tracking {
                    dict: d,
                    depth: self.stack.len(),
                    rect: None,
                    name,
                    broken: false,
                    goto: None,
                });
            }
            b"eann" | b"eannot" | b"endann" => {
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
                // `pdf_doc_bookmarks_add` ends with `pdf_doc_add_goto`
                let goto = self.doc.add_goto(&d);
                self.doc.outlines.push(Outline {
                    level,
                    open,
                    dict: d,
                    goto,
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
            b"tounicode" | b"mapfile" | b"encrypt" => {
                self.doc.ignored_special(&format!("pdf:{}", lossy(&cmd)));
            }
            _ => self.doc.unknown_special(&format!("pdf:{}", lossy(&cmd))),
        }
    }

    /// The colours of `pdf:bcolor`/`pdf:scolor` (spc_pdfm.c): a fill colour
    /// and an optional stroke colour (the fill's when there is none), or
    /// `fill COLOUR` and `stroke COLOUR` in any order, each starting from
    /// the colour in effect (the colour stack's top) and kept where a
    /// colour does not read (`spc_util_read_pdfcolor`'s default).
    fn read_fill_stroke(&mut self, p: &mut Parser) -> Option<(Col, Col)> {
        if !matches!(p.peek(), Some(b'f' | b's')) {
            let f = read_color(p)?;
            let s = read_color(p).unwrap_or_else(|| f.clone());
            return Some((f, s));
        }
        let (mut f, mut s) = self
            .doc
            .colors
            .last()
            .cloned()
            .unwrap_or_else(|| (Col::black(), Col::black()));
        loop {
            let which = if p.keyword(b"fill") {
                &mut f
            } else if p.keyword(b"stroke") {
                &mut s
            } else {
                // dvipdfmx loops here for ever on anything else
                break;
            };
            if p.at_end() {
                return None;
            }
            if let Some(c) = read_color(p) {
                *which = c;
            }
        }
        Some((f, s))
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
                    // specials.c: the DVI position through dvipdfmx's own
                    // CTM, `ROUND(…, .01)`
                    let (x, y) = self.stream_point_device();
                    let v = if n == b"xpos" { x } else { y };
                    let v = (v.to_f64() / 0.01 + 0.5).floor() * 0.01;
                    Obj::num(v, 2)
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

    /// The current point as `@xpos`/`@ypos` give it: the DVI position
    /// (`dvi_dev_xpos`, not less `pdf:bcontent`'s origin) through
    /// dvipdfmx's own CTM.
    fn stream_point_device(&self) -> (Fx, Fx) {
        let (x, y) = self.user_point();
        self.cur_ref().gs.dev_ctm.apply(x, y)
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
            // only through the resolver: confinement (FLASHTEX_CONFINE_READS)
            // and the read set apply to it as to every other file
            let path =
                flashtex_engine::system::find_file(&f, flashtex_engine::resolver::Format::Pict);
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
        let name = match p.peek() {
            Some(b'@') => match p.object() {
                Some(Obj::Named(n)) => Some(n),
                _ => None,
            },
            _ => None,
        };
        let t = read_dims(p);
        if t.bbox.is_some() && (t.width.is_some() || t.height.is_some()) {
            self.doc
                .diag("pdf:ann: You can't specify both bbox and width/height.".into());
            return;
        }
        let Some(Obj::Dict(d)) = p.object() else {
            self.doc.diag("pdf:ann without a dictionary".into());
            return;
        };
        let d = self.fixup_dict(d);
        let r = self.annot_rect(&t);
        let goto = self.doc.add_goto(&d);
        self.add_annot(d, r, name, goto);
    }

    /// dvipdfmx's `set_rect_for_annot`: the box of `width`, `height` and
    /// `depth` (times `\mag`) or of `bbox` at the current point, through
    /// dvipdfmx's own CTM.
    fn annot_rect(&self, t: &Dims) -> [f64; 4] {
        let (x, y) = self.user_point();
        let offs = match t.bbox {
            Some(b) => [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])],
            None => {
                let mag = self.mag as f64 / 1000.0;
                super::page::rect_corners(
                    0.0,
                    -mag * t.depth.unwrap_or(0.0),
                    mag * t.width.unwrap_or(0.0),
                    mag * t.height.unwrap_or(0.0),
                )
            }
        };
        self.dev_rect(x, y, offs)
    }

    /// `pdf:xann width … height … depth …` (or `bbox …`): the box at the
    /// current point widens the link being tracked, as for a `\phantom`
    /// inside it (dvipdfmx's `spc_handler_pdfm_xann`). Outside a link it
    /// is ignored silently, as dvipdfmx ignores it.
    fn xann(&mut self, p: &mut Parser) {
        if self.doc.tracking.is_none() || self.doc.nolink {
            return;
        }
        let t = read_dims(p);
        if t.bbox.is_some() && (t.width.is_some() || t.height.is_some()) {
            self.doc
                .diag("pdf:xann: You can't specify both bbox and width/height.".into());
            return;
        }
        let r = self.annot_rect(&t);
        self.expand_annot(r);
    }

    /// `dvipdfmx:` specials (dvipdfmx's spc_dvipdfmx.c): `config`, read
    /// before the run, and `catch_phantom`.
    fn dvipdfmx_special(&mut self, r: &[u8]) {
        let mut p = Parser::new(r);
        match p.word() {
            Some(b"config") => {}
            Some(b"catch_phantom") => self.catch_phantom(&mut p),
            _ => self.doc.unknown_special("dvipdfmx:"),
        }
    }

    /// `dvipdfmx:catch_phantom true|false [height h] [depth d]`: whether a
    /// move to the right inside a link widens it ([`Phantom`]). `true`
    /// alone uses the current font's size; with a height or a depth, that
    /// box (a depth alone has height 0). It stays in effect after the link.
    fn catch_phantom(&mut self, p: &mut Parser) {
        let on = if p.keyword(b"true") {
            true
        } else if p.keyword(b"false") {
            false
        } else {
            self.doc
                .diag("dvipdfmx:catch_phantom: a boolean value expected but not found".into());
            return;
        };
        self.doc.phantom = if on { Phantom::FontSize } else { Phantom::Off };
        if !on {
            return;
        }
        let t = read_dims(p);
        if t.height.is_some() || t.depth.is_some() {
            // `phantom_height / dvi2pts` assigned to an `spt_t`: truncated
            // to DVI units (dvi2pts from XeTeX's num/den, without mag).
            let dvi2pts = 25_400_000.0 / 473_628_672.0 * (72.0 / 254_000.0);
            let spt = |v: Option<f64>| (v.unwrap_or(0.0) / dvi2pts) as i32;
            self.doc.phantom = Phantom::Box {
                height: spt(t.height),
                depth: spt(t.depth),
            };
        }
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
        if t.info().error {
            // spc_util_read_dimtrns failed: nothing is drawn
            self.doc
                .diag(format!("pdf:uxobj: invalid options: `{}'", lossy(p.s)));
            return;
        }
        match self.doc.named.get(&name).cloned() {
            Some(Named::Form(id)) => self.place_form(id, &t),
            Some(Named::Image(id)) => self.place_image(id, &t),
            _ => self
                .doc
                .diag(format!("pdf:uxobj: no XObject @{}", lossy(&name))),
        }
    }

    /// `pdf:uxobj @form [dims]`: a `pdf:bxobj` form placed as
    /// `pdf_dev_put_image` places it (`scale_to_fit_F` over the form's
    /// box as `pdf_ximage_set_form` keeps it, then the options' matrix
    /// about the current point; `clip` with a `bbox`).
    fn place_form(&mut self, id: u32, t: &Dims) {
        let Some(f) = self.doc.forms.get((id as usize).wrapping_sub(1)) else {
            return;
        };
        let ti = t.info();
        let attr = crate::pic::dpx_form_bounds(f.dl.pdf_box, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let (m1, v) = scale_to_fit_f(&ti, attr);
        let place = Mat(m1.map(fx)).then(&Mat(ti.matrix.map(fx)));
        if ti.clip {
            let (x, y) = self.user_point();
            let (x, y) = self.rel(x, y);
            let to_stream = place
                .then(&Mat::translate(x, y))
                .then(&self.cur_ref().gs.ctm);
            let s = self.cur();
            s.save();
            let m = s.matrix(to_stream.to_f64());
            s.dl.paths.push(Path {
                paint: paint::CLIP,
                matrix: m,
                stroke: None,
                segs: clip_rect_segs(v),
            });
            let n = s.dl.paths.len() as u32 - 1;
            s.push(Item::Clip(n));
        }
        self.draw_form(id, place);
        if ti.clip {
            self.cur().restore();
        }
        if self.doc.tracking.is_some() {
            // the form's box, placed at the current point (device space)
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
        if t.info().error {
            // spc_util_read_blahblah failed: "Reading option field in
            // pdf:image failed." and nothing is loaded
            self.doc
                .diag(format!("pdf:image: invalid options: `{}'", lossy(p.s)));
            return;
        }
        let Some(file) = p.literal_string() else {
            self.doc
                .diag(format!("pdf:image without a file name: `{}'", lossy(p.s)));
            return;
        };
        let f = String::from_utf8_lossy(&file).into_owned();
        // A path the engine's own picture lookup found (`\XeTeXpicfile`,
        // `\XeTeXpdffile`, which `pic_out` writes as `pdf:image`) is used as
        // it is; any other name goes through the resolver, so that
        // confinement (FLASHTEX_CONFINE_READS) and the read set apply to it.
        let path = if self.doc.pictures.contains(&f) {
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

    /// Draw image `id` at the current point as `t` says: dvipdfmx's
    /// `pdf_dev_put_image`. `pdf_ximage_scale_image` fits the image
    /// (`scale_to_fit_I` for raster images, whose unit square it maps;
    /// `scale_to_fit_F` for PDF pages, in the form's space), then the
    /// options' matrix applies about the current point; `clip` (on by
    /// default with a `bbox`) clips to the box in the fitted space.
    fn place_image(&mut self, id: u32, t: &Dims) {
        let Some(img) = self.doc.images.get(id).cloned() else {
            return;
        };
        let ti = t.info();
        let raster = img.kind != super::images::Kind::Pdf;
        let (m1, r) = if raster {
            scale_to_fit_i(&ti, &img)
        } else {
            scale_to_fit_f(&ti, img.attr_bbox)
        };
        // image space -> user space with the reference point at (0, 0)
        let place = Mat(m1.map(fx)).then(&Mat(ti.matrix.map(fx)));
        // The form's /Matrix is flashtex_pdf's (`from_pdf_page_box`):
        // xdvipdfmx's (`set_transform_matrix`) less (llx, lly) for a
        // rotated page, made up here; the clip and the link box are in
        // xdvipdfmx's space.
        let draw = if !raster && img.rotate != 0 {
            Mat::translate(fx(img.bbox[0]), fx(img.bbox[1])).then(&place)
        } else {
            place
        };
        let v = r;
        let (x, y) = self.user_point();
        let (x, y) = self.rel(x, y);
        let ctm = self.cur_ref().gs.ctm;
        let to_stream = place.then(&Mat::translate(x, y)).then(&ctm);
        let clip = ti.clip;
        let s = self.cur();
        if clip {
            s.save();
            let m = s.matrix(to_stream.to_f64());
            s.dl.paths.push(Path {
                paint: paint::CLIP,
                matrix: m,
                stroke: None,
                segs: clip_rect_segs(v),
            });
            let n = s.dl.paths.len() as u32 - 1;
            s.push(Item::Clip(n));
        }
        let m = draw.then(&Mat::translate(x, y)).then(&ctm);
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
            // the image's box through the placement and dvipdfmx's own CTM
            // (`pdf_dev_put_image`'s `pdf_dev_set_rect` after its concat)
            let dev = self.cur_ref().gs.dev_ctm;
            let (ux, uy) = self.user_point();
            let (dx, dy) = self.dev_rel(ux, uy);
            let to_dev = place.then(&Mat::translate(dx, dy)).then(&dev);
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
    /// The C's `xscale` and `yscale` after the keys in their order (`scale`
    /// sets both, `xscale`/`yscale` one), from 1.
    pub sxy: (f64, f64),
}

/// `transform_info` as `spc_util_read_blahblah` (and
/// `spc_read_dimtrns_pdfm`) leave it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Info {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    /// `INFO_HAS_WIDTH`, `INFO_HAS_HEIGHT` (set by `height` and by `depth`),
    /// `INFO_HAS_USER_BBOX`, `INFO_DO_CLIP`.
    pub has_width: bool,
    pub has_height: bool,
    pub has_bbox: bool,
    pub clip: bool,
    pub bbox: [f64; 4],
    pub matrix: [f64; 6],
    /// The consistency check failed ("Can't supply overall scale along
    /// with axis scales."): the special does nothing.
    pub error: bool,
}

impl Dims {
    /// The C's checks after the keys: `xscale` with a width, else `yscale`
    /// with a height or depth, is ignored; else `scale` with an axis scale
    /// is an error. `matrix` replaces scales and rotation
    /// (`make_transmatrix` only without it). Clipping is on by default,
    /// and only with a `bbox`.
    pub fn info(&self) -> Info {
        let has_width = self.width.is_some();
        let has_height = self.height.is_some() || self.depth.is_some();
        let (mut xs, mut ys) = self.sxy;
        let mut error = false;
        if self.xscale.is_some() && has_width {
            xs = 1.0;
        } else if self.yscale.is_some() && has_height {
            ys = 1.0;
        } else if self.scale.is_some() && (self.xscale.is_some() || self.yscale.is_some()) {
            error = true;
        }
        let matrix = match self.matrix {
            Some(m) => m,
            None => {
                // make_transmatrix(M, 0, 0, xscale, yscale, M_PI * rotate / 180)
                let r = std::f64::consts::PI * self.rotate.unwrap_or(0.0) / 180.0;
                let (c, s) = (r.cos(), r.sin());
                [xs * c, xs * s, -ys * s, ys * c, 0.0, 0.0]
            }
        };
        Info {
            width: self.width.unwrap_or(0.0),
            height: self.height.unwrap_or(0.0),
            depth: self.depth.unwrap_or(0.0),
            has_width,
            has_height,
            has_bbox: self.bbox.is_some(),
            clip: self.clip.unwrap_or(true) && self.bbox.is_some(),
            bbox: self.bbox.unwrap_or([0.0; 4]),
            matrix,
            error,
        }
    }
}

/// `pdf_dev_rectclip(r.llx, r.lly, r.urx - r.llx, r.ury - r.lly)` as
/// `pdf_dev__rectshape` writes it: the corner and the two lengths, each
/// to the device's 3 decimals, in the fitted space (a raster image's unit
/// square, where the rounding shows at the image's size).
fn clip_rect_segs(v: [f64; 4]) -> Vec<flashtex_display_list::page::Seg> {
    let r = |x: f64| super::num::round_num(x, 3);
    super::page::rect_segs(r(v[0]), r(v[1]), r(v[2] - v[0]), r(v[3] - v[1]))
}

/// `scale_to_fit_I` (pdfximage.c): the matrix that maps a raster image's
/// unit square, and the clip rectangle in that square. Natural sizes come
/// from the pixels and the density; a width or height alone keeps the
/// *pixel* aspect ratio, as the C does.
pub fn scale_to_fit_i(p: &Info, img: &super::images::Image) -> ([f64; 6], [f64; 4]) {
    let (w_px, h_px) = (img.pixels.0 as f64, img.pixels.1 as f64);
    // I->attr.width * I->attr.xdensity
    let (nat_w, nat_h) = (img.bbox[2] - img.bbox[0], img.bbox[3] - img.bbox[1]);
    let (mut wd0, mut ht0, xscale, yscale, d_x, d_y);
    if p.has_bbox {
        wd0 = p.bbox[2] - p.bbox[0];
        ht0 = p.bbox[3] - p.bbox[1];
        xscale = nat_w / wd0;
        yscale = nat_h / ht0;
        d_x = -p.bbox[0] / wd0;
        d_y = -p.bbox[1] / ht0;
    } else {
        wd0 = nat_w;
        ht0 = nat_h;
        xscale = 1.0;
        yscale = 1.0;
        d_x = 0.0;
        d_y = 0.0;
    }
    if wd0 == 0.0 {
        wd0 = 1.0;
    }
    if ht0 == 0.0 {
        ht0 = 1.0;
    }
    let (s_x, s_y, dp);
    if p.has_width && p.has_height {
        s_x = p.width * xscale;
        s_y = (p.height + p.depth) * yscale;
        dp = p.depth * yscale;
    } else if p.has_width {
        s_x = p.width * xscale;
        s_y = s_x * (h_px / w_px);
        dp = 0.0;
    } else if p.has_height {
        s_y = (p.height + p.depth) * yscale;
        s_x = s_y * (w_px / h_px);
        dp = p.depth * yscale;
    } else {
        s_x = wd0;
        s_y = ht0;
        dp = 0.0;
    }
    let m = [
        s_x,
        0.0,
        0.0,
        s_y,
        d_x * s_x / xscale,
        d_y * s_y / yscale - dp,
    ];
    let r = if p.has_bbox {
        [
            p.bbox[0] / nat_w,
            p.bbox[1] / nat_h,
            p.bbox[2] / nat_w,
            p.bbox[3] / nat_h,
        ]
    } else {
        [0.0, 0.0, 1.0, 1.0]
    };
    (m, r)
}

/// `scale_to_fit_F` (pdfximage.c): the matrix for a form (a PDF page, or a
/// `pdf:bxobj` form) whose box is `attr` (`I->attr.bbox`), and the clip
/// rectangle in the form's space.
pub fn scale_to_fit_f(p: &Info, attr: [f64; 4]) -> ([f64; 6], [f64; 4]) {
    let (mut wd0, mut ht0, d_x, d_y);
    if p.has_bbox {
        wd0 = p.bbox[2] - p.bbox[0];
        ht0 = p.bbox[3] - p.bbox[1];
        d_x = -p.bbox[0];
        d_y = -p.bbox[1];
    } else {
        wd0 = attr[2] - attr[0];
        ht0 = attr[3] - attr[1];
        d_x = 0.0;
        d_y = 0.0;
    }
    if wd0 == 0.0 {
        wd0 = 1.0;
    }
    if ht0 == 0.0 {
        ht0 = 1.0;
    }
    let (s_x, s_y, dp);
    if p.has_width && p.has_height {
        s_x = p.width / wd0;
        s_y = (p.height + p.depth) / ht0;
        dp = p.depth;
    } else if p.has_width {
        s_x = p.width / wd0;
        s_y = s_x;
        dp = 0.0;
    } else if p.has_height {
        s_y = (p.height + p.depth) / ht0;
        s_x = s_y;
        dp = p.depth;
    } else {
        s_x = 1.0;
        s_y = 1.0;
        dp = 0.0;
    }
    let m = [s_x, 0.0, 0.0, s_y, s_x * d_x, s_y * d_y - dp];
    let r = if p.has_bbox { p.bbox } else { attr };
    (m, r)
}

pub fn read_dims(p: &mut Parser) -> Dims {
    let mut t = Dims {
        sxy: (1.0, 1.0),
        ..Dims::default()
    };
    loop {
        let save = p.i;
        let Some(w) = p.word() else { break };
        match w {
            b"width" => t.width = p.dimension(),
            b"height" => t.height = p.dimension(),
            b"depth" => t.depth = p.dimension(),
            b"scale" => {
                t.scale = p.number();
                if let Some(v) = t.scale {
                    t.sxy = (v, v);
                }
            }
            b"xscale" => {
                t.xscale = p.number();
                if let Some(v) = t.xscale {
                    t.sxy.0 = v;
                }
            }
            b"yscale" => {
                t.yscale = p.number();
                if let Some(v) = t.yscale {
                    t.sxy.1 = v;
                }
            }
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
            // strcasecmp; no box name: pdf_page_boundary__auto
            b"pagebox" => {
                t.pagebox = p
                    .word()
                    .map(|w| super::images::pagebox_code(&w.to_ascii_lowercase()).unwrap_or(0))
            }
            b"hide" => t.hide = true,
            _ => {
                p.i = save;
                break;
            }
        }
    }
    t
}

/// `pdf:btrans`'s transformation: `ti.matrix` of `spc_read_dimtrns_pdfm`
/// ([`Dims::info`]): `matrix a b c d e f` if given, else the scales and
/// `rotate` (`make_transmatrix`).
pub fn read_transform(p: &mut Parser) -> Mat {
    Mat(read_dims(p).info().matrix.map(fx))
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

    /// A 7 x 5 pixel raster image at `dpi`.
    fn raster(dpi: (f64, f64)) -> super::super::images::Image {
        let (w, h) = (7.0 * 72.0 / dpi.0, 5.0 * 72.0 / dpi.1);
        super::super::images::Image {
            id: 1,
            key: [0; 32],
            kind: super::super::images::Kind::Png,
            path: String::new(),
            bbox: [0.0, 0.0, w, h],
            pixels: (7, 5),
            res: dpi,
            page: 1,
            page_box: "crop",
            rotate: 0,
            matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            attr_bbox: [0.0, 0.0, w, h],
            info: flashtex_display_list::json::Json::Null,
        }
    }

    fn info(s: &str) -> Info {
        read_dims(&mut Parser::new(s.as_bytes())).info()
    }

    fn close(a: &[f64], b: &[f64], tol: f64) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
    }

    /// Expected values: TeX Live 2026 xetex -ini + xdvipdfmx 20260317
    /// (macOS arm64), each `\special{pdf:image ...}` on its own page, the
    /// image's `cm` and clip `re` read from the PDF (lane AUDIT-DVIPDFMX).
    #[test]
    fn pdf_image_fits_as_xdvipdfmx() {
        let img = raster((72.0, 72.0));
        let fit = |s: &str, img: &super::super::images::Image| {
            let ti = info(s);
            let (m1, r) = scale_to_fit_i(&ti, img);
            let m = Mat(m1.map(fx)).then(&Mat(ti.matrix.map(fx))).to_f64();
            (m, r, ti)
        };
        // `bbox` without a size: the whole image at the box's size,
        // clipped by default ("q 3 0 0 2 -.429 -.4 cm n .143 .2 .429 .4 re W n")
        let (m, r, ti) = fit("bbox 1 1 4 3", &img);
        assert!(
            close(&m, &[3.0, 0.0, 0.0, 2.0, -0.428571, -0.4], 1e-5),
            "{m:?}"
        );
        assert!(ti.clip);
        let segs = clip_rect_segs(r);
        assert_eq!(
            segs,
            super::super::page::rect_segs(0.143, 0.2, 0.429, 0.4),
            "the rectangle as `.143 .2 .429 .4 re`"
        );
        // `clip 0` turns it off
        assert!(!info("bbox 1 1 4 3 clip 0").clip);
        // no bbox: no clip
        assert!(!info("width 3pt").clip);
        // `depth` alone is a height: "13.948 0 0 9.963 ... -9.963"
        let (m, _, _) = fit("depth 10pt", &img);
        assert!(
            close(&m, &[13.9477, 0.0, 0.0, 9.9626, 0.0, -9.9626], 1e-3),
            "{m:?}"
        );
        // `xscale` with `width` is ignored: "29.888 0 0 21.349"
        let (m, _, _) = fit("width 30pt xscale 2", &img);
        assert!(
            close(&m, &[29.888, 0.0, 0.0, 21.349, 0.0, 0.0], 1e-3),
            "{m:?}"
        );
        // `scale` with `xscale` is an error: no image
        assert!(info("scale 2 xscale 3").error);
        // `matrix` replaces `scale`: "7 0 0 5 5 6"
        let (m, _, _) = fit("matrix 1 0 0 1 5 6 scale 2", &img);
        assert!(close(&m, &[7.0, 0.0, 0.0, 5.0, 5.0, 6.0], 1e-9), "{m:?}");
        // a width alone keeps the pixel aspect ratio, not the density's:
        // 144 x 300 dpi, "49.8132 0 0 35.58086"
        let dpi = raster((144.0, 300.0));
        let (m, _, _) = fit("width 50pt", &dpi);
        assert!(
            close(&m, &[49.8132, 0.0, 0.0, 35.58086, 0.0, 0.0], 1e-4),
            "{m:?}"
        );
        // and a height alone: "69.73848 0 0 49.8132"
        let (m, _, _) = fit("height 50pt", &dpi);
        assert!(
            close(&m, &[69.73848, 0.0, 0.0, 49.8132, 0.0, 0.0], 1e-4),
            "{m:?}"
        );
    }

    /// Forms (`scale_to_fit_F`): a `pdf:bxobj @f1 bbox 10 10 80 50` form
    /// placed `height 40bp` is scaled by 40/50, not 40/40, because
    /// `pdf_ximage_set_form` never sets its second corner's y (measured 0
    /// on the ruling build); `width 35bp` by 35/70. A PDF page's /Rotate
    /// 270 over CropBox [10 10 80 50]: /Matrix [0 1 -1 0 60 0].
    #[test]
    fn forms_fit_as_xdvipdfmx() {
        let id = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let attr = crate::pic::dpx_form_bounds([10.0, 10.0, 80.0, 50.0], id);
        assert_eq!(attr, [10.0, 0.0, 80.0, 50.0]);
        let (m, r) = scale_to_fit_f(&info("height 40bp"), attr);
        assert!(close(&m, &[0.8, 0.0, 0.0, 0.8, 0.0, 0.0], 1e-12), "{m:?}");
        assert_eq!(r, attr);
        let (m, _) = scale_to_fit_f(&info("width 35bp"), attr);
        assert!(close(&m, &[0.5, 0.0, 0.0, 0.5, 0.0, 0.0], 1e-12), "{m:?}");
        // a user bbox moves the form by its corner: [1 0 0 1 -0 -0]
        let (m, r) = scale_to_fit_f(&info("bbox 0 0 10 10"), attr);
        assert!(close(&m, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 1e-12), "{m:?}");
        assert_eq!(r, [0.0, 0.0, 10.0, 10.0]);
        let rot = [0.0, 1.0, -1.0, 0.0, 60.0, 0.0];
        assert_eq!(
            crate::pic::dpx_form_bounds([10.0, 10.0, 80.0, 50.0], rot),
            [10.0, 10.0, 60.0, 80.0]
        );
    }
}
