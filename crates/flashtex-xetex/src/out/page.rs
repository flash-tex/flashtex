//! One page of XeTeX's output stream into a display-list page.
//!
//! `ship_out` writes each page as XDV (`xetex.web`'s own DVI code, extended
//! by XeTeX with `define_native_font`, `set_glyphs` and
//! `set_text_and_glyphs`); everything else a page holds arrives as
//! `\special`s in dvipdfmx's language (PLAN.md §3.2). This module reads a
//! page's bytes, from its `bop` to its `eop`, as dvipdfmx reads them, and
//! builds the display-list page (positioned glyphs, rules, paths, images,
//! forms, links, destinations) plus, in a [`Supplement`], what the PDF
//! needs and the display list cannot carry.
//!
//! **Coordinates.** A DVI position (h, v) is, in dvipdfmx's user space,
//! (h·k, −v·k) bp, k the size of a DVI unit (1/65781.76 bp times `\mag`
//! over 1000); the page's content starts with dvipdfmx's
//! `1 0 0 1 72 H−72 cm` (TeX's origin one inch from the top left). The
//! current transformation matrix (CTM) is changed by the specials that
//! write PDF operators (`pdf:code`, `pdf:btrans`, `x:scale`, ...), and
//! `pdf:bcontent` moves the origin of later text to the current point;
//! every glyph, rule and image is placed through them, as the PDF draws
//! it. Positions are exact decimals ([`Fx`], 10⁻¹² bp), rounded once to
//! scaled points for the display list (spec §4.2).

use super::doc::{Annot, Built, Col, Doc, Raw, Supplement};
use super::fonts::{
    FontDef, NativeDef, TfmDef, XDV_FLAG_COLORED, XDV_FLAG_EMBOLDEN, XDV_FLAG_EXTEND,
    XDV_FLAG_SLANT, XDV_FLAG_VERTICAL,
};
use super::pdfobj::Obj;
use flashtex_display_list::page::{
    flags, paint, Color, Dest, Item, Link, LinkKind, Page, Path, RuleKind, Seg, StreamKind, Stroke,
    NO_COLUMN,
};
use flashtex_engine::displaylist::fixed::{Fx, Mat, ONE};
use std::collections::HashMap;

/// What the page reader needs of the engine: a TFM font's character
/// dimensions (DVI font number `k`, character `c`): width, height, depth in
/// DVI units.
pub trait Metrics {
    fn tfm_char(&self, k: i32, c: u32) -> Option<(i32, i32, i32)>;
}

/// The graphics state of the stream being built.
#[derive(Clone, Debug)]
pub struct GState {
    pub ctm: Mat,
    pub fill: Col,
    pub stroke: Col,
    pub fill_alpha: f64,
    pub stroke_alpha: f64,
    pub line_width: f64,
    pub cap: u8,
    pub join: u8,
    pub miter: f64,
    pub dash: Vec<f64>,
    pub phase: f64,
    /// A fill or stroke colour the display list cannot express (a pattern,
    /// a separation) is in effect.
    pub odd_fill: bool,
    pub odd_stroke: bool,
    // What the display list's reader has been told; SAVE/RESTORE restore
    // these with the state (spec §4.3).
    pub told_fill: Col,
    pub told_stroke: Col,
    pub told_fill_alpha: f64,
    pub told_stroke_alpha: f64,
    pub told_tr: u8,
    /// The line state stroked glyphs use (spec §11.4), once told.
    pub told_line: Option<Stroke>,
}

impl GState {
    fn new(ctm: Mat) -> GState {
        GState {
            ctm,
            fill: Col::black(),
            stroke: Col::black(),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            line_width: 1.0,
            cap: 0,
            join: 0,
            miter: 10.0,
            dash: Vec::new(),
            phase: 0.0,
            odd_fill: false,
            odd_stroke: false,
            told_fill: Col::black(),
            told_stroke: Col::black(),
            told_fill_alpha: 1.0,
            told_stroke_alpha: 1.0,
            told_tr: 0,
            told_line: None,
        }
    }

    pub fn stroke_params(&self) -> Stroke {
        Stroke {
            width: self.line_width,
            cap: self.cap,
            join: self.join,
            miter: self.miter,
            dash: self.dash.clone(),
            phase: self.phase,
        }
    }
}

/// A page or form being built.
pub struct Stream {
    pub dl: Page,
    pub sup: Supplement,
    mats: HashMap<[u64; 6], u32>,
    pub gs: GState,
    pub stack: Vec<GState>,
    /// `pdf:bcontent`'s coordinate stack (dvipdfmx user space, bp).
    pub coords: Vec<(Fx, Fx)>,
    /// The box height in bp: page-space y is `height − Y` (spec §4.2).
    pub height: Fx,
    /// The glyph matrix the reader has (`u32::MAX`: none yet, spec §4.3).
    told_matrix: u32,
    pub fonts: Vec<u16>,
    pub images: Vec<u32>,
    pub forms: Vec<u32>,
    /// The current path of literal PDF operators (user space).
    pub path: Vec<Seg>,
    pub pending_clip: Option<u8>,
    /// Forms: the name, and the user-space point the form's origin is at.
    pub form_name: Option<Vec<u8>>,
    pub form_origin: (Fx, Fx),
}

