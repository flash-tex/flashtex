//! Converts pdfTeX `\showbox` dumps of the microtype oracle format into
//! this crate's items (shared by `microtype_oracle` and `emergency_oracle`).

#![allow(dead_code)]

#[path = "../../../microtype/tests/common/mod.rs"]
pub mod common;

use std::collections::BTreeMap;
use std::rc::Rc;

pub use common::*;
use flashtex_paragraph_layout::{FontId, Glue, GlueOrder, Glyph, GlyphRun, Item, MicroGlyph, MicroItem, MicroRun, Penalty};
use flashtex_microtype::pdftex::FontParams;

pub fn pt(sp: i32) -> f64 {
    f64::from(sp) / 65536.0
}

pub fn glue(g: &GlueSpec) -> Glue {
    let order = |o: Order| match o {
        Order::Normal => GlueOrder::Finite,
        Order::Fil => GlueOrder::Fil,
        Order::Fill => GlueOrder::Fill,
        Order::Filll => GlueOrder::Filll,
    };
    Glue {
        width: pt(g.width),
        stretch: pt(g.stretch),
        stretch_order: order(g.stretch_order),
        shrink: pt(g.shrink),
        shrink_order: order(g.shrink_order),
        source: None,
    }
}

pub struct Conv<'f> {
    pub fonts: &'f BTreeMap<String, FontEntry>,
    pub params: BTreeMap<String, Rc<FontParams>>,
    pub ids: BTreeMap<String, FontId>,
}

