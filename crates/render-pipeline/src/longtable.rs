//! longtable v4.24 (`longtable.sty`, 2025-10-13): how a longtable's rows
//! are split into chunks and what the page's vertical list receives.
//!
//! A longtable is not a box in a paragraph. `\LT@array` opens
//! `\halign to\hsize` with `\tabskip\LTleft` before the first column and
//! `\tabskip\LTright` after the last (lines 167-171), so the table is set
//! to `\hsize` with the two skips carrying the difference; `[l]`/`[c]`/`[r]`
//! set them to `0pt`/`\fill` (120-126) and the package default is `\fill`
//! on both sides, i.e. centred. Line 191 sets `\lineskip\z@` and
//! `\baselineskip\z@`, so every row, rule row and head or foot box is
//! contributed to the page's vertical list with no interline glue.
//!
//! `\endfirsthead`/`\endhead`/`\endfoot`/`\endlastfoot` (`\LT@end@hd@ft`,
//! 520-562) each close the chunk before them and save it in a box; what
//! follows the last of them is the body. Every chunk — heads, feet and
//! `\kill` rows alike — is measured by `\LT@get@widths` (395-429), so the
//! column widths are those of all the rows together even though only some
//! are set on a given page.
//!
//! `\LT@start` (196-241) puts `\LTpre` glue before the table, reduces
//! `\vsize`/`\pagegoal` by `\ht\LT@foot` and sets `\maxdepth\z@` while the
//! table is being set, then contributes the first head. `\LT@output`
//! (487-517) is the output routine for the region: an ordinary break
//! appends `\LT@foot` to the page and starts the next one with `\LT@head`;
//! the end-of-table penalty (`-\LT@end@pen`, 30000) appends `\LT@lastfoot`,
//! ejecting one more page with the ordinary foot first when the last foot
//! is taller than the foot and no longer fits. `\endlongtable` closes with
//! `\addvspace\LTpost` (280).
//!
//! # `\LTchunksize` is deliberately not modelled
//!
//! The body is also cut into chunks of `\LTchunksize` rows (65: 200 by
//! default), each its own `\halign`: `\LT@t@bularcr` closes the chunk at
//! row `\LTchunksize` (303-312) and `\LT@get@widths` (395-429) folds its
//! column widths into `\LT@save@row` with `\LT@max@sel`, the running
//! maximum. A chunk can therefore only be as wide as the widest cell *seen
//! so far*, so on a run where a later chunk turns out wider, the earlier
//! chunks are already set narrower — and that is exactly the state
//! longtable reports as `Column widths have changed` (260-266). It also
//! writes the final widths to the `.aux` (253-259), so the next run starts
//! from the maximum and every chunk is set at the same width.
//!
//! This module measures the whole table once, which is what a converged
//! run produces. Verified against pdflatex on two shapes: 260 rows with
//! the widest cell past the default 200-row boundary, and 30 rows with
//! `\LTchunksize=5` and the widest cell in the sixth chunk. Both warn on
//! pass 1 and are silent on pass 2, and the corpus references (two passes,
//! like every oracle run here) match this module's single measurement to
//! the last decimal — 104-longtable-chunk-boundary and
//! 105-longtable-ltchunksize. Modelling chunking would only reproduce the
//! *unconverged* first pass, which is not what a document renders as.

use crate::table::{self, MCell, Metrics, TableEntry, TableItem, Widths};
use flashtex_compiler::tabular::{self as ct, LongtableSection};

/// `\LTpre`/`\LTpost` default to `\bigskipamount`, `\LTcapwidth` to 4in
/// (longtable.sty 61-67).
pub const LTCAPWIDTH_PT: f64 = 4.0 * 72.27;

/// Which chunk an entry belongs to. The order is the order the chunks are
/// *saved* in, not the order they are set in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    FirstHead,
    Head,
    Foot,
    LastFoot,
    Body,
}

/// The entry indices of each chunk, in source order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parts {
    pub first_head: Vec<usize>,
    pub head: Vec<usize>,
    pub foot: Vec<usize>,
    pub last_foot: Vec<usize>,
    pub body: Vec<usize>,
}

impl Parts {
    /// `\ifvoid\LT@firsthead\copy\LT@head\else\box\LT@firsthead\fi`
    /// (longtable.sty 239): the head of the first page.
    pub fn opening_head(&self) -> &[usize] {
        if self.first_head.is_empty() {
            &self.head
        } else {
            &self.first_head
        }
    }

    /// `\box\ifvoid\LT@lastfoot\LT@foot\else\LT@lastfoot\fi` (506).
    pub fn closing_foot(&self) -> &[usize] {
        if self.last_foot.is_empty() {
            &self.foot
        } else {
            &self.last_foot
        }
    }
}

