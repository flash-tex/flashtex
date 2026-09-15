//! Footnotes: `\footnote`, `\footnotemark`, `\footnotetext` (latex.ltx
//! `ltfloat`/`ltoutput`, TeX Live 2026) with article/report/book's
//! `\@makefntext` and `\footnoterule` and `size1x.clo`'s dimensions.
//!
//! * The mark is `\@makefnmark`, `\hbox{\@textsuperscript{\normalfont
//!   \@thefnmark}}`: the number in the `\sf@size` of the current size
//!   (`\DeclareMathSizes`), raised as a text-style superscript of an empty
//!   nucleus (TeX §758: `sup2` of the symbol font at the current size, at
//!   least the box depth plus 4/5 of its x-height), `\scriptspace` wider.
//!   `\@footnotemark` puts `\nobreak` before it and keeps the space factor.
//! * The note is `\insert\footins{\reset@font\footnotesize ...
//!   \@makefntext{\rule\z@\footnotesep\ignorespaces #1\@finalstrut
//!   \strutbox}}`: `\noindent\hb@xt@1.8em{\hss\@makefnmark}` then the text,
//!   `\columnwidth` wide, first line at least `\footnotesep` high, last line
//!   at least `\dp\strutbox` deep, `\interlinepenalty` 100
//!   (`\interfootnotelinepenalty`) plus the club and widow penalties.
//! * The `\insert` migrates out of the line that holds the command (TeX
//!   §655) and is charged against the page goal by
//!   [`pagebuild::break_pages_inserts`], which also splits and holds over
//!   notes and sets `\skip\footins`, `\footnoterule` (`\kern-3pt \hrule
//!   width .4\columnwidth \kern2.6pt`) and the notes at the column foot.
//!
//! With floats, [`crate::typeset::floatpage::paginate`] charges the same
//! insertions against `\@colroom` and sets the notes between the text and
//! the bottom floats.
//!
//! `\thanks` notes (symbol marks from the compiler) are anchored after the
//! title's last line with `\rlap`ped marks (see `Context::title_blocks`).
//!
//! Not here yet: `minipage` footnotes (`\@mpfootnotetext`, alph marks, set
//! at the minipage's end; see [`MinipageNotes`] for the hook) and footnotes
//! in headings, captions and table cells (reported).

use flashtex_compiler::Span;
use flashtex_paragraph_layout as pl;

use crate::adapter::{self, Item as AItem, ParaStyle, TextStyle};
use crate::display::Diagnostic;
use crate::pagebuild::{self, Insertions, InsertArea, PageParams, Placed, VBlock, VItem};
use crate::style::Stylesheet;

use super::{broken_of, drop_trailing_break, line_extents, vskips_of, BoxRec, BuiltBlock, Context, CLUB_PENALTY, WIDOW_PENALTY};

/// `\scriptspace` (plain TeX and LaTeX: 0.5pt).
pub const SCRIPT_SPACE: f64 = 0.5;
/// `\interfootnotelinepenalty` (latex.ltx: 100).
const INTERFOOTNOTE_LINE_PENALTY: i32 = 100;
/// `\dimen\footins` (latex.ltx: 8in).
const FOOTINS_MAX: f64 = 8.0 * 72.27;
/// `\floatingpenalty` in `\@footnotetext` (`\@MM`).
const FLOATING_PENALTY: i32 = 20_000;
/// `\footnoterule` (article.cls): `\kern-3\p@ \hrule\@width.4\columnwidth
/// \kern2.6\p@`, the rule 0.4pt high.
const RULE: (f64, f64, f64) = (-3.0, 0.4, 2.6);
const RULE_WIDTH_FRACTION: f64 = 0.4;

/// A note's text waiting for the page builder.
#[derive(Debug, Clone)]
pub struct NoteSrc {
    pub number: String,
    pub span: Span,
    pub items: Vec<AItem>,
}

