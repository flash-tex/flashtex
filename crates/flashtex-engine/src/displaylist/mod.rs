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
//! ([`init_from_env`]: `fd:N`, an inherited descriptor, or a file path) the
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
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

static ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static DL: RefCell<Option<Box<State>>> = const { RefCell::new(None) };
    /// Where emitted pages go (apart from `DL`, so that a sink may call
    /// back into the writer: [`Peer::send`] reads resources).
    static SINK: RefCell<Option<Box<dyn Sink>>> = const { RefCell::new(None) };
    static SIDE: RefCell<Side> = RefCell::new(Side::new());
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
    span_ids: HashMap<(u32, u32), u32>,
    /// Spans of lines an edit replaced (`move_lines`): kept, never reused.
    retired: HashSet<u32>,
    // Resources by key: descriptions that do not depend on engine state.
    fonts: HashMap<[u8; 32], FontRes>,
    images: HashMap<[u8; 32], Json>,
    programs: HashMap<String, Program>,
    encodings: HashMap<String, Option<Arc<Vec<Vec<u8>>>>>,
    // Engine state as the writer saw it: dropped at every restore.
    font_keys: HashMap<u32, [u8; 32]>,
    image_keys: HashMap<u32, [u8; 32]>,
    widths: HashMap<u32, Option<Box<[i64; 256]>>>,
    capture: Option<Capture>,
    cwd: Option<std::path::PathBuf>,
}

impl State {
    fn new() -> State {
        State {
            files: Vec::new(),
            file_paths: Vec::new(),
            file_ids: HashMap::new(),
            spans: Vec::new(),
            span_ids: HashMap::new(),
            retired: HashSet::new(),
            fonts: HashMap::new(),
            images: HashMap::new(),
            programs: HashMap::new(),
            encodings: HashMap::new(),
            font_keys: HashMap::new(),
            image_keys: HashMap::new(),
            widths: HashMap::new(),
            capture: None,
            cwd: std::env::current_dir().ok(),
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
        if let Some(&s) = self.span_ids.get(&(file, line)) {
            return s;
        }
        self.spans.push((file, line));
        let id = self.spans.len() as u32;
        self.span_ids.insert((file, line), id);
        id
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
    SIDE.with(|s| s.borrow_mut().reset());
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
    SIDE.with(|s| s.borrow_mut().reset());
}

/// Start writing display lists if `FLASHTEX_DISPLAY_LIST` asks for it
/// (before the engine allocates its first node). `FLASHTEX_DISPLAY_LIST_HAVE_FONTS`
/// lists font keys (hex, comma-separated) whose programs the reader holds.
pub fn init_from_env() {
    let Some(spec) = std::env::var_os("FLASHTEX_DISPLAY_LIST") else {
        return;
    };
    let spec = spec.to_string_lossy().into_owned();
    let w: Box<dyn Write> = if let Some(fd) = spec.strip_prefix("fd:") {
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
    let peer = Peer {
        have_fonts: parse_font_keys(
            &std::env::var("FLASHTEX_DISPLAY_LIST_HAVE_FONTS").unwrap_or_default(),
        ),
        ..Peer::default()
    };
    init_with_sink(Box::new(StreamSink { w: Some(w), peer }));
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
            let held = self.have_fonts.contains(&key);
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
    fn sources_for(&mut self, spans: &[u32]) -> Option<Vec<u8>> {
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
        let f = f as u32 + 1;
        let delta = new_end as i64 - old_end as i64;
        for (i, (file, line)) in st.spans.iter_mut().enumerate() {
            if *file != f || *line < from {
                continue;
            }
            if *line < old_end {
                st.retired.insert(i as u32 + 1);
            } else {
                *line = (*line as i64 + delta).max(1) as u32;
            }
        }
        st.span_ids.clear();
        for (i, &at) in st.spans.iter().enumerate() {
            if !st.retired.contains(&(i as u32 + 1)) {
                st.span_ids.entry(at).or_insert(i as u32 + 1);
            }
        }
    });
    forget_file_cache();
}

// ---------------------------------------------------------------------------
// the side table: a copy-on-write array indexed like `mem`

/// Entries per chunk of the side table (16 KB).
const SIDE_SHIFT: usize = 11;
const SIDE_CHUNK: usize = 1 << SIDE_SHIFT;

type Chunk = [Loc; SIDE_CHUNK];

/// The side table: `mem_max + 1` locations in chunks shared with the
/// snapshots taken since each was last written (the first write after a
/// snapshot copies the chunk).
struct Side {
    chunks: Vec<Arc<Chunk>>,
    /// Whether chunk `c` is this table's alone (writable in place).
    owned: Vec<bool>,
    /// `chunks[c]`'s data, for the hooks.
    ptrs: Vec<*mut Loc>,
}

fn zero_chunk() -> Arc<Chunk> {
    thread_local! {
        static ZERO: Arc<Chunk> = Arc::new([0; SIDE_CHUNK]);
    }
    ZERO.with(Arc::clone)
}

impl Side {
    fn new() -> Side {
        Side {
            chunks: Vec::new(),
            owned: Vec::new(),
            ptrs: Vec::new(),
        }
    }

    fn len_chunks() -> usize {
        (crate::generated::consts::mem_max as usize + 1).div_ceil(SIDE_CHUNK)
    }

    /// Every entry 0.
    fn reset(&mut self) {
        let z = zero_chunk();
        self.set_chunks(vec![z; Self::len_chunks()]);
    }

    fn set_chunks(&mut self, chunks: Vec<Arc<Chunk>>) {
        self.owned = vec![false; chunks.len()];
        self.ptrs = chunks
            .iter()
            .map(|c| Arc::as_ptr(c) as *const Loc as *mut Loc)
            .collect();
        self.chunks = chunks;
        self.publish();
    }

    fn publish(&mut self) {
        SIDE_PTRS.store(self.ptrs.as_mut_ptr(), Ordering::Relaxed);
        SIDE_OWNED.store(self.owned.as_mut_ptr(), Ordering::Relaxed);
        SIDE_CHUNKS.store(self.ptrs.len() as u32, Ordering::Relaxed);
    }

    /// Make chunk `c` writable in place.
    #[cold]
    fn own(&mut self, c: usize) {
        let data = Arc::make_mut(&mut self.chunks[c]);
        self.ptrs[c] = data.as_mut_ptr();
        self.owned[c] = true;
    }
}

// The hooks read and write the side table on TeX's inner loop (every node
// and token allocated), so they go through these process statics rather
// than the thread-local state: a display list is written by one engine per
// process, on one thread.
static SIDE_PTRS: AtomicPtr<*mut Loc> = AtomicPtr::new(std::ptr::null_mut());
static SIDE_OWNED: AtomicPtr<bool> = AtomicPtr::new(std::ptr::null_mut());
static SIDE_CHUNKS: AtomicU32 = AtomicU32::new(0);

/// The side table as it is now, for a checkpoint (`crate::pdftex::CState`):
/// its chunks, shared until either side writes them.
#[derive(Clone, Default)]
pub struct Snap(Option<Arc<Vec<Arc<Chunk>>>>);

impl std::fmt::Debug for Snap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "displaylist::Snap({})", self.0.is_some())
    }
}

