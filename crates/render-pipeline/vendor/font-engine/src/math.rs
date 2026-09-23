//! OpenType `MATH` table (OpenType 1.9 chapter 6.3): `MathConstants` (all
//! 56 values), `MathGlyphInfo` (per-glyph italics correction, top-accent
//! attachment, extended-shape coverage and the `MathKernInfo` staircase
//! kern tables) and `MathVariants` (`minConnectorOverlap`, the vertical and
//! horizontal size variants of a glyph and its `GlyphAssembly` parts for a
//! delimiter taller than every variant).
//!
//! `MathConstants` and the italics/top-accent lists are required: a face
//! whose table is malformed there fails to load. The kern tables and the
//! variants are read leniently -- a malformed `MathKernInfo` or
//! `MathVariants` leaves the face loadable with those lookups empty --
//! because text faces occasionally carry a stub `MATH` table and nothing in
//! text shaping needs either. Device tables are ignored (values are the
//! unhinted design values).

use std::collections::BTreeMap;

use crate::GlyphId;
use crate::otl::Coverage;
use crate::reader::{i16_at, u16_at, u32_at};

/// `MathConstants`, in font units (percent fields in percent).
/// Field order follows the OpenType specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathConstants {
    pub script_percent_scale_down: i16,
    pub script_script_percent_scale_down: i16,
    pub delimited_sub_formula_min_height: u16,
    pub display_operator_min_height: u16,
    pub math_leading: i16,
    pub axis_height: i16,
    pub accent_base_height: i16,
    pub flattened_accent_base_height: i16,
    pub subscript_shift_down: i16,
    pub subscript_top_max: i16,
    pub subscript_baseline_drop_min: i16,
    pub superscript_shift_up: i16,
    pub superscript_shift_up_cramped: i16,
    pub superscript_bottom_min: i16,
    pub superscript_baseline_drop_max: i16,
    pub sub_superscript_gap_min: i16,
    pub superscript_bottom_max_with_subscript: i16,
    pub space_after_script: i16,
    pub upper_limit_gap_min: i16,
    pub upper_limit_baseline_rise_min: i16,
    pub lower_limit_gap_min: i16,
    pub lower_limit_baseline_drop_min: i16,
    pub stack_top_shift_up: i16,
    pub stack_top_display_style_shift_up: i16,
    pub stack_bottom_shift_down: i16,
    pub stack_bottom_display_style_shift_down: i16,
    pub stack_gap_min: i16,
    pub stack_display_style_gap_min: i16,
    pub stretch_stack_top_shift_up: i16,
    pub stretch_stack_bottom_shift_down: i16,
    pub stretch_stack_gap_above_min: i16,
    pub stretch_stack_gap_below_min: i16,
    pub fraction_numerator_shift_up: i16,
    pub fraction_numerator_display_style_shift_up: i16,
    pub fraction_denominator_shift_down: i16,
    pub fraction_denominator_display_style_shift_down: i16,
    pub fraction_numerator_gap_min: i16,
    pub fraction_num_display_style_gap_min: i16,
    pub fraction_rule_thickness: i16,
    pub fraction_denominator_gap_min: i16,
    pub fraction_denom_display_style_gap_min: i16,
    pub skewed_fraction_horizontal_gap: i16,
    pub skewed_fraction_vertical_gap: i16,
    pub overbar_vertical_gap: i16,
    pub overbar_rule_thickness: i16,
    pub overbar_extra_ascender: i16,
    pub underbar_vertical_gap: i16,
    pub underbar_rule_thickness: i16,
    pub underbar_extra_ascender: i16,
    pub radical_vertical_gap: i16,
    pub radical_display_style_vertical_gap: i16,
    pub radical_rule_thickness: i16,
    pub radical_extra_ascender: i16,
    pub radical_kern_before_degree: i16,
    pub radical_kern_after_degree: i16,
    pub radical_degree_bottom_raise_percent: i16,
}

#[derive(Debug, Clone)]
struct GlyphValues {
    coverage: Coverage,
    values: Vec<i16>,
}

