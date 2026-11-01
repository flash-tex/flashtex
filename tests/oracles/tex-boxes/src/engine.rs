//! A primitive-level box builder: TeX's semantic nest (§§211–219), the
//! grouping/save-stack rules for registers (§§268–284), and the box-related
//! parts of `main_control` (§§1055–1110, 1167–1168, 1193–1196).
//!
//! The engine does no tokenization or macro expansion. A front end (KC-101
//! `tex-expansion`, or the test DSL) calls one method per executed primitive.

use std::collections::HashMap;

use crate::display::{Printer, ShowLimits, show_box_string};
use crate::node::{
    BoxNode, CharMetrics, GlueKind, GlueNode, GlueSpec, KernKind, LeaderKind, ListKind, NoChars, Node, Rule, SkipParam,
    Whatsit,
};
use crate::pack::{PackOrigin, PackParams, PackReport, PackSpec, hpack, vpack, vpackage};
use crate::scaled::{IGNORE_DEPTH, MAX_DIMEN, Scaled, half};

/// TeX's modes (§211). `Vertical` is the outer (main) vertical list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Vertical,
    InternalVertical,
    Horizontal,
    RestrictedHorizontal,
    Math,
}

impl Mode {
    pub fn is_vertical(self) -> bool {
        matches!(self, Mode::Vertical | Mode::InternalVertical)
    }
    pub fn is_horizontal(self) -> bool {
        matches!(self, Mode::Horizontal | Mode::RestrictedHorizontal)
    }
}

/// `\hbox`, `\vbox`, `\vtop` (§1071).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxKind {
    HBox,
    VBox,
    VTop,
}

/// What happens to a finished box (§1071 context codes, §1075 `box_end`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxContext {
    /// Append to the current list shifted by the amount (`\raise`, `\lower`,
    /// `\moveleft`, `\moveright`; plain boxes use `Shift(0)`).
    Shift(Scaled),
    SetBox { register: u32, global: bool },
    ShipOut,
    Leaders(LeaderKind),
}

impl BoxContext {
    pub const APPEND: BoxContext = BoxContext::Shift(0);
}

/// Which dimension of a box register (`\wd`, `\ht`, `\dp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxDim {
    Width,
    Height,
    Depth,
}

/// Errors the engine reports instead of TeX's interactive recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoxError {
    /// TeX's `print_err` text.
    Tex(String),
    GroupMismatch { expected: &'static str },
}

impl std::fmt::Display for BoxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoxError::Tex(s) => write!(f, "! {s}"),
            BoxError::GroupMismatch { expected } => write!(f, "group mismatch: expected {expected}"),
        }
    }
}

impl std::error::Error for BoxError {}

pub type BoxResult<T = ()> = Result<T, BoxError>;

/// A line produced by a [`LineBreaker`], before justification (§880).
#[derive(Debug, Clone)]
pub struct BrokenLine {
    /// Line material; if `break_glue` is set its final node is the glue at
    /// which the break occurred and is replaced by `\rightskip` (§881).
    pub nodes: Vec<Node>,
    pub break_glue: bool,
    pub width: Scaled,
    pub indent: Scaled,
    /// Interline penalty appended after the line (0 = none, §890).
    pub penalty_after: i32,
}

/// Parameters handed to a line breaker.
#[derive(Debug, Clone, Copy)]
pub struct LineBreakParams {
    pub hsize: Scaled,
    pub pack: PackParams,
}

/// Paragraph builder hook (`line_break`, §§813–890). The engine performs
/// §816's preprocessing (final glue → `\penalty10000`, `\parfillskip`) and
/// §§880–890 justification; implementors choose breakpoints. KC-102/
/// paragraph-layout plug in here.
pub trait LineBreaker {
    fn break_paragraph(&mut self, list: Vec<Node>, params: &LineBreakParams, metrics: &dyn CharMetrics) -> Vec<BrokenLine>;
}

/// Sets the whole paragraph as one line of width `\hsize`. Exact whenever
/// TeX would choose a single line (e.g. the material fits within `\hsize`
/// and `\parfillskip` has infinite stretch).
#[derive(Debug, Default, Clone, Copy)]
pub struct SingleLineBreaker;

impl LineBreaker for SingleLineBreaker {
    fn break_paragraph(&mut self, list: Vec<Node>, params: &LineBreakParams, _metrics: &dyn CharMetrics) -> Vec<BrokenLine> {
        vec![BrokenLine { nodes: list, break_glue: false, width: params.hsize, indent: 0, penalty_after: 0 }]
    }
}

/// Page builder hook (`build_page`, §994), called whenever TeX would call it
/// for the outer vertical list. KC-106 implements this.
pub trait PageBuilder {
    fn build_page(&mut self, contributions: &mut Vec<Node>);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Key {
    Box(u32),
    Int(String),
    Dimen(String),
    Skip(String),
}

#[derive(Debug, Clone)]
enum Value {
    Box(Option<BoxNode>),
    Int(i32),
    Dimen(Scaled),
    Skip(GlueSpec),
}

#[derive(Debug, Clone)]
struct Slot {
    value: Value,
    level: u16,
}

#[derive(Debug, Clone)]
enum GroupKind {
    Simple,
    Box { kind: BoxKind, spec: PackSpec, context: BoxContext, adjusted: bool },
    VCenter { spec: PackSpec },
    Math,
    Adjust,
}

impl GroupKind {
    fn name(&self) -> &'static str {
        match self {
            GroupKind::Simple => "simple group",
            GroupKind::Box { .. } => "box group",
            GroupKind::VCenter { .. } => "vcenter group",
            GroupKind::Math => "math shift group",
            GroupKind::Adjust => "vadjust group",
        }
    }
}

