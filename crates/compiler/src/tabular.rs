//! `tabular`/`tabular*` text tables, laid out with the LaTeX kernel's own
//! alignment model (`latex.ltx`, `\@array`/`\@mkpream`, no `array` package).
//!
//! The parser (`parser/tabular.rs`) turns the column specification into one
//! [`ColumnTemplate`] per column exactly the way `\@mkpream` builds its
//! `\halign` preamble: `\tabcolsep` glue on both sides of every `l`/`c`/`r`/
//! `p{}` entry unless an `@{}` expression replaces it, `|` rules that belong to
//! the column they follow (or the first column when leading), and
//! `\doublerulesep` between `||`. Layout then follows TeX's `\halign`:
//!
//! * column widths are the widest entry, spanned entries (`\multicolumn`) push
//!   any excess into the last column they span (TeX §801), and a column with no
//!   entries at all is zero-width with zero `\tabskip` after it;
//! * the kernel's `\@arrayrule` is `\hskip-.5\arrayrulewidth\vrule\hskip-.5
//!   \arrayrulewidth`, so a vertical rule takes no horizontal space and is
//!   centred on its boundary;
//! * every row carries `\@arstrut` (height `0.7\baselineskip`, depth
//!   `0.3\baselineskip`, both scaled by `\arraystretch`); rows stack with no
//!   interline glue; `\\[<dimen>]` deepens the row (positive) or adds
//!   `\noalign` space (non-positive);
//! * `\hline` is a full-width `\arrayrulewidth` rule taking vertical space, a
//!   second `\hline` directly after it is `\doublerulesep` below; `\cline` is
//!   drawn over the named columns and takes no net vertical space;
//! * booktabs `\toprule`/`\midrule`/`\bottomrule`/`\cmidrule` use booktabs'
//!   own widths and separations (`.08em`, `.05em`, `.03em`, `.4ex`, `.65ex`);
//! * the finished box is `\vcenter`ed on the math axis (`[c]`), or has the
//!   first (`[t]`, `\vtop`) or last (`[b]`, `\vbox`) item on the baseline;
//! * `tabular*` distributes the leftover width over `\extracolsep{\fill}`
//!   glue; the glue after the last column is always zero (`\tabskip\z@skip`
//!   precedes the preamble's `\cr`).
//!
//! The compiler's `\baselineskip` is `LINE_SPACING` times the font size (14.4pt
//! at 12pt, where LaTeX's `size12.clo` uses 14.5pt), so the strut follows that
//! same baseline model rather than a second one.

use crate::layout::{self, LayoutCursor};
use crate::math::{MathBox, MathItem, MathRule, FRACTION_RULE_CHAR, MATH_AXIS_EM};
use crate::parser::{Inline, TextStyle};
use crate::Span;

/// `\tabcolsep` (LaTeX kernel default).
pub const TABCOLSEP_PT: f64 = 6.0;
/// `\arrayrulewidth` (LaTeX kernel default).
pub const ARRAYRULEWIDTH_PT: f64 = 0.4;
/// `\doublerulesep` (LaTeX kernel default).
pub const DOUBLERULESEP_PT: f64 = 2.0;
/// booktabs `\heavyrulewidth`, `\lightrulewidth`, `\cmidrulewidth` (em).
const HEAVY_RULE_EM: f64 = 0.08;
const LIGHT_RULE_EM: f64 = 0.05;
const CMID_RULE_EM: f64 = 0.03;
/// booktabs `\aboverulesep`/`\belowrulesep` (ex) and `\cmidrulekern` (em).
const ABOVE_RULE_SEP_EX: f64 = 0.4;
const BELOW_RULE_SEP_EX: f64 = 0.65;
const CMID_RULE_KERN_EM: f64 = 0.5;
/// TeX's `\maxdimen`, the measure of an unbreakable `l`/`c`/`r` entry.
pub(crate) const MAX_DIMEN_PT: f64 = 16383.99;

#[derive(Debug, Clone, PartialEq)]
pub struct Tabular {
    pub columns: Vec<ColumnTemplate>,
    pub entries: Vec<Entry>,
    pub position: VerticalPosition,
    /// `tabular*`'s width argument; `None` for plain `tabular`.
    pub width: Option<Length>,
    /// `\arraystretch` in effect at `\begin`.
    pub arraystretch: f64,
    /// Text style (notably a size declaration) in effect at `\begin`.
    pub style: TextStyle,
    /// The preamble was built by the `array` package (array.sty v2.6n):
    /// `|` takes `\arrayrulewidth` of width, `p`/`m`/`b` entries start with
    /// the row strut, and `\hline\hline` is `\doublerulesep` apart.
    pub array_package: bool,
    /// `\begin{tabular}` through `\end{tabular}`.
    pub span: Span,
    /// See `Inline::Text::space_before`.
    pub space_before: bool,
    /// colortbl `\arrayrulecolor` in force at `\begin` (`None`: black).
    pub rule_color: Option<ColorSpec>,
    /// colortbl `\doublerulesepcolor` in force at `\begin` (`None`: the
    /// `\doublerulesep` gaps stay unpainted).
    pub double_rule_sep_color: Option<ColorSpec>,
    /// `longtable` (longtable.sty v4.24): the table is a block of its own
    /// that breaks across pages; `None` for `tabular`/`tabular*`.
    pub longtable: Option<Longtable>,
}

/// A colour as the document wrote it: `[model]{spec}`, e.g. `{gray!20}` or
/// `[rgb]{1,0,0}`. Resolving it to operator values is the colour model's job
/// (xcolor/color), not the table parser's.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorSpec {
    pub model: Option<String>,
    pub spec: String,
    pub span: Span,
}

/// colortbl `\columncolor`/`\rowcolor[model]{spec}[left][right]`: the fill
/// and its overhangs into the column separation (colortbl.sty 105-115 and
/// 220-229; `None` keeps the default, `\col@sep`).
#[derive(Debug, Clone, PartialEq)]
pub struct ColorFill {
    pub color: ColorSpec,
    pub left_pt: Option<f64>,
    pub right_pt: Option<f64>,
}

/// multirow.sty v2.9 `\multirow[vpos]{nrows}[bigstruts]{width}[vmove]{text}`.
#[derive(Debug, Clone, PartialEq)]
pub struct Multirow {
    /// `nrows`: may be negative (the entry spans rows above) or fractional.
    pub rows: f64,
    pub vpos: MultirowPos,
    /// The `bigstruts` argument (`\multirow@piii`): a leading `t`, then `b`,
    /// then the number of `\bigstrut`s.
    pub bigstrut_count: i32,
    pub bigstrut_top: bool,
    pub bigstrut_bottom: bool,
    pub width: MultirowWidth,
    /// `vmove`, a dimension added to the raise.
    pub vmove_pt: f64,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultirowPos {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MultirowWidth {
    /// `*`: the text in an `\hbox` at its natural width.
    Natural,
    /// `=`: a paragraph of the column's `\hsize`.
    Column,
    /// A paragraph of this width.
    Fixed(Length),
}

/// A dimension TeX evaluates in the font in force where it is used
/// (booktabs' `\cmidrule(l{.5em})`, `\addlinespace[1ex]`):
/// `pt + em * quad + ex * x-height`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontDimen {
    pub pt: f64,
    pub em: f64,
    pub ex: f64,
}

impl FontDimen {
    pub fn resolve(self, em: f64, ex: f64) -> f64 {
        self.pt + self.em * em + self.ex * ex
    }
}

/// `longtable`'s optional alignment `[l]`/`[c]`/`[r]` (longtable.sty
/// 120-126); `None` keeps `\LTleft`/`\LTright`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongtableAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Longtable {
    pub align: Option<LongtableAlign>,
    /// The `table` counter after `\LT@array`'s `\refstepcounter` (every
    /// longtable steps it, captioned or not).
    pub number: u32,
}

