//! TeX's page builder (tex.web part 45, §980–§1028): moves nodes from the
//! contribution list to the current page, tracks `\pagegoal`,
//! `\pagetotal`, stretch by order, shrink and depth, handles insertions
//! (§1008–§1010, §1018–§1022), chooses the least-cost break and fires the
//! output routine (§1012–§1026). `\tracingpages` diagnostics are reproduced
//! byte for byte.

use std::collections::{BTreeMap, VecDeque};

use crate::node::{BoxNode, GlueKind, GlueSpec, InsNode, Node, Order};
use crate::pack::{vpack_natural, vpackage, PackSpec, VBox};
use crate::scaled::{
    badness, print_scaled, x_over_n, Scaled, AWFUL_BAD, DEPLORABLE, EJECT_PENALTY, INF_BAD, INF_PENALTY, MAX_DIMEN,
};
use crate::split::{prune_page_top, vert_break};

/// Frozen-at-first-box page specifications and related parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageParams {
    /// `\vsize`.
    pub vsize: Scaled,
    /// `\maxdepth`.
    pub max_depth: Scaled,
    /// `\topskip`.
    pub top_skip: GlueSpec,
    /// `\holdinginserts`.
    pub holding_inserts: i32,
    /// `\maxdeadcycles`.
    pub max_dead_cycles: i32,
}

impl Default for PageParams {
    fn default() -> Self {
        PageParams {
            vsize: 0,
            max_depth: 0,
            top_skip: GlueSpec::ZERO,
            holding_inserts: 0,
            max_dead_cycles: 25,
        }
    }
}

/// The registers of one insertion class `n`: `\count n`, `\dimen n`,
/// `\skip n` and `\box n`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InsertClass {
    pub count: i32,
    pub dimen: Scaled,
    pub skip: GlueSpec,
    /// `\box n` (`None` = void). Output routines take it.
    pub contents: Option<VBox>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Contents {
    Empty,
    InsertsOnly,
    BoxThere,
}

/// A page-insertion record (§981).
#[derive(Debug, Clone)]
struct InsRecord {
    number: u8,
    split_up: bool,
    height: Scaled,
    /// Index into the broken insertion's list (`None` = `null`).
    broken_ptr: Option<usize>,
    /// Page index of the insertion that might split.
    broken_ins: usize,
    last_ins_ptr: Option<usize>,
    best_ins_ptr: Option<usize>,
}

/// `\topmark`, `\firstmark`, `\botmark` ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Marks {
    pub top: Option<u32>,
    pub first: Option<u32>,
    pub bot: Option<u32>,
}

/// What `fire_up` hands to the output routine.
#[derive(Debug, Clone, PartialEq)]
pub struct FiredPage {
    /// `\box255`, packed to `best_size` with `page_max_depth`.
    pub box255: VBox,
    /// `\outputpenalty`.
    pub output_penalty: i32,
    pub best_size: Scaled,
    pub page_max_depth: Scaled,
    /// `\insertpenalties` during output: the number of held-over insertions.
    pub held_inserts: i32,
    /// `page_so_far` when the page fired (`\pagegoal`, `\pagetotal`, the
    /// four stretch totals, `\pageshrink`, `\pagedepth`), as `\pageshrink`
    /// etc. read them inside the output routine.
    pub page_so_far: [Scaled; 8],
    pub marks: Marks,
}

/// What the output routine leaves behind (§1026).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OutputResult {
    /// The internal vertical list the routine built; it goes back in front
    /// of the contributions.
    pub material: Vec<Node>,
    /// A `\shipout` happened (resets `\deadcycles`), or the routine set
    /// `\deadcycles=0`.
    pub reset_dead_cycles: bool,
}

/// A user output routine.
pub trait OutputRoutine {
    fn output(&mut self, builder: &mut PageBuilder, page: FiredPage) -> OutputResult;
}

/// No `\output`: §1023 ships `\box255` as it is.
#[derive(Debug, Default)]
pub struct DefaultOutput {
    pub shipped: Vec<VBox>,
}

