//! PREAMBLE-MIDLINE (design note #1594; DESIGN.md §5.2, §5.3): a preamble
//! edit restarts in the middle of the main file's line, with the rest of
//! that line read again from the new text.
//!
//! `\usepackage{x}` and `\documentclass{x}` look for an optional `[date]`
//! (`\@ifnextchar[`, a `\futurelet`) *before* they load the file, so the
//! first token of the next line is read before the package is. Every
//! checkpoint taken while the package loads, or after it at the line's end
//! (`Point::PreambleLine`), has read the main file past that line, and an
//! edit in it (a `\title` keystroke) restarted before the package and
//! loaded it again.
//!
//! * **The checkpoint** (`Globals::midline_checkpoint`, [`here`]): at the
//!   first `big_switch` after an `\input` file closes back into the main
//!   file at its top level (`in_open = 1`), with every level above the main
//!   file a token list and `first = limit + 1` (nothing else uses the buffer
//!   above the line: no `\scantokens`, `\read` or `\csname` is live): one
//!   after every package, like the line-end checkpoint it may stand in for
//!   (which is then taken only if `preamble_line_s` has passed). It is a
//!   `Point::PreambleLine` with a [`MidLine`] beside it.
//! * **What the state depends on.** `get_next` has read `buffer[start..loc)`
//!   and looked at most three places past `loc` (`buffer[loc..=loc+3]`: the
//!   `^^` sequences of tex.web §352 and §355, read before it decides the
//!   token ends), and at whether those places are inside the line (`k <=
//!   limit`, `k < limit`, `k + 2 <= limit` for `k <= loc + 1`). Nothing else
//!   reads the line's rest, except what prints it: `show_context` (an error,
//!   `\show`) and the diagnostics' frames, when the main file is the
//!   innermost file shown, and `firm_up_the_line` under `\pausing`. A
//!   checkpoint is a mid-line restart point only if none of these printed
//!   the line since it was read ([`MainRead`]: `shown`, `prompted`), and the
//!   buffer is what `input_ln` made of the file's line: its bytes through
//!   `xord`, trailing blanks dropped, then `\endlinechar` *as it was when the
//!   line was read* (`MainRead::eol`; the restored `eqtb` may hold another).
//!   A `^^` sequence in a control sequence's name rewrites the buffer
//!   (§355): such a line is not one.
//! * **The restart test** ([`check`]): the record's main file stream holds
//!   the old file's line that starts at `at`, the last line end before the
//!   change; the change is past `at`; the new file's line at `at`, as
//!   `input_ln` would make it with the old `\endlinechar`, has the same
//!   buffer over `start..=min(limit, loc+3)` and the same `min(limit,
//!   loc+4)`. The caller (`incr::Session::preamble_restart`) then applies
//!   `consumed_nothing_changed` with the main file consumed up to `at`.
//! * **The refill** (`Globals::midline_after_restore`): the main file is
//!   reopened at `at` and its line read again as `input_ln` reads it (the
//!   same bytes, else the restart is abandoned); the buffer from `start`,
//!   `limit`, `first` and `last` (every later position moved by the line's
//!   change of length) are set as a run from the format has them there.
//!   The stream's offset is the new line's end, so the journal, the
//!   convergence test and later restarts see the new text's offsets.
//! * **Stale checkpoints.** The checkpoints taken while the old line was
//!   buffered (the restart point and those before it on the same line) hold
//!   the old line: they are not the new text's. Every candidate's record is
//!   checked against the text the last run read ([`check`]: the line its
//!   stream holds must be that text's line ending at its offset), and the
//!   session drops them after the run (`incr`). The run takes a new
//!   mid-line checkpoint at once after the refill, so the next keystroke on
//!   the line restarts there.
//! * **Persistence.** Nothing here is persisted: S₀ is never a mid-line
//!   point, and a host opened from a persisted S₀ has no checkpoint before
//!   it. `FLASHTEX_PREAMBLE_MIDLINE=off` disables the checkpoints.

use crate::checkpoint::ExtRecord;
use crate::generated::consts::{
    buf_size, end_line_char_code, int_base, nonstop_mode, pausing_code, token_list,
};
use crate::generated::Globals;
use crate::incr::Edit;
use crate::system::Stream;