/// A persisted checkpoint (S₀, `crate::host`) does not carry the side
/// table: nodes made before it are restored without a source span.
impl crate::persist::Codec for Snap {
    fn enc(&self, _w: &mut Vec<u8>) {}
    fn dec(_r: &mut crate::persist::Reader) -> Result<Self, String> {
        Ok(Snap(None))
    }
}

/// Snapshot the side table (a checkpoint is being taken).
pub fn snapshot() -> Snap {
    if !enabled() {
        return Snap(None);
    }
    SIDE.with(|s| {
        let mut s = s.borrow_mut();
        let v = Arc::new(s.chunks.clone());
        s.owned.iter_mut().for_each(|o| *o = false);
        Snap(Some(v))
    })
}

/// Put a snapshot back (a checkpoint is being restored), and forget what
/// was cached from the engine state it replaces.
pub fn restore(snap: &Snap) {
    if !enabled() {
        return;
    }
    SIDE.with(|s| {
        let mut s = s.borrow_mut();
        match &snap.0 {
            Some(v) => s.set_chunks(v.as_ref().clone()),
            None => s.reset(),
        }
    });
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
/// While `hyphenate` rebuilds a word: HYPH_ON, and the word's place.
static HYPH_ON: AtomicBool = AtomicBool::new(false);
static HYPH_LOC: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
fn side_get(p: i32) -> Loc {
    let ptrs = SIDE_PTRS.load(Ordering::Relaxed);
    let c = (p as u32 as usize) >> SIDE_SHIFT;
    if ptrs.is_null() || p < 0 || c >= SIDE_CHUNKS.load(Ordering::Relaxed) as usize {
        return 0;
    }
    // SAFETY: `ptrs` is `Side::ptrs` (published by `Side::publish` and
    // replaced only on the engine's thread, the only one that uses it), with
    // `SIDE_CHUNKS` entries, each a whole chunk.
    unsafe { *(*ptrs.add(c)).add(p as usize & (SIDE_CHUNK - 1)) }
}

#[inline(always)]
fn side_set(p: i32, v: Loc) {
    let ptrs = SIDE_PTRS.load(Ordering::Relaxed);
    let c = (p as u32 as usize) >> SIDE_SHIFT;
    if ptrs.is_null() || p < 0 || c >= SIDE_CHUNKS.load(Ordering::Relaxed) as usize {
        return;
    }
    // SAFETY: as in `side_get`; `SIDE_OWNED` has as many entries. `own`
    // replaces the chunk's pointer, so it is read again after.
    unsafe {
        if !*SIDE_OWNED.load(Ordering::Relaxed).add(c) {
            SIDE.with(|s| s.borrow_mut().own(c));
        }
        let ptrs = SIDE_PTRS.load(Ordering::Relaxed);
        *(*ptrs.add(c)).add(p as usize & (SIDE_CHUNK - 1)) = v;
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
        // The keys of the fonts and images the items use, then the spans
        // the items and links name.
        let fonts: Vec<(u32, [u8; 32])> = out
            .fonts
            .iter()
            .map(|&f| (f, self.dl_font_key(f)))
            .collect();
        let images: Vec<(u32, [u8; 32])> = out
            .images
            .iter()
            .filter_map(|&n| self.dl_image_key(n).map(|k| (n, k)))
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
        with_sink(|s| s.emit(e));
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
            let span = loc_span(side_get(i));
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
        if let Some(fm) = &fm {
            slant = fm.slant;
            extend = fm.extend;
            ps_name = fm.ps_name.clone().unwrap_or_default();
            if fm.is_type1() && fm.is_included() {
                if let Some(ff) = &fm.ff_name {
                    let name = String::from_utf8_lossy(ff).into_owned();
                    if let Some(path) = find_quietly(&name, Format::Type1) {
                        if let Some((data, sha)) = read_program(&path) {
                            program = data;
                            program_sha = sha;
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
            names = Some(Arc::new(builtin_encoding(&program)));
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
        let key = h.finish();
        let known = with(|st| st.fonts.contains_key(&key)).unwrap_or(true);
        if !known {
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
            with(|st| st.fonts.insert(key, FontRes { info, program }));
        }
        with(|st| st.font_keys.insert(f, key));
        key
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
