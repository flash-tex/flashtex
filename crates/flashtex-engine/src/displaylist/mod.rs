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
//!   node allocated gets the source span (file and line) and column of the
//!   moment in a side table indexed like `mem` ([`Globals::dl_new_node`]),
//!   and the traversal marks the stream offset at which it outputs each node
//!   ([`Globals::dl_node`]), so each item takes the span of the node that
//!   drew it.
//! * **Links and destinations** are read from pdfTeX's own lists
//!   (`pdf_link_list`, `pdf_dest_list`) at the end of the page, where their
//!   rectangles are final.
//!
//! **Where the pages go** ([`Sink`]): with `FLASHTEX_DISPLAY_LIST`
//! ([`init_from_env`]: `fd:N`, an inherited descriptor; `socket:PATH` or
//! `pipe:NAME`, a listening endpoint; or a file path) the
//! frames are written as the engine ships each page out, with the fonts,
//! images and sources each page needs before it. The engine host
//! (`crate::host::server`) installs its own sink ([`init_with_sink`]) and
//! serves the same frames to its clients ([`Peer`]). Nothing runs unless one
//! of them was set up; the hooks then cost one relaxed atomic load.
//!
//! **Checkpoints** (DESIGN.md §5.2): the side table is engine state that
//! lives outside the word space, like pdfTeX's C parts, so it travels in
//! their snapshot (`crate::pdftex::CState`, [`snapshot`]/[`restore`]) as
//! copy-on-write chunks. What the writer caches from engine state (font and
//! image keys by id, glyph widths, the file-name cache) is dropped at every
//! restore. File and span ids, and resources by key, are names: they
//! outlive restores and compiles, so a page kept from an earlier compile
//! still means what it meant ([`move_lines`] keeps spans on their lines
//! across an edit).

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
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

static ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static DL: RefCell<Option<Box<State>>> = const { RefCell::new(None) };
    /// Where emitted pages go (apart from `DL`, so that a sink may call
    /// back into the writer: [`Peer::send`] reads resources).
    static SINK: RefCell<Option<Box<dyn Sink>>> = const { RefCell::new(None) };
    /// Nanoseconds this thread spent turning shipped streams into display
    /// lists (`dl_emit` before the sink), for the host's stage timings.
    static EMIT_NS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Nanoseconds this thread has spent building display lists (interpreting
/// the shipped content stream, encoding the page), not counting the sink.
pub fn emit_ns() -> u64 {
    EMIT_NS.with(|c| c.get())
}

/// Whether a display list is being written (in this process).
#[inline(always)]
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

// eqtb locations the writer reads: pdftex.web's `count_base` and
// `int_base+mag_code`, taken from the translation (web2rust emits WEB's
// macros as constants), so a regeneration moves them too.
// `tests::eqtb_locations_match_the_translation` checks that `pdf_ship_out`
// and `pdf_print_mag_bp` still read them.
const COUNT_BASE: usize = crate::generated::consts::count_base as usize;
const MAG_LOC: usize =
    (crate::generated::consts::int_base + crate::generated::consts::mag_code) as usize;

/// Packed source location: span (32 bits, 0 = none) and column (16,
/// [`NO_COLUMN`] = unknown).
type Loc = u64;

fn loc_pack(span: u32, col: u16) -> Loc {
    ((span as u64) << 16) | col as u64
}

fn loc_span(l: Loc) -> u32 {
    (l >> 16) as u32
}
fn loc_col(l: Loc) -> u16 {
    (l & 0xffff) as u16
}

struct Capture {
    form: bool,
    /// Page: its 0-based index. Form: the `/Fm` number.
    id: u32,
    bytes: Vec<u8>,
    /// Where in the PDF buffer the stream's unread bytes start.
    buf_start: i32,
    markers: Vec<Marker>,
    last_loc: Loc,
    draft: bool,
}

/// A font resource, by key.
struct FontRes {
    info: Json,
    program: Arc<Vec<u8>>,
    /// Its `format`.
    format: &'static str,
    /// Why its glyphs cannot be drawn from the display list, if they
    /// cannot (a Type 3 font without its bitmaps).
    problem: Option<String>,
}

/// The `format`s every reader of 3.1 knows; the others (`truetype`,
/// `opentype`, `type3`) carry a program only for a reader that lists them
/// in `COMPILE.font_formats` (docs/protocol/display-list-v3.md §5.1).
const BASE_FONT_FORMATS: &[&str] = &["type1", "none"];

/// A Type 3 font's advances: per code the numerator, and the denominator
/// (`None`: not a font the writer can place).
type Advances = Option<(Box<[i64; 256]>, i64)>;

/// How pdfTeX writes font `/F<n>` (writefont.c's `dopdffont`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FontKind {
    /// A map entry with a font file or a built-in font.
    Mapped,
    /// Type 3 from a PK file (no map entry, or a bitmap entry).
    Pk,
    /// Type 3 from a `.pgc` file.
    Pgc,
}

/// A font file's program, as last read.
struct Program {
    len: u64,
    mtime: Option<std::time::SystemTime>,
    data: Arc<Vec<u8>>,
    sha: [u8; 32],
}

struct State {
    // Names: they outlive restores and compiles.
    files: Vec<Vec<u8>>,
    file_paths: Vec<String>,
    file_ids: HashMap<Vec<u8>, u32>,
    /// Span id - 1 -> (file, line).
    spans: Vec<(u32, u32)>,
    /// (file, line) -> the span of that line: the lowest live span there.
    span_ids: SpanIndex,
    /// Spans of lines an edit replaced (`move_lines`): kept, never reused.
    /// A bit per span id.
    retired: Vec<u64>,
    // Resources by key: descriptions that do not depend on engine state.
    fonts: HashMap<[u8; 32], FontRes>,
    images: HashMap<[u8; 32], Json>,
    programs: HashMap<String, Program>,
    encodings: HashMap<String, Option<Arc<Vec<Vec<u8>>>>>,
    /// Type 1 programs' built-in encodings, by the program's SHA-256.
    builtin_encodings: HashMap<[u8; 32], Arc<Vec<Vec<u8>>>>,
    // Engine state as the writer saw it: dropped at every restore.
    font_keys: HashMap<u32, [u8; 32]>,
    image_keys: HashMap<u32, [u8; 32]>,
    widths: HashMap<u32, Option<Box<[i64; 256]>>>,
    /// Type 3 (PK) fonts' advances: numerators and their denominator.
    advances: HashMap<u32, Advances>,
    font_kinds: HashMap<u32, FontKind>,
    capture: Option<Capture>,
    cwd: Option<std::path::PathBuf>,
    /// The last page or form emitted at each index, with what it was made
    /// from (`Memo`): a stream shipped again from the same inputs -- a
    /// document's next `.aux` pass, the unchanged pages a restart re-runs --
    /// is the same display list, which is then not built again. Names, like
    /// the spans: it outlives restores (each entry checks itself).
    memo: HashMap<(bool, u32), Memo>,
}

impl State {
    fn new() -> State {
        State {
            files: Vec::new(),
            file_paths: Vec::new(),
            file_ids: HashMap::new(),
            spans: Vec::new(),
            span_ids: SpanIndex::default(),
            retired: Vec::new(),
            fonts: HashMap::new(),
            images: HashMap::new(),
            programs: HashMap::new(),
            encodings: HashMap::new(),
            builtin_encodings: HashMap::new(),
            font_keys: HashMap::new(),
            image_keys: HashMap::new(),
            widths: HashMap::new(),
            advances: HashMap::new(),
            font_kinds: HashMap::new(),
            capture: None,
            cwd: std::env::current_dir().ok(),
            memo: HashMap::new(),
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
        self.file_paths.push(path);
        id
    }

    fn span_id(&mut self, file: u32, line: u32) -> u32 {
        if file == 0 {
            return 0;
        }
        if let Some(s) = self.span_ids.get(file, line) {
            return s;
        }
        self.spans.push((file, line));
        let id = self.spans.len() as u32;
        self.span_ids.insert_new(file, line, id);
        id
    }

    fn is_retired(&self, id: u32) -> bool {
        self.retired
            .get(id as usize / 64)
            .is_some_and(|w| w >> (id % 64) & 1 == 1)
    }

    fn retire(&mut self, id: u32) {
        let i = id as usize / 64;
        if self.retired.len() <= i {
            self.retired.resize(i + 1, 0);
        }
        self.retired[i] |= 1 << (id % 64);
    }
}

/// `State::span_ids`: for each file, a table by line (lines below
/// [`SpanIndex::DENSE`]), and a map for the lines past it. `move_lines`
/// re-keys every span after an edit, which a hashed (file, line) map made
/// a hash removal and insertion per span: 0.7 ms a keystroke in the middle
/// of a 1,000-page document (lane P4-PAGE-COST).
#[derive(Default)]
struct SpanIndex {
    /// File id - 1 -> line -> span id (0: none).
    dense: Vec<Vec<u32>>,
    far: HashMap<(u32, u32), u32>,
}

impl SpanIndex {
    const DENSE: u32 = 1 << 20;

    fn get(&self, file: u32, line: u32) -> Option<u32> {
        if line >= Self::DENSE {
            return self.far.get(&(file, line)).copied();
        }
        let id = *self.dense.get(file as usize - 1)?.get(line as usize)?;
        (id != 0).then_some(id)
    }

