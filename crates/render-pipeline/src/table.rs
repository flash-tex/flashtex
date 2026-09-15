//! `tabular`/`tabular*` in the pipeline: the LaTeX kernel's alignment model
//! and booktabs' rules, laid out over the compiler's parsed table
//! (`flashtex_compiler::tabular::Tabular`, whose column templates are built
//! the way `\@mkpream` builds the `\halign` preamble) with cells shaped by
//! the pipeline's own TFM text path (`typeset::Context::table_box`).
//!
//! Citations are to TeX Live 2026 `latex.ltx` and booktabs v1.61803398.
//!
//! * `\@tabular` (latex.ltx 16560): `\leavevmode\hbox{$ ... $}` with
//!   `\m@th`; `\@array` (16564) opens `\vtop` for `[t]`, `\vbox` for `[b]`,
//!   else `\vcenter` (centred on the math axis of the current size: cmsy's
//!   `axis_height` is 0.25 em), sets `\lineskip\z@skip\baselineskip\z@skip`
//!   so rows abut, and starts every row with `\@arstrut`, a rule of height
//!   `\arraystretch\ht\strutbox` and depth `\arraystretch\dp\strutbox`
//!   (`\strutbox` is `.7\baselineskip`/`.3\baselineskip` of the size in
//!   force, size1x.clo `\@setfontsize`).
//! * `\@tabclassz` (16652): `l`/`c`/`r` are `\hskip1sp\ignorespaces #\unskip`
//!   with `\hfil` on the open sides; `\@tabacol` (16629) puts `\tabcolsep`
//!   before and after each; `\@arrayrule` (16716) is
//!   `\hskip-.5\arrayrulewidth\vrule\hskip-.5\arrayrulewidth`, so a `|` takes
//!   no width and is centred on its boundary, running the full row height;
//!   `\@classi` puts `\doublerulesep` between `||`, `\@classii` half a rule
//!   width before `@` after `|`.
//! * `\halign` (TeX §801): a column is as wide as its widest entry; a
//!   spanned entry (`\multicolumn`, 16603) pushes only its excess over the
//!   columns before its last one into that last column; a column with no
//!   entry is zero-wide with zero `\tabskip` after it. `tabular*` (16557) is
//!   `\halign to<width>`: the leftover goes to `\extracolsep{\fill}`
//!   `\tabskip` glue; the glue after the last column is `\z@skip`.
//! * `p{w}` (`\@classv` 16702, `\@startpbox`/`\@endpbox` 16755): a `\vtop`
//!   of `\hsize` w under `\@arrayparboxrestore` (16272: no indent,
//!   `\parfillskip\@flushglue`, `\normalbaselineskip`, `\sloppy`) whose last
//!   line gets `\@finalstrut\@arstrutbox` (16396: the strut's depth).
//! * `\\[d]` (`\@argtabularcr` 16593): `d > 0` deepens the row to
//!   `d + \dp\@arstrutbox`; otherwise `\cr\noalign{\vskip d}`.
//! * `\hline` (16728): a full-width `\arrayrulewidth` rule taking its height;
//!   a second `\hline` directly after it is `\doublerulesep` below its top.
//!   `\cline{a-b}` (16738): an `\omit`ted row of leaders `\arrayrulewidth`
//!   high from column a's left edge to column b's right edge, followed by
//!   `\noalign{\vskip-\arrayrulewidth}` (no net space).
//! * booktabs: `\toprule`/`\midrule`/`\bottomrule` are `\vskip` (`\abovetopsep`
//!   = 0 for the top rule, `\aboverulesep` = .4ex otherwise, or
//!   `\doublerulesep` when the previous rule was also a booktabs rule), an
//!   `\hrule` of `\heavyrulewidth` (.08em) / `\lightrulewidth` (.05em), then
//!   `\belowrulesep` (.65ex; `\belowbottomsep` = 0 after `\bottomrule`) unless
//!   another booktabs rule follows. `\cmidrule[w](trim){a-b}` is
//!   `\aboverulesep` (if the previous rule class is 0), a row of `\cmidrulewidth`
//!   (.03em) leaders trimmed by `\cmidrulekern` (.5em) on the requested sides,
//!   then `\vskip-w` before another `\cmidrule` or `\belowrulesep`. The em/ex
//!   are those of the body font (the dimensions are assigned when booktabs is
//!   loaded).
//!
//! array.sty v2.6n (TeX Live 2026), when the document loads it, replaces the
//! preamble builder (the compiler's `array_column_templates`) and changes:
//!
//! * `\@array` (array.sty 207): `\@arstrutbox` is `\arraystretch` times
//!   `\ht\strutbox + \extrarowheight` high (the depth is unchanged);
//! * `\@arrayrule` (175) is `\vline`, a rule `\arrayrulewidth` wide that takes
//!   its width; `!{\vrule width d}` likewise;
//! * `\@startpbox` (189) starts the first paragraph with a strut of
//!   `\ht\@arstrutbox`; `p` is a `\vtop`, `b` a `\vbox`, and `m` a `\vbox`
//!   that `\ar@align@mcell` (164) lowers by half of its height less
//!   `\ht\@arstrutbox` plus `\baselineskip` when it is taller than
//!   `\ht\strutbox`; `\@array` has set `\baselineskip` to 0 there;
//! * `\@xhline` (440) puts a second `\hline` `\doublerulesep` below the first
//!   rule's bottom (the kernel subtracts `\arrayrulewidth`);
//! * `w{align}{width}` (446) sets the entry in `\makebox[width][align]`.
//!
//! Lengths the kernel allows to change (`\tabcolsep`, `\arrayrulewidth`,
//! `\doublerulesep`) are read from `\setlength` in the source by the adapter:
//! the compiler bakes the kernel defaults into its templates, so its spaces
//! of exactly those defaults are mapped back to the document's values.

use flashtex_compiler::parser::{Inline, ParagraphStyle};
use flashtex_compiler::tabular::{self as ct, Align, BookRule, BoxAlign, Length, VerticalPosition};
use flashtex_compiler::Span;

use crate::adapter::Item;

/// The measure an unbreakable entry is set in: just under TeX's `\maxdimen`
/// (16383.99998pt), which paragraph-layout rejects as a line width.
pub const MAX_DIMEN_PT: f64 = 16383.0;
/// cmsy `axis_height` (Latin Modern/Computer Modern) in em.
pub const AXIS_EM: f64 = 0.25;
/// booktabs widths (em) and separations (ex / em).
const HEAVY_RULE_EM: f64 = 0.08;
const LIGHT_RULE_EM: f64 = 0.05;
const CMID_RULE_EM: f64 = 0.03;
const ABOVE_RULE_SEP_EX: f64 = 0.4;
const BELOW_RULE_SEP_EX: f64 = 0.65;
const CMID_RULE_KERN_EM: f64 = 0.5;
/// booktabs `\defaultaddspace` (.5em) and `\cmidrulesep` (`\doublerulesep`
/// when booktabs is loaded: 2pt).
const DEFAULT_ADD_SPACE_EM: f64 = 0.5;
const CMIDRULESEP_PT: f64 = 2.0;

