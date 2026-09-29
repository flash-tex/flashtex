//! `pdftoepdf.cc`, ported: PDF inclusion (`\pdfximage` of a PDF file).
//!
//! pdfTeX reads the included PDF with xpdf and copies the selected page as a
//! form XObject: its content stream unchanged, its resources object by
//! object, each indirect object once per document under a new number, and
//! its embedded Type 1 fonts replaced by pdfTeX's own subsets of the same
//! fonts (found through the font map) with new `/Encoding` dictionaries.
//! The parser here is xpdf itself, TeX Live's copy, linked unmodified
//! ([`super::xpdf`]); this file is the part of pdftoepdf.cc that writes
//! pdfTeX's output, which the C++ does through the engine's globals.
//!
//! Where pdftoepdf.cc has undefined behaviour on a malformed file (reading
//! an object as a type it isn't, failed `assert`s), this port stops the run
//! with a pdfTeX-style error instead (DESIGN.md §4.5); each such place says
//! so.

use super::xpdf::{self, Doc, Font, Obj};
use crate::generated::Globals;
use std::collections::HashMap;

const PDF_KEY_PREFIX: &[u8] = b"PTEX";
const MASK_SUPPRESS_PTEX_FILENAME: i32 = 0x02;
const MASK_SUPPRESS_PTEX_PAGENUMBER: i32 = 0x04;
const MASK_SUPPRESS_PTEX_INFODICT: i32 = 0x08;

#[derive(Clone, Copy, PartialEq, Eq)]
enum InObjType {
    Font,
    FontDesc,
    Other,
}

/// `InObj`: an indirect object of the included PDF that is copied.
struct InObj {
    /// `ref`: (num, gen) in the original PDF.
    r: (i32, i32),
    ty: InObjType,
    /// `num`: the object number in the output PDF.
    num: i32,
    /// `fd`: the font descriptor (arena index), for fonts.
    fd: Option<usize>,
    enc_objnum: i32,
    written: bool,
}

/// The objects list of one document: `inObjList` in order, and an index by
/// original reference (the C code searches the list; the first entry with a
/// reference is the one it finds, which is the only one).
#[derive(Default)]
struct InObjList {
    list: Vec<InObj>,
    index: HashMap<(i32, i32), usize>,
}

/// `PdfDocument`: an open included PDF.
pub struct PdfDocument {
    file_name: Vec<u8>,
    doc: Doc,
    in_objs: InObjList,
    /// `occurences`: references to the document; it is deleted when this
    /// drops below 0.
    occurences: i32,
}

/// `UsedEncoding`: a replaced font whose `/Encoding` is written.
struct UsedEncoding {
    enc_objnum: i32,
    font: Font,
}

#[derive(Default)]
pub struct State {
    /// `pdfDocuments`, by handle (`None` once deleted).
    docs: Vec<Option<PdfDocument>>,
    /// `isInit`.
    is_init: bool,
}

/// What `read_pdf_info` leaves in pdfTeX's `epdf_*` globals.
pub struct PdfInfo {
    pub page_num: i32,
    pub width: f32,
    pub height: f32,
    pub orig_x: f32,
    pub orig_y: f32,
    pub rotate: f32,
    pub num_pages: i32,
    pub has_page_group: bool,
    /// `epdf_doc`: the document's handle.
    pub doc: usize,
}

/// The objects list and encodings of the `write_epdf` in progress, with
/// the document (`xref`, `inObjList`, `encodingList`).
struct Ctx<'a> {
    doc: &'a Doc,
    in_objs: &'a mut InObjList,
    encodings: Vec<UsedEncoding>,
}

/// `zround` (texmfmp.c).
fn zround(r: f64) -> i32 {
    if r > 2147483647.0 {
        2147483647
    } else if r < -2147483647.0 {
        -2147483647
    } else if r >= 0.0 {
        (r + 0.5) as i32
    } else {
        (r - 0.5) as i32
    }
}

/// `convertNumToPDF`: a number with at most six decimals and never in
/// exponent form.
fn convert_num_to_pdf(n: f64) -> Vec<u8> {
    const FACT: i32 = 1_000_000; // must be 10^precision
    const EPSILON: f64 = 0.5E-6; // 2epsilon must be 10^-precision
    let mut buf = Vec::new();
    if n.abs() < EPSILON {
        buf.push(b'0');
        return buf;
    }
    let mut n = n;
    if n < 0.0 {
        buf.push(b'-');
        n = -n;
    }
    n += EPSILON; // for rounding
    let ival = n.floor() as i32;
    n -= ival as f64;
    buf.extend_from_slice(ival.to_string().as_bytes());
    let mut fval = (n * FACT as f64).floor() as i32;
    if fval != 0 {
        buf.push(b'.');
        let mut digits = [b'0'; 6];
        let mut k = 6usize;
        // trailing zeros become the end of the string
        while fval % 10 == 0 && k > 0 {
            k -= 1;
            fval /= 10;
        }
        let len = k;
        while k > 0 {
            k -= 1;
            digits[k] = (fval % 10) as u8 + b'0';
            fval /= 10;
        }
        buf.extend_from_slice(&digits[..len]);
    }
    buf
}

