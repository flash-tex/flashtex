//! S₁: a persisted body checkpoint at the last viewed page (lane COLD-OPEN).
//!
//! Reopening a known document at page V ran the body from S₀ (§5.1) to V
//! first: about 25 s at page ~883 of *Infinite Descent ×2*. S₁ is the page
//! checkpoint just before the last viewport, persisted after a compile has
//! settled, so that a reopen opens S₀, puts S₁'s state on top of it and
//! ships page V at once.
//!
//! **What it holds.** A file like S₀'s ([`super::S0Image`]'s layout): a
//! header and the word space's chunks that differ from S₀'s file at S₁
//! (all of the scalar region always: opening S₀ sets scalars), written over
//! S₀'s through the write barrier, so that S₀ stays restorable and S₁'s undo
//! log is S₀'s pre-images. The header: the tag of the S₀ it extends
//! ([`key_tag`]), S₁'s [`Key`] over the reads between S₀ and S₁ (files with
//! their content hashes, the prefixes of the input files open at S₁, the
//! lookups and their directories), the host record, the output files open
//! there (their first bytes, the PDF's by length and hash: it is too large
//! to copy, and the reopen verifies the PDF on disk and truncates it), the
//! terminal, the diagnostics notes, the output opens, the journal's entries
//! between S₀ and S₁, the frames of the pages before it, and the display
//! list's side table by place (file, line, column), which a new process
//! names afresh.
//!
//! **Why it is what a full run reaches.** As for S₀: S₁ is a checkpoint like
//! any other, and the run from S₀ to it read only what its key and S₀'s
//! say, which a reopen checks; the engine is deterministic. It is taken only
//! after the `.aux` read, before any external effect, with no L5 patch, from
//! a complete run whose `.aux` is the one it read (`incr::Session::prepare_s1`
//! says why not otherwise). The reopened session's history before S₁ (the
//! read-set, the line journal, the page checkpoints between S₀ and S₁) is not
//! in the process: `incr::Session` runs every compile whose restart point is
//! before S₁ from S₀ with nothing kept, and fills the pages before S₁ from S₀
//! in the background, converging with S₁'s run like any other run.

use super::{hash_prefix, Key, S0Image, Scratch, MAGIC as S0_MAGIC, S0_PAGE};
use crate::arena::{CheckpointId, CHUNK_BYTES};
use crate::checkpoint::ExtRecord;
use crate::generated::Globals;
use crate::os::MappedFile;
use crate::persist::{Codec, Reader};
use crate::system::{self, FileRead, ReadLog, StatSig, Stream};
use std::collections::HashMap;

/// v1: the first.
pub const MAGIC: &[u8] = b"flashtex S1 v1";

/// A read of the journal between S₀ and S₁: `system::FileRead` without its
/// content (path, hash, signature, (closed_at, written_before, stamp, size)).
pub type Read = (
    String,
    [u64; 2],
    StatSig,
    (Option<u64>, bool, Option<i64>, Option<u64>),
);

/// S₁'s header.
pub struct Head {
    /// The S₀ it extends ([`key_tag`]).
    pub s0_tag: [u64; 2],
    pub key: Key,
    pub rec: ExtRecord,
    /// Pages shipped before S₁, and in the run it was taken from.
    pub pages: u64,
    pub total: u64,
    /// The frames (PDF bytes' hash and length) of the pages before S₁.
    pub frames: Vec<([u64; 2], u64)>,
    /// The output files open at S₁: path, length there, the bytes (`None`:
    /// the PDF, by its hash).
    pub outputs: Vec<(String, u64, Option<Vec<u8>>, [u64; 2])>,
    pub terminal: Vec<u8>,
    pub notes: Vec<crate::diag::Note>,
    pub opens: Vec<String>,
    /// The journal from S₀'s counts to S₁'s: files, lookups, outputs.
    pub reads: Vec<Read>,
    pub lookups: Vec<(String, u8, Option<bool>, Option<String>)>,
    pub outs: Vec<String>,
    /// The side table's places, and its nonzero entries at S₁ (byte offset
    /// in the space, place).
    pub places: Vec<(Vec<u8>, u32, u32)>,
    pub side: Vec<(u64, u32)>,
    /// The position corrections S₁ owes (`incr::Reloc`'s pieces and
    /// overrides), applied after it is loaded.
    pub reloc: (Vec<(i64, i64)>, Vec<(u64, u64, u64)>),
    pub sizes: (u64, u64),
}

