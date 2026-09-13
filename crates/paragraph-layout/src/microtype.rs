//! pdfTeX character protrusion and font expansion (the LaTeX `microtype`
//! package) in the Knuth–Plass breaker and the line packer.
//!
//! The rules and their integer arithmetic are `crates/microtype`'s
//! (`flashtex_microtype::pdftex`, transcribed from `pdftex.web`); this
//! module applies them to this crate's horizontal list:
//!
//! * **Breaking** (`try_break`): a candidate line's shortfall is computed in
//!   scaled points, the line's total protrusion is added
//!   (`\pdfprotrudechars > 1`), and the result goes through
//!   [`adjust_shortfall`] with the line's font stretch/shrink
//!   (`\pdfadjustspacing > 1`) before TeX's integer badness.
//! * **Packing** (`post_line_break` + `hpack`): margin kerns for the first
//!   and last protrudable characters, the line's `font_expand_ratio`, each
//!   character's expansion and expanded width, then glue set on what is
//!   left, positioned the way `hlist_out` rounds glue.
//!
//! Everything microtype-affected is integer sp, as pdfTeX: glyph widths and
//! kerns come from the caller exactly ([`MicroGlyph`]), the other items'
//! dimensions are rounded from points once. [`crate::layout_paragraph`] is
//! not affected; this path is entered only through
//! [`layout_paragraph_microtype`](crate::linebreak::layout_paragraph_microtype).
//!
//! What pdfTeX walks node by node is mapped onto items: a [`GlyphRun`] with
//! a [`MicroRun`] is a sequence of characters (a glyph whose `code` is
//! `None` is a skipable font kern), every other box is a non-skipable
//! non-character node, penalties are skipable, a kern is skipable when it
//! has zero width or is a font kern, a glue when all of its components are
//! zero (pdfTeX's `zero_glue`). A font kern stretches only between two
//! characters of one font that sit directly next to it — never across a
//! discretionary, as `kern_stretch` checks `link(prev_char_p) = p`.

use std::rc::Rc;

pub use flashtex_microtype as mt;
use flashtex_microtype::arith::{Scaled, badness as tex_badness};
use flashtex_microtype::pdftex::{
    FontParams, ParagraphExpansion, adjust_shortfall, expanded_width, line_expand_ratio,
};

use crate::items::{Glue, GlueOrder, GlyphRun, Item};

/// One glyph of a run in pdfTeX terms, all lengths in scaled points of the
/// unexpanded font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MicroGlyph {
    /// Character code (TFM slot) in the run's font; `None` for a glyph that
    /// is not a character (a boundary kern carried as an empty glyph).
    pub code: Option<u8>,
    /// `char_width(f)(c)`: the part of the glyph's advance that expands.
    pub width: Scaled,
    /// The font kern after the glyph inside the run (`Glyph::advance` in
    /// points is `width + kern`); keeps its width under expansion.
    pub kern: Scaled,
}

/// A glyph run's pdfTeX view: its font's microtype parameters and one
/// [`MicroGlyph`] per [`GlyphRun::glyphs`] entry.
#[derive(Debug, Clone, PartialEq)]
pub struct MicroRun {
    pub params: Rc<FontParams>,
    pub glyphs: Vec<MicroGlyph>,
}

impl MicroRun {
    fn width(&self) -> i64 {
        self.glyphs.iter().map(|g| i64::from(g.width) + i64::from(g.kern)).sum()
    }

    /// `(Σ char_stretch + kern_stretch, Σ char_shrink + kern_shrink)`.
    fn font_var(&self) -> (i64, i64) {
        let p = &*self.params;
        if !p.is_expandable() {
            return (0, 0);
        }
        let (mut st, mut sh) = (0i64, 0i64);
        for (k, g) in self.glyphs.iter().enumerate() {
            let Some(c) = g.code else { continue };
            st += i64::from(p.char_stretch(c, g.width));
            sh += i64::from(p.char_shrink(c, g.width));
            // An in-run font kern between two characters of this font.
            if g.kern != 0 && self.glyphs.get(k + 1).is_some_and(|n| n.code.is_some()) {
                st += i64::from(p.kern_stretch(c, g.kern));
                sh += i64::from(p.kern_shrink(c, g.kern));
            }
        }
        (st, sh)
    }

    fn first_char(&self) -> Option<u8> {
        self.glyphs.iter().find_map(|g| g.code)
    }

    fn last_char(&self) -> Option<u8> {
        self.glyphs.iter().rev().find_map(|g| g.code)
    }
}