#[derive(Debug)]
struct SaveFrame {
    group: GroupKind,
    saved: Vec<(Key, Slot)>,
}

#[derive(Debug, Clone)]
struct ListState {
    mode: Mode,
    list: Vec<Node>,
    prev_depth: Scaled,
    mode_line: i32,
}

/// The box builder.
pub struct BoxEngine {
    nest: Vec<ListState>,
    saves: Vec<SaveFrame>,
    eq: HashMap<Key, Slot>,
    pub metrics: Box<dyn CharMetrics>,
    pub line_breaker: Box<dyn LineBreaker>,
    pub page_builder: Option<Box<dyn PageBuilder>>,
    last_badness: i32,
    reports: Vec<PackReport>,
    log: String,
    /// Current input line, used in box diagnostics.
    pub line: i32,
    pack_begin_line: i32,
    pub output_active: bool,
    pending_leaders: Option<(LeaderKind, Node)>,
    shipped: Vec<BoxNode>,
    /// `axis_height(cur_size)` of `\textfont2` for `\vcenter` (§736).
    pub axis_height: Scaled,
    /// `max_print_line` used when rendering diagnostics.
    pub max_print_line: usize,
    /// Warnings emitted by the LaTeX layer.
    pub warnings: Vec<String>,
    /// TeX's `term_offset`: characters on the current terminal line. `print_nl`
    /// in `write_out` (§1370) breaks the line when either the terminal or the
    /// log is mid-line, so messages written to terminal+log (LaTeX warnings)
    /// gain a leading newline in the log when the terminal is dirty.
    pub term_offset: usize,
    /// Run LaTeX's (≥ 2021-06) standard `\everypar` from `new_graf`; see
    /// [`BoxEngine::latex_standard_everypar`]. Off for plain TeX.
    pub latex_para_hooks: bool,
    /// `\g_para_indent_box`.
    para_indent_box: Option<BoxNode>,
}

impl Default for BoxEngine {
    fn default() -> Self {
        BoxEngine::new(Box::new(NoChars))
    }
}

fn skip_param(name: &str) -> Option<SkipParam> {
    Some(match name {
        "lineskip" => SkipParam::LineSkip,
        "baselineskip" => SkipParam::BaselineSkip,
        "parskip" => SkipParam::ParSkip,
        "leftskip" => SkipParam::LeftSkip,
        "rightskip" => SkipParam::RightSkip,
        "topskip" => SkipParam::TopSkip,
        "splittopskip" => SkipParam::SplitTopSkip,
        "tabskip" => SkipParam::TabSkip,
        "spaceskip" => SkipParam::SpaceSkip,
        "xspaceskip" => SkipParam::XSpaceSkip,
        "parfillskip" => SkipParam::ParFillSkip,
        "abovedisplayskip" => SkipParam::AboveDisplaySkip,
        "belowdisplayskip" => SkipParam::BelowDisplaySkip,
        "abovedisplayshortskip" => SkipParam::AboveDisplayShortSkip,
        "belowdisplayshortskip" => SkipParam::BelowDisplayShortSkip,
        _ => return None,
    })
}

impl BoxEngine {
    /// A fresh engine in outer vertical mode with IniTeX parameter values.
    pub fn new(metrics: Box<dyn CharMetrics>) -> BoxEngine {
        BoxEngine {
            nest: vec![ListState { mode: Mode::Vertical, list: Vec::new(), prev_depth: IGNORE_DEPTH, mode_line: 0 }],
            saves: Vec::new(),
            eq: HashMap::new(),
            metrics,
            line_breaker: Box::new(SingleLineBreaker),
            page_builder: None,
            last_badness: 0,
            reports: Vec::new(),
            log: String::new(),
            line: 0,
            pack_begin_line: 0,
            output_active: false,
            pending_leaders: None,
            term_offset: 0,
            latex_para_hooks: false,
            para_indent_box: None,
            shipped: Vec::new(),
            axis_height: 0,
            max_print_line: 79,
            warnings: Vec::new(),
        }
    }

    // ----- state accessors -------------------------------------------------

    pub fn mode(&self) -> Mode {
        self.cur().mode
    }
    fn cur(&self) -> &ListState {
        self.nest.last().expect("nest never empty")
    }
    fn cur_mut(&mut self) -> &mut ListState {
        self.nest.last_mut().expect("nest never empty")
    }
    /// The current list (`link(head)..tail`).
    pub fn current_list(&self) -> &[Node] {
        &self.cur().list
    }
    pub fn take_current_list(&mut self) -> Vec<Node> {
        std::mem::take(&mut self.cur_mut().list)
    }
    pub fn prev_depth(&self) -> Scaled {
        self.cur().prev_depth
    }
    pub fn set_prev_depth(&mut self, d: Scaled) {
        self.cur_mut().prev_depth = d;
    }
    /// `\badness`.
    pub fn last_badness(&self) -> i32 {
        self.last_badness
    }
    /// All over/underfull reports so far.
    pub fn reports(&self) -> &[PackReport] {
        &self.reports
    }
    /// The transcript text of all reports and warnings, in order.
    pub fn log(&self) -> &str {
        &self.log
    }
    /// Appends raw text to the transcript (used by the LaTeX layer).
    pub fn log_text(&mut self, s: &str) {
        self.log.push_str(s);
    }
    /// Whether the transcript is mid-line (TeX's `file_offset > 0`).
    pub fn log_mid_line(&self) -> bool {
        !self.log.is_empty() && !self.log.ends_with('\n')
    }
    pub fn shipped(&self) -> &[BoxNode] {
        &self.shipped
    }
    pub fn cur_level(&self) -> usize {
        self.saves.len() + 1
    }

