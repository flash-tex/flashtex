//! Incremental layout reuse: one typeset block per cache entry.
//!
//! A block's layout (shaped runs, line breaks, box records, math boxes,
//! diagnostics) depends only on its own items, the flags the page builder
//! hands it, and the document style. The key therefore hashes the items
//! with source offsets made relative to the block's first byte, plus those
//! flags and a stylesheet fingerprint. On a hit the cached records are
//! cloned back with every source offset shifted by the block's new base,
//! so the output is byte-identical to a fresh compile: nothing in layout
//! depends on an absolute offset except the provenance fields, and those
//! are all relocated (`relocate_block`). Blocks whose items straddle two
//! documents are never cached. Diagnostics captured while a block was
//! built are replayed at the same point with the same once-only keys.
//!
//! The cache is bounded: past `MAX_BLOCKS` entries it is cleared.

use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use flashtex_compiler::math::{MathList, Nucleus};
use flashtex_compiler::{DocumentId, Span};

use crate::adapter::{CharSrc, Item};
use crate::display::Diagnostic;
use crate::typeset::{BoxRec, BuiltBlock, MathRec};

pub const MAX_BLOCKS: usize = 50_000;

/// A block as built, with its records relative to the block.
#[derive(Clone)]
pub struct CachedBlock {
    pub block: BuiltBlock,
    /// The block's `recs` indices are absolute into the context that built
    /// it; `rec_base`/`math_base` are what to subtract.
    pub rec_base: usize,
    pub math_base: usize,
    pub recs: Vec<BoxRec>,
    pub maths: Vec<MathRec>,
    /// `(once-only key, diagnostic)` emitted while building the block.
    pub diagnostics: Vec<(Option<String>, Diagnostic)>,
    /// The document and first source byte the offsets are relative to.
    pub document: DocumentId,
    pub base: usize,
    pub path: String,
}

/// The display items of one block's lines in line-local coordinates
/// (x absolute on the page, y relative to the line baseline) with source
/// offsets as of the request that built it (`base` is the block's first
/// source byte then). Placing a line adds the baseline tick to every y
/// and the byte delta to every source; both are exact integer moves.
pub struct AssembledBlock {
    /// The line items, shared with every page that places one of the lines
    /// instead of cloned into it (`display::Placed`).
    pub items: Rc<crate::display::LineItems>,
    pub faces: Vec<Rc<crate::fonts::LoadedFace>>,
    /// `(tfm font, face, exact)` resource selections made for math glyphs.
    pub resources: Vec<(String, String, bool)>,
    /// `(tfm font, code, char)` math glyphs with no outline mapping.
    pub unmapped: Vec<(String, u8, char)>,
}

/// Adapter output for one compiler block: its items with source offsets
/// as of the request that built it.
pub struct AdaptedBlock {
    pub items: Vec<Item>,
    pub base: usize,
}

#[derive(Default)]
pub struct RenderCache {
    blocks: RefCell<HashMap<u64, Rc<CachedBlock>>>,
    assembled: RefCell<HashMap<u64, Rc<AssembledBlock>>>,
    adapted: RefCell<HashMap<u64, Rc<AdaptedBlock>>>,
    hits: RefCell<u64>,
    misses: RefCell<u64>,
}

impl RenderCache {
    pub fn new() -> RenderCache {
        RenderCache::default()
    }

    pub fn get(&self, key: u64) -> Option<Rc<CachedBlock>> {
        let hit = self.blocks.borrow().get(&key).cloned();
        if hit.is_some() {
            *self.hits.borrow_mut() += 1;
        } else {
            *self.misses.borrow_mut() += 1;
        }
        hit
    }

    pub fn insert(&self, key: u64, block: CachedBlock) {
        let mut b = self.blocks.borrow_mut();
        if b.len() >= MAX_BLOCKS {
            b.clear();
            self.assembled.borrow_mut().clear();
        }
        b.insert(key, Rc::new(block));
    }

    pub fn adapted(&self, key: u64) -> Option<Rc<AdaptedBlock>> {
        self.adapted.borrow().get(&key).cloned()
    }

    pub fn insert_adapted(&self, key: u64, block: AdaptedBlock) {
        let mut a = self.adapted.borrow_mut();
        if a.len() >= MAX_BLOCKS {
            a.clear();
        }
        a.insert(key, Rc::new(block));
    }

    pub fn assembled(&self, key: u64) -> Option<Rc<AssembledBlock>> {
        self.assembled.borrow().get(&key).cloned()
    }

