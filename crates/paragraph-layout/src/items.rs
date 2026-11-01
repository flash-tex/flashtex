//! The horizontal list: boxes, glue, penalties and kerns, plus a builder that
//! turns text into items using a [`FontMetricsSource`].
//!
//! The breaker ([`crate::linebreak`]) works on `&[Item]` only. Callers that
//! already have shaped glyph runs (the font engine's output) construct
//! [`Item::Box`] directly; [`ParagraphBuilder`] is the convenience path for
//! plain text and is what the golden tests and the oracle comparison use.
//!
//! All lengths are in the caller's linear unit (points in this project).

use std::ops::Range;
use std::panic::{self, AssertUnwindSafe};

use crate::hyphenate::{Hyphenator, HyphenatorError};
use crate::metrics::{FontId, FontMetricsSource};

/// A penalty at or above this value forbids a break (TeX `\penalty10000`).
pub const INFINITE_PENALTY: i32 = 10_000;
/// A penalty at or below this value forces a break (TeX `\penalty-10000`).
pub const FORCED_BREAK: i32 = -10_000;

/// One positioned-later glyph inside a run.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    /// Original glyph id from the metrics source, passed through unchanged.
    pub gid: u32,
    /// Unkerned advance width.
    pub advance: f64,
    /// Kern applied *after* this glyph (before the next glyph of the same run).
    /// The pen moves by `advance + kern`. Always 0 on the last glyph of a run;
    /// kerns between runs are separate [`Item::Kern`]s.
    pub kern: f64,
    /// Source byte range (cluster) this glyph renders. A ligature covers the
    /// bytes of all its components; a hyphen inserted at a discretionary covers
    /// the marker bytes (possibly empty).
    pub cluster: Range<usize>,
}

/// A box: a run of glyphs in one font at one size. Unbreakable.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphRun {
    pub font: FontId,
    pub size: f64,
    pub glyphs: Vec<Glyph>,
    /// Sum of `advance + kern` over the glyphs.
    pub width: f64,
    /// Extent above the baseline: the tallest glyph box in the run
    /// ([`FontMetricsSource::glyph_height`], scaled to `size`), which is the
    /// font ascender for a metrics source without per-glyph boxes.
    pub height: f64,
    /// Extent below the baseline, positive: the deepest glyph box in the run
    /// ([`FontMetricsSource::glyph_depth`], scaled to `size`).
    pub depth: f64,
    /// Source byte range covered by the whole run.
    pub source: Range<usize>,
}

/// Infinity order of a glue component. `Finite` < `Fil` < `Fill` < `Filll`;
/// a higher order dominates all lower ones when a line is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GlueOrder {
    Finite,
    Fil,
    Fill,
    Filll,
}

/// Stretchable/shrinkable space.
#[derive(Debug, Clone, PartialEq)]
pub struct Glue {
    pub width: f64,
    pub stretch: f64,
    pub stretch_order: GlueOrder,
    pub shrink: f64,
    pub shrink_order: GlueOrder,
    /// Source bytes of the whitespace this glue came from, if any.
    pub source: Option<Range<usize>>,
}

impl Glue {
    pub const fn fixed(width: f64) -> Glue {
        Glue {
            width,
            stretch: 0.0,
            stretch_order: GlueOrder::Finite,
            shrink: 0.0,
            shrink_order: GlueOrder::Finite,
            source: None,
        }
    }

    pub const fn finite(width: f64, stretch: f64, shrink: f64) -> Glue {
        Glue {
            width,
            stretch,
            stretch_order: GlueOrder::Finite,
            shrink,
            shrink_order: GlueOrder::Finite,
            source: None,
        }
    }

    /// `0pt plus 1fil` — LaTeX's `\parfillskip` and `\raggedright` `\rightskip`.
    pub const fn fil() -> Glue {
        Glue {
            width: 0.0,
            stretch: 1.0,
            stretch_order: GlueOrder::Fil,
            shrink: 0.0,
            shrink_order: GlueOrder::Finite,
            source: None,
        }
    }
}

