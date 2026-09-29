//! The display-list writer: `display-list-v3` (docs/protocol/display-list-v3.md)
//! at every `\shipout`, for the preview (DESIGN.md §6.1).
//!
//! How it observes the engine, without changing anything TeX computes:
//!
//! * **Geometry** is read from the page's PDF content stream as pdfTeX's own
//!   traversal writes it. `pdfshipoutbegin` and `pdfshipoutend` (the C
//!   calls that bracket the stream in `pdf_ship_out`) start and finish a
//!   capture; `write_pdf`/`write_zip` hand over the bytes of each buffer
//!   flush in between ([`tap`]). The stream is then read as a PDF viewer
//!   reads it ([`interp`]), in exact decimal arithmetic, so the glyph and
//!   rule positions are the PDF's.
//! * **Provenance** comes from the hooks of `changes/displaylist.ch`: every
//!   node allocated gets the source position of the moment in a side table
//!   indexed like `mem` ([`Globals::dl_new_node`]), and the traversal marks
//!   the stream offset at which it outputs each node
//!   ([`Globals::dl_node`]), so each item takes the span of the node that
//!   drew it.
//! * **Links and destinations** are read from pdfTeX's own lists
//!   (`pdf_link_list`, `pdf_dest_list`) at the end of the page, where their
//!   rectangles are final.
//!
//! Nothing runs unless `FLASHTEX_DISPLAY_LIST` names a sink ([`init_from_env`]):
//! `fd:N` (an inherited file descriptor, as the engine host passes a pipe)
//! or a file path. The hooks then cost one relaxed atomic load.

pub mod fixed;
pub mod interp;

use crate::generated::Globals;
use crate::resolver::Format;
use fixed::{Fx, Mat};
use flashtex_display_list::frame::write_frame;
use flashtex_display_list::json::{s as js, Json};
use flashtex_display_list::kind;
use flashtex_display_list::page::{flags, Dest, Link, LinkKind, Page, StreamKind, NO_COLUMN};
use flashtex_display_list::resource::{Font, Sources};
use flashtex_display_list::sha256::{hex, sha256, Sha256};
use interp::Marker;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{
    AtomicBool, AtomicI32, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering,
};

static ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static DL: RefCell<Option<Box<State>>> = const { RefCell::new(None) };
}

/// Whether a display list is being written (in this process).
#[inline(always)]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

// eqtb locations the writer reads. They are pdftex.web's `count_base` and
// `int_base+mag_code` in this engine's layout (with changes/synctex.ch's
// extra integer parameter); `tests::eqtb_locations_match_the_translation`
// checks them against src/generated/ so that a regeneration cannot move
// them silently.
const COUNT_BASE: usize = 629128;
const MAG_LOC: usize = 629035;

/// Packed source location: file (16 bits, 0 = unknown), line (32), column
/// (16, [`NO_COLUMN`] = unknown).
type Loc = u64;

fn loc_pack(file: u32, line: u32, col: u16) -> Loc {
    ((file as u64 & 0xffff) << 48) | ((line as u64) << 16) | col as u64
}

fn loc_file(l: Loc) -> u32 {
    (l >> 48) as u32
}
fn loc_line(l: Loc) -> u32 {
    ((l >> 16) & 0xffff_ffff) as u32
}
fn loc_col(l: Loc) -> u16 {
    (l & 0xffff) as u16
}

struct Capture {
    form: bool,
    /// Page: ship-out index. Form: the `/Fm` number.
    id: u32,
    bytes: Vec<u8>,
    /// Where in the PDF buffer the stream's unread bytes start.
    buf_start: i32,
    markers: Vec<Marker>,
    last_loc: Loc,
    draft: bool,
}

struct State {
    sink: Option<Box<dyn Write>>,
    side: Vec<Loc>,
    files: Vec<Vec<u8>>,
    file_ids: HashMap<Vec<u8>, u32>,
    spans: HashMap<(u32, u32), u32>,
    new_sources: Sources,
    capture: Option<Capture>,
    pages: u32,
    fonts_sent: HashMap<u32, [u8; 32]>,
    images_sent: HashMap<u32, [u8; 32]>,
    widths: HashMap<u32, Option<Box<[i64; 256]>>>,
    have_fonts: Vec<[u8; 32]>,
    cwd: Option<std::path::PathBuf>,
}