impl GlyphValues {
    fn parse(b: &[u8], at: usize) -> Result<GlyphValues, crate::Error> {
        let coverage = Coverage::parse(b, at + usize::from(u16_at(b, at)?))?;
        let n = usize::from(u16_at(b, at + 2)?);
        let mut values = Vec::with_capacity(n);
        for i in 0..n {
            values.push(i16_at(b, at + 4 + 4 * i)?); // MathValueRecord: value, deviceOffset
        }
        Ok(GlyphValues { coverage, values })
    }

    fn get(&self, gid: GlyphId) -> Option<i16> {
        let i = self.coverage.index(gid.0)?;
        self.values.get(usize::from(i)).copied()
    }
}

/// One `MathGlyphVariantRecord`: a size variant of a glyph and its advance
/// measurement along the construction's axis (height + depth of a vertical
/// variant, width of a horizontal one), font units. Variants are listed in
/// the table's order, which the specification requires to be increasing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphVariant {
    pub gid: GlyphId,
    pub advance: u16,
}

/// One `GlyphPartRecord` of a `GlyphAssembly`, font units.
///
/// `full_advance` is the part's extent along the assembly axis. The
/// connectors are how much of the part may be overlapped by its neighbour
/// before it (`start_connector`) and after it (`end_connector`); a joint may
/// overlap by at most `min(end connector of the earlier part, start
/// connector of the later)` and at least [`MathTable::min_connector_overlap`].
/// `extender` is `partFlags` bit 0 (`fExtender`): the part repeats as often
/// as the wanted size needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphPart {
    pub gid: GlyphId,
    pub start_connector: u16,
    pub end_connector: u16,
    pub full_advance: u16,
    pub extender: bool,
}

/// A `GlyphAssembly`: the parts in the table's order (bottom to top for a
/// vertical construction, left to right for a horizontal one) and the
/// italics correction of the assembled glyph, font units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphAssembly {
    pub italics_correction: i16,
    pub parts: Vec<GlyphPart>,
}

/// One `MathGlyphConstruction`: the variants and, when the glyph has one,
/// the assembly.
#[derive(Debug, Clone, Default)]
struct Construction {
    variants: Vec<GlyphVariant>,
    assembly: Option<GlyphAssembly>,
}

/// The corner of a glyph a `MathKern` table belongs to (`MathKernInfoRecord`
/// field order): where a superscript attaches to a base (`TopRight`), a
/// subscript (`BottomRight`), and the script's own corners facing the base
/// (`BottomLeft` under a superscript, `TopLeft` above a subscript).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KernCorner {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

impl KernCorner {
    fn index(self) -> usize {
        match self {
            KernCorner::TopRight => 0,
            KernCorner::TopLeft => 1,
            KernCorner::BottomRight => 2,
            KernCorner::BottomLeft => 3,
        }
    }
}

/// One `MathKern` staircase: `heights` (ascending) and one more kern value
/// than heights.
#[derive(Debug, Clone, Default)]
struct MathKern {
    heights: Vec<i16>,
    kerns: Vec<i16>,
}

impl MathKern {
    fn parse(b: &[u8], at: usize) -> Result<MathKern, crate::Error> {
        let n = usize::from(u16_at(b, at)?);
        let mut heights = Vec::with_capacity(n);
        for i in 0..n {
            heights.push(i16_at(b, at + 2 + 4 * i)?); // MathValueRecord: value, deviceOffset
        }
        let mut kerns = Vec::with_capacity(n + 1);
        for i in 0..=n {
            kerns.push(i16_at(b, at + 2 + 4 * n + 4 * i)?);
        }
        Ok(MathKern { heights, kerns })
    }

    /// The kern at `height`: the value paired with the next correction
    /// height above `height`, or the last value when no height is above it
    /// (OpenType 1.9 §6.3.3 "MathKern table"; the LuaTeX manual's "the next
    /// higher height and kern pair, or the highest one in the character").
    fn at(&self, height: i16) -> i16 {
        let i = self.heights.iter().filter(|h| **h <= height).count();
        self.kerns.get(i).copied().unwrap_or(0)
    }
}

/// The `MathVariants` and `MathKernInfo` sub-tables, read leniently.
#[derive(Debug, Clone, Default)]
struct Extras {
    extended_shapes: Option<Coverage>,
    kern_coverage: Option<Coverage>,
    /// Per covered glyph, its four corners in [`KernCorner`] order.
    kerns: Vec<[Option<MathKern>; 4]>,
    min_connector_overlap: u16,
    vertical: BTreeMap<u16, Construction>,
    horizontal: BTreeMap<u16, Construction>,
}

