//! TeX Live's xpdf 4.06 (third_party/xpdf, unmodified, C++), through the
//! C interface of `csrc/xpdf_shim.cc`: the PDF parser pdfTeX's
//! `pdftoepdf.cc` uses to include PDF files, and so the one
//! [`super::pdftoepdf`] uses. See docs/evidence/pdf-images-2026-09-29/ for
//! why it is linked rather than rewritten.
//!
//! [`Obj`] owns one xpdf `Object` (freed on drop); [`Doc`] one `PDFDoc`;
//! [`Font`] one `GfxFont`. The shim checks every type xpdf's accessors
//! assume, so no call here can read the wrong member of xpdf's unions.

use std::ffi::{c_char, c_int, c_void, CStr, CString};

#[repr(C)]
struct RawObj {
    _private: [u8; 0],
}

/// The C functions of TeX Live's libraries that build.rs compiled (it sets
/// `cfg(flashtex_images)` when it has), declared as `extern "C"`; without
/// build.rs (scripts/flashtex-etrip.sh builds the engine as a scratch
/// package, which reads no images) stand-ins with the same signatures, which
/// [`Globals::read_image`](crate::generated::Globals::read_image) never
/// lets run ([`LINKED`] is false there).
macro_rules! linked_or_unlinked {
    ($(fn $name:ident($($a:ident: $t:ty),* $(,)?) $(-> $r:ty)?;)*) => {
        #[cfg(flashtex_images)]
        extern "C" {
            $(fn $name($($a: $t),*) $(-> $r)?;)*
        }
        $(
            #[cfg(not(flashtex_images))]
            #[allow(unused_variables, clippy::too_many_arguments)]
            unsafe fn $name($($a: $t),*) $(-> $r)? {
                panic!("TeX Live's libpng and xpdf are not linked into this build (see build.rs)")
            }
        )*
    };
}
pub(crate) use linked_or_unlinked;

/// Are libpng and xpdf linked (build.rs ran)?
pub const LINKED: bool = cfg!(flashtex_images);

linked_or_unlinked! {
    fn ftx_init();
    fn ftx_doc_open(file_name: *const c_char) -> *mut c_void;
    fn ftx_doc_ok(d: *mut c_void) -> c_int;
    fn ftx_doc_free(d: *mut c_void);
    fn ftx_doc_pdf_version(d: *mut c_void) -> f64;
    fn ftx_doc_num_pages(d: *mut c_void) -> c_int;
    fn ftx_doc_find_dest_page(d: *mut c_void, name: *const c_char) -> c_int;
    fn ftx_page_box(d: *mut c_void, n: c_int, which: c_int, out: *mut f64) -> c_int;
    fn ftx_page_rotate(d: *mut c_void, n: c_int) -> c_int;
    fn ftx_page_group(d: *mut c_void, n: c_int) -> *mut RawObj;
    fn ftx_page_resources(d: *mut c_void, n: c_int) -> *mut RawObj;
    fn ftx_page_contents(d: *mut c_void, n: c_int) -> *mut RawObj;
    fn ftx_page_ref(d: *mut c_void, n: c_int, num: *mut c_int, gen: *mut c_int) -> c_int;
    fn ftx_doc_info_nf(d: *mut c_void) -> *mut RawObj;
    fn ftx_xref_fetch(d: *mut c_void, num: c_int, gen: c_int) -> *mut RawObj;
    fn ftx_obj_free(o: *mut RawObj);
    fn ftx_obj_type(o: *mut RawObj) -> c_int;
    fn ftx_obj_type_name(o: *mut RawObj) -> *const c_char;
    fn ftx_obj_bool(o: *mut RawObj) -> c_int;
    fn ftx_obj_int(o: *mut RawObj) -> c_int;
    fn ftx_obj_num(o: *mut RawObj) -> f64;
    fn ftx_obj_string(o: *mut RawObj, len: *mut c_int) -> *const c_char;
    fn ftx_obj_name(o: *mut RawObj) -> *const c_char;
    fn ftx_obj_ref(o: *mut RawObj, num: *mut c_int, gen: *mut c_int);
    fn ftx_obj_fetch(d: *mut c_void, o: *mut RawObj) -> *mut RawObj;
    fn ftx_array_len(o: *mut RawObj) -> c_int;
    fn ftx_array_get_nf(o: *mut RawObj, i: c_int) -> *mut RawObj;
    fn ftx_array_get(o: *mut RawObj, i: c_int) -> *mut RawObj;
    fn ftx_dict_len(o: *mut RawObj) -> c_int;
    fn ftx_dict_key(o: *mut RawObj, i: c_int) -> *const c_char;
    fn ftx_dict_val_nf(o: *mut RawObj, i: c_int) -> *mut RawObj;
    fn ftx_dict_val(o: *mut RawObj, i: c_int) -> *mut RawObj;
    fn ftx_dict_lookup(o: *mut RawObj, key: *const c_char) -> *mut RawObj;
    fn ftx_dict_lookup_nf(o: *mut RawObj, key: *const c_char) -> *mut RawObj;
    fn ftx_dict_copy(d: *mut c_void, src: *mut RawObj) -> *mut RawObj;
    fn ftx_stream_dict(o: *mut RawObj) -> *mut RawObj;
    fn ftx_stream_bytes(o: *mut RawObj, raw: c_int, len: *mut usize) -> *mut u8;
    fn ftx_free(p: *mut c_void);
    fn ftx_font_make( d: *mut c_void, tag: *const c_char, num: c_int, gen: c_int, fontdict: *mut RawObj, ) -> *mut c_void;
    fn ftx_font_is_cid(f: *mut c_void) -> c_int;
    fn ftx_font_char_name(f: *mut c_void, i: c_int) -> *const c_char;
    fn ftx_font_free(f: *mut c_void);
}