    pub fn insert_assembled(&self, key: u64, block: AssembledBlock) -> Rc<AssembledBlock> {
        let rc = Rc::new(block);
        let mut a = self.assembled.borrow_mut();
        if a.len() >= MAX_BLOCKS {
            a.clear();
        }
        a.insert(key, rc.clone());
        rc
    }

    pub fn len(&self) -> usize {
        self.blocks.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.borrow().is_empty()
    }

    /// Measurement only (`memsize`): the cached blocks, for deep heap
    /// accounting. Clones the `Rc`s, so nothing is borrowed across the walk.
    pub fn debug_blocks(&self) -> Vec<Rc<CachedBlock>> {
        self.blocks.borrow().values().cloned().collect()
    }

    /// Measurement only (`memsize`): the assembled blocks.
    pub fn debug_assembled(&self) -> Vec<Rc<AssembledBlock>> {
        self.assembled.borrow().values().cloned().collect()
    }

    /// Measurement only (`memsize`): the adapted blocks.
    pub fn debug_adapted(&self) -> Vec<Rc<AdaptedBlock>> {
        self.adapted.borrow().values().cloned().collect()
    }

    pub fn debug_blocks_len(&self) -> usize {
        self.blocks.borrow().len()
    }

    pub fn debug_assembled_len(&self) -> usize {
        self.assembled.borrow().len()
    }

    pub fn debug_adapted_len(&self) -> usize {
        self.adapted.borrow().len()
    }

    /// `(hits, misses)` since creation.
    pub fn stats(&self) -> (u64, u64) {
        (*self.hits.borrow(), *self.misses.borrow())
    }
}

/// The document and lowest source byte of a block's items; `None` when
/// the items come from more than one document or carry no source.
pub fn block_origin(items: &[Item]) -> Option<(DocumentId, usize)> {
    let mut origin: Option<(DocumentId, usize)> = None;
    let mut note = |c: &CharSrc| -> bool {
        match origin {
            None => {
                origin = Some((c.document, c.start));
                true
            }
            Some((d, s)) => {
                if d != c.document {
                    return false;
                }
                if c.start < s {
                    origin = Some((d, c.start));
                }
                true
            }
        }
    };
    for it in items {
        match it {
            Item::Word(w) => {
                for seg in &w.segments {
                    for c in &seg.chars {
                        if !note(c) {
                            return None;
                        }
                    }
                }
            }
            // A table's cell blocks hold absolute record indices and
            // spans: blocks containing one are never cached.
            Item::Table(_) | Item::ColorBox(_) => return None,
            Item::Math { span, .. } => {
                if !note(&CharSrc {
                    document: span.document,
                    start: span.start,
                    end: span.end,
                }) {
                    return None;
                }
            }
            _ => {}
        }
    }
    origin
}

/// Hashes the items with offsets relative to `base`.
pub fn hash_items(items: &[Item], base: usize, h: &mut DefaultHasher) {
    for it in items {
        match it {
            Item::Word(w) => {
                0u8.hash(h);
                for seg in &w.segments {
                    seg.text.hash(h);
                    seg.style.bold.hash(h);
                    seg.style.italic.hash(h);
                    seg.style.color.hash(h);
                    (seg.style.slanted, seg.style.caps, seg.style.family, seg.style.undefined).hash(h);
                    for c in &seg.chars {
                        (c.start.wrapping_sub(base)).hash(h);
                        (c.end.wrapping_sub(base)).hash(h);
                    }
                }
            }
            Item::Space { style, factor, no_break } => {
                1u8.hash(h);
                style.bold.hash(h);
                style.italic.hash(h);
                (style.slanted, style.caps, style.family, style.undefined).hash(h);
                factor.hash(h);
                no_break.hash(h);
            }
            Item::Math { list, span } => {
                2u8.hash(h);
                hash_math(list, h);
                (span.start.wrapping_sub(base)).hash(h);
                (span.end.wrapping_sub(base)).hash(h);
            }
            Item::LineBreak { skip_pt } => {
                3u8.hash(h);
                skip_pt.to_bits().hash(h);
            }
            Item::Quad { em } => {
                4u8.hash(h);
                em.to_bits().hash(h);
            }
            Item::Label { key } => {
                5u8.hash(h);
                key.hash(h);
            }
            Item::ItalicCorrection => 6u8.hash(h),
            Item::HFill { fill } => {
                7u8.hash(h);
                fill.hash(h);
            }
            Item::HSpace { pt } => {
                8u8.hash(h);
                pt.to_bits().hash(h);
            }
            Item::Logo { logo, style, span } => {
                9u8.hash(h);
                logo.hash(h);
                style.hash(h);
                (span.start.wrapping_sub(base)).hash(h);
                (span.end.wrapping_sub(base)).hash(h);
            }
            Item::Rule { rule, style, span } => {
                10u8.hash(h);
                rule.hash(h);
                style.hash(h);
                (span.start.wrapping_sub(base)).hash(h);
                (span.end.wrapping_sub(base)).hash(h);
            }
            Item::Kern { amount, style } => {
                11u8.hash(h);
                amount.hash(h);
                style.hash(h);
            }
            Item::Table(t) => {
                9u8.hash(h);
                format!("{t:?}").hash(h);
            }
            Item::Footnote { number, mark, span, text } => {
                10u8.hash(h);
                number.hash(h);
                mark.hash(h);
                (span.start.wrapping_sub(base)).hash(h);
                (span.end.wrapping_sub(base)).hash(h);
                text.is_some().hash(h);
                if let Some(t) = text {
                    hash_items(t, base, h);
                }
            }
            Item::ColorBox(b) => {
                11u8.hash(h);
                format!("{b:?}").hash(h);
            }
        }
    }
}

