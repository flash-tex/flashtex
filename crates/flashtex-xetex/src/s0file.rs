//! A persisted S₀ for the Unicode host (lane UNICODE-POLISH; DESIGN.md
//! §5.1; the pdfTeX engine's `--s0-cache`, `host::S0Image` and
//! `host::read_s0`, whose file layout this follows): a reopened document
//! starts from its S₀ in a new process, with no format load and no
//! preamble.
//!
//! The file holds S₀'s word space (its nonzero chunks, in whole
//! `S0_PAGE`-aligned pages that a reopen maps copy-on-write,
//! `Arena::map_pages`), S₀'s key (`flashtex_engine::host::Key`, checked by
//! the same code as the in-process S₀, and its arm-window reads) and record
//! (`crate::ckpt::Record`), and the state outside the word space that a
//! checkpoint saves with it (`state.rs`): the protrusion codes,
//! `full_name_of_file`, the source map at S₀, the font manager as the
//! lookups made of it (a manager made anew over the same catalog that makes
//! them again holds the same maps: the font directories are S₀'s key), and
//! the handle table with each object as the recipe it is made from again:
//! a glyph-info array or a picture's path as its data, a font's layout
//! engine as its file, face, size and options (made again with FreeType and
//! HarfBuzz from the file; the last shaping's shaper and script are the
//! record's), a TECkit mapping as its file, an OpenType assembly as its
//! parts. Every file an object is made from is kept by its content's hash:
//! a font or mapping file changed while the document was closed refuses S₀.
//!
//! [`Image::write`] writes through a temporary file of its own and a
//! rename, on a thread of its own: the engine thread only copies the
//! chunks out ([`prepare`]).

use crate::ckpt::Record;
use crate::generated::Globals;
use crate::native::font_inst::FontInst;
use crate::native::layout::LayoutEngine;
use crate::state::{GlyphInfo, Handles, Object};
use flashtex_engine::arena::{CheckpointId, CHUNK_BYTES};
use flashtex_engine::host::Key;
use flashtex_engine::persist::{Codec, Reader};
use std::ffi::CString;
use std::sync::Arc;

const MAGIC: &[u8] = b"flashtex xetex S0 v1";

/// The granule of S₀'s pages in the file: Apple silicon's page, which every
/// supported system's page size divides (as the pdfTeX engine's v5).
const S0_PAGE: usize = 16384;

/// What a persisted S₀ keeps besides the engine's state: what the resident
/// engine checks and keeps of it (`crate::resident`).
pub struct Meta {
    /// The resident engine's key of the state after the format load (the
    /// command line, the first line, the directories) and the format file
    /// with its identity, encoded by `crate::resident`.
    pub point: Vec<u8>,
    pub key: Key,
    pub font_dirs: Option<[u64; 2]>,
    pub dl_on: bool,
    pub rec: Record,
    /// The output files the run had opened by S₀, in order.
    pub opens: Vec<String>,
    /// The user's files open at S₀, as the last run read them.
    pub texts: Vec<(String, Vec<u8>)>,
}

impl Meta {
    fn enc(&self, w: &mut Vec<u8>) {
        self.point.enc(w);
        self.key.enc(w);
        self.font_dirs.enc(w);
        self.dl_on.enc(w);
        self.rec.enc(w);
        self.opens.enc(w);
        self.texts.enc(w);
    }

    fn dec(r: &mut Reader) -> Result<Meta, String> {
        Ok(Meta {
            point: Codec::dec(r)?,
            key: Codec::dec(r)?,
            font_dirs: Codec::dec(r)?,
            dl_on: Codec::dec(r)?,
            rec: Codec::dec(r)?,
            opens: Codec::dec(r)?,
            texts: Codec::dec(r)?,
        })
    }
}

/// S₀ taken out of the engine for writing: the header and a copy of the
/// nonzero chunks, `CHUNK_BYTES` each in `data`.
pub struct Image {
    head: Vec<u8>,
    chunks: Vec<u32>,
    data: Vec<u8>,
    /// Milliseconds taken by the header, S₀'s view and the copy.
    pub ms: [f64; 3],
}

