//! Glyph positions from typst-pdf's own content stream, as the reference
//! PDF viewer computes them (DESIGN.md §15.5; display-list-v3 spec §4.2,
//! §11.2).
//!
//! Typst's frame positions miss the exported PDF's by up to 6·10⁻⁵ bp
//! (krilla writes f32), which moves pixels at 2× and 3×. So for every page
//! it sends, the host exports that page with typst-pdf (one export for all
//! the pages of a compile, untagged), reads the page's content stream and
//! computes each glyph's origin and glyph matrix in binary64 exactly as the
//! spec's §4.2 says the viewer does: an operand with k fraction digits is
//! its digits times the double nearest 10⁻ᵏ, a `TJ` adjustment and a width
//! the nearest double, products row by column, sums left to right.
//!
//! The derivation fails closed: an operator, font kind or filter it does
//! not know inside text is an error, never a guess, and the caller flags the
//! page INCOMPLETE.

use std::collections::HashMap;
use std::num::NonZeroUsize;

use typst_layout::PagedDocument;

use flashtex_display_list::page::{paint, Seg, Stroke};

use crate::pdf::{Dict, Lexer, Num, Obj, Pdf, Tok};

/// A 2-D affine matrix `[a b c d e f]` in binary64.
pub type F6 = [f64; 6];

const IDENTITY: F6 = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `m × n` (apply `m`, then `n`), each sum left to right, no fused
/// multiply-add (spec §4.2).
fn then(m: &F6, n: &F6) -> F6 {
    let [a, b, c, d, e, f] = *m;
    let [a2, b2, c2, d2, e2, f2] = *n;
    [
        a * a2 + b * c2,
        a * b2 + b * d2,
        c * a2 + d * c2,
        c * b2 + d * d2,
        e * a2 + f * c2 + e2,
        e * b2 + f * d2 + f2,
    ]
}

/// `[1 0 0 1 tx ty] × m`.
fn translate(m: &F6, tx: f64, ty: f64) -> F6 {
    let [a, b, c, d, e, f] = *m;
    [a, b, c, d, tx * a + ty * c + e, tx * b + ty * d + f]
}

/// One glyph as the viewer draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    /// The origin (X, Y) in stream space (bp, y up).
    pub origin: [f64; 2],
    /// The linear part of the text rendering matrix
    /// `[Tfs·Th 0 0 Tfs 0 Ts] × Tm × CTM` (the GLYPH's glyph matrix, §4.4).
    pub matrix: [f64; 4],
    /// What it is painted with: an index into [`PagePos::paints`].
    pub paint: u32,
    /// The CTM's scale when its linear part is a similarity (`[a b -b a]`
    /// or `[a b b -a]`): `sqrt(a·a + b·b)`, which maps a stroked glyph's
    /// line width from user space to stream space (spec §11.4); `None`
    /// under a non-uniform scale or a skew.
    pub pen_scale: Option<f64>,
}

/// The scale of a similarity's linear part `[a b c d]`, or `None`.
fn similarity_scale(m: &F6) -> Option<f64> {
    let [a, b, c, d, _, _] = *m;
    ((a == d && b == -c) || (a == -d && b == c)).then(|| (a * a + b * b).sqrt())
}

/// A colour space as the content stream selects it (`cs`/`CS`, or the
/// Device operators `g`/`rg`/`k`).
#[derive(Clone, Debug, PartialEq)]
pub enum Space {
    Gray,
    Rgb,
    Cmyk,
    /// ICCBased: its component count and the profile's bytes (decoded).
    Icc {
        n: u8,
        profile: std::sync::Arc<Vec<u8>>,
    },
    /// Separation with a Type 2 tint transform `c0 + t^e (c1 − c0)`.
    Separation {
        name: String,
        alternate: Box<Space>,
        c0: Vec<f64>,
        c1: Vec<f64>,
        e: f64,
    },
    /// A pattern (gradients, tilings): not a colour v3 draws.
    Pattern,
    /// Anything else: named, not drawn.
    Other(String),
}

/// A colour: its space and the components the PDF writes, read as §4.2
/// reads an operand.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfColor {
    pub space: Space,
    pub comps: Vec<f64>,
}

impl PdfColor {
    fn black() -> PdfColor {
        PdfColor {
            space: Space::Gray,
            comps: vec![0.0],
        }
    }
}

/// The paint state a glyph or path is drawn with (spec §11.3, §11.4).
#[derive(Clone, Debug, PartialEq)]
pub struct Paint {
    pub fill: PdfColor,
    pub stroke: PdfColor,
    /// Constant alpha (`ca`, `CA` of the selected ExtGState).
    pub fill_alpha: f64,
    pub stroke_alpha: f64,
    /// The text render mode (`Tr`).
    pub render: u8,
    /// The line state (for stroked glyphs).
    pub line: Stroke,
    /// An ExtGState key v3.3 does not draw (a soft mask, a blend mode,
    /// overprint, ...): what is painted with it is not the PDF's.
    pub unsupported_state: Option<String>,
}

/// One path the content stream paints or clips with, in its own numbers
/// (spec §4.4: "the PDF's numbers, in user space"), read as §4.2 reads them.
#[derive(Clone, Debug, PartialEq)]
pub struct PathOp {
    /// Paint bits (`page::paint`): fill, even-odd fill, stroke, clip.
    pub paint: u8,
    /// The CTM: user space to stream space.
    pub ctm: F6,
    /// The line state, when the path is stroked.
    pub stroke: Option<Stroke>,
    pub segs: Vec<Seg>,
    /// What it is painted with: an index into [`PagePos::paints`].
    pub paint_state: u32,
}

/// How an image XObject's data is encoded in the PDF.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageEncoding {
    /// Samples, Flate-compressed.
    Flate,
    /// Samples, uncompressed.
    Plain,
    /// The JPEG file's bytes (DCTDecode), passed through.
    Jpeg,
}

/// An image XObject as typst-pdf (krilla) writes it (spec §11.5), its
/// streams still encoded: decoding is left to the one compile that sends
/// the image.
#[derive(Clone, Debug, PartialEq)]
pub struct PdfImage {
    pub width: u32,
    pub height: u32,
    /// Bits per component (8 or 16).
    pub bits: u8,
    /// Components per pixel (1, 3, 4).
    pub components: u8,
    /// The ICC profile when the colour space is ICCBased.
    pub icc: Option<std::sync::Arc<Vec<u8>>>,
    /// `/Interpolate true` (Typst's `smooth` scaling).
    pub interpolate: bool,
    pub encoding: ImageEncoding,
    pub data: Vec<u8>,
    /// The soft mask: 8-bit grey samples of the same size (encoded as
    /// `mask_encoding` says).
    pub mask: Option<(ImageEncoding, Vec<u8>)>,
}

fn decode_samples(enc: ImageEncoding, data: &[u8]) -> Result<Vec<u8>, String> {
    match enc {
        ImageEncoding::Flate => crate::pdf::inflate(data),
        ImageEncoding::Plain | ImageEncoding::Jpeg => Ok(data.to_vec()),
    }
}

