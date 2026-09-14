//! Measurement-only deep heap accounting (FT-070).
//!
//! Answers "where did the resident memory go" with a number per structure
//! instead of a guess. Nothing here is on a render path: it is read-only,
//! walks already-built structures, and is called by the performance harness
//! and by tests. It must never influence output.
//!
//! Counted bytes are *allocated* bytes — `Vec::capacity()`, not `len()` —
//! because that is what the allocator is holding. Shared owners (`Rc`) are
//! charged once, to the first walk that reaches them, so the totals of the
//! display list and the render cache can be added without double counting.

use std::collections::HashSet;

use crate::display::{Cluster, DisplayList, GlyphRun, Item, Page, Provenance};
use crate::incremental::RenderCache;

/// One named line of the breakdown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub name: &'static str,
    pub bytes: u64,
    /// How many of the thing (glyphs, clusters, runs, pages, blocks).
    pub count: u64,
}

/// A breakdown: named lines, in the order they were recorded.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub lines: Vec<Line>,
    /// `Rc` allocations already charged, so a second walk does not
    /// re-count a shared owner.
    seen: HashSet<usize>,
}

impl Report {
    pub fn new() -> Report {
        Report::default()
    }

    fn add(&mut self, name: &'static str, bytes: u64, count: u64) {
        if let Some(l) = self.lines.iter_mut().find(|l| l.name == name) {
            l.bytes += bytes;
            l.count += count;
        } else {
            self.lines.push(Line { name, bytes, count });
        }
    }

    /// True the first time this address is offered; charges a shared owner
    /// exactly once.
    fn first_time(&mut self, addr: usize) -> bool {
        self.seen.insert(addr)
    }

    pub fn total(&self) -> u64 {
        self.lines.iter().map(|l| l.bytes).sum()
    }

    pub fn get(&self, name: &str) -> u64 {
        self.lines.iter().find(|l| l.name == name).map_or(0, |l| l.bytes)
    }

    pub fn count_of(&self, name: &str) -> u64 {
        self.lines.iter().find(|l| l.name == name).map_or(0, |l| l.count)
    }

    /// `name  bytes (MiB)  count`, biggest line first.
    pub fn table(&self) -> String {
        let mut rows = self.lines.clone();
        rows.sort_by(|a, b| b.bytes.cmp(&a.bytes));
        let total = self.total().max(1);
        let mut s = String::new();
        s.push_str(&format!(
            "{:<38} {:>13} {:>9} {:>7}  {}\n",
            "structure", "bytes", "MiB", "%", "count"
        ));
        for r in rows {
            s.push_str(&format!(
                "{:<38} {:>13} {:>9.1} {:>6.1}%  {}\n",
                r.name,
                r.bytes,
                r.bytes as f64 / (1024.0 * 1024.0),
                100.0 * r.bytes as f64 / total as f64,
                r.count
            ));
        }
        s.push_str(&format!(
            "{:<38} {:>13} {:>9.1} {:>6.1}%\n",
            "TOTAL",
            self.total(),
            self.total() as f64 / (1024.0 * 1024.0),
            100.0
        ));
        s
    }
}

fn vec_bytes<T>(v: &[T]) -> u64 {
    // `capacity` is what the allocator holds; a slice only exposes `len`,
    // so callers pass the `Vec` and this sees its full spare capacity via
    // the monomorphised helper below.
    (v.len() * std::mem::size_of::<T>()) as u64
}

fn vec_cap_bytes<T>(v: &Vec<T>) -> u64 {
    (v.capacity() * std::mem::size_of::<T>()) as u64
}