/// The microtype side of one horizontal-list item (indexed like the items).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MicroItem {
    /// For an [`Item::Box`] glyph run set in a microtype-configured font.
    pub run: Option<MicroRun>,
    /// For a penalty's `pre_break` / `post_break` runs.
    pub pre_break: Option<MicroRun>,
    pub post_break: Option<MicroRun>,
    /// For an [`Item::Kern`]: a font kern (pdfTeX `subtype normal`), which is
    /// skipable when looking for a protrudable character and stretches with
    /// its font when it sits directly between two characters of one font.
    pub font_kern: bool,
    /// For an [`Item::Box`] holding an inline formula: the formula's
    /// top-level hlist as pdfTeX's `mlist_to_hlist` leaves it between the
    /// `\mathon`/`\mathoff` nodes. Without it the box is one opaque
    /// non-character node.
    pub math: Option<MicroMath>,
}

/// One node of an inline formula's top-level hlist, lengths in sp.
///
/// pdfTeX's line breaker and `hpack` walk these like any other node of the
/// paragraph: character nodes add `char_stretch`/`char_shrink` and are
/// expanded, glue (`\thinmuskip`, `\medmuskip`, `\thickmuskip`) stretches
/// and shrinks with the line, and the first/last character can protrude.
/// Everything math-layout packed into a box (scripts, fractions, operators,
/// `\left...\right`, `\text`) is a [`MathNode::Box`].
#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    /// A character node. `params` is `None` for a font microtype does not
    /// configure (OML/OMS/OMX: no protrusion, no expansion).
    Char { params: Option<Rc<FontParams>>, code: u8, width: Scaled },
    /// A kern. `normal` is pdfTeX's `subtype normal` (an italic correction),
    /// which is skipable when looking for a protrudable character.
    Kern { width: Scaled, normal: bool },
    /// Math glue with finite stretch and shrink.
    Glue { width: Scaled, stretch: Scaled, shrink: Scaled },
    /// A box or rule: a non-skipable non-character node.
    Box { width: Scaled },
}

/// An inline formula as pdfTeX's nodes: see [`MathNode`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MicroMath {
    pub nodes: Vec<MathNode>,
}

/// What the protrusion search finds at one end of a formula.
enum MathEdge<'a> {
    /// A character: its font's parameters (`None`: protrudes by 0).
    Char(Option<&'a FontParams>, u8),
    /// A non-skipable non-character node ends the search.
    Stop,
    /// Only skipable nodes: the search continues past the formula.
    Through,
}

impl MicroMath {
    fn width(&self) -> i64 {
        self.nodes
            .iter()
            .map(|n| i64::from(match n {
                MathNode::Char { width, .. }
                | MathNode::Kern { width, .. }
                | MathNode::Glue { width, .. }
                | MathNode::Box { width } => *width,
            }))
            .sum()
    }

    /// `(Σ glue stretch, Σ glue shrink)`: finite order only.
    fn glue(&self) -> (i64, i64) {
        self.nodes.iter().fold((0, 0), |(st, sh), n| match n {
            MathNode::Glue { stretch, shrink, .. } => (st + i64::from(*stretch), sh + i64::from(*shrink)),
            _ => (st, sh),
        })
    }

    /// `(Σ char_stretch + kern_stretch, Σ char_shrink + kern_shrink)`: a
    /// normal kern stretches when a character of one expandable font sits
    /// directly on both sides of it (`kern_stretch`).
    fn font_var(&self) -> (i64, i64) {
        let (mut st, mut sh) = (0i64, 0i64);
        for (k, n) in self.nodes.iter().enumerate() {
            match n {
                MathNode::Char { params: Some(p), code, width } if p.is_expandable() => {
                    st += i64::from(p.char_stretch(*code, *width));
                    sh += i64::from(p.char_shrink(*code, *width));
                }
                MathNode::Kern { width, normal: true } if k > 0 => {
                    if let (Some(MathNode::Char { params: Some(l), code, .. }), Some(MathNode::Char { params: Some(r), .. })) =
                        (self.nodes.get(k - 1), self.nodes.get(k + 1))
                        && (Rc::ptr_eq(l, r) || l == r)
                        && l.is_expandable()
                    {
                        st += i64::from(l.kern_stretch(*code, *width));
                        sh += i64::from(l.kern_shrink(*code, *width));
                    }
                }
                _ => {}
            }
        }
        (st, sh)
    }

    fn edge<'a>(nodes: impl Iterator<Item = &'a MathNode>) -> MathEdge<'a> {
        for n in nodes {
            match n {
                MathNode::Char { params, code, .. } => return MathEdge::Char(params.as_deref(), *code),
                MathNode::Kern { width, normal } if *normal || *width == 0 => {}
                MathNode::Glue { width: 0, stretch: 0, shrink: 0 } => {}
                _ => return MathEdge::Stop,
            }
        }
        MathEdge::Through
    }
}