/// Start writing display lists if `FLASHTEX_DISPLAY_LIST` asks for it
/// (before the engine allocates its first node). `FLASHTEX_DISPLAY_LIST_HAVE_FONTS`
/// lists font keys (hex, comma-separated) whose programs the reader holds.
pub fn init_from_env() {
    let Some(spec) = std::env::var_os("FLASHTEX_DISPLAY_LIST") else {
        return;
    };
    let spec = spec.to_string_lossy().into_owned();
    let sink: Box<dyn Write> = if let Some(fd) = spec.strip_prefix("fd:") {
        use std::os::fd::FromRawFd;
        let Ok(fd) = fd.parse::<i32>() else {
            eprintln!("FLASHTEX_DISPLAY_LIST: bad descriptor `{fd}'");
            return;
        };
        // The descriptor was inherited for exactly this.
        Box::new(std::io::BufWriter::with_capacity(1 << 16, unsafe {
            std::fs::File::from_raw_fd(fd)
        }))
    } else {
        match std::fs::File::create(&spec) {
            Ok(f) => Box::new(std::io::BufWriter::with_capacity(1 << 16, f)),
            Err(e) => {
                eprintln!("FLASHTEX_DISPLAY_LIST: {spec}: {e}");
                return;
            }
        }
    };
    let have_fonts = std::env::var("FLASHTEX_DISPLAY_LIST_HAVE_FONTS")
        .unwrap_or_default()
        .split(',')
        .filter_map(|h| {
            let h = h.trim();
            if h.len() != 64 {
                return None;
            }
            let mut k = [0u8; 32];
            for i in 0..32 {
                k[i] = u8::from_str_radix(&h[2 * i..2 * i + 2], 16).ok()?;
            }
            Some(k)
        })
        .collect();
    let st = State {
        sink: Some(sink),
        // calloc'd: pages of mem that never hold a node are never touched.
        side: vec![0; crate::generated::consts::mem_max as usize + 1],
        files: Vec::new(),
        file_ids: HashMap::new(),
        spans: HashMap::new(),
        new_sources: Sources::default(),
        capture: None,
        pages: 0,
        fonts_sent: HashMap::new(),
        images_sent: HashMap::new(),
        widths: HashMap::new(),
        have_fonts,
        cwd: std::env::current_dir().ok(),
    };
    let mut st = Box::new(st);
    SIDE_PTR.store(st.side.as_mut_ptr(), Ordering::Relaxed);
    SIDE_LEN.store(st.side.len(), Ordering::Relaxed);
    DL.with(|d| *d.borrow_mut() = Some(st));
    ENABLED.store(true, Ordering::Relaxed);
}

/// Flush the sink (the end of the run).
pub fn finish() {
    if !enabled() {
        return;
    }
    DL.with(|d| {
        if let Some(st) = d.borrow_mut().as_mut() {
            if let Some(w) = st.sink.as_mut() {
                let _ = w.flush();
            }
        }
    });
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    DL.with(|d| d.borrow_mut().as_mut().map(|st| f(st)))
}

impl State {
    fn send(&mut self, k: u8, body: &[u8]) {
        if let Some(w) = self.sink.as_mut() {
            if write_frame(w, k, body).is_err() {
                // The reader went away: stop writing, keep typesetting.
                self.sink = None;
            }
        }
    }
    fn flush(&mut self) {
        if let Some(w) = self.sink.as_mut() {
            if w.flush().is_err() {
                self.sink = None;
            }
        }
    }

    fn file_id(&mut self, name: &[u8]) -> u32 {
        if let Some(&i) = self.file_ids.get(name) {
            return i;
        }
        if self.files.len() >= 0x7ffe {
            return 0;
        }
        self.files.push(name.to_vec());
        let id = self.files.len() as u32;
        self.file_ids.insert(name.to_vec(), id);
        let mut path = String::from_utf8_lossy(name).into_owned();
        if !path.starts_with('/') {
            if let Some(cwd) = &self.cwd {
                let p = cwd.join(path.trim_start_matches("./"));
                path = p.to_string_lossy().into_owned();
            }
        }
        self.new_sources.files.push((id, path));
        id
    }

    fn span_id(&mut self, file: u32, line: u32) -> u32 {
        if file == 0 {
            return 0;
        }
        if let Some(&s) = self.spans.get(&(file, line)) {
            return s;
        }
        let id = self.spans.len() as u32 + 1;
        self.spans.insert((file, line), id);
        self.new_sources.spans.push((id, file, line));
        id
    }
}

// ---------------------------------------------------------------------------
// hooks (changes/displaylist.ch) and taps (pdfshipoutbegin/end, write_pdf)

// The side table and the location cache are read and written on TeX's
// inner loop (every node and token allocated), so they live in process
// statics rather than the thread-local state: a display list is written by
// one engine per process (the engine host runs one per compile).
static SIDE_PTR: AtomicPtr<Loc> = AtomicPtr::new(std::ptr::null_mut());
static SIDE_LEN: AtomicUsize = AtomicUsize::new(0);
/// The file-name string of the innermost file, and its file id.
static FILE_NAME: AtomicI32 = AtomicI32::new(-1);
static FILE_ID: AtomicU32 = AtomicU32::new(0);
/// While `hyphenate` rebuilds a word: HYPH_ON, and the word's place.
static HYPH_ON: AtomicBool = AtomicBool::new(false);
static HYPH_LOC: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
fn side_get(p: i32) -> Loc {
    let (ptr, len) = (
        SIDE_PTR.load(Ordering::Relaxed),
        SIDE_LEN.load(Ordering::Relaxed),
    );
    if ptr.is_null() || p < 0 || p as usize >= len {
        return 0;
    }
    // SAFETY: `ptr` is the side table's buffer (`State::side`, never
    // reallocated), `len` its length; only the engine's thread uses it.
    unsafe { *ptr.add(p as usize) }
}

#[inline(always)]
fn side_set(p: i32, v: Loc) {
    let (ptr, len) = (
        SIDE_PTR.load(Ordering::Relaxed),
        SIDE_LEN.load(Ordering::Relaxed),
    );
    if !ptr.is_null() && p >= 0 && (p as usize) < len {
        // SAFETY: as in `side_get`.
        unsafe { *ptr.add(p as usize) = v }
    }
}