/// Hashes a math list's structure (spans are not part of layout).
pub fn hash_math(list: &MathList, h: &mut DefaultHasher) {
    list.atoms.len().hash(h);
    for a in &list.atoms {
        match &a.nucleus {
            Nucleus::Symbol(s) => {
                0u8.hash(h);
                s.hash(h);
            }
            Nucleus::Rule(rule) => {
                15u8.hash(h);
                rule.hash(h);
            }
            Nucleus::Fraction { numerator, denominator } => {
                1u8.hash(h);
                hash_math(numerator, h);
                hash_math(denominator, h);
            }
            Nucleus::Radical(r) => {
                2u8.hash(h);
                hash_math(r, h);
            }
            Nucleus::Text(s) => {
                3u8.hash(h);
                s.hash(h);
            }
            Nucleus::Space { em, .. } => {
                4u8.hash(h);
                em.to_bits().hash(h);
                #[cfg(feature = "amsmath-inline")]
                if let Nucleus::Space { font_em, .. } = &a.nucleus {
                    font_em.hash(h);
                }
            }
            Nucleus::Matrix { rows, columns, left, right } => {
                5u8.hash(h);
                columns.hash(h);
                left.hash(h);
                right.hash(h);
                rows.len().hash(h);
                for row in rows {
                    row.len().hash(h);
                    for cell in row {
                        hash_math(cell, h);
                    }
                }
            }
            Nucleus::Bold(s) => {
                6u8.hash(h);
                s.hash(h);
            }
            Nucleus::Framed { body, frame } => {
                7u8.hash(h);
                (*frame as u8).hash(h);
                hash_math(body, h);
            }
            Nucleus::Stacked { base, over, under } => {
                8u8.hash(h);
                hash_math(base, h);
                for part in [over, under] {
                    match part {
                        Some(l) => {
                            1u8.hash(h);
                            hash_math(l, h);
                        }
                        None => 0u8.hash(h),
                    }
                }
            }
            Nucleus::Accent { accent, body } => {
                9u8.hash(h);
                accent.command().hash(h);
                hash_math(body, h);
            }
            // `\big(`..`\Bigg)` and `\left`/`\right` (pin `d416472a`): the
            // glyph, its cmex10 step and the delimiter role all drive layout.
            Nucleus::SizedDelimiter { glyph, scale, role } => {
                10u8.hash(h);
                glyph.hash(h);
                scale.to_bits().hash(h);
                (*role as u8).hash(h);
            }
            // `\mathbin{...}` and kin: the forced class lives in the
            // compiler's crate-private `class_override`, which the pipeline
            // re-reads from the source bytes (`typeset::class_override_of`);
            // the block key already covers those bytes.
            Nucleus::Group(body) => {
                11u8.hash(h);
                hash_math(body, h);
            }
            #[cfg(feature = "amsmath-inline")]
            Nucleus::GenFraction { numerator, denominator, thickness_pt, left, right, style } => {
                12u8.hash(h);
                hash_math(numerator, h);
                hash_math(denominator, h);
                thickness_pt.map(f64::to_bits).hash(h);
                left.hash(h);
                right.hash(h);
                style.map(|s| s as u8).hash(h);
            }
            #[cfg(feature = "amsmath-inline")]
            Nucleus::Phantom { body, horizontal, vertical } => {
                13u8.hash(h);
                horizontal.hash(h);
                vertical.hash(h);
                hash_math(body, h);
            }
            #[cfg(feature = "amsmath-inline")]
            Nucleus::Operator { body, limits } => {
                14u8.hash(h);
                limits.hash(h);
                hash_math(body, h);
            }
            #[cfg(feature = "amsmath-inline")]
            Nucleus::SubArray { rows, align } => {
                15u8.hash(h);
                align.hash(h);
                rows.len().hash(h);
                for r in rows {
                    hash_math(r, h);
                }
            }
            #[cfg(feature = "amsmath-inline")]
            Nucleus::ExtArrow { arrow, above, below } => {
                16u8.hash(h);
                arrow.hash(h);
                hash_math(above, h);
                hash_math(below, h);
            }
        }
        match &a.superscript {
            Some(s) => {
                1u8.hash(h);
                hash_math(s, h);
            }
            None => 0u8.hash(h),
        }
        match &a.subscript {
            Some(s) => {
                1u8.hash(h);
                hash_math(s, h);
            }
            None => 0u8.hash(h),
        }
    }
}