/// A dimension rounded to TeX's scaled points.
pub fn sp(pt: f64) -> f64 {
    (pt * 65536.0).round() / 65536.0
}

/// `\baselineskip` of a size declaration under the 10/11/12pt class options
/// (size10.clo/size11.clo/size12.clo `\@setfontsize`), keyed by the font size
/// in hundredths of a point as the adapter declares it. Unknown sizes use
/// 1.2 times the size.
pub fn baselineskip_pt(class_size: u32, size_cpt: u16) -> f64 {
    let table: &[(u16, f64)] = match class_size {
        12 => &[(600, 7.0), (800, 9.5), (1000, 12.0), (1095, 13.6), (1200, 14.5), (1440, 18.0), (1728, 22.0), (2074, 25.0), (2488, 30.0)],
        11 => &[(600, 7.0), (800, 9.5), (900, 11.0), (1000, 12.0), (1095, 13.6), (1200, 14.0), (1440, 18.0), (1728, 22.0), (2074, 25.0), (2488, 30.0)],
        _ => &[(500, 6.0), (700, 8.0), (800, 9.5), (900, 11.0), (1000, 12.0), (1200, 14.0), (1440, 18.0), (1728, 22.0), (2074, 25.0), (2488, 30.0)],
    };
    table.iter().find(|(s, _)| *s == size_cpt).map_or(f64::from(size_cpt) / 100.0 * 1.2, |(_, b)| *b)
}

pub fn resolve(length: Length, measure: f64) -> f64 {
    match length {
        Length::Pt(pt) => pt,
        Length::TextWidth(factor) => factor * measure,
    }
}

/// The kernel lengths a document may change with `\setlength`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableLengths {
    pub tabcolsep: f64,
    pub arrayrulewidth: f64,
    pub doublerulesep: f64,
    /// array.sty's `\extrarowheight` (0pt by default).
    pub extrarowheight: f64,
}

impl Default for TableLengths {
    fn default() -> Self {
        TableLengths {
            tabcolsep: ct::TABCOLSEP_PT,
            arrayrulewidth: ct::ARRAYRULEWIDTH_PT,
            doublerulesep: ct::DOUBLERULESEP_PT,
            extrarowheight: 0.0,
        }
    }
}

impl TableLengths {
    /// Reads each length through `value(name, default)` (a lookup of the
    /// document's assignments, which an `\addtolength` makes relative to
    /// the default), keeping the kernel default when the document does not
    /// set it.
    pub fn read(mut value: impl FnMut(&str, f64) -> Option<f64>) -> TableLengths {
        let d = TableLengths::default();
        TableLengths {
            tabcolsep: value("tabcolsep", d.tabcolsep).unwrap_or(d.tabcolsep),
            arrayrulewidth: value("arrayrulewidth", d.arrayrulewidth).unwrap_or(d.arrayrulewidth),
            doublerulesep: value("doublerulesep", d.doublerulesep).unwrap_or(d.doublerulesep),
            extrarowheight: value("extrarowheight", d.extrarowheight).unwrap_or(d.extrarowheight),
        }
    }

    /// A compiler template space, which is one of the kernel defaults the
    /// preamble builder inserts, at this document's value.
    fn space(&self, pt: f64) -> f64 {
        if pt == ct::TABCOLSEP_PT {
            self.tabcolsep
        } else if pt == ct::DOUBLERULESEP_PT {
            self.doublerulesep
        } else if pt == ct::ARRAYRULEWIDTH_PT / 2.0 {
            self.arrayrulewidth / 2.0
        } else {
            pt
        }
    }
}