/// Which part of a longtable the rows before `\endfirsthead` etc. become.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongtableSection {
    FirstHead,
    Head,
    Foot,
    LastFoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalPosition {
    Top,
    Center,
    Bottom,
}

/// A width that may be relative to the text measure, which only layout knows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Pt(f64),
    /// A multiple of `\textwidth` (`\linewidth`/`\columnwidth` alike here).
    TextWidth(f64),
}

/// One column of the alignment preamble: `u` material, entry, `v` material.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnTemplate {
    pub before: Vec<Material>,
    pub align: Align,
    pub after: Vec<Material>,
    /// `\tabskip` glue after this column: `\extracolsep{\fill}` makes it fill.
    pub fill_after: bool,
    /// colortbl `>{\columncolor...}`.
    pub color: Option<ColorFill>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Material {
    /// `\tabcolsep`, `\doublerulesep` or the half rule width before `|@`.
    Space(f64),
    /// A kernel `|` (no width, centred on its boundary), at the source `|`.
    Rule(Span),
    /// A rule that takes its width: array.sty's `|` (`\vline`, width
    /// `\arrayrulewidth` when `width_pt` is `None`) or a `\vrule`/`\vline`
    /// in `!{}`/`@{}`. It runs the height and depth of the row.
    VLine { span: Span, width_pt: Option<f64> },
    /// `@{text}` material other than `\extracolsep`.
    Text(Vec<Inline>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Align {
    Left,
    Center,
    Right,
    /// `p{width}`: a top-aligned paragraph box of that width.
    Paragraph(Length),
    /// array's `m{width}`: the paragraph `\vbox` centred by `\ar@align@mcell`.
    Middle(Length),
    /// array's `b{width}`: the paragraph `\vbox`, last line on the baseline.
    Bottom(Length),
    /// array's `w{align}{width}`/`W`: the entry in `\makebox[width][align]`.
    Fixed(Length, BoxAlign),
}

impl Align {
    /// The paragraph width of a `p`/`m`/`b` entry.
    pub fn paragraph_width(self) -> Option<Length> {
        match self {
            Align::Paragraph(w) | Align::Middle(w) | Align::Bottom(w) => Some(w),
            _ => None,
        }
    }
}

/// `\makebox`'s position argument (`s` is set as `l`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookRule {
    Top,
    Mid,
    Bottom,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Row(Row),
    HLine {
        span: Span,
    },
    /// Zero-based, inclusive column range.
    CLine {
        first: usize,
        last: usize,
        span: Span,
    },
    BookRule {
        kind: BookRule,
        width_pt: Option<f64>,
        span: Span,
    },
    CMidRule {
        first: usize,
        last: usize,
        trim_left: bool,
        trim_right: bool,
        width_pt: Option<f64>,
        /// `(l{<dimen>})`/`(r{<dimen>})` (booktabs `\@setrulekerning`): the
        /// trim replacing `\cmidrulekern` on that side.
        kern_left: Option<FontDimen>,
        kern_right: Option<FontDimen>,
        span: Span,
    },
    /// `\\[<dimen>]` with a non-positive dimension: `\noalign{\vspace}`.
    VSpace {
        pt: f64,
    },
    /// booktabs `\addlinespace[<dimen>]` (`None`: `\defaultaddspace`, .5em).
    AddLineSpace {
        space: Option<FontDimen>,
        span: Span,
    },
    /// booktabs `\specialrule{width}{above}{below}`.
    SpecialRule {
        width_pt: f64,
        above_pt: f64,
        below_pt: f64,
        span: Span,
    },
    /// booktabs `\morecmidrules`: the next `\cmidrule` goes `\cmidrulesep`
    /// below the previous one instead of beside it.
    MoreCmidRules {
        span: Span,
    },
    /// colortbl `\arrayrulecolor` between rows: rules from here on.
    RuleColor {
        color: ColorSpec,
    },
    /// colortbl `\doublerulesepcolor` between rows.
    DoubleRuleSepColor {
        color: ColorSpec,
    },
    /// longtable `\endfirsthead`/`\endhead`/`\endfoot`/`\endlastfoot`: every
    /// entry since the previous section marker (or the start) belongs to it.
    Section {
        kind: LongtableSection,
        span: Span,
    },
    /// longtable `\caption[short]{text}` (`\LT@makecaption`): a row of its
    /// own, "Table n: text" in a `\LTcapwidth` parbox centred on the table.
    Caption {
        content: Vec<Inline>,
        /// `None` for `\caption*`.
        number: Option<u32>,
        span: Span,
    },
    /// longtable `\newpage`/`\pagebreak` between rows (`\noalign{\break}`).
    PageBreak {
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    /// `\\[<dimen>]` with a positive dimension: row depth is at least the
    /// strut depth plus this.
    pub extra_depth_pt: f64,
    /// colortbl `\rowcolor` at the start of the row.
    pub color: Option<ColorFill>,
    /// longtable `\\*`: no page break after this row.
    pub nobreak: bool,
    /// longtable `\kill`: the row sets column widths but is not typeset.
    pub kill: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub content: Vec<Inline>,
    /// Columns spanned (`\multicolumn`), at least 1.
    pub columns: usize,
    /// `\multicolumn`'s own template, replacing the column's.
    pub template: Option<ColumnTemplate>,
    /// `\centering`/`\raggedright`/`\raggedleft` in force at the end of a
    /// `p`/`m`/`b` entry (typically from `>{\centering\arraybackslash}`).
    pub alignment: Option<crate::parser::ParagraphStyle>,
    /// array `>{}`/`<{}` tokens were inserted around the entry, so its text
    /// style comes from them as well as from the entry's own source.
    pub declarations: bool,
    /// colortbl `\cellcolor[model]{spec}` anywhere in the entry.
    pub color: Option<ColorSpec>,
    /// The entry is a `\multirow`; `content` is its text.
    pub multirow: Option<Multirow>,
}

impl Tabular {
    /// Rebuilds the table with every source span mapped, or `None` when any
    /// span cannot be mapped (incremental reuse then re-lays the block out).
    pub fn try_map_spans(
        &self,
        span: &mut dyn FnMut(Span) -> Option<Span>,
        inlines: &mut dyn FnMut(&[Inline]) -> Option<Vec<Inline>>,
    ) -> Option<Tabular> {
        fn template(
            t: &ColumnTemplate,
            span: &mut dyn FnMut(Span) -> Option<Span>,
            inlines: &mut dyn FnMut(&[Inline]) -> Option<Vec<Inline>>,
        ) -> Option<ColumnTemplate> {
            let mut material = |m: &[Material]| -> Option<Vec<Material>> {
                m.iter()
                    .map(|m| {
                        Some(match m {
                            Material::Space(pt) => Material::Space(*pt),
                            Material::Rule(s) => Material::Rule(span(*s)?),
                            Material::VLine { span: s, width_pt } => Material::VLine {
                                span: span(*s)?,
                                width_pt: *width_pt,
                            },
                            Material::Text(content) => Material::Text(inlines(content)?),
                        })
                    })
                    .collect()
            };
            Some(ColumnTemplate {
                before: material(&t.before)?,
                align: t.align,
                after: material(&t.after)?,
                fill_after: t.fill_after,
                color: match &t.color {
                    Some(fill) => Some(ColorFill {
                        color: color_spec(&fill.color, span)?,
                        ..fill.clone()
                    }),
                    None => None,
                },
            })
        }
        fn color_spec(
            c: &ColorSpec,
            span: &mut dyn FnMut(Span) -> Option<Span>,
        ) -> Option<ColorSpec> {
            Some(ColorSpec {
                span: span(c.span)?,
                ..c.clone()
            })
        }
        let columns = self
            .columns
            .iter()
            .map(|t| template(t, span, inlines))
            .collect::<Option<Vec<_>>>()?;
        let mut entries = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            entries.push(match entry {
                Entry::Row(row) => Entry::Row(Row {
                    cells: row
                        .cells
                        .iter()
                        .map(|cell| {
                            Some(Cell {
                                content: inlines(&cell.content)?,
                                columns: cell.columns,
                                template: match &cell.template {
                                    Some(t) => Some(template(t, span, inlines)?),
                                    None => None,
                                },
                                alignment: cell.alignment,
                                declarations: cell.declarations,
                                color: match &cell.color {
                                    Some(c) => Some(color_spec(c, span)?),
                                    None => None,
                                },
                                multirow: match &cell.multirow {
                                    Some(m) => Some(Multirow {
                                        span: span(m.span)?,
                                        ..m.clone()
                                    }),
                                    None => None,
                                },
                            })
                        })
                        .collect::<Option<Vec<_>>>()?,
                    extra_depth_pt: row.extra_depth_pt,
                    color: match &row.color {
                        Some(fill) => Some(ColorFill {
                            color: color_spec(&fill.color, span)?,
                            ..fill.clone()
                        }),
                        None => None,
                    },
                    nobreak: row.nobreak,
                    kill: row.kill,
                }),
                Entry::HLine { span: s } => Entry::HLine { span: span(*s)? },
                Entry::CLine {
                    first,
                    last,
                    span: s,
                } => Entry::CLine {
                    first: *first,
                    last: *last,
                    span: span(*s)?,
                },
                Entry::BookRule {
                    kind,
                    width_pt,
                    span: s,
                } => Entry::BookRule {
                    kind: *kind,
                    width_pt: *width_pt,
                    span: span(*s)?,
                },
                Entry::CMidRule {
                    first,
                    last,
                    trim_left,
                    trim_right,
                    width_pt,
                    kern_left,
                    kern_right,
                    span: s,
                } => Entry::CMidRule {
                    first: *first,
                    last: *last,
                    trim_left: *trim_left,
                    trim_right: *trim_right,
                    width_pt: *width_pt,
                    kern_left: *kern_left,
                    kern_right: *kern_right,
                    span: span(*s)?,
                },
                Entry::VSpace { pt } => Entry::VSpace { pt: *pt },
                Entry::AddLineSpace { space, span: s } => Entry::AddLineSpace {
                    space: *space,
                    span: span(*s)?,
                },
                Entry::SpecialRule {
                    width_pt,
                    above_pt,
                    below_pt,
                    span: s,
                } => Entry::SpecialRule {
                    width_pt: *width_pt,
                    above_pt: *above_pt,
                    below_pt: *below_pt,
                    span: span(*s)?,
                },
                Entry::MoreCmidRules { span: s } => Entry::MoreCmidRules { span: span(*s)? },
                Entry::RuleColor { color } => Entry::RuleColor {
                    color: color_spec(color, span)?,
                },
                Entry::DoubleRuleSepColor { color } => Entry::DoubleRuleSepColor {
                    color: color_spec(color, span)?,
                },
                Entry::Section { kind, span: s } => Entry::Section {
                    kind: *kind,
                    span: span(*s)?,
                },
                Entry::Caption {
                    content,
                    number,
                    span: s,
                } => Entry::Caption {
                    content: inlines(content)?,
                    number: *number,
                    span: span(*s)?,
                },
                Entry::PageBreak { span: s } => Entry::PageBreak { span: span(*s)? },
            });
        }
        Some(Tabular {
            columns,
            entries,
            position: self.position,
            width: self.width,
            arraystretch: self.arraystretch,
            style: self.style,
            array_package: self.array_package,
            span: span(self.span)?,
            space_before: self.space_before,
            rule_color: match &self.rule_color {
                Some(c) => Some(color_spec(c, span)?),
                None => None,
            },
            double_rule_sep_color: match &self.double_rule_sep_color {
                Some(c) => Some(color_spec(c, span)?),
                None => None,
            },
            longtable: self.longtable.clone(),
        })
    }

    /// Every inline list in the table, for reference/label visitors.
    pub fn inline_lists(&self) -> Vec<&[Inline]> {
        let mut lists: Vec<&[Inline]> = Vec::new();
        for t in &self.columns {
            push_material_lists(t, &mut lists);
        }
        for entry in &self.entries {
            if let Entry::Row(row) = entry {
                for cell in &row.cells {
                    lists.push(&cell.content);
                    if let Some(t) = &cell.template {
                        push_material_lists(t, &mut lists);
                    }
                }
            }
            if let Entry::Caption { content, .. } = entry {
                lists.push(content);
            }
        }
        lists
    }
}

fn push_material_lists<'a>(t: &'a ColumnTemplate, lists: &mut Vec<&'a [Inline]>) {
    for m in t.before.iter().chain(&t.after) {
        if let Material::Text(content) = m {
            lists.push(content);
        }
    }
}

