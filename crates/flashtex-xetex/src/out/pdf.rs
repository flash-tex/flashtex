//! FlashTeX's PDF writer for Unicode mode (PLAN.md §3.2): the PDF is
//! written **from the display list** — every glyph, rule, path, clip,
//! colour, alpha, image and form of a page is the page's display-list item
//! — plus what the display list cannot carry, kept beside it
//! ([`super::doc::Supplement`]: the operators of `UNSUPPORTED` items, the
//! `@resources` and `@thispage` additions, the whole annotation
//! dictionaries) and the document's own parts ([`Doc`]: outline, document
//! information, named destinations and objects, catalog entries).
//!
//! * Native fonts are embedded as `Type0`/`Identity-H` fonts whose CIDs are
//!   the font's own glyph ids, as xdvipdfmx writes them: CFF outlines as a
//!   CID-keyed CFF subset with CID = glyph id (`CIDFontType0C`), TrueType
//!   outlines as a subset that keeps glyph ids in place (`CIDFontType2`,
//!   `/CIDToGIDMap /Identity`), both by FlashTeX's own subsetters
//!   (`flashtex_pdf::cff`, `flashtex_pdf::truetype`), with `/W` from the
//!   font's advances and a `ToUnicode` CMap from its `cmap`.
//! * TFM fonts are `Type1` fonts with a subset of the map file's program
//!   (`flashtex_pdf::type1`), the encoding as `/Differences`, `/Widths` from
//!   the TFM.
//! * PNG images are copied unchanged (their `IDAT` data with the PNG
//!   predictor, as pdfTeX and dvipdfmx do) when they can be; JPEG passes
//!   through; a PDF page becomes a form XObject (`flashtex_pdf::images`).
//! * Streams are compressed with TeX Live's zlib, linked by the pdfTeX
//!   engine (`flashtex_engine::pdftex::zlib`).
//!
//! Positions: a glyph is placed with its own text matrix at the display
//! list's position (sp, written to 10⁻⁵ bp, which reads back to the same
//! sp), so the PDF draws every glyph where TeX put it; xdvipdfmx's PDF puts
//! them within its own rounding of TeX's positions (PLAN.md §3.5's parity
//! is structural and visual, not byte-level).

use super::doc::{Doc, Named, Supplement};
use super::fonts::{FontRes, ResKind};
use super::images::Kind;
use super::pdfobj::{fmt_num, write_name, Dict, Obj, Writer as ObjWriter};
use flashtex_display_list::page::{paint, Item, Page, RuleKind, Seg};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// sp per bp.
const K: f64 = 65781.76;

/// What the writer needs beyond the document.
pub struct Options {
    /// `/Producer`.
    pub producer: String,
    /// `/CreationDate` and `/ModDate` (`D:...`), unless the document gives
    /// them.
    pub date: Option<String>,
    /// Compress streams (zlib level 9).
    pub compress: bool,
}

/// The PDF of `doc`.
pub fn write(doc: &Doc, opts: &Options) -> Result<(Vec<u8>, Vec<String>), String> {
    let mut w = Writer::new(doc, opts);
    w.run()?;
    let warnings = std::mem::take(&mut w.warnings);
    Ok((w.finish(), warnings))
}

/// zlib (TeX Live's) at level 9.
pub fn compress(data: &[u8]) -> Vec<u8> {
    use flashtex_engine::pdftex::zlib::{ZStream, Z_FINISH, Z_OK, Z_STREAM_END};
    let mut out = vec![0u8; data.len() + data.len() / 1000 + 64];
    let mut z = ZStream::new();
    if z.deflate_init(9) != Z_OK {
        return Vec::new();
    }
    z.next_in = data.as_ptr();
    z.avail_in = data.len() as u32;
    z.next_out = out.as_mut_ptr();
    z.avail_out = out.len() as u32;
    let r = z.deflate(Z_FINISH);
    let n = z.total_out as usize;
    z.deflate_end();
    if r != Z_STREAM_END {
        return Vec::new();
    }
    out.truncate(n);
    out
}

/// A number with at most `d` fraction digits.
fn n(v: f64, d: usize) -> String {
    fmt_num(v, d)
}

struct Writer<'a> {
    doc: &'a Doc,
    opts: &'a Options,
    out: Vec<u8>,
    /// Byte offset of object n at index n - 1 (`None`: not written yet).
    offsets: Vec<Option<usize>>,
    pages: Vec<u32>,
    pages_root: u32,
    catalog: u32,
    fonts: BTreeMap<u16, u32>,
    images: BTreeMap<u32, u32>,
    forms: BTreeMap<u32, u32>,
    named: BTreeMap<Vec<u8>, u32>,
    named_queue: Vec<Vec<u8>>,
    names_obj: Option<u32>,
    /// The glyphs (native fonts) and codes (TFM fonts) each font draws.
    used: BTreeMap<u16, BTreeSet<u16>>,
    page_group: Vec<bool>,
    /// For a native font with CID-keyed CFF outlines: each glyph id's CID
    /// (the font's charset), which the content stream writes as the code.
    cids: BTreeMap<u16, BTreeMap<u16, u16>>,
    /// What the PDF does not carry as the document asks (reported).
    pub warnings: Vec<String>,
}

/// The resources a content stream uses.
#[derive(Default)]
struct Res {
    fonts: BTreeSet<u16>,
    images: BTreeSet<u32>,
    forms: BTreeSet<u32>,
    gs: BTreeMap<String, String>,
    group: bool,
}

impl<'a> Writer<'a> {
    fn new(doc: &'a Doc, opts: &'a Options) -> Self {
        Writer {
            doc,
            opts,
            out: Vec::new(),
            offsets: Vec::new(),
            pages: Vec::new(),
            pages_root: 0,
            catalog: 0,
            fonts: BTreeMap::new(),
            images: BTreeMap::new(),
            forms: BTreeMap::new(),
            named: BTreeMap::new(),
            named_queue: Vec::new(),
            names_obj: None,
            used: BTreeMap::new(),
            page_group: Vec::new(),
            cids: BTreeMap::new(),
            warnings: Vec::new(),
        }
    }

    fn alloc(&mut self) -> u32 {
        self.offsets.push(None);
        self.offsets.len() as u32
    }

    fn begin(&mut self, num: u32) {
        self.offsets[num as usize - 1] = Some(self.out.len());
        let _ = writeln!(Bytes(&mut self.out), "{num} 0 obj");
    }

    fn object(&mut self, num: u32, body: &[u8]) {
        self.begin(num);
        self.out.extend_from_slice(body);
        self.out.extend_from_slice(b"\nendobj\n");
    }

