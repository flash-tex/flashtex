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
//! **A delta against a base.** Most of S₀'s word space is the state after
//! the format load, unchanged: XeTeX's `eqtb` over 0x110000 code points
//! (catcodes, case, space-factor, math and delimiter codes) is about 90 MB
//! of nonzero chunks, which every document of a format shares. So a page
//! of S₀ that is byte for byte a page of the *base* -- the word space after
//! the format load, written once per format file, engine build and layout
//! beside the S₀s (`<hash>.xs0base`) -- is recorded as a reference, not
//! stored: the S₀ file holds the pages that differ. The base is made from
//! the engine's format point when there is none yet; S₀ records the base's
//! identity (a nonce of the file written, which is never rewritten in
//! place), and a reopen maps the referenced pages copy-on-write from the
//! base and the rest from S₀'s own file. A base that is missing, made
//! again, or unreadable refuses S₀ (the run starts from the start, and
//! saves again). Which pages are references is decided by comparing the
//! bytes, so a base made by another document of the same format serves
//! too, however its state after the format load differed. A base that does
//! not open (cut short, not a base) is made again in its place at the next
//! save. After a save, a base no S₀ in the cache refers to (an old format
//! file's, an old engine build's) is removed once it is a day old, unless
//! the engine uses it ([`collect_bases`]).
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
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAGIC: &[u8] = b"flashtex xetex S0 v2";

/// A base file (see the module documentation).
const BASE_MAGIC: &[u8] = b"flashtex xetex S0 base v1";
const BASE_EXT: &str = "xs0base";

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
/// nonzero chunks, `CHUNK_BYTES` each in `data`, and the base it is to be
/// written against.
pub struct Image {
    head: Vec<u8>,
    /// The chunks of the space.
    space: usize,
    chunks: Vec<u32>,
    data: Vec<u8>,
    base: Option<ImageBase>,
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

/// Where a document's S₀ finds its base, and what the base stands for.
pub struct BaseSpec {
    /// The S₀ cache's directory, where the bases live beside the S₀s.
    pub dir: PathBuf,
    /// The format file and its identity, as `crate::resident` encodes them:
    /// with the engine build and the word space's layout, which base file.
    pub format: Vec<u8>,
    /// The engine's checkpoint after the format load, if it has one: a base
    /// is made from it when there is none yet.
    pub point: Option<CheckpointId>,
}

/// Which base file `spec` names for the engine `g`.
pub fn base_path(spec: &BaseSpec, g: &Globals) -> PathBuf {
    let mut key = vec![];
    BASE_MAGIC.to_vec().enc(&mut key);
    flashtex_engine::host::engine_build().enc(&mut key);
    spec.format.enc(&mut key);
    (g.arena.len_bytes() as u64).enc(&mut key);
    (g.arena.scalar_bytes() as u64).enc(&mut key);
    let h = flashtex_engine::persist::hash128(&key);
    spec.dir
        .join(format!("{:016x}{:016x}.{BASE_EXT}", h[0], h[1]))
}

/// A new base's identity: unique to the file written (a base is never
/// rewritten in place; a new one is a new identity).
fn new_nonce(path: &Path) -> [u64; 2] {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    // (the process, the thread and the time: two writers of one base are
    // two processes, or two threads of one)
    let seed = format!(
        "{}\0{}\0{:?}\0{t}",
        path.display(),
        std::process::id(),
        std::thread::current().id(),
    );
    flashtex_engine::persist::hash128(seed.as_bytes())
}

/// A base file, opened: its identity and its pages, mapped.
struct BaseFile {
    file: std::fs::File,
    map: flashtex_engine::os::MappedFile,
    nonce: [u64; 2],
    pages: Vec<u32>,
    data_off: usize,
}

impl BaseFile {
    fn open(path: &Path) -> Result<BaseFile, String> {
        let name = path.display().to_string();
        let file = std::fs::File::open(path).map_err(|e| format!("{name}: {e}"))?;
        let map = flashtex_engine::os::MappedFile::of(&file, &name)?;
        let bytes = map.bytes();
        let hlen = u64::from_le_bytes(
            bytes
                .get(..8)
                .ok_or("not an S0 base")?
                .try_into()
                .expect("8 bytes"),
        ) as usize;
        let mut r = Reader::new(bytes.get(8..8 + hlen).ok_or("S0 base truncated")?);
        if Vec::<u8>::dec(&mut r)? != BASE_MAGIC {
            return Err("not an S0 base".into());
        }
        let nonce = <[u64; 2]>::dec(&mut r)?;
        let _chunks = Vec::<u32>::dec(&mut r)?;
        let pages = Vec::<u32>::dec(&mut r)?;
        let data_off = (8 + hlen).next_multiple_of(S0_PAGE);
        if bytes.len() < data_off + pages.len() * S0_PAGE {
            return Err("S0 base truncated".into());
        }
        Ok(BaseFile {
            file,
            map,
            nonce,
            pages,
            data_off,
        })
    }