/// `stripzeros` (utils.c): drop trailing zeros of the decimals of every
/// number in `a` (and the point if nothing is left after it).
pub fn strip_zeros(a: &[u8]) -> Vec<u8> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum S {
        NoNum,
        DotNoNum,
        Int,
        Dot,
        LeadDot,
        Frac,
    }
    let mut buf = a.to_vec();
    let (mut s, mut t) = (S::NoNum, S::NoNum);
    let (mut p, mut q, mut r) = (0usize, 0usize, 0usize);
    while p < buf.len() {
        let c = buf[p];
        s = match s {
            S::NoNum => {
                if c.is_ascii_digit() {
                    S::Int
                } else if c == b'.' {
                    S::LeadDot
                } else {
                    s
                }
            }
            S::DotNoNum => {
                if c != b'.' && !c.is_ascii_digit() {
                    S::NoNum
                } else {
                    s
                }
            }
            S::Int => {
                if c == b'.' {
                    S::Dot
                } else if !c.is_ascii_digit() {
                    S::NoNum
                } else {
                    s
                }
            }
            S::Dot | S::LeadDot => {
                if c.is_ascii_digit() {
                    S::Frac
                } else if c == b'.' {
                    S::DotNoNum
                } else {
                    S::NoNum
                }
            }
            S::Frac => {
                if c == b'.' {
                    S::DotNoNum
                } else if !c.is_ascii_digit() {
                    S::NoNum
                } else {
                    s
                }
            }
        };
        match s {
            S::Dot => r = q,
            S::LeadDot => r = q + 1,
            S::Frac => {
                if c > b'0' {
                    r = q + 1;
                }
            }
            S::NoNum => {
                if (t == S::Frac || t == S::Dot) && r != 0 {
                    q = r;
                    r -= 1;
                    if buf[r] == b'.' {
                        // was a LEADDOT
                        buf[r] = b'0';
                    }
                    r = 0;
                }
            }
            S::DotNoNum | S::Int => {}
        }
        buf[q] = buf[p];
        q += 1;
        p += 1;
        t = s;
    }
    buf.truncate(q);
    buf
}

impl Globals {
    /// `pdfout(c)` (ptexmac.h): one byte into the PDF buffer; unlike
    /// `pdf_puts` it leaves `pdflastbyte` alone.
    pub(crate) fn c_pdf_out(&mut self, c: u8) {
        self.c_pdf_room(1);
        let p = self.pdf_ptr;
        self.pdf_buf_set(p, c as i32);
        self.pdf_ptr += 1;
    }

    /// `convertStringToPDFString` (utils.c): `in` as the inside of a PDF
    /// string literal.
    pub(crate) fn convert_string_to_pdf_string(&mut self, input: &[u8]) -> Vec<u8> {
        const MAX_PSTRING_LEN: usize = 1024;
        let mut out = Vec::new();
        for &c in input {
            // check_buf(j + sizeof(buf), MAX_PSTRING_LEN)
            if out.len() + 5 > MAX_PSTRING_LEN {
                self.pdftex_fail(
                    "buffer overflow at file ../../../texk/web2c/pdftexdir/utils.c, line 438",
                );
            }
            if !(b'!'..=b'~').contains(&c) {
                // convert control characters into oct
                out.extend_from_slice(format!("\\{c:03o}").as_bytes());
            } else if c == b'(' || c == b')' {
                out.push(b'\\');
                out.push(c);
            } else if c == b'\\' {
                out.extend_from_slice(b"\\\\");
            } else {
                out.push(c);
            }
        }
        out
    }

    /// `find_add_document`: the open document `file_name`, opened now if it
    /// is not open yet; `occurences` counts the other references.
    fn find_add_document(&mut self, st: &mut State, file_name: &[u8]) -> usize {
        for (i, d) in st.docs.iter_mut().enumerate() {
            if let Some(d) = d {
                if d.file_name == file_name {
                    d.occurences += 1;
                    return i;
                }
            }
        }
        let doc = Doc::open(file_name);
        if !doc.ok() {
            self.pdftex_fail("xpdf: reading PDF image failed");
        }
        st.docs.push(Some(PdfDocument {
            file_name: file_name.to_vec(),
            doc,
            in_objs: InObjList::default(),
            occurences: 0,
        }));
        st.docs.len() - 1
    }

    /// `addInObj`: the output object number of indirect object `r`, given
    /// one (and queued for `writeRefs`) if it has none yet.
    fn add_in_obj(
        &mut self,
        cx: &mut Ctx,
        ty: InObjType,
        r: (i32, i32),
        fd: Option<usize>,
        e: i32,
    ) -> i32 {
        if r.0 == 0 {
            self.pdftex_fail("PDF inclusion: invalid reference");
        }
        if let Some(&i) = cx.in_objs.index.get(&r) {
            return cx.in_objs.list[i].num;
        }
        // new objects go at the end: the list is being written out while
        // they are added
        let num = if ty == InObjType::FontDesc {
            self.with_fonts(|_, st| {
                let d = st.wf.fds[fd.unwrap()].as_ref().unwrap();
                // get_fd_objnum: assert(fd->fd_objnum != 0)
                d.fd_objnum
            })
        } else {
            self.pdf_new_objnum()
        };
        cx.in_objs.index.insert(r, cx.in_objs.list.len());
        cx.in_objs.list.push(InObj {
            r,
            ty,
            num,
            fd,
            enc_objnum: e,
            written: false,
        });
        num
    }