/// The class's footnote dimensions (`size10/11/12.clo`).
#[derive(Debug, Clone, Copy)]
pub struct FootnoteParams {
    /// `\footnotesize` and its `\baselineskip`.
    pub size: f64,
    pub baselineskip: f64,
    /// `\footnotesep`.
    pub sep: f64,
    /// `\skip\footins`.
    pub skip: (f64, f64, f64),
}

impl FootnoteParams {
    pub fn of(style: &Stylesheet) -> FootnoteParams {
        use flashtex_document_style::BaseSize;
        match style.base {
            BaseSize::Pt10 => FootnoteParams { size: 8.0, baselineskip: 9.5, sep: 6.65, skip: (9.0, 4.0, 2.0) },
            BaseSize::Pt11 => FootnoteParams { size: 9.0, baselineskip: 11.0, sep: 7.7, skip: (10.0, 4.0, 2.0) },
            BaseSize::Pt12 => FootnoteParams { size: 10.0, baselineskip: 12.0, sep: 8.4, skip: (10.8, 4.0, 2.0) },
        }
    }

    /// `\dp\strutbox` at `\footnotesize` (`.3\baselineskip`).
    pub fn strut_depth(self) -> f64 {
        0.3 * self.baselineskip
    }
}

/// `\sf@size` for a text size: fontmath.ltx's `\DeclareMathSizes` table
/// (the nearest entry; 70% of other sizes).
pub fn script_size(size: f64) -> f64 {
    const TABLE: [(f64, f64); 12] = [
        (5.0, 5.0),
        (6.0, 5.0),
        (7.0, 5.0),
        (8.0, 6.0),
        (9.0, 6.0),
        (10.0, 7.0),
        (10.95, 8.0),
        (12.0, 8.0),
        (14.4, 10.0),
        (17.28, 12.0),
        (20.74, 14.4),
        (24.88, 20.74),
    ];
    TABLE.iter().find(|(t, _)| (t - size).abs() < 0.01).map_or(0.7 * size, |(_, s)| *s)
}

/// TS1 metrics (`ts1-lmr<design>.tfm`, TeX Live 2026: width, height and
/// depth in em) of the `\@fnsymbol` characters, by design size.
#[rustfmt::skip]
const TS1_SYMBOLS: [(f64, [(char, f64, f64, f64); 6]); 8] = [
    (5.0, [('\u{2217}', 0.680399, 0.469, 0.0), ('\u{2020}', 0.666666, 0.688891, 0.194446), ('\u{2021}', 0.666666, 0.688891, 0.194446), ('\u{2016}', 0.645999, 0.500501, 0.275), ('\u{a7}', 0.666501, 0.688891, 0.194446), ('\u{b6}', 0.875, 0.688891, 0.194446)]),
    (6.0, [('\u{2217}', 0.611112, 0.468501, 0.0), ('\u{2020}', 0.574084, 0.688889, 0.194446), ('\u{2021}', 0.574084, 0.688889, 0.194446), ('\u{2016}', 0.578667, 0.4925, 0.268001), ('\u{a7}', 0.598, 0.688889, 0.194446), ('\u{b6}', 0.768498, 0.688889, 0.194446)]),
    (7.0, [('\u{2217}', 0.5694275, 0.474498, 0.0), ('\u{2020}', 0.52381, 0.68888, 0.194445), ('\u{2021}', 0.52381, 0.68888, 0.194445), ('\u{2016}', 0.538713, 0.492499, 0.262), ('\u{a7}', 0.560499, 0.68888, 0.194445), ('\u{b6}', 0.708333, 0.68888, 0.194445)]),
    (8.0, [('\u{2217}', 0.531124, 0.474998, 0.0), ('\u{2020}', 0.47225, 0.694437, 0.194445), ('\u{2021}', 0.47225, 0.694437, 0.194445), ('\u{2016}', 0.501751, 0.494999, 0.257999), ('\u{a7}', 0.520124, 0.694437, 0.194445), ('\u{b6}', 0.649313, 0.694437, 0.194437)]),
    (9.0, [('\u{2217}', 0.513777, 0.471, 0.0), ('\u{2020}', 0.456777, 0.694445, 0.194445), ('\u{2021}', 0.456777, 0.694445, 0.194445), ('\u{2016}', 0.4853325, 0.492499, 0.2560005), ('\u{a7}', 0.4985, 0.694445, 0.194445), ('\u{b6}', 0.628111, 0.694445, 0.194445)]),
    (10.0, [('\u{2217}', 0.5, 0.467999, 0.0), ('\u{2020}', 0.44445, 0.69445, 0.194443), ('\u{2021}', 0.44445, 0.69445, 0.194443), ('\u{2016}', 0.4722, 0.492999, 0.256), ('\u{a7}', 0.483999, 0.69445, 0.194443), ('\u{b6}', 0.611099, 0.69445, 0.194443)]),
    (12.0, [('\u{2217}', 0.489459, 0.469499, 0.0), ('\u{2020}', 0.444444, 0.694416, 0.194444), ('\u{2021}', 0.444444, 0.694416, 0.194444), ('\u{2016}', 0.462375, 0.4915, 0.252), ('\u{a7}', 0.474625, 0.694416, 0.194444), ('\u{b6}', 0.611083, 0.694416, 0.194444)]),
    (17.0, [('\u{2217}', 0.469763, 0.473495, 0.0), ('\u{2020}', 0.444444, 0.688831, 0.2160015), ('\u{2021}', 0.444444, 0.688831, 0.194502), ('\u{2016}', 0.432494, 0.5, 0.246007), ('\u{a7}', 0.460764, 0.688831, 0.194502), ('\u{b6}', 0.611111, 0.688831, 0.194502)]),
];