/// A potential break point with a cost.
#[derive(Debug, Clone, PartialEq)]
pub struct Penalty {
    /// TeX semantics: `>= 10000` never breaks, `<= -10000` always breaks.
    pub value: i32,
    /// True for hyphenation points (TeX "flagged" penalties); consecutive
    /// flagged breaks incur `double_hyphen_demerits`.
    pub flagged: bool,
    /// Material typeset at the end of the line *only if* the line breaks here
    /// (the hyphen of a discretionary). Its width counts toward that line.
    pub pre_break: Option<GlyphRun>,
    /// True when produced by an automatic hyphenator; ignored in the first
    /// (pretolerance) pass like TeX does.
    pub automatic: bool,
    /// `\discretionary` post-break text: typeset at the start of the next
    /// line *only if* the line breaks here. Its width counts toward that line.
    pub post_break: Option<GlyphRun>,
    /// `\discretionary` no-break text: the number of items directly after
    /// this penalty that are typeset only when the line does *not* break here
    /// (TeX's `replace_count`). They must not contain legal breakpoints.
    pub replace_count: usize,
}

/// Fixed horizontal displacement. Discarded at a line break; a legal break
/// point only when immediately followed by glue (TeX rule).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kern {
    pub width: f64,
}

/// One node of the horizontal list.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Box(GlyphRun),
    Glue(Glue),
    Penalty(Penalty),
    Kern(Kern),
}

impl Item {
    pub fn penalty(value: i32) -> Item {
        Item::Penalty(Penalty {
            value,
            flagged: false,
            pre_break: None,
            automatic: false,
            post_break: None,
            replace_count: 0,
        })
    }

    pub fn kern(width: f64) -> Item {
        Item::Kern(Kern { width })
    }

    pub fn is_discardable(&self) -> bool {
        !matches!(self, Item::Box(_))
    }
}

/// Builds a paragraph's horizontal list from text.
///
/// Space handling follows TeX's `\spacefactor` (`\nonfrenchspacing` by default):
/// a space after `.`, `?`, `!` gets `extra_space` added and triple stretch;
/// after `,` 1.25x stretch, `;` 1.5x, `:` 2x; a capital letter resets the
/// factor to 999 so an abbreviation like `A.` does not get sentence spacing.
/// Runs of whitespace collapse into one glue whose source range spans them all.
pub struct ParagraphBuilder<'h> {
    items: Vec<Item>,
    hyphenator: &'h dyn Hyphenator,
    pub french_spacing: bool,
    pub hyphen_penalty: i32,
    pub ex_hyphen_penalty: i32,
    space_factor: u32,
    /// Last glyph char and its font, for kerns across builder calls.
    last_char: Option<(char, FontId)>,
    /// TeX only hyphenates a word that follows glue (so never the first word
    /// of a paragraph, which follows the `\parindent` box); automatic points
    /// are dropped elsewhere. Explicit `\-` is always honoured.
    after_glue: bool,
}

impl<'h> ParagraphBuilder<'h> {
    pub fn new(hyphenator: &'h dyn Hyphenator) -> Self {
        ParagraphBuilder {
            items: Vec::new(),
            hyphenator,
            french_spacing: false,
            hyphen_penalty: 50,
            ex_hyphen_penalty: 50,
            space_factor: 1000,
            last_char: None,
            after_glue: false,
        }
    }