fn resolve(length: Length, measure: f64) -> f64 {
    match length {
        Length::Pt(pt) => pt,
        Length::TextWidth(factor) => factor * measure,
    }
}

/// A laid-out `u`/`v` piece of a template.
enum Piece {
    Space(f64),
    Rule(Span),
    VLine(Span, f64),
    Text(MathBox),
}

impl Piece {
    fn width(&self) -> f64 {
        match self {
            Piece::Space(pt) => *pt,
            Piece::Rule(_) => 0.0,
            Piece::VLine(_, width) => *width,
            Piece::Text(b) => b.width,
        }
    }
}

struct LaidCell {
    column: usize,
    columns: usize,
    align: Align,
    before: Vec<Piece>,
    content: MathBox,
    last_baseline: f64,
    after: Vec<Piece>,
}

impl LaidCell {
    fn natural(&self) -> f64 {
        self.before.iter().map(Piece::width).sum::<f64>()
            + self.content.width
            + self.after.iter().map(Piece::width).sum::<f64>()
    }
}

fn rule_item(x: f64, top: f64, width: f64, height: f64, size: f64, span: Span) -> MathItem {
    MathItem {
        font: Some(layout::Font::TimesRoman),
        text: FRACTION_RULE_CHAR.to_string(),
        x,
        baseline: top + height,
        size,
        span,
        rule: Some(MathRule {
            y: top,
            width,
            height,
        }),
    }
}

