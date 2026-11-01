//! TeX's node model (tex.web part 10, §§133–161), using owned vectors instead
//! of linked lists in `mem`.

use crate::scaled::{NULL_FLAG, Scaled};

/// Order of infinity of a glue component (§150 `glue_ord`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum GlueOrder {
    #[default]
    Normal = 0,
    Fil = 1,
    Fill = 2,
    Filll = 3,
}

impl GlueOrder {
    pub const ALL: [GlueOrder; 4] = [GlueOrder::Normal, GlueOrder::Fil, GlueOrder::Fill, GlueOrder::Filll];
    pub fn index(self) -> usize {
        self as usize
    }
}

/// A glue specification (§150).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlueSpec {
    pub width: Scaled,
    pub stretch: Scaled,
    pub stretch_order: GlueOrder,
    pub shrink: Scaled,
    pub shrink_order: GlueOrder,
}

impl GlueSpec {
    pub const ZERO: GlueSpec = GlueSpec {
        width: 0,
        stretch: 0,
        stretch_order: GlueOrder::Normal,
        shrink: 0,
        shrink_order: GlueOrder::Normal,
    };
    /// `fil_glue` (§162): `\hfil`/`\vfil`.
    pub const FIL: GlueSpec = GlueSpec { stretch: 65536, stretch_order: GlueOrder::Fil, ..GlueSpec::ZERO };
    /// `fill_glue` (§162): `\hfill`/`\vfill`.
    pub const FILL: GlueSpec = GlueSpec { stretch: 65536, stretch_order: GlueOrder::Fill, ..GlueSpec::ZERO };
    /// `ss_glue` (§162): `\hss`/`\vss`.
    pub const SS: GlueSpec = GlueSpec {
        stretch: 65536,
        stretch_order: GlueOrder::Fil,
        shrink: 65536,
        shrink_order: GlueOrder::Fil,
        width: 0,
    };
    /// `fil_neg_glue` (§162): `\hfilneg`/`\vfilneg`.
    pub const FIL_NEG: GlueSpec = GlueSpec { stretch: -65536, stretch_order: GlueOrder::Fil, ..GlueSpec::ZERO };

    pub fn fixed(width: Scaled) -> GlueSpec {
        GlueSpec { width, ..GlueSpec::ZERO }
    }
    pub fn is_zero(&self) -> bool {
        self.width == 0 && self.stretch == 0 && self.shrink == 0
    }
}

/// Glue parameters (§224), in `print_skip_param` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkipParam {
    LineSkip,
    BaselineSkip,
    ParSkip,
    AboveDisplaySkip,
    BelowDisplaySkip,
    AboveDisplayShortSkip,
    BelowDisplayShortSkip,
    LeftSkip,
    RightSkip,
    TopSkip,
    SplitTopSkip,
    TabSkip,
    SpaceSkip,
    XSpaceSkip,
    ParFillSkip,
    ThinMuSkip,
    MedMuSkip,
    ThickMuSkip,
}

impl SkipParam {
    /// `print_skip_param` (§225), without the escape character.
    pub fn name(self) -> &'static str {
        match self {
            SkipParam::LineSkip => "lineskip",
            SkipParam::BaselineSkip => "baselineskip",
            SkipParam::ParSkip => "parskip",
            SkipParam::AboveDisplaySkip => "abovedisplayskip",
            SkipParam::BelowDisplaySkip => "belowdisplayskip",
            SkipParam::AboveDisplayShortSkip => "abovedisplayshortskip",
            SkipParam::BelowDisplayShortSkip => "belowdisplayshortskip",
            SkipParam::LeftSkip => "leftskip",
            SkipParam::RightSkip => "rightskip",
            SkipParam::TopSkip => "topskip",
            SkipParam::SplitTopSkip => "splittopskip",
            SkipParam::TabSkip => "tabskip",
            SkipParam::SpaceSkip => "spaceskip",
            SkipParam::XSpaceSkip => "xspaceskip",
            SkipParam::ParFillSkip => "parfillskip",
            SkipParam::ThinMuSkip => "thinmuskip",
            SkipParam::MedMuSkip => "medmuskip",
            SkipParam::ThickMuSkip => "thickmuskip",
        }
    }
}

/// Leader flavours (§149 `a_leaders`, `c_leaders`, `x_leaders`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderKind {
    Aligned,
    Centered,
    Expanded,
}

/// Glue node subtype (§149).
#[derive(Debug, Clone, PartialEq)]
pub enum GlueKind {
    Normal,
    Param(SkipParam),
    CondMath,
    Mu,
    Leaders(LeaderKind, Box<Node>),
}

/// A glue node. `zero_glue_ref` records whether the node points at TeX's
/// shared `zero_glue` spec, which only matters for `short_display` (§175).
#[derive(Debug, Clone, PartialEq)]
pub struct GlueNode {
    pub spec: GlueSpec,
    pub kind: GlueKind,
    pub zero_glue_ref: bool,
}

