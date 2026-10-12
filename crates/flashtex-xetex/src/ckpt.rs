//! Checkpoints of a resident engine's run (lane XETEX-INCR;
//! docs/design/xetex/INCREMENTAL.md §4, DESIGN.md §5.1): the begin-document
//! snapshot S₀, taken where `changes/checkpoint.ch` asks
//! (`flashtex_checkpoint_hook`), and what a restore of it puts back.
//!
//! A checkpoint is the word space (with the scalar globals spilled into
//! it) and the handle table, protrusion codes and font manager saved with it
//! (`state.rs`'s `Globals::checkpoint`), plus a [`Record`] of the rest:
//! where every file global's stream stands (`system::FileSnap`: an input
//! file's next byte, an output file's bytes so far, which a restore writes
//! back), the captured terminal, the run's other state outside the word
//! space ([`RunState`]), how many external effects (`\write18`) the run
//! had made, and each font's layout engine's shaping state: the engines
//! (HarfBuzz and FreeType objects) are shared with the live run, whose
//! caches change no answer (`tests`), but whose last shaper and script
//! `usingOpenType`, `usingGraphite` and `getDefaultDirection` read.
//!
//! **S₀** is taken at the first `big_switch` after `\document`'s body has
//! been consumed, as the pdfTeX engine takes it (`crate::resident` arms it
//! for the runs that may take one). Its key is the pdfTeX engine's
//! (`flashtex_engine::host::Key`, checked by the same code): every file the
//! run read before it (content hash and stat signature, font files
//! included), every lookup, the prefix of each input file still open, the
//! files it wrote and closed, the clock, the first line, the engine build;
//! and, when the run looked a font up by name, the font directories, which
//! decide what the fonts' catalog finds ([`S0::font_dirs`]). The run's
//! output must not have begun: a page shipped before S₀ (whose display list
//! and PDF records a restore would have to bring back) means no S₀.

use crate::generated::consts::{hash_base, hash_prime};
use crate::generated::Globals;
use crate::system::{AlphaFile, ByteFile, FileSnap, FileVisit, Stream, WordFile};
use flashtex_engine::arena::CheckpointId;
use flashtex_engine::host::Key;
use flashtex_engine::system::{ReadLog, StatSig};

/// `ckpt_request`: find `\document` and arm S₀ (the run's first
/// `big_switch`).
pub const REQ_LOOKUP: i32 = 1;
/// `ckpt_request`: take S₀ (`changes/checkpoint.ch`'s `pop_input`).
pub const REQ_S0: i32 = 2;
/// `ckpt_request` (and `ckpt_on_shipout`): a page checkpoint, after a
/// shipout.
pub const REQ_PAGE: i32 = 3;

/// The output's state at a page checkpoint: the document (its pages and
/// forms shared, `out::doc::Doc`) and its PDF file's name, if open.
#[derive(Clone)]
pub struct OutSnap {
    pub doc: crate::out::doc::Doc,
    pub pdf_name: String,
    pub pdf_open: bool,
}

/// A checkpoint taken after a page shipped (DESIGN.md §5.2): a later run
/// whose edit comes after what it had read restarts here
/// (`crate::resident`).
pub struct PageCk {
    pub id: CheckpointId,
    /// The pages shipped by then.
    pub pages: usize,
    pub rec: Record,
    pub out: Option<OutSnap>,
    /// The read journal's lengths then (files, lookups, outputs).
    pub reads: (usize, usize, usize),
    /// How many output opens the run had made (`Resident::opens`).
    pub opens: usize,
}

/// The run's state outside the word space that a checkpoint keeps besides
/// what `state.rs` saves with the word space.
#[derive(Clone, Debug, PartialEq)]
pub struct RunState {
    start: Option<(i64, Vec<u8>)>,
    terminating: bool,
    tex_input_type: bool,
    req_engine: u8,
    saved_mapping_name: Option<Vec<u8>>,
    pictures: std::collections::BTreeSet<String>,
}