/// xpdf's `ObjType`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjType {
    Bool,
    Int,
    Real,
    String,
    Name,
    Null,
    Array,
    Dict,
    Stream,
    Ref,
    Cmd,
    Error,
    Eof,
    None,
}

/// A NUL-terminated copy of `s` (cut at its first NUL, as C sees it).
fn cstr(s: &[u8]) -> CString {
    let end = s.iter().position(|&b| b == 0).unwrap_or(s.len());
    CString::new(&s[..end]).expect("no NUL")
}

/// The bytes of a C string xpdf owns (copied).
fn bytes_of(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    // SAFETY: the shim returns NUL-terminated strings that live at least
    // as long as the object they came from, which the caller holds.
    unsafe { CStr::from_ptr(p) }.to_bytes().to_vec()
}

/// `globalParams = new GlobalParams(); globalParams->setErrQuiet(gFalse)`,
/// once.
pub fn init() {
    // SAFETY: no arguments; idempotent.
    unsafe { ftx_init() }
}

/// An xpdf `Object`.
pub struct Obj(*mut RawObj);

impl Drop for Obj {
    fn drop(&mut self) {
        // SAFETY: we own the object (every constructor below takes a fresh
        // one from the shim).
        unsafe { ftx_obj_free(self.0) }
    }
}

impl Obj {
    fn wrap(p: *mut RawObj) -> Option<Obj> {
        (!p.is_null()).then_some(Obj(p))
    }

    fn new(p: *mut RawObj) -> Obj {
        assert!(!p.is_null(), "xpdf shim returned no object");
        Obj(p)
    }

    pub fn obj_type(&self) -> ObjType {
        // SAFETY: self.0 is a live object.
        match unsafe { ftx_obj_type(self.0) } {
            0 => ObjType::Bool,
            1 => ObjType::Int,
            2 => ObjType::Real,
            3 => ObjType::String,
            4 => ObjType::Name,
            5 => ObjType::Null,
            6 => ObjType::Array,
            7 => ObjType::Dict,
            8 => ObjType::Stream,
            9 => ObjType::Ref,
            10 => ObjType::Cmd,
            11 => ObjType::Error,
            12 => ObjType::Eof,
            _ => ObjType::None,
        }
    }

    pub fn is_bool(&self) -> bool {
        self.obj_type() == ObjType::Bool
    }
    pub fn is_int(&self) -> bool {
        self.obj_type() == ObjType::Int
    }
    pub fn is_real(&self) -> bool {
        self.obj_type() == ObjType::Real
    }
    pub fn is_num(&self) -> bool {
        matches!(self.obj_type(), ObjType::Int | ObjType::Real)
    }
    pub fn is_string(&self) -> bool {
        self.obj_type() == ObjType::String
    }
    pub fn is_name(&self) -> bool {
        self.obj_type() == ObjType::Name
    }
    pub fn is_null(&self) -> bool {
        self.obj_type() == ObjType::Null
    }
    pub fn is_array(&self) -> bool {
        self.obj_type() == ObjType::Array
    }
    pub fn is_dict(&self) -> bool {
        self.obj_type() == ObjType::Dict
    }
    pub fn is_stream(&self) -> bool {
        self.obj_type() == ObjType::Stream
    }
    pub fn is_ref(&self) -> bool {
        self.obj_type() == ObjType::Ref
    }

    /// `getTypeName()`.
    pub fn type_name(&self) -> Vec<u8> {
        // SAFETY: a static string.
        bytes_of(unsafe { ftx_obj_type_name(self.0) })
    }