impl PdfImage {
    /// IMAGE_DATA part 0: the samples, rows top first, or the JPEG file.
    pub fn data_part(&self) -> Result<Vec<u8>, String> {
        let d = decode_samples(self.encoding, &self.data)?;
        if self.encoding != ImageEncoding::Jpeg {
            let row =
                (self.width as usize * self.components as usize * self.bits as usize).div_ceil(8);
            if d.len() != row * self.height as usize {
                return Err(format!(
                    "image samples: {} bytes for {}x{}x{} at {} bits",
                    d.len(),
                    self.width,
                    self.height,
                    self.components,
                    self.bits
                ));
            }
        }
        Ok(d)
    }

    /// IMAGE_DATA part 1: the soft mask's samples.
    pub fn mask_part(&self) -> Result<Option<Vec<u8>>, String> {
        let Some((enc, data)) = &self.mask else {
            return Ok(None);
        };
        let d = decode_samples(*enc, data)?;
        if d.len() != self.width as usize * self.height as usize {
            return Err(format!("soft mask: {} bytes", d.len()));
        }
        Ok(Some(d))
    }
}

/// One `Do`: an image or form XObject drawn with the CTM (for an image, the
/// matrix mapping the unit square to stream space, spec §5.2).
#[derive(Clone, Debug, PartialEq)]
pub struct ImageOp {
    pub ctm: F6,
    /// The constant fill alpha in effect (`ca`).
    pub fill_alpha: f64,
    /// A Form XObject (a PDF image, or what typst-pdf groups), not an image.
    pub form: bool,
    /// The image, or why the host cannot send it (fail closed).
    pub image: Result<std::sync::Arc<PdfImage>, String>,
}

/// One page of the export.
#[derive(Clone, Debug, PartialEq)]
pub struct PagePos {
    /// The MediaBox, as the viewer reads its numbers.
    pub media_box: [f64; 4],
    /// Every glyph the content stream shows, in painting order.
    pub glyphs: Vec<Glyph>,
    /// Every path it paints or clips with, in painting order.
    pub paths: Vec<PathOp>,
    /// The paint states glyphs and paths refer to.
    pub paints: Vec<Paint>,
    /// Every XObject the content stream draws, in painting order.
    pub images: Vec<ImageOp>,
}

/// Export `pages` (0-based, ascending) of `doc` with typst-pdf, untagged,
/// and derive each one's glyph positions. One export for all of them.
///
/// The export is of a document made of those pages alone: typst-pdf's cost
/// grows with the whole document (its outline, named destinations and
/// link targets), but a page's content stream depends only on the page's
/// frame, so the positions are the same (the positions checker compares
/// them with the whole document's export, `examples/positions_suite.rs`).
/// When that export fails (a link whose target is on another page, say),
/// the whole document is exported with `page_ranges` instead.
pub fn derive(doc: &PagedDocument, pages: &[usize]) -> Result<Vec<PagePos>, String> {
    if pages.is_empty() {
        return Ok(vec![]);
    }
    let options = typst_pdf::PdfOptions {
        tagged: false,
        ..Default::default()
    };
    let sub = PagedDocument::new(
        pages.iter().map(|&i| doc.pages()[i].clone()).collect(),
        typst::model::DocumentInfo::default(),
    );
    let bytes = match typst_pdf::pdf(&sub, &options) {
        Ok(b) => b,
        Err(_) => {
            let ranges = pages
                .iter()
                .map(|&i| {
                    let n = NonZeroUsize::new(i + 1);
                    n..=n
                })
                .collect();
            let options = typst_pdf::PdfOptions {
                page_ranges: Some(typst::layout::PageRanges::new(ranges)),
                ..options
            };
            typst_pdf::pdf(doc, &options).map_err(|errs| {
                let m: Vec<String> = errs.iter().map(|e| e.message.to_string()).collect();
                format!("typst-pdf export failed: {}", m.join("; "))
            })?
        }
    };
    let out = derive_pdf(&bytes)?;
    if out.len() != pages.len() {
        return Err(format!(
            "the export has {} pages for {} requested",
            out.len(),
            pages.len()
        ));
    }
    Ok(out)
}

/// Every page of a typst-pdf PDF.
pub fn derive_pdf(bytes: &[u8]) -> Result<Vec<PagePos>, String> {
    let pdf = Pdf::parse(bytes)?;
    let mut out = Vec::new();
    for (n, page) in pdf.pages()? {
        out.push(derive_page(&pdf, &page).map_err(|e| format!("page object {n}: {e}"))?);
    }
    Ok(out)
}

fn nums(pdf: &Pdf, o: &Obj) -> Result<Vec<Num>, String> {
    match pdf.resolve(o)? {
        Obj::Arr(a) => a
            .iter()
            .map(|x| match pdf.resolve(x)? {
                Obj::Num(n) => Ok(n),
                o => Err(format!("number expected, got {o:?}")),
            })
            .collect(),
        o => Err(format!("array expected, got {o:?}")),
    }
}

fn derive_page(pdf: &Pdf, page: &Dict) -> Result<PagePos, String> {
    let mb = pdf
        .inherited(page, "MediaBox")?
        .ok_or("page without /MediaBox")?;
    let mb = nums(pdf, &mb)?;
    if mb.len() != 4 {
        return Err("a /MediaBox of other than 4 numbers".into());
    }
    let media_box = [
        mb[0].viewer(),
        mb[1].viewer(),
        mb[2].viewer(),
        mb[3].viewer(),
    ];
    let res = match pdf.inherited(page, "Resources")? {
        Some(r) => pdf.dict(&r)?,
        None => Dict::default(),
    };
    let mut content = Vec::new();
    match page.get("Contents") {
        None => {}
        Some(Obj::Ref(r)) => match pdf.get(*r)?.obj {
            Obj::Arr(parts) => {
                for p in parts {
                    let Obj::Ref(r) = p else {
                        return Err("/Contents entry is not a reference".into());
                    };
                    content.extend_from_slice(&pdf.stream(r)?.1);
                    content.push(b'\n');
                }
            }
            _ => content = pdf.stream(*r)?.1,
        },
        Some(Obj::Arr(parts)) => {
            for p in parts {
                let Obj::Ref(r) = p else {
                    return Err("/Contents entry is not a reference".into());
                };
                content.extend_from_slice(&pdf.stream(*r)?.1);
                content.push(b'\n');
            }
        }
        Some(o) => return Err(format!("bad /Contents {o:?}")),
    }
    let mut it = Interp {
        pdf,
        fonts: HashMap::new(),
        glyphs: Vec::new(),
        paths: Vec::new(),
        paints: Vec::new(),
        profiles: HashMap::new(),
        images: Vec::new(),
        xobjects: HashMap::new(),
        depth: 0,
    };
    let mut gs = Gs::default();
    it.run(&content, &res, &mut gs)?;
    Ok(PagePos {
        media_box,
        glyphs: it.glyphs,
        paths: it.paths,
        paints: it.paints,
        images: it.images,
    })
}