/// `\pdfprotrudechars`, `\pdfadjustspacing` and the per-item data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Microtype {
    pub protrude_chars: i32,
    pub adjust_spacing: i32,
    /// One entry per input item; missing entries count as
    /// `MicroItem::default()` (a plain item).
    pub items: Vec<MicroItem>,
}

/// What packing did to one line, for painting.
#[derive(Debug, Clone, PartialEq)]
pub struct MicroLine {
    /// `font_expand_ratio` (−1000..1000).
    pub expand_ratio: i32,
    /// Margin kerns inserted at the line start and end (sp, ≤ 0 for a
    /// protruding character), already included in the run positions.
    pub left_margin_kern: Scaled,
    pub right_margin_kern: Scaled,
    /// Per [`crate::Line::runs`] entry, per glyph: the expansion in
    /// thousandths (`(1000 + e) / 1000` is the horizontal scale pdfTeX draws
    /// the glyph with). Empty for a run without a [`MicroRun`].
    pub expansion: Vec<Vec<i32>>,
}

/// Points to scaled points, rounded once.
pub(crate) fn sp(pt: f64) -> i64 {
    (pt * 65536.0).round() as i64
}

pub(crate) fn pt(sp: i64) -> f64 {
    sp as f64 / 65536.0
}

fn order_idx(o: GlueOrder) -> usize {
    match o {
        GlueOrder::Finite => 0,
        GlueOrder::Fil => 1,
        GlueOrder::Fill => 2,
        GlueOrder::Filll => 3,
    }
}

fn zero_glue(g: &Glue) -> bool {
    g.width == 0.0 && g.stretch == 0.0 && g.shrink == 0.0
}

/// Prefix sums in sp plus the paragraph's expansion parameters.
pub(crate) struct MtCtx<'a> {
    pub(crate) items: &'a [Item],
    mt: &'a Microtype,
    empty: MicroItem,
    par: ParagraphExpansion,
    w: Vec<i64>,
    st: [Vec<i64>; 4],
    sh: Vec<i64>,
    fs: Vec<i64>,
    fk: Vec<i64>,
}

/// A usable micro run for `run`: glyph counts must agree.
fn usable<'r>(run: &GlyphRun, m: Option<&'r MicroRun>) -> Option<&'r MicroRun> {
    m.filter(|m| m.glyphs.len() == run.glyphs.len())
}