/// A file an object of the handle table is made from, with its content's
/// hash then.
type Made = (String, [u64; 2]);

fn hash_of(path: &str) -> Result<[u64; 2], String> {
    flashtex_engine::persist::hash128_file(path, None)
        .map(|(h, _)| h)
        .map_err(|e| format!("{path}: {e}"))
}

fn f32_bits(v: f32) -> u32 {
    v.to_bits()
}

/// Write `o` as its recipe; the files it is made from go to `made`.
fn enc_object(o: &Object, w: &mut Vec<u8>, made: &mut Vec<Made>) -> Result<(), String> {
    match o {
        Object::GlyphInfo(gi) => {
            0u8.enc(w);
            gi.locations.enc(w);
            gi.ids.enc(w);
        }
        Object::PicPath(p) => {
            1u8.enc(w);
            p.to_vec().enc(w);
        }
        Object::Other(a) => {
            if let Some(e) = a.downcast_ref::<LayoutEngine>() {
                2u8.enc(w);
                let path = String::from_utf8_lossy(&e.font.filename).into_owned();
                made.push((path, hash_of(&String::from_utf8_lossy(&e.font.filename))?));
                e.font.filename.enc(w);
                e.font.index.enc(w);
                f32_bits(e.font.point_size).enc(w);
                e.font.vertical.enc(w);
                e.script.enc(w);
                e.language_src.enc(w);
                let feats: Vec<(u32, u32, u32, u32)> = e
                    .features
                    .iter()
                    .map(|f| (f.tag, f.value, f.start, f.end))
                    .collect();
                feats.enc(w);
                let shapers: Option<Vec<Vec<u8>>> = e
                    .shaper_list
                    .as_ref()
                    .map(|l| l.iter().map(|c| c.as_bytes().to_vec()).collect());
                shapers.enc(w);
                e.rgb_value.enc(w);
                f32_bits(e.extend).enc(w);
                f32_bits(e.slant).enc(w);
                f32_bits(e.embolden).enc(w);
                e.req_engine.enc(w);
            } else if let Some(m) = a.downcast_ref::<crate::xetex_ext::Mapping>() {
                let (path, byte_mapping) = m.1.as_ref().ok_or("a mapping of no file")?;
                3u8.enc(w);
                made.push((path.clone(), hash_of(path)?));
                path.enc(w);
                byte_mapping.enc(w);
            } else if let Some(asm) = a.downcast_ref::<crate::native::otmath::Assembly>() {
                4u8.enc(w);
                let parts: Vec<(u32, i32, i32, (i32, u32))> = asm
                    .parts
                    .iter()
                    .map(|p| {
                        (
                            p.glyph,
                            p.start_connector_length,
                            p.end_connector_length,
                            (p.full_advance, p.flags),
                        )
                    })
                    .collect();
                parts.enc(w);
            } else if let Some(b) = a.downcast_ref::<Vec<u8>>() {
                5u8.enc(w);
                b.enc(w);
            } else {
                return Err("an object S0 cannot keep".into());
            }
        }
    }
    Ok(())
}