/// The text state and CTM: graphics state, saved by `q`, restored by `Q`.
#[derive(Clone)]
struct Gs {
    ctm: F6,
    tc: f64,
    tw: f64,
    tz: f64,
    tl: f64,
    ts: f64,
    fs: f64,
    font: Option<u32>,
    /// The line state (`w`, `J`, `j`, `M`, `d`).
    line: Stroke,
    fill: PdfColor,
    stroke: PdfColor,
    fill_alpha: f64,
    stroke_alpha: f64,
    render: u8,
    /// ExtGState keys in effect that v3.3 does not draw (a later `gs`
    /// that sets one back to its default clears it).
    unsupported_state: std::collections::BTreeSet<String>,
}

impl Default for Gs {
    fn default() -> Gs {
        Gs {
            ctm: IDENTITY,
            tc: 0.0,
            tw: 0.0,
            tz: 100.0,
            tl: 0.0,
            ts: 0.0,
            fs: 0.0,
            font: None,
            line: Stroke {
                width: 1.0,
                cap: 0,
                join: 0,
                miter: 10.0,
                dash: vec![],
                phase: 0.0,
            },
            fill: PdfColor::black(),
            stroke: PdfColor::black(),
            fill_alpha: 1.0,
            stroke_alpha: 1.0,
            render: 0,
            unsupported_state: Default::default(),
        }
    }
}

/// A font's widths: how a string splits into codes and each code's advance
/// at size 1 (the width W the viewer uses, in thousandths of text space).
enum Widths {
    /// Type0 with Identity-H: 2-byte CIDs, `/W` and `/DW`.
    Cid { w: HashMap<u32, f64>, dw: f64 },
    /// Type3: 1-byte codes; W is `/Widths[code − FirstChar]` times the
    /// FontMatrix's `a` times 1000, the double nearest that product.
    Type3 { first: u32, w: Vec<f64> },
}

struct Interp<'p, 'a> {
    pdf: &'p Pdf<'a>,
    /// Fonts by object number (resources map names to them).
    fonts: HashMap<u32, std::rc::Rc<Widths>>,
    glyphs: Vec<Glyph>,
    paths: Vec<PathOp>,
    paints: Vec<Paint>,
    /// ICC profiles by object number, decoded once: (/N, the profile).
    profiles: HashMap<u32, (u8, std::sync::Arc<Vec<u8>>)>,
    images: Vec<ImageOp>,
    /// XObjects by object number, read once: (a form?, the image).
    xobjects: HashMap<u32, (bool, Result<std::sync::Arc<PdfImage>, String>)>,
    depth: u32,
}

/// The double nearest to the product of decimals `w × a × 1000`.
fn nearest_product(w: &Num, a: &Num) -> Option<f64> {
    let m = w.digits?.checked_mul(a.digits?)?;
    let exp = 3 - (w.k as i64 + a.k as i64);
    let neg = w.neg != a.neg;
    format!("{}{}e{}", if neg { "-" } else { "" }, m, exp)
        .parse()
        .ok()
}

impl Interp<'_, '_> {
    fn font(&mut self, r: u32) -> Result<std::rc::Rc<Widths>, String> {
        if let Some(f) = self.fonts.get(&r) {
            return Ok(f.clone());
        }
        let pdf = self.pdf;
        let d = pdf.dict(&Obj::Ref(r))?;
        let w = match d.get("Subtype").and_then(Obj::name) {
            Some(b"Type0") => {
                match d.get("Encoding").and_then(Obj::name) {
                    Some(b"Identity-H") => {}
                    e => return Err(format!("Type0 font with encoding {e:?}")),
                }
                let desc =
                    match pdf.resolve(d.get("DescendantFonts").ok_or("no /DescendantFonts")?)? {
                        Obj::Arr(a) if a.len() == 1 => pdf.dict(&a[0])?,
                        _ => return Err("bad /DescendantFonts".into()),
                    };
                let dw = match desc.get("DW") {
                    Some(o) => match pdf.resolve(o)? {
                        Obj::Num(n) => n.nearest,
                        _ => return Err("bad /DW".into()),
                    },
                    None => 1000.0,
                };
                let mut w = HashMap::new();
                if let Some(wo) = desc.get("W") {
                    let Obj::Arr(a) = pdf.resolve(wo)? else {
                        return Err("bad /W".into());
                    };
                    let int = |o: &Obj| -> Result<u32, String> {
                        o.num()
                            .and_then(Num::as_i64)
                            .and_then(|v| u32::try_from(v).ok())
                            .ok_or_else(|| format!("bad /W entry {o:?}"))
                    };
                    let mut i = 0;
                    while i < a.len() {
                        let first = int(&a[i])?;
                        match a.get(i + 1) {
                            Some(Obj::Arr(ws)) => {
                                for (j, x) in ws.iter().enumerate() {
                                    let v = x.num().ok_or("bad /W width")?;
                                    w.insert(first + j as u32, v.nearest);
                                }
                                i += 2;
                            }
                            Some(Obj::Num(_)) => {
                                let last = int(&a[i + 1])?;
                                let v = a
                                    .get(i + 2)
                                    .and_then(Obj::num)
                                    .ok_or("bad /W range width")?;
                                for c in first..=last {
                                    w.insert(c, v.nearest);
                                }
                                i += 3;
                            }
                            o => return Err(format!("bad /W entry {o:?}")),
                        }
                    }
                }
                Widths::Cid { w, dw }
            }
            Some(b"Type3") => {
                let fm = nums(pdf, d.get("FontMatrix").ok_or("Type3 without /FontMatrix")?)?;
                if fm.len() != 6 {
                    return Err("bad /FontMatrix".into());
                }
                let first = d
                    .get("FirstChar")
                    .and_then(Obj::num)
                    .and_then(Num::as_i64)
                    .ok_or("Type3 without /FirstChar")? as u32;
                let widths = nums(pdf, d.get("Widths").ok_or("Type3 without /Widths")?)?;
                let w = widths
                    .iter()
                    .map(|x| nearest_product(x, &fm[0]).ok_or("Type3 width out of range"))
                    .collect::<Result<Vec<f64>, _>>()?;
                Widths::Type3 { first, w }
            }
            t => return Err(format!("font of subtype {t:?}")),
        };
        let w = std::rc::Rc::new(w);
        self.fonts.insert(r, w.clone());
        Ok(w)
    }

    /// XObject `r`: whether it is a form, and the image it is (or why the
    /// host cannot send it).
    fn xobject(&mut self, r: u32) -> (bool, Result<std::sync::Arc<PdfImage>, String>) {
        if let Some(x) = self.xobjects.get(&r) {
            return x.clone();
        }
        let x = match self.pdf.get(r) {
            Err(e) => (false, Err(e)),
            Ok(ind) => match &ind.obj {
                Obj::Dict(d) => match d.get("Subtype").and_then(Obj::name) {
                    Some(b"Form") => (true, Err("a Form XObject".into())),
                    Some(b"Image") => (
                        false,
                        self.image(d, ind.stream.unwrap_or_default())
                            .map(std::sync::Arc::new),
                    ),
                    s => (false, Err(format!("an XObject of subtype {s:?}"))),
                },
                o => (false, Err(format!("XObject {r} is {o:?}"))),
            },
        };
        self.xobjects.insert(r, x.clone());
        x
    }

