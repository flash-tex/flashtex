//! PDF operators that specials write into the page (`pdf:code`,
//! `pdf:literal`, `pdf:content`), read as a PDF viewer reads them into
//! display-list items: the graphics state (`q`, `Q`, `cm`, line state,
//! colours, constant alpha through `gs`), paths and their painting and
//! clipping. What the display list cannot express (shadings, patterns,
//! inline images, text shown by the operators, other `ExtGState` keys,
//! XObjects not made by the specials) is an UNSUPPORTED item whose
//! operators the PDF writer copies ([`super::doc::Raw`]).

use super::doc::{Col, Raw};
use super::page::{Builder, Metrics};
use super::pdfobj::{is_delim, is_white, Obj, Parser};
use flashtex_display_list::page::{paint, Item, Path, Seg};
use flashtex_engine::displaylist::fixed::{Fx, Mat};

/// A token of a content stream.
#[derive(Clone, Debug)]
enum Tok {
    Obj(Obj),
    Op(Vec<u8>),
}

fn tokens(s: &[u8]) -> Vec<(usize, usize, Tok)> {
    let mut out = Vec::new();
    let mut p = Parser::new(s);
    loop {
        p.skip_white();
        let st = p.i;
        if st >= s.len() {
            break;
        }
        let c = s[st];
        if c == b'[' || c == b'(' || c == b'<' || c == b'/' || c == b'@' {
            match p.object() {
                Some(o) => out.push((st, p.i, Tok::Obj(o))),
                None => {
                    p.i = st + 1;
                    out.push((st, p.i, Tok::Op(vec![c])));
                }
            }
            continue;
        }
        if c == b']' || c == b'>' || c == b')' || c == b'{' || c == b'}' {
            p.i += 1;
            out.push((st, p.i, Tok::Op(vec![c])));
            continue;
        }
        let mut e = st;
        while e < s.len() && !is_white(s[e]) && !is_delim(s[e]) {
            e += 1;
        }
        let w = &s[st..e];
        p.i = e;
        let t = if super::pdfobj::is_number(w) {
            Tok::Obj(Obj::Num(w.to_vec()))
        } else {
            match w {
                b"true" => Tok::Obj(Obj::Bool(true)),
                b"false" => Tok::Obj(Obj::Bool(false)),
                b"null" => Tok::Obj(Obj::Null),
                _ => Tok::Op(w.to_vec()),
            }
        };
        out.push((st, e, t));
    }
    out
}

fn num(o: &Obj) -> Option<f64> {
    o.as_num()
}