    fn pack_params(&self) -> PackParams {
        PackParams {
            hbadness: self.int("hbadness"),
            vbadness: self.int("vbadness"),
            hfuzz: self.dimen("hfuzz"),
            vfuzz: self.dimen("vfuzz"),
            overfull_rule: self.dimen("overfullrule"),
            show_limits: ShowLimits { depth: self.int("showboxdepth"), breadth: self.int("showboxbreadth") },
            max_print_line: self.max_print_line,
        }
    }

    fn origin(&self) -> PackOrigin {
        PackOrigin { line: self.line, pack_begin_line: self.pack_begin_line, output_active: self.output_active }
    }

    fn record(&mut self, last_badness: i32, report: Option<PackReport>) {
        self.last_badness = last_badness;
        if let Some(r) = report {
            self.log.push_str(&r.text);
            self.reports.push(r);
        }
    }

    // ----- eqtb: registers and parameters (§§277–283) ----------------------

    fn get(&self, key: &Key) -> Option<&Value> {
        self.eq.get(key).map(|s| &s.value)
    }

    fn define(&mut self, key: Key, value: Value, global: bool) {
        let level = self.cur_level() as u16;
        if global {
            // geq_define / geq_word_define
            self.eq.insert(key, Slot { value, level: 1 });
            return;
        }
        let old = self.eq.get(&key).cloned();
        let old_level = old.as_ref().map_or(1, |s| s.level);
        if old_level != level && level > 1 {
            let saved = old.unwrap_or_else(|| Slot { value: default_value(&key), level: 1 });
            self.saves.last_mut().expect("level>1 has a frame").saved.push((key.clone(), saved));
        }
        self.eq.insert(key, Slot { value, level });
    }

    fn unsave(&mut self) -> GroupKind {
        let frame = self.saves.pop().expect("unsave without group");
        for (key, saved) in frame.saved.into_iter().rev() {
            let cur_level = self.eq.get(&key).map_or(1, |s| s.level);
            if cur_level != 1 {
                self.eq.insert(key, saved);
            }
        }
        frame.group
    }

    fn new_save_level(&mut self, group: GroupKind) {
        self.saves.push(SaveFrame { group, saved: Vec::new() });
    }

    pub fn int(&self, name: &str) -> i32 {
        match self.get(&Key::Int(name.to_string())) {
            Some(Value::Int(v)) => *v,
            _ => 0,
        }
    }
    pub fn set_int(&mut self, name: &str, v: i32, global: bool) {
        self.define(Key::Int(name.to_string()), Value::Int(v), global);
    }
    pub fn dimen(&self, name: &str) -> Scaled {
        match self.get(&Key::Dimen(name.to_string())) {
            Some(Value::Dimen(v)) => *v,
            _ => 0,
        }
    }
    pub fn set_dimen(&mut self, name: &str, v: Scaled, global: bool) {
        self.define(Key::Dimen(name.to_string()), Value::Dimen(v), global);
    }
    /// Skip register or glue parameter value; the flag is true when it
    /// points at `zero_glue` (§1229 `trap_zero_glue`).
    pub fn skip(&self, name: &str) -> (GlueSpec, bool) {
        match self.get(&Key::Skip(name.to_string())) {
            Some(Value::Skip(v)) => (*v, v.is_zero()),
            _ => (GlueSpec::ZERO, true),
        }
    }
    pub fn set_skip(&mut self, name: &str, v: GlueSpec, global: bool) {
        self.define(Key::Skip(name.to_string()), Value::Skip(v), global);
    }

    /// The contents of box register `n` (`\box` register value).
    pub fn box_register(&self, n: u32) -> Option<&BoxNode> {
        match self.get(&Key::Box(n)) {
            Some(Value::Box(b)) => b.as_ref(),
            _ => None,
        }
    }
    /// `\setbox n = <box>` with an already-built box.
    pub fn set_box_register(&mut self, n: u32, b: Option<BoxNode>, global: bool) {
        self.define(Key::Box(n), Value::Box(b), global);
    }
    /// `\box n` semantics: take the box, voiding the register at its level.
    pub fn take_box_register(&mut self, n: u32) -> Option<BoxNode> {
        match self.eq.get_mut(&Key::Box(n)) {
            Some(Slot { value: Value::Box(b), .. }) => b.take(),
            _ => None,
        }
    }
    /// `\wd n`, `\ht n`, `\dp n` (0 for a void box).
    pub fn box_dimen(&self, n: u32, which: BoxDim) -> Scaled {
        self.box_register(n).map_or(0, |b| match which {
            BoxDim::Width => b.width,
            BoxDim::Height => b.height,
            BoxDim::Depth => b.depth,
        })
    }
    /// `\wd n = d` etc. (§1247 `alter_box_dimen`): not subject to grouping.
    pub fn set_box_dimen(&mut self, n: u32, which: BoxDim, d: Scaled) {
        if let Some(Slot { value: Value::Box(Some(b)), .. }) = self.eq.get_mut(&Key::Box(n)) {
            match which {
                BoxDim::Width => b.width = d,
                BoxDim::Height => b.height = d,
                BoxDim::Depth => b.depth = d,
            }
        }
    }
    /// `\showbox n` body text.
    pub fn show_box(&self, n: u32) -> String {
        let limits = ShowLimits { depth: self.int("showboxdepth"), breadth: self.int("showboxbreadth") };
        let node = self.box_register(n).map(|b| Node::Box(b.clone()));
        show_box_string(node.as_ref(), limits, self.metrics.as_ref())
    }

