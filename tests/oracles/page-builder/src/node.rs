//! Vertical-list nodes (tex.web part 10), reduced to what vertical mode,
//! `\vsplit` and the page builder observe.

use crate::scaled::Scaled;

/// Glue stretch/shrink order (§150).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum Order {
    #[default]
    Normal,
    Fil,
    Fill,
    Filll,
}

impl Order {
    pub fn index(self) -> usize {
        self as usize
    }
}

/// A glue specification (§150).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct GlueSpec {
    pub width: Scaled,
    pub stretch: Scaled,
    pub stretch_order: Order,
    pub shrink: Scaled,
    pub shrink_order: Order,
}

impl GlueSpec {
    pub const ZERO: GlueSpec = GlueSpec {
        width: 0,
        stretch: 0,
        stretch_order: Order::Normal,
        shrink: 0,
        shrink_order: Order::Normal,
    };

    pub fn fixed(width: Scaled) -> GlueSpec {
        GlueSpec { width, ..GlueSpec::ZERO }
    }

    pub fn new(width: Scaled, stretch: Scaled, shrink: Scaled) -> GlueSpec {
        GlueSpec { width, stretch, shrink, ..GlueSpec::ZERO }
    }

    /// `\vfil`.
    pub fn fil() -> GlueSpec {
        GlueSpec { stretch: crate::scaled::UNITY, stretch_order: Order::Fil, ..GlueSpec::ZERO }
    }

    /// `\vfill`.
    pub fn fill() -> GlueSpec {
        GlueSpec { stretch: crate::scaled::UNITY, stretch_order: Order::Fill, ..GlueSpec::ZERO }
    }

    /// `\vss`.
    pub fn ss() -> GlueSpec {
        GlueSpec {
            stretch: crate::scaled::UNITY,
            stretch_order: Order::Fil,
            shrink: crate::scaled::UNITY,
            shrink_order: Order::Fil,
            width: 0,
        }
    }

    /// `-glue` as produced by `\vskip-\lastskip` (§1238 negation keeps orders).
    pub fn negated(self) -> GlueSpec {
        GlueSpec { width: -self.width, stretch: -self.stretch, shrink: -self.shrink, ..self }
    }

    /// `\advance<skip register> by <glue>` (§1239): `self` is the register,
    /// `val` the scanned glue.
    pub fn advanced_by(self, val: GlueSpec) -> GlueSpec {
        let r = self;
        let mut q = val;
        q.width = q.width.wrapping_add(r.width);
        if q.stretch == 0 {
            q.stretch_order = Order::Normal;
        }
        if q.stretch_order == r.stretch_order {
            q.stretch = q.stretch.wrapping_add(r.stretch);
        } else if q.stretch_order < r.stretch_order && r.stretch != 0 {
            q.stretch = r.stretch;
            q.stretch_order = r.stretch_order;
        }
        if q.shrink == 0 {
            q.shrink_order = Order::Normal;
        }
        if q.shrink_order == r.shrink_order {
            q.shrink = q.shrink.wrapping_add(r.shrink);
        } else if q.shrink_order < r.shrink_order && r.shrink != 0 {
            q.shrink = r.shrink;
            q.shrink_order = r.shrink_order;
        }
        q
    }
}

/// Which parameter a glue node came from (§149 subtypes); only
/// `TopSkip`/`SplitTopSkip` matter to the algorithms, the rest aid tracing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum GlueKind {
    #[default]
    Normal,
    BaselineSkip,
    LineSkip,
    TopSkip,
    SplitTopSkip,
    ParSkip,
    AboveDisplaySkip,
    BelowDisplaySkip,
    AboveDisplayShortSkip,
    BelowDisplayShortSkip,
    /// `\leaders`/`\cleaders`/`\xleaders` glue: measured as glue.
    Leaders,
}

/// An `\hbox` or `\vbox` as the vertical list sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct BoxNode {
    pub vertical: bool,
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
    /// `shift_amount` (moves the box right in a vertical list).
    pub shift: Scaled,
    /// Caller payload identifying the box (a line, a display...).
    pub id: u32,
}

impl BoxNode {
    pub fn hbox(width: Scaled, height: Scaled, depth: Scaled, id: u32) -> BoxNode {
        BoxNode { vertical: false, width, height, depth, shift: 0, id }
    }
}

/// An `\insert` node (§140).
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct InsNode {
    /// Insertion class `n` of `\insert n`.
    pub number: u8,
    /// Natural height plus depth of the vlist.
    pub height: Scaled,
    /// `\splitmaxdepth` in force (TeX stores it in `depth`).
    pub split_max_depth: Scaled,
    /// `\splittopskip` in force.
    pub split_top_skip: GlueSpec,
    /// `\floatingpenalty` in force.
    pub float_cost: i32,
    pub list: Vec<Node>,
}

/// A node of a vertical list.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node {
    Box(BoxNode),
    /// A rule; `width: None` is a running width.
    Rule { width: Option<Scaled>, height: Scaled, depth: Scaled },
    Ins(InsNode),
    /// `\mark`; the id refers to caller-held token lists.
    Mark(u32),
    /// `\write`, `\special`, `\pdfsavepos`, colour stacks...: zero-size.
    Whatsit(u32),
    Glue { spec: GlueSpec, kind: GlueKind },
    /// `explicit` is `\kern` (vs. font or accent kerns).
    Kern { width: Scaled, explicit: bool },
    Penalty(i32),
}

impl Node {
    pub fn glue(spec: GlueSpec) -> Node {
        Node::Glue { spec, kind: GlueKind::Normal }
    }

    pub fn kern(width: Scaled) -> Node {
        Node::Kern { width, explicit: true }
    }

    /// tex.web `type` codes (§133–§159), used for ordering tests.
    pub fn type_code(&self) -> u8 {
        match self {
            Node::Box(b) => u8::from(b.vertical),
            Node::Rule { .. } => 2,
            Node::Ins(_) => 3,
            Node::Mark(_) => 4,
            Node::Whatsit(_) => 8,
            Node::Glue { .. } => 10,
            Node::Kern { .. } => 11,
            Node::Penalty(_) => 12,
        }
    }

    /// `precedes_break` (§148): glue after this node is a legal breakpoint.
    pub fn precedes_break(&self) -> bool {
        self.type_code() < 9
    }

    pub fn is_box_or_rule(&self) -> bool {
        matches!(self, Node::Box(_) | Node::Rule { .. })
    }

    pub fn is_discardable(&self) -> bool {
        matches!(self, Node::Glue { .. } | Node::Kern { .. } | Node::Penalty(_))
    }

    /// Height and depth of a box or rule (rules with running dimensions
    /// are never running in height/depth inside a vertical list).
    pub fn height_depth(&self) -> (Scaled, Scaled) {
        match self {
            Node::Box(b) => (b.height, b.depth),
            Node::Rule { height, depth, .. } => (*height, *depth),
            _ => (0, 0),
        }
    }
}