crate::codec_struct!(Head {
    s0_tag,
    key,
    rec,
    pages,
    total,
    frames,
    outputs,
    terminal,
    notes,
    opens,
    reads,
    lookups,
    outs,
    places,
    side,
    reloc,
    sizes
});

/// The image table's encoding.
fn img_enc(c: &crate::pdftex::CState) -> Vec<u8> {
    let mut w = vec![];
    (*c.img).enc(&mut w);
    w
}

/// What names an S₀ for the S₁ that extends it: its key without the stat
/// signatures (a check refreshes them: `Key::check_refresh`). The key says
/// all S₀ depends on, so an S₀ file with this key holds this S₀.
pub fn key_tag(k: &Key) -> [u64; 2] {
    let mut k = k.clone();
    for f in k.files.iter_mut() {
        f.2 = StatSig::default();
    }
    for p in k.prefixes.iter_mut() {
        p.3 = StatSig::default();
    }
    for d in k.dirs.iter_mut() {
        d.1 = StatSig::default();
    }
    let mut b = vec![];
    k.enc(&mut b);
    crate::persist::hash128(&b)
}

/// Where each chunk `present` lists is in a file whose pages (`pages`)
/// start at `data_off` (S₀'s layout, v5).
pub(super) fn chunk_offsets(
    present: &[u32],
    pages: &[u32],
    data_off: usize,
) -> Result<Vec<usize>, String> {
    let per = S0_PAGE / CHUNK_BYTES;
    let mut at = Vec::with_capacity(present.len());
    let mut k = 0;
    for &c in present {
        let pg = c / per as u32;
        while k < pages.len() && pages[k] < pg {
            k += 1;
        }
        if pages.get(k) != Some(&pg) {
            return Err("a chunk outside its pages".into());
        }
        at.push(data_off + k * S0_PAGE + (c as usize % per) * CHUNK_BYTES);
    }
    Ok(at)
}

/// A persisted S₀'s key and chunks, as a reopen loads them.
struct S0Chunks {
    map: MappedFile,
    key: Key,
    at: HashMap<u32, usize>,
}

impl S0Chunks {
    fn open(path: &str) -> Result<S0Chunks, String> {
        let map = MappedFile::open(path)?;
        let (key, present, pages, data_off) = {
            let bytes = map.bytes();
            let hlen =
                u64::from_le_bytes(bytes.get(..8).ok_or("not an S0 file")?.try_into().unwrap())
                    as usize;
            let mut r = Reader::new(bytes.get(8..8 + hlen).ok_or("S0 file truncated")?);
            if Vec::<u8>::dec(&mut r)? != S0_MAGIC {
                return Err("not an S0 file".into());
            }
            let key = Key::dec(&mut r)?;
            ExtRecord::dec(&mut r)?;
            Vec::<(String, Vec<u8>)>::dec(&mut r)?;
            Vec::<u8>::dec(&mut r)?;
            Vec::<crate::diag::Note>::dec(&mut r)?;
            Vec::<(i32, crate::diag::Site)>::dec(&mut r)?;
            Vec::<String>::dec(&mut r)?;
            u64::dec(&mut r)?;
            u64::dec(&mut r)?;
            let present = Vec::<u32>::dec(&mut r)?;
            let pages = Vec::<u32>::dec(&mut r)?;
            let data_off = (8 + hlen).next_multiple_of(S0_PAGE);
            if bytes.len() < data_off + pages.len() * S0_PAGE {
                return Err("S0 file truncated".into());
            }
            (key, present, pages, data_off)
        };
        let offs = chunk_offsets(&present, &pages, data_off)?;
        Ok(S0Chunks {
            map,
            key,
            at: present.into_iter().zip(offs).collect(),
        })
    }