#[derive(Debug, Clone)]
pub struct MathTable {
    pub constants: MathConstants,
    italics_correction: Option<GlyphValues>,
    top_accent: Option<GlyphValues>,
    extras: Extras,
}

impl MathTable {
    pub fn parse(b: &[u8]) -> Result<MathTable, crate::Error> {
        let version = u32_at(b, 0)?;
        if version >> 16 != 1 {
            return Err(crate::Error::Unsupported(format!(
                "MATH table version {}.{}",
                version >> 16,
                version & 0xFFFF
            )));
        }
        let constants_at = usize::from(u16_at(b, 4)?);
        let glyph_info_at = usize::from(u16_at(b, 6)?);
        let c = constants_at;
        let mut vals = [0i16; 51];
        for (i, v) in vals.iter_mut().enumerate() {
            *v = i16_at(b, c + 8 + 4 * i)?;
        }
        let constants = MathConstants {
            script_percent_scale_down: i16_at(b, c)?,
            script_script_percent_scale_down: i16_at(b, c + 2)?,
            delimited_sub_formula_min_height: u16_at(b, c + 4)?,
            display_operator_min_height: u16_at(b, c + 6)?,
            math_leading: vals[0],
            axis_height: vals[1],
            accent_base_height: vals[2],
            flattened_accent_base_height: vals[3],
            subscript_shift_down: vals[4],
            subscript_top_max: vals[5],
            subscript_baseline_drop_min: vals[6],
            superscript_shift_up: vals[7],
            superscript_shift_up_cramped: vals[8],
            superscript_bottom_min: vals[9],
            superscript_baseline_drop_max: vals[10],
            sub_superscript_gap_min: vals[11],
            superscript_bottom_max_with_subscript: vals[12],
            space_after_script: vals[13],
            upper_limit_gap_min: vals[14],
            upper_limit_baseline_rise_min: vals[15],
            lower_limit_gap_min: vals[16],
            lower_limit_baseline_drop_min: vals[17],
            stack_top_shift_up: vals[18],
            stack_top_display_style_shift_up: vals[19],
            stack_bottom_shift_down: vals[20],
            stack_bottom_display_style_shift_down: vals[21],
            stack_gap_min: vals[22],
            stack_display_style_gap_min: vals[23],
            stretch_stack_top_shift_up: vals[24],
            stretch_stack_bottom_shift_down: vals[25],
            stretch_stack_gap_above_min: vals[26],
            stretch_stack_gap_below_min: vals[27],
            fraction_numerator_shift_up: vals[28],
            fraction_numerator_display_style_shift_up: vals[29],
            fraction_denominator_shift_down: vals[30],
            fraction_denominator_display_style_shift_down: vals[31],
            fraction_numerator_gap_min: vals[32],
            fraction_num_display_style_gap_min: vals[33],
            fraction_rule_thickness: vals[34],
            fraction_denominator_gap_min: vals[35],
            fraction_denom_display_style_gap_min: vals[36],
            skewed_fraction_horizontal_gap: vals[37],
            skewed_fraction_vertical_gap: vals[38],
            overbar_vertical_gap: vals[39],
            overbar_rule_thickness: vals[40],
            overbar_extra_ascender: vals[41],
            underbar_vertical_gap: vals[42],
            underbar_rule_thickness: vals[43],
            underbar_extra_ascender: vals[44],
            radical_vertical_gap: vals[45],
            radical_display_style_vertical_gap: vals[46],
            radical_rule_thickness: vals[47],
            radical_extra_ascender: vals[48],
            radical_kern_before_degree: vals[49],
            radical_kern_after_degree: vals[50],
            radical_degree_bottom_raise_percent: i16_at(b, c + 8 + 4 * 51)?,
        };
        let mut italics_correction = None;
        let mut top_accent = None;
        let mut extras = Extras::default();
        if glyph_info_at != 0 {
            let ic = usize::from(u16_at(b, glyph_info_at)?);
            if ic != 0 {
                italics_correction = Some(GlyphValues::parse(b, glyph_info_at + ic)?);
            }
            let ta = usize::from(u16_at(b, glyph_info_at + 2)?);
            if ta != 0 {
                top_accent = Some(GlyphValues::parse(b, glyph_info_at + ta)?);
            }
            // Lenient from here on (see the module comment).
            let es = usize::from(u16_at(b, glyph_info_at + 4).unwrap_or(0));
            if es != 0 {
                extras.extended_shapes = Coverage::parse(b, glyph_info_at + es).ok();
            }
            let ki = usize::from(u16_at(b, glyph_info_at + 6).unwrap_or(0));
            if ki != 0
                && let Ok((coverage, kerns)) = parse_kern_info(b, glyph_info_at + ki)
            {
                extras.kern_coverage = Some(coverage);
                extras.kerns = kerns;
            }
        }
        let variants_at = usize::from(u16_at(b, 8).unwrap_or(0));
        if variants_at != 0
            && let Ok((overlap, vertical, horizontal)) = parse_variants(b, variants_at)
        {
            extras.min_connector_overlap = overlap;
            extras.vertical = vertical;
            extras.horizontal = horizontal;
        }
        Ok(MathTable {
            constants,
            italics_correction,
            top_accent,
            extras,
        })
    }