/// What `input_ln` did with the main file's current line (`line`), noted
/// as the line was read (tex.web §362, §538) and kept with the state: set
/// by the read, cleared by every restore (`Globals::restore_ext`), set
/// again from a restored mid-line checkpoint's [`MidLine`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MainRead {
    /// The line's number.
    pub line: i32,
    /// `\endlinechar` when it was read: `buffer[limit]` when in 0..=255.
    pub eol: i32,
    /// `\pausing > 0` in a mode above `\nonstopmode`: `firm_up_the_line`
    /// showed the line and read the terminal.
    pub prompted: bool,
    /// The line was printed since: the main file's level in
    /// `show_context` (`ls_print_level`) or a diagnostics frame.
    pub shown: bool,
}

/// A mid-line `Point::PreambleLine`: where the main file's line stood.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MidLine {
    pub line: i32,
    pub start: i32,
    pub loc: i32,
    pub limit: i32,
    /// `\endlinechar` when the line was read ([`MainRead::eol`]).
    pub eol: i32,
    /// `buffer[start..=min(limit, loc + 3)]`: what `get_next` looked at.
    pub seen: Vec<i32>,
}

/// A mid-line restart's refill of the main file's line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refill {
    pub path: String,
    /// Where the line starts, in the old and the new text alike.
    pub at: u64,
    /// The new line's bytes, and the offset after its end of line.
    pub line: Vec<u8>,
    pub end: u64,
    /// The old line's end (the record's offset).
    pub old_end: u64,
}

/// Mid-line checkpoints are taken (`FLASHTEX_PREAMBLE_MIDLINE=off`: not).
pub fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("FLASHTEX_PREAMBLE_MIDLINE").as_deref() != Ok("off"))
}

fn eqtb_int(g: &Globals, code: i32) -> i32 {
    g.eqtb[(int_base + code - 1) as usize].int()
}

/// The buffer `input_ln` and §362 make of `line` at `start`: the bytes
/// through `xord`, trailing blanks (after `xord`) dropped, then `eol` at
/// `limit` when it is in 0..=255 (else `limit` is one less). The values
/// from `start` to `limit` inclusive, and `last` (one past the last
/// non-blank).
pub fn fill(xord: impl Fn(u8) -> i32, start: i32, line: &[u8], eol: i32) -> (Vec<i32>, i32) {
    let mut v: Vec<i32> = line.iter().map(|&c| xord(c)).collect();
    let n = v
        .iter()
        .rposition(|&c| c != b' ' as i32)
        .map_or(0, |i| i + 1);
    v.truncate(n);
    let last = start + n as i32;
    if (0..=255).contains(&eol) {
        v.push(eol);
    } else {
        v.pop();
    }
    (v, last)
}

/// Where the line `line`, which a reader at `off` read last, starts in
/// `text` (`read_tex_line`: a line ends at an LF, a CR or a CR LF, or the
/// end of the file): `None` when `text` does not have that line there.
pub fn line_start(text: &[u8], off: usize, line: &[u8]) -> Option<usize> {
    if off > text.len() {
        return None;
    }
    let t = match off.checked_sub(1).map(|i| text[i]) {
        Some(b'\n') if off >= 2 && text[off - 2] == b'\r' => 2,
        Some(b'\n') => 1,
        // (an LF after it would have been read with it)
        Some(b'\r') if text.get(off) != Some(&b'\n') => 1,
        Some(b'\r') => return None,
        _ if off == text.len() => 0,
        _ => return None,
    };
    let s = off.checked_sub(t + line.len())?;
    if &text[s..off - t] != line {
        return None;
    }
    // a whole line: after a line end or at the start (a CR LF is one)
    if s > 0 {
        match text[s - 1] {
            b'\n' => {}
            b'\r' if text.get(s) != Some(&b'\n') => {}
            _ => return None,
        }
    }
    Some(s)
}