impl RunState {
    pub fn of(g: &Globals) -> RunState {
        let h = &g.host;
        RunState {
            start: h.start.clone(),
            terminating: h.terminating,
            tex_input_type: h.tex_input_type,
            req_engine: h.req_engine,
            saved_mapping_name: h.saved_mapping_name.clone(),
            pictures: h.pictures.clone(),
        }
    }

    pub fn put(&self, g: &mut Globals) {
        let h = &mut g.host;
        h.start = self.start.clone();
        h.terminating = self.terminating;
        h.tex_input_type = self.tex_input_type;
        h.req_engine = self.req_engine;
        h.saved_mapping_name = self.saved_mapping_name.clone();
        h.pictures = self.pictures.clone();
        h.first_line = None;
    }
}

/// The state outside the word space at a checkpoint.
#[derive(Clone, Debug)]
pub struct Record {
    /// Every file global, in `visit_files` order.
    pub files: Vec<FileSnap>,
    /// What the run had written to its (captured) terminal: S₀ keeps the
    /// bytes; a page checkpoint their number only (the last run's terminal
    /// starts with them: `restore_record`'s `base`).
    pub terminal: Vec<u8>,
    pub terminal_len: usize,
    pub run: RunState,
    /// `flashtex_engine::system::external_effects_len()`.
    pub effects: usize,
    /// Each layout engine's shaping state (`LayoutEngine::shaping_state`),
    /// by handle: the engines are shared with the live run, which shapes
    /// on after the checkpoint.
    pub engines: Vec<(i32, (Option<String>, u32))>,
}

/// S₀ as a run took it, before its key is made (`crate::resident`).
pub struct Taken {
    pub id: CheckpointId,
    pub rec: Record,
    /// The read journal up to S₀.
    pub reads: ReadLog,
    /// The files the run wrote and closed before S₀, with their contents.
    pub written: Vec<(String, Vec<u8>)>,
    /// The job name.
    pub job_name: String,
    /// The font directories' state, when the run looked a font up by name.
    pub font_dirs: Option<[u64; 2]>,
    /// How many output opens the run had made (`Resident::opens`).
    pub opens: usize,
}

/// S₀ of a resident engine: the checkpoint, its record and its key.
pub struct S0 {
    pub id: CheckpointId,
    pub rec: Record,
    pub key: Key,
    /// The font directories' state (`resident::Fonts::dirs`) when S₀ was
    /// taken, if the run looked a font up by name before it: a font added,
    /// removed or renamed changes what a name finds.
    pub font_dirs: Option<[u64; 2]>,
    /// The read journal's lengths at S₀ (files, lookups, outputs).
    pub reads: (usize, usize, usize),
    /// How many output opens the run had made by then.
    pub opens: usize,
    /// The run made the display list's source map (`crate::dlsource`): a
    /// run that makes it cannot go on from an S₀ whose nodes have no
    /// positions, nor the other way.
    pub dl_on: bool,
}

struct SnapFiles {
    out: Vec<FileSnap>,
    err: Option<String>,
    /// Keep the output files' bytes (S₀), not only their lengths.
    keep: bool,
}

impl SnapFiles {
    fn push(&mut self, r: Result<FileSnap, String>) {
        match r {
            Ok(s) => self.out.push(s),
            Err(e) => {
                self.err.get_or_insert(e);
            }
        }
    }
}

impl FileVisit for SnapFiles {
    fn alpha(&mut self, f: &mut AlphaFile) {
        let r = f.snapshot(self.keep);
        self.push(r)
    }
    fn byte(&mut self, f: &mut ByteFile) {
        let r = f.snapshot(self.keep);
        self.push(r)
    }
    fn word(&mut self, f: &mut WordFile) {
        let r = f.snapshot(self.keep);
        self.push(r)
    }
}