    /// Chunk `c` as a reopen loads it (zero where the file has none).
    fn same(&self, c: usize, d: &[u8]) -> bool {
        match self.at.get(&(c as u32)) {
            Some(&o) => &self.map.bytes()[o..o + CHUNK_BYTES] == d,
            None => d.iter().all(|&b| b == 0),
        }
    }
}

/// What [`prepare`] takes from the session.
pub struct Input<'a> {
    pub s0_id: CheckpointId,
    pub s0_key: &'a Key,
    /// S₀'s file, which the reopen will load S₁ over.
    pub s0_file: &'a str,
    /// S₁: a page checkpoint of the live chain, `pages` shipped before it.
    pub id: CheckpointId,
    pub pages: usize,
    pub total: usize,
    pub frames: Vec<([u64; 2], u64)>,
    pub journal: &'a ReadLog,
    pub reloc: (Vec<(i64, i64)>, Vec<(u64, u64, u64)>),
    pub clock: (i64, i32),
    pub first_line: &'a [u8],
}

/// The byte range of the space's region `name`.
fn region(g: &Globals, name: &str) -> std::ops::Range<usize> {
    g.arena
        .regions
        .iter()
        .find(|r| r.name == name)
        .map_or(0..0, |r| r.off..r.off + r.bytes)
}

