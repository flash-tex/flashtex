//! The display list's source map (spec §5.3, `SOURCES`, `SPAN` and each
//! GLYPH's column): which source line and column made what a page draws,
//! for the preview's reverse and forward search. The engine side of the
//! pdfTeX engine's (`crates/flashtex-engine/src/displaylist/mod.rs`:
//! `dl_new_node`, `dl_here`, the spans), for XeTeX: the hooks of
//! `changes/displaylist.ch` call the routines here; they do nothing unless
//! a display list was asked for (`FLASHTEX_DISPLAY_LIST`) and never change
//! a variable of TeX's.
//!
//! * As a node is allocated, its source position goes into a side table
//!   indexed like `mem`: the span (file, line) of the innermost open file's
//!   line and the byte column of the last character read from it (XeTeX's
//!   `buffer` holds Unicode scalars; columns are UTF-8 bytes, as the
//!   editor counts them).
//! * A native word's node is made once the whole word has been read, so
//!   its column would be the word's end; [`Globals::dl_native_word`] (from
//!   `set_native_metrics`) moves it to the word's first character, found in
//!   the line before that point.
//! * As `hlist_out` and `vlist_out` output a node, its position is noted
//!   with the XDV stream's offset at that moment (a marker); the page reader
//!   (`out::page`) gives what it draws from there on that span and column.

use crate::generated::consts::token_list;
use crate::generated::types::in_state_record;
use crate::generated::Globals;
use std::collections::HashMap;

/// A node's position: the span in bits 16..48, the byte column in bits
/// 0..16 (`NO_COLUMN`: none), bit 63 when a native word's column is its
/// first character's.
pub type Loc = u64;

pub const NO_COLUMN: u16 = 0xFFFF;
const START_FOUND: u64 = 1 << 63;

fn pack(span: u32, col: u16) -> Loc {
    (span as u64) << 16 | col as u64
}

pub fn loc_span(l: Loc) -> u32 {
    (l >> 16) as u32
}

pub fn loc_col(l: Loc) -> u16 {
    l as u16
}

/// One marker: from XDV stream offset `pos` on, what is drawn is at `span`
/// and `col`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marker {
    pub pos: u64,
    pub span: u32,
    pub col: u16,
}

/// The source map's state (a field of the engine's `Host`).
#[derive(Default)]
pub struct DlSource {
    /// Whether a display list is written (decided at the first node).
    on: Option<bool>,
    /// The position of the node at each `mem` location.
    side: Side,
    /// File id - 1 → absolute path; name bytes → file id.
    files: Vec<String>,
    file_ids: HashMap<Vec<u8>, u32>,
    /// Span id - 1 → (file id, line); (file id, line) → span id.
    spans: Vec<(u32, u32)>,
    span_ids: HashMap<(u32, u32), u32>,
    /// The last (file name string, line) looked up, and its span.
    last: (i32, u32, u32),
    /// The input stack level found last as the innermost file line.
    file_level: usize,
    not_a_node: bool,
    /// The last column worked out: (file name, line, line start, character
    /// index, its byte column), carried forward as TeX reads on.
    col_cache: (i32, i32, i32, usize, usize),
    hyph: Option<Loc>,
    /// Markers not yet taken by a page, in stream order.
    pub markers: Vec<Marker>,
    last_marked: Loc,
}

/// A node's source position as a place: (file path, line, column and
/// flag), from the tables of the run that made it (the convergence test,
/// `crate::conv`, compares two runs' positions this way: their span and
/// file ids are numbered as each run met them).
pub type Place<'a> = Option<(&'a str, u32, u64)>;

/// The span and file tables of a run ([`DlSource::tables`]).
#[derive(Clone, Default)]
pub struct Tables {
    spans: Vec<(u32, u32)>,
    files: Vec<String>,
}

impl Tables {
    pub fn place(&self, l: Loc) -> Place<'_> {
        if l == 0 {
            return None;
        }
        let s = loc_span(l);
        let rest = l & !(0xFFFF_FFFFu64 << 16);
        let (f, line) = self.spans.get((s as usize).checked_sub(1)?).copied()?;
        let path = self.files.get((f as usize).checked_sub(1)?)?;
        Some((path.as_str(), line, rest))
    }
}

impl DlSnap {
    /// Node `p`'s position in this snapshot.
    pub fn at(&self, p: i32) -> Loc {
        self.side.get(p as u32 as usize)
    }
}

impl DlSource {
    /// The span and file tables as they are (a run's, all of whose
    /// positions they name).
    pub fn tables(&self) -> Tables {
        Tables {
            spans: self.spans.clone(),
            files: self.files.clone(),
        }
    }

    /// Node `p`'s position now.
    pub fn at(&self, p: i32) -> Loc {
        self.get(p)
    }

