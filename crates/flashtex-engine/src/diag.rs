//! The diagnostics side channel (lane P5-DIAGNOSTICS): what the engine
//! knows about each error and warning at the moment it reports it, kept as
//! [`Note`]s for the host, which turns them into `diag-v1` messages
//! (docs/protocol/display-list-v3.md §6.7, `src/host/diag.rs`).
//!
//! `changes/diagnostics.ch` calls the routines here (`dg_*`) where TeX
//! reports something: `print_err` (the message starts), `error` (the
//! message is printed and the input stack, help lines and `use_err_help`
//! are as the error left them), `pdf_warning`, the overfull/underfull box
//! reports of `hpack` and `vpackage`, `\def` (definition sites for the
//! trace) and `write_out` (LaTeX's and packages' warnings are
//! `\immediate\write`s to the terminal). They do nothing unless the host
//! turned the side channel on ([`set_enabled`]); they only read TeX's
//! variables and never print, so the terminal and the log are exactly
//! what they are without them (DESIGN.md §1.1, P-T1).
//!
//! What a note holds, all from state TeX keeps anyway:
//!
//! * where TeX was reading ([`Note::pos`]): the innermost *file* level of
//!   the input stack, which is the level TeX's context display ends with
//!   (`l.<line> <text read>` / `<text still to read>`), with the column of
//!   that split (`loc - start`), and the start of the token before it;
//! * every level of the input stack ([`Note::frames`], innermost first),
//!   which TeX shows only `\errorcontextlines` of (LaTeX: none but the top
//!   and bottom): macros with their names, parameter text and body split
//!   where TeX is, and the macro's definition site when this run saw the
//!   `\def`;
//! * the help lines (or the `\errhelp` text), which TeX writes to the log
//!   only;
//! * for a box report, the source positions of the box's first and last
//!   character (the display list's side table), which TeX's report reduces
//!   to "in paragraph at lines a--b".
//!
//! **Checkpoints.** The notes are part of the host state a checkpoint
//! records (`crate::checkpoint::ExtRecord::notes`, like the captured
//! terminal): a restore cuts them back, a convergence splices the old
//! run's later notes in, and S₀ persists them, so an incremental compile
//! reports exactly what a run from scratch reports (the soundness tests).
//! The definition sites are a side table keyed by the macro's token list
//! and a fingerprint of its first tokens, checked when read, so a restore
//! that frees or reuses a token list cannot give a macro a wrong site.

use crate::generated::Globals;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// eqtb and hash locations this module reads, in this build's layout
// (pdftex.web §222, §230, §236 with web2rust-default.args's sizes and the
// change files' extra parameters). `tests::locations_match_the_translation`
// checks them against src/generated/, so a regeneration cannot move them
// silently.
const ACTIVE_BASE: i32 = 1;
const SINGLE_BASE: i32 = 257;
const NULL_CS: i32 = 513;
const HASH_BASE: i32 = 514;
/// `prim_eqtb_base`..: the frozen `\pdfprimitive` names (`print_cs`).
const PRIM_EQTB_BASE: i32 = 615_526;
const PRIM_EQTB_END: i32 = 617_626;
const UNDEFINED_CONTROL_SEQUENCE: i32 = 626_627;
const ERR_HELP_LOC: i32 = 627_167;
const CAT_CODE_BASE: i32 = 627_738;
const ESCAPE_CHAR_LOC: i32 = 629_063;
const END_LINE_CHAR_LOC: i32 = 629_066;
const NEW_LINE_CHAR_LOC: i32 = 629_067;
const CS_TOKEN_FLAG: i32 = 4095;
/// `list_ptr(r)` is `link(r+list_offset)`.
const LIST_OFFSET: i32 = 5;
/// Selector codes (§54).
const TERM_ONLY: i32 = 17;
const TERM_AND_LOG: i32 = 19;
/// Text kept of one side of a context line.
const TEXT_CAP: usize = 240;
/// Levels kept of the input stack: the innermost `MAX_FRAMES - 1` and the
/// bottom one (a note is sent with every compile of the document).
const MAX_FRAMES: usize = 24;

static ON: AtomicBool = AtomicBool::new(false);