impl OutputRoutine for DefaultOutput {
    fn output(&mut self, _b: &mut PageBuilder, page: FiredPage) -> OutputResult {
        self.shipped.push(page.box255);
        OutputResult { material: Vec::new(), reset_dead_cycles: true }
    }
}

/// The page builder state.
#[derive(Debug, Clone)]
pub struct PageBuilder {
    pub params: PageParams,
    pub classes: BTreeMap<u8, InsertClass>,
    /// When set, `\tracingpages=1` diagnostic lines are appended.
    pub trace: Option<Vec<String>>,
    pub dead_cycles: i32,
    pub marks: Marks,
    contributions: VecDeque<Node>,
    page: Vec<Node>,
    contents: Contents,
    so_far: [Scaled; 8],
    page_max_depth: Scaled,
    best_page_break: usize,
    least_page_cost: i32,
    best_size: Scaled,
    last_glue: Option<GlueSpec>,
    last_penalty: i32,
    last_kern: Scaled,
    insert_penalties: i32,
    records: Vec<InsRecord>,
    output_active: bool,
}

const GOAL: usize = 0;
const TOTAL: usize = 1;
const SHRINK: usize = 6;
const DEPTH: usize = 7;

impl PageBuilder {
    pub fn new(params: PageParams) -> PageBuilder {
        PageBuilder {
            params,
            classes: BTreeMap::new(),
            trace: None,
            dead_cycles: 0,
            marks: Marks::default(),
            contributions: VecDeque::new(),
            page: Vec::new(),
            contents: Contents::Empty,
            so_far: [0; 8],
            page_max_depth: 0,
            best_page_break: 0,
            least_page_cost: 0,
            best_size: 0,
            last_glue: None,
            last_penalty: 0,
            last_kern: 0,
            insert_penalties: 0,
            records: Vec::new(),
            output_active: false,
        }
    }

    pub fn with_trace(mut self) -> PageBuilder {
        self.trace = Some(Vec::new());
        self
    }

    /// Appends nodes to the contribution list (TeX's vertical mode `tail`).
    pub fn contribute<I: IntoIterator<Item = Node>>(&mut self, nodes: I) {
        self.contributions.extend(nodes);
    }

    pub fn contributions(&self) -> &VecDeque<Node> {
        &self.contributions
    }

    /// The current page (or, during output, the held-over insertions).
    pub fn current_page(&self) -> &[Node] {
        &self.page
    }

    pub fn page_is_empty(&self) -> bool {
        self.page.is_empty()
    }

    pub fn output_active(&self) -> bool {
        self.output_active
    }

    /// `\pagegoal`, `\pagetotal`, `\pagestretch`, `\pagefilstretch`,
    /// `\pagefillstretch`, `\pagefilllstretch`, `\pageshrink`, `\pagedepth`.
    pub fn page_so_far(&self) -> [Scaled; 8] {
        let mut v = self.so_far;
        if self.contents == Contents::Empty {
            // §421: \pagegoal reads \maxdimen on an empty page.
            v[GOAL] = MAX_DIMEN;
        }
        v
    }

    /// Assigning `\pagegoal` etc. (§1245 `alter_page_so_far`).
    pub fn set_page_so_far(&mut self, index: usize, value: Scaled) {
        self.so_far[index] = value;
    }

    /// `\lastskip`, `\lastpenalty`, `\lastkern` when the contribution list
    /// is empty (§424).
    pub fn last_skip(&self) -> Option<GlueSpec> {
        self.last_glue
    }
    pub fn last_penalty(&self) -> i32 {
        self.last_penalty
    }
    pub fn last_kern(&self) -> Scaled {
        self.last_kern
    }

    fn trace_line(&mut self, line: String) {
        if let Some(t) = self.trace.as_mut() {
            t.push(line);
        }
    }

    /// `print_totals` (§985).
    fn totals_string(&self) -> String {
        let mut s = print_scaled(self.so_far[TOTAL]);
        for (i, unit) in [(2, ""), (3, "fil"), (4, "fill"), (5, "filll")] {
            if self.so_far[i] != 0 {
                s.push_str(" plus ");
                s.push_str(&print_scaled(self.so_far[i]));
                s.push_str(unit);
            }
        }
        if self.so_far[SHRINK] != 0 {
            s.push_str(" minus ");
            s.push_str(&print_scaled(self.so_far[SHRINK]));
        }
        s
    }