    /// Whether a display list is made.
    pub fn on(&self) -> bool {
        self.on == Some(true)
    }

    fn file_id(&mut self, name: &[u8]) -> u32 {
        if let Some(&i) = self.file_ids.get(name) {
            return i;
        }
        let mut path = String::from_utf8_lossy(name).into_owned();
        if !path.starts_with('/') {
            if let Ok(cwd) = std::env::current_dir() {
                path = cwd
                    .join(path.trim_start_matches("./"))
                    .to_string_lossy()
                    .into_owned();
            }
        }
        self.files.push(path);
        let id = self.files.len() as u32;
        self.file_ids.insert(name.to_vec(), id);
        id
    }

    fn span_id(&mut self, file: u32, line: u32) -> u32 {
        if let Some(&s) = self.span_ids.get(&(file, line)) {
            return s;
        }
        self.spans.push((file, line));
        let id = self.spans.len() as u32;
        self.span_ids.insert((file, line), id);
        id
    }

    /// A page was shipped: the next page's first node is marked again.
    pub fn page_done(&mut self) {
        self.last_marked = 0;
    }

    /// Where span `s` is: (file id, line).
    pub fn span(&self, s: u32) -> Option<(u32, u32)> {
        self.spans.get((s as usize).checked_sub(1)?).copied()
    }

    /// The absolute path of file id `f`.
    pub fn file(&self, f: u32) -> Option<&str> {
        self.files
            .get((f as usize).checked_sub(1)?)
            .map(String::as_str)
    }

    #[inline(always)]
    fn get(&self, p: i32) -> Loc {
        self.side.get(p as u32 as usize)
    }

    #[inline(always)]
    fn set(&mut self, p: i32, v: Loc) {
        self.side.set(p as u32 as usize, v);
    }

    /// A resident engine's run: the source map is made when its display
    /// list is (its host says so; a run of its own reads
    /// `FLASHTEX_DISPLAY_LIST`), over a `mem` of `mem_len` words.
    pub fn set_on(&mut self, on: bool, mem_len: usize) {
        if on && self.side.len() != mem_len {
            self.side = Side::zeros(mem_len);
        }
        self.on = Some(on);
    }

    /// Whether the source map is made.
    pub fn is_on(&self) -> bool {
        self.on == Some(true)
    }

    /// The state at a checkpoint (`state.rs`'s `Globals::checkpoint`): the
    /// side table shared chunk by chunk (copied on the next write to each),
    /// the files and spans by their number (they only grow, in the order
    /// a run meets them, so a restore cuts them back), the rest copied.
    pub fn snapshot(&mut self) -> DlSnap {
        DlSnap {
            on: self.on,
            side: self.side.share(),
            files: self.files.len(),
            spans: self.spans.len(),
            last: self.last,
            file_level: self.file_level,
            not_a_node: self.not_a_node,
            col_cache: self.col_cache,
            hyph: self.hyph,
            markers: self.markers.clone(),
            last_marked: self.last_marked,
        }
    }

    /// Put a [`snapshot`](Self::snapshot) back (the files and spans made
    /// since, by the run it was taken in or a later one, dropped).
    pub fn restore(&mut self, s: &DlSnap) {
        self.on = s.on;
        self.side = s.side.clone();
        self.files.truncate(s.files);
        self.file_ids.retain(|_, id| *id as usize <= s.files);
        self.spans.truncate(s.spans);
        self.span_ids.retain(|_, id| *id as usize <= s.spans);
        self.last = s.last;
        self.file_level = s.file_level;
        self.not_a_node = s.not_a_node;
        self.col_cache = s.col_cache;
        self.hyph = s.hyph;
        self.markers = s.markers.clone();
        self.last_marked = s.last_marked;
    }
}

/// [`DlSource`] at a checkpoint.
#[derive(Clone)]
pub struct DlSnap {
    on: Option<bool>,
    side: Side,
    files: usize,
    spans: usize,
    last: (i32, u32, u32),
    file_level: usize,
    not_a_node: bool,
    col_cache: (i32, i32, i32, usize, usize),
    hyph: Option<Loc>,
    markers: Vec<Marker>,
    last_marked: Loc,
}

/// The side table: `mem`'s size in chunks, each either this engine's own
/// (written in place) or shared with checkpoints (copied at its first
/// write). Every chunk starts as one shared chunk of zeros, so an engine
/// with no display list holds none of it.
#[derive(Clone, Default)]
struct Side {
    chunks: Vec<Chunk>,
    len: usize,
}

#[derive(Clone)]
enum Chunk {
    Own(Box<[Loc]>),
    Shared(std::sync::Arc<[Loc]>),
}

const SIDE_SHIFT: usize = 13;
const SIDE_CHUNK: usize = 1 << SIDE_SHIFT;

