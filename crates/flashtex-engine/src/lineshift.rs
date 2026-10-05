//! DESIGN.md §5.3, rule (c), the line half: input line numbers shifted by an
//! edit's line delta, with every read of a line number into output or state
//! kept a barrier.
//!
//! An edit that adds or removes line breaks (a newline typed, a paragraph
//! split or joined) moves every later line of its file by the same number,
//! δ. Nothing else about the rest of the document changes, but TeX's state
//! remembers line numbers, so the old run's state after the edit never equals
//! the new run's, and the edit used to re-typeset every later page. This
//! module says where the state holds a line number and what each one may do.
//! Lines are numbered as the old file has them: below [`Shift::first`] the
//! edit moved nothing; from [`Shift::after`] on, every line moved by δ; the
//! lines between hold the edit and have no single new number.
//!
//! * `line` and `line_stack[k]`: input level *j*'s line (`line` for the
//!   innermost level, `line_stack[j+1]` below it), and `input_file[j]` names
//!   the file. The convergence test allows new = old + δ for a level that
//!   reads the edited file (test (a) has put it past the edit); the old
//!   run's later checkpoints get the same correction whenever one is
//!   restored ([`Shift::relocate`], through `incr::Reloc`).
//! * The line each open semantic level (`mode_line`), group (e-TeX's saved
//!   line) and conditional (`if_line`, the condition stack) began on, with
//!   the file it was read from (changes/lineshift.ch: `ls_nest_tag`,
//!   `ls_grp_tag`, `ls_cond_tag`, the reading level's SyncTeX tag). A line
//!   of the edited file from `after` on must be the old one + δ in the new
//!   state (`crate::iso` asks [`held_ok`]), any other must be equal; the
//!   old run's later checkpoints are corrected by the same rule.
//!   `pack_begin_line` and conditionals deeper than the tags reach have no
//!   file: they must be equal, and an old checkpoint where one may be a
//!   moved line is *dirty* ([`Rec::marks`]).
//! * SyncTeX's file tag and line in nodes (changes/synctex.ch): written by
//!   `get_node`, copied by `copy_node_list`, read by nothing (the `.synctex`
//!   file is not written). `crate::iso` compares the tag and not the line.
//! * Reads into output or state: the run's *journal* (changes/lineshift.ch)
//!   records every `\inputlineno` read and every line number printed, with
//!   the file where TeX knows it. An entry of the old run after the
//!   convergence point that may be a moved line is a barrier: the old run's
//!   pages are kept up to the last checkpoint before it, and the run goes on
//!   live from there.
//! * The exception: `\the\inputlineno` read by `scan_toks`'s own `\the` in
//!   the body of a definition (LaTeX's `\begin`:
//!   `\edef\@currenvline{\on@line}`) puts its digits into that definition's
//!   token list and nowhere else. The list is marked (a [`Taint`]); a use of
//!   it (expanded, compared, shown) is a journal entry then, and freeing it
//!   ends the mark. A checkpoint where a marked list that may hold a moved
//!   line is alive is dirty.
//!
//! A dirty checkpoint of the old run is never restored after the
//! convergence (its page is kept; `incr` drops the checkpoint). The
//! journal's entries since the checkpoint before, the marks and the live
//! taints are part of each checkpoint's host record ([`Rec`],
//! `ExtRecord::lines`).

use crate::generated::Globals;
use std::cell::{Cell, RefCell};

/// A line number read or printed: the SyncTeX tag of the file it counts
/// lines of (the opening of the file: changes/synctex.ch) when `known`, and
/// the number. `known` with tag 0: not a line of a source file (a
/// `\scantokens` pseudo file, the terminal), never moved by an edit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineRead {
    pub tag: i32,
    pub known: bool,
    pub line: i32,
}
crate::codec_struct!(LineRead { tag, known, line });

/// A token list holding the digits of `\inputlineno` reads: its reference
/// count's location and its first token's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Taint {
    pub head: i32,
    pub first: i32,
    pub reads: Vec<LineRead>,
}
crate::codec_struct!(Taint { head, first, reads });

/// A checkpoint's line state: the journal's entries since the checkpoint
/// before it (`here`: the run's reads and prints of line numbers between
/// the two), the largest line number of no known file the state holds
/// (`marks`), and the live taints. The journal lives in the records, an
/// interval each, so that it follows the checkpoints through every restore,
/// jump, pass and thinning (`Globals::retain_checkpoints` hands a dropped
/// checkpoint's entries to the next one kept).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rec {
    pub here: Vec<LineRead>,
    pub marks: i32,
    pub taints: Vec<Taint>,
}
crate::codec_struct!(Rec {
    here,
    marks,
    taints
});

