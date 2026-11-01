//! TeX-oriented horizontal shaping: characters to glyphs with clusters that
//! map back to byte ranges of the input, plus explicit missing-glyph and
//! unsupported-feature reporting.
//!
//! Pipeline (all steps optional through [`ShapeOptions`] except the first):
//!
//! 1. **Mapping.** Each scalar becomes one cluster holding one glyph from
//!    the character map. Absent glyphs become `.notdef` and are listed in
//!    [`Shaped::missing`]; default-ignorable characters (ZWJ, ZWNJ, ZWSP,
//!    word joiner, BOM, variation selectors) become empty clusters so their
//!    bytes stay addressable without producing a glyph.
//! 2. **Mark composition.** A combining mark after a base is first composed
//!    through the canonical pairwise table into a precomposed character when
//!    the face has that glyph (so `e` + U+0301 shapes exactly like `é`);
//!    otherwise, if the face has the mark, it joins the base's cluster as a
//!    zero-advance glyph; otherwise it is a missing glyph in that cluster.
//!    No GPOS mark attachment is applied (see README).
//! 3. **Ligatures.** GSUB `liga` LigatureSubst (or, for Core 14 faces and as
//!    a fallback for fonts whose cmap maps U+FB00..U+FB04, the standard
//!    f-ligatures) merge adjacent single-glyph clusters into one cluster whose
//!    `source_range` and `text` cover every component.
//! 4. **Kerning.** GPOS PairPos (`kern` feature), the legacy `kern` table, or
//!    AFM `KPX` pairs adjust the advance of the glyph before the pair.
//!
//! Scripts needing bidi, joining or reordering (Hebrew, Arabic, Syriac,
//! Indic, Thai, Khmer, ...) and bidi controls are rejected with
//! [`Error::UnsupportedScript`] rather than shaped wrongly.

use std::ops::Range;

use crate::generated::{COMBINING_MARKS, COMPOSITIONS};
use crate::{Error, Face, FontId, GlyphId, KerningSource, Unsupported};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeOptions {
    /// Apply GSUB `liga` (and the f-ligature fallback) ligatures.
    pub ligatures: bool,
    /// Apply pair kerning.
    pub kerning: bool,
    /// Compose base + combining mark into precomposed glyphs when available.
    pub compose_marks: bool,
    /// When no GSUB ligature applies, map `ff`, `fi`, `fl`, `ffi`, `ffl` through
    /// the cmap (U+FB00..U+FB04). Always used by Core 14 faces.
    pub cmap_ligature_fallback: bool,
    /// Fail with [`Error::UnsupportedFeature`] when a requested feature
    /// (`kerning` → `kern`, `ligatures` → `liga`, `compose_marks` → `mark`)
    /// depends on a lookup type this engine skipped in this font, instead of
    /// returning partially shaped text with only a note.
    pub fail_on_unsupported_lookups: bool,
}

impl Default for ShapeOptions {
    fn default() -> Self {
        ShapeOptions {
            ligatures: true,
            kerning: true,
            compose_marks: true,
            cmap_ligature_fallback: true,
            fail_on_unsupported_lookups: true,
        }
    }
}

impl ShapeOptions {
    /// Mapping only: no ligatures, kerning or composition.
    pub const PLAIN: ShapeOptions = ShapeOptions {
        ligatures: false,
        kerning: false,
        compose_marks: false,
        cmap_ligature_fallback: false,
        fail_on_unsupported_lookups: false,
    };
}

/// One positioned glyph. All values are in font units of the shaped face;
/// `advance` already includes kerning. `gid` is the face's ORIGINAL id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub gid: GlyphId,
    pub advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}

/// Glyphs produced from one indivisible piece of source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster {
    pub glyphs: Vec<Glyph>,
    /// Byte range of the input string this cluster renders.
    pub source_range: Range<usize>,
    /// The exact input text of `source_range`, for `ActualText` and hit
    /// testing. Equal to `text[source_range]`.
    pub text: String,
    /// Index into [`Shaped::fonts`] of the face that rendered this cluster.
    /// 0 is the primary face; anything else is an explicit fallback.
    pub font: usize,
}

