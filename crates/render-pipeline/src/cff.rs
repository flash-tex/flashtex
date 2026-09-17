//! Minimal CFF (Compact Font Format) reader: enough of the Type 2 charstring
//! interpreter to compute per-glyph bounding boxes (control-point bounds,
//! which contain the curve) and advance widths. No rasterization.
//!
//! font-engine d0c64cf parses `OTTO` faces and exposes the raw `CFF ` table
//! (`TrueTypeFace::cff_table`) but derives no glyph bounds for it. Glyph
//! heights and depths drive TeX line boxes, math shifts, radical rules and
//! hit rectangles, so the pipeline computes them here. Requested font-engine
//! API: `Face::glyph_bounds(gid)` (see docs/proposals/rendering-abi.md);
//! delete this file when it lands.

use flashtex_font_engine::Error;

pub(crate) fn u16_at(b: &[u8], at: usize) -> Result<u16, Error> {
    match b.get(at..at + 2) {
        Some(s) => Ok(u16::from_be_bytes([s[0], s[1]])),
        None => Err(Error::Malformed(format!("read of 2 bytes at {at} past end ({})", b.len()))),
    }
}

pub(crate) fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    match b.get(at..at + 4) {
        Some(s) => Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]])),
        None => Err(Error::Malformed(format!("read of 4 bytes at {at} past end ({})", b.len()))),
    }
}

/// Font-unit bounding box `[x_min, y_min, x_max, y_max]`; `None` for an empty
/// glyph (no marking contours).
pub type BBox = Option<[i32; 4]>;

#[derive(Debug, Clone)]
struct Index {
    /// Absolute byte ranges of every item.
    items: Vec<(usize, usize)>,
    /// Absolute offset just past the INDEX.
    end: usize,
}

fn read_index(b: &[u8], at: usize) -> Result<Index, Error> {
    let count = usize::from(u16_at(b, at)?);
    if count == 0 {
        return Ok(Index {
            items: Vec::new(),
            end: at + 2,
        });
    }
    let off_size = usize::from(*b.get(at + 2).ok_or_else(|| malformed("INDEX offSize"))?);
    if !(1..=4).contains(&off_size) {
        return Err(malformed("INDEX offSize"));
    }
    let offsets_at = at + 3;
    let read_off = |i: usize| -> Result<usize, Error> {
        let p = offsets_at + i * off_size;
        let s = b.get(p..p + off_size).ok_or_else(|| malformed("INDEX offset"))?;
        Ok(s.iter().fold(0usize, |acc, x| (acc << 8) | usize::from(*x)))
    };
    let data_start = offsets_at + (count + 1) * off_size - 1;
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let s = data_start + read_off(i)?;
        let e = data_start + read_off(i + 1)?;
        if s > e || e > b.len() {
            return Err(malformed("INDEX item range"));
        }
        items.push((s, e));
    }
    let end = data_start + read_off(count)?;
    Ok(Index { items, end })
}

fn malformed(what: &str) -> Error {
    Error::Malformed(format!("CFF: {what}"))
}