    /// `freeze_page_specs` (§987).
    fn freeze_page_specs(&mut self, s: Contents) {
        self.contents = s;
        self.so_far[GOAL] = self.params.vsize;
        self.page_max_depth = self.params.max_depth;
        self.so_far[DEPTH] = 0;
        // Indexed as in tex.web's `for k:=1 to 6 do page_so_far[k]:=0`.
        #[allow(clippy::needless_range_loop)]
        for i in 1..=6 {
            self.so_far[i] = 0;
        }
        self.least_page_cost = AWFUL_BAD;
        if self.trace.is_some() {
            let line = format!(
                "%% goal height={}, max depth={}",
                print_scaled(self.so_far[GOAL]),
                print_scaled(self.page_max_depth)
            );
            self.trace_line(line);
        }
    }

    /// `build_page` (§994). Returns `Some` when the output routine must run;
    /// call [`PageBuilder::finish_output`] afterwards.
    pub fn build_page(&mut self) -> Option<FiredPage> {
        if self.output_active {
            return None;
        }
        while let Some(p) = self.contributions.front() {
            // §996
            self.last_penalty = 0;
            self.last_kern = 0;
            self.last_glue = None;
            match p {
                Node::Glue { spec, .. } => self.last_glue = Some(*spec),
                Node::Penalty(v) => self.last_penalty = *v,
                Node::Kern { width, .. } => self.last_kern = *width,
                _ => {}
            }
            enum Act {
                Contribute,
                Update,
                Break(i32),
                Discard,
            }
            // §1000
            let act = match p {
                Node::Box(_) | Node::Rule { .. } => {
                    let (h, d) = p.height_depth();
                    if self.contents < Contents::BoxThere {
                        // §1001
                        if self.contents == Contents::Empty {
                            self.freeze_page_specs(Contents::BoxThere);
                        } else {
                            self.contents = Contents::BoxThere;
                        }
                        let mut g = self.params.top_skip;
                        g.width = if g.width > h { g.width - h } else { 0 };
                        self.contributions.push_front(Node::Glue { spec: g, kind: GlueKind::TopSkip });
                        continue;
                    }
                    // §1002
                    self.so_far[TOTAL] = self.so_far[TOTAL].wrapping_add(self.so_far[DEPTH]).wrapping_add(h);
                    self.so_far[DEPTH] = d;
                    Act::Contribute
                }
                Node::Whatsit(_) | Node::Mark(_) => Act::Contribute,
                Node::Glue { .. } => {
                    if self.contents < Contents::BoxThere {
                        Act::Discard
                    } else if self.page.last().is_some_and(Node::precedes_break) {
                        Act::Break(0)
                    } else {
                        Act::Update
                    }
                }
                Node::Kern { .. } => {
                    if self.contents < Contents::BoxThere {
                        Act::Discard
                    } else if self.contributions.len() == 1 {
                        // A kern is not contributed until its successor is known.
                        return None;
                    } else if matches!(self.contributions[1], Node::Glue { .. }) {
                        Act::Break(0)
                    } else {
                        Act::Update
                    }
                }
                Node::Penalty(v) => {
                    if self.contents < Contents::BoxThere {
                        Act::Discard
                    } else {
                        Act::Break(*v)
                    }
                }
                Node::Ins(_) => {
                    self.append_insertion();
                    Act::Contribute
                }
            };
            let act = match act {
                Act::Break(pi) => {
                    if pi < INF_PENALTY {
                        if let Some(fired) = self.consider_break(pi) {
                            return Some(fired);
                        }
                    }
                    if matches!(self.contributions.front(), Some(Node::Penalty(_))) {
                        Act::Contribute
                    } else {
                        Act::Update
                    }
                }
                other => other,
            };
            match act {
                Act::Discard => {
                    self.contributions.pop_front();
                    continue;
                }
                Act::Update => {
                    // §1004
                    match self.contributions.front() {
                        Some(Node::Glue { spec, .. }) => {
                            let q = *spec;
                            self.so_far[2 + q.stretch_order.index()] += q.stretch;
                            self.so_far[SHRINK] += q.shrink;
                            if q.shrink_order != Order::Normal && q.shrink != 0 {
                                // "Infinite glue shrinkage found on current page":
                                // TeX makes it finite and continues.
                                if let Some(Node::Glue { spec, .. }) = self.contributions.front_mut() {
                                    spec.shrink_order = Order::Normal;
                                }
                            }
                            self.so_far[TOTAL] = self.so_far[TOTAL].wrapping_add(self.so_far[DEPTH]).wrapping_add(q.width);
                        }
                        Some(Node::Kern { width, .. }) => {
                            self.so_far[TOTAL] = self.so_far[TOTAL].wrapping_add(self.so_far[DEPTH]).wrapping_add(*width);
                        }
                        _ => unreachable!(),
                    }
                    self.so_far[DEPTH] = 0;
                }
                _ => {}
            }
            // contribute: §1003
            if self.so_far[DEPTH] > self.page_max_depth {
                self.so_far[TOTAL] = self.so_far[TOTAL] + self.so_far[DEPTH] - self.page_max_depth;
                self.so_far[DEPTH] = self.page_max_depth;
            }
            // §998
            let node = self.contributions.pop_front().expect("node");
            self.page.push(node);
        }
        None
    }