impl Globals {
    /// `dl_new_node(p)`: node (or token) `p` was just allocated.
    #[inline(always)]
    pub fn dl_new_node(&mut self, p: i32) {
        // Tokens stored while a definition, a macro's arguments or a token
        // list are scanned (`scanner_status` other than `normal`) never
        // become nodes; they are most of the allocations, so they are
        // skipped before anything else is looked at.
        if enabled() && self.scanner_status == 0 {
            self.dl_note_node(p);
        }
    }

    #[inline(never)]
    fn dl_note_node(&mut self, p: i32) {
        let v = if HYPH_ON.load(Ordering::Relaxed) {
            HYPH_LOC.load(Ordering::Relaxed)
        } else if self.output_active {
            // Material the output routine makes (running heads, the page's
            // own boxes) has no place in the source.
            0
        } else {
            self.dl_here()
        };
        side_set(p, v);
    }

    /// The source position TeX is reading at: the innermost open file, its
    /// line, and the column after the last character read from it.
    fn dl_here(&mut self) -> Loc {
        let level = self.in_open;
        if level <= 0 {
            return 0;
        }
        let name = self.full_source_filename_stack[level as usize];
        if name == 0 {
            return 0;
        }
        let line = self.line.max(0) as u32;
        // The innermost input level that reads a file line.
        let is_file =
            |r: &crate::generated::types::in_state_record| r.state_field != 0 && r.name_field > 17;
        let rec = if is_file(&self.cur_input) {
            Some(self.cur_input)
        } else {
            (0..self.input_ptr as usize)
                .rev()
                .map(|k| self.input_stack[k])
                .find(|r| is_file(r))
        };
        let col = match rec {
            Some(r) if r.loc_field > r.start_field => {
                ((r.loc_field - r.start_field - 1).min(NO_COLUMN as i32 - 1)) as u16
            }
            Some(_) => 0,
            None => NO_COLUMN,
        };
        let file = if FILE_NAME.load(Ordering::Relaxed) == name {
            FILE_ID.load(Ordering::Relaxed)
        } else {
            let bytes = self.str_bytes(name);
            let f = with(|st| st.file_id(&bytes)).unwrap_or(0);
            FILE_NAME.store(name, Ordering::Relaxed);
            FILE_ID.store(f, Ordering::Relaxed);
            f
        };
        loc_pack(file, line, col)
    }

    /// `dl_copy(r, p)`: node `r` is a copy of node `p` (`copy_node_list`).
    #[inline(always)]
    pub fn dl_copy(&mut self, r: i32, p: i32) {
        if enabled() {
            side_set(r, side_get(p));
        }
    }

    /// `dl_hyph_begin(ha)`: `hyphenate` is about to rebuild the word that
    /// starts at node `ha`.
    pub fn dl_hyph_begin(&mut self, ha: i32) {
        if enabled() {
            HYPH_LOC.store(side_get(ha), Ordering::Relaxed);
            HYPH_ON.store(true, Ordering::Relaxed);
        }
    }

    pub fn dl_hyph_end(&mut self) {
        if enabled() {
            HYPH_ON.store(false, Ordering::Relaxed);
        }
    }

    /// `dl_node(p)`: the traversal is about to output node `p`.
    #[inline(always)]
    pub fn dl_node(&mut self, p: i32) {
        if enabled() {
            self.dl_mark(p);
        }
    }

    #[inline(never)]
    fn dl_mark(&mut self, p: i32) {
        let pdf_ptr = self.pdf_ptr;
        with(|st| {
            let loc = side_get(p);
            if loc == 0 {
                return;
            }
            let Some(cap) = st.capture.as_ref() else {
                return;
            };
            if loc == cap.last_loc {
                return;
            }
            let offset = cap.bytes.len() as i64 + (pdf_ptr - cap.buf_start) as i64;
            let span = st.span_id(loc_file(loc), loc_line(loc));
            let cap = st.capture.as_mut().unwrap();
            cap.last_loc = loc;
            cap.markers.push(Marker {
                offset: offset.clamp(0, u32::MAX as i64) as u32,
                span,
                col: loc_col(loc),
            });
        });
    }
}

/// The PDF buffer is about to be written out (`write_pdf`, `write_zip`):
/// the bytes of a stream being captured are taken first.
pub fn tap(g: &Globals) {
    if !enabled() {
        return;
    }
    with(|st| {
        if let Some(cap) = st.capture.as_mut() {
            let (a, b) = (cap.buf_start.max(0) as usize, g.pdf_ptr.max(0) as usize);
            if a < b {
                let buf = if g.pdf_buf_is_os {
                    &g.pdf_os_buf
                } else {
                    &g.pdf_op_buf
                };
                cap.bytes.extend(buf[a..b].iter().map(|&x| x as u8));
            }
            cap.buf_start = 0;
        }
    });
}

impl Globals {
    /// From `pdfshipoutbegin`: the page's (or form's) content stream starts.
    pub fn dl_shipout_begin(&mut self, shipping_page: bool) {
        if !enabled() {
            return;
        }
        let id = if shipping_page {
            with(|st| st.pages).unwrap_or(0)
        } else {
            self.obj_tab[self.pdf_cur_form as usize].int0 as u32
        };
        let draft = self.fixed_pdf_draftmode != 0;
        let start = self.pdf_ptr;
        with(|st| {
            st.capture = Some(Capture {
                form: !shipping_page,
                id,
                bytes: Vec::with_capacity(1 << 16),
                buf_start: start,
                markers: Vec::new(),
                last_loc: 0,
                draft,
            });
        });
    }

