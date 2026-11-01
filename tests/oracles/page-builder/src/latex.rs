//! LaTeX's page-level macros (latex.ltx 2026 `ltspace`/`ltoutput`) on
//! top of the TeX machinery: `\pagebreak[n]`, `\nopagebreak[n]`,
//! `\newpage`, `\clearpage`, `\enlargethispage`, and a model of the output
//! routine for single-column pages without floats or marginpars
//! (`\@makecol` with its `build/column/outputbox` socket, `\@opcol`,
//! `\@doclearpage`, `\raggedbottom`/`\flushbottom`).

use crate::node::{BoxNode, GlueSpec, Node, Order};
use crate::pack::{vlist_positions, vpackage, PackSpec, VBox};
use crate::page::{insert_node, FiredPage, OutputResult, OutputRoutine, PageBuilder};
use crate::scaled::{Scaled, EJECT_PENALTY, INF_PENALTY, MAX_DIMEN};
use crate::split::vsplit;

/// `\@lowpenalty`, `\@medpenalty`, `\@highpenalty` (article.cls: 51, 151, 301).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriorityPenalties {
    pub low: i32,
    pub med: i32,
    pub high: i32,
}

impl PriorityPenalties {
    pub const ARTICLE: PriorityPenalties = PriorityPenalties { low: 51, med: 151, high: 301 };

    /// `\@getpen{n}`.
    pub fn get(self, n: i32) -> i32 {
        match n {
            0 => 0,
            1 => self.low,
            2 => self.med,
            3 => self.high,
            _ => INF_PENALTY,
        }
    }
}

/// `\pagebreak[n]` (default 4) in vertical mode: `\penalty -\@getpen{n}`.
/// In horizontal mode the same penalty goes into a `\vadjust` after the line.
pub fn pagebreak(n: Option<i32>, p: PriorityPenalties) -> Node {
    Node::Penalty(-p.get(n.unwrap_or(4)))
}

/// `\nopagebreak[n]` (default 4).
pub fn nopagebreak(n: Option<i32>, p: PriorityPenalties) -> Node {
    Node::Penalty(p.get(n.unwrap_or(4)))
}

/// `\newpage` after `\par`: `\vfil\penalty-\@M`.
pub fn newpage() -> Vec<Node> {
    vec![Node::glue(GlueSpec::fil()), Node::Penalty(EJECT_PENALTY)]
}

/// `\clearpage` in one-column mode: `\newpage\write\m@ne{}\vbox{}\penalty-\@Mi`.
pub fn clearpage(write_id: u32) -> Vec<Node> {
    let mut v = newpage();
    v.push(Node::Whatsit(write_id));
    v.push(Node::Box(BoxNode { vertical: true, ..BoxNode::default() }));
    v.push(Node::Penalty(-10_001));
    v
}

/// `\enlargethispage{amount}` / `\enlargethispage*{amount}`: an
/// `\insert\@kludgeins{\vskip-amount}` (star: `{\hbox{\kern1pt}\vskip-amount}`).
pub fn enlargethispage(amount: Scaled, star: bool, kludgeins: u8, split_top_skip: GlueSpec, split_max_depth: Scaled, floating_penalty: i32) -> Node {
    let mut list = Vec::new();
    if star {
        list.push(Node::Box(BoxNode::hbox(crate::scaled::UNITY, 0, 0, u32::MAX)));
    }
    list.push(Node::glue(GlueSpec::fixed(-amount)));
    insert_node(kludgeins, list, split_top_skip, split_max_depth, floating_penalty)
}

/// A shipped text column.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    /// `\@outputbox` after `\@makecol` (packed to `\@colht`).
    pub vbox: VBox,
    /// Every box of the column that came from the page (nested boxes the
    /// output routine creates are flattened): the box and its baseline
    /// measured from the top of the column.
    pub lines: Vec<(BoxNode, Scaled)>,
    pub output_penalty: i32,
}

/// Footnote rendering inputs owned by the footnote implementation.
#[derive(Debug, Clone, PartialEq)]
pub struct FootnoteSpec {
    /// `\footins`.
    pub class: u8,
    /// `\footnoterule` material (article: `\kern-3pt \hrule width .4\columnwidth \kern2.6pt`).
    pub rule: Vec<Node>,
}

