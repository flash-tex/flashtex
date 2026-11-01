//! The simulated "typesetting" hot loop.
//!
//! Each simulated page is a stream of reads and writes over the flat word space. The
//! stream is generated once and replayed identically against every mechanism, so the
//! only difference between mechanisms is the cost of an access, never the work done.
//!
//! # Where the locality model comes from
//!
//! DESIGN Appendix B.1, measured on pdfTeX: 768k `mem` words in use on a 183-page
//! document, **about 58 KB freed per `\shipout`**, 2.4 ms per body page. That 58 KB is
//! the key number: TeX's `get_node` takes nodes off the rover free list, so a page's
//! allocation churn is reused storage in a region that drifts slowly forward, not fresh
//! storage scattered over all of `mem`. A page's *newly dirtied* `mem` is therefore tens
//! of kilobytes, while a page's *read* set is much larger (font metrics, `eqtb`, the
//! node graph of material already built).
//!
//! So the model is:
//!
//! * `mem` — writes land in a drifting window (`win_words`, default 16,384 words =
//!   128 KB, about 2x the measured 58 KB churn), touched at node granularity (2–8
//!   consecutive words, as `get_node` hands out node records). A `graph_mult` multiple
//!   of the window behind the head models re-writing nodes of material already built
//!   (`hpack`/`vpack` writing `glue_set`, `\output` shifting boxes). A `spread`
//!   fraction goes uniformly over all of `mem`: box registers, token lists, marks and
//!   inserts, which really are scattered.
//! * `eqtb` — concentrated writes: `eq_define` hits the integer/dimen/glue parameter
//!   block (the first `EQTB_HOT` words) most of the time, with a tail over codes and
//!   registers.
//! * `save_stack` — sequential near the group nesting depth, which oscillates.
//! * `str_pool` — append at the top (`\csname`, `\the`, file names).
//! * `font_info` — reads only; scattered by glyph, clustered by font.
//! * `hash` — reads only; scattered by control-sequence name.
//!
//! `Locality::uniform()` is the pessimal control: every write uniform over its arena.
//! Both are reported, because the answer to §5.2 depends on which one is true.

use crate::layout::{Layout, Range};

#[derive(Clone, Copy, Debug)]
pub struct Op(u64);

const WRITE_BIT: u64 = 1 << 63;

impl Op {
    #[inline(always)]
    pub fn read(i: usize) -> Op {
        Op(i as u64)
    }
    #[inline(always)]
    pub fn write(i: usize) -> Op {
        Op(i as u64 | WRITE_BIT)
    }
    #[inline(always)]
    pub fn index(self) -> usize {
        (self.0 & !WRITE_BIT) as usize
    }
    #[inline(always)]
    pub fn is_write(self) -> bool {
        self.0 & WRITE_BIT != 0
    }
}

/// Fraction knobs are per-mille so the generator stays integer-only and deterministic.
#[derive(Clone, Copy, Debug)]
pub struct Locality {
    pub name: &'static str,
    /// Words in the drifting `mem` allocation window.
    pub win_words: usize,
    /// The "already built" region behind the head, as a multiple of `win_words`.
    pub graph_mult: usize,
    /// Per-mille of `mem` writes that go uniformly over all of `mem`.
    pub mem_write_spread_pm: u32,
    /// Per-mille of `mem` reads that go uniformly over all of `mem`.
    pub mem_read_spread_pm: u32,
    /// Per-mille of `eqtb` writes that hit the hot parameter block.
    pub eqtb_hot_pm: u32,
}

/// `eqtb` words that `eq_define` hits overwhelmingly: `int_pars`, `dimen_pars`,
/// `glue_pars` and the local font/lang state. Two 16 KB chunks' worth.
pub const EQTB_HOT: usize = 4096;

impl Locality {
    /// Anchored on DESIGN B.1 (58 KB freed per shipout).
    pub fn tex_freelist() -> Locality {
        Locality {
            name: "tex-freelist",
            win_words: 16_384,
            graph_mult: 8,
            mem_write_spread_pm: 20,
            mem_read_spread_pm: 150,
            eqtb_hot_pm: 900,
        }
    }

    /// Pessimal control: no locality at all.
    pub fn uniform() -> Locality {
        Locality {
            name: "uniform",
            win_words: 0,
            graph_mult: 0,
            mem_write_spread_pm: 1000,
            mem_read_spread_pm: 1000,
            eqtb_hot_pm: 0,
        }
    }