    // ----- groups ------------------------------------------------------------

    /// `{` or `\begingroup`.
    pub fn begin_group(&mut self) {
        self.new_save_level(GroupKind::Simple);
    }
    /// `}` or `\endgroup` closing a simple group.
    pub fn end_group(&mut self) -> BoxResult {
        match self.saves.last().map(|f| &f.group) {
            Some(GroupKind::Simple) => {
                self.unsave();
                Ok(())
            }
            Some(g) => Err(BoxError::GroupMismatch { expected: g.name() }),
            None => Err(BoxError::Tex("Too many }'s".into())),
        }
    }

    fn push_nest(&mut self, mode: Mode) {
        let line = self.line;
        self.nest.push(ListState { mode, list: Vec::new(), prev_depth: IGNORE_DEPTH, mode_line: line });
    }

    fn pop_nest(&mut self) -> ListState {
        assert!(self.nest.len() > 1, "cannot pop the outer vertical list");
        self.nest.pop().expect("nest")
    }

    fn tail_append(&mut self, n: Node) {
        self.cur_mut().list.push(n);
    }

    fn maybe_build_page(&mut self) {
        if self.nest.len() == 1 {
            if let Some(pb) = self.page_builder.as_mut() {
                let list = &mut self.nest[0].list;
                pb.build_page(list);
            }
        }
    }

    // ----- mode switching helpers (§1090, §1094) ------------------------------

    /// Called before horizontal material in vertical mode (`back_input;
    /// new_graf(true)`, §1090).
    fn ensure_horizontal(&mut self) {
        if self.mode().is_vertical() {
            self.new_graf(true);
        }
    }

    /// `head_for_vmode` (§1095) for vertical material in horizontal mode.
    fn ensure_vertical(&mut self, what: &str) -> BoxResult {
        match self.mode() {
            Mode::Horizontal => self.par(),
            Mode::RestrictedHorizontal => {
                if what == "hrule" {
                    Err(BoxError::Tex("You can't use `\\hrule' here except with leaders".into()))
                } else {
                    Err(BoxError::Tex("Missing } inserted".into()))
                }
            }
            Mode::Math => Err(BoxError::Tex("Missing $ inserted".into())),
            _ => Ok(()),
        }
    }

    // ----- paragraphs ------------------------------------------------------------

    /// `new_graf` (§1091), including `\everypar` (the LaTeX para hooks when
    /// [`BoxEngine::latex_para_hooks`] is set).
    pub fn new_graf(&mut self, indented: bool) {
        self.new_graf_primitive(indented);
        if self.latex_para_hooks {
            self.latex_standard_everypar();
        }
    }

    /// LaTeX's `\g__para_standard_everypar_tl` (ltpara, LaTeX 2021-06+), as
    /// traced from pdfLaTeX:
    ///
    /// ```text
    /// \box_gset_to_last:N \g_para_indent_box
    /// \group_begin: \tex_par:D \group_end:
    /// \@kernel@before@para@before \hook_use:n {para/before}
    /// \group_begin: \tex_everypar:D {}
    ///   \skip_set:Nn \tex_parskip:D {\if@minipage -\tex_parskip:D \else: \c_zero_skip \fi:}
    ///   \tex_noindent:D
    /// \group_end:
    /// \@kernel@before@para@begin \hook_use:n {para/begin}
    /// \__para_handle_indent:   % \box_use_drop:N \g_para_indent_box
    /// \the\toks12              % the user-level \everypar
    /// ```
    ///
    /// The re-started paragraph appends a second `\parskip` glue (0pt, or
    /// `-\parskip` under `\if@minipage`) whenever the vertical list is
    /// non-empty, exactly as pdfTeX's `\showbox` shows. Hooks are empty here.
    fn latex_standard_everypar(&mut self) {
        self.para_indent_box = match self.cur().list.last() {
            Some(Node::Box(_)) if !self.tail_inside_disc() => match self.cur_mut().list.pop() {
                Some(Node::Box(mut b)) => {
                    b.shift = 0;
                    Some(b)
                }
                _ => None,
            },
            _ => None,
        };
        self.begin_group();
        let _ = self.par();
        let _ = self.end_group();
        self.begin_group();
        let (ps, _) = self.skip("parskip");
        let v = if self.int("@minipage") != 0 {
            GlueSpec { width: -ps.width, stretch: -ps.stretch, shrink: -ps.shrink, ..ps }
        } else {
            GlueSpec::ZERO
        };
        self.set_skip("parskip", v, false);
        self.new_graf_primitive(false);
        let _ = self.end_group();
        if let Some(b) = self.para_indent_box.take() {
            self.tail_append(Node::Box(b));
        }
        // User \everypar: \@setminipage's {\@minipagefalse\everypar{}}.
        if self.int("everypar-minipagefalse") != 0 {
            self.set_int("@minipage", 0, true);
            self.set_int("everypar-minipagefalse", 0, false);
        }
    }