    /// Appends text starting at source byte `source_start`. Whitespace becomes
    /// interword glue; each maximal non-space chunk is a word.
    ///
    /// # Errors
    ///
    /// Propagates [`HyphenatorError`] from [`Self::word`] the first time the
    /// configured [`Hyphenator`] misbehaves; text already appended before
    /// that word stays in the builder.
    pub fn text(
        &mut self,
        font: &dyn FontMetricsSource,
        size: f64,
        text: &str,
        source_start: usize,
    ) -> Result<(), HyphenatorError> {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let is_ws = |b: u8| b == b' ' || b == b'\n' || b == b'\t' || b == b'\r';
            let start = i;
            if is_ws(bytes[i]) {
                while i < bytes.len() && is_ws(bytes[i]) {
                    i += 1;
                }
                self.space(font, size, source_start + start..source_start + i);
            } else {
                while i < bytes.len() && !is_ws(bytes[i]) {
                    i += 1;
                }
                // TeX only tries to hyphenate a word that directly follows
                // glue (§894): never the first word of a paragraph, nor one
                // after `\\`, a kern or a box.
                let after_glue = matches!(self.items.last(), Some(Item::Glue(_)));
                self.word_with(
                    font,
                    size,
                    &text[start..i],
                    source_start + start,
                    after_glue,
                )?;
            }
        }
        Ok(())
    }

    /// Appends one word (no whitespace), applying kerns, ligatures and the
    /// hyphenator. `\-` markers are consulted through the hyphenator.
    ///
    /// The configured [`Hyphenator`] is implemented by the caller, so it is
    /// untrusted from this crate's point of view: this call is guarded the
    /// same way [`crate::adapter::try_layout_paragraph`] guards the breaker
    /// itself, via [`std::panic::catch_unwind`]. A `hyphenate` call that
    /// panics — and a `hyphenate` call that returns normally but names a
    /// byte offset that is not a valid place to split `word` (out of range,
    /// off a UTF-8 char boundary, or out of order), which would otherwise
    /// panic a few lines below when it is used to slice `word` — both become
    /// [`HyphenatorError`] instead of a panic escaping this function.
    ///
    /// # Errors
    ///
    /// Returns [`HyphenatorError`] if the hyphenator panics or returns an
    /// invalid [`HyphenationPoint`]. Nothing is appended to the builder in
    /// that case.
    pub fn word(
        &mut self,
        font: &dyn FontMetricsSource,
        size: f64,
        word: &str,
        source_start: usize,
    ) -> Result<(), HyphenatorError> {
        self.word_with(font, size, word, source_start, true)
    }

    /// [`Self::word`] with control over automatic points: when
    /// `allow_automatic` is false, only explicit discretionaries are kept.
    fn word_with(
        &mut self,
        font: &dyn FontMetricsSource,
        size: f64,
        word: &str,
        source_start: usize,
        allow_automatic: bool,
    ) -> Result<(), HyphenatorError> {
        let hyphenator = self.hyphenator;
        let points = panic::catch_unwind(AssertUnwindSafe(|| hyphenator.hyphenate(word))).map_err(
            |payload| HyphenatorError::Panicked {
                word: word.to_string(),
                message: crate::panic_message(&*payload),
            },
        )?;

        // Validate every point before mutating `self` or slicing `word`: a
        // misbehaving-but-non-panicking implementation can still name an
        // offset that isn't a legal split point, and slicing `word` with it
        // below would panic instead of erroring.
        let mut cursor = 0usize;
        for p in &points {
            let end = p
                .offset
                .checked_add(p.marker_len)
                .filter(|&e| e <= word.len());
            let valid = end.is_some_and(|end| {
                p.offset >= cursor && word.is_char_boundary(p.offset) && word.is_char_boundary(end)
            });
            if !valid {
                return Err(HyphenatorError::InvalidPoint {
                    word: word.to_string(),
                    offset: p.offset,
                    reason: "offset is out of range, not a UTF-8 char boundary, \
                             or out of order with a previous point",
                });
            }
            cursor = end.expect("checked by `valid` above");
        }

        let mut points = points;
        if !allow_automatic {
            points.retain(|p| !p.automatic);
        }
        // An explicit hyphen character is followed by an empty discretionary
        // (TeX §1039, `\exhyphenpenalty`): a legal break with no added hyphen.
        // A run of hyphens (`--`, `---`) gets one, after the run.
        let bytes = word.as_bytes();
        let ex_hyphens: Vec<usize> = (0..bytes.len())
            .filter(|&i| bytes[i] == b'-' && (i == 0 || bytes[i - 1] != b'\\'))
            .map(|i| i + 1)
            .filter(|&at| at < bytes.len() && bytes[at] != b'-')
            .filter(|&at| !points.iter().any(|p| p.offset == at))
            .collect();
        let mut frag_start = 0;
        let mut ex_iter = ex_hyphens.into_iter().peekable();
        for p in &points {
            while let Some(at) = ex_iter.next_if(|&at| at <= p.offset && at > frag_start) {
                self.fragment(font, size, &word[frag_start..at], source_start + frag_start);
                self.explicit_hyphen_break();
                frag_start = at;
            }
            let frag = &word[frag_start..p.offset];
            self.fragment(font, size, frag, source_start + frag_start);
            // The discretionary: hyphen glyph shown only if the line breaks here.
            let mut hyphen = shape_run(font, size, "-", source_start + p.offset);
            // The hyphen's cluster is the marker bytes (empty for automatic points).
            for g in &mut hyphen.glyphs {
                g.cluster = source_start + p.offset..source_start + p.offset + p.marker_len;
            }
            hyphen.source = hyphen.glyphs[0].cluster.clone();
            let value = if p.marker_len > 0 || p.automatic {
                self.hyphen_penalty
            } else {
                self.ex_hyphen_penalty
            };
            self.items.push(Item::Penalty(Penalty {
                value,
                flagged: true,
                pre_break: Some(hyphen),
                automatic: p.automatic,
                post_break: None,
                replace_count: 0,
            }));
            frag_start = p.offset + p.marker_len;
            if p.marker_len > 0 {
                // An explicit `\-` discretionary node sits between the letters,
                // so TeX applies no font kern across it; automatic points are
                // reconstituted with the kern (§903-918).
                self.last_char = None;
            }
        }
        for at in ex_iter {
            if at > frag_start {
                self.fragment(font, size, &word[frag_start..at], source_start + frag_start);
                self.explicit_hyphen_break();
                frag_start = at;
            }
        }
        self.fragment(font, size, &word[frag_start..], source_start + frag_start);
        Ok(())
    }

    /// The empty discretionary TeX appends after an explicit hyphen.
    fn explicit_hyphen_break(&mut self) {
        self.items.push(Item::Penalty(Penalty {
            value: self.ex_hyphen_penalty,
            flagged: true,
            pre_break: None,
            automatic: false,
            post_break: None,
            replace_count: 0,
        }));
    }

    /// `\discretionary{pre}{post}{nobreak}` in one font. `pre` ends the line
    /// and `post` starts the next one if the paragraph breaks here; otherwise
    /// `nobreak` is typeset. The penalty is `\hyphenpenalty` when `pre` is
    /// nonempty, else `\exhyphenpenalty` (TeX §869). All three texts carry
    /// `source` as their span (the command's bytes).
    pub fn discretionary(
        &mut self,
        font: &dyn FontMetricsSource,
        size: f64,
        pre: &str,
        post: &str,
        no_break: &str,
        source: Range<usize>,
    ) {
        let run = |text: &str| {
            (!text.is_empty()).then(|| {
                let mut r = shape_run(font, size, text, source.start);
                for g in &mut r.glyphs {
                    g.cluster = source.clone();
                }
                r.source = source.clone();
                r
            })
        };
        let pre_break = run(pre);
        let post_break = run(post);
        let no_break_run = run(no_break);
        let value = if pre_break.is_some() {
            self.hyphen_penalty
        } else {
            self.ex_hyphen_penalty
        };
        self.items.push(Item::Penalty(Penalty {
            value,
            flagged: true,
            pre_break,
            automatic: false,
            post_break,
            replace_count: usize::from(no_break_run.is_some()),
        }));
        if let Some(r) = no_break_run {
            self.items.push(Item::Box(r));
        }
        self.last_char = None;
    }

    fn fragment(
        &mut self,
        font: &dyn FontMetricsSource,
        size: f64,
        frag: &str,
        source_start: usize,
    ) {
        if frag.is_empty() {
            return;
        }
        let first = frag.chars().next().unwrap();
        // Kern across a builder-call or discretionary boundary in the same font.
        if let Some((prev, id)) = self.last_char
            && id == font.font_id()
        {
            let k = font.kern(prev, first) * size / font.units_per_em();
            if k != 0.0 {
                self.items.push(Item::kern(k));
            }
        }
        let run = shape_run(font, size, frag, source_start);
        let last_char = frag.chars().last().unwrap();
        self.update_space_factor(frag);
        self.last_char = Some((last_char, font.font_id()));
        self.items.push(Item::Box(run));
    }

    fn update_space_factor(&mut self, frag: &str) {
        for ch in frag.chars() {
            let code = match ch {
                '.' | '?' | '!' => 3000,
                ':' => 2000,
                ';' => 1500,
                ',' => 1250,
                ')' | ']' | '\'' | '"' => 0, // keep previous factor
                c if c.is_uppercase() => 999,
                _ => 1000,
            };
            if code == 0 {
                continue;
            }
            self.space_factor = if code > 1000 && self.space_factor < 1000 {
                1000
            } else {
                code
            };
        }
    }

    /// Appends interword glue for `font`, honouring the current space factor.
    pub fn space(&mut self, font: &dyn FontMetricsSource, size: f64, source: Range<usize>) {
        let scale = size / font.units_per_em();
        let f = if self.french_spacing {
            1000
        } else {
            self.space_factor
        };
        let mut width = font.space() * scale;
        if f >= 2000 {
            width += font.extra_space() * scale;
        }
        let stretch = font.space_stretch() * scale * f64::from(f) / 1000.0;
        let shrink = font.space_shrink() * scale * 1000.0 / f64::from(f);
        self.items.push(Item::Glue(Glue {
            width,
            stretch,
            stretch_order: GlueOrder::Finite,
            shrink,
            shrink_order: GlueOrder::Finite,
            source: Some(source),
        }));
        self.space_factor = 1000;
        self.last_char = None;
        self.after_glue = true;
    }

    pub fn glue(&mut self, glue: Glue) {
        self.items.push(Item::Glue(glue));
        self.last_char = None;
        self.after_glue = true;
    }

    pub fn penalty(&mut self, value: i32) {
        self.items.push(Item::penalty(value));
        self.after_glue = false;
    }

    pub fn kern(&mut self, width: f64) {
        self.items.push(Item::kern(width));
        self.last_char = None;
        self.after_glue = false;
    }

    /// `\\`: fill the rest of the line and force a break (TeX: `\hfil\break`).
    pub fn line_break(&mut self) {
        self.items.push(Item::Glue(Glue::fil()));
        self.items.push(Item::penalty(FORCED_BREAK));
        self.last_char = None;
        self.after_glue = false;
    }

    /// Finishes the list TeX-style: trailing glue is removed, then
    /// `\penalty10000 \parfillskip \penalty-10000` is appended.
    pub fn finish(mut self, par_fill_skip: Glue) -> Vec<Item> {
        while matches!(self.items.last(), Some(Item::Glue(_))) {
            self.items.pop();
        }
        self.items.push(Item::penalty(INFINITE_PENALTY));
        self.items.push(Item::Glue(par_fill_skip));
        self.items.push(Item::penalty(FORCED_BREAK));
        self.items
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }
}