/// A table as the pipeline sets it: the compiler's structure with every
/// inline list already converted to pipeline items.
#[derive(Debug, Clone, PartialEq)]
pub struct TableItem {
    pub columns: Vec<TableColumn>,
    pub entries: Vec<TableEntry>,
    pub position: VerticalPosition,
    pub width: Option<Length>,
    pub arraystretch: f64,
    /// Size declaration in force at `\begin` (hundredths of a point; 0 keeps
    /// the surrounding size).
    pub size_cpt: u16,
    pub lengths: TableLengths,
    /// The preamble is array.sty's (see the module comment).
    pub array_package: bool,
    pub span: Span,
    /// colortbl `\arrayrulecolor`/`\doublerulesepcolor` at `\begin`.
    pub rule_color: Option<ct::ColorSpec>,
    pub double_rule_sep_color: Option<ct::ColorSpec>,
    /// longtable (see `crate::longtable`).
    pub longtable: Option<ct::Longtable>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub before: Vec<TableMaterial>,
    pub align: Align,
    pub after: Vec<TableMaterial>,
    /// `\extracolsep{\fill}` `\tabskip` glue after this column.
    pub fill_after: bool,
    /// colortbl `>{\columncolor}`.
    pub color: Option<ct::ColorFill>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TableMaterial {
    Space(f64),
    Rule(Span),
    /// A rule that takes its width (array's `|`, `!{\vrule}`).
    VLine(Span, f64),
    Text(Vec<Item>),
    /// The `\doublerulesep` between `||` (colortbl `\@classvi`: a
    /// `\vrule` of `\doublerulesepcolor` when one is set, else a skip).
    DoubleRuleGap(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    pub items: Vec<Item>,
    pub columns: usize,
    pub template: Option<TableColumn>,
    /// `\centering`/`\raggedright`/`\raggedleft` of a `p`/`m`/`b` entry.
    pub alignment: Option<ParagraphStyle>,
    /// colortbl `\cellcolor`.
    pub color: Option<ct::ColorSpec>,
    /// multirow `\multirow` (the entry's items are its text).
    pub multirow: Option<ct::Multirow>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TableEntry {
    Row {
        cells: Vec<TableCell>,
        extra_depth_pt: f64,
        /// colortbl `\rowcolor`.
        color: Option<ct::ColorFill>,
        /// longtable `\\*`.
        nobreak: bool,
        /// longtable `\kill`: widths only.
        kill: bool,
    },
    HLine { span: Span },
    CLine { first: usize, last: usize, span: Span },
    BookRule { kind: BookRule, width_pt: Option<f64>, span: Span },
    CMidRule {
        first: usize,
        last: usize,
        trim_left: bool,
        trim_right: bool,
        width_pt: Option<f64>,
        /// booktabs `\@setrulekerning`: `(l{<dimen>})`/`(r{<dimen>})` replacing
        /// `\cmidrulekern`, in the font in force at the rule.
        kern_left: Option<ct::FontDimen>,
        kern_right: Option<ct::FontDimen>,
        span: Span,
    },
    VSpace { pt: f64 },
    /// booktabs `\addlinespace[<dimen>]` (`None`: `\defaultaddspace`).
    AddLineSpace { space: Option<ct::FontDimen> },
    SpecialRule { width_pt: f64, above_pt: f64, below_pt: f64, span: Span },
    MoreCmidRules,
    RuleColor(ct::ColorSpec),
    DoubleRuleSepColor(ct::ColorSpec),
    Section(ct::LongtableSection),
    /// longtable `\caption` (`\LT@makecaption`): a `\multicolumn` row over
    /// every column holding a `\parbox[t]\LTcapwidth`. `box_` is that
    /// parbox once the typesetter has set it; until then the entry has no
    /// extent.
    Caption { items: Vec<Item>, number: Option<u32>, span: Span, box_: Option<CaptionBox> },
    PageBreak,
}

fn material(m: &[ct::Material], lengths: TableLengths, items_of: &mut dyn FnMut(&[Inline], bool) -> Vec<Item>) -> Vec<TableMaterial> {
    let mut out = Vec::with_capacity(m.len());
    for (i, piece) in m.iter().enumerate() {
        let is_rule = |k: Option<&ct::Material>| matches!(k, Some(ct::Material::Rule(_) | ct::Material::VLine { .. }));
        out.push(match piece {
            // `\@classvi` puts exactly `\doublerulesep` between two rules.
            ct::Material::Space(pt) if *pt == ct::DOUBLERULESEP_PT && i > 0 && is_rule(m.get(i - 1)) && is_rule(m.get(i + 1)) => {
                TableMaterial::DoubleRuleGap(lengths.doublerulesep)
            }
            ct::Material::Space(pt) => TableMaterial::Space(lengths.space(*pt)),
            ct::Material::Rule(span) => TableMaterial::Rule(*span),
            ct::Material::VLine { span, width_pt } => TableMaterial::VLine(*span, width_pt.unwrap_or(lengths.arrayrulewidth)),
            ct::Material::Text(inlines) => TableMaterial::Text(items_of(inlines, false)),
        });
    }
    out
}

fn column(t: &ct::ColumnTemplate, lengths: TableLengths, items_of: &mut dyn FnMut(&[Inline], bool) -> Vec<Item>) -> TableColumn {
    TableColumn {
        before: material(&t.before, lengths, items_of),
        align: t.align,
        after: material(&t.after, lengths, items_of),
        fill_after: t.fill_after,
        color: t.color.clone(),
    }
}

/// Converts the compiler's table, turning every inline list into items with
/// `items_of` (its flag: take weight and shape from the compiler's scoping,
/// for entries with array `>{}`/`<{}` declarations).
pub fn from_compiler(t: &ct::Tabular, lengths: TableLengths, size_cpt: u16, items_of: &mut dyn FnMut(&[Inline], bool) -> Vec<Item>) -> TableItem {
    let columns = t.columns.iter().map(|c| column(c, lengths, items_of)).collect();
    let mut entries = Vec::with_capacity(t.entries.len());
    for e in &t.entries {
        entries.push(match e {
            ct::Entry::Row(row) => TableEntry::Row {
                cells: row
                    .cells
                    .iter()
                    .map(|c| TableCell {
                        items: items_of(&c.content, c.declarations),
                        columns: c.columns,
                        template: c.template.as_ref().map(|tp| column(tp, lengths, items_of)),
                        alignment: c.alignment,
                        color: c.color.clone(),
                        multirow: c.multirow.clone(),
                    })
                    .collect(),
                extra_depth_pt: row.extra_depth_pt,
                color: row.color.clone(),
                nobreak: row.nobreak,
                kill: row.kill,
            },
            ct::Entry::HLine { span } => TableEntry::HLine { span: *span },
            ct::Entry::CLine { first, last, span } => TableEntry::CLine { first: *first, last: *last, span: *span },
            ct::Entry::BookRule { kind, width_pt, span } => TableEntry::BookRule { kind: *kind, width_pt: *width_pt, span: *span },
            ct::Entry::CMidRule { first, last, trim_left, trim_right, width_pt, kern_left, kern_right, span } => TableEntry::CMidRule {
                first: *first,
                last: *last,
                trim_left: *trim_left,
                trim_right: *trim_right,
                width_pt: *width_pt,
                kern_left: *kern_left,
                kern_right: *kern_right,
                span: *span,
            },
            ct::Entry::VSpace { pt } => TableEntry::VSpace { pt: *pt },
            ct::Entry::AddLineSpace { space, .. } => TableEntry::AddLineSpace { space: *space },
            ct::Entry::SpecialRule { width_pt, above_pt, below_pt, span } => TableEntry::SpecialRule {
                width_pt: *width_pt,
                above_pt: *above_pt,
                below_pt: *below_pt,
                span: *span,
            },
            ct::Entry::MoreCmidRules { .. } => TableEntry::MoreCmidRules,
            ct::Entry::RuleColor { color } => TableEntry::RuleColor(color.clone()),
            ct::Entry::DoubleRuleSepColor { color } => TableEntry::DoubleRuleSepColor(color.clone()),
            ct::Entry::Section { kind, .. } => TableEntry::Section(*kind),
            ct::Entry::Caption { content, number, span } => {
                // `\LT@c@ption` sets `#1{#2: }#3`: `\fnum@table` is
                // `\tablename~\thetable`, with a tie, and `\caption*`
                // (`number` `None`) drops the whole prefix.
                let mut items = Vec::new();
                if let Some(n) = number {
                    items.extend(crate::adapter::command_words(&format!("{} {n}:", crate::floats::FloatKind::Table.name()), *span));
                    if let Some(Item::Space { no_break, .. }) = items.get_mut(1) {
                        *no_break = true;
                    }
                    // The space after `:` carries TeX's space factor
                    // (`\sfcode`\:` = 2000), so it takes the face's
                    // `\fontdimen7` extra space.
                    items.push(Item::Space {
                        style: crate::adapter::TextStyle::default(),
                        factor: crate::adapter::space_factor(':', 1000),
                        no_break: false,
                    });
                }
                items.extend(items_of(content, false));
                TableEntry::Caption { items, number: *number, span: *span, box_: None }
            }
            ct::Entry::PageBreak { .. } => TableEntry::PageBreak,
        });
    }
    TableItem {
        columns,
        entries,
        position: t.position,
        width: t.width,
        arraystretch: t.arraystretch,
        size_cpt,
        lengths,
        array_package: t.array_package,
        span: t.span,
        rule_color: t.rule_color.clone(),
        double_rule_sep_color: t.double_rule_sep_color.clone(),
        longtable: t.longtable.clone(),
    }
}

/// Width, height and depth of a set box.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dims {
    pub width: f64,
    pub height: f64,
    pub depth: f64,
}

/// A measured `u`/`v` template piece.
#[derive(Debug, Clone, PartialEq)]
pub enum MPiece {
    Space(f64),
    Rule(Span),
    VLine(Span, f64),
    Text(Dims),
    DoubleRuleGap(f64),
}

impl MPiece {
    fn width(&self) -> f64 {
        match self {
            MPiece::Space(pt) => *pt,
            MPiece::Rule(_) => 0.0,
            MPiece::VLine(_, width) => *width,
            MPiece::Text(d) => d.width,
            MPiece::DoubleRuleGap(width) => *width,
        }
    }
}

/// A measured entry: template pieces around the entry box.
#[derive(Debug, Clone, PartialEq)]
pub struct MCell {
    pub column: usize,
    pub columns: usize,
    pub align: Align,
    pub before: Vec<MPiece>,
    /// The entry: an hbox for `l`/`c`/`r`, the `\vtop` for `p{}`, the
    /// (lowered) `\vbox` for `m{}`/`b{}`, the `\makebox` for `w{}{}`.
    pub content: Dims,
    pub after: Vec<MPiece>,
    /// Where the entry's first line sits relative to the box's reference
    /// point: x within the box (`w`), y below it (`m`/`b`: negative).
    pub content_offset: (f64, f64),
}

/// The lines of a set paragraph entry: the first line's height, the distance
/// from the first to the last baseline, and the last line's depth.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ParLines {
    pub first_height: f64,
    pub inner: f64,
    pub last_depth: f64,
}

/// The box of a `p`/`m`/`b` entry and its first baseline's offset below the
/// box's reference point (`\@startpbox`/`\@endpbox`, `\ar@align@mcell`).
pub fn parbox(align: Align, lines: ParLines, array_package: bool, m: &Metrics) -> (Dims, f64) {
    let width = match align.paragraph_width() {
        Some(len) => resolve(len, m.measure).max(0.0),
        None => 0.0,
    };
    // array: `\everypar{\vrule\@height\ht\@arstrutbox}`; both: the final
    // strut's depth.
    let first = if array_package { lines.first_height.max(m.strut_height) } else { lines.first_height };
    let last = lines.last_depth.max(m.strut_depth);
    match align {
        Align::Bottom(_) => (Dims { width, height: first + lines.inner, depth: last }, -lines.inner),
        Align::Middle(_) => {
            let height = first + lines.inner;
            // `\lower.5\dimen@` with `\dimen@ = \ht - \ht\@arstrutbox +
            // \baselineskip`, and `\baselineskip` is 0 inside `\@array`.
            let s = if height > m.plain_strut_height { sp(0.5 * sp(height - m.strut_height)) } else { 0.0 };
            (Dims { width, height: height - s, depth: last + s }, s - lines.inner)
        }
        _ => (Dims { width, height: first, depth: lines.inner + last }, 0.0),
    }
}

/// `\makebox[width][align]`: the box width and the entry's x within it.
pub fn fixed_box(align: BoxAlign, width: f64, natural: f64) -> (f64, f64) {
    let dx = match align {
        BoxAlign::Left => 0.0,
        BoxAlign::Center => (width - natural) / 2.0,
        BoxAlign::Right => width - natural,
    };
    (width, dx)
}

impl MCell {
    fn natural(&self) -> f64 {
        self.before.iter().map(MPiece::width).sum::<f64>() + self.content.width + self.after.iter().map(MPiece::width).sum::<f64>()
    }
}

/// The columns each cell of a row occupies and its template (`None` when
/// the row has more cells than the preamble has columns: TeX's "extra
/// alignment tab" recovery, which the compiler already diagnosed).
pub fn row_slots<'t>(table: &'t TableItem, cells: &'t [TableCell]) -> Vec<(usize, usize, Option<&'t TableColumn>)> {
    let n = table.columns.len().max(1);
    let mut out = Vec::with_capacity(cells.len());
    let mut column = 0;
    for cell in cells {
        if column >= n {
            break;
        }
        let columns = cell.columns.clamp(1, n - column);
        out.push((column, columns, cell.template.as_ref().or_else(|| table.columns.get(column))));
        column += columns;
    }
    out
}