    /// A named mid-point, for the sensitivity sweep: locality present but weaker.
    pub fn with_spread(mut self, pm: u32) -> Locality {
        self.name = "tex-freelist(swept)";
        self.mem_write_spread_pm = pm;
        self
    }
}

/// How much of the state a page touches, and how the touches split.
#[derive(Clone, Copy, Debug)]
pub struct PageShape {
    /// Per-mille of all state words touched (read or written) per page. DESIGN's
    /// assignment: 1–5%, i.e. 10–50 per mille.
    pub touch_pm: u32,
    /// Per-mille of ops that are writes. TeX's inner loops are read-heavy.
    pub write_pm: u32,
}

impl PageShape {
    pub fn touch(pct: u32) -> PageShape {
        PageShape {
            touch_pm: pct * 10,
            write_pm: 250,
        }
    }

    /// A page whose *write* set is the given percentage of state words, for the
    /// "memory per checkpoint at 1% and 5% dirty per page" question. Every `mem` access
    /// becomes a write; `eqtb`, `save_stack` and `str_pool` keep their own read shares,
    /// and `hash`/`font_info` stay read-only because nothing writes them mid-page.
    pub fn dirty(pct: u32) -> PageShape {
        PageShape {
            touch_pm: pct * 10,
            write_pm: 1000,
        }
    }
    pub fn ops(&self, layout: &Layout) -> usize {
        (layout.total_words as u64 * self.touch_pm as u64 / 1000) as usize
    }
}

/// Where the ops of one page go, by arena. Per-mille of the page's op budget.
/// Reads dominate `font_info` and `hash`; writes dominate `mem`, `eqtb`, `save_stack`.
struct Mix {
    mem: u32,
    eqtb: u32,
    hash: u32,
    save_stack: u32,
    str_pool: u32,
    font_info: u32,
}

const MIX: Mix = Mix {
    mem: 600,
    eqtb: 140,
    hash: 90,
    save_stack: 60,
    str_pool: 30,
    font_info: 80,
};

/// SplitMix64 — deterministic, fast enough that generation is not the bottleneck.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    #[inline(always)]
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    #[inline(always)]
    pub fn below(&mut self, n: usize) -> usize {
        // Lemire-style multiply-shift; the modulo bias is irrelevant here.
        ((self.next() as u128 * n as u128) >> 64) as usize
    }
    #[inline(always)]
    pub fn pm(&mut self, per_mille: u32) -> bool {
        self.below(1000) < per_mille as usize
    }
}

/// Generates the op stream. `head` drifts forward across pages, wrapping, exactly as
/// TeX's rover walks `mem`.
pub struct Generator {
    layout: Layout,
    loc: Locality,
    shape: PageShape,
    rng: Rng,
    head: usize,
    save_depth: usize,
    pool_top: usize,
    font: usize,
}

impl Generator {
    pub fn new(layout: Layout, loc: Locality, shape: PageShape, seed: u64) -> Generator {
        Generator {
            layout,
            loc,
            shape,
            rng: Rng::new(seed),
            head: 0,
            save_depth: 0,
            pool_top: 0,
            font: 0,
        }
    }

    pub fn ops_per_page(&self) -> usize {
        self.shape.ops(&self.layout)
    }

    #[inline(always)]
    fn in_range(&mut self, r: Range) -> usize {
        r.at(self.rng.below(r.len))
    }