/// Splits a longtable's entries into its chunks (`\LT@end@hd@ft`): the
/// entries before each terminator form that chunk, and what follows the
/// last terminator is the body. A terminator that repeats takes the rows
/// gathered since the previous one, exactly as the package's boxes do.
pub fn parts(table: &TableItem) -> Parts {
    let mut parts = Parts::default();
    let mut pending: Vec<usize> = Vec::new();
    for (i, entry) in table.entries.iter().enumerate() {
        match entry {
            TableEntry::Section(kind) => {
                let slot = match kind {
                    LongtableSection::FirstHead => &mut parts.first_head,
                    LongtableSection::Head => &mut parts.head,
                    LongtableSection::Foot => &mut parts.foot,
                    LongtableSection::LastFoot => &mut parts.last_foot,
                };
                slot.append(&mut pending);
            }
            _ => pending.push(i),
        }
    }
    parts.body = pending;
    parts
}

/// A longtable entry stream restricted to one chunk, so `table::layout_with`
/// can set it as its own box.
fn chunk(table: &TableItem, indices: &[usize]) -> TableItem {
    let mut out = table.clone();
    out.entries = indices.iter().filter_map(|i| table.entries.get(*i).cloned()).collect();
    out
}

/// One chunk set at its own origin (y down from its top).
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// The source entry index of each laid-out entry, so the caller can
    /// map a band back to the table.
    pub entries: Vec<usize>,
    pub geometry: table::Geometry,
    /// Height and depth of the chunk as a `\vbox`: everything above the
    /// last box's baseline, and that box's depth.
    pub height: f64,
    pub depth: f64,
}

/// Which row of `rows` a `Row` entry is, so a chunk's `layout_with` call
/// gets the measured cells belonging to its own rows.
fn row_indices(table: &TableItem, indices: &[usize]) -> Vec<usize> {
    let mut map = vec![usize::MAX; table.entries.len()];
    let mut n = 0;
    for (i, e) in table.entries.iter().enumerate() {
        if matches!(e, TableEntry::Row { .. }) {
            map[i] = n;
            n += 1;
        }
    }
    indices.iter().filter_map(|i| map.get(*i).copied()).filter(|r| *r != usize::MAX).collect()
}

/// Lays one chunk out against the shared column geometry. `placed` rows
/// and `bands` entries are renumbered back to the whole table's indices,
/// so the caller keeps one measurement map for every chunk.
pub fn chunk_geometry(table: &TableItem, rows: &[Vec<MCell>], m: &Metrics, cols: &Widths, indices: &[usize]) -> Chunk {
    let item = chunk(table, indices);
    let global_rows = row_indices(table, indices);
    let picked: Vec<Vec<MCell>> = global_rows.iter().map(|r| rows.get(*r).cloned().unwrap_or_default()).collect();
    let mut geometry = table::layout_with(&item, &picked, m, cols, table::Options { longtable: true });
    for p in &mut geometry.placed {
        if p.row == usize::MAX {
            // A caption is keyed by its entry index, not by a row.
            p.cell = indices.get(p.cell).copied().unwrap_or(p.cell);
        } else {
            p.row = global_rows.get(p.row).copied().unwrap_or(p.row);
        }
    }
    for b in &mut geometry.bands {
        b.entry = indices.get(b.entry).copied().unwrap_or(b.entry);
    }
    // A `\vbox`'s height runs to the last box's baseline; only a data row
    // has a baseline, so a chunk ending in a rule has no depth.
    let total = geometry.depth;
    let (height, depth) = match geometry.bands.iter().rev().find(|b| b.baseline.is_some()) {
        Some(b) if (b.bottom - total).abs() < 1e-9 => (b.baseline.unwrap_or(total), total - b.baseline.unwrap_or(total)),
        _ => (total, 0.0),
    };
    Chunk { entries: indices.to_vec(), geometry, height, depth }
}

/// What one item of the page's vertical list is: `\halign` contributes a
/// box per row, and `\noalign` glue between them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnitKind {
    /// A row box: a data row, or one of `\LT@hline`'s two leader rows.
    Box,
    /// `\noalign{\vskip}`: `\LT@sep`, `\addlinespace`, booktabs' rule
    /// separations, `\\[<dimen>]`. A legal breakpoint that carries no ink.
    Glue,
}