struct RestoreFiles<'a> {
    snaps: &'a [FileSnap],
    i: usize,
    err: Option<String>,
}

impl RestoreFiles<'_> {
    fn next(&mut self) -> Option<&FileSnap> {
        self.i += 1;
        self.snaps.get(self.i - 1)
    }
    fn note(&mut self, r: Result<(), String>) {
        if let Err(e) = r {
            self.err.get_or_insert(e);
        }
    }
}

impl FileVisit for RestoreFiles<'_> {
    fn alpha(&mut self, f: &mut AlphaFile) {
        let r = match self.next().cloned() {
            Some(s) => f.restore(&s),
            None => Err("the record has fewer files".into()),
        };
        self.note(r)
    }
    fn byte(&mut self, f: &mut ByteFile) {
        let r = match self.next().cloned() {
            Some(s) => f.restore(&s),
            None => Err("the record has fewer files".into()),
        };
        self.note(r)
    }
    fn word(&mut self, f: &mut WordFile) {
        let r = match self.next().cloned() {
            Some(s) => f.restore(&s),
            None => Err("the record has fewer files".into()),
        };
        self.note(r)
    }
}

impl Globals {
    /// `changes/checkpoint.ch`: at `big_switch` while `ckpt_request` is
    /// nonzero.
    pub fn flashtex_checkpoint_hook(&mut self) {
        let req = std::mem::replace(&mut self.ckpt_request, 0);
        match req {
            REQ_LOOKUP => {
                if let Some(cs) = self.find_cs(b"document") {
                    self.ckpt_arm_cs = cs;
                }
            }
            REQ_S0 => self.take_s0(),
            REQ_PAGE => self.take_page(),
            _ => {}
        }
    }

    /// The control sequence named `name` (letters), without entering it
    /// (`id_lookup`'s search, read only).
    pub fn find_cs(&self, name: &[u8]) -> Option<i32> {
        let mut h: i32 = 0;
        for &c in name {
            h = h + h + c as i32;
            while h >= hash_prime {
                h -= hash_prime;
            }
        }
        let mut p = h + hash_base;
        loop {
            let e = self.hash[(p - hash_base) as usize];
            let t = e.rh();
            if t > 0 && self.str_is(t, name) {
                return Some(p);
            }
            if e.lh() == 0 {
                return None;
            }
            p = e.lh();
        }
    }

    /// Whether string `s` (a pool string, past the 65536 single
    /// characters) is `name`.
    fn str_is(&self, s: i32, name: &[u8]) -> bool {
        const FIRST: i32 = 65536;
        if s < FIRST || s >= self.str_ptr {
            return false;
        }
        let a = self.str_start[(s - FIRST) as usize] as usize;
        let b = self.str_start[(s + 1 - FIRST) as usize] as usize;
        b - a == name.len() && (a..b).zip(name).all(|(k, &c)| self.str_pool[k] == c as i32)
    }

    /// The state outside the word space now (every output stream flushed
    /// first). `Err`: no checkpoint can be taken here.
    pub fn capture_record(&mut self, keep: bool) -> Result<Record, String> {
        let mut v = SnapFiles {
            out: vec![],
            err: None,
            keep,
        };
        self.visit_files(&mut v);
        if let Some(e) = v.err {
            return Err(e);
        }
        let engines = self
            .host
            .handles
            .objects()
            .filter_map(|(h, o)| match o {
                crate::state::Object::Other(a) => a
                    .downcast_ref::<crate::native::layout::LayoutEngine>()
                    .map(|e| (h, e.shaping_state())),
                _ => None,
            })
            .collect();
        Ok(Record {
            files: v.out,
            terminal: if keep {
                self.term_out.captured_bytes().to_vec()
            } else {
                vec![]
            },
            terminal_len: self.term_out.captured_bytes().len(),
            run: RunState::of(self),
            effects: flashtex_engine::system::external_effects_len(),
            engines,
        })
    }