    /// From `pdfshipoutend`: the stream is complete (`pdf_end_text` has run).
    pub fn dl_shipout_end(&mut self, shipping_page: bool) {
        if !enabled() {
            return;
        }
        tap(self);
        let Some(Some(cap)) = with(|st| st.capture.take()) else {
            return;
        };
        if cap.form == shipping_page {
            return; // not the stream that began (cannot happen in pdfTeX)
        }
        self.dl_emit(cap);
    }

    fn dl_emit(&mut self, cap: Capture) {
        let saved_scaled_out = self.scaled_out;
        let kind = if cap.form {
            StreamKind::Form
        } else {
            StreamKind::Page
        };
        // The box: MediaBox [0 0 w h] (pdf_print_mag_bp) or BBox [0 0 w h+d]
        // (pdf_print_bp), as the numbers pdfTeX prints.
        let (wsp, hsp) = if cap.form {
            (
                self.pdf_xform_width,
                self.pdf_xform_height + self.pdf_xform_depth,
            )
        } else {
            (self.cur_page_width, self.cur_page_height)
        };
        let mag = self.eqtb[MAG_LOC - 1].int();
        let (bw, bh) = if cap.form {
            (self.dl_bp(wsp), self.dl_bp(hsp))
        } else {
            (self.dl_mag_bp(wsp, mag), self.dl_mag_bp(hsp, mag))
        };
        let ctm = if !cap.form && mag != 1000 {
            let m = Fx::from_int(mag as i64).mul_div(1, 1000);
            Mat([m, Fx::ZERO, Fx::ZERO, m, Fx::ZERO, Fx::ZERO])
        } else {
            Mat::IDENTITY
        };
        let mut env = WidthEnv {
            g: self,
            prefix: Vec::new(),
        };
        env.prefix = if env.g.pdf_resname_prefix != 0 {
            env.g.str_bytes(env.g.pdf_resname_prefix)
        } else {
            Vec::new()
        };
        let mut out = if cap.draft {
            let mut p = Page::new(kind, cap.id);
            p.flags |= flags::NO_GEOMETRY;
            interp::Output {
                page: p,
                fonts: vec![],
                images: vec![],
                forms: vec![],
            }
        } else {
            interp::interpret(&mut env, kind, cap.id, &cap.bytes, bh, ctm, &cap.markers)
        };
        let page = &mut out.page;
        page.width = wsp;
        page.height = hsp;
        page.pdf_box = [0.0, 0.0, bw.to_f64(), bh.to_f64()];
        if !cap.form {
            for k in 0..10 {
                page.counts[k] = self.eqtb[COUNT_BASE + k - 1].int();
            }
            self.dl_links(page, mag);
        }
        // Resources first: fonts, images, then the sources the items name.
        for &f in &out.fonts {
            self.dl_font(f);
        }
        for &n in &out.images {
            self.dl_image(n);
        }
        let (fonts, images) =
            with(|st| (st.fonts_sent.clone(), st.images_sent.clone())).unwrap_or_default();
        let fk = |f: u16| fonts.get(&(f as u32)).copied().unwrap_or([0; 32]);
        let ik = |i: u32| images.get(&i).copied().unwrap_or([0; 32]);
        out.page.hash = out.page.content_hash(&fk, &ik);
        let body = out.page.encode();
        self.scaled_out = saved_scaled_out;
        with(|st| {
            if !st.new_sources.files.is_empty() || !st.new_sources.spans.is_empty() {
                let src = std::mem::take(&mut st.new_sources);
                st.send(kind::SOURCES, src.to_json().to_string().as_bytes());
            }
            st.send(if cap.form { kind::FORM } else { kind::PAGE }, &body);
            st.flush();
            if !cap.form {
                st.pages += 1;
            }
        });
    }

    /// `pdf_print_bp(s)`'s number, exactly.
    fn dl_bp(&mut self, s: i32) -> Fx {
        let d = self.fixed_decimal_digits;
        let v = self.divide_scaled(s, self.one_hundred_bp, d + 2);
        Fx(v as i128 * 10i128.pow((12 - d) as u32))
    }

    /// `pdf_print_mag_bp(s)`'s number.
    fn dl_mag_bp(&mut self, s: i32, mag: i32) -> Fx {
        let s = if mag != 1000 {
            self.round_xn_over_d(s, mag, 1000)
        } else {
            s
        };
        self.dl_bp(s)
    }
}

struct WidthEnv<'a> {
    g: &'a mut Globals,
    prefix: Vec<u8>,
}