#[derive(Default)]
struct St {
    /// The next `\inputlineno` read goes into the definition being scanned.
    confine: bool,
    /// The confined reads of the definition being scanned.
    def_reads: Vec<LineRead>,
    taints: Vec<Taint>,
    /// The journal's entries since the last checkpoint.
    events: Vec<LineRead>,
}

thread_local! {
    static ST: RefCell<St> = RefCell::new(St::default());
    /// `ST.taints.len()`, for the hooks on hot paths.
    static TAINTS: Cell<usize> = const { Cell::new(0) };
    /// `scan_toks`'s `\the` in a definition's body is about to call
    /// `the_toks` (`ls_the_begin`, taken by `ls_the_take`).
    static THE_DEF: Cell<bool> = const { Cell::new(false) };
    /// The definition being scanned holds confined reads (`def_reads`).
    static DEF_READS: Cell<bool> = const { Cell::new(false) };
}

fn with<R>(f: impl FnOnce(&mut St) -> R) -> R {
    ST.with(|s| f(&mut s.borrow_mut()))
}

/// A new job in this thread (`crate::pdftex::reset_state`).
pub fn reset() {
    with(|s| *s = St::default());
    TAINTS.with(|t| t.set(0));
    THE_DEF.with(|t| t.set(false));
    DEF_READS.with(|t| t.set(false));
}

/// This state's line record, for a checkpoint (or the record of a run's
/// end, or of a convergence point, which become checkpoints' records too):
/// the journal's entries since the last checkpoint go into it.
pub fn record(g: &Globals) -> Rec {
    let marks = marks(g);
    with(|s| Rec {
        here: std::mem::take(&mut s.events),
        marks,
        taints: s.taints.clone(),
    })
}

/// Put a checkpoint's line record back: nothing has been read since it.
pub fn restore(r: &Rec) {
    with(|s| {
        s.events.clear();
        s.taints = r.taints.clone();
        s.confine = false;
        s.def_reads.clear();
    });
    TAINTS.with(|t| t.set(r.taints.len()));
    THE_DEF.with(|t| t.set(false));
    DEF_READS.with(|t| t.set(false));
}

/// The largest line number the state holds without a file:
/// `pack_begin_line`, and the lines of conditionals nested deeper than
/// `ls_cond_size` (absolute values: a negative line is an alignment's).
pub fn marks(g: &Globals) -> i32 {
    let mut m = g.pack_begin_line.unsigned_abs();
    let d = g.ls_cond_depth;
    let size = ls_cond_size(g);
    if d > size {
        // the current conditional's line and those the nodes keep, from
        // the top (node j holds conditional d-1-j's)
        m = m.max(g.if_line.unsigned_abs());
        let mut p = g.cond_ptr;
        let mut k = d - 1;
        while p > 0 && k > size {
            let (Some(w), Some(l)) = (g.mem.get(p as usize), g.mem.get(p as usize + 1)) else {
                return i32::MAX;
            };
            m = m.max(l.int().unsigned_abs());
            p = w.hh_rh();
            k -= 1;
        }
    }
    m.min(i32::MAX as u32) as i32
}

fn ls_cond_size(g: &Globals) -> i32 {
    g.ls_cond_tag.len() as i32 - 1
}

// ---------------------------------------------------------------------------
// the shift of one edit

/// One edited file's line shift (line numbers as the old file has them):
/// lines from `after` on moved by `delta`; lines before `first` did not
/// move; the lines between hold the edit.
/// `tags`: the SyncTeX tags of every opening of the edited file so far
/// ([`Shift::with_tags`]; changes/lineshift.ch's `ls_tag_file`), which say
/// which lines are the file's: a level's, group's or conditional's line is
/// the file of the tag it was begun under, even once that opening is closed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shift {
    pub path: String,
    pub delta: i32,
    pub first: i32,
    pub after: i32,
    pub tags: Vec<i32>,
}

/// A [`Shift`] whose lines before the edit are not counted yet: the edit
/// (`Session::changes`) and the file as it is now, whose bytes before the
/// edit are the old ones.
#[derive(Clone, Debug)]
pub struct Pending {
    pub path: String,
    delta: i32,
    /// Where the count of the lines before the edit stops (see `of`).
    s: usize,
    /// Line ends in the edited bytes (old), from `s` on.
    mid: i64,
    /// The edit ends where a line starts.
    at_start: bool,
    /// Byte `s` of the old file is a line end.
    s_ends: bool,
    new: std::sync::Arc<Vec<u8>>,
}

