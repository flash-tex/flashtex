//! Building the main vertical list: `append_to_vlist` (§679), rules
//! (§1056), paragraph lines with their penalties (§880, §888–§890),
//! displays (§1199–§1206) and LaTeX's `\addvspace`/`\addpenalty`
//! (ltspace.dtx), which inspect the list tail.

use crate::node::{BoxNode, GlueKind, GlueSpec, Node};
use crate::scaled::{Scaled, IGNORE_DEPTH, INF_PENALTY};

/// `\baselineskip`, `\lineskip`, `\lineskiplimit`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterlineParams {
    pub baseline_skip: GlueSpec,
    pub line_skip: GlueSpec,
    pub line_skip_limit: Scaled,
}

/// Penalty parameters read by `post_line_break` (§890).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinePenalties {
    pub inter_line: i32,
    pub club: i32,
    /// `\widowpenalty`, or `\displaywidowpenalty` when the paragraph part
    /// is followed by a display (TeX's `final_widow_penalty`).
    pub final_widow: i32,
    pub broken: i32,
}

impl LinePenalties {
    /// Plain TeX / LaTeX kernel defaults.
    pub const LATEX: LinePenalties = LinePenalties { inter_line: 0, club: 150, final_widow: 150, broken: 100 };
}

/// One line of a paragraph as produced by the line breaker.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub hbox: BoxNode,
    /// The line ends at a discretionary break (adds `\brokenpenalty`).
    pub disc_break: bool,
    /// `\vadjust` and `\insert`/`\mark` material migrating out of the line
    /// (§889): appended after the box, before the interline penalty.
    pub adjust: Vec<Node>,
}

impl Line {
    pub fn new(hbox: BoxNode) -> Line {
        Line { hbox, disc_break: false, adjust: Vec::new() }
    }
}

/// How `\[ ... \]` is appended (§1203–§1205, without equation numbers on
/// separate lines).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Display {
    /// The equation box, with `shift` already set to `s + d`.
    pub hbox: BoxNode,
    pub pre_penalty: i32,
    pub post_penalty: i32,
    pub above: GlueSpec,
    pub below: GlueSpec,
    /// `\abovedisplayshortskip`/`\belowdisplayshortskip` were chosen
    /// (`d + s > \predisplaysize`).
    pub short: bool,
}

/// TeX's vertical-mode state: the list, `\prevdepth` and `\prevgraf`.
#[derive(Debug, Clone, PartialEq)]
pub struct VListBuilder {
    pub list: Vec<Node>,
    pub prev_depth: Scaled,
    pub prev_graf: i32,
    pub params: InterlineParams,
}

impl VListBuilder {
    pub fn new(params: InterlineParams) -> VListBuilder {
        // Vertical mode starts with prev_depth = ignore_depth (§215).
        VListBuilder { list: Vec::new(), prev_depth: IGNORE_DEPTH, prev_graf: 0, params }
    }

    /// Takes the nodes accumulated so far (for feeding the page builder)
    /// while keeping `\prevdepth`.
    pub fn drain(&mut self) -> Vec<Node> {
        std::mem::take(&mut self.list)
    }

    /// `append_to_vlist(b)` (§679).
    pub fn append_box(&mut self, b: BoxNode) {
        if self.prev_depth > IGNORE_DEPTH {
            let bs = self.params.baseline_skip;
            let d = bs.width - self.prev_depth - b.height;
            if d < self.params.line_skip_limit {
                self.list.push(Node::Glue { spec: self.params.line_skip, kind: GlueKind::LineSkip });
            } else {
                self.list.push(Node::Glue { spec: GlueSpec { width: d, ..bs }, kind: GlueKind::BaselineSkip });
            }
        }
        self.list.push(Node::Box(b));
        self.prev_depth = b.depth;
    }