impl<'a> MtCtx<'a> {
    pub(crate) fn new(items: &'a [Item], mt: &'a Microtype) -> MtCtx<'a> {
        let n = items.len();
        let mut c = MtCtx {
            items,
            mt,
            empty: MicroItem::default(),
            par: ParagraphExpansion::default(),
            w: vec![0; n + 1],
            st: [vec![0; n + 1], vec![0; n + 1], vec![0; n + 1], vec![0; n + 1]],
            sh: vec![0; n + 1],
            fs: vec![0; n + 1],
            fk: vec![0; n + 1],
        };
        if mt.adjust_spacing > 1 {
            let mut par = ParagraphExpansion::default();
            for (i, it) in items.iter().enumerate() {
                let m = c.micro(i);
                let runs = [
                    match it {
                        Item::Box(b) => usable(b, m.run.as_ref()),
                        _ => None,
                    },
                    m.pre_break.as_ref(),
                    m.post_break.as_ref(),
                ];
                for r in runs.into_iter().flatten() {
                    // pdfTeX stops with an error on disagreeing limits; such a
                    // font simply does not set the paragraph's parameters here.
                    let _ = par.note_font(&r.params);
                }
                if let Some(m) = c.math_of(i) {
                    for n in &m.nodes {
                        if let MathNode::Char { params: Some(p), .. } = n {
                            let _ = par.note_font(p);
                        }
                    }
                }
            }
            c.par = par;
        }
        for (i, it) in items.iter().enumerate() {
            let (mut w, mut st, mut sh, mut fs, mut fk) = (0i64, [0i64; 4], 0i64, 0i64, 0i64);
            match it {
                Item::Box(_) if c.math_of(i).is_some() => {
                    let m = c.math_of(i).expect("checked");
                    w = m.width();
                    (st[0], sh) = m.glue();
                    if c.mt.adjust_spacing > 1 {
                        (fs, fk) = m.font_var();
                    }
                }
                Item::Box(b) => match usable(b, c.micro(i).run.as_ref()) {
                    Some(r) => {
                        w = r.width();
                        (fs, fk) = c.var(r);
                    }
                    None => w = sp(b.width),
                },
                Item::Glue(g) => {
                    w = sp(g.width);
                    st[order_idx(g.stretch_order)] = sp(g.stretch);
                    sh = sp(g.shrink);
                }
                Item::Kern(k) => {
                    w = sp(k.width);
                    (fs, fk) = c.kern_var(i);
                }
                Item::Penalty(_) => {}
            }
            c.w[i + 1] = c.w[i] + w;
            for (o, s) in st.iter().enumerate() {
                c.st[o][i + 1] = c.st[o][i] + s;
            }
            c.sh[i + 1] = c.sh[i] + sh;
            c.fs[i + 1] = c.fs[i] + fs;
            c.fk[i + 1] = c.fk[i] + fk;
        }
        c
    }

    fn micro(&self, i: usize) -> &MicroItem {
        self.mt.items.get(i).unwrap_or(&self.empty)
    }

    /// The formula nodes of item `i`, when it is a box that carries them and
    /// they add up to the box's width (to 0.001pt); otherwise the box stays
    /// one opaque node.
    fn math_of(&self, i: usize) -> Option<&MicroMath> {
        let Some(Item::Box(b)) = self.items.get(i) else { return None };
        self.micro(i).math.as_ref().filter(|m| b.glyphs.is_empty() && (m.width() - sp(b.width)).abs() <= 66)
    }

    fn var(&self, r: &MicroRun) -> (i64, i64) {
        if self.mt.adjust_spacing > 1 { r.font_var() } else { (0, 0) }
    }

    fn run_of(&self, i: usize) -> Option<&MicroRun> {
        match self.items.get(i) {
            Some(Item::Box(b)) => usable(b, self.micro(i).run.as_ref()),
            _ => None,
        }
    }

    /// `kern_stretch`/`kern_shrink` of the kern item `i`: a font kern with
    /// a character run directly on both sides in one expandable font.
    fn kern_var(&self, i: usize) -> (i64, i64) {
        let Some(Item::Kern(k)) = self.items.get(i) else { return (0, 0) };
        if self.mt.adjust_spacing <= 1 || !self.micro(i).font_kern || i == 0 {
            return (0, 0);
        }
        let (Some(l), Some(r)) = (self.run_of(i - 1), self.run_of(i + 1)) else { return (0, 0) };
        let (Some(lc), Some(_)) = (l.last_char(), r.first_char()) else { return (0, 0) };
        let same = Rc::ptr_eq(&l.params, &r.params) || l.params == r.params;
        if !same || !l.params.is_expandable() || l.glyphs.last().is_some_and(|g| g.code.is_none()) {
            return (0, 0);
        }
        let kw = sp(k.width) as Scaled;
        (i64::from(l.params.kern_stretch(lc, kw)), i64::from(l.params.kern_shrink(lc, kw)))
    }

    /// Whether item `i` is `cp_skipable`.
    fn skipable(&self, i: usize) -> bool {
        match &self.items[i] {
            Item::Penalty(_) => true,
            Item::Kern(k) => k.width == 0.0 || self.micro(i).font_kern,
            Item::Glue(g) => zero_glue(g),
            Item::Box(b) => b.width == 0.0 && b.height == 0.0 && b.depth == 0.0 && b.glyphs.is_empty(),
        }
    }

    /// `find_protchar_left` from item `from`: the protrudable character's
    /// (params, code), or `None` when the search ends on a non-character.
    fn left_char(&self, from: usize) -> Option<(&FontParams, u8)> {
        for i in from..self.items.len() {
            if let Some(m) = self.math_of(i) {
                match MicroMath::edge(m.nodes.iter()) {
                    MathEdge::Char(p, c) => return p.map(|p| (p, c)),
                    MathEdge::Stop => return None,
                    MathEdge::Through => continue,
                }
            }
            if let Some(r) = self.run_of(i) {
                match r.first_char() {
                    Some(c) => return Some((&r.params, c)),
                    None => continue,
                }
            }
            if !self.skipable(i) {
                return None;
            }
        }
        None
    }

    /// `find_protchar_right` over items `[lo, hi)` walking backwards.
    fn right_char(&self, lo: usize, hi: usize) -> Option<(&FontParams, u8)> {
        for i in (lo..hi).rev() {
            if let Some(m) = self.math_of(i) {
                match MicroMath::edge(m.nodes.iter().rev()) {
                    MathEdge::Char(p, c) => return p.map(|p| (p, c)),
                    MathEdge::Stop => return None,
                    MathEdge::Through => continue,
                }
            }
            if let Some(r) = self.run_of(i) {
                match r.last_char() {
                    Some(c) => return Some((&r.params, c)),
                    None => continue,
                }
            }
            if !self.skipable(i) {
                return None;
            }
        }
        None
    }

    fn left_pw(ch: Option<(&FontParams, u8)>) -> i64 {
        ch.map_or(0, |(p, c)| i64::from(p.left_protrusion(c)))
    }

    fn right_pw(ch: Option<(&FontParams, u8)>) -> i64 {
        ch.map_or(0, |(p, c)| i64::from(p.right_protrusion(c)))
    }

    /// The protrudable characters of the line `start..brk` after the break
    /// `after`: (left, right). `packing` selects `post_line_break`'s right
    /// search, which on the paragraph's last line starts before
    /// `\parfillskip` instead of at it.
    fn protrusion(
        &self,
        after: Option<usize>,
        start: usize,
        brk: usize,
        line_no: usize,
        parindent: f64,
        packing: bool,
    ) -> (i64, i64) {
        let items = self.items;
        let pre = self.micro(brk).pre_break.as_ref().filter(|_| pre_break_of(items, brk).is_some());
        let right = match pre {
            Some(r) => r.last_char().map(|c| (&*r.params, c)),
            None => {
                let terminal = packing && brk + 1 == items.len() && brk > start && matches!(items[brk - 1], Item::Glue(_));
                self.right_char(start, if terminal { brk - 1 } else { brk })
            }
        };
        let post = after.and_then(|a| self.micro(a).post_break.as_ref().filter(|_| post_break_of(items, a).is_some()));
        let left = match post {
            Some(r) => r.first_char().map(|c| (&*r.params, c)),
            // The `\parindent` box opens the first line: not skipable.
            None if line_no == 0 && after.is_none() && parindent != 0.0 => None,
            None => self.left_char(start),
        };
        (Self::left_pw(left), Self::right_pw(right))
    }
}