    /// An image XObject's dictionary as typst-pdf writes it; anything else
    /// (a /Decode array, an image mask, predictors, a colour key mask) is
    /// refused, never approximated.
    fn image(&mut self, d: &Dict, raw: &[u8]) -> Result<PdfImage, String> {
        let pdf = self.pdf;
        let int = |k: &str| -> Result<u32, String> {
            match d.get(k).map(|o| pdf.resolve(o)).transpose()? {
                Some(Obj::Num(n)) => n
                    .as_i64()
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or_else(|| format!("image /{k} {n:?}")),
                o => Err(format!("image /{k} {o:?}")),
            }
        };
        for k in ["Decode", "DecodeParms", "ImageMask", "Mask", "Matte"] {
            if let Some(o) = d.get(k) {
                if !(k == "ImageMask" && *o == Obj::Bool(false)) {
                    return Err(format!("an image with /{k}"));
                }
            }
        }
        let encoding = |d: &Dict| -> Result<ImageEncoding, String> {
            match d.get("Filter").map(|f| pdf.resolve(f)).transpose()? {
                None => Ok(ImageEncoding::Plain),
                Some(Obj::Name(f)) if f == b"FlateDecode" => Ok(ImageEncoding::Flate),
                Some(Obj::Name(f)) if f == b"DCTDecode" => Ok(ImageEncoding::Jpeg),
                Some(f) => Err(format!("an image with filter {f:?}")),
            }
        };
        let (width, height) = (int("Width")?, int("Height")?);
        let bits = int("BitsPerComponent")?;
        if bits != 8 && bits != 16 {
            return Err(format!("an image of {bits} bits per component"));
        }
        let enc = encoding(d)?;
        let cs = d
            .get("ColorSpace")
            .ok_or("an image without /ColorSpace")?
            .clone();
        let (components, icc) = match self.space(&cs, 0)? {
            Space::Gray => (1, None),
            Space::Rgb => (3, None),
            Space::Cmyk => (4, None),
            Space::Icc { n, profile } => (n, Some(profile)),
            s => return Err(format!("an image in colour space {s:?}")),
        };
        let interpolate = matches!(d.get("Interpolate"), Some(Obj::Bool(true)));
        let mask = match d.get("SMask") {
            None => None,
            Some(Obj::Ref(m)) => {
                let ind = pdf.get(*m)?;
                let Obj::Dict(md) = &ind.obj else {
                    return Err("an /SMask that is not a stream".into());
                };
                for k in ["Decode", "DecodeParms", "Matte", "SMask", "Mask"] {
                    if md.get(k).is_some() {
                        return Err(format!("a soft mask with /{k}"));
                    }
                }
                let mint = |k: &str| md.get(k).and_then(Obj::num).and_then(Num::as_i64);
                if mint("Width") != Some(width as i64)
                    || mint("Height") != Some(height as i64)
                    || mint("BitsPerComponent") != Some(8)
                    || md.get("ColorSpace").and_then(Obj::name) != Some(b"DeviceGray")
                {
                    return Err("a soft mask other than 8-bit grey of the image's size".into());
                }
                let menc = encoding(md)?;
                if menc == ImageEncoding::Jpeg {
                    return Err("a JPEG soft mask".into());
                }
                Some((menc, ind.stream.unwrap_or_default().to_vec()))
            }
            Some(o) => return Err(format!("an /SMask {o:?}")),
        };
        Ok(PdfImage {
            width,
            height,
            bits: bits as u8,
            components,
            icc,
            interpolate,
            encoding: enc,
            data: raw.to_vec(),
            mask,
        })
    }

