//! Convergence (DESIGN.md §5.3, docs/design/xetex/INCREMENTAL.md §5 item
//! 5): a run restarted from S₀ or a page checkpoint compares its state
//! after each page with the last complete run's after the same page; where
//! they are the same up to where dynamic memory put things, the rest of the
//! old run stands.
//!
//! Step 1, measurement only (`FLASHTEX_UNICODE_CONV_DEBUG=1`): the test
//! runs at each page checkpoint of a restarted run until it passes (where
//! a jump would be made), and the run goes on to its end as without it.
//! At the end the claim is checked against what the run made: every page
//! after the convergence point must be the old run's page (its display
//! list frame). Notes go to standard error, prefixed `[conv]`.
//!
//! The test, after the pdfTeX engine's (`crates/flashtex-engine/src/incr.rs`
//! `IncrRun::test` and `same_words`):
//! 1. the file globals: the same files at the same places (an edited
//!    file's offset after the edit, moved by it), the same external
//!    effects and run state;
//! 2. the reads: the old run reads no changed file from here on, makes no
//!    lookup whose answer changed, and reads no file either run writes (up
//!    to its last page checkpoint; `\end{document}`'s reads would run live);
//! 3. the state outside the word space ([`Globals::host_vs`]);
//! 4. the word space: the differing words, less dead ones ([`dead_word`])
//!    and positions in the XDV stream (which only feeds the display list,
//!    page by page, in PDF mode), must be free cells or lie in what the
//!    structural comparison (`crate::iso`) walks, and that comparison must
//!    find the two states alike.

use crate::ckpt::Record;
use crate::generated::Globals;
use crate::iso::{EqtbScan, HostSide, Iso};
use crate::statediff::{ScalarSlot, WordDiff};
use crate::system::Stream;
use flashtex_engine::arena::{Branch, CheckpointId, ChunkDiff};
use flashtex_engine::system::ReadLog;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

/// An old page checkpoint after the restart point.
pub struct OldPage {
    pub pages: usize,
    pub id: CheckpointId,
    pub rec: Record,
    pub reads: (usize, usize, usize),
}

/// An edit of a user's file: bytes `prefix..old_end` of the old text
/// became `prefix..old_end + delta` of the new.
#[derive(Clone, Debug)]
pub struct Edit {
    pub path: String,
    pub prefix: u64,
    pub old_end: u64,
    pub delta: i64,
}

impl Edit {
    /// The edit between `old` and `new`.
    pub fn between(path: &str, old: &[u8], new: &[u8]) -> Edit {
        let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
        let room = old.len().min(new.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(room)
            .take_while(|(a, b)| a == b)
            .count();
        Edit {
            path: path.to_string(),
            prefix: prefix as u64,
            old_end: (old.len() - suffix) as u64,
            delta: new.len() as i64 - old.len() as i64,
        }
    }
}

/// What the restarted run needs of the last complete one.
pub struct Old {
    pub branch: Branch,
    pub pages: Vec<OldPage>,
    /// The user's files' edits, and every file read since S₀ that changed
    /// (an edit's or another's).
    pub edits: Vec<Edit>,
    pub changed: HashSet<String>,
    /// The first lookup since S₀ whose answer changed (its index).
    pub lookup: Option<usize>,
    /// The old run's read journal.
    pub journal: Option<ReadLog>,
    /// Its source map's span and file tables.
    pub tables: crate::dlsource::Tables,
    /// The `eqtb` pointer entries of the restore target.
    pub eqtb_pointers: Arc<Vec<i32>>,
    /// Where the test passed: (pages shipped, index into `pages`).
    pub claim: Option<(usize, usize)>,
    /// Each test: pages shipped, the answer, milliseconds.
    pub tests: Vec<(usize, Result<usize, String>, f64)>,
}

impl Old {
    pub fn summary(&self) -> String {
        let ms: f64 = self.tests.iter().map(|t| t.2).sum();
        let fails: Vec<String> = self
            .tests
            .iter()
            .filter_map(|(p, r, ms)| r.as_ref().err().map(|e| format!("p{p} {ms:.1}ms: {e}")))
            .take(6)
            .collect();
        format!(
            "{} tests, {:.1} ms; claim {:?}; {}",
            self.tests.len(),
            ms,
            self.claim.map(|c| c.0),
            fails.join(" | ")
        )
    }
}

/// Whether the measurement is on.
pub fn on() -> bool {
    std::env::var_os("FLASHTEX_UNICODE_CONV_DEBUG").is_some_and(|v| v != "0")
}

impl Globals {
    /// After the page checkpoint of page `pages`: the test against the old
    /// run's, until one passes.
    pub(crate) fn conv_test(&mut self, pages: usize) {
        let Some(c) = self.host.resident.as_mut().and_then(|r| r.conv.take()) else {
            return;
        };
        let mut c = c;
        if c.claim.is_none() {
            if let Some(i) = c.pages.iter().position(|p| p.pages == pages) {
                let t = Instant::now();
                let r = self.conv_check(&c, i);
                let ms = t.elapsed().as_secs_f64() * 1e3;
                eprintln!("[conv] page {pages}: {r:?} ({ms:.2} ms)");
                if r.is_ok() {
                    c.claim = Some((pages, i));
                }
                c.tests.push((pages, r, ms));
            }
        }
        if let Some(r) = self.host.resident.as_mut() {
            r.conv = Some(c);
        }
    }

