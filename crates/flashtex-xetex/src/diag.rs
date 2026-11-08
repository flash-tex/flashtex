//! The diagnostics side channel for XeTeX (the pdfTeX engine's
//! `crates/flashtex-engine/src/diag.rs`): what the engine knows about each
//! error and warning at the moment it reports it, kept as
//! [`flashtex_engine::diag::Note`]s, which the host turns into `diag-v1`
//! messages with the pdfTeX engine host's builder
//! (`flashtex_engine::host::diag`, docs/protocol/display-list-v3.md §6.7).
//!
//! `changes/diagnostics.ch` calls the routines here (`dg_*`) where TeX
//! reports something: `print_err` (the message starts), `error` (the
//! message is printed and the input stack, help lines and `use_err_help`
//! are as the error left them), the overfull/underfull box reports of
//! `hpack` and `vpackage`, `\def` (definition sites for the trace) and
//! `write_out` (LaTeX's and packages' warnings are `\immediate\write`s to
//! the terminal). They do nothing unless the run's terminal is captured (a
//! resident engine's run, [`DiagState::on`]); they only read TeX's
//! variables and never print, so the terminal and the log are exactly what
//! they are without them (DESIGN.md §1.1, P-T1).
//!
//! XeTeX differences from the pdfTeX engine's:
//!
//! * `buffer` holds Unicode scalars and the pool UTF-16: a note's text is
//!   UTF-8, and every column is a UTF-8 byte column of the line, as the
//!   editor counts them (and as the pdfTeX engine's are, its buffer holding
//!   the file's bytes);
//! * tokens are `cmd * max_char_val + chr` (`max_char_val` = `"200000`) and
//!   `null` is `min_halfword`;
//! * the token-list types after `backed_up` are one higher
//!   (`backed_up_char`), and `<XeTeXinterchartoks>` is one more;
//! * TeX breaks terminal lines after `max_print_line` *characters* (a UTF-8
//!   character's bytes count once: `print_raw_char`'s `incr_offset`);
//! * a box's extent is read from the display list's side table
//!   (`crate::dlsource`) at its characters and native words;
//! * the state is the engine's own (a field of `Host`), not the process's:
//!   a checkpoint's record keeps how many notes there were (S₀ keeps the
//!   notes and the definition sites themselves, `crate::ckpt::Record`).

use crate::generated::consts as web;
use crate::generated::Globals;
use flashtex_engine::diag::{Frame, Kind, Note, Pos, Site, FLAG_NOT_ON_TERMINAL, FLAG_OUTPUT_ACTIVE};
use flashtex_engine::persist::Codec;
use std::collections::HashMap;
use std::sync::Arc;

// eqtb and hash locations this module reads, in this build's layout,
// taken from the translation (web2rust emits WEB's macros as constants).
// `tests::locations_match_the_translation` checks that the routines this
// module mirrors still read them.
const ACTIVE_BASE: i32 = web::active_base;
const SINGLE_BASE: i32 = web::single_base;
const NULL_CS: i32 = web::null_cs;
const HASH_BASE: i32 = web::hash_base;
const PRIM_EQTB_BASE: i32 = web::prim_eqtb_base;
const PRIM_EQTB_END: i32 = web::frozen_null_font;
const UNDEFINED_CONTROL_SEQUENCE: i32 = web::undefined_control_sequence;
const ERR_HELP_LOC: i32 = web::err_help_loc;
const CAT_CODE_BASE: i32 = web::cat_code_base;
const ESCAPE_CHAR_LOC: i32 = web::int_base + web::escape_char_code;
const END_LINE_CHAR_LOC: i32 = web::int_base + web::end_line_char_code;
const NEW_LINE_CHAR_LOC: i32 = web::int_base + web::new_line_char_code;
const EQTB_SIZE: i32 = web::eqtb_size;
const EQTB_TOP: i32 = web::eqtb_top;
const CS_TOKEN_FLAG: i32 = web::cs_token_flag;
const MAX_CHAR_VAL: i32 = web::max_char_val;
const TOKEN_LIST: i32 = web::token_list;
/// xetex.web's `null` (`min_halfword`, tex.ch's -"FFFFFFF).
const NULL: i32 = -0x0FFF_FFFF;
/// `list_ptr(r)` is `link(r+list_offset)`.
const LIST_OFFSET: i32 = 5;
/// Node types and the whatsit subtypes that hold text (xetex.web §135ff.).
const HLIST_NODE: i32 = 0;
const VLIST_NODE: i32 = 1;
const WHATSIT_NODE: i32 = 8;
const NATIVE_WORD_NODE: i32 = 40;
const NATIVE_WORD_NODE_AT: i32 = 41;
const GLYPH_NODE: i32 = 42;
/// Selector codes (§54).
const TERM_ONLY: i32 = web::term_only;
const TERM_AND_LOG: i32 = web::term_and_log;
/// The pool strings start here; the strings below are single characters.
const FIRST_STRING: i32 = 65536;
/// Text kept of one side of a context line.
const TEXT_CAP: usize = 240;
/// Levels kept of the input stack: the innermost `MAX_FRAMES - 1` and the
/// bottom one (a note is sent with every compile of the document).
const MAX_FRAMES: usize = 24;
/// Notes after which a note keeps only the file level of its trace (a
/// document with thousands of warnings sends every one each compile).
const FULL_TRACES: usize = 1000;

/// A definition site in [`DiagState::defs`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Def {
    /// The file (in [`DiagState::files`]).
    pub file: u32,
    pub line: i32,
    pub col: i32,
    pub print: u64,
}
flashtex_engine::codec_struct!(Def {
    file,
    line,
    col,
    print
});

/// The definition sites as a record keeps them (S₀): each file name once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sites {
    pub files: Vec<Vec<u8>>,
    /// By token list, in order.
    pub defs: Vec<(i32, Def)>,
}
flashtex_engine::codec_struct!(Sites { files, defs });