impl Pending {
    /// Whether `o` is the same edit: the same bytes changed the same way,
    /// so that either one's shift is the other's. Two names of one file
    /// (a link, a case variant) give the same edit; two files that only
    /// look alike may too, and then dropping either shift drops nothing.
    pub fn same_edit(&self, o: &Pending) -> bool {
        (self.delta, self.s, self.mid, self.at_start, self.s_ends)
            == (o.delta, o.s, o.mid, o.at_start, o.s_ends)
            && (std::sync::Arc::ptr_eq(&self.new, &o.new) || *self.new == *o.new)
    }

    /// The shift, given that TeX's line at byte `at` of the file (just past
    /// a line end, before the edit) is `line`: what a level reading it has
    /// counted there ((0, 0): the file's start).
    pub fn resolve(self, line: i64, at: usize) -> Shift {
        // the line ends before byte `s`
        let before = if at <= self.s {
            line + line_ends(&self.new, at, self.s) as i64
        } else if at == self.s + 1 {
            // (byte `s` is a line end or not by the old byte after it)
            line - self.s_ends as i64
        } else {
            line - line_ends(&self.new, self.s, at) as i64
        };
        Shift {
            path: self.path,
            delta: self.delta,
            first: clamp(1 + before),
            // the first line that starts at or after the edit's end
            after: clamp(1 + before + self.mid + if self.at_start { 0 } else { 1 }),
            tags: vec![],
        }
    }
}

/// The line count and the byte offset where input level `j` of `g` (just
/// restored to checkpoint `rec`) reads `path`: (0, 0) when no level does.
pub fn level_at(g: &Globals, rec: &crate::checkpoint::ExtRecord, path: &str) -> (i64, usize) {
    for j in (1..=g.in_open.max(0)).rev() {
        if !level_file(g, j).is_some_and(|p| same_path(p, path)) {
            continue;
        }
        // the record's files: term_in, term_out, pool_file, log_file, then
        // input_file[1..] (`Globals::visit_files`)
        if let Some(crate::system::Stream::In { offset, .. }) =
            rec.files.get(3 + j as usize).map(|f| &f.stream)
        {
            return (level_line(g, j) as i64, *offset as usize);
        }
    }
    (0, 0)
}

/// Line ends in `b[lo..hi]` as `read_tex_line` counts them: a LF, a CR not
/// followed by a LF (the next byte looked at even past `hi`), a CR LF once.
pub fn line_ends(b: &[u8], lo: usize, hi: usize) -> usize {
    let hi = hi.min(b.len());
    if lo >= hi {
        return 0;
    }
    let s = &b[lo..hi];
    let lf = s.iter().filter(|&&c| c == b'\n').count();
    if !s.contains(&b'\r') {
        return lf;
    }
    let cr_alone = (lo..hi)
        .filter(|&i| b[i] == b'\r' && b.get(i + 1) != Some(&b'\n'))
        .count();
    lf + cr_alone
}