impl Stream {
    pub fn new(kind: StreamKind, index: u32, height: Fx, ctm: Mat) -> Stream {
        Stream {
            dl: Page::new(kind, index),
            sup: Supplement::default(),
            mats: HashMap::new(),
            gs: GState::new(ctm),
            stack: Vec::new(),
            coords: Vec::new(),
            height,
            told_matrix: u32::MAX,
            fonts: Vec::new(),
            images: Vec::new(),
            forms: Vec::new(),
            path: Vec::new(),
            pending_clip: None,
            form_name: None,
            form_origin: (Fx::ZERO, Fx::ZERO),
        }
    }

    /// The index of matrix `m` in MATRICES (0 for the identity).
    pub fn matrix(&mut self, m: [f64; 6]) -> u32 {
        if m == [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] {
            return 0;
        }
        let k = m.map(f64::to_bits);
        if let Some(&n) = self.mats.get(&k) {
            return n;
        }
        self.dl.matrices.push(m);
        let n = self.dl.matrices.len() as u32;
        self.mats.insert(k, n);
        n
    }

    pub fn push(&mut self, it: Item) {
        self.dl.items.push(it);
    }

    pub fn save(&mut self) {
        self.stack.push(self.gs.clone());
        self.push(Item::Save);
    }

    /// `Q`: false (and nothing done) when there is nothing to restore.
    pub fn restore(&mut self) -> bool {
        match self.stack.pop() {
            Some(g) => {
                self.gs = g;
                self.push(Item::Restore);
                true
            }
            None => false,
        }
    }

    /// Mark the stream incomplete with `what` (spec §4.7) and put an
    /// UNSUPPORTED item here, whose supplement entry is `raw`.
    pub fn unsupported(&mut self, what: String, raw: Option<Raw>) {
        self.dl.flags |= flags::INCOMPLETE;
        let n = self.dl.unsupported.len() as u32;
        self.dl.unsupported.push(what);
        self.sup.raw.push(raw);
        self.push(Item::Unsupported(n));
    }

    /// Tell the reader the fill colour and alpha `fill`/`alpha` (those of
    /// the state unless given) before something is painted with them.
    pub fn sync_fill(&mut self, fill: Option<&Col>, alpha: Option<f64>) {
        let want = fill.cloned().unwrap_or_else(|| self.gs.fill.clone());
        if want != self.gs.told_fill {
            self.push(Item::FillColor(Color(want.0.clone())));
            self.gs.told_fill = want;
        }
        let a = alpha.unwrap_or(self.gs.fill_alpha);
        if a != self.gs.told_fill_alpha {
            self.push(Item::FillAlpha(a));
            self.gs.told_fill_alpha = a;
        }
    }

    pub fn sync_stroke(&mut self) {
        if self.gs.stroke != self.gs.told_stroke {
            self.push(Item::StrokeColor(Color(self.gs.stroke.0.clone())));
            self.gs.told_stroke = self.gs.stroke.clone();
        }
        if self.gs.stroke_alpha != self.gs.told_stroke_alpha {
            self.push(Item::StrokeAlpha(self.gs.stroke_alpha));
            self.gs.told_stroke_alpha = self.gs.stroke_alpha;
        }
    }

    fn sync_text_render(&mut self, mode: u8) {
        if mode != self.gs.told_tr {
            self.push(Item::TextRender(mode));
            self.gs.told_tr = mode;
        }
    }

    fn sync_glyph_matrix(&mut self, m: [f64; 4]) {
        let n = self.matrix([m[0], m[1], m[2], m[3], 0.0, 0.0]);
        if n != self.told_matrix {
            self.push(Item::Matrix(n));
            self.told_matrix = n;
        }
    }

    /// Stream space (bp, y up) to page space (sp, y down): spec §4.2.
    pub fn to_page(&self, x: Fx, y: Fx) -> (i32, i32) {
        (clamp_i32(x.to_sp()), clamp_i32((self.height - y).to_sp()))
    }

    fn use_font(&mut self, id: u16) {
        if !self.fonts.contains(&id) {
            self.fonts.push(id);
        }
    }

    pub fn use_image(&mut self, id: u32) {
        if !self.images.contains(&id) {
            self.images.push(id);
        }
    }

    pub fn use_form(&mut self, id: u32) {
        if !self.forms.contains(&id) {
            self.forms.push(id);
        }
    }

    pub fn finish(self) -> Built {
        Built {
            dl: self.dl,
            sup: self.sup,
            fonts: self.fonts,
            images: self.images,
            forms: self.forms,
        }
    }
}