    /// The index of `gs`'s paint state (consecutive equal states share one).
    fn paint(&mut self, gs: &Gs) -> u32 {
        let p = Paint {
            fill: gs.fill.clone(),
            stroke: gs.stroke.clone(),
            fill_alpha: gs.fill_alpha,
            stroke_alpha: gs.stroke_alpha,
            render: gs.render,
            line: gs.line.clone(),
            unsupported_state: (!gs.unsupported_state.is_empty()).then(|| {
                gs.unsupported_state
                    .iter()
                    .map(|k| format!("ExtGState /{k}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            }),
        };
        if self.paints.last() != Some(&p) {
            self.paints.push(p);
        }
        self.paints.len() as u32 - 1
    }

    /// A colour space object (a name or an array), as `cs` selects it.
    fn space(&mut self, o: &Obj, depth: u32) -> Result<Space, String> {
        if depth > 4 {
            return Err("colour spaces nested too deep".into());
        }
        let pdf = self.pdf;
        let o = pdf.resolve(o)?;
        Ok(match &o {
            Obj::Name(n) => match n.as_slice() {
                b"DeviceGray" | b"G" => Space::Gray,
                b"DeviceRGB" | b"RGB" => Space::Rgb,
                b"DeviceCMYK" | b"CMYK" => Space::Cmyk,
                b"Pattern" => Space::Pattern,
                other => Space::Other(String::from_utf8_lossy(other).into_owned()),
            },
            Obj::Arr(a) => match a.first().and_then(Obj::name) {
                Some(b"ICCBased") => {
                    let Some(Obj::Ref(r)) = a.get(1) else {
                        return Err("ICCBased without a stream".into());
                    };
                    // Each ICC stream is decoded once per page.
                    let (n, profile) = match self.profiles.get(r) {
                        Some(p) => p.clone(),
                        None => {
                            let (d, data) = pdf.stream(*r)?;
                            let n = d.get("N").and_then(Obj::num).and_then(Num::as_i64);
                            let n = match n {
                                Some(n @ (1 | 3 | 4)) => n as u8,
                                n => return Err(format!("ICCBased with /N {n:?}")),
                            };
                            let p = (n, std::sync::Arc::new(data));
                            self.profiles.insert(*r, p.clone());
                            p
                        }
                    };
                    Space::Icc { n, profile }
                }
                Some(b"Separation") => {
                    let name = match a.get(1) {
                        Some(Obj::Name(n)) => String::from_utf8_lossy(n).into_owned(),
                        _ => return Err("Separation without a name".into()),
                    };
                    let alternate = self.space(
                        a.get(2).ok_or("Separation without an alternate")?,
                        depth + 1,
                    )?;
                    let f = pdf.dict(a.get(3).ok_or("Separation without a function")?)?;
                    if f.get("FunctionType")
                        .and_then(Obj::num)
                        .and_then(Num::as_i64)
                        != Some(2)
                    {
                        return Err("a Separation tint transform other than Type 2".into());
                    }
                    // PDF 32000-1 §7.10.3: C0 defaults to [0.0], C1 to
                    // [1.0]; the Domain must be [0 1] (a tint).
                    let get = |k: &str, default: f64| -> Result<Vec<f64>, String> {
                        match f.get(k) {
                            Some(o) => Ok(nums(pdf, o)?.iter().map(Num::viewer).collect()),
                            None => Ok(vec![default]),
                        }
                    };
                    let (c0, c1) = (get("C0", 0.0)?, get("C1", 1.0)?);
                    let domain: Vec<f64> = match f.get("Domain") {
                        Some(o) => nums(pdf, o)?.iter().map(Num::viewer).collect(),
                        None => return Err("a tint transform without /Domain".into()),
                    };
                    if domain != [0.0, 1.0] {
                        return Err(format!("a tint transform with /Domain {domain:?}"));
                    }
                    let n_alt = match &alternate {
                        Space::Gray => 1,
                        Space::Rgb => 3,
                        Space::Cmyk => 4,
                        Space::Icc { n, .. } => *n as usize,
                        a => return Err(format!("a Separation with alternate {a:?}")),
                    };
                    if c0.len() != n_alt || c1.len() != n_alt {
                        return Err(format!(
                            "a Separation whose C0/C1 ({}, {}) do not fit its alternate ({n_alt})",
                            c0.len(),
                            c1.len()
                        ));
                    }
                    let e = f
                        .get("N")
                        .and_then(Obj::num)
                        .map(Num::viewer)
                        .unwrap_or(1.0);
                    Space::Separation {
                        name,
                        alternate: Box::new(alternate),
                        c0,
                        c1,
                        e,
                    }
                }
                Some(other) => Space::Other(String::from_utf8_lossy(other).into_owned()),
                None => return Err("an empty colour space array".into()),
            },
            o => return Err(format!("colour space {o:?}")),
        })
    }

    fn run(&mut self, content: &[u8], res: &Dict, gs: &mut Gs) -> Result<(), String> {
        self.depth += 1;
        if self.depth > 16 {
            return Err("Form XObjects nested too deep".into());
        }
        let pdf = self.pdf;
        let font_res = match res.get("Font") {
            Some(f) => pdf.dict(f)?,
            None => Dict::default(),
        };
        let cs_res = match res.get("ColorSpace") {
            Some(f) => pdf.dict(f)?,
            None => Dict::default(),
        };
        let gs_res = match res.get("ExtGState") {
            Some(f) => pdf.dict(f)?,
            None => Dict::default(),
        };
        let xobj_res = match res.get("XObject") {
            Some(f) => pdf.dict(f)?,
            None => Dict::default(),
        };
        let mut stack: Vec<Gs> = Vec::new();
        let (mut tm, mut tlm) = (IDENTITY, IDENTITY);
        let mut in_text = false;
        // The path under construction, its current point, a pending clip.
        let mut segs: Vec<Seg> = Vec::new();
        let mut cur = (0.0, 0.0);
        let mut start = (0.0, 0.0);
        let mut clip: u8 = 0;
        let mut args: Vec<Tok> = Vec::new();
        let mut lx = Lexer::new(content);
        while let Some(t) = lx.token()? {
            let op = match t {
                Tok::Kw(op) => op,
                Tok::ArrOpen => {
                    // An array operand (TJ): collect it whole.
                    let mut v = Vec::new();
                    loop {
                        match lx.token()?.ok_or("unterminated array")? {
                            Tok::ArrClose => break,
                            t @ (Tok::Num(_) | Tok::Str(_)) => v.push(t),
                            t => return Err(format!("unexpected {t:?} in a TJ array")),
                        }
                    }
                    args.push(Tok::Kw(b"[".to_vec()));
                    args.extend(v);
                    args.push(Tok::Kw(b"]".to_vec()));
                    continue;
                }
                Tok::DictOpen => {
                    // An inline property list (BDC): skip it whole.
                    let mut depth = 1;
                    while depth > 0 {
                        match lx.token()?.ok_or("unterminated dictionary")? {
                            Tok::DictOpen => depth += 1,
                            Tok::DictClose => depth -= 1,
                            _ => {}
                        }
                    }
                    args.push(Tok::Kw(b"<<>>".to_vec()));
                    continue;
                }
                t => {
                    args.push(t);
                    continue;
                }
            };
            let n = |i: usize| -> Result<Num, String> {
                match args.get(i) {
                    Some(Tok::Num(v)) => Ok(*v),
                    a => Err(format!(
                        "{}: operand {i} is {a:?}",
                        String::from_utf8_lossy(&op)
                    )),
                }
            };
            let v = |i: usize| -> Result<f64, String> { Ok(n(i)?.viewer()) };
            match op.as_slice() {
                b"q" => stack.push(gs.clone()),
                b"Q" => *gs = stack.pop().ok_or("Q without q")?,
                b"cm" => {
                    let m = [v(0)?, v(1)?, v(2)?, v(3)?, v(4)?, v(5)?];
                    gs.ctm = then(&m, &gs.ctm);
                }
                b"BT" => {
                    in_text = true;
                    tm = IDENTITY;
                    tlm = IDENTITY;
                }
                b"ET" => in_text = false,
                b"Tf" => {
                    let Some(Tok::Name(name)) = args.first() else {
                        return Err("Tf without a font name".into());
                    };
                    let r = match font_res.0.iter().find(|(k, _)| k == name) {
                        Some((_, Obj::Ref(r))) => *r,
                        _ => {
                            return Err(format!(
                                "font /{} is not a resource",
                                String::from_utf8_lossy(name)
                            ))
                        }
                    };
                    self.font(r)?;
                    gs.font = Some(r);
                    gs.fs = v(1)?;
                }
                b"Tc" => gs.tc = v(0)?,
                b"Tr" => gs.render = n(0)?.as_i64().unwrap_or(0) as u8,
                // Colours (spec §11.3), in the PDF's numbers.
                b"cs" | b"CS" => {
                    let Some(Tok::Name(name)) = args.first() else {
                        return Err("cs without a name".into());
                    };
                    let space = match name.as_slice() {
                        b"DeviceGray" => Space::Gray,
                        b"DeviceRGB" => Space::Rgb,
                        b"DeviceCMYK" => Space::Cmyk,
                        b"Pattern" => Space::Pattern,
                        _ => match cs_res.0.iter().find(|(k, _)| k == name) {
                            Some((_, o)) => {
                                let o = o.clone();
                                self.space(&o, 0)?
                            }
                            None => {
                                return Err(format!(
                                    "colour space /{} is not a resource",
                                    String::from_utf8_lossy(name)
                                ))
                            }
                        },
                    };
                    // A new space starts at its initial colour (zeros; a
                    // Separation's tint 1).
                    let comps = match &space {
                        Space::Gray | Space::Separation { .. } => {
                            vec![if matches!(space, Space::Separation { .. }) {
                                1.0
                            } else {
                                0.0
                            }]
                        }
                        Space::Rgb => vec![0.0; 3],
                        Space::Cmyk => vec![0.0, 0.0, 0.0, 1.0],
                        Space::Icc { n, .. } => vec![0.0; *n as usize],
                        _ => vec![],
                    };
                    let c = PdfColor { space, comps };
                    if op == b"cs" {
                        gs.fill = c;
                    } else {
                        gs.stroke = c;
                    }
                }
                b"sc" | b"scn" | b"SC" | b"SCN" => {
                    let comps: Vec<f64> = args
                        .iter()
                        .filter_map(|t| match t {
                            Tok::Num(x) => Some(x.viewer()),
                            _ => None,
                        })
                        .collect();
                    let c = if op[0] == b's' {
                        &mut gs.fill
                    } else {
                        &mut gs.stroke
                    };
                    c.comps = comps;
                }
                b"g" | b"G" | b"rg" | b"RG" | b"k" | b"K" => {
                    let space = match op[0].to_ascii_lowercase() {
                        b'g' => Space::Gray,
                        b'r' => Space::Rgb,
                        _ => Space::Cmyk,
                    };
                    let k = match space {
                        Space::Gray => 1,
                        Space::Rgb => 3,
                        _ => 4,
                    };
                    let comps = (0..k).map(v).collect::<Result<Vec<f64>, String>>()?;
                    let c = PdfColor { space, comps };
                    if op[0].is_ascii_lowercase() {
                        gs.fill = c;
                    } else {
                        gs.stroke = c;
                    }
                }
                b"gs" => {
                    let Some(Tok::Name(name)) = args.first() else {
                        return Err("gs without a name".into());
                    };
                    let d = match gs_res.0.iter().find(|(k, _)| k == name) {
                        Some((_, o)) => pdf.dict(o)?,
                        None => {
                            return Err(format!(
                                "ExtGState /{} is not a resource",
                                String::from_utf8_lossy(name)
                            ))
                        }
                    };
                    if let Some(a) = d.get("ca").and_then(Obj::num) {
                        gs.fill_alpha = a.viewer();
                    }
                    if let Some(a) = d.get("CA").and_then(Obj::num) {
                        gs.stroke_alpha = a.viewer();
                    }
                    if let Some(w) = d.get("LW").and_then(Obj::num) {
                        gs.line.width = w.viewer();
                    }
                    // Every other key changes what is drawn in a way v3.3
                    // does not carry (spec §11.3): a soft mask, a blend
                    // mode, overprint, a transfer function, ... What is
                    // painted under it makes the page INCOMPLETE.
                    for (k, v) in &d.0 {
                        let ok = match k.as_slice() {
                            b"Type" | b"ca" | b"CA" | b"LW" => true,
                            b"SMask" => pdf.resolve(v)? == Obj::Name(b"None".to_vec()),
                            b"BM" => matches!(
                                pdf.resolve(v)?,
                                Obj::Name(n) if n == b"Normal" || n == b"Compatible"
                            ),
                            b"AIS" => pdf.resolve(v)? == Obj::Bool(false),
                            _ => false,
                        };
                        let k = String::from_utf8_lossy(k).into_owned();
                        if ok {
                            gs.unsupported_state.remove(&k);
                        } else {
                            gs.unsupported_state.insert(k);
                        }
                    }
                }
                b"Tw" => gs.tw = v(0)?,
                b"Tz" => gs.tz = v(0)?,
                b"TL" => gs.tl = v(0)?,
                b"Ts" => gs.ts = v(0)?,
                b"Tm" => {
                    tm = [v(0)?, v(1)?, v(2)?, v(3)?, v(4)?, v(5)?];
                    tlm = tm;
                }
                b"Td" | b"TD" => {
                    let (tx, ty) = (v(0)?, v(1)?);
                    if op == b"TD" {
                        gs.tl = -ty;
                    }
                    tlm = translate(&tlm, tx, ty);
                    tm = tlm;
                }
                b"T*" => {
                    tlm = translate(&tlm, 0.0, -gs.tl);
                    tm = tlm;
                }
                b"Tj" | b"TJ" | b"'" | b"\"" => {
                    if !in_text {
                        return Err("text shown outside BT/ET".into());
                    }
                    // The font in effect: `Tf` is graphics state, so after
                    // a `Q` it may be an earlier one.
                    let w = match gs.font {
                        Some(r) => self.font(r)?,
                        None => return Err("text without a font".into()),
                    };
                    if op == b"'" || op == b"\"" {
                        if op == b"\"" {
                            gs.tw = v(0)?;
                            gs.tc = v(1)?;
                        }
                        tlm = translate(&tlm, 0.0, -gs.tl);
                        tm = tlm;
                    }
                    let show: Vec<Tok> = match op.as_slice() {
                        b"TJ" => match (args.first(), args.last()) {
                            (Some(Tok::Kw(o)), Some(Tok::Kw(c))) if o == b"[" && c == b"]" => {
                                args[1..args.len() - 1].to_vec()
                            }
                            _ => return Err("TJ without an array".into()),
                        },
                        _ => match args.last() {
                            Some(t @ Tok::Str(_)) => vec![t.clone()],
                            _ => return Err("a text operator without a string".into()),
                        },
                    };
                    for e in show {
                        match e {
                            Tok::Str(s) => self.show(&s, &w, gs, &mut tm)?,
                            Tok::Num(adj) => {
                                let tx = -adj.nearest / 1000.0 * gs.fs;
                                let tx = tx * gs.tz / 100.0;
                                tm = translate(&tm, tx, 0.0);
                            }
                            _ => unreachable!(),
                        }
                    }
                }
                // An XObject is a frame image (raster, SVG or PDF): typst-pdf
                // draws each with one `Do`, and the glyphs an SVG or PDF image
                // shows are the image's, not the page's text runs. Page text is
                // never put in a Form XObject (checked over Typst's test suite
                // by examples/positions_suite.rs: a run drawn in one would
                // leave the walk short, and the page INCOMPLETE).
                // Images are recorded for the IMAGE items (spec §5.2,
                // §11.5): the CTM maps an image's unit square to stream space.
                b"Do" => {
                    let Some(Tok::Name(name)) = args.first() else {
                        return Err("Do without a name".into());
                    };
                    let (form, image) = match xobj_res.0.iter().find(|(k, _)| k == name) {
                        Some((_, Obj::Ref(r))) => {
                            let r = *r;
                            self.xobject(r)
                        }
                        _ => (
                            false,
                            Err(format!(
                                "XObject /{} is not a resource",
                                String::from_utf8_lossy(name)
                            )),
                        ),
                    };
                    self.images.push(ImageOp {
                        ctm: gs.ctm,
                        fill_alpha: gs.fill_alpha,
                        form,
                        image,
                    });
                }
                b"BI" => return Err("inline image".into()),
                // Paths (spec §4.4), in the PDF's numbers.
                b"m" => {
                    cur = (v(0)?, v(1)?);
                    start = cur;
                    segs.push(Seg::Move(cur.0, cur.1));
                }
                b"l" => {
                    cur = (v(0)?, v(1)?);
                    segs.push(Seg::Line(cur.0, cur.1));
                }
                b"c" => {
                    let (a, b, c, d, e, f) = (v(0)?, v(1)?, v(2)?, v(3)?, v(4)?, v(5)?);
                    segs.push(Seg::Curve(a, b, c, d, e, f));
                    cur = (e, f);
                }
                b"v" => {
                    let (c, d, e, f) = (v(0)?, v(1)?, v(2)?, v(3)?);
                    segs.push(Seg::Curve(cur.0, cur.1, c, d, e, f));
                    cur = (e, f);
                }
                b"y" => {
                    let (a, b, e, f) = (v(0)?, v(1)?, v(2)?, v(3)?);
                    segs.push(Seg::Curve(a, b, e, f, e, f));
                    cur = (e, f);
                }
                b"h" => {
                    segs.push(Seg::Close);
                    cur = start;
                }
                b"re" => {
                    let (x, y, w, h) = (v(0)?, v(1)?, v(2)?, v(3)?);
                    segs.extend([
                        Seg::Move(x, y),
                        Seg::Line(x + w, y),
                        Seg::Line(x + w, y + h),
                        Seg::Line(x, y + h),
                        Seg::Close,
                    ]);
                    cur = (x, y);
                    start = cur;
                }
                b"W" => clip = paint::CLIP,
                b"W*" => clip = paint::CLIP_EVEN_ODD,
                b"f" | b"F" | b"f*" | b"S" | b"s" | b"B" | b"B*" | b"b" | b"b*" | b"n" => {
                    if matches!(op.as_slice(), b"s" | b"b" | b"b*") {
                        segs.push(Seg::Close);
                    }
                    let bits = match op.as_slice() {
                        b"f" | b"F" => paint::FILL,
                        b"f*" => paint::FILL_EVEN_ODD,
                        b"S" | b"s" => paint::STROKE,
                        b"B" | b"b" => paint::FILL | paint::STROKE,
                        b"B*" | b"b*" => paint::FILL_EVEN_ODD | paint::STROKE,
                        _ => 0,
                    } | clip;
                    if bits != 0 {
                        let paint_state = self.paint(gs);
                        self.paths.push(PathOp {
                            paint: bits,
                            ctm: gs.ctm,
                            stroke: (bits & paint::STROKE != 0).then(|| gs.line.clone()),
                            segs: std::mem::take(&mut segs),
                            paint_state,
                        });
                    }
                    segs.clear();
                    clip = 0;
                }
                b"w" => gs.line.width = v(0)?,
                b"J" => gs.line.cap = n(0)?.as_i64().unwrap_or(0) as u8,
                b"j" => gs.line.join = n(0)?.as_i64().unwrap_or(0) as u8,
                b"M" => gs.line.miter = v(0)?,
                b"d" => {
                    // `[a b ...] phase d`: the array was collected as [ ... ].
                    let mut dash = Vec::new();
                    let mut phase = 0.0;
                    let mut inside = false;
                    for a in &args {
                        match a {
                            Tok::Kw(k) if k == b"[" => inside = true,
                            Tok::Kw(k) if k == b"]" => inside = false,
                            Tok::Num(x) if inside => dash.push(x.viewer()),
                            Tok::Num(x) => phase = x.viewer(),
                            _ => {}
                        }
                    }
                    gs.line.dash = dash;
                    gs.line.phase = phase;
                }
                _ => {}
            }
            args.clear();
        }
        self.depth -= 1;
        Ok(())
    }

    fn show(&mut self, s: &[u8], w: &Widths, gs: &Gs, tm: &mut F6) -> Result<(), String> {
        let codes: Vec<(u32, bool)> = match w {
            Widths::Cid { .. } => {
                if !s.len().is_multiple_of(2) {
                    return Err("a CID string of odd length".into());
                }
                s.chunks(2)
                    .map(|c| (u16::from_be_bytes([c[0], c[1]]) as u32, false))
                    .collect()
            }
            Widths::Type3 { .. } => s.iter().map(|&c| (c as u32, c == 32)).collect(),
        };
        for (code, single_space) in codes {
            let trm = then(tm, &gs.ctm);
            let th = gs.tz / 100.0;
            let origin = [gs.ts * trm[2] + trm[4], gs.ts * trm[3] + trm[5]];
            let matrix = [
                gs.fs * th * trm[0],
                gs.fs * th * trm[1],
                gs.fs * trm[2],
                gs.fs * trm[3],
            ];
            let paint = self.paint(gs);
            self.glyphs.push(Glyph {
                origin,
                matrix,
                paint,
                pen_scale: similarity_scale(&gs.ctm),
            });
            let wv = match w {
                Widths::Cid { w, dw } => *w.get(&code).unwrap_or(dw),
                Widths::Type3 { first, w } => *code
                    .checked_sub(*first)
                    .and_then(|i| w.get(i as usize))
                    .ok_or_else(|| format!("Type3 code {code} has no width"))?,
            };
            let mut tx = wv / 1000.0 * gs.fs + gs.tc;
            if single_space {
                tx += gs.tw;
            }
            let tx = tx * gs.tz / 100.0;
            *tm = translate(tm, tx, 0.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PDF with the given objects (1-based) and a cross-reference table.
    fn pdf(objs: &[&str]) -> Vec<u8> {
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offs = vec![];
        for (i, o) in objs.iter().enumerate() {
            offs.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
        }
        let x = out.len();
        out.extend_from_slice(
            format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).as_bytes(),
        );
        for o in offs {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(
            format!(
                "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{x}\n%%EOF\n",
                objs.len() + 1
            )
            .as_bytes(),
        );
        out
    }

    fn doc(content: &str, fonts: &str, extra: &[&str]) -> Vec<u8> {
        doc_res(content, &format!("/Font<<{fonts}>>"), extra)
    }

    fn doc_res(content: &str, res: &str, extra: &[&str]) -> Vec<u8> {
        let mut objs = vec![
            "<</Type/Catalog/Pages 2 0 R>>".to_string(),
            "<</Type/Pages/Kids[3 0 R]/Count 1>>".to_string(),
            format!(
                "<</Type/Page/Parent 2 0 R/MediaBox[0 0 595.2756 841.8898]/Contents 4 0 R\
                 /Resources<<{res}>>>>"
            ),
            format!(
                "<</Length {}>>\nstream\n{content}\nendstream",
                content.len()
            ),
        ];
        objs.extend(extra.iter().map(|s| s.to_string()));
        let refs: Vec<&str> = objs.iter().map(|s| s.as_str()).collect();
        pdf(&refs)
    }

    /// The text operators typst-pdf does not write today (TD, T*, `"`, Tz,
    /// Ts, Tc, Tw, Type 3 widths) follow spec §4.2 too.
    #[test]
    fn text_operators_follow_the_viewer() {
        let content = "q 1 0 0 1 10.5 20.25 cm BT /F1 12 Tf 2 Tz 0.5 Tc 3 Tw 1.5 Ts \
                       0 14 TD (A ) Tj T* [(B) -250 (C)] TJ ET Q \
                       BT /F0 10 Tf 1 0 0 1 100 200 Tm 7 TL 1 2 <0044> \" ET";
        let mut widths = vec!["0"; 36];
        widths[0] = "250";
        widths[33] = "600";
        widths[34] = "500";
        widths[35] = "400";
        let t3 = format!(
            "<</Type/Font/Subtype/Type3/FirstChar 32/Widths[{}]/FontMatrix[0.001 0 0 0.001 0 0]>>",
            widths.join(" ")
        );
        let bytes = doc(
            content,
            "/F0 5 0 R/F1 6 0 R",
            &[
                "<</Type/Font/Subtype/Type0/Encoding/Identity-H/DescendantFonts[7 0 R]>>",
                &t3,
                "<</Type/Font/Subtype/CIDFontType0/DW 1000/W[68[555.5]]>>",
            ],
        );
        let pages = derive_pdf(&bytes).unwrap();
        let p = &pages[0];
        assert_eq!(p.media_box, [0.0, 0.0, 5952756.0 * 1e-4, 8418898.0 * 1e-4]);
        let g = &p.glyphs;
        assert_eq!(g.len(), 5);
        let ctm = [1.0, 0.0, 0.0, 1.0, 105.0 * 0.1, 2025.0 * 0.01];
        let o = |tm: &F6| {
            let t = then(tm, &ctm);
            [1.5 * t[2] + t[4], 1.5 * t[3] + t[5]]
        };
        // TD: TL = -14, the line moves up 14; Ts lifts the origin.
        let tm0 = translate(&IDENTITY, 0.0, 14.0);
        assert_eq!(g[0].origin, o(&tm0));
        assert_eq!(g[0].matrix, [12.0 * 2.0 / 100.0, 0.0, 0.0, 12.0]);
        // 'A' (code 65, width 600 × 0.001 × 1000), Tc, then ' ' (code 32: Tw too).
        let adv = |w: f64, tw: f64| (w / 1000.0 * 12.0 + 0.5 + tw) * 2.0 / 100.0;
        let tm1 = translate(&tm0, adv(600.0, 0.0), 0.0);
        assert_eq!(g[1].origin, o(&tm1));
        // T* moves down TL from the line start: B, an adjustment, C.
        let tm2 = translate(&tm0, 0.0, 14.0);
        assert_eq!(g[2].origin, o(&tm2));
        let tm3 = translate(&tm2, adv(500.0, 0.0), 0.0);
        let tm3 = translate(&tm3, 250.0 / 1000.0 * 12.0 * 2.0 / 100.0, 0.0);
        assert_eq!(g[3].origin, o(&tm3));
        // `"` sets Tw and Tc and moves to the next line (TL 7): CID 68.
        let tm = translate(&[1.0, 0.0, 0.0, 1.0, 100.0, 200.0], 0.0, -7.0);
        assert_eq!(g[4].origin, [tm[4], tm[5]]);
        assert_eq!(g[4].matrix, [10.0, 0.0, 0.0, 10.0]);
    }

    /// An ExtGState key v3.3 does not draw (a soft mask, a blend mode)
    /// marks what is painted under it; setting it back clears it.
    #[test]
    fn extgstate_keys_not_drawn_mark_the_paint() {
        let bytes = doc_res(
            "/G0 gs 0 0 10 10 re f q /G1 gs 0 0 5 5 re f /G2 gs 1 1 2 2 re f Q \
             /G2 gs 2 2 1 1 re f /G3 gs 3 3 1 1 re f",
            "/ExtGState<</G0 5 0 R/G1 6 0 R/G2 7 0 R/G3 8 0 R>>",
            &[
                "<</Type/ExtGState/ca 0.5/BM/Normal/SMask/None>>",
                "<</Type/ExtGState/SMask<</S/Luminosity/G 9 0 R>>>>",
                "<</Type/ExtGState/BM/Multiply>>",
                "<</Type/ExtGState/BM/Normal>>",
            ],
        );
        let p = &derive_pdf(&bytes).unwrap()[0];
        let state = |i: usize| {
            p.paints[p.paths[i].paint_state as usize]
                .unsupported_state
                .clone()
        };
        assert_eq!(state(0), None);
        assert_eq!(state(1).as_deref(), Some("ExtGState /SMask"));
        assert_eq!(state(2).as_deref(), Some("ExtGState /BM, ExtGState /SMask"));
        // Q restored the state before /G1; /G2 sets Multiply, /G3 Normal.
        assert_eq!(state(3).as_deref(), Some("ExtGState /BM"));
        assert_eq!(state(4), None);
    }

    /// A Separation's tint transform: C0 and C1 default to [0] and [1];
    /// a Domain other than [0 1], or C0/C1 that do not fit the alternate,
    /// are refused.
    #[test]
    fn separation_defaults_and_domain() {
        let page = |f: &str| {
            derive_pdf(&doc_res(
                "/CS0 cs 0.5 scn 0 0 1 1 re f",
                "/ColorSpace<</CS0[/Separation/Spot/DeviceGray 5 0 R]>>",
                &[f],
            ))
        };
        let p = &page("<</FunctionType 2/Domain[0 1]/N 1>>").unwrap()[0];
        match &p.paints[p.paths[0].paint_state as usize].fill.space {
            Space::Separation { c0, c1, e, .. } => {
                assert_eq!(
                    (c0.as_slice(), c1.as_slice(), *e),
                    (&[0.0][..], &[1.0][..], 1.0)
                )
            }
            s => panic!("{s:?}"),
        }
        assert!(page("<</FunctionType 2/Domain[0 2]/N 1>>").is_err());
        assert!(page("<</FunctionType 2/N 1>>").is_err());
        assert!(page("<</FunctionType 2/Domain[0 1]/C1[1 0 0]/N 1>>").is_err());
    }

    /// A stroked glyph's pen scale is the CTM's when it is a similarity.
    #[test]
    fn pen_scale_only_for_similarities() {
        let m = |a: f64, b: f64, c: f64, d: f64| similarity_scale(&[a, b, c, d, 0.0, 0.0]);
        assert_eq!(m(2.0, 0.0, 0.0, 2.0), Some(2.0));
        assert_eq!(m(0.6, 0.8, -0.8, 0.6), Some(1.0));
        assert_eq!(m(-1.0, 0.0, 0.0, 1.0), Some(1.0));
        assert_eq!(m(1.5, 0.0, 0.0, 1.0), None);
        assert_eq!(m(1.0, 0.0, 0.3, 1.0), None);
    }

    #[test]
    fn unknown_fonts_and_inline_images_fail_closed() {
        let t1 = ["<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>"];
        assert!(derive_pdf(&doc("BT /F0 1 Tf (a) Tj ET", "/F0 5 0 R", &t1)).is_err());
        assert!(derive_pdf(&doc("BI /W 1 /H 1 ID x EI", "/F0 5 0 R", &t1)).is_err());
        assert!(derive_pdf(&doc("(a) Tj", "/F0 5 0 R", &t1)).is_err());
        let ok = derive_pdf(&doc("q Q", "/F0 5 0 R", &t1)).unwrap();
        assert!(ok[0].glyphs.is_empty());
    }
}