    pub fn get_bool(&self) -> bool {
        // SAFETY: live object; the shim checks the type.
        unsafe { ftx_obj_bool(self.0) != 0 }
    }
    pub fn get_int(&self) -> i32 {
        // SAFETY: as above.
        unsafe { ftx_obj_int(self.0) }
    }
    /// `getNum()` (and `getReal()`).
    pub fn get_num(&self) -> f64 {
        // SAFETY: as above.
        unsafe { ftx_obj_num(self.0) }
    }
    /// The string's bytes (`getCString()` for `getLength()` bytes).
    pub fn get_string(&self) -> Vec<u8> {
        let mut len: c_int = 0;
        // SAFETY: the shim returns `len` bytes owned by the object.
        unsafe {
            let p = ftx_obj_string(self.0, &mut len);
            if p.is_null() || len <= 0 {
                return Vec::new();
            }
            std::slice::from_raw_parts(p as *const u8, len as usize).to_vec()
        }
    }
    pub fn get_name(&self) -> Vec<u8> {
        // SAFETY: the name lives as long as the object.
        bytes_of(unsafe { ftx_obj_name(self.0) })
    }
    /// `getRef()`: (num, gen).
    pub fn get_ref(&self) -> (i32, i32) {
        let (mut n, mut g) = (0, 0);
        // SAFETY: live object; out-parameters.
        unsafe { ftx_obj_ref(self.0, &mut n, &mut g) };
        (n, g)
    }

    /// `fetch(xref, &out)`: the referenced object, or a copy.
    pub fn fetch(&self, doc: &Doc) -> Obj {
        // SAFETY: live object and document.
        Obj::new(unsafe { ftx_obj_fetch(doc.0, self.0) })
    }

    pub fn array_len(&self) -> i32 {
        // SAFETY: live object; the shim checks the type.
        unsafe { ftx_array_len(self.0) }
    }
    pub fn array_get_nf(&self, i: i32) -> Obj {
        // SAFETY: as above; the shim checks the index.
        Obj::new(unsafe { ftx_array_get_nf(self.0, i) })
    }
    pub fn array_get(&self, i: i32) -> Obj {
        // SAFETY: as above.
        Obj::new(unsafe { ftx_array_get(self.0, i) })
    }

    pub fn dict_len(&self) -> i32 {
        // SAFETY: as above.
        unsafe { ftx_dict_len(self.0) }
    }
    pub fn dict_key(&self, i: i32) -> Vec<u8> {
        // SAFETY: the key lives as long as the dictionary.
        bytes_of(unsafe { ftx_dict_key(self.0, i) })
    }
    pub fn dict_val_nf(&self, i: i32) -> Obj {
        // SAFETY: as above.
        Obj::new(unsafe { ftx_dict_val_nf(self.0, i) })
    }
    pub fn dict_val(&self, i: i32) -> Obj {
        // SAFETY: as above.
        Obj::new(unsafe { ftx_dict_val(self.0, i) })
    }
    pub fn dict_lookup(&self, key: &[u8]) -> Obj {
        let k = cstr(key);
        // SAFETY: as above.
        Obj::new(unsafe { ftx_dict_lookup(self.0, k.as_ptr()) })
    }
    pub fn dict_lookup_nf(&self, key: &[u8]) -> Obj {
        let k = cstr(key);
        // SAFETY: as above.
        Obj::new(unsafe { ftx_dict_lookup_nf(self.0, k.as_ptr()) })
    }

    /// The dictionary of a stream (shared with it).
    pub fn stream_dict(&self) -> Obj {
        // SAFETY: as above.
        Obj::new(unsafe { ftx_stream_dict(self.0) })
    }

    /// `copyStream`'s bytes: the stream's own (decoded) bytes, or
    /// (`raw`) those of `getUndecodedStream()`.
    pub fn stream_bytes(&self, raw: bool) -> Vec<u8> {
        let mut len = 0usize;
        // SAFETY: the shim returns a malloc'ed buffer of `len` bytes (or
        // NULL), which we copy and free.
        unsafe {
            let p = ftx_stream_bytes(self.0, raw as c_int, &mut len);
            if p.is_null() {
                return Vec::new();
            }
            let v = std::slice::from_raw_parts(p, len).to_vec();
            ftx_free(p as *mut c_void);
            v
        }
    }
}

/// An xpdf `PDFDoc`.
pub struct Doc(*mut c_void);

impl Drop for Doc {
    fn drop(&mut self) {
        // SAFETY: we own the document.
        unsafe { ftx_doc_free(self.0) }
    }
}