impl interp::Env for WidthEnv<'_> {
    fn width(&mut self, font: u32, code: u8) -> Option<i64> {
        let g = &mut *self.g;
        if let Some(t) = with(|st| {
            st.widths
                .get(&font)
                .map(|t| t.as_ref().map(|w| w[code as usize]))
        })
        .flatten()
        {
            return t;
        }
        if font == 0 || font as i32 > g.font_ptr {
            return None;
        }
        // writefont.c's /Widths: divide_scaled(get_charwidth(f, i),
        // pdf_font_size[f], 4) tenths, f the font that owns /F<font>.
        let f = font as i32;
        let size = g.pdf_font_size[f as usize];
        let table = (size != 0).then(|| {
            let mut t = Box::new([0i64; 256]);
            for c in 0..256 {
                let w = g.get_charwidth(f, c);
                t[c as usize] = g.divide_scaled(w, size, 4) as i64;
            }
            t
        });
        let w = table.as_ref().map(|t| t[code as usize]);
        with(|st| st.widths.insert(font, table));
        w
    }
    fn resname_prefix(&self) -> &[u8] {
        &self.prefix
    }
}

// ---------------------------------------------------------------------------
// links and destinations

impl Globals {
    fn m_rh(&self, p: i32) -> i32 {
        self.mem[p as usize].hh().rh()
    }
    fn m_lh(&self, p: i32) -> i32 {
        self.mem[p as usize].hh().lh()
    }
    fn m_b0(&self, p: i32) -> i32 {
        self.mem[p as usize].hh().b0()
    }
    fn m_b1(&self, p: i32) -> i32 {
        self.mem[p as usize].hh().b1()
    }
    fn m_int(&self, p: i32) -> i32 {
        self.mem[p as usize].int()
    }

    /// `tokens_to_string(p)`'s bytes, with the string pool as it was.
    fn dl_tokens(&mut self, p: i32) -> Vec<u8> {
        if p == 0 {
            return Vec::new();
        }
        let saved_last = self.last_tokens_string;
        let s = self.tokens_to_string(p);
        let b = self.str_bytes(s);
        self.flush_str(s);
        self.last_tokens_string = saved_last;
        b
    }

    fn dl_links(&mut self, page: &mut Page, mag: i32) {
        let scale = |v: i32, g: &mut Globals| {
            if mag != 1000 {
                g.round_xn_over_d(v, mag, 1000)
            } else {
                v
            }
        };
        let mut k = self.pdf_link_list;
        let mut guard = 0;
        while k != 0 && guard < 1_000_000 {
            guard += 1;
            let objnum = self.m_lh(k);
            let i = self.obj_tab[objnum as usize].int4; // obj_annot_ptr
            let rect = [
                scale(self.m_int(i + 1), self),
                scale(self.m_int(i + 2), self),
                scale(self.m_int(i + 3), self),
                scale(self.m_int(i + 4), self),
            ];
            let a = self.m_rh(i + 5); // pdf_link_action
            let span = with(|st| {
                let l = side_get(i);
                st.span_id(loc_file(l), loc_line(l))
            })
            .unwrap_or(0);
            let typ = self.m_b0(a);
            let named = self.m_b1(a);
            let id = self.m_rh(a);
            let file = self.dl_tokens(self.m_lh(a + 1));
            let (lk, data) = match typ {
                0 => {
                    // page: "N" + the page spec
                    let spec = self.dl_tokens(self.m_lh(a + 2));
                    let mut d = id.to_string().into_bytes();
                    d.push(b' ');
                    d.extend_from_slice(&spec);
                    (LinkKind::GotoPage, d)
                }
                1 if named % 2 == 1 => (LinkKind::GotoName, self.dl_tokens(id)),
                1 => (LinkKind::GotoNum, id.to_string().into_bytes()),
                2 => (
                    LinkKind::Thread,
                    if named % 2 == 1 {
                        self.dl_tokens(id)
                    } else {
                        id.to_string().into_bytes()
                    },
                ),
                _ => {
                    let raw = self.dl_tokens(self.m_lh(a + 2));
                    match uri_of(&raw) {
                        Some(u) => (LinkKind::Uri, u),
                        None => (LinkKind::Raw, raw),
                    }
                }
            };
            page.links.push(Link {
                rect,
                span,
                kind: lk,
                file,
                data,
            });
            k = self.m_rh(k);
        }
        let mut k = self.pdf_dest_list;
        guard = 0;
        while k != 0 && guard < 1_000_000 {
            guard += 1;
            let objnum = self.m_lh(k);
            let i = self.obj_tab[objnum as usize].int4; // obj_dest_ptr
            let named = self.m_b1(i + 5) > 0;
            let id = self.m_rh(i + 5);
            let name = if named {
                self.dl_tokens(id)
            } else {
                id.to_string().into_bytes()
            };
            let zoom = self.m_lh(i + 6);
            let rect = [
                scale(self.m_int(i + 1), self),
                scale(self.m_int(i + 2), self),
                scale(self.m_int(i + 3), self),
                scale(self.m_int(i + 4), self),
            ];
            page.dests.push(Dest {
                named,
                name,
                kind: self.m_b0(i + 5) as u8,
                rect,
                zoom,
            });
            k = self.m_rh(k);
        }
    }
}

/// A path the engine found (maybe relative to its working directory), made
/// absolute.
fn absolute(name: &[u8]) -> String {
    let p = String::from_utf8_lossy(name).into_owned();
    if p.starts_with('/') {
        return p;
    }
    match std::env::current_dir() {
        Ok(d) => d
            .join(p.trim_start_matches("./"))
            .to_string_lossy()
            .into_owned(),
        Err(_) => p,
    }
}