fn clamp(x: i64) -> i32 {
    x.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

impl Shift {
    /// The shift of `old` → `new`, whose bytes differ only in
    /// `prefix..old_end` (old) and `prefix..new_end` (new); `None` when the
    /// edit moves no line.
    pub fn of(
        path: &str,
        old: &[u8],
        new: &[u8],
        prefix: usize,
        old_end: usize,
        new_end: usize,
    ) -> Option<Shift> {
        let new = std::sync::Arc::new(new.to_vec());
        Some(Shift::pending(path, old, &new, prefix, old_end, new_end)?.resolve(0, 0))
    }

    /// [`Shift::of`] without counting the lines before the edit, which
    /// [`Pending::resolve`] counts from a place in the file whose line TeX
    /// knows (a 1,000-page source is 2-4 MB; the restart point is a page
    /// or less before the edit).
    pub fn pending(
        path: &str,
        old: &[u8],
        new: &std::sync::Arc<Vec<u8>>,
        prefix: usize,
        old_end: usize,
        new_end: usize,
    ) -> Option<Pending> {
        if old_end > old.len() || new_end > new.len() || prefix > old_end.min(new_end) {
            return None;
        }
        // A CR at prefix-1 is a line end or not by the byte after it, which
        // the edit may change: count from there. Every line end before it,
        // and every one from the edit's end on, is the same in both files.
        let s = prefix.saturating_sub(1);
        let delta = line_ends(new, s, new_end) as i64 - line_ends(old, s, old_end) as i64;
        if delta == 0 {
            return None;
        }
        Some(Pending {
            path: path.to_string(),
            delta: clamp(delta),
            s,
            mid: line_ends(old, s, old_end) as i64,
            at_start: old_end == 0 || line_ends(old, old_end - 1, old_end) == 1,
            s_ends: line_ends(old, s, s + 1) == 1,
            new: new.clone(),
        })
    }

    /// Whether `r` may be a line this shift moved: a line of the edited
    /// file (one of `tags`), or of a file TeX does not know, at or after
    /// `first`.
    pub fn moves(&self, r: &LineRead) -> bool {
        r.line != 0
            && r.line.unsigned_abs() as i64 >= self.first as i64
            && (!r.known || self.tags.contains(&r.tag))
    }

    /// This shift with `tags` set from the state `g`: every tag given to an
    /// opening of the edited file (`ls_tag_file`). `Err` when the tags have
    /// run past what `ls_tag_file` keeps.
    pub fn with_tags(&self, g: &Globals) -> Result<Shift, String> {
        let n = g.synctex_tag_counter.max(0);
        let size = g.ls_tag_file.len() as i32 - 1;
        if n > size {
            return Err(format!(
                "{n} files opened: their tags are not all kept ({size})"
            ));
        }
        let mut tags = vec![];
        for t in 1..=n {
            let s = g.ls_tag_file[t as usize];
            if s > 0 && s < g.str_ptr {
                let name = String::from_utf8_lossy(&g.str_bytes(s)).into_owned();
                if same_path(&name, &self.path) {
                    tags.push(t);
                }
            }
        }
        Ok(Shift {
            tags,
            ..self.clone()
        })
    }

    /// Whether a checkpoint with this line record holds a line of no known
    /// file that this shift may have moved, or a marked list that may: then
    /// restoring it cannot give the new numbering.
    pub fn dirty(&self, r: &Rec) -> bool {
        r.marks as i64 >= self.first as i64
            || r.taints
                .iter()
                .any(|t| t.reads.iter().any(|x| self.moves(x)))
    }

    /// Correct a restored state of the old run's (taken at or after the
    /// convergence point) to the new numbering. Every file level that
    /// reads the edited file is past the edit (the convergence test saw it
    /// there, and the old run read nothing of a changed file again), so its
    /// line moves by `delta`; so does every open level's, group's and
    /// conditional's line of that file from `after` on.
    pub fn relocate(&self, g: &mut Globals) {
        let d = self.delta;
        let tags = &self.tags;
        for j in 1..=g.in_open.max(0) {
            if level_file(g, j).is_none() || !level_tag(g, j).is_some_and(|t| tags.contains(&t)) {
                continue;
            }
            if j == g.in_open {
                g.line += d;
            } else if let Some(&x) = g.line_stack.get(j as usize) {
                // line_stack[j+1] is element j
                g.line_stack[j as usize] = x + d;
            }
        }
        if tags.is_empty() {
            return;
        }
        let moved = |tag: i32, v: i32| -> Option<i32> {
            (tags.contains(&tag) && v.unsigned_abs() as i64 >= self.after as i64)
                .then(|| shifted(v, d))
        };
        // the semantic levels
        let np = g.nest_ptr.max(0);
        for k in 0..np {
            let t = g.ls_nest_tag.get(k as usize).copied().unwrap_or(-1);
            let v = g.nest[k as usize].ml_field;
            if let Some(v) = moved(t, v) {
                g.nest[k as usize].ml_field = v;
            }
        }
        let t = g.ls_nest_tag.get(np as usize).copied().unwrap_or(-1);
        if let Some(v) = moved(t, g.cur_list.ml_field) {
            g.cur_list.ml_field = v;
        }
        // e-TeX's group lines, saved below each boundary
        if g.eTeX_mode == 1 {
            let mut b = g.cur_boundary;
            let mut lvl = g.cur_level;
            let mut steps = 0;
            while b > 0 && steps < 1 << 16 {
                let t = g.ls_grp_tag.get(lvl.max(0) as usize).copied().unwrap_or(-1);
                let v = g.save_stack[b as usize - 1].int();
                if let Some(v) = moved(t, v) {
                    g.save_stack[b as usize - 1].set_int(v);
                }
                b = g.save_stack[b as usize].hh_rh();
                lvl -= 1;
                steps += 1;
            }
        }
        // the conditionals: the current one's, then those the nodes keep
        // (node j from the top holds conditional depth-1-j's)
        let depth = g.ls_cond_depth;
        let tag = |g: &Globals, k: i32| -> i32 {
            if k >= 1 && k <= ls_cond_size(g) {
                g.ls_cond_tag[k as usize]
            } else {
                -1
            }
        };
        if let Some(v) = moved(tag(g, depth), g.if_line) {
            g.if_line = v;
        }
        let mut p = g.cond_ptr;
        let mut k = depth - 1;
        while p > 0 && k >= 1 {
            let t = tag(g, k);
            let v = g.mem[p as usize + 1].int();
            if let Some(v) = moved(t, v) {
                g.mem[p as usize + 1].set_int(v);
            }
            p = g.mem[p as usize].hh_rh();
            k -= 1;
        }
    }
}

/// A line `v` moved by `d` (a negative line is an alignment's: its
/// absolute value moves).
fn shifted(v: i32, d: i32) -> i32 {
    if v < 0 {
        v - d
    } else {
        v + d
    }
}

/// The same file named two ways (`./a.tex`, `a.tex`, its absolute name): the
/// names resolved against the working directory the run opens files from,
/// links followed where the file exists (cached per name).
pub fn same_path(a: &str, b: &str) -> bool {
    a == b || canonical(a) == canonical(b)
}

thread_local! {
    /// `canonical`'s answers, until `forget_paths`.
    static CANONICAL: RefCell<std::collections::HashMap<String, std::path::PathBuf>> =
        RefCell::new(std::collections::HashMap::new());
}

/// Forget every name's resolution (`same_path`): a compile resolves names
/// afresh, since a link may have become a directory, or the working
/// directory another job's, since the last.
pub fn forget_paths() {
    CANONICAL.with(|c| c.borrow_mut().clear());
}

fn canonical(p: &str) -> std::path::PathBuf {
    use CANONICAL as CACHE;
    if let Some(c) = CACHE.with(|c| c.borrow().get(p).cloned()) {
        return c;
    }
    let path = std::path::Path::new(p);
    let c = std::fs::canonicalize(path).unwrap_or_else(|_| {
        // (a name that does not resolve: its lexical form, made absolute)
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        };
        let mut out = std::path::PathBuf::new();
        for c in abs.components() {
            match c {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    out.pop();
                }
                c => out.push(c),
            }
        }
        out
    });
    CACHE.with(|m| m.borrow_mut().insert(p.to_string(), c.clone()));
    c
}