impl Side {
    fn zeros(len: usize) -> Side {
        let zero: std::sync::Arc<[Loc]> = vec![0; SIDE_CHUNK].into();
        Side {
            chunks: (0..len.div_ceil(SIDE_CHUNK))
                .map(|_| Chunk::Shared(zero.clone()))
                .collect(),
            len,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    fn get(&self, i: usize) -> Loc {
        if i >= self.len {
            return 0;
        }
        match &self.chunks[i >> SIDE_SHIFT] {
            Chunk::Own(c) => c[i & (SIDE_CHUNK - 1)],
            Chunk::Shared(c) => c[i & (SIDE_CHUNK - 1)],
        }
    }

    #[inline(always)]
    fn set(&mut self, i: usize, v: Loc) {
        if i >= self.len {
            return;
        }
        let c = &mut self.chunks[i >> SIDE_SHIFT];
        if let Chunk::Shared(sh) = c {
            *c = Chunk::Own(sh.to_vec().into_boxed_slice());
        }
        if let Chunk::Own(o) = c {
            o[i & (SIDE_CHUNK - 1)] = v;
        }
    }

    /// The table, every chunk now shared (this engine's chunks moved, not
    /// copied).
    fn share(&mut self) -> Side {
        for c in self.chunks.iter_mut() {
            if let Chunk::Own(o) = c {
                let b = std::mem::take(o);
                *c = Chunk::Shared(b.into());
            }
        }
        self.clone()
    }
}

/// UTF-8 length of a Unicode scalar as XeTeX's `buffer` holds it.
#[inline(always)]
fn utf8_len(c: i32) -> usize {
    match c {
        ..=0x7F => 1,
        0x80..=0x7FF => 2,
        0x800..=0xFFFF => 3,
        _ => 4,
    }
}

impl Globals {
    #[inline(always)]
    fn dl_on(&mut self) -> bool {
        match self.host.dl.on {
            Some(on) => on,
            None => {
                // `FLASHTEX_SOURCE_MAP=0` turns it off (measurements).
                let on = std::env::var_os(flashtex_display_list::endpoint::ENV)
                    .is_some_and(|v| !v.is_empty())
                    && std::env::var_os("FLASHTEX_SOURCE_MAP").is_none_or(|v| v != "0");
                if on {
                    self.host.dl.side = Side::zeros(self.mem.len());
                }
                self.host.dl.on = Some(on);
                on
            }
        }
    }

    /// `dl_new_node(p)`: node (or token) `p` was just allocated.
    #[inline(always)]
    pub fn dl_new_node(&mut self, p: i32) {
        // Tokens stored while a definition or a macro's arguments are
        // scanned never become nodes: skipped first.
        if self.scanner_status == 0 && !self.host.dl.not_a_node && self.dl_on() {
            let v = if let Some(h) = self.host.dl.hyph {
                h
            } else if self.output_active {
                // Material the output routine makes has no place in the source.
                0
            } else {
                self.dl_here()
            };
            self.host.dl.set(p, v);
        }
    }

    /// The position TeX reads at: the span of the innermost open file's
    /// line and the byte column of the last character read from it.
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
        let rec = if is_file(&self.cur_input) {
            Some(self.cur_input)
        } else {
            self.dl_file_level()
        };
        let col = match rec {
            Some(r) if r.loc_field > r.start_field => {
                let (a, b) = (
                    r.start_field as usize,
                    ((r.loc_field - 1) as usize).min(self.buffer.len()),
                );
                // the bytes of buffer[a..b], from the last ones on the same line
                let c = self.host.dl.col_cache;
                let (mut k, mut n) =
                    if c.0 == name && c.1 == self.line && c.2 == r.start_field && c.3 <= b {
                        (c.3, c.4)
                    } else {
                        (a, 0)
                    };
                while k < b {
                    n += utf8_len(self.buffer[k]);
                    k += 1;
                }
                self.host.dl.col_cache = (name, self.line, r.start_field, b, n);
                n.min(NO_COLUMN as usize - 1) as u16
            }
            Some(_) => 0,
            None => NO_COLUMN,
        };
        let d = &mut self.host.dl;
        let span = if d.last.0 == name && d.last.1 == line {
            d.last.2
        } else {
            let bytes = crate::system::tex_string_utf8(self, name);
            let d = &mut self.host.dl;
            let f = d.file_id(&bytes);
            let s = d.span_id(f, line);
            d.last = (name, line, s);
            s
        };
        pack(span, col)
    }

    /// The innermost level of `input_stack` below `cur_input` reading a
    /// file line (the pdfTeX engine's `file_level`, with its hint).
    fn dl_file_level(&mut self) -> Option<in_state_record> {
        let top = self.input_ptr.max(0) as usize;
        let k = self.host.dl.file_level;
        if k < top {
            let r = self.input_stack[k];
            if r.state_field != token_list && r.index_field == self.in_open && is_file(&r) {
                return Some(r);
            }
        }
        let k = (0..top).rev().find(|&k| is_file(&self.input_stack[k]))?;
        self.host.dl.file_level = k;
        Some(self.input_stack[k])
    }

    /// `dl_copy(r, p)`: node `r` is a copy of node `p`, or a native word
    /// made from it.
    #[inline(always)]
    pub fn dl_copy(&mut self, r: i32, p: i32) {
        if self.host.dl.on == Some(true) {
            let v = self.host.dl.get(p);
            self.host.dl.set(r, v);
        }
    }

    /// `dl_hyph_begin(ha)`: the word at node `ha` is about to be rebuilt.
    pub fn dl_hyph_begin(&mut self, ha: i32) {
        if self.host.dl.on == Some(true) {
            self.host.dl.hyph = Some(self.host.dl.get(ha));
        }
    }

    pub fn dl_hyph_end(&mut self) {
        self.host.dl.hyph = None;
    }

    #[inline(always)]
    pub fn dl_token_begin(&mut self) {
        self.host.dl.not_a_node = true;
    }

    #[inline(always)]
    pub fn dl_token_end(&mut self) {
        self.host.dl.not_a_node = false;
    }

    /// `dl_node(p)`: `hlist_out` or `vlist_out` is about to output node `p`.
    #[inline(always)]
    pub fn dl_node(&mut self, p: i32) {
        if self.host.dl.on != Some(true) {
            return;
        }
        let loc = self.host.dl.get(p) & !START_FOUND;
        if loc == 0 || loc == self.host.dl.last_marked {
            return;
        }
        let pos = (self.dvi_offset as i64 + self.dvi_ptr as i64).max(0) as u64;
        let d = &mut self.host.dl;
        d.last_marked = loc;
        d.markers.push(Marker {
            pos,
            span: loc_span(loc),
            col: loc_col(loc),
        });
    }

    /// A native word node `p` is measured: if it was made on the line TeX
    /// reads now and its column is still where the word ended, the column
    /// becomes that of its first character, found by matching its text
    /// backwards in the line from there. Once only.
    pub fn dl_native_word(&mut self, p: i32, text: &[u16]) {
        if self.host.dl.on != Some(true) || text.is_empty() {
            return;
        }
        let loc = self.host.dl.get(p);
        if loc == 0 || loc & START_FOUND != 0 || loc_col(loc) == NO_COLUMN {
            return;
        }
        let here = self.dl_here();
        if loc_span(here) != loc_span(loc) {
            return;
        }
        let Some(r) = (if is_file(&self.cur_input) {
            Some(self.cur_input)
        } else {
            self.dl_file_level()
        }) else {
            return;
        };
        // the word's scalars
        let word: Vec<i32> = char::decode_utf16(text.iter().copied())
            .map(|c| c.map_or(0xFFFD, |c| c as i32))
            .collect();
        let (a, end) = (
            r.start_field as usize,
            (r.loc_field.max(r.start_field)) as usize,
        );
        let end = end.min(self.buffer.len());
        // the last occurrence ending at or before `end`
        let n = word.len();
        let mut found = None;
        let mut k = end;
        while k >= a + n {
            if self.buffer[k - n..k] == word[..] {
                found = Some(k - n);
                break;
            }
            k -= 1;
        }
        let v = match found {
            Some(s) => {
                let col: usize = (a..s).map(|k| utf8_len(self.buffer[k])).sum();
                pack(loc_span(loc), col.min(NO_COLUMN as usize - 1) as u16) | START_FOUND
            }
            None => loc | START_FOUND,
        };
        self.host.dl.set(p, v);
    }

    /// The `SOURCES` the reader needs for `spans` that it was not sent yet
    /// (`sent` records them), as the frame's JSON body.
    pub fn dl_sources(
        &self,
        spans: &[u32],
        sent: &mut std::collections::HashSet<u32>,
        files_sent: &mut std::collections::HashSet<u32>,
    ) -> Option<Vec<u8>> {
        let d = &self.host.dl;
        let mut src = flashtex_display_list::resource::Sources::default();
        for &s in spans {
            if s == 0 || !sent.insert(s) {
                continue;
            }
            let Some((file, line)) = d.span(s) else {
                continue;
            };
            if files_sent.insert(file) {
                if let Some(p) = d.file(file) {
                    src.files.push((file, p.to_string()));
                }
            }
            src.spans.push((s, file, line));
        }
        (!src.files.is_empty() || !src.spans.is_empty())
            .then(|| src.to_json().to_string().into_bytes())
    }
}

/// An input level reading a file line (not a token list, not the terminal
/// or a pseudo file).
fn is_file(r: &in_state_record) -> bool {
    r.state_field != token_list && r.name_field > 17
}