fn pre_break_of(items: &[Item], i: usize) -> Option<&GlyphRun> {
    match &items[i] {
        Item::Penalty(p) => p.pre_break.as_ref(),
        _ => None,
    }
}

fn post_break_of(items: &[Item], i: usize) -> Option<&GlyphRun> {
    match &items[i] {
        Item::Penalty(p) => p.post_break.as_ref(),
        _ => None,
    }
}

/// The integer measure of one candidate line.
pub(crate) struct MtMeasure {
    /// Shortfall after protrusion and the expansion rule (sp).
    pub shortfall: i64,
    pub stretch: [i64; 4],
    pub shrink: i64,
    pub badness: f64,
}

impl<'a> MtCtx<'a> {
    fn run_or_width(&self, run: &GlyphRun, m: Option<&MicroRun>) -> (i64, i64, i64) {
        match usable(run, m) {
            Some(r) => {
                let (s, k) = self.var(r);
                (r.width(), s, k)
            }
            None => (sp(run.width), 0, 0),
        }
    }

    /// `try_break`'s shortfall and badness for the line `start..brk`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn measure(
        &self,
        after: Option<usize>,
        start: usize,
        brk: usize,
        line_no: usize,
        extra_stretch: f64,
        left: &Glue,
        right: &Glue,
        parindent: f64,
        line_width: f64,
    ) -> MtMeasure {
        let items = self.items;
        let mut nat = self.w[brk] - self.w[start] + sp(left.width) + sp(right.width);
        if line_no == 0 {
            nat += sp(parindent);
        }
        let mut fs = self.fs[brk] - self.fs[start];
        let mut fk = self.fk[brk] - self.fk[start];
        if let Some(h) = pre_break_of(items, brk) {
            let (w, s, k) = self.run_or_width(h, self.micro(brk).pre_break.as_ref());
            nat += w;
            fs += s;
            fk += k;
        }
        if let Some(a) = after
            && let Some(h) = post_break_of(items, a)
        {
            let (w, s, k) = self.run_or_width(h, self.micro(a).post_break.as_ref());
            nat += w;
            fs += s;
            fk += k;
        }
        let mut stretch = [0i64; 4];
        for (o, s) in stretch.iter_mut().enumerate() {
            *s = self.st[o][brk] - self.st[o][start];
        }
        stretch[order_idx(left.stretch_order)] += sp(left.stretch);
        stretch[order_idx(right.stretch_order)] += sp(right.stretch);
        stretch[0] += sp(extra_stretch);
        let shrink = self.sh[brk] - self.sh[start] + sp(left.shrink) + sp(right.shrink);
        let mut shortfall = sp(line_width) - nat;
        if self.mt.protrude_chars > 1 {
            let (l, r) = self.protrusion(after, start, brk, line_no, parindent, false);
            shortfall += l + r;
        }
        if self.mt.adjust_spacing > 1 && shortfall != 0 && self.par.step > 0 {
            shortfall = i64::from(adjust_shortfall(clamp(shortfall), clamp(fs), clamp(fk), 0, 0, self.par));
        }
        let badness = if shortfall > 0 {
            if stretch[1] != 0 || stretch[2] != 0 || stretch[3] != 0 {
                0.0
            } else if shortfall > 7_230_584 && stretch[0] < 1_663_497 {
                crate::linebreak::INF_BAD
            } else {
                f64::from(tex_badness(clamp(shortfall), clamp(stretch[0])))
            }
        } else if -shortfall > shrink {
            crate::linebreak::AWFUL_BAD
        } else {
            f64::from(tex_badness(clamp(-shortfall), clamp(shrink)))
        };
        MtMeasure { shortfall, stretch, shrink, badness }
    }
}