impl<M: Metrics> Builder<'_, M> {
    /// Interpret PDF operators `s` at this point of the stream.
    pub fn literal(&mut self, s: &[u8]) {
        let toks = tokens(s);
        let mut ops: Vec<Obj> = Vec::new();
        let mut ops_start = 0usize;
        let mut i = 0;
        while i < toks.len() {
            let (st, en, t) = &toks[i];
            match t {
                Tok::Obj(o) => {
                    if ops.is_empty() {
                        ops_start = *st;
                    }
                    ops.push(o.clone());
                }
                Tok::Op(op) => {
                    let start = if ops.is_empty() { *st } else { ops_start };
                    // A text object or an inline image runs to its end.
                    if op == b"BT" || op == b"BI" {
                        let end_op: &[u8] = if op == b"BT" { b"ET" } else { b"EI" };
                        let mut j = i + 1;
                        while j < toks.len() && !matches!(&toks[j].2, Tok::Op(o) if o == end_op) {
                            j += 1;
                        }
                        let end = toks.get(j).map_or(s.len(), |t| t.1);
                        let what = if op == b"BT" {
                            "text shown by PDF operators"
                        } else {
                            "an inline image"
                        };
                        self.raw(what, &s[start..end], true);
                        ops.clear();
                        i = j + 1;
                        continue;
                    }
                    self.operator(op, &ops, &s[start..*en]);
                    ops.clear();
                }
            }
            i += 1;
        }
    }

    /// An UNSUPPORTED item for operators `ops` (written as they are).
    fn raw(&mut self, what: &str, ops: &[u8], paints: bool) {
        let ctm = self.cur_ref().gs.ctm.to_f64();
        self.cur().unsupported(
            what.to_string(),
            Some(Raw {
                ops: ops.to_vec(),
                ctm,
                paints,
            }),
        );
    }

    fn operator(&mut self, op: &[u8], a: &[Obj], text: &[u8]) {
        let n = |i: usize| a.get(i).and_then(num);
        let pt = |i: usize| Some((n(i)?, n(i + 1)?));
        match op {
            b"q" => self.cur().save(),
            b"Q" => {
                self.cur().restore();
            }
            b"cm" => {
                if a.len() == 6 {
                    let m: Option<Vec<Fx>> = a
                        .iter()
                        .map(|o| match o {
                            Obj::Num(t) => Fx::parse(t),
                            _ => None,
                        })
                        .collect();
                    if let Some(m) = m {
                        let m = Mat([m[0], m[1], m[2], m[3], m[4], m[5]]);
                        let s = self.cur();
                        s.gs.ctm = m.then(&s.gs.ctm);
                    }
                }
            }
            b"w" => {
                if let Some(v) = n(0) {
                    self.cur().gs.line_width = v;
                }
            }
            b"J" => {
                if let Some(v) = n(0) {
                    self.cur().gs.cap = v as u8;
                }
            }
            b"j" => {
                if let Some(v) = n(0) {
                    self.cur().gs.join = v as u8;
                }
            }
            b"M" => {
                if let Some(v) = n(0) {
                    self.cur().gs.miter = v;
                }
            }
            b"d" => {
                if let (Some(Obj::Array(d)), Some(ph)) = (a.first(), n(1)) {
                    let d: Vec<f64> = d.iter().filter_map(num).collect();
                    let s = self.cur();
                    s.gs.dash = d;
                    s.gs.phase = ph;
                }
            }
            b"m" => {
                if let Some((x, y)) = pt(0) {
                    self.cur().path.push(Seg::Move(x, y));
                }
            }
            b"l" => {
                if let Some((x, y)) = pt(0) {
                    self.cur().path.push(Seg::Line(x, y));
                }
            }
            b"c" => {
                if let (Some((x1, y1)), Some((x2, y2)), Some((x3, y3))) = (pt(0), pt(2), pt(4)) {
                    self.cur().path.push(Seg::Curve(x1, y1, x2, y2, x3, y3));
                }
            }
            b"v" => {
                if let (Some((x2, y2)), Some((x3, y3))) = (pt(0), pt(2)) {
                    let (x1, y1) = current_point(&self.cur_ref().path);
                    self.cur().path.push(Seg::Curve(x1, y1, x2, y2, x3, y3));
                }
            }
            b"y" => {
                if let (Some((x1, y1)), Some((x3, y3))) = (pt(0), pt(2)) {
                    self.cur().path.push(Seg::Curve(x1, y1, x3, y3, x3, y3));
                }
            }
            b"h" => self.cur().path.push(Seg::Close),
            b"re" => {
                if let (Some((x, y)), Some((w, h))) = (pt(0), pt(2)) {
                    self.cur().path.extend(super::page::rect_segs(x, y, w, h));
                }
            }
            b"S" => self.paint(paint::STROKE, false),
            b"s" => self.paint(paint::STROKE, true),
            b"f" | b"F" => self.paint(paint::FILL, false),
            b"f*" => self.paint(paint::FILL_EVEN_ODD, false),
            b"B" => self.paint(paint::FILL | paint::STROKE, false),
            b"B*" => self.paint(paint::FILL_EVEN_ODD | paint::STROKE, false),
            b"b" => self.paint(paint::FILL | paint::STROKE, true),
            b"b*" => self.paint(paint::FILL_EVEN_ODD | paint::STROKE, true),
            b"n" => self.paint(0, false),
            b"W" => self.cur().pending_clip = Some(paint::CLIP),
            b"W*" => self.cur().pending_clip = Some(paint::CLIP_EVEN_ODD),
            b"g" | b"G" | b"rg" | b"RG" | b"k" | b"K" => {
                let v: Option<Vec<f64>> = a.iter().map(num).collect();
                let want = match op {
                    b"g" | b"G" => 1,
                    b"rg" | b"RG" => 3,
                    _ => 4,
                };
                if let Some(v) = v.filter(|v| v.len() == want) {
                    let c = Col(v);
                    if op[0].is_ascii_lowercase() {
                        self.set_colors(Some(c), None);
                    } else {
                        self.set_colors(None, Some(c));
                    }
                }
            }
            b"cs" | b"CS" => {
                let fill = op == b"cs";
                let device = matches!(
                    a.first().and_then(Obj::as_name),
                    Some(b"DeviceGray" | b"DeviceRGB" | b"DeviceCMYK")
                );
                if device {
                    let c = match a.first().and_then(Obj::as_name) {
                        Some(b"DeviceGray") => Col(vec![0.0]),
                        Some(b"DeviceRGB") => Col(vec![0.0; 3]),
                        _ => Col(vec![0.0, 0.0, 0.0, 1.0]),
                    };
                    if fill {
                        self.set_colors(Some(c), None);
                    } else {
                        self.set_colors(None, Some(c));
                    }
                } else {
                    let s = self.cur();
                    if fill {
                        s.gs.odd_fill = true;
                    } else {
                        s.gs.odd_stroke = true;
                    }
                    self.raw("a colour space other than the Device spaces", text, false);
                }
            }
            b"sc" | b"scn" | b"SC" | b"SCN" => {
                let fill = op[0] == b's';
                let odd = if fill {
                    self.cur_ref().gs.odd_fill
                } else {
                    self.cur_ref().gs.odd_stroke
                };
                let v: Option<Vec<f64>> = a.iter().map(num).collect();
                match v {
                    Some(v) if !odd && matches!(v.len(), 1 | 3 | 4) => {
                        let c = Col(v);
                        if fill {
                            self.set_colors(Some(c), None);
                        } else {
                            self.set_colors(None, Some(c));
                        }
                    }
                    _ => {
                        let s = self.cur();
                        if fill {
                            s.gs.odd_fill = true;
                        } else {
                            s.gs.odd_stroke = true;
                        }
                        self.raw("a pattern or separation colour", text, false);
                    }
                }
            }
            b"gs" => self.ext_gstate(a.first(), text),
            b"Do" => {
                let name = a.first().and_then(Obj::as_name).map(<[u8]>::to_vec);
                self.raw(
                    &format!(
                        "XObject {} drawn by PDF operators",
                        name.map(|n| String::from_utf8_lossy(&n).into_owned())
                            .unwrap_or_default()
                    ),
                    text,
                    true,
                );
            }
            b"sh" => self.raw("a shading (sh)", text, true),
            b"BMC" | b"BDC" | b"EMC" | b"MP" | b"DP" | b"ri" | b"i" | b"d0" | b"d1" | b"BX"
            | b"EX" => {}
            b"Tc" | b"Tw" | b"Tz" | b"TL" | b"Tf" | b"Tr" | b"Ts" => {
                // text state outside a text object: kept for later text
                self.raw("text state set by PDF operators", text, false);
            }
            _ => {
                let what = format!("PDF operator `{}'", String::from_utf8_lossy(op));
                self.raw(&what, text, true);
            }
        }
    }

    /// Paint the current path (`bits`: fill/stroke), closing it first for
    /// `close`, and clip with it when a `W`/`W*` came before.
    fn paint(&mut self, bits: u8, close: bool) {
        let s = self.cur();
        let mut segs = std::mem::take(&mut s.path);
        if close {
            segs.push(Seg::Close);
        }
        let clip = s.pending_clip.take();
        if segs.is_empty() {
            return;
        }
        let odd = (bits & (paint::FILL | paint::FILL_EVEN_ODD) != 0 && s.gs.odd_fill)
            || (bits & paint::STROKE != 0 && s.gs.odd_stroke);
        let m = s.matrix(s.gs.ctm.to_f64());
        if bits != 0 {
            if bits & (paint::FILL | paint::FILL_EVEN_ODD) != 0 {
                s.sync_fill(None, None);
            }
            if bits & paint::STROKE != 0 {
                s.sync_stroke();
            }
            if odd {
                // painted with a colour the display list cannot express:
                // the reader is told so where it is drawn
                s.dl.flags |= flashtex_display_list::page::flags::INCOMPLETE;
                s.dl.unsupported
                    .push("a path painted with a pattern or separation colour".into());
                s.sup.raw.push(None);
                let k = s.dl.unsupported.len() as u32 - 1;
                s.push(Item::Unsupported(k));
            }
            let stroke = (bits & paint::STROKE != 0).then(|| s.gs.stroke_params());
            s.dl.paths.push(Path {
                paint: bits,
                matrix: m,
                stroke,
                segs: segs.clone(),
            });
            let k = s.dl.paths.len() as u32 - 1;
            s.push(Item::Path(k));
        }
        if let Some(c) = clip {
            s.dl.paths.push(Path {
                paint: c,
                matrix: m,
                stroke: None,
                segs,
            });
            let k = s.dl.paths.len() as u32 - 1;
            s.push(Item::Clip(k));
        }
    }

    /// `/name gs`: the constant alphas and line width of the ExtGState in
    /// the stream's resources; any other key cannot be expressed.
    fn ext_gstate(&mut self, name: Option<&Obj>, text: &[u8]) {
        let Some(name) = name.and_then(Obj::as_name) else {
            return;
        };
        let res = self.cur_ref().sup.resources.clone();
        let eg = res.get(b"ExtGState").map(|o| self.doc.deref(o).clone());
        let d = eg
            .as_ref()
            .and_then(Obj::as_dict)
            .and_then(|d| d.get(name))
            .map(|o| self.doc.deref(o).clone());
        let Some(Obj::Dict(d)) = d else {
            self.raw(
                &format!("ExtGState /{} not found", String::from_utf8_lossy(name)),
                text,
                false,
            );
            return;
        };
        let mut other = false;
        for (k, v) in &d.0 {
            let v = self.doc.deref(v).clone();
            match k.as_slice() {
                b"ca" => {
                    if let Some(a) = v.as_num() {
                        self.cur().gs.fill_alpha = a;
                    }
                }
                b"CA" => {
                    if let Some(a) = v.as_num() {
                        self.cur().gs.stroke_alpha = a;
                    }
                }
                b"LW" => {
                    if let Some(w) = v.as_num() {
                        self.cur().gs.line_width = w;
                    }
                }
                b"Type" => {}
                b"AIS" if v == Obj::Bool(false) => {}
                b"BM" if matches!(v.as_name(), Some(b"Normal" | b"Compatible")) => {}
                b"SMask" if v.as_name() == Some(b"None") => {}
                _ => other = true,
            }
        }
        if other {
            self.raw(
                &format!(
                    "ExtGState /{} beyond constant alpha",
                    String::from_utf8_lossy(name)
                ),
                text,
                false,
            );
        }
    }
}