/// Make an object again from its recipe (fonts with the engine's FreeType
/// library `lib`).
fn dec_object(
    r: &mut Reader,
    lib: &Arc<crate::native::font_inst::FtLibrary>,
) -> Result<Object, String> {
    use crate::fontlibs::hb;
    Ok(match u8::dec(r)? {
        0 => Object::GlyphInfo(Arc::new(GlyphInfo {
            locations: Codec::dec(r)?,
            ids: Codec::dec(r)?,
        })),
        1 => Object::PicPath(Vec::<u8>::dec(r)?.into()),
        2 => {
            let filename = Vec::<u8>::dec(r)?;
            let index = u32::dec(r)?;
            let point_size = f32::from_bits(u32::dec(r)?);
            let vertical = bool::dec(r)?;
            let script = u32::dec(r)?;
            let language = Option::<Vec<u8>>::dec(r)?;
            let features = Vec::<(u32, u32, u32, u32)>::dec(r)?
                .into_iter()
                .map(|(tag, value, start, end)| hb::hb_feature_t {
                    tag,
                    value,
                    start,
                    end,
                })
                .collect();
            let shapers = Option::<Vec<Vec<u8>>>::dec(r)?
                .map(|l| {
                    l.into_iter()
                        .map(|b| CString::new(b).map_err(|e| e.to_string()))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?;
            let rgb = u32::dec(r)?;
            let extend = f32::from_bits(u32::dec(r)?);
            let slant = f32::from_bits(u32::dec(r)?);
            let embolden = f32::from_bits(u32::dec(r)?);
            let req_engine = u8::dec(r)?;
            let mut font = FontInst::new(lib, &filename, index, point_size, &mut |_| None)
                .ok_or_else(|| {
                    format!(
                        "{}: the font does not load",
                        String::from_utf8_lossy(&filename)
                    )
                })?;
            font.vertical = vertical;
            Object::Other(Arc::new(LayoutEngine::new(
                font,
                script,
                language.as_deref(),
                features,
                shapers,
                rgb,
                extend,
                slant,
                embolden,
                req_engine,
            )))
        }
        3 => {
            let path = String::dec(r)?;
            let byte_mapping = bool::dec(r)?;
            let tec = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
            let mut m = crate::xetex_ext::Mapping::new(tec, byte_mapping)
                .ok_or_else(|| format!("{path}: the mapping does not load"))?;
            m.1 = Some((path, byte_mapping));
            Object::Other(Arc::new(m))
        }
        4 => {
            let parts = Vec::<(u32, i32, i32, (i32, u32))>::dec(r)?
                .into_iter()
                .map(|(glyph, s, e, (a, flags))| hb::hb_ot_math_glyph_part_t {
                    glyph,
                    start_connector_length: s,
                    end_connector_length: e,
                    full_advance: a,
                    flags,
                })
                .collect();
            Object::Other(Arc::new(crate::native::otmath::Assembly { parts }))
        }
        5 => Object::Other(Arc::new(Vec::<u8>::dec(r)?)),
        t => return Err(format!("S0 file: bad object {t}")),
    })
}

/// The chunks of the space written so far (`Arena::touched`), one bit
/// each: taken when a save is asked for, so that a save made later
/// (`crate::resident::Engine::save_pending`, after other runs touched more
/// chunks) writes the chunks it would have written then.
pub fn touched(g: &Globals) -> Vec<u64> {
    let mut t = vec![0u64; g.arena.chunks().div_ceil(64)];
    for c in 0..g.arena.chunks() {
        if g.arena.touched(c) {
            t[c >> 6] |= 1 << (c & 63);
        }
    }
    t
}

fn bit(t: &[u64], c: usize) -> bool {
    t.get(c >> 6).is_some_and(|w| w >> (c & 63) & 1 == 1)
}

/// How many chunks are copied or written between two reads of `stop`.
const STOP_EVERY: usize = 64;

/// Take S₀ (checkpoint `id` of `g`, `meta` its key and record) out of the
/// engine: the header, and a copy of S₀'s nonzero chunks among `touched`
/// (`touched`, taken when the save was asked for). S₀ is a checkpoint: its
/// words are the same whatever runs came after it. `None` when `stop` said
/// so before the copy was done (a run is waiting: the copy is made again
/// later).
pub fn prepare(
    g: &mut Globals,
    id: CheckpointId,
    meta: &Meta,
    touched: &[u64],
    stop: &dyn Fn() -> bool,
) -> Result<Option<Image>, String> {
    if stop() {
        return Ok(None);
    }
    let t = std::time::Instant::now();
    let head = header(g, id, meta)?;
    let t_head = t.elapsed().as_secs_f64() * 1e3;
    if stop() {
        return Ok(None);
    }
    let t = std::time::Instant::now();
    let view = g.arena.view_at(id)?;
    let t_view = t.elapsed().as_secs_f64() * 1e3;
    let t = std::time::Instant::now();
    let mut chunks = vec![];
    let mut data = vec![];
    for c in 0..g.arena.chunks() {
        if c % STOP_EVERY == 0 && stop() {
            return Ok(None);
        }
        if !bit(touched, c) {
            continue;
        }
        let d = view.chunk(c);
        if d.iter().any(|&b| b != 0) {
            chunks.push(c as u32);
            data.extend_from_slice(d);
        }
    }
    let ms = [t_head, t_view, t.elapsed().as_secs_f64() * 1e3];
    Ok(Some(Image {
        head,
        chunks,
        data,
        ms,
    }))
}

/// Write S₀ to `path` from the engine itself, with no copy of its chunks
/// (Low Memory: the copy is the size of the file), the chunks as
/// [`prepare`] takes them. Returns the file's bytes, or `None` when `stop`
/// said so first (nothing is left at `path`'s temporary name).
pub fn write_in_place(
    g: &mut Globals,
    id: CheckpointId,
    meta: &Meta,
    path: &str,
    touched: &[u64],
    stop: &dyn Fn() -> bool,
) -> Result<Option<u64>, String> {
    let head = header(g, id, meta)?;
    let view = g.arena.view_at(id)?;
    let mut chunks: Vec<u32> = vec![];
    for c in 0..g.arena.chunks() {
        if c % STOP_EVERY == 0 && stop() {
            return Ok(None);
        }
        if bit(touched, c) && view.chunk(c).iter().any(|&b| b != 0) {
            chunks.push(c as u32);
        }
    }
    write_file(
        head,
        &chunks,
        &|i| view.chunk(chunks[i] as usize),
        path,
        stop,
    )
}

/// The file's header up to the chunks' list.
fn header(g: &mut Globals, id: CheckpointId, meta: &Meta) -> Result<Vec<u8>, String> {
    let mut head = vec![];
    MAGIC.to_vec().enc(&mut head);
    meta.enc(&mut head);
    let (mgr, full_name, protrusion, handles, dl) =
        g.saved_at(id).ok_or("no host state saved with S0")?;
    mgr.map(|m| m.lookups.clone()).enc(&mut head);
    full_name.enc(&mut head);
    // (in key order, which the HashMap's own encoding is not: the same S₀
    // makes the same file, whichever process writes it; read back as the
    // map it was)
    let mut codes: Vec<(&(i32, u32, i32), &i32)> = protrusion.iter().collect();
    codes.sort_unstable();
    codes.len().enc(&mut head);
    for (k, v) in codes {
        k.enc(&mut head);
        v.enc(&mut head);
    }
    let mut made: Vec<Made> = vec![];
    let mut objects = vec![];
    handles.enc_with(&mut objects, &mut |o, w| enc_object(o, w, &mut made))?;
    let mut dl_bytes = vec![];
    g.host.dl.enc_at(dl, &mut dl_bytes);
    made.sort();
    made.dedup();
    made.enc(&mut head);
    objects.enc(&mut head);
    dl_bytes.enc(&mut head);
    (g.arena.len_bytes() as u64).enc(&mut head);
    (g.arena.scalar_bytes() as u64).enc(&mut head);
    Ok(head)
}

impl Image {
    /// The bytes it holds in memory.
    pub fn bytes(&self) -> usize {
        self.head.len() + self.data.len()
    }

    /// Write the file to `path` through a temporary file of this writer's
    /// own and a rename (`path` is always a whole S₀ or what it was): the
    /// header, the nonzero chunks' list and their pages' list, then those
    /// pages whole, `S0_PAGE`-aligned. Returns the file's bytes.
    pub fn write(self, path: &str) -> Result<u64, String> {
        let Image {
            head, chunks, data, ..
        } = self;
        write_file(
            head,
            &chunks,
            &|i| &data[i * CHUNK_BYTES..(i + 1) * CHUNK_BYTES],
            path,
            &|| false,
        )
        .map(|n| n.expect("a write nothing stops"))
    }
}

/// Write the file: `head`, the list of the nonzero `chunks` and of their
/// pages, then those pages whole (`chunk(i)` is the `i`th chunk's bytes).
/// `None` when `stop` said so before the end (the temporary file removed).
fn write_file<'a>(
    mut head: Vec<u8>,
    chunks: &[u32],
    chunk: &dyn Fn(usize) -> &'a [u8],
    path: &str,
    stop: &dyn Fn() -> bool,
) -> Result<Option<u64>, String> {
    use std::io::Write;
    chunks.to_vec().enc(&mut head);
    let per = (S0_PAGE / CHUNK_BYTES) as u32;
    let mut pages: Vec<u32> = chunks.iter().map(|&c| c / per).collect();
    pages.dedup();
    pages.enc(&mut head);
    let data_off = (8 + head.len()).next_multiple_of(S0_PAGE);
    let (tmp, f) = {
        let mut k = 0u32;
        loop {
            let tmp = format!(
                "{path}.{}.{}.{k}.tmp",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos())
            );
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
            {
                Ok(f) => break (tmp, f),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && k < 100 => k += 1,
                Err(e) => return Err(format!("{tmp}: {e}")),
            }
        }
    };
    let gone = |e: String| {
        let _ = std::fs::remove_file(&tmp);
        e
    };
    let mut f = std::io::BufWriter::with_capacity(1 << 20, f);
    let pad = vec![0u8; data_off - 8 - head.len()];
    f.write_all(&(head.len() as u64).to_le_bytes())
        .and_then(|_| f.write_all(&head))
        .and_then(|_| f.write_all(&pad))
        .map_err(|e| gone(format!("{tmp}: {e}")))?;
    let zero = [0u8; CHUNK_BYTES];
    let mut next = chunks.iter().enumerate().peekable();
    for (n, &pg) in pages.iter().enumerate() {
        if n % STOP_EVERY == 0 && n > 0 && stop() {
            drop(f);
            let _ = std::fs::remove_file(&tmp);
            return Ok(None);
        }
        for c in pg * per..(pg + 1) * per {
            let d = match next.peek() {
                Some(&(i, &pc)) if pc == c => {
                    next.next();
                    chunk(i)
                }
                _ => &zero[..],
            };
            f.write_all(d).map_err(|e| gone(format!("{tmp}: {e}")))?;
        }
    }
    let f = f.into_inner().map_err(|e| gone(format!("{tmp}: {e}")))?;
    f.sync_all().ok();
    drop(f);
    std::fs::rename(&tmp, path).map_err(|e| gone(format!("{path}: {e}")))?;
    std::fs::metadata(path)
        .map(|m| Some(m.len()))
        .map_err(|e| format!("{path}: {e}"))
}