    /// The table's `MathConstants`.
    pub fn constants(&self) -> &MathConstants {
        &self.constants
    }

    /// Whether `gid` is in the `ExtendedShapeCoverage`: a glyph (a large
    /// operator, an integral) whose vertical extent is not its design size,
    /// so a superscript on it is placed by its actual height rather than
    /// the size's default shift.
    pub fn is_extended_shape(&self, gid: GlyphId) -> bool {
        self.extras
            .extended_shapes
            .as_ref()
            .is_some_and(|c| c.index(gid.0).is_some())
    }

    /// The `MathKernInfo` kern at `corner` of `gid` for a script whose
    /// relevant edge is `height` font units above the baseline, in font
    /// units (usually negative: the script moves in). 0 for a glyph without
    /// a table at that corner.
    pub fn kern(&self, gid: GlyphId, corner: KernCorner, height: i16) -> i16 {
        let Some(i) = self
            .extras
            .kern_coverage
            .as_ref()
            .and_then(|c| c.index(gid.0))
        else {
            return 0;
        };
        self.extras
            .kerns
            .get(usize::from(i))
            .and_then(|corners| corners[corner.index()].as_ref())
            .map_or(0, |k| k.at(height))
    }

    /// Whether `gid` has a `MathKernInfo` record at all.
    pub fn has_kern_info(&self, gid: GlyphId) -> bool {
        self.extras
            .kern_coverage
            .as_ref()
            .is_some_and(|c| c.index(gid.0).is_some())
    }

    /// `MathVariants.minConnectorOverlap`, font units: the least two parts
    /// of an assembly may overlap (20 in Latin Modern Math). 0 when the face
    /// has no `MathVariants`.
    pub fn min_connector_overlap(&self) -> u16 {
        self.extras.min_connector_overlap
    }

    /// The vertical size variants of `gid` in the table's (increasing)
    /// order, empty when the glyph has no vertical construction. Fonts
    /// usually list the base glyph itself first (Latin Modern Math's `∑`:
    /// `[3060, 3074]`).
    pub fn vertical_variants(&self, gid: GlyphId) -> &[GlyphVariant] {
        self.extras
            .vertical
            .get(&gid.0)
            .map_or(&[], |c| c.variants.as_slice())
    }

    /// The horizontal size variants of `gid` (wide accents, over/underbrace
    /// pieces), as [`MathTable::vertical_variants`].
    pub fn horizontal_variants(&self, gid: GlyphId) -> &[GlyphVariant] {
        self.extras
            .horizontal
            .get(&gid.0)
            .map_or(&[], |c| c.variants.as_slice())
    }

    /// The vertical `GlyphAssembly` of `gid`, parts bottom to top, if it
    /// has one.
    pub fn vertical_assembly(&self, gid: GlyphId) -> Option<&GlyphAssembly> {
        self.extras.vertical.get(&gid.0)?.assembly.as_ref()
    }

    /// The horizontal `GlyphAssembly` of `gid`, parts left to right, if it
    /// has one.
    pub fn horizontal_assembly(&self, gid: GlyphId) -> Option<&GlyphAssembly> {
        self.extras.horizontal.get(&gid.0)?.assembly.as_ref()
    }