fn pieces(c: &mut LayoutCursor, material: &[Material], size: f64) -> Vec<Piece> {
    material
        .iter()
        .map(|m| match m {
            Material::Space(pt) => Piece::Space(*pt),
            Material::Rule(span) => Piece::Rule(*span),
            Material::VLine { span, width_pt } => {
                Piece::VLine(*span, width_pt.unwrap_or(ARRAYRULEWIDTH_PT))
            }
            Material::Text(content) => Piece::Text(c.inline_box(content, size, None).0),
        })
        .collect()
}

/// Lays a table out as one box whose origin is its reference baseline.
pub(crate) fn layout(c: &mut LayoutCursor, table: &Tabular, size: f64) -> MathBox {
    let measure = c.measure_pt();
    let body = c.body_size_pt();
    let n = table.columns.len().max(1);
    let baselineskip = layout::LINE_SPACING * size;
    let strut_height = table.arraystretch * 0.7 * baselineskip;
    let strut_depth = table.arraystretch * 0.3 * baselineskip;
    let fallback = ColumnTemplate {
        before: Vec::new(),
        align: Align::Left,
        after: Vec::new(),
        fill_after: false,
        color: None,
    };

    // Lay out every entry once; widths and heights come from these boxes.
    let mut laid_rows: Vec<Vec<LaidCell>> = Vec::new();
    for entry in &table.entries {
        let Entry::Row(row) = entry else {
            continue;
        };
        let mut cells = Vec::with_capacity(row.cells.len());
        let mut column = 0;
        for cell in &row.cells {
            if column >= n {
                break;
            }
            let template = cell
                .template
                .as_ref()
                .or_else(|| table.columns.get(column))
                .unwrap_or(&fallback);
            let columns = cell.columns.clamp(1, n - column);
            let before = pieces(c, &template.before, size);
            let measure_box = match template.align {
                Align::Paragraph(width) | Align::Middle(width) | Align::Bottom(width) => {
                    Some(resolve(width, measure).max(0.0))
                }
                _ => None,
            };
            let (mut content, last_baseline) = c.inline_box(&cell.content, size, measure_box);
            if let Some(width) = measure_box {
                content.width = width;
            }
            if let Align::Fixed(width, _) = template.align {
                content.width = resolve(width, measure).max(0.0);
            }
            let after = pieces(c, &template.after, size);
            cells.push(LaidCell {
                column,
                columns,
                align: template.align,
                before,
                content,
                last_baseline,
                after,
            });
            column += columns;
        }
        laid_rows.push(cells);
    }

    // Column widths, TeX §801: w[k][j] is the widest entry spanning k..=j.
    let mut w = vec![vec![f64::NEG_INFINITY; n]; n];
    for cell in laid_rows.iter().flatten() {
        let last = cell.column + cell.columns - 1;
        w[cell.column][last] = w[cell.column][last].max(cell.natural());
    }
    let mut widths = vec![0.0f64; n];
    let mut tabskip = vec![0.0f64; n];
    let mut fill = vec![false; n];
    for k in 0..n {
        let has_entries = w[k][k] > f64::NEG_INFINITY;
        widths[k] = if has_entries { w[k][k] } else { 0.0 };
        fill[k] = has_entries && k + 1 < n && table.columns.get(k).is_some_and(|t| t.fill_after);
        // Spans starting here carry what this column cannot hold onward.
        let (done, rest) = w.split_at_mut(k + 1);
        if let Some(next) = rest.first_mut() {
            for (j, spanned) in done[k].iter().enumerate().skip(k + 1) {
                if *spanned > f64::NEG_INFINITY {
                    next[j] = next[j].max(spanned - widths[k] - tabskip[k]);
                }
            }
        }
    }
    let natural_width: f64 = widths.iter().sum::<f64>() + tabskip.iter().sum::<f64>();
    let box_width = match table.width {
        Some(width) => {
            let target = resolve(width, measure);
            let fills = fill.iter().filter(|f| **f).count();
            if fills > 0 && target > natural_width {
                let share = (target - natural_width) / fills as f64;
                for (skip, _) in tabskip.iter_mut().zip(&fill).filter(|(_, f)| **f) {
                    *skip += share;
                }
            }
            target
        }
        None => natural_width,
    };
    let mut column_x = vec![0.0f64; n + 1];
    for k in 0..n {
        column_x[k + 1] = column_x[k] + widths[k] + tabskip[k];
    }
    let column_right = |k: usize| column_x[k] + widths[k];

    let em = body;
    let ex = layout::x_height_pt(layout::Font::TimesRoman, body);
    let mut items: Vec<MathItem> = Vec::new();
    // Vertical rules as (x, top, bottom, span), merged where rows abut.
    let mut vrules: Vec<(f64, f64, f64, f64, Span)> = Vec::new();
    let mut y = 0.0f64;
    let mut first_height = None;
    let mut last_depth = 0.0;
    let mut last_rule_class = 0u8;
    let mut rows = laid_rows.into_iter();
    for (index, entry) in table.entries.iter().enumerate() {
        let next = table.entries.get(index + 1);
        let next_is_booktabs =
            matches!(next, Some(Entry::BookRule { .. } | Entry::CMidRule { .. }));
        last_depth = 0.0;
        match entry {
            Entry::Row(row) => {
                let cells = rows.next().unwrap_or_default();
                if row.kill {
                    // longtable `\kill`: measured for the widths only.
                    continue;
                }
                let mut height = strut_height;
                let mut depth = strut_depth;
                if row.extra_depth_pt > 0.0 {
                    depth = depth.max(strut_depth + row.extra_depth_pt);
                }
                for cell in &cells {
                    height = height.max(cell.content.ascent);
                    depth = depth.max(cell.content.descent);
                    if cell.align.paragraph_width().is_some() {
                        depth = depth.max(cell.last_baseline + strut_depth);
                    }
                    for piece in cell.before.iter().chain(&cell.after) {
                        if let Piece::Text(b) = piece {
                            height = height.max(b.ascent);
                            depth = depth.max(b.descent);
                        }
                    }
                }
                first_height.get_or_insert(height);
                let top = y;
                let baseline = y + height;
                for cell in cells {
                    let left = column_x[cell.column];
                    let right = column_right(cell.column + cell.columns - 1);
                    let mut x = left;
                    let mut place_piece =
                        |piece: Piece, x: f64, items: &mut Vec<MathItem>| match piece {
                            Piece::Space(_) => {}
                            Piece::Rule(span) => vrules.push((
                                x - ARRAYRULEWIDTH_PT / 2.0,
                                ARRAYRULEWIDTH_PT,
                                top,
                                top + height + depth,
                                span,
                            )),
                            Piece::VLine(span, width) => {
                                vrules.push((x, width, top, top + height + depth, span))
                            }
                            Piece::Text(b) => push_box(items, b, x, baseline),
                        };
                    for piece in cell.before {
                        let width = piece.width();
                        place_piece(piece, x, &mut items);
                        x += width;
                    }
                    let after_width: f64 = cell.after.iter().map(Piece::width).sum();
                    let content_x = match cell.align {
                        Align::Left
                        | Align::Paragraph(_)
                        | Align::Middle(_)
                        | Align::Bottom(_)
                        | Align::Fixed(..) => x,
                        Align::Right => right - after_width - cell.content.width,
                        Align::Center => x + (right - after_width - x - cell.content.width) / 2.0,
                    };
                    push_box(&mut items, cell.content, content_x, baseline);
                    let mut x = right - after_width;
                    for piece in cell.after {
                        let width = piece.width();
                        place_piece(piece, x, &mut items);
                        x += width;
                    }
                }
                y += height + depth;
                last_depth = depth;
            }
            Entry::HLine { span } => {
                first_height.get_or_insert(ARRAYRULEWIDTH_PT);
                push_rule(
                    &mut items,
                    0.0,
                    y,
                    box_width,
                    ARRAYRULEWIDTH_PT,
                    size,
                    *span,
                );
                y += ARRAYRULEWIDTH_PT;
                if matches!(next, Some(Entry::HLine { .. })) {
                    y += DOUBLERULESEP_PT;
                    if !table.array_package {
                        y -= ARRAYRULEWIDTH_PT;
                    }
                }
            }
            Entry::CLine { first, last, span } => {
                first_height.get_or_insert(ARRAYRULEWIDTH_PT);
                let (first, last) = (*first.min(&(n - 1)), *last.min(&(n - 1)));
                let x = column_x[first];
                push_rule(
                    &mut items,
                    x,
                    y,
                    column_right(last) - x,
                    ARRAYRULEWIDTH_PT,
                    size,
                    *span,
                );
            }
            Entry::VSpace { pt } => {
                first_height.get_or_insert(0.0);
                y += pt;
            }
            Entry::BookRule {
                kind,
                width_pt,
                span,
            } => {
                let width = width_pt.unwrap_or(match kind {
                    BookRule::Mid => LIGHT_RULE_EM * em,
                    BookRule::Top | BookRule::Bottom => HEAVY_RULE_EM * em,
                });
                let above = match kind {
                    BookRule::Top => 0.0,
                    BookRule::Mid | BookRule::Bottom => ABOVE_RULE_SEP_EX * ex,
                };
                // `\vskip` glue always precedes the rule, so a `[t]` table
                // starting with a booktabs rule has zero height.
                first_height.get_or_insert(0.0);
                y += if last_rule_class == 0 {
                    above
                } else {
                    DOUBLERULESEP_PT
                };
                push_rule(&mut items, 0.0, y, box_width, width, size, *span);
                y += width;
                if next_is_booktabs {
                    last_rule_class = 1;
                } else {
                    last_rule_class = 0;
                    if *kind != BookRule::Bottom {
                        y += BELOW_RULE_SEP_EX * ex;
                    }
                }
            }
            Entry::CMidRule {
                first,
                last,
                trim_left,
                trim_right,
                width_pt,
                kern_left,
                kern_right,
                span,
            } => {
                let width = width_pt.unwrap_or(CMID_RULE_EM * em);
                if last_rule_class == 0 {
                    first_height.get_or_insert(0.0);
                    y += ABOVE_RULE_SEP_EX * ex;
                } else {
                    first_height.get_or_insert(width);
                }
                let (first, last) = (*first.min(&(n - 1)), *last.min(&(n - 1)));
                let kern = CMID_RULE_KERN_EM * em;
                let left = column_x[first]
                    + if *trim_left {
                        kern_left.map_or(kern, |d| d.resolve(em, ex))
                    } else {
                        0.0
                    };
                let right = column_right(last)
                    - if *trim_right {
                        kern_right.map_or(kern, |d| d.resolve(em, ex))
                    } else {
                        0.0
                    };
                push_rule(&mut items, left, y, right - left, width, size, *span);
                y += width;
                if matches!(next, Some(Entry::CMidRule { .. })) {
                    y -= width;
                    last_rule_class = 1;
                } else {
                    y += BELOW_RULE_SEP_EX * ex;
                    last_rule_class = 0;
                }
            }
            Entry::AddLineSpace { space, .. } => {
                first_height.get_or_insert(0.0);
                y += space.map_or(0.5 * em, |d| d.resolve(em, ex));
                last_rule_class = 2;
            }
            Entry::SpecialRule {
                width_pt,
                above_pt,
                below_pt,
                span,
            } => {
                first_height.get_or_insert(0.0);
                y += above_pt;
                push_rule(&mut items, 0.0, y, box_width, *width_pt, size, *span);
                y += width_pt + below_pt;
                last_rule_class = 2;
            }
            // The compiler's own layout paints no colour and does not break
            // pages; the render pipeline does both.
            Entry::MoreCmidRules { .. }
            | Entry::RuleColor { .. }
            | Entry::DoubleRuleSepColor { .. }
            | Entry::Section { .. }
            | Entry::PageBreak { .. } => {}
            Entry::Caption { content, .. } => {
                let (b, _) = c.inline_box(content, size, None);
                let height = b.ascent.max(strut_height);
                first_height.get_or_insert(height);
                let x = ((box_width - b.width) / 2.0).max(0.0);
                push_box(&mut items, b, x, y + height);
                y += height + strut_depth + baselineskip;
            }
        }
    }

    // Merge vertical rule segments that continue one another.
    vrules.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.total_cmp(&b.2)));
    let mut merged: Vec<(f64, f64, f64, f64, Span)> = Vec::new();
    for rule in vrules {
        match merged.last_mut() {
            Some(last)
                if last.0 == rule.0
                    && last.1 == rule.1
                    && last.4 == rule.4
                    && (last.3 - rule.2).abs() < 1e-9 =>
            {
                last.3 = rule.3;
            }
            _ => merged.push(rule),
        }
    }
    for (x, width, top, bottom, span) in merged {
        push_rule(&mut items, x, top, width, bottom - top, size, span);
    }

    let total = y;
    let reference = match table.position {
        VerticalPosition::Top => first_height.unwrap_or(0.0),
        VerticalPosition::Bottom => total - last_depth,
        VerticalPosition::Center => total / 2.0 + MATH_AXIS_EM * size,
    };
    for item in &mut items {
        item.baseline -= reference;
        if let Some(rule) = &mut item.rule {
            rule.y -= reference;
        }
    }
    MathBox {
        items,
        width: box_width,
        ascent: reference,
        descent: total - reference,
    }
}

