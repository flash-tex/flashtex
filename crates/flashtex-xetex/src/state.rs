//! The engine's state outside the word space (docs/design/xetex/PLAN.md
//! §4.7): what TeX Live's XeTeX keeps in C globals and C pointers.
//!
//! `Globals::host` (web2rust's `--host-state`) holds one [`Host`] per
//! engine; this crate has no process-wide mutable state (`static`,
//! `thread_local!`). The runtime it shares with the pdfTeX engine still has
//! some (`flashtex_engine::system`'s run configuration, resolver, recorder
//! and external-effect log): those move into per-engine state with the
//! shared runtime crate in phase S3 (PLAN.md §3.3), so until then one
//! process runs one XeTeX engine at a time.
//!
//! `Globals` is `Send` (the pdfTeX engine's `Arena` is), so a host may move
//! an engine to another thread: the objects behind handles are `Arc`s of
//! `Send + Sync` values (`tests/send.rs`).
//!
//! The C pointers `xetex.web` keeps in `mem` and in its font arrays (a
//! native word node's glyph-info array, a font's layout engine and TECkit
//! mapping, an OpenType assembly, a picture's path; `changes/ext.ch`) are
//! integer handles into [`Handles`], which a checkpoint saves with the word
//! space ([`Globals::checkpoint`]): restoring one gives back the objects the
//! `mem` words it restores refer to, and the allocator's counter and free
//! list, so that a run continued from a checkpoint allocates the same
//! handles as the run that made it.

use crate::generated::Globals;
use std::collections::HashMap;
use std::sync::Arc;

/// An object a handle refers to. Objects are immutable once allocated (a
/// change is a new object), so a checkpoint shares them with the live
/// engine and copies only the table.
#[derive(Clone)]
pub enum Object {
    /// A native word node's glyph-info array (XeTeX_ext.c's
    /// `native_glyph_info_ptr`: the glyphs' positions, then their ids).
    GlyphInfo(Arc<GlyphInfo>),
    /// A picture's path (XeTeX_pic.c's `pic_path`), until `pic_path_to_mem`
    /// copies it into `mem`.
    PicPath(Arc<[u8]>),
    /// An object of a later phase (a font's layout engine, a TECkit
    /// mapping, an OpenType assembly), held as `Arc<dyn Any + Send + Sync>` so that the
    /// table and its checkpoints need not know its type.
    Other(Arc<dyn std::any::Any + Send + Sync>),
}

/// XeTeX_ext.c's glyph-info array of a native word node: for each glyph its
/// position (`FixedPoint`, x and y in scaled points) and its glyph id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphInfo {
    pub locations: Vec<(i32, i32)>,
    pub ids: Vec<u16>,
}

/// The handle allocator of `changes/ext.ch`: a counter and a free list, and
/// the table behind them. Handle 0 is C's null pointer and never refers to
/// an object.
#[derive(Clone, Default)]
pub struct Handles {
    /// Slot `h - 1` holds the object of handle `h`, `None` once freed.
    table: Vec<Option<Object>>,
    /// Freed handles, reused last freed first.
    free: Vec<i32>,
}

impl Handles {
    /// A new handle for `o`: the last one freed, else the next unused one.
    pub fn alloc(&mut self, o: Object) -> i32 {
        if let Some(h) = self.free.pop() {
            self.table[(h - 1) as usize] = Some(o);
            return h;
        }
        self.table.push(Some(o));
        i32::try_from(self.table.len()).expect("handle table overflow")
    }

    /// The object of handle `h`, if `h` refers to one.
    pub fn get(&self, h: i32) -> Option<&Object> {
        if h <= 0 {
            return None;
        }
        self.table.get((h - 1) as usize)?.as_ref()
    }

