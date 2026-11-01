//! Explicit box output and the flattener that turns it into positioned glyph
//! and rule runs.
//!
//! Coordinates inside a box follow TeX: the reference point is the left end of
//! the baseline, `height` extends above it and `depth` below it. Children of a
//! container carry an explicit offset `(dx, dy)` from the parent's reference
//! point, with `dy` positive **downwards** (TeX's `shift_amount` sign), so a
//! superscript has negative `dy` and a subscript positive `dy`. That makes the
//! flattening step a pure translation with no per-kind rules.

use crate::metrics::FontId;
use crate::source::SourceTag;

#[derive(Debug, Clone, PartialEq)]
pub enum BoxKind {
    /// One glyph drawn from `font_id` at `size` pt.
    Glyph {
        font_id: FontId,
        gid: u16,
        ch: char,
        size: f64,
    },
    /// A filled rectangle occupying the box's width/height/depth.
    Rule,
    /// Children laid out left to right; each child's `dx` is already absolute
    /// within the box.
    HBox(Vec<Child>),
    /// Children stacked vertically; each child's `dy` is absolute within the box.
    VBox(Vec<Child>),
    /// Horizontal space with no ink (italic corrections, script space, …).
    Kern,
    /// Glue: inter-atom spacing (`\thinmuskip`/`\medmuskip`/`\thickmuskip`)
    /// or explicit `\mskip`/`\hskip`. `mu` is the natural size in math units
    /// (0 for point glue); the box `width` is the natural width in points.
    /// `stretch`/`shrink` are TeX's glue components, already converted from
    /// mu to points for finite orders (tex.web §716 `math_glue`), which
    /// [`MathBox::pack_to`] sets like `hpack`.
    Glue {
        mu: f64,
        stretch: Flex,
        shrink: Flex,
    },
}

/// The order of infinity of a glue component (tex.web §150: `normal`,
/// `fil`, `fill`, `filll`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum GlueOrder {
    #[default]
    Normal,
    Fil,
    Fill,
    Filll,
}

impl GlueOrder {
    pub const ALL: [GlueOrder; 4] = [
        GlueOrder::Normal,
        GlueOrder::Fil,
        GlueOrder::Fill,
        GlueOrder::Filll,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

/// One stretch or shrink component of a glue: `amount` in points for
/// [`GlueOrder::Normal`], in fil units otherwise (`plus 1fill` is
/// `Flex { amount: 1.0, order: Fill }`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Flex {
    pub amount: f64,
    pub order: GlueOrder,
}

impl Flex {
    pub const ZERO: Flex = Flex {
        amount: 0.0,
        order: GlueOrder::Normal,
    };

    /// A finite component of `amount` points.
    pub fn pt(amount: f64) -> Flex {
        Flex {
            amount,
            order: GlueOrder::Normal,
        }
    }
}

/// What `hpack` sums over a list before setting it (tex.web §649
/// `total_stretch`/`total_shrink`, indexed by [`GlueOrder`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlueTotals {
    /// The list's natural width.
    pub natural: f64,
    pub stretch: [f64; 4],
    pub shrink: [f64; 4],
}

impl GlueTotals {
    /// tex.web §659: the highest order with nonzero total stretch.
    pub fn stretch_order(&self) -> GlueOrder {
        highest(&self.stretch)
    }

    /// tex.web §665: the highest order with nonzero total shrink.
    pub fn shrink_order(&self) -> GlueOrder {
        highest(&self.shrink)
    }
}

fn highest(totals: &[f64; 4]) -> GlueOrder {
    GlueOrder::ALL
        .into_iter()
        .rev()
        .find(|o| totals[o.index()] != 0.0)
        .unwrap_or(GlueOrder::Normal)
}

/// How the glue of a packed box is set (tex.web §135 `glue_sign`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlueSign {
    #[default]
    Normal,
    Stretching,
    Shrinking,
}

/// A box's glue setting: `\showbox`'s `glue set [-]ratio[fil..]`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GlueSet {
    pub sign: GlueSign,
    pub order: GlueOrder,
    pub ratio: f64,
}