    /// A stream object: `dict` is its entries without `/Length` (and
    /// without `/Filter` when `compress`).
    fn stream(&mut self, num: u32, dict: &[u8], data: &[u8], compress: bool) {
        let z;
        let (data, filter): (&[u8], &[u8]) = if compress && self.opts.compress {
            z = self::compress(data);
            if z.is_empty() && !data.is_empty() {
                (data, b"")
            } else {
                (&z, b" /Filter /FlateDecode")
            }
        } else {
            (data, b"")
        };
        self.begin(num);
        self.out.extend_from_slice(b"<<");
        self.out.extend_from_slice(dict);
        self.out.extend_from_slice(filter);
        let _ = writeln!(Bytes(&mut self.out), " /Length {}>>\nstream", data.len());
        self.out.extend_from_slice(data);
        self.out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    fn run(&mut self) -> Result<(), String> {
        let doc = self.doc;
        self.catalog = self.alloc();
        self.pages_root = self.alloc();
        for _ in &doc.pages {
            let p = self.alloc();
            self.pages.push(p);
        }
        // What every font draws, from every page and form.
        for b in doc.pages.iter().chain(doc.forms.iter()) {
            for it in &b.dl.items {
                if let Item::Glyph { font, code, .. } = it {
                    self.used.entry(*font).or_default().insert(*code);
                }
            }
        }
        for (&id, used) in &self.used {
            let Some(r) = doc.fonts.resource(id) else {
                continue;
            };
            let ResKind::Native {
                index, cff: true, ..
            } = &r.kind
            else {
                continue;
            };
            if let Some(m) = cid_map(&r.program, *index, used) {
                self.cids.insert(id, m);
            }
        }
        for i in 0..doc.pages.len() {
            self.page(i)?;
        }
        // Forms, images and fonts the pages used (forms may use forms).
        let mut done_forms = BTreeSet::new();
        loop {
            let todo: Vec<u32> = self
                .forms
                .keys()
                .copied()
                .filter(|f| !done_forms.contains(f))
                .collect();
            if todo.is_empty() {
                break;
            }
            for f in todo {
                done_forms.insert(f);
                self.form(f)?;
            }
        }
        let imgs: Vec<(u32, u32)> = self.images.iter().map(|(&k, &v)| (k, v)).collect();
        for (id, num) in imgs {
            self.image(id, num)?;
        }
        let fonts: Vec<(u16, u32)> = self.fonts.iter().map(|(&k, &v)| (k, v)).collect();
        for (id, num) in fonts {
            self.font(id, num)?;
        }
        self.catalog_and_tree()?;
        // Named objects, last: anything above may have referenced them.
        while let Some(name) = self.named_queue.pop() {
            self.named_object(&name);
        }
        Ok(())
    }

    // ---- references -----------------------------------------------------------

    /// The text of a reference to named object `n` (`@n` in a special).
    fn resolve(&mut self, n: &[u8]) -> Option<String> {
        if let Some(p) = n.strip_prefix(b"page") {
            if let Ok(k) = std::str::from_utf8(p).unwrap_or("").parse::<usize>() {
                return self
                    .pages
                    .get(k.wrapping_sub(1))
                    .map(|o| format!("{o} 0 R"));
            }
        }
        match n {
            b"catalog" => return Some(format!("{} 0 R", self.catalog)),
            b"pages" => return Some(format!("{} 0 R", self.pages_root)),
            b"names" => {
                let o = match self.names_obj {
                    Some(o) => o,
                    None => {
                        let o = self.alloc();
                        self.names_obj = Some(o);
                        o
                    }
                };
                return Some(format!("{o} 0 R"));
            }
            _ => {}
        }
        match self.doc.named.get(n)? {
            Named::Form(id) => {
                let o = self.form_obj(*id);
                Some(format!("{o} 0 R"))
            }
            Named::Image(id) => {
                let o = self.image_obj(*id);
                Some(format!("{o} 0 R"))
            }
            _ => {
                if let Some(&o) = self.named.get(n) {
                    return Some(format!("{o} 0 R"));
                }
                let o = self.alloc();
                self.named.insert(n.to_vec(), o);
                self.named_queue.push(n.to_vec());
                Some(format!("{o} 0 R"))
            }
        }
    }

    fn ser(&mut self, o: &Obj) -> Vec<u8> {
        let mut out = Vec::new();
        let mut resolve = |n: &[u8]| self.resolve(n);
        ObjWriter {
            named: &mut resolve,
        }
        .write(o, &mut out);
        out
    }

    fn ser_dict_entries(&mut self, d: &Dict) -> Vec<u8> {
        let mut out = Vec::new();
        for (k, v) in &d.0 {
            write_name(k, &mut out);
            out.push(b' ');
            let s = self.ser(v);
            out.extend_from_slice(&s);
        }
        out
    }

    fn named_object(&mut self, name: &[u8]) {
        let num = self.named[name];
        match self.doc.named.get(name).cloned() {
            Some(Named::Obj(o)) => {
                let body = self.ser(&o);
                self.object(num, &body);
            }
            Some(Named::Stream(d, data)) => {
                let mut d = d.clone();
                d.remove(b"Length");
                let has_filter = d.get(b"Filter").is_some();
                let entries = self.ser_dict_entries(&d);
                self.stream(num, &entries, &data, !has_filter);
            }
            _ => self.object(num, b"null"),
        }
    }

    fn font_obj(&mut self, id: u16) -> u32 {
        if let Some(&o) = self.fonts.get(&id) {
            return o;
        }
        let o = self.alloc();
        self.fonts.insert(id, o);
        o
    }

    fn image_obj(&mut self, id: u32) -> u32 {
        if let Some(&o) = self.images.get(&id) {
            return o;
        }
        let o = self.alloc();
        self.images.insert(id, o);
        o
    }

    fn form_obj(&mut self, id: u32) -> u32 {
        if let Some(&o) = self.forms.get(&id) {
            return o;
        }
        let o = self.alloc();
        self.forms.insert(id, o);
        o
    }

    // ---- pages and forms -------------------------------------------------------

    fn page(&mut self, i: usize) -> Result<(), String> {
        let doc = self.doc;
        let b = &doc.pages[i];
        let num = self.pages[i];
        let mut res = Res::default();
        let content = self.content(&b.dl, &b.sup, &mut res);
        let contents = self.alloc();
        self.stream(contents, b"", &content, true);
        let resources = self.resources(&res, &b.sup)?;
        // annotations
        let mut annots = Vec::new();
        for a in &b.sup.annots {
            let mut d = a.dict.clone();
            if d.get(b"Type").is_none() {
                d.0.insert(0, (b"Type".to_vec(), Obj::Name(b"Annot".to_vec())));
            }
            d.set(
                b"Rect",
                Obj::Array(a.rect.iter().map(|v| Obj::num(*v, 3)).collect()),
            );
            let o = self.alloc();
            let body = self.ser(&Obj::Dict(d));
            self.object(o, &body);
            annots.push(o);
        }
        let mut d = Vec::new();
        let [x0, y0, x1, y1] = b.dl.pdf_box;
        let _ = write!(
            Bytes(&mut d),
            "<</Type /Page /Parent {} 0 R /MediaBox [{} {} {} {}] /Contents {contents} 0 R /Resources ",
            self.pages_root,
            n(x0, 3),
            n(y0, 3),
            n(x1, 3),
            n(y1, 3)
        );
        d.extend_from_slice(&resources);
        if !annots.is_empty() {
            d.extend_from_slice(b" /Annots [");
            for (k, o) in annots.iter().enumerate() {
                if k > 0 {
                    d.push(b' ');
                }
                let _ = write!(Bytes(&mut d), "{o} 0 R");
            }
            d.push(b']');
        }
        if res.group && b.sup.page_dict.get(b"Group").is_none() {
            d.extend_from_slice(
                b" /Group <</Type /Group /S /Transparency /CS /DeviceRGB /I true>>",
            );
        }
        let mut extra = b.sup.page_dict.clone();
        for k in [
            &b"Type"[..],
            b"Parent",
            b"MediaBox",
            b"Contents",
            b"Resources",
            b"Annots",
        ] {
            extra.remove(k);
        }
        if !extra.0.is_empty() {
            d.push(b' ');
            let e = self.ser_dict_entries(&extra);
            d.extend_from_slice(&e);
        }
        d.extend_from_slice(b">>");
        self.object(num, &d);
        self.page_group.push(res.group);
        Ok(())
    }

    fn form(&mut self, id: u32) -> Result<(), String> {
        let doc = self.doc;
        let num = self.forms[&id];
        let Some(b) = doc.forms.get(id as usize - 1) else {
            self.object(num, b"null");
            return Ok(());
        };
        let mut res = Res::default();
        let content = self.content(&b.dl, &b.sup, &mut res);
        let resources = self.resources(&res, &b.sup)?;
        let [x0, y0, x1, y1] = b.dl.pdf_box;
        let mut d = Vec::new();
        let _ = write!(
            Bytes(&mut d),
            "/Type /XObject /Subtype /Form /FormType 1 /BBox [{} {} {} {}] /Resources ",
            n(x0, 5),
            n(y0, 5),
            n(x1, 5),
            n(y1, 5)
        );
        d.extend_from_slice(&resources);
        let mut extra = b.sup.form_dict.clone();
        for k in [
            &b"Type"[..],
            b"Subtype",
            b"BBox",
            b"Resources",
            b"Length",
            b"Filter",
        ] {
            extra.remove(k);
        }
        if !extra.0.is_empty() {
            d.push(b' ');
            let e = self.ser_dict_entries(&extra);
            d.extend_from_slice(&e);
        }
        self.stream(num, &d, &content, true);
        Ok(())
    }

    /// The `/Resources` dictionary of a content stream: what its items use,
    /// and what `pdf:put @resources` added.
    fn resources(&mut self, res: &Res, sup: &Supplement) -> Result<Vec<u8>, String> {
        let mut cats: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
        let mut font = Vec::new();
        for &f in &res.fonts {
            let o = self.font_obj(f);
            let _ = write!(Bytes(&mut font), "/F{f} {o} 0 R");
        }
        let mut xo = Vec::new();
        for &i in &res.images {
            let o = self.image_obj(i);
            let _ = write!(Bytes(&mut xo), "/Im{i} {o} 0 R");
        }
        for &f in &res.forms {
            let o = self.form_obj(f);
            let _ = write!(Bytes(&mut xo), "/Fm{f} {o} 0 R");
        }
        let mut gs = Vec::new();
        for (k, v) in &res.gs {
            let _ = write!(Bytes(&mut gs), "/{k} {v}");
        }
        cats.insert(b"Font".to_vec(), font);
        cats.insert(b"XObject".to_vec(), xo);
        cats.insert(b"ExtGState".to_vec(), gs);
        let mut out = b"<<".to_vec();
        let mut done = BTreeSet::new();
        for (k, v) in sup.resources.0.clone() {
            let ours = cats.get(&k).cloned().unwrap_or_default();
            write_name(&k, &mut out);
            out.push(b' ');
            done.insert(k.clone());
            // a dictionary the special gave, directly or by name
            let as_dict = match &v {
                Obj::Dict(d) => Some(d.clone()),
                Obj::Named(nm) => self.doc.named_dict(nm).cloned(),
                _ => None,
            };
            match as_dict {
                Some(d) if !ours.is_empty() || matches!(v, Obj::Dict(_)) => {
                    out.extend_from_slice(b"<<");
                    let e = self.ser_dict_entries(&d);
                    out.extend_from_slice(&e);
                    out.extend_from_slice(&ours);
                    out.extend_from_slice(b">>");
                }
                _ => {
                    let s = self.ser(&v);
                    out.extend_from_slice(&s);
                }
            }
        }
        for (k, v) in cats {
            if done.contains(&k) || v.is_empty() {
                continue;
            }
            write_name(&k, &mut out);
            out.extend_from_slice(b" <<");
            out.extend_from_slice(&v);
            out.extend_from_slice(b">>");
        }
        out.extend_from_slice(b" /ProcSet [/PDF /Text /ImageB /ImageC /ImageI]>>");
        Ok(out)
    }

    /// A page's or form's content stream, from its display list.
    fn content(&mut self, p: &Page, sup: &Supplement, res: &mut Res) -> Vec<u8> {
        let doc = self.doc;
        let h = p.pdf_box[3];
        let x_of = |sp: i32| sp as f64 / K;
        let y_of = |sp: i32| h - sp as f64 / K;
        let mut c = Vec::new();
        let mut in_text = false;
        let mut matrix = [1.0, 0.0, 0.0, 1.0];
        // in a text object: the font and size set
        let mut cur_font: Option<(u16, String)> = None;
        let end_text = |c: &mut Vec<u8>, in_text: &mut bool| {
            if *in_text {
                c.extend_from_slice(b"ET\n");
                *in_text = false;
            }
        };
        for it in &p.items {
            match it {
                Item::Glyph {
                    font, code, x, y, ..
                } => {
                    if !in_text {
                        c.extend_from_slice(b"BT\n");
                        in_text = true;
                        cur_font = None;
                    }
                    res.fonts.insert(*font);
                    let fr = doc.fonts.resource(*font);
                    let (two_byte, adjust) = match fr.map(|r| &r.kind) {
                        Some(ResKind::Native { .. }) => (true, None),
                        Some(ResKind::Type1 { slant, extend, .. })
                            if *slant != 0 || *extend != 0 =>
                        {
                            let e = if *extend != 0 {
                                *extend as f64 / 1000.0
                            } else {
                                1.0
                            };
                            (false, Some((e, *slant as f64 / 1000.0)))
                        }
                        _ => (false, None),
                    };
                    let [mut a, mut b, mut cc, mut d] = matrix;
                    if let Some((e, s)) = adjust {
                        // [e 0 s 1] x M
                        let (na, nb) = (e * a, e * b);
                        let (nc, nd) = (s * a + cc, s * b + d);
                        a = na;
                        b = nb;
                        cc = nc;
                        d = nd;
                    }
                    let upright = b == 0.0 && cc == 0.0 && a == d && a > 0.0;
                    let size = if upright { n(a, 6) } else { "1".to_string() };
                    if cur_font.as_ref() != Some(&(*font, size.clone())) {
                        let _ = writeln!(Bytes(&mut c), "/F{font} {size} Tf");
                        cur_font = Some((*font, size));
                    }
                    let (gx, gy) = (n(x_of(*x), 5), n(y_of(*y), 5));
                    if upright {
                        let _ = write!(Bytes(&mut c), "1 0 0 1 {gx} {gy} Tm");
                    } else {
                        let _ = write!(
                            Bytes(&mut c),
                            "{} {} {} {} {gx} {gy} Tm",
                            n(a, 6),
                            n(b, 6),
                            n(cc, 6),
                            n(d, 6)
                        );
                    }
                    if two_byte {
                        let code = self
                            .cids
                            .get(font)
                            .and_then(|m| m.get(code))
                            .copied()
                            .unwrap_or(*code);
                        let _ = writeln!(Bytes(&mut c), "<{code:04X}>Tj");
                    } else {
                        let _ = writeln!(Bytes(&mut c), "<{:02X}>Tj", code & 0xFF);
                    }
                }
                Item::Matrix(k) => {
                    let m = p.matrix(*k);
                    matrix = [m[0], m[1], m[2], m[3]];
                }
                Item::FillColor(col) => color_op(&mut c, &col.0, false),
                Item::StrokeColor(col) => color_op(&mut c, &col.0, true),
                Item::TextRender(m) => {
                    let _ = writeln!(Bytes(&mut c), "{m} Tr");
                }
                Item::FillAlpha(a) | Item::StrokeAlpha(a) => {
                    let fill = matches!(it, Item::FillAlpha(_));
                    let key = if fill { "ca" } else { "CA" };
                    let name = format!(
                        "Gs{}{}",
                        if fill { "f" } else { "s" },
                        n(*a, 5).replace('.', "_")
                    );
                    res.gs
                        .insert(name.clone(), format!("<</{key} {}>>", n(*a, 5)));
                    let _ = writeln!(Bytes(&mut c), "/{name} gs");
                }
                Item::LineState(st) => {
                    let _ = write!(
                        Bytes(&mut c),
                        "{} w {} J {} j {} M [",
                        n(st.width, 6),
                        st.cap,
                        st.join,
                        n(st.miter, 6)
                    );
                    for (i, v) in st.dash.iter().enumerate() {
                        if i > 0 {
                            c.push(b' ');
                        }
                        c.extend_from_slice(n(*v, 6).as_bytes());
                    }
                    let _ = writeln!(Bytes(&mut c), "] {} d", n(st.phase, 6));
                }
                Item::Span(_) => {}
                Item::Save => {
                    end_text(&mut c, &mut in_text);
                    c.extend_from_slice(b"q\n");
                }
                Item::Restore => {
                    end_text(&mut c, &mut in_text);
                    c.extend_from_slice(b"Q\n");
                }
                Item::Rule {
                    kind,
                    x,
                    y,
                    w,
                    h: rh,
                } => {
                    end_text(&mut c, &mut in_text);
                    let (l, t) = (x_of(*x), y_of(*y));
                    let (wb, hb) = (*w as f64 / K, *rh as f64 / K);
                    let b = t - hb;
                    match kind {
                        RuleKind::Fill => {
                            let _ = writeln!(
                                Bytes(&mut c),
                                "{} {} {} {} re f",
                                n(l, 5),
                                n(b, 5),
                                n(wb, 5),
                                n(hb, 5)
                            );
                        }
                        RuleKind::StrokeH => {
                            let ym = b + hb / 2.0;
                            let _ = writeln!(
                                Bytes(&mut c),
                                "q 0 J [] 0 d {} w {} {} m {} {} l S Q",
                                n(hb, 5),
                                n(l, 5),
                                n(ym, 5),
                                n(l + wb, 5),
                                n(ym, 5)
                            );
                        }
                        RuleKind::StrokeV => {
                            let xm = l + wb / 2.0;
                            let _ = writeln!(
                                Bytes(&mut c),
                                "q 0 J [] 0 d {} w {} {} m {} {} l S Q",
                                n(wb, 5),
                                n(xm, 5),
                                n(b, 5),
                                n(xm, 5),
                                n(t, 5)
                            );
                        }
                    }
                }
                Item::Path(k) => {
                    end_text(&mut c, &mut in_text);
                    let Some(path) = p.paths.get(*k as usize) else {
                        continue;
                    };
                    c.extend_from_slice(b"q ");
                    let m = p.matrix(path.matrix);
                    if m != [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] {
                        write_matrix(&mut c, &m);
                        c.extend_from_slice(b" cm ");
                    }
                    if let Some(s) = &path.stroke {
                        let _ = write!(
                            Bytes(&mut c),
                            "{} w {} J {} j {} M [",
                            n(s.width, 6),
                            s.cap,
                            s.join,
                            n(s.miter, 6)
                        );
                        for (i, v) in s.dash.iter().enumerate() {
                            if i > 0 {
                                c.push(b' ');
                            }
                            c.extend_from_slice(n(*v, 6).as_bytes());
                        }
                        let _ = write!(Bytes(&mut c), "] {} d ", n(s.phase, 6));
                    }
                    write_segs(&mut c, &path.segs, None);
                    let fill = path.paint & paint::FILL != 0;
                    let eo = path.paint & paint::FILL_EVEN_ODD != 0;
                    let stroke = path.paint & paint::STROKE != 0;
                    let op = match (fill, eo, stroke) {
                        (_, true, true) => "B*",
                        (true, _, true) => "B",
                        (_, true, false) => "f*",
                        (true, _, false) => "f",
                        (false, false, true) => "S",
                        _ => "n",
                    };
                    let _ = writeln!(Bytes(&mut c), "{op} Q");
                }
                Item::Clip(k) => {
                    end_text(&mut c, &mut in_text);
                    let Some(path) = p.paths.get(*k as usize) else {
                        continue;
                    };
                    let m = p.matrix(path.matrix);
                    // in stream space, so that the clip outlives no `cm`
                    write_segs(&mut c, &path.segs, Some(&m));
                    let op = if path.paint & paint::CLIP_EVEN_ODD != 0 {
                        "W* n"
                    } else {
                        "W n"
                    };
                    let _ = writeln!(Bytes(&mut c), "{op}");
                }
                Item::Image { id, matrix: mk } => {
                    end_text(&mut c, &mut in_text);
                    res.images.insert(*id);
                    if doc.images.get(*id).is_some_and(|i| i.kind == Kind::Png) {
                        // a PNG with alpha needs the page's transparency group
                        res.group |= png_has_alpha(doc, *id);
                    }
                    c.extend_from_slice(b"q ");
                    write_matrix(&mut c, &p.matrix(*mk));
                    let _ = writeln!(Bytes(&mut c), " cm /Im{id} Do Q");
                }
                Item::Form { id, matrix: mk } => {
                    end_text(&mut c, &mut in_text);
                    res.forms.insert(*id);
                    c.extend_from_slice(b"q ");
                    write_matrix(&mut c, &p.matrix(*mk));
                    let _ = writeln!(Bytes(&mut c), " cm /Fm{id} Do Q");
                }
                Item::Unsupported(k) => {
                    let Some(Some(raw)) = sup.raw.get(*k as usize) else {
                        continue;
                    };
                    end_text(&mut c, &mut in_text);
                    if raw.paints {
                        c.extend_from_slice(b"q ");
                        write_matrix(&mut c, &raw.ctm);
                        c.extend_from_slice(b" cm\n");
                        c.extend_from_slice(&raw.ops);
                        c.extend_from_slice(b"\nQ\n");
                    } else {
                        c.extend_from_slice(&raw.ops);
                        c.push(b'\n');
                    }
                }
                _ => {}
            }
        }
        end_text(&mut c, &mut in_text);
        c
    }

    // ---- images ---------------------------------------------------------------

    fn image(&mut self, id: u32, num: u32) -> Result<(), String> {
        let Some(img) = self.doc.images.get(id).cloned() else {
            self.object(num, b"null");
            return Ok(());
        };
        let data = std::fs::read(&img.path).map_err(|e| format!("{}: {e}", img.path))?;
        if img.kind == Kind::Png {
            if let Some(PngPass {
                dict,
                color: col,
                n_base,
                palette,
                idat,
            }) = png_passthrough(&data)
            {
                let base = if let Some(icc) = &col.icc {
                    let o = self.alloc();
                    let d = format!(" /N {n_base} /Filter /FlateDecode");
                    self.stream(o, d.as_bytes(), icc, false);
                    format!("[/ICCBased {o} 0 R]")
                } else {
                    cal_space(&col, n_base).unwrap_or_else(|| {
                        if n_base == 1 {
                            "/DeviceGray"
                        } else {
                            "/DeviceRGB"
                        }
                        .to_string()
                    })
                };
                let space = match &palette {
                    Some(p) => {
                        let hex: String = p.iter().map(|b| format!("{b:02x}")).collect();
                        format!("[/Indexed {base} {} <{hex}>]", p.len() / 3 - 1)
                    }
                    None => base,
                };
                let mut dict = dict.replace("@CS@", &space);
                if let Some(intent) = col.srgb_intent.filter(|_| col.icc.is_none()) {
                    let name = [
                        "Perceptual",
                        "RelativeColorimetric",
                        "Saturation",
                        "AbsoluteColorimetric",
                    ]
                    .get(intent as usize)
                    .copied()
                    .unwrap_or("Perceptual");
                    dict.push_str(&format!(" /Intent /{name}"));
                }
                self.stream(num, dict.as_bytes(), &idat, false);
                return Ok(());
            }
        }
        let jpeg_profile = if img.kind == Kind::Jpeg {
            jpeg_icc(&data)
        } else {
            None
        };

        let x = match img.kind {
            Kind::Png => flashtex_pdf::images::from_png(&data),
            Kind::Jpeg => flashtex_pdf::images::from_jpeg(&data),
            Kind::Pdf => flashtex_pdf::images::from_pdf_page(&data, img.page.max(1) as u32),
            Kind::Bmp => Err("BMP images are not written yet".into()),
        };
        let x = match x {
            Ok(x) => x,
            Err(e) => {
                self.object(num, b"null");
                return Err(format!("{}: {e}", img.path));
            }
        };
        // a JPEG's ICC profile: an ICCBased space in place of the Device one
        let icc_space = jpeg_profile.map(|p| {
            let o = self.alloc();
            let comps = if p.len() > 20 && &p[16..20] == b"GRAY" {
                1
            } else if p.len() > 20 && &p[16..20] == b"CMYK" {
                4
            } else {
                3
            };
            let d = format!(" /N {comps}");
            self.stream(o, d.as_bytes(), &p, true);
            format!("[/ICCBased {o} 0 R]")
        });
        // objects[0] is `num`; the others follow it
        let mut nums = vec![num];
        for _ in 1..x.objects.len() {
            nums.push(self.alloc());
        }
        for (i, o) in x.objects.iter().enumerate() {
            let mut body = String::new();
            for piece in &o.dict {
                match piece {
                    flashtex_pdf::images::Piece::Text(t) => match (&icc_space, i) {
                        (Some(cs), 0) => body.push_str(
                            &t.replace("/ColorSpace /DeviceRGB", &format!("/ColorSpace {cs}"))
                                .replace("/ColorSpace /DeviceGray", &format!("/ColorSpace {cs}"))
                                .replace("/ColorSpace /DeviceCMYK", &format!("/ColorSpace {cs}")),
                        ),
                        _ => body.push_str(t),
                    },
                    flashtex_pdf::images::Piece::Ref(r) => {
                        let _ = write!(body, "{} 0 R", nums[*r]);
                    }
                }
            }
            match &o.stream {
                Some(s) => {
                    let mut d = String::from(" ");
                    d.push_str(&body);
                    if i == 0 && icc_space.is_some() {
                        let s = jpeg_without_icc(s);
                        self.stream(nums[i], d.as_bytes(), &s, false);
                    } else {
                        self.stream(nums[i], d.as_bytes(), s, false);
                    }
                }
                None => self.object(nums[i], body.as_bytes()),
            }
        }
        Ok(())
    }

    // ---- fonts ----------------------------------------------------------------

    fn font(&mut self, id: u16, num: u32) -> Result<(), String> {
        let Some(r) = self.doc.fonts.resource(id).cloned() else {
            self.object(num, b"null");
            return Ok(());
        };
        let used = self.used.get(&id).cloned().unwrap_or_default();
        match &r.kind {
            ResKind::Native {
                index,
                cff,
                ps_name,
                ..
            } => self.native_font(num, &r, *index, *cff, ps_name, &used),
            ResKind::Type1 {
                ps_name, encoding, ..
            } => self.type1_font(num, &r, ps_name, encoding, &used),
            ResKind::Missing => {
                // nothing can draw it; Helvetica stands in so the PDF is valid
                self.object(num, b"<</Type /Font /Subtype /Type1 /BaseFont /Helvetica>>");
                Ok(())
            }
        }
    }

    fn native_font(
        &mut self,
        num: u32,
        r: &FontRes,
        index: u32,
        cff: bool,
        ps_name: &str,
        used: &BTreeSet<u16>,
    ) -> Result<(), String> {
        let font = flashtex_pdf::truetype::TrueTypeFont::parse_face(r.program.to_vec(), index)?;
        let upm = font.units_per_em as f64;
        let k = 1000.0 / upm;
        let info = FaceInfo::read(&r.program, index);
        // the codes of the content stream: glyph ids, or the CIDs of a
        // CID-keyed CFF font (its charset)
        let cids = self.cids.get(&r.id).cloned();
        let code_of = |g: u16| cids.as_ref().and_then(|m| m.get(&g)).copied().unwrap_or(g);
        // the program, unless the font's licence forbids embedding it
        let file = if info.embedding_forbidden() {
            self.warnings.push(format!(
                "{ps_name}: the font's OS/2 fsType ({:#06x}) does not allow embedding; it is not embedded",
                info.fs_type
            ));
            None
        } else {
            let (file_key, sub, program, subset) = font_program(&font, &r.program, cff, used)?;
            let ff = self.alloc();
            self.stream(ff, sub.as_bytes(), &program, true);
            Some((format!(" /{file_key} {ff} 0 R"), subset))
        };
        // a subset's name has its tag (PDF 32000-1 §9.6.4); a whole font's not
        let name = if ps_name.is_empty() { "Font" } else { ps_name };
        let base = match &file {
            Some((_, true)) => format!("{}+{name}", subset_tag(ps_name, used)),
            _ => name.to_string(),
        };
        let file = file.map(|f| f.0);
        let fd = self.alloc();
        let [bx0, by0, bx1, by1] = font.bbox;
        let mut d = String::new();
        let _ = write!(
            d,
            "<</Type /FontDescriptor /FontName /{base} /Flags {} /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV {}{}>>",
            info.flags(font.italic_angle),
            n(bx0 as f64 * k, 3),
            n(by0 as f64 * k, 3),
            n(bx1 as f64 * k, 3),
            n(by1 as f64 * k, 3),
            n(font.italic_angle, 3),
            n(font.ascender as f64 * k, 3),
            n(font.descender as f64 * k, 3),
            n(font.cap_height.unwrap_or(font.ascender) as f64 * k, 3),
            info.stem_v(),
            file.unwrap_or_default(),
        );
        self.object(fd, d.as_bytes());
        // widths
        let mut wa = String::from("[");
        let mut prev: Option<u16> = None;
        let by_code: BTreeMap<u16, u16> = used.iter().map(|&g| (code_of(g), g)).collect();
        for (&c, &g) in &by_code {
            let w = n(font.advance(g) as f64 * k, 3);
            if prev.is_some_and(|p| p + 1 == c) {
                let _ = write!(wa, " {w}");
            } else {
                if prev.is_some() {
                    wa.push(']');
                }
                let _ = write!(wa, " {c} [{w}");
            }
            prev = Some(c);
        }
        if prev.is_some() {
            wa.push(']');
        }
        wa.push_str(" ]");
        // a CID-keyed font's own character collection, else Identity
        let ros = if cids.is_some() {
            font.cff_table().and_then(super::cffcid::ros)
        } else {
            None
        };
        let csi = match ros {
            Some((r, o, sup)) => {
                let (mut rs, mut os) = (Vec::new(), Vec::new());
                super::pdfobj::write_string(&r, &mut rs);
                super::pdfobj::write_string(&o, &mut os);
                format!(
                    "<</Registry {} /Ordering {} /Supplement {sup}>>",
                    String::from_utf8_lossy(&rs),
                    String::from_utf8_lossy(&os)
                )
            }
            None => "<</Registry (Adobe) /Ordering (Identity) /Supplement 0>>".to_string(),
        };
        let cid = self.alloc();
        let mut d = String::new();
        let _ = write!(
            d,
            "<</Type /Font /Subtype /{} /BaseFont /{base} /CIDSystemInfo {csi} /FontDescriptor {fd} 0 R /DW {} /W {wa}{}>>",
            if cff { "CIDFontType0" } else { "CIDFontType2" },
            n(font.advance(0) as f64 * k, 3),
            if cff { "" } else { " /CIDToGIDMap /Identity" }
        );
        self.object(cid, d.as_bytes());
        // ToUnicode from the font's cmap
        let cmap_text = super::tounicode::cmap_reverse(&r.program, index);
        let map: BTreeMap<u16, Vec<char>> =
            super::tounicode::glyph_text(&r.program, index, &cmap_text, used)
                .into_iter()
                .map(|(g, t)| (code_of(g), t))
                .collect();
        let tu = self.alloc();
        let cmap = to_unicode_cmap(&base, &map, true);
        self.stream(tu, b"", cmap.as_bytes(), true);
        let mut d = String::new();
        let _ = write!(
            d,
            "<</Type /Font /Subtype /Type0 /BaseFont /{base} /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {tu} 0 R>>"
        );
        self.object(num, d.as_bytes());
        Ok(())
    }

    fn type1_font(
        &mut self,
        num: u32,
        r: &FontRes,
        ps_name: &str,
        encoding: &[Vec<u8>],
        used: &BTreeSet<u16>,
    ) -> Result<(), String> {
        let t1 = flashtex_pdf::type1::Type1Font::parse_pfb(&r.program)
            .map_err(|e| format!("{ps_name}: {e}"))?;
        let names: BTreeSet<String> = used
            .iter()
            .filter_map(|&c| encoding.get(c as usize))
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .filter(|n| n != ".notdef")
            .collect();
        let sub = t1.subset(&names).map_err(|e| format!("{ps_name}: {e}"))?;
        let flashtex_pdf::exact::FontProgram::Type1 {
            bytes,
            length1,
            length2,
            length3,
        } = &sub.program
        else {
            return Err(format!("{ps_name}: not a Type 1 program"));
        };
        let tag = subset_tag(ps_name, used);
        let base = format!("{tag}+{ps_name}");
        let ff = self.alloc();
        let d = format!(" /Length1 {length1} /Length2 {length2} /Length3 {length3}");
        self.stream(ff, d.as_bytes(), bytes, true);
        let bbox = t1.font_bbox().unwrap_or([0, -250, 1000, 750]);
        let fd = self.alloc();
        let d = format!(
            "<</Type /FontDescriptor /FontName /{base} /Flags {} /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV {} /FontFile {ff} 0 R>>",
            type1_flags(&r.program, t1.italic_angle().unwrap_or("0")),
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            t1.italic_angle().unwrap_or("0"),
            bbox[3],
            bbox[1],
            bbox[3],
            t1.std_vw().unwrap_or(80)
        );
        self.object(fd, d.as_bytes());
        let first = used.iter().next().copied().unwrap_or(0).min(255);
        let last = used.iter().next_back().copied().unwrap_or(0).min(255);
        let widths = self.doc.fonts.widths.get(&r.id);
        let mut wa = String::from("[");
        for c in first..=last {
            let w = widths.and_then(|m| m.get(&c)).copied().unwrap_or(0.0);
            let _ = write!(wa, " {}", n(w, 3));
        }
        wa.push_str(" ]");
        let mut diffs = String::from("[");
        let mut prev: Option<u16> = None;
        for &c in used {
            let name = encoding
                .get(c as usize)
                .map(|n| String::from_utf8_lossy(n).into_owned())
                .unwrap_or_else(|| ".notdef".into());
            if prev.is_none_or(|p| p + 1 != c) {
                let _ = write!(diffs, " {c}");
            }
            let mut nb = Vec::new();
            write_name(name.as_bytes(), &mut nb);
            diffs.push(' ');
            diffs.push_str(&String::from_utf8_lossy(&nb));
            prev = Some(c);
        }
        diffs.push_str(" ]");
        let mut map = BTreeMap::new();
        for &c in used {
            if let Some(s) = encoding
                .get(c as usize)
                .and_then(|n| glyph_name_to_unicode(n))
            {
                map.insert(c, s);
            }
        }
        let tu = self.alloc();
        let cmap = to_unicode_cmap(&base, &map, false);
        self.stream(tu, b"", cmap.as_bytes(), true);
        let d = format!(
            "<</Type /Font /Subtype /Type1 /BaseFont /{base} /FirstChar {first} /LastChar {last} /Widths {wa} /Encoding <</Type /Encoding /Differences {diffs}>> /FontDescriptor {fd} 0 R /ToUnicode {tu} 0 R>>"
        );
        self.object(num, d.as_bytes());
        Ok(())
    }

    // ---- the document --------------------------------------------------------

    fn catalog_and_tree(&mut self) -> Result<(), String> {
        let doc = self.doc;
        // the page tree
        let mut d = format!("<</Type /Pages /Count {} /Kids [", self.pages.len());
        for (i, p) in self.pages.iter().enumerate() {
            if i > 0 {
                d.push(' ');
            }
            let _ = write!(d, "{p} 0 R");
        }
        d.push_str("]>>");
        let root = self.pages_root;
        self.object(root, d.as_bytes());
        // the outline
        let outlines = if doc.outlines.is_empty() {
            None
        } else {
            Some(self.outlines())
        };
        // named destinations
        let mut names = doc.names.clone();
        if !doc.dests.is_empty() {
            let mut sorted: BTreeMap<Vec<u8>, Obj> = BTreeMap::new();
            for (k, v) in &doc.dests {
                sorted.entry(k.clone()).or_insert_with(|| v.clone());
            }
            let mut arr = Vec::new();
            for (k, v) in sorted {
                arr.push(Obj::Str(k));
                arr.push(v);
            }
            let tree = self.alloc();
            let body = self.ser(&Obj::Dict(Dict(vec![(b"Names".to_vec(), Obj::Array(arr))])));
            self.object(tree, &body);
            names.set(b"Dests", Obj::Ref(tree, 0));
        }
        let names_ref = if !names.0.is_empty() || self.names_obj.is_some() {
            let o = match self.names_obj {
                Some(o) => o,
                _ => {
                    let o = self.alloc();
                    self.names_obj = Some(o);
                    o
                }
            };
            let body = self.ser(&Obj::Dict(names));
            self.object(o, &body);
            Some(o)
        } else {
            None
        };
        // the catalog
        let mut cat = doc.catalog.clone();
        for k in [&b"Type"[..], b"Pages", b"Outlines", b"Names"] {
            cat.remove(k);
        }
        let mut d = format!("<</Type /Catalog /Pages {root} 0 R");
        if let Some(o) = outlines {
            let _ = write!(d, " /Outlines {o} 0 R");
        }
        if let Some(o) = names_ref {
            let _ = write!(d, " /Names {o} 0 R");
        }
        let mut body = d.into_bytes();
        if !cat.0.is_empty() {
            body.push(b' ');
            let e = self.ser_dict_entries(&cat);
            body.extend_from_slice(&e);
        }
        body.extend_from_slice(b">>");
        let c = self.catalog;
        self.object(c, &body);
        Ok(())
    }

    /// The outline tree from the flat list of `pdf:outline` entries.
    fn outlines(&mut self) -> u32 {
        let doc = self.doc;
        let n_items = doc.outlines.len();
        let root = self.alloc();
        let nums: Vec<u32> = (0..n_items).map(|_| self.alloc()).collect();
        // parent of each item (None: the root), by levels
        let mut parent: Vec<Option<usize>> = vec![None; n_items];
        let mut stack: Vec<usize> = Vec::new();
        for (i, o) in doc.outlines.iter().enumerate() {
            while let Some(&top) = stack.last() {
                if doc.outlines[top].level >= o.level {
                    stack.pop();
                } else {
                    break;
                }
            }
            parent[i] = stack.last().copied();
            stack.push(i);
        }
        let children =
            |p: Option<usize>| -> Vec<usize> { (0..n_items).filter(|&i| parent[i] == p).collect() };
        // visible descendants (counting through open items)
        fn visible(
            i: usize,
            kids: &dyn Fn(Option<usize>) -> Vec<usize>,
            open: &dyn Fn(usize) -> bool,
        ) -> i64 {
            let c = kids(Some(i));
            c.iter()
                .map(|&k| 1 + if open(k) { visible(k, kids, open) } else { 0 })
                .sum()
        }
        let open = |i: usize| doc.outlines[i].open;
        for i in 0..n_items {
            let o = &doc.outlines[i];
            let mut d = o.dict.clone();
            for k in [
                &b"Parent"[..],
                b"Prev",
                b"Next",
                b"First",
                b"Last",
                b"Count",
            ] {
                d.remove(k);
            }
            let mut body = b"<<".to_vec();
            let e = self.ser_dict_entries(&d);
            body.extend_from_slice(&e);
            let par = parent[i].map_or(root, |p| nums[p]);
            let sibs = children(parent[i]);
            let pos = sibs.iter().position(|&s| s == i).unwrap_or(0);
            let _ = write!(Bytes(&mut body), " /Parent {par} 0 R");
            if pos > 0 {
                let _ = write!(Bytes(&mut body), " /Prev {} 0 R", nums[sibs[pos - 1]]);
            }
            if pos + 1 < sibs.len() {
                let _ = write!(Bytes(&mut body), " /Next {} 0 R", nums[sibs[pos + 1]]);
            }
            let kids = children(Some(i));
            if let (Some(f), Some(l)) = (kids.first(), kids.last()) {
                let cnt = visible(i, &children, &open);
                let cnt = if o.open { cnt } else { -(kids.len() as i64) };
                let _ = write!(
                    Bytes(&mut body),
                    " /First {} 0 R /Last {} 0 R /Count {cnt}",
                    nums[*f],
                    nums[*l]
                );
            }
            body.extend_from_slice(b">>");
            self.object(nums[i], &body);
        }
        let top = children(None);
        let total: i64 = top
            .iter()
            .map(|&k| {
                1 + if open(k) {
                    visible(k, &children, &open)
                } else {
                    0
                }
            })
            .sum();
        let mut body = b"<</Type /Outlines".to_vec();
        if let (Some(f), Some(l)) = (top.first(), top.last()) {
            let _ = write!(
                Bytes(&mut body),
                " /First {} 0 R /Last {} 0 R /Count {total}",
                nums[*f],
                nums[*l]
            );
        }
        body.extend_from_slice(b">>");
        self.object(root, &body);
        root
    }

    fn finish(mut self) -> Vec<u8> {
        // the document information
        let info = self.alloc();
        let mut di = self.doc.docinfo.clone();
        di.0.retain(|(_, v)| !matches!(v, Obj::Str(s) if s.is_empty()));
        if di.get(b"Creator").is_none() {
            if let Some(c) = &self.doc.dvi_comment {
                di.set(b"Creator", Obj::Str(c.clone()));
            }
        }
        if di.get(b"Producer").is_none() {
            di.set(
                b"Producer",
                Obj::Str(self.opts.producer.clone().into_bytes()),
            );
        }
        if let Some(date) = &self.opts.date {
            if di.get(b"CreationDate").is_none() {
                di.set(b"CreationDate", Obj::Str(date.clone().into_bytes()));
            }
            if di.get(b"ModDate").is_none() {
                di.set(b"ModDate", Obj::Str(date.clone().into_bytes()));
            }
        }
        let body = self.ser(&Obj::Dict(di));
        self.object(info, &body);
        while let Some(name) = self.named_queue.pop() {
            self.named_object(&name);
        }
        // unwritten objects (a reference nothing filled in) are null
        for i in 0..self.offsets.len() {
            if self.offsets[i].is_none() {
                self.object(i as u32 + 1, b"null");
            }
        }
        let (maj, min) = self.doc.version;
        let mut out = format!("%PDF-{maj}.{min}\n%\u{00d0}\u{00d4}\u{00c5}\u{00d8}\n").into_bytes();
        let shift = out.len();
        out.extend_from_slice(&self.out);
        let xref = out.len();
        let _ = write!(
            Bytes(&mut out),
            "xref\n0 {}\n0000000000 65535 f \n",
            self.offsets.len() + 1
        );
        for o in &self.offsets {
            let _ = writeln!(Bytes(&mut out), "{:010} 00000 n ", o.unwrap_or(0) + shift);
        }
        let id = flashtex_display_list::sha256::sha256(&out);
        let idh: String = id[..16].iter().map(|b| format!("{b:02x}")).collect();
        let _ = write!(
            Bytes(&mut out),
            "trailer\n<</Size {} /Root {} 0 R /Info {info} 0 R /ID [<{idh}> <{idh}>]>>\nstartxref\n{xref}\n%%EOF\n",
            self.offsets.len() + 1,
            self.catalog
        );
        out
    }
}

struct Bytes<'a>(&'a mut Vec<u8>);
impl std::fmt::Write for Bytes<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