/// The box (width, height, depth in points) of a `\@fnsymbol` mark at
/// `size` in TS1 Latin Modern (`ts1lmr.fd`'s design size for `size`);
/// `None` when `text` is not made of those symbols. The T1 metrics the
/// text path uses have no such characters.
pub fn ts1_mark_box(text: &str, size: f64) -> Option<(f64, f64, f64)> {
    let design = match size {
        s if s < 5.5 => 5.0,
        s if s < 6.5 => 6.0,
        s if s < 7.5 => 7.0,
        s if s < 8.5 => 8.0,
        s if s < 9.5 => 9.0,
        s if s < 11.0 => 10.0,
        s if s < 15.0 => 12.0,
        _ => 17.0,
    };
    let (_, table) = TS1_SYMBOLS.iter().find(|(d, _)| *d == design)?;
    let (mut w, mut h, mut d) = (0.0, 0.0f64, 0.0f64);
    for c in text.chars() {
        let &(_, cw, ch, cd) = table.iter().find(|(x, ..)| *x == c)?;
        w += cw * size;
        h = h.max(ch * size);
        d = d.max(cd * size);
    }
    (!text.is_empty()).then_some((w, h, d))
}

/// `sup2` (fontdimen 14) of the Latin Modern symbol font LaTeX selects at
/// `size` (`omslmsy.fd`: lmsy5..lmsy10 by size), in points.
pub fn sup2_pt(size: f64) -> f64 {
    let ratio = match size {
        s if s < 5.5 => 0.403555,
        s if s < 6.5 => 0.419635,
        s if s < 7.5 => 0.431115,
        s if s < 8.5 => 0.352917,
        s if s < 9.5 => 0.424811,
        _ => 0.362892,
    };
    ratio * size
}

/// cmr x-height per em, the same constant the compiler's
/// `CMR_EX_PER_EM` carries (4.30554pt at 10pt).
const CMR_EX_PER_EM: f64 = 0.430555;