fn clamp_i32(v: i64) -> i32 {
    v.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// The DVI registers.
#[derive(Clone, Copy, Debug, Default)]
pub struct Regs {
    pub h: i32,
    pub v: i32,
    pub w: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// The builder of one page.
pub struct Builder<'a, M: Metrics> {
    pub doc: &'a mut Doc,
    pub metrics: &'a M,
    /// bp per DVI unit, as the fraction num/den: `mag` · 100 /
    /// (6578176 · 1000).
    mag: i32,
    pub r: Regs,
    pub stack: Vec<Regs>,
    pub font: Option<i32>,
    /// The page at the bottom, a form being defined (`pdf:bxobj`) above.
    pub streams: Vec<Stream>,
    /// 1-based page number.
    pub page_no: u32,
    pub page_width: Fx,
    pub page_height: Fx,
}

/// dvipdfmx's default page origin: one inch from the left and from the
/// top (its `-x 1in -y 1in`).
pub const ORIGIN: i128 = 72 * ONE;

impl<'a, M: Metrics> Builder<'a, M> {
    /// Build page `page_no` (1-based) from its bytes, `bop` to `eop`.
    pub fn page(
        doc: &'a mut Doc,
        metrics: &'a M,
        mag: i32,
        page_no: u32,
        bytes: &[u8],
    ) -> Result<Built, String> {
        let (w, h) = page_size(doc, bytes);
        let mut b = Builder {
            doc,
            metrics,
            mag: if mag > 0 { mag } else { 1000 },
            r: Regs::default(),
            stack: Vec::new(),
            font: None,
            streams: Vec::new(),
            page_no,
            page_width: w,
            page_height: h,
        };
        // dvipdfmx's MediaBox is the size to 0.01 bp; its content starts
        // with `q 1 0 0 1 72 H-72 cm`, H the size itself (measured).
        let (bw, bh) = (round_to(w, 2), round_to(h, 2));
        let mut page = Stream::new(StreamKind::Page, page_no - 1, bh, Mat::IDENTITY);
        page.save();
        page.gs.ctm = Mat::translate(Fx(ORIGIN), h - Fx(ORIGIN));
        page.dl.pdf_box = [0.0, 0.0, bw.to_f64(), bh.to_f64()];
        page.dl.width = clamp_i32(w.to_sp());
        page.dl.height = clamp_i32(h.to_sp());
        b.streams.push(page);
        // The colour in effect (dvipdfmx's colour stack lasts across pages).
        if let Some((f, s)) = b.doc.colors.last().cloned() {
            b.set_colors(Some(f), Some(s));
        }
        b.run(bytes)?;
        // An annotation still open at the end of the page breaks here and
        // continues on the next page.
        b.break_annot();
        // A form left open by a page's end is closed with it.
        while b.streams.len() > 1 {
            b.doc.diag("pdf:bxobj without pdf:exobj on the page".into());
            b.end_form(None);
        }
        let page = b.streams.pop().expect("the page");
        Ok(page.finish())
    }

    pub fn cur(&mut self) -> &mut Stream {
        self.streams.last_mut().expect("a stream")
    }

    pub fn cur_ref(&self) -> &Stream {
        self.streams.last().expect("a stream")
    }

    /// DVI units to bp (exact to 10⁻¹² bp).
    pub fn dvi_to_bp(&self, d: i32) -> Fx {
        let n = d as i128 * ONE * 100 * self.mag as i128;
        Fx(super::div_round(n, 6_578_176 * 1000))
    }

    /// The current point in dvipdfmx's user space (bp, relative to the
    /// page's origin, before any `pdf:bcontent`).
    pub fn user_point(&self) -> (Fx, Fx) {
        (self.dvi_to_bp(self.r.h), -self.dvi_to_bp(self.r.v))
    }

    /// The top of `pdf:bcontent`'s coordinate stack (0, 0 if empty), or a
    /// form's origin.
    pub fn coord_top(&self) -> (Fx, Fx) {
        let s = self.cur_ref();
        s.coords.last().copied().unwrap_or(s.form_origin)
    }

    /// User-space point (x, y) as the current CTM's input: relative to the
    /// coordinate stack's top.
    pub fn rel(&self, x: Fx, y: Fx) -> (Fx, Fx) {
        let (tx, ty) = self.coord_top();
        (x - tx, y - ty)
    }

    /// The current point in stream space.
    pub fn stream_point(&self) -> (Fx, Fx) {
        let (x, y) = self.user_point();
        let (x, y) = self.rel(x, y);
        self.cur_ref().gs.ctm.apply(x, y)
    }

    /// The current point in page space without the transformations of
    /// the specials (dvipdfmx's device coordinates): for link rectangles
    /// and destinations.
    fn device_point(&self, dx: i32, dy: i32) -> (Fx, Fx) {
        let (x, y) = (
            self.dvi_to_bp(self.r.h + dx),
            -self.dvi_to_bp(self.r.v + dy),
        );
        (x + Fx(ORIGIN), self.page_height - Fx(ORIGIN) + y)
    }

    // ---- the XDV stream ---------------------------------------------------

    fn run(&mut self, b: &[u8]) -> Result<(), String> {
        let mut i = 0usize;
        let rd = |i: &mut usize, n: usize| -> Result<u32, String> {
            let s = b.get(*i..*i + n).ok_or("XDV page truncated")?;
            *i += n;
            Ok(s.iter().fold(0u32, |a, &c| (a << 8) | c as u32))
        };
        let rds = |i: &mut usize, n: usize| -> Result<i32, String> {
            let s = b.get(*i..*i + n).ok_or("XDV page truncated")?;
            *i += n;
            let mut v: i32 = if s[0] & 0x80 != 0 { -1 } else { 0 };
            for &c in s {
                v = (v << 8) | c as i32;
            }
            Ok(v)
        };
        while i < b.len() {
            let op = b[i];
            i += 1;
            match op {
                0..=127 => self.set_char(op as u32, true),
                128..=131 => {
                    let c = rd(&mut i, (op - 127) as usize)?;
                    self.set_char(c, true);
                }
                132 | 137 => {
                    let a = rds(&mut i, 4)?;
                    let w = rds(&mut i, 4)?;
                    self.rule(a, w);
                    if op == 132 {
                        self.r.h = self.r.h.wrapping_add(w);
                    }
                }
                133..=136 => {
                    let c = rd(&mut i, (op - 132) as usize)?;
                    self.set_char(c, false);
                }
                138 => {}
                139 => {
                    i += 44; // c0..c9, p
                    self.r = Regs::default();
                    self.stack.clear();
                    self.font = None;
                }
                140 => return Ok(()),
                141 => self.stack.push(self.r),
                142 => {
                    if let Some(r) = self.stack.pop() {
                        self.r = r;
                    }
                    self.after_pop();
                }
                143..=146 => {
                    let d = rds(&mut i, (op - 142) as usize)?;
                    self.r.h = self.r.h.wrapping_add(d);
                }
                147 => self.r.h = self.r.h.wrapping_add(self.r.w),
                148..=151 => {
                    self.r.w = rds(&mut i, (op - 147) as usize)?;
                    self.r.h = self.r.h.wrapping_add(self.r.w);
                }
                152 => self.r.h = self.r.h.wrapping_add(self.r.x),
                153..=156 => {
                    self.r.x = rds(&mut i, (op - 152) as usize)?;
                    self.r.h = self.r.h.wrapping_add(self.r.x);
                }
                157..=160 => {
                    let d = rds(&mut i, (op - 156) as usize)?;
                    self.r.v = self.r.v.wrapping_add(d);
                }
                161 => self.r.v = self.r.v.wrapping_add(self.r.y),
                162..=165 => {
                    self.r.y = rds(&mut i, (op - 161) as usize)?;
                    self.r.v = self.r.v.wrapping_add(self.r.y);
                }
                166 => self.r.v = self.r.v.wrapping_add(self.r.z),
                167..=170 => {
                    self.r.z = rds(&mut i, (op - 166) as usize)?;
                    self.r.v = self.r.v.wrapping_add(self.r.z);
                }
                171..=234 => self.font = Some((op - 171) as i32),
                235..=238 => self.font = Some(rd(&mut i, (op - 234) as usize)? as i32),
                239..=242 => {
                    let k = rd(&mut i, (op - 238) as usize)? as usize;
                    let s = b.get(i..i + k).ok_or("XDV special truncated")?;
                    i += k;
                    self.special(s);
                }
                243..=246 => {
                    let k = rd(&mut i, (op - 242) as usize)? as i32;
                    let checksum = rd(&mut i, 4)?;
                    let size = rds(&mut i, 4)?;
                    let dsize = rds(&mut i, 4)?;
                    let a = rd(&mut i, 1)? as usize;
                    let l = rd(&mut i, 1)? as usize;
                    let n = b.get(i..i + a + l).ok_or("XDV fnt_def truncated")?;
                    i += a + l;
                    self.doc.fonts.define(
                        k,
                        FontDef::Tfm(TfmDef {
                            checksum,
                            size,
                            dsize,
                            name: n[a..].to_vec(),
                        }),
                    );
                }
                247 => {
                    // pre: i[1] num[4] den[4] mag[4] k[1] x[k]
                    i += 13;
                    let k = rd(&mut i, 1)? as usize;
                    i += k;
                }
                252 => {
                    let k = rd(&mut i, 4)? as i32;
                    let size = rds(&mut i, 4)?;
                    let fl = rd(&mut i, 2)? as u16;
                    let l = rd(&mut i, 1)? as usize;
                    let path = b
                        .get(i..i + l)
                        .ok_or("XDV define_native_font truncated")?
                        .to_vec();
                    i += l;
                    let index = rd(&mut i, 4)?;
                    let rgba = if fl & XDV_FLAG_COLORED != 0 {
                        Some(rd(&mut i, 4)?)
                    } else {
                        None
                    };
                    let extend = if fl & XDV_FLAG_EXTEND != 0 {
                        Some(rds(&mut i, 4)?)
                    } else {
                        None
                    };
                    let slant = if fl & XDV_FLAG_SLANT != 0 {
                        Some(rds(&mut i, 4)?)
                    } else {
                        None
                    };
                    let embolden = if fl & XDV_FLAG_EMBOLDEN != 0 {
                        Some(rds(&mut i, 4)?)
                    } else {
                        None
                    };
                    self.doc.fonts.define(
                        k,
                        FontDef::Native(NativeDef {
                            path,
                            index,
                            size,
                            flags: fl,
                            rgba,
                            extend,
                            slant,
                            embolden,
                        }),
                    );
                }
                253 | 254 => {
                    if op == 254 {
                        let l = rd(&mut i, 2)? as usize;
                        i += 2 * l;
                    }
                    let w = rds(&mut i, 4)?;
                    let n = rd(&mut i, 2)? as usize;
                    let mut xy = Vec::with_capacity(n);
                    for _ in 0..n {
                        let x = rds(&mut i, 4)?;
                        let y = rds(&mut i, 4)?;
                        xy.push((x, y));
                    }
                    let mut gids = Vec::with_capacity(n);
                    for _ in 0..n {
                        gids.push(rd(&mut i, 2)? as u16);
                    }
                    self.set_glyphs(&xy, &gids);
                    self.r.h = self.r.h.wrapping_add(w);
                }
                248 => return Ok(()), // post: past the last page
                _ => return Err(format!("XDV opcode {op} in a page")),
            }
        }
        Ok(())
    }

    // ---- glyphs and rules -------------------------------------------------

    /// The glyph matrix for a font of `size` bp with `extend` and `slant`,
    /// under the current CTM: `[size·e 0 size·s size] × CTM` (linear part).
    fn glyph_matrix(&self, size: Fx, extend: f64, slant: f64) -> [f64; 4] {
        let s = size.to_f64();
        let (a, c, d) = (s * extend, s * slant, s);
        let [ma, mb, mc, md, _, _] = self.cur_ref().gs.ctm.to_f64();
        [a * ma, a * mb, c * ma + d * mc, c * mb + d * md]
    }

    fn set_char(&mut self, c: u32, advance: bool) {
        let Some(k) = self.font else {
            return;
        };
        let Some(l) = self.doc.fonts.get(k).cloned() else {
            return;
        };
        let FontDef::Tfm(t) = &l.def else {
            // a native font's characters come as glyphs
            return;
        };
        let (wd, ht, dp) = self.metrics.tfm_char(k, c).unwrap_or((0, 0, 0));
        if let Some(p) = &l.problem {
            let p = p.clone();
            self.cur().unsupported(p, None);
        } else {
            if t.size > 0 {
                let w = wd as f64 / t.size as f64 * 1000.0;
                self.doc
                    .fonts
                    .widths
                    .entry(l.res)
                    .or_default()
                    .entry(c.min(0xFFFF) as u16)
                    .or_insert(w);
            }
            let size = self.dvi_to_bp(t.size);
            let m = self.glyph_matrix(size, 1.0, 0.0);
            let (x, y) = self.stream_point();
            let s = self.cur();
            s.sync_fill(None, None);
            s.sync_text_render(0);
            s.sync_glyph_matrix(m);
            s.use_font(l.res);
            let (px, py) = s.to_page(x, y);
            s.push(Item::Glyph {
                font: l.res,
                code: c.min(0xFFFF) as u16,
                x: px,
                y: py,
                col: NO_COLUMN,
            });
        }
        self.track(0, wd, ht, dp);
        if advance {
            self.r.h = self.r.h.wrapping_add(wd);
        }
    }

    fn set_glyphs(&mut self, xy: &[(i32, i32)], gids: &[u16]) {
        let Some(k) = self.font else {
            return;
        };
        let Some(l) = self.doc.fonts.get(k).cloned() else {
            return;
        };
        let FontDef::Native(n) = &l.def else {
            return;
        };
        if let Some(p) = &l.problem {
            let p = p.clone();
            self.cur().unsupported(p, None);
            return;
        }
        let size = self.dvi_to_bp(n.size);
        let (extend, slant, embolden) = l.transform();
        if n.flags & XDV_FLAG_VERTICAL != 0 {
            self.cur()
                .unsupported("a vertical native font".into(), None);
        }
        // `embolden=` (fontspec's FakeBold): xdvipdfmx fills and strokes the
        // glyphs (`2 Tr`) with a line of `embolden` (XeTeX's, a tenth of
        // fontspec's FakeBold) times the size in TeX points over 10
        // (measured: FakeBold=1.5 at 10 pt is `0.149994 w`), in
        // stream space for the display list (spec §11.4).
        let bold = (embolden != 0.0).then(|| {
            let w = embolden / 10.0 * n.size as f64 / 65536.0;
            let [a, b, _, _, _, _] = self.cur_ref().gs.ctm.to_f64();
            Stroke {
                width: w * (a * a + b * b).sqrt(),
                cap: 0,
                join: 0,
                miter: 10.0,
                dash: vec![],
                phase: 0.0,
            }
        });
        let m = self.glyph_matrix(size, extend, slant);
        let color = n.rgba.map(|c| {
            let ch = |s: u32| ((c >> s) & 0xFF) as f64 / 255.0;
            (Col(vec![ch(24), ch(16), ch(8)]), ch(0))
        });
        let upm = l.upm.max(1) as f64;
        for (&(dx, dy), &g) in xy.iter().zip(gids) {
            let (ux, uy) = (
                self.dvi_to_bp(self.r.h.wrapping_add(dx)),
                -self.dvi_to_bp(self.r.v.wrapping_add(dy)),
            );
            let (rx, ry) = self.rel(ux, uy);
            let (x, y) = self.cur_ref().gs.ctm.apply(rx, ry);
            let s = self.cur();
            match &color {
                Some((c, a)) => s.sync_fill(Some(c), Some(*a)),
                None => s.sync_fill(None, None),
            }
            match &bold {
                Some(st) => {
                    s.sync_stroke();
                    if s.gs.told_line.as_ref() != Some(st) {
                        s.push(Item::LineState(st.clone()));
                        s.gs.told_line = Some(st.clone());
                    }
                    s.sync_text_render(2);
                }
                None => s.sync_text_render(0),
            }
            s.sync_glyph_matrix(m);
            s.use_font(l.res);
            let (px, py) = s.to_page(x, y);
            s.push(Item::Glyph {
                font: l.res,
                code: g,
                x: px,
                y: py,
                col: NO_COLUMN,
            });
            if self.doc.tracking.is_some() {
                // dvipdfmx's box for a native glyph: its advance, and the
                // font's ascent and descent (`hhea`).
                let sz = size.to_f64();
                let adv = l.advance(g) as f64 * sz * extend / upm;
                let asc = l.ascent as f64 * sz / upm;
                let desc = -(l.descent as f64) * sz / upm;
                let (bx, by) = self.device_point(dx, dy);
                let (bx, by) = (bx.to_f64(), by.to_f64());
                self.expand_annot([bx, by - desc, bx + adv, by + asc]);
            }
        }
    }

    /// A rule of height `a` and width `w` (DVI units) at the current point.
    fn rule(&mut self, a: i32, w: i32) {
        if a <= 0 || w <= 0 {
            return;
        }
        let (ux, uy) = self.user_point();
        let (x0, y0) = self.rel(ux, uy);
        let (wb, hb) = (self.dvi_to_bp(w), self.dvi_to_bp(a));
        let ctm = self.cur_ref().gs.ctm;
        self.track(0, w, a, 0);
        // dvipdfmx strokes a rule whose smaller side is at most 5 bp along
        // its longer side, and fills the others (measured, TeX Live 2026).
        let five = Fx(5 * ONE);
        let (kind, stroke) = if hb <= wb && hb <= five {
            (RuleKind::StrokeH, true)
        } else if wb < hb && wb <= five {
            (RuleKind::StrokeV, true)
        } else {
            (RuleKind::Fill, false)
        };
        let s = self.cur();
        if stroke {
            s.sync_stroke();
        } else {
            s.sync_fill(None, None);
        }
        if ctm.is_translation() {
            let (lx, by) = ctm.apply(x0, y0);
            let (l, b) = s.to_page(lx, by);
            let (r, t) = s.to_page(lx + wb, by + hb);
            s.push(Item::Rule {
                kind,
                x: l,
                y: t,
                w: r - l,
                h: b - t,
            });
        } else {
            let (x, y, w, h) = (x0.to_f64(), y0.to_f64(), wb.to_f64(), hb.to_f64());
            let m = s.matrix(ctm.to_f64());
            let path = match kind {
                RuleKind::Fill => Path {
                    paint: paint::FILL,
                    matrix: m,
                    stroke: None,
                    segs: rect_segs(x, y, w, h),
                },
                RuleKind::StrokeH => Path {
                    paint: paint::STROKE,
                    matrix: m,
                    stroke: Some(Stroke {
                        width: h,
                        cap: 0,
                        join: 0,
                        miter: 10.0,
                        dash: vec![],
                        phase: 0.0,
                    }),
                    segs: vec![Seg::Move(x, y + h / 2.0), Seg::Line(x + w, y + h / 2.0)],
                },
                _ => Path {
                    paint: paint::STROKE,
                    matrix: m,
                    stroke: Some(Stroke {
                        width: w,
                        cap: 0,
                        join: 0,
                        miter: 10.0,
                        dash: vec![],
                        phase: 0.0,
                    }),
                    segs: vec![Seg::Move(x + w / 2.0, y), Seg::Line(x + w / 2.0, y + h)],
                },
            };
            s.dl.paths.push(path);
            let n = s.dl.paths.len() as u32 - 1;
            s.push(Item::Path(n));
        }
    }

    // ---- link annotations (pdf:bann ... pdf:eann) ---------------------------

    /// What is drawn at the current point, `wd` wide, `ht` high and `dp`
    /// deep (DVI units), widens the link being tracked.
    fn track(&mut self, dx: i32, wd: i32, ht: i32, dp: i32) {
        if self.doc.tracking.is_none() {
            return;
        }
        let (x, y) = self.device_point(dx, 0);
        let (x, y) = (x.to_f64(), y.to_f64());
        let w = self.dvi_to_bp(wd).to_f64();
        let h = self.dvi_to_bp(ht).to_f64();
        let d = self.dvi_to_bp(dp).to_f64();
        self.expand_annot([x, y - d, x + w, y + h]);
    }

    pub fn expand_annot(&mut self, r: [f64; 4]) {
        let depth = self.stack.len();
        let Some(t) = self.doc.tracking.as_mut() else {
            return;
        };
        if depth < t.depth {
            return;
        }
        t.rect = Some(match t.rect {
            None => r,
            Some(o) => [
                o[0].min(r[0]),
                o[1].min(r[1]),
                o[2].max(r[2]),
                o[3].max(r[3]),
            ],
        });
    }

    /// A `pop` that leaves the level where `pdf:bann` was ends the part of
    /// the link on that line (dvipdfmx breaks a link at the end of the box
    /// it began in).
    fn after_pop(&mut self) {
        let depth = self.stack.len();
        if self
            .doc
            .tracking
            .as_ref()
            .is_some_and(|t| depth < t.depth && t.rect.is_some())
        {
            self.break_annot();
        }
    }

    /// Emit the part of the tracked link drawn so far on this page.
    pub fn break_annot(&mut self) {
        let Some(t) = self.doc.tracking.as_mut() else {
            return;
        };
        let Some(r) = t.rect.take() else {
            return;
        };
        let dict = t.dict.clone();
        self.add_link(dict, r);
    }

    /// A link annotation with rectangle `r` (stream space, bp).
    pub fn add_link(&mut self, dict: super::pdfobj::Dict, r: [f64; 4]) {
        let page = self.streams.first_mut().expect("the page");
        let (l, t) = page.to_page(fx(r[0]), fx(r[3]));
        let (rr, b) = page.to_page(fx(r[2]), fx(r[1]));
        let (kind, file, data) = link_action(&dict);
        let link_index = if dict.get(b"Subtype").and_then(Obj::as_name) == Some(b"Link") {
            page.dl.links.push(Link {
                rect: [l, t, rr, b],
                span: 0,
                kind,
                file,
                data,
            });
            Some(page.dl.links.len() - 1)
        } else {
            None
        };
        page.sup.annots.push(Annot {
            dict,
            rect: r,
            link: link_index,
        });
    }

    /// A named destination on this page: `name`, and the destination array
    /// (stream-space numbers, `@thispage` already resolved).
    pub fn add_dest(&mut self, name: Vec<u8>, arr: &[Obj]) {
        let kind_of = |n: &[u8]| -> Option<u8> {
            Some(match n {
                b"XYZ" => 0,
                b"Fit" => 1,
                b"FitH" => 2,
                b"FitV" => 3,
                b"FitB" => 4,
                b"FitBH" => 5,
                b"FitBV" => 6,
                b"FitR" => 7,
                _ => return None,
            })
        };
        let on_this_page = matches!(arr.first(), Some(Obj::Named(n)) if *n == format!("page{}", self.page_no).as_bytes());
        if let (true, Some(kind)) = (
            on_this_page,
            arr.get(1).and_then(Obj::as_name).and_then(kind_of),
        ) {
            let page = self.streams.first().expect("the page");
            let num = |i: usize| arr.get(i).and_then(Obj::as_num);
            let x = |v: Option<f64>| v.map_or(0, |v| clamp_i32(fx(v).to_sp()));
            let y = |v: Option<f64>| v.map_or(0, |v| clamp_i32((page.height - fx(v)).to_sp()));
            let (rect, zoom) = match kind {
                0 => (
                    [x(num(2)), y(num(3)), 0, 0],
                    num(4).map_or(0, |z| (z * 1000.0).round() as i32),
                ),
                2 | 5 => ([0, y(num(2)), 0, 0], 0),
                3 | 6 => ([x(num(2)), 0, 0, 0], 0),
                7 => ([x(num(2)), y(num(5)), x(num(4)), y(num(3))], 0),
                _ => ([0; 4], 0),
            };
            let page = self.streams.first_mut().expect("the page");
            page.dl.dests.push(Dest {
                named: true,
                name: name.clone(),
                kind,
                rect,
                zoom,
            });
        }
        self.doc.dests.push((name, Obj::Array(arr.to_vec())));
    }

    // ---- colours -------------------------------------------------------------

    /// Set the PDF fill and/or stroke colour (dvipdfmx's `pdf_dev_set_color`).
    pub fn set_colors(&mut self, fill: Option<Col>, stroke: Option<Col>) {
        let s = self.cur();
        if let Some(f) = fill {
            s.gs.fill = f;
            s.gs.odd_fill = false;
        }
        if let Some(c) = stroke {
            s.gs.stroke = c;
            s.gs.odd_stroke = false;
        }
    }

    /// After dvipdfmx's own `Q`: the colour stack's top is set again
    /// (`pdf_dev_reset_color`).
    pub fn reset_color(&mut self) {
        let (f, s) = self
            .doc
            .colors
            .last()
            .cloned()
            .unwrap_or((Col::black(), Col::black()));
        self.set_colors(Some(f), Some(s));
    }

    // ---- forms (pdf:bxobj ... pdf:exobj) ------------------------------------

    /// `pdf:bxobj @name` with the box `bbox` (user space relative to the
    /// current point, bp: llx, lly, urx, ury).
    pub fn begin_form(&mut self, name: Vec<u8>, bbox: [Fx; 4]) {
        let (ux, uy) = self.user_point();
        // The form's id is given when it is done (`end_form`).
        let mut f = Stream::new(StreamKind::Form, 0, bbox[3], Mat::IDENTITY);
        f.dl.pdf_box = bbox.map(Fx::to_f64);
        f.dl.width = clamp_i32((bbox[2] - bbox[0]).to_sp());
        f.dl.height = clamp_i32((bbox[3] - bbox[1]).to_sp());
        f.form_name = Some(name);
        f.form_origin = (ux, uy);
        if let Some((fc, sc)) = self.doc.colors.last().cloned() {
            f.gs.fill = fc;
            f.gs.stroke = sc;
        }
        self.streams.push(f);
    }

    /// `pdf:exobj [<<dict>>]`: the form is done; it becomes the named form.
    pub fn end_form(&mut self, dict: Option<super::pdfobj::Dict>) {
        if self.streams.len() < 2 {
            self.doc.diag("pdf:exobj without pdf:bxobj".into());
            return;
        }
        let mut f = self.streams.pop().expect("a form");
        if let Some(d) = dict {
            f.sup.form_dict.merge(&d);
        }
        let name = f.form_name.clone().unwrap_or_default();
        let built = f.finish();
        self.doc.forms.push(built);
        let id = self.doc.forms.len() as u32;
        self.doc.forms[id as usize - 1].dl.index = id;
        self.doc.named.insert(name, super::doc::Named::Form(id));
    }

    /// Draw form `id` (its origin) at the current point.
    pub fn draw_form(&mut self, id: u32, extra: Mat) {
        let (x, y) = self.user_point();
        let (x, y) = self.rel(x, y);
        let m = extra
            .then(&Mat::translate(x, y))
            .then(&self.cur_ref().gs.ctm);
        let s = self.cur();
        let n = s.matrix(m.to_f64());
        s.use_form(id);
        s.push(Item::Form { id, matrix: n });
    }
}

/// `v` rounded to `digits` decimals (halves away from zero).
pub fn round_to(v: Fx, digits: u32) -> Fx {
    let q = ONE / 10i128.pow(digits);
    Fx(super::div_round(v.0, q) * q)
}

pub fn fx(v: f64) -> Fx {
    Fx((v * ONE as f64).round() as i128)
}

pub fn rect_segs(x: f64, y: f64, w: f64, h: f64) -> Vec<Seg> {
    vec![
        Seg::Move(x, y),
        Seg::Line(x + w, y),
        Seg::Line(x + w, y + h),
        Seg::Line(x, y + h),
        Seg::Close,
    ]
}

/// A link annotation's action as the display list says it (spec §4.5).
fn link_action(d: &super::pdfobj::Dict) -> (LinkKind, Vec<u8>, Vec<u8>) {
    let raw = |o: &Obj| {
        let mut out = Vec::new();
        let mut named = |n: &[u8]| Some(format!("@{}", String::from_utf8_lossy(n)));
        super::pdfobj::Writer { named: &mut named }.write(o, &mut out);
        out
    };
    if let Some(dest) = d.get(b"Dest") {
        return match dest {
            Obj::Str(s) | Obj::Name(s) => (LinkKind::GotoName, Vec::new(), s.clone()),
            o => (LinkKind::Raw, Vec::new(), raw(o)),
        };
    }
    let Some(a) = d.get(b"A") else {
        return (LinkKind::Raw, Vec::new(), Vec::new());
    };
    let Some(ad) = a.as_dict() else {
        return (LinkKind::Raw, Vec::new(), raw(a));
    };
    match ad.get(b"S").and_then(Obj::as_name) {
        Some(b"GoTo") => match ad.get(b"D") {
            Some(Obj::Str(s) | Obj::Name(s)) => (LinkKind::GotoName, Vec::new(), s.clone()),
            _ => (LinkKind::Raw, Vec::new(), raw(a)),
        },
        Some(b"URI") => match ad.get(b"URI") {
            Some(Obj::Str(s)) => (LinkKind::Uri, Vec::new(), s.clone()),
            _ => (LinkKind::Raw, Vec::new(), raw(a)),
        },
        Some(b"GoToR") => {
            let file = match ad.get(b"F") {
                Some(Obj::Str(s)) => s.clone(),
                _ => Vec::new(),
            };
            match ad.get(b"D") {
                Some(Obj::Str(s) | Obj::Name(s)) => (LinkKind::GotoName, file, s.clone()),
                _ => (LinkKind::Raw, Vec::new(), raw(a)),
            }
        }
        _ => (LinkKind::Raw, Vec::new(), raw(a)),
    }
}

/// The page's size: the last `pdf:pagesize`/`papersize` special of the
/// page (dvipdfmx scans a page's specials for it before the page), else
/// the size an earlier page set, else the default paper.
fn page_size(doc: &mut Doc, bytes: &[u8]) -> (Fx, Fx) {
    for s in super::special::scan_specials(bytes) {
        if let Some(sz) = super::special::paper_of(&s) {
            match sz {
                Some(sz) => doc.paper = Some(sz),
                None => doc.paper = None,
            }
        }
    }
    doc.paper
        .or(doc.default_paper)
        .unwrap_or((Fx(612 * ONE), Fx(792 * ONE)))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub struct NoTfm;
    impl Metrics for NoTfm {
        fn tfm_char(&self, _k: i32, _c: u32) -> Option<(i32, i32, i32)> {
            Some((65536 * 5, 65536 * 7, 65536 * 2))
        }
    }

    pub fn page_bytes(body: &[u8]) -> Vec<u8> {
        let mut b = vec![139];
        b.extend_from_slice(&[0; 44]);
        b.extend_from_slice(body);
        b.push(140);
        b
    }

    pub fn special(s: &str) -> Vec<u8> {
        let mut v = vec![239, s.len() as u8];
        v.extend_from_slice(s.as_bytes());
        v
    }

    #[test]
    fn a_page_that_cannot_be_read_is_an_error_not_dropped() {
        let mut doc = Doc::new();
        // an opcode XDV does not have
        let bad = page_bytes(&[250]);
        let e = Builder::page(&mut doc, &NoTfm, 1000, 1, &bad).unwrap_err();
        assert!(e.contains("opcode 250"), "{e}");
        // a rule cut short
        let mut cut = vec![139];
        cut.extend_from_slice(&[0; 44]);
        cut.extend_from_slice(&[137, 0, 0]);
        let e = Builder::page(&mut doc, &NoTfm, 1000, 1, &cut).unwrap_err();
        assert!(e.contains("truncated"), "{e}");
    }

    #[test]
    fn rules_are_placed_and_classified_as_dvipdfmx_draws_them() {
        let mut doc = Doc::new();
        // \hrule height 2bp width 100bp at v = 10bp; a 10bp-high rule.
        let bp = |x: f64| (x * 65781.76).round() as i32;
        let mut body = special("pdf:pagesize width 614.295pt height 794.96999pt");
        body.push(160); // down4
        body.extend_from_slice(&bp(10.0).to_be_bytes());
        body.push(137); // put_rule
        body.extend_from_slice(&bp(2.0).to_be_bytes());
        body.extend_from_slice(&bp(100.0).to_be_bytes());
        body.push(137);
        body.extend_from_slice(&bp(10.0).to_be_bytes());
        body.extend_from_slice(&bp(100.0).to_be_bytes());
        let built = Builder::page(&mut doc, &NoTfm, 1000, 1, &page_bytes(&body)).unwrap();
        let rules: Vec<_> = built
            .dl
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Rule { kind, x, y, w, h } => Some((*kind, *x, *y, *w, *h)),
                _ => None,
            })
            .collect();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].0, RuleKind::StrokeH);
        assert_eq!(rules[1].0, RuleKind::Fill);
        // left at 72bp, bottom at 72 + 10bp from the top
        assert_eq!(rules[0].1, bp(72.0));
        // the bottom edge, rounded once: 72 bp (4736286.72 sp) + v
        assert_eq!(
            rules[0].2 + rules[0].4,
            (4736286.72 + bp(10.0) as f64).round() as i32
        );
        assert!((built.dl.pdf_box[2] - 612.0).abs() < 1e-3);
    }
}