    /// §1005–§1007. Returns the fired page when it is time to break.
    fn consider_break(&mut self, pi: i32) -> Option<FiredPage> {
        let goal = self.so_far[GOAL];
        let total = self.so_far[TOTAL];
        let b = if total < goal {
            if self.so_far[3] != 0 || self.so_far[4] != 0 || self.so_far[5] != 0 {
                0
            } else {
                badness(goal - total, self.so_far[2])
            }
        } else if total - goal > self.so_far[SHRINK] {
            AWFUL_BAD
        } else {
            badness(total - goal, self.so_far[SHRINK])
        };
        let mut c = if b < AWFUL_BAD {
            if pi <= EJECT_PENALTY {
                pi
            } else if b < INF_BAD {
                b + pi + self.insert_penalties
            } else {
                DEPLORABLE
            }
        } else {
            b
        };
        if self.insert_penalties >= 10000 {
            c = AWFUL_BAD;
        }
        if self.trace.is_some() {
            let star = |v: i32| if v == AWFUL_BAD { "*".to_string() } else { v.to_string() };
            let line = format!(
                "% t={} g={} b={} p={} c={}{}",
                self.totals_string(),
                print_scaled(goal),
                star(b),
                pi,
                star(c),
                if c <= self.least_page_cost { "#" } else { "" }
            );
            self.trace_line(line);
        }
        if c <= self.least_page_cost {
            self.best_page_break = self.page.len();
            self.best_size = goal;
            self.least_page_cost = c;
            for r in &mut self.records {
                r.best_ins_ptr = r.last_ins_ptr;
            }
        }
        if c == AWFUL_BAD || pi <= EJECT_PENALTY {
            return Some(self.fire_up());
        }
        None
    }