/// The program of a native font for its font descriptor: the key, the
/// stream's subtype entry, the bytes, and whether they are a subset (which
/// the font's name then says with its tag).
fn font_program(
    font: &flashtex_pdf::truetype::TrueTypeFont,
    whole: &[u8],
    cff: bool,
    used: &BTreeSet<u16>,
) -> Result<(&'static str, &'static str, Vec<u8>, bool), String> {
    if !cff {
        return Ok(("FontFile2", "", font.subset_keep_gids(used)?, true));
    }
    let table = font.cff_table().ok_or("no CFF table")?;
    let parsed = flashtex_pdf::cff::CffFont::parse(table).map_err(|e| format!("{e:?}"))?;
    if parsed.is_cid_keyed() {
        // A CID-keyed CFF (CJK fonts): its subset keeps the CIDs, which the
        // content stream writes (`cid_map`); whole if it cannot be subset.
        return Ok(match super::cffcid::subset(table, used) {
            Ok(b) => ("FontFile3", "/Subtype /CIDFontType0C", b, true),
            Err(_) => (
                "FontFile3",
                "/Subtype /CIDFontType0C",
                table.to_vec(),
                false,
            ),
        });
    }
    Ok(match parsed.subset(used) {
        Ok(s) => ("FontFile3", "/Subtype /CIDFontType0C", s.bytes, true),
        // a `seac` glyph: the whole font, as OpenType (a name-keyed CFF's
        // CIDs are its glyph ids, PDF 32000-1 §9.7.4.2)
        Err(_) => ("FontFile3", "/Subtype /OpenType", whole.to_vec(), false),
    })
}