/// Which part of a cell a placed box is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    Before(usize),
    Content,
    After(usize),
}

/// A box placed in the table: `baseline` is its reference point's y below
/// the table's baseline (negative above), `x` its left edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub row: usize,
    pub cell: usize,
    pub slot: Slot,
    pub x: f64,
    pub baseline: f64,
}

/// A longtable caption's `\parbox[t]\LTcapwidth` once it is set
/// (`\LT@makecaption`, longtable.sty 475-485). `\parbox[t]` is a `\vtop`:
/// its reference point is the first line's baseline, so `height` is that
/// line's height and `depth` everything below it — the remaining lines,
/// the last line's depth and the closing `\vskip\baselineskip`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaptionBox {
    /// `\LTcapwidth`: the parbox is centred on the table, and a caption
    /// that fits on one line is centred inside it.
    pub width: f64,
    pub height: f64,
    pub depth: f64,
}

/// A filled rule; `top` is relative to the table's baseline (y down).
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedRule {
    pub x: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub span: Span,
    /// colortbl colour as written (`None`: black).
    pub color: Option<ct::ColorSpec>,
    /// The colour resolved to sRGB by the typesetter.
    pub rgb: Option<[f64; 3]>,
}

/// The vertical extent of one table entry (y down from the table's
/// baseline); `baseline` for rows. Kill rows have none.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub entry: usize,
    pub top: f64,
    pub baseline: Option<f64>,
    pub bottom: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    pub placed: Vec<Placed>,
    pub rules: Vec<PlacedRule>,
    /// colortbl fills, painted before the entries and rules.
    pub fills: Vec<PlacedRule>,
    pub bands: Vec<Band>,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
}

/// Font- and page-dependent values the geometry needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// `\ht\@arstrutbox`/`\dp\@arstrutbox`.
    pub strut_height: f64,
    pub strut_depth: f64,
    /// `\ht\strutbox` (`\ar@align@mcell`'s threshold).
    pub plain_strut_height: f64,
    /// Body font quad and x-height (booktabs' em/ex).
    pub em: f64,
    pub ex: f64,
    /// Math axis height at the table's size (`\vcenter`).
    pub axis: f64,
    /// `\textwidth`, for `p{.5\textwidth}` and `tabular*{\textwidth}`.
    pub measure: f64,
}

/// Lays the table out. `rows[i]` holds the measured cells of the i-th
/// `Row` entry, in the order [`row_slots`] returns them.
pub fn layout(table: &TableItem, rows: &[Vec<MCell>], m: &Metrics) -> Geometry {
    let widths = widths(table, rows, m);
    layout_with(table, rows, m, &widths, Options::default())
}