    /// The primitive part of `new_graf` (§1091) without `\everypar`.
    fn new_graf_primitive(&mut self, indented: bool) {
        let cur = self.cur();
        if cur.mode == Mode::Vertical || !cur.list.is_empty() {
            let (spec, zero) = self.skip("parskip");
            self.tail_append(Node::param_glue(SkipParam::ParSkip, spec, zero));
        }
        self.push_nest(Mode::Horizontal);
        if indented {
            let mut b = BoxNode::null(ListKind::H);
            b.width = self.dimen("parindent");
            self.tail_append(Node::Box(b));
        }
        if self.nest.len() == 2 {
            // nest_ptr=1
            if let Some(pb) = self.page_builder.as_mut() {
                pb.build_page(&mut self.nest[0].list);
            }
        }
    }

    /// `\leavevmode` (`\unhbox\voidb@x`).
    pub fn leavevmode(&mut self) {
        self.ensure_horizontal();
    }

    /// `\indent` (§§1092–1093).
    pub fn indent(&mut self) {
        if self.mode().is_vertical() {
            self.new_graf(true);
        } else {
            let mut b = BoxNode::null(ListKind::H);
            b.width = self.dimen("parindent");
            self.tail_append(Node::Box(b));
        }
    }

    /// `\noindent`.
    pub fn noindent(&mut self) {
        if self.mode().is_vertical() {
            self.new_graf(false);
        }
    }