    /// `\hrule` in vertical mode (§1056).
    pub fn hrule(&mut self, width: Option<Scaled>, height: Scaled, depth: Scaled) {
        self.list.push(Node::Rule { width, height, depth });
        self.prev_depth = IGNORE_DEPTH;
    }

    /// `\nointerlineskip`.
    pub fn no_interline_skip(&mut self) {
        self.prev_depth = IGNORE_DEPTH;
    }

    pub fn vskip(&mut self, spec: GlueSpec) {
        self.list.push(Node::glue(spec));
    }

    pub fn param_glue(&mut self, spec: GlueSpec, kind: GlueKind) {
        self.list.push(Node::Glue { spec, kind });
    }

    pub fn kern(&mut self, width: Scaled) {
        self.list.push(Node::kern(width));
    }

    pub fn penalty(&mut self, p: i32) {
        self.list.push(Node::Penalty(p));
    }

    pub fn push(&mut self, node: Node) {
        self.list.push(node);
    }

    /// The lines of a paragraph (or of the part before a display): §888–§890.
    /// `prev_graf` is the number of lines already set in this paragraph
    /// (0, or +3 per display, §1200).
    pub fn paragraph(&mut self, lines: Vec<Line>, pens: LinePenalties) {
        let best_line = self.prev_graf + lines.len() as i32 + 1;
        for (cur_line, line) in (self.prev_graf + 1..).zip(lines) {
            self.append_box(line.hbox);
            self.list.extend(line.adjust);
            if cur_line + 1 != best_line {
                let mut pen = pens.inter_line;
                if cur_line == self.prev_graf + 1 {
                    pen += pens.club;
                }
                if cur_line + 2 == best_line {
                    pen += pens.final_widow;
                }
                if line.disc_break {
                    pen += pens.broken;
                }
                if pen != 0 {
                    self.list.push(Node::Penalty(pen));
                }
            }
        }
        self.prev_graf = best_line - 1;
    }

    /// A display (§1203–§1205, §1200 `prev_graf + 3`).
    pub fn display(&mut self, d: Display) {
        self.list.push(Node::Penalty(d.pre_penalty));
        let (k1, k2) = if d.short {
            (GlueKind::AboveDisplayShortSkip, GlueKind::BelowDisplayShortSkip)
        } else {
            (GlueKind::AboveDisplaySkip, GlueKind::BelowDisplaySkip)
        };
        self.list.push(Node::Glue { spec: d.above, kind: k1 });
        self.append_box(d.hbox);
        self.list.push(Node::Penalty(d.post_penalty));
        self.list.push(Node::Glue { spec: d.below, kind: k2 });
        self.prev_graf += 3;
    }

    /// `\lastskip` on this list (0pt when the tail is not glue).
    pub fn last_skip(&self) -> GlueSpec {
        match self.list.last() {
            Some(Node::Glue { spec, .. }) => *spec,
            _ => GlueSpec::ZERO,
        }
    }

    /// LaTeX `\addvspace{skip}` in vertical mode outside minipages.
    pub fn addvspace(&mut self, skip: GlueSpec) {
        let last = self.last_skip();
        if last.width == 0 {
            self.vskip(skip);
            return;
        }
        // \@xaddvskip
        let mut b = skip;
        if last.width < b.width {
            self.vskip(last.negated());
            self.vskip(b);
        } else if b.width < 0 && last.width >= 0 {
            b = b.advanced_by(last);
            self.vskip(last.negated());
            self.vskip(b);
        }
    }

    /// LaTeX `\addpenalty{p}`; `nobreak` is `\if@nobreak`.
    pub fn addpenalty(&mut self, p: i32, nobreak: bool, max_depth: Scaled) {
        if nobreak {
            return;
        }
        let last = self.last_skip();
        if last.width == 0 {
            self.penalty(p);
            return;
        }
        let a = last;
        let mut b = last;
        b.width += if self.prev_depth > max_depth {
            max_depth
        } else if self.prev_depth == IGNORE_DEPTH {
            0
        } else {
            self.prev_depth
        };
        self.vskip(b.negated());
        self.penalty(p);
        if a.width != b.width {
            // \advance\@tempskipb -\@tempskipa
            let diff = b.advanced_by(a.negated());
            self.vskip(diff);
        }
        self.vskip(a);
    }