    /// Page `p` of the space as the base holds it (`None`: all zero).
    fn page(&self, p: u32) -> Option<&[u8]> {
        let k = self.pages.binary_search(&p).ok()?;
        let at = self.data_off + k * S0_PAGE;
        Some(&self.map.bytes()[at..at + S0_PAGE])
    }
}

/// A chunk of zeros.
const ZERO: [u8; CHUNK_BYTES] = [0; CHUNK_BYTES];

/// What S₀ is compared with: a base file, the engine's format point (a
/// base being made from it), or a copy of the point (a base made now, for
/// the writer thread).
enum Against<'a> {
    File(BaseFile),
    Point(flashtex_engine::arena::View<'a>),
    Copy(Vec<u32>, Vec<u8>),
}

impl Against<'_> {
    /// Chunk `c` of the space as the base holds it.
    fn chunk(&self, c: u32) -> &[u8] {
        let per = (S0_PAGE / CHUNK_BYTES) as u32;
        match self {
            Against::File(b) => match b.page(c / per) {
                Some(pg) => {
                    let i = (c % per) as usize;
                    &pg[i * CHUNK_BYTES..(i + 1) * CHUNK_BYTES]
                }
                None => &ZERO,
            },
            // (a chunk the arena never wrote is zero)
            Against::Point(v) => v.chunk(c as usize),
            Against::Copy(cs, d) => match cs.binary_search(&c) {
                Ok(i) => &d[i * CHUNK_BYTES..(i + 1) * CHUNK_BYTES],
                Err(_) => &ZERO,
            },
        }
    }
}

/// The nonzero chunks of the space at checkpoint `view` (`touched`: the
/// arena's, now: a chunk never written is zero then too).
fn nonzero(arena: &flashtex_engine::arena::Arena, view: &flashtex_engine::arena::View) -> Vec<u32> {
    (0..arena.chunks())
        .filter(|&c| arena.touched(c) && view.chunk(c).iter().any(|&b| b != 0))
        .map(|c| c as u32)
        .collect()
}

/// Split S₀'s nonzero `chunks` (`mine`: S₀'s chunk, of a space of
/// `space` chunks) into the chunks of the pages it holds itself and of
/// those equal, byte for byte, to the base's, which it refers to.
fn split<'a>(
    space: usize,
    chunks: &[u32],
    mine: &dyn Fn(u32) -> &'a [u8],
    against: &Against,
) -> (Vec<u32>, Vec<u32>) {
    let per = (S0_PAGE / CHUNK_BYTES) as u32;
    let equal =
        |p: u32| (p * per..((p + 1) * per).min(space as u32)).all(|c| mine(c) == against.chunk(c));
    let (mut own, mut from_base) = (vec![], vec![]);
    let mut k = 0;
    while k < chunks.len() {
        let p = chunks[k] / per;
        let mut j = k;
        while j < chunks.len() && chunks[j] / per == p {
            j += 1;
        }
        if equal(p) {
            from_base.extend_from_slice(&chunks[k..j]);
        } else {
            own.extend_from_slice(&chunks[k..j]);
        }
        k = j;
    }
    (own, from_base)
}