impl Conv<'_> {
    fn font(&mut self, name: &str) -> (FontId, Rc<FontParams>) {
        let n = self.ids.len() as u8;
        let id = *self.ids.entry(name.to_string()).or_insert_with(|| FontId([n; 32]));
        let p = self.params.entry(name.to_string()).or_insert_with(|| Rc::new(self.fonts[name].params.clone())).clone();
        (id, p)
    }

    /// A run of characters (with the normal kerns between them) of one font
    /// starting at `nodes[i]`; returns the run and the index after it.
    fn run(&mut self, nodes: &[Node], mut i: usize, stop: usize) -> (GlyphRun, MicroRun, usize) {
        let Node::Char { font, .. } = &nodes[i] else { unreachable!() };
        let font = font.clone();
        let (id, params) = self.font(&font);
        let size: f64 = self.fonts[&font].size.trim_end_matches("pt").parse().unwrap();
        let mut glyphs = Vec::new();
        let mut micro = Vec::new();
        while let Some(Node::Char { font: f, code, .. }) = nodes.get(i).filter(|_| i < stop) {
            if *f != font {
                break;
            }
            let w = self.fonts[&font].widths[*code as usize];
            i += 1;
            let mut kern = 0;
            if let (Some(Node::Kern { width, kind: KernKind::Normal }), Some(Node::Char { font: f2, .. })) = (nodes.get(i), nodes.get(i + 1)) {
                if *f2 == font && i + 1 < stop {
                    kern = *width;
                    i += 1;
                }
            }
            glyphs.push(Glyph { gid: u32::from(*code), advance: pt(w + kern), kern: 0.0, cluster: 0..0 });
            micro.push(MicroGlyph { code: Some(*code), width: w, kern });
        }
        let width = glyphs.iter().map(|g| g.advance).sum();
        (
            GlyphRun { font: id, size, glyphs, width, height: 0.0, depth: 0.0, source: 0..0 },
            MicroRun { params, glyphs: micro },
            i,
        )
    }

    /// Pre/post-break text as one run (characters and normal kerns only).
    fn text(&mut self, nodes: &[Node]) -> (Option<GlyphRun>, Option<MicroRun>) {
        if nodes.is_empty() {
            return (None, None);
        }
        let (r, m, end) = self.run(nodes, 0, nodes.len());
        assert_eq!(end, nodes.len(), "discretionary text beyond one run: {nodes:?}");
        (Some(r), Some(m))
    }

    pub fn items(&mut self, nodes: &[Node], explicit: &[bool], hyphen_penalty: i32, ex_hyphen_penalty: i32) -> (Vec<Item>, Vec<MicroItem>) {
        let mut items = Vec::new();
        let mut micro = Vec::new();
        let mut i = 0;
        let mut disc_no = 0;
        // (item index of the penalty, nodes of replace text left)
        let mut pending: Option<(usize, usize, usize)> = None;
        while i < nodes.len() {
            match &nodes[i] {
                Node::Char { .. } => {
                    // Never let a run cross the end of a replace text.
                    let stop = pending.map_or(nodes.len(), |(_, _, stop)| stop);
                    let (r, m, end) = self.run(nodes, i, stop);
                    items.push(Item::Box(r));
                    micro.push(MicroItem { run: Some(m), ..MicroItem::default() });
                    i = end;
                }
                Node::Glue(g) => {
                    items.push(Item::Glue(glue(g)));
                    micro.push(MicroItem::default());
                    i += 1;
                }
                Node::Kern { width, kind: KernKind::Normal }
                    if matches!(items.last(), Some(Item::Box(r)) if !r.glyphs.is_empty())
                        && !matches!(nodes.get(i + 1), Some(Node::Char { .. })) =>
                {
                    // A font kern after a run's last character (e.g. the
                    // right-boundary kern after `''`) is part of the word:
                    // tex.web §866 never breaks at a non-explicit kern.
                    let Some(Item::Box(r)) = items.last_mut() else { unreachable!() };
                    r.glyphs.last_mut().unwrap().advance += pt(*width);
                    r.width += pt(*width);
                    if let Some(m) = micro.last_mut().and_then(|m: &mut MicroItem| m.run.as_mut()) {
                        m.glyphs.last_mut().unwrap().kern += *width;
                    }
                    i += 1;
                }
                Node::Kern { width, kind } => {
                    items.push(Item::kern(pt(*width)));
                    micro.push(MicroItem { font_kern: *kind == KernKind::Normal, ..MicroItem::default() });
                    i += 1;
                }
                Node::Math { width, .. } => {
                    items.push(Item::kern(pt(*width)));
                    micro.push(MicroItem::default());
                    i += 1;
                }
                Node::Penalty(v) => {
                    items.push(Item::penalty(*v));
                    micro.push(MicroItem::default());
                    i += 1;
                }
                Node::HBox { width, .. } | Node::VBox { width, .. } | Node::Rule { width } => {
                    items.push(Item::Box(GlyphRun { font: FontId([255; 32]), size: 0.0, glyphs: vec![], width: pt(*width), height: 0.0, depth: 0.0, source: 0..0 }));
                    micro.push(MicroItem::default());
                    i += 1;
                }
                Node::Disc { pre, post, replace } => {
                    let (pre_r, pre_m) = self.text(pre);
                    let (post_r, post_m) = self.text(post);
                    let automatic = !explicit[disc_no];
                    disc_no += 1;
                    items.push(Item::Penalty(Penalty {
                        value: if pre.is_empty() { ex_hyphen_penalty } else { hyphen_penalty },
                        flagged: true,
                        pre_break: pre_r,
                        automatic,
                        post_break: post_r,
                        replace_count: 0,
                    }));
                    micro.push(MicroItem { pre_break: pre_m, post_break: post_m, ..MicroItem::default() });
                    i += 1;
                    if *replace > 0 {
                        pending = Some((items.len() - 1, items.len(), i + replace));
                    }
                }
                Node::Other(o) => panic!("unsupported node {o}"),
            }
            if let Some((pen, first, stop)) = pending {
                if i >= stop {
                    assert_eq!(i, stop, "replace text split");
                    let count = items.len() - first;
                    let Item::Penalty(p) = &mut items[pen] else { unreachable!() };
                    p.replace_count = count;
                    pending = None;
                }
            }
        }
        (items, micro)
    }
}

/// Character codes of the top-level chars before each discretionary.
pub fn disc_contexts(nodes: &[Node]) -> Vec<Vec<u8>> {
    let mut text = Vec::new();
    let mut out = Vec::new();
    for n in nodes {
        match n {
            Node::Char { code, .. } => text.push(*code),
            Node::Disc { .. } => out.push(text.clone()),
            _ => {}
        }
    }
    out
}