    /// §1008–§1010 for the insertion at the front of the contributions.
    fn append_insertion(&mut self) {
        if self.contents == Contents::Empty {
            self.freeze_page_specs(Contents::InsertsOnly);
        }
        let (n, ins_height, ins_depth, float_cost) = match self.contributions.front() {
            Some(Node::Ins(i)) => (i.number, i.height, i.split_max_depth, i.float_cost),
            _ => unreachable!(),
        };
        let class = self.classes.get(&n).cloned().unwrap_or_default();
        let ri = match self.records.iter().position(|r| r.number >= n) {
            Some(i) if self.records[i].number == n => i,
            pos => {
                // §1009
                let at = pos.unwrap_or(self.records.len());
                let height = class.contents.as_ref().map_or(0, |b| b.height + b.depth);
                let h = if class.count == 1000 { height } else { x_over_n(height, 1000).wrapping_mul(class.count) };
                let q = class.skip;
                self.so_far[GOAL] = self.so_far[GOAL] - h - q.width;
                self.so_far[2 + q.stretch_order.index()] += q.stretch;
                self.so_far[SHRINK] += q.shrink;
                self.records.insert(
                    at,
                    InsRecord {
                        number: n,
                        split_up: false,
                        height,
                        broken_ptr: None,
                        broken_ins: 0,
                        last_ins_ptr: None,
                        best_ins_ptr: None,
                    },
                );
                at
            }
        };
        if self.records[ri].split_up {
            self.insert_penalties += float_cost;
            return;
        }
        self.records[ri].last_ins_ptr = Some(self.page.len());
        let delta = self.so_far[GOAL] - self.so_far[TOTAL] - self.so_far[DEPTH] + self.so_far[SHRINK];
        let h = if class.count == 1000 { ins_height } else { x_over_n(ins_height, 1000).wrapping_mul(class.count) };
        if (h <= 0 || h <= delta) && ins_height.wrapping_add(self.records[ri].height) <= class.dimen {
            self.so_far[GOAL] -= h;
            self.records[ri].height += ins_height;
            return;
        }
        // §1010
        let mut w = if class.count <= 0 {
            MAX_DIMEN
        } else {
            let mut w = self.so_far[GOAL] - self.so_far[TOTAL] - self.so_far[DEPTH];
            if class.count != 1000 {
                w = x_over_n(w, class.count).wrapping_mul(1000);
            }
            w
        };
        if w > class.dimen - self.records[ri].height {
            w = class.dimen - self.records[ri].height;
        }
        let list = match self.contributions.front() {
            Some(Node::Ins(i)) => &i.list,
            _ => unreachable!(),
        };
        let vb = vert_break(list, w, ins_depth);
        let q_penalty = match list.get(vb.index) {
            None => Some(EJECT_PENALTY),
            Some(Node::Penalty(v)) => Some(*v),
            Some(_) => None,
        };
        let at_end = vb.index >= list.len();
        self.records[ri].height += vb.height_plus_depth;
        if self.trace.is_some() {
            let line = format!(
                "% split{} to {},{} p={}",
                n,
                print_scaled(w),
                print_scaled(vb.height_plus_depth),
                q_penalty.map_or("0".to_string(), |v| v.to_string())
            );
            self.trace_line(line);
        }
        let mut bhd = vb.height_plus_depth;
        if class.count != 1000 {
            bhd = x_over_n(bhd, 1000).wrapping_mul(class.count);
        }
        self.so_far[GOAL] -= bhd;
        let page_len = self.page.len();
        let r = &mut self.records[ri];
        r.split_up = true;
        r.broken_ptr = if at_end { None } else { Some(vb.index) };
        r.broken_ins = page_len;
        if let Some(p) = q_penalty {
            self.insert_penalties += p;
        }
    }