/// S₀'s header around `body` (from [`header`]): the magic and the base it
/// refers to (none when no page comes from one) first, where
/// [`base_of`] reads it, and the chunks it takes from the base last.
fn finish_head(body: Vec<u8>, base: Option<(String, [u64; 2])>, from_base: &[u32]) -> Vec<u8> {
    let mut head = vec![];
    MAGIC.to_vec().enc(&mut head);
    base.filter(|_| !from_base.is_empty()).enc(&mut head);
    head.extend_from_slice(&body);
    from_base.to_vec().enc(&mut head);
    head
}

/// The base an S₀ file refers to (its name in the cache), read from the
/// file's first bytes; `None` for no base, or a file of no S₀ of this
/// version.
pub fn base_of(xs0: &Path) -> Option<String> {
    use std::io::Read;
    let mut b = vec![];
    std::fs::File::open(xs0)
        .ok()?
        .take(4096)
        .read_to_end(&mut b)
        .ok()?;
    let hlen = u64::from_le_bytes(b.get(..8)?.try_into().ok()?) as usize;
    let mut r = Reader::new(b.get(8..(8 + hlen).min(b.len()))?);
    if Vec::<u8>::dec(&mut r).ok()? != MAGIC {
        return None;
    }
    Option::<(String, [u64; 2])>::dec(&mut r)
        .ok()?
        .map(|(n, _)| n)
}

/// How long a base no S₀ refers to is kept (another host may have just
/// written it, its S₀ not yet).
pub const BASE_GRACE: std::time::Duration = std::time::Duration::from_secs(24 * 3600);

/// Remove the bases in the S₀ cache `dir` that no S₀ there refers to,
/// older than `grace`, but those in `keep` (the bases this process uses).
/// A base another process has mapped stays valid after the unlink on Unix
/// (the mapping keeps the file); on Windows the removal of a file in use
/// fails and the base is left. Returns the bases removed.
pub fn collect_bases(dir: &Path, keep: &[PathBuf], grace: std::time::Duration) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };
    let (mut bases, mut used) = (vec![], std::collections::HashSet::new());
    for e in entries.flatten() {
        let p = e.path();
        match p.extension().and_then(|x| x.to_str()) {
            Some(BASE_EXT) => bases.push(p),
            Some("xs0") => used.extend(base_of(&p)),
            _ => {}
        }
    }
    let now = std::time::SystemTime::now();
    let mut gone = vec![];
    for b in bases {
        if used.contains(&file_name(&b)) || keep.iter().any(|k| k == &b) {
            continue;
        }
        let old = std::fs::metadata(&b)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age >= grace);
        if old && std::fs::remove_file(&b).is_ok() {
            gone.push(b);
        }
    }
    gone
}

/// A base file's name in the cache.
fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A new base's header up to its chunks' list.
fn base_header(nonce: [u64; 2]) -> Vec<u8> {
    let mut head = vec![];
    BASE_MAGIC.to_vec().enc(&mut head);
    nonce.enc(&mut head);
    head
}

/// The base S₀ is taken out against, for the writer.
enum ImageBase {
    /// A base file there is: the writer compares S₀ with it.
    File(PathBuf),
    /// A base made now from the format point: the writer writes it, then
    /// compares S₀ with it.
    New {
        path: PathBuf,
        nonce: [u64; 2],
        chunks: Vec<u32>,
        data: Vec<u8>,
    },
}

/// Take S₀ (checkpoint `id` of `g`, `meta` its key and record) out of the
/// engine: the header and a copy of the space's nonzero chunks at S₀, and
/// the base to write it against (`base`: where the bases are; `None`:
/// none): the base file there is, else a copy of the format point to make
/// one from. The writer ([`Image::write`]) compares, off the engine thread.
pub fn prepare(
    g: &mut Globals,
    id: CheckpointId,
    meta: &Meta,
    base: Option<&BaseSpec>,
) -> Result<Image, String> {
    let head = header(g, id, meta)?;
    let view = g.arena.view_at(id)?;
    let chunks = nonzero(&g.arena, &view);
    let mut data = Vec::with_capacity(chunks.len() * CHUNK_BYTES);
    for &c in &chunks {
        data.extend_from_slice(view.chunk(c as usize));
    }
    let base = base.and_then(|spec| {
        let path = base_path(spec, g);
        // (a base that does not open, truncated or not a base, is made
        // again: written through a rename, it replaces the file)
        if BaseFile::open(&path).is_ok() {
            return Some(ImageBase::File(path));
        }
        let pv = g.arena.view_at(spec.point?).ok()?;
        let chunks = nonzero(&g.arena, &pv);
        let mut data = Vec::with_capacity(chunks.len() * CHUNK_BYTES);
        for &c in &chunks {
            data.extend_from_slice(pv.chunk(c as usize));
        }
        let nonce = new_nonce(&path);
        Some(ImageBase::New {
            path,
            nonce,
            chunks,
            data,
        })
    });
    Ok(Image {
        head,
        space: g.arena.chunks(),
        chunks,
        data,
        base,
    })
}