    /// Italics correction of `gid` in font units (0 when the font lists none).
    pub fn italics_correction(&self, gid: GlyphId) -> i16 {
        self.italics_correction
            .as_ref()
            .and_then(|v| v.get(gid))
            .unwrap_or(0)
    }

    /// Horizontal top-accent attachment point, if the font lists one; the
    /// specification's default is half the advance width.
    pub fn top_accent_attachment(&self, gid: GlyphId) -> Option<i16> {
        self.top_accent.as_ref().and_then(|v| v.get(gid))
    }
}

/// The glyph ids a coverage table covers, in coverage-index order.
fn covered_glyphs(c: &Coverage) -> Vec<u16> {
    match c {
        Coverage::Glyphs(v) => v.clone(),
        Coverage::Ranges(v) => v.iter().flat_map(|(start, end, _)| *start..=*end).collect(),
    }
}

/// `MathKernInfo` at `at`: `mathKernCoverageOffset`, `mathKernCount`, then
/// one `MathKernInfoRecord` of four offsets (top right, top left, bottom
/// right, bottom left; 0 for no table) per covered glyph, all offsets from
/// the start of `MathKernInfo`.
#[allow(clippy::type_complexity)]
fn parse_kern_info(
    b: &[u8],
    at: usize,
) -> Result<(Coverage, Vec<[Option<MathKern>; 4]>), crate::Error> {
    let coverage = Coverage::parse(b, at + usize::from(u16_at(b, at)?))?;
    let n = usize::from(u16_at(b, at + 2)?);
    let mut records = Vec::with_capacity(n);
    for i in 0..n {
        let rec = at + 4 + 8 * i;
        let mut corners: [Option<MathKern>; 4] = [None, None, None, None];
        for (k, corner) in corners.iter_mut().enumerate() {
            let off = usize::from(u16_at(b, rec + 2 * k)?);
            if off != 0 {
                *corner = Some(MathKern::parse(b, at + off)?);
            }
        }
        records.push(corners);
    }
    Ok((coverage, records))
}

/// `MathVariants` at `at`: `minConnectorOverlap`, the vertical and
/// horizontal coverage offsets and counts, then one array of construction
/// offsets -- `vertGlyphCount` vertical entries followed by
/// `horizGlyphCount` horizontal ones -- each in its coverage's index order.
/// Every offset is from the start of `MathVariants`.
#[allow(clippy::type_complexity)]
fn parse_variants(
    b: &[u8],
    at: usize,
) -> Result<
    (
        u16,
        BTreeMap<u16, Construction>,
        BTreeMap<u16, Construction>,
    ),
    crate::Error,
> {
    let overlap = u16_at(b, at)?;
    let vert_cov = usize::from(u16_at(b, at + 2)?);
    let horiz_cov = usize::from(u16_at(b, at + 4)?);
    let vert_count = usize::from(u16_at(b, at + 6)?);
    let horiz_count = usize::from(u16_at(b, at + 8)?);
    let mut vertical = BTreeMap::new();
    let mut horizontal = BTreeMap::new();
    if vert_cov != 0 {
        let gids = covered_glyphs(&Coverage::parse(b, at + vert_cov)?);
        for (i, gid) in gids.into_iter().enumerate().take(vert_count) {
            let cons = at + usize::from(u16_at(b, at + 10 + 2 * i)?);
            vertical.insert(gid, parse_construction(b, cons)?);
        }
    }
    if horiz_cov != 0 {
        let gids = covered_glyphs(&Coverage::parse(b, at + horiz_cov)?);
        for (i, gid) in gids.into_iter().enumerate().take(horiz_count) {
            let cons = at + usize::from(u16_at(b, at + 10 + 2 * vert_count + 2 * i)?);
            horizontal.insert(gid, parse_construction(b, cons)?);
        }
    }
    Ok((overlap, vertical, horizontal))
}