impl Doc {
    /// `new PDFDoc(new GString(file_name))`.
    pub fn open(file_name: &[u8]) -> Doc {
        let n = cstr(file_name);
        // SAFETY: a NUL-terminated name; PDFDoc copies it.
        let p = unsafe { ftx_doc_open(n.as_ptr()) };
        assert!(!p.is_null(), "xpdf returned no document");
        Doc(p)
    }
    /// `isOk() && okToPrint()`.
    pub fn ok(&self) -> bool {
        // SAFETY: live document.
        unsafe { ftx_doc_ok(self.0) != 0 }
    }
    pub fn pdf_version(&self) -> f64 {
        // SAFETY: live document.
        unsafe { ftx_doc_pdf_version(self.0) }
    }
    pub fn num_pages(&self) -> i32 {
        // SAFETY: live document.
        unsafe { ftx_doc_num_pages(self.0) }
    }
    /// The page of named destination `name`: -1 if none, 0 if it is not a
    /// page.
    pub fn find_dest_page(&self, name: &[u8]) -> i32 {
        let n = cstr(name);
        // SAFETY: live document, NUL-terminated name.
        unsafe { ftx_doc_find_dest_page(self.0, n.as_ptr()) }
    }
    /// Box `which` (0 media, 1 crop, 2 bleed, 3 trim, 4 art) of page `n`:
    /// (x1, y1, x2, y2).
    pub fn page_box(&self, n: i32, which: i32) -> Option<[f64; 4]> {
        let mut b = [0f64; 4];
        // SAFETY: `b` has the four doubles the shim writes.
        (unsafe { ftx_page_box(self.0, n, which, b.as_mut_ptr()) } != 0).then_some(b)
    }
    pub fn page_rotate(&self, n: i32) -> i32 {
        // SAFETY: live document; the shim checks `n`.
        unsafe { ftx_page_rotate(self.0, n) }
    }
    pub fn page_group(&self, n: i32) -> Option<Obj> {
        // SAFETY: as above.
        Obj::wrap(unsafe { ftx_page_group(self.0, n) })
    }
    pub fn page_resources(&self, n: i32) -> Option<Obj> {
        // SAFETY: as above.
        Obj::wrap(unsafe { ftx_page_resources(self.0, n) })
    }
    pub fn page_contents(&self, n: i32) -> Obj {
        // SAFETY: as above.
        Obj::new(unsafe { ftx_page_contents(self.0, n) })
    }
    pub fn page_ref(&self, n: i32) -> Option<(i32, i32)> {
        let (mut num, mut gen) = (0, 0);
        // SAFETY: as above.
        (unsafe { ftx_page_ref(self.0, n, &mut num, &mut gen) } != 0).then_some((num, gen))
    }
    pub fn doc_info_nf(&self) -> Obj {
        // SAFETY: live document.
        Obj::new(unsafe { ftx_doc_info_nf(self.0) })
    }
    /// `xref->fetch(num, gen, &obj)`.
    pub fn fetch(&self, num: i32, gen: i32) -> Obj {
        // SAFETY: live document.
        Obj::new(unsafe { ftx_xref_fetch(self.0, num, gen) })
    }
    /// pdftoepdf.cc's `initDictFromDict`: a new dictionary with the
    /// (unfetched) entries of dictionary or stream `src`.
    pub fn dict_copy(&self, src: &Obj) -> Obj {
        // SAFETY: live document and object.
        Obj::new(unsafe { ftx_dict_copy(self.0, src.0) })
    }
    /// `GfxFont::makeFont(xref, tag, ref, fontdict->getDict())`.
    pub fn make_font(&self, tag: &[u8], r: (i32, i32), fontdict: &Obj) -> Option<Font> {
        let t = cstr(tag);
        // SAFETY: live document and object; NUL-terminated tag.
        let p = unsafe { ftx_font_make(self.0, t.as_ptr(), r.0, r.1, fontdict.0) };
        (!p.is_null()).then_some(Font(p))
    }
}

/// An xpdf `GfxFont`.
pub struct Font(*mut c_void);

impl Drop for Font {
    fn drop(&mut self) {
        // SAFETY: we own the font.
        unsafe { ftx_font_free(self.0) }
    }
}

impl Font {
    pub fn is_cid(&self) -> bool {
        // SAFETY: live font.
        unsafe { ftx_font_is_cid(self.0) != 0 }
    }
    /// `getCharName(i)` of an 8-bit font, or `None`.
    pub fn char_name(&self, i: i32) -> Option<Vec<u8>> {
        // SAFETY: live font; the shim checks the font kind and `i`.
        let p = unsafe { ftx_font_char_name(self.0, i) };
        (!p.is_null()).then(|| bytes_of(p))
    }
}