/// The line `read_tex_line` reads at `s` in `text` and the offset after it
/// (and after its end of line).
pub fn line_at(text: &[u8], s: usize) -> (&[u8], usize) {
    let e = text[s..]
        .iter()
        .position(|&c| c == b'\n' || c == b'\r')
        .map_or(text.len(), |i| s + i);
    let end = if e == text.len() {
        e
    } else if text[e] == b'\r' && text.get(e + 1) == Some(&b'\n') {
        e + 2
    } else {
        e + 1
    };
    (&text[s..e], end)
}

/// What a candidate restart point before S₀ is for a preamble edit.
#[derive(Debug)]
pub enum Verdict {
    /// Its record holds a line the text the last run read does not have
    /// there (a checkpoint the refill of a mid-line restart left behind):
    /// not a restart point.
    Stale,
    /// The ordinary test decides (`consumed_nothing_changed`).
    Plain,
    /// A mid-line restart: the ordinary test with the main file consumed
    /// up to the line's start, then this refill.
    Refill(Refill),
}

/// The record `rec` of a checkpoint before S₀ (with its [`MidLine`] if it
/// is a mid-line one) against `edits`: `old(path)` is a changed file as the
/// last run read it, `now(path)` as it is now. See the module comment.
pub fn check<'a>(
    g: &Globals,
    rec: &ExtRecord,
    mid: Option<&MidLine>,
    edits: &[Edit],
    old: impl Fn(&str) -> Option<&'a [u8]>,
    now: impl Fn(&str) -> Option<&'a [u8]>,
) -> Verdict {
    // every stream on a changed file holds the line the old text has there
    for e in edits {
        let Some(text) = old(&e.path) else { continue };
        for f in &rec.files {
            if let Stream::In { path, offset } = &f.stream {
                if *path == e.path
                    && f.have_line
                    && line_start(text, *offset as usize, &f.line).is_none()
                {
                    return Verdict::Stale;
                }
            }
        }
    }
    let Some(m) = mid else { return Verdict::Plain };
    // the main file: `input_file[1]` (term_in, term_out, pool_file,
    // log_file, then the input files: `Globals::visit_files`)
    let Some(f) = rec.files.get(4) else {
        return Verdict::Plain;
    };
    let Stream::In { path, offset } = &f.stream else {
        return Verdict::Plain;
    };
    let Some(e) = edits.iter().find(|e| e.path == *path) else {
        return Verdict::Plain;
    };
    let (Some(old_text), Some(new_text)) = (old(path), now(path)) else {
        return Verdict::Plain;
    };
    if !f.have_line || crate::incr::read_through(*offset, e, Some(new_text)) <= e.prefix {
        return Verdict::Plain;
    }
    let Some(at) = line_start(old_text, *offset as usize, &f.line) else {
        return Verdict::Stale;
    };
    // the change is inside the line (or after it): the line's start and
    // the end of the line before are unchanged
    if at as u64 >= e.prefix || at >= new_text.len() {
        return Verdict::Plain;
    }
    let (line, end) = line_at(new_text, at);
    // `input_ln` gives up on a line that reaches `buf_size - 1`
    if m.start as i64 + line.len() as i64 + 1 >= buf_size as i64 {
        return Verdict::Plain;
    }
    let (buf, _) = fill(|c| g.xord[c as usize], m.start, line, m.eol);
    let limit = m.start + buf.len() as i32 - 1;
    let w = m.loc + 4;
    if limit.min(w) != m.limit.min(w) {
        return Verdict::Plain;
    }
    let n = (m.limit.min(m.loc + 3) - m.start + 1).max(0) as usize;
    if buf.get(..n) != m.seen.get(..n) || m.seen.len() != n {
        return Verdict::Plain;
    }
    Verdict::Refill(Refill {
        path: path.clone(),
        at: at as u64,
        line: line.to_vec(),
        end: end as u64,
        old_end: *offset,
    })
}

/// The main file's input level (`input_stack[1]`, or `cur_input` when it is
/// the innermost).
fn main_level(g: &Globals) -> Option<crate::generated::types::in_state_record> {
    match g.input_ptr {
        p if p < 1 => None,
        1 => Some(g.cur_input),
        _ => Some(g.input_stack[1]),
    }
}