/// The result of [`MathBox::pack_to`].
#[derive(Debug, Clone, PartialEq)]
pub struct Packed {
    /// The box set to the requested width, glue widths and child offsets
    /// adjusted (glyph and rule geometry unchanged).
    pub root: MathBox,
    pub set: GlueSet,
    pub totals: GlueTotals,
    /// How far the contents still exceed the width when the finite shrink
    /// ran out (tex.web §664 "Report an overfull hbox"); 0 otherwise.
    pub overfull: f64,
}

/// A child box positioned inside a container.
#[derive(Debug, Clone, PartialEq)]
pub struct Child {
    /// Horizontal offset of the child's reference point from the parent's.
    pub dx: f64,
    /// Vertical offset of the child's baseline from the parent's baseline,
    /// positive downwards.
    pub dy: f64,
    pub content: MathBox,
}

/// A laid-out box with TeX dimensions in points.
#[derive(Debug, Clone, PartialEq)]
pub struct MathBox {
    pub kind: BoxKind,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    /// Provenance of a glyph or rule leaf (see [`crate::source`]); layout
    /// fills it from the atom that produced the leaf. Containers, kerns and
    /// glue leave it unset. Never read for geometry.
    pub tag: SourceTag,
}

impl MathBox {
    pub fn empty() -> MathBox {
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::HBox(Vec::new()),
            width: 0.0,
            height: 0.0,
            depth: 0.0,
        }
    }

    pub fn kern(width: f64) -> MathBox {
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::Kern,
            width,
            height: 0.0,
            depth: 0.0,
        }
    }

    /// Rigid glue of natural `width` points (`mu` math units).
    pub fn glue(width: f64, mu: f64) -> MathBox {
        MathBox::glue_flex(width, mu, Flex::ZERO, Flex::ZERO)
    }

    /// Glue of natural `width` points with TeX stretch and shrink.
    pub fn glue_flex(width: f64, mu: f64, stretch: Flex, shrink: Flex) -> MathBox {
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::Glue {
                mu,
                stretch,
                shrink,
            },
            width,
            height: 0.0,
            depth: 0.0,
        }
    }

    /// The natural width and the total stretch and shrink per order of this
    /// box's own list (tex.web §656): the glue among an [`BoxKind::HBox`]'s
    /// direct children. Glue inside nested boxes is rigid to the outer box,
    /// as in TeX, and a box that is not an hbox has no glue.
    pub fn glue_totals(&self) -> GlueTotals {
        let mut totals = GlueTotals {
            natural: self.width,
            ..GlueTotals::default()
        };
        if let BoxKind::HBox(children) = &self.kind {
            for c in children {
                if let BoxKind::Glue {
                    stretch, shrink, ..
                } = &c.content.kind
                {
                    totals.stretch[stretch.order.index()] += stretch.amount;
                    totals.shrink[shrink.order.index()] += shrink.amount;
                }
            }
        }
        totals
    }

    /// TeX `hpack(p, width, exactly)` (tex.web §649–§667) applied to this
    /// box's list: the excess or deficit is taken up by the glue of the
    /// highest order present, and finite shrink is never set beyond its
    /// total (glue set 1.0; the rest is reported as `overfull`). Only the
    /// widths of the direct glue children change; every later child moves
    /// by the accumulated difference. A box without glue (or not an hbox)
    /// keeps its contents and just takes the new width, as TeX does.
    pub fn pack_to(&self, width: f64) -> Packed {
        let totals = self.glue_totals();
        let x = width - totals.natural;
        let mut set = GlueSet::default();
        let mut overfull = 0.0;
        if x > 0.0 {
            let o = totals.stretch_order();
            if totals.stretch[o.index()] != 0.0 {
                set = GlueSet {
                    sign: GlueSign::Stretching,
                    order: o,
                    ratio: x / totals.stretch[o.index()],
                };
            }
        } else if x < 0.0 {
            let o = totals.shrink_order();
            let total = totals.shrink[o.index()];
            if total != 0.0 {
                set = GlueSet {
                    sign: GlueSign::Shrinking,
                    order: o,
                    ratio: -x / total,
                };
            }
            // §664: finite shrink cannot go past its total.
            if total < -x && o == GlueOrder::Normal {
                if total != 0.0 {
                    set.ratio = 1.0;
                }
                overfull = -x - total;
            }
        }
        let mut root = self.clone();
        root.width = width;
        if let BoxKind::HBox(children) = &mut root.kind
            && set.sign != GlueSign::Normal
        {
            let mut shift = 0.0;
            for c in children.iter_mut() {
                c.dx += shift;
                if let BoxKind::Glue {
                    stretch, shrink, ..
                } = c.content.kind
                {
                    let delta = match set.sign {
                        GlueSign::Stretching if stretch.order == set.order => {
                            stretch.amount * set.ratio
                        }
                        GlueSign::Shrinking if shrink.order == set.order => {
                            -shrink.amount * set.ratio
                        }
                        _ => 0.0,
                    };
                    c.content.width += delta;
                    shift += delta;
                }
            }
        }
        Packed {
            root,
            set,
            totals,
            overfull,
        }
    }

    /// A rule of `width` × (`height` + `depth`) around the baseline.
    pub fn rule(width: f64, height: f64, depth: f64) -> MathBox {
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::Rule,
            width,
            height,
            depth,
        }
    }

    pub fn glyph(g: &crate::metrics::Glyph) -> MathBox {
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::Glyph {
                font_id: g.font_id,
                gid: g.gid,
                ch: g.ch,
                size: g.size,
            },
            width: g.width,
            height: g.height,
            depth: g.depth,
        }
    }

    pub fn total_height(&self) -> f64 {
        self.height + self.depth
    }

    /// TeX `hpack(..., natural)`: boxes side by side, each shifted by `dy`.
    pub fn hbox(items: Vec<(f64, MathBox)>) -> MathBox {
        let mut children = Vec::with_capacity(items.len());
        let (mut x, mut height, mut depth) = (0.0f64, 0.0f64, 0.0f64);
        for (dy, b) in items {
            height = height.max(b.height - dy);
            depth = depth.max(b.depth + dy);
            let w = b.width;
            children.push(Child {
                dx: x,
                dy,
                content: b,
            });
            x += w;
        }
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::HBox(children),
            width: x,
            height,
            depth,
        }
    }

    /// An hbox with a single unshifted child list.
    pub fn hlist(boxes: Vec<MathBox>) -> MathBox {
        MathBox::hbox(boxes.into_iter().map(|b| (0.0, b)).collect())
    }

    /// TeX `vpack(..., natural)` with the baseline at the bottom item's
    /// baseline: items are stacked top to bottom; `dx` shifts an item right.
    /// `kerns` between items are expressed as [`MathBox::kern`] boxes whose
    /// `width` is reinterpreted as vertical size.
    pub fn vbox(items: Vec<(f64, MathBox)>) -> MathBox {
        let mut children = Vec::with_capacity(items.len());
        let mut width = 0.0f64;
        // Lay out from the top with y measured downwards from the top edge.
        let mut y = 0.0f64;
        let mut last_baseline = 0.0f64;
        let mut placed = Vec::with_capacity(items.len());
        for (dx, b) in items {
            if matches!(b.kind, BoxKind::Kern) {
                y += b.width;
                continue;
            }
            width = width.max(b.width + dx);
            let baseline = y + b.height;
            let bottom = baseline + b.depth;
            placed.push((dx, baseline, b));
            last_baseline = baseline;
            y = bottom;
        }
        let total = y;
        let height = last_baseline;
        let depth = total - last_baseline;
        for (dx, baseline, b) in placed {
            children.push(Child {
                dx,
                dy: baseline - height,
                content: b,
            });
        }
        MathBox {
            tag: SourceTag::NONE,
            kind: BoxKind::VBox(children),
            width,
            height,
            depth,
        }
    }

    /// TeX `vpack` with the baseline at the **top** item's baseline
    /// (`vtop`).
    pub fn vtop(items: Vec<(f64, MathBox)>) -> MathBox {
        let mut b = MathBox::vbox(items);
        if let BoxKind::VBox(children) = &mut b.kind
            && let Some(first) = children.first()
        {
            let shift = first.dy;
            let first_height = first.content.height;
            for c in children.iter_mut() {
                c.dy -= shift;
            }
            let total = b.height + b.depth;
            b.height = first_height;
            b.depth = total - first_height;
        }
        b
    }

    /// TeX `rebox`: centre this box in a box of width `w` (Rule 15, 13).
    pub fn rebox(self, w: f64) -> MathBox {
        if self.width >= w {
            return self;
        }
        let pad = (w - self.width) / 2.0;
        MathBox::hlist(vec![MathBox::kern(pad), self, MathBox::kern(pad)])
    }

    /// This box with `tag` on itself (a leaf).
    pub fn with_tag(mut self, tag: SourceTag) -> MathBox {
        self.tag = tag;
        self
    }

    /// Fills the unset tag fields of every glyph and rule leaf from `outer`
    /// (innermost-first inheritance, see [`crate::source`]).
    pub fn inherit_tag(&mut self, outer: SourceTag) {
        if outer.is_none() {
            return;
        }
        match &mut self.kind {
            BoxKind::Glyph { .. } | BoxKind::Rule => self.tag.inherit(outer),
            BoxKind::HBox(children) | BoxKind::VBox(children) => {
                for c in children {
                    c.content.inherit_tag(outer);
                }
            }
            BoxKind::Kern | BoxKind::Glue { .. } => {}
        }
    }

    /// Wrap in an hbox shifted by `dy` (positive down), as TeX does when it
    /// `hpack`s a box that carries a `shift_amount`.
    pub fn shifted(self, dy: f64) -> MathBox {
        MathBox::hbox(vec![(dy, self)])
    }
}