/// For a CID-keyed CFF face: each used glyph id's CID (the font's charset,
/// through which a CIDFontType0 font selects glyphs). `None` for any other
/// face.
fn cid_map(program: &[u8], index: u32, used: &BTreeSet<u16>) -> Option<BTreeMap<u16, u16>> {
    let font = flashtex_pdf::truetype::TrueTypeFont::parse_face(program.to_vec(), index).ok()?;
    let cff = flashtex_pdf::cff::CffFont::parse(font.cff_table()?).ok()?;
    if !cff.is_cid_keyed() {
        return None;
    }
    Some(
        used.iter()
            .map(|&g| (g, cff.charset_entry(g).unwrap_or(0)))
            .collect(),
    )
}

/// What the font descriptor takes from a face's `OS/2`, `post` and `head`.
#[derive(Default)]
struct FaceInfo {
    fs_type: u16,
    weight: u16,
    family_class: u16,
    fixed_pitch: bool,
    italic_style: bool,
}

impl FaceInfo {
    fn read(data: &[u8], index: u32) -> FaceInfo {
        use super::tounicode::table;
        let rd16 = |t: &[u8], o: usize| t.get(o..o + 2).map(|s| u16::from_be_bytes([s[0], s[1]]));
        let mut f = FaceInfo {
            weight: 400,
            ..FaceInfo::default()
        };
        if let Some(os2) = table(data, index, b"OS/2") {
            f.weight = rd16(os2, 4).unwrap_or(400);
            f.fs_type = rd16(os2, 8).unwrap_or(0);
            f.family_class = rd16(os2, 30).unwrap_or(0);
            f.italic_style = rd16(os2, 62).is_some_and(|s| s & 1 != 0);
        }
        if let Some(post) = table(data, index, b"post") {
            f.fixed_pitch = post.get(12..16).is_some_and(|s| s.iter().any(|&b| b != 0));
        }
        if let Some(head) = table(data, index, b"head") {
            f.italic_style |= rd16(head, 44).is_some_and(|m| m & 2 != 0);
        }
        f
    }