fn shift(v: usize, delta: isize) -> usize {
    (v as isize + delta) as usize
}

fn shift_range(r: &mut std::ops::Range<usize>, delta: isize) {
    r.start = shift(r.start, delta);
    r.end = shift(r.end, delta);
}

fn shift_span(s: &mut Span, delta: isize) {
    *s = Span::in_document(s.document, shift(s.start, delta), shift(s.end, delta));
}

/// Moves the source spans math-layout copied onto a formula's leaves.
#[cfg(feature = "math-glyph-spans")]
fn shift_tags(b: &mut flashtex_math_layout::MathBox, delta: isize) {
    if let Some(s) = &mut b.tag.span {
        s.start = shift(s.start, delta);
        s.end = shift(s.end, delta);
    }
    if let flashtex_math_layout::BoxKind::HBox(children) | flashtex_math_layout::BoxKind::VBox(children) = &mut b.kind {
        for c in children {
            shift_tags(&mut c.content, delta);
        }
    }
}

/// Moves every source offset of a cached block by `delta` bytes.
pub fn relocate_block(b: &mut BuiltBlock, recs: &mut [BoxRec], maths: &mut [MathRec], diags: &mut [(Option<String>, Diagnostic)], path: &str, delta: isize) {
    if delta == 0 {
        return;
    }
    for line in &mut b.block.lines.lines {
        for run in &mut line.runs {
            shift_range(&mut run.source, delta);
            for g in &mut run.glyphs {
                shift_range(&mut g.cluster, delta);
            }
        }
    }
    for d in &mut b.block.lines.diagnostics {
        if let Some(s) = &mut d.source {
            shift_range(s, delta);
        }
        for r in &mut d.boxes {
            shift_range(r, delta);
        }
    }
    for item in &mut b.items {
        if let flashtex_paragraph_layout::Item::Box(run) = item {
            shift_range(&mut run.source, delta);
            for g in &mut run.glyphs {
                shift_range(&mut g.cluster, delta);
            }
        }
    }
    for rec in recs {
        if let BoxRec::Text { clusters, .. } = rec {
            for c in clusters {
                shift_span(&mut c.span, delta);
            }
        }
    }
    for m in maths {
        shift_span(&mut m.span, delta);
        #[cfg(feature = "math-glyph-spans")]
        {
            shift_tags(&mut m.root, delta);
            for (r, _) in &mut m.span_paints {
                shift_range(r, delta);
            }
        }
    }
    for (_, d) in diags {
        for s in &mut d.sources {
            if &*s.path == path {
                s.start_byte = shift(s.start_byte, delta);
                s.end_byte = shift(s.end_byte, delta);
            }
        }
    }
}

/// A stable fingerprint of everything outside the items that a block's
/// layout depends on.
pub fn style_fingerprint(style: &crate::style::Stylesheet) -> u64 {
    let mut h = DefaultHasher::new();
    format!("{style:?}").hash(&mut h);
    h.finish()
}