/// Take S₁ out of the engine for writing (`super::s0write` writes it), on
/// the engine's thread. `Err` says why there is none.
pub fn prepare(g: &mut Globals, i: Input) -> Result<S0Image, String> {
    let rec0 = g.record_of(i.s0_id)?;
    let mut rec = g.record_of(i.id)?;
    if rec.effects_len != 0 {
        return Err("an external command ran before it".into());
    }
    if rec0.reads.0 > rec.reads.0 || rec0.reads.1 > rec.reads.1 || rec0.reads.2 > rec.reads.2 {
        return Err("its journal counts are before S0's".into());
    }
    let j = i.journal;
    if j.files.len() < rec.reads.0 || j.lookups.len() < rec.reads.1 || j.outputs.len() < rec.reads.2
    {
        return Err("the journal is shorter than its counts".into());
    }
    // An input file open at S₁ is keyed by the bytes consumed (`make_key`).
    // A read of it after S₀ and before that open (a chapter `\input` twice,
    // *Infinite Descent ×2*'s) is keyed by the content it read, as a file
    // read and closed is.
    let delta = &j.files[rec0.reads.0..rec.reads.0];
    let mut earlier = vec![];
    for f in &rec.files {
        if let Stream::In { path, .. } = &f.stream {
            let open_at_s0 = rec0
                .files
                .iter()
                .any(|g| matches!(&g.stream, Stream::In { path: p, .. } if p == path));
            let opens: Vec<&FileRead> = delta
                .iter()
                .filter(|r| r.path == *path && r.closed_at.is_none())
                .collect();
            // (the last is the open at S₁, unless that was open at S₀)
            let n = opens.len() - usize::from(!open_at_s0).min(opens.len());
            for r in &opens[..n] {
                let hash = match (&r.content, r.hash) {
                    (Some(c), [0, 0]) => crate::persist::hash128(c),
                    (_, h) => h,
                };
                earlier.push((path.clone(), hash, r.stat));
            }
        }
    }
    let key = super::make_key_from(
        g,
        i.id,
        &rec,
        j,
        (rec0.reads.0, rec0.reads.1),
        i.clock,
        i.first_line,
    )?;
    let mut key = key;
    key.files.extend(earlier);
    // An input file open at S₀ and closed by S₁ (the `.aux`, which S₀'s key
    // takes unread: its prefix there is empty) was read after S₀: keyed by
    // what its read gave, as a file opened after S₀ is. (One still open at
    // S₁ is keyed by its prefix there.)
    for f in &rec0.files {
        let Stream::In { path, .. } = &f.stream else {
            continue;
        };
        let open_at_s1 = rec
            .files
            .iter()
            .any(|g| matches!(&g.stream, Stream::In { path: p, .. } if p == path));
        if open_at_s1 {
            continue;
        }
        let r = j.files[..rec0.reads.0]
            .iter()
            .rev()
            .find(|r| r.path == *path && r.closed_at.is_none())
            .ok_or_else(|| format!("{path}, open at S0, has no read in the journal"))?;
        let hash = match (&r.content, r.hash) {
            (Some(c), [0, 0]) => crate::persist::hash128(c),
            (_, h) => h,
        };
        key.files.push((path.clone(), hash, r.stat));
    }
    if !key.barriers.is_empty() {
        return Err("an external command ran before it".into());
    }
    if !key.arm.is_empty() {
        return Err("a read at the document's start is after S0".into());
    }
    // The output files open at S₁, up to their length there.
    let mut outputs = vec![];
    for f in &rec.files {
        if let Stream::Out { path, len, .. } = &f.stream {
            if path.ends_with(".pdf") {
                outputs.push((path.clone(), *len, None, hash_prefix(path, *len)?));
            } else {
                let d = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
                let p = d
                    .get(..*len as usize)
                    .ok_or_else(|| format!("{path} is shorter than at S1"))?;
                outputs.push((path.clone(), *len, Some(p.to_vec()), [0, 0]));
            }
        }
    }
    let terminal = system::terminal_bytes()
        .get(..rec.terminal_len)
        .ok_or("the terminal is shorter than at S1")?
        .to_vec();
    let notes: Vec<crate::diag::Note> = crate::diag::notes()
        .get(..rec.notes)
        .ok_or("the notes are fewer than at S1")?
        .iter()
        .map(|n| (**n).clone())
        .collect();
    let opens = system::opens_since(0)
        .get(..rec.opens)
        .ok_or("the output opens are fewer than at S1")?
        .to_vec();
    let reads: Vec<Read> = j.files[rec0.reads.0..rec.reads.0]
        .iter()
        .map(|f| {
            (
                f.path.clone(),
                match (&f.content, f.hash) {
                    (Some(c), [0, 0]) => crate::persist::hash128(c),
                    (_, h) => h,
                },
                f.stat,
                (f.closed_at, f.written_before, f.stamp, f.size),
            )
        })
        .collect();
    let lookups = j.lookups[rec0.reads.1..rec.reads.1]
        .iter()
        .map(|l| {
            (
                l.name.clone(),
                super::format_index(l.format),
                l.must_exist,
                l.found.clone(),
            )
        })
        .collect();
    let outs = j.outputs[rec0.reads.2..rec.reads.2].to_vec();
    let s0 = S0Chunks::open(i.s0_file)?;
    if key_tag(&s0.key) != key_tag(i.s0_key) {
        return Err("the stored S0 is another".into());
    }
    // The chunks: those that differ from S₀'s file, and the scalars' (which
    // opening S₀ sets: `fill_scalars`, the checkpoint's spill). The side
    // table's entries go by place; its words in the chunks are zero, as in
    // S₀'s file.
    let side_r = region(g, "dl_side");
    let scalar_chunks = g.arena.scalar_bytes().div_ceil(CHUNK_BYTES);
    // The side table's chunks at S₀: an entry the same as there dates from
    // before S₀, which S₀'s file holds blank (a node made then is restored
    // without a source span, `super::prepare_s0`), and so does S₁ then; a
    // run from S₀ in the reopened process has it blank too.
    let side_s0: HashMap<usize, Vec<u8>> = {
        let v0 = g.arena.view_at(i.s0_id)?;
        (side_r.start / CHUNK_BYTES..side_r.end.div_ceil(CHUNK_BYTES))
            .filter(|&c| c < g.arena.chunks() && g.arena.touched(c))
            .map(|c| (c, v0.chunk(c).to_vec()))
            .collect()
    };
    let view = g.arena.view_at(i.id)?;
    let mut places: Vec<(Vec<u8>, u32, u32)> = vec![];
    let mut place_at: HashMap<(Vec<u8>, u32, u32), u32> = HashMap::new();
    let mut side: Vec<(u64, u32)> = vec![];
    let mut chunks: Vec<u32> = vec![];
    let mut buf = [0u8; CHUNK_BYTES];
    for c in 0..g.arena.chunks() {
        if !g.arena.touched(c) {
            continue;
        }
        buf.copy_from_slice(view.chunk(c));
        let (lo, hi) = (c * CHUNK_BYTES, (c + 1) * CHUNK_BYTES);
        if hi > side_r.start && lo < side_r.end {
            let (a, b) = (side_r.start.max(lo), side_r.end.min(hi));
            for off in (a..b).step_by(8) {
                let w = &mut buf[off - lo..off - lo + 8];
                let v = u64::from_le_bytes(w.try_into().unwrap());
                let at_s0 = side_s0
                    .get(&c)
                    .map(|d| &d[off - lo..off - lo + 8])
                    .map(|b| u64::from_le_bytes(b.try_into().unwrap()));
                if v == 0 || at_s0 == Some(v) {
                    w.fill(0);
                    continue;
                }
                // (a column without a span names no place: blank)
                let Some((name, line, col)) = crate::displaylist::entry_place(v) else {
                    if v >> 16 != 0 {
                        return Err("a source span this process does not know".into());
                    }
                    w.fill(0);
                    continue;
                };
                let k = (name, line, col as u32);
                let n = places.len() as u32;
                let p = *place_at.entry(k.clone()).or_insert_with(|| {
                    places.push(k);
                    n
                });
                side.push((off as u64, p));
                w.fill(0);
            }
        }
        if c < scalar_chunks || !s0.same(c, &buf) {
            chunks.push(c as u32);
        }
    }
    let mut data = Scratch::zeroed(chunks.len() * CHUNK_BYTES);
    for (to, &c) in data
        .as_chunks_mut::<CHUNK_BYTES>()
        .0
        .iter_mut()
        .zip(&chunks)
    {
        let c = c as usize;
        to.copy_from_slice(view.chunk(c));
        let (lo, hi) = (c * CHUNK_BYTES, (c + 1) * CHUNK_BYTES);
        if hi > side_r.start && lo < side_r.end {
            // (as above: every side-table word goes by place or is blank)
            let (a, b) = (side_r.start.max(lo), side_r.end.min(hi));
            to[a - lo..b - lo].fill(0);
        }
    }
    drop(view);
    // (as S₀'s: a process that opens S₁ has its own read-set, empty; the
    // run it took from S₀ is another, `Dorm::epoch` 0)
    rec.rs = 0;
    rec.dorm.epoch = 0;
    let head = Head {
        s0_tag: key_tag(i.s0_key),
        key,
        rec,
        pages: i.pages as u64,
        total: i.total as u64,
        frames: i.frames,
        outputs,
        terminal,
        notes,
        opens,
        reads,
        lookups,
        outs,
        places,
        side,
        reloc: i.reloc,
        sizes: (g.arena.len_bytes() as u64, g.arena.scalar_bytes() as u64),
    };
    let mut h = vec![];
    MAGIC.to_vec().enc(&mut h);
    head.enc(&mut h);
    Ok(S0Image {
        head: h,
        tail: Box::new(|_| {}),
        chunks,
        data,
        all: true,
    })
}