/// The column geometry of a table: what TeX’s `\halign` settles after
/// §801. longtable measures it over every row of every chunk (head, foot
/// and `\kill` rows included) and then sets each chunk with it, so it is
/// computed apart from the vertical pass.
#[derive(Debug, Clone, PartialEq)]
pub struct Widths {
    /// Left edge of each column, plus the table's right edge.
    pub column_x: Vec<f64>,
    /// Natural width of each column (without the following `\tabskip`).
    pub widths: Vec<f64>,
    /// The whole table's width (`\hsize` for `tabular*`).
    pub box_width: f64,
}

impl Widths {
    /// The right edge of column `k`'s material (before its `\tabskip`).
    fn right_of(&self, k: usize) -> f64 {
        self.column_x[k] + self.widths[k]
    }
}

/// How [`layout_with`] differs from a plain `tabular`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Options {
    /// longtable: `\hline` is `\LT@hline` (longtable.sty 430-454), two
    /// `\multispan` leader rows `\LT@sep` apart rather than one `\hrule`,
    /// and the geometry keeps the vertical origin at the top (each chunk is
    /// its own box in the page's vertical list, not a box on a baseline).
    pub longtable: bool,
}

/// TeX §801 over every row: the column widths and the table's width.
pub fn widths(table: &TableItem, rows: &[Vec<MCell>], m: &Metrics) -> Widths {
    let n = table.columns.len().max(1);

    // TeX §801: w[k][j] is the widest entry spanning columns k..=j.
    let mut w = vec![vec![f64::NEG_INFINITY; n]; n];
    for cell in rows.iter().flatten() {
        let first = cell.column.min(n - 1);
        let last = (cell.column + cell.columns.max(1) - 1).min(n - 1);
        w[first][last] = w[first][last].max(cell.natural());
    }
    let mut widths = vec![0.0f64; n];
    let mut tabskip = vec![0.0f64; n];
    let mut fill = vec![false; n];
    for k in 0..n {
        let has_entries = w[k][k] > f64::NEG_INFINITY;
        widths[k] = if has_entries { w[k][k] } else { 0.0 };
        fill[k] = has_entries && k + 1 < n && table.columns.get(k).is_some_and(|c| c.fill_after);
        if k + 1 < n {
            for j in k + 1..n {
                if w[k][j] > f64::NEG_INFINITY {
                    let excess = w[k][j] - widths[k] - tabskip[k];
                    w[k + 1][j] = w[k + 1][j].max(excess);
                }
            }
        }
    }
    let natural: f64 = widths.iter().sum::<f64>() + tabskip.iter().sum::<f64>();
    let box_width = match table.width {
        Some(len) => {
            let target = resolve(len, m.measure);
            let fills = fill.iter().filter(|f| **f).count();
            if fills > 0 && target > natural {
                let share = (target - natural) / fills as f64;
                for (skip, f) in tabskip.iter_mut().zip(&fill) {
                    if *f {
                        *skip += share;
                    }
                }
            }
            target
        }
        None => natural,
    };
    let mut column_x = vec![0.0f64; n + 1];
    for k in 0..n {
        column_x[k + 1] = column_x[k] + widths[k] + tabskip[k];
    }
    Widths { column_x, widths, box_width }
}