/// Places a line-local item: every y moves by `dy` ticks and every source
/// offset in `path` by `delta` bytes.
pub fn place_item(item: &crate::display::Item, dy: crate::display::Tick, path: &str, delta: isize) -> crate::display::Item {
    use crate::display::{Item, Provenance, Tick};
    let add = |t: Tick| Tick(t.0 + dy.0);
    let shift_prov = |p: &Provenance| -> Provenance {
        if delta == 0 {
            return p.clone();
        }
        match p {
            Provenance::Source(s) if &*s.path == path => Provenance::Source(crate::display::SourceRange {
                path: s.path.clone(),
                start_byte: shift(s.start_byte, delta),
                end_byte: shift(s.end_byte, delta),
            }),
            other => other.clone(),
        }
    };
    match item {
        Item::GlyphRun(r) => {
            let mut r = r.clone();
            for g in &mut r.glyphs {
                g.baseline_y = add(g.baseline_y);
            }
            for c in &mut r.clusters {
                // The carets derive from this rect, so moving it moves them
                // by exactly the same `dy` the three separate fields used
                // to be moved by.
                c.hit_rect.top = add(c.hit_rect.top);
                c.provenance = shift_prov(&c.provenance);
            }
            Item::GlyphRun(r)
        }
        Item::Path(p) => {
            let mut p = p.clone();
            for c in &mut p.commands {
                *c = c.map_y(&add);
            }
            for clip in &mut p.clips {
                for c in &mut clip.commands {
                    *c = c.map_y(&add);
                }
            }
            p.provenance = shift_prov(&p.provenance);
            Item::Path(p)
        }
        Item::Rule(rule) => {
            let mut rule = rule.clone();
            rule.top = add(rule.top);
            rule.provenance = shift_prov(&rule.provenance);
            Item::Rule(rule)
        }
        Item::Image(img) => {
            let mut img = img.clone();
            img.top = add(img.top);
            img.transform[5] += dy.to_bp();
            img.provenance = shift_prov(&img.provenance);
            Item::Image(img)
        }
    }
}

fn shift_math(list: &mut MathList, delta: isize) {
    for a in &mut list.atoms {
        shift_span(&mut a.span, delta);
        match &mut a.nucleus {
            Nucleus::Symbol(_) | Nucleus::Text(_) | Nucleus::Space { .. } | Nucleus::Bold(_) | Nucleus::SizedDelimiter { .. } | Nucleus::Rule(_) => {}
            Nucleus::Fraction { numerator, denominator } => {
                shift_math(numerator, delta);
                shift_math(denominator, delta);
            }
            Nucleus::Radical(r) | Nucleus::Framed { body: r, .. } | Nucleus::Accent { body: r, .. } | Nucleus::Group(r) => shift_math(r, delta),
            Nucleus::Stacked { base, over, under } => {
                shift_math(base, delta);
                for part in [over, under].into_iter().flatten() {
                    shift_math(part, delta);
                }
            }
            Nucleus::Matrix { rows, .. } => {
                for cell in rows.iter_mut().flatten() {
                    shift_math(cell, delta);
                }
            }
            #[cfg(feature = "amsmath-inline")]
            Nucleus::GenFraction { numerator, denominator, .. } => {
                shift_math(numerator, delta);
                shift_math(denominator, delta);
            }
            #[cfg(feature = "amsmath-inline")]
            Nucleus::Phantom { body, .. } | Nucleus::Operator { body, .. } => shift_math(body, delta),
            #[cfg(feature = "amsmath-inline")]
            Nucleus::SubArray { rows, .. } => rows.iter_mut().for_each(|r| shift_math(r, delta)),
            #[cfg(feature = "amsmath-inline")]
            Nucleus::ExtArrow { above, below, .. } => {
                shift_math(above, delta);
                shift_math(below, delta);
            }
        }
        if let Some(s) = &mut a.superscript {
            shift_math(s, delta);
        }
        if let Some(s) = &mut a.subscript {
            shift_math(s, delta);
        }
    }
}

/// Clones adapted items with every source offset moved by `delta`.
pub fn relocate_items(items: &[Item], delta: isize) -> Vec<Item> {
    let mut out = items.to_vec();
    if delta == 0 {
        return out;
    }
    for it in &mut out {
        match it {
            Item::Word(w) => {
                for seg in &mut w.segments {
                    for c in &mut seg.chars {
                        c.start = shift(c.start, delta);
                        c.end = shift(c.end, delta);
                    }
                }
            }
            Item::Math { list, span } => {
                shift_span(span, delta);
                shift_math(list, delta);
            }
            Item::Logo { span, .. } | Item::Rule { span, .. } => shift_span(span, delta),
            Item::Footnote { span, text, .. } => {
                shift_span(span, delta);
                if let Some(t) = text {
                    *t = relocate_items(t, delta);
                }
            }
            _ => {}
        }
    }
    out
}