/// Parses a DICT into (operator, operands) pairs. Operators >= 0x0c00 are the
/// escaped two-byte operators.
fn parse_dict(b: &[u8]) -> Result<Vec<(u16, Vec<f64>)>, Error> {
    let mut out = Vec::new();
    let mut operands: Vec<f64> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let b0 = b[i];
        match b0 {
            0..=21 => {
                let op = if b0 == 12 {
                    i += 1;
                    0x0c00 | u16::from(*b.get(i).ok_or_else(|| malformed("dict escape"))?)
                } else {
                    u16::from(b0)
                };
                out.push((op, std::mem::take(&mut operands)));
                i += 1;
            }
            28 => {
                operands.push(f64::from(i16_at_raw(b, i + 1)?));
                i += 3;
            }
            29 => {
                operands.push(f64::from(i32_at_raw(b, i + 1)?));
                i += 5;
            }
            30 => {
                // Real number: nibbles until 0xf.
                let mut s = String::new();
                i += 1;
                'outer: loop {
                    let byte = *b.get(i).ok_or_else(|| malformed("real"))?;
                    i += 1;
                    for nib in [byte >> 4, byte & 0xf] {
                        match nib {
                            0..=9 => s.push((b'0' + nib) as char),
                            0xa => s.push('.'),
                            0xb => s.push('E'),
                            0xc => s.push_str("E-"),
                            0xe => s.push('-'),
                            0xf => break 'outer,
                            _ => {}
                        }
                    }
                }
                operands.push(s.parse::<f64>().unwrap_or(0.0));
            }
            32..=246 => {
                operands.push(f64::from(i32::from(b0) - 139));
                i += 1;
            }
            247..=250 => {
                let b1 = i32::from(*b.get(i + 1).ok_or_else(|| malformed("dict int"))?);
                operands.push(f64::from((i32::from(b0) - 247) * 256 + b1 + 108));
                i += 2;
            }
            251..=254 => {
                let b1 = i32::from(*b.get(i + 1).ok_or_else(|| malformed("dict int"))?);
                operands.push(f64::from(-(i32::from(b0) - 251) * 256 - b1 - 108));
                i += 2;
            }
            _ => return Err(malformed("dict byte")),
        }
    }
    Ok(out)
}

fn i16_at_raw(b: &[u8], at: usize) -> Result<i16, Error> {
    u16_at(b, at).map(|v| v as i16)
}

fn i32_at_raw(b: &[u8], at: usize) -> Result<i32, Error> {
    u32_at(b, at).map(|v| v as i32)
}

#[derive(Debug, Clone, Default)]
struct Private {
    subrs: Vec<(usize, usize)>,
    nominal_width_x: f64,
    default_width_x: f64,
}

/// A parsed CFF program (single font, CID-keyed or not) sufficient for
/// bounding boxes.
#[derive(Debug, Clone)]
pub struct Cff {
    char_strings: Vec<(usize, usize)>,
    global_subrs: Vec<(usize, usize)>,
    /// One Private DICT for plain fonts; one per FD for CID-keyed fonts.
    privates: Vec<Private>,
    /// Glyph -> FD index for CID-keyed fonts (empty otherwise).
    fd_select: Vec<u8>,
    pub is_cid: bool,
}