/// The vertical pass over `table.entries` with the column geometry already
/// settled, so a longtable can set several chunks against one measurement.
pub fn layout_with(table: &TableItem, rows: &[Vec<MCell>], m: &Metrics, cols: &Widths, opts: Options) -> Geometry {
    let n = table.columns.len().max(1);
    let arw = table.lengths.arrayrulewidth;
    let dbl = table.lengths.doublerulesep;
    let column_x = &cols.column_x;
    let box_width = cols.box_width;
    let right_of = |k: usize| cols.right_of(k);

    let mut placed = Vec::new();
    let mut rules: Vec<PlacedRule> = Vec::new();
    let mut fills: Vec<PlacedRule> = Vec::new();
    let mut bands: Vec<Band> = Vec::new();
    // `|` rules as (x, width, top, bottom, span, colour), merged where rows abut.
    let mut vrules: Vec<(f64, f64, f64, f64, Span, Option<ct::ColorSpec>)> = Vec::new();
    let mut y = 0.0f64;
    let mut first_height: Option<f64> = None;
    let mut last_depth = 0.0;
    let mut last_rule_class = 0u8;
    let mut row_index = 0usize;
    // colortbl `\CT@arc@`/`\CT@drsc@` (global, so they change mid-table).
    let mut rule_color = table.rule_color.clone();
    let mut gap_color = table.double_rule_sep_color.clone();
    let rule = |rules: &mut Vec<PlacedRule>, x: f64, top: f64, width: f64, height: f64, span: Span, color: &Option<ct::ColorSpec>| {
        if width > 0.0 && height > 0.0 {
            rules.push(PlacedRule { x, top, width, height, span, color: color.clone(), rgb: None });
        }
    };
    for (index, entry) in table.entries.iter().enumerate() {
        let next = table.entries.get(index + 1);
        // booktabs `\@BTendrule` (booktabs.sty 110-116) peeks at the next
        // token for another booktabs rule.
        let next_is_booktabs = matches!(
            next,
            Some(TableEntry::BookRule { .. } | TableEntry::CMidRule { .. } | TableEntry::SpecialRule { .. } | TableEntry::AddLineSpace { .. })
        );
        last_depth = 0.0;
        let band_top = y;
        let mut band_baseline = None;
        match entry {
            TableEntry::Row { extra_depth_pt, kill, color: row_color, cells: tcells, .. } => {
                let ri = row_index;
                row_index += 1;
                if *kill {
                    // longtable `\kill`: the row only gave its widths.
                    continue;
                }
                let cells = rows.get(ri).map_or(&[][..], Vec::as_slice);
                let mut height = m.strut_height;
                let mut depth = m.strut_depth;
                if *extra_depth_pt > 0.0 {
                    depth = depth.max(m.strut_depth + extra_depth_pt);
                }
                for cell in cells {
                    height = height.max(cell.content.height);
                    depth = depth.max(cell.content.depth);
                    for piece in cell.before.iter().chain(&cell.after) {
                        if let MPiece::Text(d) = piece {
                            height = height.max(d.height);
                            depth = depth.max(d.depth);
                        }
                    }
                }
                first_height.get_or_insert(height);
                let top = y;
                let baseline = y + height;
                band_baseline = Some(baseline);
                for (ci, cell) in cells.iter().enumerate() {
                    let first = cell.column.min(n - 1);
                    let last = (cell.column + cell.columns.max(1) - 1).min(n - 1);
                    let left = column_x[first];
                    let right = right_of(last);
                    let before_width: f64 = cell.before.iter().map(MPiece::width).sum();
                    let after_width: f64 = cell.after.iter().map(MPiece::width).sum();
                    // colortbl `\@classz` (colortbl.sty 60-66): column, row and
                    // cell colour are `\color`s in that order (the last wins),
                    // painted by `\CT@@do@color` (78-88) as leaders `\vrule`
                    // running the row's height and depth, from `\@tempdimb`
                    // before the entry box to `\@tempdimc` after the stretched
                    // entry. Both default to `\col@sep`; a column's then a
                    // row's `[left][right]` replace them.
                    let tcell = tcells.get(ci);
                    let template = tcell.and_then(|c| c.template.as_ref()).or_else(|| table.columns.get(cell.column));
                    let column_fill = template.and_then(|t| t.color.as_ref());
                    let color = tcell
                        .and_then(|c| c.color.clone())
                        .or_else(|| row_color.as_ref().map(|f| f.color.clone()))
                        .or_else(|| column_fill.map(|f| f.color.clone()));
                    if let Some(color) = color {
                        let (mut b, mut c) = (table.lengths.tabcolsep, table.lengths.tabcolsep);
                        for fill in [column_fill, row_color.as_ref()].into_iter().flatten() {
                            if let Some(l) = fill.left_pt {
                                b = l;
                            }
                            if let Some(r) = fill.right_pt {
                                c = r;
                            }
                        }
                        let x0 = left + before_width - b;
                        let x1 = right - after_width + c;
                        let span = color.span;
                        rule(&mut fills, x0, top, x1 - x0, height + depth, span, &Some(color));
                    }
                    let mut place = |piece: &MPiece, x: f64, slot: Slot, placed: &mut Vec<Placed>| match piece {
                        MPiece::Space(_) => {}
                        // colortbl `\@classvi` and `\@arrayrule` (colortbl.sty
                        // 146-166) put `{\CT@drsc@\vrule\@width\doublerulesep}`
                        // and `{\CT@arc@\vline}` into the preamble with
                        // `\@addtopreamble`'s `\edef`: the colours in force at
                        // `\begin` are baked in, whatever changes mid-table.
                        MPiece::DoubleRuleGap(width) => {
                            if table.double_rule_sep_color.is_some() {
                                vrules.push((x, *width, top, top + height + depth, table.span, table.double_rule_sep_color.clone()))
                            }
                        }
                        MPiece::Rule(span) => vrules.push((x - arw / 2.0, arw, top, top + height + depth, *span, table.rule_color.clone())),
                        MPiece::VLine(span, width) => vrules.push((x, *width, top, top + height + depth, *span, table.rule_color.clone())),
                        MPiece::Text(_) => placed.push(Placed { row: ri, cell: ci, slot, x, baseline }),
                    };
                    let mut x = left;
                    for (pi, piece) in cell.before.iter().enumerate() {
                        place(piece, x, Slot::Before(pi), &mut placed);
                        x += piece.width();
                    }
                    let content_x = match cell.align {
                        Align::Left | Align::Paragraph(_) | Align::Middle(_) | Align::Bottom(_) | Align::Fixed(..) => x,
                        Align::Right => right - after_width - cell.content.width,
                        Align::Center => x + (right - after_width - x - cell.content.width) / 2.0,
                    };
                    placed.push(Placed {
                        row: ri,
                        cell: ci,
                        slot: Slot::Content,
                        x: content_x + cell.content_offset.0,
                        baseline: baseline + cell.content_offset.1,
                    });
                    let mut x = right - after_width;
                    for (pi, piece) in cell.after.iter().enumerate() {
                        place(piece, x, Slot::After(pi), &mut placed);
                        x += piece.width();
                    }
                }
                y += height + depth;
                last_depth = depth;
            }
            // The second `\hline` of `\hline\hline` was gobbled by
            // `\@gtempa` (longtable.sty 437, 454): the pair is one rule row,
            // `\doublerulesep`, another rule row.
            TableEntry::HLine { .. } if opts.longtable && matches!(table.entries.get(index.wrapping_sub(1)), Some(TableEntry::HLine { .. })) => {}
            TableEntry::HLine { span } if opts.longtable => {
                // `\LT@hline` (longtable.sty 430-454): `\hline` is two
                // `\multispan\LT@cols` leader rows of `\arrayrulewidth`
                // with `\LT@sep` between them, not one `\hrule`. A single
                // `\hline` separates them by `-\arrayrulewidth`, so they
                // coincide; `\hline\hline` (the second one gobbled) by
                // `\doublerulesep`. Both rows are real boxes, so a
                // longtable paints two rules where a tabular paints one.
                first_height.get_or_insert(arw);
                let double = matches!(next, Some(TableEntry::HLine { .. }));
                rule(&mut rules, 0.0, y, box_width, arw, *span, &rule_color);
                y += arw;
                let sep = if double { dbl } else { -arw };
                if double && gap_color.is_some() {
                    rule(&mut rules, 0.0, y, box_width, sep, *span, &gap_color);
                }
                y += sep;
                rule(&mut rules, 0.0, y, box_width, arw, *span, &rule_color);
                y += arw;
            }
            TableEntry::HLine { span } => {
                first_height.get_or_insert(arw);
                rule(&mut rules, 0.0, y, box_width, arw, *span, &rule_color);
                y += arw;
                if matches!(next, Some(TableEntry::HLine { .. })) {
                    let gap = if table.array_package { dbl } else { dbl - arw };
                    // colortbl `\@xhline` (colortbl.sty 175-183).
                    if gap_color.is_some() {
                        rule(&mut rules, 0.0, y, box_width, gap, *span, &gap_color);
                    }
                    y += gap;
                }
            }
            TableEntry::CLine { first, last, span } => {
                first_height.get_or_insert(arw);
                let (first, last) = ((*first).min(n - 1), (*last).min(n - 1));
                let x = column_x[first];
                rule(&mut rules, x, y, right_of(last) - x, arw, *span, &rule_color);
            }
            TableEntry::VSpace { pt } => {
                first_height.get_or_insert(0.0);
                y += pt;
            }
            TableEntry::BookRule { kind, width_pt, span } => {
                let width = width_pt.unwrap_or(match kind {
                    BookRule::Mid => LIGHT_RULE_EM * m.em,
                    BookRule::Top | BookRule::Bottom => HEAVY_RULE_EM * m.em,
                });
                // `\@BTrule` always appends its `\vskip` first, so a `[t]`
                // table opening with a booktabs rule has zero height.
                first_height.get_or_insert(0.0);
                // booktabs.sty 94-96: `\@aboverulesep` after class 0 (0 for
                // `\toprule`), `\doublerulesep` after class 1, nothing after
                // a class 2 `\specialrule`/`\addlinespace`.
                y += match last_rule_class {
                    0 if *kind == BookRule::Top => 0.0,
                    0 => ABOVE_RULE_SEP_EX * m.ex,
                    1 => dbl,
                    _ => 0.0,
                };
                rule(&mut rules, 0.0, y, box_width, width, *span, &rule_color);
                y += width;
                last_rule_class = if next_is_booktabs { 1 } else { 0 };
                if last_rule_class != 1 && *kind != BookRule::Bottom {
                    y += BELOW_RULE_SEP_EX * m.ex;
                }
            }
            TableEntry::SpecialRule { width_pt, above_pt, below_pt, span } => {
                // booktabs.sty 77-79, 94, 117: class 2 always takes its skips.
                first_height.get_or_insert(0.0);
                y += above_pt;
                rule(&mut rules, 0.0, y, box_width, *width_pt, *span, &rule_color);
                y += width_pt;
                last_rule_class = if next_is_booktabs { 2 } else { 0 };
                y += below_pt;
            }
            TableEntry::AddLineSpace { space } => {
                // booktabs.sty 80-83: `\@belowrulesep` is the space, class 2.
                first_height.get_or_insert(0.0);
                last_rule_class = if next_is_booktabs { 2 } else { 0 };
                y += space.map_or(DEFAULT_ADD_SPACE_EM * m.em, |d| d.resolve(m.em, m.ex));
            }
            TableEntry::CMidRule { first, last, trim_left, trim_right, width_pt, kern_left, kern_right, span } => {
                let width = width_pt.unwrap_or(CMID_RULE_EM * m.em);
                if last_rule_class == 0 {
                    first_height.get_or_insert(0.0);
                    y += ABOVE_RULE_SEP_EX * m.ex;
                } else {
                    first_height.get_or_insert(width);
                }
                let (first, last) = ((*first).min(n - 1), (*last).min(n - 1));
                let kern = CMID_RULE_KERN_EM * m.em;
                let left = column_x[first] + if *trim_left { kern_left.map_or(kern, |d| d.resolve(m.em, m.ex)) } else { 0.0 };
                let right = right_of(last) - if *trim_right { kern_right.map_or(kern, |d| d.resolve(m.em, m.ex)) } else { 0.0 };
                rule(&mut rules, left, y, right - left, width, *span, &rule_color);
                y += width;
                // `\@xcmidrule` (booktabs.sty 151-161).
                match next {
                    Some(TableEntry::CMidRule { .. }) => {
                        y -= width;
                        last_rule_class = 1;
                    }
                    Some(TableEntry::MoreCmidRules) => {
                        y += CMIDRULESEP_PT;
                        last_rule_class = 1;
                    }
                    _ => {
                        y += BELOW_RULE_SEP_EX * m.ex;
                        last_rule_class = 0;
                    }
                }
            }
            TableEntry::MoreCmidRules => {}
            TableEntry::RuleColor(color) => rule_color = Some(color.clone()),
            TableEntry::DoubleRuleSepColor(color) => gap_color = Some(color.clone()),
            // `\LT@makecaption`: `\LT@mcol\LT@cols c{\hbox to\z@{\hss
            // <parbox> \hss}}` — a row like any other, so it takes the
            // `\@arstrut`, and a zero-width box centred in the table, so a
            // wide caption never widens a column.
            TableEntry::Caption { box_: Some(b), .. } => {
                let height = m.strut_height.max(b.height);
                let depth = m.strut_depth.max(b.depth);
                first_height.get_or_insert(height);
                let baseline = y + height;
                band_baseline = Some(baseline);
                placed.push(Placed {
                    row: usize::MAX,
                    cell: index,
                    slot: Slot::Content,
                    x: (box_width - b.width) / 2.0,
                    baseline,
                });
                y += height + depth;
                last_depth = depth;
            }
            // The longtable block (`crate::longtable`) handles these.
            TableEntry::Section(_) | TableEntry::Caption { .. } | TableEntry::PageBreak => {}
        }
        bands.push(Band { entry: index, top: band_top, baseline: band_baseline, bottom: y });
    }

    // One rule per `|` over consecutive rows it runs through.
    vrules.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.total_cmp(&b.2)));
    let mut merged: Vec<(f64, f64, f64, f64, Span, Option<ct::ColorSpec>)> = Vec::new();
    for r in vrules {
        match merged.last_mut() {
            Some(last) if last.0 == r.0 && last.1 == r.1 && last.4 == r.4 && last.5 == r.5 && (last.3 - r.2).abs() < 1e-9 => last.3 = r.3,
            _ => merged.push(r),
        }
    }
    for (x, width, top, bottom, span, color) in merged {
        rule(&mut rules, x, top, width, bottom - top, span, &color);
    }

    let total = y;
    // longtable chunks go into the page's vertical list as boxes of their
    // own, with `\baselineskip\z@` (longtable.sty 191): the origin stays
    // at the top and the caller reads the bands.
    let reference = if opts.longtable {
        0.0
    } else {
        match table.position {
            VerticalPosition::Top => first_height.unwrap_or(0.0),
            VerticalPosition::Bottom => total - last_depth,
            VerticalPosition::Center => total / 2.0 + m.axis,
        }
    };
    for p in &mut placed {
        p.baseline -= reference;
    }
    for r in rules.iter_mut().chain(fills.iter_mut()) {
        r.top -= reference;
    }
    for b in &mut bands {
        b.top -= reference;
        b.bottom -= reference;
        if let Some(bl) = &mut b.baseline {
            *bl -= reference;
        }
    }
    Geometry {
        placed,
        rules,
        fills,
        bands,
        width: box_width,
        height: reference,
        depth: total - reference,
    }
}