/// Turn the side channel on (the host) or off.
pub fn set_enabled(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

#[inline(always)]
pub fn enabled() -> bool {
    ON.load(Ordering::Relaxed)
}

/// What reported the note.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `error` after `print_err`.
    Error,
    /// `error` without a message of `print_err`'s: `\show`, `\showbox`,
    /// `\showthe`, `\showlists`, ... in a mode that stops.
    Show,
    /// A `\write` to the terminal whose text says "Warning", or that
    /// shows an error of its own (a line starting `! `).
    Write,
    /// An overfull, underfull, tight or loose box.
    Box,
    /// `pdf_warning`.
    PdfWarning,
}
crate::codec_enum!(Kind {
    Error,
    Show,
    Write,
    Box,
    PdfWarning
});

/// A place in a file.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Pos {
    /// The file's name as TeX has it (`./main.tex`, a kpathsea path).
    pub file: Vec<u8>,
    /// 1-based.
    pub line: i32,
    /// The 0-based byte column of the split point of TeX's context line
    /// (what TeX has read of the line), or -1.
    pub col: i32,
    /// The 0-based byte column where the token before the split starts,
    /// or -1.
    pub from: i32,
}
crate::codec_struct!(Pos {
    file,
    line,
    col,
    from
});

/// One level of the input stack.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Frame {
    /// `file`, `scantokens`, `terminal`, `insert`, `read`, or a token
    /// list's type: `macro`, `argument`, `template`, `backed_up`,
    /// `recently_read`, `inserted`, `output`, `everypar`, `everymath`,
    /// `everydisplay`, `everyhbox`, `everyvbox`, `everyjob`, `everycr`,
    /// `mark`, `everyeof`, `write`.
    pub kind: String,
    /// A macro's name as TeX prints it (`\section`); `read`: the stream.
    pub name: Vec<u8>,
    /// A file level's place (`col` is the split).
    pub pos: Option<Pos>,
    /// What TeX has read of the level and what it has still to read (a
    /// line of a file, or a token list shown as TeX shows it).
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    /// A macro's definition site (file, line; `col` -1), when this run
    /// saw the definition.
    pub def: Option<Pos>,
}
crate::codec_struct!(Frame {
    kind,
    name,
    pos,
    before,
    after,
    def
});

/// One error or warning, as the engine knew it when it reported it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub kind: Kind,
    /// Where the report starts and where the part this note covers ends,
    /// in the captured terminal's bytes.
    pub at: usize,
    pub end: usize,
    /// What the terminal shows of the report: an error's message after
    /// `! ` or `file:line: ` (without the `.` `error` adds); a write's
    /// text; a box report's two lines; a pdfTeX warning.
    pub text: Vec<u8>,
    /// Help lines in the order the log shows them (or the `\errhelp`
    /// text's lines).
    pub help: Vec<Vec<u8>>,
    /// The input stack, innermost level first, down to the level TeX's
    /// context display ends with.
    pub frames: Vec<Frame>,
    /// The innermost file level: where TeX was reading.
    pub pos: Option<Pos>,
    /// A box report: `pack_begin_line` (negative: an alignment; 0: none)
    /// and `line`.
    pub lines: (i32, i32),
    /// A box report: the display-list source locations (span, column) of
    /// its first and last character; (0, _) when unknown.
    pub first: (u32, u16),
    pub last: (u32, u16),
    /// Bit 0: `output_active`. Bit 1: the terminal did not show it.
    pub flags: u32,
}
crate::codec_struct!(Note {
    kind,
    at,
    end,
    text,
    help,
    frames,
    pos,
    lines,
    first,
    last,
    flags
});

pub const FLAG_OUTPUT_ACTIVE: u32 = 1;
pub const FLAG_NOT_ON_TERMINAL: u32 = 2;

/// A definition site (`\def` and its relatives).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub file: Vec<u8>,
    pub line: i32,
    /// A fingerprint of the token list's first tokens when it was defined.
    pub print: u64,
}
crate::codec_struct!(Site { file, line, print });