/// Single-column LaTeX output routine without floats or marginpars.
#[derive(Debug, Clone)]
pub struct LatexOutput {
    /// `\@colht` (= `\textheight`).
    pub colht: Scaled,
    /// `\@maxdepth` (= `\maxdepth` at `\begin{document}`).
    pub maxdepth: Scaled,
    /// `\raggedbottom` (else `\flushbottom`).
    pub raggedbottom: bool,
    /// `\@kludgeins`.
    pub kludgeins: u8,
    pub footnotes: Option<FootnoteSpec>,
    /// `\splittopskip`/`\splitmaxdepth` during output.
    pub split_top_skip: GlueSpec,
    pub split_max_depth: Scaled,
    pub columns: Vec<Column>,
    /// `\@holdpg`.
    pub holdpg: Vec<Node>,
    /// Outputs this model does not implement (float/marginpar special
    /// penalties -10002/-10003); their box255 contents were returned.
    pub unsupported: usize,
    nested: Option<VBox>,
}

const SYNTHETIC: u32 = u32::MAX;

impl LatexOutput {
    pub fn new(colht: Scaled, maxdepth: Scaled, raggedbottom: bool, kludgeins: u8) -> LatexOutput {
        LatexOutput {
            colht,
            maxdepth,
            raggedbottom,
            kludgeins,
            footnotes: None,
            split_top_skip: GlueSpec::fixed(crate::scaled::pt(10.0)),
            split_max_depth: MAX_DIMEN,
            columns: Vec::new(),
            holdpg: Vec::new(),
            unsupported: 0,
            nested: None,
        }
    }

    fn textbottom(&self) -> Vec<Node> {
        if self.raggedbottom {
            // \vskip \z@ \@plus.0001fil
            vec![Node::glue(GlueSpec { stretch: 7, stretch_order: Order::Fil, ..GlueSpec::ZERO })]
        } else {
            Vec::new()
        }
    }

    /// `\@outputbox@append{extra}`.
    fn append(&self, ob: VBox, extra: Vec<Node>) -> VBox {
        let mut list = ob.list;
        list.extend(extra);
        vpackage(list, PackSpec::NATURAL, self.maxdepth)
    }

    /// `\@makecol`.
    fn makecol(&mut self, builder: &mut PageBuilder, page_box: VBox, page_shrink: Scaled) -> VBox {
        // \@outputbox@removebskip
        let mut list = page_box.list;
        let mut reinsert = None;
        if let Some(Node::Glue { spec, .. }) = list.last() {
            if spec.stretch_order > Order::Normal {
                reinsert = Some(*spec);
                list.pop();
            }
        }
        let mut ob = vpackage(list, PackSpec::NATURAL, self.maxdepth);
        let footins = self
            .footnotes
            .as_ref()
            .and_then(|f| builder.classes.get_mut(&f.class))
            .and_then(|c| c.contents.take())
            .filter(|b| !b.list.is_empty() || b.height != 0);
        // Socket build/column/outputbox with the kernel's assigned plug
        // `footnotes-floats-legacy` and build/column/baselineattach = off
        // (no floats here): an empty \@outputbox@append when footnotes are
        // present, then \@outputbox@reinsertbskip, then the footnotes.
        if footins.is_some() {
            ob = self.append(ob, Vec::new());
        }
        if let Some(skip) = reinsert {
            let mut extra = Vec::new();
            // \@backup@outputbox@depth unless footnotes (or bottom floats) follow
            if footins.is_none() && ob.depth > 0 {
                extra.push(Node::glue(GlueSpec::fixed(-ob.depth.min(self.maxdepth))));
            }
            extra.push(Node::glue(skip));
            ob = self.append(ob, extra);
        }
        if let (Some(fb), Some(spec)) = (footins, self.footnotes.clone()) {
            let skip = builder.classes.get(&spec.class).map(|c| c.skip).unwrap_or_default();
            let mut extra = vec![Node::glue(skip)];
            extra.extend(spec.rule);
            extra.extend(fb.list);
            ob = self.append(ob, extra);
        }
        let kludge = builder.classes.get_mut(&self.kludgeins).and_then(|c| c.contents.take());
        match kludge {
            None => {
                // \@make@normalcolbox
                let d = ob.depth;
                let mut l = ob.list;
                l.push(Node::glue(GlueSpec::fixed(-d)));
                l.extend(self.textbottom());
                vpackage(l, PackSpec::Exactly(self.colht), MAX_DIMEN)
            }
            Some(k) => {
                // \@make@specialcolbox
                let d = ob.depth;
                ob = self.append(ob, vec![Node::glue(GlueSpec::fixed(-d))]);
                if k.width > 0 {
                    let tempdima = self.colht - ob.height + page_shrink;
                    let mut l = ob.list;
                    l.push(Node::glue(GlueSpec::fixed(tempdima)));
                    l.extend(self.textbottom());
                    vpackage(l, PackSpec::Exactly(self.colht), MAX_DIMEN)
                } else {
                    let tempdima = self.colht - k.height;
                    let mut l = ob.list;
                    l.extend(self.textbottom());
                    let inner = vpackage(l, PackSpec::Exactly(tempdima), MAX_DIMEN);
                    let outer = vec![inner.as_node(SYNTHETIC), Node::glue(GlueSpec::ss())];
                    let v = vpackage(outer, PackSpec::Exactly(self.colht), MAX_DIMEN);
                    // The nested box's list is needed for positions.
                    self.nested = Some(inner);
                    v
                }
            }
        }
    }