/// Rules must have positive extent to be valid rules-v1 geometry.
fn push_rule(
    items: &mut Vec<MathItem>,
    x: f64,
    top: f64,
    width: f64,
    height: f64,
    size: f64,
    span: Span,
) {
    if width > 0.0 && height > 0.0 {
        items.push(rule_item(x, top, width, height, size, span));
    }
}

fn push_box(items: &mut Vec<MathItem>, b: MathBox, x: f64, baseline: f64) {
    items.extend(b.items.into_iter().map(|mut item| {
        item.x += x;
        item.baseline += baseline;
        if let Some(rule) = &mut item.rule {
            rule.y += baseline;
        }
        item
    }));
}

#[cfg(test)]
mod tests {
    //! Expected geometry is derived from the kernel definitions quoted in the
    //! module comment; each formula was checked against pdflatex box
    //! dimensions with fixed-width `\hbox to` entries (at 10pt: `|l|c|r|` with
    //! 10/20/30pt entries is 96pt wide, `\multicolumn{2}{c}` of 100pt over
    //! 10pt+20pt columns is 112pt, a booktabs top/mid/bottom table of two rows
    //! is 35.14pt tall). Here the body is 12pt: `\baselineskip` 14.4pt, strut
    //! 10.08pt + 4.32pt, math axis 3pt.
    use crate::diagnostics::Diagnostic;
    use crate::layout::{self, TextItem};
    use crate::parser::parse;