fn clamp(v: i64) -> Scaled {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as Scaled
}

/// One node of a packed line.
enum PNode<'r> {
    Glue(Glue),
    Fixed(i64),
    Run { run: &'r GlyphRun, micro: Option<&'r MicroRun>, is_hyphen: bool },
    /// An inline formula's box walked node by node.
    Math { run: &'r GlyphRun, math: &'r MicroMath },
}

/// A line packed by [`MtCtx::pack`].
pub(crate) struct Packed {
    pub runs: Vec<crate::linebreak::PositionedRun>,
    pub micro: MicroLine,
    pub height: f64,
    pub depth: f64,
    pub natural: f64,
    pub set_width: f64,
    pub ratio: f64,
    /// hpack's `last_badness` over the packed (expanded) line: badness of
    /// the shortfall against finite stretch, 0 with infinite stretch,
    /// `awful_bad` when the line is wider than its shrink allows.
    pub badness: f64,
}

impl<'a> MtCtx<'a> {
    /// `post_line_break` + `hpack(…, cal_expand_ratio)` + `hlist_out`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn pack(
        &self,
        after: Option<usize>,
        start: usize,
        brk: usize,
        index: usize,
        left: &Glue,
        right: &Glue,
        parindent: f64,
        line_width: f64,
    ) -> Packed {
        let items = self.items;
        let (lpw, rpw) = if self.mt.protrude_chars > 0 {
            self.protrusion(after, start, brk, index, parindent, true)
        } else {
            (0, 0)
        };
        let mut nodes: Vec<PNode<'_>> = Vec::new();
        nodes.push(PNode::Glue(left.clone()));
        if lpw != 0 {
            nodes.push(PNode::Fixed(-lpw));
        }
        if index == 0 && parindent != 0.0 {
            nodes.push(PNode::Fixed(sp(parindent)));
        }
        if let Some(a) = after
            && let Some(h) = post_break_of(items, a)
        {
            nodes.push(PNode::Run { run: h, micro: usable(h, self.micro(a).post_break.as_ref()), is_hyphen: false });
        }
        // Index of the node each item became, for the kern neighbours.
        let mut kern_var: Vec<(usize, i64, i64)> = Vec::new();
        for i in start..brk {
            match &items[i] {
                Item::Box(b) => match self.math_of(i) {
                    Some(math) => nodes.push(PNode::Math { run: b, math }),
                    None => nodes.push(PNode::Run { run: b, micro: self.run_of(i), is_hyphen: false }),
                },
                Item::Glue(g) => nodes.push(PNode::Glue(g.clone())),
                Item::Kern(k) => {
                    let (s, sh) = if i > start && i + 1 < brk { self.kern_var(i) } else { (0, 0) };
                    kern_var.push((nodes.len(), s, sh));
                    nodes.push(PNode::Fixed(sp(k.width)));
                }
                Item::Penalty(_) => {}
            }
        }
        if let Some(h) = pre_break_of(items, brk) {
            nodes.push(PNode::Run { run: h, micro: usable(h, self.micro(brk).pre_break.as_ref()), is_hyphen: true });
        }
        if rpw != 0 {
            nodes.push(PNode::Fixed(-rpw));
        }
        nodes.push(PNode::Glue(right.clone()));

        let target = sp(line_width);
        let totals = |nodes: &[PNode<'_>], exp: &[Vec<i32>]| -> (i64, [i64; 4], i64) {
            let (mut x, mut st, mut sh) = (0i64, [0i64; 4], 0i64);
            let mut ri = 0;
            for n in nodes {
                match n {
                    PNode::Glue(g) => {
                        x += sp(g.width);
                        st[order_idx(g.stretch_order)] += sp(g.stretch);
                        sh += sp(g.shrink);
                    }
                    PNode::Fixed(w) => x += w,
                    PNode::Run { run, micro, .. } => {
                        x += match micro {
                            Some(m) => m
                                .glyphs
                                .iter()
                                .enumerate()
                                .map(|(k, g)| {
                                    let e = exp.get(ri).and_then(|v| v.get(k)).copied().unwrap_or(0);
                                    i64::from(expanded_width(g.width, e)) + i64::from(g.kern)
                                })
                                .sum(),
                            None => sp(run.width),
                        };
                        ri += 1;
                    }
                    PNode::Math { math, .. } => {
                        for (k, n) in math.nodes.iter().enumerate() {
                            match n {
                                MathNode::Char { width, .. } => {
                                    let e = exp.get(ri).and_then(|v| v.get(k)).copied().unwrap_or(0);
                                    x += i64::from(expanded_width(*width, e));
                                }
                                MathNode::Glue { width, stretch, shrink } => {
                                    x += i64::from(*width);
                                    st[0] += i64::from(*stretch);
                                    sh += i64::from(*shrink);
                                }
                                MathNode::Kern { width, .. } | MathNode::Box { width } => x += i64::from(*width),
                            }
                        }
                        ri += 1;
                    }
                }
            }
            (x, st, sh)
        };
        let top = |t: &[i64; 4]| (0..4).rev().find(|&o| t[o] != 0).unwrap_or(0);
        let mut expansion: Vec<Vec<i32>> = nodes
            .iter()
            .filter_map(|n| match n {
                PNode::Run { micro, .. } => Some(micro.map_or_else(Vec::new, |m| vec![0; m.glyphs.len()])),
                PNode::Math { math, .. } => Some(vec![0; math.nodes.len()]),
                _ => None,
            })
            .collect();
        let mut ratio = 0;
        if self.mt.adjust_spacing > 0 {
            let (x, st, _) = totals(&nodes, &expansion);
            let (mut fs, mut fk) = (0i64, 0i64);
            for n in &nodes {
                let (s, k) = match n {
                    PNode::Run { micro: Some(m), .. } => m.font_var(),
                    PNode::Math { math, .. } => math.font_var(),
                    _ => (0, 0),
                };
                fs += s;
                fk += k;
            }
            for (_, s, k) in &kern_var {
                fs += s;
                fk += k;
            }
            let excess = target - x;
            // Only finite shrink exists in this crate's glue.
            let finite = if excess > 0 { top(&st) == 0 } else { true };
            ratio = line_expand_ratio(clamp(excess), clamp(fs), clamp(fk), finite);
            if ratio != 0 {
                let mut ri = 0;
                for n in &nodes {
                    match n {
                        PNode::Run { micro, .. } => {
                            if let Some(m) = micro {
                                for (k, g) in m.glyphs.iter().enumerate() {
                                    if let Some(c) = g.code {
                                        expansion[ri][k] = m.params.char_expansion(c, ratio);
                                    }
                                }
                            }
                            ri += 1;
                        }
                        PNode::Math { math, .. } => {
                            for (k, mn) in math.nodes.iter().enumerate() {
                                if let MathNode::Char { params: Some(p), code, .. } = mn
                                    && p.is_expandable()
                                {
                                    expansion[ri][k] = p.char_expansion(*code, ratio);
                                }
                            }
                            ri += 1;
                        }
                        _ => {}
                    }
                }
            }
        }
        let (x, st, sh) = totals(&nodes, &expansion);
        let excess = target - x;
        let badness = if excess > 0 {
            if top(&st) != 0 { 0.0 } else { f64::from(tex_badness(clamp(excess), clamp(st[0]))) }
        } else if excess < 0 {
            if sh < -excess { crate::linebreak::AWFUL_BAD } else { f64::from(tex_badness(clamp(-excess), clamp(sh))) }
        } else {
            0.0
        };
        // hpack's glue setting.
        #[derive(PartialEq, Clone, Copy)]
        enum Sign {
            Normal,
            Stretching,
            Shrinking,
        }
        let (mut sign, mut order, mut set) = (Sign::Normal, 0usize, 0.0f64);
        if excess > 0 {
            order = top(&st);
            if st[order] != 0 {
                sign = Sign::Stretching;
                set = excess as f64 / st[order] as f64;
            } else {
                order = 0;
            }
        } else if excess < 0 {
            order = 0;
            if sh != 0 {
                sign = Sign::Shrinking;
                set = (-excess) as f64 / sh as f64;
            }
            if sh < -excess {
                set = 1.0;
            }
        }
        // hlist_out.
        let (mut cur_h, mut cur_g, mut cur_glue) = (0i64, 0i64, 0.0f64);
        let mut runs = Vec::new();
        let (mut height, mut depth) = (0.0f64, 0.0f64);
        let mut ri = 0;
        // hlist_out's glue: the set glue rounded from the running total.
        let mut set_glue = |cur_h: &mut i64, width: i64, stretch_order: usize, stretch: i64, shrink: i64| {
            let mut rule_wd = width - cur_g;
            match sign {
                Sign::Stretching if stretch_order == order => {
                    cur_glue += stretch as f64;
                    cur_g = (set * cur_glue).round() as i64;
                }
                Sign::Shrinking if order == 0 => {
                    cur_glue -= shrink as f64;
                    cur_g = (set * cur_glue).round() as i64;
                }
                _ => {}
            }
            rule_wd += cur_g;
            *cur_h += rule_wd;
        };
        for n in &nodes {
            match n {
                PNode::Glue(g) => set_glue(&mut cur_h, sp(g.width), order_idx(g.stretch_order), sp(g.stretch), sp(g.shrink)),
                PNode::Math { run, math } => {
                    let x0 = cur_h;
                    let mut glyphs = Vec::with_capacity(math.nodes.len());
                    for (k, mn) in math.nodes.iter().enumerate() {
                        let before = cur_h;
                        match mn {
                            MathNode::Char { width, .. } => {
                                let e = expansion[ri].get(k).copied().unwrap_or(0);
                                cur_h += i64::from(expanded_width(*width, e));
                            }
                            MathNode::Glue { width, stretch, shrink } => {
                                set_glue(&mut cur_h, i64::from(*width), 0, i64::from(*stretch), i64::from(*shrink));
                            }
                            MathNode::Kern { width, .. } | MathNode::Box { width } => cur_h += i64::from(*width),
                        }
                        // One entry per node: where it starts in the formula.
                        glyphs.push(crate::linebreak::PositionedGlyph {
                            gid: 0,
                            x_offset: pt(before - x0),
                            advance: pt(cur_h - before),
                            cluster: 0..0,
                        });
                    }
                    runs.push(crate::linebreak::PositionedRun {
                        x: pt(x0),
                        baseline_y: 0.0,
                        width: pt(cur_h - x0),
                        font: run.font,
                        size: run.size,
                        glyphs,
                        source: run.source.clone(),
                        is_hyphen: false,
                    });
                    height = height.max(run.height);
                    depth = depth.max(run.depth);
                    ri += 1;
                }
                PNode::Fixed(w) => cur_h += w,
                PNode::Run { run, micro, is_hyphen } => {
                    let x0 = cur_h;
                    let glyphs = match micro {
                        Some(m) => run
                            .glyphs
                            .iter()
                            .zip(&m.glyphs)
                            .enumerate()
                            .map(|(k, (g, mg))| {
                                let e = expansion[ri].get(k).copied().unwrap_or(0);
                                let adv = i64::from(expanded_width(mg.width, e)) + i64::from(mg.kern);
                                let pg = crate::linebreak::PositionedGlyph {
                                    gid: g.gid,
                                    x_offset: pt(cur_h - x0),
                                    advance: pt(adv),
                                    cluster: g.cluster.clone(),
                                };
                                cur_h += adv;
                                pg
                            })
                            .collect(),
                        None => {
                            let mut off = 0.0;
                            let v = run
                                .glyphs
                                .iter()
                                .map(|g| {
                                    let pg = crate::linebreak::PositionedGlyph {
                                        gid: g.gid,
                                        x_offset: off,
                                        advance: g.advance + g.kern,
                                        cluster: g.cluster.clone(),
                                    };
                                    off += g.advance + g.kern;
                                    pg
                                })
                                .collect();
                            cur_h += sp(run.width);
                            v
                        }
                    };
                    runs.push(crate::linebreak::PositionedRun {
                        x: pt(x0),
                        baseline_y: 0.0,
                        width: pt(cur_h - x0),
                        font: run.font,
                        size: run.size,
                        glyphs,
                        source: run.source.clone(),
                        is_hyphen: *is_hyphen,
                    });
                    height = height.max(run.height);
                    depth = depth.max(run.depth);
                    ri += 1;
                }
            }
        }
        let signed = match sign {
            Sign::Normal => 0.0,
            Sign::Stretching => set,
            Sign::Shrinking => -set,
        };
        Packed {
            runs,
            micro: MicroLine { expand_ratio: ratio, left_margin_kern: -lpw as Scaled, right_margin_kern: -rpw as Scaled, expansion },
            height,
            depth,
            natural: pt(x),
            // A line set exactly to the measure reports the measure itself.
            set_width: if cur_h == target { line_width } else { pt(cur_h) },
            ratio: signed,
            badness,
        }
    }
}