/// `\@textsuperscript` raise for text of `size` whose superscript box has
/// depth `content_depth`: TeX §758 with an empty nucleus, exactly the
/// shift [`Context::footnote_mark`] applies — `sup2` of the symbol font
/// at the current size, at least the box depth plus a quarter of the
/// x-height (Appendix G rule 18c).
pub fn textsup_shift_pt(size: f64, content_depth: f64) -> f64 {
    sup2_pt(size).max(content_depth + 0.25 * CMR_EX_PER_EM * size)
}

/// `\@textsubscript` drop for text of `size` set at `sf` whose subscript
/// box has height `content_height`: Appendix G rule 18 with an empty
/// nucleus, the same formula the `\LaTeX`e epsilon uses — `sub_drop` at
/// the script size, `sub1` at the current size, and the box height less
/// four-fifths of the x-height. `sub1` is lmsy10's .15em and `sub_drop`
/// .05em (`params::math_params`, level 0).
pub fn textsub_shift_pt(size: f64, sf: f64, content_height: f64) -> f64 {
    (0.05 * sf).max(0.15 * size).max(content_height - 0.8 * CMR_EX_PER_EM * size)
}

/// Hook for `minipage` footnotes (`\@mpfootnotetext`): a minipage collects
/// its notes (marks `\thempfootnote`, `{\itshape\@alph\c@mpfootnote}`) and
/// sets them at its own foot, `\vskip\skip\@mpfootins \footnoterule
/// \unvbox\@mpfootins`, before `\@iiiparbox` boxes it. The box layout
/// calls [`Context::minipage_notes`] with the notes in order and the
/// minipage's `\hsize`, and appends the returned lines after its body.
#[derive(Debug, Clone, Default)]
pub struct MinipageNotes {
    /// `\skip\@mpfootins` (= `\skip\footins`) above the rule, the rule's
    /// top (from the end of the body) and the note lines: blocks appended
    /// to the context's block list, baselines from the end of the body.
    pub rule_top: f64,
    pub lines: Vec<Placed>,
    /// Height the notes add below the body's last baseline (to the last
    /// note's baseline) and the depth of the last note line.
    pub height: f64,
    pub depth: f64,
}

impl<'a> Context<'a> {
    /// `\@makefnmark` for `number` in text of `size`: the raised script-size
    /// box, `\scriptspace` included in its width.
    pub(super) fn footnote_mark(&mut self, number: &str, span: Span, size: f64) -> Option<(pl::GlyphRun, usize)> {
        let sf = script_size(size);
        let seg = adapter::Segment {
            text: number.to_string(),
            chars: number.chars().map(|_| adapter::CharSrc { document: span.document, start: span.start, end: span.end }).collect(),
            style: TextStyle { size_cpt: (sf * 100.0).round() as u16, ..TextStyle::default() },
        };
        let (mut run, rec) = self.text_box(&seg, sf)?;
        if let Some((w, h, d)) = ts1_mark_box(number, sf) {
            run.width = w;
            run.height = h;
            run.depth = d;
            if let BoxRec::Text { height, depth, .. } = &mut self.recs[rec] {
                *height = h;
                *depth = d;
            }
        }
        // §758 with an empty nucleus (height 0): shift_up = sup2, at least
        // the superscript's depth plus |math_x_height| / 4 (Appendix G
        // rule 18c).
        let x_height = 0.430555 * size;
        let shift = sup2_pt(size).max(run.depth + 0.25 * x_height);
        if let BoxRec::Text { face, glyphs, height, depth, .. } = &mut self.recs[rec] {
            let units = (shift * f64::from(face.units_per_em) / sf).round() as i32;
            for g in glyphs.iter_mut() {
                g.y_offset_units += units;
            }
            *height += shift;
            *depth = (*depth - shift).max(0.0);
        }
        run.height += shift;
        run.depth = (run.depth - shift).max(0.0);
        // §756: `\scriptspace` is part of the superscript box (never a
        // separate, discardable kern).
        run.width += SCRIPT_SPACE;
        Some((run, rec))
    }