    fn laid_out(source: &str) -> (Vec<TextItem>, Vec<Diagnostic>) {
        let parsed = parse(source);
        let pages = layout::layout(&parsed.blocks);
        let items = pages.into_iter().flat_map(|page| page.items).collect();
        (items, parsed.diagnostics)
    }

    /// (x, y, width, height) of every rule, in emission order.
    fn rules(items: &[TextItem]) -> Vec<(f64, f64, f64, f64)> {
        items
            .iter()
            .filter_map(|item| {
                item.rule
                    .map(|rule| (item.x_pt, rule.y_pt, rule.width_pt, rule.height_pt))
            })
            .collect()
    }

    fn text<'a>(items: &'a [TextItem], text: &str) -> &'a TextItem {
        items
            .iter()
            .find(|item| item.rule.is_none() && item.text == text)
            .unwrap_or_else(|| panic!("no item {text:?}"))
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 0.011,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn kernel_vertical_rules_take_no_width_and_hlines_span_the_table() {
        let (items, diagnostics) = laid_out(
            "\\begin{tabular}{|l|c|r|}\\hline \\hspace{10pt}&\\hspace{20pt}&\\hspace{30pt}\\\\\\hline\\end{tabular}",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let rules = rules(&items);
        let (x0, y0, w0, h0) = rules[0];
        close(w0, 96.0);
        close(h0, 0.4);
        close(rules[1].0, x0);
        close(rules[1].1, y0 + 0.4 + 14.4);
        close(rules[1].2, 96.0);
        let verticals: Vec<_> = rules[2..].to_vec();
        assert_eq!(verticals.len(), 4);
        for ((x, y, w, h), boundary) in verticals.into_iter().zip([0.0, 22.0, 54.0, 96.0]) {
            close(x, x0 + boundary - 0.2);
            close(y, y0 + 0.4);
            close(w, 0.4);
            close(h, 14.4);
        }
    }

    #[test]
    fn multicolumn_excess_widens_the_last_spanned_column() {
        let (items, diagnostics) = laid_out(
            "\\begin{tabular}{ll}\\multicolumn{2}{c}{\\hspace{100pt}}\\\\ \\hspace{10pt}&X\\\\\\hline\\end{tabular}",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let (x0, _, width, _) = rules(&items)[0];
        close(width, 112.0);
        // Column 0 is 22pt; column 1 absorbs the excess and X sits after its
        // \tabcolsep.
        close(text(&items, "X").x_pt, x0 + 22.0 + 6.0);
    }

    #[test]
    fn position_argument_selects_the_reference_baseline() {
        for (position, row, offset) in [("[t]", "A", 0.0), ("[b]", "B", 0.0)] {
            let (items, _) = laid_out(&format!(
                "Base \\begin{{tabular}}{position}{{l}}A\\\\B\\end{{tabular}}"
            ));
            close(
                text(&items, row).baseline_y_pt,
                text(&items, "Base").baseline_y_pt + offset,
            );
        }
        // [c]: 28.8pt tall, centred on the 3pt math axis.
        let (items, _) = laid_out("Base \\begin{tabular}{l}A\\\\B\\end{tabular}");
        let base = text(&items, "Base").baseline_y_pt;
        close(text(&items, "A").baseline_y_pt, base - (14.4 + 3.0) + 10.08);
        close(text(&items, "B").baseline_y_pt, base - (14.4 + 3.0) + 24.48);
    }

    #[test]
    fn row_spacing_follows_struts_arraystretch_and_row_arguments() {
        let (items, _) = laid_out(
            "\\begin{tabular}[t]{l}A\\\\[5pt]B\\\\[-2pt]C\\\\\\hline\\hline\\end{tabular}",
        );
        let a = text(&items, "A").baseline_y_pt;
        let b = text(&items, "B").baseline_y_pt;
        let c = text(&items, "C").baseline_y_pt;
        close(b - a, 4.32 + 5.0 + 10.08);
        close(c - b, 14.4 - 2.0);
        let rules = rules(&items);
        close(rules[0].1, c + 4.32);
        close(rules[1].1 - rules[0].1, 2.0);

        let (items, diagnostics) = laid_out(
            "\\renewcommand{\\arraystretch}{1.5}\\begin{tabular}[t]{l}A\\\\B\\end{tabular}",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        close(
            text(&items, "B").baseline_y_pt - text(&items, "A").baseline_y_pt,
            1.5 * 14.4,
        );
    }

    #[test]
    fn cline_takes_no_vertical_space_and_covers_its_columns() {
        let (items, _) = laid_out(
            "\\begin{tabular}[t]{ll}\\hspace{10pt}&A\\\\\\cline{2-2}\\hspace{10pt}&B\\end{tabular}",
        );
        let a = text(&items, "A");
        let b = text(&items, "B");
        close(b.baseline_y_pt - a.baseline_y_pt, 14.4);
        let (x, y, _, h) = rules(&items)[0];
        close(x, a.x_pt - 6.0);
        close(y, a.baseline_y_pt + 4.32);
        close(h, 0.4);
    }

    #[test]
    fn tabular_star_fill_pushes_later_columns_to_the_requested_width() {
        let (items, diagnostics) = laid_out(
            "\\begin{tabular*}{200pt}{@{\\extracolsep{\\fill}}ll|}\\hline \\hspace{10pt}&X\\end{tabular*}",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let rules = rules(&items);
        let (x0, _, width, _) = rules[0];
        close(width, 200.0);
        close(rules[1].0, x0 + 200.0 - 0.2);
    }

    #[test]
    fn paragraph_column_wraps_at_its_width_and_the_row_grows() {
        let (items, diagnostics) =
            laid_out("\\begin{tabular}[t]{p{40pt}|l}one two three four five six & X\\end{tabular}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let one = text(&items, "one");
        let six = text(&items, "six");
        assert!(six.baseline_y_pt > one.baseline_y_pt, "p entry wrapped");
        close(text(&items, "X").baseline_y_pt, one.baseline_y_pt);
        let (x, y, _, h) = rules(&items)[0];
        close(x, one.x_pt + 40.0 + 6.0 - 0.2);
        close(y, one.baseline_y_pt - 10.08);
        close(y + h, six.baseline_y_pt + 4.32);
    }

    #[test]
    fn booktabs_rules_use_booktabs_widths_and_separations() {
        let (items, _) = laid_out(
            "\\documentclass{article}\\usepackage{booktabs}\\begin{document}\\begin{tabular}[t]{l}\\toprule A\\\\\\midrule B\\\\\\bottomrule\\end{tabular}\\end{document}",
        );
        let ex = layout::x_height_pt(layout::Font::TimesRoman, 12.0);
        let rules = rules(&items);
        let a = text(&items, "A").baseline_y_pt;
        let b = text(&items, "B").baseline_y_pt;
        close(rules[0].3, 0.96);
        close(a, rules[0].1 + 0.96 + 0.65 * ex + 10.08);
        close(rules[1].3, 0.6);
        close(rules[1].1, a + 4.32 + 0.4 * ex);
        close(b, rules[1].1 + 0.6 + 0.65 * ex + 10.08);
        close(rules[2].1, b + 4.32 + 0.4 * ex);
        close(rules[2].3, 0.96);
    }

    #[test]
    fn items_keep_exact_source_spans() {
        let source =
            "\\begin{tabular}{l|r}\\hline Alpha & \\multicolumn{1}{c}{Beta}\\\\\\end{tabular}";
        let (items, _) = laid_out(source);
        for item in &items {
            let slice = &source[item.span.start..item.span.end];
            match item.text.as_str() {
                "Alpha" | "Beta" => assert_eq!(slice, item.text),
                _ if item.rule.is_some() && item.rule.unwrap().width_pt < 1.0 => {
                    assert_eq!(slice, "|")
                }
                _ if item.rule.is_some() => assert_eq!(slice, "\\hline"),
                other => panic!("unexpected item {other:?}"),
            }
        }
    }

    #[test]
    fn malformed_tables_are_diagnosed_and_recovered() {
        let (items, diagnostics) = laid_out("\\begin{tabular}{ll}A & B & C\\\\\\end{tabular}");
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("extra alignment tab")));
        // TeX turns the extra `&` into `\cr`: C starts the next row.
        let a = text(&items, "A");
        let c = text(&items, "C");
        close(c.x_pt, a.x_pt);
        close(c.baseline_y_pt - a.baseline_y_pt, 14.4);

        let (_, diagnostics) = laid_out("\\begin{tabular}{m{1cm}l}A & B\\end{tabular}");
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("need the array package")));

        let (_, diagnostics) = laid_out("\\begin{tabular}{ll}A & B");
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("unterminated environment 'tabular'")));

        let (_, diagnostics) = laid_out("\\begin{tabular}{ll}A\\\\\\cline{1-3}\\end{tabular}");
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("\\cline{1-3}")));
    }

    fn array_table(preamble: &str, table: &str) -> (super::Tabular, Vec<Diagnostic>) {
        let parsed = parse(&format!(
            "\\documentclass{{article}}\\usepackage{{array}}{preamble}\\begin{{document}}{table}\\end{{document}}"
        ));
        let table = parsed
            .blocks
            .iter()
            .find_map(|block| match block {
                crate::parser::Block::Paragraph(content) => {
                    content.iter().find_map(|inline| match inline {
                        crate::parser::Inline::Tabular(t) => Some((**t).clone()),
                        _ => None,
                    })
                }
                _ => None,
            })
            .expect("a table");
        (table, parsed.diagnostics)
    }

    fn spaces(material: &[super::Material]) -> Vec<String> {
        material
            .iter()
            .map(|m| match m {
                super::Material::Space(pt) => format!("{pt}"),
                super::Material::Rule(_) => "rule".into(),
                super::Material::VLine { width_pt, .. } => format!("vline{width_pt:?}"),
                super::Material::Text(_) => "text".into(),
            })
            .collect()
    }

    /// array.sty `\@mkpream`: `|` is a `\vline`, `||` has `\doublerulesep`
    /// between, `!{}` keeps the `\tabcolsep`s around it, `>{}`/`<{}` tokens
    /// wrap every entry of their column.
    #[test]
    fn array_preamble_builds_width_taking_rules_and_declarations() {
        use super::{Align, BoxAlign, Length};
        let (t, diagnostics) = array_table(
            "",
            "\\begin{tabular}{||>{\\bfseries}l<{:}!{\\vrule width 1pt}m{2cm}|w{r}{1cm}}A & B & C\\end{tabular}",
        );
        assert!(t.array_package);
        assert!(
            diagnostics
                .iter()
                .all(|d| !d.message.contains("tabular") && !d.message.contains("column")),
            "{diagnostics:?}"
        );
        assert_eq!(t.columns.len(), 3);
        assert_eq!(
            spaces(&t.columns[0].before),
            ["vlineNone", "2", "vlineNone", "6"]
        );
        assert_eq!(spaces(&t.columns[0].after), ["6", "vlineSome(1.0)"]);
        assert_eq!(spaces(&t.columns[1].before), ["6"]);
        assert_eq!(spaces(&t.columns[1].after), ["6", "vlineNone"]);
        assert_eq!(spaces(&t.columns[2].before), ["6"]);
        assert_eq!(spaces(&t.columns[2].after), ["6"]);
        assert!(
            matches!(t.columns[1].align, Align::Middle(Length::Pt(w)) if (w - 56.905).abs() < 0.01)
        );
        assert!(matches!(
            t.columns[2].align,
            Align::Fixed(Length::Pt(_), BoxAlign::Right)
        ));
        let super::Entry::Row(row) = &t.entries[0] else {
            panic!("a row")
        };
        assert!(row.cells[0].declarations);
        let text: Vec<_> = row.cells[0]
            .content
            .iter()
            .filter_map(|inline| match inline {
                crate::parser::Inline::Text { text, style, .. } => {
                    Some((text.as_str(), style.bold))
                }
                _ => None,
            })
            .collect();
        assert_eq!(text, [("A", true), (":", true)]);
        assert!(!row.cells[1].declarations);
    }

    #[test]
    fn newcolumntype_expands_with_arguments_and_centering_reaches_the_entry() {
        use super::Align;
        let (t, diagnostics) = array_table(
            "\\newcolumntype{C}[1]{>{\\centering\\arraybackslash}p{#1}}",
            "\\begin{tabular}{C{2cm}l}Some words & x\\end{tabular}",
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| !d.message.contains("column") && !d.message.contains("arraybackslash")),
            "{diagnostics:?}"
        );
        assert_eq!(t.columns.len(), 2);
        assert!(matches!(t.columns[0].align, Align::Paragraph(_)));
        let super::Entry::Row(row) = &t.entries[0] else {
            panic!("a row")
        };
        assert_eq!(
            row.cells[0].alignment,
            Some(crate::parser::ParagraphStyle::Center)
        );
        assert_eq!(row.cells[1].alignment, None);
    }

    #[test]
    fn array_misplaced_less_than_becomes_a_bang_expression() {
        let (t, diagnostics) = array_table("", "\\begin{tabular}{<{x}l}A\\end{tabular}");
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("changed to !{..}")));
        assert_eq!(spaces(&t.columns[0].before), ["text", "6"]);
    }

    #[test]
    fn escaped_ampersand_is_text_not_an_alignment_tab() {
        let (items, diagnostics) = laid_out("\\begin{tabular}{l}R\\&D\\end{tabular}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert!(items.iter().any(|item| item.text == "&"));
    }

    fn tables(packages: &str, body: &str) -> (Vec<super::Tabular>, Vec<Diagnostic>) {
        let parsed = parse(&format!(
            "\\documentclass{{article}}\\usepackage{packages}\\begin{{document}}{body}\\end{{document}}"
        ));
        let tables = parsed
            .blocks
            .iter()
            .flat_map(|block| match block {
                crate::parser::Block::Paragraph(content) => content
                    .iter()
                    .filter_map(|inline| match inline {
                        crate::parser::Inline::Tabular(t) => Some((**t).clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect();
        (tables, parsed.diagnostics)
    }

    fn texts(content: &[crate::parser::Inline]) -> String {
        content
            .iter()
            .filter_map(|inline| match inline {
                crate::parser::Inline::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn rows(t: &super::Tabular) -> Vec<&super::Row> {
        t.entries
            .iter()
            .filter_map(|e| match e {
                super::Entry::Row(r) => Some(r),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn booktabs_trims_addlinespace_specialrule_and_morecmidrules() {
        use super::Entry;
        let (t, diagnostics) = tables(
            "{booktabs}",
            "\\begin{tabular}{ll}a&b\\\\\\cmidrule(l{2pt}r){1-2}\\morecmidrules\\cmidrule(lr){1-1}\\addlinespace\\addlinespace[3pt]\\specialrule{1pt}{2pt}{3pt}c&d\\end{tabular}",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let e = &t[0].entries;
        assert!(
            matches!(e[1], Entry::CMidRule { first: 0, last: 1, trim_left: true, trim_right: true, kern_left: Some(l), kern_right: None, .. } if (l.pt - 2.0).abs() < 1e-9)
        );
        assert!(matches!(e[2], Entry::MoreCmidRules { .. }));
        assert!(matches!(
            e[3],
            Entry::CMidRule {
                kern_left: None,
                kern_right: None,
                trim_left: true,
                trim_right: true,
                ..
            }
        ));
        assert!(matches!(e[4], Entry::AddLineSpace { space: None, .. }));
        assert!(
            matches!(e[5], Entry::AddLineSpace { space: Some(p), .. } if (p.pt - 3.0).abs() < 1e-9)
        );
        assert!(
            matches!(e[6], Entry::SpecialRule { width_pt, above_pt, below_pt, .. } if width_pt == 1.0 && above_pt == 2.0 && below_pt == 3.0)
        );
        assert!(matches!(e[7], Entry::Row(_)));
    }

    #[test]
    fn multirow_arguments_are_parsed_and_the_text_is_the_entry() {
        use super::{Length, MultirowPos, MultirowWidth};
        let (t, diagnostics) = tables(
            "{multirow}",
            "\\begin{tabular}{ll}\\multirow{2}{*}{Alpha} & x\\\\ & y\\\\\\multirow[t]{-2}[tb3]{2cm}[1pt]{Beta} & z\\\\\\multicolumn{2}{c}{\\multirow{3}{=}{Gamma}}\\end{tabular}",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let rows = rows(&t[0]);
        let a = &rows[0].cells[0];
        let m = a.multirow.as_ref().expect("multirow");
        assert_eq!(
            (m.rows, m.vpos, m.width),
            (2.0, MultirowPos::Center, MultirowWidth::Natural)
        );
        assert_eq!(texts(&a.content), "Alpha");
        let b = rows[2].cells[0].multirow.as_ref().expect("multirow");
        assert_eq!(
            (
                b.rows,
                b.vpos,
                b.bigstrut_top,
                b.bigstrut_bottom,
                b.bigstrut_count
            ),
            (-2.0, MultirowPos::Top, true, true, 3)
        );
        assert!(
            matches!(b.width, MultirowWidth::Fixed(Length::Pt(w)) if (w - 56.905).abs() < 0.01)
        );
        assert_eq!(b.vmove_pt, 1.0);
        let g = &rows[3].cells[0];
        assert_eq!(g.columns, 2);
        assert_eq!(
            g.multirow.as_ref().map(|m| m.width),
            Some(MultirowWidth::Column)
        );
        assert_eq!(texts(&g.content), "Gamma");
    }

    #[test]
    fn colortbl_row_cell_column_and_rule_colours() {
        use super::Entry;
        let (t, diagnostics) = tables(
            "{colortbl}\\arrayrulecolor{blue}",
            "\\begin{tabular}{>{\\columncolor[gray]{.9}[2pt]}l|c}\\rowcolor{red!20}[1pt][3pt]a&\\cellcolor[HTML]{00FF00}b\\\\\\arrayrulecolor{green}\\hline c&d\\end{tabular}",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let t = &t[0];
        assert_eq!(t.rule_color.as_ref().map(|c| c.spec.as_str()), Some("blue"));
        let fill = t.columns[0].color.as_ref().expect("column colour");
        assert_eq!(
            (fill.color.model.as_deref(), fill.color.spec.as_str()),
            (Some("gray"), ".9")
        );
        assert_eq!((fill.left_pt, fill.right_pt), (Some(2.0), Some(2.0)));
        assert!(t.columns[1].color.is_none());
        let rows = rows(t);
        let row = rows[0].color.as_ref().expect("row colour");
        assert_eq!(
            (row.color.spec.as_str(), row.left_pt, row.right_pt),
            ("red!20", Some(1.0), Some(3.0))
        );
        assert!(rows[1].color.is_none());
        assert_eq!(texts(&rows[0].cells[0].content), "a");
        let cell = rows[0].cells[1].color.as_ref().expect("cell colour");
        assert_eq!(
            (cell.model.as_deref(), cell.spec.as_str()),
            (Some("HTML"), "00FF00")
        );
        assert_eq!(texts(&rows[0].cells[1].content), "b");
        assert!(t
            .entries
            .iter()
            .any(|e| matches!(e, Entry::RuleColor { color } if color.spec == "green")));

        let (t, diagnostics) = tables(
            "[table]{xcolor}",
            "\\begin{tabular}{l}\\rowcolor{gray}a\\end{tabular}",
        );
        assert!(
            diagnostics.iter().all(|d| !d.message.contains("rowcolor")),
            "{diagnostics:?}"
        );
        assert!(self::rows(&t[0])[0].color.is_some());
    }

    #[test]
    fn longtable_sections_captions_kill_and_breaks() {
        use super::{Entry, LongtableAlign, LongtableSection};
        let (t, diagnostics) = tables(
            "{longtable}",
            "Before.\\begin{longtable}[l]{ll}\\caption{First}\\label{t:a}\\\\ H&I\\\\\\endfirsthead\\caption*{Again}\\\\\\endhead F&G\\\\\\endfoot\\endlastfoot wide&x\\kill a&b\\\\* \\newpage c&d\\\\\\end{longtable}\\begin{longtable}{l}x\\end{longtable}",
        );
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(t.len(), 2);
        let lt = t[0].longtable.as_ref().expect("longtable");
        assert_eq!((lt.align, lt.number), (Some(LongtableAlign::Left), 1));
        assert_eq!(t[1].longtable.as_ref().map(|l| l.number), Some(2));
        let kinds: Vec<&str> = t[0]
            .entries
            .iter()
            .map(|e| match e {
                Entry::Caption {
                    number: Some(_), ..
                } => "caption",
                Entry::Caption { number: None, .. } => "caption*",
                Entry::Section {
                    kind: LongtableSection::FirstHead,
                    ..
                } => "firsthead",
                Entry::Section {
                    kind: LongtableSection::Head,
                    ..
                } => "head",
                Entry::Section {
                    kind: LongtableSection::Foot,
                    ..
                } => "foot",
                Entry::Section {
                    kind: LongtableSection::LastFoot,
                    ..
                } => "lastfoot",
                Entry::Row(r) if r.kill => "kill",
                Entry::Row(r) if r.nobreak => "row*",
                Entry::Row(_) => "row",
                Entry::PageBreak { .. } => "break",
                _ => "other",
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "caption",
                "row",
                "firsthead",
                "caption*",
                "head",
                "row",
                "foot",
                "lastfoot",
                "kill",
                "row*",
                "break",
                "row"
            ]
        );
        let Entry::Caption { content, .. } = &t[0].entries[0] else {
            panic!()
        };
        assert!(content
            .iter()
            .any(|i| matches!(i, crate::parser::Inline::Label { .. })));
    }
}