    /// `fire_up` (§1012–§1022).
    fn fire_up(&mut self) -> FiredPage {
        let best = self.best_page_break;
        // §1013
        let bp = if best < self.page.len() { self.page.get_mut(best) } else { self.contributions.front_mut() };
        let output_penalty = match bp {
            Some(Node::Penalty(v)) => {
                let o = *v;
                *v = INF_PENALTY;
                o
            }
            _ => INF_PENALTY,
        };
        if self.marks.bot.is_some() {
            self.marks.top = self.marks.bot;
            self.marks.first = None;
        }
        // §1014
        self.insert_penalties = 0;
        let holding = self.params.holding_inserts > 0;
        let mut queues: BTreeMap<u8, Vec<Node>> = BTreeMap::new();
        if !holding {
            // §1018
            for r in &self.records {
                if r.best_ins_ptr.is_some() {
                    let class = self.classes.entry(r.number).or_default();
                    let list = class.contents.take().map(|b| b.list).unwrap_or_default();
                    queues.insert(r.number, list);
                }
            }
        }
        let mut page = std::mem::take(&mut self.page);
        let rest = page.split_off(best.min(page.len()));
        let mut kept = Vec::with_capacity(page.len());
        let mut hold = Vec::new();
        for (idx, p) in page.into_iter().enumerate() {
            match p {
                Node::Ins(mut ins) if !holding => {
                    // §1020
                    let ri = self.records.iter().position(|r| r.number == ins.number).expect("record");
                    let mut wait;
                    if self.records[ri].best_ins_ptr.is_none() {
                        wait = true;
                    } else {
                        wait = false;
                        let mut material = std::mem::take(&mut ins.list);
                        if self.records[ri].best_ins_ptr == Some(idx) {
                            // §1021
                            let r = &self.records[ri];
                            if r.split_up && r.broken_ins == idx {
                                if let Some(bp) = r.broken_ptr {
                                    let tail = material.split_off(bp.min(material.len()));
                                    let pruned = prune_page_top(tail, ins.split_top_skip);
                                    if !pruned.is_empty() {
                                        let v = vpack_natural(pruned.clone());
                                        ins.height = v.height + v.depth;
                                        ins.list = pruned;
                                        wait = true;
                                    }
                                }
                            }
                            self.records[ri].best_ins_ptr = None;
                            let q = queues.entry(ins.number).or_default();
                            q.extend(material);
                            let list = std::mem::take(q);
                            self.classes.entry(ins.number).or_default().contents = Some(vpack_natural(list));
                        } else {
                            queues.entry(ins.number).or_default().extend(material);
                        }
                    }
                    // §1022
                    if wait {
                        hold.push(Node::Ins(ins));
                        self.insert_penalties += 1;
                    }
                }
                Node::Mark(m) => {
                    // §1016
                    if self.marks.first.is_none() {
                        self.marks.first = Some(m);
                    }
                    self.marks.bot = Some(m);
                    kept.push(Node::Mark(m));
                }
                other => kept.push(other),
            }
        }
        // §1017
        for n in rest.into_iter().rev() {
            self.contributions.push_front(n);
        }
        let box255 = vpackage(kept, PackSpec::Exactly(self.best_size), self.page_max_depth);
        // §991 start a new current page
        self.contents = Contents::Empty;
        self.page = hold;
        self.last_glue = None;
        self.last_penalty = 0;
        self.last_kern = 0;
        self.so_far[DEPTH] = 0;
        let page_max_depth = self.page_max_depth;
        self.page_max_depth = 0;
        self.records.clear();
        if self.marks.top.is_some() && self.marks.first.is_none() {
            self.marks.first = self.marks.top;
        }
        self.output_active = true;
        self.dead_cycles += 1;
        FiredPage {
            box255,
            output_penalty,
            best_size: self.best_size,
            page_max_depth,
            held_inserts: self.insert_penalties,
            page_so_far: self.so_far,
            marks: self.marks,
        }
    }

    /// §1026: the output routine ended with `material` as its vertical list.
    pub fn finish_output(&mut self, material: Vec<Node>) {
        self.output_active = false;
        self.insert_penalties = 0;
        let mut front = std::mem::take(&mut self.page);
        front.extend(material);
        for n in front.into_iter().rev() {
            self.contributions.push_front(n);
        }
    }

    /// Runs `build_page` and the output routine until the contributions are
    /// exhausted (or only a trailing kern waits for its successor).
    pub fn run<O: OutputRoutine>(&mut self, out: &mut O) {
        while let Some(page) = self.build_page() {
            let result = if self.dead_cycles >= self.params.max_dead_cycles {
                // §1024: "Output loop": ship box255 without the routine.
                OutputResult { material: Vec::new(), reset_dead_cycles: true }
            } else {
                out.output(self, page)
            };
            if result.reset_dead_cycles {
                self.dead_cycles = 0;
            }
            self.finish_output(result.material);
        }
    }

    /// `\end` (§1054 `its_all_over`): while anything is pending, contribute
    /// `\hbox to\hsize{}\vfill\penalty-'10000000000` and build pages.
    pub fn end<O: OutputRoutine>(&mut self, hsize: Scaled, out: &mut O) {
        self.run(out);
        let mut guard = 0;
        while !(self.page.is_empty() && self.contributions.is_empty() && self.dead_cycles == 0) {
            self.contribute([
                Node::Box(BoxNode::hbox(hsize, 0, 0, u32::MAX)),
                Node::glue(GlueSpec::fill()),
                Node::Penalty(-0x4000_0000),
            ]);
            self.run(out);
            guard += 1;
            if guard > 100 {
                break;
            }
        }
    }
}

