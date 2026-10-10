//! Embedded Type 1 fonts kept across runs in this process (lane COLD-FIXED).
//!
//! Every run writes its fonts at the end of the document, which a resident
//! host re-runs at every keystroke (DESIGN.md §5.3: `\end{document}` always
//! re-runs), so the same fonts were subset again and again with the same
//! glyphs: decrypting the font, walking the charstrings, encrypting the
//! subset (51 M of a 600 M-instruction keystroke on a four-page article).
//!
//! [`embed`] is [`subset::embed`] memoised. Its result is a function of
//! what it reads, all of which is the key: the font program's bytes (by
//! content hash), the map entry's slant, extension and subset flag, the
//! descriptor's font name, glyph sets (`gl_tree`, `tx_tree`), all-glyphs flag
//! and font dimensions as they come in, the persistent `lastargOtherSubr3`,
//! and where the font buffer starts. A hit gives the same bytes, the same
//! changes to the descriptor and the same `lastargOtherSubr3` afterwards.
//!
//! What it asks of the engine is replayed, not reused: each call to
//! [`Host`] is recorded in order, and on a hit made again. The subset tag
//! depends on the tags this run has already given out (`make_subset_tag`
//! avoids collisions), so it is asked for again with the same glyphs and
//! name, and written into the font name where the run put it; a warning
//! (`pdftex_warn`: a subr that does not end in `return`) goes to the log
//! again. So a font from the cache is the font a run from scratch writes.
//!
//! The cache is bounded by the performance mode (`set_limit`: 4 MB of fonts
//! in Low Memory, 32 MB in Balanced, 128 MB in High Performance; the oldest
//! go first) and is off with `FLASHTEX_T1_CACHE=0`.

use super::subset::{self, Embedded, Job};
use super::{Host, Result};
use crate::pdftex::fonts::GlyphNames;
use crate::pdftex::writefont::IntParm;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::Mutex;

/// What [`subset::embed`] reads.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Key {
    font: [u64; 2],
    font_len: usize,
    slant: i32,
    extend: i32,
    subsetted: bool,
    fontname: Option<Vec<u8>>,
    all_glyphs: bool,
    font_dim: Vec<(i32, bool)>,
    tx_tree: Option<BTreeSet<i32>>,
    gl_tree: Option<BTreeSet<Vec<u8>>>,
    last_arg_other_subr3: i32,
    fb_base: i32,
}

/// A call `embed` made to its [`Host`], in order.
#[derive(Clone)]
enum Call {
    Warn(Vec<u8>),
    Tag {
        glyphs: BTreeSet<Vec<u8>>,
        fontname: Vec<u8>,
        tag: [u8; 6],
    },
}

/// What `embed` gave and changed.
#[derive(Clone)]
struct Entry {
    bytes: Vec<u8>,
    length1: i32,
    length2: i32,
    /// Where the subset tag is in `bytes`, if it was written there.
    tag_at: Option<usize>,
    calls: Vec<Call>,
    fontname: Option<Vec<u8>>,
    subset_tag: Option<[u8; 6]>,
    font_dim: Vec<IntParm>,
    gl_tree: Option<BTreeSet<Vec<u8>>>,
    builtin_glyph_names: Option<GlyphNames>,
    last_arg_other_subr3: i32,
}

impl Entry {
    fn size(&self) -> usize {
        self.bytes.len() + 4096
    }
}

/// The bytes of fonts kept, at most (`set_limit`: the performance mode's).
static LIMIT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(32 << 20);