/// Notes as a checkpoint's record keeps them (S₀: every note so far; a
/// page checkpoint: none, its count only), shared with the live run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeptNotes(pub Vec<Arc<Note>>);

impl flashtex_engine::persist::Codec for KeptNotes {
    fn enc(&self, w: &mut Vec<u8>) {
        self.0.len().enc(w);
        for n in &self.0 {
            (**n).enc(w);
        }
    }
    fn dec(r: &mut flashtex_engine::persist::Reader) -> Result<Self, String> {
        Ok(KeptNotes(
            Vec::<Note>::dec(r)?.into_iter().map(Arc::new).collect(),
        ))
    }
}

/// The side channel's state (a field of the engine's `Host`).
#[derive(Default)]
pub struct DiagState {
    /// The run's terminal is captured and its notes are wanted (a
    /// resident engine's run).
    pub on: bool,
    notes: Vec<Arc<Note>>,
    /// The terminal length when `print_err` started the message.
    mark: Option<usize>,
    /// `dg_write_begin`: the terminal length, and whether the write goes
    /// to the terminal.
    write: Option<(usize, bool)>,
    /// `dg_box_begin`.
    box_at: Option<usize>,
    /// `dg_def_begin`: the file (in `files`), line and column.
    def_at: Option<(u32, i32, i32)>,
    /// Token list (a macro's `def_ref`) -> where it was defined.
    defs: HashMap<i32, Def>,
    /// The names of the files definitions were made in, each once.
    files: Vec<Vec<u8>>,
    file_ids: HashMap<Vec<u8>, u32>,
    /// The file name last looked up: (string number, its pool text, its
    /// number in `files`).
    name_cache: Option<(i32, Vec<i32>, u32)>,
    /// Where the last walk found the innermost file level below
    /// `cur_input` (a hint).
    file_level: usize,
}

impl DiagState {
    fn file_id(&mut self, name: &[u8]) -> u32 {
        if let Some(&id) = self.file_ids.get(name) {
            return id;
        }
        let id = self.files.len() as u32;
        self.files.push(name.to_vec());
        self.file_ids.insert(name.to_vec(), id);
        id
    }