/// Heap held by a `Provenance` *beyond* the inline enum, which the
/// `Cluster` already pays for.
fn provenance_heap(p: &Provenance, r: &mut Report) {
    match p {
        // A source range is now an index and two offsets: 12 bytes inline,
        // no heap at all. The document paths are one table per display
        // list, charged with the list rather than per cluster.
        Provenance::Source(_) => {}
        Provenance::Sources(v) => {
            r.add(
                "Provenance::Sources spill (boxed)",
                std::mem::size_of::<Vec<crate::display::SourceRange>>() as u64 + vec_cap_bytes(v),
                v.len() as u64,
            );
        }
        Provenance::Synthetic(s) => r.add(
            "Provenance::Synthetic string (boxed)",
            std::mem::size_of::<String>() as u64 + s.capacity() as u64,
            1,
        ),
    }
}

fn walk_glyph_run(run: &GlyphRun, r: &mut Report) {
    r.add("GlyphRun count (header in Item slot)", 0, 1);
    r.add("GlyphRun.text (ActualText)", run.text.capacity() as u64, 0);
    r.add("GlyphRun.glyphs", vec_cap_bytes(&run.glyphs), run.glyphs.len() as u64);

    // The cluster vector, split by what is inside a `Cluster`, so the
    // per-glyph carets/hit_rects/sources can be read off directly.
    let n = run.clusters.len() as u64;
    let cap = run.clusters.capacity() as u64;
    let each = std::mem::size_of::<Cluster>() as u64;
    let rect = std::mem::size_of::<crate::display::Rect>() as u64;
    let prov = std::mem::size_of::<Provenance>() as u64;
    let rest = each - rect - prov;
    r.add("Cluster.hit_rect (per glyph)", cap * rect, n);
    r.add("Cluster.provenance inline (per glyph)", cap * prov, n);
    r.add("Cluster.text_start/end (per glyph)", cap * rest, n);
    // One per run that ends a word, where there used to be a caret pair on
    // every cluster.
    r.add(
        "GlyphRun.end_caret (per run)",
        std::mem::size_of::<Option<crate::display::EndCaret>>() as u64,
        u64::from(run.end_caret.is_some()),
    );

    let addr = std::rc::Rc::as_ptr(&run.font_id) as *const u8 as usize;
    if r.first_time(addr) {
        r.add("GlyphRun.font_id Rc<str> (shared)", run.font_id.len() as u64 + 16, 1);
    }
    for c in &run.clusters {
        provenance_heap(&c.provenance, r);
    }
}

/// Heap held by one display item, charged to named lines.
pub fn walk_item(item: &Item, r: &mut Report) {
    match item {
        Item::GlyphRun(run) => walk_glyph_run(run, r),
        Item::Rule(rule) => {
            r.add("Rule", 0, 1);
            provenance_heap(&rule.provenance, r);
        }
        Item::Path(p) => {
            let mut clip = 0;
            for c in &p.clips {
                clip += vec_cap_bytes(&c.commands);
            }
            r.add("PathItem.commands", vec_cap_bytes(&p.commands), p.commands.len() as u64);
            r.add("PathItem.clips", vec_cap_bytes(&p.clips) + clip, p.clips.len() as u64);
            provenance_heap(&p.provenance, r);
        }
        Item::Image(i) => {
            let addr = std::rc::Rc::as_ptr(&i.resource) as usize;
            if r.first_time(addr) {
                r.add(
                    "ImageResource (shared)",
                    std::mem::size_of::<crate::display::ImageResource>() as u64 + i.resource.path.capacity() as u64,
                    1,
                );
            }
            provenance_heap(&i.provenance, r);
        }
    }
}

fn walk_page(p: &Page, r: &mut Report) {
    r.add("Page.items vec (Item slots)", vec_cap_bytes(&p.items), p.items.len() as u64);
    for it in &p.items {
        walk_item(it, r);
    }
}

