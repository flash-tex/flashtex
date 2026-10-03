//! FlashTeX PDF writer (task FT-009).
//!
//! Turns runtime-v1 `compile_result` pages (`docs/contracts/runtime-v1.md`) into
//! a PDF 1.4 document. Everything here is original: the file structure, content
//! streams, cross-reference table, and the JSON reader are written by hand. No
//! TeX engine and no external crate is involved.
//!
//! Honest scope of this milestone:
//!
//! - Text only. Every `kind: text` item becomes one `Tj` at its baseline. Any
//!   other item kind is skipped and reported in the warnings list.
//! - By default the only fonts are the base-14 `Times-Roman` (WinAnsiEncoding)
//!   and `Symbol`; nothing is embedded, so glyph shapes and advance widths come
//!   from the viewer's substitutes. Characters outside both are written as `?`
//!   and reported, never silently dropped. Callers can opt in to embedding a
//!   subset of a Unicode TrueType font for those characters ([`RenderOptions`]).
//! - Pages are always white. The writer takes no theme input, so a dark preview
//!   in the Mac app cannot leak into the export.

// Clippy debt in frozen old-engine code (D13): allowed, not refactored.
#![allow(
    clippy::chunks_exact_to_as_chunks,
    clippy::cloned_ref_to_slice_refs,
    clippy::manual_range_patterns,
    clippy::needless_late_init,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::useless_vec,
    reason = "frozen old engine (D13): fixes only; clippy debt is allowed rather than refactored"
)]

pub mod cff;
pub mod compare;
pub mod deflate;
pub mod embed;
pub mod encoding;
pub mod exact;
pub mod images;
pub mod inflate;
pub mod json;
pub mod navigation;
pub mod protocol;
pub mod raster;
pub mod reader;
pub mod sha256;
pub mod truetype;
pub mod type1;
pub mod v2;
pub mod verify;
pub mod writer;

/// One positioned text run. Coordinates follow runtime-v1: points, origin at the
/// top-left of the page, `baseline_y_pt` measured downwards.
#[derive(Debug, Clone, PartialEq)]
pub struct TextItem {
    pub text: String,
    pub x_pt: f64,
    pub baseline_y_pt: f64,
    pub font_size_pt: f64,
    /// `font-hints-v1` face request; `None` means legacy font selection.
    pub font: Option<FontHint>,
}

/// A `font-hints-v1` face request (`docs/contracts/runtime-v1-layout-capabilities.md`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontHint {
    pub family: String,
    pub weight: Weight,
    pub style: Style,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Weight {
    #[default]
    Normal,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum Style {
    #[default]
    Normal,
    Italic,
}

/// A `rules-v1` rectangle: `(x_pt, y_pt)` is the top-left corner in page
/// coordinates (y downward), width and height are positive.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleItem {
    pub x_pt: f64,
    pub y_pt: f64,
    pub width_pt: f64,
    pub height_pt: f64,
}

/// A page item; order is paint order.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Text(TextItem),
    Rule(RuleItem),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub number: u32,
    pub width_pt: f64,
    pub height_pt: f64,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompileResult {
    pub pages: Vec<Page>,
    /// The accepted `layout_capabilities` set. `None` is the legacy route
    /// (no negotiation): unknown item kinds are skipped with a warning and
    /// U+2500 runs are drawn as fraction rules. `Some` is the negotiated
    /// route: unknown kinds are errors, `rule` items need `rules-v1`, and
    /// font hints need `font-hints-v1`.
    pub capabilities: Option<Vec<String>>,
}

impl CompileResult {
    pub fn legacy(&self) -> bool {
        self.capabilities.is_none()
    }

    pub fn accepts(&self, capability: &str) -> bool {
        self.capabilities
            .as_ref()
            .is_some_and(|c| c.iter().any(|x| x == capability))
    }
}

pub const CAP_RULES_V1: &str = "rules-v1";
pub const CAP_FONT_HINTS_V1: &str = "font-hints-v1";

/// The rendered document plus everything the writer had to approximate.
#[derive(Debug, Clone, PartialEq)]
pub struct PdfOutput {
    pub bytes: Vec<u8>,
    /// Human-readable notes about substitutions and skipped input. Empty means
    /// the document was reproduced exactly within this writer's stated scope.
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PdfError {
    /// The input was not valid JSON.
    Json(String),
    /// The envelope was well-formed JSON but not a runtime-v1 `compile_result`.
    Protocol(String),
    /// The pages themselves cannot be represented (e.g. non-positive size).
    Invalid(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::Json(m) => write!(f, "invalid JSON: {m}"),
            PdfError::Protocol(m) => write!(f, "unsupported protocol input: {m}"),
            PdfError::Invalid(m) => write!(f, "cannot render: {m}"),
        }
    }
}

impl std::error::Error for PdfError {}

/// Rendering choices. The default embeds nothing.
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    /// A Unicode OpenType font to embed. `None` keeps the `?` + warning
    /// behaviour for characters outside WinAnsi and Symbol.
    pub embed_font: Option<embed::EmbedFont>,
    /// Which font ordinary text is set in. [`encoding::Face::Embedded`] makes
    /// the embedded font the document face (Symbol, then Times, as
    /// fallbacks); it is ignored with a warning when nothing is embedded.
    /// [`RenderOptions::default_face_for`] picks `Embedded` for Latin Modern.
    pub face: encoding::Face,
}

impl RenderOptions {
    /// The face the CLI implies for a font: Latin Modern (PostScript names
    /// starting with `LM`) becomes the document face; anything else stays a
    /// gap-filler behind Times.
    pub fn default_face_for(font: &embed::EmbedFont) -> encoding::Face {
        if font.font.postscript_name.starts_with("LM") {
            encoding::Face::Embedded
        } else {
            encoding::Face::Times
        }
    }

    /// Embed `font` and use it as the document face.
    pub fn with_document_face(font: embed::EmbedFont) -> Self {
        RenderOptions {
            embed_font: Some(font),
            face: encoding::Face::Embedded,
        }
    }
}

/// Renders positioned pages to PDF bytes with default options (no embedding).
/// See the crate docs for scope.
pub fn render_pdf(result: &CompileResult) -> Result<PdfOutput, PdfError> {
    writer::render(result, &RenderOptions::default())
}

/// Renders positioned pages to PDF bytes with explicit options.
pub fn render_pdf_with(
    result: &CompileResult,
    options: &RenderOptions,
) -> Result<PdfOutput, PdfError> {
    writer::render(result, options)
}

/// Convenience: parse a runtime-v1 `compile_result` envelope and render it
/// with default options. Warnings from both stages (skipped item kinds,
/// substituted characters) are concatenated in order.
pub fn render_envelope(envelope_json: &str) -> Result<PdfOutput, PdfError> {
    render_envelope_with(envelope_json, &RenderOptions::default())
}

/// [`render_envelope`] with explicit options.
pub fn render_envelope_with(
    envelope_json: &str,
    options: &RenderOptions,
) -> Result<PdfOutput, PdfError> {
    let parsed = protocol::parse_compile_result(envelope_json)?;
    let mut out = render_pdf_with(&parsed.result, options)?;
    let mut warnings = parsed.warnings;
    warnings.append(&mut out.warnings);
    out.warnings = warnings;
    Ok(out)
}