#[cfg(test)]
mod tests {
    //! Font-free geometry checks against kernel arithmetic (12pt body:
    //! `\baselineskip` 14.5pt, strut 10.15pt + 4.35pt).
    use super::*;

    fn span() -> Span {
        Span::new(0, 1)
    }

    fn col(align: Align, rule_before: bool, rule_after: bool) -> TableColumn {
        let mut before = Vec::new();
        if rule_before {
            before.push(TableMaterial::Rule(span()));
        }
        before.push(TableMaterial::Space(6.0));
        let mut after = vec![TableMaterial::Space(6.0)];
        if rule_after {
            after.push(TableMaterial::Rule(span()));
        }
        TableColumn { before, align, after, fill_after: false, color: None }
    }

    fn table(columns: Vec<TableColumn>, entries: Vec<TableEntry>) -> TableItem {
        TableItem {
            columns,
            entries,
            position: VerticalPosition::Top,
            width: None,
            arraystretch: 1.0,
            size_cpt: 0,
            lengths: TableLengths::default(),
            array_package: false,
            span: span(),
            rule_color: None,
            double_rule_sep_color: None,
            longtable: None,
        }
    }

    fn row(n: usize) -> TableEntry {
        TableEntry::Row {
            cells: (0..n).map(|_| TableCell { items: Vec::new(), columns: 1, template: None, alignment: None, color: None, multirow: None }).collect(),
            extra_depth_pt: 0.0,
            color: None,
            nobreak: false,
            kill: false,
        }
    }

