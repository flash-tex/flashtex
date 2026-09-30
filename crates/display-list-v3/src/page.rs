//! The `PAGE` and `FORM` bodies (spec §4): a fixed header, then tagged
//! sections. The item list is in painting order, as the PDF content stream
//! paints.
//!
//! Coordinates. Glyph origins, rules, links and destinations are in scaled
//! points (sp, 65536 per TeX point), x to the right from the left edge and y
//! down from the top edge of the page (or of the form's bounding box).
//! Matrices, path coordinates and colours are the PDF's own numbers (f64),
//! in the stream's PDF space: points (bp), origin at the bottom-left corner,
//! y up. A client that draws in a top-left, y-down space in points composes
//! every matrix with `[1 0 0 -1 0 H]`, H the box height in bp.

use crate::frame::{Cursor, Put};
use crate::sha256::Sha256;

/// Header flags.
pub mod flags {
    /// The stream used something this version cannot express (listed in
    /// the UNSUPPORTED section and marked in the items); drawing the items
    /// alone may differ from the PDF. A client may render the PDF page.
    pub const INCOMPLETE: u32 = 1;
    /// No geometry (\pdfdraftmode, or DVI mode): the items are empty.
    pub const NO_GEOMETRY: u32 = 2;
}

/// Section tags.
pub mod section {
    pub const MATRICES: u32 = 1;
    pub const PATHS: u32 = 2;
    pub const ITEMS: u32 = 3;
    pub const LINKS: u32 = 4;
    pub const DESTS: u32 = 5;
    pub const UNSUPPORTED: u32 = 6;
}

/// Item opcodes.
pub mod op {
    pub const GLYPH: u8 = 0x01;
    pub const RULE: u8 = 0x02;
    pub const PATH: u8 = 0x03;
    pub const CLIP: u8 = 0x04;
    pub const IMAGE: u8 = 0x05;
    pub const FORM: u8 = 0x06;
    pub const SAVE: u8 = 0x07;
    pub const RESTORE: u8 = 0x08;
    pub const FILL_COLOR: u8 = 0x09;
    pub const STROKE_COLOR: u8 = 0x0A;
    pub const MATRIX: u8 = 0x0B;
    pub const SPAN: u8 = 0x0C;
    pub const TEXT_RENDER: u8 = 0x0D;
    pub const UNSUPPORTED: u8 = 0x0E;
}

/// Unknown column (the node was not made while reading a file line).
pub const NO_COLUMN: u16 = 0xFFFF;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleKind {
    /// `re f`: fill the rectangle.
    Fill = 0,
    /// A stroked horizontal line along the rectangle's middle, line width
    /// = its height, butt caps (pdfTeX's rule of at most 1 bp height).
    StrokeH = 1,
    /// A stroked vertical line along the middle, line width = its width.
    StrokeV = 2,
}

impl RuleKind {
    fn from(b: u8) -> Result<RuleKind, String> {
        Ok(match b {
            0 => RuleKind::Fill,
            1 => RuleKind::StrokeH,
            2 => RuleKind::StrokeV,
            _ => return Err(format!("unknown rule kind {b}")),
        })
    }
}

/// A colour: DeviceGray (1), DeviceRGB (3) or DeviceCMYK (4) components.
#[derive(Clone, Debug, PartialEq)]
pub struct Color(pub Vec<f64>);

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// Glyph `code` of font resource `font`, origin at (x, y) sp, drawn
    /// with the current glyph matrix, fill colour and text render mode.
    /// `col` is the source column (see `Span`), or [`NO_COLUMN`].
    Glyph {
        font: u16,
        code: u16,
        x: i32,
        y: i32,
        col: u16,
    },
    /// A rule: left x, top y, width, height (sp), painted with the fill
    /// colour (`Fill`) or the stroke colour (the stroked kinds).
    Rule {
        kind: RuleKind,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    },
    /// Paint path `n` of the PATHS section.
    Path(u32),
    /// Intersect the clip with path `n` (until the matching `Restore`).
    Clip(u32),
    /// Draw image resource `id` with matrix `m` (maps the unit square).
    Image {
        id: u32,
        matrix: u32,
    },
    /// Draw form `id` with matrix `m` (maps the form's PDF space).
    Form {
        id: u32,
        matrix: u32,
    },
    Save,
    Restore,
    FillColor(Color),
    StrokeColor(Color),
    /// The glyph matrix for the following glyphs: entries a, b, c, d of
    /// matrix `n` map text space (1 unit = 1 em) to PDF space.
    Matrix(u32),
    /// The source span of the following items (0 = none).
    Span(u32),
    /// PDF text render mode (0 fill, 1 stroke, 2 fill+stroke, 3 invisible).
    TextRender(u8),
    /// Something at this point was not expressed: entry `n` of UNSUPPORTED.
    Unsupported(u32),
}