    /// `FLASHTEX_UNICODE_CHECKMEM=1`: walk the live state from the roots
    /// (`Iso::walk_one`) and print the allocated cells nothing reaches: a
    /// root or node field the structural comparison misses (the pdfTeX
    /// engine's `checkmem`).
    pub(crate) fn check_mem(&mut self, page: usize) {
        if std::env::var_os("FLASHTEX_UNICODE_CHECKMEM").is_none() {
            return;
        }
        let layout = crate::statediff::scalar_layout(self);
        let g: &Globals = self;
        let w = |p: usize| g.mem[p].to_bits();
        let Some(free) = free_cells(&w, g.avail, g.rover, g.lo_mem_max, g.hi_mem_min, g.mem_end)
        else {
            eprintln!("[checkmem] page {page}: the free lists cannot be read");
            return;
        };
        if let Ok((nodes, missed, n_missed, n_leaked)) =
            Iso::classify_unreached(g, &layout, &free, g.hyph_list.len(), 12)
        {
            eprintln!(
                "[checkmem] page {page}: {n_missed} cells referenced but unreached {missed:?}, {n_leaked} leaked ({nodes} nodes)"
            );
        }
        match Iso::walk_one(g, &layout, &free, g.hyph_list.len(), 12) {
            Err(e) => eprintln!("[checkmem] page {page}: walk failed: {e}"),
            Ok((nodes, missed, count)) => {
                let cells: Vec<String> = missed
                    .iter()
                    .map(|&p| format!("{p}:{:#018x}", g.mem[p as usize].to_bits()))
                    .collect();
                eprintln!(
                    "[checkmem] page {page}: {count} allocated cells unreached ({nodes} nodes): {}",
                    cells.join(" ")
                );
                if count > 0 {
                    for &m in missed.iter().take(1) {
                        let who = crate::iso::who_points(g, &layout, &[m], 60);
                        eprintln!("[checkmem]   {m} pointed at from: {}", who.join(" "));
                    }
                }
            }
        }
    }