    /// Key (file, line) to `id` unless it has a span already.
    fn insert_new(&mut self, file: u32, line: u32, id: u32) {
        if line >= Self::DENSE {
            self.far.entry((file, line)).or_insert(id);
            return;
        }
        let f = file as usize - 1;
        if self.dense.len() <= f {
            self.dense.resize_with(f + 1, Vec::new);
        }
        let t = &mut self.dense[f];
        if t.len() <= line as usize {
            t.resize(line as usize + 1, 0);
        }
        if t[line as usize] == 0 {
            t[line as usize] = id;
        }
    }

    /// Drop every key of `file` on a line from `from` on.
    fn clear_from(&mut self, file: u32, from: u32) {
        if let Some(t) = self.dense.get_mut(file as usize - 1) {
            t.truncate(from as usize);
        }
        if !self.far.is_empty() {
            self.far.retain(|&(f, l), _| f != file || l < from);
        }
    }

    #[cfg(test)]
    fn to_map(&self) -> HashMap<(u32, u32), u32> {
        let mut m = self.far.clone();
        for (f, t) in self.dense.iter().enumerate() {
            for (l, &id) in t.iter().enumerate() {
                if id != 0 {
                    m.insert((f as u32 + 1, l as u32), id);
                }
            }
        }
        m
    }
}

// ---------------------------------------------------------------------------
// set-up, sinks and peers

/// One page or form the writer produced: the encoded `PAGE`/`FORM` body and
/// what a reader needs before it.
#[derive(Clone, Debug)]
pub struct Emitted {
    pub form: bool,
    /// Page: its 0-based index in the document (pdfTeX's `total_pages` at
    /// ship-out, which a checkpoint restores). Form: its `/Fm` number.
    pub index: u32,
    pub body: Arc<Vec<u8>>,
    /// The content hash the body carries (spec §4.6).
    pub hash: [u8; 32],
    /// Font and image ids the items use, with their keys, in order of
    /// first use; forms the items draw; spans the items and links name.
    pub fonts: Vec<(u32, [u8; 32])>,
    pub images: Vec<(u32, [u8; 32])>,
    pub forms: Vec<u32>,
    pub spans: Vec<u32>,
}

/// Where the writer's pages go. Called on the engine's thread, while the
/// engine is inside `pdf_ship_out`: a sink must not run the engine.
pub trait Sink {
    fn emit(&mut self, e: Emitted);
    fn flush(&mut self) {}
}

/// Start the writer (before the engine allocates its first node) with
/// `sink`. Names and resources start empty; the working directory is the
/// one relative file names are taken from.
pub fn init_with_sink(sink: Box<dyn Sink>) {
    DL.with(|d| *d.borrow_mut() = Some(Box::new(State::new())));
    SINK.with(|s| *s.borrow_mut() = Some(sink));
    forget_engine_state();
    ENABLED.store(true, Ordering::Relaxed);
}

/// Replace the sink, keeping names and resources.
pub fn set_sink(sink: Box<dyn Sink>) {
    SINK.with(|s| *s.borrow_mut() = Some(sink));
}

/// Stop writing (the sink is dropped; names and resources too).
pub fn shut_down() {
    ENABLED.store(false, Ordering::Relaxed);
    SINK.with(|s| *s.borrow_mut() = None);
    DL.with(|d| *d.borrow_mut() = None);
}

/// Start writing display lists if `FLASHTEX_DISPLAY_LIST` asks for it
/// (before the engine allocates its first node). `FLASHTEX_DISPLAY_LIST_HAVE_FONTS`
/// lists font keys (hex, comma-separated) whose programs the reader holds.
pub fn init_from_env() {
    // The grammar (`fd:N`, `socket:PATH`, `pipe:NAME` or a file) lives in
    // one place, the protocol crate (spec §6.6).
    use flashtex_display_list::endpoint::{Endpoint, ENV};
    let ep = match Endpoint::from_env() {
        None => return,
        Some(Ok(ep)) => ep,
        Some(Err(e)) => {
            eprintln!("{e}");
            return;
        }
    };
    let w: Box<dyn Write> = match ep.open_writer() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("{ENV}: {ep}: {e}");
            return;
        }
    };
    let peer = Peer {
        have_fonts: parse_font_keys(
            &std::env::var("FLASHTEX_DISPLAY_LIST_HAVE_FONTS").unwrap_or_default(),
        ),
        font_formats: std::env::var("FLASHTEX_DISPLAY_LIST_FONT_FORMATS")
            .ok()
            .map(|s| parse_font_formats(&s)),
        ..Peer::default()
    };
    init_with_sink(Box::new(StreamSink { w: Some(w), peer }));
}

/// `COMPILE.font_formats` (or `FLASHTEX_DISPLAY_LIST_FONT_FORMATS`): font
/// formats, separated by commas.
pub fn parse_font_formats(s: &str) -> HashSet<String> {
    s.split(',')
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty())
        .collect()
}

/// Font keys in hex, separated by commas (anything else is skipped).
pub fn parse_font_keys(s: &str) -> HashSet<[u8; 32]> {
    s.split(',').filter_map(|h| parse_key(h.trim())).collect()
}

/// A 64-digit hex key.
pub fn parse_key(h: &str) -> Option<[u8; 32]> {
    if h.len() != 64 {
        return None;
    }
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = u8::from_str_radix(h.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(k)
}

/// Flush the sink (the end of the run).
pub fn finish() {
    if !enabled() {
        return;
    }
    with_sink(|s| s.flush());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> Option<R> {
    DL.with(|d| d.borrow_mut().as_mut().map(|st| f(st)))
}

/// Run `f` with the sink taken out, so that it may call back into the
/// writer.
fn with_sink(f: impl FnOnce(&mut dyn Sink)) {
    let Some(mut s) = SINK.with(|s| s.borrow_mut().take()) else {
        return;
    };
    f(&mut *s);
    SINK.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            *slot = Some(s);
        }
    });
}

/// The standalone engine's sink: frames to a file or descriptor, as the
/// pages are shipped.
struct StreamSink {
    w: Option<Box<dyn Write>>,
    peer: Peer,
}

impl Sink for StreamSink {
    fn emit(&mut self, e: Emitted) {
        let Some(w) = self.w.as_mut() else { return };
        let ok = self
            .peer
            .send(&e, &mut |k, b| write_frame(&mut *w, k, b).is_ok())
            && w.flush().is_ok();
        if !ok {
            // The reader went away: stop writing, keep typesetting.
            self.w = None;
        }
    }
    fn flush(&mut self) {
        if let Some(w) = self.w.as_mut() {
            if w.flush().is_err() {
                self.w = None;
            }
        }
    }
}

/// What one reader has been sent: which resource each id stands for, and
/// where each span it knows is. [`Peer::send`] sends a page with exactly
/// what the reader lacks before it: a `FONT` for an id it has not seen (or
/// that stood for another font), an `IMAGE` likewise, a `SOURCES` for new
/// files and spans (or spans that moved).
#[derive(Default)]
pub struct Peer {
    /// Font keys whose programs the reader holds (`have_fonts`).
    pub have_fonts: HashSet<[u8; 32]>,
    /// The font formats beyond [`BASE_FONT_FORMATS`] whose programs the
    /// reader takes (`COMPILE.font_formats`); `None`: all (a file or
    /// descriptor sink, whose reader is our own).
    pub font_formats: Option<HashSet<String>>,
    fonts: HashMap<u32, [u8; 32]>,
    images: HashMap<u32, [u8; 32]>,
    spans: HashMap<u32, (u32, u32)>,
    files: HashSet<u32>,
}

impl Peer {
    /// Forget what was sent (the reader starts afresh).
    pub fn reset(&mut self) {
        self.fonts.clear();
        self.images.clear();
        self.spans.clear();
        self.files.clear();
    }

    /// Send `e`, preceded by the resources and sources the reader lacks.
    /// `send` returns `false` once the reader is gone; so does this.
    pub fn send(&mut self, e: &Emitted, send: &mut dyn FnMut(u8, &[u8]) -> bool) -> bool {
        for &(id, key) in &e.fonts {
            if self.fonts.get(&id) == Some(&key) {
                continue;
            }
            // A program in a format the reader did not ask for is sent as
            // if held: the reader sees the format and draws the page from
            // the PDF.
            let withheld = self.font_formats.as_ref().is_some_and(|ok| {
                with(|st| st.fonts.get(&key).map(|f| f.format))
                    .flatten()
                    .is_some_and(|f| !BASE_FONT_FORMATS.contains(&f) && !ok.contains(f))
            });
            let held = self.have_fonts.contains(&key) || withheld;
            if let Some(body) = font_body(id, &key, held) {
                if !send(kind::FONT, &body) {
                    return false;
                }
            }
            self.fonts.insert(id, key);
        }
        for &(id, key) in &e.images {
            if self.images.get(&id) == Some(&key) {
                continue;
            }
            if let Some(body) = image_body(id, &key) {
                if !send(kind::IMAGE, &body) {
                    return false;
                }
            }
            self.images.insert(id, key);
        }
        if let Some(src) = self.sources_for(&e.spans) {
            if !send(kind::SOURCES, &src) {
                return false;
            }
        }
        send(if e.form { kind::FORM } else { kind::PAGE }, &e.body)
    }