impl Cluster {
    pub fn advance(&self) -> i64 {
        self.glyphs.iter().map(|g| i64::from(g.advance)).sum()
    }
}

/// A character the face has no glyph for; `.notdef` was emitted in its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingGlyph {
    pub ch: char,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shaped {
    pub clusters: Vec<Cluster>,
    /// Faces used, in chain order; `clusters[i].font` indexes this. The
    /// glyph ids and advances of a cluster are in ITS face's units; every
    /// face in a chain must therefore share `units_per_em` (checked).
    pub fonts: Vec<FontId>,
    pub missing: Vec<MissingGlyph>,
    /// Parse-time unsupported-feature notes of the face, copied so a
    /// consumer holding only the result still sees what was skipped.
    pub unsupported: Vec<Unsupported>,
    pub units_per_em: u16,
    /// Which kerning data was consulted (even if no pair matched); `None`
    /// when kerning was disabled or the face has none.
    pub kerning_source: KerningSource,
    /// Number of ligatures applied.
    pub ligatures_applied: usize,
}

impl Shaped {
    /// Total advance in font units.
    pub fn advance_units(&self) -> i64 {
        self.clusters.iter().map(Cluster::advance).sum()
    }

    /// Total advance in points at `size_pt`.
    pub fn width_pt(&self, size_pt: f64) -> f64 {
        self.advance_units() as f64 * size_pt / f64::from(self.units_per_em)
    }

    /// Every glyph in order (original ids).
    pub fn glyphs(&self) -> impl Iterator<Item = &Glyph> {
        self.clusters.iter().flat_map(|c| c.glyphs.iter())
    }

    /// Cluster containing byte `offset` of the source, for click-to-source.
    pub fn cluster_at_byte(&self, offset: usize) -> Option<&Cluster> {
        self.clusters
            .iter()
            .find(|c| c.source_range.contains(&offset))
    }

    /// Cluster under horizontal position `x` (font units from the start).
    pub fn cluster_at_x(&self, x: i64) -> Option<&Cluster> {
        let mut pen = 0i64;
        for c in &self.clusters {
            let w = c.advance();
            if x >= pen && x < pen + w {
                return Some(c);
            }
            pen += w;
        }
        None
    }
}

fn is_mark(cp: u32) -> bool {
    COMBINING_MARKS.binary_search(&cp).is_ok()
}

fn compose(base: u32, mark: u32) -> Option<u32> {
    COMPOSITIONS
        .binary_search_by_key(&(base, mark), |(b, m, _)| (*b, *m))
        .ok()
        .map(|i| COMPOSITIONS[i].2)
}

fn is_default_ignorable(cp: u32) -> bool {
    matches!(cp, 0x200B..=0x200D | 0x2060 | 0xFEFF | 0xFE00..=0xFE0F | 0xE0100..=0xE01EF)
}

/// Returns why a scalar cannot be shaped by this engine, if it cannot.
pub fn unsupported_reason(ch: char) -> Option<&'static str> {
    Some(match ch as u32 {
        0x0590..=0x05FF | 0xFB1D..=0xFB4F => "Hebrew needs bidi reordering",
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => {
            "Arabic needs joining and bidi reordering"
        }
        0x0700..=0x074F => "Syriac needs joining and bidi reordering",
        0x0780..=0x07BF => "Thaana needs bidi reordering",
        0x07C0..=0x07FF => "NKo needs joining and bidi reordering",
        0x0900..=0x0DFF => "Indic scripts need syllable reordering",
        0x0E00..=0x0E7F => "Thai needs mark positioning",
        0x0E80..=0x0EFF => "Lao needs mark positioning",
        0x0F00..=0x0FFF => "Tibetan needs stacking",
        0x1000..=0x109F => "Myanmar needs syllable reordering",
        0x1780..=0x17FF => "Khmer needs syllable reordering",
        0x1800..=0x18AF => "Mongolian needs joining and vertical layout",
        0x200E | 0x200F | 0x202A..=0x202E | 0x2066..=0x2069 => "bidi control characters",
        _ => return None,
    })
}