    /// `fsType` (OpenType spec, OS/2): "Restricted License embedding"
    /// (only bit 1 of the usage bits) or "Bitmap embedding only" (bit 9).
    fn embedding_forbidden(&self) -> bool {
        self.fs_type & 0x000F == 0x0002 || self.fs_type & 0x0200 != 0
    }

    /// The descriptor's `/Flags` (PDF 32000-1 Table 123): FixedPitch,
    /// Serif (an `sFamilyClass` of 1-5 or 7), Symbolic (a font drawn by
    /// glyph id has no standard encoding), Italic.
    fn flags(&self, italic_angle: f64) -> u32 {
        let mut f = 4;
        if self.fixed_pitch {
            f |= 1;
        }
        if matches!(self.family_class >> 8, 1..=5 | 7) {
            f |= 2;
        }
        if self.italic_style || italic_angle != 0.0 {
            f |= 64;
        }
        f
    }

    /// `/StemV` from the weight class, as PDF writers estimate it when the
    /// font gives none: 50 + (weight / 65)².
    fn stem_v(&self) -> u32 {
        let w = self.weight as f64 / 65.0;
        (50.0 + w * w).round() as u32
    }
}

/// A Type 1 font's `/Flags`: Symbolic (TeX's fonts are re-encoded), and
/// FixedPitch and Italic from its `FontInfo`.
fn type1_flags(program: &[u8], italic_angle: &str) -> u32 {
    let clear_end = program.len().min(16 * 1024);
    let clear = &program[..clear_end];
    let mut f = 4;
    if clear.windows(17).any(|w| w == b"/isFixedPitch tru") {
        f |= 1;
    }
    if italic_angle.trim().parse::<f64>().is_ok_and(|a| a != 0.0) {
        f |= 64;
    }
    f
}