#[derive(Default)]
struct St {
    notes: Vec<Arc<Note>>,
    /// The terminal length when `print_err` started the message.
    mark: Option<usize>,
    /// `dg_write_begin`: the terminal length, and whether the write goes
    /// to the terminal.
    write: Option<(usize, bool)>,
    /// `dg_box_begin`.
    box_at: Option<usize>,
    /// `dg_def_begin`.
    def_at: Option<(Vec<u8>, i32)>,
    /// Token list (a macro's `def_ref`) -> where it was defined.
    defs: HashMap<i32, Site>,
    /// The file name last looked up: (string number, its bytes).
    name_cache: Option<(i32, Vec<u8>)>,
}

thread_local! {
    static ST: RefCell<St> = RefCell::new(St::default());
}

fn with<R>(f: impl FnOnce(&mut St) -> R) -> R {
    ST.with(|s| f(&mut s.borrow_mut()))
}

// ---------------------------------------------------------------------------
// The notes as host state (checkpoints, S₀, the host)

/// How many notes there are.
pub fn len() -> usize {
    with(|s| s.notes.len())
}

/// Cut the notes back to `n` (restoring a checkpoint).
pub fn truncate(n: usize) {
    with(|s| {
        s.notes.truncate(n);
        s.mark = None;
        s.write = None;
        s.box_at = None;
        s.def_at = None;
    })
}

/// The notes from `n` on (shared: a restore keeps the old run's, which
/// every keystroke's restore would otherwise copy).
pub fn notes_from(n: usize) -> Vec<Arc<Note>> {
    with(|s| s.notes.get(n..).map(|v| v.to_vec()).unwrap_or_default())
}

/// Every note.
pub fn notes() -> Vec<Arc<Note>> {
    notes_from(0)
}

/// Notes after which a note keeps only the file level of its trace (a
/// document with thousands of warnings sends every one each compile).
const FULL_TRACES: usize = 1000;

fn push(mut n: Note) {
    with(|s| {
        if s.notes.len() >= FULL_TRACES && n.frames.len() > 1 {
            let bottom = n.frames.pop();
            n.frames.clear();
            n.frames.extend(bottom);
        }
        s.notes.push(Arc::new(n));
    })
}

/// Append notes (a convergence splices the old run's later notes in),
/// with their terminal offsets moved by `shift`.
pub fn append(v: &[Arc<Note>], shift: i64) {
    with(|s| {
        for n in v {
            if shift == 0 {
                s.notes.push(n.clone());
            } else {
                let mut n = (**n).clone();
                n.at = (n.at as i64 + shift).max(0) as usize;
                n.end = (n.end as i64 + shift).max(0) as usize;
                s.notes.push(Arc::new(n));
            }
        }
    })
}

/// A new run from scratch (a new engine): no notes, no definition sites.
pub fn reset() {
    with(|s| *s = St::default())
}

/// The definition sites, for S₀.
pub fn sites() -> Vec<(i32, Site)> {
    with(|s| {
        let mut v: Vec<(i32, Site)> = s.defs.iter().map(|(k, v)| (*k, v.clone())).collect();
        v.sort_by_key(|x| x.0);
        v
    })
}

/// Put definition sites back (opening S₀).
pub fn set_sites(v: Vec<(i32, Site)>) {
    with(|s| s.defs = v.into_iter().collect())
}

// ---------------------------------------------------------------------------
// Reading TeX's state (never writing it)

fn str_bytes(g: &Globals, s: i32) -> Vec<u8> {
    if s < 0 || s >= g.str_ptr {
        return Vec::new();
    }
    let (a, b) = (g.str_start[s as usize], g.str_start[s as usize + 1]);
    if a < 0 || b < a {
        return Vec::new();
    }
    (a..b).map(|i| g.str_pool[i as usize] as u8).collect()
}

fn eqtb_int(g: &Globals, loc: i32) -> i32 {
    g.eqtb[(loc - 1) as usize].int()
}