/// The source file input level `j` (1..=`in_open`) reads, as the run
/// opened it: `None` for a level that is not a file's (`\scantokens`, the
/// terminal, `\read`), which `begin_file_reading` marks with no full name.
pub fn level_file(g: &Globals, j: i32) -> Option<&str> {
    if j < 1 || j > g.in_open {
        return None;
    }
    if g.full_source_filename_stack
        .get(j as usize)
        .copied()
        .unwrap_or(0)
        == 0
    {
        return None;
    }
    g.input_file
        .get(j as usize - 1)
        .and_then(|f| f.opened_path())
}

/// The SyncTeX tag of input level `j`: its record's (the current one, or
/// the input stack's that is not a token list and has that index).
fn level_tag(g: &Globals, j: i32) -> Option<i32> {
    const TOKEN_LIST: i32 = 0;
    let is = |r: &crate::generated::types::in_state_record| {
        r.state_field != TOKEN_LIST && r.index_field == j
    };
    if is(&g.cur_input) {
        return Some(g.cur_input.synctex_tag_field);
    }
    (0..g.input_ptr.max(0) as usize)
        .rev()
        .filter_map(|k| g.input_stack.get(k))
        .find(|r| is(r))
        .map(|r| r.synctex_tag_field)
}

/// Level `j`'s line now.
fn level_line(g: &Globals, j: i32) -> i32 {
    if j == g.in_open {
        g.line
    } else {
        g.line_stack.get(j as usize).copied().unwrap_or(0)
    }
}

fn level_read(g: &Globals, j: i32) -> LineRead {
    LineRead {
        tag: if level_file(g, j).is_some() {
            level_tag(g, j).unwrap_or(0)
        } else {
            0
        },
        known: true,
        line: level_line(g, j),
    }
}

// ---------------------------------------------------------------------------
// the convergence test's view of the state

/// The shifts a convergence test allows, in order: those the old
/// checkpoint still owes (`incr::Reloc`: it is stored as an earlier run
/// numbered its lines, and corrected only when restored), then the edit's;
/// each with its tags ([`Shift::with_tags`]).
#[derive(Clone, Debug, Default)]
pub struct Active {
    stages: Vec<Shift>,
}

impl Active {
    pub fn new(shifts: &[Shift]) -> Active {
        Active {
            stages: shifts.to_vec(),
        }
    }

    /// The new numbering of a line `o` of the file whose opening had
    /// SyncTeX tag `tag`: each shift of that file moves it from its `after`
    /// on.
    fn held(&self, tag: i32, o: i32) -> i32 {
        let mut v = o;
        for s in &self.stages {
            if s.tags.contains(&tag) && v.unsigned_abs() as i64 >= s.after as i64 {
                v = shifted(v, s.delta);
            }
        }
        v
    }