impl Cff {
    pub fn parse(b: &[u8]) -> Result<Cff, Error> {
        if b.len() < 4 || b[0] != 1 {
            return Err(malformed("header"));
        }
        let hdr_size = usize::from(b[2]);
        let name = read_index(b, hdr_size)?;
        let top = read_index(b, name.end)?;
        let _strings = read_index(b, top.end)?;
        let global = read_index(b, _strings.end)?;
        let (ts, te) = *top.items.first().ok_or_else(|| malformed("no Top DICT"))?;
        let top_dict = parse_dict(&b[ts..te])?;
        let mut char_strings_off = None;
        let mut private_ref = None;
        let mut fdarray_off = None;
        let mut fdselect_off = None;
        let mut is_cid = false;
        let mut charstring_type = 2.0;
        for (op, args) in &top_dict {
            match *op {
                17 => char_strings_off = args.first().map(|v| *v as usize),
                18 => {
                    if args.len() == 2 {
                        private_ref = Some((args[0] as usize, args[1] as usize));
                    }
                }
                0x0c06 => charstring_type = args.first().copied().unwrap_or(2.0),
                0x0c1e => is_cid = true,
                0x0c24 => fdarray_off = args.first().map(|v| *v as usize),
                0x0c25 => fdselect_off = args.first().map(|v| *v as usize),
                _ => {}
            }
        }
        if charstring_type != 2.0 {
            return Err(Error::Unsupported("CFF CharstringType 1".into()));
        }
        let cs_off = char_strings_off.ok_or_else(|| malformed("no CharStrings"))?;
        let char_strings = read_index(b, cs_off)?.items;
        let mut privates = Vec::new();
        let mut fd_select = Vec::new();
        if is_cid {
            let fda = fdarray_off.ok_or_else(|| malformed("CID font without FDArray"))?;
            let fds = read_index(b, fda)?;
            for (s, e) in fds.items {
                let d = parse_dict(&b[s..e])?;
                let pr = d
                    .iter()
                    .find(|(op, _)| *op == 18)
                    .and_then(|(_, a)| (a.len() == 2).then(|| (a[0] as usize, a[1] as usize)));
                privates.push(match pr {
                    Some((size, off)) => read_private(b, off, size)?,
                    None => Private::default(),
                });
            }
            let n = char_strings.len();
            fd_select = vec![0u8; n];
            if let Some(fso) = fdselect_off {
                match *b.get(fso).ok_or_else(|| malformed("FDSelect"))? {
                    0 => {
                        for (g, slot) in fd_select.iter_mut().enumerate() {
                            *slot = *b.get(fso + 1 + g).ok_or_else(|| malformed("FDSelect 0"))?;
                        }
                    }
                    3 => {
                        let n_ranges = usize::from(u16_at(b, fso + 1)?);
                        let sentinel = usize::from(u16_at(b, fso + 3 + n_ranges * 3)?);
                        for r in 0..n_ranges {
                            let first = usize::from(u16_at(b, fso + 3 + r * 3)?);
                            let fd = *b.get(fso + 5 + r * 3).ok_or_else(|| malformed("FDSelect 3"))?;
                            let next = if r + 1 < n_ranges {
                                usize::from(u16_at(b, fso + 3 + (r + 1) * 3)?)
                            } else {
                                sentinel
                            };
                            for slot in fd_select.iter_mut().take(next.min(n)).skip(first) {
                                *slot = fd;
                            }
                        }
                    }
                    f => return Err(Error::Unsupported(format!("FDSelect format {f}"))),
                }
            }
        } else {
            privates.push(match private_ref {
                Some((size, off)) => read_private(b, off, size)?,
                None => Private::default(),
            });
        }
        Ok(Cff {
            char_strings,
            global_subrs: global.items,
            privates,
            fd_select,
            is_cid,
        })
    }

    pub fn num_glyphs(&self) -> usize {
        self.char_strings.len()
    }

    /// Bounding box of `gid` in font units, plus the charstring's advance
    /// width if it declares one (`None` = defaultWidthX).
    pub fn glyph_bbox(&self, data: &[u8], gid: u16) -> Result<(Option<[f64; 4]>, Option<f64>), Error> {
        let (s, e) = *self
            .char_strings
            .get(usize::from(gid))
            .ok_or(Error::GlyphOutOfRange(gid))?;
        let private = if self.is_cid {
            let fd = usize::from(self.fd_select.get(usize::from(gid)).copied().unwrap_or(0));
            self.privates.get(fd).cloned().unwrap_or_default()
        } else {
            self.privates[0].clone()
        };
        let mut st = T2State {
            data,
            global: &self.global_subrs,
            local: &private.subrs,
            stack: Vec::new(),
            x: 0.0,
            y: 0.0,
            n_stems: 0,
            width: None,
            width_parsed: false,
            nominal_width: private.nominal_width_x,
            bbox: None,
            trans: Vec::new(),
            depth: 0,
            open: false,
        };
        st.run(&data[s..e])?;
        let width = st.width.or(Some(private.default_width_x));
        Ok((st.bbox, width))
    }
}

fn read_private(b: &[u8], off: usize, size: usize) -> Result<Private, Error> {
    let d = parse_dict(b.get(off..off + size).ok_or_else(|| malformed("Private range"))?)?;
    let mut p = Private::default();
    for (op, args) in d {
        match op {
            19 => {
                if let Some(rel) = args.first() {
                    p.subrs = read_index(b, off + *rel as usize)?.items;
                }
            }
            20 => p.default_width_x = args.first().copied().unwrap_or(0.0),
            21 => p.nominal_width_x = args.first().copied().unwrap_or(0.0),
            _ => {}
        }
    }
    Ok(p)
}