/// Stroke parameters of a path.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub width: f64,
    pub cap: u8,
    pub join: u8,
    pub miter: f64,
    pub dash: Vec<f64>,
    pub phase: f64,
}

/// Path segment.
#[derive(Clone, Debug, PartialEq)]
pub enum Seg {
    Move(f64, f64),
    Line(f64, f64),
    Curve(f64, f64, f64, f64, f64, f64),
    Close,
}

/// Paint bits.
pub mod paint {
    pub const FILL: u8 = 1;
    pub const FILL_EVEN_ODD: u8 = 2;
    pub const STROKE: u8 = 4;
    pub const CLIP: u8 = 8;
    pub const CLIP_EVEN_ODD: u8 = 16;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    /// [`paint`] bits.
    pub paint: u8,
    /// The CTM (index into MATRICES; 0 = identity), mapping the segments'
    /// user space to PDF space.
    pub matrix: u32,
    pub stroke: Option<Stroke>,
    pub segs: Vec<Seg>,
}

/// A link annotation's action.
#[derive(Clone, Debug, PartialEq)]
pub enum LinkKind {
    /// `goto name{..}`: a named destination.
    GotoName = 1,
    /// `goto num N`: a numbered destination (data: decimal).
    GotoNum = 2,
    /// `goto page N {..}` (data: "N" then the destination spec).
    GotoPage = 3,
    /// A URI (`user{/S/URI/URI(..)}`), data: the URI bytes.
    Uri = 4,
    /// Any other action: the action dictionary as pdfTeX writes it.
    Raw = 5,
    /// An article thread.
    Thread = 6,
}