/// Write S₀ to `path` from the engine itself, with no copy of its chunks
/// (Low Memory: the copy is the size of the file), against the base there
/// is or, made first from the format point, a new one. Returns the file's
/// bytes.
pub fn write_in_place(
    g: &mut Globals,
    id: CheckpointId,
    meta: &Meta,
    path: &str,
    base: Option<&BaseSpec>,
) -> Result<u64, String> {
    let body = header(g, id, meta)?;
    let g: &Globals = g;
    let view = g.arena.view_at(id)?;
    let mut against: Option<(Against, String, [u64; 2])> = None;
    if let Some(spec) = base {
        let bp = base_path(spec, g);
        if let Ok(b) = BaseFile::open(&bp) {
            let nonce = b.nonce;
            against = Some((Against::File(b), file_name(&bp), nonce));
        } else if let Some(Ok(pv)) = spec.point.map(|p| g.arena.view_at(p)) {
            let nonce = new_nonce(&bp);
            let chunks = nonzero(&g.arena, &pv);
            let made = write_file(
                base_header(nonce),
                &chunks,
                &|i| pv.chunk(chunks[i] as usize),
                &bp.to_string_lossy(),
            );
            if made.is_ok() {
                against = Some((Against::Point(pv), file_name(&bp), nonce));
            }
        }
    }
    let chunks = nonzero(&g.arena, &view);
    let (own, from_base, name) = match &against {
        Some((a, name, nonce)) => {
            let (own, from_base) = split(g.arena.chunks(), &chunks, &|c| view.chunk(c as usize), a);
            (own, from_base, Some((name.clone(), *nonce)))
        }
        None => (chunks, vec![], None),
    };
    let head = finish_head(body, name, &from_base);
    write_file(head, &own, &|i| view.chunk(own[i] as usize), path)
}

/// The file's header from its key on, up to the base's chunks
/// ([`finish_head`] puts it together).
fn header(g: &mut Globals, id: CheckpointId, meta: &Meta) -> Result<Vec<u8>, String> {
    let mut head = vec![];
    meta.enc(&mut head);
    let (mgr, full_name, protrusion, handles, dl) =
        g.saved_at(id).ok_or("no host state saved with S0")?;
    mgr.map(|m| m.lookups.clone()).enc(&mut head);
    full_name.enc(&mut head);
    protrusion.enc(&mut head);
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
        self.head.len()
            + self.data.len()
            + match &self.base {
                Some(ImageBase::New { data, .. }) => data.len(),
                _ => 0,
            }
    }

    /// Write the file to `path` through a temporary file of this writer's
    /// own and a rename (`path` is always a whole S₀ or what it was): the
    /// header, the chunks' list and their pages' list, then those pages
    /// whole, `S0_PAGE`-aligned, but the pages equal to the base's. A base
    /// made with it is written first, the same way; a base that cannot be
    /// read or written leaves S₀ whole. Returns the S₀ file's bytes.
    pub fn write(self, path: &str) -> Result<u64, String> {
        let Image {
            head,
            space,
            chunks,
            data,
            base,
        } = self;
        let against = match base {
            None => None,
            Some(ImageBase::File(bp)) => BaseFile::open(&bp).ok().map(|b| {
                let nonce = b.nonce;
                (Against::File(b), file_name(&bp), nonce)
            }),
            Some(ImageBase::New {
                path: bp,
                nonce,
                chunks: bc,
                data: bd,
            }) => write_file(
                base_header(nonce),
                &bc,
                &|i| &bd[i * CHUNK_BYTES..(i + 1) * CHUNK_BYTES],
                &bp.to_string_lossy(),
            )
            .ok()
            .map(|_| (Against::Copy(bc, bd), file_name(&bp), nonce)),
        };
        let mine = |c: u32| -> &[u8] {
            match chunks.binary_search(&c) {
                Ok(i) => &data[i * CHUNK_BYTES..(i + 1) * CHUNK_BYTES],
                Err(_) => &ZERO,
            }
        };
        let (own, from_base, name) = match &against {
            Some((a, name, nonce)) => {
                let (own, from_base) = split(space, &chunks, &mine, a);
                (own, from_base, Some((name.clone(), *nonce)))
            }
            None => (chunks.clone(), vec![], None),
        };
        let head = finish_head(head, name, &from_base);
        write_file(head, &own, &|i| mine(own[i]), path)
    }
}