/// Shapes `text` with `face`. See the module docs for the pipeline.
pub fn shape(face: &dyn Face, text: &str, opts: &ShapeOptions) -> Result<Shaped, Error> {
    let unsupported_for = |feature: &str| -> Option<Error> {
        if !opts.fail_on_unsupported_lookups {
            return None;
        }
        face.unsupported()
            .iter()
            .find(|u| u.feature == feature)
            .map(|u| Error::UnsupportedFeature {
                table: u.table,
                feature: u.feature,
                detail: u.detail.clone(),
            })
    };
    // Pair features are exercised by any text of two or more scalars.
    if text.chars().nth(1).is_some() {
        if opts.kerning
            && let Some(e) = unsupported_for("kern")
        {
            return Err(e);
        }
        if opts.ligatures
            && let Some(e) = unsupported_for("liga")
        {
            return Err(e);
        }
    }
    let notdef_advance = i32::from(face.advance(GlyphId::NOTDEF).unwrap_or(0));
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut missing = Vec::new();

    // 1 + 2: mapping and mark composition.
    for (byte_offset, ch) in text.char_indices() {
        if let Some(reason) = unsupported_reason(ch) {
            return Err(Error::UnsupportedScript {
                ch,
                byte_offset,
                reason,
            });
        }
        let cp = ch as u32;
        let end = byte_offset + ch.len_utf8();
        if is_default_ignorable(cp) {
            clusters.push(Cluster {
                glyphs: Vec::new(),
                source_range: byte_offset..end,
                text: ch.to_string(),
                font: 0,
            });
            continue;
        }
        if opts.compose_marks
            && is_mark(cp)
            && let Some(last) = clusters.last_mut()
        {
            let base_char = last.text.chars().last();
            let single_glyph = last.glyphs.len() == 1;
            if let (Some(base), true) = (base_char, single_glyph)
                && let Some(pre) = compose(base as u32, cp)
                && let Some(c) = char::from_u32(pre)
                && let Some(gid) = face.glyph_id(c)
            {
                let adv = i32::from(face.advance(gid)?);
                last.glyphs[0] = Glyph {
                    gid,
                    advance: adv,
                    x_offset: 0,
                    y_offset: 0,
                };
                // Keep the source text verbatim: the cluster still reads
                // "e\u{301}" even though one precomposed glyph renders it.
                last.source_range.end = end;
                last.text.push(ch);
                continue;
            }
            if !last.glyphs.is_empty() {
                // Attach as a zero-advance mark glyph (or missing).
                // A mark that could not be composed exercises the `mark`
                // feature; refuse rather than approximate when the font's
                // mark lookups are of a kind this engine does not apply.
                if let Some(e) = unsupported_for("mark") {
                    return Err(e);
                }
                let base_adv = last.glyphs.last().map_or(0, |g| g.advance);
                let base_gid = last.glyphs[0].gid;
                match face.glyph_id(ch) {
                    Some(gid) => {
                        let mark_adv = i32::from(face.advance(gid)?);
                        // GPOS MarkToBase when the font has it; otherwise the
                        // approximation: zero-advance marks sit where the
                        // designer hung them, spacing ones are centred.
                        let (x_offset, y_offset) = match face.mark_attachment(base_gid, gid) {
                            Some((dx, dy)) => (i32::from(dx) - base_adv, i32::from(dy)),
                            None if mark_adv == 0 => (0, 0),
                            None => (-(base_adv + mark_adv) / 2, 0),
                        };
                        last.glyphs.push(Glyph {
                            gid,
                            advance: 0,
                            x_offset,
                            y_offset,
                        });
                    }
                    None => {
                        missing.push(MissingGlyph { ch, byte_offset });
                        last.glyphs.push(Glyph {
                            gid: GlyphId::NOTDEF,
                            advance: 0,
                            x_offset: 0,
                            y_offset: 0,
                        });
                    }
                }
                last.source_range.end = end;
                last.text.push(ch);
                continue;
            }
        }
        let glyph = match face.glyph_id(ch) {
            Some(gid) => Glyph {
                gid,
                advance: i32::from(face.advance(gid)?),
                x_offset: 0,
                y_offset: 0,
            },
            None => {
                missing.push(MissingGlyph { ch, byte_offset });
                Glyph {
                    gid: GlyphId::NOTDEF,
                    advance: notdef_advance,
                    x_offset: 0,
                    y_offset: 0,
                }
            }
        };
        clusters.push(Cluster {
            glyphs: vec![glyph],
            source_range: byte_offset..end,
            text: ch.to_string(),
            font: 0,
        });
    }

    // 3: ligatures over runs of single-glyph, non-missing clusters.
    let mut ligatures_applied = 0;
    // Monospaced faces never ligate (a 600-unit "fi" would swallow a cell),
    // even when their character map carries f-ligature glyphs, as Courier's
    // AFM does.
    if opts.ligatures && !face.is_fixed_pitch() {
        // One pass per GSUB lookup in lookup order, plus a final pass for the
        // cmap fallback (pass index == ligature_passes()).
        let gsub_passes = face.ligature_passes();
        let fallback_pass = if opts.cmap_ligature_fallback { 1 } else { 0 };
        for pass in 0..gsub_passes + fallback_pass {
            let mut i = 0;
            while i < clusters.len() {
                let max = (clusters.len() - i).min(4);
                let mut run: Vec<GlyphId> = Vec::with_capacity(max);
                for c in &clusters[i..i + max] {
                    if c.glyphs.len() != 1 || c.glyphs[0].gid == GlyphId::NOTDEF {
                        break;
                    }
                    run.push(c.glyphs[0].gid);
                }
                let mut found = if run.len() >= 2 && pass < gsub_passes {
                    face.longest_ligature(pass, &run)
                } else {
                    None
                };
                if pass == gsub_passes && run.len() >= 2 {
                    // Each component must be its own one-character cluster, so
                    // an already-formed "fi" cluster is never re-ligated.
                    let texts: Vec<&str> = clusters[i..i + run.len()]
                        .iter()
                        .map(|c| c.text.as_str())
                        .collect();
                    for (seq, lig) in [
                        (&["f", "f", "i"][..], '\u{FB03}'),
                        (&["f", "f", "l"][..], '\u{FB04}'),
                        (&["f", "f"][..], '\u{FB00}'),
                        (&["f", "i"][..], '\u{FB01}'),
                        (&["f", "l"][..], '\u{FB02}'),
                    ] {
                        if texts.starts_with(seq)
                            && let Some(gid) = face.glyph_id(lig)
                        {
                            found = Some((gid, seq.len()));
                            break;
                        }
                    }
                }
                if let Some((gid, len)) = found
                    && len >= 2
                {
                    let merged: Vec<Cluster> = clusters.drain(i..i + len).collect();
                    let text: String = merged.iter().map(|c| c.text.as_str()).collect();
                    clusters.insert(
                        i,
                        Cluster {
                            glyphs: vec![Glyph {
                                gid,
                                advance: i32::from(face.advance(gid)?),
                                x_offset: 0,
                                y_offset: 0,
                            }],
                            source_range: merged[0].source_range.start
                                ..merged[len - 1].source_range.end,
                            text,
                            font: 0,
                        },
                    );
                    ligatures_applied += 1;
                }
                i += 1;
            }
        }
    }

    // 4: kerning between the last glyph of one cluster and the first of the next.
    let mut kerning_source = KerningSource::None;
    if opts.kerning {
        kerning_source = face.kerning_source();
        let n = clusters.len();
        for i in 0..n.saturating_sub(1) {
            let (left, right) = {
                let l = clusters[i].glyphs.last().map(|g| g.gid);
                let r = clusters[i + 1].glyphs.first().map(|g| g.gid);
                match (l, r) {
                    (Some(l), Some(r)) if l != GlyphId::NOTDEF && r != GlyphId::NOTDEF => (l, r),
                    _ => continue,
                }
            };
            let (adj, _) = face.kerning(left, right);
            if adj != 0
                && let Some(g) = clusters[i].glyphs.last_mut()
            {
                g.advance += i32::from(adj);
            }
        }
    }

    Ok(Shaped {
        clusters,
        fonts: vec![face.id().clone()],
        missing,
        unsupported: face.unsupported().to_vec(),
        units_per_em: face.units_per_em(),
        kerning_source,
        ligatures_applied,
    })
}