    fn ship(&mut self, column: VBox, output_penalty: i32) {
        let mut lines = Vec::new();
        let nested = self.nested.take();
        flatten(&column, 0, nested.as_ref(), &mut lines);
        self.columns.push(Column { vbox: column, lines, output_penalty });
    }
}

fn flatten(vbox: &VBox, offset: Scaled, nested: Option<&VBox>, out: &mut Vec<(BoxNode, Scaled)>) {
    let pos = vlist_positions(vbox);
    for (node, y) in vbox.list.iter().zip(pos) {
        if let Node::Box(b) = node {
            if b.id == SYNTHETIC && b.vertical {
                if let Some(inner) = nested {
                    flatten(inner, offset + y - b.height, None, out);
                    continue;
                }
            }
            out.push((*b, offset + y));
        }
    }
}

impl OutputRoutine for LatexOutput {
    fn output(&mut self, builder: &mut PageBuilder, page: FiredPage) -> OutputResult {
        let p = page.output_penalty;
        if p > -10_001 {
            // \@makecol \@opcol
            let column = self.makecol(builder, page.box255, page.page_so_far[6]);
            self.ship(column, p);
            return OutputResult { material: Vec::new(), reset_dead_cycles: true };
        }
        // \@specialoutput
        if p > -10_002 {
            // \@doclearpage
            let footins_void = self
                .footnotes
                .as_ref()
                .and_then(|f| builder.classes.get(&f.class))
                .is_none_or(|c| c.contents.is_none());
            if footins_void {
                if let Some(c) = builder.classes.get_mut(&self.kludgeins) {
                    c.contents = None;
                }
                let split = vsplit(page.box255.list, 0, self.split_top_skip, self.split_max_depth);
                return OutputResult { material: split.extracted.list, reset_dead_cycles: false };
            }
            // \setbox\@cclv\vbox{\box\@cclv\vfil}
            let inner = page.box255.as_node(SYNTHETIC);
            let wrapped = vpackage(vec![inner, Node::glue(GlueSpec::fil())], PackSpec::NATURAL, MAX_DIMEN);
            self.nested = Some(page.box255);
            let column = self.makecol(builder, wrapped, page.page_so_far[6]);
            self.ship(column, p);
            return OutputResult { material: clearpage(0), reset_dead_cycles: true };
        }
        if p < -10_003 {
            let reset = p < -20_000;
            self.holdpg = page.box255.list;
            return OutputResult { material: Vec::new(), reset_dead_cycles: reset };
        }
        self.unsupported += 1;
        let mut list = page.box255.list;
        if matches!(list.last(), Some(Node::Box(_))) {
            list.pop();
        }
        if matches!(list.last(), Some(Node::Glue { .. })) {
            list.pop();
        }
        OutputResult { material: list, reset_dead_cycles: false }
    }
}
