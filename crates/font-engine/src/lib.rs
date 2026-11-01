//! FlashTeX font engine: an original Rust implementation of font loading,
//! metrics, TeX-oriented shaping, deterministic subsetting, and the data a PDF
//! writer needs to embed a font. No external crates; no existing TeX engine.
//!
//! Two font backends share one [`Face`] API:
//!
//! * [`truetype::TrueTypeFace`] parses TrueType/OpenType fonts with `glyf`
//!   outlines (including `.ttc` collections) and reads `kern`, `GPOS` pair
//!   adjustment and `GSUB` ligature lookups.
//! * [`core14::Core14Face`] carries the Adobe Core 14 AFM widths and kerning
//!   pairs for Times (four faces), Helvetica, Courier and Symbol so the
//!   standard PDF fonts can be measured without any font file on disk.
//!
//! Shaping ([`shape::shape`]) returns clusters that map back to byte ranges
//! of the input string and reports missing glyphs and unsupported features
//! explicitly. Subsetting ([`subset`]) renumbers glyphs only inside the
//! embedding helper and returns the old-to-new map. See README.md for the
//! proposed ABI and the list of unsupported features.

pub mod adapters;
pub mod core14;
pub mod embed;
pub mod encoding;
#[rustfmt::skip]
mod generated;
mod gpos;
mod gsub;
mod kern;
pub mod manifest;
pub mod math;
mod otl;
mod reader;
pub mod resolve;
pub mod sha256;
pub mod shape;
pub mod subset;
pub mod truetype;

use std::fmt;
use std::path::{Path, PathBuf};

pub use core14::Core14Face;
pub use shape::{Cluster, Glyph, MissingGlyph, ShapeOptions, Shaped};
pub use truetype::{Outlines, TrueTypeFace};

/// Errors from loading, parsing, shaping or subsetting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Io(String),
    /// The data violates the format it claims to be.
    Malformed(String),
    /// Well-formed data using a feature this crate does not implement.
    Unsupported(String),
    MissingTable(String),
    /// A glyph id outside the face's glyph range was requested.
    GlyphOutOfRange(u16),
    /// Text containing a script or control the shaper cannot shape correctly.
    UnsupportedScript {
        ch: char,
        byte_offset: usize,
        reason: &'static str,
    },
    /// A requested shaping feature depends on a lookup type this engine does
    /// not implement in this font (e.g. contextual kerning). Raised instead
    /// of shaping partially when `ShapeOptions::fail_on_unsupported_lookups`.
    UnsupportedFeature {
        table: &'static str,
        feature: &'static str,
        detail: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(m) => write!(f, "I/O error: {m}"),
            Error::Malformed(m) => write!(f, "malformed font data: {m}"),
            Error::Unsupported(m) => write!(f, "unsupported: {m}"),
            Error::MissingTable(t) => write!(f, "required table {t} is missing"),
            Error::GlyphOutOfRange(g) => write!(f, "glyph id {g} out of range"),
            Error::UnsupportedScript {
                ch,
                byte_offset,
                reason,
            } => write!(
                f,
                "cannot shape U+{:04X} at byte {byte_offset}: {reason}",
                *ch as u32
            ),
            Error::UnsupportedFeature {
                table,
                feature,
                detail,
            } => write!(f, "{table} feature {feature}: {detail}"),
        }
    }
}

impl std::error::Error for Error {}

/// A glyph index in the face's ORIGINAL numbering. Shaping output always uses
/// original ids; only [`subset::Subset`] introduces renumbered ids, and it
/// carries the explicit map between the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlyphId(pub u16);

impl GlyphId {
    pub const NOTDEF: GlyphId = GlyphId(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style {
    Upright,
    Italic,
    Oblique,
}

/// Where a face's data came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontSource {
    /// A font program on disk (`face_index` selects a face in a `.ttc`).
    File { path: PathBuf, face_index: u32 },
    /// A font program supplied as bytes with no path.
    Memory { face_index: u32 },
    /// One of the built-in Adobe Core 14 metric sets; no program is available.
    Core14 { afm_name: &'static str },
}

/// Stable, content-addressed font identity.
///
/// `content_sha256` is SHA-256 over the complete font program bytes (the
/// whole `.ttc` for collections) followed by the 4-byte big-endian face
/// index, or over the AFM-derived table name for Core 14 faces. Two faces
/// with equal `content_sha256` have byte-identical programs and therefore
/// identical glyph ids, metrics and shaping.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontId {
    pub family: String,
    /// CSS-style weight, 100..=900 (OS/2 usWeightClass or AFM Weight).
    pub weight: u16,
    pub style: Style,
    pub source: FontSource,
    pub content_sha256: [u8; 32],
}

impl FontId {
    pub fn content_hex(&self) -> String {
        sha256::hex(&self.content_sha256)
    }
}

/// Vertical metrics in font units; divide by `units_per_em` for em fractions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalMetrics {
    pub ascender: i16,
    pub descender: i16,
    pub line_gap: i16,
    pub cap_height: i16,
    pub x_height: i16,
    /// `None` when the font does not declare it (older OS/2 versions).
    pub cap_height_declared: bool,
    pub x_height_declared: bool,
}

