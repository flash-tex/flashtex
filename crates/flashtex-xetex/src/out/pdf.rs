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
//!   font's advances and, as xdvipdfmx writes one, a `ToUnicode` CMap (`tounicode`).
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

use super::agl::Agl;
use super::doc::{Doc, Named, Outline, Supplement};
use super::fonts::{FaceData, FontRes, ResKind};
use super::images::Kind;
use super::num::{fmt_num, push_num, push_uint};
use super::pdfobj::{write_name, Dict, Obj, Writer as ObjWriter};
use flashtex_display_list::page::{paint, Item, Page, RuleKind, Seg, Stroke};
use flashtex_display_list::sha256::{sha256, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::sync::Arc;

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
    /// The file so far, from its header on.
    out: Vec<u8>,
    /// The hash of `out` for the trailer's `/ID`, as it is written.
    id: IdHash,
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
    /// By `.enc` file: the `/Encoding` entry and ToUnicode object its Type 1
    /// fonts share ([`Writer::shared_encoding`]).
    encodings: BTreeMap<Vec<u8>, (String, Option<u32>)>,
    page_group: Vec<bool>,
    /// For a native font with CID-keyed CFF outlines: each glyph id's CID
    /// (the font's charset), which the content stream writes as the code.
    cids: BTreeMap<u16, BTreeMap<u16, u16>>,
    /// The Adobe Glyph List, as xdvipdfmx loads it (`agl_init_map`), read
    /// when a font first needs a glyph name's text (a resident engine's
    /// runs share it: `FontCache::agl`).
    agl: Option<Arc<Agl>>,
    /// Without a resident engine's cache, the faces this PDF parsed.
    faces: HashMap<[u8; 32], Arc<FaceData>>,
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
            out: header(doc.version),
            id: IdHash::new(),
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
            encodings: BTreeMap::new(),
            page_group: Vec::new(),
            cids: BTreeMap::new(),
            agl: None,
            faces: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    fn agl(&mut self) -> Arc<Agl> {
        let cache = &self.doc.fonts.cache;
        self.agl
            .get_or_insert_with(|| match cache {
                Some(c) => c.agl(),
                None => Arc::new(Agl::load()),
            })
            .clone()
    }

    /// Face `index` of the program whose SHA-256 is `sha`, parsed once (by
    /// the resident engine's cache, else once per PDF).
    fn face(&mut self, sha: &[u8; 32], program: &Arc<Vec<u8>>, index: u32) -> Arc<FaceData> {
        let fonts = &self.doc.fonts;
        match &fonts.cache {
            Some(c) => c.face(sha, program, index),
            None => self
                .faces
                .entry(super::fonts::face_key(sha, index))
                .or_insert_with(|| {
                    fonts
                        .parsed_face(sha, index)
                        .unwrap_or_else(|| Arc::new(FaceData::new(program, index)))
                })
                .clone(),
        }
    }

    fn alloc(&mut self) -> u32 {
        self.offsets.push(None);
        self.offsets.len() as u32
    }

    fn begin(&mut self, num: u32) {
        self.id.feed(&self.out, IdHash::CHUNK);
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
        let all_used = std::mem::take(&mut self.used);
        for (&id, used) in &all_used {
            let Some(r) = doc.fonts.resource(id) else {
                continue;
            };
            let ResKind::Native {
                index,
                cff: true,
                sha,
                ..
            } = &r.kind
            else {
                continue;
            };
            let data = self.face(sha, &r.program, *index);
            let mut memo = data.memo.lock().unwrap_or_else(|e| e.into_inner());
            let m = match &memo.cids {
                Some((u, m)) if u == used => m.clone(),
                _ => {
                    let m = data.font.as_ref().ok().and_then(|f| cid_map(f, used));
                    memo.cids = Some((used.clone(), m.clone()));
                    m
                }
            };
            drop(memo);
            if let Some(m) = m {
                self.cids.insert(id, m);
            }
        }
        self.used = all_used;
        for i in 0..doc.pages.len() {
            self.page(i)?;
        }
        // Forms, images and fonts the pages used (forms may use forms), then
        // the catalog, then named objects (anything above may have
        // referenced them). A named object may itself refer to a form or an
        // image no page draws: pgf's soft mask, `/SMask <</G @pgfsmask5>>`
        // in `@pgfextgs`. So the round repeats until nothing new was
        // referenced; a form only a named object names is written too.
        let mut done_forms = BTreeSet::new();
        let mut done_images = BTreeSet::new();
        let mut done_fonts = BTreeSet::new();
        let mut catalog = false;
        loop {
            let mut wrote = false;
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
                wrote = true;
            }
            let imgs: Vec<(u32, u32)> = self
                .images
                .iter()
                .filter(|(k, _)| !done_images.contains(*k))
                .map(|(&k, &v)| (k, v))
                .collect();
            for (id, num) in imgs {
                done_images.insert(id);
                self.image(id, num)?;
                wrote = true;
            }
            let fonts: Vec<(u16, u32)> = self
                .fonts
                .iter()
                .filter(|(k, _)| !done_fonts.contains(*k))
                .map(|(&k, &v)| (k, v))
                .collect();
            for (id, num) in fonts {
                done_fonts.insert(id);
                self.font(id, num)?;
                wrote = true;
            }
            if !catalog {
                catalog = true;
                self.catalog_and_tree()?;
            }
            while let Some(name) = self.named_queue.pop() {
                self.named_object(&name);
                wrote = true;
            }
            if !wrote {
                break;
            }
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
        // dvipdfmx's `pdf_doc_add_annot`: the dictionary as the special gave
        // it (no `/Type` added) with its `/Rect`.
        for a in &b.sup.annots {
            // `pdf:ann @name`: the named object, written once wherever it
            // is named (its dictionary already has the `/Rect`)
            if let Some(n) = &a.name {
                if self.resolve(n).is_some() {
                    if let Some(&o) = self.named.get(n.as_slice()) {
                        annots.push(o);
                        continue;
                    }
                }
            }
            let mut d = a.dict.clone();
            if let Some(g) = &a.goto {
                g.apply(&mut d);
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
        let mut cur_font: Option<(u16, Vec<u8>)> = None;
        // the size this glyph's `Tf` gives (reused)
        let mut size = Vec::with_capacity(16);
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
                    size.clear();
                    if upright {
                        push_num(&mut size, a, 6);
                    } else {
                        size.push(b'1');
                    }
                    if !matches!(&cur_font, Some((f, s)) if f == font && *s == size) {
                        // `/F{font} {size} Tf`
                        c.extend_from_slice(b"/F");
                        push_uint(&mut c, *font as u64);
                        c.push(b' ');
                        c.extend_from_slice(&size);
                        c.extend_from_slice(b" Tf\n");
                        cur_font = Some((*font, size.clone()));
                    }
                    // `a b c d x y Tm`
                    if upright {
                        c.extend_from_slice(b"1 0 0 1 ");
                    } else {
                        for v in [a, b, cc, d] {
                            push_num(&mut c, v, 6);
                            c.push(b' ');
                        }
                    }
                    push_num(&mut c, x_of(*x), 5);
                    c.push(b' ');
                    push_num(&mut c, y_of(*y), 5);
                    c.extend_from_slice(b" Tm");
                    // `<code>Tj`, four hex digits (Identity-H) or two
                    if two_byte {
                        let code = self
                            .cids
                            .get(font)
                            .and_then(|m| m.get(code))
                            .copied()
                            .unwrap_or(*code);
                        push_hex_tj(&mut c, code as u32, 4);
                    } else {
                        push_hex_tj(&mut c, (code & 0xFF) as u32, 2);
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
                    write_stroke(&mut c, st);
                    c.push(b'\n');
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
                            // `l b w h re f`
                            push_nums(&mut c, &[l, b, wb, hb], 5);
                            c.extend_from_slice(b" re f\n");
                        }
                        RuleKind::StrokeH => {
                            let ym = b + hb / 2.0;
                            // `q 0 J [] 0 d h w l y m r y l S Q`
                            c.extend_from_slice(b"q 0 J [] 0 d ");
                            push_num(&mut c, hb, 5);
                            c.extend_from_slice(b" w ");
                            push_nums(&mut c, &[l, ym], 5);
                            c.extend_from_slice(b" m ");
                            push_nums(&mut c, &[l + wb, ym], 5);
                            c.extend_from_slice(b" l S Q\n");
                        }
                        RuleKind::StrokeV => {
                            let xm = l + wb / 2.0;
                            c.extend_from_slice(b"q 0 J [] 0 d ");
                            push_num(&mut c, wb, 5);
                            c.extend_from_slice(b" w ");
                            push_nums(&mut c, &[xm, b], 5);
                            c.extend_from_slice(b" m ");
                            push_nums(&mut c, &[xm, t], 5);
                            c.extend_from_slice(b" l S Q\n");
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
                        write_stroke(&mut c, s);
                        c.push(b' ');
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
                    c.extend_from_slice(op.as_bytes());
                    c.extend_from_slice(b" Q\n");
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
                    c.extend_from_slice(op.as_bytes());
                    c.push(b'\n');
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
                    c.extend_from_slice(b" cm /Im");
                    push_uint(&mut c, *id as u64);
                    c.extend_from_slice(b" Do Q\n");
                }
                Item::Form { id, matrix: mk } => {
                    end_text(&mut c, &mut in_text);
                    res.forms.insert(*id);
                    c.extend_from_slice(b"q ");
                    write_matrix(&mut c, &p.matrix(*mk));
                    c.extend_from_slice(b" cm /Fm");
                    push_uint(&mut c, *id as u64);
                    c.extend_from_slice(b" Do Q\n");
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
                trans,
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
                match &trans {
                    PngTrans::None => {}
                    PngTrans::Key(k) => {
                        let k: Vec<String> = k.iter().map(u32::to_string).collect();
                        dict.push_str(&format!(" /Mask [{}]", k.join(" ")));
                    }
                    PngTrans::Alpha(a) => {
                        // the decoded indices: the PNG without its tRNS
                        let rows = flashtex_pdf::raster::decode_png(&png_without_trns(&data))
                            .map_err(|e| format!("{}: {e}", img.path))?;
                        let sm =
                            png_soft_mask(&rows.samples, rows.bits, rows.width, rows.height, a);
                        let o = self.alloc();
                        let d = format!(
                            " /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8",
                            rows.width, rows.height
                        );
                        self.stream(o, d.as_bytes(), &sm, true);
                        dict.push_str(&format!(" /SMask {o} 0 R"));
                    }
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
            // xdvipdfmx's box (`set_bounding_box`); the placement
            // (special.rs `place_image`) makes up for /Matrix's convention
            Kind::Pdf => flashtex_pdf::images::from_pdf_page_box(
                &data,
                img.page.max(1) as u32,
                Some(img.bbox),
            ),
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
                sha,
                ..
            } => {
                let data = self.face(sha, &r.program, *index);
                self.native_font(num, &r, &data, *index, *cff, ps_name, &used)
            }
            ResKind::Type1 {
                ps_name,
                encoding,
                enc,
                ..
            } => self.type1_font(num, &r, ps_name, encoding, enc.as_deref(), &used),
            ResKind::Missing => {
                // nothing can draw it; Helvetica stands in so the PDF is valid
                self.object(num, b"<</Type /Font /Subtype /Type1 /BaseFont /Helvetica>>");
                Ok(())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn native_font(
        &mut self,
        num: u32,
        r: &FontRes,
        data: &FaceData,
        index: u32,
        cff: bool,
        ps_name: &str,
        used: &BTreeSet<u16>,
    ) -> Result<(), String> {
        let font = data.font.as_ref().map_err(String::clone)?;
        let upm = font.units_per_em as f64;
        let k = 1000.0 / upm;
        let info = FaceInfo::read(&r.program, index);
        // the codes of the content stream: glyph ids, or the CIDs of a
        // CID-keyed CFF font (its charset)
        let cids = self.cids.get(&r.id).cloned();
        let code_of = |g: u16| cids.as_ref().and_then(|m| m.get(&g)).copied().unwrap_or(g);
        // the program, whatever the font's licence says: XeTeX runs
        // `xdvipdfmx -q -E`, and `-E` (ignore_font_license) makes
        // tt_get_fontdesc embed every font, silently (measured: fsType 2)
        let file = {
            // (the last run's, when it drew the same glyphs)
            let made = {
                let mut memo = data.memo.lock().unwrap_or_else(|e| e.into_inner());
                match &memo.program {
                    Some((c, u, p)) if *c == cff && u == used => p.clone(),
                    _ => {
                        let p = Arc::new(font_program(font, &r.program, cff, used));
                        memo.program = Some((cff, used.clone(), p.clone()));
                        p
                    }
                }
            };
            let (file_key, sub, program, subset) = made.as_ref().as_ref().map_err(String::clone)?;
            let ff = self.alloc();
            self.stream(ff, sub.as_bytes(), program, true);
            Some((format!(" /{file_key} {ff} 0 R"), *subset))
        };
        // a CFF font is named by its CFF's Name INDEX (cidtype0.c
        // `cff_get_name`), not the `name` table: the faces of a collection
        // may share one CFF
        let cff_name = if cff {
            font.cff_table()
                .and_then(|t| flashtex_pdf::cff::CffFont::parse(t).ok())
                .map(|c| c.name())
                .filter(|n| !n.is_empty())
        } else {
            None
        };
        let ps_name = cff_name.as_deref().unwrap_or(ps_name);
        // a subset's name has its tag (PDF 32000-1 §9.6.4); a whole font's not
        let name = if ps_name.is_empty() { "Font" } else { ps_name };
        let base = match &file {
            Some((_, true)) => format!("{}+{name}", subset_tag(ps_name, used)),
            _ => name.to_string(),
        };
        let file = file.map(|f| f.0);
        let fd = self.alloc();
        // a CFF font's own StdVW (CIDFont_type0_t1cdofont), not a
        // CID-keyed one's
        let std_vw = if cff && cids.is_none() {
            font.cff_table()
                .and_then(|t| super::cffcid::private_value(t, 11))
        } else {
            None
        };
        let d = format!(
            "<</Type /FontDescriptor /FontName /{base}{}{}>>",
            info.descriptor_entries(std_vw),
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
        // xdvipdfmx writes no ToUnicode for a standard Adobe character
        // collection (Type0Font_attach_ToUnicode_stream)
        let acc = ros
            .as_ref()
            .is_some_and(|(reg, ord, _)| super::tounicode::is_acc_font(reg, ord));
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
        // ToUnicode as xdvipdfmx derives it (super::tounicode), by code
        let map = if acc {
            None
        } else {
            let codes: BTreeSet<u16> = used.iter().map(|&g| code_of(g)).collect();
            let agl = self.agl();
            // (the last run's, for the same codes and glyph list)
            let mut memo = data.memo.lock().unwrap_or_else(|e| e.into_inner());
            match &memo.to_unicode {
                Some((c, a, m)) if *c == codes && Arc::ptr_eq(a, &agl) => m.clone(),
                _ => {
                    let m = super::tounicode::otf_to_unicode(&r.program, index, &codes, &agl)
                        .map(Arc::new);
                    memo.to_unicode = Some((codes, agl.clone(), m.clone()));
                    m
                }
            }
        };
        let cmap = map.and_then(|map| {
            let name = format!("{base}-UTF16");
            super::tounicode::cmap_stream(name.as_bytes(), &map, true)
        });
        let tu = match cmap {
            Some(cmap) => {
                let tu = self.alloc();
                self.stream(tu, b"", cmap.as_bytes(), true);
                format!(" /ToUnicode {tu} 0 R")
            }
            None => String::new(),
        };
        let mut d = String::new();
        let _ = write!(
            d,
            "<</Type /Font /Subtype /Type0 /BaseFont /{base} /Encoding /Identity-H /DescendantFonts [{cid} 0 R]{tu}>>"
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
        enc: Option<&(Vec<u8>, Vec<u8>)>,
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
            type1_flags(
                t1.font_name().unwrap_or(ps_name),
                t1.italic_angle().unwrap_or("0"),
                type1_force_bold(&r.program)
            ),
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
        // the program's own widths, `ROUND(w, 0.1)`, 0 for codes not used
        // (type1.c add_metrics; the supported FontMatrix scales by 1)
        let mut wa = String::from("[");
        for c in first..=last {
            let w = if used.contains(&c) {
                encoding
                    .get(c as usize)
                    .map_or(0.0, |g| type1_width(&t1, &String::from_utf8_lossy(g)))
            } else {
                0.0
            };
            let _ = write!(wa, " {}", n(w, 8));
        }
        wa.push_str(" ]");
        // the encoding and ToUnicode: an `.enc` file's, shared by all its
        // fonts (`pdf_encoding_complete`, `pdf_close_fonts`); for the
        // program's built-in encoding no `/Encoding` and a ToUnicode of
        // the font's own (type1.c's `pdf_create_ToUnicode_CMap(fullname, ..)`)
        let (enc_entry, tu) = match enc {
            Some((ident, enc_name)) => self.shared_encoding(ident, enc_name, encoding),
            None => {
                let name = format!("{base}-UTF16");
                (
                    String::new(),
                    self.type1_to_unicode(name.as_bytes(), encoding, used),
                )
            }
        };
        let tu = tu
            .map(|t| format!(" /ToUnicode {t} 0 R"))
            .unwrap_or_default();
        let d = format!(
            "<</Type /Font /Subtype /Type1 /BaseFont /{base} /FirstChar {first} /LastChar {last} /Widths {wa}{enc_entry} /FontDescriptor {fd} 0 R{tu}>>"
        );
        self.object(num, d.as_bytes());
        Ok(())
    }

    /// An `.enc` file's encoding as the fonts using it share it (written at
    /// its first font): the `/Encoding` entry (`create_encoding_resource`:
    /// `/BaseEncoding /WinAnsiEncoding` when `is_similar_charset`, and
    /// `make_encoding_differences` for the codes any of its fonts uses) and
    /// the ToUnicode CMap `enc_name-UTF16` of those codes.
    fn shared_encoding(
        &mut self,
        ident: &[u8],
        enc_name: &[u8],
        encoding: &[Vec<u8>],
    ) -> (String, Option<u32>) {
        if let Some(e) = self.encodings.get(ident) {
            return e.clone();
        }
        let mut union = BTreeSet::new();
        for (id, u) in &self.used {
            if let Some(ResKind::Type1 { enc: Some(e), .. }) =
                self.doc.fonts.resource(*id).map(|r| &r.kind)
            {
                if e.0 == ident {
                    union.extend(u.iter().copied());
                }
            }
        }
        let (base, diffs) = encoding_differences(encoding, &union);
        let entry = if !diffs.is_empty() {
            let e = self.alloc();
            let d = format!(
                "<<{} /Differences [{diffs} ]>>",
                if base {
                    " /BaseEncoding /WinAnsiEncoding"
                } else {
                    ""
                }
            );
            self.object(e, d.as_bytes());
            format!(" /Encoding {e} 0 R")
        } else if base {
            " /Encoding /WinAnsiEncoding".to_string()
        } else {
            String::new()
        };
        let mut name = enc_name.to_vec();
        name.extend_from_slice(b"-UTF16");
        let tu = self.type1_to_unicode(&name, encoding, &union);
        self.encodings.insert(ident.to_vec(), (entry.clone(), tu));
        (entry, tu)
    }

    /// `pdf_create_ToUnicode_CMap`: the ToUnicode stream of these codes'
    /// glyph names, written; none if a name has no text or none has any.
    fn type1_to_unicode(
        &mut self,
        name: &[u8],
        encoding: &[Vec<u8>],
        codes: &BTreeSet<u16>,
    ) -> Option<u32> {
        let names: BTreeMap<u16, Vec<u8>> = codes
            .iter()
            .filter_map(|&c| {
                encoding
                    .get(c as usize)
                    .filter(|n| !n.is_empty() && n.as_slice() != b".notdef")
                    .map(|n| (c, n.clone()))
            })
            .collect();
        let map = super::tounicode::encoding_to_unicode(&names, &self.agl())?;
        let cmap = super::tounicode::cmap_stream(name, &map, false)?;
        let tu = self.alloc();
        self.stream(tu, b"", cmap.as_bytes(), true);
        Some(tu)
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
        // with dvipdfmx's destination optimisation (`Goto`), only the
        // destinations a link or an outline entry names, under their new
        // names (pdfnames.c's `flat_table` with the `gotos` filter), in
        // the order of the new names (`cmp_key`)
        let mut sorted: BTreeMap<Vec<u8>, Obj> = BTreeMap::new();
        for (k, v) in &doc.dests {
            let k = if doc.check_gotos {
                match doc.gotos.get(k) {
                    Some(n) => n.clone(),
                    None => continue,
                }
            } else {
                k.clone()
            };
            sorted.entry(k).or_insert_with(|| v.clone());
        }
        if !sorted.is_empty() {
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
        for k in [&b"Type"[..], b"Pages", b"Names"] {
            cat.remove(k);
        }
        // pdf_doc_close_bookmarks replaces `/Outlines` only when there is
        // an outline: a document's own `/Outlines` stays otherwise
        if outlines.is_some() {
            cat.remove(b"Outlines");
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

    /// The outline tree, built from the `pdf:outline` entries as
    /// dvipdfmx's `spc_handler_pdfm_outline` and pdfdoc.c's
    /// `pdf_doc_bookmarks_up`/`_down`/`_add` build it, and written as
    /// `flush_bookmarks` writes it:
    ///
    /// * a level counts from the lowest level seen so far
    ///   (`lowest_level`, from 255);
    /// * a jump of more than one level down fills each level skipped with
    ///   dvipdfmx's `<No Title>` entry (red, italic, a JavaScript alert);
    /// * `/Count` of an entry with children is the number of entries
    ///   visible below it when it is open (its children, and theirs
    ///   through open children), negated when it is closed;
    /// * the root has `/First`, `/Last` and `/Count`, and no `/Type`.
    ///
    /// An entry's own keys stay unless one of these replaces them.
    fn outlines(&mut self) -> u32 {
        let items = outline_items(&self.doc.outlines);
        // the items with a dictionary in the chain from `first`
        let chain = |first: Option<usize>| -> Vec<usize> {
            let mut v = Vec::new();
            let mut it = first;
            while let Some(i) = it {
                if items[i].dict.is_none() {
                    break;
                }
                v.push(i);
                it = items[i].next;
            }
            v
        };
        // flush_bookmarks's count for each item's children: the entries
        // visible below it when it is open. Children come after their
        // parent, so from the last item back.
        let mut below = vec![0i64; items.len()];
        for i in (0..items.len()).rev() {
            below[i] = chain(items[i].first)
                .iter()
                .map(|&c| 1 + if items[c].is_open != 0 { below[c] } else { 0 })
                .sum();
        }
        let root = self.alloc();
        let top = chain(Some(0));
        // object numbers, a parent's before its children's; each entry
        // with its parent's object and its previous sibling
        let mut nums: Vec<u32> = vec![0; items.len()];
        let mut order: Vec<(usize, u32, Option<usize>)> = Vec::new();
        let mut todo = vec![(top.clone(), root)];
        while let Some((sibs, par)) = todo.pop() {
            for &i in &sibs {
                nums[i] = self.alloc();
            }
            for (k, &i) in sibs.iter().enumerate() {
                order.push((i, par, k.checked_sub(1).map(|p| sibs[p])));
                let kids = chain(items[i].first);
                if !kids.is_empty() {
                    todo.push((kids, nums[i]));
                }
            }
        }
        for (i, par, prev) in order {
            let mut d = items[i].dict.clone().unwrap_or_default();
            let kids = chain(items[i].first);
            if let (Some(&f), Some(&l)) = (kids.first(), kids.last()) {
                d.set(b"First", Obj::Ref(nums[f], 0));
                d.set(b"Last", Obj::Ref(nums[l], 0));
                let c = if items[i].is_open != 0 {
                    below[i]
                } else {
                    -below[i]
                };
                d.set(b"Count", Obj::Num(c.to_string().into_bytes()));
            }
            d.set(b"Parent", Obj::Ref(par, 0));
            if let Some(p) = prev {
                d.set(b"Prev", Obj::Ref(nums[p], 0));
            }
            if let Some(n) = items[i].next.filter(|&n| items[n].dict.is_some()) {
                d.set(b"Next", Obj::Ref(nums[n], 0));
            }
            let body = self.ser(&Obj::Dict(d));
            self.object(nums[i], &body);
        }
        let total: i64 = top
            .iter()
            .map(|&k| 1 + if items[k].is_open != 0 { below[k] } else { 0 })
            .sum();
        let mut body = b"<<".to_vec();
        if let (Some(&f), Some(&l)) = (top.first(), top.last()) {
            let _ = write!(
                Bytes(&mut body),
                "/First {} 0 R /Last {} 0 R /Count {total}",
                nums[f],
                nums[l]
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
        let shift = header(self.doc.version).len();
        let mut out = std::mem::take(&mut self.out);
        let xref = out.len();
        let _ = write!(
            Bytes(&mut out),
            "xref\n0 {}\n0000000000 65535 f \n",
            self.offsets.len() + 1
        );
        for o in &self.offsets {
            let _ = writeln!(Bytes(&mut out), "{:010} 00000 n ", o.unwrap_or(shift));
        }
        let id = self.id.finish(&out);
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

/// An outline item as pdfdoc.c keeps it (`pdf_olitem`): item 0 is the
/// first item `pdf_doc_init_bookmarks` makes.
struct OlItem {
    dict: Option<Dict>,
    first: Option<usize>,
    next: Option<usize>,
    parent: Option<usize>,
    /// 1 open, 0 closed, -1 the place for a next sibling (which counts as
    /// open in `flush_bookmarks`'s `if (item->is_open)` if a `<No Title>`
    /// entry fills it).
    is_open: i32,
}

/// dvipdfmx's entry for a level an outline skipped
/// (`pdf_doc_bookmarks_down`).
fn no_title_entry() -> Dict {
    let num = |t: &[u8]| Obj::Num(t.to_vec());
    let js = b"app.alert(\"The author of this document made this bookmark item empty!\", 3, 0)";
    Dict(vec![
        (b"Title".to_vec(), Obj::Str(b"<No Title>".to_vec())),
        (
            b"C".to_vec(),
            Obj::Array(vec![num(b"1"), num(b"0"), num(b"0")]),
        ),
        (b"F".to_vec(), num(b"1")),
        (
            b"A".to_vec(),
            Obj::Dict(Dict(vec![
                (b"S".to_vec(), Obj::Name(b"JavaScript".to_vec())),
                (b"JS".to_vec(), Obj::Str(js.to_vec())),
            ])),
        ),
    ])
}

/// The outline items `pdf:outline` entries make, as
/// `spc_handler_pdfm_outline` moves through levels
/// (`pdf_doc_bookmarks_up`, `_down`) and `pdf_doc_bookmarks_add` adds an
/// entry, with TeX Live's `-O 0` (an entry without `[]` is closed).
fn outline_items(entries: &[Outline]) -> Vec<OlItem> {
    fn new_item(items: &mut Vec<OlItem>, parent: Option<usize>, is_open: i32) -> usize {
        items.push(OlItem {
            dict: None,
            first: None,
            next: None,
            parent,
            is_open,
        });
        items.len() - 1
    }
    let mut items = Vec::new();
    let mut current = new_item(&mut items, None, 1);
    let mut depth: i64 = 1;
    let mut lowest: i64 = 255;
    for o in entries {
        lowest = lowest.min(o.level as i64);
        let level = o.level as i64 + 1 - lowest;
        while depth > level {
            // up: to the parent's next sibling
            let Some(parent) = items[current].parent else {
                break;
            };
            current = match items[parent].next {
                Some(n) => n,
                None => {
                    let gp = items[parent].parent;
                    let n = new_item(&mut items, gp, 0);
                    items[parent].next = Some(n);
                    n
                }
            };
            depth -= 1;
        }
        while depth < level {
            // down: a first child, the level filled if it is empty
            if items[current].dict.is_none() {
                items[current].dict = Some(no_title_entry());
            }
            let f = new_item(&mut items, Some(current), 0);
            items[current].first = Some(f);
            current = f;
            depth += 1;
        }
        let mut item = current;
        if items[item].dict.is_some() {
            item = match items[item].next {
                Some(n) => n,
                None => {
                    let p = items[item].parent;
                    let n = new_item(&mut items, p, -1);
                    items[item].next = Some(n);
                    n
                }
            };
        }
        let mut d = o.dict.clone();
        if let Some(g) = &o.goto {
            g.apply(&mut d);
        }
        items[item].dict = Some(d);
        items[item].first = None;
        items[item].is_open = i32::from(o.open);
        let p = items[item].parent;
        let n = new_item(&mut items, p, -1);
        items[item].next = Some(n);
        current = item;
    }
    items
}

/// The file's first two lines: the version, and a comment of non-ASCII
/// characters (in UTF-8).
fn header((maj, min): (u32, u32)) -> Vec<u8> {
    format!("%PDF-{maj}.{min}\n%\u{00d0}\u{00d4}\u{00c5}\u{00d8}\n").into_bytes()
}

/// The SHA-256 of the file up to its trailer, which the trailer's `/ID`
/// is made of. It is taken on a thread of its own while the objects are
/// written, so that the end of the run has only the xref table left to
/// hash, not the whole file (12 MB, 9 ms, for a 109-page preview, whose
/// streams are stored); where no thread can be started, in line as the
/// objects are written. Either way it hashes the same bytes in the same
/// order, so the `/ID` is the same.
struct IdHash {
    /// How much of the file has been hashed or sent to be.
    done: usize,
    how: IdHow,
}

enum IdHow {
    Thread(
        std::sync::mpsc::Sender<Vec<u8>>,
        std::thread::JoinHandle<[u8; 32]>,
    ),
    Inline(Sha256),
    /// The thread is gone (it cannot be, short of a panic): hash the whole
    /// file at the end.
    Lost,
}

impl IdHash {
    /// The new bytes sent at a time.
    const CHUNK: usize = 1 << 18;

    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let thread = std::thread::Builder::new()
            .name("pdf-id".into())
            .spawn(move || {
                let mut h = Sha256::new();
                for chunk in rx {
                    h.update(&chunk);
                }
                h.finish()
            });
        let how = match thread {
            Ok(j) => IdHow::Thread(tx, j),
            Err(_) => IdHow::Inline(Sha256::new()),
        };
        IdHash { done: 0, how }
    }

    /// Hash `out[done..]`, `out` being the file so far, once that is at
    /// least `min` bytes.
    fn feed(&mut self, out: &[u8], min: usize) {
        let new = &out[self.done..];
        if new.is_empty() || new.len() < min {
            return;
        }
        match &mut self.how {
            IdHow::Thread(tx, _) => {
                if tx.send(new.to_vec()).is_err() {
                    self.how = IdHow::Lost;
                }
            }
            IdHow::Inline(h) => h.update(new),
            IdHow::Lost => {}
        }
        self.done = out.len();
    }

    /// The hash of `out`, the whole file up to its trailer.
    fn finish(&mut self, out: &[u8]) -> [u8; 32] {
        self.feed(out, 0);
        match std::mem::replace(&mut self.how, IdHow::Lost) {
            IdHow::Thread(tx, j) => {
                drop(tx);
                j.join().unwrap_or_else(|_| sha256(out))
            }
            IdHow::Inline(h) => h.finish(),
            IdHow::Lost => sha256(out),
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

/// The program of a native font for its font descriptor: the key, the
/// stream's subtype entry, the bytes, and whether they are a subset (which
/// the font's name then says with its tag).
fn font_program(
    font: &flashtex_pdf::truetype::TrueTypeFont,
    whole: &[u8],
    cff: bool,
    used: &BTreeSet<u16>,
) -> Result<Program, String> {
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

/// What the PDF writer last made for one face (`super::fonts::FaceData`),
/// with the input it made it from: each a function of the face and that
/// input, used again when a later run asks with the same input (a
/// keystroke seldom changes which glyphs a font draws).
#[derive(Default)]
pub struct FaceMemo {
    /// [`cid_map`]'s, for these glyphs.
    cids: Option<(BTreeSet<u16>, Option<CidMap>)>,
    /// [`font_program`]'s, for this outline kind and these glyphs.
    program: Option<ProgramMemo>,
    /// `otf_to_unicode`'s, for these codes and this glyph list.
    to_unicode: Option<TextMemo>,
}

/// A glyph id's CID.
type CidMap = BTreeMap<u16, u16>;

/// [`FaceMemo::program`]: the outline kind and glyphs, and the program.
type ProgramMemo = (bool, BTreeSet<u16>, Arc<Result<Program, String>>);

/// [`FaceMemo::to_unicode`]: the codes and glyph list, and the text.
type TextMemo = (BTreeSet<u16>, Arc<Agl>, Option<Arc<TextMap>>);

/// A code's text.
type TextMap = BTreeMap<u16, Vec<char>>;

/// What [`font_program`] makes: the descriptor's key and stream entries,
/// the program, and whether it is a subset.
type Program = (&'static str, &'static str, Vec<u8>, bool);

/// For a CID-keyed CFF face: each used glyph id's CID (the font's charset,
/// through which a CIDFontType0 font selects glyphs). `None` for any other
/// face.
fn cid_map(
    font: &flashtex_pdf::truetype::TrueTypeFont,
    used: &BTreeSet<u16>,
) -> Option<BTreeMap<u16, u16>> {
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

/// What the font descriptor takes from a face's `OS/2`, `post` and
/// `head`, read as xdvipdfmx's `tt_read_os2__table`, `tt_read_post_table`
/// and `tt_read_head_table` read them (tt_table.c).
struct FaceInfo {
    /// The `OS/2` table's fields; a face without one has
    /// `tt_read_os2__table`'s stand-ins (version 0, weight 400, typo
    /// ascender 880 and descender -120, the rest 0).
    os2_version: u16,
    avg_width: i16,
    weight: u16,
    family_class: i16,
    panose: [u8; 10],
    fs_selection: u16,
    /// 0 when the table is shorter than 78 bytes (C leaves them unset).
    typo_ascender: i16,
    typo_descender: i16,
    /// Read for an `OS/2` of version 2 and later only.
    x_height: i16,
    cap_height: i16,
    /// `head`: units per em and the bounding box.
    upm: u16,
    bbox: [i16; 4],
    /// `post`: `italicAngle` and `isFixedPitch`.
    italic_angle: f64,
    fixed_pitch: bool,
}

impl Default for FaceInfo {
    fn default() -> Self {
        FaceInfo {
            os2_version: 0,
            avg_width: 0,
            weight: 400,
            family_class: 0,
            panose: [0; 10],
            fs_selection: 0,
            typo_ascender: 880,
            typo_descender: -120,
            x_height: 0,
            cap_height: 0,
            upm: 1000,
            bbox: [0; 4],
            italic_angle: 0.0,
            fixed_pitch: false,
        }
    }
}

impl FaceInfo {
    fn read(data: &[u8], index: u32) -> FaceInfo {
        use super::tounicode::table;
        let rd16 = |t: &[u8], o: usize| t.get(o..o + 2).map(|s| u16::from_be_bytes([s[0], s[1]]));
        let i16_at = |t: &[u8], o: usize| rd16(t, o).unwrap_or(0) as i16;
        let mut f = FaceInfo::default();
        if let Some(os2) = table(data, index, b"OS/2") {
            f.os2_version = rd16(os2, 0).unwrap_or(0);
            f.avg_width = i16_at(os2, 2);
            f.weight = rd16(os2, 4).unwrap_or(0);
            f.family_class = i16_at(os2, 30);
            for (k, p) in f.panose.iter_mut().enumerate() {
                *p = os2.get(32 + k).copied().unwrap_or(0);
            }
            f.fs_selection = rd16(os2, 62).unwrap_or(0);
            let long = os2.len() >= 78;
            f.typo_ascender = if long { i16_at(os2, 68) } else { 0 };
            f.typo_descender = if long { i16_at(os2, 70) } else { 0 };
            if long && f.os2_version > 1 {
                f.x_height = i16_at(os2, 86);
                f.cap_height = i16_at(os2, 88);
            }
        }
        if let Some(head) = table(data, index, b"head") {
            f.upm = rd16(head, 18).unwrap_or(1000);
            for (k, b) in f.bbox.iter_mut().enumerate() {
                *b = i16_at(head, 36 + 2 * k);
            }
        }
        if let Some(post) = table(data, index, b"post") {
            let fixed = post
                .get(4..8)
                .map_or(0, |s| i32::from_be_bytes([s[0], s[1], s[2], s[3]]));
            f.italic_angle = fixed as f64 / 65536.0;
            f.fixed_pitch = post.get(12..16).is_some_and(|s| s.iter().any(|&b| b != 0));
        }
        f
    }

    /// `PDFUNIT`: font units to thousandths of an em, rounded to an integer
    /// (`ROUND(1000.0*v/unitsPerEm, 1)`).
    fn pdfunit(&self, v: f64) -> f64 {
        (1000.0 * v / self.upm as f64 + 0.5).floor()
    }

    /// The descriptor's `/Flags` as `tt_get_fontdesc` sets them: Symbolic,
    /// Italic and ForceBold from `fsSelection`, Serif unless the
    /// `sFamilyClass` class is 8 (sans serif), Script for class 10,
    /// FixedPitch from `post`.
    fn flags(&self) -> u32 {
        let mut f = 4;
        if self.fs_selection & 1 != 0 {
            f |= 64;
        }
        if self.fs_selection & (1 << 5) != 0 {
            f |= 1 << 18;
        }
        let class = (self.family_class >> 8) & 0xff;
        if class != 8 {
            f |= 2;
        }
        if class == 10 {
            f |= 8;
        }
        if self.fixed_pitch {
            f |= 1;
        }
        f
    }

    /// `/StemV` as `tt_get_fontdesc` estimates it from the weight class,
    /// `(w/65.)*(w/65.)+50` stored in an `int` (truncated).
    fn stem_v(&self) -> f64 {
        let w = self.weight as f64 / 65.0;
        (w * w + 50.0) as i32 as f64
    }

    /// The `FontDescriptor`'s entries after `/FontName`, as
    /// `tt_get_fontdesc` (tt_aux.c) writes them for a CID-keyed font:
    /// `/Ascent`, `/Descent`, `/StemV` (`stem_v`, or a CFF's own `StdVW`),
    /// `/CapHeight` (and `/XHeight`) from an `OS/2` of version 2 exactly,
    /// else the typo ascender, `/AvgWidth` when not 0, `/FontBBox` from
    /// `head`, `/ItalicAngle` from `post`, `/Flags`, `/Style` with the
    /// Panose bytes. Numbers to eight decimals, as dvipdfmx's `p_dtoa`.
    fn descriptor_entries(&self, stem_v: Option<f64>) -> String {
        let u = |v: i16| fmt_num(self.pdfunit(v as f64), 8);
        let mut d = String::new();
        let _ = write!(
            d,
            " /Ascent {} /Descent {} /StemV {}",
            u(self.typo_ascender),
            u(self.typo_descender),
            fmt_num(stem_v.unwrap_or_else(|| self.stem_v()), 8)
        );
        if self.os2_version == 2 {
            let _ = write!(
                d,
                " /CapHeight {} /XHeight {}",
                u(self.cap_height),
                u(self.x_height)
            );
        } else {
            let _ = write!(d, " /CapHeight {}", u(self.typo_ascender));
        }
        if self.avg_width != 0 {
            let _ = write!(d, " /AvgWidth {}", u(self.avg_width));
        }
        let [x0, y0, x1, y1] = self.bbox;
        let _ = write!(
            d,
            " /FontBBox [{} {} {} {}] /ItalicAngle {} /Flags {}",
            u(x0),
            u(y0),
            u(x1),
            u(y1),
            fmt_num(self.italic_angle, 8),
            self.flags()
        );
        let mut panose = Vec::with_capacity(12);
        panose.extend_from_slice(&self.family_class.to_be_bytes());
        panose.extend_from_slice(&self.panose);
        let mut s = Vec::new();
        super::pdfobj::write_string(&panose, &mut s);
        let _ = write!(d, " /Style <</Panose {}>>", String::from_utf8_lossy(&s));
        d
    }
}

/// Whether a PFB program's Private dict says `/ForceBold true` (its
/// eexec-encrypted part, decrypted).
fn type1_force_bold(pfb: &[u8]) -> bool {
    let mut at = 0;
    while pfb.get(at) == Some(&0x80) {
        let Some(len) = pfb
            .get(at + 2..at + 6)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        else {
            return false;
        };
        let body = pfb.get(at + 6..at + 6 + len).unwrap_or(&[]);
        if pfb.get(at + 1) == Some(&2) {
            let plain = flashtex_pdf::type1::decrypt(body, 55665, 4);
            return plain.windows(15).any(|w| w == b"/ForceBold true");
        }
        at += 6 + len;
    }
    false
}

/// Glyph `name`'s `/Widths` entry as xdvipdfmx's `add_metrics` (type1.c)
/// writes it: the charstring's width, `ROUND(w, 0.1)`; 0 for a glyph the
/// program lacks.
fn type1_width(t1: &flashtex_pdf::type1::Type1Font, name: &str) -> f64 {
    t1.advance_width(name).map_or(0.0, |w| {
        ((w.num as f64 / w.den as f64) / 0.1 + 0.5).floor() * 0.1
    })
}

/// A Type 1 font's `/Flags` as xdvipdfmx's `get_font_attr` (type1.c) sets
/// them from its program's `FontName` and `ItalicAngle`: Symbolic, Serif
/// unless the name contains `Sans`, SmallCap if it contains `Caps`, Italic
/// for an angle other than 0, ForceBold from the Private dict. (Its
/// FixedPitch also comes from the Private dict, where a Type 1 font's
/// `isFixedPitch` is not.)
fn type1_flags(font_name: &str, italic_angle: &str, force_bold: bool) -> u32 {
    let mut f = 4;
    if force_bold {
        f |= 1 << 18;
    }
    if !font_name.contains("Sans") {
        f |= 2;
    }
    if font_name.contains("Caps") {
        f |= 1 << 17;
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
        push_num(c, *x, 5);
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
        push_num(c, *v, if i < 4 { 8 } else { 6 });
    }
}

/// Path segments, mapped by `m` when given.
fn write_segs(c: &mut Vec<u8>, segs: &[Seg], m: Option<&[f64; 6]>) {
    // `x y ` (six decimals), mapped
    let pt = |c: &mut Vec<u8>, x: f64, y: f64| {
        let (x, y) = match m {
            Some(m) => (x * m[0] + y * m[2] + m[4], x * m[1] + y * m[3] + m[5]),
            None => (x, y),
        };
        push_num(c, x, 6);
        c.push(b' ');
        push_num(c, y, 6);
        c.push(b' ');
    };
    for s in segs {
        match *s {
            Seg::Move(x, y) => {
                pt(c, x, y);
                c.extend_from_slice(b"m ");
            }
            Seg::Line(x, y) => {
                pt(c, x, y);
                c.extend_from_slice(b"l ");
            }
            Seg::Curve(x1, y1, x2, y2, x3, y3) => {
                pt(c, x1, y1);
                pt(c, x2, y2);
                pt(c, x3, y3);
                c.extend_from_slice(b"c ");
            }
            Seg::Close => c.extend_from_slice(b"h "),
        }
    }
}

/// `v1 v2 ...`, each with at most `d` decimals.
fn push_nums(c: &mut Vec<u8>, v: &[f64], d: usize) {
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            c.push(b' ');
        }
        push_num(c, *x, d);
    }
}

/// `w w c J j j m M [dash] phase d` (six decimals), no separator after.
fn write_stroke(c: &mut Vec<u8>, s: &Stroke) {
    push_num(c, s.width, 6);
    c.extend_from_slice(b" w ");
    push_uint(c, s.cap as u64);
    c.extend_from_slice(b" J ");
    push_uint(c, s.join as u64);
    c.extend_from_slice(b" j ");
    push_num(c, s.miter, 6);
    c.extend_from_slice(b" M [");
    push_nums(c, &s.dash, 6);
    c.extend_from_slice(b"] ");
    push_num(c, s.phase, 6);
    c.extend_from_slice(b" d");
}

/// `<XXXX>Tj` and a newline: `code` in `width` upper-case hex digits
/// (`format!("<{code:0width$X}>Tj")` for a code that fits).
fn push_hex_tj(c: &mut Vec<u8>, code: u32, width: usize) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    c.push(b'<');
    let digits = width.max((32 - code.leading_zeros() as usize).div_ceil(4));
    for i in (0..digits).rev() {
        c.push(HEX[((code >> (4 * i)) & 0xF) as usize]);
    }
    c.extend_from_slice(b">Tj\n");
}

/// An `.enc` encoding's base and `Differences` entries for the codes its
/// fonts use: `is_similar_charset` against WinAnsi (64 codes alike, a
/// `.notdef` counting as alike), then `make_encoding_differences`
/// (`pdfencoding.c`): the used codes whose glyph differs from the base's,
/// a run's first code before its names; `.notdef` is no glyph.
fn encoding_differences(encoding: &[Vec<u8>], used: &BTreeSet<u16>) -> (bool, String) {
    let glyph = |c: usize| {
        encoding
            .get(c)
            .filter(|n| !n.is_empty() && n.as_slice() != b".notdef")
    };
    let same = (0..256)
        .filter(|&c| glyph(c).is_none_or(|n| n.as_slice() == WIN_ANSI[c].as_bytes()))
        .count();
    let base = same >= 64;
    let mut diffs = String::new();
    let mut skipping = true;
    for (c, win) in WIN_ANSI.iter().enumerate() {
        match glyph(c).filter(|_| used.contains(&(c as u16))) {
            Some(n) if !(base && n.as_slice() == win.as_bytes()) => {
                if skipping {
                    let _ = write!(diffs, " {c}");
                }
                let mut nb = Vec::new();
                write_name(n, &mut nb);
                diffs.push(' ');
                diffs.push_str(&String::from_utf8_lossy(&nb));
                skipping = false;
            }
            _ => skipping = true,
        }
    }
    (base, diffs)
}

/// `WinAnsiEncoding[]` (`pdfencoding.c`): the base xdvipdfmx gives an
/// `.enc` encoding that agrees with it in 64 codes (`is_similar_charset`).
const WIN_ANSI: [&str; 256] = [
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    ".notdef",
    "space",
    "exclam",
    "quotedbl",
    "numbersign",
    "dollar",
    "percent",
    "ampersand",
    "quotesingle",
    "parenleft",
    "parenright",
    "asterisk",
    "plus",
    "comma",
    "hyphen",
    "period",
    "slash",
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "colon",
    "semicolon",
    "less",
    "equal",
    "greater",
    "question",
    "at",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "bracketleft",
    "backslash",
    "bracketright",
    "asciicircum",
    "underscore",
    "grave",
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "braceleft",
    "bar",
    "braceright",
    "asciitilde",
    "bullet",
    "Euro",
    "bullet",
    "quotesinglbase",
    "florin",
    "quotedblbase",
    "ellipsis",
    "dagger",
    "daggerdbl",
    "circumflex",
    "perthousand",
    "Scaron",
    "guilsinglleft",
    "OE",
    "bullet",
    "Zcaron",
    "bullet",
    "bullet",
    "quoteleft",
    "quoteright",
    "quotedblleft",
    "quotedblright",
    "bullet",
    "endash",
    "emdash",
    "tilde",
    "trademark",
    "scaron",
    "guilsinglright",
    "oe",
    "bullet",
    "zcaron",
    "Ydieresis",
    "space",
    "exclamdown",
    "cent",
    "sterling",
    "currency",
    "yen",
    "brokenbar",
    "section",
    "dieresis",
    "copyright",
    "ordfeminine",
    "guillemotleft",
    "logicalnot",
    "hyphen",
    "registered",
    "macron",
    "degree",
    "plusminus",
    "twosuperior",
    "threesuperior",
    "acute",
    "mu",
    "paragraph",
    "periodcentered",
    "cedilla",
    "onesuperior",
    "ordmasculine",
    "guillemotright",
    "onequarter",
    "onehalf",
    "threequarters",
    "questiondown",
    "Agrave",
    "Aacute",
    "Acircumflex",
    "Atilde",
    "Adieresis",
    "Aring",
    "AE",
    "Ccedilla",
    "Egrave",
    "Eacute",
    "Ecircumflex",
    "Edieresis",
    "Igrave",
    "Iacute",
    "Icircumflex",
    "Idieresis",
    "Eth",
    "Ntilde",
    "Ograve",
    "Oacute",
    "Ocircumflex",
    "Otilde",
    "Odieresis",
    "multiply",
    "Oslash",
    "Ugrave",
    "Uacute",
    "Ucircumflex",
    "Udieresis",
    "Yacute",
    "Thorn",
    "germandbls",
    "agrave",
    "aacute",
    "acircumflex",
    "atilde",
    "adieresis",
    "aring",
    "ae",
    "ccedilla",
    "egrave",
    "eacute",
    "ecircumflex",
    "edieresis",
    "igrave",
    "iacute",
    "icircumflex",
    "idieresis",
    "eth",
    "ntilde",
    "ograve",
    "oacute",
    "ocircumflex",
    "otilde",
    "odieresis",
    "divide",
    "oslash",
    "ugrave",
    "uacute",
    "ucircumflex",
    "udieresis",
    "yacute",
    "thorn",
    "ydieresis",
];

/// Six capital letters naming a subset, from the font and its glyphs.
fn subset_tag(name: &str, used: &BTreeSet<u16>) -> String {
    let mut h = Sha256::new();
    h.update(name.as_bytes());
    for g in used {
        h.update(&g.to_le_bytes());
    }
    let d = h.finish();
    d[..6].iter().map(|b| (b'A' + b % 26) as char).collect()
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
    /// What `tRNS` makes of the image (pngimage.c `check_transparency`).
    trans: PngTrans,
}

/// pngimage.c's transparency of an 8-bit-or-less PNG without an alpha
/// channel (`check_transparency`, `create_ckey_mask`, `create_soft_mask`).
#[derive(Clone, Debug, PartialEq)]
enum PngTrans {
    None,
    /// `PDF_TRANS_TYPE_BINARY`: `/Mask` with these numbers (a grey or RGB
    /// image's `tRNS` colour, each twice; a palette's fully transparent
    /// indices, each twice).
    Key(Vec<u32>),
    /// `PDF_TRANS_TYPE_ALPHA`: a palette `tRNS` with partial alpha; an
    /// 8-bit `/SMask` from these alphas (index `i` < their count, else 255).
    Alpha(Vec<u8>),
}

/// pngimage.c `create_soft_mask`'s samples: index `i` of the image is read
/// at bit `bpc * i` of the decoded rows *as if they were not padded* (the
/// C's `image_data_ptr[bpc * i / 8] >> (shift - bpc * i % 8)`), so with
/// rows that do not fill their last byte the mask drifts from the second
/// row on (measured: a 7-pixel-wide 4-bit palette PNG's pixel 7 takes
/// index 0 of row 0's padding nibble on TeX Live 2026's xdvipdfmx).
fn png_soft_mask(rows: &[u8], bpc: u8, width: u32, height: u32, trans: &[u8]) -> Vec<u8> {
    let bpc = bpc as usize;
    let mask = 0xffu32 >> (8 - bpc);
    let shift = 8 - bpc;
    let n = width as usize * height as usize;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let byte = rows.get(bpc * i / 8).copied().unwrap_or(0) as u32;
        let idx = ((byte >> (shift - bpc * i % 8)) & mask) as usize;
        out.push(if idx < trans.len() { trans[idx] } else { 0xff });
    }
    out
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
    let mut trns: Option<&[u8]> = None;
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
            b"tRNS" => trns = Some(body),
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
    // A 16-bit grey or RGB PNG with a colour key passes through too (PDF
    // takes 16-bit samples with the PNG predictor): the other route has no
    // colour keys.
    let key16 = bits == 16 && matches!(ctype, 0 | 2) && trns.is_some();
    if interlace != 0 || (bits > 8 && !key16) || !matches!(ctype, 0 | 2 | 3) {
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
    // libpng keeps a tRNS of the right length only (png_handle_tRNS: grey
    // 2 bytes, RGB 6, a palette's 1 to its entries); a 16-bit sample in
    // the chunk is the value as written.
    let be16 = |b: &[u8], i: usize| u16::from_be_bytes([b[i], b[i + 1]]) as u32;
    let trans = match (ctype, trns) {
        (0, Some(t)) if t.len() == 2 => PngTrans::Key(vec![be16(t, 0); 2]),
        (2, Some(t)) if t.len() == 6 => {
            let (r, g, b) = (be16(t, 0), be16(t, 2), be16(t, 4));
            PngTrans::Key(vec![r, r, g, g, b, b])
        }
        (3, Some(t)) if !t.is_empty() && t.len() <= palette.as_ref().map_or(0, |p| p.len() / 3) => {
            if t.iter().all(|&a| a == 0x00 || a == 0xff) {
                let mut k = Vec::new();
                for (i, &a) in t.iter().enumerate() {
                    if a == 0 {
                        k.extend([i as u32, i as u32]);
                    }
                }
                PngTrans::Key(k)
            } else {
                PngTrans::Alpha(t.to_vec())
            }
        }
        _ => PngTrans::None,
    };
    Some(PngPass {
        dict,
        color: col,
        n_base: if ctype == 0 { 1 } else { 3 },
        palette,
        idat,
        trans,
    })
}

/// `data` with its `tRNS` chunks left out (a PNG still).
fn png_without_trns(data: &[u8]) -> Vec<u8> {
    let mut out = data[..8.min(data.len())].to_vec();
    let mut pos = 8;
    while pos + 8 <= data.len() {
        let len =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let end = (pos + 12 + len).min(data.len());
        if &data[pos + 4..pos + 8] != b"tRNS" {
            out.extend_from_slice(&data[pos..end]);
        }
        pos = end;
    }
    out
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

/// What this writer draws each RULE of `p` with (spec §4.4
/// `RULE_GEOMETRY`), in item order: the numbers [`Writer::content`] writes
/// (five decimals), read back, with no CTM translation. The preview then
/// strokes a fraction bar on the PDF's own coordinates (from sp, a 0.68 bp
/// rule's end column differed by one coverage level at 4 px/pt).
pub fn rule_geometry(p: &Page) -> Vec<[f64; 7]> {
    let h = p.pdf_box[3];
    let r = |v: f64| super::num::round_num(v, 5);
    p.items
        .iter()
        .filter_map(|it| {
            let Item::Rule {
                kind,
                x,
                y,
                w,
                h: rh,
            } = it
            else {
                return None;
            };
            let (l, t) = (*x as f64 / K, h - *y as f64 / K);
            let (wb, hb) = (*w as f64 / K, *rh as f64 / K);
            let b = t - hb;
            Some(match kind {
                RuleKind::Fill => [0.0, 0.0, r(l), r(b), r(wb), r(hb), 0.0],
                RuleKind::StrokeH => {
                    let ym = b + hb / 2.0;
                    [0.0, 0.0, r(l), r(ym), r(l + wb), r(ym), r(hb)]
                }
                RuleKind::StrokeV => {
                    let xm = l + wb / 2.0;
                    [0.0, 0.0, r(xm), r(b), r(xm), r(t), r(wb)]
                }
            })
        })
        .collect()
}

/// The colour space this writer gives a raster image's samples, for the
/// `IMAGE` resource's `color_space` (spec §5.2), so that the preview draws
/// the samples as the PDF does: `{"type": "icc"}` (the profile the file
/// embeds: a PNG's `iCCP`, a JPEG's `ICC_PROFILE`), or `{"type": "calgray"
/// | "calrgb", "gamma", "white_point", "matrix"}` (a PNG's `sRGB`, or its
/// `gAMA`/`cHRM`), read from the very array [`cal_space`] writes, with
/// `intent` when the PDF names one. None: the Device space of the samples
/// (pdfTeX's rule, and this writer's for every other image).
pub fn image_color_space(kind: Kind, data: &[u8]) -> Option<flashtex_display_list::json::Json> {
    use flashtex_display_list::json::Json;
    let icc = || Some(Json::Obj(vec![("type".into(), Json::Str("icc".into()))]));
    match kind {
        Kind::Png => {
            let p = png_passthrough(data)?;
            if p.color.icc.is_some() {
                return icc();
            }
            let space = cal_space(&p.color, p.n_base)?;
            // `[/CalGray <</Gamma g /WhitePoint [x y z]>>]` or `[/CalRGB
            // <</Gamma [g g g] /Matrix [9] /WhitePoint [x y z]>>]`
            let mut key = "";
            let mut nums: Vec<(&str, f64)> = vec![];
            for tok in space.split(|c: char| c.is_whitespace() || "[]<>".contains(c)) {
                if let Some(k) = tok.strip_prefix('/') {
                    key = k;
                } else if let Ok(v) = tok.parse::<f64>() {
                    nums.push((key, v));
                }
            }
            let of = |k: &str| -> Vec<Json> {
                nums.iter()
                    .filter(|e| e.0 == k)
                    .map(|e| Json::Num(e.1))
                    .collect()
            };
            let rgb = space.starts_with("[/CalRGB");
            let mut kv = vec![
                (
                    "type".into(),
                    Json::Str(if rgb { "calrgb" } else { "calgray" }.into()),
                ),
                ("gamma".into(), of("Gamma").into_iter().next()?),
                ("white_point".into(), Json::Arr(of("WhitePoint"))),
            ];
            if rgb {
                kv.push(("matrix".into(), Json::Arr(of("Matrix"))));
            }
            if let Some(i) = p.color.srgb_intent {
                let name = [
                    "Perceptual",
                    "RelativeColorimetric",
                    "Saturation",
                    "AbsoluteColorimetric",
                ]
                .get(i as usize)
                .copied()
                .unwrap_or("Perceptual");
                kv.push(("intent".into(), Json::Str(name.into())));
            }
            Some(Json::Obj(kv))
        }
        Kind::Jpeg => {
            jpeg_icc(data)?;
            icc()
        }
        _ => None,
    }
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

    /// xdvipdfmx 20260317's `/Encoding` of `lm-ec.enc` (TeX Live 2026
    /// `xetex`) when ec-lmr10 draws `abc xyz` and ec-lmbx10 `abcdk` and
    /// codes 128..139: one WinAnsi-based encoding for both fonts,
    /// Differences only for the union's codes WinAnsi has otherwise.
    #[test]
    fn enc_file_encoding_differences() {
        let mut ec: Vec<Vec<u8>> = WIN_ANSI.iter().map(|n| n.as_bytes().to_vec()).collect();
        let names = [
            "Abreve", "Aogonek", "Cacute", "Ccaron", "Dcaron", "Ecaron", "Eogonek", "Gbreve",
            "Lacute", "Lcaron", "Lslash", "Nacute",
        ];
        for (i, n) in names.iter().enumerate() {
            ec[128 + i] = n.as_bytes().to_vec();
        }
        ec[11] = b"ff".to_vec(); // unused: not written
        let used: BTreeSet<u16> = b"abcdkxyz"
            .iter()
            .map(|&c| c as u16)
            .chain(128..140)
            .collect();
        assert_eq!(
            encoding_differences(&ec, &used),
            (
                true,
                " 128 /Abreve /Aogonek /Cacute /Ccaron /Dcaron /Ecaron /Eogonek /Gbreve /Lacute /Lcaron /Lslash /Nacute".to_string()
            )
        );
        // fewer than 64 codes like WinAnsi: no base, every used glyph
        let other: Vec<Vec<u8>> = (0..256).map(|c| format!("g{c}").into_bytes()).collect();
        assert_eq!(
            encoding_differences(&other, &[3u16, 4, 9].into_iter().collect()),
            (false, " 3 /g3 /g4 9 /g9".to_string())
        );
    }

    /// A page drawing glyph `gid` of `path` (face 0) at 10 pt, written.
    fn one_glyph_pdf(path: &str, gid: u16) -> Vec<u8> {
        glyphs_pdf(path, &[gid], None)
    }

    /// A page drawing glyphs `gids` of `path` (face 0) at 10 pt, written
    /// with the resident engine's font cache `cache` (or none).
    fn glyphs_pdf(
        path: &str,
        gids: &[u16],
        cache: Option<std::sync::Arc<super::super::fonts::FontCache>>,
    ) -> Vec<u8> {
        face_glyphs_pdf(path, 0, gids, cache)
    }
    fn face_glyphs_pdf(
        path: &str,
        index: u32,
        gids: &[u16],
        cache: Option<std::sync::Arc<super::super::fonts::FontCache>>,
    ) -> Vec<u8> {
        let mut doc = Doc::new();
        doc.fonts.cache = cache;
        doc.fonts.define(
            0,
            FontDef::Native(NativeDef {
                path: path.as_bytes().to_vec(),
                index,
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
        p.items = vec![Item::Matrix(1)];
        for (k, &gid) in gids.iter().enumerate() {
            p.items.push(Item::Glyph {
                font: res,
                code: gid,
                x: 657_818 + 400_000 * k as i32,
                y: 3_289_088,
                col: flashtex_display_list::page::NO_COLUMN,
            });
        }
        doc.pages.push(std::sync::Arc::new(Built {
            dl: p,
            sup: Supplement::default(),
            fonts: vec![res],
            images: vec![],
            forms: vec![],
            frame: std::sync::OnceLock::new(),
            forms_at: 0,
            replay: None,
        }));
        let opts = Options {
            producer: "test".into(),
            date: None,
            compress: false,
        };
        write(&doc, &opts).unwrap().0
    }

    /// The `/ID` hash, on its thread or in line, is the SHA-256 of the
    /// whole file however it was fed.
    #[test]
    fn id_hash_is_the_whole_file_hash() {
        let data: Vec<u8> = (0..3_000_000u32)
            .map(|i| (i.wrapping_mul(2654435761) >> 11) as u8)
            .collect();
        for inline in [false, true] {
            for steps in [
                &[0usize][..],
                &[10, 70_000, 300_000, 300_001, 2_999_999],
                &[1 << 18, 1 << 19],
            ] {
                let mut id = IdHash::new();
                if inline {
                    id.how = IdHow::Inline(Sha256::new());
                }
                for &n in steps {
                    id.feed(&data[..n], IdHash::CHUNK);
                    id.feed(&data[..n], 0);
                }
                assert_eq!(
                    id.finish(&data),
                    sha256(&data),
                    "inline {inline}, steps {steps:?}"
                );
            }
        }
    }

    /// A CID-keyed CFF font (CJK): the content stream draws its glyphs by
    /// their CIDs (the charset), the CIDFont program is the font's CFF, and
    /// `/W` is keyed by CID. TeX Live's HaranoAji Mincho; skipped without it.
    /// A CFF font is named by its CFF's Name INDEX, as xdvipdfmx names it:
    /// face 1 of macOS's Hiragino Sans GB (`name` table:
    /// `.HiraginoSansGBInterface-W3`) shares face 0's CFF, and TeX Live 2026
    /// xetex + xdvipdfmx names it `HiraginoSansGB-W3` (measured). Skipped
    /// without the font.
    #[test]
    fn cff_fonts_are_named_by_their_cff() {
        let path = "/System/Library/Fonts/Hiragino Sans GB.ttc";
        if !std::path::Path::new(path).exists() {
            eprintln!("skipped: no {path}");
            return;
        }
        let bytes = face_glyphs_pdf(path, 1, &[34], None);
        let pdf = PdfFile::parse(&bytes).unwrap();
        let page = pdf.pages().unwrap()[0];
        let f = *pdf.page_fonts(page).values().next().unwrap();
        let d = pdf
            .resolve(&f.get("DescendantFonts").and_then(Obj::as_array).unwrap()[0])
            .as_dict()
            .unwrap();
        let base = d.get("BaseFont").and_then(Obj::as_name).unwrap();
        assert!(base.ends_with("+HiraginoSansGB-W3"), "{base}");
    }

    #[test]
    fn type1_widths_and_flags_are_xdvipdfmxs() {
        // measured (TeX Live 2026 xetex + xdvipdfmx 20260317, `\font\a=cmr10`
        // etc.): CMR10 /Widths 555 (fi) 750 (H) 444 (e) 277 (l) 500 (o),
        // /Flags 6; CMTI10 70; CMTT10 6; CMSY10 70; CMBX12 262150 (ForceBold)
        let font = |name: &str| {
            let path = std::process::Command::new("kpsewhich")
                .arg(name)
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|p| !p.is_empty())?;
            let data = std::fs::read(path).ok()?;
            Some((flashtex_pdf::type1::Type1Font::parse_pfb(&data).ok()?, data))
        };
        let Some((cmr, _)) = font("cmr10.pfb") else {
            eprintln!("skipped: no cmr10.pfb (TeX Live)");
            return;
        };
        let w: Vec<f64> = ["fi", "H", "e", "l", "o"]
            .iter()
            .map(|g| type1_width(&cmr, g))
            .collect();
        assert_eq!(w, [555.0, 750.0, 444.0, 277.0, 500.0]);
        for (name, flags) in [
            ("cmr10.pfb", 6),
            ("cmti10.pfb", 70),
            ("cmtt10.pfb", 6),
            ("cmsy10.pfb", 70),
            ("cmbx12.pfb", 262150),
        ] {
            let (t1, data) = font(name).unwrap();
            assert_eq!(
                type1_flags(
                    t1.font_name().unwrap(),
                    t1.italic_angle().unwrap_or("0"),
                    type1_force_bold(&data)
                ),
                flags,
                "{name}"
            );
        }
    }

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
        // no ToUnicode: Adobe-Japan1 is a collection viewers know, and
        // xdvipdfmx writes none for it (CIDFont_is_ACCFont)
        assert!(f.get("ToUnicode").is_none());
        // its character collection, as xdvipdfmx writes it
        let csi = d.get("CIDSystemInfo").and_then(Obj::as_dict).unwrap();
        assert_eq!(csi.get("Ordering"), Some(&Obj::String(b"Japan1".to_vec())));
    }

    /// A resident engine's runs write the fonts' PDF objects from what the
    /// cache kept (the parsed face, the glyph list, the last subset,
    /// ToUnicode and CID map): byte for byte what a write without the
    /// cache makes, for the same glyphs again, for other glyphs, and for
    /// the first ones once more. A CFF and a TrueType face, and a
    /// CID-keyed CFF one when TeX Live has HaranoAji Mincho.
    #[test]
    fn the_font_cache_writes_what_a_write_without_it_does() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let mut fonts = vec![
            format!("{dir}/apps/mac/Fonts/lmmono10-regular.otf"),
            format!("{dir}/apps/mac/Sources/FlashTeXMac/Resources/Fonts/JetBrainsMono-Regular.ttf"),
        ];
        if let Some(p) = std::process::Command::new("kpsewhich")
            .arg("HaranoAjiMincho-Regular.otf")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|p| !p.is_empty())
        {
            fonts.push(p);
        }
        let runs: [&[u16]; 5] = [
            &[36, 37, 38],
            &[36, 37, 38],
            &[36, 37, 38, 70],
            &[36, 37, 38],
            &[5],
        ];
        for path in &fonts {
            let cache = std::sync::Arc::new(super::super::fonts::FontCache::default());
            for gids in runs {
                let with = glyphs_pdf(path, gids, Some(cache.clone()));
                let without = glyphs_pdf(path, gids, None);
                assert!(with == without, "{path}: glyphs {gids:?}");
            }
        }
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
        let rev = super::super::tounicode::cmap_inverse(&data, 0);
        let font = flashtex_pdf::truetype::TrueTypeFont::parse(data).unwrap();
        for c in ['日', '文'] {
            let g = font.glyph_id(c).unwrap();
            assert_eq!(rev.get(&g), Some(&c));
        }
    }

    /// The descriptor's entries as xdvipdfmx's `tt_get_fontdesc` writes
    /// them, measured (TeX Live 2026 xetex + xdvipdfmx 20260317, the fonts
    /// drawn by `\font\f="[path]"`): LM Mono 10 and LM Roman 10 Italic,
    /// whose `/StemV` is their CFF's `StdVW`, and a face without `OS/2`.
    #[test]
    fn font_descriptors_are_xdvipdfmxs() {
        let entries = |name: &str| {
            let path = format!("{}/../../apps/mac/Fonts/{name}", env!("CARGO_MANIFEST_DIR"));
            let data = std::fs::read(path).unwrap();
            let font = flashtex_pdf::truetype::TrueTypeFont::parse(data.clone()).unwrap();
            let std_vw = super::super::cffcid::private_value(font.cff_table().unwrap(), 11);
            FaceInfo::read(&data, 0).descriptor_entries(std_vw)
        };
        assert_eq!(
            entries("lmmono10-regular.otf"),
            " /Ascent 778 /Descent -222 /StemV 69 /CapHeight 778 /AvgWidth 500 \
             /FontBBox [-451 -316 731 1016] /ItalicAngle 0 /Flags 7 \
             /Style <</Panose (\\000\\000\\000\\000\\005\\011\\000\\000\\000\\000\\000\\000)>>"
        );
        assert_eq!(
            entries("lmroman10-italic.otf"),
            " /Ascent 806 /Descent -194 /StemV 56 /CapHeight 806 /AvgWidth 548 \
             /FontBBox [-458 -290 1386 1125] /ItalicAngle -14.03619385 /Flags 70 \
             /Style <</Panose (\\000\\000\\000\\000\\005\\000\\000\\000\\000\\000\\000\\000)>>"
        );
        // no OS/2 (Courier.ttc, measured: /Ascent 430 /Descent -59 at
        // 2048 units per em, /StemV 87, /Flags 6)
        let none = FaceInfo {
            upm: 2048,
            ..FaceInfo::default()
        };
        assert_eq!(
            none.descriptor_entries(None),
            " /Ascent 430 /Descent -59 /StemV 87 /CapHeight 430 \
             /FontBBox [0 0 0 0] /ItalicAngle 0 /Flags 6 \
             /Style <</Panose (\\000\\000\\000\\000\\000\\000\\000\\000\\000\\000\\000\\000)>>"
        );
    }

    /// A grey PNG with an `sRGB` chunk: the IMAGE's `color_space` is the
    /// `CalGray` the PDF writes, number for number, with its `/Intent`.
    #[test]
    fn image_color_space_is_the_pdfs() {
        let chunk = |kind: &[u8], body: &[u8]| {
            let mut c = (body.len() as u32).to_be_bytes().to_vec();
            c.extend_from_slice(kind);
            c.extend_from_slice(body);
            c.extend_from_slice(&[0; 4]);
            c
        };
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend(chunk(b"IHDR", &[0, 0, 0, 2, 0, 0, 0, 2, 8, 0, 0, 0, 0]));
        png.extend(chunk(b"sRGB", &[0]));
        png.extend(chunk(b"IDAT", &[0x78, 0x9C]));
        png.extend(chunk(b"IEND", &[]));
        let p = png_passthrough(&png).unwrap();
        assert_eq!(
            cal_space(&p.color, p.n_base).unwrap(),
            "[/CalGray <</Gamma 2.2 /WhitePoint [.95046 1 1.08906]>>]"
        );
        let cs = image_color_space(Kind::Png, &png).unwrap();
        assert_eq!(
            cs.to_string(),
            r#"{"type":"calgray","gamma":2.2,"white_point":[0.95046,1,1.08906],"intent":"Perceptual"}"#
        );
        // a PNG without colour chunks, and a JPEG without a profile: Device
        let mut plain = png.clone();
        let s = plain.windows(4).position(|w| w == b"sRGB").unwrap() - 4;
        plain.drain(s..s + 13);
        assert!(image_color_space(Kind::Png, &plain).is_none());
        assert!(image_color_space(Kind::Jpeg, &[0xFF, 0xD8, 0xFF, 0xD9]).is_none());
    }

    /// The outline written for `entries` (level, `[]`, title): the root's
    /// `/Count`, whether it has `/Type`, and each entry in order as
    /// "depth title /Count".
    fn outline_of(entries: &[(i32, bool, &str)]) -> (String, bool, Vec<String>) {
        use super::super::doc::Outline;
        use super::super::pdfobj::{Dict as D, Obj as O};
        let mut doc = Doc::new();
        for &(level, open, title) in entries {
            doc.outlines.push(Outline {
                level,
                open,
                dict: D(vec![(b"Title".to_vec(), O::Str(title.as_bytes().to_vec()))]),
                goto: None,
            });
        }
        let opts = Options {
            producer: "test".into(),
            date: None,
            compress: false,
        };
        let bytes = write(&doc, &opts).unwrap().0;
        let pdf = PdfFile::parse(&bytes).unwrap();
        let cat = pdf.catalog().unwrap();
        let root = pdf.resolve(cat.get("Outlines").unwrap()).as_dict().unwrap();
        fn walk(
            pdf: &PdfFile,
            d: &std::collections::BTreeMap<String, Obj>,
            depth: usize,
            out: &mut Vec<String>,
        ) {
            let mut it = d.get("First").map(|o| pdf.resolve(o));
            while let Some(e) = it.and_then(Obj::as_dict) {
                let t = match e.get("Title") {
                    Some(Obj::String(s)) => String::from_utf8_lossy(s).into_owned(),
                    _ => "?".into(),
                };
                let c = e.get("Count").and_then(Obj::as_number).unwrap_or("");
                out.push(format!("{depth} {t} {c}").trim_end().to_string());
                walk(pdf, e, depth + 1, out);
                it = e.get("Next").map(|o| pdf.resolve(o));
            }
        }
        let mut got = Vec::new();
        walk(&pdf, root, 1, &mut got);
        let count = root
            .get("Count")
            .and_then(Obj::as_number)
            .unwrap_or("")
            .to_string();
        (count, root.contains_key("Type"), got)
    }

    /// The outline tree xdvipdfmx (TeX Live 2026, `-O 0`) builds from these
    /// entries, measured (#2357): a closed entry's `/Count` counts the
    /// entries visible below it once opened (B: -3, F: -2), a jump of two
    /// levels (I to J) fills the level skipped with `<No Title>`, and the
    /// root has no `/Type`.
    #[test]
    fn the_outline_tree_is_xdvipdfmxs() {
        let (count, typed, got) = outline_of(&[
            (1, true, "A"),
            (2, false, "B"),
            (3, true, "C"),
            (4, true, "D"),
            (4, false, "E"),
            (1, false, "F"),
            (2, true, "G"),
            (3, true, "H"),
            (1, false, "I"),
            (3, false, "J"),
        ]);
        assert_eq!(count, "4");
        assert!(!typed);
        assert_eq!(
            got,
            [
                "1 A 1",
                "2 B -3",
                "3 C 2",
                "4 D",
                "4 E",
                "1 F -2",
                "2 G 1",
                "3 H",
                "1 I -1",
                "2 <No Title> -1",
                "3 J",
            ]
        );
    }

    /// Levels count from the lowest seen so far (#2366, measured): A2 and
    /// B3 start at depth 1 and 2; C1 then lowers the base, so D3 is two
    /// levels below C1, under a `<No Title>` entry.
    #[test]
    fn outline_levels_count_from_the_lowest_so_far() {
        let (count, _, got) = outline_of(&[
            (2, true, "A2"),
            (3, true, "B3"),
            (1, true, "C1"),
            (3, true, "D3"),
            (1, false, "E1"),
            (2, false, "F2"),
            (2, false, "G2"),
        ]);
        assert_eq!(count, "5");
        assert_eq!(
            got,
            [
                "1 A2 1",
                "2 B3",
                "1 C1 1",
                "2 <No Title> -1",
                "3 D3",
                "1 E1 -2",
                "2 F2",
                "2 G2",
            ]
        );
    }

    /// RULE_GEOMETRY is the numbers the content stream draws each rule with.
    #[test]
    fn rule_geometry_is_what_the_content_stream_draws() {
        let mut p = Page::new(StreamKind::Page, 0);
        p.pdf_box = [0.0, 0.0, 612.0, 792.0];
        let (x, y, w, h) = (21851747, 22122140, 2095109, 44560);
        for kind in [RuleKind::Fill, RuleKind::StrokeH, RuleKind::StrokeV] {
            p.items.push(Item::Rule { kind, x, y, w, h });
        }
        let g = rule_geometry(&p);
        assert_eq!(g.len(), 3);
        let (l, t) = (x as f64 / K, 792.0 - y as f64 / K);
        let (wb, hb) = (w as f64 / K, h as f64 / K);
        let r = |v: f64| n(v, 5).parse::<f64>().unwrap();
        assert_eq!(g[0], [0.0, 0.0, r(l), r(t - hb), r(wb), r(hb), 0.0]);
        let ym = t - hb + hb / 2.0;
        assert_eq!(g[1], [0.0, 0.0, r(l), r(ym), r(l + wb), r(ym), r(hb)]);
        assert_eq!(g[1][2], 332.1855);
        let xm = l + wb / 2.0;
        assert_eq!(g[2], [0.0, 0.0, r(xm), r(t - hb), r(xm), r(t), r(wb)]);
    }

    /// A form that only a named object refers to is written: pgf's soft
    /// mask is a `pdf:bxobj @pgfsmask5` form that no page draws, named
    /// only by `pdf:put @pgfextgs << /pgfsmask5 << /SMask << /G @pgfsmask5
    /// >> >> >>` (beamer's shadowed title block, u11-beamer-fontspec in
    /// the M3 export gate). It was referenced but written as `null`.
    #[test]
    fn a_form_only_a_named_object_names_is_written() {
        use super::super::page::tests::{page_bytes, special, NoTfm};
        use super::super::page::Builder;
        let mut doc = Doc::new();
        let mut body = special("pdf:obj @pgfextgs <<>>");
        body.extend(special("pdf:put @resources << /ExtGState @pgfextgs >>"));
        body.extend(special(
            "pdf:bxobj @pgfsmask5 width 20pt height 10pt depth 0pt",
        ));
        body.push(137); // put_rule inside the form
        body.extend_from_slice(&(5i32 << 16).to_be_bytes());
        body.extend_from_slice(&(10i32 << 16).to_be_bytes());
        body.extend(special(
            "pdf:exobj <</Group <</S/Transparency /CS/DeviceGray>> >>",
        ));
        body.extend(special(
            "pdf:put @pgfextgs << /pgfsmask5 << /SMask << /S /Luminosity /G @pgfsmask5>> >> >>",
        ));
        body.extend(special("pdf:code /pgfsmask5 gs"));
        let built = Builder::page(&mut doc, &NoTfm, 1000, 1, &page_bytes(&body)).unwrap();
        doc.pages.push(std::sync::Arc::new(built));
        let opts = Options {
            producer: "test".into(),
            date: None,
            compress: false,
        };
        let bytes = write(&doc, &opts).unwrap().0;
        let text = String::from_utf8_lossy(&bytes);
        let at = text.find("/G ").expect("the soft mask's /G");
        let num: String = text[at + 3..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        assert!(text[at + 3 + num.len()..].starts_with(" 0 R"), "{text}");
        let obj = text
            .find(&format!("\n{num} 0 obj\n"))
            .map(|i| &text[i..])
            .expect("the form's object");
        let body = &obj[..obj.find("endobj").unwrap()];
        assert!(
            body.contains("/Subtype /Form") || body.contains("/Subtype/Form"),
            "{body}"
        );
        assert!(body.contains("/Transparency"), "{body}");
        let pdf = PdfFile::parse(&bytes).unwrap();
        assert_eq!(pdf.pages().unwrap().len(), 1);
    }

    /// hyperref's form fields are `pdf:ann @textN …` with the AcroForm's
    /// `/Fields` naming them (`pdf:put @afields @textN`): the widget is one
    /// object, in the page's `/Annots` and in `/Fields`, with what `pdf:put
    /// @textN` added and no `/Type` (dvipdfmx's `spc_push_object` and
    /// `pdf_doc_add_annot`; xdvipdfmx's PDF of the parity case
    /// d10-hyperref-forms-transformed). `/Fields` held `null`s before.
    #[test]
    fn a_named_annotation_is_the_object_its_name_refers_to() {
        use super::super::page::tests::{page_bytes, special, NoTfm};
        use super::super::page::Builder;
        let mut doc = Doc::new();
        let mut body = special("pdf:obj @afields []");
        body.extend(special(
            "pdf:ann @w width 20pt height 10pt depth 2pt <</Subtype/Widget/FT/Tx/T(w)>>",
        ));
        body.extend(special("pdf:put @w <</V(filled)>>"));
        body.extend(special("pdf:put @afields @w"));
        body.extend(special(
            "pdf:put @catalog <</AcroForm<</Fields @afields>>>>",
        ));
        let built = Builder::page(&mut doc, &NoTfm, 1000, 1, &page_bytes(&body)).unwrap();
        doc.pages.push(std::sync::Arc::new(built));
        let opts = Options {
            producer: "test".into(),
            date: None,
            compress: false,
        };
        let bytes = write(&doc, &opts).unwrap().0;
        let text = String::from_utf8_lossy(&bytes);
        let num_after = |key: &str| -> String {
            let at = text.find(key).unwrap_or_else(|| panic!("no {key}: {text}"));
            text[at + key.len()..]
                .trim_start_matches([' ', '['])
                .chars()
                .take_while(char::is_ascii_digit)
                .collect()
        };
        let annot = num_after("/Annots");
        let fields = num_after("/Fields");
        let obj = |n: &str| -> String {
            let i = text
                .find(&format!("\n{n} 0 obj\n"))
                .unwrap_or_else(|| panic!("no object {n}: {text}"));
            let o = &text[i..];
            o[..o.find("endobj").unwrap()].to_string()
        };
        let f = obj(&fields);
        assert!(f.contains(&format!("{annot} 0 R")), "{f}\n{text}");
        let w = obj(&annot);
        assert!(
            w.contains("/Widget") && w.contains("(filled)") && w.contains("/Rect"),
            "{w}"
        );
        assert!(!w.contains("/Annot"), "{w}");
        let pdf = PdfFile::parse(&bytes).unwrap();
        assert_eq!(pdf.pages().unwrap().len(), 1);
    }

    /// dvipdfmx's destination optimisation, measured with TeX Live 2026's
    /// xetex and xdvipdfmx on a plain XeTeX file with these specials: the
    /// GoTo destinations of outline entries and links become `(0)`, `(1)`,
    /// … in the order they are met (an undefined one takes a number too);
    /// a destination array, a URI and a GoToR keep theirs; only the
    /// destinations so named are written, under the new names (here 0, 1,
    /// 3 and, from page 2, 4: `unused` is dropped). A `dvipdfmx:config C
    /// 0x0010` on page 2 changes nothing (dvipdfmx reads page 1's); on page
    /// 1 it keeps every name (hyperref's case).
    #[test]
    fn destinations_are_renamed_as_dvipdfmx_does() {
        use super::super::page::tests::{page_bytes, special, NoTfm};
        use super::super::page::Builder;
        let page1 = |config: bool| {
            let mut body = if config {
                special("dvipdfmx:config C 0x0010")
            } else {
                Vec::new()
            };
            for s in [
                "pdf:dest (alpha) [@thispage /Fit]",
                "pdf:dest (beta) [@thispage /Fit]",
                "pdf:dest (unused) [@thispage /FitH 500]",
                "pdf:dest (gamma) [@thispage /Fit]",
                "pdf:outline 1 <</Title (One) /A <</S/GoTo/D (alpha)>>>>",
                "pdf:outline 2 <</Title (Two) /Dest (beta)>>",
                "pdf:outline 1 <</Title (Three) /Dest [@thispage /Fit]>>",
                "pdf:ann width 20pt height 10pt <</Subtype/Link/A<</S/GoTo/D(beta)>>>>",
                "pdf:ann width 20pt height 10pt <</Subtype/Link/Dest(alpha)>>",
                "pdf:ann width 20pt height 10pt <</Subtype/Link/A<</S/URI/URI(http://x)>>>>",
                "pdf:ann width 20pt height 10pt <</Subtype/Link/A<</S/GoTo/D(missing)>>>>",
                "pdf:ann width 20pt height 10pt <</Subtype/Link/A<</S/GoToR/F(o.pdf)/D(alpha)>>>>",
                "pdf:ann width 20pt height 10pt <</Subtype/Link/Dest(gamma)>>",
            ] {
                body.extend(special(s));
            }
            page_bytes(&body)
        };
        let mut page2 = special("dvipdfmx:config C 0x0010");
        page2.extend(special("pdf:dest (delta) [@thispage /Fit]"));
        page2.extend(special(
            "pdf:ann width 20pt height 10pt <</Subtype/Link/A<</S/GoTo/D(delta)>>>>",
        ));
        let page2 = page_bytes(&page2);
        let opts = Options {
            producer: "test".into(),
            date: None,
            compress: false,
        };
        for config in [false, true] {
            let mut doc = Doc::new();
            for (n, b) in [(1, page1(config)), (2, page2.clone())] {
                let built = Builder::page(&mut doc, &NoTfm, 1000, n, &b).unwrap();
                doc.pages.push(std::sync::Arc::new(built));
            }
            let bytes = write(&doc, &opts).unwrap().0;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let pdf = PdfFile::parse(&bytes).unwrap();
            assert_eq!(pdf.pages().unwrap().len(), 2);
            let gotos: Vec<Option<Vec<u8>>> = doc.pages[0]
                .sup
                .annots
                .iter()
                .map(|a| a.goto.as_ref().map(|g| g.name.clone()))
                .collect();
            let outl: Vec<Option<Vec<u8>>> = doc
                .outlines
                .iter()
                .map(|o| o.goto.as_ref().map(|g| g.name.clone()))
                .collect();
            let at = text.find("/Names [").expect("the Dests tree");
            let dests = &text[at..at + text[at..].find("endobj").unwrap()];
            if config {
                assert!(!doc.check_gotos);
                assert!(gotos.iter().chain(&outl).all(Option::is_none));
                for n in ["(alpha)", "(beta)", "(unused)", "(gamma)", "(delta)"] {
                    assert!(dests.contains(n), "{n}: {dests}");
                }
                continue;
            }
            let s = |v: &str| Some(v.as_bytes().to_vec());
            assert_eq!(outl, [s("0"), s("1"), None]);
            assert_eq!(gotos, [s("1"), s("0"), None, s("2"), None, s("3")]);
            let p2: Vec<_> = doc.pages[1]
                .sup
                .annots
                .iter()
                .map(|a| a.goto.clone())
                .collect();
            assert_eq!(p2[0].as_ref().map(|g| g.name.clone()), s("4"));
            for n in ["(0)", "(1)", "(3)", "(4)"] {
                assert!(dests.contains(n), "{n}: {dests}");
            }
            for n in ["(2)", "(alpha)", "(beta)", "(unused)", "(gamma)", "(delta)"] {
                assert!(!dests.contains(n), "{n}: {dests}");
            }
            // the GoToR keeps its name; no link names an old one
            assert_eq!(text.matches("(alpha)").count(), 1, "{text}");
            assert!(!text.contains("(beta)") && !text.contains("(missing)"));
        }
    }

    /// Destinations named through `@name` objects, as `pdf_deref_obj`
    /// follows them (measured with TeX Live 2026's xetex and xdvipdfmx, a
    /// plain XeTeX file with these specials): `/Dest @dn` and `/D @dd`
    /// become the new name in the link (`(0)`, `(2)`); `/A @act` is renamed
    /// in the named action itself, so a second link to it finds `(1)`, which
    /// no destination has, and makes it `(3)`; `/Dest @later`, made after
    /// the link, stays a reference to `(delta)`, which is then not written
    /// as a destination; the outline's `/Dest @dd` is `(2)`.
    #[test]
    fn destinations_through_named_objects_are_renamed_as_dvipdfmx_does() {
        use super::super::page::tests::{page_bytes, special, NoTfm};
        use super::super::page::Builder;
        use super::super::pdfobj::Obj as P;
        let mut body = Vec::new();
        for s in [
            "pdf:dest (alpha) [@thispage /Fit]",
            "pdf:dest (beta) [@thispage /Fit]",
            "pdf:dest (gamma) [@thispage /Fit]",
            "pdf:dest (delta) [@thispage /Fit]",
            "pdf:obj @dn (alpha)",
            "pdf:obj @act <</S/GoTo/D(beta)>>",
            "pdf:obj @dd (gamma)",
            "pdf:ann width 20pt height 10pt <</Subtype/Link/Dest @dn>>",
            "pdf:ann width 20pt height 10pt <</Subtype/Link/A @act>>",
            "pdf:ann width 20pt height 10pt <</Subtype/Link/A <</S/GoTo/D @dd>>>>",
            "pdf:ann width 20pt height 10pt <</Subtype/Link/Dest @later>>",
            "pdf:ann width 20pt height 10pt <</Subtype/Link/A @act>>",
            "pdf:obj @later (delta)",
            "pdf:outline 1 <</Title (O) /Dest @dd>>",
        ] {
            body.extend(special(s));
        }
        let mut doc = Doc::new();
        let built = Builder::page(&mut doc, &NoTfm, 1000, 1, &page_bytes(&body)).unwrap();
        let s = |v: &str| Some(v.as_bytes().to_vec());
        let gotos: Vec<_> = built
            .sup
            .annots
            .iter()
            .map(|a| a.goto.as_ref().map(|g| g.name.clone()))
            .collect();
        assert_eq!(gotos, [s("0"), None, s("2"), None, None]);
        assert_eq!(
            doc.outlines[0].goto.as_ref().map(|g| g.name.clone()),
            s("2")
        );
        let Some(Named::Obj(P::Dict(act))) = doc.named.get(&b"act"[..]) else {
            panic!("@act");
        };
        assert_eq!(act.get(b"D"), Some(&P::Str(b"3".to_vec())));
        doc.pages.push(std::sync::Arc::new(built));
        let opts = Options {
            producer: "test".into(),
            date: None,
            compress: false,
        };
        let bytes = write(&doc, &opts).unwrap().0;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let at = text.find("/Names [").expect("the Dests tree");
        let dests = &text[at..at + text[at..].find("endobj").unwrap()];
        for n in ["(0)", "(1)", "(2)"] {
            assert!(dests.contains(n), "{n}: {dests}");
        }
        for n in ["(3)", "(alpha)", "(beta)", "(gamma)", "(delta)"] {
            assert!(!dests.contains(n), "{n}: {dests}");
        }
    }

    /// A PNG's chunks with dummy CRCs (png_passthrough does not check them).
    fn png(ihdr: (u32, u32, u8, u8), extra: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut chunk = |k: &[u8; 4], b: &[u8]| {
            v.extend((b.len() as u32).to_be_bytes());
            v.extend(k);
            v.extend(b);
            v.extend([0; 4]);
        };
        let mut h = ihdr.0.to_be_bytes().to_vec();
        h.extend(ihdr.1.to_be_bytes());
        h.extend([ihdr.2, ihdr.3, 0, 0, 0]);
        chunk(b"IHDR", &h);
        for (k, b) in extra {
            chunk(k, b);
        }
        chunk(b"IDAT", &[]);
        chunk(b"IEND", &[]);
        v
    }

    /// pngimage.c's transparency, measured on TeX Live 2026's xdvipdfmx
    /// (lane AUDIT-DVIPDFMX): grey `tRNS` 10 -> `/Mask [10 10]`; RGB
    /// (0, 255, 0) -> `/Mask [0 0 255 255 0 0]`; palette ff ff 00 -> `/Mask
    /// [2 2]`; palette 00 ff 00 -> `/Mask [0 0 2 2]`; palette 00 80 ff 40 ->
    /// an `/SMask`.
    #[test]
    fn png_trns_as_xdvipdfmx() {
        let plte = [0u8; 18];
        let t = |d: Vec<u8>| png_passthrough(&d).expect("passes through").trans;
        assert_eq!(
            t(png((7, 5, 8, 0), &[(b"tRNS", &[0, 10])])),
            PngTrans::Key(vec![10, 10])
        );
        assert_eq!(
            t(png((7, 5, 8, 2), &[(b"tRNS", &[0, 0, 0, 255, 0, 0])])),
            PngTrans::Key(vec![0, 0, 255, 255, 0, 0])
        );
        assert_eq!(
            t(png(
                (7, 5, 4, 3),
                &[(b"PLTE", &plte), (b"tRNS", &[0xff, 0xff, 0])]
            )),
            PngTrans::Key(vec![2, 2])
        );
        assert_eq!(
            t(png(
                (7, 5, 8, 3),
                &[(b"PLTE", &plte), (b"tRNS", &[0, 0xff, 0])]
            )),
            PngTrans::Key(vec![0, 0, 2, 2])
        );
        assert_eq!(
            t(png(
                (7, 5, 4, 3),
                &[(b"PLTE", &plte), (b"tRNS", &[0, 0x80, 0xff, 0x40])]
            )),
            PngTrans::Alpha(vec![0, 0x80, 0xff, 0x40])
        );
        // a tRNS longer than the palette is dropped by libpng
        assert_eq!(
            t(png((7, 5, 4, 3), &[(b"PLTE", &plte), (b"tRNS", &[0; 7])])),
            PngTrans::None
        );
        assert_eq!(t(png((7, 5, 8, 0), &[])), PngTrans::None);
        // 16-bit grey, tRNS 300: "/BitsPerComponent 16 ... /Mask [300 300]"
        let d = png((7, 5, 16, 0), &[(b"tRNS", &[1, 44])]);
        let p = png_passthrough(&d).expect("passes through");
        assert_eq!(p.trans, PngTrans::Key(vec![300, 300]));
        assert!(p.dict.contains("/BitsPerComponent 16"), "{}", p.dict);
        // without a key it still takes the other route
        assert!(png_passthrough(&png((7, 5, 16, 0), &[])).is_none());
    }

    /// The soft mask of a 7 x 5, 4-bit palette PNG with `tRNS` 00 80 ff
    /// 40, as TeX Live 2026's xdvipdfmx writes it (its `/SMask` stream,
    /// decoded): `create_soft_mask` reads the packed rows as if unpadded.
    #[test]
    fn png_soft_mask_reads_rows_unpadded() {
        let rows = [
            0x00, 0x12, 0x34, 0x50, 0x12, 0x23, 0x45, 0x00, 0x23, 0x45, 0x50, 0x10, 0x34, 0x50,
            0x12, 0x20, 0x55, 0x01, 0x23, 0x40,
        ];
        let want = [
            0x00, 0x00, 0x80, 0xff, 0x40, 0xff, 0xff, 0x00, 0x80, 0xff, 0xff, 0x40, 0xff, 0xff,
            0x00, 0x00, 0xff, 0x40, 0xff, 0xff, 0xff, 0x00, 0x80, 0x00, 0x40, 0xff, 0xff, 0x00,
            0x80, 0xff, 0xff, 0x00, 0xff, 0xff, 0x00,
        ];
        assert_eq!(png_soft_mask(&rows, 4, 7, 5, &[0, 0x80, 0xff, 0x40]), want);
    }
}