    /// Put the state outside the word space back as `rec` has it; a record
    /// that kept only its terminal's length takes that much of `base`.
    pub fn restore_record(&mut self, rec: &Record, base: Option<&[u8]>) -> Result<(), String> {
        let mut v = RestoreFiles {
            snaps: &rec.files,
            i: 0,
            err: None,
        };
        self.visit_files(&mut v);
        if let Some(e) = v.err {
            return Err(format!("cannot restore the files: {e}"));
        }
        let terminal = if rec.terminal.len() == rec.terminal_len {
            rec.terminal.clone()
        } else {
            match base.and_then(|b| b.get(..rec.terminal_len)) {
                Some(b) => b.to_vec(),
                None => return Err("the terminal before the checkpoint is not kept".into()),
            }
        };
        self.term_out.set_captured(terminal);
        rec.run.put(self);
        for (h, st) in &rec.engines {
            if let Some(e) = self.layout_engine(*h) {
                e.set_shaping_state(st);
            }
        }
        flashtex_engine::system::truncate_external_effects(rec.effects);
        Ok(())
    }

    /// Take a page checkpoint here, if this run is to: after S₀, which
    /// this run took or started from (`crate::resident`). A checkpoint that
    /// cannot be taken here (a pipe is open) is skipped: the next page's
    /// may be.
    fn take_page(&mut self) {
        if !self.host.resident.as_ref().is_some_and(|r| r.pages_on) {
            return;
        }
        let out = self.host.out.as_ref().map(|o| OutSnap {
            doc: o.doc.clone(),
            pdf_name: o.pdf_name.clone(),
            pdf_open: o.pdf.is_some(),
        });
        let Ok(rec) = self.capture_record(false) else {
            return;
        };
        let reads = flashtex_engine::system::reads_len();
        let pages = self.total_pages.max(0) as usize;
        let id = self.checkpoint();
        if let Some(r) = self.host.resident.as_mut() {
            let opens = r.opens.len();
            r.pages.push(PageCk {
                id,
                pages,
                rec,
                out,
                reads,
                opens,
            });
        }
        self.conv_test(pages);
        self.check_mem(pages);
    }

    /// Take S₀ here, if this run is to (`crate::resident`).
    fn take_s0(&mut self) {
        if !self.host.resident.as_ref().is_some_and(|r| r.want_s0) {
            return;
        }
        let r = self.s0_here();
        if let Some(res) = self.host.resident.as_mut() {
            // pages from here on get checkpoints
            res.pages_on |= r.is_ok();
            res.s0 = Some(r);
        }
    }

    fn s0_here(&mut self) -> Result<Taken, String> {
        // the output has not begun: nothing of it to restore
        if let Some(o) = self.host.out.as_ref() {
            if !o.doc.pages.is_empty() || o.pdf.is_some() {
                return Err("a page was shipped before the document's start".into());
            }
        }
        let rec = self.capture_record(true)?;
        let reads =
            flashtex_engine::system::reads_so_far().ok_or("the run's reads are not recorded")?;
        if let Some(b) = reads.barriers.first() {
            return Err(format!("the preamble ran an external command ({b})"));
        }
        // the files written and closed before S₀, as they are now
        let open: Vec<&str> = rec
            .files
            .iter()
            .filter_map(|f| match &f.stream {
                Stream::Out { path, .. } => Some(path.as_str()),
                _ => None,
            })
            .collect();
        let mut written = vec![];
        for p in &reads.outputs {
            if !open.contains(&p.as_str()) {
                let d = std::fs::read(p).map_err(|e| format!("{p}: {e}"))?;
                written.push((p.clone(), d));
            }
        }
        let job_name = if self.job_name > 0 {
            String::from_utf8_lossy(&crate::system::tex_string_utf8(self, self.job_name))
                .into_owned()
        } else {
            String::new()
        };
        let named = self.host.font_mgr.is_some();
        let font_dirs = match (named, self.host.resident.as_mut()) {
            (true, Some(r)) => Some(r.font_dirs()),
            _ => None,
        };
        let opens = self.host.resident.as_ref().map_or(0, |r| r.opens.len());
        let id = self.checkpoint();
        Ok(Taken {
            id,
            rec,
            reads,
            written,
            job_name,
            font_dirs,
            opens,
        })
    }
}