    /// `\nobreak`.
    pub fn nobreak(&mut self) {
        self.penalty(INF_PENALTY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scaled::pt;

    fn params() -> InterlineParams {
        InterlineParams {
            baseline_skip: GlueSpec::fixed(pt(12.0)),
            line_skip: GlueSpec::fixed(pt(1.0)),
            line_skip_limit: 0,
        }
    }

    #[test]
    fn baselineskip_and_lineskip() {
        let mut v = VListBuilder::new(params());
        v.append_box(BoxNode::hbox(0, pt(7.0), pt(2.0), 0));
        v.append_box(BoxNode::hbox(0, pt(6.0), pt(2.0), 1));
        v.append_box(BoxNode::hbox(0, pt(11.0), pt(0.0), 2));
        assert_eq!(v.list.len(), 5);
        assert!(matches!(v.list[1], Node::Glue { kind: GlueKind::BaselineSkip, spec } if spec.width == pt(4.0)));
        assert!(matches!(v.list[3], Node::Glue { kind: GlueKind::LineSkip, spec } if spec.width == pt(1.0)));
        v.hrule(None, pt(0.4), 0);
        v.append_box(BoxNode::hbox(0, pt(7.0), 0, 3));
        assert!(matches!(v.list[6], Node::Box(_)), "no interline glue after a rule");
    }

    #[test]
    fn paragraph_penalties_follow_post_line_break() {
        let mut v = VListBuilder::new(params());
        let lines = (0..4).map(|i| Line::new(BoxNode::hbox(0, pt(7.0), pt(2.0), i))).collect();
        v.paragraph(lines, LinePenalties::LATEX);
        let pens: Vec<i32> = v.list.iter().filter_map(|n| if let Node::Penalty(p) = n { Some(*p) } else { None }).collect();
        assert_eq!(pens, vec![150, 0, 150].into_iter().filter(|p| *p != 0).collect::<Vec<_>>());
        let mut v = VListBuilder::new(params());
        let lines = (0..2).map(|i| Line::new(BoxNode::hbox(0, pt(7.0), pt(2.0), i))).collect();
        v.paragraph(lines, LinePenalties::LATEX);
        assert!(v.list.contains(&Node::Penalty(300)));
    }

    #[test]
    fn addvspace_keeps_the_larger_skip() {
        let mut v = VListBuilder::new(params());
        v.vskip(GlueSpec::fixed(pt(6.0)));
        v.addvspace(GlueSpec::fixed(pt(10.0)));
        assert_eq!(v.list.len(), 3);
        assert_eq!(v.list[1], Node::glue(GlueSpec::fixed(pt(-6.0))));
        v.addvspace(GlueSpec::fixed(pt(4.0)));
        assert_eq!(v.list.len(), 3);
    }

    #[test]
    fn addpenalty_moves_the_penalty_before_the_skip() {
        let mut v = VListBuilder::new(params());
        v.append_box(BoxNode::hbox(0, pt(7.0), pt(2.0), 0));
        v.vskip(GlueSpec::new(pt(5.0), pt(1.0), 0));
        v.addpenalty(-300, false, pt(4.0));
        // box, 5 plus 1, -(5+2) plus -1, penalty, (7-5) plus 0, 5 plus 1.
        assert_eq!(v.list[2], Node::glue(GlueSpec::new(pt(-7.0), pt(-1.0), 0)));
        assert_eq!(v.list[3], Node::Penalty(-300));
        assert_eq!(v.list[4], Node::glue(GlueSpec::new(pt(2.0), 0, 0)));
        assert_eq!(v.list[5], Node::glue(GlueSpec::new(pt(5.0), pt(1.0), 0)));
    }
}
