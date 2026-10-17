//! What the runs of a resident engine (`crate::resident`) share of the
//! native fonts they load: the FreeType faces with their HarfBuzz fonts
//! ([`Face`]) and the TECkit mappings, each kept while its file is
//! unchanged. A keystroke's run restarts after the preamble and loads again
//! every font the body selects (NFSS loads a font at its first use); XeTeX
//! opens each file again, which here is a lookup.
//!
//! **What is kept is what a new open would make.** Each value is a pure
//! function of its file's bytes and its key (path and face index; path and
//! direction), and holds no state between the engine's calls that a later
//! answer reads:
//!
//! * a [`Face`] is size-free (the size is the [`FontInst`]'s, made anew at
//!   each load), and what FreeType and HarfBuzz keep inside it is caches of
//!   the file (the glyph slot every call loads before reading it, the
//!   tables, shaper data and shape plans);
//! * a [`Mapping`] is reset after every conversion (`crate::xetex_ext`).
//!
//! The layout engine, which keeps the last shaping's shaper and script
//! (#1790), is never shared: each load makes a new one over the face.
//!
//! **Keyed by the file's identity.** An entry is used while the file's
//! [`StatSig`] (length, modification time, inode) is the one taken before
//! it was opened and again after; a racy signature (within the file
//! system's time tick) equals none, so a file changed within the tick is
//! opened again. The lookup that found the path, and the read journal's
//! note of the file (S₀'s key), are the caller's, as before.
//!
//! `FLASHTEX_UNICODE_FONT_CACHE=0` turns the cache off (every load opens
//! its file, as a run in a process of its own does).
//!
//! [`FontInst`]: super::font_inst::FontInst

use super::font_inst::{Face, FtLibrary};
use crate::xetex_ext::Mapping;
use flashtex_engine::system::StatSig;
use std::collections::HashMap;
use std::sync::Arc;

/// Runs an unused entry is kept for.
const KEEP_RUNS: u64 = 4;

struct Kept<T> {
    sig: StatSig,
    value: Arc<T>,
    /// The run that last used it.
    last: u64,
}

/// The faces and mappings a resident engine's runs loaded (see the
/// module's documentation).
pub struct LoadCache {
    faces: HashMap<(Vec<u8>, u32), Kept<Face>>,
    mappings: HashMap<(String, bool), Kept<Mapping>>,
    run: u64,
    on: bool,
}

impl Default for LoadCache {
    fn default() -> Self {
        LoadCache {
            faces: HashMap::new(),
            mappings: HashMap::new(),
            run: 0,
            on: std::env::var("FLASHTEX_UNICODE_FONT_CACHE").map_or(true, |v| v != "0"),
        }
    }
}

/// `path`'s value in `map` if its file is unchanged, else `make`'s (kept
/// when the file's signature held over the making).
fn get_or_make<K: std::hash::Hash + Eq, T>(
    map: &mut HashMap<K, Kept<T>>,
    on: bool,
    run: u64,
    key: K,
    path: &str,
    make: impl FnOnce() -> Option<T>,
) -> Option<Arc<T>> {
    if !on {
        return make().map(Arc::new);
    }
    let before = StatSig::of(path);
    if let (Some(k), Some(sig)) = (map.get_mut(&key), before.as_ref()) {
        if k.sig == *sig {
            k.last = run;
            return Some(k.value.clone());
        }
    }
    map.remove(&key);
    let value = Arc::new(make()?);
    if let Some(sig) = before {
        if Some(sig) == StatSig::of(path) {
            map.insert(
                key,
                Kept {
                    sig,
                    value: value.clone(),
                    last: run,
                },
            );
        }
    }
    Some(value)
}

impl LoadCache {
    /// A cache that keeps nothing (`on` false), or the default.
    pub fn new(on: bool) -> LoadCache {
        LoadCache {
            on,
            ..LoadCache::default()
        }
    }

    /// Face `index` of the font file at `path`, as [`Face::open`] opens it
    /// (no AFM file is attached: see `create_font_from_file`).
    pub fn face(&mut self, lib: &Arc<FtLibrary>, path: &[u8], index: u32) -> Option<Arc<Face>> {
        let p = String::from_utf8_lossy(path).into_owned();
        get_or_make(
            &mut self.faces,
            self.on,
            self.run,
            (path.to_vec(), index),
            &p,
            || Face::open(lib, path, index, &mut |_| None),
        )
    }