    /// A `SOURCES` body for the spans (and their files) the reader does not
    /// have where they are now.
    pub fn sources_for(&mut self, spans: &[u32]) -> Option<Vec<u8>> {
        let mut src = Sources::default();
        with(|st| {
            for &s in spans {
                let Some(&(file, line)) = st.spans.get(s as usize - 1) else {
                    continue;
                };
                if self.spans.get(&s) == Some(&(file, line)) {
                    continue;
                }
                if self.files.insert(file) {
                    if let Some(p) = st.file_paths.get(file as usize - 1) {
                        src.files.push((file, p.clone()));
                    }
                }
                src.spans.push((s, file, line));
                self.spans.insert(s, (file, line));
            }
        });
        (!src.files.is_empty() || !src.spans.is_empty())
            .then(|| src.to_json().to_string().into_bytes())
    }

    /// A `SOURCES` body re-declaring the spans the reader knows that have
    /// moved since ([`move_lines`]), if any.
    pub fn moved_spans(&mut self) -> Option<Vec<u8>> {
        let mut moved: Vec<u32> = with(|st| {
            self.spans
                .iter()
                .filter(|(s, at)| st.spans.get(**s as usize - 1) != Some(*at))
                .map(|(s, _)| *s)
                .collect()
        })
        .unwrap_or_default();
        moved.sort_unstable();
        self.sources_for(&moved)
    }
}

/// The `FONT` body of font `id` standing for `key` (without the program
/// when the reader holds it).
pub fn font_body(id: u32, key: &[u8; 32], held: bool) -> Option<Vec<u8>> {
    with(|st| {
        st.fonts.get(key).map(|f| {
            Font {
                id: id as u16,
                key: *key,
                info: f.info.clone(),
                program: if held {
                    Vec::new()
                } else {
                    f.program.as_ref().clone()
                },
            }
            .encode()
        })
    })
    .flatten()
}

/// The `IMAGE` body of image `id` standing for `key`.
pub fn image_body(id: u32, key: &[u8; 32]) -> Option<Vec<u8>> {
    with(|st| {
        st.images.get(key).map(|info| {
            let mut kv = vec![
                ("id".to_string(), Json::Int(id as i64)),
                ("key".to_string(), js(hex(key))),
            ];
            if let Json::Obj(rest) = info {
                kv.extend(rest.iter().cloned());
            }
            Json::Obj(kv).to_string().into_bytes()
        })
    })
    .flatten()
}

impl Globals {
    /// The source location the side table holds for node `p`: (span,
    /// column), `None` when it has none (or no display list is written).
    pub fn dl_node_loc(&self, p: i32) -> Option<(u32, u16)> {
        if !enabled() {
            return None;
        }
        let l = self.side_get(p);
        (loc_span(l) != 0).then(|| (loc_span(l), loc_col(l)))
    }
}

/// The span of `line` of the file TeX names `name` (made if new): the
/// diagnostics side channel's places travel as the pages' spans do.
pub fn span_for(name: &[u8], line: u32) -> Option<u32> {
    with(|st| {
        let f = st.file_id(name);
        st.span_id(f, line)
    })
    .filter(|&s| s != 0)
}

/// Where span `s` is now: (file id, line).
pub fn span_location(s: u32) -> Option<(u32, u32)> {
    with(|st| st.spans.get((s as usize).checked_sub(1)?).copied()).flatten()
}

/// The absolute path of file id `f`.
pub fn file_path(f: u32) -> Option<String> {
    with(|st| st.file_paths.get((f as usize).checked_sub(1)?).cloned()).flatten()
}

/// Every file the writer has named: (id, absolute path).
pub fn files() -> Vec<(u32, String)> {
    with(|st| {
        st.file_paths
            .iter()
            .enumerate()
            .map(|(i, p)| (i as u32 + 1, p.clone()))
            .collect()
    })
    .unwrap_or_default()
}

/// Lines of the file at `path` (absolute) changed: its old lines
/// `from..old_end` (1-based; `old_end` exclusive) are now `from..new_end`.
/// Spans on later lines move with their lines, so that pages kept from
/// before the edit still point at their source; spans inside the change
/// keep their line but are no longer given to new material, which gets
/// spans of its own.
pub fn move_lines(path: &str, from: u32, old_end: u32, new_end: u32) {
    with(|st| {
        let Some(f) = st.file_paths.iter().position(|p| p == path) else {
            return;
        };
        st.move_spans_of(f as u32 + 1, from, old_end, new_end);
    });
    forget_file_cache();
}