    /// The note of `self.notes[n]` set as `\@footnotetext` sets it (see the
    /// module docs), `width` wide.
    pub(super) fn footnote_block(&mut self, n: usize, width: f64) -> Option<BuiltBlock> {
        let fp = FootnoteParams::of(self.style);
        let note = self.notes.get(n)?.clone();
        let (mut list, mut recs, labels, mut skips) = self.hlist(&note.items, fp.size, TextStyle::default(), ParaStyle::Plain);
        let _ = drop_trailing_break(&mut list, &mut recs, &mut skips, ParaStyle::Plain);
        // `\noindent\hb@xt@1.8em{\hss\@makefnmark}`.
        let em = self.text_params(TextStyle::default(), fp.size).quad;
        let mut lead: Vec<(pl::Item, Option<usize>)> = Vec::new();
        match self.footnote_mark(&note.number, note.span, fp.size) {
            Some((run, rec)) => {
                lead.push((pl::Item::kern(1.8 * em - run.width), None));
                lead.push((pl::Item::Box(run), Some(rec)));
            }
            None => lead.push((pl::Item::kern(1.8 * em), None)),
        }
        let shift = lead.len();
        for (i, (item, rec)) in lead.into_iter().enumerate() {
            list.insert(i, item);
            recs.insert(i, rec);
        }
        for (at, _) in &mut skips {
            *at += shift;
        }
        let mut params = self.line_params(false, fp.baselineskip, ParaStyle::Plain, 0.0);
        params.line_width = width;
        let lines = self.break_paragraph(&list, &params, &note.items, Some(&recs))?;
        self.report_overfull(&lines, &list, &recs);
        let mut extents = line_extents(&lines);
        if let Some(first) = extents.first_mut() {
            first.0 = first.0.max(fp.sep);
        }
        if let Some(last) = extents.last_mut() {
            last.1 = last.1.max(fp.strut_depth());
        }
        let vertical = VBlock {
            lines: extents,
            penalty_before: None,
            space_before: None,
            parskip: None,
            interline_penalty: INTERFOOTNOTE_LINE_PENALTY,
            club_penalty: CLUB_PENALTY,
            widow_penalty: WIDOW_PENALTY,
            penalty_after: None,
            space_after: None,
            no_interline_first: false,
            no_interline_after: false,
            baselineskip: Some(fp.baselineskip),
            vskip_after: vskips_of(&lines, &skips),
            broken_penalty: broken_of(&lines),
            pre_space_after: None,
            lineskip: None,
            contributed: None,
            line_penalty: Vec::new(),
            depth_after: pagebuild::DepthAfter::default(),
        };
        Some(BuiltBlock { block: pl::ParagraphBlock::body(lines), items: list, recs, vertical, labels, cache_key: None })
    }

    /// The notes of a `minipage` `width` wide (see [`MinipageNotes`]): the
    /// note blocks are appended to `blocks`.
    pub fn minipage_notes(&mut self, blocks: &mut Vec<BuiltBlock>, notes: &[usize], width: f64, page: &PageParams) -> Option<MinipageNotes> {
        let fp = FootnoteParams::of(self.style);
        let mut list: Vec<VItem> = Vec::new();
        for &n in notes {
            let Some(b) = self.footnote_block(n, width) else { continue };
            let bi = blocks.len();
            list.extend(note_vlist(page, &fp, &b, bi));
            blocks.push(b);
        }
        if list.is_empty() {
            return None;
        }
        let mut y = fp.skip.0;
        let rule_top = y + RULE.0;
        y += RULE.0 + RULE.1 + RULE.2;
        let (mut d, mut lines) = (0.0, Vec::new());
        for v in &list {
            match v {
                VItem::Box { height, depth, payload } => {
                    y += d + height;
                    d = *depth;
                    lines.push(Placed { payload: *payload, baseline: y, height: *height, depth: *depth });
                }
                VItem::Glue { width, .. } => {
                    y += d + width;
                    d = 0.0;
                }
                VItem::Penalty(_) => {}
            }
        }
        Some(MinipageNotes { rule_top, lines, height: y, depth: d })
    }
}