/// The display list's deep heap, split per structure.
pub fn display_list(dl: &DisplayList, r: &mut Report) {
    r.add("DisplayList.pages vec", vec_cap_bytes(&dl.pages), dl.pages.len() as u64);
    for p in &dl.pages {
        walk_page(p, r);
    }
    let mut diag = vec_cap_bytes(&dl.diagnostics);
    for d in &dl.diagnostics {
        diag += d.code.capacity() as u64 + d.message.capacity() as u64 + vec_cap_bytes(&d.sources);
        diag += d.recovery.as_ref().map_or(0, |s| s.capacity() as u64);
    }
    r.add("DisplayList.diagnostics", diag, dl.diagnostics.len() as u64);
    let mut fonts = vec_cap_bytes(&dl.fonts);
    for f in &dl.fonts {
        fonts += f.sha256.capacity() as u64 + f.format.capacity() as u64 + f.postscript_name.capacity() as u64;
        fonts += f.path.as_ref().map_or(0, |s| s.capacity() as u64);
    }
    r.add("DisplayList.fonts", fonts, dl.fonts.len() as u64);
    let mut docs = vec_cap_bytes(&dl.documents);
    for d in &dl.documents {
        docs += d.path.capacity() as u64 + d.sha256.capacity() as u64;
    }
    r.add("DisplayList.documents", docs, dl.documents.len() as u64);
    let _ = vec_bytes::<u8>(&[]);
}

/// The render cache's deep heap, split per sub-cache. Call this *after*
/// [`display_list`] on the same [`Report`] so shared `Rc`s are charged to
/// the display list and the cache's own lines are the additional cost.
pub fn render_cache(cache: &RenderCache, r: &mut Report) {
    let mut spine = 0u64;
    for rc in cache.debug_assembled() {
        let addr = std::rc::Rc::as_ptr(&rc) as usize;
        if !r.first_time(addr) {
            continue;
        }
        spine += vec_cap_bytes(&rc.lines);
        for line in &rc.lines {
            spine += vec_cap_bytes(line);
            for it in line {
                walk_assembled_item(it, r);
            }
        }
        spine += vec_cap_bytes(&rc.faces);
        for (a, b, _) in &rc.resources {
            spine += a.capacity() as u64 + b.capacity() as u64;
        }
        spine += vec_cap_bytes(&rc.resources) + vec_cap_bytes(&rc.unmapped);
    }
    r.add("RenderCache.assembled spine", spine, cache.debug_assembled_len() as u64);

    let mut recs = 0u64;
    let mut rec_text = 0u64;
    let mut rec_clusters = 0u64;
    let mut rec_glyphs = 0u64;
    let mut maths = 0u64;
    let mut other = 0u64;
    let mut blk_items = 0u64;
    let mut blk_recs = 0u64;
    let mut blk_labels = 0u64;
    for rc in cache.debug_blocks() {
        let addr = std::rc::Rc::as_ptr(&rc) as usize;
        if !r.first_time(addr) {
            continue;
        }
        recs += vec_cap_bytes(&rc.recs);
        for rec in &rc.recs {
            if let crate::typeset::BoxRec::Text { text, clusters, glyphs, .. } = rec {
                rec_text += text.capacity() as u64;
                rec_clusters += vec_cap_bytes(clusters);
                rec_glyphs += vec_cap_bytes(glyphs);
            }
        }
        maths += vec_cap_bytes(&rc.maths);
        other += rc.path.capacity() as u64 + vec_cap_bytes(&rc.diagnostics);
        blk_items += vec_cap_bytes(&rc.block.items);
        blk_recs += vec_cap_bytes(&rc.block.recs);
        blk_labels += vec_cap_bytes(&rc.block.labels);
    }
    r.add("RenderCache.blocks BoxRec slots", recs, 0);
    r.add("RenderCache.blocks BoxRec::Text.text", rec_text, 0);
    r.add("RenderCache.blocks ClusterRec", rec_clusters, 0);
    r.add("RenderCache.blocks GlyphRec", rec_glyphs, 0);
    r.add("RenderCache.blocks MathRec slots", maths, 0);
    r.add("RenderCache.blocks BuiltBlock.items", blk_items, 0);
    r.add("RenderCache.blocks BuiltBlock.recs", blk_recs, 0);
    r.add("RenderCache.blocks BuiltBlock.labels", blk_labels, 0);
    r.add("RenderCache.blocks path+diagnostics", other, cache.debug_blocks_len() as u64);

    let mut adapted = 0u64;
    for rc in cache.debug_adapted() {
        let addr = std::rc::Rc::as_ptr(&rc) as usize;
        if !r.first_time(addr) {
            continue;
        }
        adapted += vec_cap_bytes(&rc.items);
    }
    r.add("RenderCache.adapted", adapted, cache.debug_adapted_len() as u64);
}