    /// The shift of input level `j`'s line: every shift of the file it
    /// reads.
    fn level(&self, g: &Globals, j: i32) -> i64 {
        if level_file(g, j).is_none() {
            return 0;
        }
        let Some(t) = level_tag(g, j) else {
            return 0;
        };
        self.stages
            .iter()
            .filter(|s| s.tags.contains(&t))
            .map(|s| s.delta as i64)
            .sum()
    }
}

thread_local! {
    /// The shifts the convergence test in progress allows (`Obs::test`).
    static ACTIVE: RefCell<Active> = RefCell::new(Active::default());
}

/// Run `f` with `a` allowed by [`shifted_word`] and [`held_ok`].
pub fn with_active<R>(a: Active, f: impl FnOnce() -> R) -> R {
    ACTIVE.with(|x| *x.borrow_mut() = a);
    let r = f();
    ACTIVE.with(|x| *x.borrow_mut() = Active::default());
    r
}

/// Whether a line the old state holds as `o` and the new one as `n` is the
/// same line: `n` is `o` in the new numbering (for a line of an edited file,
/// its reading level's SyncTeX tag `tag`, from the shift's `after` on: moved
/// by δ, which it must be then; any other line: equal).
pub fn held_ok(tag: i32, o: i32, n: i32) -> bool {
    ACTIVE.with(|a| n == a.borrow().held(tag, o))
}