fn cat_code(g: &Globals, c: u8) -> i32 {
    g.eqtb[(CAT_CODE_BASE + c as i32 - 1) as usize].hh_rh()
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

/// `print_esc(s)`'s bytes for a string (or a character).
fn push_esc(g: &Globals, out: &mut Vec<u8>, name: &[u8]) {
    let c = eqtb_int(g, ESCAPE_CHAR_LOC);
    if (0..256).contains(&c) {
        out.push(c as u8);
    }
    out.extend_from_slice(name);
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
                let c = (p - SINGLE_BASE) as u8;
                push_esc(g, out, &[c]);
                if cat_code(g, c) == 11 {
                    out.push(b' ');
                }
            }
        } else if p < ACTIVE_BASE {
            push_esc(g, out, b"IMPOSSIBLE.");
        } else {
            out.push((p - ACTIVE_BASE) as u8);
        }
    } else if p >= UNDEFINED_CONTROL_SEQUENCE {
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
    let mut match_chr = b'#';
    let mut n = b'0';
    let mut count = 0;
    while p != 0 && count < 100_000 {
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
            let (m, c) = (t / 256, (t % 256) as u8);
            match m {
                1 | 2 | 3 | 4 | 7 | 8 | 10 | 11 | 12 => out.push(c),
                6 => {
                    out.push(c);
                    out.push(c);
                }
                5 => {
                    out.push(match_chr);
                    if c <= 9 {
                        out.push(c + b'0');
                    } else {
                        out.push(b'!');
                        break;
                    }
                }
                13 => {
                    match_chr = c;
                    out.push(c);
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
    after.truncate(TEXT_CAP);
    (before, after)
}

fn cap_before(b: &mut Vec<u8>) {
    if b.len() > TEXT_CAP {
        let cut = b.len() - TEXT_CAP;
        b.drain(..cut);
    }
}

/// A fingerprint of a token list's first 64 tokens and whether it has
/// more (definition sites are checked against it when read).
fn fingerprint(g: &Globals, mut p: i32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut k = 0;
    while p != 0 && k < 64 {
        if !in_token_mem(g, p) {
            return 0;
        }
        h = (h ^ info(g, p) as u32 as u64).wrapping_mul(0x0100_0000_01b3);
        p = link(g, p);
        k += 1;
    }
    h ^ (p != 0) as u64
}

impl Globals {
    /// The bytes of file-name string `s` (the last one is cached: a
    /// definition asks for the same file's name again and again).
    fn dg_name(&self, s: i32) -> Vec<u8> {
        if s <= 0 {
            return Vec::new();
        }
        let hit = with(|st| match &st.name_cache {
            Some((k, b)) if *k == s => {
                // the string may have been freed and remade since
                let (a, e) = (self.str_start[s as usize], self.str_start[s as usize + 1]);
                (e - a) as usize == b.len()
                    && (a..e)
                        .zip(b.iter())
                        .all(|(i, &c)| self.str_pool[i as usize] as u8 == c)
            }
            _ => false,
        });
        if hit {
            return with(|st| st.name_cache.as_ref().unwrap().1.clone());
        }
        let b = str_bytes(self, s);
        with(|st| st.name_cache = Some((s, b.clone())));
        b
    }

    /// The line a file level (`index` = its `in_open` level) is at.
    fn dg_level_line(&self, index: i32) -> i32 {
        if index == self.in_open {
            self.line
        } else {
            self.line_stack[index as usize]
        }
    }

    /// Where TeX is reading: the innermost file level (the level TeX's
    /// context display ends with), cheaply (no text).
    fn dg_here(&self) -> Option<(Vec<u8>, i32)> {
        let is_file =
            |r: &crate::generated::types::in_state_record| r.state_field != 0 && r.name_field > 19;
        let rec = if is_file(&self.cur_input) {
            self.cur_input
        } else {
            *(0..self.input_ptr as usize)
                .rev()
                .map(|k| &self.input_stack[k])
                .find(|r| is_file(r))?
        };
        let index = rec.index_field;
        let name = self.full_source_filename_stack[index as usize];
        Some((self.dg_name(name), self.dg_level_line(index)))
    }

    /// The input stack, innermost level first, as `show_context` walks it
    /// (every level, not only `\errorcontextlines` of them; a backed-up
    /// list already read is left out, as there), and the bottom file
    /// level's place.
    fn dg_frames(&self) -> (Vec<Frame>, Option<Pos>) {
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
            if r.state_field != 0 && (r.name_field > 19 || k == 0) {
                bottom = true;
            }
            if r.state_field != 0 {
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
                }
                // The line, split where TeX is (`@<Pseudoprint the line@>`).
                let end_line = eqtb_int(self, END_LINE_CHAR_LOC);
                let limit = r.limit_field;
                let j = if limit >= 0 && self.buffer[limit as usize] == end_line {
                    limit
                } else {
                    limit + 1
                };
                let start = r.start_field;
                let split = r.loc_field.clamp(start, j.max(start));
                let byte = |i: i32| self.buffer[i as usize] as u8;
                f.before = (start..split).map(byte).collect();
                cap_before(&mut f.before);
                f.after = (split..j.max(split)).map(byte).take(TEXT_CAP).collect();
                if r.name_field > 17 {
                    let name = self.full_source_filename_stack[r.index_field as usize];
                    let line = self.dg_level_line(r.index_field);
                    let from = token_start(self, start, split);
                    let p = Pos {
                        file: self.dg_name(name),
                        line,
                        col: split - start,
                        from: from - start,
                    };
                    if pos.is_none() && r.name_field > 19 {
                        pos = Some(p.clone());
                    }
                    f.pos = Some(p);
                }
                frames.push(f);
            } else if k == top || r.index_field != 3 || r.loc_field != 0 {
                let t = r.index_field;
                let kind = match t {
                    0 => "argument",
                    1 | 2 => "template",
                    3 => {
                        if r.loc_field == 0 {
                            "recently_read"
                        } else {
                            "backed_up"
                        }
                    }
                    4 => "inserted",
                    5 => "macro",
                    6 => "output",
                    7 => "everypar",
                    8 => "everymath",
                    9 => "everydisplay",
                    10 => "everyhbox",
                    11 => "everyvbox",
                    12 => "everyjob",
                    13 => "everycr",
                    14 => "mark",
                    19 => "everyeof",
                    20 => "write",
                    _ => "?",
                };
                let mut f = Frame {
                    kind: kind.into(),
                    ..Frame::default()
                };
                let list = if t < 5 {
                    r.start_field
                } else {
                    link(self, r.start_field)
                };
                if t == 5 {
                    let mut n = Vec::new();
                    push_cs(self, &mut n, r.name_field);
                    while n.last() == Some(&b' ') {
                        n.pop();
                    }
                    f.name = n;
                    f.def = with(|st| st.defs.get(&r.start_field).cloned())
                        .filter(|s| s.print == fingerprint(self, r.start_field))
                        .map(|s| Pos {
                            file: s.file,
                            line: s.line,
                            col: -1,
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
            if p == 0 || !in_token_mem(self, p) {
                return Vec::new();
            }
            let (text, _) = token_text(self, link(self, p), -1);
            let nl = eqtb_int(self, NEW_LINE_CHAR_LOC);
            return text
                .split(|&c| c == b'\n' || (0..256).contains(&nl) && c as i32 == nl)
                .map(|l| l.to_vec())
                .collect();
        }
        (0..self.help_ptr.max(0))
            .rev()
            .map(|k| str_bytes(self, self.help_line[k as usize]))
            .collect()
    }

    /// `print_err` starts a message.
    pub fn dg_mark(&mut self) {
        if enabled() {
            let n = crate::system::terminal_len();
            with(|s| s.mark = Some(n));
        }
    }

    /// `error`: the message is on the terminal, the input stack and the
    /// help are as the error left them.
    pub fn dg_error(&mut self) {
        if enabled() {
            self.dg_note_error();
        }
    }

    #[inline(never)]
    fn dg_note_error(&mut self) {
        let now = crate::system::terminal_len();
        let mark = with(|s| s.mark.take());
        let (kind, at) = match mark {
            Some(at) if at <= now => (Kind::Error, at),
            _ => {
                // `\show` and its relatives print "> ..." and stop
                let tail = crate::system::terminal_slice(now.saturating_sub(4096), now);
                let at = tail
                    .windows(3)
                    .rposition(|w| w == b"\n> ")
                    .map(|i| now - tail.len() + i)
                    .unwrap_or(now);
                (Kind::Show, at)
            }
        };
        let mut text = crate::system::terminal_slice(at, now);
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
        push(note);
    }

    /// `pdf_warning`: its message is on the terminal.
    pub fn dg_pdf_warning(&mut self) {
        if enabled() {
            let now = crate::system::terminal_len();
            let at = with(|s| s.mark.take()).unwrap_or(now).min(now);
            let mut text = crate::system::terminal_slice(at, now);
            unwrap_lines(&mut text, self.max_print_line, 0);
            let (frames, pos) = self.dg_frames();
            {
                push(Note {
                    kind: Kind::PdfWarning,
                    at,
                    end: now,
                    text,
                    help: Vec::new(),
                    frames,
                    pos,
                    lines: (0, self.line),
                    first: (0, 0),
                    last: (0, 0),
                    flags: 0,
                });
            }
        }
    }

    /// A box report's position part starts.
    pub fn dg_box_begin(&mut self, _r: i32) {
        if enabled() {
            let n = crate::system::terminal_len();
            with(|s| s.box_at = Some(n));
        }
    }

    /// A box report is on the terminal (its two lines).
    pub fn dg_box_end(&mut self, r: i32) {
        if enabled() {
            self.dg_note_box(r);
        }
    }

    #[inline(never)]
    fn dg_note_box(&mut self, r: i32) {
        let now = crate::system::terminal_len();
        let Some(mid) = with(|s| s.box_at.take()) else {
            return;
        };
        // The report's first line starts after the newline before `mid`.
        let back = crate::system::terminal_slice(mid.saturating_sub(400), mid);
        let at = back
            .iter()
            .rposition(|&c| c == b'\n')
            .map(|i| mid - back.len() + i + 1)
            .unwrap_or(mid.saturating_sub(back.len()));
        let mut text = crate::system::terminal_slice(at, now);
        unwrap_lines(&mut text, self.max_print_line, 0);
        while text.last() == Some(&b'\n') {
            text.pop();
        }
        let (first, last) = self.dg_box_extent(link(self, r + LIST_OFFSET));
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
        push(note);
    }

    /// The display-list source locations of the first and last characters
    /// in the node list `p` (descending into boxes), bounded.
    fn dg_box_extent(&self, p: i32) -> ((u32, u16), (u32, u16)) {
        let mut first = (0, 0);
        let mut last = (0, 0);
        let mut stack = vec![p];
        let mut seen = 0;
        while let Some(mut q) = stack.pop() {
            while q != 0 && seen < 200_000 {
                seen += 1;
                if q < 0 || q > self.mem_end {
                    break;
                }
                let is_char = q >= self.hi_mem_min;
                if is_char {
                    if let Some(l) = crate::displaylist::node_loc(q) {
                        if first.0 == 0 {
                            first = l;
                        }
                        last = l;
                    }
                } else {
                    let t = self.mem[q as usize].hh_b0();
                    // hlist_node, vlist_node: their lists, in order
                    if t == 0 || t == 1 {
                        let inner = link(self, q + LIST_OFFSET);
                        if inner != 0 {
                            stack.push(link(self, q));
                            stack.push(inner);
                            break;
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
        if enabled() {
            let here = self.dg_here();
            with(|s| s.def_at = here);
        }
    }

    /// `\def` made token list `p` the macro's meaning.
    pub fn dg_def(&mut self, p: i32) {
        if enabled() {
            if let Some((file, line)) = with(|s| s.def_at.take()) {
                let print = fingerprint(self, p);
                with(|s| {
                    s.defs.insert(p, Site { file, line, print });
                });
            }
        }
    }

    /// `write_out` is about to show a `\write`'s text.
    pub fn dg_write_begin(&mut self, j: i32) {
        if enabled() {
            let to_terminal =
                j != 18 && (self.selector == TERM_ONLY || self.selector == TERM_AND_LOG);
            let n = crate::system::terminal_len();
            with(|s| s.write = Some((n, to_terminal)));
        }
    }

    /// `write_out` showed it.
    pub fn dg_write_end(&mut self, _j: i32) {
        if enabled() {
            self.dg_note_write();
        }
    }

    #[inline(never)]
    fn dg_note_write(&mut self) {
        let Some((at, to_terminal)) = with(|s| s.write.take()) else {
            return;
        };
        if !to_terminal {
            return;
        }
        let now = crate::system::terminal_len();
        let mut text = crate::system::terminal_slice(at, now);
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
        push(note);
    }
}

fn contains(h: &[u8], n: &[u8]) -> bool {
    h.windows(n.len()).any(|w| w == n)
}

/// Where the token before `split` starts in `buffer[start..split]`: a
/// control word or symbol with its escape character, a multi-byte UTF-8
/// character whole, else one byte; spaces before `split` are skipped
/// first. Catcodes are the current ones.
fn token_start(g: &Globals, start: i32, split: i32) -> i32 {
    let b = |i: i32| g.buffer[i as usize] as u8;
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
        if k > start && cat_code(g, b(k - 1)) != 0 {
            // a word: its last letter is the token, unless the word is
            // one UTF-8 character
            let mut s = e - 1;
            while s > start && (b(s) & 0xC0) == 0x80 {
                s -= 1;
            }
            return s;
        }
        return e - 1;
    }
    if e - 2 >= start && cat_code(g, b(e - 2)) == 0 {
        return e - 2;
    }
    let mut s = e - 1;
    while s > start && (b(s) & 0xC0) == 0x80 {
        s -= 1;
    }
    s
}

/// When the text before `e` ends with a group (`}`) or an optional
/// argument (`]`): where the command those arguments belong to starts
/// (`\ref{x}`, `\section*[a]{b}`), or the first group's `{` when no
/// control sequence precedes them. `None` otherwise.
fn call_start(g: &Globals, start: i32, e: i32) -> Option<i32> {
    let b = |i: i32| g.buffer[i as usize] as u8;
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
        let (close_is, open): (bool, u8) = if cat(k2 - 1) == 2 {
            (true, 0)
        } else if b(k2 - 1) == b']' && cat(k2 - 1) == 12 {
            (true, b'[')
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
                } else if b(i) == b'[' && depth == 0 {
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
    if k2 > start && b(k2 - 1) == b'*' {
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
/// at terminal column `col0`): after exactly `max_print_line` bytes on a
/// line, the newline is TeX's, not the text's.
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
            col += 1;
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
            format!("if (p < {HASH_BASE}i32)"),
            format!("if (p >= {SINGLE_BASE}i32)"),
            format!("if (p == {NULL_CS}i32)"),
            format!("if (p < {ACTIVE_BASE}i32)"),
            format!("if (p >= {UNDEFINED_CONTROL_SEQUENCE}i32)"),
            format!("((p >= {PRIM_EQTB_BASE}i32) && (p < {PRIM_EQTB_END}i32))"),
            format!("self.eqtb[(((({CAT_CODE_BASE}i32).wrapping_add(p)).wrapping_sub({SINGLE_BASE}i32)) - 1)"),
        ] {
            assert!(print_cs.contains(&c), "print_cs: {c}");
        }
        assert!(body("print_esc").contains(&format!(
            "c = self.eqtb[(({ESCAPE_CHAR_LOC}i32) - 1) as usize].int();"
        )));
        assert!(body("give_err_help").contains(&format!(
            "self.eqtb[(({ERR_HELP_LOC}i32) - 1) as usize].hh().rh()"
        )));
        let sc = body("show_context");
        assert!(sc.contains(&format!(
            "self.eqtb[(({END_LINE_CHAR_LOC}i32) - 1) as usize].int()"
        )));
        assert!(sc.contains("if ((self.cur_input.name_field > 19i32) || (self.base_ptr == 0i32))"));
        assert!(body("print").contains(&format!(
            "self.eqtb[(({NEW_LINE_CHAR_LOC}i32) - 1) as usize].int()"
        )));
        assert!(body("show_token_list").contains(&format!(
            "if (self.mem[(p) as usize].hh().lh() >= {CS_TOKEN_FLAG}i32)"
        )));
    }

    #[test]
    fn unwrapping_and_prefixes() {
        let mut t = b"\n./a.tex:12: Undefined control sequence".to_vec();
        strip_error_prefix(&mut t, 79);
        assert_eq!(t, b"Undefined control sequence");
        let mut t = b"\n! LaTeX Error: x.\n\nSee".to_vec();
        strip_error_prefix(&mut t, 79);
        assert_eq!(t, b"LaTeX Error: x.\n\nSee");
        let mut t = vec![b'a'; 79];
        t.push(b'\n');
        t.extend_from_slice(b"bc\n\nd");
        unwrap_lines(&mut t, 79, 0);
        assert_eq!(t.len(), 79 + 2 + 3);
        assert!(t.ends_with(b"abc\n\nd"));
    }
}