/// One item of a chunk's contribution to the vertical list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub kind: UnitKind,
    /// The chunk-relative extent this unit covers (y down from the chunk's
    /// top), so the caller can cut the geometry to it.
    pub top: f64,
    pub bottom: f64,
    /// The row baseline inside `top..bottom`, for a data row.
    pub baseline: Option<f64>,
    /// `\penalty` right before this unit; `None` for no penalty node.
    pub penalty_before: Option<i32>,
    /// The table entry this unit came from.
    pub entry: usize,
}

impl Unit {
    /// Height and depth of the unit as a box (a rule row has no depth).
    pub fn height_depth(&self) -> (f64, f64) {
        match self.baseline {
            Some(b) => (b - self.top, self.bottom - b),
            None => (self.bottom - self.top, 0.0),
        }
    }
}

/// `\penalty\@M` (longtable.sty 432, 453) and `\penalty-\@lowpenalty` /
/// `-\@medpenalty` (441, 438) around `\LT@hline`'s two leader rows.
const INF: i32 = 10_000;
const LOW: i32 = -1;
const MED: i32 = -2;
/// `\break` (longtable.sty 135): `\penalty-\@M` forces the page out.
const EJECT: i32 = -10_000;

/// The vertical-list items of a chunk, in order. Data rows and rule rows
/// are boxes; `\noalign` skips between them are glue, which is where the
/// page builder may break. `\LT@hline` contributes *two* leader rows with
/// `\LT@sep` between them, so a page can end on one rule and the next
/// begin on the other.
pub fn units(table: &TableItem, ch: &Chunk) -> Vec<Unit> {
    let arw = table.lengths.arrayrulewidth;
    let dbl = table.lengths.doublerulesep;
    let mut out: Vec<Unit> = Vec::new();
    for band in &ch.geometry.bands {
        let entry = band.entry;
        match table.entries.get(entry) {
            Some(TableEntry::Row { .. }) => {
                out.push(Unit { kind: UnitKind::Box, top: band.top, bottom: band.bottom, baseline: band.baseline, penalty_before: None, entry });
            }
            Some(TableEntry::HLine { .. }) => {
                let double = matches!(table.entries.get(entry + 1), Some(TableEntry::HLine { .. }));
                let sep = if double { dbl } else { -arw };
                let first_bottom = band.top + arw;
                out.push(Unit { kind: UnitKind::Box, top: band.top, bottom: first_bottom, baseline: None, penalty_before: Some(INF), entry });
                let second_top = first_bottom + sep;
                out.push(Unit {
                    kind: UnitKind::Glue,
                    top: first_bottom,
                    bottom: second_top,
                    baseline: None,
                    penalty_before: Some(if double { MED } else { LOW }),
                    entry,
                });
                out.push(Unit { kind: UnitKind::Box, top: second_top, bottom: band.bottom, baseline: None, penalty_before: None, entry });
                // `\penalty\@M` closes `\LT@@hline`.
                out.push(Unit { kind: UnitKind::Glue, top: band.bottom, bottom: band.bottom, baseline: None, penalty_before: Some(INF), entry });
            }
            // `\newpage`/`\pagebreak` inside a longtable: `\noalign{\break}`
            // (longtable.sty 135-137), an eject penalty before the next row.
            Some(TableEntry::PageBreak) => {
                out.push(Unit { kind: UnitKind::Glue, top: band.top, bottom: band.top, baseline: None, penalty_before: Some(EJECT), entry });
            }
            // A `\kill` row leaves no band at all; the rest are `\noalign`
            // rules or skips. Rules are boxes (a `\multispan` leader row or
            // an `\hrule`), pure spacing is glue.
            Some(TableEntry::VSpace { .. } | TableEntry::AddLineSpace { .. }) => {
                out.push(Unit { kind: UnitKind::Glue, top: band.top, bottom: band.bottom, baseline: None, penalty_before: None, entry });
            }
            Some(_) if band.bottom > band.top => {
                out.push(Unit { kind: UnitKind::Box, top: band.top, bottom: band.bottom, baseline: None, penalty_before: None, entry });
            }
            _ => {}
        }
    }
    // `\\*` is `\noalign{\nobreak}` (longtable.sty 296-298): no break
    // between that row and whatever follows it.
    for i in 0..out.len() {
        let nobreak = matches!(table.entries.get(out[i].entry), Some(TableEntry::Row { nobreak: true, .. }));
        if nobreak && i + 1 < out.len() && out[i + 1].penalty_before.is_none() {
            out[i + 1].penalty_before = Some(INF);
        }
    }
    out
}