/// The cache's own copy of the display shapes, reported on its own lines so
/// it is never confused with the display list's.
fn walk_assembled_item(item: &Item, r: &mut Report) {
    match item {
        Item::GlyphRun(run) => {
            r.add("AssembledBlock GlyphRun.text", run.text.capacity() as u64, 0);
            r.add("AssembledBlock GlyphRun.glyphs", vec_cap_bytes(&run.glyphs), run.glyphs.len() as u64);
            r.add("AssembledBlock Cluster vec", vec_cap_bytes(&run.clusters), run.clusters.len() as u64);
            for c in &run.clusters {
                provenance_heap(&c.provenance, r);
            }
        }
        other => walk_item(other, r),
    }
}

/// Static sizes of the per-glyph structures, for the record.
pub fn struct_sizes() -> Vec<(&'static str, usize)> {
    use crate::display::{Caret, Carets, Glyph, Rect, SourceRange};
    vec![
        ("Glyph", std::mem::size_of::<Glyph>()),
        ("Cluster", std::mem::size_of::<Cluster>()),
        ("Carets", std::mem::size_of::<Carets>()),
        ("Caret", std::mem::size_of::<Caret>()),
        ("Rect", std::mem::size_of::<Rect>()),
        ("Provenance", std::mem::size_of::<Provenance>()),
        ("SourceRange", std::mem::size_of::<SourceRange>()),
        ("Item", std::mem::size_of::<Item>()),
        ("GlyphRun", std::mem::size_of::<GlyphRun>()),
        ("Paint", std::mem::size_of::<crate::display::Paint>()),
        ("DeviceColor", std::mem::size_of::<flashtex_compiler::color::DeviceColor>()),
        ("Rule", std::mem::size_of::<crate::display::Rule>()),
        ("PathItem", std::mem::size_of::<crate::display::PathItem>()),
        ("Image", std::mem::size_of::<crate::display::Image>()),
        ("pl::Item", std::mem::size_of::<flashtex_paragraph_layout::Item>()),
        ("BoxRec", std::mem::size_of::<crate::typeset::BoxRec>()),
        ("ClusterRec", std::mem::size_of::<crate::typeset::ClusterRec>()),
        ("GlyphRec", std::mem::size_of::<crate::typeset::GlyphRec>()),
        ("MathRec", std::mem::size_of::<crate::typeset::MathRec>()),
    ]
}

/// FT-070: how many carets a display list carries, and on how many runs.
///
/// This began as an audit that compared every stored `Cluster.carets`
/// against the value derived from the cluster's own geometry. Over the
/// twelve corpus cases — 1 991 552 clusters — the start caret matched in
/// every single one, the end caret never sat on a cluster other than its
/// run's last, and its `top`/`height` always equalled that cluster's hit
/// rect. That is what licensed deriving them instead of storing them
/// (`display::Carets`), so the comparison has nothing left to compare and
/// what remains is the census.
#[derive(Debug, Default, Clone, Copy)]
pub struct CaretAudit {
    pub clusters: u64,
    pub runs: u64,
    /// Runs that end a word, and so carry an end caret.
    pub runs_with_end_caret: u64,
}

pub fn audit_carets(dl: &DisplayList, a: &mut CaretAudit) {
    for p in &dl.pages {
        for it in &p.items {
            let Item::GlyphRun(run) = it else { continue };
            a.runs += 1;
            a.clusters += run.clusters.len() as u64;
            if run.end_caret.is_some() {
                a.runs_with_end_caret += 1;
            }
        }
    }
}