fn current_point(path: &[Seg]) -> (f64, f64) {
    let mut start = (0.0, 0.0);
    let mut cur = (0.0, 0.0);
    for s in path {
        match *s {
            Seg::Move(x, y) => {
                start = (x, y);
                cur = (x, y);
            }
            Seg::Line(x, y) => cur = (x, y),
            Seg::Curve(_, _, _, _, x, y) => cur = (x, y),
            Seg::Close => cur = start,
        }
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::super::doc::Doc;
    use super::super::page::tests::{page_bytes, special, NoTfm};
    use super::super::page::Builder;
    use flashtex_display_list::page::{paint, Item};

    #[test]
    fn tikz_like_paths_colours_and_alpha() {
        let mut doc = Doc::new();
        let mut body = Vec::new();
        for s in [
            "pdf:obj @pgfextgs <<>>",
            "pdf:put @resources << /ExtGState @pgfextgs >>",
            "pdf:put @pgfextgs << /pgf@ca0.5 << /ca 0.5 >> >>",
            "pdf:bcontent",
            "pdf:code q 0.3985 w",
            "color push rgb 1 0 0",
            "pdf:code 0.0 0.0 m 84.36938 28.12314 l S",
            "pdf:code /pgf@ca0.5 gs 0 0 10 10 re f",
            "color pop",
            "pdf:code Q",
            "pdf:econtent",
            "pdf:code /Sh sh",
        ] {
            body.extend(special(s));
        }
        let built = Builder::page(&mut doc, &NoTfm, 1000, 1, &page_bytes(&body)).unwrap();
        let paths = &built.dl.paths;
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].paint, paint::STROKE);
        assert_eq!(paths[0].stroke.as_ref().unwrap().width, 0.3985);
        assert_eq!(paths[1].paint, paint::FILL);
        let m = built.dl.matrix(paths[0].matrix);
        assert_eq!(m, [1.0, 0.0, 0.0, 1.0, 72.0, 720.0]);
        assert!(built.dl.items.contains(&Item::FillAlpha(0.5)));
        assert!(built
            .dl
            .items
            .iter()
            .any(|i| matches!(i, Item::StrokeColor(c) if c.0 == vec![1.0, 0.0, 0.0])));
        assert_eq!(built.dl.unsupported, vec!["a shading (sh)".to_string()]);
        assert_eq!(built.sup.raw[0].as_ref().unwrap().ops, b"/Sh sh");
    }
}