    fn measured(t: &TableItem, widths: &[&[f64]]) -> Vec<Vec<MCell>> {
        let mut rows = Vec::new();
        let mut wi = 0;
        for e in &t.entries {
            let TableEntry::Row { cells, .. } = e else { continue };
            let ws = widths[wi];
            wi += 1;
            rows.push(
                row_slots(t, cells)
                    .into_iter()
                    .zip(ws)
                    .map(|((column, columns, tp), w)| {
                        let piece = |m: &TableMaterial| match m {
                            TableMaterial::Space(pt) => MPiece::Space(*pt),
                            TableMaterial::Rule(s) => MPiece::Rule(*s),
                            TableMaterial::VLine(s, w) => MPiece::VLine(*s, *w),
                            TableMaterial::Text(_) => MPiece::Text(Dims::default()),
                            TableMaterial::DoubleRuleGap(w) => MPiece::DoubleRuleGap(*w),
                        };
                        MCell {
                            column,
                            columns,
                            align: tp.map_or(Align::Left, |c| c.align),
                            before: tp.map_or(Vec::new(), |c| c.before.iter().map(piece).collect()),
                            content: Dims { width: *w, height: 8.0, depth: 2.0 },
                            after: tp.map_or(Vec::new(), |c| c.after.iter().map(piece).collect()),
                            content_offset: (0.0, 0.0),
                        }
                    })
                    .collect(),
            );
        }
        rows
    }

    fn metrics() -> Metrics {
        Metrics { strut_height: 10.15, strut_depth: 4.35, plain_strut_height: 10.15, em: 11.74988, ex: 5.16, axis: 3.0, measure: 469.75 }
    }

    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "expected {b}, got {a}");
    }

    #[test]
    fn rules_take_no_width_and_hlines_span_the_table() {
        let t = table(
            vec![col(Align::Left, true, true), col(Align::Center, false, true), col(Align::Right, false, true)],
            vec![TableEntry::HLine { span: span() }, row(3), TableEntry::HLine { span: span() }],
        );
        let g = layout(&t, &measured(&t, &[&[10.0, 20.0, 30.0]]), &metrics());
        close(g.width, 96.0);
        let h: Vec<_> = g.rules.iter().filter(|r| r.width > 1.0).collect();
        close(h[1].top - h[0].top, 0.4 + 14.5);
        let v: Vec<_> = g.rules.iter().filter(|r| r.width < 1.0).collect();
        assert_eq!(v.len(), 4);
        for (r, x) in v.iter().zip([0.0, 22.0, 54.0, 96.0]) {
            close(r.x, x - 0.2);
            close(r.height, 14.5);
        }
        // [t]: the first item is the rule.
        close(g.height, 0.4);
    }

    #[test]
    fn multicolumn_excess_goes_to_the_last_spanned_column() {
        let mut t = table(vec![col(Align::Left, false, false), col(Align::Left, false, false)], vec![row(1), row(2)]);
        if let TableEntry::Row { cells, .. } = &mut t.entries[0] {
            cells[0].columns = 2;
            cells[0].template = Some(col(Align::Center, false, false));
        }
        let g = layout(&t, &measured(&t, &[&[100.0], &[10.0, 5.0]]), &metrics());
        close(g.width, 112.0);
        let second = g.placed.iter().find(|p| p.row == 1 && p.cell == 1 && p.slot == Slot::Content).unwrap();
        close(second.x, 22.0 + 6.0);
    }

    #[test]
    fn double_hline_and_booktabs_spacing() {
        let t = table(
            vec![col(Align::Left, false, false)],
            vec![TableEntry::HLine { span: span() }, TableEntry::HLine { span: span() }, row(1)],
        );
        let g = layout(&t, &measured(&t, &[&[5.0]]), &metrics());
        close(g.rules[1].top - g.rules[0].top, 2.0);
        let m = metrics();
        let t = table(
            vec![col(Align::Left, false, false)],
            vec![
                TableEntry::BookRule { kind: BookRule::Top, width_pt: None, span: span() },
                row(1),
                TableEntry::BookRule { kind: BookRule::Mid, width_pt: None, span: span() },
                row(1),
                TableEntry::BookRule { kind: BookRule::Bottom, width_pt: None, span: span() },
            ],
        );
        let g = layout(&t, &measured(&t, &[&[5.0], &[5.0]]), &m);
        close(g.rules[0].height, 0.08 * m.em);
        let first = g.placed.iter().find(|p| p.row == 0).unwrap();
        close(first.baseline - g.rules[0].top, 0.08 * m.em + 0.65 * m.ex + 10.15);
        close(g.rules[1].top - first.baseline, 4.35 + 0.4 * m.ex);
        close(g.depth + g.height, g.rules[2].top + g.rules[2].height + g.height);
    }

    /// array.sty geometry against pdfLaTeX (tabular corpus 41-43, 12pt):
    /// a three-line `m{3cm}` entry makes the row 24.65pt high and 18.85pt
    /// deep with its first baseline 14.5pt above the row's; `b{3cm}` puts the
    /// last line on the baseline; `|` takes `\arrayrulewidth`.
    #[test]
    fn array_m_and_b_entries_and_width_taking_rules() {
        let m = metrics();
        let lines = ParLines { first_height: 8.0, inner: 29.0, last_depth: 2.0 };
        let (d, shift) = parbox(Align::Middle(Length::Pt(85.0)), lines, true, &m);
        close(d.height, 24.65);
        close(d.depth, 18.85);
        close(shift, -14.5);
        let (d, shift) = parbox(Align::Bottom(Length::Pt(85.0)), lines, true, &m);
        close(d.height, 39.15);
        close(d.depth, 4.35);
        close(shift, -29.0);
        // One line no taller than `\strutbox` stays on the baseline.
        let one = ParLines { first_height: 8.0, inner: 0.0, last_depth: 2.0 };
        let (d, shift) = parbox(Align::Middle(Length::Pt(85.0)), one, true, &m);
        close(d.height, 10.15);
        close(shift, 0.0);

        let mut left = col(Align::Left, false, false);
        left.before.insert(0, TableMaterial::VLine(span(), 0.4));
        left.after.push(TableMaterial::VLine(span(), 0.4));
        let mut t = table(vec![left], vec![row(1), TableEntry::HLine { span: span() }, TableEntry::HLine { span: span() }]);
        t.array_package = true;
        let g = layout(&t, &measured(&t, &[&[10.0]]), &m);
        close(g.width, 22.8);
        let v: Vec<_> = g.rules.iter().filter(|r| r.width < 1.0).collect();
        close(v[0].x, 0.0);
        close(v[1].x, 22.4);
        let h: Vec<_> = g.rules.iter().filter(|r| r.width > 1.0).collect();
        close(h[1].top - h[0].top, 0.4 + 2.0);
    }

    #[test]
    fn baselineskips_follow_the_size_files() {
        assert_eq!(baselineskip_pt(12, 1200), 14.5);
        assert_eq!(baselineskip_pt(12, 1095), 13.6);
        assert_eq!(baselineskip_pt(11, 1095), 13.6);
        assert_eq!(baselineskip_pt(10, 1000), 12.0);
    }
}