/// The index of a format in `Format::all()` (the key's lookups).
fn format_index(f: flashtex_engine::resolver::Format) -> u8 {
    flashtex_engine::resolver::Format::all()
        .iter()
        .position(|&x| x == f)
        .unwrap_or(0) as u8
}

/// S₀'s key (the pdfTeX engine's `host::make_key`, for this engine's
/// record): `clock` is the session's, `first_line` the run's.
pub fn make_key(t: &Taken, clock: (i64, i32), first_line: &[u8]) -> Result<Key, String> {
    let mut prefixes = vec![];
    let mut open = vec![];
    for f in &t.rec.files {
        if let Stream::In { path, offset } = &f.stream {
            let (h, n) = flashtex_engine::persist::hash128_file(path, Some(*offset))
                .map_err(|e| format!("{path}: {e}"))?;
            if n < *offset {
                return Err(format!("{path} is shorter than the {offset} bytes read"));
            }
            prefixes.push((
                path.clone(),
                *offset,
                h,
                StatSig::of(path).unwrap_or_default(),
            ));
            open.push(path.clone());
        }
    }
    let files = t
        .reads
        .files
        .iter()
        .filter(|f| !open.contains(&f.path) || f.closed_at == Some(u64::MAX))
        .map(|f| {
            let hash = match (&f.content, f.hash) {
                (Some(c), [0, 0]) => flashtex_engine::persist::hash128(c),
                (_, h) => h,
            };
            let hash = match f.stamp {
                Some(s) => [
                    hash[0] ^ (s as u64).rotate_left(17) ^ 0x5354_414d_5000_0000,
                    hash[1],
                ],
                None => hash,
            };
            (f.path.clone(), hash, f.stat)
        })
        .collect();
    let mut seen = std::collections::HashSet::new();
    let lookups = t
        .reads
        .lookups
        .iter()
        .map(|l| {
            (
                l.name.clone(),
                format_index(l.format),
                l.must_exist,
                l.found.clone(),
            )
        })
        .filter(|k| seen.insert(k.clone()))
        .collect();
    let fx = flashtex_engine::system::external_effects();
    Ok(Key {
        build: flashtex_engine::host::engine_build(),
        clock,
        source_date_epoch: std::env::var("SOURCE_DATE_EPOCH").ok(),
        force_source_date: std::env::var("FORCE_SOURCE_DATE").ok(),
        first_line: first_line.to_vec(),
        job_name: t.job_name.clone(),
        files,
        prefixes,
        lookups,
        barriers: (0..t.rec.effects)
            .map(|i| fx.get(i).map_or("external", |e| e.kind).to_string())
            .collect(),
        written: t.written.clone(),
        dirs: t.reads.dirs.clone(),
        arm: vec![],
    })
}

#[cfg(test)]
mod tests {
    //! INCREMENTAL.md §4's two beliefs, tested: the objects a checkpoint
    //! shares with the live run (a font's layout engine with its HarfBuzz
    //! and FreeType objects, a TECkit converter) answer alike whatever was
    //! asked of them in between -- their caches change no answer -- and
    //! the state they keep that later answers do read (the last shaping's
    //! shaper and script) is what a checkpoint's record keeps and a
    //! restore puts back.
    use super::*;
    use crate::native::font_inst::{FontInst, FtLibrary};
    use crate::native::layout::LayoutEngine;
    use crate::state::Object;
    use flashtex_engine::resolver::Format;
    use std::sync::Arc;