/// A mid-line restart point here, if this is one (see the module comment):
/// called by the hook just after an `\input` file closed back into the main
/// file.
pub fn here(g: &Globals) -> Option<MidLine> {
    let r = midline_here(g);
    if debug() {
        match &r {
            Ok(m) => eprintln!(
                "[midline] line {} column {}: taken",
                m.line,
                m.loc - m.start
            ),
            Err(why) => eprintln!("[midline] line {}: not here: {why}", g.line),
        }
    }
    r.ok()
}

/// `FLASHTEX_INCR_DEBUG`: say where a mid-line checkpoint is not taken.
fn debug() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("FLASHTEX_INCR_DEBUG").is_some())
}

fn midline_here(g: &Globals) -> Result<MidLine, &'static str> {
    if !enabled() || g.in_open != 1 || g.input_ptr < 1 {
        return Err("off, or not in the main file");
    }
    // every level above the main file a token list
    if g.input_ptr > 1
        && (g.cur_input.state_field != token_list
            || !(2..g.input_ptr as usize).all(|k| g.input_stack[k].state_field == token_list))
    {
        return Err("a level above the main file is not a token list");
    }
    let b = main_level(g).ok_or("no main level")?;
    let (start, loc, limit) = (b.start_field, b.loc_field, b.limit_field);
    if b.state_field == token_list || b.name_field <= 19 || b.index_field != 1 {
        return Err("the main level is not the main file");
    }
    if !(start <= loc && loc <= limit) {
        return Err("not in the middle of the line");
    }
    if g.first != limit + 1 {
        return Err("the buffer is in use above the line");
    }
    let r = g.main_read().ok_or("the line's read is not known")?;
    if r.line != g.line {
        return Err("the line's read is not known (another line)");
    }
    if r.prompted || r.shown {
        return Err("the line was shown");
    }
    let line = g.input_file[0]
        .read_line()
        .ok_or("the file's line is not at hand")?;
    let (buf, _) = fill(|c| g.xord[c as usize], start, line, r.eol);
    if buf.len() as i32 != limit - start + 1
        || !buf
            .iter()
            .enumerate()
            .all(|(i, &c)| g.buffer[start as usize + i] == c)
    {
        return Err("the buffer is not the file's line");
    }
    let hi = limit.min(loc + 3);
    Ok(MidLine {
        line: r.line,
        start,
        loc,
        limit,
        eol: r.eol,
        seen: (start..=hi).map(|i| g.buffer[i as usize]).collect(),
    })
}

impl Globals {
    /// What `input_ln` did with the main file's current line.
    pub fn main_read(&self) -> Option<MainRead> {
        self.layer_ref()?.main_read.get()
    }

    /// `input_ln` is about to read a line at `in_open = 1` in the
    /// preamble: if it is the main file's (§362, §538: the reading level is
    /// the main file's, not a `\read`'s or the terminal's), note what the
    /// read does with it ([`MainRead`]).
    pub fn midline_note_read(&mut self) {
        let c = self.cur_input;
        if c.state_field == token_list || c.name_field <= 19 || c.index_field != 1 {
            return;
        }
        let r = MainRead {
            line: self.line,
            eol: eqtb_int(self, end_line_char_code),
            prompted: eqtb_int(self, pausing_code) > 0 && self.interaction > nonstop_mode,
            shown: false,
        };
        self.layer().main_read.set(Some(r));
    }

    /// Input level `j` was printed with its line (`show_context`, a
    /// diagnostics frame): the main file's (`j = 1`) is then not a mid-line
    /// restart point's until its next line.
    pub fn midline_note_shown(&self, j: i32) {
        if j != 1 {
            return;
        }
        if let Some(l) = self.layer_ref() {
            if let Some(mut r) = l.main_read.get() {
                r.shown = true;
                l.main_read.set(Some(r));
            }
        }
    }