/// A note block's vertical list (`\insert` contents) with line payloads in
/// block `bi`.
fn note_vlist(page: &PageParams, fp: &FootnoteParams, b: &BuiltBlock, bi: usize) -> Vec<VItem> {
    let p = PageParams { baselineskip: fp.baselineskip, ..*page };
    let mut v = pagebuild::vlist(&p, std::slice::from_ref(&b.vertical));
    for item in &mut v {
        if let VItem::Box { payload, .. } = item {
            payload.0 = bi;
        }
    }
    v
}

/// Sets every anchored note and returns the insertion class for the page
/// builder, or `None` when the document has no footnotes. Marks whose line
/// is not in the body's vertical list (headings, captions, cells) are
/// reported and their notes dropped.
pub(super) fn prepare(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, page: &PageParams) -> Option<Insertions> {
    let anchors = std::mem::take(&mut ctx.note_anchors);
    if anchors.is_empty() {
        return None;
    }
    let mut line_of: std::collections::HashMap<usize, (usize, usize)> = std::collections::HashMap::new();
    for (bi, b) in blocks.iter().enumerate() {
        for (li, l) in b.block.lines.lines.iter().enumerate() {
            for it in l.items.clone() {
                if let Some(Some(r)) = b.recs.get(it) {
                    line_of.entry(*r).or_insert((bi, li));
                }
            }
        }
    }
    let fp = FootnoteParams::of(ctx.style);
    let mut ins = Insertions {
        skip: fp.skip,
        max: FOOTINS_MAX,
        split_top_skip: fp.sep,
        split_max_depth: fp.strut_depth(),
        floating_penalty: FLOATING_PENALTY,
        rule: RULE,
        ..Insertions::default()
    };
    let width = ctx.style.text_width_pt;
    let mut seen = std::collections::HashSet::new();
    for (rec, n) in anchors {
        if !seen.insert(n) {
            continue;
        }
        let Some(&(bi, li)) = line_of.get(&rec) else {
            let span = ctx.notes[n].span;
            let src = vec![ctx.source(span)];
            ctx.diagnostics.push(Diagnostic::warning(
                "unsupported_block",
                format!("footnote {} is not in a body paragraph (heading, caption or table cell): its text is not set", ctx.notes[n].number),
                src,
            ));
            continue;
        };
        let Some(b) = ctx.footnote_block(n, width) else { continue };
        let nb = blocks.len();
        let v = note_vlist(page, &fp, &b, nb);
        blocks.push(b);
        ins.after.entry((bi, li)).or_default().push(ins.notes.len());
        ins.notes.push(v);
    }
    // Notes inside note text are not set (the compiler diagnoses them).
    ctx.note_anchors.clear();
    (!ins.notes.is_empty()).then_some(ins)
}

/// Adds each column's `\footnoterule` (a rule block per column) and note
/// lines to the built pages.
pub(super) fn place(ctx: &mut Context, blocks: &mut Vec<BuiltBlock>, pages: &mut [pagebuild::BuiltPage], areas: Vec<Option<InsertArea>>) {
    let width = RULE_WIDTH_FRACTION * ctx.style.text_width_pt;
    for (page, area) in pages.iter_mut().zip(areas) {
        let Some(area) = area else { continue };
        let span = area.lines.first().and_then(|l| blocks.get(l.payload.0)).and_then(|b| b.recs.iter().flatten().next().copied()).and_then(|r| match &ctx.recs[r] {
            BoxRec::Text { clusters, .. } => clusters.first().map(|c| c.span),
            _ => None,
        });
        let rule = ctx.rule_block_sized(span.unwrap_or(Span::new(0, 0)), width, RULE.1, 0.0);
        let rb = blocks.len();
        blocks.push(rule);
        page.lines.push(Placed { payload: (rb, 0), baseline: area.rule_top + RULE.1, height: RULE.1, depth: 0.0 });
        page.lines.extend(area.lines);
    }
}