    fn u16s(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn engine() -> Option<LayoutEngine> {
        let path = flashtex_engine::system::find_file("lmroman10-regular.otf", Format::OpenType)?;
        let lib = FtLibrary::new();
        let font = FontInst::new(&lib, path.as_bytes(), 0, 10.0, &mut |_| None)?;
        Some(LayoutEngine::new(
            font,
            u32::from_be_bytes(*b"latn"),
            None,
            vec![],
            None,
            0xFF,
            0.0,
            0.0,
            0.0,
            b'O',
        ))
    }

    type Shaped = (Vec<u32>, Vec<f32>, Vec<(f32, f32)>);

    fn shape(e: &LayoutEngine, s: &str) -> Shaped {
        let u = u16s(s);
        e.layout_chars(&u, 0, u.len(), false);
        (e.glyphs(), e.glyph_advances(), e.glyph_positions())
    }

    #[test]
    fn a_layout_engine_answers_alike_after_other_shaping() {
        let Some(e) = engine() else {
            eprintln!("skipped: no lmroman10-regular.otf (TeX Live)");
            return;
        };
        let word = "office affluent flow — “naïve” Ťŕ ffi 1234";
        let first = shape(&e, word);
        let bounds: Vec<_> = first.0.iter().map(|&g| e.glyph_bounds(g)).collect();
        for other in [
            "The quick brown fox jumps over the lazy dog",
            "ΑΒΓ αβγ — Ελληνικά",
            "ﬁﬂ fi fl ffl ffi",
            "0123456789 +−×÷ ½ ¾",
            "",
            "Ab\u{0301}c\u{0327} combining",
        ] {
            shape(&e, other);
            for g in 0..300 {
                e.glyph_bounds(g);
                e.glyph_width(g);
            }
        }
        assert_eq!(
            shape(&e, word),
            first,
            "shaping depends on what came before"
        );
        let again: Vec<_> = first.0.iter().map(|&g| e.glyph_bounds(g)).collect();
        assert_eq!(again, bounds);
    }

    #[test]
    fn teckit_mappings_answer_alike_after_other_conversions() {
        if flashtex_engine::system::find_file("tex-text.tec", Format::MiscFonts).is_none() {
            eprintln!("skipped: no tex-text.tec (TeX Live)");
            return;
        }
        let mut g = Globals::new();
        let m = g.load_mapping_file(b"tex-text", false);
        assert!(m > 0, "the mapping did not load");
        let convert = |g: &mut Globals, s: &str| -> Vec<i32> {
            let n = g.apply_mapping(m, &u16s(s));
            (0..n as usize).map(|k| g.mapped_text[k]).collect()
        };
        let text = "--- `` '' -- ?` !` x---y";
        let first = convert(&mut g, text);
        assert!(!first.is_empty());
        let long = "x".repeat(5000);
        for other in ["`", "-", "--", long.as_str(), "''`", ""] {
            convert(&mut g, other);
        }
        assert_eq!(convert(&mut g, text), first);
    }

    #[test]
    fn a_restore_puts_back_the_layout_engines_shaping_state() {
        let Some(e) = engine() else {
            eprintln!("skipped: no lmroman10-regular.otf (TeX Live)");
            return;
        };
        let mut g = Globals::new();
        g.term_out = AlphaFile::captured_terminal();
        let h = g.host.handles.alloc(Object::Other(Arc::new(e)));
        let unshaped = g.layout_engine(h).unwrap().shaping_state();
        let rec = g.capture_record(true).unwrap();
        assert_eq!(rec.engines, vec![(h, unshaped.clone())]);
        // the run goes on: the engine shapes
        shape(&g.layout_engine(h).unwrap(), "office");
        let shaped = g.layout_engine(h).unwrap().shaping_state();
        assert_ne!(shaped, unshaped);
        g.restore_record(&rec, None).unwrap();
        assert_eq!(g.layout_engine(h).unwrap().shaping_state(), unshaped);
    }
}