/// An S₁ file, read and mapped.
pub struct File {
    map: MappedFile,
    pub head: Head,
    present: Vec<u32>,
    at: Vec<usize>,
}

impl File {
    pub fn read(path: &str) -> Result<File, String> {
        let map = MappedFile::open(path)?;
        let (head, present, pages, data_off) = {
            let bytes = map.bytes();
            let hlen =
                u64::from_le_bytes(bytes.get(..8).ok_or("not an S1 file")?.try_into().unwrap())
                    as usize;
            let mut r = Reader::new(bytes.get(8..8 + hlen).ok_or("S1 file truncated")?);
            if Vec::<u8>::dec(&mut r)? != MAGIC {
                return Err("not an S1 file".into());
            }
            let head = Head::dec(&mut r)?;
            let present = Vec::<u32>::dec(&mut r)?;
            let pages = Vec::<u32>::dec(&mut r)?;
            let data_off = (8 + hlen).next_multiple_of(S0_PAGE);
            if bytes.len() < data_off + pages.len() * S0_PAGE {
                return Err("S1 file truncated".into());
            }
            (head, present, pages, data_off)
        };
        let at = chunk_offsets(&present, &pages, data_off)?;
        Ok(File {
            map,
            head,
            present,
            at,
        })
    }

    /// Whether S₁ is what a full run reaches from S₀ (`s0`, whose key the
    /// caller checked) in this process now, for a run that stops at page
    /// `stop_at` (pages shipped): `Err` says why not. Changes nothing.
    pub fn check(
        &self,
        s0: &Key,
        s0_rec: &ExtRecord,
        clock: (i64, i32),
        first_line: &[u8],
        stop_at: Option<usize>,
    ) -> Result<(), String> {
        let h = &self.head;
        // The images read since S₀ need no reader again, or S₁ would hold
        // handles a new process does not have (`load_into`)
        if img_enc(&h.rec.cstate) != img_enc(&s0_rec.cstate) {
            h.rec.cstate.img.settled()?;
        }
        if h.key.build != super::engine_build() {
            return Err("S1 was saved by another engine build".into());
        }
        if h.s0_tag != key_tag(s0) {
            return Err("S1 extends another S0".into());
        }
        match stop_at {
            Some(s) if (h.pages as usize) < s => {}
            Some(_) => return Err("the viewport is before S1".into()),
            None => return Err("no viewport".into()),
        }
        if h.frames.len() != h.pages as usize || h.opens.len() != h.rec.opens {
            return Err("S1 file: its counts do not match its record".into());
        }
        h.key.check(clock, first_line)?;
        h.key.check_arm()?;
        for (p, len, bytes, hash) in &h.outputs {
            if bytes.is_none() && hash_prefix(p, *len).ok() != Some(*hash) {
                return Err(format!("{p} is not the one S1 was taken with"));
            }
        }
        Ok(())
    }