    /// Just after checkpoint `id` was restored: its main line's [`MainRead`]
    /// back if it is a mid-line point, then `refill` if any (see the module
    /// comment). `Err`: the file is not the text the refill was decided on.
    pub fn midline_after_restore(
        &mut self,
        id: u64,
        refill: Option<&Refill>,
    ) -> Result<(), String> {
        let Some(m) = self
            .layer()
            .midlines
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, m)| m.clone())
        else {
            return match refill {
                Some(_) => Err("no mid-line record".into()),
                None => Ok(()),
            };
        };
        self.layer().main_read.set(Some(MainRead {
            line: m.line,
            eol: m.eol,
            prompted: false,
            shown: false,
        }));
        let Some(f) = refill else { return Ok(()) };
        let b = main_level(self).ok_or("no main file level")?;
        if self.in_open != 1
            || b.index_field != 1
            || (b.start_field, b.loc_field, b.limit_field) != (m.start, m.loc, m.limit)
            || self.first != m.limit + 1
            || self.line != m.line
        {
            return Err("the restored state is not the mid-line point's".into());
        }
        let mut file = std::mem::take(&mut self.input_file[0]);
        let read = file.reread_line(&f.path, f.at);
        self.input_file[0] = file;
        let (line, end) = read?;
        if line != f.line || end != f.end {
            return Err(format!(
                "{}: the line at {} is not the one the restart was decided on",
                f.path, f.at
            ));
        }
        let (buf, nb) = fill(|c| self.xord[c as usize], m.start, &line, m.eol);
        // `input_ln`'s high-water mark (accounting: DESIGN.md §1.1)
        if !line.is_empty() {
            self.max_buf_stack = self.max_buf_stack.max(m.start + line.len() as i32);
        }
        for (i, &c) in buf.iter().enumerate() {
            self.buffer[m.start as usize + i] = c;
        }
        let limit = m.start + buf.len() as i32 - 1;
        let _ = nb;
        // Every buffer place used since the line was read (the package's
        // lines, from `first = limit + 1` on) moves with the line's end in
        // a run from the format: `last`, which the last `input_ln` set.
        if self.last >= m.limit {
            self.last += limit - m.limit;
        }
        self.first = limit + 1;
        if self.input_ptr == 1 {
            self.cur_input.limit_field = limit;
        } else {
            self.input_stack[1].limit_field = limit;
        }
        // the new line's mid-line checkpoint, at once (`midline_checkpoint`)
        self.request_midline_checkpoint();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_start_finds_the_line_a_reader_read_last() {
        let t = b"ab\ncd\r\nef\rgh";
        assert_eq!(line_start(t, 3, b"ab"), Some(0));
        assert_eq!(line_start(t, 7, b"cd"), Some(3));
        assert_eq!(line_start(t, 10, b"ef"), Some(7));
        assert_eq!(line_start(t, 12, b"gh"), Some(10));
        // another line, or a reader not at a line's end
        assert_eq!(line_start(t, 7, b"cx"), None);
        assert_eq!(line_start(t, 6, b"cd"), None);
        assert_eq!(line_start(t, 3, b"b"), None);
        // a CR whose LF the reader would have read with it
        assert_eq!(line_start(b"ab\r\ncd", 3, b"ab"), None);
        assert_eq!(line_start(b"ab\r\ncd", 4, b""), None);
    }

    #[test]
    fn line_at_reads_as_read_tex_line_does() {
        for t in [
            &b"x\\title{a}  \ny"[..],
            b"x\\title\r\ny",
            b"x\\t\rq",
            b"x\\title",
        ] {
            let (l, end) = line_at(t, 1);
            let mut r = std::io::BufReader::new(std::io::Cursor::new(&t[1..]));
            let mut line = vec![];
            assert!(crate::system::read_tex_line(&mut r, &mut line));
            use std::io::Seek;
            let pos = r.stream_position().unwrap() as usize + 1;
            assert_eq!((l, end), (&line[..], pos), "{t:?}");
        }
    }

    #[test]
    fn fill_strips_blanks_and_ends_with_the_endlinechar() {
        let id = |c: u8| c as i32;
        assert_eq!(fill(id, 5, b"ab  ", 13), (vec![97, 98, 13], 7));
        assert_eq!(fill(id, 5, b"ab \t", 13), (vec![97, 98, 32, 9, 13], 9));
        assert_eq!(fill(id, 5, b"ab", -1), (vec![97], 7));
        assert_eq!(fill(id, 5, b"   ", 13), (vec![13], 5));
        assert_eq!(fill(id, 5, b"", -1), (vec![], 5));
    }
}