impl GlueNode {
    pub fn new(spec: GlueSpec) -> GlueNode {
        GlueNode { spec, kind: GlueKind::Normal, zero_glue_ref: false }
    }
}

/// `glue_sign` (§135).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GlueSign {
    #[default]
    Normal,
    Stretching,
    Shrinking,
}

/// Horizontal or vertical list box (§135 `hlist_node`/`vlist_node`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    H,
    V,
}

/// An `hlist_node` or `vlist_node` (§135).
#[derive(Debug, Clone, PartialEq)]
pub struct BoxNode {
    pub kind: ListKind,
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
    pub shift: Scaled,
    pub list: Vec<Node>,
    /// `glue_set` as web2c's `glueratio` (C `double`).
    pub glue_set: f64,
    pub glue_sign: GlueSign,
    pub glue_order: GlueOrder,
}

impl BoxNode {
    /// `new_null_box` (§136).
    pub fn null(kind: ListKind) -> BoxNode {
        BoxNode {
            kind,
            width: 0,
            height: 0,
            depth: 0,
            shift: 0,
            list: Vec::new(),
            glue_set: 0.0,
            glue_sign: GlueSign::Normal,
            glue_order: GlueOrder::Normal,
        }
    }
}

/// A rule (§138); `NULL_FLAG` marks a running dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
}

impl Rule {
    /// `new_rule` (§139): all dimensions running.
    pub fn running() -> Rule {
        Rule { width: NULL_FLAG, height: NULL_FLAG, depth: NULL_FLAG }
    }
    /// `\vrule` defaults (§463).
    pub fn vrule() -> Rule {
        Rule { width: crate::scaled::DEFAULT_RULE, ..Rule::running() }
    }
    /// `\hrule` defaults (§463).
    pub fn hrule() -> Rule {
        Rule { height: crate::scaled::DEFAULT_RULE, depth: 0, ..Rule::running() }
    }
}

/// `is_running` (§138).
pub fn is_running(d: Scaled) -> bool {
    d == NULL_FLAG
}

/// Kern subtype (§155).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernKind {
    /// Font kern (inserted from the TFM lig/kern program).
    Normal,
    Explicit,
    Accent,
    Mu,
}

/// pdfTeX margin kern side (`margin_kern_node`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarginSide {
    Left,
    Right,
}

/// An opaque whatsit (§1341). `display` is printed after the escape
/// character by `show_box`; `tag` identifies it in shipout events.
#[derive(Debug, Clone, PartialEq)]
pub struct Whatsit {
    pub tag: u32,
    pub display: String,
}

/// An insertion node (§140).
#[derive(Debug, Clone, PartialEq)]
pub struct Insert {
    pub number: u16,
    pub height: Scaled,
    pub depth: Scaled,
    pub split_top: GlueSpec,
    pub float_cost: i32,
    pub list: Vec<Node>,
}

/// A node in an hlist or vlist.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Char { font: u32, ch: u32 },
    Ligature { font: u32, ch: u32, original: Vec<Node>, left_boundary: bool, right_boundary: bool },
    Box(BoxNode),
    Rule(Rule),
    Glue(GlueNode),
    Kern { width: Scaled, kind: KernKind },
    MarginKern { width: Scaled, side: MarginSide },
    Penalty(i32),
    Math { on: bool, width: Scaled },
    Disc { pre: Vec<Node>, post: Vec<Node>, replace_count: u16 },
    Whatsit(Whatsit),
    Mark(String),
    Insert(Insert),
    Adjust(Vec<Node>),
}

impl Node {
    pub fn hbox(b: BoxNode) -> Node {
        Node::Box(b)
    }
    pub fn glue(spec: GlueSpec) -> Node {
        Node::Glue(GlueNode::new(spec))
    }
    pub fn param_glue(param: SkipParam, spec: GlueSpec, zero_glue_ref: bool) -> Node {
        Node::Glue(GlueNode { spec, kind: GlueKind::Param(param), zero_glue_ref })
    }
    pub fn kern(width: Scaled) -> Node {
        Node::Kern { width, kind: KernKind::Explicit }
    }
    pub fn as_box(&self) -> Option<&BoxNode> {
        match self {
            Node::Box(b) => Some(b),
            _ => None,
        }
    }
    pub fn is_glue(&self) -> bool {
        matches!(self, Node::Glue(_))
    }
}

/// Character metrics source. Character and ligature nodes are opaque to
/// this crate: widths, heights and depths come from the font layer.
pub trait CharMetrics {
    /// `(width, height, depth)` of character `ch` in `font`.
    fn char_dims(&self, font: u32, ch: u32) -> (Scaled, Scaled, Scaled);
    /// Font identifier as printed by `show_box` (§267), e.g. `\OT1/cmr/m/n/10`.
    fn font_identifier(&self, font: u32) -> String {
        format!("\\FONT{font}")
    }
}

/// Metrics for lists that contain no characters.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoChars;

impl CharMetrics for NoChars {
    fn char_dims(&self, _font: u32, _ch: u32) -> (Scaled, Scaled, Scaled) {
        (0, 0, 0)
    }
}