    /// How many notes there are.
    pub fn len(&self) -> usize {
        self.notes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    /// The notes from `n` on (shared).
    pub fn notes_from(&self, n: usize) -> Vec<Arc<Note>> {
        self.notes.get(n..).map(|v| v.to_vec()).unwrap_or_default()
    }

    /// The notes as they are now, from S₀'s record or a restore's base.
    pub fn set_notes(&mut self, v: Vec<Arc<Note>>) {
        self.notes = v;
        self.clear_pending();
    }

    /// No report in progress (a restore happens between commands).
    fn clear_pending(&mut self) {
        self.mark = None;
        self.write = None;
        self.box_at = None;
        self.def_at = None;
    }

    /// A run from the state after the format is loaded: no notes, and no
    /// definition site (every definition after that point is made again).
    pub fn clear(&mut self) {
        let on = self.on;
        *self = DiagState::default();
        self.on = on;
    }

    fn push(&mut self, mut n: Note) {
        if self.notes.len() >= FULL_TRACES && n.frames.len() > 1 {
            let bottom = n.frames.pop();
            n.frames.clear();
            n.frames.extend(bottom);
        }
        self.notes.push(Arc::new(n));
    }

    /// The definition sites, for S₀ (by token list, in order).
    pub fn sites(&self) -> Sites {
        let mut defs: Vec<(i32, Def)> = self.defs.iter().map(|(k, d)| (*k, *d)).collect();
        defs.sort_unstable_by_key(|x| x.0);
        Sites {
            files: self.files.clone(),
            defs,
        }
    }

    /// Put definition sites back (S₀ opened from its file).
    pub fn set_sites(&mut self, s: &Sites) {
        self.defs.clear();
        for (k, d) in &s.defs {
            let Some(name) = s.files.get(d.file as usize) else {
                continue;
            };
            let file = self.file_id(name);
            self.defs.insert(*k, Def { file, ..*d });
        }
    }

    /// Whether no definition site is known (a new engine).
    pub fn no_sites(&self) -> bool {
        self.defs.is_empty()
    }

    fn site(&self, d: &Def) -> Site {
        Site {
            file: self.files[d.file as usize].clone(),
            line: d.line,
            col: d.col,
            print: d.print,
        }
    }
}

/// Notes `v` with their terminal offsets moved by `shift` (a convergence
/// splices the old run's later notes in after the new run's terminal).
pub fn shifted(v: &[Arc<Note>], shift: i64) -> Vec<Arc<Note>> {
    v.iter()
        .map(|n| {
            if shift == 0 {
                n.clone()
            } else {
                let mut m = (**n).clone();
                m.at = (m.at as i64 + shift).max(0) as usize;
                m.end = (m.end as i64 + shift).max(0) as usize;
                Arc::new(m)
            }
        })
        .collect()
}

/// The places (file path, line) of the source map's spans that `notes`
/// name (a box's first and last character), for the host, which builds the
/// `DIAG`s after the run.
pub fn note_spans(g: &Globals, notes: &[Arc<Note>]) -> HashMap<u32, (String, u32)> {
    let mut out = HashMap::new();
    if !g.host.dl.is_on() {
        return out;
    }
    for n in notes {
        for (s, _) in [n.first, n.last] {
            if s != 0 && !out.contains_key(&s) {
                if let Some((f, line)) = g.host.dl.span(s) {
                    if let Some(path) = g.host.dl.file(f) {
                        out.insert(s, (path.to_string(), line));
                    }
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Reading TeX's state (never writing it)

fn push_utf8(out: &mut Vec<u8>, c: i32) {
    let ch = char::from_u32(c as u32).unwrap_or('\u{FFFD}');
    let mut tmp = [0u8; 4];
    out.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
}

fn utf8_len(c: i32) -> i32 {
    match c {
        0..=0x7F => 1,
        0x80..=0x7FF => 2,
        0x800..=0xFFFF => 3,
        _ => 4,
    }
}

/// String `s` as UTF-8 (a single character below 65536).
fn str_bytes(g: &Globals, s: i32) -> Vec<u8> {
    let mut v = Vec::new();
    if (0..FIRST_STRING).contains(&s) {
        push_utf8(&mut v, s);
        return v;
    }
    if s < FIRST_STRING || s >= g.str_ptr {
        return v;
    }
    let k = (s - FIRST_STRING) as usize;
    let (a, b) = (g.str_start[k], g.str_start[k + 1]);
    if a < 0 || b < a {
        return v;
    }
    let mut i = a;
    while i < b {
        let mut c = g.str_pool[i as usize];
        if (0xD800..=0xDBFF).contains(&c) && i + 1 < b {
            let lo = g.str_pool[(i + 1) as usize];
            if (0xDC00..=0xDFFF).contains(&lo) {
                c = 0x10000 + (c - 0xD800) * 0x400 + lo - 0xDC00;
                i += 1;
            }
        }
        push_utf8(&mut v, c);
        i += 1;
    }
    v
}

fn eqtb_int(g: &Globals, loc: i32) -> i32 {
    g.eqtb[(loc - 1) as usize].int()
}

fn cat_code(g: &Globals, c: i32) -> i32 {
    if !(0..0x110000).contains(&c) {
        return 12;
    }
    g.eqtb[(CAT_CODE_BASE + c - 1) as usize].hh_rh()
}

fn info(g: &Globals, p: i32) -> i32 {
    g.mem[p as usize].hh_lh()
}

fn link(g: &Globals, p: i32) -> i32 {
    g.mem[p as usize].hh_rh()
}

/// A token-list pointer TeX could follow.
fn in_token_mem(g: &Globals, p: i32) -> bool {
    p >= g.hi_mem_min && p <= g.mem_end
}

/// A pointer into `mem`.
fn in_mem(g: &Globals, p: i32) -> bool {
    p >= 0 && p <= g.mem_end
}

/// `print_esc(s)`'s bytes for a name.
fn push_esc(g: &Globals, out: &mut Vec<u8>, name: &[u8]) {
    let c = eqtb_int(g, ESCAPE_CHAR_LOC);
    if (0..0x110000).contains(&c) {
        push_utf8(out, c);
    }
    out.extend_from_slice(name);
}

/// A control sequence `print_cs` prints by name (tex.ch §262 with
/// `hash_extra`).
fn valid_cs(p: i32) -> bool {
    p >= ACTIVE_BASE && !((UNDEFINED_CONTROL_SEQUENCE..=EQTB_SIZE).contains(&p) || p > EQTB_TOP)
}

/// A macro's name for a trace: `print_cs(p)` with a backslash whatever
/// `\escapechar` is (LaTeX's error macros set it to -1) and no space
/// after.
fn cs_name(g: &Globals, p: i32) -> Vec<u8> {
    let mut v = vec![b'\\'];
    if p == NULL_CS {
        v.extend_from_slice(b"csname\\endcsname");
    } else if (SINGLE_BASE..HASH_BASE).contains(&p) {
        push_utf8(&mut v, p - SINGLE_BASE);
    } else if (ACTIVE_BASE..SINGLE_BASE).contains(&p) {
        // an active character is its own name
        let mut a = Vec::new();
        push_utf8(&mut a, p - ACTIVE_BASE);
        return a;
    } else if !valid_cs(p) {
        v.extend_from_slice(b"IMPOSSIBLE.");
    } else {
        let t = g.hash[(p - HASH_BASE) as usize].rh();
        if t < 0 || t >= g.str_ptr {
            v.extend_from_slice(b"NONEXISTENT.");
        } else if (PRIM_EQTB_BASE..PRIM_EQTB_END).contains(&p) {
            v.extend(str_bytes(g, g.prim[(p - PRIM_EQTB_BASE) as usize].rh() - 1));
        } else {
            v.extend(str_bytes(g, t));
        }
    }
    v
}

/// `print_cs(p)`, as bytes.
fn push_cs(g: &Globals, out: &mut Vec<u8>, p: i32) {
    if p < HASH_BASE {
        if p >= SINGLE_BASE {
            if p == NULL_CS {
                push_esc(g, out, b"csname");
                push_esc(g, out, b"endcsname");
                out.push(b' ');
            } else {
                let c = p - SINGLE_BASE;
                let mut name = Vec::new();
                push_utf8(&mut name, c);
                push_esc(g, out, &name);
                if cat_code(g, c) == 11 {
                    out.push(b' ');
                }
            }
        } else if p < ACTIVE_BASE {
            push_esc(g, out, b"IMPOSSIBLE.");
        } else {
            push_utf8(out, p - ACTIVE_BASE);
        }
    } else if !valid_cs(p) {
        push_esc(g, out, b"IMPOSSIBLE.");
    } else {
        let t = g.hash[(p - HASH_BASE) as usize].rh();
        if t < 0 || t >= g.str_ptr {
            push_esc(g, out, b"NONEXISTENT.");
        } else {
            let s = if (PRIM_EQTB_BASE..PRIM_EQTB_END).contains(&p) {
                g.prim[(p - PRIM_EQTB_BASE) as usize].rh() - 1
            } else {
                t
            };
            push_esc(g, out, &str_bytes(g, s));
            out.push(b' ');
        }
    }
}

/// `show_token_list(p, q, ∞)` as bytes, split where `q` starts (TeX's
/// context display splits there): (before, after). Capped at
/// [`TEXT_CAP`] bytes a side and 100,000 tokens.
fn token_text(g: &Globals, mut p: i32, q: i32) -> (Vec<u8>, Vec<u8>) {
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut in_after = false;
    let mut match_chr = b'#' as i32;
    let mut n = b'0';
    let mut count = 0;
    while p != NULL && count < 100_000 {
        count += 1;
        if p == q {
            in_after = true;
        }
        let out = if in_after { &mut after } else { &mut before };
        if out.len() > TEXT_CAP * 4 {
            if in_after {
                break;
            }
            // keep the end of `before`: drop its start
            let cut = out.len() - TEXT_CAP * 2;
            out.drain(..cut);
        }
        if !in_token_mem(g, p) {
            push_esc(g, out, b"CLOBBERED.");
            break;
        }
        let t = info(g, p);
        if t >= CS_TOKEN_FLAG {
            push_cs(g, out, t - CS_TOKEN_FLAG);
        } else if t < 0 {
            push_esc(g, out, b"BAD.");
        } else {
            let (m, c) = (t / MAX_CHAR_VAL, t % MAX_CHAR_VAL);
            match m {
                1 | 2 | 3 | 4 | 7 | 8 | 10 | 11 | 12 => push_utf8(out, c),
                6 => {
                    push_utf8(out, c);
                    push_utf8(out, c);
                }
                5 => {
                    push_utf8(out, match_chr);
                    if c <= 9 {
                        out.push(c as u8 + b'0');
                    } else {
                        out.push(b'!');
                        break;
                    }
                }
                13 => {
                    match_chr = c;
                    push_utf8(out, c);
                    n += 1;
                    out.push(n);
                    if n > b'9' {
                        break;
                    }
                }
                14 => {
                    if c == 0 {
                        out.extend_from_slice(b"->");
                    }
                }
                _ => push_esc(g, out, b"BAD."),
            }
        }
        p = link(g, p);
    }
    cap_before(&mut before);
    cap_after(&mut after);
    (before, after)
}

/// The last [`TEXT_CAP`] bytes of `b`, from a character's start.
fn cap_before(b: &mut Vec<u8>) {
    if b.len() > TEXT_CAP {
        let mut cut = b.len() - TEXT_CAP;
        while cut < b.len() && (b[cut] & 0xC0) == 0x80 {
            cut += 1;
        }
        b.drain(..cut);
    }
}

/// The first [`TEXT_CAP`] bytes of `b`, ending at a character's end.
fn cap_after(b: &mut Vec<u8>) {
    if b.len() > TEXT_CAP {
        let mut end = TEXT_CAP;
        while end > 0 && (b[end] & 0xC0) == 0x80 {
            end -= 1;
        }
        b.truncate(end);
    }
}

/// A fingerprint of a token list's first 64 tokens and whether it has
/// more (definition sites are checked against it when read).
fn fingerprint(g: &Globals, p: i32) -> u64 {
    // (after the reference count, which every use of the macro changes)
    if !in_token_mem(g, p) {
        return 0;
    }
    let mut p = link(g, p);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut k = 0;
    while p != NULL && k < 64 {
        if !in_token_mem(g, p) {
            return 0;
        }
        h = (h ^ info(g, p) as u32 as u64).wrapping_mul(0x0100_0000_01b3);
        p = link(g, p);
        k += 1;
    }
    h ^ (p != NULL) as u64
}

/// The UTF-8 bytes of `buffer[a..b]`.
fn buffer_bytes(g: &Globals, a: i32, b: i32) -> i32 {
    (a..b).map(|i| utf8_len(g.buffer[i as usize])).sum()
}

/// `buffer[a..b]` as UTF-8.
fn buffer_text(g: &Globals, a: i32, b: i32) -> Vec<u8> {
    let mut v = Vec::new();
    for i in a..b {
        push_utf8(&mut v, g.buffer[i as usize]);
    }
    v
}

/// A level of the input stack that reads a file line.
fn is_file(r: &crate::generated::types::in_state_record) -> bool {
    r.state_field != TOKEN_LIST && r.name_field > 19
}

impl Globals {
    /// The side channel records (`DiagState::on`).
    #[inline(always)]
    fn dg_on(&self) -> bool {
        self.host.diag.on
    }

    fn term_len(&self) -> usize {
        self.term_out.captured_bytes().len()
    }

    fn term_slice(&self, a: usize, b: usize) -> Vec<u8> {
        let t = self.term_out.captured_bytes();
        let b = b.min(t.len());
        t.get(a.min(b)..b).map(|s| s.to_vec()).unwrap_or_default()
    }

    /// The number (in `DiagState::files`) of file-name string `s`'s bytes
    /// (the last one is cached: a definition asks for the same file's name
    /// again and again).
    fn dg_name_id(&mut self, s: i32) -> u32 {
        if s < FIRST_STRING || s >= self.str_ptr {
            return self.host.diag.file_id(b"");
        }
        let k = (s - FIRST_STRING) as usize;
        let (a, e) = (self.str_start[k] as usize, self.str_start[k + 1] as usize);
        if let Some((cs, pool, id)) = &self.host.diag.name_cache {
            // the string may have been freed and remade since
            if *cs == s && pool.len() == e - a && (a..e).zip(pool).all(|(i, &c)| self.str_pool[i] == c)
            {
                return *id;
            }
        }
        let b = str_bytes(self, s);
        let pool: Vec<i32> = (a..e).map(|i| self.str_pool[i]).collect();
        let d = &mut self.host.diag;
        let id = d.file_id(&b);
        d.name_cache = Some((s, pool, id));
        id
    }

    fn dg_name(&mut self, s: i32) -> Vec<u8> {
        let id = self.dg_name_id(s);
        self.host.diag.files[id as usize].clone()
    }

    /// The line a file level (`index` = its `in_open` level) is at.
    fn dg_level_line(&self, index: i32) -> i32 {
        if index == self.in_open {
            self.line
        } else {
            self.line_stack[index as usize]
        }
    }

    /// The innermost level of `input_stack` below `cur_input` reading a
    /// file line (with a hint).
    fn dg_file_level(&mut self) -> Option<crate::generated::types::in_state_record> {
        let top = self.input_ptr.max(0) as usize;
        let k = self.host.diag.file_level;
        if k < top {
            let r = self.input_stack[k];
            if r.index_field == self.in_open && is_file(&r) {
                return Some(r);
            }
        }
        let k = (0..top).rev().find(|&k| is_file(&self.input_stack[k]))?;
        self.host.diag.file_level = k;
        Some(self.input_stack[k])
    }

    /// Where TeX is reading: the innermost file level (the level TeX's
    /// context display ends with), cheaply (no text): the file, the line,
    /// and, when TeX is reading the file itself (not a macro called from
    /// it), the byte column of the token it read last; else -1.
    fn dg_here(&mut self) -> Option<(u32, i32, i32)> {
        let (rec, col) = if is_file(&self.cur_input) {
            let r = self.cur_input;
            let (start, limit) = (r.start_field, r.limit_field);
            let split = r.loc_field.clamp(start, (limit + 1).max(start));
            let from = token_start(self, start, split);
            (r, buffer_bytes(self, start, from))
        } else {
            (self.dg_file_level()?, -1)
        };
        let index = rec.index_field;
        let name = self.full_source_filename_stack[index as usize];
        let id = self.dg_name_id(name);
        Some((id, self.dg_level_line(index), col))
    }

    /// The input stack, innermost level first, as `show_context` walks it
    /// (every level, not only `\errorcontextlines` of them; a backed-up
    /// list already read is left out, as there), and the bottom file
    /// level's place.
    fn dg_frames(&mut self) -> (Vec<Frame>, Option<Pos>) {
        let mut frames = Vec::new();
        let mut pos = None;
        let top = self.input_ptr;
        let mut k = top;
        loop {
            let r = if k == top {
                self.cur_input
            } else {
                self.input_stack[k as usize]
            };
            let mut bottom = false;
            if r.state_field != TOKEN_LIST && (r.name_field > 19 || k == 0) {
                bottom = true;
            }
            if r.state_field != TOKEN_LIST {
                let mut f = Frame::default();
                if r.name_field <= 17 {
                    if r.name_field == 0 {
                        f.kind = if k == 0 { "terminal" } else { "insert" }.into();
                    } else {
                        f.kind = "read".into();
                        f.name = if r.name_field == 17 {
                            b"*".to_vec()
                        } else {
                            (r.name_field - 1).to_string().into_bytes()
                        };
                    }
                } else {
                    f.kind = if r.name_field > 19 {
                        "file"
                    } else {
                        "scantokens"
                    }
                    .into();
                    // the line, split where TeX is, is shown: a read of it
                    // (READ-TRACK, `crate::dormant`), as `show_context`'s
                    self.flashtex_level_shown();
                }
                let end_line = eqtb_int(self, END_LINE_CHAR_LOC);
                let limit = r.limit_field;
                let j = if limit >= 0 && self.buffer[limit as usize] == end_line {
                    limit
                } else {
                    limit + 1
                };
                let start = r.start_field;
                let split = r.loc_field.clamp(start, j.max(start));
                f.before = buffer_text(self, start, split);
                cap_before(&mut f.before);
                f.after = buffer_text(self, split, j.max(split));
                cap_after(&mut f.after);
                if r.name_field > 17 {
                    let name = self.full_source_filename_stack[r.index_field as usize];
                    let line = self.dg_level_line(r.index_field);
                    let from = token_start(self, start, split);
                    let p = Pos {
                        file: self.dg_name(name),
                        line,
                        col: buffer_bytes(self, start, split),
                        from: buffer_bytes(self, start, from),
                    };
                    if pos.is_none() && r.name_field > 19 {
                        pos = Some(p.clone());
                    }
                    f.pos = Some(p);
                }
                frames.push(f);
            } else if k == top || r.index_field != web::backed_up || r.loc_field != NULL {
                let t = r.index_field;
                let kind = match t {
                    0 => "argument",
                    1 | 2 => "template",
                    3 | 4 => {
                        if r.loc_field == NULL {
                            "recently_read"
                        } else {
                            "backed_up"
                        }
                    }
                    5 => "inserted",
                    6 => "macro",
                    7 => "output",
                    8 => "everypar",
                    9 => "everymath",
                    10 => "everydisplay",
                    11 => "everyhbox",
                    12 => "everyvbox",
                    13 => "everyjob",
                    14 => "everycr",
                    15 => "mark",
                    16 => "everyeof",
                    17 => "interchartoks",
                    18 => "write",
                    _ => "?",
                };
                let mut f = Frame {
                    kind: kind.into(),
                    ..Frame::default()
                };
                let list = if t < web::macro_ {
                    r.start_field
                } else if in_token_mem(self, r.start_field) {
                    link(self, r.start_field)
                } else {
                    NULL
                };
                if t == web::macro_ {
                    f.name = cs_name(self, r.name_field);
                    let d = &self.host.diag;
                    f.def = d
                        .defs
                        .get(&r.start_field)
                        .map(|x| d.site(x))
                        .filter(|s| s.print == fingerprint(self, r.start_field))
                        .map(|s| Pos {
                            file: s.file,
                            line: s.line,
                            col: s.col,
                            from: -1,
                        });
                }
                if in_token_mem(self, r.start_field) {
                    let (b, a) = token_text(self, list, r.loc_field);
                    f.before = b;
                    f.after = a;
                }
                frames.push(f);
            }
            if bottom || k == 0 {
                break;
            }
            k -= 1;
        }
        if frames.len() > MAX_FRAMES {
            let bottom = frames.pop();
            frames.truncate(MAX_FRAMES - 1);
            frames.extend(bottom);
        }
        (frames, pos)
    }

    /// The help lines of the error being reported, as the log shows them.
    fn dg_help(&self) -> Vec<Vec<u8>> {
        if self.use_err_help {
            let p = self.eqtb[(ERR_HELP_LOC - 1) as usize].hh_rh();
            if p == NULL || !in_token_mem(self, p) {
                return Vec::new();
            }
            let (text, _) = token_text(self, link(self, p), -1);
            let nl = eqtb_int(self, NEW_LINE_CHAR_LOC);
            let mut nl_bytes = Vec::new();
            if (0..0x110000).contains(&nl) && nl != b'\n' as i32 {
                push_utf8(&mut nl_bytes, nl);
            }
            let mut lines = Vec::new();
            let mut cur = Vec::new();
            let mut i = 0;
            while i < text.len() {
                if text[i] == b'\n' {
                    lines.push(std::mem::take(&mut cur));
                    i += 1;
                } else if !nl_bytes.is_empty() && text[i..].starts_with(&nl_bytes) {
                    lines.push(std::mem::take(&mut cur));
                    i += nl_bytes.len();
                } else {
                    cur.push(text[i]);
                    i += 1;
                }
            }
            lines.push(cur);
            return lines;
        }
        (0..self.help_ptr.max(0))
            .rev()
            .map(|k| str_bytes(self, self.help_line[k as usize]))
            .collect()
    }

    /// `print_err` starts a message.
    pub fn dg_mark(&mut self) {
        if self.dg_on() {
            let n = self.term_len();
            self.host.diag.mark = Some(n);
        }
    }

    /// `error`: the message is on the terminal, the input stack and the
    /// help are as the error left them.
    pub fn dg_error(&mut self) {
        if self.dg_on() {
            self.dg_note_error();
        }
    }

    #[inline(never)]
    fn dg_note_error(&mut self) {
        let now = self.term_len();
        let mark = self.host.diag.mark.take();
        let (kind, at) = match mark {
            Some(at) if at <= now => (Kind::Error, at),
            _ => {
                // `\show` and its relatives print "> ..." and stop
                let tail = self.term_slice(now.saturating_sub(4096), now);
                let at = tail
                    .windows(3)
                    .rposition(|w| w == b"\n> ")
                    .map(|i| now - tail.len() + i)
                    .unwrap_or(now);
                (Kind::Show, at)
            }
        };
        let mut text = self.term_slice(at, now);
        strip_error_prefix(&mut text, self.max_print_line);
        let help = self.dg_help();
        let (frames, pos) = self.dg_frames();
        let flags = if self.selector == TERM_ONLY || self.selector == TERM_AND_LOG {
            0
        } else {
            FLAG_NOT_ON_TERMINAL
        };
        let note = Note {
            kind,
            at,
            end: now,
            text,
            help,
            frames,
            pos,
            lines: (0, self.line),
            first: (0, 0),
            last: (0, 0),
            flags: flags
                | if self.output_active {
                    FLAG_OUTPUT_ACTIVE
                } else {
                    0
                },
        };
        self.host.diag.push(note);
    }

    /// A box report's position part starts.
    pub fn dg_box_begin(&mut self, _r: i32) {
        if self.dg_on() {
            let n = self.term_len();
            self.host.diag.box_at = Some(n);
        }
    }

    /// A box report is on the terminal (its two lines).
    pub fn dg_box_end(&mut self, r: i32) {
        if self.dg_on() {
            self.dg_note_box(r);
        }
    }

    #[inline(never)]
    fn dg_note_box(&mut self, r: i32) {
        let now = self.term_len();
        let Some(mid) = self.host.diag.box_at.take() else {
            return;
        };
        // The report's first line starts after the newline before `mid`.
        let back = self.term_slice(mid.saturating_sub(400), mid);
        let at = back
            .iter()
            .rposition(|&c| c == b'\n')
            .map(|i| mid - back.len() + i + 1)
            .unwrap_or(mid.saturating_sub(back.len()));
        let mut text = self.term_slice(at, now);
        unwrap_lines(&mut text, self.max_print_line, 0);
        while text.last() == Some(&b'\n') {
            text.pop();
        }
        let (first, last) = if in_mem(self, r) {
            self.dg_box_extent(link(self, r + LIST_OFFSET))
        } else {
            ((0, 0), (0, 0))
        };
        let (frames, pos) = self.dg_frames();
        let flags = if self.output_active {
            FLAG_OUTPUT_ACTIVE
        } else {
            0
        };
        let note = Note {
            kind: Kind::Box,
            at,
            end: now,
            text,
            help: Vec::new(),
            frames,
            pos,
            lines: (self.pack_begin_line, self.line),
            first,
            last,
            flags,
        };
        self.host.diag.push(note);
    }

    /// The display-list source locations of the first and last characters
    /// (and native words) in the node list `p` (descending into boxes),
    /// bounded.
    fn dg_box_extent(&self, p: i32) -> ((u32, u16), (u32, u16)) {
        let mut first = (0, 0);
        let mut last = (0, 0);
        if !self.host.dl.is_on() {
            return (first, last);
        }
        let mut stack = vec![p];
        let mut seen = 0;
        let mut note = |l: crate::dlsource::Loc| {
            let s = crate::dlsource::loc_span(l);
            if s != 0 {
                let v = (s, crate::dlsource::loc_col(l));
                if first.0 == 0 {
                    first = v;
                }
                last = v;
            }
        };
        while let Some(mut q) = stack.pop() {
            while q != NULL && seen < 200_000 {
                seen += 1;
                if !in_mem(self, q) {
                    break;
                }
                if q >= self.hi_mem_min {
                    note(self.host.dl.at(q));
                } else {
                    let t = self.mem[q as usize].hh_b0();
                    if t == HLIST_NODE || t == VLIST_NODE {
                        // their lists, in order
                        let inner = link(self, q + LIST_OFFSET);
                        if inner != NULL {
                            stack.push(link(self, q));
                            stack.push(inner);
                            break;
                        }
                    } else if t == WHATSIT_NODE {
                        let s = self.mem[q as usize].hh_b1();
                        if s == NATIVE_WORD_NODE || s == NATIVE_WORD_NODE_AT || s == GLYPH_NODE {
                            note(self.host.dl.at(q));
                        }
                    }
                }
                q = link(self, q);
            }
        }
        (first, last)
    }

    /// `\def` has its control sequence: the definition starts here.
    pub fn dg_def_begin(&mut self) {
        if self.dg_on() {
            let here = self.dg_here();
            self.host.diag.def_at = here;
        }
    }

    /// `\def` made token list `p` the macro's meaning.
    pub fn dg_def(&mut self, p: i32) {
        if self.dg_on() {
            let print = fingerprint(self, p);
            let d = &mut self.host.diag;
            if let Some((file, line, col)) = d.def_at.take() {
                d.defs.insert(
                    p,
                    Def {
                        file,
                        line,
                        col,
                        print,
                    },
                );
            }
        }
    }

    /// `write_out` is about to show a `\write`'s text.
    pub fn dg_write_begin(&mut self, j: i32) {
        if self.dg_on() {
            let to_terminal =
                j != 18 && (self.selector == TERM_ONLY || self.selector == TERM_AND_LOG);
            let n = self.term_len();
            self.host.diag.write = Some((n, to_terminal));
        }
    }

    /// `write_out` showed it.
    pub fn dg_write_end(&mut self, _j: i32) {
        if self.dg_on() {
            self.dg_note_write();
        }
    }

    #[inline(never)]
    fn dg_note_write(&mut self) {
        let Some((at, to_terminal)) = self.host.diag.write.take() else {
            return;
        };
        if !to_terminal {
            return;
        }
        let now = self.term_len();
        let mut text = self.term_slice(at, now);
        // A warning, or an error a macro prints itself (LaTeX's missing
        // file: `\typeout{! LaTeX Error: File ... not found.}`, then a
        // `\read` from the terminal).
        if !contains(&text, b"Warning") && !contains(&text, b"\n! ") && !text.starts_with(b"! ") {
            return;
        }
        unwrap_lines(&mut text, self.max_print_line, 0);
        let (frames, pos) = self.dg_frames();
        let note = Note {
            kind: Kind::Write,
            at,
            end: now,
            text,
            help: Vec::new(),
            frames,
            pos,
            lines: (0, self.line),
            first: (0, 0),
            last: (0, 0),
            flags: if self.output_active {
                FLAG_OUTPUT_ACTIVE
            } else {
                0
            },
        };
        self.host.diag.push(note);
    }
}

fn contains(h: &[u8], n: &[u8]) -> bool {
    h.windows(n.len()).any(|w| w == n)
}

/// Where the token before `split` starts in `buffer[start..split]` (an
/// index): a control word or symbol with its escape character, else one
/// character; spaces before `split` are skipped first. Catcodes are the
/// current ones.
fn token_start(g: &Globals, start: i32, split: i32) -> i32 {
    let b = |i: i32| g.buffer[i as usize];
    let mut e = split;
    while e > start && cat_code(g, b(e - 1)) == 10 {
        e -= 1;
    }
    if e <= start {
        return split.min(start.max(e));
    }
    if let Some(s) = call_start(g, start, e) {
        return s;
    }
    let c = b(e - 1);
    if cat_code(g, c) == 11 {
        let mut k = e - 1;
        while k > start && cat_code(g, b(k - 1)) == 11 {
            k -= 1;
        }
        if k > start && cat_code(g, b(k - 1)) == 0 {
            return k - 1;
        }
        // a word: its last letter is the token
        return e - 1;
    }
    if e - 2 >= start && cat_code(g, b(e - 2)) == 0 {
        return e - 2;
    }
    e - 1
}

/// When the text before `e` ends with a group (`}`) or an optional
/// argument (`]`): where the command those arguments belong to starts
/// (`\ref{x}`, `\section*[a]{b}`), or the first group's `{` when no
/// control sequence precedes them. `None` otherwise.
fn call_start(g: &Globals, start: i32, e: i32) -> Option<i32> {
    let b = |i: i32| g.buffer[i as usize];
    let cat = |i: i32| cat_code(g, b(i));
    let skip_spaces = |mut k: i32| {
        while k > start && cat(k - 1) == 10 {
            k -= 1;
        }
        k
    };
    let mut k = e;
    let mut groups = false;
    loop {
        let k2 = skip_spaces(k);
        if k2 <= start {
            break;
        }
        let (close_is, open): (bool, i32) = if cat(k2 - 1) == 2 {
            (true, 0)
        } else if b(k2 - 1) == b']' as i32 && cat(k2 - 1) == 12 {
            (true, b'[' as i32)
        } else {
            (false, 0)
        };
        if !close_is {
            break;
        }
        // back to the matching opener
        let mut depth = 0;
        let mut i = k2 - 1;
        let mut found = None;
        while i >= start {
            let c = cat(i);
            let escaped = i > start && cat(i - 1) == 0;
            if !escaped {
                if open == 0 {
                    if c == 2 {
                        depth += 1;
                    } else if c == 1 {
                        depth -= 1;
                        if depth == 0 {
                            found = Some(i);
                            break;
                        }
                    }
                } else if c == 2 {
                    depth += 1;
                } else if c == 1 {
                    depth -= 1;
                } else if b(i) == open && depth == 0 {
                    found = Some(i);
                    break;
                }
            }
            i -= 1;
        }
        let Some(o) = found else {
            break;
        };
        groups = true;
        k = o;
    }
    if !groups {
        return None;
    }
    // the command before the arguments (a starred form's `*` with it)
    let mut k2 = skip_spaces(k);
    if k2 > start && b(k2 - 1) == b'*' as i32 {
        k2 -= 1;
    }
    if k2 > start && cat(k2 - 1) == 11 {
        let mut j = k2 - 1;
        while j > start && cat(j - 1) == 11 {
            j -= 1;
        }
        if j > start && cat(j - 1) == 0 {
            return Some(j - 1);
        }
    } else if k2 - 2 >= start && cat(k2 - 2) == 0 {
        return Some(k2 - 2);
    }
    Some(k)
}

/// Take TeX's line breaks at `max_print_line` out of `text` (which starts
/// at terminal column `col0`): after exactly `max_print_line` characters
/// on a line, the newline is TeX's, not the text's. A UTF-8 character
/// counts once (`print_raw_char` counts its last byte only).
pub fn unwrap_lines(text: &mut Vec<u8>, max_print_line: i32, col0: usize) {
    if max_print_line <= 0 {
        return;
    }
    let m = max_print_line as usize;
    let mut out = Vec::with_capacity(text.len());
    let mut col = col0;
    for &c in text.iter() {
        if c == b'\n' {
            if col == m {
                col = 0;
                continue;
            }
            out.push(c);
            col = 0;
        } else {
            out.push(c);
            if (c & 0xC0) != 0x80 {
                col += 1;
            }
        }
    }
    *text = out;
}

/// An error's terminal text without the newline `print_nl` may start with
/// and the `! ` or `file:line: ` of `print_err`, TeX's line breaks taken
/// out.
fn strip_error_prefix(text: &mut Vec<u8>, max_print_line: i32) {
    while text.first() == Some(&b'\n') {
        text.remove(0);
    }
    unwrap_lines(text, max_print_line, 0);
    if text.starts_with(b"! ") {
        text.drain(..2);
        return;
    }
    // `file:line: `: the first `:<digits>: ` on the first line
    let first_line = text.iter().position(|&c| c == b'\n').unwrap_or(text.len());
    let mut i = 0;
    while i < first_line {
        if text[i] == b':' {
            let mut k = i + 1;
            while k < first_line && text[k].is_ascii_digit() {
                k += 1;
            }
            if k > i + 1 && k + 1 < text.len() && text[k] == b':' && text[k + 1] == b' ' {
                text.drain(..k + 2);
                return;
            }
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The locations above are the translation's: `print_cs`, `print_esc`,
    /// `give_err_help`, `show_context` read them.
    #[test]
    fn locations_match_the_translation() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated");
        let mut all = String::new();
        for e in std::fs::read_dir(dir).unwrap() {
            all.push_str(&std::fs::read_to_string(e.unwrap().path()).unwrap());
        }
        let body = |name: &str| {
            let s = all.split(&format!("pub fn {name}(")).nth(1).unwrap();
            s[..s.find("\n    pub fn ").unwrap_or(s.len())].to_string()
        };
        let print_cs = body("print_cs");
        for c in [
            "if (p < hash_base)",
            "if (p >= single_base)",
            "if (p == null_cs)",
            "if (p < active_base)",
            "if (((p >= undefined_control_sequence) && (p <= eqtb_size)) || (p > eqtb_top))",
            "((p >= prim_eqtb_base) && (p < frozen_null_font))",
        ] {
            assert!(print_cs.contains(c), "print_cs: {c}");
        }
        assert!(print_cs.contains(&format!(
            "self.eqtb[crate::ix::U(((((cat_code_base).wrapping_add(p)).wrapping_sub({SINGLE_BASE}i32)) - 1) as usize)].hh().rh()"
        )));
        assert!(body("print_esc").contains(&format!(
            "self.eqtb[crate::ix::U((({ESCAPE_CHAR_LOC}i32) - 1) as usize)].int()"
        )));
        assert!(body("give_err_help").contains("err_help_loc"));
        let sc = body("show_context");
        assert!(sc.contains(&format!(
            "self.eqtb[crate::ix::U((({END_LINE_CHAR_LOC}i32) - 1) as usize)].int()"
        )));
        assert!(sc.contains("if ((self.cur_input.name_field > 19i32) || (self.base_ptr == 0i32))"));
        assert!(sc.contains(&format!("self.cur_input.loc_field != ({}i32).wrapping_neg()", -NULL)));
        assert!(body("show_token_list").contains("max_char_val"));
        assert!(body("print").contains(&format!("{NEW_LINE_CHAR_LOC}i32")));
    }

    #[test]
    fn unwrapping_counts_characters() {
        let mut t = b"\n./a.tex:12: Undefined control sequence".to_vec();
        strip_error_prefix(&mut t, 79);
        assert_eq!(t, b"Undefined control sequence");
        // 78 ASCII bytes and one two-byte character fill a line of 79
        let mut t = vec![b'a'; 78];
        t.extend_from_slice("é".as_bytes());
        t.push(b'\n');
        t.extend_from_slice(b"bc\n\nd");
        unwrap_lines(&mut t, 79, 0);
        assert!(t.ends_with("ébc\n\nd".as_bytes()), "{:?}", String::from_utf8_lossy(&t));
    }

    #[test]
    fn caps_keep_whole_characters() {
        let mut b = "é".repeat(200).into_bytes();
        cap_before(&mut b);
        assert!(std::str::from_utf8(&b).is_ok());
        let mut a = "é".repeat(200).into_bytes();
        cap_after(&mut a);
        assert!(std::str::from_utf8(&a).is_ok() && a.len() <= TEXT_CAP);
    }
}