impl State {
    /// `move_lines` for file `f`. Every span of the file on a line from
    /// `from` on is retired (before `old_end`) or moved (from `old_end` on,
    /// to a line from `new_end` on), and no other span's line is that high,
    /// so only those spans' keys change: they are taken out of `span_ids`
    /// and the moved ones put back, in span order (the lowest span of a
    /// line keeps it, as when the whole table was rebuilt). A long document
    /// rebuilt the table at every keystroke (lane P4-SPLIT-LATENCY), and
    /// at every change of a file no span names a moved line of.
    ///
    /// Each key of the file from `from` on names a span on that line (keys
    /// only ever name a span where it is), so they all go, and the moved
    /// spans that are not retired take their new lines in span order.
    fn move_spans_of(&mut self, f: u32, from: u32, old_end: u32, new_end: u32) {
        let delta = new_end as i64 - old_end as i64;
        self.span_ids.clear_from(f, from);
        for i in 0..self.spans.len() {
            let (file, line) = self.spans[i];
            if file != f || line < from {
                continue;
            }
            let id = i as u32 + 1;
            if line < old_end {
                self.retire(id);
            } else {
                let line = (line as i64 + delta).max(1) as u32;
                self.spans[i].1 = line;
                if !self.is_retired(id) {
                    self.span_ids.insert_new(f, line, id);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// the side table: `Globals::dl_side`, indexed like `mem`
//
// The side table is an array of the word space (changes/displaylist.ch), so
// checkpoints keep it as they keep `mem`: as the words that changed since the
// last checkpoint, within the undo logs' budget (DESIGN.md §5.2). It used to
// be a copy-on-write table of 16 KB chunks outside the space, which every
// checkpoint shared and every page copied chunk by chunk: 126 MB for the
// 92 checkpoints of full-10, and several GB on 1,000 pages, outside the
// budget (docs/evidence/p4-memory-2026-09-30/).

/// A checkpoint was restored: forget what was cached from the engine state
/// it replaces (the side table itself is restored with the word space).
pub fn restored() {
    if !enabled() {
        return;
    }
    forget_engine_state();
}

/// A new job starts in this thread (`crate::pdftex::reset_state`): font,
/// image and string numbers start again.
pub fn engine_reset() {
    if !enabled() {
        return;
    }
    forget_engine_state();
}

fn forget_engine_state() {
    with(|st| {
        st.font_keys.clear();
        st.image_keys.clear();
        st.widths.clear();
        st.advances.clear();
        st.font_kinds.clear();
        st.capture = None;
    });
    forget_file_cache();
    HYPH_ON.store(false, Ordering::Relaxed);
}

fn forget_file_cache() {
    LAST_NAME.store(-1, Ordering::Relaxed);
}
// hooks (changes/displaylist.ch) and taps (pdfshipoutbegin/end, write_pdf)

/// The innermost file's name string and line, and their span (a cache:
/// the span changes when the line does; -1 after a restore, which may give
/// string numbers to other names).
static LAST_NAME: AtomicI32 = AtomicI32::new(-1);
static LAST_LINE: AtomicU32 = AtomicU32::new(0);
static LAST_SPAN: AtomicU32 = AtomicU32::new(0);
/// Where `dl_here` last found the innermost input level that reads a file
/// line, as an index of `input_stack` (a hint: `Globals::file_level`
/// checks it against the state before it uses it).
static FILE_LEVEL: AtomicUsize = AtomicUsize::new(usize::MAX);
/// Between `dl_token_begin` and `dl_token_end`: an allocation that is not a
/// node of a list TeX ships.
static NOT_A_NODE: AtomicBool = AtomicBool::new(false);
/// While `hyphenate` rebuilds a word: HYPH_ON, and the word's place.
static HYPH_ON: AtomicBool = AtomicBool::new(false);
static HYPH_LOC: AtomicU64 = AtomicU64::new(0);

impl Globals {
    /// The side table's entry for `mem` location `p` (0 outside it).
    #[inline(always)]
    fn side_get(&self, p: i32) -> Loc {
        match self.dl_side.get(p as u32 as usize) {
            Some(w) => w.0,
            None => 0,
        }
    }

    /// Set the side table's entry for `mem` location `p`, through the word
    /// space's write barrier; an unchanged entry is not written.
    #[inline(always)]
    fn side_set(&mut self, p: i32, v: Loc) {
        let i = p as u32 as usize;
        if i < self.dl_side.len() && self.dl_side[i].0 != v {
            self.dl_side[i] = crate::generated::types::memory_word(v);
        }
    }
}

impl Globals {
    /// `dl_new_node(p)`: node (or token) `p` was just allocated.
    #[inline(always)]
    pub fn dl_new_node(&mut self, p: i32) {
        // Tokens stored while a definition, a macro's arguments or a token
        // list are scanned (`scanner_status` other than `normal`) never
        // become nodes; they are most of the allocations, so they are
        // skipped before anything else is looked at. So are the
        // allocations between `dl_token_begin` and `dl_token_end`.
        if self.scanner_status == 0 && enabled() && !NOT_A_NODE.load(Ordering::Relaxed) {
            self.dl_note_node(p);
        }
    }

    /// `back_input` allocates its token, `conditional` its condition-stack
    /// node (changes/displaylist.ch): neither becomes part of a list TeX
    /// ships, so neither needs a source position. The side table's entry
    /// of the location keeps whatever it held; the location is noted again
    /// when it is next allocated as a node, before anything reads it.
    #[inline(always)]
    pub fn dl_token_begin(&mut self) {
        NOT_A_NODE.store(true, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn dl_token_end(&mut self) {
        NOT_A_NODE.store(false, Ordering::Relaxed);
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
        self.side_set(p, v);
    }

    /// The source position TeX is reading at: the span of the innermost open
    /// file's line, and the column after the last character read from it.
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
            self.file_level(&FILE_LEVEL, is_file)
        };
        let col = match rec {
            Some(r) if r.loc_field > r.start_field => {
                ((r.loc_field - r.start_field - 1).min(NO_COLUMN as i32 - 1)) as u16
            }
            Some(_) => 0,
            None => NO_COLUMN,
        };
        let span = if LAST_NAME.load(Ordering::Relaxed) == name
            && LAST_LINE.load(Ordering::Relaxed) == line
        {
            LAST_SPAN.load(Ordering::Relaxed)
        } else {
            let bytes = self.str_bytes(name);
            let s = with(|st| {
                let f = st.file_id(&bytes);
                st.span_id(f, line)
            })
            .unwrap_or(0);
            LAST_NAME.store(name, Ordering::Relaxed);
            LAST_LINE.store(line, Ordering::Relaxed);
            LAST_SPAN.store(s, Ordering::Relaxed);
            s
        };
        loc_pack(span, col)
    }

    /// The innermost level of `input_stack` (below `cur_input`) for which
    /// `is_file` holds: what walking the stack down from its top finds,
    /// without the walk when the level found last time is still that level.
    ///
    /// Every level whose state is not `token_list` is one that
    /// `begin_file_reading` pushed (or the bottom level), and
    /// `begin_file_reading` gives it `index:=in_open` after
    /// `incr(in_open)`, which `end_file_reading` takes back (§328, §329):
    /// the open levels carry the indexes 1 to `in_open` in stack order. So
    /// a level below the top whose state is not `token_list` and whose
    /// index is `in_open` has no such level above it but `cur_input`, and
    /// if `is_file` holds for it, it is the level the walk finds (the walk
    /// skips token lists, whose state is `token_list`). The hint is checked
    /// against the state as it is now, so a restore or anything else that
    /// moved the stack since leaves the result exact.
    /// `hint` is the caller's own (each `is_file` its own).
    #[inline]
    pub(crate) fn file_level(
        &self,
        hint: &AtomicUsize,
        is_file: impl Fn(&crate::generated::types::in_state_record) -> bool,
    ) -> Option<crate::generated::types::in_state_record> {
        let top = self.input_ptr.max(0) as usize;
        let k = hint.load(Ordering::Relaxed);
        if k < top {
            let r = self.input_stack[k];
            if r.state_field != crate::generated::consts::token_list
                && r.index_field == self.in_open
                && is_file(&r)
            {
                return Some(r);
            }
        }
        let k = (0..top).rev().find(|&k| is_file(&self.input_stack[k]))?;
        hint.store(k, Ordering::Relaxed);
        Some(self.input_stack[k])
    }

    /// `dl_copy(r, p)`: node `r` is a copy of node `p` (`copy_node_list`).
    #[inline(always)]
    pub fn dl_copy(&mut self, r: i32, p: i32) {
        if enabled() {
            let v = self.side_get(p);
            self.side_set(r, v);
        }
    }

    /// `dl_hyph_begin(ha)`: `hyphenate` is about to rebuild the word that
    /// starts at node `ha`.
    pub fn dl_hyph_begin(&mut self, ha: i32) {
        if enabled() {
            HYPH_LOC.store(self.side_get(ha), Ordering::Relaxed);
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
            let loc = self.side_get(p);
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
            let span = loc_span(loc);
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
        // A page's index is the number of pages shipped before it:
        // `total_pages`, which `pdf_ship_out` increments after the page's
        // stream, and which a restored checkpoint carries.
        let id = if shipping_page {
            self.total_pages.max(0) as u32
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
        let _m = crate::memstat::scope(crate::memstat::tag::DL);
        let t_emit = std::time::Instant::now();
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
        let prefix = if self.pdf_resname_prefix != 0 {
            self.str_bytes(self.pdf_resname_prefix)
        } else {
            Vec::new()
        };
        // What the page carries besides its content (independent of it):
        // the counts and the links (`dl_links` reads pdfTeX's lists).
        let mut frame = Page::new(kind, cap.id);
        frame.width = wsp;
        frame.height = hsp;
        frame.pdf_box = [0.0, 0.0, bw.to_f64(), bh.to_f64()];
        if !cap.form {
            for k in 0..10 {
                frame.counts[k] = self.eqtb[COUNT_BASE + k - 1].int();
            }
            self.dl_links(&mut frame, mag);
        }
        // Everything the display list is a function of, but the engine's
        // answers to the interpreter (`Memo::queries`) and the resource keys.
        let digest = {
            let mut m: Vec<u8> = Vec::with_capacity(256 + cap.markers.len() * 10);
            m.push(cap.form as u8);
            m.push(cap.draft as u8);
            m.extend(cap.id.to_le_bytes());
            for v in [bw.0, bh.0].into_iter().chain(ctm.0.iter().map(|f| f.0)) {
                m.extend(v.to_le_bytes());
            }
            m.extend((prefix.len() as u64).to_le_bytes());
            m.extend(&prefix);
            for mk in &cap.markers {
                m.extend(mk.offset.to_le_bytes());
                m.extend(mk.span.to_le_bytes());
                m.extend(mk.col.to_le_bytes());
            }
            m.extend(frame.encode());
            let (a, b) = (
                crate::persist::hash128(&m),
                crate::persist::hash128(&cap.bytes),
            );
            [a[0], a[1], b[0], b[1]]
        };
        // (FLASHTEX_NO_DL_MEMO=1 builds every display list, for A/B)
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let off = *OFF.get_or_init(|| std::env::var_os("FLASHTEX_NO_DL_MEMO").is_some());
        let memo = with(|st| st.memo.get(&(cap.form, cap.id)).cloned())
            .flatten()
            .filter(|_| !off);
        if let Some(m) = memo.filter(|m| m.digest == digest) {
            let holds = self.dl_memo_holds(&m, &prefix);
            self.scaled_out = saved_scaled_out;
            if holds {
                EMIT_NS.with(|c| c.set(c.get() + t_emit.elapsed().as_nanos() as u64));
                with_sink(|s| s.emit(m.emitted));
                return;
            }
        }
        let mut env = Recording {
            env: WidthEnv {
                g: self,
                prefix: prefix.clone(),
            },
            queries: HashMap::new(),
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
        let mut queries: Vec<(Query, Answer)> = env.queries.into_iter().collect();
        queries.sort_unstable_by_key(|q| q.0);
        let page = &mut out.page;
        page.width = wsp;
        page.height = hsp;
        page.pdf_box = frame.pdf_box;
        page.counts = frame.counts;
        page.links.append(&mut frame.links);
        page.dests.append(&mut frame.dests);
        // The keys of the fonts and images the items use, then the spans
        // the items and links name.
        let fonts: Vec<(u32, [u8; 32])> = out
            .fonts
            .iter()
            .map(|&f| (f, self.dl_font_key(f)))
            .collect();
        let image_keys: Vec<(u32, Option<[u8; 32]>)> = out
            .images
            .iter()
            .map(|&n| (n, self.dl_image_key(n)))
            .collect();
        let images: Vec<(u32, [u8; 32])> = image_keys
            .iter()
            .filter_map(|&(n, k)| k.map(|k| (n, k)))
            .collect();
        let fk = |f: u16| {
            fonts
                .iter()
                .find(|x| x.0 == f as u32)
                .map_or([0; 32], |x| x.1)
        };
        let ik = |i: u32| images.iter().find(|x| x.0 == i).map_or([0; 32], |x| x.1);
        out.page.hash = out.page.content_hash(&fk, &ik);
        let body = out.page.encode();
        self.scaled_out = saved_scaled_out;
        let mut spans: Vec<u32> = Vec::new();
        let mut seen: HashSet<u32> = HashSet::new();
        for s in cap
            .markers
            .iter()
            .map(|m| m.span)
            .chain(out.page.links.iter().map(|l| l.span))
        {
            if s != 0 && seen.insert(s) {
                spans.push(s);
            }
        }
        let e = Emitted {
            form: cap.form,
            index: cap.id,
            body: Arc::new(body),
            hash: out.page.hash,
            fonts,
            images,
            forms: out.forms.clone(),
            spans,
        };
        with(|st| {
            st.memo.insert(
                (e.form, e.index),
                Memo {
                    digest,
                    queries: Arc::new(queries),
                    image_keys: Arc::new(image_keys),
                    emitted: e.clone(),
                },
            )
        });
        EMIT_NS.with(|c| c.set(c.get() + t_emit.elapsed().as_nanos() as u64));
        with_sink(|s| s.emit(e));
    }

    /// Whether a memo's display list is the one the engine would build now
    /// from the same stream (its `digest` matched): the engine still gives
    /// the interpreter the same answers, and the fonts and images it names
    /// still have the same keys.
    fn dl_memo_holds(&mut self, m: &Memo, prefix: &[u8]) -> bool {
        let mut env = WidthEnv {
            g: self,
            prefix: prefix.to_vec(),
        };
        let same = m.queries.iter().all(|(q, a)| q.ask(&mut env) == *a);
        same && m
            .emitted
            .fonts
            .iter()
            .all(|&(f, k)| self.dl_font_key(f) == k)
            && m.image_keys.iter().all(|&(n, k)| self.dl_image_key(n) == k)
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

/// A PK font's Type 3 description ([`Globals::dl_type3`]).
struct Type3 {
    matrix: String,
    dpi: u32,
    /// The PK file's path and its bitmap program.
    program: Option<(String, Vec<u8>)>,
}

/// `pdf_print_real(m, d)`'s text: `m / 10^d`, without trailing zeros.
fn format_real(m: i32, d: u32) -> String {
    let neg = m < 0;
    let m = (m as i64).abs();
    let p = 10i64.pow(d);
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    s.push_str(&(m / p).to_string());
    let frac = m % p;
    if frac > 0 {
        let digits = format!("{frac:0width$}", width = d as usize);
        s.push('.');
        s.push_str(digits.trim_end_matches('0'));
    }
    s
}

/// A display list as `dl_emit` built it (`State::memo`).
#[derive(Clone)]
struct Memo {
    /// The stream's bytes and markers, its index and box, the counts and
    /// links (`dl_emit`).
    digest: [u64; 4],
    /// Every question the interpreter asked the engine, with its answer.
    queries: Arc<Vec<(Query, Answer)>>,
    /// The images the items name, with their keys as `dl_image_key` gave them.
    image_keys: Arc<Vec<(u32, Option<[u8; 32]>)>>,
    emitted: Emitted,
}

/// A question the interpreter asks the engine (`interp::Env`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Query {
    Width(u32, u8),
    Advance(u32, u8),
    FontProblem(u32),
}

#[derive(Clone, PartialEq)]
enum Answer {
    Width(Option<i64>),
    Advance(Option<(i64, i64)>),
    FontProblem(Option<String>),
}

impl Query {
    fn ask(self, env: &mut WidthEnv<'_>) -> Answer {
        use interp::Env;
        match self {
            Query::Width(f, c) => Answer::Width(env.width(f, c)),
            Query::Advance(f, c) => Answer::Advance(env.advance(f, c)),
            Query::FontProblem(f) => Answer::FontProblem(env.font_problem(f)),
        }
    }
}

/// The engine's answers to the interpreter, noted (`Memo::queries`).
struct Recording<'a> {
    env: WidthEnv<'a>,
    queries: HashMap<Query, Answer>,
}

impl interp::Env for Recording<'_> {
    fn width(&mut self, font: u32, code: u8) -> Option<i64> {
        let q = Query::Width(font, code);
        if let Some(Answer::Width(a)) = self.queries.get(&q) {
            return *a;
        }
        let a = self.env.width(font, code);
        self.queries.insert(q, Answer::Width(a));
        a
    }
    fn advance(&mut self, font: u32, code: u8) -> Option<(i64, i64)> {
        let q = Query::Advance(font, code);
        if let Some(Answer::Advance(a)) = self.queries.get(&q) {
            return *a;
        }
        let a = self.env.advance(font, code);
        self.queries.insert(q, Answer::Advance(a));
        a
    }
    fn font_problem(&mut self, font: u32) -> Option<String> {
        let q = Query::FontProblem(font);
        if let Some(Answer::FontProblem(a)) = self.queries.get(&q) {
            return a.clone();
        }
        let a = self.env.font_problem(font);
        self.queries.insert(q, Answer::FontProblem(a.clone()));
        a
    }
    fn resname_prefix(&self) -> &[u8] {
        self.env.resname_prefix()
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
    fn advance(&mut self, font: u32, code: u8) -> Option<(i64, i64)> {
        let g = &mut *self.g;
        if font == 0 || font as i32 > g.font_ptr {
            return None;
        }
        let f = font as i32;
        match g.dl_font_kind(f) {
            FontKind::Mapped => self.width(font, code).map(|w| (w, 10_000)),
            // (a .pgc font's widths and /FontMatrix are its file's)
            FontKind::Pgc => None,
            FontKind::Pk => {
                if let Some(t) = with(|st| {
                    st.advances
                        .get(&font)
                        .map(|t| t.as_ref().map(|(w, d)| (w[code as usize], *d)))
                })
                .flatten()
                {
                    return t;
                }
                // writet3.c: /Widths entry i is pdfprintreal(pk_char_width(f,
                // w_i), 2), /FontMatrix pdfprintreal(pk_font_scale, 5): the
                // advance per unit size is w_i/100 * s/10^5.
                let saved = g.scaled_out;
                let s = g.get_pk_font_scale(f) as i64;
                let mut t = Box::new([0i64; 256]);
                for c in 0..256 {
                    let w = g.get_charwidth(f, c);
                    t[c as usize] = g.pk_char_width(f, w) as i64 * s;
                }
                g.scaled_out = saved;
                let r = Some((t[code as usize], 10_000_000));
                with(|st| st.advances.insert(font, Some((t, 10_000_000))));
                r
            }
        }
    }
    fn font_problem(&mut self, font: u32) -> Option<String> {
        let key = self.g.dl_font_key(font);
        with(|st| st.fonts.get(&key).and_then(|f| f.problem.clone())).flatten()
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
            let span = loc_span(self.side_get(i));
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
            let kind = self.m_b0(i + 5);
            // Only the words pdfTeX writes for this kind (`pdf_print_dests`'
            // `/XYZ left top zoom`, `/FitH top`, `/FitV left`, `/FitR` the
            // rectangle): `do_dest` sets just those unless a matrix is in
            // use, and `\pdfdest` sets the zoom for `xyz` alone. The others
            // hold whatever the node's memory held before (`get_node` does
            // not clear it; `pdf_bottom` of an `xyz` dest is never set), so
            // a restored run and a run from scratch differ there.
            let (left, top, right_bottom, zoom) = match kind {
                0 => (true, true, false, true),       // xyz
                2 | 5 => (false, true, false, false), // fith, fitbh
                3 | 6 => (true, false, false, false), // fitv, fitbv
                7 => (true, true, true, false),       // fitr
                _ => (false, false, false, false),    // fit, fitb
            };
            let word = |on: bool, at: i32, g: &mut Globals| {
                if on {
                    let v = g.m_int(at);
                    scale(v, g)
                } else {
                    0
                }
            };
            let rect = [
                word(left, i + 1, self),
                word(top, i + 2, self),
                word(right_bottom, i + 3, self),
                word(right_bottom, i + 4, self),
            ];
            page.dests.push(Dest {
                named,
                name,
                kind: kind as u8,
                rect,
                zoom: if zoom { self.m_lh(i + 6) } else { 0 },
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
    /// The key of font `/F<f>` as the engine has it now, describing it in
    /// the resource store the first time the key is seen.
    fn dl_font_key(&mut self, f: u32) -> [u8; 32] {
        if let Some(k) = with(|st| st.font_keys.get(&f).copied()).flatten() {
            return k;
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
        let mut program: Arc<Vec<u8>> = Arc::new(Vec::new());
        let mut program_sha = sha256(&[]);
        let mut file = None;
        let mut names: Option<Arc<Vec<Vec<u8>>>> = None;
        let (mut slant, mut extend, mut ps_name) = (0, 0, Vec::new());
        // Type 3: pdfTeX's /FontMatrix, and the PK resolution
        let (mut t3_matrix, mut t3_dpi, mut problem) = (None, None, None);
        // a TrueType subfont: the character codes and the cmap
        let mut subfont: Option<(Vec<i32>, i16, i16)> = None;
        let kind = if fi <= self.font_ptr {
            self.dl_font_kind(fi)
        } else {
            FontKind::Mapped
        };
        if let Some(fm) = fm.as_ref().filter(|_| kind == FontKind::Mapped) {
            slant = fm.slant;
            extend = fm.extend;
            ps_name = fm.ps_name.clone().unwrap_or_default();
            let file_format = if fm.is_type1() {
                Some((Format::Type1, "type1"))
            } else if fm.is_truetype() {
                Some((Format::TrueType, "truetype"))
            } else if fm.is_opentype() {
                Some((Format::OpenType, "opentype"))
            } else {
                None
            };
            if let (Some((ff_format, name)), true) = (file_format, fm.is_included()) {
                if let Some(ff) = &fm.ff_name {
                    let ff = String::from_utf8_lossy(ff).into_owned();
                    if let Some(path) = find_quietly(&ff, ff_format) {
                        if let Some((data, sha)) = read_program(&path) {
                            program = data;
                            program_sha = sha;
                            format = name;
                            file = Some(path);
                        }
                    }
                }
            }
            if fm.is_truetype() && format == "none" {
                format = "truetype";
            } else if fm.is_opentype() && format == "none" {
                format = "opentype";
            }
            if let Some(c) = &fm.subfont {
                subfont = Some((c.clone(), fm.pid, fm.eid));
            }
            if let Some(enc) = &fm.encname {
                names = read_enc(&String::from_utf8_lossy(enc));
            }
        } else if kind != FontKind::Mapped {
            format = "type3";
            if let Some(enc) = fm.as_ref().and_then(|fm| fm.encname.clone()) {
                names = read_enc(&String::from_utf8_lossy(&enc));
            }
            if kind == FontKind::Pgc {
                problem = Some("Type 3 font from a .pgc file".to_string());
            } else {
                let t3 = self.dl_type3(fi);
                t3_matrix = Some(t3.matrix);
                t3_dpi = Some(t3.dpi);
                match t3.program {
                    Some((path, data)) => {
                        program_sha = sha256(&data);
                        program = Arc::new(data);
                        file = Some(path);
                    }
                    None => problem = Some("Type 3 font without its PK file".to_string()),
                }
            }
        }
        if names.is_none() && format == "type1" {
            names = Some(builtin_encoding_of(&program, &program_sha));
        }
        let mut h = Sha256::new();
        h.update(b"display-list-v3 font\0");
        h.update(format.as_bytes());
        h.update(&[0]);
        h.update(&program_sha);
        if let Some(n) = &names {
            for g in n.iter() {
                h.update(g);
                h.update(&[0]);
            }
        }
        h.update(&slant.to_le_bytes());
        h.update(&extend.to_le_bytes());
        // (the formats 3.1 did not describe: their own parts of the key)
        if let Some(m) = &t3_matrix {
            h.update(b"\0matrix\0");
            h.update(m.as_bytes());
        }
        if let Some((codes, pid, eid)) = &subfont {
            h.update(b"\0subfont\0");
            for c in codes {
                h.update(&c.to_le_bytes());
            }
            h.update(&pid.to_le_bytes());
            h.update(&eid.to_le_bytes());
        }
        if let Some(p) = &problem {
            h.update(b"\0problem\0");
            h.update(p.as_bytes());
        }
        let key = h.finish();
        let known = with(|st| st.fonts.contains_key(&key)).unwrap_or(true);
        if !known {
            let prefix = if self.pdf_resname_prefix != 0 {
                self.str_bytes(self.pdf_resname_prefix)
            } else {
                Vec::new()
            };
            let lossy = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
            let mut info: Vec<(String, Json)> = vec![
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
            ];
            if let Some(m) = &t3_matrix {
                // pdfTeX's /FontMatrix of the Type 3 font, as it writes it
                if let Some(e) = info.iter_mut().find(|(k, _)| k == "font_matrix") {
                    e.1 = js(m.as_str());
                }
            }
            if let Some(d) = t3_dpi {
                info.push(("dpi".into(), Json::Int(d as i64)));
            }
            if let Some((codes, pid, eid)) = &subfont {
                info.push((
                    "subfont".into(),
                    Json::Arr(codes.iter().map(|&c| Json::Int(c as i64)).collect()),
                ));
                info.push((
                    "cmap".into(),
                    Json::Arr(vec![Json::Int(*pid as i64), Json::Int(*eid as i64)]),
                ));
            }
            if let Some(p) = &problem {
                info.push(("problem".into(), js(p.as_str())));
            }
            let info = Json::Obj(info);
            with(|st| {
                st.fonts.insert(
                    key,
                    FontRes {
                        info,
                        program,
                        format,
                        problem,
                    },
                )
            });
        }
        with(|st| st.font_keys.insert(f, key));
        key
    }

    /// How pdfTeX writes font `f` (writefont.c's `dopdffont`): from its map
    /// entry, or as Type 3 from a `.pgc` file if there is one, else from its
    /// PK file.
    fn dl_font_kind(&mut self, f: i32) -> FontKind {
        if let Some(k) = with(|st| st.font_kinds.get(&(f as u32)).copied()).flatten() {
            return k;
        }
        let ptr = self.pdf_font_map[f as usize];
        let mapped = ptr > 0 && self.with_fonts(|_, st| !st.map.fm(ptr).is_pk());
        let kind = if mapped {
            FontKind::Mapped
        } else {
            let mut pgc = self.c_string(self.font_name[f as usize]);
            pgc.extend_from_slice(b".pgc");
            if find_quietly(&String::from_utf8_lossy(&pgc), Format::MiscFonts).is_some() {
                FontKind::Pgc
            } else {
                FontKind::Pk
            }
        };
        with(|st| st.font_kinds.insert(f as u32, kind));
        kind
    }

    /// A PK font's Type 3 description: pdfTeX's `/FontMatrix`
    /// (`pdfprintreal(pk_font_scale, 5)` twice), the resolution writet3
    /// asks for, and the glyph bitmaps of its PK file, if kpathsea finds
    /// the file without running mktexpk (which the engine runs only when
    /// it writes the font, at the end of the document).
    fn dl_type3(&mut self, f: i32) -> Type3 {
        let saved = self.scaled_out;
        let scale = self.get_pk_font_scale(f);
        self.scaled_out = saved;
        let s = format_real(scale, 5);
        let matrix = format!("{s} 0 0 {s} 0 0");
        let dpi = self.pk_dpi(f);
        let name = self.c_string(self.font_name[f as usize]);
        let program = crate::system::find_pk_quietly(&String::from_utf8_lossy(&name), dpi)
            .filter(|g| {
                g.name == name
                    && crate::pdftex::writet3::bitmap_tolerance(
                        g.dpi as f32 as f64,
                        dpi as f32 as f64,
                    )
            })
            .and_then(|g| {
                let path = g.path.to_string_lossy().into_owned();
                let (data, _) = read_program(&path)?;
                crate::pdftex::writet3::type3_bitmap_program(data.as_ref().clone())
                    .map(|p| (path, p))
            });
        Type3 {
            matrix,
            dpi,
            program,
        }
    }

    /// The key of image `/Im<n>` as the engine has it now (describing it in
    /// the resource store), or `None` if the page's image list lacks it.
    fn dl_image_key(&mut self, n: u32) -> Option<[u8; 32]> {
        if let Some(k) = with(|st| st.image_keys.get(&n).copied()).flatten() {
            return Some(k);
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
        let img = img?;
        let info = self.dl_image_info(img);
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
        with(|st| {
            st.images.entry(key).or_insert(info);
            st.image_keys.insert(n, key);
        });
        Some(key)
    }

    fn dl_image_info(&mut self, img: i32) -> Json {
        use crate::pdftex::images::{
            ImageData, IMAGE_TYPE_JBIG2, IMAGE_TYPE_JPG, IMAGE_TYPE_PDF, IMAGE_TYPE_PNG,
        };
        crate::pdftex::with_state(|st| {
            let Some(e) = st.img.images.get(img as usize) else {
                return Json::Null;
            };
            // By what `read_image` found, which `delete_image` (once the
            // XObject is written) does not take: `image_type` and `file`,
            // not `data` and `name`.
            let typ = match e.image_type {
                IMAGE_TYPE_PDF => "pdf",
                IMAGE_TYPE_PNG => "png",
                IMAGE_TYPE_JPG => "jpeg",
                IMAGE_TYPE_JBIG2 => "jbig2",
                _ => "none",
            };
            // A PDF page's box in bp, as the PDF gives it (protocol §5.2):
            // pdfTeX's own fields hold it in scaled points (`bp2int`).
            // Rounded to 1e-4 bp, which drops only the f32's binary tail.
            let bp = |v: f32| Json::Num((v as f64 * 1e4).round() / 1e4);
            let (width, height) = match &e.data {
                ImageData::Pdf(p) => (bp(p.box_bp[2]), bp(p.box_bp[3])),
                _ => (Json::Int(e.width as i64), Json::Int(e.height as i64)),
            };
            let extra = match &e.data {
                ImageData::Pdf(p) => vec![
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
                    ("orig_x".to_string(), bp(p.box_bp[0])),
                    ("orig_y".to_string(), bp(p.box_bp[1])),
                ],
                _ => vec![],
            };
            let mut kv = vec![
                ("type".to_string(), js(typ)),
                (
                    "file".to_string(),
                    e.file
                        .as_ref()
                        .map(|n| js(absolute(n)))
                        .unwrap_or(Json::Null),
                ),
                ("width".to_string(), width),
                ("height".to_string(), height),
                ("rotate".to_string(), Json::Int(e.rotate as i64)),
                ("x_res".to_string(), Json::Int(e.x_res as i64)),
                ("y_res".to_string(), Json::Int(e.y_res as i64)),
            ];
            kv.extend(extra);
            Json::Obj(kv)
        })
    }
}

/// A file the resolver finds, without it counting as a read of the run:
/// the writer's lookups are not the engine's (its read journal, which the
/// incremental system compares across runs, must not see them).
fn find_quietly(name: &str, format: Format) -> Option<String> {
    crate::system::lookup_again(&crate::system::Lookup {
        name: name.to_string(),
        format,
        must_exist: None,
        found: None,
    })
}

/// A font program and its SHA-256, read once per change of the file.
fn read_program(path: &str) -> Option<(Arc<Vec<u8>>, [u8; 32])> {
    let m = std::fs::metadata(path).ok()?;
    let (len, mtime) = (m.len(), m.modified().ok());
    if let Some(hit) = with(|st| {
        st.programs
            .get(path)
            .filter(|p| p.len == len && p.mtime == mtime)
            .map(|p| (p.data.clone(), p.sha))
    })
    .flatten()
    {
        return Some(hit);
    }
    let data = Arc::new(std::fs::read(path).ok()?);
    let sha = sha256(&data);
    with(|st| {
        st.programs.insert(
            path.to_string(),
            Program {
                len,
                mtime,
                data: data.clone(),
                sha,
            },
        )
    });
    Some((data, sha))
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
fn read_enc(name: &str) -> Option<Arc<Vec<Vec<u8>>>> {
    if let Some(hit) = with(|st| st.encodings.get(name).cloned()).flatten() {
        return hit;
    }
    let names = find_quietly(name, Format::Enc)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|d| parse_enc(&d))
        .map(Arc::new);
    with(|st| st.encodings.insert(name.to_string(), names.clone()));
    names
}

/// `read_enc`'s parse of an encoding file's bytes.
fn parse_enc(data: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut out = vec![b".notdef".to_vec(); 256];
    // Strip comments, then take the names between the first `[` and `]`.
    let mut text = Vec::with_capacity(data.len());
    let mut in_comment = false;
    for &c in data {
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
/// [`builtin_encoding`] of a program whose SHA-256 is `sha`, worked out once
/// per program: every restore forgets the fonts' keys (`restored`), and the
/// next page worked them out again, decrypting each Type 1 program's
/// cleartext for its encoding (0.2 ms of every keystroke on a 1,000-page
/// hyperref document; lane P4-PAGE-COST).
fn builtin_encoding_of(program: &[u8], sha: &[u8; 32]) -> Arc<Vec<Vec<u8>>> {
    if let Some(hit) = with(|st| st.builtin_encodings.get(sha).cloned()).flatten() {
        return hit;
    }
    let names = Arc::new(builtin_encoding(program));
    with(|st| st.builtin_encodings.insert(*sha, names.clone()));
    names
}

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
    let toks: Vec<&[u8]> = ps_tokens(rest).take_while(|t| *t != b"def").collect();
    if toks.first() == Some(&&b"["[..]) {
        let mut i = 0;
        for t in &toks[1..] {
            if *t == b"]" {
                break;
            }
            if let Some(name) = t.strip_prefix(b"/") {
                if i < 256 {
                    out[i] = name.to_vec();
                }
                i += 1;
            }
        }
        return out;
    }
    // pdfTeX's `t1_builtin_enc` matches `sscanf(p, "dup %i%255s put")`, so
    // `dup 1/uni6301 put` (no space before the name) is an entry too.
    for w in toks.windows(4) {
        if w[0] == b"dup" && w[3] == b"put" && w[2].starts_with(b"/") {
            if let Some(code) = c_int(w[1]).filter(|c| (0..256).contains(c)) {
                out[code as usize] = w[2][1..].to_vec();
            }
        }
    }
    out
}

/// PostScript tokens of `s`: whitespace separates tokens, `[ ] { }` are
/// tokens of their own, `%` starts a comment, and `/`, `(` and `<` start a
/// new token even with no whitespace before them (a string or hex string
/// is one token).
fn ps_tokens(s: &[u8]) -> impl Iterator<Item = &[u8]> {
    let delim = |c: u8| c.is_ascii_whitespace() || b"[]{}()<>/%".contains(&c);
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < s.len() {
            if s[i].is_ascii_whitespace() {
                i += 1;
            } else if s[i] == b'%' {
                while i < s.len() && s[i] != b'\n' && s[i] != b'\r' {
                    i += 1;
                }
            } else {
                break;
            }
        }
        if i >= s.len() {
            return None;
        }
        let start = i;
        match s[i] {
            b'[' | b']' | b'{' | b'}' | b')' | b'>' => i += 1,
            b'(' => {
                let mut depth = 0usize;
                while i < s.len() {
                    match s[i] {
                        b'\\' => i += 1,
                        b'(' => depth += 1,
                        b')' => {
                            depth -= 1;
                            if depth == 0 {
                                i += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            b'<' => {
                i += 1;
                if s.get(i) == Some(&b'<') {
                    i += 1;
                } else {
                    while i < s.len() && s[i] != b'>' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            c => {
                i += 1;
                if c == b'/' && s.get(i) == Some(&b'/') {
                    i += 1;
                }
                while i < s.len() && !delim(s[i]) {
                    i += 1;
                }
            }
        }
        i = i.min(s.len());
        Some(&s[start..i])
    })
}

/// C's `%i`, as pdfTeX's `sscanf` reads the code: decimal, `0x` hex or
/// leading-`0` octal, optionally signed.
fn c_int(t: &[u8]) -> Option<i64> {
    let s = std::str::from_utf8(t).ok()?;
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let v = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(h, 16).ok()?
    } else if s.len() > 1 && s.starts_with('0') {
        i64::from_str_radix(&s[1..], 8).ok()?
    } else {
        s.parse().ok()?
    };
    Some(if neg { -v } else { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Globals::file_level` finds what walking the input stack finds, on
    /// random sequences of the pushes and pops TeX makes: token lists
    /// (`begin_token_list`, state `token_list`, `index` its token type)
    /// and `begin_file_reading` levels (`index:=in_open`; real files,
    /// `\read` levels and pseudo files by `name`), with the hint left from
    /// each previous call, and with a hint from another stack (a restore).
    #[test]
    fn file_level_finds_what_the_walk_finds() {
        use crate::generated::consts::token_list;
        use crate::generated::types::in_state_record;
        let mut g = Globals::new();
        let is_file = |r: &in_state_record| r.state_field != 0 && r.name_field > 17;
        let walk = |g: &Globals| {
            (0..g.input_ptr as usize)
                .rev()
                .map(|k| g.input_stack[k])
                .find(|r| is_file(r))
        };
        let hint = AtomicUsize::new(usize::MAX);
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut rnd = |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        // the bottom level: the terminal
        g.input_ptr = 0;
        g.in_open = 0;
        g.cur_input = in_state_record {
            state_field: 1,
            index_field: 0,
            name_field: 0,
            ..Default::default()
        };
        let mut checked = 0;
        for step in 0..200_000 {
            let push = g.input_ptr < 60 && (g.input_ptr == 0 || rnd(100) < 52);
            if push {
                let k = g.input_ptr as usize;
                g.input_stack[k] = g.cur_input;
                g.input_ptr += 1;
                g.cur_input = if rnd(5) == 0 && g.in_open < 14 {
                    g.in_open += 1;
                    let name = match rnd(6) {
                        0 => 1 + rnd(17) as i32, // `\read`, the terminal
                        1 => 18 + rnd(2) as i32, // a pseudo file
                        _ => 100 + rnd(1000) as i32,
                    };
                    in_state_record {
                        state_field: 1 + rnd(3) as i32,
                        index_field: g.in_open,
                        name_field: name,
                        ..Default::default()
                    }
                } else {
                    in_state_record {
                        state_field: token_list,
                        index_field: rnd(20) as i32,
                        name_field: rnd(2000) as i32,
                        ..Default::default()
                    }
                };
            } else {
                if g.cur_input.state_field != token_list {
                    g.in_open -= 1;
                }
                g.input_ptr -= 1;
                g.cur_input = g.input_stack[g.input_ptr as usize];
            }
            if step % 997 == 0 {
                // a hint from another stack (a restore)
                hint.store(rnd(70) as usize, Ordering::Relaxed);
            }
            let want = walk(&g);
            let got = g.file_level(&hint, is_file);
            assert_eq!(
                got.map(|r| (r.index_field, r.name_field, r.state_field)),
                want.map(|r| (r.index_field, r.name_field, r.state_field)),
                "step {step}"
            );
            checked += want.is_some() as usize;
        }
        assert!(checked > 10_000, "too few file levels found: {checked}");
    }

    /// The eqtb locations above are the translation's: `pdf_ship_out`
    /// prints `count(k)` and `pdf_print_mag_bp` reads `mag` there.
    #[test]
    fn eqtb_locations_match_the_translation() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
        let mut all = String::new();
        for e in std::fs::read_dir(dir).unwrap() {
            all.push_str(&std::fs::read_to_string(e.unwrap().path()).unwrap());
        }
        // A routine indexes `eqtb` through its local view (web2rust
        // --array-view, crate::arena::ArrView): the same element.
        let all = all.replace("__av_eqtb[", "self.eqtb[");
        // Subscripts are wrapped in `crate::ix::U(...)` (web2rust
        // --index-type, src/ix.rs). `count_base` is a named macro constant;
        // `int_base+mag_code` is folded by TANGLE into one number.
        assert!(all.contains(
            "self.print_int(((self.eqtb[crate::ix::U((((count_base).wrapping_add(k)) - 1)"
        ));
        let mag_bp = all.split("pub fn pdf_print_mag_bp").nth(1).unwrap();
        assert!(mag_bp[..400].contains(&format!(
            "self.eqtb[crate::ix::U((({MAG_LOC}i32) - 1) as usize)].int() != 1000i32"
        )));
    }

    /// `State::move_spans_of` leaves the table as rebuilding it whole did:
    /// random spans, retired ones and moves, against that rebuild.
    #[test]
    fn moving_spans_keeps_the_table_a_rebuild_makes() {
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut rnd = |n: u32| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as u32
        };
        for round in 0..600 {
            // every other round about the line the index's table ends at
            let base = if round % 2 == 0 {
                0
            } else {
                SpanIndex::DENSE - 15
            };
            let mut st = State::new();
            for _ in 0..60 {
                let (f, l) = (1 + rnd(2), base + 1 + rnd(30));
                st.span_id(f, l);
            }
            for _ in 0..4 {
                let from = base + 1 + rnd(30);
                let old_end = from + rnd(4);
                let new_end = (from + rnd(6)).max(1);
                let f = 1 + rnd(2);
                // the rebuild, on a copy
                let mut spans = st.spans.clone();
                let retired_of = |st: &State| -> HashSet<u32> {
                    (1..=st.spans.len() as u32)
                        .filter(|&id| st.is_retired(id))
                        .collect()
                };
                let mut retired = retired_of(&st);
                let delta = new_end as i64 - old_end as i64;
                for (i, (file, line)) in spans.iter_mut().enumerate() {
                    if *file != f || *line < from {
                        continue;
                    }
                    if *line < old_end {
                        retired.insert(i as u32 + 1);
                    } else {
                        *line = (*line as i64 + delta).max(1) as u32;
                    }
                }
                let mut want: HashMap<(u32, u32), u32> = HashMap::new();
                for (i, &at) in spans.iter().enumerate() {
                    if !retired.contains(&(i as u32 + 1)) {
                        want.entry(at).or_insert(i as u32 + 1);
                    }
                }
                st.move_spans_of(f, from, old_end, new_end);
                assert_eq!(st.spans, spans);
                assert_eq!(retired_of(&st), retired);
                assert_eq!(st.span_ids.to_map(), want);
                // new spans after the move, as a compile makes them
                for _ in 0..10 {
                    let (f, l) = (1 + rnd(2), base + 1 + rnd(30));
                    st.span_id(f, l);
                }
            }
        }
    }

    /// `move_lines`: spans after an edit move with their lines, spans of
    /// replaced lines are kept but not reused, and a reader is told where
    /// the spans it holds went. (The writer's state only: the hooks, which
    /// the process-wide `ENABLED` turns on, stay off.)
    #[test]
    fn spans_follow_moved_lines() {
        DL.with(|d| *d.borrow_mut() = Some(Box::new(State::new())));
        let (a, b, c) = with(|st| {
            let f = st.file_id(b"/p/main.tex");
            (st.span_id(f, 5), st.span_id(f, 10), st.span_id(f, 20))
        })
        .unwrap();
        let mut peer = Peer::default();
        assert!(peer.sources_for(&[a, b, c]).is_some());
        assert!(peer.moved_spans().is_none());
        // Old lines 8..12 became 8..9: three lines fewer.
        move_lines("/p/main.tex", 8, 12, 9);
        assert_eq!(span_location(a), Some((1, 5)));
        assert_eq!(span_location(b), Some((1, 10)));
        assert_eq!(span_location(c), Some((1, 17)));
        // New text on line 10 gets a span of its own; line 17 is c's.
        let (d, e) = with(|st| (st.span_id(1, 10), st.span_id(1, 17))).unwrap();
        assert_ne!(d, b);
        assert_eq!(e, c);
        let moved = peer.moved_spans().expect("c moved");
        let j = Json::parse(std::str::from_utf8(&moved).unwrap()).unwrap();
        let src = Sources::from_json(&j).unwrap();
        assert_eq!(src.spans, vec![(c, 1, 17)]);
        assert!(src.files.is_empty(), "the reader has the file already");
        assert!(peer.moved_spans().is_none());
        DL.with(|d| *d.borrow_mut() = None);
    }

    /// A destination carries only the words pdfTeX writes for its kind.
    /// `get_node` does not clear a node, and `do_dest` leaves the rest of
    /// a dest node as the memory held it (`pdf_bottom` of an `xyz` dest
    /// is never set): a restored run and a run from scratch hold different
    /// values there (the fixture soundness test, conf-paper edit 0).
    #[test]
    fn a_dest_carries_only_the_words_its_kind_uses() {
        let mut g = Globals::new();
        // The dest list's one entry (node 100) names object 1, whose dest
        // node (200) is all stale words except what the kind sets.
        let (k, i) = (100i32, 200i32);
        // (the arrays a format load sizes)
        if g.mem.len() < 300 {
            g.mem.resize_len(300);
        }
        if g.obj_tab.len() < 2 {
            g.obj_tab.resize_len(2);
        }
        let dest = |g: &mut Globals, kind: i32| -> Dest {
            for w in 0..7 {
                g.mem[(i + w) as usize].set_int(0x5a5a + w);
                g.mem[(i + w) as usize].set_hh_lh(0x3c3c + w);
            }
            g.mem[(i + 1) as usize].set_int(1000); // pdf_left
            g.mem[(i + 2) as usize].set_int(2000); // pdf_top
            g.mem[(i + 5) as usize].set_hh_b0(kind); // pdf_dest_type
            g.mem[(i + 5) as usize].set_hh_b1(0); // pdf_dest_named_id: num
            g.mem[(i + 5) as usize].set_hh_rh(7); // pdf_dest_id
            g.mem[k as usize].set_hh_lh(1);
            g.mem[k as usize].set_hh_rh(0);
            g.obj_tab[1].int4 = i; // obj_dest_ptr
            g.pdf_dest_list = k;
            g.pdf_link_list = 0;
            let mut page = Page::new(StreamKind::Page, 0);
            g.dl_links(&mut page, 1000);
            assert_eq!(page.dests.len(), 1);
            page.dests.pop().unwrap()
        };
        let d = dest(&mut g, 0);
        assert_eq!((d.named, &d.name[..], d.kind), (false, &b"7"[..], 0));
        assert_eq!((d.rect, d.zoom), ([1000, 2000, 0, 0], 0x3c3c + 6), "xyz");
        assert_eq!(
            (dest(&mut g, 1).rect, dest(&mut g, 1).zoom),
            ([0; 4], 0),
            "fit"
        );
        assert_eq!(dest(&mut g, 2).rect, [0, 2000, 0, 0], "fith");
        assert_eq!(dest(&mut g, 3).rect, [1000, 0, 0, 0], "fitv");
        assert_eq!(dest(&mut g, 4).rect, [0; 4], "fitb");
        assert_eq!(dest(&mut g, 5).rect, [0, 2000, 0, 0], "fitbh");
        assert_eq!(dest(&mut g, 6).rect, [1000, 0, 0, 0], "fitbv");
        let r = dest(&mut g, 7);
        assert_eq!(
            (r.rect, r.zoom),
            ([1000, 2000, 0x5a5a + 3, 0x5a5a + 4], 0),
            "fitr"
        );
    }

    #[test]
    fn locations_pack() {
        let l = loc_pack(1234567, 42);
        assert_eq!((loc_span(l), loc_col(l)), (1234567, 42));
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
        // No whitespace before the name, as in the Arphic gbsnu fonts.
        let tight = b"/Encoding 256 array\n 0 1 255 { 1 index exch /.notdef put} for\ndup 1/uni6301 put\ndup 0x41/A put\ndup 7 /uni6307 put\nreadonly def\n";
        let e = builtin_encoding(tight);
        assert_eq!(e[1], b"uni6301");
        assert_eq!(e[0x41], b"A");
        assert_eq!(e[7], b"uni6307");
        assert_eq!(e[2], b".notdef");
        let arr = builtin_encoding(b"/Encoding[/a/b /.notdef/c]readonly def\n");
        assert_eq!(&arr[..4], &[&b"a"[..], b"b", b".notdef", b"c"]);
        assert_eq!(arr[4], b".notdef");
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

    /// A program's built-in encoding is worked out once and kept across
    /// restores (`restored` forgets the fonts' keys, not this).
    #[test]
    fn builtin_encodings_are_kept_by_program() {
        DL.with(|d| *d.borrow_mut() = Some(Box::new(State::new())));
        let pfa = b"/Encoding 256 array\ndup 65 /A put\nreadonly def\n";
        let other = b"/Encoding StandardEncoding def\n";
        let (sa, so) = (sha256(pfa), sha256(other));
        let a = builtin_encoding_of(pfa, &sa);
        assert_eq!(*a, builtin_encoding(pfa));
        forget_engine_state();
        assert!(Arc::ptr_eq(&a, &builtin_encoding_of(pfa, &sa)));
        let o = builtin_encoding_of(other, &so);
        assert_eq!(*o, builtin_encoding(other));
        assert!(!Arc::ptr_eq(&a, &o));
        DL.with(|d| *d.borrow_mut() = None);
    }

    /// A real font whose encoding writes `dup 1/uni6301 put`; skipped when
    /// the local TeX Live has no Arphic gbsnu fonts.
    #[test]
    fn builtin_encoding_of_arphic_gbsnu() {
        let Ok(out) = std::process::Command::new("kpsewhich")
            .arg("gbsnu63.pfb")
            .output()
        else {
            return;
        };
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let Ok(pfb) = std::fs::read(&path) else {
            return;
        };
        let e = builtin_encoding(&pfb);
        assert_eq!(e[1], b"uni6301");
        assert_eq!(e[2], b"uni6302");
        assert_eq!(e[7], b"uni6307");
        assert_eq!(e[0], b".notdef");
    }
}