/// Write the file: `head`, the list of the nonzero `chunks` and of their
/// pages, then those pages whole (`chunk(i)` is the `i`th chunk's bytes).
fn write_file<'a>(
    mut head: Vec<u8>,
    chunks: &[u32],
    chunk: &dyn Fn(usize) -> &'a [u8],
    path: &str,
) -> Result<u64, String> {
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
    for &pg in &pages {
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
        .map(|m| m.len())
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
    /// Of those, the pages mapped from the base.
    pub base_pages: usize,
    /// The base it was opened against.
    pub base: Option<PathBuf>,
}

/// Put the `chunks` of a file's `pages` into the fresh word space of `g`:
/// the file is `file` (`bytes` mapped), its pages' data at `data_off` in
/// the order of `pages`. Each run of pages that follow one another both
/// in the space and in the file is mapped copy-on-write
/// (`Arena::map_pages`), a page past the space's end or on a system that
/// does not map files is copied. Returns the pages mapped.
fn place(
    g: &mut Globals,
    file: &std::fs::File,
    bytes: &[u8],
    data_off: usize,
    pages: &[u32],
    chunks: &[u32],
) -> Result<usize, String> {
    let per = (S0_PAGE / CHUNK_BYTES) as u32;
    let index = |p: u32| pages.binary_search(&p).ok();
    let mut want: Vec<u32> = chunks.iter().map(|&c| c / per).collect();
    want.dedup();
    let mut mapped: std::collections::HashSet<u32> = Default::default();
    let mut i = 0;
    while i < want.len() {
        let k0 = index(want[i]).ok_or("S0 file: a chunk outside its pages")?;
        let mut j = i + 1;
        while j < want.len() && want[j] == want[j - 1] + 1 && index(want[j]) == Some(k0 + (j - i)) {
            j += 1;
        }
        let run: Vec<u32> = want[i..j]
            .iter()
            .copied()
            .take_while(|&p| (p as usize + 1) * S0_PAGE <= g.arena.len_bytes())
            .collect();
        if !run.is_empty()
            && g.arena
                .map_pages(file, (data_off + k0 * S0_PAGE) as u64, &run, S0_PAGE)
        {
            mapped.extend(run);
        }
        i = j;
    }
    for &c in chunks {
        let p = c / per;
        if mapped.contains(&p) {
            g.arena.mark_loaded(c as usize);
        } else {
            let k = index(p).ok_or("S0 file: a chunk outside its pages")?;
            let off = data_off + k * S0_PAGE + (c % per) as usize * CHUNK_BYTES;
            g.arena.load_chunk(
                c as usize,
                bytes
                    .get(off..off + CHUNK_BYTES)
                    .ok_or("S0 file truncated")?,
            );
        }
    }
    Ok(mapped.len())
}

/// Open the S₀ file at `path` into a new engine. `check` runs once the
/// header is read (the resident engine's checks of S₀'s key, command line,
/// format and font directories); `catalog` is the fonts' catalog for the
/// font manager's lookups, asked only when S₀ had a font manager. The base
/// an S₀ refers to is looked for beside it, and must be the very file S₀
/// was written against (its identity).
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
        return Err("not a Unicode-mode S0 file of this version".into());
    }
    let base_ref = Option::<(String, [u64; 2])>::dec(&mut r)?;
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
    let from_base = Vec::<u32>::dec(&mut r)?;
    let own = Vec::<u32>::dec(&mut r)?;
    let pages = Vec::<u32>::dec(&mut r)?;
    let data_off = (8 + hlen).next_multiple_of(S0_PAGE);
    if bytes.len() < data_off + pages.len() * S0_PAGE {
        return Err("S0 file truncated".into());
    }
    let base = match &base_ref {
        None => None,
        Some((name, nonce)) => {
            if name.contains(['/', '\\']) {
                return Err("S0 file: a bad base name".into());
            }
            let bp = Path::new(path)
                .parent()
                .unwrap_or(Path::new("."))
                .join(name);
            let b = BaseFile::open(&bp).map_err(|e| format!("its base: {e}"))?;
            if b.nonce != *nonce {
                return Err("its base was made again".into());
            }
            Some((b, bp))
        }
    };
    let mut g = Globals::new();
    if g.arena.len_bytes() != arena_len || g.arena.scalar_bytes() != scalar_bytes {
        return Err("S0 was saved with another word-space layout".into());
    }
    if own
        .iter()
        .chain(&from_base)
        .any(|&c| c as usize >= g.arena.chunks())
    {
        return Err("S0 file: a chunk outside the space".into());
    }
    // Mapped (copy on write: the pages stay the files', out of the
    // footprint, until the engine writes them) where they can be, the rest
    // copied: S₀'s own pages from its file, the others from the base.
    let mut mapped_pages = place(&mut g, &file, bytes, data_off, &pages, &own)?;
    let mut base_pages = 0;
    if let Some((b, _)) = &base {
        base_pages = place(
            &mut g,
            &b.file,
            b.map.bytes(),
            b.data_off,
            &b.pages,
            &from_base,
        )?;
        mapped_pages += base_pages;
    } else if !from_base.is_empty() {
        return Err("S0 file: chunks of a base it has not".into());
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
        base_pages,
        base: base.map(|(_, bp)| bp),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The collector removes the bases no S₀ refers to that are older than
    /// the grace, and nothing else: not a referenced base, not a recent
    /// one (its S₀ may be on its way), not one the process keeps.
    #[test]
    fn stale_bases_are_collected() {
        let dir = std::env::temp_dir().join(format!("flashtex-s0-gc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 24 * 3600);
        let base = |n: &str, aged: bool| {
            let p = dir.join(n);
            std::fs::write(&p, b"a base").unwrap();
            if aged {
                std::fs::File::options()
                    .write(true)
                    .open(&p)
                    .unwrap()
                    .set_modified(old)
                    .unwrap();
            }
            p
        };
        let s0 = |n: &str, refers: Option<&str>| {
            let head = finish_head(vec![], refers.map(|r| (r.to_string(), [1, 2])), &[7]);
            let mut f = (head.len() as u64).to_le_bytes().to_vec();
            f.extend_from_slice(&head);
            std::fs::write(dir.join(n), f).unwrap();
        };
        let used = base("used.xs0base", true);
        let stale = base("stale.xs0base", true);
        let recent = base("recent.xs0base", false);
        let kept = base("kept.xs0base", true);
        s0("doc1.xs0", Some("used.xs0base"));
        s0("doc2.xs0", None);
        std::fs::write(dir.join("doc3.xs0"), b"not an S0").unwrap();
        assert_eq!(
            base_of(&dir.join("doc1.xs0")).as_deref(),
            Some("used.xs0base")
        );
        assert_eq!(base_of(&dir.join("doc2.xs0")), None);
        assert_eq!(base_of(&dir.join("doc3.xs0")), None);
        let gone = collect_bases(&dir, std::slice::from_ref(&kept), BASE_GRACE);
        assert_eq!(gone, vec![stale.clone()]);
        assert!(used.is_file() && recent.is_file() && kept.is_file() && !stale.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