impl LinkKind {
    fn from(b: u8) -> LinkKind {
        match b {
            1 => LinkKind::GotoName,
            2 => LinkKind::GotoNum,
            3 => LinkKind::GotoPage,
            4 => LinkKind::Uri,
            6 => LinkKind::Thread,
            _ => LinkKind::Raw,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// Rectangle (sp, y down): left, top, right, bottom.
    pub rect: [i32; 4],
    pub span: u32,
    pub kind: LinkKind,
    /// A file (`goto file(..)`), when the action names one.
    pub file: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dest {
    /// true: `name` is a name; false: a decimal number.
    pub named: bool,
    pub name: Vec<u8>,
    /// 0 xyz, 1 fit, 2 fith, 3 fitv, 4 fitb, 5 fitbh, 6 fitbv, 7 fitr.
    pub kind: u8,
    /// left, top, right, bottom (sp, y down); xyz uses left/top.
    pub rect: [i32; 4],
    /// xyz zoom in thousandths (0 = keep).
    pub zoom: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamKind {
    Page,
    Form,
}

/// A decoded page or form.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub kind: StreamKind,
    /// Page: 0-based ship-out index. Form: the form's id (pdfTeX's `/Fm` n).
    pub index: u32,
    pub flags: u32,
    /// Page: TeX's page width and height (sp). Form: box width and
    /// height + depth.
    pub width: i32,
    pub height: i32,
    /// \count0..\count9 at ship-out (pages; zero for forms).
    pub counts: [i32; 10],
    /// The PDF box: MediaBox (page) or BBox (form), in bp.
    pub pdf_box: [f64; 4],
    pub hash: [u8; 32],
    /// Matrices 1..=n (index 0 is the identity and is not stored).
    pub matrices: Vec<[f64; 6]>,
    pub paths: Vec<Path>,
    pub items: Vec<Item>,
    pub links: Vec<Link>,
    pub dests: Vec<Dest>,
    pub unsupported: Vec<String>,
}

impl Page {
    pub fn new(kind: StreamKind, index: u32) -> Page {
        Page {
            kind,
            index,
            flags: 0,
            width: 0,
            height: 0,
            counts: [0; 10],
            pdf_box: [0.0; 4],
            hash: [0; 32],
            matrices: Vec::new(),
            paths: Vec::new(),
            items: Vec::new(),
            links: Vec::new(),
            dests: Vec::new(),
            unsupported: Vec::new(),
        }
    }

    /// Matrix `n` (0 = identity).
    pub fn matrix(&self, n: u32) -> [f64; 6] {
        if n == 0 {
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        } else {
            self.matrices
                .get(n as usize - 1)
                .copied()
                .unwrap_or([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
        }
    }

    /// Encode the body. The header's `hash` is written as given.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(256 + self.items.len() * 16);
        out.put_u32(self.index);
        out.put_u32(self.flags);
        out.put_i32(self.width);
        out.put_i32(self.height);
        for c in self.counts {
            out.put_i32(c);
        }
        for v in self.pdf_box {
            out.put_f64(v);
        }
        out.extend_from_slice(&self.hash);
        let sections: Vec<(u32, Vec<u8>)> = vec![
            (section::MATRICES, self.enc_matrices()),
            (section::PATHS, self.enc_paths()),
            (section::ITEMS, self.enc_items(false)),
            (section::LINKS, self.enc_links()),
            (section::DESTS, self.enc_dests()),
            (section::UNSUPPORTED, self.enc_unsupported()),
        ];
        out.put_u32(sections.len() as u32);
        for (tag, data) in sections {
            out.put_u32(tag);
            out.put_u32(data.len() as u32);
            out.extend_from_slice(&data);
        }
        out
    }

    fn enc_matrices(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(4 + self.matrices.len() * 48);
        o.put_u32(self.matrices.len() as u32);
        for m in &self.matrices {
            for v in m {
                o.put_f64(*v);
            }
        }
        o
    }

    fn enc_paths(&self) -> Vec<u8> {
        let mut o = Vec::new();
        o.put_u32(self.paths.len() as u32);
        for p in &self.paths {
            o.put_u8(p.paint);
            o.put_u32(p.matrix);
            if p.paint & paint::STROKE != 0 {
                let s = p.stroke.clone().unwrap_or(Stroke {
                    width: 1.0,
                    cap: 0,
                    join: 0,
                    miter: 10.0,
                    dash: vec![],
                    phase: 0.0,
                });
                o.put_f64(s.width);
                o.put_u8(s.cap);
                o.put_u8(s.join);
                o.put_f64(s.miter);
                o.put_u16(s.dash.len() as u16);
                for d in &s.dash {
                    o.put_f64(*d);
                }
                o.put_f64(s.phase);
            }
            o.put_u32(p.segs.len() as u32);
            for s in &p.segs {
                match s {
                    Seg::Move(x, y) => {
                        o.put_u8(0);
                        o.put_f64(*x);
                        o.put_f64(*y);
                    }
                    Seg::Line(x, y) => {
                        o.put_u8(1);
                        o.put_f64(*x);
                        o.put_f64(*y);
                    }
                    Seg::Curve(a, b, c, d, e, f) => {
                        o.put_u8(2);
                        for v in [a, b, c, d, e, f] {
                            o.put_f64(*v);
                        }
                    }
                    Seg::Close => o.put_u8(3),
                }
            }
        }
        o
    }

    /// The ITEMS section. With `for_hash`, spans and columns are left out
    /// (they locate the source; they do not change what is drawn).
    fn enc_items(&self, for_hash: bool) -> Vec<u8> {
        let mut o = Vec::with_capacity(self.items.len() * 15);
        for it in &self.items {
            match it {
                Item::Glyph {
                    font,
                    code,
                    x,
                    y,
                    col,
                } => {
                    o.put_u8(op::GLYPH);
                    o.put_u16(*font);
                    o.put_u16(*code);
                    o.put_i32(*x);
                    o.put_i32(*y);
                    o.put_u16(if for_hash { 0 } else { *col });
                }
                Item::Rule { kind, x, y, w, h } => {
                    o.put_u8(op::RULE);
                    o.put_u8(*kind as u8);
                    o.put_i32(*x);
                    o.put_i32(*y);
                    o.put_i32(*w);
                    o.put_i32(*h);
                }
                Item::Path(n) => {
                    o.put_u8(op::PATH);
                    o.put_u32(*n);
                }
                Item::Clip(n) => {
                    o.put_u8(op::CLIP);
                    o.put_u32(*n);
                }
                Item::Image { id, matrix } => {
                    o.put_u8(op::IMAGE);
                    o.put_u32(*id);
                    o.put_u32(*matrix);
                }
                Item::Form { id, matrix } => {
                    o.put_u8(op::FORM);
                    o.put_u32(*id);
                    o.put_u32(*matrix);
                }
                Item::Save => o.put_u8(op::SAVE),
                Item::Restore => o.put_u8(op::RESTORE),
                Item::FillColor(c) | Item::StrokeColor(c) => {
                    o.put_u8(if matches!(it, Item::FillColor(_)) {
                        op::FILL_COLOR
                    } else {
                        op::STROKE_COLOR
                    });
                    o.put_u8(c.0.len() as u8);
                    for v in &c.0 {
                        o.put_f64(*v);
                    }
                }
                Item::Matrix(n) => {
                    o.put_u8(op::MATRIX);
                    o.put_u32(*n);
                }
                Item::Span(n) => {
                    if !for_hash {
                        o.put_u8(op::SPAN);
                        o.put_u32(*n);
                    }
                }
                Item::TextRender(m) => {
                    o.put_u8(op::TEXT_RENDER);
                    o.put_u8(*m);
                }
                Item::Unsupported(n) => {
                    o.put_u8(op::UNSUPPORTED);
                    o.put_u32(*n);
                }
            }
        }
        o
    }

    fn enc_links(&self) -> Vec<u8> {
        let mut o = Vec::new();
        o.put_u32(self.links.len() as u32);
        for l in &self.links {
            for v in l.rect {
                o.put_i32(v);
            }
            o.put_u32(l.span);
            o.put_u8(l.kind.clone() as u8);
            o.put_u32(l.file.len() as u32);
            o.extend_from_slice(&l.file);
            o.put_u32(l.data.len() as u32);
            o.extend_from_slice(&l.data);
        }
        o
    }

    fn enc_dests(&self) -> Vec<u8> {
        let mut o = Vec::new();
        o.put_u32(self.dests.len() as u32);
        for d in &self.dests {
            o.put_u8(d.named as u8);
            o.put_u32(d.name.len() as u32);
            o.extend_from_slice(&d.name);
            o.put_u8(d.kind);
            for v in d.rect {
                o.put_i32(v);
            }
            o.put_i32(d.zoom);
        }
        o
    }

    fn enc_unsupported(&self) -> Vec<u8> {
        let mut o = Vec::new();
        o.put_u32(self.unsupported.len() as u32);
        for s in &self.unsupported {
            let b = s.as_bytes();
            let n = b.len().min(u16::MAX as usize);
            o.put_u16(n as u16);
            o.extend_from_slice(&b[..n]);
        }
        o
    }

    /// The content hash (spec §4.6): SHA-256 over what is drawn -- the box,
    /// the matrices, paths, items without spans and columns, the
    /// unsupported list, and the key of every font and image the items use
    /// (in order of first use), so that it is stable across compiles and
    /// sessions. Forms are referenced by id; a client caching a page that
    /// draws forms keys it with the forms' own hashes too.
    pub fn content_hash(
        &self,
        font_key: &dyn Fn(u16) -> [u8; 32],
        image_key: &dyn Fn(u32) -> [u8; 32],
    ) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"display-list-v3 content\0");
        h.update(&[(self.kind == StreamKind::Form) as u8]);
        let mut head = Vec::new();
        head.put_i32(self.width);
        head.put_i32(self.height);
        for v in self.pdf_box {
            head.put_f64(v);
        }
        h.update(&head);
        for data in [
            self.enc_matrices(),
            self.enc_paths(),
            self.enc_items(true),
            self.enc_unsupported(),
        ] {
            h.update(&(data.len() as u64).to_le_bytes());
            h.update(&data);
        }
        let mut fonts: Vec<u16> = Vec::new();
        let mut images: Vec<u32> = Vec::new();
        for it in &self.items {
            match it {
                Item::Glyph { font, .. } if !fonts.contains(font) => fonts.push(*font),
                Item::Image { id, .. } if !images.contains(id) => images.push(*id),
                _ => {}
            }
        }
        for f in fonts {
            h.update(&f.to_le_bytes());
            h.update(&font_key(f));
        }
        for i in images {
            h.update(&i.to_le_bytes());
            h.update(&image_key(i));
        }
        h.finish()
    }

    /// Decode a PAGE (`kind` Page) or FORM body.
    pub fn decode(kind: StreamKind, body: &[u8]) -> Result<Page, String> {
        let mut c = Cursor::new(body);
        let mut p = Page::new(kind, c.u32()?);
        p.flags = c.u32()?;
        p.width = c.i32()?;
        p.height = c.i32()?;
        for i in 0..10 {
            p.counts[i] = c.i32()?;
        }
        for i in 0..4 {
            p.pdf_box[i] = c.f64()?;
        }
        p.hash.copy_from_slice(c.take(32)?);
        let n = c.count(8)?;
        for _ in 0..n {
            let tag = c.u32()?;
            let len = c.u32()? as usize;
            let data = c.take(len)?;
            let mut d = Cursor::new(data);
            match tag {
                section::MATRICES => {
                    let n = d.count(48)?;
                    for _ in 0..n {
                        let mut m = [0.0; 6];
                        for v in &mut m {
                            *v = d.f64()?;
                        }
                        p.matrices.push(m);
                    }
                }
                section::PATHS => p.paths = dec_paths(&mut d)?,
                section::ITEMS => p.items = dec_items(&mut d)?,
                section::LINKS => {
                    let n = d.count(29)?;
                    for _ in 0..n {
                        let mut rect = [0; 4];
                        for v in &mut rect {
                            *v = d.i32()?;
                        }
                        let span = d.u32()?;
                        let kind = LinkKind::from(d.u8()?);
                        let fl = d.u32()? as usize;
                        let file = d.take(fl)?.to_vec();
                        let dl = d.u32()? as usize;
                        let data = d.take(dl)?.to_vec();
                        p.links.push(Link {
                            rect,
                            span,
                            kind,
                            file,
                            data,
                        });
                    }
                }
                section::DESTS => {
                    let n = d.count(26)?;
                    for _ in 0..n {
                        let named = d.u8()? != 0;
                        let nl = d.u32()? as usize;
                        let name = d.take(nl)?.to_vec();
                        let kind = d.u8()?;
                        let mut rect = [0; 4];
                        for v in &mut rect {
                            *v = d.i32()?;
                        }
                        let zoom = d.i32()?;
                        p.dests.push(Dest {
                            named,
                            name,
                            kind,
                            rect,
                            zoom,
                        });
                    }
                }
                section::UNSUPPORTED => {
                    let n = d.count(2)?;
                    for _ in 0..n {
                        let l = d.u16()? as usize;
                        p.unsupported
                            .push(String::from_utf8_lossy(d.take(l)?).into_owned());
                    }
                }
                _ => {} // a later minor version's section: skipped
            }
        }
        // Every reference must resolve (fail closed).
        for it in &p.items {
            match it {
                Item::Path(n) | Item::Clip(n) if *n as usize >= p.paths.len() => {
                    return Err(format!("item names path {n} of {}", p.paths.len()))
                }
                Item::Matrix(n) | Item::Image { matrix: n, .. } | Item::Form { matrix: n, .. }
                    if *n as usize > p.matrices.len() =>
                {
                    return Err(format!("item names matrix {n} of {}", p.matrices.len()))
                }
                Item::Unsupported(n) if *n as usize >= p.unsupported.len() => {
                    return Err(format!("item names unsupported entry {n}"))
                }
                _ => {}
            }
        }
        for path in &p.paths {
            if path.matrix as usize > p.matrices.len() {
                return Err(format!("path names matrix {}", path.matrix));
            }
        }
        Ok(p)
    }
}

fn dec_paths(d: &mut Cursor) -> Result<Vec<Path>, String> {
    let n = d.count(9)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let paint = d.u8()?;
        let matrix = d.u32()?;
        let stroke = if paint & paint::STROKE != 0 {
            let width = d.f64()?;
            let cap = d.u8()?;
            let join = d.u8()?;
            let miter = d.f64()?;
            let nd = d.u16()? as usize;
            let mut dash = Vec::with_capacity(nd);
            for _ in 0..nd {
                dash.push(d.f64()?);
            }
            let phase = d.f64()?;
            Some(Stroke {
                width,
                cap,
                join,
                miter,
                dash,
                phase,
            })
        } else {
            None
        };
        let ns = d.count(1)?;
        let mut segs = Vec::with_capacity(ns);
        for _ in 0..ns {
            segs.push(match d.u8()? {
                0 => Seg::Move(d.f64()?, d.f64()?),
                1 => Seg::Line(d.f64()?, d.f64()?),
                2 => Seg::Curve(d.f64()?, d.f64()?, d.f64()?, d.f64()?, d.f64()?, d.f64()?),
                3 => Seg::Close,
                o => return Err(format!("unknown path segment {o}")),
            });
        }
        out.push(Path {
            paint,
            matrix,
            stroke,
            segs,
        });
    }
    Ok(out)
}

fn dec_items(d: &mut Cursor) -> Result<Vec<Item>, String> {
    let mut out = Vec::with_capacity(d.left() / 12);
    while d.left() > 0 {
        let o = d.u8()?;
        out.push(match o {
            op::GLYPH => Item::Glyph {
                font: d.u16()?,
                code: d.u16()?,
                x: d.i32()?,
                y: d.i32()?,
                col: d.u16()?,
            },
            op::RULE => Item::Rule {
                kind: RuleKind::from(d.u8()?)?,
                x: d.i32()?,
                y: d.i32()?,
                w: d.i32()?,
                h: d.i32()?,
            },
            op::PATH => Item::Path(d.u32()?),
            op::CLIP => Item::Clip(d.u32()?),
            op::IMAGE => Item::Image {
                id: d.u32()?,
                matrix: d.u32()?,
            },
            op::FORM => Item::Form {
                id: d.u32()?,
                matrix: d.u32()?,
            },
            op::SAVE => Item::Save,
            op::RESTORE => Item::Restore,
            op::FILL_COLOR | op::STROKE_COLOR => {
                let n = d.u8()?;
                if !matches!(n, 1 | 3 | 4) {
                    return Err(format!("colour with {n} components"));
                }
                let mut v = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    v.push(d.f64()?);
                }
                if o == op::FILL_COLOR {
                    Item::FillColor(Color(v))
                } else {
                    Item::StrokeColor(Color(v))
                }
            }
            op::MATRIX => Item::Matrix(d.u32()?),
            op::SPAN => Item::Span(d.u32()?),
            op::TEXT_RENDER => Item::TextRender(d.u8()?),
            op::UNSUPPORTED => Item::Unsupported(d.u32()?),
            _ => return Err(format!("unknown item opcode {o:#04x} at byte {}", d.i - 1)),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_round_trip() {
        let mut p = Page::new(StreamKind::Page, 3);
        p.width = 100;
        p.height = 200;
        p.counts[0] = 4;
        p.pdf_box = [0.0, 0.0, 612.0, 792.0];
        p.matrices.push([9.9626, 0.0, 0.0, 9.9626, 0.0, 0.0]);
        p.paths.push(Path {
            paint: paint::STROKE | paint::FILL,
            matrix: 1,
            stroke: Some(Stroke {
                width: 0.4,
                cap: 1,
                join: 0,
                miter: 10.0,
                dash: vec![3.0, 2.0],
                phase: 0.5,
            }),
            segs: vec![
                Seg::Move(0.0, 0.0),
                Seg::Curve(1.0, 2.0, 3.0, 4.0, 5.0, 6.0),
                Seg::Close,
            ],
        });
        p.unsupported.push("sh".into());
        p.items = vec![
            Item::Span(7),
            Item::Matrix(1),
            Item::FillColor(Color(vec![0.0, 0.0, 1.0])),
            Item::Glyph {
                font: 41,
                code: 65,
                x: 5,
                y: 6,
                col: 3,
            },
            Item::Rule {
                kind: RuleKind::StrokeH,
                x: 1,
                y: 2,
                w: 3,
                h: 4,
            },
            Item::Save,
            Item::Clip(0),
            Item::Path(0),
            Item::Image { id: 1, matrix: 1 },
            Item::Form { id: 2, matrix: 0 },
            Item::Restore,
            Item::TextRender(3),
            Item::Unsupported(0),
        ];
        p.links.push(Link {
            rect: [1, 2, 3, 4],
            span: 7,
            kind: LinkKind::GotoName,
            file: vec![],
            data: b"sec.1".to_vec(),
        });
        p.dests.push(Dest {
            named: true,
            name: b"sec.1".to_vec(),
            kind: 0,
            rect: [1, 2, 0, 0],
            zoom: 0,
        });
        let k = |_: u16| [1u8; 32];
        let ik = |_: u32| [2u8; 32];
        p.hash = p.content_hash(&k, &ik);
        let q = Page::decode(StreamKind::Page, &p.encode()).unwrap();
        assert_eq!(p, q);
        // Spans and columns do not change the hash; positions do.
        let mut r = q.clone();
        r.items[0] = Item::Span(9);
        if let Item::Glyph { col, .. } = &mut r.items[3] {
            *col = 99;
        }
        assert_eq!(r.content_hash(&k, &ik), p.hash);
        if let Item::Glyph { x, .. } = &mut r.items[3] {
            *x = 6;
        }
        assert_ne!(r.content_hash(&k, &ik), p.hash);
        // Corrupt input fails closed.
        let mut bad = p.encode();
        bad.truncate(bad.len() - 3);
        assert!(Page::decode(StreamKind::Page, &bad).is_err());
    }
}