    /// Put S₁'s state into `g`, which holds S₀ just checkpointed: the word
    /// space through the write barrier, the side table by place, the
    /// output files (the PDF truncated), the files written and closed
    /// before it, the terminal, notes and output opens, and the host
    /// record. The caller applies `head.reloc`, extends the journal and
    /// takes the checkpoint.
    pub fn load_into(&self, g: &mut Globals, s0: CheckpointId) -> Result<(), String> {
        let h = &self.head;
        let mut rec = h.rec.clone();
        // The image table, when no image was read between S₀ and S₁: S₀'s
        // own copy (verified when S₀ was opened), so that the convergence
        // test of the fill, which compares tables only by copy
        // (`images::State::same_as`), finds S₁'s run's the same.
        let s0_images = {
            let rec0 = g.record_of(s0)?;
            let same = img_enc(&rec.cstate) == img_enc(&rec0.cstate);
            if same {
                rec.cstate.img = rec0.cstate.img.clone();
            }
            same
        };
        if (g.arena.len_bytes() as u64, g.arena.scalar_bytes() as u64) != h.sizes {
            return Err("S1 was saved with another word-space layout".into());
        }
        if self.present.iter().any(|&c| c as usize >= g.arena.chunks()) {
            return Err("S1 file: a chunk outside the space".into());
        }
        let bytes = self.map.bytes();
        for (&c, &o) in self.present.iter().zip(&self.at) {
            g.arena
                .write_through(c as usize * CHUNK_BYTES, &bytes[o..o + CHUNK_BYTES]);
        }
        for &(off, p) in &h.side {
            let (name, line, col) = h
                .places
                .get(p as usize)
                .ok_or("S1 file: a side-table entry without its place")?;
            let v = crate::displaylist::entry_at(name, *line, *col as u16).unwrap_or(0);
            g.arena.write_through(off as usize, &v.to_le_bytes());
        }
        g.fill_scalars();
        crate::displaylist::restored();
        for (p, len, d, _) in &h.outputs {
            match d {
                Some(d) => std::fs::write(p, d).map_err(|e| format!("{p}: {e}"))?,
                None => std::fs::OpenOptions::new()
                    .write(true)
                    .open(p)
                    .and_then(|f| f.set_len(*len))
                    .map_err(|e| format!("{p}: {e}"))?,
            }
            system::stamp_output(p);
        }
        h.key.rewrite_outputs()?;
        system::truncate_terminal(0);
        system::append_terminal(&h.terminal);
        crate::diag::truncate(0);
        let notes: Vec<std::sync::Arc<crate::diag::Note>> =
            h.notes.iter().cloned().map(std::sync::Arc::new).collect();
        crate::diag::append(&notes, 0);
        system::truncate_external_effects(0);
        g.restore_ext(&rec)?;
        // (S₀'s table is verified, and reading it again would copy it; a
        // later one holds only written images, `check`)
        if !s0_images {
            g.verify_persisted_images_settled()?;
        }
        // (`restore_ext` left placeholders for the opens)
        system::truncate_opens(0);
        system::append_opens(&h.opens);
        Ok(())
    }