    /// One page's worth of ops, appended to `out` (cleared first).
    ///
    /// The arenas are interleaved op by op, not emitted arena-by-arena: a real engine
    /// reads `font_info` for a glyph, writes the char node into `mem`, reads `eqtb` for
    /// `\hyphenchar` and pushes `save_stack`, all within a few instructions. Emitting
    /// them in blocks and shuffling afterwards would have destroyed each arena's own
    /// locality (the drifting `mem` window, the `save_stack` walk, the `str_pool`
    /// append), which is precisely what the benchmark is measuring.
    pub fn page(&mut self, out: &mut Vec<Op>) {
        out.clear();
        let n = self.ops_per_page();
        let mem = self.layout.mem;
        let eqtb = self.layout.eqtb;
        let hash = self.layout.hash;
        let save = self.layout.save_stack;
        let pool = self.layout.str_pool;
        let font_info = self.layout.font_info;

        let win = self.loc.win_words.min(mem.len);
        let graph = (win * self.loc.graph_mult).min(mem.len);
        let hot = EQTB_HOT.min(eqtb.len);
        let per_font = (font_info.len / 16).max(1);

        // The window drifts forward by roughly one window per page: the free list has
        // moved on. Wrap, because mem is reused, not grown.
        if win > 0 {
            self.head = (self.head + win) % mem.len;
        }
        let head = self.head;

        // Cumulative per-mille arena weights, for one weighted draw per step.
        let c_mem = MIX.mem;
        let c_eqtb = c_mem + MIX.eqtb;
        let c_hash = c_eqtb + MIX.hash;
        let c_save = c_hash + MIX.save_stack;
        let c_pool = c_save + MIX.str_pool;
        let c_font = c_pool + MIX.font_info;
        debug_assert_eq!(c_font, 1000);

        while out.len() < n {
            let pick = self.rng.below(1000) as u32;
            if pick < c_mem {
                // mem: node allocation, node mutation, scattered long-lived writes.
                let is_write = self.rng.pm(self.shape.write_pm);
                let spread_pm = if is_write {
                    self.loc.mem_write_spread_pm
                } else {
                    self.loc.mem_read_spread_pm
                };
                let off = if win == 0 || self.rng.pm(spread_pm) {
                    // Scattered: box registers, token lists, marks, inserts.
                    self.rng.below(mem.len)
                } else if self.rng.pm(800) {
                    // In the allocation window: get_node / free_node churn.
                    (head + self.rng.below(win)) % mem.len
                } else {
                    // Behind the head: material already built, being repacked.
                    let back = self.rng.below(graph.max(1));
                    (head + mem.len - back) % mem.len
                };
                // Node records are 2–8 contiguous words; touch a whole one.
                let node = 2 + 2 * self.rng.below(4);
                for k in 0..node {
                    if out.len() >= n {
                        break;
                    }
                    let idx = mem.at((off + k) % mem.len);
                    out.push(if is_write {
                        Op::write(idx)
                    } else {
                        Op::read(idx)
                    });
                }
            } else if pick < c_eqtb {
                // eqtb: eq_define, concentrated on the parameter block. Written more
                // often than mem, because grouping assigns constantly.
                let idx = if self.rng.pm(self.loc.eqtb_hot_pm) {
                    eqtb.at(self.rng.below(hot))
                } else {
                    self.in_range(eqtb)
                };
                out.push(if self.rng.pm(450) {
                    Op::write(idx)
                } else {
                    Op::read(idx)
                });
            } else if pick < c_hash {
                // hash: control-sequence lookup, read-only.
                out.push(Op::read(self.in_range(hash)));
            } else if pick < c_save {
                // save_stack: push/pop around an oscillating nesting depth.
                if self.rng.pm(500) {
                    self.save_depth = (self.save_depth + 2).min(save.len - 8);
                } else {
                    self.save_depth = self.save_depth.saturating_sub(2);
                }
                let idx = save.at(self.save_depth + self.rng.below(4));
                out.push(if self.rng.pm(500) {
                    Op::write(idx)
                } else {
                    Op::read(idx)
                });
            } else if pick < c_pool {
                // str_pool: append at the top.
                if self.rng.pm(700) {
                    self.pool_top = (self.pool_top + 1) % pool.len;
                    out.push(Op::write(pool.at(self.pool_top)));
                } else {
                    out.push(Op::read(self.in_range(pool)));
                }
            } else {
                // font_info: char metrics, read-only, clustered per font.
                if self.rng.pm(40) {
                    self.font = (self.font + 1) % 16;
                }
                let base = self.font * per_font;
                out.push(Op::read(
                    font_info.at((base + self.rng.below(per_font)) % font_info.len),
                ));
            }
        }
        out.truncate(n);
    }
}

/// Distinct 16 KB chunks written by a stream — the memory a checkpoint costs under
/// either copy-on-write mechanism, since both copy at 16 KB granularity on this host.
pub struct DirtySet {
    bits: Vec<u64>,
    pub count: usize,
}

impl DirtySet {
    pub fn new(chunks: usize) -> DirtySet {
        DirtySet {
            bits: vec![0; chunks.div_ceil(64)],
            count: 0,
        }
    }
    #[inline(always)]
    pub fn mark_chunk(&mut self, c: usize) {
        let w = c >> 6;
        let b = 1u64 << (c & 63);
        if self.bits[w] & b == 0 {
            self.bits[w] |= b;
            self.count += 1;
        }
    }
}