fn color_op(c: &mut Vec<u8>, v: &[f64], stroke: bool) {
    let op = match (v.len(), stroke) {
        (1, false) => "g",
        (1, true) => "G",
        (3, false) => "rg",
        (3, true) => "RG",
        (4, false) => "k",
        (4, true) => "K",
        _ => return,
    };
    for x in v {
        c.extend_from_slice(n(*x, 5).as_bytes());
        c.push(b' ');
    }
    c.extend_from_slice(op.as_bytes());
    c.push(b'\n');
}

fn write_matrix(c: &mut Vec<u8>, m: &[f64; 6]) {
    for (i, v) in m.iter().enumerate() {
        if i > 0 {
            c.push(b' ');
        }
        c.extend_from_slice(n(*v, if i < 4 { 8 } else { 6 }).as_bytes());
    }
}

/// Path segments, mapped by `m` when given.
fn write_segs(c: &mut Vec<u8>, segs: &[Seg], m: Option<&[f64; 6]>) {
    let tr = |x: f64, y: f64| -> (String, String) {
        match m {
            Some(m) => (
                n(x * m[0] + y * m[2] + m[4], 6),
                n(x * m[1] + y * m[3] + m[5], 6),
            ),
            None => (n(x, 6), n(y, 6)),
        }
    };
    for s in segs {
        match *s {
            Seg::Move(x, y) => {
                let (a, b) = tr(x, y);
                let _ = write!(Bytes(c), "{a} {b} m ");
            }
            Seg::Line(x, y) => {
                let (a, b) = tr(x, y);
                let _ = write!(Bytes(c), "{a} {b} l ");
            }
            Seg::Curve(x1, y1, x2, y2, x3, y3) => {
                let (a, b) = tr(x1, y1);
                let (cc, d) = tr(x2, y2);
                let (e, f) = tr(x3, y3);
                let _ = write!(Bytes(c), "{a} {b} {cc} {d} {e} {f} c ");
            }
            Seg::Close => c.extend_from_slice(b"h "),
        }
    }
}

/// Six capital letters naming a subset, from the font and its glyphs.
fn subset_tag(name: &str, used: &BTreeSet<u16>) -> String {
    let mut h = flashtex_display_list::sha256::Sha256::new();
    h.update(name.as_bytes());
    for g in used {
        h.update(&g.to_le_bytes());
    }
    let d = h.finish();
    d[..6].iter().map(|b| (b'A' + b % 26) as char).collect()
}

/// A `ToUnicode` CMap: codes (2 bytes for `Identity-H`, else 1) to UTF-16.
fn to_unicode_cmap(name: &str, map: &BTreeMap<u16, Vec<char>>, two_byte: bool) -> String {
    let mut s = String::new();
    let _ = write!(
        s,
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo <</Registry (Adobe) /Ordering (UCS) /Supplement 0>> def\n/CMapName /{name}-UTF16 def\n/CMapType 2 def\n1 begincodespacerange\n{}\nendcodespacerange\n",
        if two_byte { "<0000> <FFFF>" } else { "<00> <FF>" }
    );
    let entries: Vec<_> = map.iter().collect();
    for chunk in entries.chunks(100) {
        let _ = writeln!(s, "{} beginbfchar", chunk.len());
        for (code, chars) in chunk {
            let mut u = String::new();
            for ch in chars.iter() {
                let mut buf = [0u16; 2];
                for unit in ch.encode_utf16(&mut buf) {
                    let _ = write!(u, "{unit:04X}");
                }
            }
            if two_byte {
                let _ = writeln!(s, "<{code:04X}> <{u}>");
            } else {
                let _ = writeln!(s, "<{:02X}> <{u}>", *code & 0xFF);
            }
        }
        s.push_str("endbfchar\n");
    }
    s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    s
}