/// One already-shaped glyph as produced by a font engine (FT-018
/// `font_engine::shape::Glyph` + its cluster): original glyph id, advance in
/// font units, and the source byte range of its cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    pub gid: u32,
    /// Advance in font units, kerning already applied (pen movement).
    pub advance_units: i64,
    /// Source byte range of the cluster this glyph belongs to.
    pub cluster: Range<usize>,
}

impl GlyphRun {
    /// Builds a box from font-engine output without re-measuring anything:
    /// `advance_units` are taken as the pen movement (kerning is already in
    /// them, so [`Glyph::kern`] is 0), glyph ids and clusters pass through
    /// unchanged, and the run's height/depth come from the engine's vertical
    /// metrics. `font` is the engine's content-addressed identity.
    pub fn from_shaped(
        font: FontId,
        size: f64,
        units_per_em: f64,
        ascender_units: f64,
        descender_units: f64,
        glyphs: &[ShapedGlyph],
        source: Range<usize>,
    ) -> GlyphRun {
        let scale = size / units_per_em;
        let mut width = 0.0;
        let glyphs: Vec<Glyph> = glyphs
            .iter()
            .map(|g| {
                let advance = g.advance_units as f64 * scale;
                width += advance;
                Glyph {
                    gid: g.gid,
                    advance,
                    kern: 0.0,
                    cluster: g.cluster.clone(),
                }
            })
            .collect();
        GlyphRun {
            font,
            size,
            glyphs,
            width,
            height: ascender_units * scale,
            depth: -descender_units * scale,
            source,
        }
    }
}