/// Builds an insertion node the way `\insert n{...}` does (§1100).
pub fn insert_node(number: u8, list: Vec<Node>, split_top_skip: GlueSpec, split_max_depth: Scaled, floating_penalty: i32) -> Node {
    let v = vpack_natural(list);
    Node::Ins(InsNode {
        number,
        height: v.height + v.depth,
        split_max_depth,
        split_top_skip,
        float_cost: floating_penalty,
        list: v.list,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaled::pt;

    fn line(id: u32) -> Node {
        Node::Box(BoxNode::hbox(pt(300.0), pt(7.0), pt(2.0), id))
    }

    fn params() -> PageParams {
        PageParams {
            vsize: pt(100.0),
            max_depth: pt(4.0),
            top_skip: GlueSpec::fixed(pt(10.0)),
            ..PageParams::default()
        }
    }

    fn para(b: &mut Vec<Node>, first_id: u32, n: u32) {
        for i in 0..n {
            if i > 0 {
                b.push(Node::Glue { spec: GlueSpec::fixed(pt(3.0)), kind: GlueKind::BaselineSkip });
            }
            b.push(line(first_id + i));
        }
    }

    #[test]
    fn breaks_when_the_page_overflows_and_traces_like_tex() {
        let mut list = Vec::new();
        para(&mut list, 0, 12);
        let mut pb = PageBuilder::new(params()).with_trace();
        pb.contribute(list);
        let mut out = DefaultOutput::default();
        pb.end(pt(300.0), &mut out);
        // Baselines at 10, 22, ..., 94 fit: 8 lines (depth 2 -> 96 <= 100).
        let first = &out.shipped[0];
        assert_eq!(first.list.iter().filter(|n| matches!(n, Node::Box(_))).count(), 8);
        let trace = pb.trace.unwrap();
        assert_eq!(trace[0], "%% goal height=100.0, max depth=4.0");
        assert_eq!(trace[1], "% t=10.0 g=100.0 b=10000 p=0 c=100000#");
    }

    #[test]
    fn forced_penalty_ejects_and_page_top_discards_glue() {
        let mut list = Vec::new();
        para(&mut list, 0, 2);
        list.push(Node::Penalty(EJECT_PENALTY));
        list.push(Node::glue(GlueSpec::fixed(pt(20.0))));
        para(&mut list, 10, 1);
        let mut pb = PageBuilder::new(params());
        pb.contribute(list);
        let mut out = DefaultOutput::default();
        pb.end(pt(300.0), &mut out);
        assert_eq!(out.shipped.len(), 2);
        let second = &out.shipped[1];
        assert!(matches!(second.list[0], Node::Glue { kind: GlueKind::TopSkip, spec } if spec.width == pt(3.0)));
        assert!(matches!(second.list[1], Node::Box(b) if b.id == 10));
    }

    #[test]
    fn footnote_insertion_reduces_the_goal() {
        let mut pb = PageBuilder::new(params());
        pb.classes.insert(
            254,
            InsertClass { count: 1000, dimen: pt(1000.0), skip: GlueSpec::fixed(pt(12.0)), contents: None },
        );
        let mut list = Vec::new();
        para(&mut list, 0, 1);
        list.push(insert_node(254, vec![line(100), Node::glue(GlueSpec::fixed(pt(3.0))), line(101)], GlueSpec::fixed(pt(10.0)), MAX_DIMEN, 0));
        let mut rest = Vec::new();
        para(&mut rest, 1, 11);
        list.push(Node::Glue { spec: GlueSpec::fixed(pt(3.0)), kind: GlueKind::BaselineSkip });
        list.extend(rest);
        pb.contribute(list);
        let mut out = DefaultOutput::default();
        pb.run(&mut out);
        // goal = 100 - 12 (skip) - 21 (7+2+3+7+2) = 67: baselines 10..58 -> 5 lines.
        assert_eq!(out.shipped[0].list.iter().filter(|n| matches!(n, Node::Box(_))).count(), 5);
        let ins = pb.classes[&254].contents.as_ref().expect("footnote box");
        assert_eq!(ins.height + ins.depth, pt(21.0));
    }
}