/// Where a kerning value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KerningSource {
    None,
    /// OpenType `GPOS` `kern` feature, PairPos lookups.
    Gpos,
    /// Legacy `kern` table, format 0.
    KernTable,
    /// AFM `KPX` pairs (Core 14 faces).
    Afm,
}

/// A parse-time discovery of a feature this crate saw but does not implement.
/// Consumers decide whether that matters for their use; the shaper also
/// copies these into every [`Shaped`] result so nothing is silently dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsupported {
    pub table: &'static str,
    /// The OpenType feature (`kern`, `liga`, `mark`) or table area the note
    /// belongs to, so the shaper can fail when that feature is requested.
    pub feature: &'static str,
    pub detail: String,
}

/// The common face API shared by TrueType and Core 14 backends.
pub trait Face {
    fn id(&self) -> &FontId;
    fn units_per_em(&self) -> u16;
    fn num_glyphs(&self) -> u16;
    fn vertical_metrics(&self) -> VerticalMetrics;
    /// Font bounding box `[x_min, y_min, x_max, y_max]` in font units.
    fn bbox(&self) -> [i16; 4];
    fn italic_angle(&self) -> f64;
    /// Advance width of `gid` in font units. `.notdef` is allowed.
    fn advance(&self, gid: GlyphId) -> Result<u16, Error>;
    /// Original glyph id for `ch` via the character map, `None` when absent.
    fn glyph_id(&self, ch: char) -> Option<GlyphId>;
    /// Horizontal kerning adjustment for the pair, in font units (usually
    /// negative), and which table supplied it.
    fn kerning(&self, left: GlyphId, right: GlyphId) -> (i16, KerningSource);
    /// Which kerning data this face would consult (regardless of any pair).
    fn kerning_source(&self) -> KerningSource;
    /// Ligature glyph the face's lookups produce for exactly this sequence.
    fn ligature(&self, components: &[GlyphId]) -> Option<GlyphId>;
    /// GPOS MarkToBase attachment: offset (dx, dy) in font units from the
    /// base glyph's origin at which `mark` is drawn, if the font defines it.
    fn mark_attachment(&self, base: GlyphId, mark: GlyphId) -> Option<(i16, i16)>;
    /// Number of ligature passes (GSUB lookups) to run in order.
    fn ligature_passes(&self) -> usize;
    /// Longest ligature of pass `pass` starting at `glyphs[0]`; returns the
    /// ligature glyph and the number of components it replaces (>= 2).
    fn longest_ligature(&self, pass: usize, glyphs: &[GlyphId]) -> Option<(GlyphId, usize)>;
    fn unsupported(&self) -> &[Unsupported];
    /// PostScript name (`name` id 6 or AFM FontName).
    fn postscript_name(&self) -> &str;
    fn is_fixed_pitch(&self) -> bool;
    /// Font-unit value scaled to points at `size_pt`.
    fn to_points(&self, units: i64, size_pt: f64) -> f64 {
        units as f64 * size_pt / f64::from(self.units_per_em())
    }
    /// Font-unit value scaled to PDF glyph space (1/1000 em), rounded.
    fn to_pdf_units(&self, units: i64) -> i32 {
        ((units as f64) * 1000.0 / f64::from(self.units_per_em())).round() as i32
    }
}

/// Loads a TrueType/OpenType face from `path` (face 0 of a collection).
pub fn load_from_path(path: &Path) -> Result<TrueTypeFace, Error> {
    load_from_path_index(path, 0)
}

/// Loads face `face_index` from a `.ttc` collection (or a single-face file
/// when `face_index` is 0).
pub fn load_from_path_index(path: &Path, face_index: u32) -> Result<TrueTypeFace, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
    TrueTypeFace::parse_with_source(
        bytes,
        FontSource::File {
            path: path.to_path_buf(),
            face_index,
        },
    )
}