    /// `addOther`.
    fn add_other(&mut self, cx: &mut Ctx, r: (i32, i32)) -> i32 {
        self.add_in_obj(cx, InObjType::Other, r, None, 0)
    }

    /// `copyName`: `/` and the name, with every byte other than letters,
    /// digits and `_.-+` as `#XX`.
    fn copy_name(&mut self, s: &[u8]) {
        self.pdf_puts(b"/");
        for &c in s {
            if c == 0 {
                break;
            }
            if c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'-' | b'+') {
                self.c_pdf_out(c);
            } else {
                self.pdf_printf(format!("#{c:02X}").as_bytes());
            }
        }
    }

    /// `copyDictEntry`.
    fn copy_dict_entry(&mut self, cx: &mut Ctx, obj: &Obj, i: i32) {
        self.copy_name(&obj.dict_key(i));
        self.pdf_puts(b" ");
        let obj1 = obj.dict_val_nf(i);
        self.copy_object(cx, &obj1);
        self.pdf_puts(b"\n");
    }

    /// `copyDict`.
    fn copy_dict(&mut self, cx: &mut Ctx, obj: &Obj) {
        if !obj.is_dict() {
            self.fail_type("PDF inclusion: invalid dict type <", obj);
        }
        for i in 0..obj.dict_len() {
            self.copy_dict_entry(cx, obj, i);
        }
    }

    /// `pdftex_fail("...<%s>", obj->getTypeName())`.
    fn fail_type(&mut self, msg: &str, obj: &Obj) -> ! {
        let t = String::from_utf8_lossy(&obj.type_name()).into_owned();
        self.pdftex_fail(&format!("{msg}{t}>"))
    }

    /// `copyFontDict`: the font dictionary with pdfTeX's descriptor, font
    /// name object and encoding in place of the original ones.
    fn copy_font_dict(&mut self, cx: &mut Ctx, obj: &Obj, fd: usize, enc_objnum: i32) {
        if !obj.is_dict() {
            self.fail_type("PDF inclusion: invalid dict type <", obj);
        }
        self.pdf_puts(b"<<\n");
        for i in 0..obj.dict_len() {
            let key = obj.dict_key(i);
            if key.starts_with(b"FontDescriptor")
                || key.starts_with(b"BaseFont")
                || key.starts_with(b"Encoding")
            {
                continue; // skip original values
            }
            self.copy_dict_entry(cx, obj, i);
        }
        // write new FontDescriptor, BaseFont, and Encoding
        let (fd_objnum, fn_objnum) = self.with_fonts(|g, st| {
            let fd_objnum = st.wf.fds[fd].as_ref().unwrap().fd_objnum;
            (fd_objnum, g.get_fn_objnum(st, fd))
        });
        self.pdf_printf(format!("/FontDescriptor {fd_objnum} 0 R\n").as_bytes());
        self.pdf_printf(format!("/BaseFont {fn_objnum} 0 R\n").as_bytes());
        self.pdf_printf(format!("/Encoding {enc_objnum} 0 R\n").as_bytes());
        self.pdf_puts(b">>");
    }

    /// `copyStream`: the bytes of a stream, as they come.
    fn copy_stream_bytes(&mut self, bytes: &[u8]) {
        let mut c2 = 0u8;
        for &c in bytes {
            self.c_pdf_out(c);
            c2 = c;
        }
        self.pdf_last_byte = c2 as i32;
    }

    /// `copyProcSet`.
    fn copy_proc_set(&mut self, obj: &Obj) {
        if !obj.is_array() {
            self.fail_type("PDF inclusion: invalid ProcSet array type <", obj);
        }
        self.pdf_puts(b"/ProcSet [ ");
        for i in 0..obj.array_len() {
            let procset = obj.array_get_nf(i);
            if !procset.is_name() {
                self.fail_type("PDF inclusion: invalid ProcSet entry type <", &procset);
            }
            self.copy_name(&procset.get_name());
            self.pdf_puts(b" ");
        }
        self.pdf_puts(b"]\n");
    }

    /// `copyFont`: a font resource. An embedded Type 1 (or Type 1C) font
    /// that the font map knows is replaced by pdfTeX's own embedding of
    /// that font (`\pdfinclusioncopyfonts=0`); any other font is copied.
    fn copy_font(&mut self, cx: &mut Ctx, tag: &[u8], font_ref: &Obj) {
        // Check whether the font has already been embedded before
        // analysing it.
        let r = font_ref.get_ref();
        if let Some(&i) = cx.in_objs.index.get(&r) {
            let num = cx.in_objs.list[i].num;
            self.copy_name(tag);
            self.pdf_printf(format!(" {num} 0 R ").as_bytes());
            return;
        }
        // Only handle included Type1 (and Type1C) fonts; anything else will
        // be copied. Type1C fonts are replaced by Type1 fonts
        // (REPLACE_TYPE1C).
        let doc = cx.doc;
        let replace = (|| {
            if self.fixed_inclusion_copy_font != 0 {
                return None;
            }
            let fontdict = font_ref.fetch(doc);
            if !fontdict.is_dict() {
                return None;
            }
            let subtype = fontdict.dict_lookup(b"Subtype");
            if !(subtype.is_name() && subtype.get_name() == b"Type1") {
                return None;
            }
            let basefont = fontdict.dict_lookup(b"BaseFont");
            if !basefont.is_name() {
                return None;
            }
            let fontdesc_ref = fontdict.dict_lookup_nf(b"FontDescriptor");
            if !fontdesc_ref.is_ref() {
                return None;
            }
            let fontdesc = fontdesc_ref.fetch(doc);
            if !fontdesc.is_dict() {
                return None;
            }
            let has_file = fontdesc.dict_lookup(b"FontFile").is_stream() || {
                let fontfile = fontdesc.dict_lookup(b"FontFile3");
                fontfile.is_stream() && {
                    let ffsubtype = fontfile.stream_dict().dict_lookup(b"Subtype");
                    ffsubtype.is_name() && ffsubtype.get_name() == b"Type1C"
                }
            };
            if !has_file {
                return None;
            }
            let name = basefont.get_name();
            let fm = self.with_fonts(|g, st| g.lookup_fontmap(st, &name))?;
            Some((fontdict, fontdesc_ref, fontdesc, fm))
        })();
        let Some((fontdict, fontdesc_ref, fontdesc, fm)) = replace else {
            self.copy_name(tag);
            self.pdf_puts(b" ");
            self.copy_object(cx, font_ref);
            return;
        };
        // round /StemV value, since the PDF input is a float (see Font
        // Descriptors in PDF reference), but we only store an integer.
        // (A missing /StemV reads as 0: pdfTeX reads a null object's number,
        // which is undefined.)
        let stem_v = zround(fontdesc.dict_lookup(b"StemV").get_num());
        let charset = fontdesc.dict_lookup(b"CharSet");
        let fd = self.with_fonts(|g, st| {
            let fd = g.epdf_create_fontdescriptor(st, fm, stem_v);
            if charset.is_string() && g.is_subsetable(st, fm) {
                // getCString(): up to the first NUL
                let cs = charset.get_string();
                let end = cs.iter().position(|&b| b == 0).unwrap_or(cs.len());
                Self::epdf_mark_glyphs(st, fd, &cs[..end]);
            } else {
                Self::embed_whole_font(st, fd);
            }
            fd
        });
        self.add_in_obj(cx, InObjType::FontDesc, fontdesc_ref.get_ref(), Some(fd), 0);
        self.copy_name(tag);
        let Some(gfont) = doc.make_font(tag, font_ref.get_ref(), &fontdict) else {
            self.pdftex_fail("PDF inclusion: invalid font dictionary");
        };
        // addEncoding, evaluated before addFont's own number
        let enc_objnum = self.pdf_new_objnum();
        cx.encodings.push(UsedEncoding {
            enc_objnum,
            font: gfont,
        });
        let num = self.add_in_obj(
            cx,
            InObjType::Font,
            font_ref.get_ref(),
            Some(fd),
            enc_objnum,
        );
        self.pdf_printf(format!(" {num} 0 R ").as_bytes());
    }

    /// `copyFontResources`.
    fn copy_font_resources(&mut self, cx: &mut Ctx, obj: &Obj) {
        if !obj.is_dict() {
            self.fail_type("PDF inclusion: invalid font resources dict type <", obj);
        }
        self.pdf_puts(b"/Font << ");
        for i in 0..obj.dict_len() {
            let font_ref = obj.dict_val_nf(i);
            if font_ref.is_ref() {
                self.copy_font(cx, &obj.dict_key(i), &font_ref);
            } else if font_ref.is_dict() {
                // some programs generate pdf with embedded font object
                self.copy_name(&obj.dict_key(i));
                self.pdf_puts(b" ");
                self.copy_object(cx, &font_ref);
            } else {
                self.fail_type("PDF inclusion: invalid font in reference type <", &font_ref);
            }
        }
        self.pdf_puts(b">>\n");
    }

    /// `copyOtherResources`: every resource but fonts and procedure sets.
    fn copy_other_resources(&mut self, cx: &mut Ctx, obj: &Obj, key: &[u8]) {
        let k = String::from_utf8_lossy(key).into_owned();
        // if Subtype is present, it must be a name
        if key == b"Subtype" {
            if !obj.is_name() {
                let t = String::from_utf8_lossy(&obj.type_name()).into_owned();
                self.pdftex_warn(&format!(
                    "PDF inclusion: Subtype in Resources dict is not a name \
                     (key '{k}', type <{t}>); ignored."
                ));
                return;
            }
        } else if !obj.is_dict() {
            let t = String::from_utf8_lossy(&obj.type_name()).into_owned();
            self.pdftex_warn(&format!(
                "PDF inclusion: invalid other resource which is no dict \
                 (key '{k}', type <{t}>); ignored."
            ));
            return;
        }
        self.copy_name(key);
        self.pdf_puts(b" ");
        self.copy_object(cx, obj);
    }

    /// `copyObject`: an object in PDF syntax; indirect objects become
    /// references to their new numbers.
    fn copy_object(&mut self, cx: &mut Ctx, obj: &Obj) {
        use super::xpdf::ObjType as T;
        match obj.obj_type() {
            T::Bool => self.pdf_printf(if obj.get_bool() { b"true" } else { b"false" }),
            T::Int => self.pdf_printf(obj.get_int().to_string().as_bytes()),
            T::Real => {
                let s = convert_num_to_pdf(obj.get_num());
                self.pdf_printf(&s);
            }
            T::String => {
                let s = obj.get_string();
                let strlen = s.iter().position(|&b| b == 0).unwrap_or(s.len());
                if strlen == s.len() {
                    self.pdf_puts(b"(");
                    for &c in &s {
                        if c == b'(' || c == b')' || c == b'\\' {
                            self.pdf_printf(&[b'\\', c]);
                        } else if !(0x20..=0x7F).contains(&c) {
                            self.pdf_printf(format!("\\{c:03o}").as_bytes());
                        } else {
                            self.c_pdf_out(c);
                        }
                    }
                    self.pdf_puts(b")");
                } else {
                    self.pdf_puts(b"<");
                    for &c in &s {
                        self.pdf_printf(format!("{c:02x}").as_bytes());
                    }
                    self.pdf_puts(b">");
                }
            }
            T::Name => self.copy_name(&obj.get_name()),
            T::Null => self.pdf_puts(b"null"),
            T::Array => {
                self.pdf_puts(b"[");
                for i in 0..obj.array_len() {
                    let obj1 = obj.array_get_nf(i);
                    if !obj1.is_name() {
                        self.pdf_puts(b" ");
                    }
                    self.copy_object(cx, &obj1);
                }
                self.pdf_puts(b"]");
            }
            T::Dict => {
                self.pdf_puts(b"<<\n");
                self.copy_dict(cx, obj);
                self.pdf_puts(b">>");
            }
            T::Stream => {
                let obj1 = cx.doc.dict_copy(obj);
                self.pdf_puts(b"<<\n");
                self.copy_dict(cx, &obj1);
                self.pdf_puts(b">>\n");
                self.pdf_puts(b"stream\n");
                let bytes = obj.stream_bytes(true);
                self.copy_stream_bytes(&bytes);
                self.pdf_puts(b"\nendstream");
            }
            T::Ref => {
                let r = obj.get_ref();
                if r.0 == 0 {
                    self.pdftex_fail(
                        "PDF inclusion: reference to invalid object \
                         (is the included pdf broken?)",
                    );
                }
                let num = self.add_other(cx, r);
                self.pdf_printf(format!("{num} 0 R").as_bytes());
            }
            _ => self.fail_type("PDF inclusion: type <", obj),
        }
    }

    /// `writeRefs`: every queued indirect object not written yet,
    /// including those queued while writing.
    fn write_refs(&mut self, cx: &mut Ctx) {
        let mut i = 0;
        while i < cx.in_objs.list.len() {
            if !cx.in_objs.list[i].written {
                cx.in_objs.list[i].written = true;
                let (r, ty, num, fd, enc) = {
                    let e = &cx.in_objs.list[i];
                    (e.r, e.ty, e.num, e.fd, e.enc_objnum)
                };
                let obj1 = cx.doc.fetch(r.0, r.1);
                match ty {
                    InObjType::Font => {
                        // assert(!obj1.isStream())
                        if obj1.is_stream() {
                            self.pdftex_fail("PDF inclusion: invalid font dict type <stream>");
                        }
                        self.pdf_begin_obj(num, 2); // \pdfobjcompresslevel = 2 is for this
                        self.copy_font_dict(cx, &obj1, fd.unwrap(), enc);
                        self.pdf_puts(b"\n");
                        self.pdf_end_obj();
                    }
                    InObjType::Other => {
                        if obj1.is_stream() {
                            self.pdf_begin_obj(num, 0);
                        } else {
                            self.pdf_begin_obj(num, 2); // \pdfobjcompresslevel = 2 is for this
                        }
                        self.copy_object(cx, &obj1);
                        self.pdf_puts(b"\n");
                        self.pdf_end_obj();
                    }
                    // /FontDescriptor is written via write_fontdescriptor()
                    InObjType::FontDesc => {}
                }
            }
            i += 1;
        }
    }

    /// `writeEncodings`: the `/Encoding` of every replaced font, the last
    /// one first (the C list is built by prepending).
    fn write_encodings(&mut self, cx: &mut Ctx) {
        let encodings = std::mem::take(&mut cx.encodings);
        for r in encodings.iter().rev() {
            let mut glyph_names: Vec<Option<Vec<u8>>> = Vec::with_capacity(256);
            for i in 0..256 {
                if r.font.is_cid() {
                    self.pdftex_fail(
                        "PDF inclusion: CID fonts are not supported \
                         (try to disable font replacement to fix this)",
                    );
                }
                glyph_names.push(r.font.char_name(i));
            }
            self.epdf_write_enc(&glyph_names, r.enc_objnum);
        }
    }

    /// `get_pagebox`: the box `\pdfximage` asked for (`/MediaBox` ...).
    fn get_pagebox(&mut self, doc: &Doc, page: i32, pagebox_spec: i32) -> [f64; 4] {
        let which = if pagebox_spec == self.pdf_box_spec_media {
            0
        } else if pagebox_spec == self.pdf_box_spec_crop {
            1
        } else if pagebox_spec == self.pdf_box_spec_bleed {
            2
        } else if pagebox_spec == self.pdf_box_spec_trim {
            3
        } else if pagebox_spec == self.pdf_box_spec_art {
            4
        } else {
            self.pdftex_fail(&format!(
                "PDF inclusion: unknown value of pagebox spec ({pagebox_spec})"
            ));
        };
        match doc.page_box(page, which) {
            Some(b) => b,
            None => self.pdftex_fail(&format!(
                "PDF inclusion: required page does not exist <{}>",
                doc.num_pages()
            )),
        }
    }

    /// `read_pdf_info`: open the PDF, check its version, select the page
    /// (by `page_name`, a named destination, or by number) and read its
    /// box, rotation and page group.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn read_pdf_info(
        &mut self,
        st: &mut State,
        image_name: &[u8],
        page_name: Option<&[u8]>,
        mut page_num: i32,
        pagebox_spec: i32,
        major_pdf_version_wanted: i32,
        minor_pdf_version_wanted: i32,
        pdf_inclusion_errorlevel: i32,
    ) -> PdfInfo {
        // initialize
        if !st.is_init {
            xpdf::init();
            st.is_init = true;
        }
        // open PDF file
        let h = self.find_add_document(st, image_name);
        let d = st.docs[h].as_ref().unwrap();
        let doc = &d.doc;
        // check PDF version (this works only for PDF 1.x)
        let found = doc.pdf_version() as f32;
        let wanted =
            (major_pdf_version_wanted as f64 + minor_pdf_version_wanted as f64 * 0.1) as f32;
        if found as f64 > wanted as f64 + 0.01 {
            let mut msg = b"PDF inclusion: found PDF version <".to_vec();
            msg.extend_from_slice(&super::cfmt::fmt_f(1, found as f64));
            msg.extend_from_slice(b">, but at most version <");
            msg.extend_from_slice(&super::cfmt::fmt_f(1, wanted as f64));
            msg.extend_from_slice(b"> allowed");
            let msg = String::from_utf8_lossy(&msg).into_owned();
            if pdf_inclusion_errorlevel > 0 {
                self.pdftex_fail(&msg);
            } else if pdf_inclusion_errorlevel < 0 {
                // do nothing
            } else {
                // = 0, give warning
                self.pdftex_warn(&msg);
            }
        }
        let num_pages = doc.num_pages();
        if let Some(name) = page_name {
            // get page by name
            let shown = String::from_utf8_lossy(name).into_owned();
            match doc.find_dest_page(name) {
                -1 => self.pdftex_fail(&format!("PDF inclusion: invalid destination <{shown}>")),
                0 => self.pdftex_fail(&format!(
                    "PDF inclusion: destination is not a page <{shown}>"
                )),
                p => page_num = p,
            }
        } else if page_num <= 0 || page_num > num_pages {
            // get page by number
            self.pdftex_fail(&format!(
                "PDF inclusion: required page does not exist <{num_pages}>"
            ));
        }
        // get the pagebox (media, crop...) to use.
        let b = self.get_pagebox(doc, page_num, pagebox_spec);
        let (orig_x, width) = if b[2] > b[0] {
            (b[0] as f32, (b[2] - b[0]) as f32)
        } else {
            (b[2] as f32, (b[0] - b[2]) as f32)
        };
        let (orig_y, height) = if b[3] > b[1] {
            (b[1] as f32, (b[3] - b[1]) as f32)
        } else {
            (b[3] as f32, (b[1] - b[3]) as f32)
        };
        // get page rotation
        let mut rotate = (doc.page_rotate(page_num) % 360) as f32;
        if rotate < 0.0 {
            rotate += 360.0;
        }
        // page group: only flag that one is present; its object number is
        // made in pdftex.web
        let has_page_group = doc.page_group(page_num).is_some();
        PdfInfo {
            page_num,
            width,
            height,
            orig_x,
            orig_y,
            rotate,
            num_pages,
            has_page_group,
            doc: h,
        }
    }

    /// `write_epdf`: the selected page of document `h` as the body of the
    /// form XObject whose dictionary pdftex.web has begun.
    pub(crate) fn write_epdf(
        &mut self,
        st: &mut State,
        h: usize,
        selected_page: i32,
        page_box: i32,
    ) {
        let suppress_ptex_info = self.get_pdf_suppress_ptex_info();
        const PAGE_DICT_KEYS: [&[u8]; 4] = [
            b"LastModified",
            b"Metadata",
            b"PieceInfo",
            b"SeparationInfo",
        ];
        let sep: &[u8] = if self.get_ptex_use_underscore() {
            b"_"
        } else {
            b"."
        };
        let Some(d) = st.docs.get_mut(h).and_then(|d| d.as_mut()) else {
            self.pdftex_fail("PDF inclusion: the included document is gone");
        };
        d.occurences -= 1;
        let file_name = d.file_name.clone();
        let mut in_objs = std::mem::take(&mut d.in_objs);
        let doc = &d.doc;
        let mut cx = Ctx {
            doc,
            in_objs: &mut in_objs,
            encodings: Vec::new(),
        };
        let Some(page_ref) = doc.page_ref(selected_page) else {
            self.pdftex_fail(&format!(
                "PDF inclusion: required page does not exist <{}>",
                doc.num_pages()
            ));
        };
        let page_obj = doc.fetch(page_ref.0, page_ref.1);
        if !page_obj.is_dict() {
            // pdfTeX reads it as a dictionary regardless
            self.fail_type("PDF inclusion: invalid page dict type <", &page_obj);
        }
        let rotate = doc.page_rotate(selected_page);
        // write the Page header
        self.pdf_puts(b"/Type /XObject\n");
        self.pdf_puts(b"/Subtype /Form\n");
        self.pdf_puts(b"/FormType 1\n");
        // write additional information
        if suppress_ptex_info & MASK_SUPPRESS_PTEX_FILENAME == 0 {
            let mut s = b"/".to_vec();
            s.extend_from_slice(PDF_KEY_PREFIX);
            s.extend_from_slice(sep);
            s.extend_from_slice(b"FileName (");
            s.extend_from_slice(&self.convert_string_to_pdf_string(&file_name));
            s.extend_from_slice(b")\n");
            self.pdf_printf(&s);
        }
        if suppress_ptex_info & MASK_SUPPRESS_PTEX_PAGENUMBER == 0 {
            let mut s = b"/".to_vec();
            s.extend_from_slice(PDF_KEY_PREFIX);
            s.extend_from_slice(sep);
            s.extend_from_slice(format!("PageNumber {selected_page}\n").as_bytes());
            self.pdf_printf(&s);
        }
        if suppress_ptex_info & MASK_SUPPRESS_PTEX_INFODICT == 0 {
            let info = doc.doc_info_nf();
            if info.is_ref() {
                // the info dict must be indirect (PDF Ref p. 61)
                let mut s = b"/".to_vec();
                s.extend_from_slice(PDF_KEY_PREFIX);
                s.extend_from_slice(sep);
                s.extend_from_slice(b"InfoDict ");
                self.pdf_printf(&s);
                let num = self.add_other(&mut cx, info.get_ref());
                self.pdf_printf(format!("{num} 0 R\n").as_bytes());
            }
        }
        // get the pagebox (media, crop...) to use.
        let pagebox = self.get_pagebox(doc, selected_page, page_box);
        let (x1, y1, x2, y2) = (pagebox[0], pagebox[1], pagebox[2], pagebox[3]);
        // handle page rotation
        if rotate != 0 && rotate % 90 == 0 {
            // this handles only the simple case: multiple of 90s. The image
            // is rotated around its center; /Rotate is clockwise while the
            // matrix is counterclockwise.
            self.tex_printf(format!(", page is rotated {rotate} degrees").as_bytes());
            let mut scale = [0f64; 6];
            let mut writematrix = false;
            match rotate {
                90 => {
                    scale[1] = -1.0;
                    scale[2] = 1.0;
                    scale[4] = x1 - y1;
                    scale[5] = y1 + x2;
                    writematrix = true;
                }
                180 => {
                    scale[0] = -1.0;
                    scale[3] = -1.0;
                    scale[4] = x1 + x2;
                    scale[5] = y1 + y2;
                    writematrix = true;
                }
                270 => {
                    scale[1] = 1.0;
                    scale[2] = -1.0;
                    scale[4] = x1 + y2;
                    scale[5] = y1 - x1;
                    writematrix = true;
                }
                _ => {}
            }
            if writematrix {
                // The matrix is only written if the image is rotated.
                let mut s = b"/Matrix [".to_vec();
                for (k, v) in scale.iter().enumerate() {
                    if k > 0 {
                        s.push(b' ');
                    }
                    s.extend_from_slice(&super::cfmt::fmt_f(8, *v));
                }
                s.extend_from_slice(b"]\n");
                let s = strip_zeros(&s);
                self.pdf_puts(&s);
            }
        }
        let mut s = b"/BBox [".to_vec();
        for (k, v) in [x1, y1, x2, y2].iter().enumerate() {
            if k > 0 {
                s.push(b' ');
            }
            s.extend_from_slice(&super::cfmt::fmt_f(8, *v));
        }
        s.extend_from_slice(b"]\n");
        let s = strip_zeros(&s);
        self.pdf_puts(&s);

        // Metadata validity check (as a stream it must be indirect)
        let dict_obj = page_obj.dict_lookup_nf(b"Metadata");
        if !dict_obj.is_null() && !dict_obj.is_ref() {
            self.pdftex_warn("PDF inclusion: /Metadata must be indirect object");
        }
        // copy selected items in Page dictionary except Resources & Group
        for key in PAGE_DICT_KEYS {
            let dict_obj = page_obj.dict_lookup_nf(key);
            if !dict_obj.is_null() {
                self.pdf_newline();
                let mut s = b"/".to_vec();
                s.extend_from_slice(key);
                s.push(b' ');
                self.pdf_printf(&s);
                self.copy_object(&mut cx, &dict_obj); // preserves indirection
            }
        }
        // handle page group
        let mut group_dict: Option<Obj> = None;
        let dict_obj = page_obj.dict_lookup_nf(b"Group");
        if !dict_obj.is_null() {
            if self.pdf_page_group_val == 0 {
                // another pdf with page group was included earlier on the
                // same page; copy the Group entry as is.
                if self.get_pdf_suppress_warning_page_group() == 0 {
                    self.pdftex_warn(
                        "PDF inclusion: multiple pdfs with page group included in a single page",
                    );
                }
                self.pdf_newline();
                self.pdf_puts(b"/Group ");
                self.copy_object(&mut cx, &dict_obj);
            } else {
                // write Group dict as a separate object, since the Page dict
                // also refers to it
                let dict_obj = page_obj.dict_lookup(b"Group");
                if !dict_obj.is_dict() {
                    self.pdftex_fail("PDF inclusion: /Group dict missing");
                }
                let Some(g) = doc.page_group(selected_page) else {
                    self.pdftex_fail("PDF inclusion: /Group dict missing");
                };
                group_dict = Some(doc.dict_copy(&g));
                self.pdf_printf(format!("/Group {} 0 R\n", self.pdf_page_group_val).as_bytes());
            }
        }
        // write the Resources dictionary
        match doc.page_resources(selected_page) {
            None => {
                // Resources can be missing (files without them have been
                // spotted in the wild); then the /Resources of the /Page
                // will be used. "This practice is not recommended".
                self.pdftex_warn(
                    "PDF inclusion: /Resources missing. 'This practice is not recommended' (PDF Ref)",
                );
            }
            Some(res) => {
                let obj1 = doc.dict_copy(&res);
                self.pdf_newline();
                self.pdf_puts(b"/Resources <<\n");
                for i in 0..obj1.dict_len() {
                    let obj2 = obj1.dict_val(i);
                    let key = obj1.dict_key(i);
                    if key == b"Font" {
                        self.copy_font_resources(&mut cx, &obj2);
                    } else if key == b"ProcSet" {
                        self.copy_proc_set(&obj2);
                    } else {
                        self.copy_other_resources(&mut cx, &obj2, &key);
                    }
                }
                self.pdf_puts(b">>\n");
            }
        }
        // write the page contents
        let contents = doc.page_contents(selected_page);
        if contents.is_stream() {
            // Variant B: copy stream without recompressing
            let sd = contents.stream_dict();
            if !sd.dict_lookup(b"F").is_null() {
                self.pdftex_fail("PDF inclusion: Unsupported external stream");
            }
            let obj1 = sd.dict_lookup(b"Length");
            if obj1.is_null() {
                // assert(!obj1->isNull())
                self.pdftex_fail("PDF inclusion: content stream without /Length");
            }
            self.pdf_puts(b"/Length ");
            self.copy_object(&mut cx, &obj1);
            self.pdf_puts(b"\n");
            let obj1 = sd.dict_lookup(b"Filter");
            if !obj1.is_null() {
                self.pdf_puts(b"/Filter ");
                self.copy_object(&mut cx, &obj1);
                self.pdf_puts(b"\n");
                let obj1 = sd.dict_lookup(b"DecodeParms");
                if !obj1.is_null() {
                    self.pdf_puts(b"/DecodeParms ");
                    self.copy_object(&mut cx, &obj1);
                    self.pdf_puts(b"\n");
                }
            }
            self.pdf_puts(b">>\nstream\n");
            let bytes = contents.stream_bytes(true);
            self.copy_stream_bytes(&bytes);
            self.pdf_end_stream();
        } else if contents.is_array() {
            self.pdf_begin_stream();
            let l = contents.array_len();
            for i in 0..l {
                let contentsobj = contents.array_get(i);
                if !contentsobj.is_stream() {
                    // pdfTeX reads it as a stream regardless
                    self.fail_type(
                        "PDF inclusion: invalid contents stream type <",
                        &contentsobj,
                    );
                }
                let bytes = contentsobj.stream_bytes(false);
                self.copy_stream_bytes(&bytes);
                if i < l - 1 {
                    self.pdf_newline(); // add a newline after each stream except the last
                }
            }
            self.pdf_end_stream();
        } else {
            // the contents are optional, but we need to include an empty
            // stream
            self.pdf_begin_stream();
            self.pdf_end_stream();
        }
        // write out all used encodings (and delete list)
        self.write_encodings(&mut cx);
        // write the Group dict if needed
        if let Some(g) = group_dict {
            self.pdf_begin_obj(self.pdf_page_group_val, 2);
            self.copy_object(&mut cx, &g);
            self.pdf_puts(b"\n");
            self.pdf_end_obj();
            // only the 1st included pdf on a page gets its Group included
            // in the Page dict
            self.pdf_page_group_val = 0;
        }
        // write out all indirect objects
        self.write_refs(&mut cx);
        drop(cx);
        // save object list
        if let Some(Some(d)) = st.docs.get_mut(h) {
            d.in_objs = in_objs;
        }
    }

    /// `epdf_delete`: the image is written and freed; close its document
    /// if nothing refers to it any more.
    pub(crate) fn epdf_delete(&mut self, st: &mut State, h: usize) {
        if let Some(slot) = st.docs.get_mut(h) {
            if slot.as_ref().is_some_and(|d| d.occurences < 0) {
                *slot = None; // delete_document
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_as_pdftoepdf_writes_them() {
        assert_eq!(convert_num_to_pdf(0.0), b"0");
        assert_eq!(convert_num_to_pdf(1.0), b"1");
        assert_eq!(convert_num_to_pdf(-2.5), b"-2.5");
        assert_eq!(convert_num_to_pdf(11.955), b"11.955");
        assert_eq!(convert_num_to_pdf(0.000001), b"0.000001");
        assert_eq!(convert_num_to_pdf(0.1234567), b"0.123457");
        assert_eq!(convert_num_to_pdf(3.0000001), b"3");
    }

    #[test]
    fn stripzeros_as_utils_c() {
        assert_eq!(
            strip_zeros(b"/BBox [0.00000000 0.00000000 13.94800000 11.95500000]\n"),
            b"/BBox [0 0 13.948 11.955]\n"
        );
        assert_eq!(strip_zeros(b"[-1.00000000 .50000000]"), b"[-1 .5]");
        assert_eq!(strip_zeros(b"[.00000000]"), b"[0]");
        assert_eq!(strip_zeros(b"a1.2.3"), b"a1.2.3");
    }
}
