//! Arena layout for the simulated engine state.
//!
//! Engine v2 keeps every mutable arena in one allocator region so that a single
//! `mach_vm_remap` (mechanism (a)) or a single chunk table (mechanism (b)) covers all
//! of it. The benchmark models that: one flat `u64` word space, with the arenas as
//! disjoint ranges inside it. Keeping them disjoint is what matters, because each
//! arena has its own access pattern and therefore its own dirty-chunk footprint.
//!
//! Sizes are TeX's, from DESIGN Appendix B.1 (`mem` 768k words used on a 183-page
//! document, 58 MB peak) and from `tex.web`'s own defaults:
//!
//! | arena        | words   | note                                                |
//! |--------------|---------|-----------------------------------------------------|
//! | `eqtb`       | 60,000  | `eqtb_size` region: pars, codes, registers, macros  |
//! | `hash`       | 65,536  | `hash_prime` 55711 rounded up to a power of two     |
//! | `save_stack` | 80,000  | `save_size`                                         |
//! | `str_pool`   | 262,144 | `pool_size`, packed into `u64` words                |
//! | `font_info`  | 650,000 | `font_mem_size`                                     |
//! | `mem`        | rest    | `mem_min..mem_max`, the dynamic node arena          |
//!
//! `mem` absorbs whatever is left of the configured total, which is how the same
//! layout covers both the DESIGN-stated `mem` range (768k–5M words) and the 64 MB /
//! 200 MB total-state points the assignment asks for.

/// 16 KB: the software copy-on-write chunk size, and the host page size on arm64
/// macOS (`getconf PAGESIZE` = 16384 on mac-m5pro-kabir).
pub const CHUNK_BYTES: usize = 16 * 1024;
pub const CHUNK_WORDS: usize = CHUNK_BYTES / 8; // 2048
pub const CHUNK_SHIFT: u32 = 11;
pub const CHUNK_MASK: usize = CHUNK_WORDS - 1;

pub const EQTB_WORDS: usize = 60_000;
pub const HASH_WORDS: usize = 65_536;
pub const SAVE_STACK_WORDS: usize = 80_000;
pub const STR_POOL_WORDS: usize = 262_144;
pub const FONT_INFO_WORDS: usize = 650_000;
pub const FIXED_WORDS: usize =
    EQTB_WORDS + HASH_WORDS + SAVE_STACK_WORDS + STR_POOL_WORDS + FONT_INFO_WORDS;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Range {
    pub start: usize,
    pub len: usize,
}

impl Range {
    #[inline(always)]
    pub fn at(&self, offset: usize) -> usize {
        debug_assert!(offset < self.len);
        self.start + offset
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub name: &'static str,
    /// Total words, always a whole number of 16 KB chunks.
    pub total_words: usize,
    pub mem: Range,
    pub eqtb: Range,
    pub hash: Range,
    pub save_stack: Range,
    pub str_pool: Range,
    pub font_info: Range,
}

impl Layout {
    /// Build a layout whose total state is `target_bytes`, rounded up to a chunk.
    pub fn with_total_bytes(name: &'static str, target_bytes: usize) -> Layout {
        let words = target_bytes / 8;
        let total_words = words.div_ceil(CHUNK_WORDS) * CHUNK_WORDS;
        assert!(
            total_words > FIXED_WORDS + 64 * 1024,
            "{name}: total state {target_bytes} B leaves no room for mem"
        );
        Layout::place(name, total_words, total_words - FIXED_WORDS)
    }

    /// Build a layout from a `mem` size, as DESIGN §5.2 states it (768k–5M words).
    pub fn with_mem_words(name: &'static str, mem_words: usize) -> Layout {
        let total = (mem_words + FIXED_WORDS).div_ceil(CHUNK_WORDS) * CHUNK_WORDS;
        Layout::place(name, total, total - FIXED_WORDS)
    }

    fn place(name: &'static str, total_words: usize, mem_words: usize) -> Layout {
        // mem first: it is the arena whose locality decides the whole question, and
        // putting it at offset 0 keeps its chunk indices easy to read in traces.
        let mut at = 0usize;
        let mut take = |n: usize| {
            let r = Range { start: at, len: n };
            at += n;
            r
        };
        let mem = take(mem_words);
        let eqtb = take(EQTB_WORDS);
        let hash = take(HASH_WORDS);
        let save_stack = take(SAVE_STACK_WORDS);
        let str_pool = take(STR_POOL_WORDS);
        let font_info = take(FONT_INFO_WORDS);
        assert_eq!(at, total_words);
        Layout {
            name,
            total_words,
            mem,
            eqtb,
            hash,
            save_stack,
            str_pool,
            font_info,
        }
    }

    pub fn total_bytes(&self) -> usize {
        self.total_words * 8
    }
    pub fn chunks(&self) -> usize {
        self.total_words / CHUNK_WORDS
    }

    pub fn describe(&self) -> String {
        format!(
            "{}: {:.1} MiB total, {} chunks of 16 KB; mem {} words ({:.1} MiB), \
             eqtb {}, hash {}, save_stack {}, str_pool {}, font_info {}",
            self.name,
            self.total_bytes() as f64 / (1024.0 * 1024.0),
            self.chunks(),
            self.mem.len,
            self.mem.len as f64 * 8.0 / (1024.0 * 1024.0),
            self.eqtb.len,
            self.hash.len,
            self.save_stack.len,
            self.str_pool.len,
            self.font_info.len,
        )
    }
}

/// The four state sizes reported. The first two pin the DESIGN §5.2 `mem` range; the
/// last two pin the assignment's 64 MB and 200 MB total-state points.
pub fn standard_layouts() -> Vec<Layout> {
    vec![
        Layout::with_mem_words("mem768k", 768_000),
        Layout::with_mem_words("mem5M", 5_000_000),
        Layout::with_total_bytes("total64MB", 64 * 1024 * 1024),
        Layout::with_total_bytes("total200MB", 200 * 1024 * 1024),
    ]
}