    /// The TECkit mapping compiled in the file at `path` (a byte mapping
    /// for a TFM font, else UTF-16), as `load_mapping_file` makes it (with
    /// the file it was made from).
    pub fn mapping(&mut self, path: &str, byte_mapping: bool) -> Option<Arc<Mapping>> {
        get_or_make(
            &mut self.mappings,
            self.on,
            self.run,
            (path.to_string(), byte_mapping),
            path,
            || {
                let tec = std::fs::read(path).ok()?;
                let mut m = Mapping::new(tec, byte_mapping)?;
                m.1 = Some((path.to_string(), byte_mapping));
                Some(m)
            },
        )
    }

    /// A run ended: entries no run used for [`KEEP_RUNS`] runs go.
    pub fn end_run(&mut self) {
        let run = self.run;
        self.faces.retain(|_, k| run - k.last < KEEP_RUNS);
        self.mappings.retain(|_, k| run - k.last < KEEP_RUNS);
        self.run += 1;
    }

    /// How many faces and mappings are kept.
    pub fn len(&self) -> (usize, usize) {
        (self.faces.len(), self.mappings.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::font_inst::{FontInst, GlyphBBox};
    use crate::native::layout::LayoutEngine;

    fn fonts_dir() -> &'static str {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/mac/Fonts/")
    }

    fn engine(font: FontInst) -> LayoutEngine {
        LayoutEngine::new(
            font,
            u32::from_be_bytes(*b"latn"),
            None,
            vec![],
            None,
            0xFF,
            1.0,
            0.0,
            0.0,
            0,
        )
    }

    type Answers = (
        Vec<u32>,
        Vec<f32>,
        Vec<(f32, f32)>,
        Vec<GlyphBBox>,
        Vec<f32>,
        [f32; 6],
        (Option<String>, u32),
        bool,
    );

    /// What a new layout engine over `font` answers for `text`: its state
    /// before shaping, the shaping, each glyph's bounds and width, and the
    /// instance's metrics.
    fn answers(font: FontInst, text: &str) -> Answers {
        let e = engine(font);
        let fresh = e.shaping_state();
        let u: Vec<u16> = text.encode_utf16().collect();
        e.layout_chars(&u, 0, u.len(), false);
        let glyphs = e.glyphs();
        let bounds = glyphs.iter().map(|&g| e.glyph_bounds(g)).collect();
        let widths = glyphs.iter().map(|&g| e.glyph_width(g)).collect();
        let f = &e.font;
        let metrics = [
            f.point_size,
            f.ascent,
            f.descent,
            f.cap_height,
            f.x_height,
            f.italic_angle,
        ];
        (
            glyphs,
            e.glyph_advances(),
            e.glyph_positions(),
            bounds,
            widths,
            metrics,
            fresh,
            e.using_open_type(),
        )
    }

    /// A face the cache kept, at any size and after other engines shaped
    /// with it, answers as a face opened anew: the same metrics, shaping,
    /// bounds and widths, and a new engine's shaping state (#1790).
    #[test]
    fn a_kept_face_answers_as_a_new_one() {
        let lib = FtLibrary::new();
        for file in ["lmmono10-regular.otf", "latinmodern-math.otf"] {
            let path = format!("{}{file}", fonts_dir());
            let mut cache = LoadCache::new(true);
            let text = "office affluent flow — “naïve” 1234 {}";
            let first = cache.face(&lib, path.as_bytes(), 0).unwrap();
            // other sizes and texts through the kept face
            for (size, other) in [(7.0, "The quick brown fox"), (12.0, "ﬁ ffl Ťŕ"), (10.0, "")]
            {
                let font = FontInst::at_size(first.clone(), path.as_bytes(), 0, size);
                let _ = answers(font, other);
            }
            for size in [10.0f32, 7.0, 14.4] {
                let kept = cache.face(&lib, path.as_bytes(), 0).unwrap();
                assert!(Arc::ptr_eq(&kept, &first), "the face is opened again");
                let shared = answers(FontInst::at_size(kept, path.as_bytes(), 0, size), text);
                let new = FontInst::new(&lib, path.as_bytes(), 0, size, &mut |_| None).unwrap();
                assert_eq!(shared, answers(new, text), "{file} at {size} pt");
            }
            // turned off, every load opens the file
            let mut off = LoadCache::new(false);
            let a = off.face(&lib, path.as_bytes(), 0).unwrap();
            let b = off.face(&lib, path.as_bytes(), 0).unwrap();
            assert!(!Arc::ptr_eq(&a, &b));
            assert_eq!(off.len(), (0, 0));
        }
    }

    /// A kept face is used while its file is unchanged: a file written
    /// again (another font, its modification time set back out of the
    /// racy window) is opened again, a racy one is never kept, and one
    /// unused for `KEEP_RUNS` runs goes.
    #[test]
    fn a_changed_file_is_opened_again() {
        let dir = std::env::temp_dir().join(format!("ft-loadcache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.otf");
        let put = |src: &str, secs_ago: u64| {
            std::fs::copy(format!("{}{src}", fonts_dir()), &path).unwrap();
            let t = std::time::SystemTime::now() - std::time::Duration::from_secs(secs_ago);
            std::fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(t)
                .unwrap();
        };
        put("lmmono10-regular.otf", 3600);
        let p = path.to_string_lossy().into_owned();
        let lib = FtLibrary::new();
        let mut cache = LoadCache::new(true);
        let a = cache.face(&lib, p.as_bytes(), 0).unwrap();
        assert!(Arc::ptr_eq(&a, &cache.face(&lib, p.as_bytes(), 0).unwrap()));
        put("latinmodern-math.otf", 7200);
        let b = cache.face(&lib, p.as_bytes(), 0).unwrap();
        assert!(!Arc::ptr_eq(&a, &b), "a changed file's face was kept");
        // SAFETY: live faces.
        let glyphs = |f: &Arc<Face>| unsafe { (*f.ft_face).num_glyphs };
        assert_ne!(glyphs(&a), glyphs(&b));
        // a racy file (written just now) is never kept
        put("lmmono10-regular.otf", 0);
        let c = cache.face(&lib, p.as_bytes(), 0).unwrap();
        assert!(!Arc::ptr_eq(
            &c,
            &cache.face(&lib, p.as_bytes(), 0).unwrap()
        ));
        put("lmmono10-regular.otf", 3600);
        cache.face(&lib, p.as_bytes(), 0).unwrap();
        assert_eq!(cache.len().0, 1);
        // the run that used it ends, then KEEP_RUNS runs that do not
        for _ in 0..KEEP_RUNS {
            cache.end_run();
        }
        assert_eq!(cache.len().0, 1);
        cache.end_run();
        assert_eq!(cache.len().0, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A kept TECkit mapping converts as one made anew, after other
    /// conversions.
    #[test]
    fn a_kept_mapping_converts_as_a_new_one() {
        use flashtex_engine::resolver::Format;
        let Some(path) = flashtex_engine::system::find_file("tex-text.tec", Format::MiscFonts)
        else {
            eprintln!("skipped: no tex-text.tec (TeX Live)");
            return;
        };
        let mut cache = LoadCache::new(true);
        let kept = cache.mapping(&path, false).unwrap();
        assert_eq!(kept.1, Some((path.clone(), false)));
        assert!(Arc::ptr_eq(&kept, &cache.mapping(&path, false).unwrap()));
        let mut g = crate::generated::Globals::new();
        let convert = |g: &mut crate::generated::Globals, m: &Arc<Mapping>, s: &str| {
            let h = g.host.handles.alloc(crate::state::Object::Other(m.clone()));
            let u: Vec<u16> = s.encode_utf16().collect();
            let n = g.apply_mapping(h, &u);
            g.host.handles.free(h);
            (0..n as usize)
                .map(|k| g.mapped_text[k])
                .collect::<Vec<i32>>()
        };
        let text = "--- `` '' -- ?` !` x---y";
        for other in ["`", "--", "''`", ""] {
            convert(&mut g, &kept, other);
        }
        let new = Arc::new(Mapping::new(std::fs::read(&path).unwrap(), false).unwrap());
        assert_eq!(convert(&mut g, &kept, text), convert(&mut g, &new, text));
    }
}