fn bias(n: usize) -> i32 {
    if n < 1240 {
        107
    } else if n < 33900 {
        1131
    } else {
        32768
    }
}

struct T2State<'a> {
    data: &'a [u8],
    global: &'a [(usize, usize)],
    local: &'a [(usize, usize)],
    stack: Vec<f64>,
    x: f64,
    y: f64,
    n_stems: usize,
    width: Option<f64>,
    width_parsed: bool,
    nominal_width: f64,
    bbox: Option<[f64; 4]>,
    trans: Vec<f64>,
    depth: usize,
    open: bool,
}

impl T2State<'_> {
    fn point(&mut self, x: f64, y: f64) {
        self.bbox = Some(match self.bbox {
            None => [x, y, x, y],
            Some([x0, y0, x1, y1]) => [x0.min(x), y0.min(y), x1.max(x), y1.max(y)],
        });
    }

    fn moveto(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
        self.open = true;
        self.point(self.x, self.y);
    }

    fn lineto(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
        self.point(self.x, self.y);
    }

    fn curveto(&mut self, dx1: f64, dy1: f64, dx2: f64, dy2: f64, dx3: f64, dy3: f64) {
        let c1x = self.x + dx1;
        let c1y = self.y + dy1;
        let c2x = c1x + dx2;
        let c2y = c1y + dy2;
        self.x = c2x + dx3;
        self.y = c2y + dy3;
        self.point(c1x, c1y);
        self.point(c2x, c2y);
        self.point(self.x, self.y);
    }

    /// Consumes the optional leading width argument of a Type 2 charstring
    /// when `extra` says the first stack-clearing operator carries one
    /// (Type 2 §3.1: exactly one operator may take one more argument than
    /// its arity, and that extra argument is `nominalWidthX + w`).
    ///
    /// This must run *on that operator*, before its own arguments are read:
    /// leaving the width on the stack shifts every argument by one, so the
    /// glyph's first `rmoveto` lands somewhere else entirely and the whole
    /// outline — and the bounding box taken from it — is translated. Only
    /// glyphs that declare a width are affected, which is why it shows up
    /// on a handful of glyphs (Latin Modern Math's radical variants among
    /// them) rather than everywhere.
    fn width_if(&mut self, extra: bool) {
        if !self.width_parsed {
            self.width_parsed = true;
            if extra {
                let w = self.stack.remove(0);
                self.width = Some(self.nominal_width + w);
            }
        }
    }

    fn run(&mut self, cs: &[u8]) -> Result<(), Error> {
        if self.depth > 10 {
            return Err(malformed("subroutine nesting"));
        }
        let mut i = 0;
        while i < cs.len() {
            let b0 = cs[i];
            i += 1;
            match b0 {
                32..=246 => self.stack.push(f64::from(i32::from(b0) - 139)),
                247..=250 => {
                    let b1 = i32::from(*cs.get(i).ok_or_else(|| malformed("t2 int"))?);
                    i += 1;
                    self.stack.push(f64::from((i32::from(b0) - 247) * 256 + b1 + 108));
                }
                251..=254 => {
                    let b1 = i32::from(*cs.get(i).ok_or_else(|| malformed("t2 int"))?);
                    i += 1;
                    self.stack.push(f64::from(-(i32::from(b0) - 251) * 256 - b1 - 108));
                }
                28 => {
                    self.stack.push(f64::from(i16_at_raw(cs, i)?));
                    i += 2;
                }
                255 => {
                    self.stack.push(f64::from(i32_at_raw(cs, i)?) / 65536.0);
                    i += 4;
                }
                1 | 3 | 18 | 23 => {
                    // hstem vstem hstemhm vstemhm
                    self.width_if(self.stack.len() % 2 == 1);
                    self.n_stems += self.stack.len() / 2;
                    self.stack.clear();
                }
                19 | 20 => {
                    // hintmask cntrmask (may carry implicit vstems)
                    self.width_if(self.stack.len() % 2 == 1);
                    self.n_stems += self.stack.len() / 2;
                    self.stack.clear();
                    i += self.n_stems.div_ceil(8);
                }
                21 => {
                    self.width_if(self.stack.len() > 2);
                    let (dx, dy) = (self.arg(0), self.arg(1));
                    self.moveto(dx, dy);
                    self.stack.clear();
                }
                22 => {
                    self.width_if(self.stack.len() > 1);
                    let dx = self.arg(0);
                    self.moveto(dx, 0.0);
                    self.stack.clear();
                }
                4 => {
                    self.width_if(self.stack.len() > 1);
                    let dy = self.arg(0);
                    self.moveto(0.0, dy);
                    self.stack.clear();
                }
                5 => {
                    let args = std::mem::take(&mut self.stack);
                    for p in args.chunks_exact(2) {
                        self.lineto(p[0], p[1]);
                    }
                }
                6 | 7 => {
                    // hlineto / vlineto alternate
                    let args = std::mem::take(&mut self.stack);
                    let mut horizontal = b0 == 6;
                    for v in args {
                        if horizontal {
                            self.lineto(v, 0.0);
                        } else {
                            self.lineto(0.0, v);
                        }
                        horizontal = !horizontal;
                    }
                }
                8 => {
                    let args = std::mem::take(&mut self.stack);
                    for c in args.chunks_exact(6) {
                        self.curveto(c[0], c[1], c[2], c[3], c[4], c[5]);
                    }
                }
                24 => {
                    // rcurveline
                    let args = std::mem::take(&mut self.stack);
                    let n_curves = (args.len().saturating_sub(2)) / 6;
                    for c in args[..n_curves * 6].chunks_exact(6) {
                        self.curveto(c[0], c[1], c[2], c[3], c[4], c[5]);
                    }
                    let rest = &args[n_curves * 6..];
                    if rest.len() >= 2 {
                        self.lineto(rest[0], rest[1]);
                    }
                }
                25 => {
                    // rlinecurve
                    let args = std::mem::take(&mut self.stack);
                    let n_lines = (args.len().saturating_sub(6)) / 2;
                    for p in args[..n_lines * 2].chunks_exact(2) {
                        self.lineto(p[0], p[1]);
                    }
                    let rest = &args[n_lines * 2..];
                    if rest.len() >= 6 {
                        self.curveto(rest[0], rest[1], rest[2], rest[3], rest[4], rest[5]);
                    }
                }
                26 | 27 => {
                    // vvcurveto / hhcurveto
                    let args = std::mem::take(&mut self.stack);
                    let mut j = 0;
                    let mut d1 = 0.0;
                    if args.len() % 4 == 1 {
                        d1 = args[0];
                        j = 1;
                    }
                    while j + 4 <= args.len() {
                        if b0 == 26 {
                            self.curveto(d1, args[j], args[j + 1], args[j + 2], 0.0, args[j + 3]);
                        } else {
                            self.curveto(args[j], d1, args[j + 1], args[j + 2], args[j + 3], 0.0);
                        }
                        d1 = 0.0;
                        j += 4;
                    }
                }
                30 | 31 => {
                    // vhcurveto / hvcurveto
                    let args = std::mem::take(&mut self.stack);
                    let mut horizontal = b0 == 31;
                    let mut j = 0;
                    while j + 4 <= args.len() {
                        let last = j + 8 > args.len();
                        let dlast = if last && j + 5 == args.len() {
                            args[j + 4]
                        } else {
                            0.0
                        };
                        if horizontal {
                            self.curveto(args[j], 0.0, args[j + 1], args[j + 2], dlast, args[j + 3]);
                        } else {
                            self.curveto(0.0, args[j], args[j + 1], args[j + 2], args[j + 3], dlast);
                        }
                        horizontal = !horizontal;
                        j += 4;
                    }
                }
                10 => {
                    let idx = self.stack.pop().ok_or_else(|| malformed("callsubr"))? as i32
                        + bias(self.local.len());
                    let (s, e) = *self
                        .local
                        .get(usize::try_from(idx).map_err(|_| malformed("subr index"))?)
                        .ok_or_else(|| malformed("subr index"))?;
                    self.depth += 1;
                    let data = self.data;
                    self.run(&data[s..e])?;
                    self.depth -= 1;
                }
                29 => {
                    let idx = self.stack.pop().ok_or_else(|| malformed("callgsubr"))? as i32
                        + bias(self.global.len());
                    let (s, e) = *self
                        .global
                        .get(usize::try_from(idx).map_err(|_| malformed("gsubr index"))?)
                        .ok_or_else(|| malformed("gsubr index"))?;
                    self.depth += 1;
                    let data = self.data;
                    self.run(&data[s..e])?;
                    self.depth -= 1;
                }
                11 => return Ok(()), // return
                14 => {
                    // endchar (width; optional seac-style accent composition ignored
                    // for bounds beyond the base outline already traced).
                    self.width_if(self.stack.len() == 1 || self.stack.len() == 5);
                    self.stack.clear();
                    return Ok(());
                }
                12 => {
                    let b1 = *cs.get(i).ok_or_else(|| malformed("escape"))?;
                    i += 1;
                    match b1 {
                        35 => {
                            // flex
                            let a = std::mem::take(&mut self.stack);
                            if a.len() >= 13 {
                                self.curveto(a[0], a[1], a[2], a[3], a[4], a[5]);
                                self.curveto(a[6], a[7], a[8], a[9], a[10], a[11]);
                            }
                        }
                        34 => {
                            // hflex
                            let a = std::mem::take(&mut self.stack);
                            if a.len() >= 7 {
                                let y0 = self.y;
                                self.curveto(a[0], 0.0, a[1], a[2], a[3], 0.0);
                                self.curveto(a[4], 0.0, a[5], y0 - self.y, a[6], 0.0);
                            }
                        }
                        36 => {
                            // hflex1
                            let a = std::mem::take(&mut self.stack);
                            if a.len() >= 9 {
                                let y0 = self.y;
                                self.curveto(a[0], a[1], a[2], a[3], a[4], 0.0);
                                self.curveto(a[5], 0.0, a[6], a[7], a[8], y0 - self.y - a[7]);
                            }
                        }
                        37 => {
                            // flex1
                            let a = std::mem::take(&mut self.stack);
                            if a.len() >= 11 {
                                let (sx, sy) = (self.x, self.y);
                                let dx: f64 = a[0] + a[2] + a[4] + a[6] + a[8];
                                let dy: f64 = a[1] + a[3] + a[5] + a[7] + a[9];
                                self.curveto(a[0], a[1], a[2], a[3], a[4], a[5]);
                                // Last point: only one coordinate given.
                                let c1x = self.x + a[6];
                                let c1y = self.y + a[7];
                                let c2x = c1x + a[8];
                                let c2y = c1y + a[9];
                                let (ex, ey) = if dx.abs() > dy.abs() {
                                    (c2x + a[10], sy)
                                } else {
                                    (sx, c2y + a[10])
                                };
                                self.point(c1x, c1y);
                                self.point(c2x, c2y);
                                self.x = ex;
                                self.y = ey;
                                self.point(ex, ey);
                            }
                        }
                        3 | 4 | 5 | 9 | 10 | 11 | 12 | 14 | 15 | 18 | 21 | 22 | 23 | 24 | 26
                        | 27 | 28 | 29 | 30 => {
                            // Arithmetic/logic operators: not used by LM; clear.
                            self.stack.clear();
                        }
                        _ => self.stack.clear(),
                    }
                }
                _ => {
                    self.stack.clear();
                }
            }
            if self.trans.len() > 48 {
                self.trans.clear();
            }
        }
        Ok(())
    }

    fn arg(&self, i: usize) -> f64 {
        self.stack.get(i).copied().unwrap_or(0.0)
    }
}

/// Integer bbox from the float one (outward rounding).
pub fn round_bbox(b: Option<[f64; 4]>) -> BBox {
    b.map(|[x0, y0, x1, y1]| [x0.floor() as i32, y0.floor() as i32, x1.ceil() as i32, y1.ceil() as i32])
}