/// One `MathGlyphConstruction` at `cons`: `glyphAssemblyOffset` (from
/// `cons`, 0 for none), `variantCount` and the 4-byte
/// `MathGlyphVariantRecord`s; a `GlyphAssembly` is a 4-byte
/// `italicsCorrection` `MathValueRecord`, `partCount` and 10-byte
/// `GlyphPartRecord`s (`glyphID`, `startConnectorLength`,
/// `endConnectorLength`, `fullAdvance`, `partFlags`).
fn parse_construction(b: &[u8], cons: usize) -> Result<Construction, crate::Error> {
    let assembly_at = usize::from(u16_at(b, cons)?);
    let n = usize::from(u16_at(b, cons + 2)?);
    let mut variants = Vec::with_capacity(n);
    for j in 0..n {
        let rec = cons + 4 + 4 * j;
        variants.push(GlyphVariant {
            gid: GlyphId(u16_at(b, rec)?),
            advance: u16_at(b, rec + 2)?,
        });
    }
    let mut assembly = None;
    if assembly_at != 0 {
        let a = cons + assembly_at;
        let italics_correction = i16_at(b, a)?;
        let count = usize::from(u16_at(b, a + 4)?);
        let mut parts = Vec::with_capacity(count);
        for k in 0..count {
            let p = a + 6 + 10 * k;
            parts.push(GlyphPart {
                gid: GlyphId(u16_at(b, p)?),
                start_connector: u16_at(b, p + 2)?,
                end_connector: u16_at(b, p + 4)?,
                full_advance: u16_at(b, p + 6)?,
                extender: u16_at(b, p + 8)? & 1 != 0,
            });
        }
        assembly = Some(GlyphAssembly {
            italics_correction,
            parts,
        });
    }
    Ok(Construction { variants, assembly })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Face;
    use std::path::Path;

    /// The bundled Latin Modern Math (`apps/mac/Fonts`); `None` with a note
    /// when this crate is built somewhere the bundle is not beside it (the
    /// pipeline's vendored copy).
    fn latin_modern_math() -> Option<crate::TrueTypeFace> {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/mac/Fonts/latinmodern-math.otf");
        if !path.is_file() {
            eprintln!("SKIP: {} not present", path.display());
            return None;
        }
        Some(crate::load_from_path(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
    }

    /// STIX Two Math as macOS ships it: the installed `MATH` face with
    /// `MathKernInfo` (Latin Modern Math carries none).
    fn stix_two_math() -> Option<crate::TrueTypeFace> {
        let path = Path::new("/System/Library/Fonts/Supplemental/STIXTwoMath.otf");
        if !path.is_file() {
            eprintln!("SKIP: {} not present", path.display());
            return None;
        }
        Some(crate::load_from_path(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
    }

    #[test]
    fn latin_modern_math_constants_all_fifty_six() {
        let Some(f) = latin_modern_math() else { return };
        let c = f.math().expect("MATH table").constants();
        // Every value as `ttx -t MATH latinmodern-math.otf` (fonttools
        // 4.65) prints it, in table order.
        assert_eq!(c.script_percent_scale_down, 70);
        assert_eq!(c.script_script_percent_scale_down, 50);
        assert_eq!(c.delimited_sub_formula_min_height, 1300);
        assert_eq!(c.display_operator_min_height, 1300);
        assert_eq!(c.math_leading, 154);
        assert_eq!(c.axis_height, 250);
        assert_eq!(c.accent_base_height, 450);
        assert_eq!(c.flattened_accent_base_height, 664);
        assert_eq!(c.subscript_shift_down, 247);
        assert_eq!(c.subscript_top_max, 344);
        assert_eq!(c.subscript_baseline_drop_min, 200);
        assert_eq!(c.superscript_shift_up, 363);
        assert_eq!(c.superscript_shift_up_cramped, 289);
        assert_eq!(c.superscript_bottom_min, 108);
        assert_eq!(c.superscript_baseline_drop_max, 250);
        assert_eq!(c.sub_superscript_gap_min, 160);
        assert_eq!(c.superscript_bottom_max_with_subscript, 344);
        assert_eq!(c.space_after_script, 56);
        assert_eq!(c.upper_limit_gap_min, 200);
        assert_eq!(c.upper_limit_baseline_rise_min, 111);
        assert_eq!(c.lower_limit_gap_min, 167);
        assert_eq!(c.lower_limit_baseline_drop_min, 600);
        assert_eq!(c.stack_top_shift_up, 444);
        assert_eq!(c.stack_top_display_style_shift_up, 677);
        assert_eq!(c.stack_bottom_shift_down, 345);
        assert_eq!(c.stack_bottom_display_style_shift_down, 686);
        assert_eq!(c.stack_gap_min, 120);
        assert_eq!(c.stack_display_style_gap_min, 280);
        assert_eq!(c.stretch_stack_top_shift_up, 111);
        assert_eq!(c.stretch_stack_bottom_shift_down, 600);
        assert_eq!(c.stretch_stack_gap_above_min, 200);
        assert_eq!(c.stretch_stack_gap_below_min, 167);
        assert_eq!(c.fraction_numerator_shift_up, 394);
        assert_eq!(c.fraction_numerator_display_style_shift_up, 677);
        assert_eq!(c.fraction_denominator_shift_down, 345);
        assert_eq!(c.fraction_denominator_display_style_shift_down, 686);
        assert_eq!(c.fraction_numerator_gap_min, 40);
        assert_eq!(c.fraction_num_display_style_gap_min, 120);
        assert_eq!(c.fraction_rule_thickness, 40);
        assert_eq!(c.fraction_denominator_gap_min, 40);
        assert_eq!(c.fraction_denom_display_style_gap_min, 120);
        assert_eq!(c.skewed_fraction_horizontal_gap, 350);
        assert_eq!(c.skewed_fraction_vertical_gap, 96);
        assert_eq!(c.overbar_vertical_gap, 120);
        assert_eq!(c.overbar_rule_thickness, 40);
        assert_eq!(c.overbar_extra_ascender, 40);
        assert_eq!(c.underbar_vertical_gap, 120);
        assert_eq!(c.underbar_rule_thickness, 40);
        assert_eq!(c.underbar_extra_ascender, 40);
        assert_eq!(c.radical_vertical_gap, 50);
        assert_eq!(c.radical_display_style_vertical_gap, 148);
        assert_eq!(c.radical_rule_thickness, 40);
        assert_eq!(c.radical_extra_ascender, 40);
        assert_eq!(c.radical_kern_before_degree, 278);
        assert_eq!(c.radical_kern_after_degree, -556);
        assert_eq!(c.radical_degree_bottom_raise_percent, 60);
    }

    #[test]
    fn latin_modern_math_variants_and_assemblies() {
        let Some(f) = latin_modern_math() else { return };
        let m = f.math().expect("MATH table");
        // `ttx`: <MinConnectorOverlap value="20"/>.
        assert_eq!(m.min_connector_overlap(), 20);
        // `∑` lists itself and one display size (glyph ids 3060 and 3074,
        // `ttx`'s MathGlyphConstruction for `summation`).
        let sum = f.glyph_id('\u{2211}').unwrap();
        let v: Vec<u16> = m.vertical_variants(sum).iter().map(|v| v.gid.0).collect();
        assert_eq!(v, vec![3060, 3074]);
        assert!(
            m.vertical_variants(sum)
                .windows(2)
                .all(|w| w[0].advance < w[1].advance)
        );
        assert!(m.vertical_assembly(sum).is_none());
        // `(`: `ttx` prints `VariantCount=8` (itself at 997 units, then
        // `parenleft.v1`..`.v7` at 1095, 1195, 1445, ...) and a three-part
        // GlyphAssembly -- bottom hook `uni239D` (its end connecting over
        // half the extender), extender `uni239C`, top hook `uni239B`.
        let paren = f.glyph_id('(').unwrap();
        assert_eq!(m.vertical_variants(paren).len(), 8);
        assert_eq!(
            m.vertical_variants(paren)[0],
            GlyphVariant {
                gid: paren,
                advance: 997
            }
        );
        assert_eq!(m.vertical_variants(paren)[1].advance, 1095);
        let a = m.vertical_assembly(paren).expect("paren assembly");
        assert_eq!(
            a.parts,
            vec![
                GlyphPart {
                    gid: GlyphId(2503),
                    start_connector: 0,
                    end_connector: 249,
                    full_advance: 1495,
                    extender: false
                },
                GlyphPart {
                    gid: GlyphId(2504),
                    start_connector: 498,
                    end_connector: 498,
                    full_advance: 498,
                    extender: true
                },
                GlyphPart {
                    gid: GlyphId(2505),
                    start_connector: 249,
                    end_connector: 0,
                    full_advance: 1495,
                    extender: false
                },
            ]
        );
        assert_eq!(a.italics_correction, 0);
        // `{` has a middle piece between two extenders; `⌈` starts with its
        // extender and `⌊` ends with it, as cmex's recipes do.
        let brace = m
            .vertical_assembly(f.glyph_id('{').unwrap())
            .expect("brace assembly");
        assert_eq!(brace.parts.len(), 5);
        assert_eq!(brace.parts.iter().filter(|p| p.extender).count(), 2);
        assert_eq!(brace.parts[2].full_advance, 1500);
        assert!(
            m.vertical_assembly(f.glyph_id('\u{2308}').unwrap())
                .unwrap()
                .parts[0]
                .extender
        );
        assert!(
            m.vertical_assembly(f.glyph_id('\u{230A}').unwrap())
                .unwrap()
                .parts
                .last()
                .unwrap()
                .extender
        );
        // Horizontal: the over-brace U+23DE is five parts -- left end,
        // extender, middle, extender, right end; the wide hat has width
        // variants; `x` stretches nowhere.
        let over = m
            .horizontal_assembly(f.glyph_id('\u{23DE}').unwrap())
            .expect("overbrace assembly");
        assert_eq!(over.parts.len(), 5);
        assert!(over.parts[1].extender && over.parts[3].extender && !over.parts[2].extender);
        assert!(
            !m.horizontal_variants(f.glyph_id('\u{0302}').unwrap())
                .is_empty()
        );
        let x = f.glyph_id('x').unwrap();
        assert!(m.vertical_variants(x).is_empty() && m.vertical_assembly(x).is_none());
        // Latin Modern Math carries no MathKernInfo (`ttx` prints none):
        // every kern is 0 and no glyph has a record.
        let f_italic = f.glyph_id('\u{1D453}').unwrap();
        assert!(!m.has_kern_info(f_italic));
        assert_eq!(m.kern(f_italic, KernCorner::TopRight, 300), 0);
        // Extended shapes: the integral is one; `x` is not.
        assert!(m.is_extended_shape(f.glyph_id('\u{222B}').unwrap()));
        assert!(!m.is_extended_shape(x));
    }

    #[test]
    fn stix_two_math_kern_staircases() {
        let Some(f) = stix_two_math() else { return };
        let m = f.math().expect("MATH table");
        // `ttx -t MATH STIXTwoMath.otf` (macOS 26 supplemental font):
        // MathKernInfoRecords index 0 is glyph `A` (id 3), TopRightMathKern
        // heights [252, 352], kerns [0, -18, -66]; index 1 is `E`, TopRight
        // heights [252] kerns [32, 31], BottomRight heights [126] kerns
        // [24, 32]; index 2 is `F`, TopRight no heights, one kern 44,
        // BottomRight heights [126, 289] kerns [-200, -44, 44].
        let a = f.glyph_id('A').unwrap();
        assert_eq!(a, GlyphId(3));
        assert!(m.has_kern_info(a));
        assert_eq!(m.kern(a, KernCorner::TopRight, 0), 0);
        assert_eq!(m.kern(a, KernCorner::TopRight, 251), 0);
        assert_eq!(m.kern(a, KernCorner::TopRight, 300), -18);
        assert_eq!(
            m.kern(a, KernCorner::TopRight, 352),
            -66,
            "at a listed height the next pair applies"
        );
        assert_eq!(m.kern(a, KernCorner::TopRight, 1000), -66);
        assert_eq!(
            m.kern(a, KernCorner::BottomRight, 300),
            0,
            "no table at this corner"
        );
        let e = f.glyph_id('E').unwrap();
        assert_eq!(m.kern(e, KernCorner::TopRight, 100), 32);
        assert_eq!(m.kern(e, KernCorner::TopRight, 400), 31);
        assert_eq!(m.kern(e, KernCorner::BottomRight, -50), 24);
        assert_eq!(m.kern(e, KernCorner::BottomRight, 200), 32);
        let ff = f.glyph_id('F').unwrap();
        assert_eq!(m.kern(ff, KernCorner::TopRight, -500), 44);
        assert_eq!(m.kern(ff, KernCorner::TopRight, 900), 44);
        assert_eq!(m.kern(ff, KernCorner::BottomRight, 0), -200);
        assert_eq!(m.kern(ff, KernCorner::BottomRight, 200), -44);
        assert_eq!(m.kern(ff, KernCorner::BottomRight, 500), 44);
        assert!(!m.has_kern_info(f.glyph_id('+').unwrap()));
    }
}