/// Where a longtable sits across the measure: `\tabskip\LTleft` before the
/// first column and `\tabskip\LTright` after the last, with the table set
/// `to\hsize` (longtable.sty 167-171). Both default to `\fill`, so the
/// natural-width table is centred; `[l]` and `[r]` flush it to a side.
pub fn indent(align: Option<ct::LongtableAlign>, table_width: f64, measure: f64) -> f64 {
    let slack = (measure - table_width).max(0.0);
    match align {
        Some(ct::LongtableAlign::Left) => 0.0,
        Some(ct::LongtableAlign::Right) => slack,
        // `[c]` and the `\LTleft=\LTright=\fill` default share the slack.
        Some(ct::LongtableAlign::Center) | None => slack / 2.0,
    }
}

/// `\LT@makecaption` (475-485): `\multicolumn` over every column holding an
/// `\hbox to\z@` centred on the table, with a `\parbox[t]\LTcapwidth`
/// inside. "Table n: " is prefixed for a numbered `\caption`; the whole
/// caption is centred on one line when it fits in `\LTcapwidth`, and set as
/// a paragraph otherwise, followed by `\vskip\baselineskip`.
pub fn caption_width(set: Option<f64>) -> f64 {
    set.unwrap_or(LTCAPWIDTH_PT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashtex_compiler::Span;

    fn row() -> TableEntry {
        TableEntry::Row { cells: Vec::new(), extra_depth_pt: 0.0, color: None, nobreak: false, kill: false }
    }

    fn table(entries: Vec<TableEntry>) -> TableItem {
        let mut t = TableItem {
            columns: Vec::new(),
            entries,
            position: ct::VerticalPosition::Center,
            width: None,
            arraystretch: 1.0,
            size_cpt: 0,
            lengths: table::TableLengths::default(),
            array_package: false,
            span: Span::new(0, 1),
            rule_color: None,
            double_rule_sep_color: None,
            longtable: Some(ct::Longtable { align: None, number: 1 }),
        };
        t.columns.push(table::TableColumn {
            before: Vec::new(),
            align: ct::Align::Left,
            after: Vec::new(),
            fill_after: false,
            color: None,
        });
        t
    }

    #[test]
    fn terminators_close_the_chunk_before_them() {
        let t = table(vec![
            row(),                                              // 0: first head
            TableEntry::Section(LongtableSection::FirstHead),    // 1
            row(),                                              // 2: head
            TableEntry::Section(LongtableSection::Head),         // 3
            row(),                                              // 4: foot
            TableEntry::Section(LongtableSection::Foot),         // 5
            TableEntry::Section(LongtableSection::LastFoot),     // 6: empty last foot
            row(),                                              // 7: body
            row(),                                              // 8: body
        ]);
        let p = parts(&t);
        assert_eq!(p.first_head, vec![0]);
        assert_eq!(p.head, vec![2]);
        assert_eq!(p.foot, vec![4]);
        assert_eq!(p.last_foot, Vec::<usize>::new());
        assert_eq!(p.body, vec![7, 8]);
        // `\ifvoid\LT@lastfoot\LT@foot`, `\ifvoid\LT@firsthead\LT@head`.
        assert_eq!(p.opening_head(), &[0]);
        assert_eq!(p.closing_foot(), &[4]);
    }

    #[test]
    fn a_table_without_terminators_is_all_body() {
        let t = table(vec![row(), row()]);
        let p = parts(&t);
        assert_eq!(p.body, vec![0, 1]);
        assert!(p.opening_head().is_empty() && p.closing_foot().is_empty());
    }

    #[test]
    fn only_a_head_makes_it_the_first_head_too() {
        let t = table(vec![row(), TableEntry::Section(LongtableSection::Head), row()]);
        let p = parts(&t);
        assert_eq!(p.opening_head(), &[0]);
        assert_eq!(p.body, vec![2]);
    }

    #[test]
    fn ltleft_and_ltright_place_the_table() {
        // `\LTleft=\LTright=\fill`: centred; `[l]`/`[r]` flush.
        assert!((indent(None, 100.0, 300.0) - 100.0).abs() < 1e-9);
        assert!((indent(Some(ct::LongtableAlign::Center), 100.0, 300.0) - 100.0).abs() < 1e-9);
        assert_eq!(indent(Some(ct::LongtableAlign::Left), 100.0, 300.0), 0.0);
        assert!((indent(Some(ct::LongtableAlign::Right), 100.0, 300.0) - 200.0).abs() < 1e-9);
        // A table wider than the measure overhangs to the right.
        assert_eq!(indent(None, 400.0, 300.0), 0.0);
    }

    #[test]
    fn ltcapwidth_defaults_to_four_inches() {
        assert!((caption_width(None) - 289.08).abs() < 1e-9);
        assert_eq!(caption_width(Some(200.0)), 200.0);
    }
}
