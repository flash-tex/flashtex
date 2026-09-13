//! Pseudocode statement lines (prepared by `crate::algorithms`) as list
//! items.
//!
//! algorithmic.sty nests one `list` per block (`ALC@g`, lines 137-145):
//! every line of a statement hangs at `\@totalleftmargin` = `\labelwidth` +
//! `\labelsep` + depth × `\algorithmicindent`, and `\ALC@item` (lines 98-121)
//! sets the label box `\hskip-\labelwidth\hskip-\ALC@tlm \hbox
//! to\labelwidth{\hfil <label>}\hskip\ALC@tlm` with `\ALC@tlm` = `\labelsep`
//! + depth × indent, so every label starts at the outer margin.
//! algorithmicx uses one list (lines 87-99) with LaTeX's own `\@item` label
//! box (`\hskip-\labelwidth\hskip-\labelsep ... \hskip\labelsep`) and starts
//! each statement with `\noindent\hskip\ALG@tlm` (line 198): the first line
//! is indented by the open blocks, a wrapped statement's later lines are
//! not. A label wider than `\labelwidth` keeps its natural width and pushes
//! the text right.

use flashtex_compiler::algorithmic::{Dialect, Dimen, LABELSEP_EM, TOPSEP_EM};
use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use super::{drop_trailing_break, line_extents, vskips_of, BuiltBlock, Context, CLUB_PENALTY, WIDOW_PENALTY};
use crate::adapter::{Item as AItem, ParaStyle, TextStyle};
use crate::pagebuild::VBlock;

#[derive(Debug, Clone, PartialEq)]
pub struct LineGeometry {
    pub dialect: Dialect,
    /// `\labelwidth` in ems: 1.2 with line numbers, 0.5 without.
    pub label_width_em: f64,
    /// Open blocks around the line.
    pub depth: u32,
    /// `\algorithmicindent`.
    pub indent: Dimen,
    /// algorithmicx `\hskip\ALG@tlm` before the text.
    pub hskip_tlm: bool,
}

/// One statement line.
#[derive(Debug, Clone, PartialEq)]
pub struct AlgLine {
    pub items: Vec<AItem>,
    /// The label material (a line number or `\item[..]` text); empty for
    /// an unnumbered line.
    pub label: Vec<AItem>,
    pub geometry: LineGeometry,
    /// algorithmicx's `\item[]\nointerlineskip` for an entity without text.
    pub no_text: bool,
    pub span: Span,
}

/// An `algorithmic` environment in the text flow.
#[derive(Debug, Clone, PartialEq)]
pub struct BareAlgorithm {
    pub lines: Vec<AlgLine>,
    pub span: Span,
}

fn natural_width(item: &pl::Item) -> f64 {
    match item {
        pl::Item::Box(run) => run.width,
        pl::Item::Glue(g) => g.width,
        pl::Item::Kern(k) => k.width,
        pl::Item::Penalty(_) => 0.0,
    }
}