/// An S₀ opened from its file into a new engine.
pub struct Opened {
    pub g: Box<Globals>,
    /// S₀'s checkpoint in the new engine.
    pub id: CheckpointId,
    pub meta: Meta,
    /// The file's pages mapped into the word space (0: its chunks copied).
    pub mapped_pages: usize,
}

/// Open the S₀ file at `path` into a new engine. `check` runs once the
/// header is read (the resident engine's checks of S₀'s key, command line,
/// format and font directories); `catalog` is the fonts' catalog for the
/// font manager's lookups, asked only when S₀ had a font manager.
pub fn open(
    path: &str,
    check: &mut dyn FnMut(&Meta) -> Result<(), String>,
    catalog: &mut dyn FnMut() -> Arc<crate::fontmgr::FontCatalog>,
) -> Result<Opened, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let map = flashtex_engine::os::MappedFile::of(&file, path)?;
    let bytes = map.bytes();
    let hlen = u64::from_le_bytes(
        bytes
            .get(..8)
            .ok_or("not an S0 file")?
            .try_into()
            .expect("8 bytes"),
    ) as usize;
    let mut r = Reader::new(bytes.get(8..8 + hlen).ok_or("S0 file truncated")?);
    if Vec::<u8>::dec(&mut r)? != MAGIC {
        return Err("not a Unicode-mode S0 file".into());
    }
    let meta = Meta::dec(&mut r)?;
    if meta.key.build != flashtex_engine::host::engine_build() {
        return Err("S0 was saved by another engine build".into());
    }
    check(&meta)?;
    let lookups = Option::<Vec<(String, i32)>>::dec(&mut r)?;
    let full_name = Option::<String>::dec(&mut r)?;
    let protrusion = crate::state::Protrusion::dec(&mut r)?;
    // the files the fonts and mappings are made from, as they were
    for (p, h) in Vec::<Made>::dec(&mut r)? {
        if hash_of(&p).ok() != Some(h) {
            return Err(format!("{p} changed"));
        }
    }
    let objects = Vec::<u8>::dec(&mut r)?;
    let dl = Vec::<u8>::dec(&mut r)?;
    let arena_len = u64::dec(&mut r)? as usize;
    let scalar_bytes = u64::dec(&mut r)? as usize;
    let present = Vec::<u32>::dec(&mut r)?;
    let pages = Vec::<u32>::dec(&mut r)?;
    let data_off = (8 + hlen).next_multiple_of(S0_PAGE);
    let per = S0_PAGE / CHUNK_BYTES;
    if bytes.len() < data_off + pages.len() * S0_PAGE {
        return Err("S0 file truncated".into());
    }
    let mut at = Vec::with_capacity(present.len());
    {
        let mut k = 0;
        for &c in &present {
            let pg = c / per as u32;
            while k < pages.len() && pages[k] < pg {
                k += 1;
            }
            if pages.get(k) != Some(&pg) {
                return Err("S0 file: a chunk outside its pages".into());
            }
            at.push(data_off + k * S0_PAGE + (c as usize % per) * CHUNK_BYTES);
        }
    }
    let mut g = Globals::new();
    if g.arena.len_bytes() != arena_len || g.arena.scalar_bytes() != scalar_bytes {
        return Err("S0 was saved with another word-space layout".into());
    }
    if present.iter().any(|&c| c as usize >= g.arena.chunks()) {
        return Err("S0 file: a chunk outside the space".into());
    }
    // Mapped (copy on write: the pages stay the file's, out of the
    // footprint, until the engine writes them), but a last page that runs
    // past the space's end (which `map_pages` refuses whole); the rest
    // copied.
    let whole = pages
        .iter()
        .take_while(|&&p| (p as usize + 1) * S0_PAGE <= g.arena.len_bytes())
        .count();
    let mapped_pages = if whole > 0
        && g.arena
            .map_pages(&file, data_off as u64, &pages[..whole], S0_PAGE)
    {
        whole
    } else {
        0
    };
    let mapped_end = pages
        .get(mapped_pages.wrapping_sub(1))
        .filter(|_| mapped_pages > 0)
        .map_or(0, |&p| (p + 1) as usize * per);
    for (&c, &off) in present.iter().zip(&at) {
        if (c as usize) < mapped_end {
            g.arena.mark_loaded(c as usize);
        } else {
            g.arena
                .load_chunk(c as usize, &bytes[off..off + CHUNK_BYTES]);
        }
    }
    g.fill_scalars();
    // the state outside the word space
    let lib = g.ft_library();
    let handles = Handles::dec_with(&mut Reader::new(&objects), &mut |r| dec_object(r, &lib))?;
    g.host.handles = handles;
    g.host.protrusion = protrusion;
    g.host.full_name_of_file = full_name;
    g.host.font_mgr = match lookups {
        None => None,
        Some(ls) => {
            let mut m = crate::fontmgr::FontMgr::new(catalog());
            for (name, size) in ls {
                m.locate(&name, size);
            }
            Some(Arc::new(m))
        }
    };
    g.host.dl = crate::dlsource::DlSource::dec(&mut Reader::new(&dl))?;
    let id = g.checkpoint();
    Ok(Opened {
        g,
        id,
        meta,
        mapped_pages,
    })
}