/// A glyph name's Unicode text: `uniXXXX`, `uXXXX[XX]`, a single ASCII
/// letter or digit, or one of the standard names of TeX's fonts.
fn glyph_name_to_unicode(name: &[u8]) -> Option<Vec<char>> {
    let s = std::str::from_utf8(name).ok()?;
    if let Some(h) = s.strip_prefix("uni") {
        if h.len() % 4 == 0 && !h.is_empty() {
            let v: Option<Vec<char>> = h
                .as_bytes()
                .chunks(4)
                .map(|c| {
                    u32::from_str_radix(std::str::from_utf8(c).ok()?, 16)
                        .ok()
                        .and_then(char::from_u32)
                })
                .collect();
            return v;
        }
    }
    if let Some(h) = s.strip_prefix('u') {
        if (4..=6).contains(&h.len()) {
            if let Some(c) = u32::from_str_radix(h, 16).ok().and_then(char::from_u32) {
                return Some(vec![c]);
            }
        }
    }
    if s.len() == 1 && s.as_bytes()[0].is_ascii_alphanumeric() {
        return Some(vec![s.as_bytes()[0] as char]);
    }
    let t: &str = match s {
        "space" => " ",
        "exclam" => "!",
        "quotedbl" => "\"",
        "numbersign" => "#",
        "dollar" => "$",
        "percent" => "%",
        "ampersand" => "&",
        "quoteright" | "quotesingle" => "\u{2019}",
        "parenleft" => "(",
        "parenright" => ")",
        "asterisk" => "*",
        "plus" => "+",
        "comma" => ",",
        "hyphen" => "-",
        "period" => ".",
        "slash" => "/",
        "zero" => "0",
        "one" => "1",
        "two" => "2",
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" => "9",
        "colon" => ":",
        "semicolon" => ";",
        "less" => "<",
        "equal" => "=",
        "greater" => ">",
        "question" => "?",
        "at" => "@",
        "bracketleft" => "[",
        "backslash" => "\\",
        "bracketright" => "]",
        "asciicircum" | "circumflex" => "^",
        "underscore" => "_",
        "quoteleft" => "\u{2018}",
        "braceleft" => "{",
        "bar" => "|",
        "braceright" => "}",
        "asciitilde" | "tilde" => "~",
        "ff" => "ff",
        "fi" => "fi",
        "fl" => "fl",
        "ffi" => "ffi",
        "ffl" => "ffl",
        "dotlessi" => "\u{131}",
        "dotlessj" => "\u{237}",
        "endash" => "\u{2013}",
        "emdash" => "\u{2014}",
        "quotedblleft" => "\u{201C}",
        "quotedblright" => "\u{201D}",
        "germandbls" => "\u{DF}",
        "ae" => "\u{E6}",
        "oe" => "\u{153}",
        "oslash" => "\u{F8}",
        "AE" => "\u{C6}",
        "OE" => "\u{152}",
        "Oslash" => "\u{D8}",
        "Gamma" => "\u{393}",
        "Delta" => "\u{394}",
        "Theta" => "\u{398}",
        "Lambda" => "\u{39B}",
        "Xi" => "\u{39E}",
        "Pi" => "\u{3A0}",
        "Sigma" => "\u{3A3}",
        "Upsilon" => "\u{3A5}",
        "Phi" => "\u{3A6}",
        "Psi" => "\u{3A8}",
        "Omega" => "\u{3A9}",
        "alpha" => "\u{3B1}",
        "beta" => "\u{3B2}",
        "gamma" => "\u{3B3}",
        "delta" => "\u{3B4}",
        "epsilon" => "\u{3F5}",
        "zeta" => "\u{3B6}",
        "eta" => "\u{3B7}",
        "theta" => "\u{3B8}",
        "iota" => "\u{3B9}",
        "kappa" => "\u{3BA}",
        "lambda" => "\u{3BB}",
        "mu" => "\u{3BC}",
        "nu" => "\u{3BD}",
        "xi" => "\u{3BE}",
        "pi" => "\u{3C0}",
        "rho" => "\u{3C1}",
        "sigma" => "\u{3C3}",
        "tau" => "\u{3C4}",
        "upsilon" => "\u{3C5}",
        "phi" => "\u{3D5}",
        "chi" => "\u{3C7}",
        "psi" => "\u{3C8}",
        "omega" => "\u{3C9}",
        "minus" => "\u{2212}",
        "multiply" => "\u{D7}",
        "divide" => "\u{F7}",
        "infinity" => "\u{221E}",
        "periodcentered" => "\u{B7}",
        "bullet" => "\u{2022}",
        "integraltext" | "integraldisplay" | "integral" => "\u{222B}",
        "contintegraltext" | "contintegraldisplay" => "\u{222E}",
        "summationtext" | "summationdisplay" | "summation" => "\u{2211}",
        "producttext" | "productdisplay" | "product" => "\u{220F}",
        "coproducttext" | "coproductdisplay" => "\u{2210}",
        "uniontext" | "uniondisplay" => "\u{22C3}",
        "intersectiontext" | "intersectiondisplay" => "\u{22C2}",
        "radical" | "radicalbig" | "radicalBig" | "radicalbigg" | "radicalBigg" => "\u{221A}",
        "arrowright" => "\u{2192}",
        "arrowleft" => "\u{2190}",
        "element" => "\u{2208}",
        "lessequal" => "\u{2264}",
        "greaterequal" => "\u{2265}",
        "plusminus" => "\u{B1}",
        "partialdiff" => "\u{2202}",
        "prime" => "\u{2032}",
        "similar" => "\u{223C}",
        "approxequal" => "\u{2248}",
        "notequal" => "\u{2260}",
        "equivalence" => "\u{2261}",
        "nabla" => "\u{2207}",
        "angbracketleft" => "\u{27E8}",
        "angbracketright" => "\u{27E9}",
        _ => return None,
    };
    Some(t.chars().collect())
}

/// What a PNG says about its colours (dvipdfmx's order: `iCCP`, then
/// `sRGB`, then `cHRM`/`gAMA`).
#[derive(Default)]
struct PngColor {
    /// The `iCCP` profile, zlib-compressed as the chunk holds it.
    icc: Option<Vec<u8>>,
    srgb_intent: Option<u8>,
    gamma: Option<f64>,
    chrm: Option<[f64; 8]>,
}

/// A PNG whose `IDAT` data a PDF can take as it is.
struct PngPass {
    /// The image dictionary's entries, `@CS@` where the colour space goes.
    dict: String,
    color: PngColor,
    /// Components of the base colour space (1 or 3).
    n_base: u8,
    palette: Option<Vec<u8>>,
    idat: Vec<u8>,
}

/// A PNG whose `IDAT` data a PDF can take as it is (8 bits or fewer per
/// sample, no alpha, no transparency key, not interlaced), as pdfTeX and
/// dvipdfmx copy it.
fn png_passthrough(data: &[u8]) -> Option<PngPass> {
    if data.len() < 8 || data[..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return None;
    }
    let mut pos = 8;
    let (mut w, mut h, mut bits, mut ctype, mut interlace) = (0u32, 0u32, 0u8, 0u8, 0u8);
    let mut plte: Option<&[u8]> = None;
    let mut idat = Vec::new();
    let mut col = PngColor::default();
    let be32 = |p: usize| -> Option<u32> {
        Some(u32::from_be_bytes(data.get(p..p + 4)?.try_into().ok()?))
    };
    loop {
        let len = be32(pos)? as usize;
        let kind = data.get(pos + 4..pos + 8)?;
        let body = data.get(pos + 8..pos + 8 + len)?;
        match kind {
            b"IHDR" => {
                w = be32(pos + 8)?;
                h = be32(pos + 12)?;
                bits = body[8];
                ctype = body[9];
                interlace = body[12];
            }
            b"PLTE" => plte = Some(body),
            b"tRNS" => return None,
            b"IDAT" => idat.extend_from_slice(body),
            b"iCCP" => {
                let z = body.iter().position(|&c| c == 0)?;
                col.icc = body.get(z + 2..).map(<[u8]>::to_vec);
            }
            b"sRGB" => col.srgb_intent = body.first().copied(),
            b"gAMA" if len == 4 => col.gamma = Some(be32(pos + 8)? as f64 / 100_000.0),
            b"cHRM" if len == 32 => {
                let mut v = [0.0; 8];
                for (i, x) in v.iter_mut().enumerate() {
                    *x = be32(pos + 8 + 4 * i)? as f64 / 100_000.0;
                }
                col.chrm = Some(v);
            }
            b"IEND" => break,
            _ => {}
        }
        pos += 12 + len;
    }
    if interlace != 0 || bits > 8 || !matches!(ctype, 0 | 2 | 3) {
        return None;
    }
    let colors = if ctype == 2 { 3 } else { 1 };
    let dict = format!(
        " /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace @CS@ /BitsPerComponent {bits} /Filter /FlateDecode /DecodeParms <</Predictor 15 /Colors {colors} /BitsPerComponent {bits} /Columns {w}>>"
    );
    let palette = if ctype == 3 {
        Some(plte?.to_vec())
    } else {
        None
    };
    Some(PngPass {
        dict,
        color: col,
        n_base: if ctype == 0 { 1 } else { 3 },
        palette,
        idat,
    })
}