    /// Free handle `h` (C's `free`); 0 and a handle already free are
    /// ignored, as `free(NULL)` is.
    pub fn free(&mut self, h: i32) {
        if h <= 0 {
            return;
        }
        if let Some(slot) = self.table.get_mut((h - 1) as usize) {
            if slot.take().is_some() {
                self.free.push(h);
            }
        }
    }

    /// How many handles refer to an object.
    pub fn live(&self) -> usize {
        self.table.iter().filter(|s| s.is_some()).count()
    }

    /// The glyph-info array of handle `h`.
    pub fn glyph_info(&self, h: i32) -> Option<&Arc<GlyphInfo>> {
        match self.get(h)? {
            Object::GlyphInfo(g) => Some(g),
            _ => None,
        }
    }
}

/// The state of one engine outside the word space.
#[derive(Default)]
pub struct Host {
    /// The first line, from the command line (texmfmp.c's `topenin`), as the
    /// raw bytes of the arguments, each followed by a space.
    pub first_line: Option<Vec<u8>>,
    /// `-no-pdf` (xetexextra.c's `nopdfoutput`). Phase S0-S1 always write
    /// XDV, so [`Host::default`] sets it.
    pub no_pdf: NoPdf,
    /// Guards `close_files_and_terminate` against being re-entered.
    pub terminating: bool,
    /// tex.ch's `tex_input_type`: true while `\input` opens a file.
    pub tex_input_type: bool,
    /// openclose.c's `fullnameoffile`: the path the last `open_input`
    /// opened, before `./` is taken off `nameoffile`.
    pub full_name_of_file: Option<String>,
    /// texmfmp.c's `start_time` and `start_time_str`, set once per run.
    pub start: Option<(i64, Vec<u8>)>,
    /// hz.cpp's `leftProt` and `rightProt`: character protrusion codes by
    /// (font, character or glyph, side), for every font (`\lpcode`,
    /// `\rpcode`).
    pub protrusion: Protrusion,
    /// The objects of `changes/ext.ch`'s handles.
    pub handles: Handles,
    /// The host state at each retained checkpoint of the word space. Each
    /// copies the handle table and the protrusion codes (the objects
    /// themselves are shared): O(live handles) per checkpoint, to be made
    /// persistent when S3 brings checkpoints into the Unicode host.
    checkpoints: HashMap<flashtex_engine::arena::CheckpointId, Saved>,
}

/// `-no-pdf`, true unless set otherwise.
#[derive(Clone, Copy)]
pub struct NoPdf(pub bool);

impl Default for NoPdf {
    fn default() -> Self {
        NoPdf(true)
    }
}

/// A protrusion code by (font, character or glyph, side).
pub type Protrusion = HashMap<(i32, u32, i32), i32>;

/// What a checkpoint saves of [`Host`]: the state that the word space's
/// words refer to or that decides what the run does next. The first line
/// and `-no-pdf` are the run's configuration, `terminating` and
/// `tex_input_type` are only true inside one command, and the start time
/// is the run's: none of them changes between checkpoints.
#[derive(Clone)]
struct Saved {
    full_name_of_file: Option<String>,
    protrusion: Protrusion,
    handles: Handles,
}

impl Globals {
    /// A checkpoint of the whole engine: the scalar globals spilled into
    /// the word space, the word space's checkpoint, and the host state the
    /// word space refers to (the handle tables, protrusion codes) saved with
    /// it (PLAN.md §4.7, DESIGN.md §5.2). Valid between commands.
    pub fn checkpoint(&mut self) -> flashtex_engine::arena::CheckpointId {
        self.spill_scalars();
        let id = self.arena.checkpoint();
        let saved = Saved {
            full_name_of_file: self.host.full_name_of_file.clone(),
            protrusion: self.host.protrusion.clone(),
            handles: self.host.handles.clone(),
        };
        self.host.checkpoints.insert(id, saved);
        id
    }