    fn conv_check(&mut self, c: &Old, i: usize) -> Result<usize, String> {
        let op = &c.pages[i];
        let new = &self
            .host
            .resident
            .as_ref()
            .and_then(|r| r.pages.last())
            .ok_or("no page checkpoint")?
            .rec;
        // 1. the file globals
        if op.rec.files.len() != new.files.len() {
            return Err("file globals differ".into());
        }
        for (a, b) in op.rec.files.iter().zip(&new.files) {
            if !a.alike_but_stream(b) {
                let mut x = format!("{a:?} vs {b:?}");
                x.truncate(400);
                return Err(format!("a file global differs: {x}"));
            }
            match (&a.stream, &b.stream) {
                (Stream::In { path, offset: x }, Stream::In { path: p, offset: y }) => {
                    if path != p {
                        return Err(format!("reading {p}, the old run {path}"));
                    }
                    match c.edits.iter().find(|e| e.path == *path) {
                        Some(e) => {
                            if *x < e.old_end || *x as i64 + e.delta != *y as i64 {
                                return Err(format!(
                                    "{p}: at {y}, the old run at {x} (edit {e:?})"
                                ));
                            }
                        }
                        None if c.changed.contains(path) => {
                            return Err(format!("reading the changed {p}"));
                        }
                        None => {
                            if x != y {
                                return Err(format!("{p}: at {y}, the old run at {x}"));
                            }
                        }
                    }
                }
                (Stream::Out { path, .. }, Stream::Out { path: p, .. }) => {
                    if path != p {
                        return Err(format!("writing {p}, the old run {path}"));
                    }
                }
                (Stream::Mem(_), Stream::Mem(_)) => {}
                (Stream::None, Stream::None) | (Stream::Terminal, Stream::Terminal) => {}
                _ => return Err("a file global's stream differs".into()),
            }
        }
        if op.rec.effects != new.effects {
            return Err("an external effect since the restart".into());
        }
        if op.rec.run != new.run {
            return Err("the run state differs".into());
        }
        let read = |e: &[(i32, (Option<String>, u32))]| -> Vec<(i32, (Option<String>, bool))> {
            e.iter()
                .map(|(h, s)| {
                    (
                        *h,
                        crate::native::layout::LayoutEngine::shaping_state_read(s),
                    )
                })
                .collect()
        };
        if read(&op.rec.engines) != read(&new.engines) {
            let (a, b) = (read(&op.rec.engines), read(&new.engines));
            let d: Vec<_> = a.iter().zip(&b).filter(|(x, y)| x != y).take(3).collect();
            return Err(format!(
                "a layout engine's shaping state differs ({} vs {}: {d:?})",
                a.len(),
                b.len()
            ));
        }
        // 2. the reads
        if c.lookup.is_some_and(|l| l >= op.reads.1) {
            return Err("the old run makes a lookup later whose answer changed".into());
        }
        let last = c.pages.last().map_or(op.reads.0, |p| p.reads.0);
        if let Some(j) = &c.journal {
            let written: HashSet<&str> = j.outputs.iter().map(String::as_str).collect();
            let later = &j.files[op.reads.0.min(j.files.len())
                ..last.min(j.files.len()).max(op.reads.0.min(j.files.len()))];
            for f in later {
                if c.changed.contains(&f.path) {
                    return Err(format!("the old run reads the changed {} later", f.path));
                }
                if written.contains(f.path.as_str()) {
                    return Err(format!(
                        "the old run reads {} later, which the runs write",
                        f.path
                    ));
                }
            }
        } else {
            return Err("no journal of the old run".into());
        }
        // 3. outside the word space, and 4. the word space
        let (old_handles, old_dl) = self.host_vs(op.id).map(|(h, d)| (h.clone(), d.clone()))?;
        let t0 = Instant::now();
        let d = self.arena.diff_branch(&c.branch, op.id)?;
        let t_diff = t0.elapsed().as_secs_f64() * 1e3;
        let words = crate::statediff::words(self, &d);
        let n_words = words.len();
        let layout = crate::statediff::scalar_layout(self);
        let g: &Globals = self;
        let left: Vec<WordDiff> = words
            .into_iter()
            .filter(|w| !dead_word(g, w) && !position_only(w))
            .collect();
        let (left, (free_o, free_n)) = drop_free_mem(g, &d, &layout, left);
        if let Some(w) = left.iter().find(|w| w.region != "mem" && !iso_covers(g, w)) {
            let n = left
                .iter()
                .filter(|w| w.region != "mem" && !iso_covers(g, w))
                .count();
            let some: Vec<String> = left
                .iter()
                .filter(|w| w.region != "mem" && !iso_covers(g, w))
                .take(6)
                .map(|w| w.to_string())
                .collect();
            let _ = w;
            return Err(format!(
                "{n} words differ outside what the structural comparison reads: {}",
                some.join(", ")
            ));
        }
        let bad_mem: Vec<usize> = left
            .iter()
            .filter(|w| w.region == "mem")
            .map(|w| w.index)
            .collect();
        let written = d.written();
        let mut written_bits = vec![0u64; g.arena.chunks().div_ceil(64) + 1];
        for &ch in &written {
            written_bits[ch as usize >> 6] |= 1 << (ch & 63);
        }
        let glyphs = |a: i32, b: i32| -> bool {
            match (old_handles.glyph_info(a), g.host.handles.glyph_info(b)) {
                (None, None) => a == 0 && b == 0,
                (Some(x), Some(y)) => Arc::ptr_eq(x, y) || **x == **y,
                _ => false,
            }
        };
        let live_tables = g.host.dl.tables();
        let side = |a: i32, b: i32| -> bool {
            let (x, y) = (old_dl.at(a), g.host.dl.at(b));
            x == y && x == 0 || c.tables.place(x) == live_tables.place(y)
        };
        let host = HostSide {
            glyphs: Some(&glyphs),
            side_eq: g.host.dl.on().then_some(&side as &dyn Fn(i32, i32) -> bool),
            eqtb_scan: Some(EqtbScan {
                written: &written,
                written_bits: &written_bits,
                pointers: &c.eqtb_pointers,
            }),
        };
        let mut never = || false;
        let t1 = Instant::now();
        let r = Iso::check(
            g,
            &d,
            &layout,
            free_o.as_deref(),
            free_n.as_deref(),
            &bad_mem,
            None,
            g.hyph_list.len(),
            host,
            &mut never,
        )
        .map_err(|e| format!("structures differ: {e}"));
        eprintln!(
            "[conv]   diff {t_diff:.2} ms ({} chunks, {n_words} words), words {:.2} ms, walk {:.2} ms",
            d.differing.len(),
            (t1 - t0).as_secs_f64() * 1e3 - t_diff,
            t1.elapsed().as_secs_f64() * 1e3
        );
        r
    }
}

/// Arrays whose elements from a pointer on are dead between two commands
/// (the pdfTeX engine's `live_len`, with XeTeX's arrays).
fn live_len(g: &Globals, region: &str) -> Option<usize> {
    let n = match region {
        "str_pool" => g.pool_ptr,
        "buffer" => g.first,
        // (`str_start` is indexed from the first string after the 65536
        // characters)
        "str_start" => g.str_ptr - 65536 + 1,
        "save_stack" => g.save_ptr,
        "input_stack" => g.input_ptr,
        "nest" => g.nest_ptr,
        "param_stack" => g.param_ptr,
        "pstack" | "trick_buf" => 0,
        "active_width" | "cur_active_width" | "background" | "break_width" | "minimal_demerits"
        | "best_place" | "best_pl_line" | "disc_width" => 0,
        // (`hyf`: set by `hyphenate`, l. 22260, and by `\patterns`
        // and `\hyphenation` as they scan, before they read it)
        "hc" | "hu" | "hyf" => 0,
        // XeTeX: the text of the native word being collected
        "native_text" => g.native_len,
        // changes/ext.ch: `apply_mapping`'s result, read by its caller
        // right after it (xetex.web's `new_native_character` and the main
        // loop's `collected:`), in the same command
        "mapped_text" => 0,
        // the XDV stream: in PDF mode it only feeds the display list, page
        // by page (`out::page_done` takes each page's bytes), and
        // `xdv_buffer` is `ship_out`'s scratch
        "dvi_buf" | "xdv_buffer" => 0,
        _ => return None,
    };
    Some(n.max(0) as usize)
}

/// Whether a differing word is dead between two commands.
fn dead_word(g: &Globals, w: &WordDiff) -> bool {
    match w.scalar {
        Some(
            "max_buf_stack"
            | "max_in_stack"
            | "max_nest_stack"
            | "max_param_stack"
            | "max_save_stack"
            | "best_height_plus_depth"
            | "long_state",
        ) => return true,
        // xetex.web: `line_break`'s copy of `prev_p` for `total_pw`, set by
        // `update_prev_p` as "Find optimal breakpoints" begins, before any
        // `try_break`; the main loop's list registers for native words, set
        // (`main_pp:=tail`, `main_ppp:=head`) before each use, in the
        // command that collects the word
        Some("global_prev_p" | "main_pp" | "main_ppp") => return true,
        // xetex.web (as tex.web): `line_break`'s globals, set as it starts
        // (`init_l_hyf`, `init_r_hyf`, `init_cur_lang` from `prev_graf`
        // and the language, l. 19750) or as each pass starts (`threshold`,
        // `second_pass`, `final_pass`, l. 20828; `l_hyf`, `r_hyf`,
        // l. 21462) before they are read; `disc_width`, set before each
        // `try_break` after a discretionary (l. 20977); `show_context`'s
        // `trick_count` and `first_count` (l. 8092)
        Some(
            "init_l_hyf" | "init_r_hyf" | "init_cur_lang" | "threshold" | "second_pass"
            | "final_pass" | "l_hyf" | "r_hyf" | "disc_width" | "trick_count" | "first_count",
        ) => return true,
        Some(_) => return false,
        None => {}
    }
    let Some(live) = live_len(g, w.region) else {
        return false;
    };
    let (r, rel) = g.arena.region_at(w.off);
    let elem = r.elem.max(1);
    let first = rel / elem;
    if first >= live {
        return true;
    }
    if elem >= 8 {
        return false;
    }
    let per = 8 / elem;
    let mask_bits = (live - first).min(per) * elem * 8;
    let mask = if mask_bits >= 64 {
        u64::MAX
    } else {
        (1u64 << mask_bits) - 1
    };
    (w.old ^ w.new) & mask == 0
}

/// Positions in the XDV stream (PDF mode: the display list takes each page
/// from `last_bop` to `dvi_offset + dvi_ptr` as it ships; no byte count is
/// printed), and the page-local state of `ship_out` (set as each page
/// starts).
fn position_only(w: &WordDiff) -> bool {
    matches!(
        w.scalar,
        Some(
            "dvi_ptr"
                | "dvi_offset"
                | "dvi_gone"
                | "last_bop"
                | "dvi_limit"
                | "half_buf"
                | "max_h"
                | "max_v"
                | "max_push"
                | "dvi_h"
                | "dvi_v"
                | "dvi_f"
        )
    )
}

/// The structural comparison's share (`crate::iso`).
fn iso_covers(g: &Globals, w: &WordDiff) -> bool {
    match w.scalar {
        Some(n) => crate::iso::scalar_covered(n),
        None => {
            matches!(
                w.region,
                "save_stack"
                    | "nest"
                    | "input_stack"
                    | "param_stack"
                    | "cur_mark"
                    | "font_glue"
                    | "hyph_list"
                    | "disc_ptr"
                    | "sa_root"
                    | "if_stack"
            ) || (w.region == "eqtb" && {
                let p = w.index as i32 + 1;
                let high = crate::iso::EQTB_SIZE + 1..=crate::iso::EQTB_SIZE + g.hash_high;
                p < crate::iso::INT_BASE || high.contains(&p)
            })
        }
    }
}

type FreeSets = (Option<Vec<u64>>, Option<Vec<u64>>);

/// The free cells of a state (the pdfTeX engine's `free_cells`).
fn free_cells(
    word: &dyn Fn(usize) -> u64,
    avail: i32,
    rover: i32,
    lo_mem_max: i32,
    hi_mem_min: i32,
    mem_end: i32,
) -> Option<Vec<u64>> {
    const EMPTY_FLAG: u32 = 0x3FFF_FFFF;
    let n = crate::generated::consts::mem_max as usize + 1;
    let mut bits = vec![0u64; n.div_ceil(64)];
    let set = |p: usize, bits: &mut Vec<u64>| bits[p >> 6] |= 1 << (p & 63);
    let rh = |w: u64| (w & 0xFFFF_FFFF) as u32 as i32;
    let lh = |w: u64| (w >> 32) as u32 as i32;
    let in_range = |p: i32| p > 0 && (p as usize) < n;
    let mut p = avail;
    let mut steps = 0usize;
    while p != -0x0FFF_FFFF {
        if !in_range(p) || p < hi_mem_min || p > mem_end || steps > n {
            return None;
        }
        set(p as usize, &mut bits);
        p = rh(word(p as usize));
        steps += 1;
    }
    if rover != 0 {
        let mut p = rover;
        let mut steps = 0usize;
        loop {
            if !in_range(p) || p >= lo_mem_max || steps > n {
                return None;
            }
            let w = word(p as usize);
            if rh(w) as u32 != EMPTY_FLAG {
                return None;
            }
            let size = lh(w);
            if size <= 0 || p + size > lo_mem_max {
                return None;
            }
            set_range(&mut bits, p as usize, (p + size) as usize);
            p = rh(word(p as usize + 1));
            steps += 1;
            if p == rover {
                break;
            }
        }
    }
    set_range(
        &mut bits,
        (lo_mem_max + 1).max(0) as usize,
        hi_mem_min.clamp(0, n as i32) as usize,
    );
    set_range(&mut bits, (mem_end + 1).max(0) as usize, n);
    Some(bits)
}

fn set_range(bits: &mut [u64], lo: usize, hi: usize) {
    let mut i = lo;
    while i < hi {
        if i & 63 == 0 && i + 64 <= hi {
            bits[i >> 6] = u64::MAX;
            i += 64;
        } else {
            bits[i >> 6] |= 1 << (i & 63);
            i += 1;
        }
    }
}

/// Drop the differing `mem` words free in both states, and `avail` and
/// `rover` (the pdfTeX engine's `drop_free_mem`).
fn drop_free_mem(
    g: &Globals,
    d: &ChunkDiff,
    layout: &[ScalarSlot],
    left: Vec<WordDiff>,
) -> (Vec<WordDiff>, FreeSets) {
    if !left
        .iter()
        .any(|w| w.region == "mem" || matches!(w.scalar, Some("avail" | "rover")))
    {
        return (left, (None, None));
    }
    let Some(mem_off) = g
        .arena
        .regions
        .iter()
        .find(|r| r.name == "mem")
        .map(|r| r.off)
    else {
        return (left, (None, None));
    };
    let scalar = |name: &str| -> Option<i32> {
        let s = layout.iter().find(|s| s.name == name)?;
        let w = d.old_word(&g.arena, s.off & !7);
        Some(((w >> ((s.off & 7) * 8)) & 0xFFFF_FFFF) as u32 as i32)
    };
    let (Some(av), Some(ro), Some(lo), Some(hi), Some(me)) = (
        scalar("avail"),
        scalar("rover"),
        scalar("lo_mem_max"),
        scalar("hi_mem_min"),
        scalar("mem_end"),
    ) else {
        return (left, (None, None));
    };
    let old_word = |p: usize| d.old_word(&g.arena, mem_off + p * 8);
    let new_word = |p: usize| g.mem[p].to_bits();
    let Some(fo) = free_cells(&old_word, av, ro, lo, hi, me) else {
        return (left, (None, None));
    };
    let Some(fnw) = free_cells(
        &new_word,
        g.avail,
        g.rover,
        g.lo_mem_max,
        g.hi_mem_min,
        g.mem_end,
    ) else {
        return (left, (Some(fo), None));
    };
    let free_both = |p: usize| (fo[p >> 6] & fnw[p >> 6]) >> (p & 63) & 1 == 1;
    let left = left
        .into_iter()
        .filter(|w| {
            if matches!(w.scalar, Some("avail" | "rover")) {
                return false;
            }
            !(w.region == "mem" && free_both(w.index))
        })
        .collect();
    (left, (Some(fo), Some(fnw)))
}

/// At the end of a measured run: whether the claim holds. `new` are the
/// run's pages, `old` the last complete run's.
pub fn truth(
    c: &Old,
    new: &[Arc<crate::out::doc::Built>],
    old: &[Arc<crate::out::doc::Built>],
) -> String {
    let frame = |b: &crate::out::doc::Built| b.frame.get().cloned();
    // the first page from which on every page is the old run's
    let mut from = new.len();
    if new.len() == old.len() {
        while from > 0
            && frame(&new[from - 1]).is_some()
            && frame(&new[from - 1]) == frame(&old[from - 1])
        {
            from -= 1;
        }
    }
    if new.iter().chain(old).any(|b| b.frame.get().is_none()) {
        return format!(
            "claim {:?}, unchecked (no display list frames)",
            c.claim.map(|c| c.0)
        );
    }
    let verdict = match c.claim {
        None => "no claim".to_string(),
        Some((k, _)) => {
            if new.len() != old.len() {
                format!(
                    "UNSOUND: claim at page {k}, but {} pages vs the old run's {}",
                    new.len(),
                    old.len()
                )
            } else if from <= k {
                format!("claim at page {k} holds")
            } else {
                let (a, b) = (frame(&new[from - 1]), frame(&old[from - 1]));
                let at = match (&a, &b) {
                    (Some(a), Some(b)) => {
                        let i = a.iter().zip(b).take_while(|(x, y)| x == y).count();
                        format!(
                            "{} vs {} bytes, first difference at {i}: {:?} vs {:?}",
                            a.len(),
                            b.len(),
                            String::from_utf8_lossy(
                                &a[i.saturating_sub(40)..(i + 40).min(a.len())]
                            ),
                            String::from_utf8_lossy(
                                &b[i.saturating_sub(40)..(i + 40).min(b.len())]
                            )
                        )
                    }
                    _ => "no frame".into(),
                };
                format!("UNSOUND: claim at page {k}, but page {from} differs ({at})")
            }
        }
    };
    format!(
        "{verdict}; the old run's pages stand from page {} of {}",
        from + 1,
        new.len()
    )
}