fn limit() -> usize {
    LIMIT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Keep at most `bytes` of fonts (`incr::Options::t1_cache`); the oldest go
/// at once if more are kept.
pub fn set_limit(bytes: usize) {
    LIMIT.store(bytes, std::sync::atomic::Ordering::Relaxed);
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(c) = c.as_mut() {
        while c.bytes > bytes {
            let Some(old) = c.order.pop_front() else {
                break;
            };
            if let Some(o) = c.map.remove(&old) {
                c.bytes -= o.size();
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Hits on this thread (the tests).
    static HITS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[derive(Default)]
struct Cache {
    map: HashMap<Key, Entry>,
    /// Insertion order, oldest first.
    order: VecDeque<Key>,
    bytes: usize,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("FLASHTEX_T1_CACHE").as_deref() != Ok("0"))
}

/// Records each call on its way to the engine.
struct Recording<'h> {
    inner: &'h mut dyn Host,
    calls: Vec<Call>,
}

impl Host for Recording<'_> {
    fn warn(&mut self, msg: &[u8]) {
        self.calls.push(Call::Warn(msg.to_vec()));
        self.inner.warn(msg);
    }

    fn subset_tag(&mut self, glyphs: &BTreeSet<Vec<u8>>, fontname: &[u8]) -> [u8; 6] {
        let tag = self.inner.subset_tag(glyphs, fontname);
        self.calls.push(Call::Tag {
            glyphs: glyphs.clone(),
            fontname: fontname.to_vec(),
            tag,
        });
        tag
    }
}

/// [`subset::embed`], from the cache when this process has embedded the
/// same font program the same way before.
pub(super) fn embed(font: &[u8], job: Job, host: &mut dyn Host) -> Result<Embedded> {
    if !enabled() {
        return subset::embed(font, job, host);
    }
    let key = Key {
        font: crate::persist::hash128(font),
        font_len: font.len(),
        slant: job.fm.slant,
        extend: job.fm.extend,
        subsetted: job.fm.is_subsetted(),
        fontname: job.fd.fontname.clone(),
        all_glyphs: job.fd.all_glyphs,
        font_dim: job.fd.font_dim.iter().map(|d| (d.val, d.set)).collect(),
        tx_tree: job.fd.tx_tree.clone(),
        gl_tree: job.fd.gl_tree.clone(),
        last_arg_other_subr3: job.persist.last_arg_other_subr3,
        fb_base: job.fb_base,
    };
    let hit = CACHE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|c| c.map.get(&key).cloned());
    if let Some(e) = hit {
        #[cfg(test)]
        HITS.with(|h| h.set(h.get() + 1));
        return Ok(replay(e, job, host));
    }
    let Job {
        fm,
        fd,
        persist,
        fb_base,
    } = job;
    let mut rec = Recording {
        inner: host,
        calls: vec![],
    };
    let job = Job {
        fm,
        fd: &mut *fd,
        persist: &mut *persist,
        fb_base,
    };
    let out = subset::embed(font, job, &mut rec)?;
    let e = Entry {
        bytes: out.bytes.clone(),
        length1: out.length1,
        length2: out.length2,
        tag_at: out.tag_at,
        calls: rec.calls,
        fontname: fd.fontname.clone(),
        subset_tag: fd.subset_tag,
        font_dim: fd.font_dim.to_vec(),
        gl_tree: fd.gl_tree.clone(),
        builtin_glyph_names: fd.builtin_glyph_names.clone(),
        last_arg_other_subr3: persist.last_arg_other_subr3,
    };
    keep(key, e);
    Ok(out)
}

fn keep(key: Key, e: Entry) {
    let size = e.size();
    let limit = limit();
    if size > limit {
        return;
    }
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let c = c.get_or_insert_with(Cache::default);
    while c.bytes + size > limit {
        let Some(old) = c.order.pop_front() else {
            break;
        };
        if let Some(o) = c.map.remove(&old) {
            c.bytes -= o.size();
        }
    }
    c.bytes += size;
    c.order.push_back(key.clone());
    c.map.insert(key, e);
}

/// A hit: the engine is asked what it was asked when the entry was made,
/// in the same order, and the descriptor and `lastargOtherSubr3` are left
/// as `embed` left them.
fn replay(e: Entry, job: Job, host: &mut dyn Host) -> Embedded {
    let mut bytes = e.bytes;
    let mut subset_tag = e.subset_tag;
    for call in &e.calls {
        match call {
            Call::Warn(msg) => host.warn(msg),
            Call::Tag {
                glyphs,
                fontname,
                tag,
            } => {
                let now = host.subset_tag(glyphs, fontname);
                if now != *tag {
                    if let Some(slot) = e.tag_at.and_then(|at| bytes.get_mut(at..at + 6)) {
                        slot.copy_from_slice(&now);
                    }
                }
                subset_tag = Some(now);
            }
        }
    }
    let fd = job.fd;
    fd.fontname = e.fontname;
    fd.subset_tag = subset_tag;
    fd.font_dim.copy_from_slice(&e.font_dim);
    fd.gl_tree = e.gl_tree;
    fd.builtin_glyph_names = e.builtin_glyph_names;
    job.persist.last_arg_other_subr3 = e.last_arg_other_subr3;
    Embedded {
        bytes,
        length1: e.length1,
        length2: e.length2,
        tag_at: e.tag_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdftex::mapfile::{FmEntry, F_SUBSETTED, F_TYPE1};
    use crate::pdftex::writefont::FdEntry;
    use crate::pdftex::writet1::Persist;

    /// A host that gives out the tags it is handed, in turn, and records
    /// every call.
    struct TagHost {
        tags: Vec<[u8; 6]>,
        calls: Vec<String>,
    }

    impl TagHost {
        fn new(tags: &[&[u8; 6]]) -> TagHost {
            TagHost {
                tags: tags.iter().rev().map(|t| **t).collect(),
                calls: vec![],
            }
        }
    }

    impl Host for TagHost {
        fn warn(&mut self, msg: &[u8]) {
            self.calls
                .push(format!("warn {}", String::from_utf8_lossy(msg)));
        }

        fn subset_tag(&mut self, glyphs: &BTreeSet<Vec<u8>>, fontname: &[u8]) -> [u8; 6] {
            let tag = self.tags.pop().unwrap_or(*b"ZZZZZZ");
            self.calls.push(format!(
                "tag {:?} {} -> {}",
                glyphs
                    .iter()
                    .map(|g| String::from_utf8_lossy(g).into_owned())
                    .collect::<Vec<_>>(),
                String::from_utf8_lossy(fontname),
                String::from_utf8_lossy(&tag)
            ));
            tag
        }
    }

    /// What a run leaves: the font, the descriptor's fields `embed` sets,
    /// `lastargOtherSubr3` and the host's calls.
    #[derive(Debug, PartialEq)]
    struct Out {
        bytes: Vec<u8>,
        lengths: (i32, i32),
        fontname: Option<Vec<u8>>,
        subset_tag: Option<[u8; 6]>,
        font_dim: Vec<(i32, bool)>,
        gl_tree: Option<BTreeSet<Vec<u8>>>,
        builtin: Option<GlyphNames>,
        persist: i32,
        calls: Vec<String>,
    }

    fn run(cached: bool, glyphs: &[&str], persist_in: i32, tags: &[&[u8; 6]]) -> Out {
        let font = super::super::subset::tests::pfa();
        let mut fm = FmEntry::new();
        fm.typ = F_SUBSETTED | F_TYPE1;
        let mut fd = FdEntry {
            gl_tree: Some(glyphs.iter().map(|g| g.as_bytes().to_vec()).collect()),
            ..Default::default()
        };
        let mut persist = Persist {
            last_arg_other_subr3: persist_in,
        };
        let mut host = TagHost::new(tags);
        let job = Job {
            fm: &fm,
            fd: &mut fd,
            persist: &mut persist,
            fb_base: 0,
        };
        let out = if cached {
            embed(&font, job, &mut host)
        } else {
            subset::embed(&font, job, &mut host)
        }
        .unwrap();
        Out {
            bytes: out.bytes,
            lengths: (out.length1, out.length2),
            fontname: fd.fontname,
            subset_tag: fd.subset_tag,
            font_dim: fd.font_dim.iter().map(|d| (d.val, d.set)).collect(),
            gl_tree: fd.gl_tree,
            builtin: fd.builtin_glyph_names,
            persist: persist.last_arg_other_subr3,
            calls: host.calls,
        }
    }

    fn hits() -> u32 {
        HITS.with(|h| h.get())
    }

    /// A font from the cache is the font `subset::embed` makes, with the
    /// same changes to the descriptor and the same calls to the engine.
    #[test]
    fn a_hit_is_what_a_run_makes() {
        let direct = run(false, &["A"], 11, &[b"AAAAAA"]);
        let first = run(true, &["A"], 11, &[b"AAAAAA"]);
        let h = hits();
        let second = run(true, &["A"], 11, &[b"AAAAAA"]);
        assert_eq!(hits(), h + 1, "the second run is a hit");
        assert_eq!(first, direct);
        assert_eq!(second, direct);
        assert!(direct.calls[0].starts_with("tag [\"A\"] Test -> AAAAAA"));
    }

    /// The tag is the one the engine gives now (another font may have taken
    /// the old one), written where the run writes it.
    #[test]
    fn a_hit_takes_the_tag_given_now() {
        run(true, &["A", "B"], 3, &[b"AAAAAA"]);
        let h = hits();
        let again = run(true, &["A", "B"], 3, &[b"QWERTY"]);
        assert_eq!(hits(), h + 1);
        let direct = run(false, &["A", "B"], 3, &[b"QWERTY"]);
        assert_eq!(again, direct);
        assert_eq!(again.subset_tag, Some(*b"QWERTY"));
        assert!(find_bytes(&again.bytes, b"/FontName /QWERTY+Test def"));
    }

    /// Warnings go to the engine again, in their place among the calls.
    #[test]
    fn warnings_are_given_again() {
        let direct = run(false, &["A", "nosuch"], 3, &[b"AAAAAA"]);
        assert!(direct
            .calls
            .iter()
            .any(|c| c == "warn glyph `nosuch' undefined"));
        run(true, &["A", "nosuch"], 3, &[b"AAAAAA"]);
        let h = hits();
        let again = run(true, &["A", "nosuch"], 3, &[b"AAAAAA"]);
        assert_eq!(hits(), h + 1);
        assert_eq!(again, direct);
    }

    /// Another glyph set, or another `lastargOtherSubr3`, is another key.
    #[test]
    fn other_inputs_are_other_fonts() {
        run(true, &["B"], 3, &[b"AAAAAA"]);
        let h = hits();
        let other = run(true, &["A", "B", "nosuch"], 3, &[b"AAAAAA"]);
        assert_eq!(hits(), h, "a miss");
        assert_eq!(other, run(false, &["A", "B", "nosuch"], 3, &[b"AAAAAA"]));
        let other = run(true, &["B"], 7, &[b"AAAAAA"]);
        assert_eq!(hits(), h, "a miss");
        assert_eq!(other, run(false, &["B"], 7, &[b"AAAAAA"]));
    }

    fn find_bytes(hay: &[u8], needle: &[u8]) -> bool {
        hay.windows(needle.len()).any(|w| w == needle)
    }
}