/// Shapes `text` with an explicit, ordered fallback chain: `faces[0]` is the
/// primary; each character is rendered by the FIRST face in the chain whose
/// character map has it (a combining mark stays with its base's face so
/// composition can apply). Characters no face has are shaped by the primary
/// as `.notdef` and listed in `missing`. The choice is a pure function of
/// the chain and the text, so it is deterministic and reported per cluster
/// through [`Cluster::font`] / [`Shaped::fonts`] — never silent.
///
/// Runs from different faces are shaped independently: no kerning or
/// ligature crosses a face boundary. All faces must share `units_per_em`.
pub fn shape_with_fallback(
    faces: &[&dyn Face],
    text: &str,
    opts: &ShapeOptions,
) -> Result<Shaped, Error> {
    let Some(primary) = faces.first() else {
        return Err(Error::Unsupported("empty fallback chain".into()));
    };
    for f in faces {
        if f.units_per_em() != primary.units_per_em() {
            return Err(Error::Unsupported(format!(
                "fallback face {} has {} units/em, primary has {}",
                f.postscript_name(),
                f.units_per_em(),
                primary.units_per_em()
            )));
        }
    }
    // Pass 1: assign a face index to every scalar.
    let mut assignment: Vec<(usize, usize, char)> = Vec::new(); // (byte, face, ch)
    let mut current = 0usize;
    for (byte_offset, ch) in text.char_indices() {
        if let Some(reason) = unsupported_reason(ch) {
            return Err(Error::UnsupportedScript {
                ch,
                byte_offset,
                reason,
            });
        }
        let cp = ch as u32;
        let face = if is_default_ignorable(cp) || (is_mark(cp) && !assignment.is_empty()) {
            current
        } else {
            faces
                .iter()
                .position(|f| f.glyph_id(ch).is_some())
                .unwrap_or(0)
        };
        current = face;
        assignment.push((byte_offset, face, ch));
    }
    // Pass 2: shape each maximal run with its face and stitch.
    let mut out = Shaped {
        clusters: Vec::new(),
        fonts: faces.iter().map(|f| f.id().clone()).collect(),
        missing: Vec::new(),
        unsupported: Vec::new(),
        units_per_em: primary.units_per_em(),
        kerning_source: KerningSource::None,
        ligatures_applied: 0,
    };
    let mut i = 0;
    while i < assignment.len() {
        let face_index = assignment[i].1;
        let start = assignment[i].0;
        let mut j = i;
        while j < assignment.len() && assignment[j].1 == face_index {
            j += 1;
        }
        let end = if j < assignment.len() {
            assignment[j].0
        } else {
            text.len()
        };
        let run = shape(faces[face_index], &text[start..end], opts)?;
        for mut c in run.clusters {
            c.source_range = c.source_range.start + start..c.source_range.end + start;
            c.font = face_index;
            out.clusters.push(c);
        }
        for m in run.missing {
            out.missing.push(MissingGlyph {
                ch: m.ch,
                byte_offset: m.byte_offset + start,
            });
        }
        for u in run.unsupported {
            if !out.unsupported.contains(&u) {
                out.unsupported.push(u);
            }
        }
        if face_index == 0 {
            out.kerning_source = run.kerning_source;
        }
        out.ligatures_applied += run.ligatures_applied;
        i = j;
    }
    Ok(out)
}