/// A CIE-based space from a PNG's `sRGB` chunk, or its `gAMA` and `cHRM`
/// (measured: dvipdfmx writes an `sRGB` grey PNG as `[/CalGray <</Gamma
/// 2.2 /WhitePoint [.95046 1 1.08906]>>]`).
fn cal_space(col: &PngColor, n: u8) -> Option<String> {
    let srgb = [0.3127, 0.329, 0.64, 0.33, 0.3, 0.6, 0.15, 0.06];
    let (gamma, chrm) = if col.srgb_intent.is_some() {
        (2.2, srgb)
    } else if col.gamma.is_some() || col.chrm.is_some() {
        let g = col
            .gamma
            .map_or(2.2, |g| if g > 0.0 { 1.0 / g } else { 2.2 });
        (g, col.chrm.unwrap_or(srgb))
    } else {
        return None;
    };
    let [xw, yw, xr, yr, xg, yg, xb, yb] = chrm;
    let wp = [xw / yw, 1.0, (1.0 - xw - yw) / yw];
    let f = n_dot;
    let wps = format!("[{} {} {}]", f(wp[0]), f(wp[1]), f(wp[2]));
    if n == 1 {
        return Some(format!(
            "[/CalGray <</Gamma {} /WhitePoint {wps}>>]",
            f(gamma)
        ));
    }
    // the primaries' XYZ, scaled so that they sum to the white point
    let z = |x: f64, y: f64| [x / y, 1.0, (1.0 - x - y) / y];
    let (r, g, b) = (z(xr, yr), z(xg, yg), z(xb, yb));
    let m = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
    let det3 = |m: &[[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let det = det3(&m);
    if det.abs() < 1e-12 {
        return None;
    }
    let solve = |c: usize| {
        let mut mm = m;
        for (row, w) in mm.iter_mut().zip(wp) {
            row[c] = w;
        }
        det3(&mm) / det
    };
    let (sr, sg, sb) = (solve(0), solve(1), solve(2));
    let mx = [
        r[0] * sr,
        r[1] * sr,
        r[2] * sr,
        g[0] * sg,
        g[1] * sg,
        g[2] * sg,
        b[0] * sb,
        b[1] * sb,
        b[2] * sb,
    ];
    let ms: Vec<String> = mx.iter().map(|v| f(*v)).collect();
    Some(format!(
        "[/CalRGB <</Gamma [{0} {0} {0}] /Matrix [{1}] /WhitePoint {wps}>>]",
        f(gamma),
        ms.join(" ")
    ))
}

/// dvipdfmx's numbers: 5 decimals, no leading zero (`.95046`).
fn n_dot(v: f64) -> String {
    let s = n(v, 5);
    if let Some(r) = s.strip_prefix("0.") {
        format!(".{r}")
    } else if let Some(r) = s.strip_prefix("-0.") {
        format!("-.{r}")
    } else {
        s
    }
}

/// A JPEG's bytes without its `APP2` `ICC_PROFILE` segments.
fn jpeg_without_icc(data: &[u8]) -> Vec<u8> {
    let mut out = data[..2.min(data.len())].to_vec();
    let mut i = 2;
    while i + 4 <= data.len() && data[i] == 0xFF {
        let m = data[i + 1];
        if m == 0xDA {
            break;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        let end = (i + 2 + len).min(data.len());
        let icc = m == 0xE2 && data.get(i + 4..i + 16) == Some(&b"ICC_PROFILE\0"[..]);
        if !icc {
            out.extend_from_slice(&data[i..end]);
        }
        i = end;
    }
    out.extend_from_slice(&data[i.min(data.len())..]);
    out
}

/// The ICC profile of a JPEG file (its `APP2` `ICC_PROFILE` segments, in
/// order), if it has one.
fn jpeg_icc(data: &[u8]) -> Option<Vec<u8>> {
    let mut parts: Vec<(u8, Vec<u8>)> = Vec::new();
    let mut i = 2;
    while i + 4 <= data.len() {
        if data[i] != 0xFF {
            break;
        }
        let m = data[i + 1];
        if m == 0xD8 || (0xD0..=0xD7).contains(&m) || m == 0x01 {
            i += 2;
            continue;
        }
        if m == 0xDA || m == 0xD9 {
            break;
        }
        let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
        let seg = data.get(i + 4..i + 2 + len)?;
        if m == 0xE2 && seg.starts_with(b"ICC_PROFILE\0") && seg.len() > 14 {
            parts.push((seg[12], seg[14..].to_vec()));
        }
        i += 2 + len;
    }
    if parts.is_empty() {
        return None;
    }
    parts.sort_by_key(|p| p.0);
    Some(parts.into_iter().flat_map(|p| p.1).collect())
}

fn png_has_alpha(doc: &Doc, id: u32) -> bool {
    let Some(img) = doc.images.get(id) else {
        return false;
    };
    let Ok(data) = std::fs::read(&img.path) else {
        return false;
    };
    data.get(25).is_some_and(|&t| t == 4 || t == 6)
}

#[cfg(test)]
mod tests {
    use super::super::doc::{Built, Doc, Supplement};
    use super::super::fonts::{FontDef, NativeDef};
    use super::*;
    use flashtex_display_list::page::{Page, StreamKind};
    use flashtex_pdf::reader::{Obj, PdfFile};

    /// A page drawing glyph `gid` of `path` (face 0) at 10 pt, written.
    fn one_glyph_pdf(path: &str, gid: u16) -> Vec<u8> {
        let mut doc = Doc::new();
        doc.fonts.define(
            0,
            FontDef::Native(NativeDef {
                path: path.as_bytes().to_vec(),
                index: 0,
                size: 10 << 16,
                flags: 0,
                rgba: None,
                extend: None,
                slant: None,
                embolden: None,
            }),
        );
        let res = doc.fonts.get(0).unwrap().res;
        let mut p = Page::new(StreamKind::Page, 0);
        p.pdf_box = [0.0, 0.0, 100.0, 100.0];
        p.matrices.push([10.0, 0.0, 0.0, 10.0, 0.0, 0.0]);
        p.items = vec![
            Item::Matrix(1),
            Item::Glyph {
                font: res,
                code: gid,
                x: 657_818,
                y: 3_289_088,
                col: flashtex_display_list::page::NO_COLUMN,
            },
        ];
        doc.pages.push(Built {
            dl: p,
            sup: Supplement::default(),
            fonts: vec![res],
            images: vec![],
            forms: vec![],
        });
        let opts = Options {
            producer: "test".into(),
            date: None,
            compress: false,
        };
        write(&doc, &opts).unwrap().0
    }

    /// A CID-keyed CFF font (CJK): the content stream draws its glyphs by
    /// their CIDs (the charset), the CIDFont program is the font's CFF, and
    /// `/W` is keyed by CID. TeX Live's HaranoAji Mincho; skipped without it.
    #[test]
    fn cid_keyed_cff_fonts_are_drawn_by_cid() {
        let Some(path) = std::process::Command::new("kpsewhich")
            .arg("HaranoAjiMincho-Regular.otf")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|p| !p.is_empty())
        else {
            eprintln!("skipped: no HaranoAjiMincho-Regular.otf (TeX Live)");
            return;
        };
        let data = std::fs::read(&path).unwrap();
        let font = flashtex_pdf::truetype::TrueTypeFont::parse(data).unwrap();
        let gid = font.glyph_id('漢').unwrap();
        let cff = flashtex_pdf::cff::CffFont::parse(font.cff_table().unwrap()).unwrap();
        assert!(cff.is_cid_keyed());
        let cid = cff.charset_entry(gid).unwrap();
        let bytes = one_glyph_pdf(&path, gid);
        let pdf = PdfFile::parse(&bytes).unwrap();
        let page = pdf.pages().unwrap()[0];
        let content = String::from_utf8(pdf.page_content(page).unwrap()).unwrap();
        assert!(content.contains(&format!("<{cid:04X}>Tj")), "{content}");
        let f = *pdf.page_fonts(page).values().next().unwrap();
        let d = pdf
            .resolve(&f.get("DescendantFonts").and_then(Obj::as_array).unwrap()[0])
            .as_dict()
            .unwrap();
        let w = d.get("W").and_then(Obj::as_array).unwrap();
        assert_eq!(w[0].as_number(), Some(cid.to_string().as_str()));
        let fd = pdf
            .resolve(d.get("FontDescriptor").unwrap())
            .as_dict()
            .unwrap();
        let ff = pdf.resolve(fd.get("FontFile3").unwrap());
        let sub = ff.as_dict().unwrap().get("Subtype").and_then(Obj::as_name);
        assert_eq!(sub, Some("CIDFontType0C"));
        let program = pdf.decode_stream(ff).unwrap();
        let sub = flashtex_pdf::cff::CffFont::parse(&program).unwrap();
        assert!(sub.is_cid_keyed());
        // a subset (xelatex's is 6.8 kB), so its name has a tag
        assert_eq!(sub.glyph_count(), 2);
        assert!(program.len() < 30_000, "{} bytes", program.len());
        let base = f.get("BaseFont").and_then(Obj::as_name).unwrap();
        assert_eq!(base.as_bytes()[6], b'+', "{base}");
        // ToUnicode: the ideograph
        let tu = pdf
            .decode_stream(pdf.resolve(f.get("ToUnicode").unwrap()))
            .unwrap();
        let tu = String::from_utf8(tu).unwrap();
        assert!(tu.contains(&format!("<{cid:04X}> <6F22>")), "{tu}");
        // its character collection, as xdvipdfmx writes it
        let csi = d.get("CIDSystemInfo").and_then(Obj::as_dict).unwrap();
        assert_eq!(csi.get("Ordering"), Some(&Obj::String(b"Japan1".to_vec())));
    }

    /// 日 and 文 share their glyphs with the Kangxi radicals U+2F47 and
    /// U+2F42: the text is the unified ideograph.
    #[test]
    fn cjk_glyphs_read_as_unified_ideographs() {
        let Some(path) = std::process::Command::new("kpsewhich")
            .arg("HaranoAjiMincho-Regular.otf")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|p| !p.is_empty())
        else {
            eprintln!("skipped: no HaranoAjiMincho-Regular.otf (TeX Live)");
            return;
        };
        let data = std::fs::read(&path).unwrap();
        let rev = super::super::tounicode::cmap_reverse(&data, 0);
        let font = flashtex_pdf::truetype::TrueTypeFont::parse(data).unwrap();
        for c in ['日', '文'] {
            let g = font.glyph_id(c).unwrap();
            assert_eq!(rev.get(&g), Some(&vec![c]));
        }
    }

    /// `/Flags`, `/StemV` and the embedding permission from the face: LM
    /// Mono is fixed pitch, regular weight, installable.
    #[test]
    fn font_descriptors_say_what_the_face_is() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../apps/mac/Fonts/lmmono10-regular.otf"
        );
        let info = FaceInfo::read(&std::fs::read(path).unwrap(), 0);
        assert_eq!(info.flags(0.0) & 1, 1, "fixed pitch");
        assert_eq!(info.flags(-9.0) & 64, 64, "italic");
        assert_eq!(info.stem_v(), 88); // weight 400
        assert!(!info.embedding_forbidden());
        let restricted = FaceInfo {
            fs_type: 0x0002,
            ..FaceInfo::default()
        };
        assert!(restricted.embedding_forbidden());
        let editable = FaceInfo {
            fs_type: 0x0008,
            ..FaceInfo::default()
        };
        assert!(!editable.embedding_forbidden());
    }
}