    /// `\par` (§1094): ends a paragraph in unrestricted horizontal mode.
    pub fn par(&mut self) -> BoxResult {
        match self.mode() {
            Mode::Vertical => {
                self.maybe_build_page();
                Ok(())
            }
            Mode::Horizontal => {
                self.end_graf();
                if self.mode() == Mode::Vertical {
                    self.maybe_build_page();
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// `end_graf` (§1096).
    fn end_graf(&mut self) {
        if self.mode() != Mode::Horizontal {
            return;
        }
        if self.cur().list.is_empty() {
            self.pop_nest();
            return;
        }
        self.line_break();
    }

    /// `line_break` preprocessing (§816) and `post_line_break` (§§877–890).
    fn line_break(&mut self) {
        let state = self.pop_nest();
        let mut list = state.list;
        self.pack_begin_line = state.mode_line;
        match list.last() {
            Some(Node::Glue(_)) => {
                list.pop();
                list.push(Node::Penalty(10000));
            }
            _ => list.push(Node::Penalty(10000)),
        }
        let (pfs, pfs_zero) = self.skip("parfillskip");
        list.push(Node::param_glue(SkipParam::ParFillSkip, pfs, pfs_zero));
        let params = LineBreakParams { hsize: self.dimen("hsize"), pack: self.pack_params() };
        let lines = self.line_breaker.break_paragraph(list, &params, self.metrics.as_ref());
        let (rs, rs_zero) = self.skip("rightskip");
        let (ls, ls_zero) = self.skip("leftskip");
        for line in lines {
            let mut nodes = line.nodes;
            // §881, §886
            if line.break_glue {
                if let Some(Node::Glue(_)) = nodes.last() {
                    nodes.pop();
                }
            }
            nodes.push(Node::param_glue(SkipParam::RightSkip, rs, rs_zero));
            // §887
            if !ls_zero {
                nodes.insert(0, Node::param_glue(SkipParam::LeftSkip, ls, false));
            }
            // §889
            let out = hpack(nodes, PackSpec::Exactly(line.width), true, &params.pack, self.origin(), self.metrics.as_ref());
            self.record(out.last_badness, out.report);
            let mut just_box = out.node;
            just_box.shift = line.indent;
            // §888
            self.append_to_vlist(just_box);
            for n in out.adjust {
                self.tail_append(n);
            }
            // §890
            if line.penalty_after != 0 {
                self.tail_append(Node::Penalty(line.penalty_after));
            }
        }
        self.pack_begin_line = 0;
    }

    /// `append_to_vlist` (§679).
    pub fn append_to_vlist(&mut self, b: BoxNode) {
        let prev_depth = self.cur().prev_depth;
        if prev_depth > IGNORE_DEPTH {
            let (bs, _) = self.skip("baselineskip");
            let d = bs.width - prev_depth - b.height;
            if d < self.dimen("lineskiplimit") {
                let (ls, ls_zero) = self.skip("lineskip");
                self.tail_append(Node::param_glue(SkipParam::LineSkip, ls, ls_zero));
            } else {
                let spec = GlueSpec { width: d, ..bs };
                self.tail_append(Node::param_glue(SkipParam::BaselineSkip, spec, false));
            }
        }
        let depth = b.depth;
        self.tail_append(Node::Box(b));
        self.cur_mut().prev_depth = depth;
    }

    // ----- simple appends ----------------------------------------------------------

    /// A character in horizontal mode (starts a paragraph in vertical mode).
    pub fn append_char(&mut self, font: u32, ch: u32) -> BoxResult {
        self.ensure_horizontal();
        self.tail_append(Node::Char { font, ch });
        Ok(())
    }

    /// `\vrule` (horizontal material, §1056).
    pub fn vrule(&mut self, rule: Rule) -> BoxResult {
        if let Some(()) = self.try_rule_leaders(rule) {
            return Ok(());
        }
        self.ensure_horizontal();
        self.tail_append(Node::Rule(rule));
        Ok(())
    }

    /// `\hrule` (vertical material, §1056).
    pub fn hrule(&mut self, rule: Rule) -> BoxResult {
        self.ensure_vertical("hrule")?;
        self.tail_append(Node::Rule(rule));
        self.cur_mut().prev_depth = IGNORE_DEPTH;
        Ok(())
    }

    fn try_rule_leaders(&mut self, _rule: Rule) -> Option<()> {
        None
    }

    /// `\leaders`/`\cleaders`/`\xleaders` followed by a rule (§1073 `scan_box`).
    pub fn leaders_rule(&mut self, kind: LeaderKind, rule: Rule) {
        self.pending_leaders = Some((kind, Node::Rule(rule)));
    }

    /// `\hskip` (and `\hfil` etc. via [`GlueSpec`] constants). `zero_glue_ref`
    /// is true when the spec came from a register holding `zero_glue`.
    pub fn hskip(&mut self, spec: GlueSpec, zero_glue_ref: bool) -> BoxResult {
        if self.pending_leaders.is_none() {
            self.ensure_horizontal();
        } else if self.mode().is_vertical() {
            self.pending_leaders = None;
            return Err(BoxError::Tex("Leaders not followed by proper glue".into()));
        }
        self.append_glue(spec, zero_glue_ref);
        Ok(())
    }

    /// `\vskip` (and `\vfil` etc.).
    pub fn vskip(&mut self, spec: GlueSpec, zero_glue_ref: bool) -> BoxResult {
        if self.pending_leaders.is_some() && !self.mode().is_vertical() {
            self.pending_leaders = None;
            return Err(BoxError::Tex("Leaders not followed by proper glue".into()));
        }
        self.ensure_vertical("vskip")?;
        self.append_glue(spec, zero_glue_ref);
        Ok(())
    }

    /// `\hskip\name` / `\vskip\name` from a skip register or parameter.
    pub fn skip_from_register(&mut self, name: &str, vertical: bool) -> BoxResult {
        let (spec, zero) = self.skip(name);
        if vertical { self.vskip(spec, zero) } else { self.hskip(spec, zero) }
    }

    /// `append_glue` (§1060) plus leader attachment (§1078).
    fn append_glue(&mut self, spec: GlueSpec, zero_glue_ref: bool) {
        let mut g = GlueNode { spec, kind: GlueKind::Normal, zero_glue_ref };
        if let Some((kind, leader)) = self.pending_leaders.take() {
            g.kind = GlueKind::Leaders(kind, Box::new(leader));
        }
        self.tail_append(Node::Glue(g));
    }

    /// `\kern` (§1061), explicit subtype.
    pub fn kern(&mut self, width: Scaled) {
        self.tail_append(Node::Kern { width, kind: KernKind::Explicit });
    }

    /// `\penalty` (§1103).
    pub fn penalty(&mut self, n: i32) {
        self.tail_append(Node::Penalty(n));
        if self.mode() == Mode::Vertical {
            self.maybe_build_page();
        }
    }

    /// A whatsit (e.g. `\pdfsavepos`, `\write`, `\special`).
    pub fn whatsit(&mut self, tag: u32, display: &str) {
        self.tail_append(Node::Whatsit(Whatsit { tag, display: display.to_string() }));
    }

    /// `\mark{...}`.
    pub fn mark(&mut self, text: &str) {
        self.tail_append(Node::Mark(text.to_string()));
    }

    /// `\vadjust{` (§1099): internal vertical mode inside a group.
    pub fn begin_vadjust(&mut self) -> BoxResult {
        if self.mode().is_vertical() {
            return Err(BoxError::Tex("You can't use `\\vadjust' in vertical mode".into()));
        }
        self.new_save_level(GroupKind::Adjust);
        self.push_nest(Mode::InternalVertical);
        self.cur_mut().prev_depth = IGNORE_DEPTH;
        Ok(())
    }

    /// `}` closing `\vadjust` (§1100).
    pub fn end_vadjust(&mut self) -> BoxResult {
        self.expect_group(|g| matches!(g, GroupKind::Adjust), "vadjust group")?;
        self.end_graf();
        self.unsave();
        let state = self.pop_nest();
        let out = vpack(state.list, PackSpec::NATURAL, &self.pack_params(), self.origin(), self.metrics.as_ref());
        self.record(out.last_badness, out.report);
        self.tail_append(Node::Adjust(out.node.list));
        if self.nest.len() == 1 {
            self.maybe_build_page();
        }
        Ok(())
    }

    fn expect_group(&self, pred: impl Fn(&GroupKind) -> bool, expected: &'static str) -> BoxResult {
        match self.saves.last() {
            Some(f) if pred(&f.group) => Ok(()),
            _ => Err(BoxError::GroupMismatch { expected }),
        }
    }

    // ----- \lastskip & friends, \unskip (§424, §1105) ---------------------------

    fn tail_is_removable(&self) -> bool {
        !(self.mode() == Mode::Vertical && self.cur().list.is_empty())
    }

    /// `\lastskip` (zero when the tail is not glue).
    pub fn last_skip(&self) -> GlueSpec {
        match self.cur().list.last() {
            Some(Node::Glue(g)) => g.spec,
            _ => GlueSpec::ZERO,
        }
    }
    /// `\lastkern`.
    pub fn last_kern(&self) -> Scaled {
        match self.cur().list.last() {
            Some(Node::Kern { width, .. }) => *width,
            _ => 0,
        }
    }
    /// `\lastpenalty`.
    pub fn last_penalty(&self) -> i32 {
        match self.cur().list.last() {
            Some(Node::Penalty(p)) => *p,
            _ => 0,
        }
    }

    fn delete_last_if(&mut self, pred: impl Fn(&Node) -> bool) {
        if !self.tail_is_removable() {
            return;
        }
        if self.cur().list.last().is_some_and(pred) && !self.tail_inside_disc() {
            self.cur_mut().list.pop();
        }
    }

    /// True when the tail belongs to a discretionary's replacement text.
    fn tail_inside_disc(&self) -> bool {
        let list = &self.cur().list;
        let n = list.len();
        for (i, node) in list.iter().enumerate() {
            if let Node::Disc { replace_count, .. } = node {
                if i + usize::from(*replace_count) >= n - 1 && *replace_count > 0 {
                    return true;
                }
            }
        }
        false
    }

    /// `\unskip`.
    pub fn unskip(&mut self) {
        self.delete_last_if(|n| matches!(n, Node::Glue(_)));
    }
    /// `\unkern`.
    pub fn unkern(&mut self) {
        self.delete_last_if(|n| matches!(n, Node::Kern { .. }));
    }
    /// `\unpenalty`.
    pub fn unpenalty(&mut self) {
        self.delete_last_if(|n| matches!(n, Node::Penalty(_)));
    }

    // ----- boxes -------------------------------------------------------------------

    /// `\hbox`/`\vbox`/`\vtop` `[to|spread <dimen>]` `{` (§1083).
    pub fn begin_box(&mut self, kind: BoxKind, spec: PackSpec, context: BoxContext) -> BoxResult {
        if let BoxContext::Shift(s) = context {
            let _ = s;
        }
        let adjusted = kind == BoxKind::HBox && matches!(context, BoxContext::Shift(_)) && self.mode().is_vertical();
        self.new_save_level(GroupKind::Box { kind, spec, context, adjusted });
        match kind {
            BoxKind::HBox => self.push_nest(Mode::RestrictedHorizontal),
            BoxKind::VBox | BoxKind::VTop => {
                self.push_nest(Mode::InternalVertical);
                self.cur_mut().prev_depth = IGNORE_DEPTH;
            }
        }
        Ok(())
    }

    /// `}` closing `\hbox`/`\vbox`/`\vtop` (§§1085–1087).
    pub fn end_box(&mut self) -> BoxResult {
        let (kind, spec, context, adjusted) = match self.saves.last().map(|f| &f.group) {
            Some(GroupKind::Box { kind, spec, context, adjusted }) => (*kind, *spec, *context, *adjusted),
            _ => return Err(BoxError::GroupMismatch { expected: "box group" }),
        };
        if kind != BoxKind::HBox {
            self.end_graf();
        }
        // package(c), §1086
        let d = self.dimen("boxmaxdepth");
        self.unsave();
        let state = self.pop_nest();
        let params = self.pack_params();
        let origin = self.origin();
        let (mut b, adjust) = if kind == BoxKind::HBox {
            let out = hpack(state.list, spec, adjusted, &params, origin, self.metrics.as_ref());
            self.record(out.last_badness, out.report);
            (out.node, out.adjust)
        } else {
            let out = vpackage(state.list, spec, d, &params, origin, self.metrics.as_ref());
            self.record(out.last_badness, out.report);
            (out.node, Vec::new())
        };
        if kind == BoxKind::VTop {
            // §1087
            let h = match b.list.first() {
                Some(Node::Box(first)) => first.height,
                Some(Node::Rule(r)) => r.height,
                _ => 0,
            };
            b.depth = b.depth - h + b.height;
            b.height = h;
        }
        self.box_end(Some(b), context, adjust)
    }

    /// `box_end` (§1075) for an already-built box (`None` = void).
    pub fn box_end(&mut self, b: Option<BoxNode>, context: BoxContext, adjust: Vec<Node>) -> BoxResult {
        match context {
            BoxContext::Shift(shift) => {
                let Some(mut b) = b else { return Ok(()) };
                b.shift = shift;
                match self.mode() {
                    Mode::Vertical | Mode::InternalVertical => {
                        self.append_to_vlist(b);
                        for n in adjust {
                            self.tail_append(n);
                        }
                        if self.mode() == Mode::Vertical {
                            self.maybe_build_page();
                        }
                    }
                    // In math mode the box becomes an Ord noad whose nucleus is the
                    // box; for lists of such atoms mlist_to_hlist emits it unchanged.
                    _ => self.tail_append(Node::Box(b)),
                }
                Ok(())
            }
            BoxContext::SetBox { register, global } => {
                self.set_box_register(register, b, global);
                Ok(())
            }
            BoxContext::ShipOut => {
                if let Some(b) = b {
                    self.shipped.push(b);
                }
                Ok(())
            }
            BoxContext::Leaders(kind) => {
                if let Some(b) = b {
                    self.pending_leaders = Some((kind, Node::Box(b)));
                }
                Ok(())
            }
        }
    }

    /// `\box n` in a box context (§1079).
    pub fn use_box(&mut self, n: u32, context: BoxContext) -> BoxResult {
        let b = self.take_box_register(n);
        self.box_end(b, context, Vec::new())
    }

    /// `\copy n` in a box context.
    pub fn copy_box(&mut self, n: u32, context: BoxContext) -> BoxResult {
        let b = self.box_register(n).cloned();
        self.box_end(b, context, Vec::new())
    }

    /// `\lastbox` in a box context (§§1080–1081).
    pub fn last_box(&mut self, context: BoxContext) -> BoxResult {
        let mut cur_box = None;
        match self.mode() {
            Mode::Math => return Err(BoxError::Tex("Sorry; this \\lastbox will be void.".into())),
            Mode::Vertical if self.cur().list.is_empty() => {
                return Err(BoxError::Tex("Sorry...I usually can't take things from the current page.".into()));
            }
            _ => {
                if matches!(self.cur().list.last(), Some(Node::Box(_))) && !self.tail_inside_disc() {
                    if let Some(Node::Box(mut b)) = self.cur_mut().list.pop() {
                        b.shift = 0;
                        cur_box = Some(b);
                    }
                }
            }
        }
        self.box_end(cur_box, context, Vec::new())
    }

    /// `\unhbox`, `\unhcopy`, `\unvbox`, `\unvcopy` (§§1109–1110).
    pub fn unpackage(&mut self, n: u32, copy: bool, vertical: bool) -> BoxResult {
        if vertical {
            self.ensure_vertical("unvbox")?;
        } else {
            self.ensure_horizontal();
        }
        let Some(b) = self.box_register(n) else { return Ok(()) };
        let incompatible = self.mode() == Mode::Math
            || (self.mode().is_vertical() && b.kind != ListKind::V)
            || (self.mode().is_horizontal() && b.kind != ListKind::H);
        if incompatible {
            return Err(BoxError::Tex("Incompatible list can't be unboxed".into()));
        }
        let list = if copy { b.list.clone() } else { self.take_box_register(n).map(|b| b.list).unwrap_or_default() };
        self.cur_mut().list.extend(list);
        Ok(())
    }

    // ----- math (minimal: box atoms and \vcenter) -----------------------------------

    /// `$` in horizontal mode (`init_math`, §1138). Starts a paragraph in
    /// vertical mode.
    pub fn begin_math(&mut self) -> BoxResult {
        self.ensure_horizontal();
        self.new_save_level(GroupKind::Math);
        self.push_nest(Mode::Math);
        Ok(())
    }

    /// Closing `$` (`after_math`, §§1194–1196). Supports math lists made of
    /// box atoms, `\vcenter` atoms, glue and kerns, which `mlist_to_hlist`
    /// passes through without inter-atom spacing.
    pub fn end_math(&mut self) -> BoxResult {
        self.expect_group(|g| matches!(g, GroupKind::Math), "math shift group")?;
        let surround = self.dimen("mathsurround");
        let state = self.pop_nest();
        self.tail_append(Node::Math { on: true, width: surround });
        self.cur_mut().list.extend(state.list);
        self.tail_append(Node::Math { on: false, width: surround });
        self.unsave();
        Ok(())
    }

    /// `\vcenter [to|spread <dimen>] {` in math mode (§1167).
    pub fn begin_vcenter(&mut self, spec: PackSpec) -> BoxResult {
        if self.mode() != Mode::Math {
            return Err(BoxError::Tex("Missing $ inserted".into()));
        }
        self.new_save_level(GroupKind::VCenter { spec });
        self.push_nest(Mode::InternalVertical);
        self.cur_mut().prev_depth = IGNORE_DEPTH;
        Ok(())
    }

    /// `}` closing `\vcenter` (§1168) followed by `make_vcenter` (§736).
    pub fn end_vcenter(&mut self) -> BoxResult {
        let spec = match self.saves.last().map(|f| &f.group) {
            Some(GroupKind::VCenter { spec }) => *spec,
            _ => return Err(BoxError::GroupMismatch { expected: "vcenter group" }),
        };
        self.end_graf();
        self.unsave();
        let state = self.pop_nest();
        let out = vpackage(state.list, spec, MAX_DIMEN, &self.pack_params(), self.origin(), self.metrics.as_ref());
        self.record(out.last_badness, out.report);
        let mut v = out.node;
        let delta = v.height + v.depth;
        v.height = self.axis_height + half(delta);
        v.depth = delta - v.height;
        self.tail_append(Node::Box(v));
        Ok(())
    }
}

fn default_value(key: &Key) -> Value {
    match key {
        Key::Box(_) => Value::Box(None),
        Key::Int(_) => Value::Int(0),
        Key::Dimen(_) => Value::Dimen(0),
        Key::Skip(_) => Value::Skip(GlueSpec::ZERO),
    }
}

/// Whether a skip parameter name maps to a glue parameter subtype.
pub fn is_glue_param(name: &str) -> bool {
    skip_param(name).is_some()
}

impl BoxEngine {
    /// Convenience for tests: render a transcript printer with this engine's
    /// `max_print_line`.
    pub fn printer(&self) -> Printer {
        Printer::new(self.max_print_line)
    }
}