/// The URI of a `/S/URI/URI(...)` action, unescaped.
fn uri_of(raw: &[u8]) -> Option<Vec<u8>> {
    let p = raw.windows(5).position(|w| w == b"/URI(")?;
    let mut out = Vec::new();
    let mut depth = 1;
    let mut i = p + 5;
    while i < raw.len() {
        let c = raw[i];
        match c {
            b'\\' if i + 1 < raw.len() => {
                i += 1;
                out.push(match raw[i] {
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b't' => b'\t',
                    o => o,
                });
            }
            b'(' => {
                depth += 1;
                out.push(c);
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(out);
                }
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    None
}

// ---------------------------------------------------------------------------
// resources

impl Globals {
    /// Send font `/F<f>` if this compile has not yet.
    fn dl_font(&mut self, f: u32) {
        if with(|st| st.fonts_sent.contains_key(&f)).unwrap_or(true) {
            return;
        }
        let fi = f as i32;
        let tex_name = if fi <= self.font_ptr {
            self.str_bytes(self.font_name[fi as usize])
        } else {
            Vec::new()
        };
        let tex_size = if fi <= self.font_ptr {
            self.font_size[fi as usize]
        } else {
            0
        };
        let ptr = if fi <= self.font_ptr {
            self.pdf_font_map[fi as usize]
        } else {
            0
        };
        let fm = if ptr > 0 {
            self.with_fonts(|_, st| Some(st.map.fm(ptr).clone()))
        } else {
            None
        };
        let mut format = "none";
        let mut program = Vec::new();
        let mut file = None;
        let mut names: Option<Vec<Vec<u8>>> = None;
        let (mut slant, mut extend, mut ps_name) = (0, 0, Vec::new());
        if let Some(fm) = &fm {
            slant = fm.slant;
            extend = fm.extend;
            ps_name = fm.ps_name.clone().unwrap_or_default();
            if fm.is_type1() && fm.is_included() {
                if let Some(ff) = &fm.ff_name {
                    let name = String::from_utf8_lossy(ff).into_owned();
                    if let Some(path) = crate::system::find_file(&name, Format::Type1) {
                        if let Ok(data) = std::fs::read(&path) {
                            program = data;
                            format = "type1";
                            file = Some(path);
                        }
                    }
                }
            } else if fm.is_truetype() {
                format = "truetype";
            } else if fm.is_opentype() {
                format = "opentype";
            } else if fm.is_pk() {
                format = "type3";
            }
            if let Some(enc) = &fm.encname {
                names = read_enc(&String::from_utf8_lossy(enc));
            }
        }
        if names.is_none() && format == "type1" {
            names = Some(builtin_encoding(&program));
        }
        let mut h = Sha256::new();
        h.update(b"display-list-v3 font\0");
        h.update(format.as_bytes());
        h.update(&[0]);
        let program_sha = sha256(&program);
        h.update(&program_sha);
        if let Some(n) = &names {
            for g in n {
                h.update(g);
                h.update(&[0]);
            }
        }
        h.update(&slant.to_le_bytes());
        h.update(&extend.to_le_bytes());
        let key = h.finish();
        let prefix = if self.pdf_resname_prefix != 0 {
            self.str_bytes(self.pdf_resname_prefix)
        } else {
            Vec::new()
        };
        let lossy = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        let info = Json::Obj(vec![
            ("pdf_name".into(), js(format!("F{f}{}", lossy(&prefix)))),
            ("tex_name".into(), js(lossy(&tex_name))),
            ("tex_size".into(), Json::Int(tex_size as i64)),
            ("ps_name".into(), js(lossy(&ps_name))),
            ("format".into(), js(format)),
            ("file".into(), file.map(js).unwrap_or(Json::Null)),
            ("program_sha256".into(), js(hex(&program_sha))),
            ("program_bytes".into(), Json::Int(program.len() as i64)),
            ("slant".into(), Json::Int(slant as i64)),
            ("extend".into(), Json::Int(extend as i64)),
            (
                "font_matrix".into(),
                if slant != 0 || extend != 0 {
                    font_matrix(&program, slant, extend)
                        .map(js)
                        .unwrap_or(Json::Null)
                } else {
                    Json::Null
                },
            ),
            (
                "encoding".into(),
                names
                    .map(|n| Json::Arr(n.iter().map(|g| js(lossy(g))).collect()))
                    .unwrap_or(Json::Null),
            ),
        ]);
        with(|st| {
            let skip = st.have_fonts.contains(&key);
            let font = Font {
                id: f as u16,
                key,
                info,
                program: if skip { Vec::new() } else { program },
            };
            st.send(kind::FONT, &font.encode());
            st.fonts_sent.insert(f, key);
        });
    }

    /// Send image `/Im<n>` if this compile has not yet.
    fn dl_image(&mut self, n: u32) {
        if with(|st| st.images_sent.contains_key(&n)).unwrap_or(true) {
            return;
        }
        // The page's ximage list names the object; obj_info is n.
        let mut k = self.pdf_ximage_list;
        let mut img = None;
        while k != 0 {
            let obj = self.m_lh(k);
            if self.obj_tab[obj as usize].int0 as u32 == n {
                let data_ptr = self.obj_tab[obj as usize].int4;
                img = Some(self.pdf_mem[(data_ptr + 4) as usize]);
                break;
            }
            k = self.m_rh(k);
        }
        let Some(img) = img else { return };
        let info = self.dl_image_info(img);
        let mut kv = vec![("id".to_string(), Json::Int(n as i64))];
        let file = info.str_field("file").map(str::to_string);
        let (size, mtime) = file
            .as_deref()
            .and_then(|f| std::fs::metadata(f).ok())
            .map(|m| {
                let t = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok());
                (m.len(), t.map(|d| d.as_nanos()).unwrap_or(0))
            })
            .unwrap_or((0, 0));
        let mut h = Sha256::new();
        h.update(b"display-list-v3 image\0");
        h.update(info.to_string().as_bytes());
        h.update(&size.to_le_bytes());
        h.update(&mtime.to_le_bytes());
        let key = h.finish();
        kv.push(("key".into(), js(hex(&key))));
        if let Json::Obj(rest) = info {
            kv.extend(rest);
        }
        let body = Json::Obj(kv).to_string();
        with(|st| {
            st.send(kind::IMAGE, body.as_bytes());
            st.images_sent.insert(n, key);
        });
    }

    fn dl_image_info(&mut self, img: i32) -> Json {
        use crate::pdftex::images::ImageData;
        crate::pdftex::with_state(|st| {
            let Some(e) = st.img.images.get(img as usize) else {
                return Json::Null;
            };
            let (typ, extra) = match &e.data {
                ImageData::Pdf(p) => (
                    "pdf",
                    vec![
                        ("page".to_string(), Json::Int(p.selected_page as i64)),
                        (
                            "page_box".to_string(),
                            js(match p.page_box {
                                1 => "media",
                                2 => "crop",
                                3 => "bleed",
                                4 => "trim",
                                5 => "art",
                                _ => "crop",
                            }),
                        ),
                        ("orig_x".to_string(), Json::Int(p.orig_x as i64)),
                        ("orig_y".to_string(), Json::Int(p.orig_y as i64)),
                    ],
                ),
                ImageData::Png(_) => ("png", vec![]),
                ImageData::Jpg(_) => ("jpeg", vec![]),
                ImageData::Jbig2(_) => ("jbig2", vec![]),
                ImageData::None => ("none", vec![]),
            };
            let mut kv = vec![
                ("type".to_string(), js(typ)),
                (
                    "file".to_string(),
                    e.name
                        .as_ref()
                        .map(|n| js(absolute(n)))
                        .unwrap_or(Json::Null),
                ),
                ("width".to_string(), Json::Int(e.width as i64)),
                ("height".to_string(), Json::Int(e.height as i64)),
                ("rotate".to_string(), Json::Int(e.rotate as i64)),
                ("x_res".to_string(), Json::Int(e.x_res as i64)),
                ("y_res".to_string(), Json::Int(e.y_res as i64)),
            ];
            kv.extend(extra);
            Json::Obj(kv)
        })
    }
}

/// The `/FontMatrix` pdfTeX writes into the embedded font when the map
/// entry slants or extends it (writet1.c's `t1_modify_fm`: the font's own
/// matrix, slanted then x-scaled in C `float`, each entry printed `%g`), as
/// the PDF's text: "a b c d e f".
fn font_matrix(program: &[u8], slant: i32, extend: i32) -> Option<String> {
    let clear = cleartext(program);
    let p = clear.windows(11).position(|w| w == b"/FontMatrix")?;
    let rest = &clear[p + 11..];
    let open = rest.iter().position(|&c| c == b'[' || c == b'{')?;
    let close = rest.iter().position(|&c| c == b']' || c == b'}')?;
    let nums: Vec<f32> = std::str::from_utf8(rest.get(open + 1..close)?)
        .ok()?
        .split_whitespace()
        .filter_map(|t| t.parse::<f32>().ok())
        .collect();
    let mut a: [f32; 6] = nums.try_into().ok()?;
    if slant != 0 {
        let s = slant as f64 * 1E-3;
        a[0] = (a[0] as f64 + a[1] as f64 * s) as f32;
        a[2] = (a[2] as f64 + a[3] as f64 * s) as f32;
        a[4] = (a[4] as f64 + a[5] as f64 * s) as f32;
    }
    if extend != 0 {
        let e = extend as f64 * 1E-3;
        a[0] = (a[0] as f64 * e) as f32;
        a[2] = (a[2] as f64 * e) as f32;
        a[4] = (a[4] as f64 * e) as f32;
    }
    let parts: Vec<String> = a
        .iter()
        .map(|&x| String::from_utf8_lossy(&crate::pdftex::cfmt::fmt_g(x as f64)).into_owned())
        .collect();
    Some(parts.join(" "))
}

/// The clear-text part of a Type 1 program (PFB segment 1, or a PFA up to
/// `eexec`).
fn cleartext(program: &[u8]) -> &[u8] {
    if program.first() == Some(&0x80) && program.len() > 6 {
        let len = u32::from_le_bytes([program[2], program[3], program[4], program[5]]) as usize;
        &program[6..(6 + len).min(program.len())]
    } else {
        let end = program
            .windows(5)
            .position(|w| w == b"eexec")
            .unwrap_or(program.len());
        &program[..end]
    }
}

/// The glyph names of an encoding file (`/Name [ /a /b ... ] def`), read as
/// writet1.c's `load_enc_file` reads them (without its log output).
fn read_enc(name: &str) -> Option<Vec<Vec<u8>>> {
    let path = crate::system::find_file(name, Format::Enc)?;
    let data = std::fs::read(path).ok()?;
    let mut out = vec![b".notdef".to_vec(); 256];
    // Strip comments, then take the names between the first `[` and `]`.
    let mut text = Vec::with_capacity(data.len());
    let mut in_comment = false;
    for &c in &data {
        match c {
            b'%' => in_comment = true,
            b'\n' | b'\r' => {
                in_comment = false;
                text.push(b' ');
            }
            _ if !in_comment => text.push(c),
            _ => {}
        }
    }
    let open = text.iter().position(|&c| c == b'[')?;
    let close = open + text[open..].iter().position(|&c| c == b']')?;
    let mut i = 0;
    for tok in text[open + 1..close].split(|&c| c == b'/' || c == b' ' || c == b'\t') {
        if tok.is_empty() {
            continue;
        }
        if i < 256 {
            out[i] = tok.to_vec();
        }
        i += 1;
    }
    Some(out)
}

/// A Type 1 font's built-in `/Encoding` (from the clear-text part of a
/// PFB or PFA): `StandardEncoding` or `dup <code> /<name> put` entries.
fn builtin_encoding(program: &[u8]) -> Vec<Vec<u8>> {
    let clear = cleartext(program);
    let mut out = vec![b".notdef".to_vec(); 256];
    let Some(p) = clear.windows(9).position(|w| w == b"/Encoding") else {
        return out;
    };
    let rest = &clear[p + 9..];
    let head: Vec<u8> = rest
        .iter()
        .copied()
        .skip_while(|c| c.is_ascii_whitespace())
        .take(16)
        .collect();
    if head.starts_with(b"StandardEncoding") {
        for (i, g) in out.iter_mut().enumerate() {
            let n = crate::pdftex::writet1::standard_glyph_name(i);
            if n != b".notdef" {
                *g = n.to_vec();
            }
        }
        return out;
    }
    // The vector ends at the first `def` token (`/.notdef` is not one).
    let toks: Vec<&[u8]> = rest
        .split(|&c| c.is_ascii_whitespace() || c == b'[' || c == b']')
        .filter(|t| !t.is_empty())
        .take_while(|t| *t != b"def")
        .collect();
    let array_form = rest.iter().find(|c| !c.is_ascii_whitespace()) == Some(&b'[');
    if array_form {
        let mut i = 0;
        for t in toks
            .iter()
            .flat_map(|t| t.split(|&c| c == b'/'))
            .filter(|t| !t.is_empty())
        {
            if t == b"readonly" {
                continue;
            }
            if i < 256 {
                out[i] = t.to_vec();
            }
            i += 1;
        }
        return out;
    }
    for w in toks.windows(4) {
        if w[0] == b"dup" && w[3] == b"put" && w[2].starts_with(b"/") {
            if let Ok(code) = std::str::from_utf8(w[1]).unwrap_or("x").parse::<usize>() {
                if code < 256 {
                    out[code] = w[2][1..].to_vec();
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The eqtb locations above are the translation's: `pdf_ship_out`
    /// prints `count(k)` and `pdf_print_mag_bp` reads `mag` there.
    #[test]
    fn eqtb_locations_match_the_translation() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
        let mut all = String::new();
        for e in std::fs::read_dir(dir).unwrap() {
            all.push_str(&std::fs::read_to_string(e.unwrap().path()).unwrap());
        }
        assert!(all.contains(&format!(
            "self.print_int(((self.eqtb[((({COUNT_BASE}i32).wrapping_add(k)) - 1)"
        )));
        let mag_bp = all.split("pub fn pdf_print_mag_bp").nth(1).unwrap();
        assert!(mag_bp[..400].contains(&format!(
            "self.eqtb[(({MAG_LOC}i32) - 1) as usize].int() != 1000i32"
        )));
    }

    #[test]
    fn locations_pack() {
        let l = loc_pack(3, 1234567, 42);
        assert_eq!((loc_file(l), loc_line(l), loc_col(l)), (3, 1234567, 42));
        assert_eq!(
            uri_of(b"/Subtype/Link/A<</S/URI/URI(http://x.org/a\\(b\\))>>").unwrap(),
            b"http://x.org/a(b)"
        );
    }

    #[test]
    fn encodings() {
        let pfa = b"%!PS\n/Encoding 256 array\n0 1 255 {1 index exch /.notdef put} for\ndup 65 /A put\ndup 97 /a put\nreadonly def\ncurrentfile eexec\n";
        let e = builtin_encoding(pfa);
        assert_eq!(e[65], b"A");
        assert_eq!(e[97], b"a");
        assert_eq!(e[66], b".notdef");
        let std = builtin_encoding(b"/Encoding StandardEncoding def\n");
        assert_eq!(std[65], b"A");
        let fm = b"/FontMatrix [0.001 0 0 0.001 0 0]readonly def\ncurrentfile eexec";
        assert_eq!(
            font_matrix(fm, 167, 0).as_deref(),
            Some("0.001 0 0.000167 0.001 0 0")
        );
        assert_eq!(
            font_matrix(fm, 0, 850).as_deref(),
            Some("0.00085 0 0 0.001 0 0")
        );
    }
}