/// A glyph placed on the page.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedGlyph {
    pub font_id: FontId,
    pub gid: u16,
    pub ch: char,
    /// Left edge of the glyph's advance box, in pt from the origin's x.
    pub x: f64,
    /// Baseline, in pt downward from the origin's y.
    pub baseline_y: f64,
    pub size: f64,
    /// The glyph box's width in pt: the TFM character width TeX advances by
    /// (plus the italic correction where `char_box` adds it, as for
    /// delimiters), which is what pdfTeX records as the glyph's `/Widths`.
    pub width: f64,
    /// The source span and attribute of the atom that produced the glyph.
    pub tag: SourceTag,
}

/// A filled rectangle on the page; `y` is its top edge (downward axis).
#[derive(Debug, Clone, PartialEq)]
pub struct PositionedRule {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// The source span and attribute of the atom that produced the rule.
    pub tag: SourceTag,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PositionedRuns {
    pub glyphs: Vec<PositionedGlyph>,
    pub rules: Vec<PositionedRule>,
}

/// Flattens a box into glyph and rule runs.
///
/// `origin` is the **top-left corner** of the box in a y-down coordinate
/// system (points): the box's baseline sits at `origin.1 + root.height`.
/// To place on a PDF page (y up), use `y_pdf = page_height - y`; a glyph's
/// text matrix origin is `(x, y_pdf(baseline_y))` and a rule is the rectangle
/// `(x, y_pdf(y + h), w, h)`.
pub fn positioned_runs(root: &MathBox, origin: (f64, f64)) -> PositionedRuns {
    let mut out = PositionedRuns::default();
    walk(root, origin.0, origin.1 + root.height, &mut out);
    out
}

fn walk(b: &MathBox, x: f64, baseline: f64, out: &mut PositionedRuns) {
    match &b.kind {
        BoxKind::Glyph {
            font_id,
            gid,
            ch,
            size,
        } => out.glyphs.push(PositionedGlyph {
            font_id: *font_id,
            gid: *gid,
            ch: *ch,
            x,
            baseline_y: baseline,
            size: *size,
            width: b.width,
            tag: b.tag,
        }),
        BoxKind::Rule => out.rules.push(PositionedRule {
            x,
            y: baseline - b.height,
            w: b.width,
            h: b.height + b.depth,
            tag: b.tag,
        }),
        BoxKind::HBox(children) | BoxKind::VBox(children) => {
            for c in children {
                walk(&c.content, x + c.dx, baseline + c.dy, out);
            }
        }
        BoxKind::Kern | BoxKind::Glue { .. } => {}
    }
}