/// Whether a differing word of the live state (`new`) and the old run's
/// (`old`) holds only line numbers that differ as the shift says: `line`
/// and `line_stack` elements (dead above `in_open`: `begin_file_reading`
/// sets `line_stack[index]` before `end_file_reading` reads it, tex.web
/// §328-§329) by the shift of the file they count, `if_line` by
/// [`held_ok`].
pub fn shifted_word(
    g: &Globals,
    w: &crate::statediff::WordDiff,
    layout: &[crate::statediff::ScalarSlot],
) -> bool {
    ACTIVE.with(|a| {
        let a = a.borrow();
        if a.stages.is_empty() {
            return false;
        }
        let i32s = |x: u64| [x as u32 as i32, (x >> 32) as u32 as i32];
        if w.region == "line_stack" {
            let (o, n) = (i32s(w.old), i32s(w.new));
            for k in 0..2 {
                if o[k] == n[k] {
                    continue;
                }
                // element e is Pascal's line_stack[e+1]: level e's line
                let e = (w.index + k) as i32;
                if e >= g.in_open {
                    continue;
                }
                if n[k] as i64 != o[k] as i64 + a.level(g, e) {
                    return false;
                }
            }
            return true;
        }
        if w.region != "(scalars)" {
            return false;
        }
        // every scalar slot in the word: only `line` and `if_line` may
        // differ
        let (wo, wn) = (w.old.to_le_bytes(), w.new.to_le_bytes());
        let base = w.index & !7;
        for s in layout {
            if s.off + s.size <= base || s.off >= base + 8 {
                continue;
            }
            let lo = s.off.max(base) - base;
            let hi = (s.off + s.size).min(base + 8) - base;
            if wo[lo..hi] == wn[lo..hi] {
                continue;
            }
            if s.size != 4 || s.off < base || s.off + 4 > base + 8 {
                return false;
            }
            let (o, n) = (
                i32::from_le_bytes(wo[lo..hi].try_into().unwrap()),
                i32::from_le_bytes(wn[lo..hi].try_into().unwrap()),
            );
            let ok = match s.name {
                "line" => n as i64 == o as i64 + a.level(g, g.in_open),
                "if_line" => {
                    let k = g.ls_cond_depth;
                    let tag = if k >= 1 && k <= ls_cond_size(g) {
                        g.ls_cond_tag[k as usize]
                    } else {
                        -1
                    };
                    n == a.held(tag, o)
                }
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        true
    })
}

// ---------------------------------------------------------------------------
// the hooks (changes/lineshift.ch)

impl Globals {
    /// `\inputlineno` was read (`cur_val:=line`).
    pub fn ls_line_read(&mut self) {
        let r = level_read(self, self.in_open);
        let confined = with(|s| {
            if std::mem::take(&mut s.confine) {
                s.def_reads.push(r);
                true
            } else {
                s.events.push(r);
                false
            }
        });
        if confined {
            DEF_READS.with(|t| t.set(true));
        }
    }

    /// `scan_toks`'s `\the` is about to call `the_toks`; `d`: in the body
    /// of a definition.
    #[inline(always)]
    pub fn ls_the_begin(&mut self, d: bool) {
        THE_DEF.with(|t| t.set(d));
    }

    /// `the_toks` starts: whether it is `scan_toks`'s `\the` in the body of
    /// a definition (and no `the_toks` nested inside it).
    #[inline(always)]
    pub fn ls_the_take(&mut self) -> bool {
        THE_DEF.with(|t| t.replace(false))
    }

    /// That `the_toks` has its token: `\inputlineno` itself, read next by
    /// its `scan_something_internal` and printed into the definition.
    pub fn ls_the_direct(&mut self) {
        use crate::generated::consts::{input_line_no_code, last_item};
        if self.cur_cmd == last_item && self.cur_chr == input_line_no_code {
            with(|s| s.confine = true);
        }
    }

    /// `scan_toks` is done; `d`: a definition, whose list is `def_ref`.
    pub fn ls_toks_done(&mut self, d: bool) {
        let reads = with(|s| {
            s.confine = false;
            std::mem::take(&mut s.def_reads)
        });
        DEF_READS.with(|t| t.set(false));
        if reads.is_empty() {
            return;
        }
        if !d {
            // (cannot happen: only a definition confines a read)
            with(|s| s.events.extend(reads));
            return;
        }
        let head = self.def_ref;
        let first = self.mem.get(head as usize).map_or(0, |w| w.hh_rh());
        let n = with(|s| {
            s.taints.push(Taint { head, first, reads });
            s.taints.len()
        });
        TAINTS.with(|t| t.set(n));
    }

    /// Token list `p` (its reference count) is expanded or compared.
    #[inline(always)]
    pub fn ls_use(&mut self, p: i32) {
        if TAINTS.with(|t| t.get()) != 0 {
            self.ls_use_slow(p, false);
        }
    }

    /// `show_token_list(p)`: `p` a list's first token (or its head). While
    /// a definition holding confined reads is being scanned, any list shown
    /// may be its unfinished body (`runaway`, tex.web §306): its reads are
    /// journalled.
    #[inline(always)]
    pub fn ls_show(&mut self, p: i32) {
        if DEF_READS.with(|t| t.get()) {
            with(|s| {
                let r = s.def_reads.clone();
                s.events.extend(r)
            });
        }
        if TAINTS.with(|t| t.get()) != 0 {
            self.ls_use_slow(p, true);
        }
    }

    #[inline(never)]
    fn ls_use_slow(&mut self, p: i32, first_too: bool) {
        if p <= 0 {
            return;
        }
        let used = with(|s| {
            let mut used = false;
            for t in &s.taints {
                if t.head == p || (first_too && t.first == p) {
                    s.events.extend(t.reads.iter().cloned());
                    used = true;
                }
            }
            used
        });
        // a recording intrinsic would replay the use without these hooks
        if used && self.intr_rec_on {
            self.flashtex_intr_abort(0);
        }
    }

    /// Token list `p` (its reference count) is freed.
    #[inline(always)]
    pub fn ls_free(&mut self, p: i32) {
        if TAINTS.with(|t| t.get()) != 0 {
            let n = with(|s| {
                s.taints.retain(|t| t.head != p);
                s.taints.len()
            });
            TAINTS.with(|t| t.set(n));
        }
    }

    /// File level `j`'s line is printed.
    pub fn ls_print_level(&mut self, j: i32) {
        let r = level_read(self, j);
        with(|s| s.events.push(r));
    }

    /// A line number `v` of a file TeX does not know is printed.
    pub fn ls_print_unknown(&mut self, v: i32) {
        if v != 0 {
            with(|s| {
                s.events.push(LineRead {
                    tag: 0,
                    known: false,
                    line: v,
                })
            });
        }
    }

    /// A box report prints `pack_begin_line` (when not 0) and `line`.
    pub fn ls_box_lines(&mut self) {
        let v = self.pack_begin_line;
        self.ls_print_unknown(v);
        let j = self.in_open;
        self.ls_print_level(j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ends_count_as_tex_reads_lines() {
        assert_eq!(line_ends(b"a\nb\r\nc\rd", 0, 8), 3);
        // a CR LF split by the range's end: the CR is not a line end
        assert_eq!(line_ends(b"a\r\nb", 0, 2), 0);
        assert_eq!(line_ends(b"a\r\nb", 0, 3), 1);
        assert_eq!(line_ends(b"a\rb", 0, 2), 1);
    }

    #[test]
    fn shift_of_edits() {
        // a space becomes a line break in line 2: line 2 holds the edit,
        // line 3 on moved by one
        let old = b"x\nfoo bar\nbaz\n";
        let new = b"x\nfoo\nbar\nbaz\n";
        let s = Shift::of("f", old, new, 5, 6, 6).unwrap();
        assert_eq!((s.delta, s.first, s.after), (1, 2, 3));
        // a blank line (paragraph split)
        let new2 = b"x\nfoo\n\nbar\nbaz\n";
        let s = Shift::of("f", old, new2, 5, 6, 7).unwrap();
        assert_eq!(s.delta, 2);
        // a letter: no shift
        assert!(Shift::of("f", old, b"x\nfoO bar\nbaz\n", 4, 5, 5).is_none());
        // a join (line break becomes a space): line 3 of the old file holds
        // it, line 4 moved back
        let s = Shift::of("f", new, old, 5, 6, 6).unwrap();
        assert_eq!((s.delta, s.first, s.after), (-1, 2, 3));
        // a line inserted between lines: the next line starts at the
        // edit's end and moves whole
        let s = Shift::of("f", b"a\nb\n", b"a\nx\nb\n", 2, 2, 4).unwrap();
        assert_eq!((s.delta, s.first, s.after), (1, 1, 2));
        // CR LF: typing an LF after a CR changes nothing
        assert!(Shift::of("f", b"a\rb", b"a\r\nb", 2, 2, 3).is_none());
        // ... a CR typed before an LF neither
        assert!(Shift::of("f", b"a\nb", b"a\r\nb", 1, 1, 2).is_none());
    }

    #[test]
    fn pending_shifts_count_from_any_line_start() {
        // every line start up to the edit gives what a count from the file's
        // start gives, CR, LF and CR LF alike, an edit after a CR included
        let old: &[u8] = b"a\nb\r\nc\rd e f\ng\r\nh\n";
        for (prefix, old_end, ins) in [(9, 10, &b"\n"[..]), (7, 7, b"\n\n"), (8, 8, b"\n")] {
            let mut new = old[..prefix].to_vec();
            new.extend_from_slice(ins);
            new.extend_from_slice(&old[old_end..]);
            let new_end = prefix + ins.len();
            let whole = Shift::of("f", old, &new, prefix, old_end, new_end).unwrap();
            let arc = std::sync::Arc::new(new.clone());
            for x in 0..=prefix {
                let starts = x == 0 || line_ends(old, x - 1, x) == 1;
                if !starts {
                    continue;
                }
                let line = line_ends(old, 0, x) as i64;
                let p = Shift::pending("f", old, &arc, prefix, old_end, new_end).unwrap();
                assert_eq!(
                    p.resolve(line, x),
                    whole,
                    "edit at {prefix}, counted from {x}"
                );
            }
        }
    }

    #[test]
    fn moved_reads() {
        let s = Shift {
            path: "./main.tex".into(),
            delta: 1,
            first: 10,
            after: 11,
            tags: vec![1, 7],
        };
        let r = |tag, known, line| LineRead { tag, known, line };
        assert!(s.moves(&r(1, true, 10)));
        assert!(s.moves(&r(7, true, 12)));
        assert!(!s.moves(&r(1, true, 9)));
        assert!(!s.moves(&r(3, true, 50)));
        assert!(!s.moves(&r(0, true, 50)));
        assert!(s.moves(&r(0, false, -50)));
        assert!(!s.moves(&r(0, false, 0)));
    }

    #[test]
    fn held_lines() {
        let sh = |delta, after| Shift {
            path: "a.tex".into(),
            delta,
            first: after - 1,
            after,
            tags: vec![3],
        };
        let a = Active {
            stages: vec![sh(2, 11)],
        };
        with_active(a, || {
            assert!(held_ok(3, 12, 14));
            assert!(held_ok(3, -12, -14));
            assert!(!held_ok(3, 12, 12));
            assert!(held_ok(3, 10, 10));
            assert!(!held_ok(3, 10, 12));
            assert!(held_ok(4, 12, 12));
            assert!(!held_ok(4, 12, 14));
        });
        assert!(!held_ok(3, 12, 14));
        // a checkpoint that owes a shift (+1 from line 20 on), then the
        // edit's (-1 from line 31 on, in the numbering after the first)
        let a = Active {
            stages: vec![sh(1, 20), sh(-1, 31)],
        };
        with_active(a, || {
            assert!(held_ok(3, 19, 19));
            assert!(held_ok(3, 25, 26));
            assert!(held_ok(3, 30, 30));
            assert!(held_ok(3, 40, 40));
        });
    }

    #[test]
    fn same_paths() {
        assert!(same_path("./a.tex", "a.tex"));
        let cwd = std::env::current_dir().unwrap();
        let abs = cwd.join("a.tex");
        assert!(same_path(abs.to_str().unwrap(), "./a.tex"));
        assert!(same_path("sub/../a.tex", "a.tex"));
        // a namesake elsewhere is another file
        let other = cwd.join("sub").join("a.tex");
        assert!(!same_path(other.to_str().unwrap(), "a.tex"));
        assert!(!same_path("/x/ya.tex", "a.tex"));
        assert!(!same_path("b.tex", "a.tex"));
        assert!(!same_path("chap/a.tex", "a.tex"));
    }
}