    /// The journal's files between S₀ and S₁, read before anything is
    /// written (`load_into` writes the output files, the `.aux` among them,
    /// which the run read before it wrote them). An input file open at S₁
    /// is taken as it is now (the key checked the bytes consumed), as S₀'s
    /// open files are; another user file's content is read again, and must
    /// be what the run read.
    pub fn journal_reads(&self) -> Result<Vec<FileRead>, String> {
        let h = &self.head;
        let open: Vec<&str> = h
            .rec
            .files
            .iter()
            .filter_map(|f| match &f.stream {
                Stream::In { path, .. } => Some(path.as_str()),
                _ => None,
            })
            .collect();
        let mut v = Vec::with_capacity(h.reads.len());
        let mut now_of: HashMap<&str, std::sync::Arc<Vec<u8>>> = HashMap::new();
        for (path, hash, stat, (closed_at, written_before, stamp, size)) in &h.reads {
            let mut f = FileRead {
                path: path.clone(),
                hash: *hash,
                stat: *stat,
                content: None,
                closed_at: *closed_at,
                written_before: *written_before,
                stamp: *stamp,
                size: *size,
            };
            if closed_at.is_none() && system::is_user_file(path) && !*written_before {
                let d = match now_of.get(path.as_str()) {
                    Some(d) => d.clone(),
                    None => {
                        let d = std::sync::Arc::new(
                            system::read_logical(path).map_err(|e| format!("{path}: {e}"))?,
                        );
                        now_of.insert(path, d.clone());
                        d
                    }
                };
                let now = crate::persist::hash128(&d);
                if open.contains(&path.as_str()) {
                    f.hash = now;
                    f.stat = StatSig::of(path).unwrap_or_default();
                } else if now != *hash {
                    return Err(format!("{path} changed"));
                }
                f.content = Some(d);
            }
            v.push(f);
        }
        Ok(v)
    }

    /// The journal's entries between S₀ and S₁ (`reads`, from
    /// [`journal_reads`](Self::journal_reads)), appended to `j` (S₀'s, as
    /// `incr::Session::open_s0` rebuilds it).
    pub fn extend_journal(&self, j: &mut ReadLog, reads: Vec<FileRead>) {
        let h = &self.head;
        for f in reads {
            j.mark_seen(&f.path);
            j.files_mut().push(f);
        }
        for (name, fmt, must, found) in &h.lookups {
            j.lookups_mut().push(system::Lookup {
                name: name.clone(),
                format: crate::resolver::Format::all()[*fmt as usize],
                must_exist: *must,
                found: found.clone(),
            });
        }
        j.outputs.extend(h.outs.iter().cloned());
        j.dirs = h.key.dirs.clone();
    }
}