impl Context<'_> {
    /// `(\@totalleftmargin, \labelwidth, the skip after the label, the skip
    /// before the text)` in points at `size`.
    fn algorithm_metrics(&self, g: &LineGeometry, size: f64) -> (f64, f64, f64, f64) {
        let p = self.text_params(TextStyle::default(), size);
        let (em, ex) = (p.quad, p.x_height);
        let labelwidth = g.label_width_em * em;
        let labelsep = LABELSEP_EM * em;
        let indent = g.indent.pt(em, ex) * f64::from(g.depth);
        match g.dialect {
            Dialect::Algorithmic => (labelwidth + labelsep + indent, labelwidth, labelsep + indent, 0.0),
            Dialect::Algpseudocode => (labelwidth + labelsep, labelwidth, labelsep, if g.hskip_tlm { indent } else { 0.0 }),
        }
    }

    /// `\topsep` of the outer list (0.2em), in points.
    pub(super) fn algorithm_topsep(&self) -> f64 {
        TOPSEP_EM * self.text_params(TextStyle::default(), self.style.body_size_pt).quad
    }

    /// The paragraph of one statement line; `None` when it has no material
    /// (an empty `\item`, which sets an empty line).
    pub(super) fn algorithm_line_block(&mut self, line: &AlgLine) -> Option<BuiltBlock> {
        let size = self.style.body_size_pt;
        let (hang, labelwidth, after_label, before_text) = self.algorithm_metrics(&line.geometry, size);
        let (mut list, mut recs, mut labels, mut skips) = self.hlist(&line.items, size, TextStyle::default(), ParaStyle::Plain);
        let mut lead: Vec<(pl::Item, Option<usize>)> = Vec::new();
        if !line.label.is_empty() {
            let (mut label_list, mut label_recs, _, _) = self.hlist(&line.label, size, TextStyle::default(), ParaStyle::Plain);
            // The label is an hbox: drop the paragraph end `hlist` appends
            // (`\penalty10000\parfillskip\penalty-10000`).
            let keep = label_list.iter().rposition(|i| matches!(i, pl::Item::Box(_))).map_or(0, |p| p + 1);
            label_list.truncate(keep);
            label_recs.truncate(keep);
            let width: f64 = label_list.iter().map(natural_width).sum();
            // `\hbox to\labelwidth{\hfil <label>}` unless the label is wider.
            lead.push((pl::Item::kern(-(labelwidth + after_label) + (labelwidth - width).max(0.0)), None));
            lead.extend(label_list.into_iter().zip(label_recs));
            lead.push((pl::Item::kern(after_label), None));
        }
        if before_text != 0.0 {
            lead.push((pl::Item::kern(before_text), None));
        }
        let n = lead.len();
        for (i, (item, rec)) in lead.into_iter().enumerate() {
            list.insert(i, item);
            recs.insert(i, rec);
        }
        for (_, at) in &mut labels {
            *at += n;
        }
        for (at, _) in &mut skips {
            *at += n;
        }
        if !list.iter().any(|i| matches!(i, pl::Item::Box(_))) {
            return None;
        }
        let trailing_skip = drop_trailing_break(&mut list, &mut recs, &mut skips, ParaStyle::Plain);
        let params = self.line_params(false, self.style.baselineskip_pt, ParaStyle::Plain, hang);
        let lines = self.break_paragraph(&list, &params, &line.items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        let vertical = VBlock {
            lines: line_extents(&lines),
            penalty_before: None,
            space_before: None,
            // The list sets `\parskip\z@`.
            parskip: None,
            interline_penalty: 0,
            club_penalty: CLUB_PENALTY,
            widow_penalty: WIDOW_PENALTY,
            penalty_after: None,
            space_after: trailing_skip.map(|pt| (pt, 0.0, 0.0)),
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: None,
            vskip_after: vskips_of(&lines, &skips),
            pre_space_after: None,
        };
        Some(BuiltBlock { block: pl::ParagraphBlock::body(lines), items: list, recs, vertical, labels, cache_key: None })
    }

    /// A bare `algorithmic` in the text flow: its lines, the first after
    /// `\addvspace\@topsep` and the last followed by `\@endparenv`'s
    /// `\addvspace\@topsepadd` (both `\topsep`; `\partopsep` is 0 in both
    /// packages). An empty line becomes `\vskip\baselineskip`-equivalent
    /// glue; a no-text line adds nothing.
    pub(super) fn algorithm_flow_blocks(&mut self, algorithm: &BareAlgorithm) -> Vec<BuiltBlock> {
        let topsep = self.algorithm_topsep();
        let mut out: Vec<BuiltBlock> = Vec::new();
        let mut pending_empty = 0.0;
        for line in &algorithm.lines {
            if line.no_text {
                continue;
            }
            match self.algorithm_line_block(line) {
                Some(mut b) => {
                    if pending_empty != 0.0 {
                        b.vertical.space_before = Some((pending_empty, 0.0, 0.0));
                        pending_empty = 0.0;
                    }
                    out.push(b);
                }
                None => pending_empty += self.style.baselineskip_pt,
            }
        }
        if let Some(first) = out.first_mut() {
            let before = first.vertical.space_before.map_or(0.0, |s| s.0);
            first.vertical.space_before = Some((before + topsep, 0.0, 0.0));
            // `\@item`: `\addpenalty\@beginparpenalty` (article: -\@lowpenalty).
            first.vertical.penalty_before = Some(-51);
        }
        if let Some(last) = out.last_mut() {
            last.vertical.space_after = Some((topsep, 0.0, 0.0));
        }
        out
    }
}