/// Shapes `text` (no whitespace) into one run: ligatures, per-glyph advances,
/// intra-run kerns and byte clusters.
pub fn shape_run(
    font: &dyn FontMetricsSource,
    size: f64,
    text: &str,
    source_start: usize,
) -> GlyphRun {
    let scale = size / font.units_per_em();
    // Collect (char, byte range) with ligature substitution.
    let mut chars: Vec<(char, Range<usize>)> = Vec::new();
    for (i, ch) in text.char_indices() {
        let r = source_start + i..source_start + i + ch.len_utf8();
        if let Some((prev, prev_r)) = chars.last()
            && let Some(lig) = font.ligature(*prev, ch)
        {
            let merged = prev_r.start..r.end;
            chars.pop();
            chars.push((lig.result, merged));
            continue;
        }
        chars.push((ch, r));
    }
    let mut glyphs = Vec::with_capacity(chars.len());
    let mut width = 0.0;
    // TeX box rule: a run is as tall/deep as its tallest/deepest glyph.
    let mut height: f64 = 0.0;
    let mut depth: f64 = 0.0;
    for (idx, (ch, cluster)) in chars.iter().enumerate() {
        let advance = font.advance(*ch) * scale;
        let kern = match chars.get(idx + 1) {
            Some((next, _)) => font.kern(*ch, *next) * scale,
            None => 0.0,
        };
        width += advance + kern;
        height = height.max(font.glyph_height(*ch) * scale);
        depth = depth.max(font.glyph_depth(*ch) * scale);
        glyphs.push(Glyph {
            gid: font.glyph_id(*ch),
            advance,
            kern,
            cluster: cluster.clone(),
        });
    }
    GlyphRun {
        font: font.font_id(),
        size,
        glyphs,
        width,
        height,
        depth,
        source: source_start..source_start + text.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core14::Core14Times;
    use crate::hyphenate::{ExplicitDiscretionary, HyphenationPoint, NoHyphenation};

    #[test]
    fn shaping_applies_kerns_and_ligatures() {
        // "AV" at 1000pt: A 722 + V 722 - 135 kern = 1309.
        let run = shape_run(&Core14Times::ROMAN, 1000.0, "AV", 0);
        assert_eq!(run.width, 1309.0);
        assert_eq!(run.glyphs[0].kern, -135.0);
        // "fi" collapses into one glyph of width 556 covering bytes 0..2.
        let run = shape_run(&Core14Times::ROMAN, 1000.0, "fi", 10);
        assert_eq!(run.glyphs.len(), 1);
        assert_eq!(run.glyphs[0].cluster, 10..12);
        assert_eq!(run.width, 556.0);
        assert_eq!(run.glyphs[0].gid, 0xFB01);
    }

    #[test]
    fn shape_run_height_depth_default_to_ascender_descender() {
        // Core14Times has no per-glyph boxes, so glyph_height/glyph_depth fall
        // back to the trait defaults: every run is exactly the font's
        // ascender/descender scaled to size, regardless of which glyphs it
        // holds (Times-Roman: ascender 683, descender -217, both in 1/1000 em
        // so size 1000 leaves them unscaled).
        let run = shape_run(&Core14Times::ROMAN, 1000.0, "Ay,", 0);
        assert_eq!(run.height, 683.0);
        assert_eq!(run.depth, 217.0);
    }

    /// A metrics source with real per-glyph boxes (TFM-style `charht`/`chardp`)
    /// for a couple of test characters, to exercise the non-default path of
    /// `glyph_height`/`glyph_depth`. Everything else defers to `Core14Times`.
    struct BoxFont;

    impl FontMetricsSource for BoxFont {
        fn font_id(&self) -> FontId {
            Core14Times::ROMAN.font_id()
        }
        fn units_per_em(&self) -> f64 {
            Core14Times::ROMAN.units_per_em()
        }
        fn advance(&self, ch: char) -> f64 {
            Core14Times::ROMAN.advance(ch)
        }
        fn kern(&self, left: char, right: char) -> f64 {
            Core14Times::ROMAN.kern(left, right)
        }
        fn glyph_id(&self, ch: char) -> u32 {
            Core14Times::ROMAN.glyph_id(ch)
        }
        fn ascender(&self) -> f64 {
            Core14Times::ROMAN.ascender()
        }
        fn descender(&self) -> f64 {
            Core14Times::ROMAN.descender()
        }
        fn line_gap(&self) -> f64 {
            0.0
        }
        fn space(&self) -> f64 {
            Core14Times::ROMAN.space()
        }
        // 'b' is a tall ascender box (above the font ascender); 'y' is a deep
        // descender box (below the font descender). Every other character
        // keeps the trait default (ascender / -descender).
        fn glyph_height(&self, ch: char) -> f64 {
            if ch == 'b' { 900.0 } else { self.ascender() }
        }
        fn glyph_depth(&self, ch: char) -> f64 {
            if ch == 'y' { 500.0 } else { -self.descender() }
        }
    }

    #[test]
    fn shape_run_height_depth_track_the_tallest_deepest_glyph() {
        // height = max(glyph_height('a')=683 default, 'b'=900, 'y'=683 default) = 900.
        // depth = max(glyph_depth('a')=217 default, 'b'=217 default, 'y'=500) = 500.
        let run = shape_run(&BoxFont, 1000.0, "aby", 0);
        assert_eq!(run.height, 900.0);
        assert_eq!(run.depth, 500.0);
        // A run without the tall/deep glyphs is back to the font defaults.
        let run = shape_run(&BoxFont, 1000.0, "aa", 0);
        assert_eq!(run.height, 683.0);
        assert_eq!(run.depth, 217.0);
    }

    #[test]
    fn builder_spaces_follow_space_factor() {
        let h = NoHyphenation;
        let mut b = ParagraphBuilder::new(&h);
        b.text(&Core14Times::ROMAN, 10.0, "end. Next, one; two: A. b", 0)
            .unwrap();
        let glue: Vec<&Glue> = b
            .items()
            .iter()
            .filter_map(|i| if let Item::Glue(g) = i { Some(g) } else { None })
            .collect();
        // After "end.": width 2.5 + extra 0.6, stretch 1.5*3, shrink 0.6/3.
        assert!((glue[0].width - 3.1).abs() < 1e-12);
        assert!((glue[0].stretch - 4.5).abs() < 1e-12);
        assert!((glue[0].shrink - 0.2).abs() < 1e-12);
        // After "Next,": factor 1250.
        assert!((glue[1].stretch - 1.875).abs() < 1e-12);
        // After "one;": 1500. After "two:": 2000 (gets extra space).
        assert!((glue[2].stretch - 2.25).abs() < 1e-12);
        assert!((glue[3].width - 3.1).abs() < 1e-12);
        // After "A.": capital sets 999, then '.' -> 1000, so plain space.
        assert!((glue[4].width - 2.5).abs() < 1e-12);
        assert!((glue[4].stretch - 1.5).abs() < 1e-12);
    }

    #[test]
    fn explicit_discretionary_builds_flagged_penalty_with_hyphen() {
        let h = ExplicitDiscretionary;
        let mut b = ParagraphBuilder::new(&h);
        b.word(&Core14Times::ROMAN, 10.0, "re\\-pro", 100).unwrap();
        let items = b.items();
        assert_eq!(items.len(), 3);
        match &items[1] {
            Item::Penalty(p) => {
                assert!(p.flagged);
                assert_eq!(p.value, 50);
                let hy = p.pre_break.as_ref().unwrap();
                assert_eq!(hy.glyphs[0].cluster, 102..104);
                assert!((hy.width - 3.33).abs() < 1e-12);
            }
            other => panic!("expected penalty, got {other:?}"),
        }
        if let Item::Box(r) = &items[2] {
            assert_eq!(r.source, 104..107);
        } else {
            panic!("expected box");
        }
    }

    /// Regression for a `Hyphenator` (implemented by callers, so untrusted
    /// from this crate's point of view) that returns a byte offset landing
    /// inside a multi-byte UTF-8 character. Before the fix, `word` used that
    /// offset to slice `word` directly (`&word[frag_start..p.offset]`),
    /// which panicked past every one of this crate's own safety nets:
    /// `try_layout_paragraph`'s `catch_unwind` only wraps the later
    /// `layout_paragraph` call, not this item-building step. Now the offset
    /// is validated before any slicing happens, and misbehaviour is a typed
    /// [`HyphenatorError`] instead of a panic.
    struct BadOffsetHyphenator;

    impl Hyphenator for BadOffsetHyphenator {
        fn hyphenate(&self, _word: &str) -> Vec<HyphenationPoint> {
            // "café" is c,a,f (1 byte each) + é (2 bytes) = 5 bytes; offset 4
            // is the second byte of 'é', not a char boundary.
            vec![HyphenationPoint {
                offset: 4,
                marker_len: 0,
                automatic: true,
            }]
        }
    }

    #[test]
    fn hyphenator_bad_byte_offset_is_a_typed_error_not_a_panic() {
        let h = BadOffsetHyphenator;
        let mut b = ParagraphBuilder::new(&h);
        assert_eq!(
            b.word(&Core14Times::ROMAN, 10.0, "café", 0),
            Err(HyphenatorError::InvalidPoint {
                word: "café".to_string(),
                offset: 4,
                reason: "offset is out of range, not a UTF-8 char boundary, \
                         or out of order with a previous point",
            })
        );
        // Nothing was appended: a failed word leaves the builder untouched.
        assert!(b.items().is_empty());
    }

    /// Regression for a `Hyphenator` whose `hyphenate` implementation itself
    /// panics (as opposed to returning a bad-but-non-panicking offset, above).
    /// Both are untrusted-caller-code failure modes this crate must not let
    /// escape past its public API.
    struct PanickingHyphenator;

    impl Hyphenator for PanickingHyphenator {
        fn hyphenate(&self, word: &str) -> Vec<HyphenationPoint> {
            panic!("adversarial hyphenator blew up on {word:?}");
        }
    }

    #[test]
    fn hyphenator_panic_is_a_typed_error_not_a_panic() {
        let h = PanickingHyphenator;
        let mut b = ParagraphBuilder::new(&h);
        match b.word(&Core14Times::ROMAN, 10.0, "word", 0) {
            Err(HyphenatorError::Panicked { word, .. }) => assert_eq!(word, "word"),
            other => panic!("expected Panicked, got {other:?}"),
        }
    }
}