    /// Put checkpoint `id` back: the word space, the scalar globals and the
    /// host state saved with it. Later checkpoints are dropped.
    pub fn restore(&mut self, id: flashtex_engine::arena::CheckpointId) -> Result<(), String> {
        let saved = self
            .host
            .checkpoints
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("no host state saved with checkpoint {id:?}"))?;
        self.arena.restore_discard(id)?;
        self.fill_scalars();
        let kept: Vec<_> = self.arena.checkpoint_ids().to_vec();
        self.host.checkpoints.retain(|k, _| kept.contains(k));
        self.host.full_name_of_file = saved.full_name_of_file;
        self.host.protrusion = saved.protrusion;
        self.host.handles = saved.handles;
        Ok(())
    }

    /// Copy the scalar globals into the word space's scalar region, through
    /// the barrier (as the pdfTeX engine's `spill_scalars`).
    pub fn spill_scalars(&mut self) {
        let mut sp = flashtex_engine::arena::Spill {
            buf: Vec::with_capacity(crate::generated::globals::SCALAR_BYTES),
        };
        self.visit_scalars(&mut sp);
        assert_eq!(sp.buf.len(), crate::generated::globals::SCALAR_BYTES);
        self.arena.write_through(0, &sp.buf);
    }

    /// Read the scalar globals back from the scalar region.
    pub fn fill_scalars(&mut self) {
        let img = self
            .arena
            .read(0, crate::generated::globals::SCALAR_BYTES)
            .to_vec();
        let mut f = flashtex_engine::arena::Fill { src: &img, pos: 0 };
        self.visit_scalars(&mut f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gi(n: u16) -> Object {
        Object::GlyphInfo(Arc::new(GlyphInfo {
            locations: vec![(i32::from(n), 0)],
            ids: vec![n],
        }))
    }

    #[test]
    fn handles_are_reused_last_freed_first_and_zero_is_none() {
        let mut h = Handles::default();
        assert!(h.get(0).is_none());
        let (a, b, c) = (h.alloc(gi(1)), h.alloc(gi(2)), h.alloc(gi(3)));
        assert_eq!((a, b, c), (1, 2, 3));
        h.free(a);
        h.free(c);
        h.free(c); // a second free is ignored, as free(NULL) is
        h.free(0);
        assert_eq!(h.live(), 1);
        assert_eq!(h.alloc(gi(4)), 3);
        assert_eq!(h.alloc(gi(5)), 1);
        assert_eq!(h.alloc(gi(6)), 4);
        assert_eq!(h.glyph_info(1).unwrap().ids, vec![5]);
    }

    /// A checkpoint saves the handle table with the word space: after a
    /// restore, the handle a `mem` word holds refers to the object it
    /// referred to at the checkpoint, and the allocator hands out the same
    /// handles as the run that made the checkpoint.
    #[test]
    fn checkpoint_restores_handles_with_the_word_space() {
        use crate::generated::types::memory_word;
        let mut g = Globals::new();
        let w = 1000usize;
        let a = g.host.handles.alloc(gi(7));
        g.mem[w] = memory_word::from_bits(a as u64);
        g.set_cp_code(1, 65, 0, 50);
        let id = g.checkpoint();
        let next_at_checkpoint = g.host.handles.clone().alloc(gi(0));

        // The run goes on: the node is freed, its handle reused for another
        // array, the word overwritten, a protrusion code changed.
        g.host.handles.free(a);
        let b = g.host.handles.alloc(gi(9));
        assert_eq!(b, a);
        g.mem[w] = memory_word::from_bits(b as u64 + 1);
        g.set_cp_code(1, 65, 0, 80);

        g.restore(id).unwrap();
        let h = g.mem[w].to_bits() as i32;
        assert_eq!(h, a);
        assert_eq!(g.host.handles.glyph_info(h).unwrap().ids, vec![7]);
        assert_eq!(g.get_cp_code(1, 65, 0), 50);
        assert_eq!(g.host.handles.alloc(gi(1)), next_at_checkpoint);
    }
}
